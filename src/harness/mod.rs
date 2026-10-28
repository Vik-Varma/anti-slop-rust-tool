//! The harness: the tool run on what a change touched, with a verdict an agent or a CI job can
//! act on. `check --since` keeps only findings on changed lines; `guard` adds the measuring-stick
//! and handoff report; `hook` speaks Claude Code's hook protocol. docs/HARNESS.md says why each
//! part exists.

pub mod agent;
pub mod diff;
pub mod guard;
pub mod hook;
pub mod json;
pub mod policy;

use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Finding, Severity, analyze_paths_with, rules};
use diff::{Base, Changed};
use policy::Policy;

/// What to look at, beyond the policy.
#[derive(Clone, Debug, Default)]
pub struct Selection {
    pub min_severity: Option<Severity>,
    pub only: Vec<String>,
    pub skip: Vec<String>,
    pub style: Option<bool>,
}

/// One run of the harness.
#[derive(Clone, Debug)]
pub struct Run {
    pub root: PathBuf,
    pub base_label: String,
    pub changes: Vec<Changed>,
    /// Findings on changed lines, after the policy and selection.
    pub findings: Vec<Finding>,
    /// Findings in the same files that the change didn't touch.
    pub pre_existing: usize,
    pub guard: guard::Report,
    /// Files that couldn't be read or parsed.
    pub errors: Vec<String>,
}

impl Run {
    pub fn blocking(&self) -> usize {
        self.findings.len() + self.guard.blocking()
    }
}

/// Analyses the changed Rust files and keeps the findings the change touched; runs the guard.
pub fn run(root: &Path, base: &Base, policy: &Policy, sel: &Selection) -> io::Result<Run> {
    let changes = diff::changes(root, base)?;
    let style = sel.style.unwrap_or(policy.style);
    let only_style = sel
        .only
        .iter()
        .any(|id| rules::get(id).is_some_and(|r| r.severity == Severity::Style));
    let files: Vec<PathBuf> = changes
        .iter()
        .filter(|c| c.is_rust() && !c.deleted && !c.binary)
        .map(|c| root.join(&c.path))
        .collect();
    let (reports, errors) = analyze_paths_with(&files, style || only_style);
    let min = sel.min_severity.unwrap_or(policy.min_severity);
    let skip: HashSet<&str> = policy
        .skip
        .iter()
        .chain(&sel.skip)
        .map(String::as_str)
        .collect();
    let mut findings = Vec::new();
    let mut pre_existing = 0;
    for (path, report) in &reports {
        let rel = path.strip_prefix(root).unwrap_or(path);
        let Some(change) = changes.iter().find(|c| c.path == rel) else {
            continue;
        };
        for f in &report.findings {
            let wanted = (f.severity >= min || sel.only.iter().any(|r| r == f.rule))
                && (sel.only.is_empty() || sel.only.iter().any(|r| r == f.rule))
                && !skip.contains(f.rule);
            if !wanted {
                continue;
            }
            if change.touches(f.line) {
                findings.push(f.clone());
            } else {
                pre_existing += 1;
            }
        }
    }
    let guard = guard::run(root, base, policy, &changes);
    Ok(Run {
        root: root.to_path_buf(),
        base_label: match base {
            Base::Rev(r) => r.clone(),
            Base::Staged => "--staged".into(),
        },
        changes,
        findings,
        pre_existing,
        guard,
        errors: errors.iter().map(ToString::to_string).collect(),
    })
}

/// Appends one line to the run log: observed counts only, no estimates. `bytes` is the size of
/// what was emitted to the agent, so the feedback's context cost is visible next to its content.
pub fn log(path: &Path, run: &Run, source: &str, bytes: usize) -> io::Result<()> {
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut by_rule: Vec<(&str, usize)> = Vec::new();
    for f in &run.findings {
        match by_rule.iter_mut().find(|(r, _)| *r == f.rule) {
            Some((_, n)) => *n += 1,
            None => by_rule.push((f.rule, 1)),
        }
    }
    let rules_json: Vec<String> = by_rule
        .iter()
        .map(|(r, n)| format!("{}: {n}", json::string(r)))
        .collect();
    let items_json: Vec<String> = run
        .guard
        .items
        .iter()
        .map(|i| {
            format!(
                "{{\"kind\": {}, \"blocking\": {}}}",
                json::string(i.kind),
                i.blocking
            )
        })
        .collect();
    let line = format!(
        "{{\"time\": {time}, \"source\": {}, \"base\": {}, \"files\": {}, \"findings\": {}, \"pre_existing\": {}, \"by_rule\": {{{}}}, \"guard\": [{}], \"blocking\": {}, \"lines_added\": {}, \"lines_removed\": {}, \"bytes_emitted\": {bytes}}}\n",
        json::string(source),
        json::string(&run.base_label),
        run.guard.files_changed,
        run.findings.len(),
        run.pre_existing,
        rules_json.join(", "),
        items_json.join(", "),
        run.blocking(),
        run.guard.lines_added,
        run.guard.lines_removed,
    );
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    f.write_all(line.as_bytes())
}
