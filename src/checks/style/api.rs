//! Signatures and type definitions: `wrapper-reference-param`, `bool-parameter`,
//! `into-parameter-in-application`, `rc-arc`, `tuple-fields`, `lifetime-name`,
//! `must-use-missing`, `non-exhaustive-missing`.

use proc_macro2::Span;
use syn::spanned::Spanned;

use super::{has_attr, is_pub, last_ident, text};
use crate::analyzer::Analyzer;
use crate::context::FileKind;

/// A function or method as the style rules see it.
pub struct FnInfo<'a> {
    pub attrs: &'a [syn::Attribute],
    /// Part of the public API: `pub` itself, or an item of a `pub` trait.
    pub is_pub: bool,
    pub sig: &'a syn::Signature,
    pub body: Option<&'a syn::Block>,
    /// The self type, for a method of an inherent impl.
    pub inherent: Option<&'a str>,
}

/// `&Vec<T>` and friends, with what the guide asks for instead.
const WRAPPERS: &[(&str, &str)] = &[
    ("Vec", "&[T]"),
    ("String", "&str"),
    ("Box", "&T"),
    ("Option", "Option<&T>"),
];

const PRIMITIVES: &[&str] = &[
    "bool", "char", "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128",
    "isize", "f32", "f64",
];

/// Verbs whose result is commonly and correctly ignored, so the function isn't a `#[must_use]`
/// candidate even when its signature looks pure.
const IGNORABLE_VERBS: &[&str] = &[
    "insert", "remove", "push", "pop", "set", "send", "write", "log", "print", "emit", "register",
    "add", "update", "clear", "reset", "notify", "record", "store", "save", "put", "try",
];

fn is_trait_bound(b: &syn::TypeParamBound, name: &str) -> bool {
    matches!(b, syn::TypeParamBound::Trait(t) if t.path.segments.last().is_some_and(|s| s.ident == name))
}

pub fn signature(a: &mut Analyzer, f: &FnInfo) {
    let comments = a.comments;
    let sig = f.sig;
    fn_lifetimes(a, sig);
    for input in &sig.inputs {
        let syn::FnArg::Typed(pt) = input else {
            continue;
        };
        if let syn::Type::Reference(r) = pt.ty.as_ref()
            && r.mutability.is_none()
            && let Some(id) = last_ident(&r.elem)
            && let Some((wrapper, better)) = WRAPPERS.iter().find(|(w, _)| id == w)
        {
            a.emit(
                "wrapper-reference-param",
                pt.ty.span(),
                format!("`&{wrapper}<..>` parameter; `{better}` accepts the same callers and more"),
            );
        }
        if f.is_pub && text(&pt.ty) == "bool" {
            let line = pt.pat.span().start().line;
            if !comments.get(line - 1).is_some_and(|l| l.has_comment()) {
                a.emit(
                    "bool-parameter",
                    pt.pat.span(),
                    format!("`{}: bool` on a public function; an enum names what each value means (or comment the parameter)", text(&pt.pat)),
                );
            }
        }
        if a.kind == FileKind::Binary
            && let syn::Type::ImplTrait(it) = pt.ty.as_ref()
            && it.bounds.iter().any(|b| is_trait_bound(b, "Into"))
        {
            into_param(a, pt.ty.span());
        }
        rc_arc(a, &pt.ty);
    }
    if a.kind == FileKind::Binary {
        for p in &sig.generics.params {
            if let syn::GenericParam::Type(t) = p
                && t.bounds.iter().any(|b| is_trait_bound(b, "Into"))
            {
                into_param(a, t.ident.span());
            }
        }
        if let Some(w) = &sig.generics.where_clause {
            for pred in &w.predicates {
                if let syn::WherePredicate::Type(t) = pred
                    && t.bounds.iter().any(|b| is_trait_bound(b, "Into"))
                {
                    into_param(a, t.bounded_ty.span());
                }
            }
        }
    }
    if let syn::ReturnType::Type(_, ty) = &sig.output {
        rc_arc(a, ty);
        if f.is_pub
            && let syn::Type::Tuple(t) = ty.as_ref()
            && t.elems.len() >= 2
        {
            a.emit(
                "tuple-fields",
                ty.span(),
                format!("a public function returning a {}-tuple; a struct with named fields says what each part is", t.elems.len()),
            );
        }
    }
    must_use(a, f);
}

fn into_param(a: &mut Analyzer, span: Span) {
    a.emit(
        "into-parameter-in-application",
        span,
        "`impl Into<T>` parameter in application code; take `T` and convert at the call site",
    );
}

/// More than two single-letter lifetimes on one function.
fn fn_lifetimes(a: &mut Analyzer, sig: &syn::Signature) {
    let mut n = 0;
    for p in &sig.generics.params {
        if let syn::GenericParam::Lifetime(l) = p
            && is_single_letter(&l.lifetime)
        {
            n += 1;
            if n == 3 {
                a.emit(
                    "lifetime-name",
                    l.lifetime.span(),
                    format!("`{}` has more than two single-letter lifetimes; name them after what they borrow", sig.ident),
                );
            }
        }
    }
}

fn is_single_letter(l: &syn::Lifetime) -> bool {
    let name = l.ident.to_string();
    name.len() == 1 && name != "_"
}

/// Single-letter lifetimes on a type definition ("especially type lifetimes").
pub fn type_lifetimes(a: &mut Analyzer, generics: &syn::Generics, item: &syn::Ident) {
    for p in &generics.params {
        if let syn::GenericParam::Lifetime(l) = p
            && is_single_letter(&l.lifetime)
        {
            a.emit(
                "lifetime-name",
                l.lifetime.span(),
                format!(
                    "lifetime `{}` on `{item}`; a type lifetime should say what it borrows",
                    l.lifetime
                ),
            );
        }
    }
}

/// Calls `f` on `ty` and every type nested in it (generic arguments, references, tuples).
pub fn walk_type<'a>(ty: &'a syn::Type, f: &mut impl FnMut(&'a syn::Type)) {
    f(ty);
    match ty {
        syn::Type::Path(p) => {
            for seg in &p.path.segments {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(t) = arg {
                            walk_type(t, f);
                        }
                    }
                }
            }
        }
        syn::Type::Reference(r) => walk_type(&r.elem, f),
        syn::Type::Tuple(t) => t.elems.iter().for_each(|e| walk_type(e, f)),
        syn::Type::Slice(s) => walk_type(&s.elem, f),
        syn::Type::Array(s) => walk_type(&s.elem, f),
        syn::Type::Ptr(p) => walk_type(&p.elem, f),
        syn::Type::Paren(p) => walk_type(&p.elem, f),
        _ => {}
    }
}

pub fn rc_arc(a: &mut Analyzer, ty: &syn::Type) {
    let mut found = Vec::new();
    walk_type(ty, &mut |t| {
        if let syn::Type::Path(p) = t
            && let Some(seg) = p.path.segments.last()
            && (seg.ident == "Rc" || seg.ident == "Arc")
            && matches!(seg.arguments, syn::PathArguments::AngleBracketed(_))
        {
            found.push((seg.ident.to_string(), seg.ident.span()));
        }
    });
    for (name, span) in found {
        a.emit(
            "rc-arc",
            span,
            format!("`{name}` shares ownership; the guide prefers unique ownership and borrows unless that is unreasonably complex"),
        );
    }
}

fn must_use(a: &mut Analyzer, f: &FnInfo) {
    if a.kind != FileKind::Library || !f.is_pub || has_attr(f.attrs, "must_use") {
        return;
    }
    let sig = f.sig;
    let name = sig.ident.to_string();
    if IGNORABLE_VERBS
        .iter()
        .any(|v| name == *v || name.starts_with(&format!("{v}_")))
    {
        return;
    }
    match sig.inputs.first() {
        Some(syn::FnArg::Receiver(r)) if r.reference.is_none() || r.mutability.is_some() => return,
        _ => {}
    }
    let mutates = sig.inputs.iter().any(|i| match i {
        syn::FnArg::Typed(p) => {
            matches!(p.ty.as_ref(), syn::Type::Reference(r) if r.mutability.is_some())
                || text(&p.ty).starts_with("implFnMut")
        }
        syn::FnArg::Receiver(_) => false,
    });
    if mutates {
        return;
    }
    let syn::ReturnType::Type(_, ty) = &sig.output else {
        return;
    };
    let query_like = match ty.as_ref() {
        syn::Type::Reference(_) => true,
        syn::Type::Path(p) => p.path.segments.last().is_some_and(|s| {
            let n = s.ident.to_string();
            n == "Option" || PRIMITIVES.contains(&n.as_str())
        }),
        _ => false,
    };
    if query_like {
        a.emit(
            "must-use-missing",
            sig.ident.span(),
            format!("`{name}` looks pure (reads `self`, returns `{}`); mark it `#[must_use]` if ignoring the result is a mistake", text(ty)),
        );
    }
}

/// `#[must_use]` on an error type.
fn error_type_must_use(
    a: &mut Analyzer,
    attrs: &[syn::Attribute],
    vis: &syn::Visibility,
    ident: &syn::Ident,
) {
    if a.kind == FileKind::Library
        && is_pub(vis)
        && ident.to_string().ends_with("Error")
        && !has_attr(attrs, "must_use")
    {
        a.emit(
            "must-use-missing",
            ident.span(),
            format!(
                "`{ident}` is an error type; mark it `#[must_use]` so a dropped error is a warning"
            ),
        );
    }
}

pub fn struct_item(a: &mut Analyzer, node: &syn::ItemStruct) {
    type_lifetimes(a, &node.generics, &node.ident);
    error_type_must_use(a, &node.attrs, &node.vis, &node.ident);
    match &node.fields {
        syn::Fields::Unnamed(u) if u.unnamed.len() >= 2 => a.emit(
            "tuple-fields",
            node.ident.span(),
            format!(
                "`{}` is a tuple struct with {} fields; name them (a one-field newtype is fine)",
                node.ident,
                u.unnamed.len()
            ),
        ),
        syn::Fields::Named(n) => {
            for field in &n.named {
                if let syn::Type::Tuple(t) = &field.ty
                    && t.elems.len() >= 2
                {
                    a.emit(
                        "tuple-fields",
                        field.ty.span(),
                        "a tuple-typed field; a struct with named fields says what each part is",
                    );
                }
            }
            if a.kind == FileKind::Library
                && is_pub(&node.vis)
                && !has_attr(&node.attrs, "non_exhaustive")
                && n.named.iter().all(|f| is_pub(&f.vis))
            {
                a.emit(
                    "non-exhaustive-missing",
                    node.ident.span(),
                    format!("`{}` is public with all fields public; without `#[non_exhaustive]`, adding a field breaks every struct literal", node.ident),
                );
            }
        }
        _ => {}
    }
    for field in node.fields.iter() {
        rc_arc(a, &field.ty);
    }
}

pub fn enum_item(a: &mut Analyzer, node: &syn::ItemEnum) {
    type_lifetimes(a, &node.generics, &node.ident);
    error_type_must_use(a, &node.attrs, &node.vis, &node.ident);
    // Two states (`Visible`/`Hidden`) are the guide's example of an enum that never changes.
    if a.kind == FileKind::Library
        && is_pub(&node.vis)
        && node.variants.len() >= 3
        && !has_attr(&node.attrs, "non_exhaustive")
    {
        a.emit(
            "non-exhaustive-missing",
            node.ident.span(),
            format!("`{}` is a public enum; without `#[non_exhaustive]`, adding a variant breaks every exhaustive `match` downstream", node.ident),
        );
    }
    for v in &node.variants {
        for field in v.fields.iter() {
            rc_arc(a, &field.ty);
        }
    }
}
