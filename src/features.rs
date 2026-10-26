//! Counts of patterns that are often called bad but that no rule flags yet. The study in `study/`
//! measures whether they predict bug fixes; a pattern becomes a rule only if it does.

use syn::visit::{self, Visit};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Features {
    /// `.clone()` calls ("clone to satisfy the borrow checker").
    pub clones: usize,
    /// `.expect(..)` calls.
    pub expects: usize,
    /// `panic!`, `unreachable!`, `assert!` and friends.
    pub panics: usize,
    /// `as` casts between any types.
    pub as_casts: usize,
    /// Numeric literals other than 0, 1 and 2 ("magic numbers").
    pub magic_numbers: usize,
    /// `unsafe` blocks.
    pub unsafe_blocks: usize,
    /// Parameters of type `bool` ("flag arguments").
    pub bool_params: usize,
    /// Comment lines with TODO, FIXME, HACK or XXX (self-admitted technical debt).
    pub debt_comments: usize,
    /// Comment lines of any kind.
    pub comment_lines: usize,
}

pub fn count(sig: &syn::Signature, block: &syn::Block, lines: &[&str], start: usize, end: usize) -> Features {
    let mut f = Features::default();
    f.bool_params = sig
        .inputs
        .iter()
        .filter(|a| matches!(a, syn::FnArg::Typed(t) if matches!(t.ty.as_ref(), syn::Type::Path(p) if p.path.is_ident("bool"))))
        .count();
    for line in lines.iter().take(end).skip(start.saturating_sub(1)) {
        let Some(at) = line.find("//") else { continue };
        f.comment_lines += 1;
        let c = &line[at..];
        if ["TODO", "FIXME", "HACK", "XXX"].iter().any(|m| c.contains(m)) {
            f.debt_comments += 1;
        }
    }
    let mut v = Counter(&mut f);
    v.visit_block(block);
    f
}

struct Counter<'f>(&'f mut Features);

impl<'ast> Visit<'ast> for Counter<'_> {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        match node.method.to_string().as_str() {
            "clone" if node.args.is_empty() => self.0.clones += 1,
            "expect" => self.0.expects += 1,
            _ => {}
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_cast(&mut self, node: &'ast syn::ExprCast) {
        self.0.as_casts += 1;
        visit::visit_expr_cast(self, node);
    }

    fn visit_expr_unsafe(&mut self, node: &'ast syn::ExprUnsafe) {
        self.0.unsafe_blocks += 1;
        visit::visit_expr_unsafe(self, node);
    }

    fn visit_lit(&mut self, node: &'ast syn::Lit) {
        let magic = match node {
            syn::Lit::Int(i) => !matches!(i.base10_digits(), "0" | "1" | "2"),
            syn::Lit::Float(f) => !matches!(f.base10_digits(), "0.0" | "1.0" | "0" | "1" | "2" | "2.0" | "0.5"),
            _ => false,
        };
        if magic {
            self.0.magic_numbers += 1;
        }
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let name = crate::checks::macro_name(node);
        if matches!(
            name.as_str(),
            "panic" | "unreachable" | "assert" | "assert_eq" | "assert_ne" | "todo" | "unimplemented"
        ) {
            self.0.panics += 1;
        }
        if let Some(args) = crate::checks::macro_args(node) {
            for a in &args {
                self.visit_expr(a);
            }
        }
    }

    // `const`/`static` items name their numbers; nested fns are measured on their own.
    fn visit_item(&mut self, _: &'ast syn::Item) {}
}
