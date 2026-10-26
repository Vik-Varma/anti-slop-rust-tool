//! Executable evidence: each test reproduces the failure a rule claims the
//! flagged pattern causes, next to a control showing the recommended fix
//! does not fail. Run with `cargo test --test demonstrations -- --nocapture`.

// These tests contain the bad patterns on purpose; Clippy flags several of them too.
#![allow(clippy::unused_io_amount, clippy::await_holding_lock, clippy::eq_op, clippy::single_match)]

use std::io::Write;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Runs `f` on a thread; returns `Some(result)` if it finished within `timeout`.
/// A deadlocked thread is leaked.
fn finishes_within<T: Send + 'static>(timeout: Duration, f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(timeout).ok()
}

// ---------------------------------------------------------------- lock-in-scrutinee

#[test]
fn lock_in_match_scrutinee_deadlocks() {
    let m = Arc::new(Mutex::new(vec![1u32]));
    let bad = {
        let m = m.clone();
        finishes_within(Duration::from_secs(2), move || {
            // The guard from the scrutinee lives until the end of the match.
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match m.lock().unwrap().first() {
                Some(_) => m.lock().unwrap().push(2),
                None => {}
            }));
            r.is_ok()
        })
    };
    // std documents re-locking as "might panic or deadlock": either way it fails.
    assert!(bad != Some(true), "re-locking inside the match must not succeed");
    println!("lock in scrutinee: {}", if bad.is_none() { "deadlocked" } else { "panicked" });

    let good = finishes_within(Duration::from_secs(2), || {
        let m = Mutex::new(vec![1u32]);
        let first = m.lock().unwrap().first().copied(); // guard dropped here
        if first.is_some() {
            m.lock().unwrap().push(2);
        }
        m.into_inner().unwrap().len()
    });
    assert_eq!(good, Some(2));
}

// ---------------------------------------------------------------- lock-across-await

#[test]
fn std_guard_across_await_deadlocks() {
    fn run(drop_before_await: bool) -> Option<u32> {
        finishes_within(Duration::from_secs(2), move || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
            let local = tokio::task::LocalSet::new();
            local.block_on(&rt, async move {
                let m = std::rc::Rc::new(Mutex::new(0u32));
                let m2 = m.clone();
                let a = tokio::task::spawn_local(async move {
                    let mut g = m2.lock().unwrap();
                    *g += 1;
                    if drop_before_await {
                        drop(g);
                        tokio::time::sleep(Duration::from_millis(20)).await;
                    } else {
                        // Task is suspended while holding the std guard...
                        tokio::time::sleep(Duration::from_millis(20)).await;
                        *g += 1;
                    }
                });
                let b = tokio::task::spawn_local(async move {
                    // ...so this blocks the only executor thread and `a` can never resume.
                    *m.lock().unwrap() += 1;
                });
                a.await.unwrap();
                b.await.unwrap();
                0
            })
        })
    }
    assert_eq!(run(true), Some(0), "control (guard dropped before .await) must finish");
    assert_eq!(run(false), None, "guard held across .await must deadlock");
}

// ---------------------------------------------------------------- blocking-in-async

#[test]
fn blocking_sleep_stalls_other_tasks() {
    async fn ticker_delay(block: bool) -> Duration {
        let ticker = tokio::spawn(async {
            let start = Instant::now();
            tokio::time::sleep(Duration::from_millis(10)).await;
            start.elapsed()
        });
        let worker = tokio::spawn(async move {
            tokio::task::yield_now().await;
            if block {
                std::thread::sleep(Duration::from_millis(300));
            } else {
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
        });
        let d = ticker.await.unwrap();
        worker.await.unwrap();
        d
    }
    let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap();
    let blocked = rt.block_on(ticker_delay(true));
    let control = rt.block_on(ticker_delay(false));
    println!("10ms timer fired after {blocked:?} with std::thread::sleep, {control:?} with tokio::time::sleep");
    assert!(blocked >= Duration::from_millis(250), "blocking call should stall the timer task");
    assert!(control < Duration::from_millis(150), "async sleep should not stall it");
}

// ---------------------------------------------------------------- float-equality

#[test]
fn float_literal_equality_fails() {
    let a = 0.1f64;
    let b = 0.2f64;
    // anti-slop: allow(float-equality) -- demonstrating the flagged pattern
    assert!(a + b != 0.3, "0.1 + 0.2 == 0.3 is false in binary floating point");
    let tenth_sum: f64 = (0..10).map(|_| 0.1).sum();
    assert!(tenth_sum != 1.0);
    assert!(((a + b) - 0.3).abs() < 1e-12, "comparison with a tolerance works");
    // Exactly representable literals (dyadic rationals) compare exactly.
    let (half, quarter) = (std::hint::black_box(0.5f64), std::hint::black_box(0.25f64));
    assert!(half + quarter == 0.75);
}

// ---------------------------------------------------------------- unused-io-amount

#[test]
fn ignored_write_count_loses_data() {
    // `Cursor<&mut [u8]>` is a std writer that performs short writes when full;
    // any socket or pipe may do the same (the `Write` contract allows it).
    let mut storage = [0u8; 8];
    let mut first = std::io::Cursor::new(&mut storage[..]);
    first.write(b"0123").unwrap();
    let n = first.write(b"456789").unwrap(); // only 4 of 6 bytes fit
    assert_eq!(n, 4, "a short write happened, and code that ignores `n` never knows");

    /// A legal `Write` impl that accepts at most 3 bytes per call.
    struct Trickle(Vec<u8>);
    impl Write for Trickle {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let n = buf.len().min(3);
            self.0.extend_from_slice(&buf[..n]);
            Ok(n)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut lossy = Trickle(Vec::new());
    lossy.write(b"hello world").unwrap(); // the flagged pattern
    assert_eq!(lossy.0, b"hel");
    let mut ok = Trickle(Vec::new());
    ok.write_all(b"hello world").unwrap(); // the fix
    assert_eq!(ok.0, b"hello world");
}

// ---------------------------------------------------------------- todo-in-production / unwrap-in-result-fn

#[test]
fn todo_panics_when_reached() {
    fn stub() -> u32 {
        todo!()
    }
    let r = std::panic::catch_unwind(stub);
    assert!(r.is_err());
}

#[test]
fn unwrap_in_result_fn_crashes_instead_of_returning_err() {
    fn parse_unwrap(s: &str) -> Result<u32, std::num::ParseIntError> {
        Ok(s.parse::<u32>().unwrap() * 2)
    }
    fn parse_question(s: &str) -> Result<u32, std::num::ParseIntError> {
        Ok(s.parse::<u32>()? * 2)
    }
    assert!(parse_question("x").is_err(), "the caller gets an error it can handle");
    assert!(std::panic::catch_unwind(|| parse_unwrap("x")).is_err(), "the whole thread panics");
}

// ---------------------------------------------------------------- panic-in-drop

/// Child half of `panic_in_drop_during_unwind_aborts`; run only in a subprocess.
#[test]
#[ignore]
fn child_panic_in_drop() {
    if std::env::var("ANTI_SLOP_CHILD").is_err() {
        return;
    }
    struct Bomb;
    impl Drop for Bomb {
        fn drop(&mut self) {
            panic!("drop failed");
        }
    }
    let _b = Bomb;
    panic!("first panic"); // unwinding drops `_b`, which panics again
}

#[test]
fn panic_in_drop_during_unwind_aborts() {
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_panic_in_drop", "--ignored", "--nocapture", "--test-threads=1"])
        .env("ANTI_SLOP_CHILD", "1")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    // A normal test failure exits with code 101; an abort kills the process by signal.
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(out.status.signal(), Some(6), "expected SIGABRT; stderr:\n{stderr}");
    }
    assert!(!out.status.success());
}

// ---------------------------------------------------------------- unbounded-send-sync

#[test]
fn unbounded_send_impl_makes_rc_sendable() {
    #[allow(dead_code)]
    struct Wrapper<T>(T);
    // The flagged pattern:
    // anti-slop: allow(unbounded-send-sync)
    unsafe impl<T> Send for Wrapper<T> {}
    fn assert_send<T: Send>() {}
    // `Rc` is deliberately !Send, yet this compiles: the impl lets safe code
    // move an `Rc` to another thread, racing on its reference count. Miri
    // reports the resulting data race (validation/ub/src/bin/send_race.rs).
    assert_send::<Wrapper<std::rc::Rc<u8>>>();
}

// ---------------------------------------------------------------- exit-in-library

/// Child half of `process_exit_skips_destructors`; run only in a subprocess.
#[test]
#[ignore]
fn child_process_exit() {
    let Ok(path) = std::env::var("ANTI_SLOP_EXIT_FILE") else { return };
    let mut w = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
    write!(w, "important data").unwrap();
    if std::env::var("ANTI_SLOP_EXIT_MODE").as_deref() == Ok("exit") {
        std::process::exit(0); // the BufWriter is never dropped, so never flushed
    }
}

#[test]
fn process_exit_skips_destructors() {
    let run = |mode: &str| {
        let path = std::env::temp_dir().join(format!("anti-slop-exit-{}-{mode}", std::process::id()));
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "child_process_exit", "--ignored", "--test-threads=1"])
            .env("ANTI_SLOP_EXIT_FILE", &path)
            .env("ANTI_SLOP_EXIT_MODE", mode)
            .output()
            .unwrap()
            .status;
        assert!(status.success());
        let content = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        content
    };
    assert_eq!(run("exit"), "", "process::exit lost the buffered write");
    assert_eq!(run("return"), "important data");
}

// ---------------------------------------------------------------- deny-warnings

#[test]
fn deny_warnings_breaks_build_when_a_dependency_deprecates_something() {
    let dir = std::env::temp_dir().join(format!("anti-slop-deny-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // Version 2 of a dependency deprecates a function (a semver-compatible change).
    std::fs::write(dir.join("dep.rs"), "#[deprecated(note = \"use g\")] pub fn f() {} pub fn g() {}").unwrap();
    let build = |src: &str, name: &str| {
        std::fs::write(dir.join(format!("{name}.rs")), src).unwrap();
        std::process::Command::new("rustc")
            .current_dir(&dir)
            .args([
                "--edition",
                "2021",
                "--crate-type",
                "lib",
                &format!("{name}.rs"),
                "--extern",
                "dep=libdep.rlib",
                "-L",
                ".",
            ])
            .output()
            .unwrap()
            .status
            .success()
    };
    let dep = std::process::Command::new("rustc")
        .current_dir(&dir)
        .args(["--edition", "2021", "--crate-type", "lib", "--crate-name", "dep", "dep.rs"])
        .status()
        .unwrap();
    assert!(dep.success());
    let with_deny = build("#![deny(warnings)]\npub fn user() { dep::f(); }", "user_deny");
    let without = build("pub fn user() { dep::f(); }", "user_plain");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!with_deny, "the downstream crate no longer compiles");
    assert!(without, "without deny(warnings) it compiles with a warning");
}
