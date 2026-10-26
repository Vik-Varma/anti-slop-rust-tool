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
    --min-severity LEVEL      note, warning or error (default note)
    --only RULE[,RULE]        run only these rules
    --skip RULE[,RULE]        don't report these rules

Silence one finding with `// anti-slop: allow(RULE) reason` on its line or the line above.";

struct Options {
    format: String,
    min_severity: Severity,
    only: Vec<String>,
    skip: Vec<String>,
    paths: Vec<PathBuf>,
}

fn parse_options(args: &[String]) -> Result<Options, String> {
    let mut o = Options {
        format: "text".into(),
        min_severity: Severity::Note,
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
            "-h" | "--help" => return Err(String::new()),
            flag if flag.starts_with("--") => return Err(format!("unknown option `{flag}`")),
            path => o.paths.push(PathBuf::from(path)),
        }
    }
    for id in o.only.iter().chain(&o.skip) {
        if rules::get(id).is_none() {
            return Err(format!("unknown rule `{id}`; `anti-slop rules` lists them"));
        }
    }
    if o.paths.is_empty() {
        return Err("give at least one file or directory".into());
    }
    Ok(o)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("rules") => list_rules(),
        Some("explain") => explain(args.get(1)),
        Some("metrics") => parse_options(&args[1..]).and_then(|o| metrics(&o)),
        Some("check") => parse_options(&args[1..]).and_then(|o| check(&o)),
        Some(_) => parse_options(&args).and_then(|o| check(&o)),
        None => Err(String::new()),
    };
    match result {
        Ok(code) => code,
        Err(msg) => {
            if !msg.is_empty() {
                eprintln!("anti-slop: {msg}\n");
            }
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn list_rules() -> Result<ExitCode, String> {
    let mut out = io::stdout().lock();
    for r in rules::all() {
        writeln!(
            out,
            "{:<32} {:<8} {:<16} {}",
            r.id,
            r.severity,
            r.category.as_str(),
            r.summary
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(ExitCode::SUCCESS)
}

fn explain(id: Option<&String>) -> Result<ExitCode, String> {
    let id = id.ok_or("explain needs a rule id")?;
    let r = rules::get(id).ok_or(format!("unknown rule `{id}`"))?;
    let mut text = format!(
        "{} ({}, {})\n\n{}\n\nWhy it is bad: {}\n\nWhat it can get wrong: {}\n\nSources:\n",
        r.id,
        r.severity,
        r.category.as_str(),
        r.summary,
        r.harm,
        r.false_positives
    );
    for s in r.sources {
        text.push_str(&format!("  - {s}\n"));
    }
    print!("{text}");
    Ok(ExitCode::SUCCESS)
}

fn keep(o: &Options, f: &Finding) -> bool {
    f.severity >= o.min_severity
        && (o.only.is_empty() || o.only.iter().any(|r| r == f.rule))
        && !o.skip.iter().any(|r| r == f.rule)
}

fn check(o: &Options) -> Result<ExitCode, String> {
    let (reports, errors) = analyze_paths(&o.paths);
    let findings: Vec<&Finding> = reports
        .iter()
        .flat_map(|(_, r)| &r.findings)
        .filter(|f| keep(o, f))
        .collect();
    let mut out = io::stdout().lock();
    let written = match o.format.as_str() {
        "text" => write_text(&mut out, &findings, reports.len()),
        "json" => write_json(&mut out, &findings),
        other => return Err(format!("unknown format `{other}`")),
    };
    written.map_err(|e| e.to_string())?;
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

fn metrics(o: &Options) -> Result<ExitCode, String> {
    let (reports, errors) = analyze_paths(&o.paths);
    let ids: Vec<&str> = rules::all().iter().map(|r| r.id).collect();
    let mut out = io::stdout().lock();
    let header = [
        "path",
        "function",
        "start",
        "end",
        "code_lines",
        "cyclomatic",
        "cognitive",
        "max_nesting",
        "params",
        "try_ops",
        "test",
    ];
    let csv = o.format != "json";
    if csv {
        writeln!(out, "{},{}", header.join(","), ids.join(",")).map_err(|e| e.to_string())?;
    }
    let mut first = true;
    if !csv {
        writeln!(out, "[").map_err(|e| e.to_string())?;
    }
    for (path, report) in &reports {
        for m in &report.functions {
            let counts: Vec<usize> = ids
                .iter()
                .map(|id| {
                    report
                        .findings
                        .iter()
                        .filter(|f| f.rule == *id && f.line >= m.start_line && f.line <= m.end_line)
                        .count()
                })
                .collect();
            let p = path.to_string_lossy();
            let line = if csv {
                let counts: Vec<String> = counts.iter().map(usize::to_string).collect();
                format!(
                    "{},{},{},{},{},{},{},{},{},{},{},{}",
                    json_str(&p),
                    json_str(&m.name),
                    m.start_line,
                    m.end_line,
                    m.code_lines,
                    m.cyclomatic,
                    m.cognitive,
                    m.max_nesting,
                    m.params,
                    m.try_ops,
                    u8::from(m.test),
                    counts.join(",")
                )
            } else {
                let counts: Vec<String> = ids
                    .iter()
                    .zip(&counts)
                    .filter(|(_, n)| **n > 0)
                    .map(|(id, n)| format!("{}: {n}", json_str(id)))
                    .collect();
                format!(
                    "{}  {{\"path\": {}, \"function\": {}, \"start\": {}, \"end\": {}, \"code_lines\": {}, \"cyclomatic\": {}, \"cognitive\": {}, \"max_nesting\": {}, \"params\": {}, \"try_ops\": {}, \"test\": {}, \"findings\": {{{}}}}}",
                    if first { "" } else { ",\n" },
                    json_str(&p),
                    json_str(&m.name),
                    m.start_line,
                    m.end_line,
                    m.code_lines,
                    m.cyclomatic,
                    m.cognitive,
                    m.max_nesting,
                    m.params,
                    m.try_ops,
                    m.test,
                    counts.join(", ")
                )
            };
            first = false;
            if csv {
                writeln!(out, "{line}")
            } else {
                write!(out, "{line}")
            }
            .map_err(|e| e.to_string())?;
        }
    }
    if !csv {
        writeln!(out, "\n]").map_err(|e| e.to_string())?;
    }
    for e in &errors {
        eprintln!("anti-slop: {e}");
    }
    Ok(if errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    })
}
