use super::{call_path, is_method, macro_name, path_ends_with, peel};
use crate::analyzer::Analyzer;

/// Receivers whose `unwrap()` re-raises another thread's panic (a poisoned lock). The std docs use
/// `lock().unwrap()` throughout, and propagating the poison is the point, so it isn't flagged.
const POISON_RECEIVERS: &[&str] = &["lock", "read", "write"];

/// Assertions aren't included: in the audit they checked internal invariants that only fail if the
/// program is already broken, which is what an assertion is for.
const PANICKING_MACROS: &[&str] = &["panic", "unreachable", "todo", "unimplemented"];

/// Methods whose `Err`/`None` comes from outside the program (text, bytes, files, the
/// environment), with their argument count so a map's `metadata(id)` isn't taken for
/// `Path::metadata()`. `None` means any count.
const INPUT_METHODS: &[(&str, Option<usize>)] = &[
    ("parse", Some(0)),
    ("decode", None),
    ("read_to_string", Some(1)),
    ("read_to_end", Some(1)),
    ("read_line", Some(1)),
    ("read_dir", Some(0)),
    ("metadata", Some(0)),
    ("canonicalize", Some(0)),
    ("to_str", Some(0)),
    ("into_string", Some(0)),
];

/// Functions likewise, by their last two path segments, so a local `open()` isn't mistaken for
/// `File::open`.
const INPUT_FUNCTIONS: &[[&str; 2]] = &[
    ["File", "open"],
    ["File", "create"],
    ["fs", "read"],
    ["fs", "read_to_string"],
    ["fs", "read_dir"],
    ["fs", "metadata"],
    ["fs", "canonicalize"],
    ["env", "var"],
    ["String", "from_utf8"],
    ["str", "from_utf8"],
    ["serde_json", "from_str"],
    ["serde_json", "from_slice"],
    ["serde_json", "from_reader"],
    ["serde_json", "from_value"],
    ["toml", "from_str"],
    ["serde_yaml", "from_str"],
    ["TcpStream", "connect"],
    ["TcpListener", "bind"],
    ["UdpSocket", "bind"],
];

/// What the receiver of an `unwrap`/`expect` reads from outside the program, if anything.
fn input_source(receiver: &syn::Expr) -> Option<String> {
    match peel(receiver) {
        syn::Expr::MethodCall(m) => {
            let name = m.method.to_string();
            if matches!(peel(&m.receiver), syn::Expr::Lit(_)) {
                return None; // `"127.0.0.1".parse()` fails on the first run, not in production
            }
            if INPUT_METHODS
                .iter()
                .any(|(n, args)| *n == name && args.is_none_or(|k| k == m.args.len()))
            {
                return Some(format!(".{name}()"));
            }
            if matches!(name.as_str(), "output" | "spawn" | "status") && spawns_command(&m.receiver)
            {
                return Some(format!("Command::{name}()"));
            }
            None
        }
        syn::Expr::Call(c) => {
            let path = call_path(c)?;
            // A parser given a literal fails on the first run, not in production. A file, variable
            // or address given a literal can still be missing at run time.
            let parses = path
                .segments
                .last()
                .is_some_and(|s| s.ident.to_string().starts_with("from_"));
            if parses
                && !c.args.is_empty()
                && c.args.iter().all(|a| matches!(peel(a), syn::Expr::Lit(_)))
            {
                return None;
            }
            INPUT_FUNCTIONS
                .iter()
                .find(|f| path_ends_with(path, *f))
                .map(|f| format!("{}::{}()", f[0], f[1]))
        }
        _ => None,
    }
}

fn spawns_command(mut e: &syn::Expr) -> bool {
    loop {
        match peel(e) {
            syn::Expr::MethodCall(m) => e = &m.receiver,
            syn::Expr::Call(c) => {
                return call_path(c).is_some_and(|p| path_ends_with(p, &["Command", "new"]));
            }
            _ => return false,
        }
    }
}

pub fn method_call(a: &mut Analyzer, node: &syn::ExprMethodCall) {
    let scope = a.scope();
    let (test, in_drop, in_const) = (scope.test, scope.in_drop, scope.in_const);
    let method = node.method.to_string();
    if test || in_const || !(method == "unwrap" && node.args.is_empty() || method == "expect") {
        return;
    }
    if in_drop {
        a.emit(
            "panic-in-drop",
            node.method.span(),
            format!("`.{method}()` inside `Drop::drop`"),
        );
        return;
    }
    // `.expect("why")` on input states that an earlier step validated it, as the Book asks.
    if let Some(source) = input_source(&node.receiver).filter(|_| method == "unwrap") {
        a.emit(
            "panic-on-input",
            node.method.span(),
            format!("`.{method}()` on {source}: bad input or a missing file crashes the program; return the error"),
        );
        return;
    }
    if method != "unwrap"
        || POISON_RECEIVERS
            .iter()
            .any(|r| is_method(&node.receiver, r, 0).is_some())
    {
        return;
    }
    match peel(&node.receiver) {
        // `"lit".parse().unwrap()` and friends fail on the first run, not in production.
        syn::Expr::MethodCall(inner) if matches!(peel(&inner.receiver), syn::Expr::Lit(_)) => {
            return;
        }
        // `write!(string, ..).unwrap()`: formatting into a `String` can't fail.
        syn::Expr::Macro(m) if matches!(macro_name(&m.mac).as_str(), "write" | "writeln") => return,
        _ => {}
    }
    a.emit(
        "unwrap",
        node.method.span(),
        "`.unwrap()` panics with no context; propagate with `?`, or state the invariant with `.expect(\"why this can't fail\")`",
    );
}

pub fn macro_call(a: &mut Analyzer, mac: &syn::Macro) {
    let name = macro_name(mac);
    let scope = a.scope();
    if scope.test {
        return;
    }
    let span = mac
        .path
        .segments
        .last()
        .map(|s| s.ident.span())
        .unwrap_or_else(proc_macro2::Span::call_site);
    if scope.in_drop && PANICKING_MACROS.contains(&name.as_str()) {
        a.emit(
            "panic-in-drop",
            span,
            format!("`{name}!` inside `Drop::drop`"),
        );
        return;
    }
    match name.as_str() {
        "todo" if !scope.in_const => a.emit(
            "todo",
            span,
            "`todo!()` panics when this code runs; finish it or return an error",
        ),
        "dbg" => a.emit(
            "dbg-macro",
            span,
            "`dbg!` left in shipping code prints to stderr in release builds",
        ),
        _ => {}
    }
}
