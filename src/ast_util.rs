//! Helpers shared by the AST-based rules.

use syn::punctuated::Punctuated;
use syn::{Expr, Macro, Token};

/// `syn` leaves macro invocations as opaque token streams. Most macros that
/// appear in function bodies (`println!`, `assert!`, `vec!`, `format!`,
/// `write!`, ...) take comma-separated expressions, so we try to parse them
/// that way to make the code inside visible to the rules.
pub fn macro_exprs(mac: &Macro) -> Option<Vec<Expr>> {
    if mac.path.is_ident("macro_rules") {
        return None;
    }
    let parsed = mac.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated).ok()?;
    Some(parsed.into_iter().collect())
}

/// Name of the macro, e.g. `println` for `std::println!`.
pub fn macro_name(mac: &Macro) -> String {
    mac.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default()
}

/// Last path segment of a path expression, e.g. `sleep` for `std::thread::sleep`.
pub fn path_last(path: &syn::Path) -> Option<String> {
    path.segments.last().map(|s| s.ident.to_string())
}

/// The full path as `a::b::c`.
pub fn path_string(path: &syn::Path) -> String {
    path.segments.iter().map(|s| s.ident.to_string()).collect::<Vec<_>>().join("::")
}

/// Strips parentheses, references and groups around an expression.
pub fn peel(mut e: &Expr) -> &Expr {
    loop {
        e = match e {
            Expr::Paren(p) => &p.expr,
            Expr::Group(g) => &g.expr,
            _ => return e,
        };
    }
}

/// Is this a method call `.name(..)`?
pub fn is_method_call<'a>(e: &'a Expr, name: &str) -> Option<&'a syn::ExprMethodCall> {
    match peel(e) {
        Expr::MethodCall(m) if m.method == name => Some(m),
        _ => None,
    }
}

/// Does this expression (anywhere in its tree) contain a method call named `name`?
pub fn contains_method_call(e: &Expr, names: &[&str]) -> bool {
    struct Finder<'n> {
        names: &'n [&'n str],
        found: bool,
    }
    impl<'ast> syn::visit::Visit<'ast> for Finder<'_> {
        fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
            if self.names.iter().any(|n| m.method == n) {
                self.found = true;
            }
            syn::visit::visit_expr_method_call(self, m);
        }
        fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}
        fn visit_expr_async(&mut self, _: &'ast syn::ExprAsync) {}
    }
    let mut f = Finder { names, found: false };
    syn::visit::Visit::visit_expr(&mut f, e);
    f.found
}
