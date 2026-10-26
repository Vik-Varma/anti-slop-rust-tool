//! `blocking-sleep-in-async`, `lock-held-across-await`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

/// On a single-threaded runtime, `thread::sleep` in one task stops every other task: a ticker
/// that should tick every 5 ms gets no ticks at all in while the sleep runs. Counting ticks, rather
/// than timing them, keeps the result independent of how noisy the machine is.
#[test]
fn blocking_sleep_stalls_other_tasks() {
    fn ticks_during_sleep(blocking: bool) -> usize {
        let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().expect("runtime");
        rt.block_on(async move {
            let ticks = Arc::new(AtomicUsize::new(0));
            let t = ticks.clone();
            let ticker = tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                    t.fetch_add(1, Ordering::SeqCst);
                }
            });
            // Let the ticker start before measuring.
            tokio::time::sleep(Duration::from_millis(20)).await;
            let before = ticks.load(Ordering::SeqCst);
            if blocking {
                std::thread::sleep(Duration::from_millis(200));
            } else {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            let during = ticks.load(Ordering::SeqCst) - before;
            ticker.abort();
            during
        })
    }
    assert_eq!(ticks_during_sleep(true), 0, "no other task ran while the thread slept");
    assert!(ticks_during_sleep(false) > 0, "with an async sleep the ticker kept running");
}

/// A std `Mutex` guard held across `.await`: while the holder is suspended, a second future on
/// the same thread blocks in `lock()`, so the holder can never resume to release it. Run in a
/// thread with a timeout, since the deadlock is the result.
#[test]
fn std_lock_across_await_deadlocks() {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().expect("runtime");
        let m = Arc::new(Mutex::new(0u32));
        rt.block_on(async {
            let holder = async {
                let mut g = m.lock().expect("lock");
                tokio::time::sleep(Duration::from_millis(20)).await;
                *g += 1;
            };
            let other = async {
                tokio::task::yield_now().await;
                *m.lock().expect("lock") += 1;
            };
            tokio::join!(holder, other);
        });
        let _ = tx.send(());
    });
    assert!(rx.recv_timeout(Duration::from_secs(2)).is_err(), "the runtime thread is deadlocked");
}

/// The same program with `tokio::sync::Mutex` finishes.
#[test]
fn async_lock_across_await_is_fine() {
    let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().expect("runtime");
    let m = Arc::new(tokio::sync::Mutex::new(0u32));
    rt.block_on(async {
        let holder = async {
            let mut g = m.lock().await;
            tokio::time::sleep(Duration::from_millis(20)).await;
            *g += 1;
        };
        let other = async {
            tokio::task::yield_now().await;
            *m.lock().await += 1;
        };
        tokio::join!(holder, other);
    });
    assert_eq!(*m.try_lock().expect("free"), 2);
}
