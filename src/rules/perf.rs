//! Avoidable allocations. Every rule here is backed by a benchmark in
//! `validation/benches` (see docs/EVIDENCE.md for the measured cost).

use super::{Input, Rule, RuleMeta, Sink};
use crate::ast_util::{expr_key, macro_exprs, macro_name, path_string, peel};
use crate::context::FileContext;
use crate::finding::{Category, Evidence, Severity};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, GenericArgument, PathArguments, Type};

pub struct NeedlessCollect;
pub struct FormatPushString;
pub struct BorrowedClone;
pub struct RedundantBox;
pub struct EagerFallbackAlloc;

static COLLECT: RuleMeta = RuleMeta {
    id: "needless-collect",
    category: Category::Performance,
    severity: Severity::Warning,
    evidence: Evidence::Strong,
    default_enabled: true,
    summary: "Iterator collected into a `Vec` only to be measured or iterated again",
    rationale: "`.collect::<Vec<_>>().len()` allocates and fills a vector just to count it; \
`.count()`, `.next().is_none()`, `.any(..)` or iterating directly avoid the allocation. The Rust \
Performance Book: 'You should avoid calling collect if the collection is then only iterated over \
again.' Our benchmark measured the cost. Only the immediate-chain form is flagged, because \
Clippy's broader `needless_collect` was demoted for false positives (collecting is sometimes \
needed to end a borrow).",
    sources: &[
        "N. Nethercote. The Rust Performance Book, 'Iterators': https://nnethercote.github.io/perf-book/iterators.html",
        "Clippy lint `needless_collect` (nursery since 1.67, PR #9705).",
        "validation/benches/benches/patterns.rs::needless_collect (this repository)",
    ],
};

static FORMAT_PUSH: RuleMeta = RuleMeta {
    id: "format-push-string",
    category: Category::Performance,
    severity: Severity::Info,
    evidence: Evidence::Moderate,
    default_enabled: true,
    summary: "`s.push_str(&format!(..))` / `s += &format!(..)` allocates a temporary String",
    rationale: "`format!` allocates a new `String` that is copied into `s` and freed. \
`write!(s, ..)` (with `std::fmt::Write`) formats directly into the existing buffer. Clippy \
`format_push_string`: 'Introduces an extra, avoidable heap allocation.' The saving is real but \
modest (see benchmark), so this is reported as info.",
    sources: &[
        "Clippy lint `format_push_string`.",
        "validation/benches/benches/patterns.rs::format_push_string (this repository)",
    ],
};

static BORROWED_CLONE: RuleMeta = RuleMeta {
    id: "borrowed-clone",
    category: Category::Performance,
    severity: Severity::Warning,
    evidence: Evidence::Strong,
    default_enabled: true,
    summary: "`&x.clone()` passed as an argument: copies a value only to borrow it",
    rationale: "Cloning then immediately borrowing creates a full copy (an allocation for \
`String`/`Vec`) that is used as a reference and dropped; `&x` (or `x.as_ref()`) gives the same \
reference without the copy. The Rust Design Patterns book lists 'clone to satisfy the borrow \
checker' as an anti-pattern, and the Perf Book notes unnecessary clones arise from 'programmer \
error, or changes in the code that render previously-necessary clone calls unnecessary'. Not \
reported when the cloned value shares a root with the call's receiver or a `&mut` argument, \
where the clone may be needed to end a borrow.",
    sources: &[
        "Rust Design Patterns, 'Clone to satisfy the borrow checker': https://rust-unofficial.github.io/patterns/anti_patterns/borrow_clone.html",
        "The Rust Performance Book, 'Heap Allocations': https://nnethercote.github.io/perf-book/heap-allocations.html",
        "validation/benches/benches/patterns.rs::borrowed_clone (this repository)",
    ],
};

static REDUNDANT_BOX: RuleMeta = RuleMeta {
    id: "redundant-box",
    category: Category::Performance,
    severity: Severity::Warning,
    evidence: Evidence::Moderate,
    default_enabled: true,
    summary: "`Box<Vec<T>>`, `Box<String>`, `Box<HashMap<..>>`: a second, useless heap indirection",
    rationale: "Collections already store their contents on the heap; boxing them adds an extra \
allocation and pointer chase for no benefit. Clippy `box_collection` (perf group): 'you just add \
another level of indirection without any benefit whatsoever.'",
    sources: &[
        "Clippy lint `box_collection` (perf).",
        "validation/benches/benches/patterns.rs::redundant_box (this repository)",
    ],
};

static EAGER: RuleMeta = RuleMeta {
    id: "eager-fallback-alloc",
    category: Category::Performance,
    severity: Severity::Info,
    evidence: Evidence::Moderate,
    default_enabled: true,
    summary: "Allocating fallback evaluated eagerly (`.unwrap_or(format!(..))`, `.ok_or(x.to_string())`)",
    rationale: "Arguments to `unwrap_or`, `ok_or`, `map_or` and `or` are evaluated even when the \
fallback is not used, so an allocation is paid on every call, including the common success \
path. The `_else` variants take a closure and only allocate when needed. The Perf Book: 'err is \
computed eagerly. If its computation is expensive, you should instead use Option::ok_or_else'. \
Only allocating arguments are flagged (`String::new()`/`Vec::new()` are free).",
    sources: &[
        "The Rust Performance Book, 'Standard Library Types': https://nnethercote.github.io/perf-book/standard-library-types.html",
        "validation/benches/benches/patterns.rs::eager_fallback (this repository)",
    ],
};

/// Shared walker giving each perf rule production-only, macro-aware traversal.
struct Walker<'s, 'c, 'a, F> {
    sink: &'s mut Sink<'c, 'a>,
    ctx: &'c FileContext<'a>,
    on_expr: F,
}

impl<F: FnMut(&Expr, &mut Vec<(proc_macro2::Span, String)>)> Walker<'_, '_, '_, F> {
    fn run(&mut self, file: &syn::File) {
        self.visit_file(file);
    }
}

impl<'ast, F: FnMut(&Expr, &mut Vec<(proc_macro2::Span, String)>)> Visit<'ast> for Walker<'_, '_, '_, F> {
    fn visit_expr(&mut self, e: &'ast Expr) {
        let mut out = Vec::new();
        (self.on_expr)(e, &mut out);
        for (span, msg) in out {
            if self.ctx.is_production_line(span.start().line) {
                self.sink.emit_span(span, msg);
            }
        }
        visit::visit_expr(self, e);
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(exprs) = macro_exprs(mac) {
            for e in &exprs {
                self.visit_expr(e);
            }
        }
    }
}

fn walk(input: &Input, sink: &mut Sink, f: impl FnMut(&Expr, &mut Vec<(proc_macro2::Span, String)>)) {
    Walker { ctx: sink.ctx(), sink, on_expr: f }.run(&input.ctx.ast);
}

fn turbofish_is_vec(m: &syn::ExprMethodCall) -> bool {
    let Some(tf) = &m.turbofish else { return false };
    tf.args.iter().any(|a| match a {
        GenericArgument::Type(Type::Path(tp)) => tp.path.segments.last().is_some_and(|s| s.ident == "Vec"),
        _ => false,
    })
}

impl Rule for NeedlessCollect {
    fn meta(&self) -> &'static RuleMeta {
        &COLLECT
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        walk(input, sink, |e, out| {
            let Expr::MethodCall(outer) = e else { return };
            let suggestion = match outer.method.to_string().as_str() {
                "len" => "use `.count()`",
                "is_empty" => "use `.next().is_none()`",
                "iter" | "into_iter" => "iterate the original iterator directly",
                "contains" => "use `.any(|x| x == ..)`",
                "first" => "use `.next()`",
                "last" => "use `.last()` on the iterator",
                _ => return,
            };
            let Expr::MethodCall(inner) = peel(&outer.receiver) else { return };
            if inner.method == "collect" && turbofish_is_vec(inner) {
                out.push((inner.method.span(), format!("collected into a Vec only to call `.{}()`; {suggestion}", outer.method)));
            }
        });
    }
}

fn is_format_macro(e: &Expr) -> bool {
    match peel(e) {
        Expr::Reference(r) => is_format_macro(&r.expr),
        Expr::Macro(m) => macro_name(&m.mac) == "format",
        _ => false,
    }
}

impl Rule for FormatPushString {
    fn meta(&self) -> &'static RuleMeta {
        &FORMAT_PUSH
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        walk(input, sink, |e, out| match e {
            Expr::MethodCall(m) if m.method == "push_str" && m.args.len() == 1 && is_format_macro(&m.args[0]) => {
                out.push((m.method.span(), "`push_str(&format!(..))` allocates a temporary; use `write!(s, ..)`".into()));
            }
            Expr::Binary(b) if matches!(b.op, syn::BinOp::AddAssign(_)) && is_format_macro(&b.right) => {
                out.push((b.op.span(), "`+= &format!(..)` allocates a temporary; use `write!(s, ..)`".into()));
            }
            _ => {}
        });
    }
}

/// The variable an expression is rooted at: `self` for `self.a.b()`, `x` for `x[0].y`.
fn root(e: &Expr) -> Option<String> {
    match peel(e) {
        Expr::Path(p) => Some(path_string(&p.path)),
        Expr::Field(f) => root(&f.base),
        Expr::MethodCall(m) => root(&m.receiver),
        Expr::Index(i) => root(&i.expr),
        Expr::Reference(r) => root(&r.expr),
        Expr::Unary(u) => root(&u.expr),
        _ => None,
    }
}

const CLONE_METHODS: &[&str] = &["clone", "to_owned", "to_vec"];

fn borrowed_clone(arg: &Expr) -> Option<&syn::ExprMethodCall> {
    let Expr::Reference(r) = peel(arg) else { return None };
    if r.mutability.is_some() {
        return None;
    }
    match peel(&r.expr) {
        Expr::MethodCall(m) if m.args.is_empty() && CLONE_METHODS.iter().any(|n| m.method == n) => Some(m),
        _ => None,
    }
}

impl Rule for BorrowedClone {
    fn meta(&self) -> &'static RuleMeta {
        &BORROWED_CLONE
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        walk(input, sink, |e, out| {
            let (args, receiver): (Vec<&Expr>, Option<&Expr>) = match e {
                Expr::Call(c) => (c.args.iter().collect(), None),
                Expr::MethodCall(m) => (m.args.iter().collect(), Some(&*m.receiver)),
                _ => return,
            };
            let mut_roots: Vec<String> = args
                .iter()
                .filter_map(|a| match peel(a) {
                    Expr::Reference(r) if r.mutability.is_some() => root(&r.expr),
                    _ => None,
                })
                .collect();
            let recv_root = receiver.and_then(root);
            for arg in &args {
                let Some(m) = borrowed_clone(arg) else { continue };
                let r = root(&m.receiver);
                if r.is_some() && (r == recv_root || mut_roots.contains(r.as_ref().unwrap_or(&String::new()))) {
                    continue;
                }
                out.push((
                    m.method.span(),
                    format!("`&{}.{}()` copies the value only to borrow it; pass `&{}`", expr_key(&m.receiver), m.method, expr_key(&m.receiver)),
                ));
            }
        });
    }
}

const COLLECTIONS: &[&str] = &["Vec", "String", "HashMap", "HashSet", "BTreeMap", "BTreeSet", "VecDeque", "BinaryHeap"];

impl Rule for RedundantBox {
    fn meta(&self) -> &'static RuleMeta {
        &REDUNDANT_BOX
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        struct V<'s, 'c, 'a> {
            sink: &'s mut Sink<'c, 'a>,
            ctx: &'c FileContext<'a>,
        }
        impl<'ast> Visit<'ast> for V<'_, '_, '_> {
            fn visit_type_path(&mut self, tp: &'ast syn::TypePath) {
                if let Some(seg) = tp.path.segments.last() {
                    if seg.ident == "Box" {
                        if let PathArguments::AngleBracketed(args) = &seg.arguments {
                            if let Some(GenericArgument::Type(Type::Path(inner))) = args.args.first() {
                                if let Some(inner_seg) = inner.path.segments.last() {
                                    let name = inner_seg.ident.to_string();
                                    if COLLECTIONS.contains(&name.as_str())
                                        && self.ctx.is_production_line(tp.span().start().line)
                                    {
                                        self.sink.emit_span(
                                            tp.span(),
                                            format!("`Box<{name}<..>>` double-boxes a heap collection; use `{name}` directly"),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                visit::visit_type_path(self, tp);
            }
        }
        V { ctx: sink.ctx(), sink }.visit_file(&input.ctx.ast);
    }
}

/// Does evaluating `e` allocate (as far as we can tell syntactically)?
fn allocates(e: &Expr) -> Option<&'static str> {
    match peel(e) {
        Expr::Macro(m) => match macro_name(&m.mac).as_str() {
            "format" => Some("format!"),
            "vec" if !m.mac.tokens.is_empty() => Some("vec![..]"),
            _ => None,
        },
        Expr::MethodCall(m) if m.args.is_empty() => match m.method.to_string().as_str() {
            "to_string" => Some(".to_string()"),
            "to_owned" => Some(".to_owned()"),
            "to_vec" => Some(".to_vec()"),
            _ => None,
        },
        Expr::Call(c) => match &*c.func {
            Expr::Path(p) if path_string(&p.path).ends_with("String::from") => Some("String::from"),
            _ => None,
        },
        _ => None,
    }
}

impl Rule for EagerFallbackAlloc {
    fn meta(&self) -> &'static RuleMeta {
        &EAGER
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        walk(input, sink, |e, out| {
            let Expr::MethodCall(m) = e else { return };
            let lazy = match m.method.to_string().as_str() {
                "unwrap_or" => "unwrap_or_else",
                "ok_or" => "ok_or_else",
                "map_or" => "map_or_else",
                "or" => "or_else",
                _ => return,
            };
            let Some(arg) = m.args.first() else { return };
            if let Some(what) = allocates(arg) {
                out.push((
                    m.method.span(),
                    format!("`{what}` in `.{}(..)` allocates even when unused; use `.{lazy}(|| ..)`", m.method),
                ));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::testing::{check_rule, check_rule_at, lines};

    #[test]
    fn needless_collect() {
        let src = r#"
fn a(v: &[u32]) -> usize {
    let n = v.iter().filter(|x| **x > 1).collect::<Vec<_>>().len();
    let e = v.iter().collect::<Vec<&u32>>().is_empty();
    let d = v.iter().collect::<std::collections::HashSet<_>>().len();
    let s = v.iter().collect::<Vec<_>>();
    n
}
"#;
        assert_eq!(lines(&check_rule("needless-collect", src)), vec![3, 4]);
    }

    #[test]
    fn format_push_string() {
        let src = "fn a(s: &mut String, x: u8) { s.push_str(&format!(\"{x}\")); *s += &format!(\"{x}\"); s.push_str(\"lit\"); }";
        assert_eq!(check_rule("format-push-string", src).len(), 2);
    }

    #[test]
    fn borrowed_clone() {
        let src = r#"
fn a(name: String, v: Vec<u8>) {
    takes(&name.clone());
    obj.method(&v.to_vec(), 1);
    self.process(&self.name.clone());
    update(&mut state, &state.key.clone());
    takes(&name);
    let owned = name.clone();
}
"#;
        assert_eq!(lines(&check_rule("borrowed-clone", src)), vec![3, 4]);
    }

    #[test]
    fn redundant_box() {
        let src = "struct S { a: Box<Vec<u8>>, b: Box<String>, c: Box<[u8]>, d: Box<dyn Fn()> }\nfn f(x: Box<std::collections::HashMap<u8, u8>>) {}";
        assert_eq!(lines(&check_rule("redundant-box", src)), vec![1, 1, 2]);
    }

    #[test]
    fn eager_fallback() {
        let src = r#"
fn a(o: Option<String>, r: Option<u8>, id: u32) {
    let x = o.clone().unwrap_or(format!("user-{id}"));
    let y = r.ok_or("missing".to_string());
    let z = o.clone().unwrap_or(String::new());
    let w = o.unwrap_or_else(|| format!("{id}"));
    let v = r.map_or(vec![1, 2], |x| vec![x]);
}
"#;
        assert_eq!(lines(&check_rule("eager-fallback-alloc", src)), vec![3, 4, 7]);
    }

    #[test]
    fn perf_rules_skip_tests() {
        let src = "fn a(o: Option<String>) { o.unwrap_or(format!(\"x\")); }";
        assert!(check_rule_at("eager-fallback-alloc", "tests/t.rs", src).is_empty());
    }
}
