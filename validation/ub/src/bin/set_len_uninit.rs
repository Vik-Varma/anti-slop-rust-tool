//! `set-len-uninit`: `set_len` right after `with_capacity` exposes uninitialised memory.
//! `cargo +nightly miri run --bin set_len_uninit` reports undefined behaviour.
fn main() {
    let n = 16;
    let mut v: Vec<u8> = Vec::with_capacity(n);
    unsafe {
        v.set_len(n);
    }
    // Reading the elements reads uninitialised memory.
    let sum: u32 = v.iter().map(|&b| b as u32).sum();
    println!("{sum}");
}
