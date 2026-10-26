//! Two threads bump a `static mut` counter. Edition 2021 so `&raw`-free code still compiles.
use std::thread;

static mut COUNTER: u64 = 0;

fn main() {
    let t = thread::spawn(|| {
        // SAFETY: none; this is the bad example the rule flags.
        unsafe { COUNTER += 1 };
    });
    // SAFETY: as above.
    unsafe { COUNTER += 1 };
    t.join().unwrap();
}
