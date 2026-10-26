use syn::spanned::Spanned;

use super::{is_method, peel};
use crate::analyzer::Analyzer;

pub fn binary(a: &mut Analyzer, node: &syn::ExprBinary) {
    if !matches!(node.op, syn::BinOp::Eq(_) | syn::BinOp::Ne(_)) {
        return;
    }
    for side in [&node.left, &node.right] {
        let Some(lit) = float_literal(side) else {
            continue;
        };
        let bits = if lit.suffix() == "f32" { 24 } else { 53 };
        if exactly_representable(lit.base10_digits(), bits) == Some(false) {
            a.emit(
                "float-eq-inexact",
                node.op.span(),
                format!(
                    "`{}` has no exact binary value, so `==` against a computed float is almost never true; compare with a tolerance",
                    lit.base10_digits()
                ),
            );
            return;
        }
    }
}

fn float_literal(e: &syn::Expr) -> Option<&syn::LitFloat> {
    match peel(e) {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Float(f),
            ..
        }) => Some(f),
        syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => float_literal(&u.expr),
        _ => None,
    }
}

/// Whether the decimal `digits` (as `syn` normalises them: `0.1`, `1e-3`, `2.5E10`) is exactly a
/// binary float with a `mantissa_bits`-bit significand. `None` when it's too large to decide here.
pub fn exactly_representable(digits: &str, mantissa_bits: u32) -> Option<bool> {
    let lower = digits.to_ascii_lowercase();
    let (number, exp) = match lower.split_once('e') {
        Some((n, e)) => (n, e.parse::<i32>().ok()?),
        None => (lower.as_str(), 0),
    };
    let (int, frac) = number.split_once('.').unwrap_or((number, ""));
    let mut m: u128 = format!("{int}{frac}")
        .trim_start_matches('0')
        .parse()
        .unwrap_or(0);
    let mut e10 = exp - i32::try_from(frac.len()).ok()?;
    if m == 0 {
        return Some(true);
    }
    while m % 10 == 0 {
        m /= 10;
        e10 += 1;
    }
    let mut odd = if e10 >= 0 {
        m.checked_mul(10u128.checked_pow(u32::try_from(e10).ok()?)?)?
    } else {
        let five = 5u128.checked_pow(u32::try_from(-e10).ok()?)?;
        if m % five != 0 {
            return Some(false);
        }
        m / five
    };
    while odd % 2 == 0 {
        odd /= 2;
    }
    Some(128 - odd.leading_zeros() <= mantissa_bits)
}

const NARROW: &[&str] = &["u8", "u16", "u32", "i8", "i16", "i32"];

pub fn cast(a: &mut Analyzer, node: &syn::ExprCast) {
    let syn::Type::Path(ty) = node.ty.as_ref() else {
        return;
    };
    let Some(target) = ty.path.get_ident().map(|i| i.to_string()) else {
        return;
    };
    if !NARROW.contains(&target.as_str()) || is_method(&node.expr, "len", 0).is_none() {
        return;
    }
    a.emit(
        "len-truncating-cast",
        node.as_token.span(),
        format!(
            "`len() as {target}` silently wraps past {target}::MAX; use `{target}::try_from(..)`"
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::exactly_representable as exact;

    #[test]
    fn decimal_fractions() {
        assert_eq!(exact("0.5", 53), Some(true));
        assert_eq!(exact("0.25", 53), Some(true));
        assert_eq!(exact("0.1", 53), Some(false));
        assert_eq!(exact("0.3", 53), Some(false));
        assert_eq!(exact("1.0", 53), Some(true));
        assert_eq!(exact("0.0", 53), Some(true));
        assert_eq!(exact("2.5e10", 53), Some(true));
        assert_eq!(exact("1e-3", 53), Some(false));
        assert_eq!(exact("0.375", 24), Some(true));
    }

    #[test]
    fn significand_width() {
        // 2^24 + 1 needs 25 bits: exact in f64, not in f32.
        assert_eq!(exact("16777217.0", 53), Some(true));
        assert_eq!(exact("16777217.0", 24), Some(false));
    }

    #[test]
    fn agrees_with_the_hardware() {
        for s in [
            "0.1", "0.2", "0.5", "1.5", "3.14", "100.0", "0.125", "1e-3", "6.25e-2",
        ] {
            let v: f64 = s.parse().expect("valid float");
            // Exact iff the shortest round-trip decimal of the f64 printed with all its digits ends.
            let exact_by_hw = format!("{v:.60}").trim_end_matches('0').len() < 40;
            assert_eq!(exact(s, 53), Some(exact_by_hw), "{s}");
        }
    }
}
