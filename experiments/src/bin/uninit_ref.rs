//! Asks `mem::uninitialized` for a reference, a type with no valid "garbage" value.
#![allow(deprecated, invalid_value)]

fn main() {
    let r = std::panic::catch_unwind(|| {
        // SAFETY: none; this is the experiment.
        let x: &u8 = unsafe { std::mem::uninitialized() };
        println!("got {:p}", x);
    });
    println!("caught: {}", r.is_err());
}
