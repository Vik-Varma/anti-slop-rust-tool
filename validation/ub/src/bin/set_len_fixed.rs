//! Control for `set_len_uninit`: initialise first (here: `vec![0; n]`). Miri is clean.
fn main() {
    let n = 16;
    let v: Vec<u8> = vec![0; n];
    let sum: u32 = v.iter().map(|&b| b as u32).sum();
    println!("{sum}");
}
