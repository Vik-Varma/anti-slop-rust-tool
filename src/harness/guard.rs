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

fn is_assertion(l: &str) -> bool {
    let t = l.trim_start();
    ["assert!", "assert_eq!", "assert_ne!", "debug_assert!"]
        .iter()
        .any(|a| t.starts_with(a))
}

/// One file's changed lines, split by hunk. A change built by hand, with no hunk offsets, is one
/// hunk.
fn hunks<'a, T>(items: &'a [T], starts: &[usize]) -> Vec<&'a [T]> {
    if starts.is_empty() {
        return if items.is_empty() {
            Vec::new()
        } else {
            vec![items]
        };
    }
    let mut out = Vec::new();
    for (i, &s) in starts.iter().enumerate() {
        let e = starts.get(i + 1).copied().unwrap_or(items.len());
        if s < e {
            out.push(&items[s..e]);
        }
    }
    out
}

/// `#[test]`, `#[tokio::test]`, `#[tokio::test(flavor = "..")]` and the like.
fn is_test_attr(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("#[test]")
        || (t.starts_with("#[") && (t.contains("::test]") || t.contains("::test(")))
}

/// The name in `pub async fn name(..)`, if the line declares a function.
fn fn_name(line: &str) -> Option<String> {
    let t = line.trim_start();
    let i = t.find("fn ")?;
    if i > 0 && !t[..i].ends_with(' ') {
        return None;
    }
    let name: String = t[i + 3..]
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// The function that the attributes just above `lines` belong to: the first line that is neither
/// an attribute, a comment nor blank.
fn following_fn<'a>(lines: impl Iterator<Item = &'a str>) -> Option<String> {
    for l in lines {
        let t = l.trim();
        if t.is_empty() || t.starts_with("#[") || t.starts_with("//") {
            continue;
        }
        return fn_name(t);
    }
    None
}

/// The tests declared in one hunk: `Some(name)` for a test attribute with its function in the same
/// hunk, `None` for an attribute whose function is not part of the hunk.
fn hunk_tests(lines: &[&str]) -> Vec<Option<String>> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| is_test_attr(l))
        .map(|(i, _)| following_fn(lines[i + 1..].iter().copied()))
        .collect()
}

fn code_lines(src: &str) -> usize {
    let lines: Vec<&str> = src.lines().collect();
    metrics::code_lines(&lines, 1, lines.len())
}

/// Files where a compiler flag takes effect, as opposed to being mentioned: cargo config,
/// manifests, CI, scripts, build scripts. Prose and ordinary source only talk about flags.
fn carries_flags(c: &Changed) -> bool {
    let p = c.path.to_string_lossy().replace('\\', "/");
    let name = c
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    p.contains(".cargo/")
        || name == "Cargo.toml"
        || name == "build.rs"
        || p.starts_with(".github/workflows/")
        || matches!(name, "Makefile" | "justfile" | "Justfile" | "Dockerfile")
        || name.ends_with(".sh")
        || name.ends_with(".yml")
        || name.ends_with(".yaml")
        || name.starts_with("rust-toolchain")
}

/// A source line split into its code, with string literals blanked out, and its comment, if
/// any. Markers inside a string literal (`"todo!("`, a quoted `// anti-slop: allow(..)`) are
/// mentions, not the thing, and this is what keeps the detectors off them.
fn split_line(line: &str) -> (String, Option<&str>) {
    let mut code = String::with_capacity(line.len());
    let mut in_string = false;
    let mut chars = line.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if in_string {
            match c {
                '\\' => {
                    chars.next();
                }
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '/' if matches!(chars.peek(), Some((_, '/')) | Some((_, '*'))) => {
                return (code, Some(&line[i..]));
            }
            _ => code.push(c),
        }
    }
    (code, None)
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
    // Tests are matched by name across the whole change, so a test moved to another file is not a
    // removed test, and a test that only gained `#[ignore]` can be told from a new one.
    let mut removed: Vec<(PathBuf, String)> = Vec::new();
    let mut removed_unnamed: Vec<PathBuf> = Vec::new();
    let mut added: Vec<String> = Vec::new();
    let mut added_unnamed = 0usize;
    for c in changes.iter().filter(|c| c.is_rust() && !c.binary) {
        for h in hunks(&c.removed, &c.removed_starts) {
            let lines: Vec<&str> = h.iter().map(String::as_str).collect();
            for t in hunk_tests(&lines) {
                match t {
                    Some(n) => removed.push((c.path.clone(), n)),
                    None => removed_unnamed.push(c.path.clone()),
                }
            }
        }
        for h in hunks(&c.added, &c.added_starts) {
            let lines: Vec<&str> = h.iter().map(|(_, l)| l.as_str()).collect();
            for t in hunk_tests(&lines) {
                match t {
                    Some(n) => added.push(n),
                    None => added_unnamed += 1,
                }
            }
        }
    }
    let removed_names: std::collections::HashSet<&str> =
        removed.iter().map(|(_, n)| n.as_str()).collect();
    let added_names: std::collections::HashSet<&str> = added.iter().map(String::as_str).collect();
    let gone: Vec<&(PathBuf, String)> = removed
        .iter()
        .filter(|(_, n)| !added_names.contains(n.as_str()))
        .collect();
    let fresh: Vec<&String> = added
        .iter()
        .filter(|n| !removed_names.contains(n.as_str()))
        .collect();
    for (i, (path, name)) in gone.iter().enumerate() {
        match fresh.get(i) {
            Some(new) => push(
                "tests-renamed",
                path,
                None,
                format!(
                    "`{name}` is gone and `{new}` was added; a rename or a replacement, check which"
                ),
            ),
            None => push(
                "tests-removed",
                path,
                None,
                format!("test `{name}` is gone and no new test replaces it"),
            ),
        }
    }
    for path in removed_unnamed
        .iter()
        .take(removed_unnamed.len().saturating_sub(added_unnamed))
    {
        push(
            "tests-removed",
            path,
            None,
            "a test attribute was removed, so the function it was on no longer runs as a test"
                .into(),
        );
    }
    let asserts_removed = changes
        .iter()
        .filter(|c| c.is_rust() && measuring_stick(c) == Some("test file"))
        .flat_map(|c| {
            c.removed
                .iter()
                .filter(|l| is_assertion(l))
                .map(move |_| &c.path)
        })
        .collect::<Vec<_>>();
    let asserts_added: usize = changes
        .iter()
        .filter(|c| c.is_rust())
        .map(|c| c.added.iter().filter(|(_, l)| is_assertion(l)).count())
        .sum();
    if asserts_removed.len() > asserts_added {
        push(
            "assertions-removed",
            asserts_removed[0],
            None,
            format!(
                "{} assertions removed from test files, {} added anywhere in the change",
                asserts_removed.len(),
                asserts_added
            ),
        );
    }
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
        if carries_flags(c) {
            for (n, l) in &c.added {
                if l.contains("target-cpu=native")
                    || l.contains("RUSTFLAGS")
                    || l.contains("rustflags")
                {
                    push("flags", &c.path, Some(*n), format!("`{}`", l.trim()));
                }
            }
        }
        if !c.is_rust() {
            continue;
        }
        // Tests are left out of the deferral check: their fixtures quote these, and the rules
        // exempt tests too.
        let test_file = measuring_stick(c) == Some("test file");
        for (n, l) in &c.added {
            let t = l.trim();
            let (code, comment) = split_line(t);
            // Only a suppression of a real rule counts; prose quoting the syntax doesn't.
            if let Some(rest) = comment.and_then(|c| c.split("anti-slop: allow(").nth(1))
                && let Some((rule, reason)) = rest.split_once(')')
                && crate::rules::get(rule.trim()).is_some()
            {
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
            if code.starts_with("#[allow(")
                || code.starts_with("#![allow(")
                || code.starts_with("#[expect(")
            {
                push("allow-attributes", &c.path, Some(*n), format!("`{t}`"));
            }
            if test_file {
                continue;
            }
            if let Some(mac) = ["todo!(", "unimplemented!("]
                .iter()
                .find(|m| code.contains(*m))
            {
                push("deferrals", &c.path, Some(*n), format!("`{mac})` added"));
            } else if let Some(comment) = comment
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
        // `#[ignore]` on a test the change adds is a new test that `cargo test` won't run, which
        // is worth a note; on a test that already existed it is a test switched off.
        for h in hunks(&c.added, &c.added_starts) {
            for (k, (n, l)) in h.iter().enumerate() {
                if !split_line(l.trim()).0.starts_with("#[ignore") {
                    continue;
                }
                match following_fn(h[k + 1..].iter().map(|(_, l)| l.as_str())) {
                    Some(name) if !removed_names.contains(name.as_str()) => push(
                        "new-test-ignored",
                        &c.path,
                        Some(*n),
                        format!(
                            "new test `{name}` is `#[ignore]`d, so `cargo test` will not run it; check that CI runs ignored tests"
                        ),
                    ),
                    _ => push(
                        "tests-ignored",
                        &c.path,
                        Some(*n),
                        "`#[ignore]` added to a test that already existed".into(),
                    ),
                }
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
        if new > policy.max_file_lines && new > old {
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
                ("tests-removed", true),
                ("assertions-removed", false),
                ("measuring-stick", false),
                ("measuring-stick", false),
                ("flags", true),
                ("suppression-without-reason", true),
                ("suppressions", false),
                ("allow-attributes", false),
                ("deferrals", false),
                ("deferrals", false),
                ("measuring-stick", false),
                ("tests-ignored", true),
            ],
            "{:#?}",
            r.items
        );
        // Dependency edits to Cargo.toml aren't a measuring stick.
        assert!(!r.items.iter().any(|i| i.path == Path::new("Cargo.toml")));
        assert_eq!(r.blocking(), 4);
    }

    #[test]
    fn mentions_are_not_the_thing() {
        let policy = Policy::default();
        let changes = vec![
            changed(
                "README.md",
                &[(1, "never add target-cpu=native or RUSTFLAGS")],
                &[],
            ),
            changed(
                "src/mention.rs",
                &[
                    (1, "const K: &str = \"RUSTFLAGS\";"),
                    (2, "let s = \"// anti-slop: allow(todo) x\";"),
                    (3, "let t = \"todo!()\";"),
                ],
                &[],
            ),
            changed(
                "tests/fixture.rs",
                &[(1, "    todo!()"), (2, "    // TODO later")],
                &[],
            ),
            changed(
                "build.rs",
                &[(
                    1,
                    "println!(\"cargo:rustc-env=RUSTFLAGS=-C target-cpu=native\");",
                )],
                &[],
            ),
            changed("src/real.rs", &[(1, "    todo!() // TODO: finish")], &[]),
        ];
        let r = run(
            Path::new("/nonexistent"),
            &Base::Rev("HEAD".into()),
            &policy,
            &changes,
        );
        let kinds: Vec<(&str, &str)> = r
            .items
            .iter()
            .map(|i| (i.kind, i.path.to_str().unwrap_or_default()))
            .collect();
        assert_eq!(
            kinds,
            vec![
                ("measuring-stick", "tests/fixture.rs"),
                ("flags", "build.rs"),
                ("deferrals", "src/real.rs"),
            ],
            "{:#?}",
            r.items
        );
        assert_eq!(r.items[2].detail, "`todo!()` added");
    }
}
