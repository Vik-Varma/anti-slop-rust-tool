//! Assorted correctness and API rules.

use super::{Input, Rule, RuleMeta, Sink};
use crate::ast_util::macro_exprs;
use crate::context::FileContext;
use crate::finding::{Category, Evidence, Severity};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{BinOp, Expr, FnArg, Lit, Type};

pub struct FloatEquality;
pub struct PtrArg;
pub struct DenyWarnings;

static FLOAT_EQ: RuleMeta = RuleMeta {
    id: "float-equality",
    category: Category::Correctness,
    severity: Severity::Warning,
    evidence: Evidence::Strong,
    default_enabled: true,
    summary: "`==`/`!=` against a float literal that binary floating point cannot represent",
    rationale: "Decimal fractions like 0.1 have no exact binary representation, so a computed \
value almost never equals the literal exactly: `0.1 + 0.2 == 0.3` is false (reproduced in \
tests/demonstrations.rs). Goldberg (ACM Computing Surveys, 1991) explains the rounding behaviour. \
Compare with a tolerance (`(a - b).abs() < eps`). Only literals that are not exactly \
representable are flagged: `x == 0.0`, `x == 0.5` or `x == 1.0` can be intentional exact \
checks, which is why Clippy's broader `float_cmp` is among its most-suppressed lints.",
    sources: &[
        "D. Goldberg. What every computer scientist should know about floating-point arithmetic. ACM Computing Surveys 23(1), 1991. doi:10.1145/103162.103163",
        "Clippy lint `float_cmp` (pedantic).",
        "tests/demonstrations.rs::float_literal_equality_fails (this repository)",
    ],
};

static PTR_ARG: RuleMeta = RuleMeta {
    id: "ptr-arg",
    category: Category::Maintainability,
    severity: Severity::Info,
    evidence: Evidence::Weak,
    default_enabled: false,
    summary: "Parameter of type `&Vec<T>`, `&String`, `&PathBuf` or `&Box<T>`",
    rationale: "`&[T]`, `&str`, `&Path` and `&T` accept strictly more callers (arrays, slices, \
literals, borrowed data) and avoid a double indirection. Clippy `ptr_arg` (style, warn by \
default); Rust Design Patterns idiom 'Use borrowed types for arguments'; API Guidelines \
C-GENERIC. Trait implementations are skipped. Opt-in: our benchmark found no measurable speed difference between `&Vec<T>` and `&[T]` parameters, and no association with bug fixes; the benefit is API flexibility only.",
    sources: &[
        "Clippy lint `ptr_arg`.",
        "Rust Design Patterns, 'Use borrowed types for arguments': https://rust-unofficial.github.io/patterns/idioms/coercion-arguments.html",
    ],
};

static DENY_WARNINGS: RuleMeta = RuleMeta {
    id: "deny-warnings",
    category: Category::Maintainability,
    severity: Severity::Warning,
    evidence: Evidence::Strong,
    default_enabled: true,
    summary: "`#![deny(warnings)]` in source code",
    rationale: "New compiler versions add lints and deprecations, so a crate that denies all \
warnings in source stops compiling for users when the toolchain updates, opting out of Rust's \
stability guarantees. Rust Design Patterns lists it as an anti-pattern; use \
`RUSTFLAGS=\"-D warnings\"` in CI or deny specific lints instead. Verified: tests/demonstrations.rs shows a downstream crate failing to compile after a dependency deprecates a function.",
    sources: &[
        "Rust Design Patterns, '#![deny(warnings)]': https://rust-unofficial.github.io/patterns/anti_patterns/deny-warnings.html",
    ],
};

/// Is the decimal float literal exactly representable as an f64 (or f32)?
pub fn exactly_representable(text: &str, is_f32: bool) -> bool {
    let t: String = text.chars().filter(|&c| c != '_').collect();
    let t = t.trim_end_matches("f64").trim_end_matches("f32");
    let (mantissa, exp) = match t.find(['e', 'E']) {
        Some(i) => (&t[..i], t[i + 1..].parse::<i32>().unwrap_or(0)),
        None => (t, 0),
    };
    let (int_part, frac_part) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{int_part}{frac_part}");
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return true; // zero
    }
    let Ok(mut m) = digits.parse::<u128>() else { return false };
    let mut e10 = exp - frac_part.len() as i32;
    while e10 < 0 && m % 10 == 0 {
        m /= 10;
        e10 += 1;
    }
    let precision_bits = if is_f32 { 24 } else { 53 };
    let fits = |v: u128| v < (1u128 << precision_bits);
    if e10 >= 0 {
        // Integer value m * 10^e10; check it fits in the mantissa.
        let mut v = m;
        for _ in 0..e10 {
            match v.checked_mul(10) {
                Some(n) => v = n,
                None => return false,
            }
        }
        let tz = v.trailing_zeros();
        return fits(v >> tz);
    }
    // value = m / (2^k * 5^k); representable iff 5^k divides m and the rest fits.
    let mut k5 = -e10;
    while k5 > 0 && m % 5 == 0 {
        m /= 5;
        k5 -= 1;
    }
    k5 == 0 && fits(m >> m.trailing_zeros())
}

fn float_literal(e: &Expr) -> Option<&syn::LitFloat> {
    match e {
        Expr::Lit(l) => match &l.lit {
            Lit::Float(f) => Some(f),
            _ => None,
        },
        Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => float_literal(&u.expr),
        Expr::Paren(p) => float_literal(&p.expr),
        _ => None,
    }
}

impl Rule for FloatEquality {
    fn meta(&self) -> &'static RuleMeta {
        &FLOAT_EQ
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        struct V<'s, 'c, 'a> {
            sink: &'s mut Sink<'c, 'a>,
        }
        impl<'ast> Visit<'ast> for V<'_, '_, '_> {
            fn visit_expr_binary(&mut self, b: &'ast syn::ExprBinary) {
                if matches!(b.op, BinOp::Eq(_) | BinOp::Ne(_))
                    && let Some(lit) = float_literal(&b.left).or_else(|| float_literal(&b.right))
                {
                    let is_f32 = lit.suffix() == "f32";
                    if !exactly_representable(&lit.to_string(), is_f32) {
                        self.sink.emit_span(
                                b.op.span(),
                                format!("exact comparison with `{lit}`, which is not representable in binary floating point; compare with a tolerance"),
                            );
                    }
                }
                visit::visit_expr_binary(self, b);
            }
            fn visit_macro(&mut self, mac: &'ast syn::Macro) {
                if let Some(exprs) = macro_exprs(mac) {
                    for e in &exprs {
                        self.visit_expr(e);
                    }
                }
            }
        }
        V { sink }.visit_file(&input.ctx.ast);
    }
}

fn ptr_arg_suggestion(ty: &Type) -> Option<(&'static str, &'static str)> {
    let Type::Reference(r) = ty else { return None };
    if r.mutability.is_some() {
        return None;
    }
    let Type::Path(tp) = &*r.elem else { return None };
    let last = tp.path.segments.last()?;
    match last.ident.to_string().as_str() {
        "Vec" => Some(("&Vec<T>", "&[T]")),
        "String" => Some(("&String", "&str")),
        "PathBuf" => Some(("&PathBuf", "&Path")),
        "Box" => Some(("&Box<T>", "&T")),
        _ => None,
    }
}

impl Rule for PtrArg {
    fn meta(&self) -> &'static RuleMeta {
        &PTR_ARG
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        struct V<'s, 'c, 'a> {
            sink: &'s mut Sink<'c, 'a>,
            ctx: &'c FileContext<'a>,
            in_trait: bool,
        }
        impl V<'_, '_, '_> {
            fn sig(&mut self, sig: &syn::Signature) {
                if self.in_trait || sig.abi.is_some() {
                    return;
                }
                for arg in &sig.inputs {
                    let FnArg::Typed(pt) = arg else { continue };
                    if let Some((have, want)) = ptr_arg_suggestion(&pt.ty)
                        && self.ctx.is_production_line(pt.ty.span().start().line)
                    {
                        self.sink.emit_span(
                            pt.ty.span(),
                            format!("parameter takes `{have}`; `{want}` accepts more callers"),
                        );
                    }
                }
            }
        }
        impl<'ast> Visit<'ast> for V<'_, '_, '_> {
            fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
                let prev = self.in_trait;
                self.in_trait = false;
                self.sig(&f.sig);
                visit::visit_item_fn(self, f);
                self.in_trait = prev;
            }
            fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
                let prev = self.in_trait;
                self.in_trait = i.trait_.is_some();
                visit::visit_item_impl(self, i);
                self.in_trait = prev;
            }
            fn visit_item_trait(&mut self, t: &'ast syn::ItemTrait) {
                let prev = self.in_trait;
                self.in_trait = true;
                visit::visit_item_trait(self, t);
                self.in_trait = prev;
            }
            fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
                self.sig(&f.sig);
                visit::visit_impl_item_fn(self, f);
            }
        }
        V { ctx: sink.ctx(), sink, in_trait: false }.visit_file(&input.ctx.ast);
    }
}

impl Rule for DenyWarnings {
    fn meta(&self) -> &'static RuleMeta {
        &DENY_WARNINGS
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        for attr in &input.ctx.ast.attrs {
            if !input.ctx.is_production_line(attr.span().start().line) {
                continue;
            }
            let path = attr.path();
            if !(path.is_ident("deny") || path.is_ident("forbid")) {
                continue;
            }
            let Ok(list) = attr.meta.require_list() else { continue };
            let lints = list.tokens.to_string();
            if lints.split(|c: char| c == ',' || c.is_whitespace()).any(|l| l == "warnings") {
                sink.emit_span(
                    attr.span(),
                    "`#![deny(warnings)]` breaks the build on new compiler warnings; deny in CI instead",
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::testing::{check_rule, lines};

    #[test]
    fn representability() {
        for ok in ["0.0", "0.5", "1.0", "-2.25", "1e3", "0.125", "1_000.0", "3.0f32", "1e10"] {
            assert!(exactly_representable(ok.trim_start_matches('-'), ok.ends_with("f32")), "{ok}");
        }
        for bad in ["0.1", "0.2", "3.14", "1e-3", "0.3f32", "2.675"] {
            assert!(!exactly_representable(bad, bad.ends_with("f32")), "{bad}");
        }
    }

    #[test]
    fn float_equality() {
        let src = "fn a(x: f64) -> bool { x == 0.1 || x != 3.14 || x == 0.0 || x == 0.5 || 0.3f32 == y }";
        assert_eq!(check_rule("float-equality", src).len(), 3);
    }

    #[test]
    fn ptr_arg() {
        let src = "fn a(v: &Vec<u8>, s: &String, p: &std::path::PathBuf, m: &mut Vec<u8>, ok: &[u8]) {}\n\
                   trait T { fn t(&self, v: &Vec<u8>); }\n";
        assert_eq!(lines(&check_rule("ptr-arg", src)), vec![1, 1, 1]);
    }

    #[test]
    fn deny_warnings() {
        assert_eq!(check_rule("deny-warnings", "#![deny(warnings)]\nfn a() {}").len(), 1);
        assert_eq!(check_rule("deny-warnings", "#![deny(missing_docs, warnings)]\nfn a() {}").len(), 1);
        assert!(check_rule("deny-warnings", "#![deny(missing_docs)]\nfn a() {}").is_empty());
    }
}
