use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::analyzer::{FileReport, analyze_source_as};
use crate::context::FileKind;
use crate::layout;

/// Directories that are never code under review.
const SKIP_DIRS: &[&str] = &[".git", "node_modules", ".cargo"];

/// `target` and `vendor` are also ordinary module names (`pub mod vendor;`), so they're skipped
/// only when they are what Cargo makes them: a build directory (Cargo marks it with
/// `CACHEDIR.TAG`, and it sits next to a manifest) or `cargo vendor` output (crates with a
/// `.cargo-checksum.json`).
fn is_generated(dir: &Path, name: &str) -> bool {
    match name {
        "target" => {
            dir.join("CACHEDIR.TAG").is_file()
                || dir.parent().is_some_and(|p| p.join("Cargo.toml").is_file())
        }
        "vendor" => fs::read_dir(dir).is_ok_and(|entries| {
            entries
                .flatten()
                .any(|e| e.path().join(".cargo-checksum.json").is_file())
        }),
        _ => false,
    }
}

#[derive(Debug)]
pub enum AnalyzeError {
    Io(PathBuf, io::Error),
    Parse(PathBuf, syn::Error),
}

impl fmt::Display for AnalyzeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalyzeError::Io(p, e) => write!(f, "{}: {e}", p.display()),
            AnalyzeError::Parse(p, e) => {
                let at = e.span().start();
                write!(
                    f,
                    "{}:{}:{}: could not parse: {e}",
                    p.display(),
                    at.line,
                    at.column + 1
                )
            }
        }
    }
}

impl std::error::Error for AnalyzeError {}

/// Every `.rs` file under `root` (or `root` itself), sorted so output is stable across runs.
pub fn rust_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if root.is_file() {
        out.push(root.to_path_buf());
        return Ok(out);
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let ty = entry.file_type()?;
            if ty.is_dir() {
                if !SKIP_DIRS.contains(&name.as_ref()) && !is_generated(&path, &name) {
                    stack.push(path);
                }
            } else if ty.is_file() && name.ends_with(".rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Analyses every Rust file under each path. Files that can't be read or parsed are returned as
/// errors alongside the reports for the rest.
pub fn analyze_paths(paths: &[PathBuf]) -> (Vec<(PathBuf, FileReport)>, Vec<AnalyzeError>) {
    let mut reports = Vec::new();
    let mut errors = Vec::new();
    // `anti-slop . src src/lib.rs` names one file three times; analyse it once.
    let mut seen = std::collections::HashSet::new();
    for root in paths {
        let files = match rust_files(root) {
            Ok(f) => f,
            Err(e) => {
                errors.push(AnalyzeError::Io(root.clone(), e));
                continue;
            }
        };
        let files: Vec<PathBuf> = files
            .into_iter()
            .filter(|f| seen.insert(fs::canonicalize(f).unwrap_or_else(|_| f.clone())))
            .collect();
        // Judge paths from the root's own name down, so `anti-slop tests` sees `tests/` but a
        // checkout that lives inside some `tests/` directory doesn't. A single file is judged from
        // its package's parent, which keeps names like `foo-test-support/` and drops the rest.
        let base = if root.is_file() {
            layout::judging_base(root)
        } else {
            root.parent().unwrap_or(Path::new("")).to_path_buf()
        };
        let kinds = layout::kinds(&base, &files);
        for file in files {
            let kind = kinds
                .get(&file)
                .copied()
                .unwrap_or_else(|| FileKind::of(&file));
            match fs::read_to_string(&file) {
                Ok(src) => match analyze_source_as(&file, &src, kind) {
                    Ok(report) => reports.push((file, report)),
                    Err(e) => errors.push(AnalyzeError::Parse(file, e)),
                },
                Err(e) => errors.push(AnalyzeError::Io(file, e)),
            }
        }
    }
    (reports, errors)
}
