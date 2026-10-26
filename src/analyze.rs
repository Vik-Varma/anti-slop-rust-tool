use crate::config::Config;
use crate::context::FileContext;
use crate::finding::Finding;
use crate::metrics::{self, FnMetrics};
use crate::rules::{Input, Rule, Sink};
use rayon::prelude::*;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Serialize)]
pub struct FileReport {
    pub file: PathBuf,
    pub findings: Vec<Finding>,
    pub functions: Vec<FnMetrics>,
    pub parse_error: Option<String>,
    #[serde(skip)]
    pub tokens: Vec<crate::duplicates::Tok>,
}

/// Finds all `.rs` files under the given paths, honouring `.gitignore`.
pub fn discover(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in paths {
        if root.is_file() {
            files.push(root.clone());
            continue;
        }
        let walker = ignore::WalkBuilder::new(root)
            .hidden(true)
            .filter_entry(|e| e.file_name() != "target")
            .build();
        for entry in walker.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().is_some_and(|e| e == "rs") {
                files.push(p.to_path_buf());
            }
        }
    }
    files.sort();
    files.dedup();
    files
}

/// The path relative to the enclosing crate root (the nearest directory with a
/// `Cargo.toml`), so that `tests/`, `benches/` etc. are recognised regardless
/// of where the crate lives on disk.
pub fn crate_relative(path: &Path) -> PathBuf {
    let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut dir = abs.parent();
    while let Some(d) = dir {
        if d.join("Cargo.toml").is_file() {
            if let Ok(rel) = abs.strip_prefix(d) {
                return rel.to_path_buf();
            }
        }
        dir = d.parent();
    }
    path.to_path_buf()
}

pub fn analyze_source(
    path: &Path,
    display: PathBuf,
    kind_path: &Path,
    source: &str,
    config: &Config,
    rules: &[Box<dyn Rule>],
) -> FileReport {
    let ctx = match FileContext::with_kind_path(path, display.clone(), kind_path, source) {
        Ok(ctx) => ctx,
        Err(e) => {
            let loc = e.span().start();
            return FileReport {
                file: display,
                parse_error: Some(format!("{}:{}: {e}", loc.line, loc.column + 1)),
                ..Default::default()
            };
        }
    };
    let fns = metrics::collect(&ctx);
    let input = Input { ctx: &ctx, fns: &fns, config };
    let mut findings = Vec::new();
    for rule in rules {
        if !config.rule_enabled(rule.meta().id) {
            continue;
        }
        let mut sink = Sink::new(&ctx, rule.meta());
        rule.check(&input, &mut sink);
        findings.append(&mut sink.findings);
    }
    findings.sort_by_key(|f| (f.line, f.column, f.rule));
    let tokens = if config.rule_enabled(crate::duplicates::META.id) {
        crate::duplicates::tokens(&ctx)
    } else {
        Vec::new()
    };
    FileReport { file: display, findings, functions: fns, parse_error: None, tokens }
}

pub fn analyze_files(files: &[PathBuf], config: &Config, rules: &[Box<dyn Rule>]) -> Vec<FileReport> {
    let mut reports: Vec<FileReport> = files
        .par_iter()
        .map(|path| {
            let kind_path = crate_relative(path);
            match std::fs::read_to_string(path) {
                Ok(src) => analyze_source(path, path.clone(), &kind_path, &src, config, rules),
                Err(e) => FileReport {
                    file: path.clone(),
                    parse_error: Some(format!("cannot read: {e}")),
                    ..Default::default()
                },
            }
        })
        .collect();
    crate::duplicates::check(&mut reports, files, config);
    reports
}
