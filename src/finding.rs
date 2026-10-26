use std::fmt;
use std::path::PathBuf;

/// How sure we are that a finding is a real defect, and how much it costs if it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Worth a look in review; the evidence says it raises risk, not that it is wrong.
    Note,
    /// A demonstrated failure mode that needs a reason to stay.
    Warning,
    /// Undefined behaviour, a guaranteed panic, or a lost error with no way to recover.
    Error,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Note => "note",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }

    pub fn parse(s: &str) -> Option<Severity> {
        match s {
            "note" => Some(Severity::Note),
            "warning" => Some(Severity::Warning),
            "error" => Some(Severity::Error),
            _ => None,
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub rule: &'static str,
    pub severity: Severity,
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub message: String,
    /// The enclosing function, as `Type::method` or `function`, when there is one.
    pub function: Option<String>,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}[{}]: {}",
            self.path.display(),
            self.line,
            self.column,
            self.severity,
            self.rule,
            self.message
        )
    }
}
