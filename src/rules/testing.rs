//! Test helpers for rules.

use crate::analyze::analyze_source;
use crate::config::Config;
use crate::finding::Finding;
use std::path::{Path, PathBuf};

/// Runs a single rule on `src` as if it were `src/lib.rs`.
pub fn check_rule(id: &str, src: &str) -> Vec<Finding> {
    check_rule_at(id, "src/lib.rs", src)
}

/// Runs a single rule on `src` as if it lived at `path` (relative to the crate root).
pub fn check_rule_at(id: &str, path: &str, src: &str) -> Vec<Finding> {
    let rule = super::find(id).unwrap_or_else(|| panic!("no rule {id}"));
    let config = Config { only: vec![id.to_string()], ..Config::default() };
    let report = analyze_source(Path::new(path), PathBuf::from(path), Path::new(path), src, &config, &[rule]);
    assert!(report.parse_error.is_none(), "parse error: {:?}", report.parse_error);
    report.findings
}

/// Lines of the findings, for compact assertions.
pub fn lines(findings: &[Finding]) -> Vec<usize> {
    findings.iter().map(|f| f.line).collect()
}
