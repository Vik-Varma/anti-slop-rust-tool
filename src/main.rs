use anti_slop::analyze::{self, FileReport};
use anti_slop::config::Config;
use anti_slop::context::FileKind;
use anti_slop::finding::Severity;
use anti_slop::rules;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "anti-slop", version, about = "Find evidence-backed bad-code patterns in Rust source")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Analyse files or directories and report findings.
    Check {
        /// Files or directories to analyse (default: current directory).
        paths: Vec<PathBuf>,
        #[arg(long, value_enum, default_value = "text")]
        format: Format,
        /// Configuration file (default: `anti-slop.toml` in the current directory, if present).
        #[arg(long)]
        config: Option<PathBuf>,
        /// Only run these rules (comma separated).
        #[arg(long, value_delimiter = ',')]
        only: Vec<String>,
        /// Disable these rules (comma separated).
        #[arg(long, value_delimiter = ',')]
        disable: Vec<String>,
        /// Enable these opt-in rules (comma separated).
        #[arg(long, value_delimiter = ',')]
        enable: Vec<String>,
        /// Run all rules, including opt-in ones.
        #[arg(long)]
        all_rules: bool,
        /// Hide findings below this severity.
        #[arg(long, default_value = "info")]
        min_severity: Severity,
        /// Exit with status 1 if any finding at or above this severity is reported.
        #[arg(long)]
        fail_on: Option<Severity>,
    },
    /// Print per-function metrics (size, complexity, nesting) as JSON lines.
    Metrics {
        paths: Vec<PathBuf>,
    },
    /// List all rules.
    Rules,
    /// Explain a rule: what it flags, why, and the supporting evidence.
    Explain { rule: String },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}

fn default_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    if paths.is_empty() { vec![PathBuf::from(".")] } else { paths }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check { paths, format, config, only, disable, enable, all_rules, min_severity, fail_on } => {
            let mut cfg = match load_config(config) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("error: {e}");
                    return ExitCode::from(2);
                }
            };
            cfg.only.extend(only);
            cfg.disable.extend(disable);
            cfg.enable.extend(enable);
            cfg.all_rules |= all_rules;
            for id in cfg.only.iter().chain(&cfg.disable).chain(&cfg.enable) {
                if rules::find(id).is_none() {
                    eprintln!("error: unknown rule `{id}` (see `anti-slop rules`)");
                    return ExitCode::from(2);
                }
            }
            let files = analyze::discover(&default_paths(paths));
            let all = rules::all_rules();
            let mut reports = analyze::analyze_files(&files, &cfg, &all);
            for r in &mut reports {
                r.findings.retain(|f| f.severity >= min_severity);
            }
            match format {
                Format::Text => print_text(&reports),
                Format::Json => print_json(&reports),
            }
            let failed = fail_on.is_some_and(|level| {
                reports.iter().flat_map(|r| &r.findings).any(|f| f.severity >= level)
            });
            if failed { ExitCode::from(1) } else { ExitCode::SUCCESS }
        }
        Command::Metrics { paths } => {
            let files = analyze::discover(&default_paths(paths));
            let cfg = Config { only: vec!["none".into()], ..Config::default() };
            for r in analyze::analyze_files(&files, &cfg, &[]) {
                if let Some(e) = &r.parse_error {
                    eprintln!("{}: parse error: {e}", r.file.display());
                }
                let kind = r.kind.unwrap_or(FileKind::Library);
                for f in &r.functions {
                    let mut v = serde_json::to_value(f).expect("FnMetrics serialises");
                    v["file"] = serde_json::Value::String(r.file.display().to_string());
                    v["file_kind"] = serde_json::Value::String(format!("{kind:?}").to_lowercase());
                    println!("{v}");
                }
            }
            ExitCode::SUCCESS
        }
        Command::Rules => {
            for r in rules::all_rules() {
                let m = r.meta();
                let opt = if m.default_enabled { "" } else { " (opt-in)" };
                println!(
                    "{:<28} {:<16} {:<8} evidence: {:<8} {}{opt}",
                    m.id, m.category, m.severity, m.evidence, m.summary
                );
            }
            ExitCode::SUCCESS
        }
        Command::Explain { rule } => match rules::find(&rule) {
            Some(r) => {
                let m = r.meta();
                println!("{} — {}\n", m.id, m.summary);
                println!("category: {}   severity: {}   evidence: {}\n", m.category, m.severity, m.evidence);
                println!("{}\n", m.rationale);
                println!("Sources:");
                for s in m.sources {
                    println!("  - {s}");
                }
                ExitCode::SUCCESS
            }
            None => {
                eprintln!("error: unknown rule `{rule}` (see `anti-slop rules`)");
                ExitCode::from(2)
            }
        },
    }
}

fn load_config(path: Option<PathBuf>) -> Result<Config, String> {
    match path {
        Some(p) => Config::load(&p),
        None => {
            let p = PathBuf::from("anti-slop.toml");
            if p.is_file() { Config::load(&p) } else { Ok(Config::default()) }
        }
    }
}

fn print_text(reports: &[FileReport]) {
    let mut total = 0;
    let mut by_sev = [0usize; 3];
    for r in reports {
        if let Some(e) = &r.parse_error {
            eprintln!("{}: could not parse: {e}", r.file.display());
        }
        for f in &r.findings {
            total += 1;
            by_sev[f.severity as usize] += 1;
            println!(
                "{}:{}:{}: {}[{}] {}",
                f.file.display(),
                f.line,
                f.column,
                f.severity,
                f.rule,
                f.message
            );
        }
    }
    eprintln!(
        "\n{} files checked, {} findings ({} error, {} warning, {} info)",
        reports.len(),
        total,
        by_sev[Severity::Error as usize],
        by_sev[Severity::Warning as usize],
        by_sev[Severity::Info as usize]
    );
}

fn print_json(reports: &[FileReport]) {
    let findings: Vec<_> = reports.iter().flat_map(|r| &r.findings).collect();
    let errors: Vec<_> = reports
        .iter()
        .filter_map(|r| r.parse_error.as_ref().map(|e| serde_json::json!({"file": r.file, "error": e})))
        .collect();
    let out = serde_json::json!({ "findings": findings, "parse_errors": errors });
    println!("{}", serde_json::to_string_pretty(&out).expect("findings serialise"));
}
