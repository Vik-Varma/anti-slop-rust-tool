use syn::spanned::Spanned;

use super::{call_path, is_method, path_ends_with, peel};
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
            None => return,
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
}

fn persist_op(e: &syn::Expr) -> Option<String> {
    match peel(e) {
        syn::Expr::MethodCall(m) if PERSIST_METHODS.contains(&m.method.to_string().as_str()) => {
            // A failed write to the terminal has nowhere to be reported; ignoring it is normal.
            if mentions_std_stream(&m.receiver) {
                return None;
            }
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

fn mentions_std_stream(e: &syn::Expr) -> bool {
    let text = quote::quote!(#e).to_string();
    text.contains("stdout") || text.contains("stderr")
}
