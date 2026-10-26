use syn::spanned::Spanned;

use super::{call_path, path_ends_with, peel};
use crate::analyzer::Analyzer;
use crate::comments::Line;

pub fn static_mut(a: &mut Analyzer, node: &syn::ItemStatic) {
    if matches!(node.mutability, syn::StaticMutability::Mut(_)) {
        a.emit(
            "static-mut",
            node.ident.span(),
            format!("`static mut {}`: any reference to it can alias a write; use an atomic, `Mutex` or `OnceLock`", node.ident),
        );
    }
}

pub fn call(a: &mut Analyzer, node: &syn::ExprCall) {
    let Some(path) = call_path(node) else { return };
    if path_ends_with(path, &["mem", "uninitialized"]) {
        a.emit(
            "uninit-memory",
            path.span(),
            "`mem::uninitialized()` is undefined behaviour for almost every type",
        );
    }
}

pub fn method_call(a: &mut Analyzer, node: &syn::ExprMethodCall) {
    if node.method != "assume_init" {
        return;
    }
    let syn::Expr::Call(inner) = peel(&node.receiver) else {
        return;
    };
    let Some(path) = call_path(inner) else { return };
    if !path_ends_with(path, &["MaybeUninit", "uninit"])
        || holds_maybe_uninit(path)
        || a.scope().uninit_array_target
    {
        return;
    }
    a.emit(
        "uninit-memory",
        node.method.span(),
        "`MaybeUninit::uninit().assume_init()` reads uninitialised memory: undefined behaviour",
    );
}

/// `MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init()` is the one sound use: the outer
/// value is an array of things that are allowed to be uninitialised.
/// `[MaybeUninit<T>; N]`, written out as a type.
pub fn is_maybe_uninit_array(ty: &syn::Type) -> bool {
    let s = quote::quote!(#ty).to_string();
    s.trim_start().starts_with('[') && s.contains("MaybeUninit")
}

fn holds_maybe_uninit(path: &syn::Path) -> bool {
    path.segments.iter().any(|seg| {
        let syn::PathArguments::AngleBracketed(args) = &seg.arguments else {
            return false;
        };
        args.args.iter().any(|arg| {
            let syn::GenericArgument::Type(ty) = arg else {
                return false;
            };
            let s = quote::quote!(#ty).to_string();
            s.trim_start().starts_with('[') && s.contains("MaybeUninit")
        })
    })
}

pub fn unsafe_block(a: &mut Analyzer, node: &syn::ExprUnsafe) {
    let line = node.unsafe_token.span().start().line;
    let first_inner = node.block.brace_token.span.open().start().line + 1;
    if has_safety_comment(a, line) || justifies(a.comments.get(first_inner - 1)) {
        return;
    }
    a.emit(
        "unsafe-without-comment",
        node.unsafe_token.span(),
        "`unsafe` block with no comment saying why its preconditions hold",
    );
}

pub fn unsafe_impl(a: &mut Analyzer, node: &syn::ItemImpl) {
    let Some(token) = &node.unsafety else { return };
    if has_safety_comment(a, token.span().start().line) {
        return;
    }
    a.emit(
        "unsafe-without-comment",
        token.span(),
        "`unsafe impl` with no comment saying why the trait's contract holds",
    );
}

/// A comment that could justify `unsafe`: any ordinary comment. Doc comments describe the item
/// to its users, not why a block inside it is sound, so they don't count.
fn justifies(line: Option<&Line>) -> bool {
    line.is_some_and(|l| l.has_comment() && !l.doc)
}

/// A comment justifying the `unsafe`: on its own line, in the run of comment and attribute lines
/// just above it, or (for `let x =` then `unsafe {` on the next line) above the statement's first
/// line. Any wording counts. The std library writes `// SAFETY:`, but the audit in
/// docs/EVIDENCE.md found real justifications in other words, and the rule is about there being
/// an argument at all.
fn has_safety_comment(a: &Analyzer, line: usize) -> bool {
    let info = |n: usize| if n == 0 { None } else { a.comments.get(n - 1) };
    if justifies(info(line)) {
        return true;
    }
    let mut n = line.saturating_sub(1);
    let mut stepped_over_code = false;
    while n > 0 {
        let Some(l) = info(n) else { return false };
        let text = a.lines.get(n - 1).map(|t| t.trim()).unwrap_or_default();
        if l.only_comment() {
            if !l.doc {
                return true;
            }
        } else if text.starts_with("#[") {
            if justifies(Some(l)) {
                return true;
            }
        } else if !stepped_over_code
            && l.code
            && (text.ends_with('=') || text.ends_with('(') || text.ends_with(','))
        {
            // The first line of a statement that continues onto the `unsafe` line.
            stepped_over_code = true;
            if justifies(Some(l)) {
                return true;
            }
        } else {
            return false;
        }
        n -= 1;
    }
    false
}
