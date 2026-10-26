//! Error handling that hides failures.

use super::{Input, Rule, RuleMeta, Sink};
use crate::ast_util::{macro_exprs, peel};
use crate::context::{FileContext, FileKind};
use crate::finding::{Category, Evidence, Severity};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, Pat, Stmt};

pub struct SwallowedError;
pub struct UnusedIoAmount;
pub struct UnitErrorType;

static SWALLOWED: RuleMeta = RuleMeta {
    id: "swallowed-error",
    category: Category::Robustness,
    severity: Severity::Warning,
    evidence: Evidence::Strong,
    default_enabled: true,
    summary: "Error is silently discarded (`Err(_) => {}`, `result.ok();`)",
    rationale: "Yuan et al. (OSDI 2014) studied 198 user-reported failures in Cassandra, HBase, \
HDFS, MapReduce and Redis: 92% of catastrophic failures resulted from incorrect handling of \
errors that the software had explicitly signalled, and in 25% of those cases the error was \
simply ignored (empty or log-only handler). An empty `Err` arm or a bare `.ok();` statement \
throws the error away with no trace. Clippy `unused_result_ok`: 'Using Result::ok() may look like \
the result is checked ... but it only silences the warning caused by #[must_use]'. Handle the \
error, propagate it with `?`, or at least log it.",
    sources: &[
        "Yuan et al. Simple Testing Can Prevent Most Critical Failures. OSDI 2014. https://www.usenix.org/conference/osdi14/technical-sessions/presentation/yuan",
        "Gunawi et al. EIO: Error Handling is Occasionally Correct. FAST 2008.",
        "Clippy lint `unused_result_ok`.",
    ],
};

static IO_AMOUNT: RuleMeta = RuleMeta {
    id: "unused-io-amount",
    category: Category::Correctness,
    severity: Severity::Error,
    evidence: Evidence::Strong,
    default_enabled: true,
    summary: "Byte count of `read`/`write` ignored (partial I/O silently lost)",
    rationale: "`Read::read` and `Write::write` may process fewer bytes than the buffer holds and \
report how many they did. Discarding the count silently drops data on short reads/writes \
(reproduced in tests/demonstrations.rs). Use `read_exact` / `write_all`. Clippy \
`unused_io_amount` is in the correctness group (deny by default).",
    sources: &[
        "std::io::Write::write docs: https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.write",
        "Clippy lint `unused_io_amount` (correctness).",
        "tests/demonstrations.rs::ignored_write_count_loses_data (this repository)",
    ],
};

static UNIT_ERR: RuleMeta = RuleMeta {
    id: "unit-error-type",
    category: Category::Robustness,
    severity: Severity::Warning,
    evidence: Evidence::Moderate,
    default_enabled: true,
    summary: "Public function returns `Result<_, ()>`",
    rationale: "A `()` error carries no information about what went wrong, implements neither \
`Error` nor `Display`, and cannot be converted with `?` into callers' error types. The Rust API \
Guidelines (C-GOOD-ERR): 'Never use () as an error type, even where there is no useful \
additional information for the error to carry.'",
    sources: &["Rust API Guidelines, C-GOOD-ERR: https://rust-lang.github.io/api-guidelines/interoperability.html#c-good-err"],
};

fn is_empty_body(e: &Expr) -> bool {
    match peel(e) {
        Expr::Block(b) => b.block.stmts.is_empty() && b.label.is_none(),
        Expr::Tuple(t) => t.elems.is_empty(),
        _ => false,
    }
}

fn is_err_pattern(p: &Pat) -> bool {
    match p {
        Pat::TupleStruct(ts) => ts.path.segments.last().is_some_and(|s| s.ident == "Err"),
        _ => false,
    }
}

struct SwallowFinder<'s, 'c, 'a> {
    sink: &'s mut Sink<'c, 'a>,
    ctx: &'c FileContext<'a>,
}

impl SwallowFinder<'_, '_, '_> {
    fn report(&mut self, span: proc_macro2::Span, msg: &str) {
        if self.ctx.is_production_line(span.start().line) {
            self.sink.emit_span(span, msg);
        }
    }
}

impl<'ast> Visit<'ast> for SwallowFinder<'_, '_, '_> {
    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        if is_err_pattern(&arm.pat) && arm.guard.is_none() && is_empty_body(&arm.body) {
            self.report(arm.pat.span(), "`Err` arm discards the error without handling or logging it");
        }
        visit::visit_arm(self, arm);
    }

    fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
        if let Expr::Let(l) = &*i.cond {
            if is_err_pattern(&l.pat) && i.then_branch.stmts.is_empty() && i.else_branch.is_none() {
                self.report(l.pat.span(), "`if let Err(..)` with an empty body discards the error");
            }
        }
        visit::visit_expr_if(self, i);
    }

    fn visit_stmt(&mut self, s: &'ast Stmt) {
        if let Stmt::Expr(e, Some(_)) = s {
            if let Expr::MethodCall(m) = peel(e) {
                if m.method == "ok" && m.args.is_empty() {
                    self.report(m.method.span(), "`.ok();` silences the Result without checking it");
                }
            }
        }
        visit::visit_stmt(self, s);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(exprs) = macro_exprs(mac) {
            for e in &exprs {
                self.visit_expr(e);
            }
        }
    }
}

impl Rule for SwallowedError {
    fn meta(&self) -> &'static RuleMeta {
        &SWALLOWED
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        SwallowFinder { ctx: sink.ctx(), sink }.visit_file(&input.ctx.ast);
    }
}

/// Strips `?`, `.await`, `.unwrap()`, `.expect(..)` from an expression.
fn strip_propagation(mut e: &Expr) -> &Expr {
    loop {
        e = match peel(e) {
            Expr::Try(t) => &t.expr,
            Expr::Await(a) => &a.base,
            Expr::MethodCall(m) if (m.method == "unwrap" && m.args.is_empty()) || (m.method == "expect" && m.args.len() == 1) => {
                &m.receiver
            }
            other => return other,
        };
    }
}

fn io_call(e: &Expr) -> Option<&syn::ExprMethodCall> {
    match strip_propagation(e) {
        Expr::MethodCall(m)
            if m.args.len() == 1
                && matches!(m.method.to_string().as_str(), "read" | "write" | "read_vectored" | "write_vectored") =>
        {
            Some(m)
        }
        _ => None,
    }
}

impl Rule for UnusedIoAmount {
    fn meta(&self) -> &'static RuleMeta {
        &IO_AMOUNT
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        struct Finder<'s, 'c, 'a> {
            sink: &'s mut Sink<'c, 'a>,
        }
        impl Finder<'_, '_, '_> {
            fn flag(&mut self, m: &syn::ExprMethodCall) {
                let fix = if m.method.to_string().starts_with("read") { "read_exact" } else { "write_all" };
                self.sink.emit_span(
                    m.method.span(),
                    format!("result of `.{}()` (bytes processed) is ignored; use `{fix}`", m.method),
                );
            }
        }
        impl<'ast> Visit<'ast> for Finder<'_, '_, '_> {
            fn visit_stmt(&mut self, s: &'ast Stmt) {
                match s {
                    // Statements whose value is discarded: `x.write(..)?;`
                    Stmt::Expr(e, Some(_)) => {
                        if let Some(m) = io_call(e) {
                            self.flag(m);
                        }
                    }
                    Stmt::Local(l) if matches!(l.pat, Pat::Wild(_)) => {
                        if let Some(init) = &l.init {
                            if let Some(m) = io_call(&init.expr) {
                                self.flag(m);
                            }
                        }
                    }
                    _ => {}
                }
                visit::visit_stmt(self, s);
            }
        }
        Finder { sink }.visit_file(&input.ctx.ast);
    }
}

impl Rule for UnitErrorType {
    fn meta(&self) -> &'static RuleMeta {
        &UNIT_ERR
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        if input.ctx.kind != FileKind::Library {
            return;
        }
        struct Finder<'s, 'c, 'a> {
            sink: &'s mut Sink<'c, 'a>,
            ctx: &'c FileContext<'a>,
        }
        impl Finder<'_, '_, '_> {
            fn check_sig(&mut self, vis: &syn::Visibility, sig: &syn::Signature) {
                if !matches!(vis, syn::Visibility::Public(_)) {
                    return;
                }
                let syn::ReturnType::Type(_, ty) = &sig.output else { return };
                let syn::Type::Path(tp) = &**ty else { return };
                let Some(last) = tp.path.segments.last() else { return };
                if last.ident != "Result" {
                    return;
                }
                let syn::PathArguments::AngleBracketed(args) = &last.arguments else { return };
                let types: Vec<&syn::Type> = args
                    .args
                    .iter()
                    .filter_map(|a| if let syn::GenericArgument::Type(t) = a { Some(t) } else { None })
                    .collect();
                if let [_, syn::Type::Tuple(t)] = types.as_slice() {
                    if t.elems.is_empty() && self.ctx.is_production_line(sig.ident.span().start().line) {
                        self.sink.emit_span(
                            sig.ident.span(),
                            format!("`{}` returns `Result<_, ()>`; use an error type that says what failed", sig.ident),
                        );
                    }
                }
            }
        }
        impl<'ast> Visit<'ast> for Finder<'_, '_, '_> {
            fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
                self.check_sig(&f.vis, &f.sig);
                visit::visit_item_fn(self, f);
            }
            fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
                self.check_sig(&f.vis, &f.sig);
                visit::visit_impl_item_fn(self, f);
            }
        }
        Finder { ctx: sink.ctx(), sink }.visit_file(&input.ctx.ast);
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::testing::{check_rule, check_rule_at, lines};

    #[test]
    fn swallowed_errors() {
        let src = r#"
fn a(r: Result<u8, String>) {
    match r {
        Ok(v) => println!("{v}"),
        Err(_) => {}
    }
    match r { Ok(_) => {}, Err(e) => () }
    match r { Ok(_) => {}, Err(e) => eprintln!("{e}") }
    std::fs::remove_file("x").ok();
    let _kept = std::fs::read("x").ok();
    if let Err(_) = r {}
}
"#;
        assert_eq!(lines(&check_rule("swallowed-error", src)), vec![5, 7, 9, 11]);
    }

    #[test]
    fn swallowed_errors_ignored_in_tests() {
        let src = "fn a(r: Result<u8, ()>) { match r { Ok(_) => {}, Err(_) => {} } }";
        assert!(check_rule_at("swallowed-error", "tests/t.rs", src).is_empty());
    }

    #[test]
    fn unused_io_amount() {
        let src = r#"
fn a(f: &mut std::fs::File, buf: &mut [u8]) -> std::io::Result<()> {
    f.write(b"hello")?;
    f.read(buf).unwrap();
    let _ = f.write(b"x");
    let n = f.read(buf)?;
    f.write_all(b"ok")?;
    lock.write().unwrap();
    Ok(())
}
"#;
        assert_eq!(lines(&check_rule("unused-io-amount", src)), vec![3, 4, 5]);
    }

    #[test]
    fn unit_error_type() {
        let src = "pub fn a() -> Result<u8, ()> { Ok(1) }\nfn b() -> Result<u8, ()> { Ok(1) }\npub fn c() -> Result<u8, E> { Ok(1) }\n";
        assert_eq!(lines(&check_rule("unit-error-type", src)), vec![1]);
    }
}
