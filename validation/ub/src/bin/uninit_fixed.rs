//! Control for `uninit_assume_init`: write before `assume_init`. Miri is clean.
use std::mem::MaybeUninit;
fn main() {
    let mut m = MaybeUninit::<u32>::uninit();
    m.write(41);
    let x = unsafe { m.assume_init() };
    println!("{}", x + 1);
}
