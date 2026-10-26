use syn::spanned::Spanned;

use crate::analyzer::Analyzer;

pub fn file_attributes(a: &mut Analyzer, attrs: &[syn::Attribute]) {
    // Only crate roots that downstream users build; an example or test breaking hurts nobody else.
    if !a.kind.ships() {
        return;
    }
    for attr in attrs {
        if !matches!(attr.style, syn::AttrStyle::Inner(_)) {
            continue;
        }
        let level = if attr.path().is_ident("deny") {
            "deny"
        } else if attr.path().is_ident("forbid") {
            "forbid"
        } else {
            continue;
        };
        // `Meta`, not `Path`: `#![deny(warnings, reason = "..")]` is still a deny.
        let Ok(list) = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        ) else {
            continue;
        };
        if list
            .iter()
            .any(|m| matches!(m, syn::Meta::Path(p) if p.is_ident("warnings")))
        {
            a.emit(
                "deny-warnings",
                attr.span(),
                format!("`#![{level}(warnings)]` turns every lint a future compiler adds into a build failure for your users; set `-D warnings` in CI instead"),
            );
        }
    }
}
