//! Panics while another panic is unwinding, from a destructor.
use std::panic;

struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        // What `self.file.take().unwrap()` does in a destructor when the invariant breaks.
        let missing: Option<u8> = None;
        missing.unwrap();
    }
}

fn main() {
    let result = panic::catch_unwind(|| {
        let _g = Guard;
        panic!("first failure");
    });
    // Never reached: the second panic aborts the process instead of unwinding to here.
    println!("caught: {}", result.is_err());
}
