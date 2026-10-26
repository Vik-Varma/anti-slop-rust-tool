//! `blocking-sleep-in-async`, `lock-held-across-await`.

use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

/// On a single-threaded runtime, `thread::sleep` in one task stops every other task: a ticker
/// that should run every 10 ms waits for the whole sleep.
#[test]
fn blocking_sleep_stalls_other_tasks() {
    fn worst_tick_gap(blocking: bool) -> Duration {
        let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().expect("runtime");
        rt.block_on(async move {
            let ticker = tokio::spawn(async {
                let mut worst = Duration::ZERO;
                let mut last = Instant::now();
                for _ in 0..20 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    worst = worst.max(last.elapsed());
                    last = Instant::now();
                }
                worst
            });
            let sleeper = tokio::spawn(async move {
                for _ in 0..2 {
                    if blocking {
                        std::thread::sleep(Duration::from_millis(100));
                    } else {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    tokio::task::yield_now().await;
                }
            });
            sleeper.await.expect("sleeper");
            ticker.await.expect("ticker")
        })
    }
    let blocked = worst_tick_gap(true);
    let fine = worst_tick_gap(false);
    assert!(blocked >= Duration::from_millis(95), "ticker stalled for {blocked:?}");
    assert!(fine < Duration::from_millis(60), "async sleep let the ticker run: {fine:?}");
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
