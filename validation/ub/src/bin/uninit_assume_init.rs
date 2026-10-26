//! `uninit-memory`: `MaybeUninit::uninit().assume_init()` is UB. Miri reports it.
use std::mem::MaybeUninit;
fn main() {
    let x: u32 = unsafe { MaybeUninit::uninit().assume_init() };
    println!("{}", x + 1);
}
