//! `unsafe` code hygiene and known-unsound idioms.

use super::{Input, Rule, RuleMeta, Sink};
use crate::ast_util::{contains_method_call, macro_exprs, path_string};
use crate::context::FileContext;
use crate::finding::{Category, Evidence, Severity};
use quote::ToTokens;
use std::collections::HashMap;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, GenericParam, Stmt, TypeParamBound, WherePredicate};

pub struct UndocumentedUnsafe;
pub struct UnboundedSendSync;
pub struct UninitMemory;
pub struct SetLenUninit;

static UNDOCUMENTED: RuleMeta = RuleMeta {
    id: "undocumented-unsafe",
    category: Category::Correctness,
    severity: Severity::Warning,
    evidence: Evidence::Moderate,
    default_enabled: true,
    summary: "`unsafe` block or impl without a `// SAFETY:` comment",
    rationale: "Every memory-safety bug in the PLDI 2020 study of real Rust programs involved \
unsafe code, and soundness depends on invariants the compiler cannot check. The Rust standard \
library requires that 'each unsafe block should have a SAFETY: comment explaining why the block \
is safe, which invariants are used and must be respected'; Clippy (`undocumented_unsafe_blocks`) \
and Effective Rust (Item 16) recommend the same. Writing the justification is how reviewers (and \
authors) spot unsoundness. The comment may precede the statement or open the block.",
    sources: &[
        "std-dev-guide, Safety comments policy: https://std-dev-guide.rust-lang.org/policy/safety-comments.html",
        "Clippy lint `undocumented_unsafe_blocks`.",
        "D. Drysdale. Effective Rust, Item 16. https://www.lurklurk.org/effective-rust/unsafe.html",
        "Qin et al. PLDI 2020. doi:10.1145/3385412.3386036",
    ],
};

static SEND_SYNC: RuleMeta = RuleMeta {
    id: "unbounded-send-sync",
    category: Category::Correctness,
    severity: Severity::Warning,
    evidence: Evidence::Strong,
    default_enabled: true,
    summary: "`unsafe impl Send/Sync` for a generic type without `T: Send`/`T: Sync` bounds",
    rationale: "`unsafe impl<T> Send for Wrapper<T>` lets `Wrapper<Rc<_>>` or \
`Wrapper<Cell<_>>` cross threads, causing data races in safe code. For `Sync`, a `T: Send` bound \
is accepted (as for `Mutex<T>`), since containers that only move values need no more. Xu et al. (TOSEM 2021) found \
missing Send/Sync bounds to be the most common pattern among memory-safety bugs in all Rust CVEs \
(30 of 185 bugs), and Rudra (Bae et al., SOSP 2021) found 178 such bugs in crates.io. Type \
parameters used only inside `PhantomData` (when the type is defined in the same file) are not \
reported.",
    sources: &[
        "Xu et al. Memory-Safety Challenge Considered Solved? An In-Depth Study with All Rust CVEs. ACM TOSEM 31(1), 2021. doi:10.1145/3466642",
        "Bae et al. Rudra: Finding Memory Safety Bugs in Rust at the Ecosystem Scale. SOSP 2021. doi:10.1145/3477132.3483570",
    ],
};

static UNINIT: RuleMeta = RuleMeta {
    id: "uninit-memory",
    category: Category::Correctness,
    severity: Severity::Error,
    evidence: Evidence::Strong,
    default_enabled: true,
    summary: "`MaybeUninit::uninit().assume_init()` or `mem::uninitialized()`",
    rationale: "Asserting that never-initialised memory is initialised is undefined behaviour for \
almost every type. `mem::uninitialized` is deprecated for this reason. Clippy \
`uninit_assumed_init` is deny-by-default: 'For most types, this is undefined behavior.'",
    sources: &[
        "std::mem::MaybeUninit docs: https://doc.rust-lang.org/std/mem/union.MaybeUninit.html",
        "Clippy lint `uninit_assumed_init` (correctness).",
    ],
};

static SET_LEN: RuleMeta = RuleMeta {
    id: "set-len-uninit",
    category: Category::Correctness,
    severity: Severity::Error,
    evidence: Evidence::Strong,
    default_enabled: true,
    summary: "`Vec::set_len` exposes uninitialised elements",
    rationale: "Calling `set_len` right after `with_capacity`/`reserve`, before writing the \
elements, makes the vector's contents uninitialised memory; reading them (or passing the buffer \
to a `Read` impl) is undefined behaviour. This uninitialised-`Vec` pattern was one of Rudra's \
most productive bug classes (Bae et al., SOSP 2021) and became Clippy's deny-by-default \
`uninit_vec` lint.",
    sources: &[
        "Bae et al. Rudra. SOSP 2021. doi:10.1145/3477132.3483570",
        "Clippy lint `uninit_vec` (correctness).",
    ],
};

/// Is there a comment mentioning safety above (or on) any of `lines`, or as the
/// first line inside a block opened on `block_open_line`?
fn has_safety_comment(ctx: &FileContext, lines: &[usize], block_open_line: Option<usize>) -> bool {
    let mentions = |text: &str| text.to_ascii_lowercase().contains("safety");
    let above = lines.iter().any(|&line| {
        ctx.comments_above(line).iter().any(|c| mentions(&c.text))
            || ctx.comments().iter().any(|c| c.line == line && mentions(&c.text))
    });
    above
        || block_open_line
            .is_some_and(|l| ctx.comments().iter().any(|c| c.line == l + 1 && c.own_line && mentions(&c.text)))
}

struct UnsafeFinder<'s, 'c, 'a> {
    sink: &'s mut Sink<'c, 'a>,
    ctx: &'c FileContext<'a>,
    /// Start lines of the enclosing statements, innermost last.
    stmt_lines: Vec<usize>,
}

impl<'ast> Visit<'ast> for UnsafeFinder<'_, '_, '_> {
    fn visit_stmt(&mut self, s: &'ast Stmt) {
        self.stmt_lines.push(s.span().start().line);
        visit::visit_stmt(self, s);
        self.stmt_lines.pop();
    }
    fn visit_expr_unsafe(&mut self, u: &'ast syn::ExprUnsafe) {
        let line = u.unsafe_token.span.start().line;
        let mut candidates = vec![line];
        candidates.extend(self.stmt_lines.iter().copied());
        let open = u.block.brace_token.span.open().start().line;
        let multiline = u.block.brace_token.span.close().start().line > open;
        let first_inside = if multiline { Some(open) } else { None };
        if self.ctx.is_production_line(line) && !has_safety_comment(self.ctx, &candidates, first_inside) {
            self.sink.emit_span(u.unsafe_token.span, "`unsafe` block without a `// SAFETY:` comment");
        }
        visit::visit_expr_unsafe(self, u);
    }
    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        if let Some(tok) = &i.unsafety {
            let line = tok.span.start().line;
            if self.ctx.is_production_line(line) && !has_safety_comment(self.ctx, &[line], None) {
                self.sink.emit_span(tok.span, "`unsafe impl` without a `// SAFETY:` comment");
            }
        }
        visit::visit_item_impl(self, i);
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(exprs) = macro_exprs(mac) {
            for e in &exprs {
                self.visit_expr(e);
            }
        }
    }
}

impl Rule for UndocumentedUnsafe {
    fn meta(&self) -> &'static RuleMeta {
        &UNDOCUMENTED
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        UnsafeFinder { ctx: sink.ctx(), sink, stmt_lines: Vec::new() }.visit_file(&input.ctx.ast);
    }
}

/// struct/enum name -> field types (as token strings).
fn field_types(file: &syn::File) -> HashMap<String, Vec<syn::Type>> {
    struct V(HashMap<String, Vec<syn::Type>>);
    impl<'ast> Visit<'ast> for V {
        fn visit_item_struct(&mut self, s: &'ast syn::ItemStruct) {
            self.0.insert(s.ident.to_string(), s.fields.iter().map(|f| f.ty.clone()).collect());
        }
        fn visit_item_enum(&mut self, e: &'ast syn::ItemEnum) {
            let tys = e.variants.iter().flat_map(|v| v.fields.iter().map(|f| f.ty.clone())).collect();
            self.0.insert(e.ident.to_string(), tys);
        }
    }
    let mut v = V(HashMap::new());
    v.visit_file(file);
    v.0
}

fn mentions_ident(ty: &syn::Type, ident: &str) -> bool {
    ty.to_token_stream().into_iter().any(|t| contains_ident(&t, ident))
}

fn contains_ident(t: &proc_macro2::TokenTree, ident: &str) -> bool {
    match t {
        proc_macro2::TokenTree::Ident(i) => i == ident,
        proc_macro2::TokenTree::Group(g) => g.stream().into_iter().any(|t| contains_ident(&t, ident)),
        _ => false,
    }
}

fn is_phantom(ty: &syn::Type) -> bool {
    matches!(ty, syn::Type::Path(tp) if tp.path.segments.last().is_some_and(|s| s.ident == "PhantomData"))
}

const NON_AUTO_STD_TRAITS: &[&str] = &[
    "Sized", "Clone", "Copy", "Debug", "Display", "Default", "PartialEq", "Eq", "PartialOrd", "Ord", "Hash",
    "Unpin", "Future", "Iterator", "AsRef", "AsMut", "Borrow", "Into", "From", "Fn", "FnMut", "FnOnce", "Any",
];

fn bound_names(bounds: &syn::punctuated::Punctuated<TypeParamBound, syn::Token![+]>) -> Vec<String> {
    bounds
        .iter()
        .filter_map(|b| match b {
            TypeParamBound::Trait(t) => t.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        })
        .collect()
}

impl Rule for UnboundedSendSync {
    fn meta(&self) -> &'static RuleMeta {
        &SEND_SYNC
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        let fields = field_types(&input.ctx.ast);
        struct V<'s, 'c, 'a, 'f> {
            sink: &'s mut Sink<'c, 'a>,
            fields: &'f HashMap<String, Vec<syn::Type>>,
        }
        impl<'ast> Visit<'ast> for V<'_, '_, '_, '_> {
            fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
                visit::visit_item_impl(self, i);
                let (Some(unsafety), Some((None, trait_path, _))) = (&i.unsafety, &i.trait_) else { return };
                let Some(trait_name) = trait_path.segments.last().map(|s| s.ident.to_string()) else { return };
                if trait_name != "Send" && trait_name != "Sync" {
                    return;
                }
                let self_name = match &*i.self_ty {
                    syn::Type::Path(tp) => tp.path.segments.last().map(|s| s.ident.to_string()),
                    _ => None,
                };
                let mut bounds: HashMap<String, Vec<String>> = HashMap::new();
                let mut params = Vec::new();
                for p in &i.generics.params {
                    if let GenericParam::Type(tp) = p {
                        params.push(tp.ident.to_string());
                        bounds.entry(tp.ident.to_string()).or_default().extend(bound_names(&tp.bounds));
                    }
                }
                if let Some(wc) = &i.generics.where_clause {
                    for pred in &wc.predicates {
                        if let WherePredicate::Type(pt) = pred {
                            let key = pt.bounded_ty.to_token_stream().to_string();
                            bounds.entry(key).or_default().extend(bound_names(&pt.bounds));
                        }
                    }
                }
                // `Sync` for a container whose values are moved in and out (like
                // `Mutex<T>` or a channel) only needs `T: Send`.
                let accepted: &[&str] = if trait_name == "Send" { &["Send"] } else { &["Sync", "Send"] };
                for p in params {
                    if !mentions_ident(&i.self_ty, &p) {
                        continue;
                    }
                    let has = bounds.get(&p).is_some_and(|b| b.iter().any(|n| accepted.contains(&n.as_str())))
                        // A bound on an associated type (`T::Target: Send`) shows the author considered it.
                        || bounds.iter().any(|(k, b)| {
                            k.starts_with(&format!("{p} ::")) && b.iter().any(|n| accepted.contains(&n.as_str()))
                        });
                    // A bound on a crate-specific trait may carry `Send`/`Sync` as a supertrait,
                    // which we cannot see; only report parameters bounded by nothing or by std
                    // traits that are known not to imply it.
                    let opaque = bounds.get(&p).is_some_and(|b| b.iter().any(|n| !NON_AUTO_STD_TRAITS.contains(&n.as_str())));
                    if has || opaque {
                        continue;
                    }
                    // Skip parameters that only appear inside PhantomData.
                    if let Some(tys) = self_name.as_ref().and_then(|n| self.fields.get(n)) {
                        let used = tys.iter().any(|t| mentions_ident(t, &p) && !is_phantom(t));
                        if !used {
                            continue;
                        }
                    }
                    self.sink.emit_span(
                        unsafety.span,
                        format!("`unsafe impl {trait_name}` does not require `{p}: {trait_name}`; any `{p}` (e.g. `Rc`) becomes {trait_name}"),
                    );
                }
            }
        }
        V { sink, fields: &fields }.visit_file(&input.ctx.ast);
    }
}

impl Rule for UninitMemory {
    fn meta(&self) -> &'static RuleMeta {
        &UNINIT
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        struct V<'s, 'c, 'a> {
            sink: &'s mut Sink<'c, 'a>,
        }
        impl<'ast> Visit<'ast> for V<'_, '_, '_> {
            fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
                if m.method == "assume_init" && self.sink.ctx().is_production_line(m.method.span().start().line) {
                    if let Expr::Call(c) = &*m.receiver {
                        if let Expr::Path(p) = &*c.func {
                            if path_string(&p.path).ends_with("MaybeUninit::uninit") {
                                self.sink.emit_span(m.method.span(), "`MaybeUninit::uninit().assume_init()` is undefined behaviour");
                            }
                        }
                    }
                }
                visit::visit_expr_method_call(self, m);
            }
            fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
                if let Expr::Path(p) = &*c.func {
                    let path = path_string(&p.path);
                    if path == "mem::uninitialized" || path.ends_with("::mem::uninitialized") {
                        self.sink.emit_span(p.span(), "`mem::uninitialized()` is deprecated and undefined behaviour for most types");
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
        V { sink }.visit_file(&input.ctx.ast);
    }
}

const WRITES: &[&str] = &[
    "as_mut_ptr", "spare_capacity_mut", "write", "copy_nonoverlapping", "copy_from_slice", "copy_to",
    "copy_to_nonoverlapping", "write_bytes", "fill", "resize", "extend_from_slice", "push", "read",
];

impl Rule for SetLenUninit {
    fn meta(&self) -> &'static RuleMeta {
        &SET_LEN
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        struct V<'s, 'c, 'a> {
            sink: &'s mut Sink<'c, 'a>,
        }
        fn stmt_expr(s: &Stmt) -> Option<&Expr> {
            match s {
                Stmt::Expr(e, _) => Some(e),
                Stmt::Local(l) => l.init.as_ref().map(|i| &*i.expr),
                _ => None,
            }
        }
        fn mentions_call(s: &Stmt, names: &[&str]) -> bool {
            struct F<'n> {
                names: &'n [&'n str],
                found: bool,
            }
            impl<'ast> Visit<'ast> for F<'_> {
                fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
                    if let Expr::Path(p) = &*c.func {
                        if p.path.segments.last().is_some_and(|s| self.names.iter().any(|n| s.ident == n)) {
                            self.found = true;
                        }
                    }
                    visit::visit_expr_call(self, c);
                }
                fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
                    if self.names.iter().any(|n| m.method == n) {
                        self.found = true;
                    }
                    visit::visit_expr_method_call(self, m);
                }
            }
            let mut f = F { names, found: false };
            f.visit_stmt(s);
            f.found
        }
        impl<'ast> Visit<'ast> for V<'_, '_, '_> {
            fn visit_block(&mut self, b: &'ast syn::Block) {
                // Like Clippy's `uninit_vec`: only `set_len` *immediately* after the
                // allocation is reported, and buffers of `MaybeUninit` are fine.
                let mut prev_allocates = false;
                for s in &b.stmts {
                    let text = s.to_token_stream().to_string();
                    if mentions_call(s, &["set_len"]) {
                        let set_len_writes = stmt_expr(s).is_some_and(|e| contains_method_call(e, WRITES));
                        let line = s.span().start().line;
                        if prev_allocates && !set_len_writes && self.sink.ctx().is_production_line(line) {
                            self.sink.emit_span(s.span(), "`set_len` right after allocating, before initialising the elements");
                        }
                        prev_allocates = false;
                    } else {
                        prev_allocates = mentions_call(s, &["with_capacity", "reserve", "reserve_exact"])
                            && !text.contains("MaybeUninit");
                    }
                }
                visit::visit_block(self, b);
            }
        }
        V { sink }.visit_file(&input.ctx.ast);
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::testing::{check_rule, lines};

    #[test]
    fn undocumented_unsafe() {
        let src = r#"
fn a(p: *const u8) -> u8 {
    unsafe { *p }
}
fn b(p: *const u8) -> u8 {
    // SAFETY: caller guarantees p is valid.
    unsafe { *p }
}
fn c(p: *const u8) -> u8 {
    // SAFETY: p is valid
    let x = foo(
        unsafe { *p },
    );
    x
}
fn d(p: *const u8) -> u8 {
    unsafe {
        // SAFETY: valid
        *p
    }
}
struct S;
unsafe impl Send for S {}
// Safety: S has no interior pointers.
unsafe impl Sync for S {}
"#;
        assert_eq!(lines(&check_rule("undocumented-unsafe", src)), vec![3, 23]);
    }

    #[test]
    fn unbounded_send_sync() {
        let src = r#"
struct W<T>(T);
unsafe impl<T> Send for W<T> {}
unsafe impl<T: Send> Send for W<T> {}
unsafe impl<T> Sync for W<T> where T: Sync {}
struct P<T> { _m: std::marker::PhantomData<T>, raw: *mut u8 }
unsafe impl<T> Send for P<T> {}
unsafe impl<T: Send> Sync for W<T> {}
unsafe impl<T> Sync for W<T> {}
struct L<T>(T);
unsafe impl<T: Link> Send for L<T> where T::Target: Send {}
unsafe impl<T: Schedule> Send for L<T> {}
unsafe impl<T: Clone> Send for L<T> {}
"#;
        assert_eq!(lines(&check_rule("unbounded-send-sync", src)), vec![3, 9, 13]);
    }

    #[test]
    fn uninit_memory() {
        let src = "fn a() -> u32 { unsafe { std::mem::MaybeUninit::uninit().assume_init() } }\n\
                   fn b() -> u32 { unsafe { std::mem::uninitialized() } }\n\
                   fn c() -> std::mem::MaybeUninit<u32> { std::mem::MaybeUninit::uninit() }\n";
        assert_eq!(lines(&check_rule("uninit-memory", src)), vec![1, 2]);
    }

    #[test]
    fn set_len_uninit() {
        let src = r#"
fn bad(n: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(n);
    unsafe { v.set_len(n); }
    v
}
fn good(n: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(n);
    unsafe {
        std::ptr::write_bytes(v.as_mut_ptr(), 0, n);
        v.set_len(n);
    }
    v
}
fn truncate(v: &mut Vec<u8>) { unsafe { v.set_len(0); } }
fn later_write(n: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(n);
    fill(&mut v);
    unsafe { v.set_len(n); }
    v
}
fn maybe_uninit(n: usize) -> Vec<std::mem::MaybeUninit<u8>> {
    let mut v: Vec<std::mem::MaybeUninit<u8>> = Vec::with_capacity(n);
    unsafe { v.set_len(n); }
    v
}
"#;
        assert_eq!(lines(&check_rule("set-len-uninit", src)), vec![4]);
    }
}
