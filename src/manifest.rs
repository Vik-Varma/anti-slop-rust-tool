//! What a file's Cargo manifests say that changes what its code does. Cargo takes profile
//! settings from the workspace root's manifest only, lets a `.cargo/config.toml` override them,
//! and ignores `panic` for tests, benchmarks, build scripts and proc macros (Cargo Book,
//! Profiles). None of this needs Cargo to run: the TOML is read line by line, which covers how
//! profiles are written in practice.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The profiles that build `file` with `panic = "abort"`, so a panic there ends the process
/// instead of unwinding. Empty when no profile does, when the file is in no package, or when the
/// setting doesn't apply to it (a proc-macro crate). Cached per package directory.
pub fn abort_profiles(file: &Path, cache: &mut HashMap<PathBuf, Vec<String>>) -> Vec<String> {
    let Some(package) = file
        .ancestors()
        .skip(1)
        .find(|d| d.join("Cargo.toml").is_file())
    else {
        return Vec::new();
    };
    cache
        .entry(package.to_path_buf())
        .or_insert_with(|| compute(package))
        .clone()
}

fn compute(package: &Path) -> Vec<String> {
    let read = |p: PathBuf| fs::read_to_string(p).unwrap_or_default();
    let manifest = read(package.join("Cargo.toml"));
    if is_proc_macro(&manifest) {
        return Vec::new();
    }
    // The root is the nearest manifest with a `[workspace]` table, this package's own included.
    let root = package
        .ancestors()
        .find(|d| has_workspace_table(&read(d.join("Cargo.toml"))))
        .unwrap_or(package);
    let mut panic = panic_settings(&read(root.join("Cargo.toml")));
    for name in ["config.toml", "config"] {
        if let Ok(config) = fs::read_to_string(root.join(".cargo").join(name)) {
            panic.extend(panic_settings(&config));
            break;
        }
    }
    let mut out: Vec<String> = panic
        .into_iter()
        .filter(|(_, v)| v == "abort")
        .map(|(k, _)| k)
        .collect();
    out.sort();
    out
}

/// Each profile's `panic` value: from `[profile.release]` with `panic = "abort"`, or the dotted
/// `profile.release.panic = "abort"`. `[profile.release.package."*"]` can't set `panic`, and is
/// left out.
fn panic_settings(toml: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for (key, value) in entries(toml) {
        let Some(profile) = key
            .strip_prefix("profile.")
            .and_then(|k| k.strip_suffix(".panic"))
        else {
            continue;
        };
        if !profile.contains('.') {
            out.insert(profile.to_string(), value);
        }
    }
    out
}

fn is_proc_macro(manifest: &str) -> bool {
    entries(manifest)
        .iter()
        .any(|(k, v)| (k == "lib.proc-macro" || k == "lib.proc_macro") && v == "true")
}

fn has_workspace_table(manifest: &str) -> bool {
    tables(manifest)
        .iter()
        .any(|t| t == "workspace" || t.starts_with("workspace."))
}

fn tables(toml: &str) -> Vec<String> {
    toml.lines().filter_map(table_header).collect()
}

/// `[a.b]` and `[[a.b]]` headers, as the dotted name `a.b`.
fn table_header(raw: &str) -> Option<String> {
    let line = strip_comment(raw).trim();
    line.strip_prefix('[')
        .and_then(|l| l.strip_suffix(']'))
        .map(|t| t.trim_matches(|c| c == '[' || c == ']').trim().to_string())
}

/// Every `key = value` pair as (`table.key`, value), with string quotes removed from the value.
pub(crate) fn entries(toml: &str) -> Vec<(String, String)> {
    let mut table = String::new();
    let mut out = Vec::new();
    for raw in toml.lines() {
        if let Some(t) = table_header(raw) {
            table = t;
            continue;
        }
        let line = strip_comment(raw);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = value
            .trim()
            .trim_matches(|c| c == '"' || c == '\'')
            .to_string();
        let full = if table.is_empty() {
            key.to_string()
        } else {
            format!("{table}.{key}")
        };
        out.push((full, value));
    }
    out
}

/// Drops a `#` comment that isn't inside a string.
fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..i],
            _ => {}
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_tables_and_comments() {
        // A dotted key belongs to the table above it, so the top-level one comes first.
        let toml = "profile.bench.panic = 'abort'\n[package]\nname = \"x\" # not a profile\n\n\
                    [profile.release]\npanic = \"abort\" # small binaries\n\n\
                    [profile.release.package.\"*\"]\npanic = \"unwind\"\n[profile.dev]\n\
                    opt-level = 1\n";
        let p = panic_settings(toml);
        assert_eq!(p.get("release").map(String::as_str), Some("abort"));
        assert_eq!(p.get("bench").map(String::as_str), Some("abort"));
        assert!(!p.contains_key("dev"));
        assert_eq!(p.len(), 2);
    }

    #[test]
    fn workspace_and_proc_macro_detection() {
        assert!(has_workspace_table("[workspace]\nmembers = [\"a\"]\n"));
        assert!(has_workspace_table(
            "[package]\nname = \"r\"\n[workspace.dependencies]\n"
        ));
        assert!(!has_workspace_table(
            "[package]\nname = \"a\"\nworkspace = \"..\"\n"
        ));
        assert!(is_proc_macro("[lib]\nproc-macro = true\n"));
        assert!(is_proc_macro("lib.proc_macro = true\n"));
        assert!(!is_proc_macro(
            "[lib]\nproc-macro = false\n[dependencies]\nproc-macro2 = \"1\"\n"
        ));
    }

    #[test]
    fn hash_inside_a_string_is_not_a_comment() {
        assert_eq!(strip_comment("name = \"a#b\" # c"), "name = \"a#b\" ");
        assert_eq!(table_header("[[bin]] # entry").as_deref(), Some("bin"));
    }
}
