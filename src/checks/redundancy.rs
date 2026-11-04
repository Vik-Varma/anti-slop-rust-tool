//! Writes that no read can see: a place assigned and then assigned again, or replaced whole,
//! before anything reads it. Syntax only, within one block. Calls and aliasing are the reason this
//! stays conservative: any statement in between that mentions the place's root, or that can leave
//! the block (`return`, `?`, `break`, `continue`, `.await`), ends the search.

use proc_macro2::Span;
use quote::quote;

use crate::analyzer::Analyzer;

/// A place the check can reason about: a local or `self`, with field accesses.
struct Place {
    root: String,
    text: String,
}

fn place(e: &syn::Expr) -> Option<Place> {
    match e {
        syn::Expr::Paren(p) => place(&p.expr),
        syn::Expr::Path(p) if p.path.segments.len() == 1 && p.qself.is_none() => {
            let id = p.path.segments[0].ident.to_string();
            // `STATIC = ..` and `CONST` names can be aliased from anywhere.
            let shouty = id
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
            (!shouty).then(|| Place {
                root: id.clone(),
                text: id,
            })
        }
        syn::Expr::Field(f) => {
            let base = place(&f.base)?;
            let member = match &f.member {
                syn::Member::Named(i) => i.to_string(),
                syn::Member::Unnamed(i) => i.index.to_string(),
            };
            Some(Place {
                root: base.root,
                text: format!("{}.{member}", base.text),
            })
        }
        _ => None,
    }
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
}

fn mentions(text: &str, root: &str) -> bool {
    words(text).any(|w| w == root)
}

/// Whether control can leave the block, or suspend, so a write before it may be observed later.
fn exits(text: &str) -> bool {
    text.contains('?')
        || words(text).any(|w| matches!(w, "return" | "break" | "continue" | "await" | "yield"))
}

/// A plain assignment statement `place = rhs;`: its place, its right-hand side's text, and the
/// span of the `=`.
fn assignment(stmt: &syn::Stmt) -> Option<(Place, String, Span)> {
    let syn::Stmt::Expr(syn::Expr::Assign(a), Some(_)) = stmt else {
        return None;
    };
    let p = place(&a.left)?;
    let rhs = &a.right;
    Some((p, quote!(#rhs).to_string(), a.eq_token.spans[0]))
}

/// Whether writing `later` replaces everything `earlier` wrote: the same place, or an ancestor.
fn covers(later: &str, earlier: &str) -> bool {
    earlier == later || (earlier.starts_with(later) && earlier[later.len()..].starts_with('.'))
}

pub fn block(a: &mut Analyzer, node: &syn::Block) {
    if a.scope().test {
        return;
    }
    let stmts = &node.stmts;
    for (i, stmt) in stmts.iter().enumerate() {
        let Some((first, _, at)) = assignment(stmt) else {
            continue;
        };
        for next in &stmts[i + 1..] {
            let text = quote!(#next).to_string();
            if let Some((later, rhs, _)) = assignment(next)
                && covers(&later.text, &first.text)
                && later.root == first.root
                && !mentions(&rhs, &first.root)
            {
                a.emit(
                    "overwritten-assignment",
                    at,
                    format!(
                        "`{}` is assigned here and written again before anything reads it; the first write has no effect (if its right-hand side has side effects, keep the call, not the assignment)",
                        first.text
                    ),
                );
                break;
            }
            if mentions(&text, &first.root) || exits(&text) {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    fn lines(src: &str) -> Vec<usize> {
        crate::analyze_source(Path::new("src/lib.rs"), src)
            .expect("parses")
            .findings
            .iter()
            .filter(|f| f.rule == "overwritten-assignment")
            .map(|f| f.line)
            .collect()
    }

    #[test]
    fn flags_overwritten_and_replaced() {
        let src = "struct S { a: u8, b: [u8; 3] }\n\
                   fn f(mut s: S, v: u8) -> S {\n\
                       s.a = 1;\n\
                       let w = v + 1;\n\
                       s.a = w;\n\
                       s.b = [0; 3];\n\
                       s = S { a: 0, b: [1; 3] };\n\
                       s\n\
                   }\n";
        assert_eq!(lines(src), vec![3, 6]);
    }

    #[test]
    fn leaves_reads_exits_and_conditionals_alone() {
        let src = "struct S { a: u8 }\n\
                   fn g(s: &mut S, c: bool) -> Option<u8> {\n\
                       s.a = 1;\n\
                       let n = s.a;\n\
                       s.a = 2;\n\
                       s.a = 3;\n\
                       if c { return None; }\n\
                       s.a = 4;\n\
                       s.a = s.a + 1;\n\
                       s.a = 5;\n\
                       helper(s);\n\
                       s.a = 6;\n\
                       Some(n)\n\
                   }\n\
                   fn helper(_: &mut S) {}\n";
        // Line 5 is overwritten by line 6 with nothing between; every other write is read,
        // can be observed through the early return, or is guarded.
        assert_eq!(lines(src), vec![5, 9]);
    }

    #[test]
    fn tests_are_exempt() {
        let src = "#[cfg(test)]\nmod t { fn f(mut x: u8) { x = 1; x = 2; let _ = x; } }\n";
        assert!(lines(src).is_empty());
    }
}
