//! Size and complexity limits. Thresholds and whether each rule earns a place at all come from
//! the study in `study/` (results in docs/EVIDENCE.md), not from taste.

use syn::spanned::Spanned;

use crate::analyzer::Analyzer;

/// SonarSource's default for cognitive complexity per function.
pub const MAX_COGNITIVE: usize = 15;
/// Clippy's `too_many_lines` default.
pub const MAX_FUNCTION_LINES: usize = 100;
/// Clippy's `too_many_arguments` default: seven is allowed, eight is too many.
pub const MAX_PARAMS: usize = 7;
pub const MAX_NESTING: usize = 5;

pub fn function(a: &mut Analyzer, sig: &syn::Signature) {
    if a.scope().test {
        return;
    }
    // The analyzer pushes this function's metrics just before running its checks.
    let Some(m) = a.report.functions.last().cloned() else {
        return;
    };
    let span = sig.ident.span();
    if m.cognitive > MAX_COGNITIVE {
        a.emit(
            "complex-function",
            span,
            format!(
                "`{}` has cognitive complexity {} (limit {MAX_COGNITIVE}); split out the decisions",
                m.name, m.cognitive
            ),
        );
    }
    if m.code_lines > MAX_FUNCTION_LINES {
        a.emit(
            "long-function",
            span,
            format!(
                "`{}` is {} lines of code (limit {MAX_FUNCTION_LINES})",
                m.name, m.code_lines
            ),
        );
    }
    if m.max_nesting > MAX_NESTING {
        a.emit(
            "deep-nesting",
            span,
            format!("`{}` nests {} levels deep (limit {MAX_NESTING}); return early or extract the inner part", m.name, m.max_nesting),
        );
    }
    if m.params > MAX_PARAMS {
        a.emit(
            "too-many-params",
            sig.inputs.span(),
            format!(
                "`{}` takes {} parameters (limit {MAX_PARAMS}); group related ones into a struct",
                m.name, m.params
            ),
        );
    }
}

pub fn file(_a: &mut Analyzer) {}
