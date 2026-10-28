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

/// The comment justifying an `unsafe` block, if there is one: see `safety_comment`, plus the
/// block's own first line (`unsafe {\n // why\n`).
pub fn block_justification<'a>(a: &Analyzer<'a>, node: &syn::ExprUnsafe) -> Option<&'a Line> {
    let line = node.unsafe_token.span().start().line;
    let first_inner = node.block.brace_token.span.open().start().line + 1;
    safety_comment(a, line).or_else(|| {
        a.comments
            .get(first_inner - 1)
            .filter(|l| justifies(Some(l)))
    })
}

/// The comment justifying an `unsafe impl`, if there is one.
pub fn impl_justification<'a>(a: &Analyzer<'a>, node: &syn::ItemImpl) -> Option<&'a Line> {
    let token = node.unsafety.as_ref()?;
    safety_comment(a, token.span().start().line)
}

pub fn unsafe_block(a: &mut Analyzer, node: &syn::ExprUnsafe) {
    if block_justification(a, node).is_some() {
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
    if impl_justification(a, node).is_some() {
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
fn safety_comment<'a>(a: &Analyzer<'a>, line: usize) -> Option<&'a Line> {
    let (comments, lines) = (a.comments, a.lines);
    let info = |n: usize| if n == 0 { None } else { comments.get(n - 1) };
    if let Some(l) = info(line).filter(|l| justifies(Some(l))) {
        return Some(l);
    }
    let mut n = line.saturating_sub(1);
    let mut stepped_over_code = false;
    while n > 0 {
        let l = info(n)?;
        let text = lines.get(n - 1).map(|t| t.trim()).unwrap_or_default();
        if l.only_comment() {
            if !l.doc {
                return Some(l);
            }
        } else if text.starts_with("#[") {
            if justifies(Some(l)) {
                return Some(l);
            }
        } else if !stepped_over_code
            && l.code
            && (text.ends_with('=') || text.ends_with('(') || text.ends_with(','))
        {
            // The first line of a statement that continues onto the `unsafe` line.
            stepped_over_code = true;
            if justifies(Some(l)) {
                return Some(l);
            }
        } else {
            return None;
        }
        n -= 1;
    }
    None
}
