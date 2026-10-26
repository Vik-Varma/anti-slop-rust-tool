use syn::spanned::Spanned;

use super::{call_path, is_method, path_ends_with, peel, strip_unwrapping};
use crate::analyzer::Analyzer;
use crate::context::FileKind;

/// Operations whose failure means data didn't reach where the caller thinks it did.
const PERSIST_METHODS: &[&str] = &[
    "write_all",
    "flush",
    "sync_all",
    "sync_data",
    "set_len",
    "write_fmt",
];
const PERSIST_FUNCTIONS: &[&[&str]] = &[&["fs", "write"], &["fs", "rename"], &["fs", "copy"]];

pub fn call(a: &mut Analyzer, node: &syn::ExprCall) {
    let Some(path) = call_path(node) else { return };
    if a.kind == FileKind::Library && !a.scope().test && path_ends_with(path, &["process", "exit"])
    {
        a.emit(
            "process-exit-in-library",
            path.span(),
            "`process::exit` in a library skips every destructor (unflushed buffers, temp files); return an error to the caller",
        );
    }
}

pub fn stmt(a: &mut Analyzer, node: &syn::Stmt) {
    if a.scope().test {
        return;
    }
    let (discarded, how) = match node {
        syn::Stmt::Local(l) if matches!(l.pat, syn::Pat::Wild(_)) => match &l.init {
            Some(init) => (init.expr.as_ref(), "`let _ =`"),
            None => return,
        },
        syn::Stmt::Expr(e, Some(_)) => match is_method(e, "ok", 0) {
            Some(m) => (&*m.receiver, "`.ok();`"),
            None => {
                unused_amount(a, e, false);
                return;
            }
        },
        _ => return,
    };
    if let Some(op) = persist_op(discarded) {
        a.emit(
            "ignored-io-error",
            discarded.span(),
            format!("{how} throws away the error from `{op}`: the data may not have been written"),
        );
    }
    if matches!(node, syn::Stmt::Local(_)) {
        unused_amount(a, discarded, true);
    }
}

fn persist_op(e: &syn::Expr) -> Option<String> {
    match peel(e) {
        syn::Expr::MethodCall(m) if PERSIST_METHODS.contains(&m.method.to_string().as_str()) => {
            Some(format!(".{}()", m.method))
        }
        syn::Expr::Call(c) => {
            let path = call_path(c)?;
            PERSIST_FUNCTIONS
                .iter()
                .find(|suffix| path_ends_with(path, suffix))
                .map(|suffix| suffix.join("::"))
        }
        _ => None,
    }
}

/// `w.write(buf)?;` and `r.read(buf)?;` drop the count of bytes moved, which may be less than
/// `buf.len()`. Only flagged when the result is clearly a `Result` (`?`, `unwrap`, `expect`,
/// `let _ =`), so a custom `write(&mut self, x)` returning `()` isn't mistaken for `io::Write`.
fn unused_amount(a: &mut Analyzer, e: &syn::Expr, discarded_by_let: bool) {
    let peeled = peel(e);
    let inner = strip_unwrapping(peeled);
    if std::ptr::eq(inner, peeled) && !discarded_by_let {
        return;
    }
    let Some(m) = is_method(inner, "write", 1).or_else(|| is_method(inner, "read", 1)) else {
        return;
    };
    let fix = if m.method == "write" {
        "write_all"
    } else {
        "read_exact"
    };
    a.emit(
        "unused-io-amount",
        m.method.span(),
        format!(
            "`.{}()` may move fewer bytes than asked and the count is ignored; use `.{fix}()`",
            m.method
        ),
    );
}
