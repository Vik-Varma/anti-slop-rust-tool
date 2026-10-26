use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use anti_slop::{Finding, Severity, analyze_paths, rules};

const USAGE: &str = "\
anti-slop: find Rust code that is bad by an evidence-backed definition

USAGE:
    anti-slop [check] [OPTIONS] <PATH>...   report findings (exit 1 if any)
    anti-slop metrics [--format csv|json] <PATH>...
                                            per-function size, complexity and finding counts
    anti-slop rules                         list the rules
    anti-slop explain <RULE>                why a rule exists, and its evidence

OPTIONS:
    --format text|json        output format (default text)
    --min-severity LEVEL      note, warning or error (default warning; notes are
                              real but usually deliberate, see `anti-slop rules`)
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
    Metrics,
}

struct Options {
    format: String,
    min_severity: Severity,
    only: Vec<String>,
    skip: Vec<String>,
    paths: Vec<PathBuf>,
}

fn parse_options(command: Command, args: &[String]) -> Result<Options, Error> {
    let mut o = Options {
        format: if command == Command::Check {
            "text"
        } else {
            "csv"
        }
        .into(),
        min_severity: Severity::Warning,
        only: vec![],
        skip: vec![],
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
            }
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
    let formats: &[&str] = if command == Command::Check {
        &["text", "json"]
    } else {
        &["csv", "json"]
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
    if o.paths.is_empty() {
        return Err("give at least one file or directory".into());
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
    for r in rules::all() {
        writeln!(
            out,
            "{:<32} {:<8} {:<16} {}",
            r.id,
            r.severity,
            r.category.as_str(),
            r.summary
        )?;
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
    io::stdout().lock().write_all(text.as_bytes())?;
    Ok(ExitCode::SUCCESS)
}

fn keep(o: &Options, f: &Finding) -> bool {
    f.severity >= o.min_severity
        && (o.only.is_empty() || o.only.iter().any(|r| r == f.rule))
        && !o.skip.iter().any(|r| r == f.rule)
}

fn check(o: &Options) -> Result<ExitCode, Error> {
    let (reports, errors) = analyze_paths(&o.paths);
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
        for r in rules::all() {
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
    let (reports, errors) = analyze_paths(&o.paths);
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
