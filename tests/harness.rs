//! The harness end to end, against a throwaway git repository: `check --since`, `guard`, the
//! policy file, and the hook protocol in both dialects. Needs `git` on the path, as the harness
//! does.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_anti-slop"))
}

fn git(repo: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write(repo: &Path, rel: &str, body: &str) {
    let p = repo.join(rel);
    fs::create_dir_all(p.parent().expect("parent")).expect("dir");
    fs::write(p, body).expect("write");
}

const BASE_LIB: &str = "pub fn old(x: f64) -> bool { x == 0.2 }\n\npub fn keep() -> u8 { 1 }\n";
const BASE_TESTS: &str =
    "#[test]\nfn a() { assert!(true); }\n\n#[test]\nfn b() { assert_eq!(1, 1); }\n";

/// A repository with one commit, then a working-tree change that trips every default block.
fn repo(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("anti-slop-harness-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("dir");
    git(&dir, &["init", "-q", "-b", "main"]);
    write(
        &dir,
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(&dir, "src/lib.rs", BASE_LIB);
    write(&dir, "tests/t.rs", BASE_TESTS);
    write(&dir, "benches/b.rs", "fn bench() { for _ in 0..1000 {} }\n");
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    dir
}

fn make_bad_change(dir: &Path) {
    write(
        dir,
        "src/lib.rs",
        &format!(
            "{BASE_LIB}\npub fn new(x: f64) -> bool {{ x == 0.1 }}\n\npub fn later() {{\n    // anti-slop: allow(todo) wip\n    todo!()\n}}\n"
        ),
    );
    write(dir, "tests/t.rs", "#[test]\nfn a() { assert!(true); }\n");
    write(dir, "benches/b.rs", "fn bench() { for _ in 0..10 {} }\n");
}

fn out_text(o: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    )
}

#[test]
fn guard_reports_and_blocks() {
    let dir = repo("guard");
    make_bad_change(&dir);
    let o = bin()
        .args(["guard", "--since", "HEAD", "--format", "agent"])
        .arg(&dir)
        .output()
        .expect("runs");
    let text = out_text(&o);
    assert_eq!(o.status.code(), Some(1), "{text}");
    assert!(text.starts_with("anti-slop: FAIL"), "{text}");
    // The new finding is listed; the pre-existing one on an untouched line is only counted.
    assert!(text.contains("src/lib.rs:5  float-eq-inexact"), "{text}");
    assert!(!text.contains("src/lib.rs:1 "), "{text}");
    assert!(text.contains("1 pre-existing finding"), "{text}");
    for needle in [
        "block  suppression-without-reason  src/lib.rs:8",
        "note   deferrals                   src/lib.rs:9",
        "block  tests-removed               tests/t.rs",
        "note   measuring-stick             benches/b.rs",
        "note   measuring-stick             tests/t.rs",
        "code lines: +",
        "anti-slop explain <rule>",
    ] {
        assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn check_since_text_and_json_and_staged() {
    let dir = repo("check");
    make_bad_change(&dir);
    let o = bin()
        .args(["check", "--since", "HEAD"])
        .arg(&dir)
        .output()
        .expect("runs");
    let text = out_text(&o);
    assert_eq!(o.status.code(), Some(1), "{text}");
    assert!(
        text.contains("float-eq-inexact") && text.contains("1 finding(s)"),
        "{text}"
    );
    assert!(!text.contains("guard:"), "{text}");
    let o = bin()
        .args(["check", "--since", "HEAD", "--format", "json"])
        .arg(&dir)
        .output()
        .expect("runs");
    let json = String::from_utf8_lossy(&o.stdout);
    assert!(
        json.trim_start().starts_with('[') && json.contains("\"rule\": \"float-eq-inexact\""),
        "{json}"
    );
    // Nothing staged yet: a clean report and exit 0.
    let o = bin()
        .args(["guard", "--staged"])
        .arg(&dir)
        .output()
        .expect("runs");
    assert_eq!(o.status.code(), Some(0), "{}", out_text(&o));
    git(&dir, &["add", "."]);
    let o = bin()
        .args(["guard", "--staged", "--format", "agent"])
        .arg(&dir)
        .output()
        .expect("runs");
    assert_eq!(o.status.code(), Some(1), "{}", out_text(&o));
    assert!(out_text(&o).contains("tests-removed"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn policy_file_changes_what_blocks() {
    let dir = repo("policy");
    make_bad_change(&dir);
    write(
        &dir,
        "anti-slop.toml",
        "block = [\"tests-removed\"]\nmin_reason_chars = 2\n",
    );
    let o = bin()
        .args(["guard", "--since", "HEAD", "--format", "agent"])
        .arg(&dir)
        .output()
        .expect("runs");
    let text = out_text(&o);
    assert_eq!(o.status.code(), Some(1), "{text}");
    // Findings still count (they always do), the short reason now passes, tests-removed blocks.
    assert!(text.contains("note   suppressions"), "{text}");
    assert!(text.contains("block  tests-removed"), "{text}");
    write(&dir, "anti-slop.toml", "blok = []\n");
    let o = bin()
        .args(["guard", "--since", "HEAD"])
        .arg(&dir)
        .output()
        .expect("runs");
    assert_eq!(o.status.code(), Some(2), "{}", out_text(&o));
    assert!(out_text(&o).contains("unknown key `blok`"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn run_log_records_counts_only() {
    let dir = repo("log");
    make_bad_change(&dir);
    let log = dir.join("runs.jsonl");
    bin()
        .args(["guard", "--since", "HEAD", "--format", "agent", "--log"])
        .arg(&log)
        .arg(&dir)
        .output()
        .expect("runs");
    let line = fs::read_to_string(&log).expect("log written");
    assert!(
        line.contains("\"source\": \"guard\"")
            && line.contains("\"findings\": 1")
            && line.contains("\"bytes_emitted\""),
        "{line}"
    );
    assert!(!line.contains("saved"), "no estimates: {line}");
    let _ = fs::remove_dir_all(&dir);
}

fn hook(event: &str, input: &str, cwd: &Path) -> Output {
    let mut child = bin()
        .args(["hook", event])
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawns");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("writes");
    child.wait_with_output().expect("finishes")
}

#[test]
fn hooks_in_both_dialects() {
    let dir = repo("hook");
    make_bad_change(&dir);
    git(&dir, &["add", "."]);
    let cwd = json_escape(&dir.to_string_lossy());
    // Claude's native shape: a commit is denied with the report as the reason.
    let o = hook(
        "pre-tool-use",
        &format!(
            r#"{{"session_id":"s1","cwd":"{cwd}","hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{{"command":"git commit -m x"}}}}"#
        ),
        &dir,
    );
    let out = String::from_utf8_lossy(&o.stdout);
    assert_eq!(o.status.code(), Some(0));
    assert!(
        out.contains("\"permissionDecision\": \"deny\"") && out.contains("tests-removed"),
        "{out}"
    );
    // An unrelated command gets no opinion.
    let o = hook(
        "pre-tool-use",
        &format!(r#"{{"cwd":"{cwd}","tool_name":"Bash","tool_input":{{"command":"cargo test"}}}}"#),
        &dir,
    );
    assert!(o.stdout.is_empty(), "{}", out_text(&o));
    // Symposium's canonical shape, from another agent's shell tool: canonical reply.
    let o = hook(
        "pre-tool-use",
        &format!(
            r#"{{"PreToolUse":{{"tool_name":"execute_bash","tool_input":{{"command":"git commit"}},"session_id":"s2","cwd":"{cwd}"}}}}"#
        ),
        &dir,
    );
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        out.starts_with("{\"PreToolUse\": {\"decision\": \"deny\""),
        "{out}"
    );
    // After an edit, the file's findings come back as context, never as a block.
    let file = json_escape(&dir.join("src/lib.rs").to_string_lossy());
    let o = hook(
        "post-tool-use",
        &format!(r#"{{"cwd":"{cwd}","tool_name":"Edit","tool_input":{{"file_path":"{file}"}}}}"#),
        &dir,
    );
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        out.contains("\"additionalContext\"")
            && out.contains("float-eq-inexact")
            && !out.contains("deny"),
        "{out}"
    );
    // Stop: blocked three times, then let through with the report on stderr.
    let session = format!("stop-{}", std::process::id());
    for round in 1..=4 {
        let o = hook(
            "stop",
            &format!(r#"{{"session_id":"{session}","cwd":"{cwd}","hook_event_name":"Stop"}}"#),
            &dir,
        );
        let out = String::from_utf8_lossy(&o.stdout);
        if round <= 3 {
            assert!(
                out.contains("\"decision\": \"block\"")
                    && out.contains(&format!("round {round} of 3")),
                "{round}: {out}"
            );
        } else {
            assert!(out.is_empty(), "{out}");
            assert!(String::from_utf8_lossy(&o.stderr).contains("letting the agent stop"));
        }
    }
    // Bad input fails open.
    let o = hook("stop", "nope", &dir);
    assert!(o.status.success() && o.stdout.is_empty());
    let _ = fs::remove_dir_all(&dir);
}

/// The agent format must keep every rule id and `file:line`, whatever else is cut.
#[test]
fn agent_format_preserves_locations() {
    let dir = repo("preserve");
    let mut body = String::from(BASE_LIB);
    for i in 0..40 {
        body.push_str(&format!(
            "pub fn f{i}(x: f64) -> bool {{ x == 0.{} }}\n",
            i + 1
        ));
    }
    write(&dir, "src/lib.rs", &body);
    let o = bin()
        .args(["check", "--since", "HEAD", "--format", "agent"])
        .arg(&dir)
        .output()
        .expect("runs");
    let text = String::from_utf8_lossy(&o.stdout);
    let shown = text
        .lines()
        .filter(|l| l.contains("float-eq-inexact"))
        .count();
    assert_eq!(shown, 30, "{text}");
    // `0.5` and `0.25` are exact, so 38 findings: 30 shown and 8 more.
    assert!(text.contains("... and 8 more"), "{text}");
    for l in text.lines().filter(|l| l.contains("float-eq-inexact")) {
        assert!(l.contains("src/lib.rs:"), "{l}");
    }
    let _ = fs::remove_dir_all(&dir);
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
