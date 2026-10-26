//! The rule catalogue: what each rule flags, why that is bad, and the evidence. Only claims that
//! survived the checks in docs/EVIDENCE.md are here; the ones that didn't are listed there too.

use crate::Severity;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    /// Wrong results, crashes or undefined behaviour.
    Correctness,
    /// Errors that vanish, so a failure goes unnoticed.
    ErrorHandling,
    /// Deadlocks and stalls.
    Concurrency,
    /// Code that measurably costs more to change safely.
    Maintainability,
}

impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Correctness => "correctness",
            Category::ErrorHandling => "error-handling",
            Category::Concurrency => "concurrency",
            Category::Maintainability => "maintainability",
        }
    }
}

#[derive(Debug)]
pub struct Rule {
    pub id: &'static str,
    pub category: Category,
    pub severity: Severity,
    /// One line: what it flags.
    pub summary: &'static str,
    /// The failure it causes, as demonstrated (see `experiments/`).
    pub harm: &'static str,
    /// What the check can get wrong without type information.
    pub false_positives: &'static str,
    pub sources: &'static [&'static str],
}

pub fn all() -> &'static [Rule] {
    RULES
}

pub fn get(id: &str) -> Option<&'static Rule> {
    RULES.iter().find(|r| r.id == id)
}

static RULES: &[Rule] = &[];
