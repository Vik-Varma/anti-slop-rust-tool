//! `float-eq-inexact`, `len-truncating-cast`.

/// Decimal fractions like 0.1 have no exact binary value, so equality with them fails on
/// results that are "obviously" equal.
#[test]
fn inexact_literals_break_equality() {
    let sum = 0.1 + 0.2;
    assert!(sum != 0.3);
    let tenth_ten_times: f64 = (0..10).map(|_| 0.1).sum();
    assert!(tenth_ten_times != 1.0);
    // Exactly representable literals behave: halves and quarters add exactly.
    assert!(0.25 + 0.25 == 0.5);
    assert!(0.5 + 0.5 == 1.0);
}

/// `as` between integers wraps silently; nothing at run time says the length was wrong.
#[test]
fn len_as_narrow_int_wraps() {
    let v = vec![0u8; 65_537];
    assert_eq!(v.len() as u16, 1);
    assert!(u16::try_from(v.len()).is_err(), "try_from reports it");
}

/// The 32-bit case on a real allocation: 4 GiB + 1 bytes, which the OS backs lazily.
#[cfg(target_pointer_width = "64")]
#[test]
fn len_as_u32_wraps_for_a_real_buffer() {
    let v = vec![0u8; (1usize << 32) + 1];
    assert_eq!(v.len() as u32, 1, "a 4 GiB message would be framed as 1 byte long");
}
