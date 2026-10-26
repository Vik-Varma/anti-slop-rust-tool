// Compiles warning-free on Rust 1.76. Newer compilers added lints this code trips, so under
// `#![deny(warnings)]` the same, unchanged source stops building.
#![deny(warnings)]

static mut HITS: u32 = 0;

pub fn hits() -> u32 {
    // SAFETY: single-threaded caller; this file is about lints, not races.
    unsafe {
        let r = &HITS;
        *r
    }
}
