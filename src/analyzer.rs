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
    analyze_source_as(path, source, FileKind::of(path))
}

/// As [`analyze_source`], with the file's kind decided by the caller (from the crate layout).
pub fn analyze_source_as(
    path: &Path,
    source: &str,
    kind: FileKind,
) -> Result<FileReport, syn::Error> {
    analyze_source_in(path, source, kind, Options::default())
}

/// What the caller knows about a file beyond its text and kind.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Build profiles that set `panic = "abort"` for the file (`crate::manifest`), by name.
    /// `analyze_source` doesn't look them up, since its path needn't exist.
    pub abort_profiles: Vec<String>,
    /// Also run the style rules (a published guide's preferences, `Severity::Style`).
    pub style: bool,
}

/// As [`analyze_source_as`], with [`Options`].
pub fn analyze_source_in(
    path: &Path,
    source: &str,
    kind: FileKind,
    options: Options,
) -> Result<FileReport, syn::Error> {
    let source = &blank_frontmatter(source);
    let file = syn::parse_file(source)?;
    let lines: Vec<&str> = source.lines().collect();
    let comments = crate::comments::lines(source);
    let mut analyzer = Analyzer {
        path: path.to_path_buf(),
        kind,
        abort_profiles: options.abort_profiles,
        style: options.style,
        lines: &lines,
        suppressions: Suppressions::parse(&comments),
        comments: &comments,
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

/// Cargo scripts open with a `---` manifest block that `syn` doesn't parse. Blanks it, line for
/// line, so every reported line number still matches the file. Follows the frontmatter syntax: an
/// optional shebang, then blank lines, then an opening fence of three or more dashes (with an
/// optional infostring), closed by a line of the same number of dashes.
pub(crate) fn blank_frontmatter(source: &str) -> std::borrow::Cow<'_, str> {
    let body = source.strip_prefix('\u{feff}').unwrap_or(source);
    let bom = source.len() - body.len();
    let mut lines: Vec<&str> = body.split('\n').collect();
    let mut first = usize::from(
        lines
            .first()
            .is_some_and(|l| l.starts_with("#!") && !l.starts_with("#![")),
    );
    while lines.get(first).is_some_and(|l| l.trim().is_empty()) {
        first += 1;
    }
    let Some(open) = lines.get(first) else {
        return source.into();
    };
    let dashes = open.chars().take_while(|c| *c == '-').count();
    if dashes < 3 {
        return source.into();
    }
    let fence = &open[..dashes];
    let Some(close) = lines
        .iter()
        .skip(first + 1)
        .position(|l| l.trim_end() == fence)
    else {
        return source.into();
    };
    for line in &mut lines[first..=first + 1 + close] {
        *line = "";
    }
    let mut out = String::with_capacity(source.len());
    out.push_str(&source[..bom]);
    out.push_str(&lines.join("\n"));
    out.into()
}

pub struct Analyzer<'a> {
    pub path: PathBuf,
    pub kind: FileKind,
    /// Build profiles that set `panic = "abort"` for this file, by name (`release`).
    pub abort_profiles: Vec<String>,
    /// Whether the style rules run (`Options::style`).
    pub style: bool,
    pub lines: &'a [&'a str],
    /// The comments on each line, from `crate::comments`.
    pub comments: &'a [crate::comments::Line],
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
        let Some(entry) = rules::get(rule) else {
            debug_assert!(
                false,
                "check emitted `{rule}`, which has no catalogue entry"
            );
            return;
        };
        let severity = entry.severity;
        self.report.findings.push(Finding {
            rule,
            severity,
            path: self.path.clone(),
            line: start.line,
            column: start.column + 1,
            message: message.into(),
            function: self.scope().function.clone(),
            function_index: self.scope().function_index,
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
            // A `const fn` called at run time is ordinary code, so its panics are real.
            in_const: false,
            uninit_array_target: false,
            returns_result: returns_result(&sig.output),
            function: Some(name),
            function_index: None,
        }
    }

    fn visit_function(&mut self, sig: &syn::Signature, block: &syn::Block, mut scope: Scope) {
        let start = sig.span().start().line;
        let end = block.span().end().line;
        let test = scope.test;
        let name = scope.function.clone().unwrap_or_default();
        let m = metrics::measure(name, sig, block, start, end, self.lines, test);
        scope.function_index = Some(self.report.functions.len());
        self.report.functions.push(m);
        // A `fn` nested in a method belongs to no impl.
        let outer_types = std::mem::take(&mut self.self_types);
        self.with_scope(scope, |a| {
            checks::function(a, sig, block);
            a.visit_block(block);
        });
        self.self_types = outer_types;
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

fn returns_result(output: &syn::ReturnType) -> bool {
    let syn::ReturnType::Type(_, ty) = output else {
        return false;
    };
    matches!(ty.as_ref(), syn::Type::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == "Result"))
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
        syn::Type::TraitObject(t) => {
            let bounds: Vec<String> = t
                .bounds
                .iter()
                .map(|b| quote::quote!(#b).to_string().replace(' ', ""))
                .collect();
            format!("dyn {}", bounds.join(" + "))
        }
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
        self.visit_function(&node.sig, &node.block, scope);
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
                        a.visit_function(&f.sig, &f.block, s);
                    }
                    other => a.visit_impl_item(other),
                }
            }
        });
        self.self_types.pop();
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.self_types.push(node.ident.to_string());
        let parent = self.scope().clone();
        let scope = Scope {
            test: parent.test || is_test_item(&node.attrs),
            ..parent
        };
        self.with_scope(scope, |a| visit::visit_item_trait(a, node));
        self.self_types.pop();
    }

    // Associated and inline consts are evaluated at compile time, where a panic is a build error.
    fn visit_impl_item_const(&mut self, node: &'ast syn::ImplItemConst) {
        let scope = Scope {
            in_const: true,
            ..self.scope().clone()
        };
        self.with_scope(scope, |a| visit::visit_impl_item_const(a, node));
    }

    fn visit_trait_item_const(&mut self, node: &'ast syn::TraitItemConst) {
        let scope = Scope {
            in_const: true,
            ..self.scope().clone()
        };
        self.with_scope(scope, |a| visit::visit_trait_item_const(a, node));
    }

    fn visit_expr_const(&mut self, node: &'ast syn::ExprConst) {
        let scope = Scope {
            in_const: true,
            ..self.scope().clone()
        };
        self.with_scope(scope, |a| visit::visit_expr_const(a, node));
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        if let Some(block) = &node.default {
            let scope = self.function_scope(&node.attrs, &node.sig);
            self.visit_function(&node.sig, block, scope);
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
            in_const: false,
            returns_result: false,
            ..self.scope().clone()
        };
        self.with_scope(scope, |a| {
            checks::async_block(a, &node.block);
            visit::visit_expr_async(a, node);
        });
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        // A sync closure inside async code runs wherever it's called, often `spawn_blocking`.
        // A closure runs when it is called, at run time, even inside a `static` initialiser
        // (`LazyLock::new(|| ..)`).
        let scope = Scope {
            in_async: node.asyncness.is_some(),
            in_const: false,
            returns_result: returns_result(&node.output),
            ..self.scope().clone()
        };
        self.with_scope(scope, |a| {
            if node.asyncness.is_some()
                && let syn::Expr::Block(b) = node.body.as_ref()
            {
                checks::async_block(a, &b.block);
            }
            visit::visit_expr_closure(a, node)
        });
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        // `if !thread::panicking() { panic!(..) }` in a destructor is the correct way to do it.
        if self.scope().in_drop
            && let Some(guarded_then) = checks::panicking_guard(&node.cond)
        {
            // The branch that only runs when the thread isn't already panicking may panic.
            self.visit_expr(&node.cond);
            let safe = Scope {
                in_drop: false,
                ..self.scope().clone()
            };
            let unsafe_branch = self.scope().clone();
            let (then_scope, else_scope) = if guarded_then {
                (safe, unsafe_branch)
            } else {
                (unsafe_branch, safe)
            };
            self.with_scope(then_scope, |a| a.visit_block(&node.then_branch));
            if let Some((_, e)) = &node.else_branch {
                self.with_scope(else_scope, |a| a.visit_expr(e));
            }
            return;
        }
        visit::visit_expr_if(self, node);
    }

    fn visit_block(&mut self, node: &'ast syn::Block) {
        if !self.scope().in_drop {
            return visit::visit_block(self, node);
        }
        // `if thread::panicking() { return; }` makes the rest of the destructor safe to panic in.
        let mut guarded = false;
        for stmt in &node.stmts {
            if guarded {
                let scope = Scope {
                    in_drop: false,
                    ..self.scope().clone()
                };
                self.with_scope(scope, |a| a.visit_stmt(stmt));
                continue;
            }
            self.visit_stmt(stmt);
            guarded = checks::returns_if_panicking(stmt);
        }
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        if let syn::Pat::Type(t) = &node.pat
            && checks::is_maybe_uninit_array(&t.ty)
        {
            let scope = Scope {
                uninit_array_target: true,
                ..self.scope().clone()
            };
            return self.with_scope(scope, |a| visit::visit_local(a, node));
        }
        visit::visit_local(self, node);
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
