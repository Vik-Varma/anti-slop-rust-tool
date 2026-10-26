use serde::Deserialize;
use std::path::Path;

/// Thresholds and rule selection. Loaded from `anti-slop.toml` when present.
///
/// Default thresholds are justified in `docs/EVIDENCE.md`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Cognitive complexity above which a function is flagged (SonarSource default: 15).
    pub max_cognitive_complexity: usize,
    /// Code lines above which a function is flagged.
    pub max_function_lines: usize,
    /// Nesting depth above which a function is flagged.
    pub max_nesting: usize,
    /// Parameters above which a function is flagged (Clippy default: 7).
    pub max_params: usize,
    /// Minimum number of tokens for a duplicated code block to be reported.
    pub min_duplicate_tokens: usize,
    /// Rule ids to disable.
    pub disable: Vec<String>,
    /// If non-empty, only these rule ids run.
    pub only: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_cognitive_complexity: 25,
            max_function_lines: 100,
            max_nesting: 5,
            max_params: 7,
            min_duplicate_tokens: 120,
            disable: Vec::new(),
            only: Vec::new(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Config, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn rule_enabled(&self, id: &str) -> bool {
        (self.only.is_empty() || self.only.iter().any(|r| r == id)) && !self.disable.iter().any(|r| r == id)
    }
}
