//! `mem::uninitialized` for an integer: std no longer aborts here, but it is still UB.
#![allow(deprecated, invalid_value)]

fn main() {
    // SAFETY: none; this is the bad example.
    let x: u32 = unsafe { std::mem::uninitialized() };
    if x == 7 {
        println!("seven");
    }
}
