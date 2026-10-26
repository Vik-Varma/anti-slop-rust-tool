use std::path::{Path, PathBuf};

use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

use crate::checks;
use crate::context::{FileKind, Scope};
use crate::finding::Finding;
use crate::metrics::{self, FunctionMetrics};
use crate::rules;
use crate::suppress::Suppressions;

/// Everything learned from one file: findings, and the shape of every function in it.
#[derive(Debug, Default)]
pub struct FileReport {
    pub findings: Vec<Finding>,
    pub functions: Vec<FunctionMetrics>,
    /// Non-blank, non-comment lines in the whole file.
    pub code_lines: usize,
}

/// Parses `source` and runs every check over it. A file that doesn't parse yields an error,
/// never a partial report, so a caller can't mistake "unparsed" for "clean".
pub fn analyze_source(path: &Path, source: &str) -> Result<FileReport, syn::Error> {
    let file = syn::parse_file(source)?;
    let lines: Vec<&str> = source.lines().collect();
    let kind = FileKind::of(path);
    let mut analyzer = Analyzer {
        path: path.to_path_buf(),
        kind,
        lines: &lines,
        suppressions: Suppressions::parse(&lines),
        scopes: vec![Scope {
            test: !kind.ships(),
            ..Scope::default()
        }],
        self_types: Vec::new(),
        report: FileReport {
            code_lines: metrics::code_lines(&lines, 1, lines.len()),
            ..FileReport::default()
        },
    };
    checks::file_level(&mut analyzer, &file);
    analyzer.visit_file(&file);
    let mut report = analyzer.report;
    report
        .findings
        .sort_by(|a, b| (a.line, a.column, a.rule).cmp(&(b.line, b.column, b.rule)));
    Ok(report)
}

pub struct Analyzer<'a> {
    pub path: PathBuf,
    pub kind: FileKind,
    pub lines: &'a [&'a str],
    suppressions: Suppressions,
    scopes: Vec<Scope>,
    self_types: Vec<String>,
    pub report: FileReport,
}

impl Analyzer<'_> {
    pub fn scope(&self) -> &Scope {
        // The root scope is pushed at construction and never popped.
        &self.scopes[self.scopes.len() - 1]
    }

    /// Records a finding for `rule` at `span`, unless a comment suppresses it there.
    pub fn emit(&mut self, rule: &'static str, span: Span, message: impl Into<String>) {
        let start = span.start();
        if self.suppressions.allows(rule, start.line) {
            return;
        }
        let severity = rules::get(rule)
            .map(|r| r.severity)
            .unwrap_or(crate::Severity::Warning);
        self.report.findings.push(Finding {
            rule,
            severity,
            path: self.path.clone(),
            line: start.line,
            column: start.column + 1,
            message: message.into(),
            function: self.scope().function.clone(),
        });
    }

    fn with_scope(&mut self, scope: Scope, f: impl FnOnce(&mut Self)) {
        self.scopes.push(scope);
        f(self);
        self.scopes.pop();
    }

    fn function_scope(&self, attrs: &[syn::Attribute], sig: &syn::Signature) -> Scope {
        let name = match self.self_types.last() {
            Some(ty) => format!("{ty}::{}", sig.ident),
            None => sig.ident.to_string(),
        };
        let parent = self.scope();
        Scope {
            test: parent.test || is_test_item(attrs),
            in_async: sig.asyncness.is_some(),
            in_drop: false,
            in_const: sig.constness.is_some(),
            function: Some(name),
        }
    }

    fn visit_function(
        &mut self,
        attrs: &[syn::Attribute],
        sig: &syn::Signature,
        block: &syn::Block,
        scope: Scope,
    ) {
        let start = sig.span().start().line;
        let end = block.span().end().line;
        let test = scope.test;
        let name = scope.function.clone().unwrap_or_default();
        let m = metrics::measure(name, sig, block, start, end, self.lines, test);
        self.report.functions.push(m);
        self.with_scope(scope, |a| {
            checks::function(a, attrs, sig, block);
            a.visit_block(block);
        });
    }
}

/// `#[test]`, `#[tokio::test]`, `#[bench]` or a `#[cfg(...)]` that only holds under `test`.
pub fn is_test_item(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let path = attr.path();
        let last = path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if path.is_ident("cfg") {
            return attr
                .parse_args::<syn::Meta>()
                .is_ok_and(|meta| cfg_only_under_test(&meta));
        }
        last == "test" || last == "bench"
    })
}

fn cfg_only_under_test(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::Path(p) => p.is_ident("test"),
        syn::Meta::List(list) => {
            let Ok(items) = list.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            ) else {
                return false;
            };
            if list.path.is_ident("all") {
                items.iter().any(cfg_only_under_test)
            } else if list.path.is_ident("any") {
                !items.is_empty() && items.iter().all(cfg_only_under_test)
            } else {
                false
            }
        }
        syn::Meta::NameValue(_) => false,
    }
}

fn type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(p) => p
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default(),
        syn::Type::Reference(r) => type_name(&r.elem),
        other => quote::quote!(#other).to_string().replace(' ', ""),
    }
}

impl<'ast> Visit<'ast> for Analyzer<'_> {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        let parent = self.scope().clone();
        let scope = Scope {
            test: parent.test || is_test_item(&node.attrs),
            ..parent
        };
        self.with_scope(scope, |a| visit::visit_item_mod(a, node));
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let scope = self.function_scope(&node.attrs, &node.sig);
        self.visit_function(&node.attrs, &node.sig, &node.block, scope);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        checks::item_impl(self, node);
        let parent = self.scope().clone();
        let is_drop = node
            .trait_
            .as_ref()
            .is_some_and(|(_, path, _)| path.segments.last().is_some_and(|s| s.ident == "Drop"));
        self.self_types.push(type_name(&node.self_ty));
        let scope = Scope {
            test: parent.test || is_test_item(&node.attrs),
            ..parent
        };
        self.with_scope(scope, |a| {
            for item in &node.items {
                match item {
                    syn::ImplItem::Fn(f) => {
                        let mut s = a.function_scope(&f.attrs, &f.sig);
                        s.in_drop = is_drop && f.sig.ident == "drop";
                        a.visit_function(&f.attrs, &f.sig, &f.block, s);
                    }
                    other => a.visit_impl_item(other),
                }
            }
        });
        self.self_types.pop();
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.self_types.push(node.ident.to_string());
        visit::visit_item_trait(self, node);
        self.self_types.pop();
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        if let Some(block) = &node.default {
            let scope = self.function_scope(&node.attrs, &node.sig);
            self.visit_function(&node.attrs, &node.sig, block, scope);
        }
    }

    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        let scope = Scope {
            in_const: true,
            ..self.scope().clone()
        };
        self.with_scope(scope, |a| visit::visit_item_const(a, node));
    }

    fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
        checks::item_static(self, node);
        let scope = Scope {
            in_const: true,
            ..self.scope().clone()
        };
        self.with_scope(scope, |a| visit::visit_item_static(a, node));
    }

    fn visit_expr_async(&mut self, node: &'ast syn::ExprAsync) {
        let scope = Scope {
            in_async: true,
            ..self.scope().clone()
        };
        self.with_scope(scope, |a| {
            checks::async_block(a, &node.block);
            visit::visit_expr_async(a, node);
        });
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        // A sync closure inside async code runs wherever it's called, often `spawn_blocking`.
        let scope = Scope {
            in_async: node.asyncness.is_some(),
            ..self.scope().clone()
        };
        self.with_scope(scope, |a| visit::visit_expr_closure(a, node));
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        checks::method_call(self, node);
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        checks::call(self, node);
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        checks::binary(self, node);
        visit::visit_expr_binary(self, node);
    }

    fn visit_expr_cast(&mut self, node: &'ast syn::ExprCast) {
        checks::cast(self, node);
        visit::visit_expr_cast(self, node);
    }

    fn visit_expr_unsafe(&mut self, node: &'ast syn::ExprUnsafe) {
        checks::unsafe_block(self, node);
        visit::visit_expr_unsafe(self, node);
    }

    fn visit_stmt(&mut self, node: &'ast syn::Stmt) {
        checks::stmt(self, node);
        visit::visit_stmt(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        checks::macro_call(self, node);
        // Format-style and assert-style macros take expressions; check inside them too.
        if let Some(args) = checks::macro_args(node) {
            for arg in &args {
                self.visit_expr(arg);
            }
        }
        visit::visit_macro(self, node);
    }
}
