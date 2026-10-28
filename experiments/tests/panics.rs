//! `unwrap`, `todo`, `panic-in-drop`, `catch-unwind-under-panic-abort`.

use std::panic;

use anti_slop_experiments::run_helper;

fn panic_message(r: Box<dyn std::any::Any + Send>) -> String {
    r.downcast_ref::<String>().cloned().or_else(|| r.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default()
}

/// `unwrap` panics, and its message carries nothing about why the value was expected.
#[test]
fn unwrap_panics_without_context() {
    let config: Option<&str> = None;
    let err = panic::catch_unwind(|| config.unwrap()).expect_err("unwrap on None panics");
    let msg = panic_message(err);
    assert_eq!(msg, "called `Option::unwrap()` on a `None` value");

    let err = panic::catch_unwind(|| config.expect("config is loaded before the server starts"))
        .expect_err("expect panics too");
    assert!(panic_message(err).contains("config is loaded"));
}

/// A panic in one thread that holds a lock poisons it for every other user.
#[test]
fn a_panic_poisons_shared_state() {
    let shared = std::sync::Arc::new(std::sync::Mutex::new(0u32));
    let s = shared.clone();
    let _ = std::thread::spawn(move || {
        let _g = s.lock().expect("first lock");
        let v: Result<u32, &str> = Err("bad input");
        v.unwrap();
    })
    .join();
    assert!(shared.lock().is_err(), "the mutex is poisoned for everyone after the panic");
}

/// `todo!()` and `unimplemented!()` compile anywhere and panic when reached.
#[test]
fn todo_compiles_and_panics_at_runtime() {
    fn parse_header(_b: &[u8]) -> u32 {
        todo!("parse the header")
    }
    let err = panic::catch_unwind(|| parse_header(b"x")).expect_err("todo panics");
    assert!(panic_message(err).starts_with("not yet implemented"));
}

/// A panic from a destructor while a panic is unwinding aborts the process: no `catch_unwind`,
/// no other destructors, no clean shutdown.
#[test]
fn panic_in_drop_during_unwind_aborts() {
    let out = run_helper(env!("CARGO_BIN_EXE_panic_in_drop"), &[]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stdout.contains("caught"), "catch_unwind never got control");
    assert!(!out.status.success(), "the process died instead of finishing: {:?}", out.status);
    assert!(stderr.contains("called `Option::unwrap()` on a `None` value"), "the second panic ran: {stderr}");
    // Unix std names the abort; Windows unwinds through SEH and fails fast without the line.
    #[cfg(not(windows))]
    assert!(stderr.contains("panic in a destructor during cleanup"), "{stderr}");
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(out.status.signal(), Some(6), "SIGABRT, not an ordinary exit");
    }
}

/// `catch_unwind` under `panic = "abort"` (the profile in panic-abort/Cargo.toml): the process
/// aborts at the panic and the `Err` arm never runs. The same source under `unwind` recovers.
/// Tests always unwind, so this has to build and run a binary.
#[test]
fn catch_unwind_does_not_catch_under_panic_abort() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/panic-abort/Cargo.toml");
    let run = |config: &[&str]| {
        std::process::Command::new(&cargo)
            .args(["run", "--quiet", "--manifest-path", manifest])
            .args(config)
            .output()
            .expect("cargo runs")
    };
    let abort = run(&[]);
    assert!(!abort.status.success(), "the process died at the panic: {:?}", abort.status);
    assert!(!String::from_utf8_lossy(&abort.stdout).contains("recovered"), "the Err arm never ran");
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(abort.status.signal(), Some(6), "SIGABRT, not an ordinary exit");
    }
    let unwind = run(&["--config", "profile.dev.panic=\"unwind\""]);
    assert!(unwind.status.success(), "{}", String::from_utf8_lossy(&unwind.stderr));
    assert!(String::from_utf8_lossy(&unwind.stdout).contains("recovered"), "same source, unwinding: the arm runs");
}
