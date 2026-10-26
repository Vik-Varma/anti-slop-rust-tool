//! `unbounded-send-sync`: `unsafe impl<T> Send for Wrapper<T>` lets safe code send an
//! `Rc` to another thread. Both threads then update the non-atomic reference count:
//! Miri reports a data race.
use std::rc::Rc;
use std::thread;

struct Wrapper<T>(T);
unsafe impl<T> Send for Wrapper<T> {}

fn main() {
    let rc = Rc::new(0u8);
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
