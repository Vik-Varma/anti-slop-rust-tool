//! The harness policy: what blocks, what is only reported, and what to diff against. Read from
//! `anti-slop.toml` at the repository root; built in when there is none. Unknown keys are errors,
//! so a misspelt key can't silently loosen the policy.

use std::fs;
use std::io;
use std::path::Path;

use crate::Severity;
use crate::manifest::entries;

/// Every kind of item `guard` can raise. A policy lists which of them block.
pub const KINDS: &[(&str, &str)] = &[
    (
        "findings",
        "warnings and errors (or the configured severity) on changed lines",
    ),
    (
        "suppression-without-reason",
        "an `anti-slop: allow(..)` added with no reason, or a short one",
    ),
    ("suppressions", "any `anti-slop: allow(..)` added"),
    (
        "allow-attributes",
        "`#[allow(..)]` or `#[expect(..)]` added",
    ),
    (
        "tests-removed",
        "more `#[test]` functions removed than added in a file",
    ),
    ("tests-ignored", "`#[ignore]` added"),
    (
        "assertions-removed",
        "more assertions removed than added in a test file",
    ),
    (
        "deferrals",
        "`todo!()`, `unimplemented!()` or a TODO/FIXME/HACK comment added",
    ),
    (
        "flags",
        "`target-cpu=native`, `RUSTFLAGS` or `rustflags` added anywhere",
    ),
    (
        "measuring-stick",
        "a benchmark, test, profile, toolchain, lint-config or CI file changed",
    ),
    ("size", "a changed file over the line limit"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    /// The revision `--since` compares against when none is given.
    pub base: String,
    pub style: bool,
    pub skip: Vec<String>,
    /// What counts as a finding on changed lines.
    pub min_severity: Severity,
    pub block: Vec<String>,
    pub min_reason_chars: usize,
    pub max_file_lines: usize,
}

impl Default for Policy {
    fn default() -> Policy {
        Policy {
            base: "HEAD".into(),
            style: false,
            skip: Vec::new(),
            min_severity: Severity::Warning,
            block: [
                "findings",
                "suppression-without-reason",
                "tests-removed",
                "tests-ignored",
                "flags",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            min_reason_chars: 12,
            max_file_lines: 1000,
        }
    }
}

pub const FILE_NAME: &str = "anti-slop.toml";

impl Policy {
    /// The policy in `root/anti-slop.toml`, or the default when the file is absent.
    pub fn load(root: &Path) -> io::Result<Policy> {
        match fs::read_to_string(root.join(FILE_NAME)) {
            Ok(text) => Policy::parse(&text).map_err(io::Error::other),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Policy::default()),
            Err(e) => Err(e),
        }
    }

    pub fn parse(text: &str) -> Result<Policy, String> {
        let mut p = Policy::default();
        for (key, value) in entries(text) {
            match key.as_str() {
                "base" => p.base = value,
                "style" => p.style = parse_bool(&key, &value)?,
                "skip" => p.skip = parse_list(&key, &value)?,
                "min_severity" => {
                    p.min_severity = Severity::parse(&value)
                        .ok_or(format!("{FILE_NAME}: unknown severity `{value}`"))?;
                }
                "block" => {
                    let list = parse_list(&key, &value)?;
                    if let Some(bad) = list.iter().find(|k| !KINDS.iter().any(|(n, _)| n == k)) {
                        return Err(format!("{FILE_NAME}: unknown item kind `{bad}` in `block`"));
                    }
                    p.block = list;
                }
                "min_reason_chars" => p.min_reason_chars = parse_num(&key, &value)?,
                "max_file_lines" => p.max_file_lines = parse_num(&key, &value)?,
                other => return Err(format!("{FILE_NAME}: unknown key `{other}`")),
            }
        }
        Ok(p)
    }

    pub fn blocks(&self, kind: &str) -> bool {
        self.block.iter().any(|k| k == kind)
    }
}

fn parse_bool(key: &str, value: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("{FILE_NAME}: `{key}` must be true or false")),
    }
}

fn parse_num(key: &str, value: &str) -> Result<usize, String> {
    value
        .parse()
        .map_err(|_| format!("{FILE_NAME}: `{key}` must be a number"))
}

/// `["a", "b"]`, as the entries reader hands it over (quotes around each item intact).
fn parse_list(key: &str, value: &str) -> Result<Vec<String>, String> {
    let inner = value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .ok_or(format!(
            "{FILE_NAME}: `{key}` must be a list like [\"a\", \"b\"]"
        ))?;
    Ok(inner
        .split(',')
        .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_rejects_unknown_keys() {
        let p = Policy::parse(
            "base = \"origin/main\"\nstyle = true\nblock = [\"findings\", \"flags\"]\n\
             min_reason_chars = 20\nskip = [\"todo\"] # none\n",
        )
        .expect("valid");
        assert_eq!(p.base, "origin/main");
        assert!(p.style && p.blocks("flags") && !p.blocks("tests-removed"));
        assert_eq!(p.min_reason_chars, 20);
        assert_eq!(p.skip, vec!["todo".to_string()]);
        assert!(Policy::parse("blok = [\"findings\"]\n").is_err());
        assert!(Policy::parse("block = [\"typo\"]\n").is_err());
        assert!(Policy::parse("min_severity = \"loud\"\n").is_err());
    }
}
