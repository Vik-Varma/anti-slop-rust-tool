//! Lock misuse and blocking in async code.

use super::{Input, Rule, RuleMeta, Sink};
use crate::ast_util::{Imports, expr_key, macro_exprs, peel};
use crate::context::FileContext;
use crate::finding::{Category, Evidence, Severity};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Block, Expr, Pat, Stmt};

pub struct LockAcrossAwait;
pub struct LockInScrutinee;
pub struct BlockingInAsync;

static ACROSS_AWAIT: RuleMeta = RuleMeta {
    id: "lock-across-await",
    category: Category::Correctness,
    severity: Severity::Error,
    evidence: Evidence::Strong,
    default_enabled: true,
    summary: "Synchronous lock guard (or `RefCell` borrow) held across `.await`",
    rationale: "While the task is suspended at `.await`, the std/parking_lot guard stays locked; \
another task scheduled on the same thread that tries to take the lock blocks the thread and the \
program deadlocks (reproduced in tests/demonstrations.rs). The Tokio docs: holding a std guard \
across `.await` 'virtually never leads to correct concurrent code in practice as it can easily \
lead to deadlocks'. Clippy `await_holding_lock` (suspicious). Fix: drop the guard in an inner \
scope before awaiting, or use an async mutex if the lock really must span the await. Note that \
using std `Mutex` in async code is otherwise fine (and preferred by Tokio). Verified: deadlock reproduced in tests/demonstrations.rs (a control that drops the guard first completes).",
    sources: &[
        "Tokio Mutex docs: https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html",
        "Tokio tutorial 'Shared state': https://tokio.rs/tokio/tutorial/shared-state",
        "Clippy lints `await_holding_lock`, `await_holding_refcell_ref`.",
        "tests/demonstrations.rs::std_guard_across_await_deadlocks (this repository)",
    ],
};

static SCRUTINEE: RuleMeta = RuleMeta {
    id: "lock-in-scrutinee",
    category: Category::Correctness,
    severity: Severity::Error,
    evidence: Evidence::Strong,
    default_enabled: true,
    summary: "Lock taken in a `match`/`if let` scrutinee and taken again in its body (self-deadlock)",
    rationale: "Temporaries in a `match`, `if let` or `while let` scrutinee live until the end of \
the whole construct, so `match m.lock().unwrap().get(k) { .. => m.lock() .. }` tries to lock a \
mutex the thread already holds and deadlocks (reproduced in tests/demonstrations.rs). Qin et al. \
(PLDI 2020) studied 59 blocking bugs in real Rust programs (Servo, Ethereum clients, TiKV, \
Redox, ...): 38 involved Mutex/RwLock and 30 of those were double locks, typically caused by \
misjudging how long a guard lives (6 with the first lock in a `match` scrutinee); 21 were fixed \
by shortening the guard's lifetime, and Clippy caught none. Bind the value with `let` first so \
the guard is dropped before the body. (A plain `if` condition drops its temporaries before the \
body and is not flagged.) Verified: deadlock reproduced in tests/demonstrations.rs.",
    sources: &[
        "Qin, Chen, Yu, Song, Zhang. Understanding Memory and Thread Safety Practices and Issues in Real-World Rust Programs. PLDI 2020. doi:10.1145/3385412.3386036",
        "Clippy lint `significant_drop_in_scrutinee` (nursery).",
        "tests/demonstrations.rs::lock_in_match_scrutinee_deadlocks (this repository)",
    ],
};

static BLOCKING: RuleMeta = RuleMeta {
    id: "blocking-in-async",
    category: Category::Performance,
    severity: Severity::Warning,
    evidence: Evidence::Strong,
    default_enabled: true,
    summary: "Blocking call (sleep, std::fs, std::net, block_on, blocking_*) inside async code",
    rationale: "Async executors run many tasks on a few threads; a blocking call stalls every task \
on that thread (reproduced in tests/demonstrations.rs: one `std::thread::sleep` delays an \
unrelated task by the full sleep). Tokio's docs: 'code running in asynchronous tasks should not \
perform operations that can block'. `blocking_lock`/`blocking_recv`/`block_on` panic outright \
when called from within a runtime. Use the async equivalent (`tokio::time::sleep`, `tokio::fs`) \
or `spawn_blocking`. Verified: in tests/demonstrations.rs a 10 ms timer fires after ~300 ms when another task calls std::thread::sleep(300 ms), versus ~11 ms with tokio::time::sleep.",
    sources: &[
        "Tokio task docs, 'Blocking and Yielding': https://docs.rs/tokio/latest/tokio/task/index.html",
        "A. Ryhl. Async: What is blocking? https://ryhl.io/blog/async-what-is-blocking/",
        "tests/demonstrations.rs::blocking_sleep_stalls_other_tasks (this repository)",
    ],
};

const LOCK_METHODS: &[&str] = &["lock", "read", "write", "borrow_mut"];

/// If `e` evaluates to a lock guard (`m.lock()`, `m.lock().unwrap()`, ...),
/// returns the lock call. Awaited locks (async mutexes) return `None`.
fn guard_call(e: &Expr) -> Option<&syn::ExprMethodCall> {
    let mut e = peel(e);
    loop {
        match e {
            Expr::Try(t) => e = peel(&t.expr),
            Expr::MethodCall(m)
                if (m.args.is_empty() && m.method == "unwrap")
                    || (m.args.len() == 1 && (m.method == "expect" || m.method == "unwrap_or_else")) =>
            {
                e = peel(&m.receiver)
            }
            Expr::MethodCall(m) if m.args.is_empty() && LOCK_METHODS.iter().any(|n| m.method == n) => {
                return Some(m);
            }
            _ => return None,
        }
    }
}

/// `stdout()`/`stderr()`/`stdin()` locks are reentrant: re-locking on the same
/// thread does not deadlock.
fn is_reentrant(receiver: &Expr) -> bool {
    let key = expr_key(receiver).to_ascii_lowercase();
    ["stdout", "stderr", "stdin"].iter().any(|s| key.contains(s))
}

/// Finds sync lock calls inside an expression (not inside closures or async
/// blocks), returning (receiver key, span). Lock calls that are awaited are
/// returned too when `include_awaited` is set.
fn lock_calls(e: &Expr, include_awaited: bool) -> Vec<(String, proc_macro2::Span)> {
    struct F {
        out: Vec<(String, proc_macro2::Span)>,
        include_awaited: bool,
        awaited: Vec<*const syn::ExprMethodCall>,
    }
    impl<'ast> Visit<'ast> for F {
        fn visit_expr_await(&mut self, a: &'ast syn::ExprAwait) {
            if let Expr::MethodCall(m) = peel(&a.base) {
                self.awaited.push(m as *const _);
            }
            visit::visit_expr_await(self, a);
        }
        fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
            let is_lock = m.args.is_empty() && LOCK_METHODS.iter().any(|n| m.method == n) && !is_reentrant(&m.receiver);
            let awaited = self.awaited.contains(&(m as *const _));
            if is_lock && (self.include_awaited || !awaited) {
                self.out.push((expr_key(&m.receiver), m.method.span()));
            }
            visit::visit_expr_method_call(self, m);
        }
        fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}
        fn visit_expr_async(&mut self, _: &'ast syn::ExprAsync) {}
        fn visit_item(&mut self, _: &'ast syn::Item) {}
        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            if let Some(exprs) = macro_exprs(mac) {
                for e in &exprs {
                    self.visit_expr(e);
                }
            }
        }
    }
    let mut f = F { out: Vec::new(), include_awaited, awaited: Vec::new() };
    f.visit_expr(e);
    f.out
}

/// First `.await` in a node (not inside closures, async blocks or items).
#[derive(Default)]
struct AwaitFinder {
    first: Option<proc_macro2::Span>,
    /// Stops the search at `drop(<name>)`.
    dropped_name: Option<String>,
    dropped: bool,
}

impl<'ast> Visit<'ast> for AwaitFinder {
    fn visit_expr_await(&mut self, a: &'ast syn::ExprAwait) {
        visit::visit_expr_await(self, a);
        if self.first.is_none() && !self.dropped {
            self.first = Some(a.await_token.span());
        }
    }
    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        if let (Expr::Path(f), Some(name)) = (&*c.func, &self.dropped_name)
            && f.path.segments.last().is_some_and(|s| s.ident == "drop")
            && c.args.len() == 1
            && expr_key(&c.args[0]) == *name
        {
            self.dropped = true;
        }
        visit::visit_expr_call(self, c);
    }
    fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}
    fn visit_expr_async(&mut self, _: &'ast syn::ExprAsync) {}
    fn visit_item(&mut self, _: &'ast syn::Item) {}
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(exprs) = macro_exprs(mac) {
            for e in &exprs {
                self.visit_expr(e);
            }
        }
    }
}

struct AcrossAwait<'s, 'c, 'a> {
    sink: &'s mut Sink<'c, 'a>,
    ctx: &'c FileContext<'a>,
    in_async: bool,
}

impl AcrossAwait<'_, '_, '_> {
    fn check_block(&mut self, block: &Block) {
        for (i, stmt) in block.stmts.iter().enumerate() {
            let Stmt::Local(local) = stmt else { continue };
            let Some(init) = &local.init else { continue };
            let name = match &local.pat {
                Pat::Ident(pi) => pi.ident.to_string(),
                Pat::Type(pt) => match &*pt.pat {
                    Pat::Ident(pi) => pi.ident.to_string(),
                    _ => continue,
                },
                _ => continue,
            };
            let Some(call) = guard_call(&init.expr) else { continue };
            let mut finder = AwaitFinder { dropped_name: Some(name.clone()), ..Default::default() };
            for later in &block.stmts[i + 1..] {
                // A new binding with the same name shadows the guard but does not drop it,
                // so keep scanning.
                finder.visit_stmt(later);
                if finder.first.is_some() || finder.dropped {
                    break;
                }
            }
            if let Some(await_span) = finder.first {
                let line = await_span.start().line;
                if self.ctx.is_production_line(line) {
                    self.sink.emit_span(
                        call.method.span(),
                        format!(
                            "guard `{name}` from `.{}()` is still held at the `.await` on line {line}; drop it first",
                            call.method
                        ),
                    );
                }
            }
        }
    }

    fn check_scrutinee(&mut self, scrutinee: &Expr, body: &dyn Fn(&mut AwaitFinder)) {
        if lock_calls(scrutinee, false).is_empty() || guard_call_in_chain(scrutinee).is_none() {
            return;
        }
        let mut finder = AwaitFinder::default();
        body(&mut finder);
        if let (Some(await_span), Some(call)) = (finder.first, guard_call_in_chain(scrutinee)) {
            let line = await_span.start().line;
            if self.ctx.is_production_line(line) {
                self.sink.emit_span(
                    call.method.span(),
                    format!(
                        "guard from `.{}()` in the scrutinee lives across the `.await` on line {line}",
                        call.method
                    ),
                );
            }
        }
    }
}

/// Any non-awaited lock call along the method chain of `e` (the receiver spine).
fn guard_call_in_chain(e: &Expr) -> Option<&syn::ExprMethodCall> {
    let mut cur = peel(e);
    loop {
        match cur {
            Expr::MethodCall(m) => {
                if m.args.is_empty() && LOCK_METHODS.iter().any(|n| m.method == n) {
                    return Some(m);
                }
                cur = peel(&m.receiver);
            }
            Expr::Try(t) => cur = peel(&t.expr),
            Expr::Field(f) => cur = peel(&f.base),
            Expr::Unary(u) => cur = peel(&u.expr),
            Expr::Reference(r) => cur = peel(&r.expr),
            Expr::Index(i) => cur = peel(&i.expr),
            _ => return None,
        }
    }
}

impl<'ast> Visit<'ast> for AcrossAwait<'_, '_, '_> {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        let prev = self.in_async;
        self.in_async = f.sig.asyncness.is_some();
        visit::visit_item_fn(self, f);
        self.in_async = prev;
    }
    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        let prev = self.in_async;
        self.in_async = f.sig.asyncness.is_some();
        visit::visit_impl_item_fn(self, f);
        self.in_async = prev;
    }
    fn visit_trait_item_fn(&mut self, f: &'ast syn::TraitItemFn) {
        let prev = self.in_async;
        self.in_async = f.sig.asyncness.is_some();
        visit::visit_trait_item_fn(self, f);
        self.in_async = prev;
    }
    fn visit_expr_async(&mut self, a: &'ast syn::ExprAsync) {
        let prev = self.in_async;
        self.in_async = true;
        visit::visit_expr_async(self, a);
        self.in_async = prev;
    }
    fn visit_expr_closure(&mut self, c: &'ast syn::ExprClosure) {
        let prev = self.in_async;
        self.in_async = c.asyncness.is_some();
        visit::visit_expr_closure(self, c);
        self.in_async = prev;
    }
    fn visit_block(&mut self, b: &'ast Block) {
        if self.in_async {
            self.check_block(b);
        }
        visit::visit_block(self, b);
    }
    fn visit_expr_match(&mut self, m: &'ast syn::ExprMatch) {
        if self.in_async {
            self.check_scrutinee(&m.expr, &|f: &mut AwaitFinder| {
                for arm in &m.arms {
                    f.visit_arm(arm);
                }
            });
        }
        visit::visit_expr_match(self, m);
    }
    fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
        if self.in_async
            && let Expr::Let(l) = &*i.cond
        {
            self.check_scrutinee(&l.expr, &|f: &mut AwaitFinder| f.visit_block(&i.then_branch));
        }
        visit::visit_expr_if(self, i);
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(exprs) = macro_exprs(mac) {
            for e in &exprs {
                self.visit_expr(e);
            }
        }
    }
}

impl Rule for LockAcrossAwait {
    fn meta(&self) -> &'static RuleMeta {
        &ACROSS_AWAIT
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        AcrossAwait { ctx: sink.ctx(), sink, in_async: false }.visit_file(&input.ctx.ast);
    }
}

struct Scrutinee<'s, 'c, 'a> {
    sink: &'s mut Sink<'c, 'a>,
    ctx: &'c FileContext<'a>,
}

impl Scrutinee<'_, '_, '_> {
    fn check(&mut self, scrutinee: &Expr, bodies: &[&Expr]) {
        let held = lock_calls(scrutinee, true);
        if held.is_empty() {
            return;
        }
        for body in bodies {
            for (key, span) in lock_calls(body, true) {
                if let Some((_, first)) = held.iter().find(|(k, _)| *k == key)
                    && self.ctx.is_production_line(span.start().line)
                {
                    self.sink.emit_span(
                            span,
                            format!(
                                "`{key}` is locked again while the guard from line {} (scrutinee) is still alive; this deadlocks",
                                first.start().line
                            ),
                        );
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for Scrutinee<'_, '_, '_> {
    fn visit_expr_match(&mut self, m: &'ast syn::ExprMatch) {
        let bodies: Vec<&Expr> = m
            .arms
            .iter()
            .flat_map(|a| a.guard.as_ref().map(|(_, g)| &**g).into_iter().chain(std::iter::once(&*a.body)))
            .collect();
        self.check(&m.expr, &bodies);
        visit::visit_expr_match(self, m);
    }
    fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
        if let Expr::Let(l) = &*i.cond {
            let then = Expr::Block(syn::ExprBlock { attrs: vec![], label: None, block: i.then_branch.clone() });
            self.check(&l.expr, &[&then]);
        }
        visit::visit_expr_if(self, i);
    }
    fn visit_expr_while(&mut self, w: &'ast syn::ExprWhile) {
        if let Expr::Let(l) = &*w.cond {
            let body = Expr::Block(syn::ExprBlock { attrs: vec![], label: None, block: w.body.clone() });
            self.check(&l.expr, &[&body]);
        }
        visit::visit_expr_while(self, w);
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(exprs) = macro_exprs(mac) {
            for e in &exprs {
                self.visit_expr(e);
            }
        }
    }
}

impl Rule for LockInScrutinee {
    fn meta(&self) -> &'static RuleMeta {
        &SCRUTINEE
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        Scrutinee { ctx: sink.ctx(), sink }.visit_file(&input.ctx.ast);
    }
}

const STD_FS_FNS: &[&str] = &[
    "read",
    "read_to_string",
    "write",
    "read_dir",
    "create_dir",
    "create_dir_all",
    "remove_file",
    "remove_dir",
    "remove_dir_all",
    "copy",
    "rename",
    "metadata",
    "canonicalize",
    "hard_link",
    "read_link",
    "set_permissions",
    "symlink_metadata",
];
const BLOCKING_METHODS: &[&str] =
    &["blocking_lock", "blocking_recv", "blocking_send", "blocking_read", "blocking_write", "block_on"];
const OFFLOAD_FNS: &[&str] = &["spawn_blocking", "block_in_place", "spawn", "scope"];

struct Blocking<'s, 'c, 'a> {
    sink: &'s mut Sink<'c, 'a>,
    ctx: &'c FileContext<'a>,
    imports: Imports,
    in_async: bool,
}

impl Blocking<'_, '_, '_> {
    fn report(&mut self, span: proc_macro2::Span, what: &str, instead: &str) {
        if self.in_async && self.ctx.is_production_line(span.start().line) {
            self.sink.emit_span(span, format!("blocking `{what}` inside async code; {instead}"));
        }
    }

    fn classify_call(&self, path: &syn::Path) -> Option<(&'static str, String)> {
        let full = self.imports.resolve(path);
        let full: Vec<&str> = full.iter().map(String::as_str).collect();
        let joined = full.join("::");
        match full.as_slice() {
            ["std", "thread", "sleep"] | ["thread", "sleep"] => Some(("use `tokio::time::sleep(..).await`", joined)),
            ["std", "fs", f] if STD_FS_FNS.contains(f) => Some(("use `tokio::fs` or `spawn_blocking`", joined)),
            ["std", "fs", "File", "open" | "create"] => Some(("use `tokio::fs::File` or `spawn_blocking`", joined)),
            ["std", "net", "TcpStream" | "TcpListener" | "UdpSocket", "connect" | "bind"] => {
                Some(("use the `tokio::net` equivalent", joined))
            }
            ["reqwest", "blocking", ..] => Some(("use the async `reqwest` client", joined)),
            ["futures", "executor", "block_on"] => Some(("`.await` the future instead", joined)),
            _ => None,
        }
    }
}

impl<'ast> Visit<'ast> for Blocking<'_, '_, '_> {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        let prev = self.in_async;
        self.in_async = f.sig.asyncness.is_some();
        visit::visit_item_fn(self, f);
        self.in_async = prev;
    }
    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        let prev = self.in_async;
        self.in_async = f.sig.asyncness.is_some();
        visit::visit_impl_item_fn(self, f);
        self.in_async = prev;
    }
    fn visit_trait_item_fn(&mut self, f: &'ast syn::TraitItemFn) {
        let prev = self.in_async;
        self.in_async = f.sig.asyncness.is_some();
        visit::visit_trait_item_fn(self, f);
        self.in_async = prev;
    }
    fn visit_expr_async(&mut self, a: &'ast syn::ExprAsync) {
        let prev = self.in_async;
        self.in_async = true;
        visit::visit_expr_async(self, a);
        self.in_async = prev;
    }
    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        if let Expr::Path(p) = &*c.func {
            if let Some((instead, what)) = self.classify_call(&p.path) {
                self.report(p.span(), &what, instead);
            }
            // Closures handed to free functions (`spawn_blocking`, `thread::spawn`, or wrappers
            // like tokio's `asyncify`) may run off the executor, so they are not inspected.
            // Closures passed to methods (iterator adapters) run inline and are.
            let offload = c.args.iter().any(|a| matches!(peel(a), Expr::Closure(_)));
            if offload {
                let prev = self.in_async;
                self.in_async = false;
                for arg in &c.args {
                    if matches!(peel(arg), Expr::Closure(_)) {
                        self.visit_expr(arg);
                    }
                }
                self.in_async = prev;
                for arg in &c.args {
                    if !matches!(peel(arg), Expr::Closure(_)) {
                        self.visit_expr(arg);
                    }
                }
                return;
            }
        }
        visit::visit_expr_call(self, c);
    }
    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if m.method == "block_on" {
            // `rt.block_on(async { .. })` from synchronous code: the future is driven by the
            // calling thread, so its body is not running inside another task's executor slot.
            self.visit_expr(&m.receiver);
            let prev = self.in_async;
            let was_async = self.in_async;
            if was_async {
                self.report(
                    m.method.span(),
                    ".block_on()",
                    "`.await` the future instead (this panics inside a runtime)",
                );
            }
            self.in_async = false;
            for arg in &m.args {
                if let Expr::Async(a) = peel(arg) {
                    self.visit_block(&a.block);
                } else {
                    self.visit_expr(arg);
                }
            }
            self.in_async = prev;
            return;
        }
        if BLOCKING_METHODS.iter().any(|n| m.method == n) {
            let instead = if m.method == "block_on" {
                "`.await` the future instead (this panics inside a runtime)"
            } else {
                "use the async version with `.await` (this panics inside a runtime)"
            };
            self.report(m.method.span(), &format!(".{}()", m.method), instead);
        }
        if OFFLOAD_FNS.iter().any(|n| m.method == n) {
            self.visit_expr(&m.receiver);
            let prev = self.in_async;
            self.in_async = false;
            for arg in &m.args {
                self.visit_expr(arg);
            }
            self.in_async = prev;
            return;
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

impl Rule for BlockingInAsync {
    fn meta(&self) -> &'static RuleMeta {
        &BLOCKING
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        let imports = Imports::collect(&input.ctx.ast);
        Blocking { ctx: sink.ctx(), sink, imports, in_async: false }.visit_file(&input.ctx.ast);
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::testing::{check_rule, lines};

    #[test]
    fn guard_across_await() {
        let src = r#"
async fn bad(m: &std::sync::Mutex<u32>) {
    let g = m.lock().unwrap();
    work().await;
}
async fn scoped(m: &std::sync::Mutex<u32>) {
    {
        let g = m.lock().unwrap();
    }
    work().await;
}
async fn dropped(m: &std::sync::Mutex<u32>) {
    let g = m.lock().unwrap();
    drop(g);
    work().await;
}
async fn tokio_mutex(m: &tokio::sync::Mutex<u32>) {
    let g = m.lock().await;
    work().await;
}
async fn value_only(m: &std::sync::Mutex<u32>) {
    let v = m.lock().unwrap().clone();
    work().await;
}
fn sync_fn(m: &std::sync::Mutex<u32>) {
    let g = m.lock().unwrap();
}
async fn in_match(m: &std::sync::Mutex<Option<u32>>) {
    match *m.lock().unwrap() {
        Some(_) => work().await,
        None => {}
    }
}
async fn refcell(c: &std::cell::RefCell<u32>) {
    let mut b = c.borrow_mut();
    work().await;
}
"#;
        assert_eq!(lines(&check_rule("lock-across-await", src)), vec![3, 29, 35]);
    }

    #[test]
    fn double_lock_in_scrutinee() {
        let src = r#"
fn bad(m: &std::sync::Mutex<Vec<u32>>) {
    match m.lock().unwrap().first() {
        Some(_) => { m.lock().unwrap().push(1); }
        None => {}
    }
    if let Some(x) = self.map.read().unwrap().get(&1) {
        self.map.write().unwrap().insert(2, *x);
    }
}
fn ok(m: &std::sync::Mutex<Vec<u32>>, n: &std::sync::Mutex<u32>) {
    let first = m.lock().unwrap().first().copied();
    match first { Some(_) => { m.lock().unwrap().push(1); } None => {} }
    match m.lock().unwrap().first() { Some(_) => { n.lock().unwrap(); } None => {} }
    match stderr.lock().write_all(b"x") { Ok(_) => stderr.lock().flush(), Err(e) => Err(e) }
    if m.lock().unwrap().is_empty() { m.lock().unwrap().push(1); }
}
"#;
        assert_eq!(lines(&check_rule("lock-in-scrutinee", src)), vec![4, 8]);
    }

    #[test]
    fn blocking_in_async() {
        let src = r#"
use std::fs;
use std::thread;
use std::fs::File;
async fn bad() {
    thread::sleep(std::time::Duration::from_millis(10));
    let s = fs::read_to_string("x");
    let f = File::open("y");
    let g = std::fs::write("z", b"");
    rx.blocking_recv();
}
async fn offloaded() {
    tokio::task::spawn_blocking(|| std::thread::sleep(d)).await;
    std::thread::spawn(move || fs::read("x"));
    asyncify(move || std::fs::remove_dir(p)).await;
}
fn sync_entry(rt: &Runtime) {
    rt.block_on(async { std::fs::read("x") });
}
async fn inline_closure(paths: Vec<String>) {
    let v: Vec<_> = paths.iter().map(|p| std::fs::read(p)).collect();
}
fn sync() { thread::sleep(d); }
async fn tokio_fs() { tokio::fs::read_to_string("x").await; tokio::time::sleep(d).await; }
"#;
        assert_eq!(lines(&check_rule("blocking-in-async", src)), vec![6, 7, 8, 9, 10, 21]);
    }

    #[test]
    fn tokio_fs_import_is_not_blocking() {
        let src = "use tokio::fs;\nasync fn a() { fs::read(\"x\").await; }\n";
        assert!(check_rule("blocking-in-async", src).is_empty());
    }
}
