//! Documentation: `missing-docs`, `doc-comment-style`, `panic-undocumented`,
//! `safety-undocumented`, `safety-comment-prefix`.

use syn::spanned::Spanned;
use syn::visit::Visit;

use super::api::FnInfo;
use super::{doc_lines, has_attr, snake_case, text};
use crate::analyzer::Analyzer;
use crate::checks::{macro_name, unsafety};
use crate::context::FileKind;

/// Words a doc comment can add without saying anything the name doesn't.
const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "this", "that", "to", "of", "for", "is", "are", "it", "its", "with", "and",
    "or", "in", "on", "from", "as", "by", "into", "returns", "return", "gets", "get", "sets",
    "set", "creates", "create", "new", "value", "given", "self",
];

/// `missing-docs` and `doc-comment-style` for one item.
pub fn item(a: &mut Analyzer, attrs: &[syn::Attribute], ident: &syn::Ident, is_pub: bool) {
    let docs = doc_lines(attrs);
    if docs.is_empty() {
        if a.kind == FileKind::Library && is_pub && !has_attr(attrs, "doc") {
            a.emit(
                "missing-docs",
                ident.span(),
                format!("`{ident}` is public and has no doc comment"),
            );
        }
        return;
    }
    style(a, &docs, ident);
}

fn style(a: &mut Analyzer, docs: &[String], ident: &syn::Ident) {
    let Some(first) = docs.iter().find(|l| !l.trim().is_empty()) else {
        return;
    };
    let first = first.trim();
    let paragraph: Vec<&str> = docs
        .iter()
        .skip_while(|l| l.trim().is_empty())
        .take_while(|l| !l.trim().is_empty())
        .map(|l| l.trim())
        .collect();
    let paragraph = paragraph.join(" ");
    let prose =
        !first.starts_with(['#', '`', '[', '|', '-', '*', '!', '<']) && !first.starts_with("```");
    if prose && first.starts_with(|c: char| c.is_ascii_lowercase()) {
        a.emit(
            "doc-comment-style",
            ident.span(),
            format!("doc comment on `{ident}` starts in lower case; the guide asks for capitalisation and punctuation"),
        );
    }
    if prose && !paragraph.ends_with(['.', '?', '!', ':', ')', '`', ']']) {
        a.emit(
            "doc-comment-style",
            ident.span(),
            format!("doc comment on `{ident}` has no end punctuation"),
        );
    }
    let sentence = paragraph.split(". ").next().unwrap_or(&paragraph);
    let words: Vec<String> = sentence
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .filter(|w| !STOP_WORDS.contains(&w.as_str()))
        .collect();
    let name = snake_case(&ident.to_string());
    let parts: Vec<&str> = name.split('_').filter(|p| !p.is_empty()).collect();
    let restates = !words.is_empty()
        && words.iter().all(|w| {
            parts
                .iter()
                .any(|p| w.starts_with(p) || p.starts_with(w.as_str()))
        });
    if restates {
        a.emit(
            "doc-comment-style",
            ident.span(),
            format!("doc comment on `{ident}` restates its name (\"{sentence}\"); say what the code doesn't"),
        );
    }
}

/// Panicking operations that mention one of the function's parameters.
struct PanicFinder<'a> {
    params: &'a [String],
    found: Option<String>,
}

impl PanicFinder<'_> {
    fn mentions_param(&self, e: &syn::Expr) -> bool {
        let t = text(e);
        self.params.iter().any(|p| {
            t.split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .any(|w| w == p)
        })
    }
}

impl<'ast> Visit<'ast> for PanicFinder<'_> {
    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        let name = macro_name(m);
        let explicit = matches!(
            name.as_str(),
            "panic" | "unreachable" | "todo" | "unimplemented"
        );
        let asserting = matches!(name.as_str(), "assert" | "assert_eq" | "assert_ne");
        let on_param = self.params.iter().any(|p| {
            m.tokens
                .to_string()
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .any(|w| w == p)
        });
        if self.found.is_none() && (explicit || (asserting && on_param)) {
            self.found = Some(format!("{name}!"));
        }
        syn::visit::visit_macro(self, m);
    }

    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if self.found.is_none()
            && (m.method == "unwrap" || m.method == "expect")
            && self.mentions_param(&m.receiver)
        {
            self.found = Some(format!(".{}()", m.method));
        }
        syn::visit::visit_expr_method_call(self, m);
    }
}

/// `missing-docs`, `doc-comment-style`, `panic-undocumented` and `safety-undocumented` for a
/// function or method.
pub fn function(a: &mut Analyzer, f: &FnInfo) {
    item(a, f.attrs, &f.sig.ident, f.is_pub);
    if a.kind != FileKind::Library || !f.is_pub {
        return;
    }
    let docs = doc_lines(f.attrs).join("\n").to_ascii_lowercase();
    if f.sig.unsafety.is_some() && !docs.contains("safety") {
        a.emit(
            "safety-undocumented",
            f.sig.ident.span(),
            format!(
                "`unsafe fn {}` has no `# Safety` section saying what the caller must uphold",
                f.sig.ident
            ),
        );
    }
    let Some(body) = f.body else { return };
    if docs.contains("panic") {
        return;
    }
    let params: Vec<String> = f
        .sig
        .inputs
        .iter()
        .filter_map(|i| match i {
            syn::FnArg::Typed(p) => match p.pat.as_ref() {
                syn::Pat::Ident(id) => Some(id.ident.to_string()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    let mut finder = PanicFinder {
        params: &params,
        found: None,
    };
    finder.visit_block(body);
    if let Some(what) = finder.found {
        a.emit(
            "panic-undocumented",
            f.sig.ident.span(),
            format!(
                "`{}` can panic on its arguments (`{what}`); add a `# Panics` section",
                f.sig.ident
            ),
        );
    }
}

pub fn trait_item(a: &mut Analyzer, t: &syn::ItemTrait) {
    let is_pub = matches!(t.vis, syn::Visibility::Public(_));
    item(a, &t.attrs, &t.ident, is_pub);
    if a.kind == FileKind::Library
        && is_pub
        && t.unsafety.is_some()
        && !doc_lines(&t.attrs)
            .join("\n")
            .to_ascii_lowercase()
            .contains("safety")
    {
        a.emit(
            "safety-undocumented",
            t.ident.span(),
            format!("`unsafe trait {}` has no `# Safety` section saying what an implementor must uphold", t.ident),
        );
    }
}

fn prefix_check(a: &mut Analyzer, line: Option<&crate::comments::Line>, span: proc_macro2::Span) {
    if let Some(l) = line
        && !l.comment.to_ascii_uppercase().contains("SAFETY")
    {
        a.emit(
            "safety-comment-prefix",
            span,
            "the comment justifying this `unsafe` doesn't start with `SAFETY:`, the marker the guide and the standard library use",
        );
    }
}

pub fn unsafe_block(a: &mut Analyzer, node: &syn::ExprUnsafe) {
    let line = unsafety::block_justification(a, node);
    prefix_check(a, line, node.unsafe_token.span());
}

pub fn unsafe_impl(a: &mut Analyzer, node: &syn::ItemImpl) {
    let Some(token) = &node.unsafety else { return };
    let line = unsafety::impl_justification(a, node);
    prefix_check(a, line, token.span());
}
