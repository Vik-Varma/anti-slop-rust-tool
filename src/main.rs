use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anti_slop::{Finding, Severity, analyze_paths_with, rules};

const USAGE: &str = "\
anti-slop: find Rust code that is bad by an evidence-backed definition

USAGE:
    anti-slop [check] [OPTIONS] <PATH>...   report findings (exit 1 if any)
    anti-slop check --since REV|--staged [PATH]
                                            only findings on lines changed since REV (or staged)
    anti-slop guard --since REV|--staged [PATH]
                                            the harness: findings on changed lines plus what the
                                            change did to tests, benchmarks, profiles and
                                            suppressions, judged by anti-slop.toml (exit 1 if
                                            anything blocks)
    anti-slop hook pre-tool-use|post-tool-use|stop
                                            Claude Code hook protocol on stdin/stdout
    anti-slop metrics [--format csv|json] <PATH>...
                                            per-function size, complexity and finding counts
    anti-slop rules                         list the rules
    anti-slop explain <RULE>                why a rule exists, and its evidence

OPTIONS:
    --format text|json|agent  output format (default text; `agent` is the summary-first
                              report for a coding agent)
    --since REV               judge only lines changed since REV (needs git)
    --staged                  judge only lines staged for commit
    --log FILE                append one JSON line per run (counts only) to FILE
    --min-severity LEVEL      style, note, warning or error (default warning; notes
                              are real but usually deliberate; style rules are Google's
                              Rust style guide, see `anti-slop rules`)
    --only RULE[,RULE]        run only these rules
    --skip RULE[,RULE]        don't report these rules

Silence one finding with `// anti-slop: allow(RULE) reason` on its line or the line above.";

/// Why the CLI stopped: a usage mistake (print the usage), or an I/O failure (just say so).
enum Error {
    Usage(String),
    Io(io::Error),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Error {
        Error::Usage(msg)
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Error {
        Error::Usage(msg.to_string())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Command {
    Check,
    Guard,
    Metrics,
}

struct Options {
    format: String,
    min_severity: Severity,
    /// Set explicitly, so the harness knows whether to defer to the policy.
    severity_given: bool,
    only: Vec<String>,
    skip: Vec<String>,
    since: Option<String>,
    staged: bool,
    log: Option<PathBuf>,
    paths: Vec<PathBuf>,
}

fn parse_options(command: Command, args: &[String]) -> Result<Options, Error> {
    let mut o = Options {
        format: if command == Command::Metrics {
            "csv"
        } else {
            "text"
        }
        .into(),
        min_severity: Severity::Warning,
        severity_given: false,
        only: vec![],
        skip: vec![],
        since: None,
        staged: false,
        log: None,
        paths: vec![],
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().cloned().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--format" => o.format = value("--format")?,
            "--min-severity" => {
                let v = value("--min-severity")?;
                o.min_severity = Severity::parse(&v).ok_or(format!("unknown severity `{v}`"))?;
                o.severity_given = true;
            }
            "--since" => o.since = Some(value("--since")?),
            "--staged" => o.staged = true,
            "--log" => o.log = Some(PathBuf::from(value("--log")?)),
            "--only" => o.only.extend(value("--only")?.split(',').map(String::from)),
            "--skip" => o.skip.extend(value("--skip")?.split(',').map(String::from)),
            flag if flag.starts_with("--") => return Err(format!("unknown option `{flag}`").into()),
            path => o.paths.push(PathBuf::from(path)),
        }
    }
    for id in o.only.iter().chain(&o.skip) {
        if rules::get(id).is_none() {
            return Err(format!("unknown rule `{id}`; `anti-slop rules` lists them").into());
        }
    }
    let formats: &[&str] = match command {
        Command::Check | Command::Guard => &["text", "json", "agent"],
        Command::Metrics => &["csv", "json"],
    };
    if !formats.contains(&o.format.as_str()) {
        return Err(format!(
            "unknown format `{}`; use {}",
            o.format,
            formats.join(" or ")
        )
        .into());
    }
    if command == Command::Metrics
        && (!o.only.is_empty() || !o.skip.is_empty() || o.min_severity != Severity::Warning)
    {
        return Err(
            "`metrics` reports every rule; --only, --skip and --min-severity don't apply".into(),
        );
    }
    if o.since.is_some() && o.staged {
        return Err("give --since or --staged, not both".into());
    }
    if command == Command::Guard && o.since.is_none() && !o.staged {
        return Err("guard needs --since REV or --staged".into());
    }
    if o.format == "agent" && o.since.is_none() && !o.staged && command == Command::Check {
        return Err("--format agent goes with --since or --staged".into());
    }
    if o.paths.is_empty() {
        if o.since.is_some() || o.staged {
            o.paths.push(PathBuf::from("."));
        } else {
            return Err("give at least one file or directory".into());
        }
    }
    Ok(o)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let result = match args.first().map(String::as_str) {
        Some("rules") => list_rules(),
        Some("explain") => explain(args.get(1)),
        Some("metrics") => parse_options(Command::Metrics, &args[1..]).and_then(|o| metrics(&o)),
        Some("guard") => parse_options(Command::Guard, &args[1..]).and_then(|o| harness(&o, true)),
        Some("hook") => hook(args.get(1)),
        Some("check") => parse_options(Command::Check, &args[1..]).and_then(|o| check(&o)),
        Some(_) => parse_options(Command::Check, &args).and_then(|o| check(&o)),
        None => Err(Error::Usage(String::new())),
    };
    match result {
        Ok(code) => code,
        // The reader went away (`anti-slop . | head`): nothing more to say.
        Err(Error::Io(e)) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(Error::Io(e)) => {
            eprintln!("anti-slop: {e}");
            ExitCode::from(2)
        }
        Err(Error::Usage(msg)) => {
            if !msg.is_empty() {
                eprintln!("anti-slop: {msg}\n");
            }
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn list_rules() -> Result<ExitCode, Error> {
    let mut out = io::stdout().lock();
    let (style, evidence): (Vec<_>, Vec<_>) =
        rules::all().partition(|r| r.severity == Severity::Style);
    for r in evidence {
        writeln!(
            out,
            "{:<32} {:<8} {:<16} {}",
            r.id,
            r.severity,
            r.category.as_str(),
            r.summary
        )?;
    }
    writeln!(
        out,
        "\nStyle rules, from Google's Rust style guide (`--min-severity style`; preferences, not defects):"
    )?;
    for r in style {
        writeln!(out, "{:<32} {}", r.id, r.summary)?;
    }
    Ok(ExitCode::SUCCESS)
}

fn explain(id: Option<&String>) -> Result<ExitCode, Error> {
    let id = id.ok_or("explain needs a rule id")?;
    let r = rules::get(id).ok_or(format!("unknown rule `{id}`"))?;
    let mut text = format!(
        "{} ({}, {}, evidence: {})\n\n{}\n\nWhy it is bad: {}\n\nWhat it can get wrong: {}\n\nAudit: {}\n\nSources:\n",
        r.id,
        r.severity,
        r.category.as_str(),
        r.evidence.as_str(),
        r.summary,
        r.harm,
        r.false_positives,
        r.precision
    );
    for s in r.sources {
        text.push_str(&format!("  - {s}\n"));
    }
    if !r.exceptions.is_empty() {
        text.push_str(&format!(
            "\nExceptions the source lists: {}\n",
            r.exceptions
        ));
    }
    io::stdout().lock().write_all(text.as_bytes())?;
    Ok(ExitCode::SUCCESS)
}

fn keep(o: &Options, f: &Finding) -> bool {
    (f.severity >= o.min_severity || o.only.iter().any(|r| r == f.rule))
        && (o.only.is_empty() || o.only.iter().any(|r| r == f.rule))
        && !o.skip.iter().any(|r| r == f.rule)
}

/// The style rules run only when they can be shown: at `--min-severity style`, or named by `--only`.
fn wants_style(o: &Options) -> bool {
    o.min_severity == Severity::Style
        || o.only
            .iter()
            .any(|id| rules::get(id).is_some_and(|r| r.severity == Severity::Style))
}

/// `check --since`/`--staged` and `guard`: the harness over a change. `check` shows only the
/// findings; `guard` also judges the change and exits 1 when the policy says something blocks.
fn harness(o: &Options, guard: bool) -> Result<ExitCode, Error> {
    use anti_slop::harness::{self, Selection, agent, diff::Base, policy::Policy};
    let start = o
        .paths
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = if start.is_dir() {
        start.clone()
    } else {
        start.parent().map(Path::to_path_buf).unwrap_or_default()
    };
    let root = harness::diff::repo_root(&dir)?;
    let policy = Policy::load(&root)?;
    let base = if o.staged {
        Base::Staged
    } else {
        Base::Rev(o.since.clone().unwrap_or_else(|| policy.base.clone()))
    };
    let sel = Selection {
        min_severity: o.severity_given.then_some(o.min_severity),
        only: o.only.clone(),
        skip: o.skip.clone(),
        style: wants_style(o).then_some(true),
    };
    let run = harness::run(&root, &base, &policy, &sel)?;
    let mut out = io::stdout().lock();
    let text = match o.format.as_str() {
        "agent" => agent::render(&run),
        "json" => {
            let findings: Vec<&Finding> = run.findings.iter().collect();
            let mut buf = Vec::new();
            write_json(&mut buf, &findings)?;
            let mut s = String::from_utf8_lossy(&buf).into_owned();
            if guard {
                let items: Vec<String> = run.guard.items.iter().map(|i| format!(
                    "  {{\"kind\": {}, \"blocking\": {}, \"path\": {}, \"line\": {}, \"detail\": {}}}",
                    json_str(i.kind), i.blocking, json_str(&i.path.to_string_lossy()),
                    i.line.map(|l| l.to_string()).unwrap_or_else(|| "null".into()), json_str(&i.detail)
                )).collect();
                s = format!(
                    "{{\"findings\": {}, \"guard\": [\n{}\n], \"blocking\": {}, \"lines_added\": {}, \"lines_removed\": {}}}\n",
                    s.trim_end(),
                    items.join(",\n"),
                    run.blocking(),
                    run.guard.lines_added,
                    run.guard.lines_removed
                );
            }
            s
        }
        _ => {
            let mut buf = Vec::new();
            let findings: Vec<&Finding> = run.findings.iter().collect();
            write_text(&mut buf, &findings, run.changes.len())?;
            let mut s = String::from_utf8_lossy(&buf).into_owned();
            if guard {
                s.push_str(
                    &agent::render(&run)
                        .lines()
                        .skip_while(|l| !l.starts_with("guard:"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
                s.push('\n');
            }
            s
        }
    };
    out.write_all(text.as_bytes())?;
    for e in &run.errors {
        eprintln!("anti-slop: {e}");
    }
    if let Some(log) = &o.log {
        harness::log(log, &run, if guard { "guard" } else { "check" }, text.len())?;
    }
    let failed = if guard {
        run.blocking() > 0
    } else {
        !run.findings.is_empty()
    };
    Ok(if !run.errors.is_empty() {
        ExitCode::from(2)
    } else if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

/// `hook EVENT`: Claude Code's protocol. Always exits 0; the JSON on stdout carries the decision.
fn hook(event: Option<&String>) -> Result<ExitCode, Error> {
    use anti_slop::harness::hook::{Event, run};
    let name = event.ok_or("hook needs an event: pre-tool-use, post-tool-use or stop")?;
    let event = Event::parse(name).ok_or(format!("unknown hook event `{name}`"))?;
    let mut input = String::new();
    io::Read::read_to_string(&mut io::stdin().lock(), &mut input)?;
    let cwd = std::env::current_dir()?;
    let outcome = run(event, &input, &cwd);
    if !outcome.stdout.is_empty() {
        io::stdout().lock().write_all(outcome.stdout.as_bytes())?;
    }
    if !outcome.stderr.is_empty() {
        eprintln!("{}", outcome.stderr);
    }
    if let (Some(run), Ok(log)) = (&outcome.run, std::env::var("ANTI_SLOP_LOG")) {
        let _ = anti_slop::harness::log(Path::new(&log), run, "hook", outcome.stdout.len());
    }
    Ok(ExitCode::SUCCESS)
}

fn check(o: &Options) -> Result<ExitCode, Error> {
    if o.since.is_some() || o.staged {
        return harness(o, false);
    }
    let (reports, errors) = analyze_paths_with(&o.paths, wants_style(o));
    let findings: Vec<&Finding> = reports
        .iter()
        .flat_map(|(_, r)| &r.findings)
        .filter(|f| keep(o, f))
        .collect();
    let mut out = io::stdout().lock();
    if o.format == "json" {
        write_json(&mut out, &findings)?;
    } else {
        write_text(&mut out, &findings, reports.len())?;
    }
    for e in &errors {
        eprintln!("anti-slop: {e}");
    }
    Ok(if !errors.is_empty() {
        ExitCode::from(2)
    } else if findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn write_text(out: &mut impl Write, findings: &[&Finding], files: usize) -> io::Result<()> {
    let mut by_rule: BTreeMap<&str, usize> = BTreeMap::new();
    for f in findings {
        writeln!(out, "{f}")?;
        *by_rule.entry(f.rule).or_default() += 1;
    }
    writeln!(out, "\n{} finding(s) in {files} file(s)", findings.len())?;
    for (rule, n) in by_rule {
        writeln!(out, "  {n:>5}  {rule}")?;
    }
    Ok(())
}

/// RFC 4180: quote the field and double any quote inside it.
fn csv_str(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

pub fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn write_json(out: &mut impl Write, findings: &[&Finding]) -> io::Result<()> {
    writeln!(out, "[")?;
    for (i, f) in findings.iter().enumerate() {
        let function = f
            .function
            .as_deref()
            .map(json_str)
            .unwrap_or_else(|| "null".into());
        writeln!(
            out,
            "  {{\"rule\": {}, \"severity\": {}, \"path\": {}, \"line\": {}, \"column\": {}, \"function\": {}, \"message\": {}}}{}",
            json_str(f.rule),
            json_str(f.severity.as_str()),
            json_str(&f.path.to_string_lossy()),
            f.line,
            f.column,
            function,
            json_str(&f.message),
            if i + 1 < findings.len() { "," } else { "" }
        )?;
    }
    writeln!(out, "]")
}

/// One row per function: its metrics, the candidate-pattern counts and the findings inside it.
/// Text fields are quoted for the output format; numbers are left bare.
fn function_rows(
    path: &str,
    report: &anti_slop::FileReport,
    quote: fn(&str) -> String,
) -> Vec<Vec<(String, String)>> {
    let mut rows = Vec::new();
    for (index, m) in report.functions.iter().enumerate() {
        let f = &m.features;
        let mut row: Vec<(String, String)> = vec![
            ("path".into(), quote(path)),
            ("function".into(), quote(&m.name)),
        ];
        let numbers = [
            ("start", m.start_line),
            ("end", m.end_line),
            ("code_lines", m.code_lines),
            ("cyclomatic", m.cyclomatic),
            ("cognitive", m.cognitive),
            ("max_nesting", m.max_nesting),
            ("params", m.params),
            ("try_ops", m.try_ops),
            ("test", usize::from(m.test)),
            ("f_clones", f.clones),
            ("f_expects", f.expects),
            ("f_panics", f.panics),
            ("f_as_casts", f.as_casts),
            ("f_magic_numbers", f.magic_numbers),
            ("f_unsafe_blocks", f.unsafe_blocks),
            ("f_bool_params", f.bool_params),
            ("f_debt_comments", f.debt_comments),
            ("f_comment_lines", f.comment_lines),
        ];
        row.extend(numbers.iter().map(|(k, v)| (k.to_string(), v.to_string())));
        // Style rules aren't counted: they're preferences, and the study's columns predate them.
        for r in rules::all().filter(|r| r.severity != Severity::Style) {
            // By the function the finding was emitted in, so nested functions don't count twice.
            let n = report
                .findings
                .iter()
                .filter(|x| x.rule == r.id && x.function_index == Some(index))
                .count();
            row.push((r.id.to_string(), n.to_string()));
        }
        rows.push(row);
    }
    rows
}

fn metrics(o: &Options) -> Result<ExitCode, Error> {
    let (reports, errors) = analyze_paths_with(&o.paths, false);
    let csv = o.format != "json";
    let mut out = io::stdout().lock();
    let mut first = true;
    let mut write_all = || -> io::Result<()> {
        if !csv {
            writeln!(out, "[")?;
        }
        for (path, report) in &reports {
            let quote = if csv { csv_str } else { json_str };
            for row in function_rows(&path.to_string_lossy(), report, quote) {
                if csv {
                    if first {
                        let names: Vec<&str> = row.iter().map(|(k, _)| k.as_str()).collect();
                        writeln!(out, "{}", names.join(","))?;
                    }
                    let values: Vec<&str> = row.iter().map(|(_, v)| v.as_str()).collect();
                    writeln!(out, "{}", values.join(","))?;
                } else {
                    let fields: Vec<String> = row
                        .iter()
                        .map(|(k, v)| format!("{}: {v}", json_str(k)))
                        .collect();
                    write!(
                        out,
                        "{}  {{{}}}",
                        if first { "" } else { ",\n" },
                        fields.join(", ")
                    )?;
                }
                first = false;
            }
        }
        if !csv {
            writeln!(out, "\n]")?;
        }
        Ok(())
    };
    write_all()?;
    for e in &errors {
        eprintln!("anti-slop: {e}");
    }
    Ok(if errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn csv_quotes_by_doubling() {
        assert_eq!(super::csv_str("we\"ird,x.rs"), "\"we\"\"ird,x.rs\"");
    }
}
