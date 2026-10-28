//! Expressions: `check-then-unwrap`, `assert-without-message`, `combinator-chain`,
//! `string-concat-plus`, `inferred-conversion-type`, `poisoned-mutex-recovery`, `catch-unwind`.

use syn::spanned::Spanned;
use syn::visit::Visit;

use super::text;
use crate::analyzer::Analyzer;
use crate::checks::{call_path, macro_args, macro_name, path_ends_with, peel};

/// Iterator adapters, for counting a combinator chain.
const ADAPTERS: &[&str] = &[
    "map",
    "filter",
    "filter_map",
    "flat_map",
    "flatten",
    "take_while",
    "skip_while",
    "map_while",
    "zip",
    "chain",
    "rev",
    "skip",
    "take",
    "step_by",
    "peekable",
    "scan",
    "inspect",
    "cloned",
    "copied",
    "enumerate",
    "fuse",
    "cycle",
];

/// A chain of five adapters is where the guide's "long and complex" starts here.
const LONG_CHAIN: usize = 5;

const LOCK_METHODS: &[&str] = &["lock", "read", "write", "try_lock", "try_read", "try_write"];

/// How many adapters sit at the outer end of `e`, counting inwards until a non-adapter.
fn chain_len(mut e: &syn::Expr) -> usize {
    let mut n = 0;
    while let syn::Expr::MethodCall(m) = peel(e) {
        if !ADAPTERS.contains(&m.method.to_string().as_str()) {
            break;
        }
        n += 1;
        e = &m.receiver;
    }
    n
}

/// Whether `e` is, or hangs off, a `lock()`-like call.
fn locks(mut e: &syn::Expr) -> bool {
    while let syn::Expr::MethodCall(m) = peel(e) {
        if m.args.is_empty() && LOCK_METHODS.contains(&m.method.to_string().as_str()) {
            return true;
        }
        e = &m.receiver;
    }
    false
}

fn is_into_inner(e: &syn::Expr) -> bool {
    let e = match peel(e) {
        syn::Expr::Block(b) if b.block.stmts.len() == 1 => match &b.block.stmts[0] {
            syn::Stmt::Expr(inner, _) => inner,
            _ => return false,
        },
        other => other,
    };
    matches!(peel(e), syn::Expr::MethodCall(m) if m.method == "into_inner" && m.args.is_empty())
}

fn report_catch_unwind(a: &mut Analyzer, span: proc_macro2::Span) {
    if a.abort_profiles.is_empty() {
        a.emit(
            "catch-unwind",
            span,
            "`catch_unwind` in first-party code; the guide prohibits it except as defence in depth that nothing relies on",
        );
    }
}

pub fn call(a: &mut Analyzer, node: &syn::ExprCall) {
    if let Some(path) = call_path(node)
        && path_ends_with(path, &["catch_unwind"])
    {
        report_catch_unwind(a, path.span());
    }
}

pub fn method_call(a: &mut Analyzer, node: &syn::ExprMethodCall) {
    let method = node.method.to_string();
    match method.as_str() {
        "catch_unwind" if node.args.is_empty() => report_catch_unwind(a, node.method.span()),
        "for_each" => a.emit(
            "combinator-chain",
            node.method.span(),
            "`for_each`; the guide says a `for` loop is usually easier to read",
        ),
        "replace"
            if node.args.len() == 2 && matches!(peel(&node.receiver), syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(_), .. })) =>
        {
            a.emit(
                "string-concat-plus",
                node.method.span(),
                "`replace` on a string literal; `format!` does the substitution without a template of your own",
            )
        }
        "unwrap_or_else" if node.args.len() == 1 && locks(&node.receiver) => {
            let recovers = match peel(&node.args[0]) {
                syn::Expr::Closure(c) => is_into_inner(&c.body),
                syn::Expr::Path(p) => path_ends_with(&p.path, &["PoisonError", "into_inner"]),
                _ => false,
            };
            if recovers {
                poison(a, node.method.span());
            }
        }
        m if ADAPTERS.contains(&m) && 1 + chain_len(&node.receiver) == LONG_CHAIN => a.emit(
            "combinator-chain",
            node.method.span(),
            format!("a chain of {LONG_CHAIN} or more iterator adapters; a loop, or a named helper for part of it, may read better"),
        ),
        _ => {}
    }
}

fn poison(a: &mut Analyzer, span: proc_macro2::Span) {
    a.emit(
        "poisoned-mutex-recovery",
        span,
        "takes the data out of a poisoned lock; the guide says never to, since the thread that poisoned it panicked mid-update",
    );
}

/// `match x.lock() { Err(p) => p.into_inner(), .. }`.
pub fn expr_match(a: &mut Analyzer, node: &syn::ExprMatch) {
    if !locks(&node.expr) {
        return;
    }
    for arm in &node.arms {
        if let syn::Pat::TupleStruct(ts) = &arm.pat
            && path_ends_with(&ts.path, &["Err"])
            && is_into_inner(&arm.body)
        {
            poison(a, arm.pat.span());
        }
    }
}

pub fn macro_call(a: &mut Analyzer, mac: &syn::Macro) {
    let name = macro_name(mac);
    let wanted = match name.as_str() {
        "assert" | "debug_assert" => 1,
        "assert_eq" | "assert_ne" | "debug_assert_eq" | "debug_assert_ne" => 2,
        _ => return,
    };
    if macro_args(mac).is_some_and(|args| args.len() == wanted) {
        a.emit(
            "assert-without-message",
            mac.path.span(),
            format!("`{name}!` with no message; say why it should hold, or what misuse it catches"),
        );
    }
}

pub fn for_loop(a: &mut Analyzer, node: &syn::ExprForLoop) {
    if chain_len(&node.expr) >= 2 {
        a.emit(
            "combinator-chain",
            node.expr.span(),
            "a combinator chain in a `for` header; bind it to a variable, then loop over that",
        );
    }
}

/// `x.is_some()` style checks in a condition: the checked expression's text, and whether the
/// `then` branch (true) or the `else` branch (false) is where it is known to hold a value.
fn checks(cond: &syn::Expr, out: &mut Vec<(String, bool)>) {
    match peel(cond) {
        syn::Expr::MethodCall(m) if m.args.is_empty() => match m.method.to_string().as_str() {
            "is_some" | "is_ok" => out.push((text(&m.receiver), true)),
            "is_none" | "is_err" => out.push((text(&m.receiver), false)),
            _ => {}
        },
        syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Not(_)) => {
            let mut inner = Vec::new();
            checks(&u.expr, &mut inner);
            out.extend(inner.into_iter().map(|(t, b)| (t, !b)));
        }
        syn::Expr::Binary(b) if matches!(b.op, syn::BinOp::And(_) | syn::BinOp::Or(_)) => {
            let and = matches!(b.op, syn::BinOp::And(_));
            let mut inner = Vec::new();
            checks(&b.left, &mut inner);
            checks(&b.right, &mut inner);
            out.extend(inner.into_iter().filter(|(_, then)| *then == and));
        }
        _ => {}
    }
}

struct UnwrapFinder<'a> {
    wanted: &'a str,
    found: Vec<proc_macro2::Span>,
}

impl<'ast> Visit<'ast> for UnwrapFinder<'_> {
    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if m.method == "unwrap" && m.args.is_empty() && text(&m.receiver) == self.wanted {
            self.found.push(m.method.span());
        }
        syn::visit::visit_expr_method_call(self, m);
    }
}

pub fn expr_if(a: &mut Analyzer, node: &syn::ExprIf) {
    let mut checked = Vec::new();
    checks(&node.cond, &mut checked);
    for (expr, in_then) in checked {
        let mut finder = UnwrapFinder {
            wanted: &expr,
            found: Vec::new(),
        };
        if in_then {
            finder.visit_block(&node.then_branch);
        } else if let Some((_, e)) = &node.else_branch {
            finder.visit_expr(e);
        }
        for span in finder.found {
            a.emit(
                "check-then-unwrap",
                span,
                format!("`{expr}` is checked, then unwrapped; `if let` or `match` proves it to the compiler instead"),
            );
        }
    }
}

struct CollectFinder {
    found: Vec<proc_macro2::Span>,
}

impl<'ast> Visit<'ast> for CollectFinder {
    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if m.method == "collect"
            && let Some(t) = &m.turbofish
            && !text(t).contains('_')
        {
            self.found.push(m.method.span());
        }
        syn::visit::visit_expr_method_call(self, m);
    }
}

pub fn local(a: &mut Analyzer, node: &syn::Local) {
    if matches!(node.pat, syn::Pat::Type(_)) {
        return;
    }
    let Some(init) = &node.init else { return };
    // Past `?`, `.unwrap()` and friends to the conversion itself.
    let mut e: &syn::Expr = &init.expr;
    loop {
        e = match peel(e) {
            syn::Expr::Try(t) => &t.expr,
            syn::Expr::MethodCall(m)
                if matches!(
                    m.method.to_string().as_str(),
                    "unwrap"
                        | "expect"
                        | "unwrap_or"
                        | "unwrap_or_else"
                        | "unwrap_or_default"
                        | "ok"
                ) =>
            {
                &m.receiver
            }
            other => {
                if let syn::Expr::MethodCall(m) = other
                    && matches!(m.method.to_string().as_str(), "into" | "parse")
                    && m.turbofish.is_none()
                    && m.args.is_empty()
                {
                    a.emit(
                        "inferred-conversion-type",
                        m.method.span(),
                        format!(
                            "`let {} = ..{}()` leaves the type to inference; write it on the `let`",
                            text(&node.pat),
                            m.method
                        ),
                    );
                }
                break;
            }
        };
    }
    let mut finder = CollectFinder { found: Vec::new() };
    finder.visit_expr(&init.expr);
    for span in finder.found {
        a.emit(
            "inferred-conversion-type",
            span,
            format!("turbofish on `collect`; the guide puts the type on the declaration: `let {}: T = ..`", text(&node.pat)),
        );
    }
}

/// Whether an operand of `+` is a string: a literal, `format!`, `to_string()`, `String::from`.
fn stringy(e: &syn::Expr) -> bool {
    match peel(e) {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(_),
            ..
        }) => true,
        syn::Expr::Macro(m) => macro_name(&m.mac) == "format",
        syn::Expr::MethodCall(m) => {
            matches!(m.method.to_string().as_str(), "to_string" | "to_owned")
        }
        syn::Expr::Call(c) => call_path(c).is_some_and(|p| {
            path_ends_with(p, &["String", "from"]) || path_ends_with(p, &["String", "new"])
        }),
        _ => false,
    }
}

pub fn binary(a: &mut Analyzer, node: &syn::ExprBinary) {
    if !matches!(node.op, syn::BinOp::Add(_)) {
        return;
    }
    // Reported once per chain, at its innermost three operands (`(a + b) + c`).
    let syn::Expr::Binary(left) = peel(&node.left) else {
        return;
    };
    if !matches!(left.op, syn::BinOp::Add(_))
        || matches!(peel(&left.left), syn::Expr::Binary(b) if matches!(b.op, syn::BinOp::Add(_)))
    {
        return;
    }
    if [&left.left, &left.right, &node.right]
        .iter()
        .any(|e| stringy(e))
    {
        a.emit(
            "string-concat-plus",
            node.op.span(),
            "three or more strings joined with `+`; `format!(\"{a}{b}{c}\")` allocates once and reads as one",
        );
    }
}
