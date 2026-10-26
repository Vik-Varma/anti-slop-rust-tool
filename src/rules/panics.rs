//! Panicking code paths in production code.

use super::{Input, Rule, RuleMeta, Sink};
use crate::ast_util::{macro_exprs, macro_name, path_string, peel};
use crate::context::{FileContext, FileKind};
use crate::finding::{Category, Evidence, Severity};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, ReturnType, Type};

pub struct TodoInProduction;
pub struct UnwrapInResultFn;
pub struct UnwrapUsed;
pub struct DbgMacro;
pub struct PanicInDrop;
pub struct ExitInLibrary;

static TODO: RuleMeta = RuleMeta {
    id: "todo-in-production",
    category: Category::Correctness,
    severity: Severity::Error,
    evidence: Evidence::Strong,
    default_enabled: true,
    summary: "`todo!()` / `unimplemented!()` reachable from production code",
    rationale: "These macros mark unfinished code and panic unconditionally when reached. Clippy's \
`todo` lint: 'The todo! macro indicates the presence of unfinished code, so it should not be \
present in production code.' Unfinished stubs are also a hallmark of machine-generated code that \
was never completed. Allowed in tests.",
    sources: &[
        "Clippy lints `todo`, `unimplemented`.",
        "tests/demonstrations.rs::todo_panics_when_reached (this repository)",
    ],
};

static UNWRAP_RESULT: RuleMeta = RuleMeta {
    id: "unwrap-in-result-fn",
    category: Category::Robustness,
    severity: Severity::Warning,
    evidence: Evidence::Strong,
    default_enabled: true,
    summary: "`.unwrap()` inside a function that already returns `Result`/`Option`",
    rationale: "The function has an error channel, yet this call turns a recoverable error into a \
crash; `?` (or `.ok_or(..)?`) would propagate it instead. This is the shape of Cloudflare's \
18 Nov 2025 outage: an oversized configuration file made a Rust proxy hit \
`called Result::unwrap() on an Err value`, taking down much of their network for hours. Clippy \
`unwrap_in_result` describes it as promoting 'recoverable errors to non-recoverable errors'. \
Exempt: `.lock().unwrap()` (idiomatic poison propagation per the std docs), \
`.join().unwrap()` (propagates a thread's panic), unwraps of calls whose arguments are all \
literals (e.g. `\"127.0.0.1\".parse().unwrap()`), code inside closures, and tests. `.expect(..)` is \
not flagged: the Rust Book recommends it for documenting why failure is impossible.",
    sources: &[
        "Cloudflare, '18 November 2025 outage': https://blog.cloudflare.com/18-november-2025-outage/",
        "Clippy lint `unwrap_in_result`.",
        "The Rust Book, ch. 9.3 'To panic! or Not to panic!'.",
    ],
};

static UNWRAP_USED: RuleMeta = RuleMeta {
    id: "unwrap-used",
    category: Category::Robustness,
    severity: Severity::Info,
    evidence: Evidence::Weak,
    default_enabled: false,
    summary: "`.unwrap()` in production code (opt-in)",
    rationale: "Every `.unwrap()` is a potential crash. However, unwrapping on a checked invariant \
is idiomatic (BurntSushi, 'Using unwrap() in Rust is Okay'), Clippy keeps `unwrap_used` in the \
restriction group, and our mining study found no association with later bug fixes once \
function size is controlled. Opt-in for codebases that ban panics.",
    sources: &[
        "Clippy lint `unwrap_used` (restriction).",
        "A. Gallant. Using unwrap() in Rust is Okay. https://burntsushi.net/unwrap/",
        "validation/mining (this repository)",
    ],
};

static DBG: RuleMeta = RuleMeta {
    id: "dbg-macro",
    category: Category::Maintainability,
    severity: Severity::Warning,
    evidence: Evidence::Moderate,
    default_enabled: true,
    summary: "Leftover `dbg!` in production code",
    rationale: "`dbg!` is a debugging aid that prints to stderr; Clippy's `dbg_macro`: 'It should \
not be present in released software or committed to a version control system.'",
    sources: &["Clippy lint `dbg_macro`."],
};

static DROP: RuleMeta = RuleMeta {
    id: "panic-in-drop",
    category: Category::Correctness,
    severity: Severity::Warning,
    evidence: Evidence::Strong,
    default_enabled: true,
    summary: "`Drop::drop` can panic",
    rationale: "Destructors run during unwinding; a panic in `drop` while another panic is in \
flight aborts the whole process (reproduced in tests/demonstrations.rs). The API Guidelines \
(C-DTOR-FAIL) say destructors must never fail and suggest a separate `close()` method returning \
`Result`. Skipped when the body checks `std::thread::panicking()`.",
    sources: &[
        "Rust API Guidelines, C-DTOR-FAIL: https://rust-lang.github.io/api-guidelines/dependability.html#c-dtor-fail",
        "tests/demonstrations.rs::panic_in_drop_during_unwind_aborts (this repository)",
    ],
};

static EXIT: RuleMeta = RuleMeta {
    id: "exit-in-library",
    category: Category::Robustness,
    severity: Severity::Warning,
    evidence: Evidence::Moderate,
    default_enabled: true,
    summary: "`std::process::exit` called from library code",
    rationale: "`process::exit` ends the process without running destructors (buffered writers \
are not flushed, temp files not removed) and takes the decision away from the application \
embedding the library. Return an error instead. Clippy `exit` (restriction) and the std docs \
note that no destructors run.",
    sources: &[
        "std::process::exit docs: https://doc.rust-lang.org/std/process/fn.exit.html",
        "Clippy lint `exit`.",
    ],
};

fn returns_result_or_option(ret: &ReturnType) -> bool {
    let ReturnType::Type(_, ty) = ret else { return false };
    match &**ty {
        Type::Path(tp) => tp
            .path
            .segments
            .last()
            .is_some_and(|s| s.ident == "Result" || s.ident == "Option"),
        _ => false,
    }
}

/// Is `recv.unwrap()` one of the accepted idioms?
fn is_exempt_unwrap_receiver(recv: &Expr) -> bool {
    match peel(recv) {
        Expr::MethodCall(m) => {
            let name = m.method.to_string();
            if m.args.is_empty() && matches!(name.as_str(), "lock" | "read" | "write" | "join") {
                return true;
            }
            all_literal_args(m.args.iter()) && !m.args.is_empty() && is_literalish(&m.receiver)
                || (m.args.is_empty() && matches!(peel(&m.receiver), Expr::Lit(_)))
        }
        Expr::Call(c) => !c.args.is_empty() && all_literal_args(c.args.iter()),
        _ => false,
    }
}

fn is_literalish(e: &Expr) -> bool {
    matches!(peel(e), Expr::Lit(_) | Expr::Path(_))
}

fn all_literal_args<'a>(mut args: impl Iterator<Item = &'a Expr>) -> bool {
    args.all(|a| matches!(peel(a), Expr::Lit(_)))
}

/// Walks function bodies (not closures or nested items), reporting unwraps.
struct UnwrapFinder<'s, 'c, 'a> {
    sink: &'s mut Sink<'c, 'a>,
    ctx: &'c FileContext<'a>,
    /// When set, only report inside functions returning Result/Option.
    only_result_fns: bool,
    in_result_fn: bool,
    in_fn: bool,
}

impl UnwrapFinder<'_, '_, '_> {
    fn fn_body(&mut self, ret: &ReturnType, body: &syn::Block, is_const: bool) {
        let (prev_r, prev_f) = (self.in_result_fn, self.in_fn);
        self.in_result_fn = returns_result_or_option(ret);
        self.in_fn = !is_const;
        self.visit_block(body);
        self.in_result_fn = prev_r;
        self.in_fn = prev_f;
    }
}

impl<'ast> Visit<'ast> for UnwrapFinder<'_, '_, '_> {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        self.fn_body(&f.sig.output, &f.block, f.sig.constness.is_some());
    }
    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        self.fn_body(&f.sig.output, &f.block, f.sig.constness.is_some());
    }
    fn visit_trait_item_fn(&mut self, f: &'ast syn::TraitItemFn) {
        if let Some(b) = &f.default {
            self.fn_body(&f.sig.output, b, f.sig.constness.is_some());
        }
    }
    fn visit_item_const(&mut self, _: &'ast syn::ItemConst) {}
    fn visit_item_static(&mut self, _: &'ast syn::ItemStatic) {}
    fn visit_expr_closure(&mut self, c: &'ast syn::ExprClosure) {
        if !self.only_result_fns {
            visit::visit_expr_closure(self, c);
        }
    }
    fn visit_expr_async(&mut self, a: &'ast syn::ExprAsync) {
        if !self.only_result_fns {
            visit::visit_expr_async(self, a);
        }
    }
    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if m.method == "unwrap" && m.args.is_empty() && self.in_fn {
            let line = m.method.span().start().line;
            let wanted = if self.only_result_fns { self.in_result_fn } else { true };
            if wanted && self.ctx.is_production_line(line) && !is_exempt_unwrap_receiver(&m.receiver) {
                let msg = if self.only_result_fns {
                    "`.unwrap()` in a function that returns Result/Option; propagate with `?` instead"
                } else {
                    "`.unwrap()` panics on None/Err"
                };
                self.sink.emit_span(m.method.span(), msg);
            }
        }
        visit::visit_expr_method_call(self, m);
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(exprs) = macro_exprs(mac) {
            for e in &exprs {
                self.visit_expr(e);
            }
        }
    }
}

impl Rule for UnwrapInResultFn {
    fn meta(&self) -> &'static RuleMeta {
        &UNWRAP_RESULT
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        let mut v = UnwrapFinder { ctx: sink.ctx(), sink, only_result_fns: true, in_result_fn: false, in_fn: false };
        v.visit_file(&input.ctx.ast);
    }
}

impl Rule for UnwrapUsed {
    fn meta(&self) -> &'static RuleMeta {
        &UNWRAP_USED
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        let mut v = UnwrapFinder { ctx: sink.ctx(), sink, only_result_fns: false, in_result_fn: false, in_fn: false };
        v.visit_file(&input.ctx.ast);
    }
}

/// Reports macro invocations by name in production code.
struct MacroFinder<'s, 'c, 'a> {
    sink: &'s mut Sink<'c, 'a>,
    ctx: &'c FileContext<'a>,
    names: &'static [&'static str],
}

impl<'ast> Visit<'ast> for MacroFinder<'_, '_, '_> {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let name = macro_name(mac);
        let span = mac.path.span();
        if self.names.contains(&name.as_str()) && self.ctx.is_production_line(span.start().line) {
            self.sink.emit_span(span, format!("`{name}!` in production code"));
        }
        if let Some(exprs) = macro_exprs(mac) {
            for e in &exprs {
                self.visit_expr(e);
            }
        }
    }
}

impl Rule for TodoInProduction {
    fn meta(&self) -> &'static RuleMeta {
        &TODO
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        MacroFinder { ctx: sink.ctx(), sink, names: &["todo", "unimplemented"] }.visit_file(&input.ctx.ast);
    }
}

impl Rule for DbgMacro {
    fn meta(&self) -> &'static RuleMeta {
        &DBG
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        MacroFinder { ctx: sink.ctx(), sink, names: &["dbg"] }.visit_file(&input.ctx.ast);
    }
}

/// Finds panicking constructs inside a `drop` body.
#[derive(Default)]
struct PanicSites {
    sites: Vec<(proc_macro2::Span, String)>,
    checks_panicking: bool,
}

impl<'ast> Visit<'ast> for PanicSites {
    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if (m.method == "unwrap" && m.args.is_empty()) || (m.method == "expect" && m.args.len() == 1) {
            self.sites.push((m.method.span(), format!("`.{}()`", m.method)));
        }
        visit::visit_expr_method_call(self, m);
    }
    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        if p.path.segments.last().is_some_and(|s| s.ident == "panicking") {
            self.checks_panicking = true;
        }
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        let name = macro_name(mac);
        if matches!(name.as_str(), "panic" | "assert" | "assert_eq" | "assert_ne" | "unreachable" | "todo" | "unimplemented") {
            self.sites.push((mac.path.span(), format!("`{name}!`")));
        }
        if let Some(exprs) = macro_exprs(mac) {
            for e in &exprs {
                self.visit_expr(e);
            }
        }
    }
}

impl Rule for PanicInDrop {
    fn meta(&self) -> &'static RuleMeta {
        &DROP
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        struct ImplFinder<'s, 'c, 'a> {
            sink: &'s mut Sink<'c, 'a>,
            ctx: &'c FileContext<'a>,
        }
        impl<'ast> Visit<'ast> for ImplFinder<'_, '_, '_> {
            fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
                let is_drop = i
                    .trait_
                    .as_ref()
                    .is_some_and(|(_, p, _)| p.segments.last().is_some_and(|s| s.ident == "Drop"));
                if is_drop {
                    for item in &i.items {
                        let syn::ImplItem::Fn(f) = item else { continue };
                        if f.sig.ident != "drop" || self.ctx.in_test(f.sig.ident.span().start().line) {
                            continue;
                        }
                        let mut sites = PanicSites::default();
                        sites.visit_block(&f.block);
                        if sites.checks_panicking {
                            continue;
                        }
                        for (span, what) in sites.sites {
                            self.sink.emit_span(
                                span,
                                format!("{what} in `Drop::drop`; a panic during unwinding aborts the process"),
                            );
                        }
                    }
                }
                visit::visit_item_impl(self, i);
            }
        }
        ImplFinder { ctx: sink.ctx(), sink }.visit_file(&input.ctx.ast);
    }
}

impl Rule for ExitInLibrary {
    fn meta(&self) -> &'static RuleMeta {
        &EXIT
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        if input.ctx.kind != FileKind::Library {
            return;
        }
        struct Finder<'s, 'c, 'a> {
            sink: &'s mut Sink<'c, 'a>,
            ctx: &'c FileContext<'a>,
            in_main: bool,
        }
        impl<'ast> Visit<'ast> for Finder<'_, '_, '_> {
            fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
                let prev = self.in_main;
                self.in_main = f.sig.ident == "main";
                visit::visit_item_fn(self, f);
                self.in_main = prev;
            }
            fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
                if let Expr::Path(p) = &*c.func {
                    let path = path_string(&p.path);
                    if (path == "process::exit" || path.ends_with("::process::exit"))
                        && !self.in_main
                        && self.ctx.is_production_line(p.span().start().line)
                    {
                        self.sink.emit_span(p.span(), "`process::exit` in library code skips destructors; return an error");
                    }
                }
                visit::visit_expr_call(self, c);
            }
            fn visit_macro(&mut self, mac: &'ast syn::Macro) {
                if let Some(exprs) = macro_exprs(mac) {
                    for e in &exprs {
                        self.visit_expr(e);
                    }
                }
            }
        }
        Finder { ctx: sink.ctx(), sink, in_main: false }.visit_file(&input.ctx.ast);
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::testing::{check_rule, check_rule_at, lines};

    #[test]
    fn todo_flagged_outside_tests() {
        let src = "fn a() { todo!() }\nfn b() { unimplemented!(\"x\") }\n#[cfg(test)]\nmod t { fn c() { todo!() } }\n";
        assert_eq!(lines(&check_rule("todo-in-production", src)), vec![1, 2]);
        assert!(check_rule_at("todo-in-production", "tests/it.rs", src).is_empty());
        assert!(check_rule_at("todo-in-production", "examples/e.rs", src).is_empty());
    }

    #[test]
    fn unwrap_in_result_fn() {
        let src = r#"
fn a(s: &str) -> Result<u32, std::num::ParseIntError> {
    let n = s.parse::<u32>().unwrap();
    Ok(n)
}
fn b(s: &str) -> u32 { s.parse().unwrap() }
fn c(m: &std::sync::Mutex<u32>) -> Option<u32> {
    let g = m.lock().unwrap();
    let ip: std::net::IpAddr = "127.0.0.1".parse().unwrap();
    let v: Vec<u32> = vec![1].into_iter().map(|x| Some(x).unwrap()).collect();
    Some(*g)
}
fn d(x: Option<u32>) -> Option<u32> { let y = x.expect("checked"); Some(y) }
fn e(x: Option<u32>) -> io::Result<u32> { Ok(x.unwrap()) }
"#;
        assert_eq!(lines(&check_rule("unwrap-in-result-fn", src)), vec![3, 14]);
    }

    #[test]
    fn unwrap_used_is_opt_in_and_skips_tests() {
        let src = "fn a(x: Option<u8>) -> u8 { x.unwrap() }\n#[test]\nfn t() { None::<u8>.unwrap(); }\n";
        assert_eq!(lines(&check_rule("unwrap-used", src)), vec![1]);
    }

    #[test]
    fn dbg_macro() {
        assert_eq!(check_rule("dbg-macro", "fn a(x: u8) -> u8 { dbg!(x) }").len(), 1);
        assert_eq!(check_rule("dbg-macro", "fn a(x: u8) { println!(\"{}\", dbg!(x)); }").len(), 1);
    }

    #[test]
    fn panic_in_drop() {
        let src = r#"
struct A; struct B;
impl Drop for A { fn drop(&mut self) { std::fs::remove_file("x").unwrap(); } }
impl Drop for B { fn drop(&mut self) { if !std::thread::panicking() { assert!(true); } } }
"#;
        assert_eq!(lines(&check_rule("panic-in-drop", src)), vec![3]);
    }

    #[test]
    fn exit_in_library() {
        let src = "fn fail() { std::process::exit(1) }\nfn main() { std::process::exit(0) }\n";
        assert_eq!(lines(&check_rule("exit-in-library", src)), vec![1]);
        assert!(check_rule_at("exit-in-library", "src/main.rs", src).is_empty());
    }
}
