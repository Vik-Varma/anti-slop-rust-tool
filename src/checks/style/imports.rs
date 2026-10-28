//! Imports and paths: `glob-import`, `use-mixes-modules`, `unqualified-function-call`,
//! `qualified-type-name`.

use syn::spanned::Spanned;

use super::{is_camel, is_snake};
use crate::analyzer::Analyzer;
use crate::checks::call_path;

pub fn item_use(a: &mut Analyzer, node: &syn::ItemUse) {
    let is_pub = !matches!(node.vis, syn::Visibility::Inherited);
    let mut prefix = Vec::new();
    let mut globs = Vec::new();
    let mut mixes = false;
    collect(&node.tree, &mut prefix, &mut globs, &mut mixes);
    if mixes {
        a.emit(
            "use-mixes-modules",
            node.use_token.span,
            "one `use` imports from more than one module; split it so nothing inside `{..}` has a `::`",
        );
    }
    let scope = a.scope();
    let (in_fn, inline_mod) = (scope.function.is_some(), scope.inline_mod);
    let non_leaf = a.style_file.non_leaf;
    for (segments, span) in globs {
        let last = segments.last().map(String::as_str).unwrap_or_default();
        // The guide's own exceptions: `use super::*` in an inline module, `use Enum::*` in a
        // function about that enum, `pub use leaf::*` from a non-leaf module, and preludes.
        if (segments == ["super"] && inline_mod)
            || (is_pub && non_leaf)
            || (in_fn && is_camel(last))
            || segments.iter().any(|s| s == "prelude")
        {
            continue;
        }
        a.emit(
            "glob-import",
            span,
            format!(
                "`use {}::*` imports everything; name what is used",
                segments.join("::")
            ),
        );
    }
}

/// Walks a `use` tree: `prefix` is the path so far, globs get their path and span, and a group
/// with a `::` path inside it sets `mixes`.
fn collect(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    globs: &mut Vec<(Vec<String>, proc_macro2::Span)>,
    mixes: &mut bool,
) {
    match tree {
        syn::UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            collect(&p.tree, prefix, globs, mixes);
            prefix.pop();
        }
        syn::UseTree::Glob(g) => globs.push((prefix.clone(), g.star_token.span)),
        syn::UseTree::Group(g) => {
            if g.items.iter().any(|t| matches!(t, syn::UseTree::Path(_))) {
                *mixes = true;
            }
            for t in &g.items {
                collect(t, prefix, globs, mixes);
            }
        }
        syn::UseTree::Name(_) | syn::UseTree::Rename(_) => {}
    }
}

/// A bare call to a function the file imported by name: `transmute(x)` after `use mem::transmute`.
pub fn call(a: &mut Analyzer, node: &syn::ExprCall) {
    let Some(path) = call_path(node) else { return };
    if path.segments.len() != 1 || path.leading_colon.is_some() {
        return;
    }
    let name = path.segments[0].ident.to_string();
    if a.style_file.imported_functions.contains(&name) {
        a.emit(
            "unqualified-function-call",
            path.span(),
            format!(
                "`{name}()` is imported by name; import its module and call `module::{name}()`"
            ),
        );
    }
}

/// Names the guide calls common enough to need their module (`io::Error`, `fmt::Result`).
const COMMON_NAMES: &[&str] = &["Error", "Result"];

/// A type or trait written with its module path (`regex::Regex`, `fmt::Display`).
pub fn path(a: &mut Analyzer, path: &syn::Path) {
    let names: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    for i in 0..names.len().saturating_sub(1) {
        let (module, ty) = (&names[i], &names[i + 1]);
        if is_snake(module) && is_camel(ty) && !COMMON_NAMES.contains(&ty.as_str()) {
            a.emit(
                "qualified-type-name",
                path.segments[i + 1].ident.span(),
                format!("`{module}::{ty}` is written with its module path; import `{ty}` and use it unqualified"),
            );
            return;
        }
    }
}
