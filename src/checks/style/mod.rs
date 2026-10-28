//! Rules from Google's Rust style guide: preferences, not reproduced failures. They run only when
//! asked (`Options::style`), report at `Severity::Style`, and each entry in `crate::rules::style`
//! says which of the guide's own exceptions it honours and which need a suppression.

mod api;
mod docs;
mod exprs;
mod imports;
mod items;

use crate::analyzer::Analyzer;

/// What the file-level pass learned, for the expression hooks that run after it.
#[derive(Clone, Debug, Default)]
pub struct StyleFile {
    /// Free functions imported by name at the top level and not defined in the file.
    pub imported_functions: Vec<String>,
    /// The file declares out-of-line submodules (`mod x;`), so it is a non-leaf module.
    pub non_leaf: bool,
}

/// Whether the style rules apply here: asked for, and shipping code.
fn on(a: &Analyzer) -> bool {
    a.style && !a.scope().test
}

pub fn file(a: &mut Analyzer, file: &syn::File) {
    if !on(a) {
        return;
    }
    let index = items::FileIndex::build(file);
    a.style_file = StyleFile {
        imported_functions: index.imported_functions.clone(),
        non_leaf: index.non_leaf,
    };
    items::file(a, file, &index);
}

pub fn item_use(a: &mut Analyzer, node: &syn::ItemUse) {
    if on(a) {
        imports::item_use(a, node);
    }
}

pub fn path(a: &mut Analyzer, node: &syn::Path) {
    if on(a) {
        imports::path(a, node);
    }
}

pub fn call(a: &mut Analyzer, node: &syn::ExprCall) {
    if on(a) {
        imports::call(a, node);
        exprs::call(a, node);
    }
}

pub fn method_call(a: &mut Analyzer, node: &syn::ExprMethodCall) {
    if on(a) {
        exprs::method_call(a, node);
    }
}

pub fn macro_call(a: &mut Analyzer, mac: &syn::Macro) {
    if on(a) {
        exprs::macro_call(a, mac);
    }
}

pub fn expr_if(a: &mut Analyzer, node: &syn::ExprIf) {
    if on(a) {
        exprs::expr_if(a, node);
    }
}

pub fn expr_match(a: &mut Analyzer, node: &syn::ExprMatch) {
    if on(a) {
        exprs::expr_match(a, node);
    }
}

pub fn for_loop(a: &mut Analyzer, node: &syn::ExprForLoop) {
    if on(a) {
        exprs::for_loop(a, node);
    }
}

pub fn local(a: &mut Analyzer, node: &syn::Local) {
    if on(a) {
        exprs::local(a, node);
    }
}

pub fn binary(a: &mut Analyzer, node: &syn::ExprBinary) {
    if on(a) {
        exprs::binary(a, node);
    }
}

pub fn unsafe_block(a: &mut Analyzer, node: &syn::ExprUnsafe) {
    if on(a) {
        docs::unsafe_block(a, node);
    }
}

pub fn unsafe_impl(a: &mut Analyzer, node: &syn::ItemImpl) {
    if on(a) {
        docs::unsafe_impl(a, node);
    }
}

// Helpers shared by the style modules.

fn is_pub(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

/// `Regex`, `ErrorKind`: an upper-case start with a lower-case letter somewhere after it, which
/// separates type names from constants (`MAX`) and modules (`io`).
fn is_camel(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_uppercase()) && s.chars().any(|c| c.is_ascii_lowercase())
}

fn is_snake(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_lowercase())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// The tokens of a node as one string with no spaces, for comparing shapes.
fn text<T: quote::ToTokens>(t: &T) -> String {
    quote::quote!(#t).to_string().replace(' ', "")
}

/// `rustfmt::skip` for `#[rustfmt::skip]`.
fn attr_name(attr: &syn::Attribute) -> String {
    attr.path()
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn has_attr(attrs: &[syn::Attribute], name: &str) -> bool {
    attrs.iter().any(|a| attr_name(a) == name)
}

/// The text of `///` and `//!` comments (and `#[doc = ".."]`), one entry per line.
fn doc_lines(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter_map(|a| match &a.meta {
            syn::Meta::NameValue(nv) if nv.path.is_ident("doc") => match &nv.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) => Some(s.value()),
                _ => None,
            },
            _ => None,
        })
        .map(|l| l.strip_prefix(' ').unwrap_or(&l).to_string())
        .collect()
}

/// The last segment of a path type: `HashMap` for `std::collections::HashMap<K, V>`.
fn last_ident(ty: &syn::Type) -> Option<&syn::Ident> {
    match ty {
        syn::Type::Path(p) => p.path.segments.last().map(|s| &s.ident),
        syn::Type::Paren(p) => last_ident(&p.elem),
        _ => None,
    }
}

/// `HttpError` to `http_error`.
fn snake_case(name: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = name.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() {
            let boundary = i > 0
                && (chars[i - 1].is_ascii_lowercase()
                    || chars[i - 1].is_ascii_digit()
                    || chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase()));
            if boundary && !out.ends_with('_') {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(*c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(snake_case("HttpError"), "http_error");
        assert_eq!(snake_case("IOError"), "io_error");
        assert_eq!(snake_case("Config"), "config");
        assert!(is_camel("ErrorKind") && !is_camel("MAX") && !is_camel("io"));
        assert!(is_snake("read_dir") && !is_snake("Vec") && !is_snake("_"));
    }
}
