//! Control for `send_race`: with `T: Send` the same program no longer compiles with
//! `Rc`, so it is written with `Arc`. Miri is clean.
use std::sync::Arc;
use std::thread;

struct Wrapper<T>(T);
unsafe impl<T: Send> Send for Wrapper<T> {}

fn main() {
    let rc = Arc::new(0u8);
    let w = Wrapper(rc.clone());
    let h = thread::spawn(move || {
        let w = w;
        for _ in 0..10 {
            let _c = w.0.clone();
        }
    });
    for _ in 0..10 {
        let _c = rc.clone();
    }
    h.join().unwrap();
}
