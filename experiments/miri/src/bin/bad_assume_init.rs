//! `MaybeUninit::uninit().assume_init()` for a plain integer.
use std::mem::MaybeUninit;

fn main() {
    // SAFETY: none; this is the bad example.
    let x: u32 = unsafe { MaybeUninit::uninit().assume_init() };
    println!("{}", x);
}
