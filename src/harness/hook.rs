//! The hook protocol: JSON in on stdin, JSON out on stdout. Two dialects are read, told apart by
//! the input's shape: Claude Code's native events (fields at the top level), and Symposium's
//! canonical events (one key naming the event, `{"PreToolUse": {..}}`), which Symposium delivers
//! for every agent it supports. The reply is written in the dialect the input came in.
//!
//! Three events:
//! - `pre-tool-use`: before a `git commit` or `git push`, run the harness on what is about to go
//!   out and deny the command while blocking items remain.
//! - `post-tool-use`: after an edit to a `.rs` file, add that file's findings to the agent's
//!   context. Nothing is blocked and nothing the tool printed is replaced.
//! - `stop`: when the agent wants to finish, run the harness on everything since the base and
//!   send it back to work while blocking items remain, at most three times per session. Only
//!   Claude's native dialect can refuse a stop; in the canonical one the report is context.
//!
//! The hook fails open: if it can't read its input, find the repository or run git, it lets the
//! action through and says so on stderr. Policy failures are the only thing it blocks on. Every
//! run's report is also saved under the temp directory, so a handoff note can point at it.

use std::fs;
use std::path::{Path, PathBuf};

use super::diff::{self, Base};
use super::json::{self, Value};
use super::policy::Policy;
use super::{Selection, agent};
use crate::analyze_paths_with;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    PreToolUse,
    PostToolUse,
    Stop,
}

impl Event {
    pub fn parse(s: &str) -> Option<Event> {
        match s {
            "pre-tool-use" | "PreToolUse" => Some(Event::PreToolUse),
            "post-tool-use" | "PostToolUse" => Some(Event::PostToolUse),
            "stop" | "Stop" => Some(Event::Stop),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Event::PreToolUse => "PreToolUse",
            Event::PostToolUse => "PostToolUse",
            Event::Stop => "Stop",
        }
    }
}

/// Which dialect the input used, so the reply matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dialect {
    Claude,
    Canonical,
}

/// What the hook writes: stdout JSON (empty means no opinion), a stderr note, and the run, for
/// the log.
#[derive(Debug, Default)]
pub struct Outcome {
    pub stdout: String,
    pub stderr: String,
    pub run: Option<super::Run>,
}

/// How many times a `stop` may be sent back to work in one session before it is let through.
pub const MAX_STOP_BLOCKS: u32 = 3;

/// The shell tool, as each agent names it.
const SHELL_TOOLS: &[&str] = &["Bash", "bash", "run_command", "execute_bash", "shell"];
/// File-editing tools, as the agents name them.
const EDIT_TOOLS: &[&str] = &[
    "Edit",
    "Write",
    "MultiEdit",
    "NotebookEdit",
    "edit",
    "write",
    "edit_file",
    "write_file",
    "create_file",
    "apply_patch",
    "replace_file_content",
    "write_to_file",
    "str_replace_editor",
];
const PATH_KEYS: &[&str] = &[
    "file_path",
    "path",
    "filePath",
    "TargetFile",
    "AbsolutePath",
];

fn fail_open(why: impl Into<String>) -> Outcome {
    Outcome {
        stderr: format!("anti-slop hook: {}; letting the action through", why.into()),
        ..Outcome::default()
    }
}

/// `git commit` or `git push` as a shell command, however it is spelled around the words.
fn is_commit_or_push(command: &str) -> Option<Base> {
    let words: Vec<&str> = command.split_whitespace().collect();
    let mut it = words.iter().peekable();
    while let Some(w) = it.next() {
        if *w == "git" {
            // Skip `git -C dir` style options.
            let mut sub = it.peek().copied();
            while let Some(s) = sub
                && s.starts_with('-')
            {
                it.next();
                if matches!(*s, "-C" | "-c" | "--git-dir" | "--work-tree") {
                    it.next();
                }
                sub = it.peek().copied();
            }
            match sub {
                Some(&"commit") => return Some(Base::Staged),
                Some(&"push") => return Some(Base::Rev("@{upstream}".into())),
                _ => {}
            }
        }
    }
    None
}

/// Splits the input into its dialect and the event's fields.
fn unwrap_event(value: &Value, event: Event) -> Option<(Dialect, &Value)> {
    if let Value::Obj(pairs) = value
        && pairs.len() == 1
        && let (key, inner) = &pairs[0]
        && matches!(
            key.as_str(),
            "PreToolUse" | "PostToolUse" | "Stop" | "UserPromptSubmit" | "SessionStart"
        )
    {
        return (key == event.name()).then_some((Dialect::Canonical, inner));
    }
    Some((Dialect::Claude, value))
}

pub fn run(event: Event, input: &str, cwd: &Path) -> Outcome {
    let value = match json::parse(input) {
        Ok(v) => v,
        Err(e) => return fail_open(format!("could not read the hook input ({e})")),
    };
    let Some((dialect, v)) = unwrap_event(&value, event) else {
        return fail_open(format!("input is for another event than {}", event.name()));
    };
    let dir = v
        .get("cwd")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(|| cwd.to_path_buf());
    let root = match diff::repo_root(&dir) {
        Ok(r) => r,
        Err(_) => return fail_open("not in a git repository"),
    };
    let policy = match Policy::load(&root) {
        Ok(p) => p,
        Err(e) => return fail_open(format!("{e}")),
    };
    match event {
        Event::PreToolUse => pre_tool_use(v, dialect, &root, &policy),
        Event::PostToolUse => post_tool_use(v, dialect, &root, &policy),
        Event::Stop => stop(v, dialect, &root, &policy),
    }
}

fn tool_name(v: &Value) -> &str {
    v.get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or_default()
}

fn input_str<'a>(v: &'a Value, keys: &[&str]) -> Option<&'a str> {
    let input = v.get("tool_input")?;
    keys.iter()
        .find_map(|k| input.get(k).and_then(Value::as_str))
}

fn deny(dialect: Dialect, text: &str) -> String {
    match dialect {
        Dialect::Claude => format!(
            "{{\"hookSpecificOutput\": {{\"hookEventName\": \"PreToolUse\", \"permissionDecision\": \"deny\", \"permissionDecisionReason\": {}}}}}\n",
            json::string(text)
        ),
        Dialect::Canonical => format!(
            "{{\"PreToolUse\": {{\"decision\": \"deny\", \"additionalContext\": {}}}}}\n",
            json::string(text)
        ),
    }
}

fn context(dialect: Dialect, event: Event, text: &str) -> String {
    match dialect {
        Dialect::Claude => format!(
            "{{\"hookSpecificOutput\": {{\"hookEventName\": \"{}\", \"additionalContext\": {}}}}}\n",
            event.name(),
            json::string(text)
        ),
        Dialect::Canonical => format!(
            "{{\"{}\": {{\"additionalContext\": {}}}}}\n",
            event.name(),
            json::string(text)
        ),
    }
}

fn pre_tool_use(v: &Value, dialect: Dialect, root: &Path, policy: &Policy) -> Outcome {
    if !SHELL_TOOLS.contains(&tool_name(v)) {
        return Outcome::default();
    }
    let command = input_str(v, &["command", "CommandLine", "cmd"]).unwrap_or_default();
    let Some(mut base) = is_commit_or_push(command) else {
        return Outcome::default();
    };
    if base == Base::Rev("@{upstream}".into()) && diff::changes(root, &base).is_err() {
        base = Base::Rev(policy.base.clone());
    }
    let run = match super::run(root, &base, policy, &Selection::default()) {
        Ok(r) => r,
        Err(e) => return fail_open(format!("{e}")),
    };
    let text = agent::render(&run);
    let saved = save_report(v, &text);
    if run.blocking() == 0 {
        return Outcome {
            run: Some(run),
            ..Outcome::default()
        };
    }
    Outcome {
        stdout: deny(dialect, &format!("{text}{saved}")),
        stderr: String::new(),
        run: Some(run),
    }
}

fn post_tool_use(v: &Value, dialect: Dialect, root: &Path, policy: &Policy) -> Outcome {
    if !EDIT_TOOLS.contains(&tool_name(v)) {
        return Outcome::default();
    }
    let Some(file) = input_str(v, PATH_KEYS) else {
        return Outcome::default();
    };
    let path = PathBuf::from(file);
    if path.extension().is_none_or(|e| e != "rs") {
        return Outcome::default();
    }
    let path = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };
    let (reports, _) = analyze_paths_with(std::slice::from_ref(&path), policy.style);
    let mut lines = Vec::new();
    for (_, r) in &reports {
        for f in &r.findings {
            if f.severity >= policy.min_severity && !policy.skip.iter().any(|s| s == f.rule) {
                lines.push(format!(
                    "{:<8} {}:{}  {}  {}",
                    f.severity,
                    path.strip_prefix(root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/"),
                    f.line,
                    f.rule,
                    f.message
                ));
            }
        }
    }
    if lines.is_empty() {
        return Outcome::default();
    }
    let n = lines.len();
    lines.truncate(agent::CAP);
    let mut text = format!(
        "anti-slop: {n} finding(s) in the file just edited:\n{}\n",
        lines.join("\n")
    );
    if n > agent::CAP {
        text.push_str(&format!("... and {} more\n", n - agent::CAP));
    }
    text.push_str(
        "`anti-slop explain <rule>` for the evidence and the exceptions the source allows.",
    );
    Outcome {
        stdout: context(dialect, Event::PostToolUse, &text),
        ..Outcome::default()
    }
}

fn session_of(v: &Value) -> String {
    v.get("session_id")
        .and_then(Value::as_str)
        .unwrap_or("none")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect()
}

fn stop_state_path(session: &str) -> PathBuf {
    std::env::temp_dir().join(format!("anti-slop-stop-{session}"))
}

/// Saves the report beside the session state, outside the workspace, and returns a line that
/// says where. A handoff note can cite the path instead of repeating the report.
fn save_report(v: &Value, text: &str) -> String {
    let path = std::env::temp_dir().join(format!("anti-slop-report-{}.txt", session_of(v)));
    match fs::write(&path, text) {
        Ok(()) => format!(
            "(full report saved to {}; cite it from a handoff rather than copying it)\n",
            path.display()
        ),
        Err(_) => String::new(),
    }
}

fn stop(v: &Value, dialect: Dialect, root: &Path, policy: &Policy) -> Outcome {
    let state = stop_state_path(&session_of(v));
    let blocked_before: u32 = fs::read_to_string(&state)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let base = Base::Rev(policy.base.clone());
    let run = match super::run(root, &base, policy, &Selection::default()) {
        Ok(r) => r,
        Err(e) => return fail_open(format!("{e}")),
    };
    let text = agent::render(&run);
    let saved = save_report(v, &text);
    if run.blocking() == 0 {
        let _ = fs::remove_file(&state);
        return Outcome {
            run: Some(run),
            ..Outcome::default()
        };
    }
    if dialect == Dialect::Canonical {
        // No way to refuse a stop here; the agent gets the report as context.
        return Outcome {
            stdout: context(dialect, Event::Stop, &format!("{text}{saved}")),
            run: Some(run),
            ..Outcome::default()
        };
    }
    if blocked_before >= MAX_STOP_BLOCKS {
        let _ = fs::remove_file(&state);
        return Outcome {
            stderr: format!(
                "anti-slop hook: {} blocking item(s) remain after {MAX_STOP_BLOCKS} rounds; letting the agent stop so a person can look\n{text}{saved}",
                run.blocking()
            ),
            run: Some(run),
            ..Outcome::default()
        };
    }
    let _ = fs::write(&state, (blocked_before + 1).to_string());
    Outcome {
        stdout: format!(
            "{{\"decision\": \"block\", \"reason\": {}}}\n",
            json::string(&format!(
                "{text}{saved}(round {} of {MAX_STOP_BLOCKS}; after that the stop goes through and the report goes to a person)",
                blocked_before + 1
            ))
        ),
        stderr: String::new(),
        run: Some(run),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_commit_and_push() {
        assert_eq!(is_commit_or_push("git commit -m 'x'"), Some(Base::Staged));
        assert_eq!(
            is_commit_or_push("cd a && git -C sub push -u origin main"),
            Some(Base::Rev("@{upstream}".into()))
        );
        assert_eq!(is_commit_or_push("git status && cargo test"), None);
        assert_eq!(is_commit_or_push("echo commit"), None);
    }

    #[test]
    fn fails_open_on_bad_input_or_wrong_event() {
        let o = run(Event::Stop, "{not json", Path::new("."));
        assert!(o.stdout.is_empty() && o.stderr.contains("letting the action through"));
        let o = run(
            Event::Stop,
            r#"{"PreToolUse": {"tool_name": "Bash"}}"#,
            Path::new("."),
        );
        assert!(o.stdout.is_empty() && o.stderr.contains("another event"));
    }

    #[test]
    fn dialect_is_read_from_the_shape() {
        let canonical =
            json::parse(r#"{"PreToolUse": {"tool_name": "bash", "cwd": "/x"}}"#).expect("json");
        let (d, inner) = unwrap_event(&canonical, Event::PreToolUse).expect("matches");
        assert_eq!(d, Dialect::Canonical);
        assert_eq!(tool_name(inner), "bash");
        let native =
            json::parse(r#"{"hook_event_name": "PreToolUse", "tool_name": "Bash", "cwd": "/x"}"#)
                .expect("json");
        let (d, inner) = unwrap_event(&native, Event::PreToolUse).expect("matches");
        assert_eq!(d, Dialect::Claude);
        assert_eq!(tool_name(inner), "Bash");
        assert!(
            deny(Dialect::Canonical, "r").starts_with("{\"PreToolUse\": {\"decision\": \"deny\"")
        );
        assert!(deny(Dialect::Claude, "r").contains("\"permissionDecision\": \"deny\""));
    }
}
