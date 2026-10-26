use serde::Serialize;
use std::fmt;
use std::path::PathBuf;

/// What kind of damage the flagged pattern does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    /// Likely to produce wrong results, crashes, deadlocks or undefined behaviour.
    Correctness,
    /// Hides or mishandles failures so that errors go unnoticed.
    Robustness,
    /// Measurable, avoidable runtime cost.
    Performance,
    /// Makes code harder to understand or change; associated with defects.
    Maintainability,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Category::Correctness => "correctness",
            Category::Robustness => "robustness",
            Category::Performance => "performance",
            Category::Maintainability => "maintainability",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        })
    }
}

impl std::str::FromStr for Severity {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "info" => Ok(Severity::Info),
            "warning" | "warn" => Ok(Severity::Warning),
            "error" => Ok(Severity::Error),
            other => Err(format!("unknown severity `{other}` (expected info, warning or error)")),
        }
    }
}

/// How well the claim that this pattern is harmful holds up.
///
/// Every rule records the grade it earned in the evidence review
/// (see `docs/EVIDENCE.md`), so users can decide how much to trust it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Evidence {
    /// Harm reproduced by our own tests/benchmarks *and* documented by
    /// peer-reviewed studies, official docs or real incidents.
    Strong,
    /// Harm reproduced or documented, but context-dependent.
    Moderate,
    /// Plausible and widely recommended, but empirical support is thin.
    Weak,
}

impl fmt::Display for Evidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Evidence::Strong => "strong",
            Evidence::Moderate => "moderate",
            Evidence::Weak => "weak",
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule: &'static str,
    pub category: Category,
    pub severity: Severity,
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub message: String,
}
