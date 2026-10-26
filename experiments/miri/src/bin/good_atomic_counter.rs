//! The fix: an atomic.
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn main() {
    let t = thread::spawn(|| COUNTER.fetch_add(1, Ordering::Relaxed));
    COUNTER.fetch_add(1, Ordering::Relaxed);
    t.join().unwrap();
    assert_eq!(COUNTER.load(Ordering::Relaxed), 2);
}
