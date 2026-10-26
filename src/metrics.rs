//! Size and complexity of each function, measured the way the literature defines them so the
//! numbers can be compared with published thresholds (see docs/EVIDENCE.md).

use syn::visit::{self, Visit};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionMetrics {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    /// Non-blank, non-comment lines from the signature to the closing brace.
    pub code_lines: usize,
    /// McCabe (1976): 1 + `if` + `while` + `for` + extra `match` arms + `&&` + `||`.
    pub cyclomatic: usize,
    /// Campbell (2018), SonarSource's cognitive complexity, with Rust's constructs mapped onto it.
    pub cognitive: usize,
    /// Deepest nesting of `if`, `match`, loops and closures.
    pub max_nesting: usize,
    /// Parameters, not counting `self`.
    pub params: usize,
    /// `?` operators: hidden early returns, kept apart from `cyclomatic` so it matches McCabe.
    pub try_ops: usize,
    pub test: bool,
}

pub fn measure(
    name: String,
    sig: &syn::Signature,
    block: &syn::Block,
    start_line: usize,
    end_line: usize,
    lines: &[&str],
    test: bool,
) -> FunctionMetrics {
    let mut c = Complexity::default();
    c.visit_block(block);
    let params = sig
        .inputs
        .iter()
        .filter(|a| matches!(a, syn::FnArg::Typed(_)))
        .count();
    FunctionMetrics {
        name,
        start_line,
        end_line,
        code_lines: code_lines(lines, start_line, end_line),
        cyclomatic: 1 + c.decisions,
        cognitive: c.cognitive,
        max_nesting: c.max_nesting,
        params,
        try_ops: c.try_ops,
        test,
    }
}

/// Counts lines in `start..=end` (1-based) that hold code, not just whitespace or a comment.
pub fn code_lines(lines: &[&str], start: usize, end: usize) -> usize {
    let mut in_block_comment = false;
    let mut n = 0;
    for line in lines.iter().take(end).skip(start.saturating_sub(1)) {
        let t = line.trim();
        if in_block_comment {
            if t.contains("*/") {
                in_block_comment = false;
            }
            continue;
        }
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        if t.starts_with("/*") {
            in_block_comment = !t.contains("*/");
            continue;
        }
        n += 1;
    }
    n
}

#[derive(Default)]
struct Complexity {
    decisions: usize,
    cognitive: usize,
    nesting: usize,
    max_nesting: usize,
    try_ops: usize,
}

impl Complexity {
    /// A structure that both costs +1 plus its nesting depth and nests what's inside it.
    fn nested(&mut self, f: impl FnOnce(&mut Self)) {
        self.cognitive += 1 + self.nesting;
        self.deeper(f);
    }

    fn deeper(&mut self, f: impl FnOnce(&mut Self)) {
        self.nesting += 1;
        self.max_nesting = self.max_nesting.max(self.nesting);
        f(self);
        self.nesting -= 1;
    }

    fn visit_if_chain(&mut self, node: &syn::ExprIf) {
        self.decisions += 1;
        self.visit_expr(&node.cond);
        self.deeper(|c| c.visit_block(&node.then_branch));
        if let Some((_, other)) = &node.else_branch {
            match other.as_ref() {
                syn::Expr::If(elif) => {
                    // `else if`: +1, no nesting penalty, and it continues the same chain.
                    self.cognitive += 1;
                    self.visit_if_chain(elif);
                }
                other => {
                    self.cognitive += 1;
                    self.deeper(|c| c.visit_expr(other));
                }
            }
        }
    }
}

fn is_logical(op: &syn::BinOp) -> bool {
    matches!(op, syn::BinOp::And(_) | syn::BinOp::Or(_))
}

/// Flattens `a && b || c` into its operators, in order, and its non-logical operands.
fn logical_chain<'a>(expr: &'a syn::Expr, ops: &mut Vec<bool>, leaves: &mut Vec<&'a syn::Expr>) {
    match expr {
        syn::Expr::Binary(b) if is_logical(&b.op) => {
            logical_chain(&b.left, ops, leaves);
            ops.push(matches!(b.op, syn::BinOp::And(_)));
            logical_chain(&b.right, ops, leaves);
        }
        other => leaves.push(other),
    }
}

impl<'ast> Visit<'ast> for Complexity {
    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        self.cognitive += 1 + self.nesting;
        self.visit_if_chain(node);
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        self.decisions += node.arms.len().saturating_sub(1);
        self.visit_expr(&node.expr);
        self.nested(|c| {
            for arm in &node.arms {
                if let Some((_, guard)) = &arm.guard {
                    c.decisions += 1;
                    c.visit_expr(guard);
                }
                c.visit_expr(&arm.body);
            }
        });
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.decisions += 1;
        self.visit_expr(&node.cond);
        self.nested(|c| c.visit_block(&node.body));
    }

    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.decisions += 1;
        self.visit_expr(&node.expr);
        self.nested(|c| c.visit_block(&node.body));
    }

    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        self.nested(|c| c.visit_block(&node.body));
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        self.deeper(|c| c.visit_expr(&node.body));
    }

    fn visit_expr_break(&mut self, node: &'ast syn::ExprBreak) {
        if node.label.is_some() {
            self.cognitive += 1;
        }
        visit::visit_expr_break(self, node);
    }

    fn visit_expr_continue(&mut self, node: &'ast syn::ExprContinue) {
        if node.label.is_some() {
            self.cognitive += 1;
        }
    }

    fn visit_expr_try(&mut self, node: &'ast syn::ExprTry) {
        self.try_ops += 1;
        visit::visit_expr_try(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if !is_logical(&node.op) {
            return visit::visit_expr_binary(self, node);
        }
        let mut ops = Vec::new();
        let mut leaves = Vec::new();
        logical_chain(&node.left, &mut ops, &mut leaves);
        ops.push(matches!(node.op, syn::BinOp::And(_)));
        logical_chain(&node.right, &mut ops, &mut leaves);
        self.decisions += ops.len();
        // Campbell: +1 for each run of the same operator.
        self.cognitive += ops.windows(2).filter(|w| w[0] != w[1]).count() + 1;
        for leaf in leaves {
            self.visit_expr(leaf);
        }
    }

    // Nested items are measured on their own.
    fn visit_item(&mut self, _: &'ast syn::Item) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics_of(src: &str) -> FunctionMetrics {
        let f: syn::ItemFn = syn::parse_str(src).expect("test source parses");
        let lines: Vec<&str> = src.lines().collect();
        measure("f".into(), &f.sig, &f.block, 1, lines.len(), &lines, false)
    }

    #[test]
    fn straight_line_code_is_one() {
        let m = metrics_of("fn f(a: u8, b: u8) -> u8 { a + b }");
        assert_eq!(
            (m.cyclomatic, m.cognitive, m.max_nesting, m.params),
            (1, 0, 0, 2)
        );
    }

    // Campbell's whitepaper, figure for `sumOfPrimes`: cognitive 7, cyclomatic 4.
    #[test]
    fn sum_of_primes_matches_the_whitepaper() {
        let m = metrics_of(
            "fn sum_of_primes(max: u32) -> u32 {
                let mut total = 0;
                'out: for i in 1..=max {
                    for j in 2..i {
                        if i % j == 0 {
                            continue 'out;
                        }
                    }
                    total += i;
                }
                total
            }",
        );
        assert_eq!(m.cognitive, 7);
        assert_eq!(m.cyclomatic, 4);
        assert_eq!(m.max_nesting, 3);
    }

    // Campbell's `getWords`: a switch with three cases plus default is cognitive 1, cyclomatic 4.
    #[test]
    fn match_is_one_cognitive_but_many_paths() {
        let m = metrics_of(
            "fn get_words(n: u32) -> &'static str {
                match n { 1 => \"one\", 2 => \"a couple\", 3 => \"a few\", _ => \"lots\" }
            }",
        );
        assert_eq!((m.cognitive, m.cyclomatic), (1, 4));
    }

    #[test]
    fn mixed_logical_operators_count_per_run() {
        let m = metrics_of("fn f(a: bool, b: bool, c: bool, d: bool) -> bool { a && b && c || d }");
        assert_eq!(m.cognitive, 2);
        assert_eq!(m.cyclomatic, 4);
    }

    #[test]
    fn else_if_does_not_nest() {
        let m = metrics_of("fn f(x: i32) -> i32 { if x > 0 { 1 } else if x < 0 { 2 } else { 3 } }");
        assert_eq!(m.cognitive, 3);
        assert_eq!(m.cyclomatic, 3);
    }

    #[test]
    fn comments_and_blank_lines_are_not_code() {
        let lines = [
            "fn f() {",
            "",
            "    // hi",
            "    /* a",
            "       b */",
            "    g();",
            "}",
        ];
        assert_eq!(code_lines(&lines, 1, 7), 3);
    }
}
