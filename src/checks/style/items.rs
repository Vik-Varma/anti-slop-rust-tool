//! The item walk and the rules that need the whole file: `len-without-is-empty`, `getter-prefix`,
//! `constructor-name`, `from-impl-without-constructor`, `std-trait-as-method`,
//! `new-without-default`, `options-struct-default`, `trait-as-organization`,
//! `ext-trait-in-bounds`, `macro-definition`, `code-before-main`, `unstable-feature`,
//! `rustfmt-skip`, `api-in-non-leaf-module`, `file-named-after-item`.

use std::collections::{HashMap, HashSet};

use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::visit::Visit;

use super::api::{self, FnInfo};
use super::{attr_name, docs, is_pub, is_snake, last_ident, snake_case, text};
use crate::analyzer::{Analyzer, is_test_item};
use crate::metrics;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recv {
    None,
    Ref,
    RefMut,
    Value,
}

#[derive(Debug)]
pub struct Method {
    pub name: String,
    pub receiver: Recv,
    pub args: usize,
    pub is_pub: bool,
    /// The return type's tokens, `Self` or `Result<Self,E>`; empty for `()`.
    pub output: String,
    pub span: Span,
}

/// What the file defines and uses, gathered before any rule runs. Test modules are left out.
#[derive(Default)]
pub struct FileIndex {
    /// Inherent methods by self type.
    pub inherent: HashMap<String, Vec<Method>>,
    /// Traits implemented for each type, by the trait's last path segment.
    pub trait_impls: HashMap<String, Vec<String>>,
    /// `#[derive(..)]` names by type.
    pub derives: HashMap<String, Vec<String>>,
    /// Structs, enums and unions defined here.
    pub types: HashSet<String>,
    /// Public types at the top level, for file naming.
    pub top_pub_types: Vec<(String, Span)>,
    /// Traits defined here: name, `pub`, span.
    pub trait_defs: Vec<(String, bool, Span)>,
    /// `impl Trait for ..` counts by trait name.
    pub impl_counts: HashMap<String, usize>,
    /// `impl From<..> for T` sites.
    pub from_impls: Vec<(String, Span)>,
    /// Trait names used as bounds, `impl Trait` or `dyn Trait`, with where.
    pub bounds: Vec<(String, Span)>,
    pub local_fns: HashSet<String>,
    pub imported_functions: Vec<String>,
    pub non_leaf: bool,
    pub top_pub_fns: usize,
}

#[derive(Default)]
struct BoundFinder {
    found: Vec<(String, Span)>,
}

impl<'ast> Visit<'ast> for BoundFinder {
    fn visit_trait_bound(&mut self, b: &'ast syn::TraitBound) {
        if let Some(s) = b.path.segments.last() {
            self.found.push((s.ident.to_string(), s.ident.span()));
        }
        syn::visit::visit_trait_bound(self, b);
    }
}

fn receiver(sig: &syn::Signature) -> Recv {
    match sig.inputs.first() {
        Some(syn::FnArg::Receiver(r)) => match (r.reference.is_some(), r.mutability.is_some()) {
            (true, true) => Recv::RefMut,
            (true, false) => Recv::Ref,
            (false, _) => Recv::Value,
        },
        _ => Recv::None,
    }
}

fn arg_count(sig: &syn::Signature) -> usize {
    sig.inputs
        .iter()
        .filter(|i| matches!(i, syn::FnArg::Typed(_)))
        .count()
}

fn output(sig: &syn::Signature) -> String {
    match &sig.output {
        syn::ReturnType::Default => String::new(),
        syn::ReturnType::Type(_, ty) => text(ty),
    }
}

fn derives(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter(|a| attr_name(a) == "derive")
        .filter_map(|a| {
            a.parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            )
            .ok()
        })
        .flat_map(|paths| {
            paths
                .iter()
                .filter_map(|p| p.segments.last().map(|s| s.ident.to_string()))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Snake-case leaves of a `use` tree with a module before them: functions, as far as syntax can
/// tell (`use std::mem::swap`), but not modules imported whole (`use std::mem`).
fn fn_imports(tree: &syn::UseTree, depth: usize, out: &mut Vec<String>) {
    match tree {
        syn::UseTree::Path(p) => fn_imports(&p.tree, depth + 1, out),
        syn::UseTree::Group(g) => g.items.iter().for_each(|t| fn_imports(t, depth, out)),
        syn::UseTree::Name(n) if depth > 0 => {
            let name = n.ident.to_string();
            if is_snake(&name) && !matches!(name.as_str(), "self" | "super" | "crate") {
                out.push(name);
            }
        }
        syn::UseTree::Rename(r) if depth > 0 => {
            let name = r.rename.to_string();
            if is_snake(&name) {
                out.push(name);
            }
        }
        _ => {}
    }
}

impl FileIndex {
    pub fn build(file: &syn::File) -> FileIndex {
        let mut ix = FileIndex::default();
        ix.index_items(&file.items, true);
        let mut bounds = BoundFinder::default();
        bounds.visit_file(file);
        ix.bounds = bounds.found;
        let local = std::mem::take(&mut ix.local_fns);
        ix.imported_functions.retain(|f| !local.contains(f));
        ix.local_fns = local;
        ix
    }

    fn index_items(&mut self, items: &[syn::Item], top: bool) {
        for item in items {
            match item {
                syn::Item::Mod(m) => {
                    if is_test_item(&m.attrs) {
                        continue;
                    }
                    match &m.content {
                        Some((_, inner)) => self.index_items(inner, false),
                        None if top => self.non_leaf = true,
                        None => {}
                    }
                }
                syn::Item::Fn(f) => {
                    self.local_fns.insert(f.sig.ident.to_string());
                    if top && is_pub(&f.vis) {
                        self.top_pub_fns += 1;
                    }
                }
                syn::Item::Struct(s) => self.add_type(&s.ident, &s.vis, &s.attrs, top),
                syn::Item::Enum(e) => self.add_type(&e.ident, &e.vis, &e.attrs, top),
                syn::Item::Union(u) => self.add_type(&u.ident, &u.vis, &u.attrs, top),
                syn::Item::Trait(t) => {
                    self.trait_defs
                        .push((t.ident.to_string(), is_pub(&t.vis), t.ident.span()));
                }
                syn::Item::Impl(i) => {
                    if is_test_item(&i.attrs) {
                        continue;
                    }
                    let Some(ty) = last_ident(&i.self_ty).map(ToString::to_string) else {
                        continue;
                    };
                    match &i.trait_ {
                        Some((_, path, _)) => {
                            let Some(seg) = path.segments.last() else {
                                continue;
                            };
                            let name = seg.ident.to_string();
                            if name == "From" {
                                self.from_impls.push((ty.clone(), seg.ident.span()));
                            }
                            *self.impl_counts.entry(name.clone()).or_insert(0) += 1;
                            self.trait_impls.entry(ty).or_default().push(name);
                        }
                        None => {
                            for it in &i.items {
                                if let syn::ImplItem::Fn(f) = it {
                                    self.inherent.entry(ty.clone()).or_default().push(Method {
                                        name: f.sig.ident.to_string(),
                                        receiver: receiver(&f.sig),
                                        args: arg_count(&f.sig),
                                        is_pub: is_pub(&f.vis),
                                        output: output(&f.sig),
                                        span: f.sig.ident.span(),
                                    });
                                }
                            }
                        }
                    }
                }
                syn::Item::Use(u) if top => fn_imports(&u.tree, 0, &mut self.imported_functions),
                _ => {}
            }
        }
    }

    fn add_type(
        &mut self,
        ident: &syn::Ident,
        vis: &syn::Visibility,
        attrs: &[syn::Attribute],
        top: bool,
    ) {
        let name = ident.to_string();
        self.derives.insert(name.clone(), derives(attrs));
        if top && is_pub(vis) {
            self.top_pub_types.push((name.clone(), ident.span()));
        }
        self.types.insert(name);
    }

    fn has_trait(&self, ty: &str, trait_name: &str) -> bool {
        self.trait_impls
            .get(ty)
            .is_some_and(|ts| ts.iter().any(|t| t == trait_name))
            || self
                .derives
                .get(ty)
                .is_some_and(|ds| ds.iter().any(|d| d == trait_name))
    }

    fn has_method(&self, ty: &str, f: impl Fn(&Method) -> bool) -> bool {
        self.inherent.get(ty).is_some_and(|ms| ms.iter().any(f))
    }
}

pub fn file(a: &mut Analyzer, file: &syn::File, ix: &FileIndex) {
    file_attrs(a, &file.attrs);
    non_leaf(a, file, ix);
    named_after(a, ix);
    type_rules(a, ix);
    trait_rules(a, ix);
    walk(a, &file.items, ix);
}

fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(i) => &i.attrs,
        syn::Item::Enum(i) => &i.attrs,
        syn::Item::ExternCrate(i) => &i.attrs,
        syn::Item::Fn(i) => &i.attrs,
        syn::Item::ForeignMod(i) => &i.attrs,
        syn::Item::Impl(i) => &i.attrs,
        syn::Item::Macro(i) => &i.attrs,
        syn::Item::Mod(i) => &i.attrs,
        syn::Item::Static(i) => &i.attrs,
        syn::Item::Struct(i) => &i.attrs,
        syn::Item::Trait(i) => &i.attrs,
        syn::Item::TraitAlias(i) => &i.attrs,
        syn::Item::Type(i) => &i.attrs,
        syn::Item::Union(i) => &i.attrs,
        syn::Item::Use(i) => &i.attrs,
        _ => &[],
    }
}

fn walk(a: &mut Analyzer, items: &[syn::Item], ix: &FileIndex) {
    for item in items {
        attr_rules(a, item_attrs(item));
        match item {
            syn::Item::Mod(m) => {
                if is_test_item(&m.attrs) {
                    continue;
                }
                docs::item(a, &m.attrs, &m.ident, is_pub(&m.vis));
                if let Some((_, inner)) = &m.content {
                    walk(a, inner, ix);
                }
            }
            syn::Item::Fn(f) => {
                let info = FnInfo {
                    attrs: &f.attrs,
                    is_pub: is_pub(&f.vis),
                    sig: &f.sig,
                    body: Some(&f.block),
                    inherent: None,
                };
                api::signature(a, &info);
                docs::function(a, &info);
                if f.attrs.iter().any(|at| {
                    matches!(
                        attr_name(at).as_str(),
                        "proc_macro" | "proc_macro_derive" | "proc_macro_attribute"
                    )
                }) {
                    a.emit(
                        "macro-definition",
                        f.sig.ident.span(),
                        format!("`{}` defines a procedural macro, the guide's last resort after functions, `macro_rules!` and code generation", f.sig.ident),
                    );
                }
            }
            syn::Item::Struct(s) => {
                api::struct_item(a, s);
                docs::item(a, &s.attrs, &s.ident, is_pub(&s.vis));
                options_default(a, s, ix);
            }
            syn::Item::Enum(e) => {
                api::enum_item(a, e);
                docs::item(a, &e.attrs, &e.ident, is_pub(&e.vis));
            }
            syn::Item::Union(u) => {
                api::type_lifetimes(a, &u.generics, &u.ident);
                docs::item(a, &u.attrs, &u.ident, is_pub(&u.vis));
            }
            syn::Item::Type(t) => {
                api::type_lifetimes(a, &t.generics, &t.ident);
                docs::item(a, &t.attrs, &t.ident, is_pub(&t.vis));
            }
            syn::Item::Const(c) => docs::item(a, &c.attrs, &c.ident, is_pub(&c.vis)),
            syn::Item::Static(s) => docs::item(a, &s.attrs, &s.ident, is_pub(&s.vis)),
            syn::Item::Trait(t) => {
                api::type_lifetimes(a, &t.generics, &t.ident);
                docs::trait_item(a, t);
                for it in &t.items {
                    if let syn::TraitItem::Fn(f) = it {
                        let info = FnInfo {
                            attrs: &f.attrs,
                            is_pub: is_pub(&t.vis),
                            sig: &f.sig,
                            body: f.default.as_ref(),
                            inherent: None,
                        };
                        api::signature(a, &info);
                        docs::function(a, &info);
                    }
                }
            }
            syn::Item::Impl(i) => {
                if is_test_item(&i.attrs) {
                    continue;
                }
                let owner = last_ident(&i.self_ty).map(ToString::to_string);
                let inherent = i.trait_.is_none();
                for it in &i.items {
                    let syn::ImplItem::Fn(f) = it else { continue };
                    let info = FnInfo {
                        attrs: &f.attrs,
                        is_pub: inherent && is_pub(&f.vis),
                        sig: &f.sig,
                        body: Some(&f.block),
                        inherent: if inherent { owner.as_deref() } else { None },
                    };
                    api::signature(a, &info);
                    docs::function(a, &info);
                    if inherent {
                        method_rules(a, &info, ix);
                    }
                }
            }
            syn::Item::Macro(m) if m.mac.path.is_ident("macro_rules") => {
                if let Some(name) = &m.ident {
                    a.emit(
                        "macro-definition",
                        name.span(),
                        format!("`macro_rules! {name}`; the guide prefers a function, type or trait where one can do the job"),
                    );
                }
            }
            _ => {}
        }
    }
}

const INIT_SECTIONS: &[&str] = &[".init_array", "__mod_init_func", ".CRT$XCU", ".ctors"];

/// `#[ctor]`, initialiser link sections and `#[rustfmt::skip]` on any item.
fn attr_rules(a: &mut Analyzer, attrs: &[syn::Attribute]) {
    let comments = a.comments;
    for attr in attrs {
        let name = attr_name(attr);
        let span = attr.meta.span();
        match name.as_str() {
            "ctor" | "ctor::ctor" | "dtor" | "ctor::dtor" => a.emit(
                "code-before-main",
                span,
                format!("`#[{name}]` runs this before (or after) `main()`, where the runtime isn't set up"),
            ),
            "link_section" => {
                if let syn::Meta::NameValue(nv) = &attr.meta
                    && let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) = &nv.value
                    && INIT_SECTIONS.iter().any(|sec| s.value().contains(sec))
                {
                    a.emit(
                        "code-before-main",
                        span,
                        format!("link section `{}` runs this before `main()`", s.value()),
                    );
                }
            }
            "rustfmt::skip" => {
                let line = span.start().line;
                let reason = [line, line.saturating_sub(1)].iter().any(|&n| {
                    n > 0
                        && comments.get(n - 1).is_some_and(|l| {
                            let c = l.comment.to_ascii_lowercase();
                            c.contains("align") || c.contains("generat")
                        })
                });
                if !reason {
                    a.emit(
                        "rustfmt-skip",
                        span,
                        "`#[rustfmt::skip]` without a reason; the guide allows it only for manual alignment or generated code (say which in a comment)",
                    );
                }
            }
            _ => {}
        }
    }
}

/// `#![feature(..)]`, also behind `cfg_attr`, and `#![rustfmt::skip]`.
fn file_attrs(a: &mut Analyzer, attrs: &[syn::Attribute]) {
    for attr in attrs {
        let name = attr_name(attr);
        let span = attr.meta.span();
        if name == "feature" {
            a.emit(
                "unstable-feature",
                span,
                "`#![feature(..)]` needs a nightly compiler, and the feature can change or vanish; the guide allows none without an approval",
            );
        } else if name == "cfg_attr"
            && matches!(&attr.meta, syn::Meta::List(l) if l.tokens.to_string().contains("feature ("))
        {
            a.emit(
                "unstable-feature",
                span,
                "`cfg_attr(.., feature(..))` enables an unstable feature on some builds; the guide allows none without an approval",
            );
        }
    }
    attr_rules(a, attrs);
}

/// Lines of definitions in a module that declares submodules; the guide allows "a small amount".
const NON_LEAF_ALLOWANCE: usize = 50;

fn non_leaf(a: &mut Analyzer, file: &syn::File, ix: &FileIndex) {
    if !ix.non_leaf {
        return;
    }
    let lines = a.lines;
    let mut total = 0;
    let mut first = None;
    for item in &file.items {
        let defining = matches!(
            item,
            syn::Item::Fn(_)
                | syn::Item::Struct(_)
                | syn::Item::Enum(_)
                | syn::Item::Impl(_)
                | syn::Item::Trait(_)
                | syn::Item::Const(_)
                | syn::Item::Static(_)
                | syn::Item::Type(_)
                | syn::Item::Union(_)
                | syn::Item::Macro(_)
        );
        if !defining || is_test_item(item_attrs(item)) {
            continue;
        }
        let span = item.span();
        total += metrics::code_lines(lines, span.start().line, span.end().line);
        first.get_or_insert(span);
    }
    if total > NON_LEAF_ALLOWANCE
        && let Some(span) = first
    {
        a.emit(
            "api-in-non-leaf-module",
            span,
            format!("this module declares submodules and also holds {total} lines of definitions; a non-leaf module should hold only `mod` declarations and re-exports"),
        );
    }
}

fn named_after(a: &mut Analyzer, ix: &FileIndex) {
    let stem = a
        .path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
    if matches!(stem.as_str(), "mod" | "lib" | "main" | "build") || ix.top_pub_fns >= 2 {
        return;
    }
    if let [(name, span)] = ix.top_pub_types.as_slice()
        && snake_case(name) != stem
    {
        a.emit(
            "file-named-after-item",
            *span,
            format!(
                "`{stem}.rs` defines `{name}`; name the file after its primary item (`{}.rs`)",
                snake_case(name)
            ),
        );
    }
}

/// Rules over a type's set of inherent methods and trait impls.
fn type_rules(a: &mut Analyzer, ix: &FileIndex) {
    let mut types: Vec<&String> = ix.inherent.keys().collect();
    types.sort();
    for ty in types {
        let methods = &ix.inherent[ty];
        if let Some(len) = methods
            .iter()
            .find(|m| m.name == "len" && m.is_pub && m.receiver == Recv::Ref && m.args == 0)
            && !ix.has_method(ty, |m| m.name == "is_empty")
        {
            a.emit(
                "len-without-is-empty",
                len.span,
                format!("`{ty}::len()` with no `is_empty()`; callers write `len() == 0` instead"),
            );
        }
        if let Some(new) = methods.iter().find(|m| {
            m.name == "new"
                && m.is_pub
                && m.receiver == Recv::None
                && m.args == 0
                && returns_self(&m.output, ty)
        }) && ix.types.contains(ty)
            && !ix.has_trait(ty, "Default")
        {
            a.emit(
                "new-without-default",
                new.span,
                format!("`{ty}::new()` takes no arguments; implement `Default` too (the guide: implement it if the type has `new()`)"),
            );
        }
    }
    for (ty, span) in &ix.from_impls {
        if ix.types.contains(ty)
            && !ix.has_method(ty, |m| {
                m.receiver == Recv::None
                    && (m.name == "new"
                        || m.name.starts_with("new_")
                        || m.name.starts_with("from_"))
            })
        {
            a.emit(
                "from-impl-without-constructor",
                *span,
                format!("`impl From<..> for {ty}` with no explicit constructor; pair it with `{ty}::new()`, `::new_from_foo()` or `::from_foo()`"),
            );
        }
    }
}

fn trait_rules(a: &mut Analyzer, ix: &FileIndex) {
    for (name, is_pub, span) in &ix.trait_defs {
        if !is_pub
            && !name.ends_with("Ext")
            && ix.impl_counts.get(name).copied().unwrap_or(0) == 1
            && !ix.bounds.iter().any(|(b, _)| b == name)
        {
            a.emit(
                "trait-as-organization",
                *span,
                format!("`{name}` is private, implemented once and never used as a bound; the guide says not to use traits merely to organise code"),
            );
        }
    }
    for (name, span) in &ix.bounds {
        if name.len() > 3 && name.ends_with("Ext") {
            a.emit(
                "ext-trait-in-bounds",
                *span,
                format!("`{name}` is an extension trait, used here as a bound; extension traits add methods and aren't meant for generics or `dyn`"),
            );
        }
    }
}

const OPTIONS_SUFFIXES: &[&str] = &["Options", "Opts", "Config", "Settings", "Params"];

fn options_default(a: &mut Analyzer, s: &syn::ItemStruct, ix: &FileIndex) {
    let name = s.ident.to_string();
    let syn::Fields::Named(fields) = &s.fields else {
        return;
    };
    if is_pub(&s.vis)
        && OPTIONS_SUFFIXES.iter().any(|suf| name.ends_with(suf))
        && fields.named.len() >= 2
        && fields.named.iter().all(|f| is_pub(&f.vis))
        && !ix.has_trait(&name, "Default")
    {
        a.emit(
            "options-struct-default",
            s.ident.span(),
            format!("`{name}` is a bag of options with public fields; implement `Default` so callers can write `{name} {{ a, ..Default::default() }}`"),
        );
    }
}

fn returns_self(output: &str, owner: &str) -> bool {
    output == "Self" || output == owner
}

/// `Result<Self, _>`, `io::Result<Self>`, `Option<Self>`.
fn returns_fallible_self(output: &str, owner: &str) -> bool {
    (output.contains("Result<") || output.contains("Option<"))
        && (output.contains("<Self") || output.contains(&format!("<{owner}")))
}

const CONSTRUCTOR_VERBS: &[&str] = &[
    "default", "open", "create", "connect", "bind", "spawn", "parse", "load", "build", "empty",
    "now", "zeroed", "random",
];

/// Standard trait methods by name: (method, trait, receiver, arguments).
const STD_TRAIT_METHODS: &[(&str, &str, Recv, usize)] = &[
    ("from_str", "FromStr", Recv::None, 1),
    ("default", "Default", Recv::None, 0),
    ("from_iter", "FromIterator", Recv::None, 1),
    ("to_string", "Display", Recv::Ref, 0),
    ("clone", "Clone", Recv::Ref, 0),
    ("eq", "PartialEq", Recv::Ref, 1),
    ("cmp", "Ord", Recv::Ref, 1),
    ("partial_cmp", "PartialOrd", Recv::Ref, 1),
    ("hash", "Hash", Recv::Ref, 1),
    ("as_ref", "AsRef", Recv::Ref, 0),
    ("as_mut", "AsMut", Recv::RefMut, 0),
    ("borrow", "Borrow", Recv::Ref, 0),
    ("deref", "Deref", Recv::Ref, 0),
    ("index", "Index", Recv::Ref, 1),
    ("next", "Iterator", Recv::RefMut, 0),
    ("into_iter", "IntoIterator", Recv::Value, 0),
    ("add", "Add", Recv::Value, 1),
    ("sub", "Sub", Recv::Value, 1),
    ("mul", "Mul", Recv::Value, 1),
    ("div", "Div", Recv::Value, 1),
    ("neg", "Neg", Recv::Value, 0),
    ("not", "Not", Recv::Value, 0),
    ("drop", "Drop", Recv::RefMut, 0),
];

/// Rules on one method of an inherent impl.
fn method_rules(a: &mut Analyzer, f: &FnInfo, ix: &FileIndex) {
    let Some(owner) = f.inherent else { return };
    let sig = f.sig;
    let name = sig.ident.to_string();
    let (recv, args, out) = (receiver(sig), arg_count(sig), output(sig));
    if f.is_pub
        && recv == Recv::Ref
        && args == 0
        && let Some(rest) = name.strip_prefix("get_")
        && !rest.is_empty()
        && !matches!(rest, "mut" | "ref")
        && !rest.starts_with("or_")
    {
        a.emit(
            "getter-prefix",
            sig.ident.span(),
            format!("getter `{name}()`; the guide names a getter after the field: `{rest}()`"),
        );
    }
    if recv == Recv::None {
        let fallible = returns_fallible_self(&out, owner);
        if returns_self(&out, owner) || fallible {
            let allowed = name == "new"
                || name.starts_with("new_")
                || name == "try_new"
                || name.starts_with("try_new_")
                || name.starts_with("from_")
                || name.starts_with("try_from_")
                || name.starts_with("with_")
                || CONSTRUCTOR_VERBS.contains(&name.as_str());
            if name == "new" && fallible {
                a.emit(
                    "constructor-name",
                    sig.ident.span(),
                    format!("`{owner}::new()` returns `{out}`; the guide names a fallible constructor `try_new()`"),
                );
            } else if !allowed {
                a.emit(
                    "constructor-name",
                    sig.ident.span(),
                    format!("constructor `{owner}::{name}()`; the guide uses `new()`, `new_with_foo()`, `try_new()` or `from_foo()`"),
                );
            }
        }
    }
    if f.is_pub
        && let Some((_, trait_name, _, _)) = STD_TRAIT_METHODS
            .iter()
            .find(|(m, _, r, n)| *m == name && *r == recv && *n == args)
    {
        let implemented = ix.has_trait(owner, trait_name)
            || (name == "to_string" && ix.has_trait(owner, "Debug"))
            || ix.has_trait(owner, "Copy") && *trait_name == "Clone";
        if !implemented {
            a.emit(
                "std-trait-as-method",
                sig.ident.span(),
                format!("`{owner}::{name}()` does what `{trait_name}` does; implement the trait (keeping the method is fine)"),
            );
        }
    }
}
