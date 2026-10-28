//! The report an agent reads: a verdict line, the counts, one line per item, a cap, and where to
//! find the rest. Summary first and the long material behind `explain`, so the report stays cheap
//! in a context window and nothing that must survive (rule, `file:line`) is dropped by whoever
//! compresses tool output downstream.

use super::Run;
use crate::Finding;

/// Findings and items shown in full before the list is cut off.
pub const CAP: usize = 30;

fn rel(path: &std::path::Path, root: &std::path::Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn finding_line(f: &Finding, root: &std::path::Path) -> String {
    format!(
        "  {:<8} {}:{}  {}  {}",
        f.severity,
        rel(&f.path, root),
        f.line,
        f.rule,
        f.message
    )
}

/// The whole report for a run: findings on changed lines, then the guard items.
pub fn render(run: &Run) -> String {
    let mut out = String::new();
    let blocking = run.blocking();
    let verdict = if blocking == 0 { "PASS" } else { "FAIL" };
    let counts = {
        let mut by: Vec<(String, usize)> = Vec::new();
        for f in &run.findings {
            match by.iter_mut().find(|(s, _)| *s == f.severity.as_str()) {
                Some((_, n)) => *n += 1,
                None => by.push((f.severity.as_str().to_string(), 1)),
            }
        }
        by.sort_by(|a, b| b.0.cmp(&a.0));
        by.iter()
            .map(|(s, n)| format!("{n} {s}{}", if *n == 1 { "" } else { "s" }))
            .collect::<Vec<_>>()
    };
    out.push_str(&format!(
        "anti-slop: {verdict}: {} on changed lines in {} file(s); {} blocking item(s) from guard\n",
        if counts.is_empty() {
            "no findings".to_string()
        } else {
            counts.join(", ")
        },
        run.guard.files_changed,
        run.guard.blocking(),
    ));
    let mut findings: Vec<&Finding> = run.findings.iter().collect();
    findings.sort_by(|a, b| {
        b.severity
            .cmp(&a.severity)
            .then_with(|| (&a.path, a.line).cmp(&(&b.path, b.line)))
    });
    for f in findings.iter().take(CAP) {
        out.push_str(&finding_line(f, &run.root));
        out.push('\n');
    }
    if findings.len() > CAP {
        out.push_str(&format!(
            "  ... and {} more (run `anti-slop check --since {}` for all)\n",
            findings.len() - CAP,
            run.base_label
        ));
    }
    if run.pre_existing > 0 {
        out.push_str(&format!(
            "  ({} pre-existing finding(s) in these files, outside the change)\n",
            run.pre_existing
        ));
    }
    out.push_str("guard:\n");
    if run.guard.items.is_empty() {
        out.push_str("  nothing to report\n");
    }
    for item in run.guard.items.iter().take(CAP) {
        out.push_str(&format!(
            "  {:<6} {:<27} {}{}  {}\n",
            if item.blocking { "block" } else { "note" },
            item.kind,
            rel(&item.path, &run.root),
            item.line.map(|l| format!(":{l}")).unwrap_or_default(),
            item.detail
        ));
    }
    if run.guard.items.len() > CAP {
        out.push_str(&format!("  ... and {} more\n", run.guard.items.len() - CAP));
    }
    out.push_str(&format!(
        "  code lines: +{} -{}\n",
        run.guard.lines_added, run.guard.lines_removed
    ));
    out.push_str(
        "next: fix or justify each blocking item. `anti-slop explain <rule>` gives the evidence and the exceptions the source allows; \
         `// anti-slop: allow(<rule>) <reason>` on the line above suppresses one finding, and the reason is read by a person.\n",
    );
    out
}
