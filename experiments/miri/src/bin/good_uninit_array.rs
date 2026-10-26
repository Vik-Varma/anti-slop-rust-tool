//! The documented sound pattern: an uninitialised array of `MaybeUninit`, then written in full.
use std::mem::MaybeUninit;

fn main() {
    // SAFETY: an array of `MaybeUninit` needs no initialisation.
    let mut a: [MaybeUninit<u32>; 4] = unsafe { MaybeUninit::uninit().assume_init() };
    for (i, slot) in a.iter_mut().enumerate() {
        slot.write(i as u32);
    }
    // SAFETY: every element was written above.
    let a: [u32; 4] = unsafe { std::mem::transmute(a) };
    assert_eq!(a, [0, 1, 2, 3]);
}
