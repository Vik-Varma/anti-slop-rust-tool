//! Per-function size and complexity metrics.

use crate::ast_util::macro_exprs;
use crate::context::{FileContext, has_test_attr};
use serde::Serialize;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{BinOp, Block, Expr, FnArg, Signature, Type};

#[derive(Debug, Clone, Serialize)]
pub struct FnMetrics {
    /// Function name.
    pub name: String,
    /// `module::Type::name` style path within the file.
    pub qual_name: String,
    /// Line of the function name.
    pub line: usize,
    pub column: usize,
    /// First line of the function including its (non-doc) attributes.
    pub start_line: usize,
    pub end_line: usize,
    /// Lines in the function that contain code (excludes blank and comment-only lines).
    pub loc: usize,
    /// Parameters, not counting `self`.
    pub params: usize,
    /// Parameters of type `bool`.
    pub bool_params: usize,
    /// McCabe cyclomatic complexity.
    pub cyclomatic: usize,
    /// SonarSource cognitive complexity.
    pub cognitive: usize,
    /// Deepest nesting of control-flow constructs.
    pub max_nesting: usize,
    pub is_test: bool,
    pub is_async: bool,
    pub is_unsafe: bool,
    /// True for trait methods and `impl Trait for T` methods, whose signature
    /// is dictated by the trait.
    pub is_trait_impl: bool,
}

/// Collects metrics for every function with a body in the file.
pub fn collect(ctx: &FileContext) -> Vec<FnMetrics> {
    let mut v = FnCollector { ctx, scope: Vec::new(), out: Vec::new(), in_trait_impl: false };
    v.visit_file(&ctx.ast);
    v.out
}

struct FnCollector<'c, 'a> {
    ctx: &'c FileContext<'a>,
    scope: Vec<String>,
    out: Vec<FnMetrics>,
    in_trait_impl: bool,
}

impl FnCollector<'_, '_> {
    fn record(&mut self, attrs: &[syn::Attribute], sig: &Signature, body: &Block, whole: proc_macro2::Span) {
        let start = attrs
            .iter()
            .filter(|a| !matches!(a.style, syn::AttrStyle::Inner(_)))
            .filter(|a| !a.path().is_ident("doc"))
            .map(|a| a.span().start().line)
            .min()
            .unwrap_or(sig.span().start().line)
            .min(sig.span().start().line);
        let end = whole.end().line;
        let loc = (start..=end).filter(|&l| self.ctx.lexed.line_has_code(l)).count();
        let (params, bool_params) = count_params(sig);
        let mut cx = Complexity::new(sig.ident.to_string());
        cx.visit_block(body);
        let name = sig.ident.to_string();
        let mut qual = self.scope.clone();
        qual.push(name.clone());
        let line = sig.ident.span().start().line;
        self.out.push(FnMetrics {
            name,
            qual_name: qual.join("::"),
            line,
            column: sig.ident.span().start().column + 1,
            start_line: start,
            end_line: end,
            loc,
            params,
            bool_params,
            cyclomatic: cx.cyclomatic,
            cognitive: cx.cognitive,
            max_nesting: cx.max_nesting,
            is_test: has_test_attr(attrs) || self.ctx.in_test(line),
            is_async: sig.asyncness.is_some(),
            is_unsafe: sig.unsafety.is_some(),
            is_trait_impl: self.in_trait_impl,
        });
    }
}

fn count_params(sig: &Signature) -> (usize, usize) {
    let mut params = 0;
    let mut bools = 0;
    for arg in &sig.inputs {
        if let FnArg::Typed(pt) = arg {
            params += 1;
            if let Type::Path(tp) = &*pt.ty {
                if tp.qself.is_none() && tp.path.is_ident("bool") {
                    bools += 1;
                }
            }
        }
    }
    (params, bools)
}

impl<'ast> Visit<'ast> for FnCollector<'_, '_> {
    fn visit_item_mod(&mut self, m: &'ast syn::ItemMod) {
        self.scope.push(m.ident.to_string());
        visit::visit_item_mod(self, m);
        self.scope.pop();
    }

    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        let ty = match &*i.self_ty {
            Type::Path(tp) => tp.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        }
        .unwrap_or_else(|| "_".into());
        let prev = self.in_trait_impl;
        self.in_trait_impl = i.trait_.is_some();
        self.scope.push(ty);
        visit::visit_item_impl(self, i);
        self.scope.pop();
        self.in_trait_impl = prev;
    }

    fn visit_item_trait(&mut self, t: &'ast syn::ItemTrait) {
        let prev = self.in_trait_impl;
        self.in_trait_impl = true;
        self.scope.push(t.ident.to_string());
        visit::visit_item_trait(self, t);
        self.scope.pop();
        self.in_trait_impl = prev;
    }

    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        let prev = self.in_trait_impl;
        self.in_trait_impl = false;
        self.record(&f.attrs, &f.sig, &f.block, f.span());
        self.scope.push(f.sig.ident.to_string());
        visit::visit_item_fn(self, f);
        self.scope.pop();
        self.in_trait_impl = prev;
    }

    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        self.record(&f.attrs, &f.sig, &f.block, f.span());
        self.scope.push(f.sig.ident.to_string());
        let prev = self.in_trait_impl;
        self.in_trait_impl = false;
        visit::visit_impl_item_fn(self, f);
        self.in_trait_impl = prev;
        self.scope.pop();
    }

    fn visit_trait_item_fn(&mut self, f: &'ast syn::TraitItemFn) {
        if let Some(body) = &f.default {
            self.record(&f.attrs, &f.sig, body, f.span());
        }
        self.scope.push(f.sig.ident.to_string());
        let prev = self.in_trait_impl;
        self.in_trait_impl = false;
        visit::visit_trait_item_fn(self, f);
        self.in_trait_impl = prev;
        self.scope.pop();
    }
}

/// Computes cyclomatic and cognitive complexity and nesting depth for one body.
///
/// Cognitive complexity follows G. Ann Campbell, "Cognitive Complexity: A new
/// way of measuring understandability" (SonarSource, 2018): +1 for each break
/// in linear flow (`if`, `else if`, `else`, `match`, loops, labelled
/// `break`/`continue`, each sequence of like boolean operators, recursion),
/// plus the current nesting level for structures that nest.
pub struct Complexity {
    fn_name: String,
    pub cyclomatic: usize,
    pub cognitive: usize,
    pub max_nesting: usize,
    nesting: usize,
}

impl Complexity {
    pub fn new(fn_name: String) -> Self {
        Complexity { fn_name, cyclomatic: 1, cognitive: 0, max_nesting: 0, nesting: 0 }
    }

    fn nested<F: FnOnce(&mut Self)>(&mut self, f: F) {
        self.nesting += 1;
        self.max_nesting = self.max_nesting.max(self.nesting);
        f(self);
        self.nesting -= 1;
    }

    fn structural(&mut self) {
        self.cognitive += 1 + self.nesting;
    }

    fn visit_if(&mut self, i: &syn::ExprIf, is_else_if: bool) {
        if is_else_if {
            self.cognitive += 1;
        } else {
            self.structural();
        }
        self.cyclomatic += 1;
        self.visit_expr(&i.cond);
        self.nested(|s| s.visit_block(&i.then_branch));
        if let Some((_, else_branch)) = &i.else_branch {
            match &**else_branch {
                Expr::If(inner) => self.visit_if(inner, true),
                other => {
                    self.cognitive += 1;
                    self.nested(|s| s.visit_expr(other));
                }
            }
        }
    }

    fn visit_logical(&mut self, e: &Expr, parent_op: Option<&BinOp>) {
        if let Expr::Binary(b) = e {
            if matches!(b.op, BinOp::And(_) | BinOp::Or(_)) {
                let same_as_parent = matches!(
                    (parent_op, &b.op),
                    (Some(BinOp::And(_)), BinOp::And(_)) | (Some(BinOp::Or(_)), BinOp::Or(_))
                );
                if !same_as_parent {
                    self.cognitive += 1;
                }
                self.cyclomatic += 1;
                self.visit_logical(&b.left, Some(&b.op));
                self.visit_logical(&b.right, Some(&b.op));
                return;
            }
        }
        self.visit_expr(e);
    }
}

impl<'ast> Visit<'ast> for Complexity {
    // Nested items are separate functions with their own metrics.
    fn visit_item(&mut self, _: &'ast syn::Item) {}

    fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
        self.visit_if(i, false);
    }

    fn visit_expr_match(&mut self, m: &'ast syn::ExprMatch) {
        self.structural();
        self.cyclomatic += m.arms.len().saturating_sub(1);
        self.visit_expr(&m.expr);
        self.nested(|s| {
            for arm in &m.arms {
                if let Some((_, guard)) = &arm.guard {
                    s.cyclomatic += 1;
                    s.visit_expr(guard);
                }
                s.visit_expr(&arm.body);
            }
        });
    }

    fn visit_expr_for_loop(&mut self, f: &'ast syn::ExprForLoop) {
        self.structural();
        self.cyclomatic += 1;
        self.visit_expr(&f.expr);
        self.nested(|s| s.visit_block(&f.body));
    }

    fn visit_expr_while(&mut self, w: &'ast syn::ExprWhile) {
        self.structural();
        self.cyclomatic += 1;
        self.visit_expr(&w.cond);
        self.nested(|s| s.visit_block(&w.body));
    }

    fn visit_expr_loop(&mut self, l: &'ast syn::ExprLoop) {
        self.structural();
        self.cyclomatic += 1;
        self.nested(|s| s.visit_block(&l.body));
    }

    fn visit_local(&mut self, local: &'ast syn::Local) {
        if let Some(init) = &local.init {
            self.visit_expr(&init.expr);
            if let Some((_, diverge)) = &init.diverge {
                // `let ... else { ... }` is a conditional branch.
                self.structural();
                self.cyclomatic += 1;
                self.nested(|s| s.visit_expr(diverge));
            }
        }
    }

    fn visit_expr_closure(&mut self, c: &'ast syn::ExprClosure) {
        self.nested(|s| s.visit_expr(&c.body));
    }

    fn visit_expr_break(&mut self, b: &'ast syn::ExprBreak) {
        if b.label.is_some() {
            self.cognitive += 1;
        }
        visit::visit_expr_break(self, b);
    }

    fn visit_expr_continue(&mut self, c: &'ast syn::ExprContinue) {
        if c.label.is_some() {
            self.cognitive += 1;
        }
    }

    fn visit_expr_binary(&mut self, b: &'ast syn::ExprBinary) {
        if matches!(b.op, BinOp::And(_) | BinOp::Or(_)) {
            // Re-enter through the logical-sequence counter.
            let e = Expr::Binary(b.clone());
            self.visit_logical(&e, None);
        } else {
            visit::visit_expr_binary(self, b);
        }
    }

    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        if let Expr::Path(p) = &*c.func {
            if p.path.segments.len() == 1 && p.path.is_ident(&self.fn_name) {
                self.cognitive += 1;
            }
        }
        visit::visit_expr_call(self, c);
    }

    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if m.method == self.fn_name {
            if let Expr::Path(p) = &*m.receiver {
                if p.path.is_ident("self") {
                    self.cognitive += 1;
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn metrics(src: &str) -> Vec<FnMetrics> {
        let ctx = FileContext::new(Path::new("src/lib.rs"), PathBuf::from("src/lib.rs"), src).unwrap();
        collect(&ctx)
    }

    #[test]
    fn straight_line_function() {
        let m = metrics("fn f(a: i32, b: bool) -> i32 {\n    // comment\n\n    a + 1\n}\n");
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].loc, 3);
        assert_eq!(m[0].params, 2);
        assert_eq!(m[0].bool_params, 1);
        assert_eq!(m[0].cyclomatic, 1);
        assert_eq!(m[0].cognitive, 0);
        assert_eq!(m[0].max_nesting, 0);
    }

    // Example from the SonarSource white paper (sumOfPrimes), translated to Rust:
    // cognitive complexity 7, cyclomatic 4.
    #[test]
    fn sonar_sum_of_primes() {
        let src = r#"
fn sum_of_primes(max: i32) -> i32 {
    let mut total = 0;
    'out: for i in 1..=max {
        for j in 2..i {
            if i % j == 0 {
                continue 'out;
            }
        }
        total += i;
    }
    total
}
"#;
        let m = metrics(src);
        assert_eq!(m[0].cognitive, 7);
        assert_eq!(m[0].cyclomatic, 4);
        assert_eq!(m[0].max_nesting, 3);
    }

    // `getWords` from the white paper: a switch counts once, cognitive 1.
    #[test]
    fn sonar_get_words() {
        let src = r#"
fn get_words(number: i32) -> &'static str {
    match number {
        1 => "one",
        2 => "a couple",
        3 => "a few",
        _ => "lots",
    }
}
"#;
        let m = metrics(src);
        assert_eq!(m[0].cognitive, 1);
        assert_eq!(m[0].cyclomatic, 4);
    }

    #[test]
    fn boolean_sequences() {
        let m = metrics("fn f(a: bool, b: bool, c: bool) -> bool { if a && b && c { true } else if a || b && c { false } else { a } }");
        // if:+1, `a && b && c`:+1, else if:+1, `a || b && c`:+2, else:+1
        assert_eq!(m[0].cognitive, 6);
    }

    #[test]
    fn nested_items_are_separate() {
        let m = metrics("fn outer() { fn inner(x: bool) { if x {} } if true {} }");
        let outer = m.iter().find(|f| f.name == "outer").unwrap();
        let inner = m.iter().find(|f| f.name == "inner").unwrap();
        assert_eq!(outer.cognitive, 1);
        assert_eq!(inner.cognitive, 1);
        assert_eq!(inner.qual_name, "outer::inner");
    }

    #[test]
    fn methods_get_type_scope_and_skip_self() {
        let m = metrics("struct S; impl S { fn m(&self, x: u8) {} }\ntrait T { fn d(&self) {} }\nimpl T for S { fn d(&self) {} }");
        assert_eq!(m[0].qual_name, "S::m");
        assert_eq!(m[0].params, 1);
        assert!(!m[0].is_trait_impl);
        assert!(m[1].is_trait_impl);
        assert!(m[2].is_trait_impl);
    }

    #[test]
    fn code_inside_macros_is_counted() {
        let m = metrics("fn f(x: Option<i32>) { println!(\"{}\", if x.is_some() { 1 } else { 2 }); }");
        assert_eq!(m[0].cognitive, 2);
    }
}
