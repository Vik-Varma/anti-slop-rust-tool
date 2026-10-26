use super::{is_method, macro_name, peel};
use crate::analyzer::Analyzer;

/// Receivers whose `unwrap()` re-raises another thread's panic (a poisoned lock). The std docs use
/// `lock().unwrap()` throughout, and propagating the poison is the point, so it isn't flagged.
const POISON_RECEIVERS: &[&str] = &["lock", "read", "write"];

const PANICKING_MACROS: &[&str] = &[
    "panic",
    "unreachable",
    "todo",
    "unimplemented",
    "assert",
    "assert_eq",
    "assert_ne",
];

pub fn method_call(a: &mut Analyzer, node: &syn::ExprMethodCall) {
    let scope = a.scope();
    let (test, in_drop, in_const) = (scope.test, scope.in_drop, scope.in_const);
    let method = node.method.to_string();
    if in_drop && (method == "unwrap" || method == "expect") && !test {
        a.emit(
            "panic-in-drop",
            node.method.span(),
            format!("`.{method}()` inside `Drop::drop`"),
        );
        return;
    }
    if method != "unwrap" || !node.args.is_empty() || test || in_const {
        return;
    }
    let poisoned = POISON_RECEIVERS
        .iter()
        .any(|r| is_method(&node.receiver, r, 0).is_some());
    if poisoned {
        return;
    }
    // `"lit".parse().unwrap()` and friends on a literal fail at the first run, not in production.
    if let syn::Expr::MethodCall(inner) = peel(&node.receiver)
        && matches!(peel(&inner.receiver), syn::Expr::Lit(_))
    {
        return;
    }
    a.emit(
        "unwrap",
        node.method.span(),
        "`.unwrap()` panics with no context; propagate with `?`, or state the invariant with `.expect(\"why this can't fail\")`",
    );
}

pub fn macro_call(a: &mut Analyzer, mac: &syn::Macro) {
    let name = macro_name(mac);
    let scope = a.scope();
    if scope.test {
        return;
    }
    let span = mac
        .path
        .segments
        .last()
        .map(|s| s.ident.span())
        .unwrap_or_else(proc_macro2::Span::call_site);
    if scope.in_drop && PANICKING_MACROS.contains(&name.as_str()) {
        a.emit(
            "panic-in-drop",
            span,
            format!("`{name}!` inside `Drop::drop`"),
        );
        return;
    }
    match name.as_str() {
        "todo" | "unimplemented" if !scope.in_const => a.emit(
            "todo",
            span,
            format!("`{name}!()` panics when this code runs; finish it or return an error"),
        ),
        "dbg" => a.emit(
            "dbg-macro",
            span,
            "`dbg!` left in shipping code prints to stderr in release builds",
        ),
        _ => {}
    }
}
