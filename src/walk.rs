use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::analyzer::{FileReport, analyze_source};

/// Directories that hold build output, dependencies or VCS data, never the code under review.
const SKIP_DIRS: &[&str] = &["target", ".git", "node_modules", "vendor", ".cargo"];

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
                if !SKIP_DIRS.contains(&name.as_ref()) {
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
    for root in paths {
        let files = match rust_files(root) {
            Ok(f) => f,
            Err(e) => {
                errors.push(AnalyzeError::Io(root.clone(), e));
                continue;
            }
        };
        for file in files {
            match fs::read_to_string(&file) {
                Ok(src) => match analyze_source(&file, &src) {
                    Ok(report) => reports.push((file, report)),
                    Err(e) => errors.push(AnalyzeError::Parse(file, e)),
                },
                Err(e) => errors.push(AnalyzeError::Io(file, e)),
            }
        }
    }
    (reports, errors)
}
