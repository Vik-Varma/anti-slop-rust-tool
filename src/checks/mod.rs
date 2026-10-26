//! The checks. Each rule's evidence and sources are in `crate::rules` and docs/EVIDENCE.md; this
//! module only matches syntax. Without type information every check here is a heuristic, and each
//! one says in its rule entry what it can mistake.

mod concurrency;
mod errors;
mod hygiene;
mod numeric;
mod panics;
mod unsafety;

use syn::punctuated::Punctuated;

use crate::analyzer::Analyzer;

pub fn file_level(a: &mut Analyzer, file: &syn::File) {
    hygiene::file_attributes(a, &file.attrs);
}

pub fn function(a: &mut Analyzer, sig: &syn::Signature, block: &syn::Block) {
    if sig.asyncness.is_some() {
        concurrency::async_body(a, block);
    }
}

pub fn async_block(a: &mut Analyzer, block: &syn::Block) {
    concurrency::async_body(a, block);
}

pub fn item_impl(a: &mut Analyzer, node: &syn::ItemImpl) {
    unsafety::unsafe_impl(a, node);
}

pub fn item_static(a: &mut Analyzer, node: &syn::ItemStatic) {
    unsafety::static_mut(a, node);
}

pub fn method_call(a: &mut Analyzer, node: &syn::ExprMethodCall) {
    panics::method_call(a, node);
    unsafety::method_call(a, node);
}

pub fn call(a: &mut Analyzer, node: &syn::ExprCall) {
    unsafety::call(a, node);
    concurrency::call(a, node);
    errors::call(a, node);
}

pub fn binary(a: &mut Analyzer, node: &syn::ExprBinary) {
    numeric::binary(a, node);
}

pub fn cast(a: &mut Analyzer, node: &syn::ExprCast) {
    numeric::cast(a, node);
}

pub fn unsafe_block(a: &mut Analyzer, node: &syn::ExprUnsafe) {
    unsafety::unsafe_block(a, node);
}

pub fn stmt(a: &mut Analyzer, node: &syn::Stmt) {
    errors::stmt(a, node);
}

pub fn macro_call(a: &mut Analyzer, node: &syn::Macro) {
    panics::macro_call(a, node);
}

/// Macros whose input is a comma-separated list of expressions. Code inside them is real code,
/// so the visitor descends into it. Anything else (DSLs, `quote!`) stays opaque.
const EXPR_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "println",
    "print",
    "eprintln",
    "eprint",
    "format",
    "format_args",
    "write",
    "writeln",
    "panic",
    "vec",
    "dbg",
];

pub fn macro_args(mac: &syn::Macro) -> Option<Punctuated<syn::Expr, syn::Token![,]>> {
    let name = macro_name(mac);
    if !EXPR_MACROS.contains(&name.as_str()) {
        return None;
    }
    mac.parse_body_with(Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated)
        .ok()
        .or_else(|| {
            // `vec![elem; n]`
            mac.parse_body_with(|input: syn::parse::ParseStream| {
                let elem: syn::Expr = input.parse()?;
                input.parse::<syn::Token![;]>()?;
                let n: syn::Expr = input.parse()?;
                let mut p = Punctuated::new();
                p.push(elem);
                p.push(n);
                Ok(p)
            })
            .ok()
        })
}

pub fn macro_name(mac: &syn::Macro) -> String {
    mac.path
        .segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default()
}

/// Whether `path` ends with `suffix`, e.g. `std::thread::sleep` and `thread::sleep` both end with
/// `["thread", "sleep"]`. Generic arguments are ignored.
pub fn path_ends_with(path: &syn::Path, suffix: &[&str]) -> bool {
    let segs: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    segs.len() >= suffix.len()
        && segs[segs.len() - suffix.len()..]
            .iter()
            .zip(suffix)
            .all(|(a, b)| a == b)
}

pub fn call_path(call: &syn::ExprCall) -> Option<&syn::Path> {
    match call.func.as_ref() {
        syn::Expr::Path(p) => Some(&p.path),
        _ => None,
    }
}

/// Peels parentheses and references so `(&x.len())` reads as `x.len()`.
pub fn peel(mut e: &syn::Expr) -> &syn::Expr {
    loop {
        e = match e {
            syn::Expr::Paren(p) => &p.expr,
            syn::Expr::Group(g) => &g.expr,
            syn::Expr::Reference(r) => &r.expr,
            _ => return e,
        }
    }
}

pub fn is_method<'a>(e: &'a syn::Expr, name: &str, args: usize) -> Option<&'a syn::ExprMethodCall> {
    match peel(e) {
        syn::Expr::MethodCall(m) if m.method == name && m.args.len() == args => Some(m),
        _ => None,
    }
}
