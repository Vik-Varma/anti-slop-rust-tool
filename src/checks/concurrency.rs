use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use super::{call_path, path_ends_with, peel};
use crate::analyzer::Analyzer;

pub fn call(a: &mut Analyzer, node: &syn::ExprCall) {
    // Tests sleep on purpose (to let a timer expire, to simulate a stalled peer).
    if !a.scope().in_async || a.scope().test {
        return;
    }
    let Some(path) = call_path(node) else { return };
    if path_ends_with(path, &["thread", "sleep"]) {
        a.emit(
            "blocking-sleep-in-async",
            path.span(),
            "`thread::sleep` in async code blocks the executor thread and every task on it; use the runtime's async sleep",
        );
    }
}

/// Looks for a lock guard bound with `let` and still alive at a later `.await` in the same block.
pub fn async_body(a: &mut Analyzer, block: &syn::Block) {
    let mut hits = Vec::new();
    scan_block(block, &mut hits);
    for (guard, name, at) in hits {
        a.emit(
            "lock-held-across-await",
            at,
            format!("guard `{name}` from `.{guard}()` is still held at this `.await`; drop it first or use an async lock"),
        );
    }
}

fn scan_block(block: &syn::Block, hits: &mut Vec<(String, String, Span)>) {
    for (i, stmt) in block.stmts.iter().enumerate() {
        if let syn::Stmt::Local(local) = stmt
            && let Some(init) = &local.init
            && let Some(guard) = lock_guard(&init.expr)
            && let Some(name) = bound_ident(&local.pat)
        {
            for later in &block.stmts[i + 1..] {
                // Shadowing (`let g = *g;`) hides the name but doesn't drop the guard, so only an
                // explicit `drop(g)` ends it.
                if drops(later, &name) {
                    break;
                }
                if let Some(at) = first_await(later) {
                    hits.push((guard.clone(), name.clone(), at));
                    break;
                }
            }
        }
        let mut inner = InnerBlocks { hits };
        inner.visit_stmt(stmt);
    }
}

/// Descends into nested blocks (`if`, `match`, loops) but not into closures, async blocks or
/// items: those run on their own schedule and are checked when the visitor reaches them.
struct InnerBlocks<'h> {
    hits: &'h mut Vec<(String, String, Span)>,
}

impl<'ast> Visit<'ast> for InnerBlocks<'_> {
    fn visit_block(&mut self, b: &'ast syn::Block) {
        scan_block(b, self.hits);
    }
    fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}
    fn visit_expr_async(&mut self, _: &'ast syn::ExprAsync) {}
    fn visit_item(&mut self, _: &'ast syn::Item) {}
}

/// `m.lock()`, `m.lock().unwrap()`, `m.write().expect(..)`, `m.read()?`: the value bound is the
/// guard itself. `m.lock().await` is an async mutex, which is made to be held across `.await`.
fn lock_guard(e: &syn::Expr) -> Option<String> {
    let mut e = peel(e);
    loop {
        match e {
            syn::Expr::Try(t) => e = peel(&t.expr),
            syn::Expr::MethodCall(m)
                if matches!(
                    m.method.to_string().as_str(),
                    "unwrap" | "expect" | "unwrap_or_else"
                ) =>
            {
                e = peel(&m.receiver)
            }
            syn::Expr::MethodCall(m)
                if m.args.is_empty()
                    && matches!(m.method.to_string().as_str(), "lock" | "read" | "write") =>
            {
                return Some(m.method.to_string());
            }
            _ => return None,
        }
    }
}

fn bound_ident(pat: &syn::Pat) -> Option<String> {
    match pat {
        syn::Pat::Ident(p) => Some(p.ident.to_string()),
        syn::Pat::Type(t) => bound_ident(&t.pat),
        _ => None,
    }
}

fn drops(stmt: &syn::Stmt, name: &str) -> bool {
    let syn::Stmt::Expr(syn::Expr::Call(c), _) = stmt else {
        return false;
    };
    call_path(c).is_some_and(|p| path_ends_with(p, &["drop"]))
        && c.args
            .iter()
            .any(|arg| matches!(peel(arg), syn::Expr::Path(p) if p.path.is_ident(name)))
}

fn first_await(stmt: &syn::Stmt) -> Option<Span> {
    struct Find(Option<Span>);
    impl<'ast> Visit<'ast> for Find {
        fn visit_expr_await(&mut self, node: &'ast syn::ExprAwait) {
            if self.0.is_none() {
                self.0 = Some(node.await_token.span());
            }
            visit::visit_expr_await(self, node);
        }
        fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}
        fn visit_expr_async(&mut self, _: &'ast syn::ExprAsync) {}
        fn visit_item(&mut self, _: &'ast syn::Item) {}
    }
    let mut f = Find(None);
    f.visit_stmt(stmt);
    f.0
}
