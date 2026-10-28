//! The measuring-stick and handoff report: what a change did to the things that judge it (tests,
//! benchmarks, profiles, lint levels), what it suppressed or deferred, and how much code it
//! added. Every item here came from a cheat an agent was seen to make (docs/HARNESS.md); none of
//! them is a verdict on the code, which is `check`'s job.

use std::fs;
use std::path::{Path, PathBuf};

use super::diff::{self, Base, Changed};
use super::policy::Policy;
use crate::metrics;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub kind: &'static str,
    pub blocking: bool,
    pub path: PathBuf,
    pub line: Option<usize>,
    pub detail: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub items: Vec<Item>,
    /// Code lines (not blank, not comment) added and removed across changed Rust files.
    pub lines_added: usize,
    pub lines_removed: usize,
    pub files_changed: usize,
}

impl Report {
    pub fn blocking(&self) -> usize {
        self.items.iter().filter(|i| i.blocking).count()
    }
}

const PROFILE_KEYS: &[&str] = &[
    "[profile",
    "opt-level",
    "lto",
    "codegen-units",
    "panic",
    "debug-assertions",
    "overflow-checks",
    "[lints",
];

/// Which measuring stick a changed file is, if any.
fn measuring_stick(c: &Changed) -> Option<&'static str> {
    let p = c.path.to_string_lossy().replace('\\', "/");
    let name = c
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let in_dir = |d: &str| p.starts_with(&format!("{d}/")) || p.contains(&format!("/{d}/"));
    if in_dir("benches") || in_dir("bench") {
        Some("benchmark")
    } else if in_dir("tests") || in_dir("test") || name.contains("test") {
        Some("test file")
    } else if name == "Cargo.toml" {
        let touched = |l: &str| PROFILE_KEYS.iter().any(|k| l.contains(k));
        (c.added.iter().any(|(_, l)| touched(l)) || c.removed.iter().any(|l| touched(l)))
            .then_some("build profile or lint level")
    } else if p.starts_with(".cargo/") || p.contains("/.cargo/") {
        Some("cargo config")
    } else if name.starts_with("rust-toolchain") {
        Some("toolchain")
    } else if p.starts_with(".github/workflows/") {
        Some("CI workflow")
    } else if matches!(
        name,
        "deny.toml" | "clippy.toml" | "rustfmt.toml" | ".rustfmt.toml"
    ) {
        Some("lint or format config")
    } else {
        None
    }
}

fn count(lines: impl Iterator<Item = impl AsRef<str>>, needle: &str) -> usize {
    lines.filter(|l| l.as_ref().contains(needle)).count()
}

fn is_assertion(l: &str) -> bool {
    let t = l.trim_start();
    ["assert!", "assert_eq!", "assert_ne!", "debug_assert!"]
        .iter()
        .any(|a| t.starts_with(a))
}

fn code_lines(src: &str) -> usize {
    let lines: Vec<&str> = src.lines().collect();
    metrics::code_lines(&lines, 1, lines.len())
}

/// Runs every detector over `changes`.
pub fn run(root: &Path, base: &Base, policy: &Policy, changes: &[Changed]) -> Report {
    let mut r = Report {
        files_changed: changes.len(),
        ..Report::default()
    };
    let mut push = |kind: &'static str, path: &Path, line: Option<usize>, detail: String| {
        r.items.push(Item {
            kind,
            blocking: policy.blocks(kind),
            path: path.to_path_buf(),
            line,
            detail,
        });
    };
    for c in changes {
        if c.binary {
            continue;
        }
        if let Some(what) = measuring_stick(c) {
            push(
                "measuring-stick",
                &c.path,
                None,
                format!(
                    "{what} changed (+{} -{} lines{})",
                    c.added.len(),
                    c.removed.len(),
                    if c.deleted { ", deleted" } else { "" }
                ),
            );
        }
        for (n, l) in &c.added {
            if l.contains("target-cpu=native") || l.contains("RUSTFLAGS") || l.contains("rustflags")
            {
                push("flags", &c.path, Some(*n), format!("`{}`", l.trim()));
            }
        }
        if !c.is_rust() {
            continue;
        }
        for (n, l) in &c.added {
            let t = l.trim();
            if let Some(rest) = t.split("anti-slop: allow(").nth(1) {
                let (rule, reason) = rest.split_once(')').unwrap_or((rest, ""));
                let reason = reason.trim();
                if reason.chars().count() < policy.min_reason_chars {
                    push(
                        "suppression-without-reason",
                        &c.path,
                        Some(*n),
                        format!(
                            "`allow({rule})` with {}",
                            if reason.is_empty() {
                                "no reason".to_string()
                            } else {
                                format!("a {}-character reason", reason.chars().count())
                            }
                        ),
                    );
                } else {
                    push(
                        "suppressions",
                        &c.path,
                        Some(*n),
                        format!("`allow({rule})`: {reason}"),
                    );
                }
            }
            if t.starts_with("#[allow(") || t.starts_with("#![allow(") || t.starts_with("#[expect(")
            {
                push("allow-attributes", &c.path, Some(*n), format!("`{t}`"));
            }
            if t.contains("#[ignore") {
                push(
                    "tests-ignored",
                    &c.path,
                    Some(*n),
                    "`#[ignore]` added".into(),
                );
            }
            if t.contains("todo!(") || t.contains("unimplemented!(") {
                push(
                    "deferrals",
                    &c.path,
                    Some(*n),
                    format!("`{}`", t.split('(').next().unwrap_or(t)),
                );
            } else if let Some(comment) = t.split("//").nth(1)
                && ["TODO", "FIXME", "HACK", "XXX"]
                    .iter()
                    .any(|m| comment.contains(m))
            {
                push(
                    "deferrals",
                    &c.path,
                    Some(*n),
                    format!("`//{}`", comment.trim_end()),
                );
            }
        }
        let tests_removed = count(c.removed.iter(), "#[test]");
        let tests_added = count(c.added.iter().map(|(_, l)| l), "#[test]");
        if tests_removed > tests_added {
            push(
                "tests-removed",
                &c.path,
                None,
                format!("{} `#[test]` removed, {} added", tests_removed, tests_added),
            );
        }
        if measuring_stick(c) == Some("test file") {
            let removed = c.removed.iter().filter(|l| is_assertion(l)).count();
            let added = c.added.iter().filter(|(_, l)| is_assertion(l)).count();
            if removed > added {
                push(
                    "assertions-removed",
                    &c.path,
                    None,
                    format!("{removed} assertions removed, {added} added"),
                );
            }
        }
        // Code lines, not raw lines, so deleting comments doesn't count as shrinking.
        let old = c
            .old_path
            .as_deref()
            .or(Some(c.path.as_path()))
            .and_then(|p| diff::old_content(root, base, p))
            .map(|s| code_lines(&s))
            .unwrap_or(0);
        let new = if c.deleted {
            0
        } else {
            fs::read_to_string(root.join(&c.path))
                .map(|s| code_lines(&s))
                .unwrap_or(0)
        };
        r.lines_added += new.saturating_sub(old);
        r.lines_removed += old.saturating_sub(new);
        if new > policy.max_file_lines {
            push(
                "size",
                &c.path,
                None,
                format!(
                    "{new} code lines, over the {}-line limit",
                    policy.max_file_lines
                ),
            );
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn changed(path: &str, added: &[(usize, &str)], removed: &[&str]) -> Changed {
        Changed {
            path: PathBuf::from(path),
            added: added.iter().map(|(n, l)| (*n, l.to_string())).collect(),
            removed: removed.iter().map(|l| l.to_string()).collect(),
            ..Changed::default()
        }
    }

    #[test]
    fn detectors() {
        let policy = Policy::default();
        let changes = vec![
            changed("benches/speed.rs", &[(1, "fn b() {}")], &[]),
            changed(
                ".cargo/config.toml",
                &[(2, "rustflags = [\"-C\", \"target-cpu=native\"]")],
                &[],
            ),
            changed("Cargo.toml", &[(5, "serde = \"1\"")], &[]),
            changed(
                "src/lib.rs",
                &[
                    (3, "    // anti-slop: allow(todo) wip"),
                    (
                        4,
                        "    // anti-slop: allow(unwrap) the index is bounded by the loop above",
                    ),
                    (5, "#[allow(dead_code)]"),
                    (6, "    todo!()"),
                    (7, "    x += 1; // TODO handle overflow"),
                ],
                &["#[test]", "#[test]"],
            ),
            changed(
                "tests/it.rs",
                &[(9, "#[ignore]"), (10, "#[test]")],
                &["assert_eq!(a, b);", "assert!(c);"],
            ),
        ];
        let r = run(
            Path::new("/nonexistent"),
            &Base::Rev("HEAD".into()),
            &policy,
            &changes,
        );
        let kinds: Vec<(&str, bool)> = r.items.iter().map(|i| (i.kind, i.blocking)).collect();
        assert_eq!(
            kinds,
            vec![
                ("measuring-stick", false),
                ("measuring-stick", false),
                ("flags", true),
                ("suppression-without-reason", true),
                ("suppressions", false),
                ("allow-attributes", false),
                ("deferrals", false),
                ("deferrals", false),
                ("tests-removed", true),
                ("measuring-stick", false),
                ("tests-ignored", true),
                ("assertions-removed", false),
            ],
            "{:#?}",
            r.items
        );
        // Dependency edits to Cargo.toml aren't a measuring stick.
        assert!(!r.items.iter().any(|i| i.path == Path::new("Cargo.toml")));
        assert_eq!(r.blocking(), 4);
    }
}
