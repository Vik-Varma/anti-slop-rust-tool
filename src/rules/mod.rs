//! The rule catalogue. Each rule documents why the pattern counts as bad
//! code and how strong the supporting evidence is.

use crate::config::Config;
use crate::context::FileContext;
use crate::finding::{Category, Evidence, Finding, Severity};
use crate::metrics::FnMetrics;

pub struct RuleMeta {
    pub id: &'static str,
    pub category: Category,
    pub severity: Severity,
    pub evidence: Evidence,
    /// One-line description of what is flagged.
    pub summary: &'static str,
    /// Why it is bad, what the evidence says, and when it is acceptable.
    pub rationale: &'static str,
    /// Where the claim comes from and how we verified it.
    pub sources: &'static [&'static str],
}

/// Inputs shared by all rules for one file.
pub struct Input<'c, 'a> {
    pub ctx: &'c FileContext<'a>,
    pub fns: &'c [FnMetrics],
    pub config: &'c Config,
}

pub trait Rule: Send + Sync {
    fn meta(&self) -> &'static RuleMeta;
    fn check(&self, input: &Input, sink: &mut Sink);
}

/// Collects findings for one rule, applying suppressions.
pub struct Sink<'c, 'a> {
    ctx: &'c FileContext<'a>,
    meta: &'static RuleMeta,
    pub findings: Vec<Finding>,
}

impl<'c, 'a> Sink<'c, 'a> {
    pub fn new(ctx: &'c FileContext<'a>, meta: &'static RuleMeta) -> Self {
        Sink { ctx, meta, findings: Vec::new() }
    }

    pub fn emit(&mut self, line: usize, column: usize, message: impl Into<String>) {
        if self.ctx.is_suppressed(self.meta.id, line) {
            return;
        }
        self.findings.push(Finding {
            rule: self.meta.id,
            category: self.meta.category,
            severity: self.meta.severity,
            file: self.ctx.display_path.clone(),
            line,
            column,
            message: message.into(),
        });
    }

    pub fn emit_span(&mut self, span: proc_macro2::Span, message: impl Into<String>) {
        let start = span.start();
        self.emit(start.line, start.column + 1, message);
    }
}

pub fn all_rules() -> Vec<Box<dyn Rule>> {
    vec![]
}

pub fn find(id: &str) -> Option<Box<dyn Rule>> {
    all_rules().into_iter().find(|r| r.meta().id == id)
}
