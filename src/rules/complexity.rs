//! Rules driven by per-function metrics.

use super::{Input, Rule, RuleMeta, Sink};
use crate::finding::{Category, Evidence, Severity};

pub struct CognitiveComplexity;
pub struct LongFunction;
pub struct DeepNesting;
pub struct TooManyParams;
pub struct BoolParams;

static COGNITIVE: RuleMeta = RuleMeta {
    id: "high-cognitive-complexity",
    category: Category::Maintainability,
    severity: Severity::Warning,
    evidence: Evidence::Moderate,
    default_enabled: true,
    summary: "Function is hard to understand (cognitive complexity above threshold)",
    rationale: "Cognitive complexity (SonarSource) adds one point per break in linear control flow \
and extra points for nesting. In a meta-analysis of 10 comprehension studies, Muñoz Barón, Wyrich \
and Wagner (ESEM 2020) found it correlates with measured comprehension time and subjective \
difficulty. Our own mining of Rust repositories (docs/EVIDENCE.md) tests whether functions above \
the threshold receive more bug fixes than functions of the same size.",
    sources: &[
        "G. A. Campbell. Cognitive Complexity: A new way of measuring understandability. SonarSource white paper, 2018.",
        "Muñoz Barón, Wyrich, Wagner. An Empirical Validation of Cognitive Complexity as a Measure of Source Code Understandability. ESEM 2020. doi:10.1145/3382494.3410636",
        "validation/mining (this repository)",
    ],
};

static LONG: RuleMeta = RuleMeta {
    id: "long-function",
    category: Category::Maintainability,
    severity: Severity::Info,
    evidence: Evidence::Weak,
    default_enabled: true,
    summary: "Function has more lines of code than the threshold",
    rationale: "Long functions are the classic 'Long Method' smell. Size is the single most \
reliable predictor of defect counts, but that is largely because bigger code has more places for \
bugs; per-line defect density does not clearly rise with function length (El Emam et al. 2001 \
on the size confounding effect). Reported as info: a long function is worth a look, not proof of \
a problem.",
    sources: &[
        "M. Fowler. Refactoring, 2nd ed. 2018 — 'Long Function'.",
        "El Emam, Benlarbi, Goel, Rai. The confounding effect of class size on the validity of object-oriented metrics. IEEE TSE 27(7), 2001.",
        "Clippy `too_many_lines` (default 100).",
    ],
};

static NESTING: RuleMeta = RuleMeta {
    id: "deep-nesting",
    category: Category::Maintainability,
    severity: Severity::Warning,
    evidence: Evidence::Moderate,
    default_enabled: true,
    summary: "Control flow is nested deeper than the threshold",
    rationale: "Every nesting level adds context the reader has to hold in mind. Controlled \
experiments (e.g. Johnson et al., 'An Empirical Study Assessing Source Code Readability in \
Comprehension', ICSME 2019) found deeper nesting reduced comprehension. Early returns, `?`, \
`let ... else` and helper functions flatten the code.",
    sources: &[
        "Johnson, Lubo, Yedla, Cáceres, Erwig. An Empirical Study Assessing Source Code Readability in Comprehension. ICSME 2019. doi:10.1109/ICSME.2019.00085",
        "validation/mining (this repository)",
    ],
};

static PARAMS: RuleMeta = RuleMeta {
    id: "too-many-params",
    category: Category::Maintainability,
    severity: Severity::Warning,
    evidence: Evidence::Weak,
    default_enabled: true,
    summary: "Function takes more parameters than the threshold",
    rationale: "Long parameter lists are hard to call correctly: arguments of the same type can be \
swapped without a compile error. Clippy's `too_many_arguments` (complexity group, default 7) \
flags them. Group related values into a struct or use a builder. Trait implementations are \
skipped because the trait dictates the signature.",
    sources: &[
        "Clippy lint `too_many_arguments`.",
        "M. Fowler. Refactoring, 2nd ed. 2018 — 'Long Parameter List'.",
    ],
};

static BOOLS: RuleMeta = RuleMeta {
    id: "bool-params",
    category: Category::Maintainability,
    severity: Severity::Info,
    evidence: Evidence::Weak,
    default_enabled: true,
    summary: "Function takes two or more `bool` parameters",
    rationale: "A call like `render(true, false)` cannot be read without looking up the signature, \
and swapping the arguments compiles. The Rust API Guidelines (C-CUSTOM-TYPE) recommend enums or \
dedicated types instead of `bool` arguments. Clippy has `fn_params_excessive_bools` (pedantic).",
    sources: &[
        "Rust API Guidelines, C-CUSTOM-TYPE: https://rust-lang.github.io/api-guidelines/type-safety.html#c-custom-type",
        "Clippy lint `fn_params_excessive_bools`.",
    ],
};

impl Rule for CognitiveComplexity {
    fn meta(&self) -> &'static RuleMeta {
        &COGNITIVE
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        let max = input.config.max_cognitive_complexity;
        for f in input.fns.iter().filter(|f| !f.is_test && f.cognitive > max) {
            sink.emit(f.line, f.column, format!("`{}` has cognitive complexity {} (max {max})", f.name, f.cognitive));
        }
    }
}

impl Rule for LongFunction {
    fn meta(&self) -> &'static RuleMeta {
        &LONG
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        let max = input.config.max_function_lines;
        for f in input.fns.iter().filter(|f| !f.is_test && f.loc > max) {
            sink.emit(f.line, f.column, format!("`{}` has {} lines of code (max {max})", f.name, f.loc));
        }
    }
}

impl Rule for DeepNesting {
    fn meta(&self) -> &'static RuleMeta {
        &NESTING
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        let max = input.config.max_nesting;
        for f in input.fns.iter().filter(|f| !f.is_test && f.max_nesting > max) {
            sink.emit(f.line, f.column, format!("`{}` nests control flow {} levels deep (max {max})", f.name, f.max_nesting));
        }
    }
}

impl Rule for TooManyParams {
    fn meta(&self) -> &'static RuleMeta {
        &PARAMS
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        let max = input.config.max_params;
        for f in input.fns.iter().filter(|f| !f.is_trait_impl && f.params > max) {
            sink.emit(f.line, f.column, format!("`{}` takes {} parameters (max {max})", f.name, f.params));
        }
    }
}

impl Rule for BoolParams {
    fn meta(&self) -> &'static RuleMeta {
        &BOOLS
    }
    fn check(&self, input: &Input, sink: &mut Sink) {
        for f in input.fns.iter().filter(|f| !f.is_trait_impl && !f.is_test && f.bool_params >= 2) {
            sink.emit(
                f.line,
                f.column,
                format!("`{}` takes {} `bool` parameters; consider enums or a options struct", f.name, f.bool_params),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rules::testing::check_rule;

    #[test]
    fn flags_complex_function() {
        let mut body = String::from("fn f(x: i32) {\n");
        for i in 0..30 {
            body.push_str(&format!("    if x == {i} {{ if x > 0 {{ println!(\"a\"); }} }}\n"));
        }
        body.push_str("}\n");
        assert_eq!(check_rule("high-cognitive-complexity", &body).len(), 1);
        assert_eq!(check_rule("high-cognitive-complexity", "fn f() { if true {} }").len(), 0);
    }

    #[test]
    fn flags_deep_nesting() {
        let src = "fn f(x: bool) { if x { if x { if x { if x { if x { if x { } } } } } } }";
        assert_eq!(check_rule("deep-nesting", src).len(), 1);
        assert_eq!(check_rule("deep-nesting", "fn f(x: bool) { if x { if x {} } }").len(), 0);
    }

    #[test]
    fn params_skip_trait_impls() {
        let src = "fn f(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) {}\n\
                   trait T { fn t(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8); }\n\
                   struct S; impl T for S { fn t(a: u8, b: u8, c: u8, d: u8, e: u8, f: u8, g: u8, h: u8) {} }";
        assert_eq!(check_rule("too-many-params", src).len(), 1);
    }

    #[test]
    fn bool_params() {
        assert_eq!(check_rule("bool-params", "fn f(a: bool, b: bool) {}").len(), 1);
        assert_eq!(check_rule("bool-params", "fn f(a: bool, b: u8) {}").len(), 0);
    }

    #[test]
    fn long_function() {
        let mut src = String::from("fn f() {\n");
        for _ in 0..120 {
            src.push_str("    let _x = 1;\n");
        }
        src.push_str("}\n");
        assert_eq!(check_rule("long-function", &src).len(), 1);
    }
}
