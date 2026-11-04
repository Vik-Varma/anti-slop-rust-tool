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

/// A repository whose base commit holds exactly `files`.
fn repo_with(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("anti-slop-harness-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("dir");
    git(&dir, &["init", "-q", "-b", "main"]);
    write(
        &dir,
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    for (path, body) in files {
        write(&dir, path, body);
    }
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-q", "-m", "base"]);
    dir
}

fn guard_text(dir: &Path) -> (Option<i32>, String) {
    let o = bin()
        .args(["guard", "--since", "HEAD", "--format", "agent"])
        .arg(dir)
        .output()
        .expect("runs");
    (o.status.code(), out_text(&o))
}

const GATE: &str = "#[test]\nfn overlapping_windows_cannot_be_reserved_together() { assert!(true); }\n\n\
                    #[test]\nfn non_overlapping_windows_can_be_reserved_together() { assert!(true); }\n\n\
                    #[test]\nfn dropping_reservation_releases_window() { assert!(true); }\n";

/// Reported against `guard` by the SteelMC #775 review: three tests moved to a new file with the
/// same names and assertions, plus two new ones, is not a removal. The new file is untracked,
/// which `git diff` alone doesn't show.
#[test]
fn moved_tests_are_not_removed_tests() {
    let dir = repo_with(
        "moved",
        &[
            ("tests/work_gate.rs", GATE),
            ("src/lib.rs", "pub fn f() {}\n"),
        ],
    );
    fs::remove_file(dir.join("tests/work_gate.rs")).expect("remove");
    let more = format!(
        "{GATE}\n#[test]\nfn extra_one() {{ assert!(true); }}\n\n#[test]\nfn extra_two() {{ assert!(true); }}\n"
    );
    write(&dir, "tests/window_gate.rs", &more);
    let (code, text) = guard_text(&dir);
    assert!(!text.contains("tests-removed"), "{text}");
    assert!(!text.contains("assertions-removed"), "{text}");
    assert_eq!(code, Some(0), "{text}");
    let _ = fs::remove_dir_all(&dir);
}

const SLOW_BASE: &str = "pub fn f() -> u8 { 1 }\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn old_fast() { assert_eq!(super::f(), 1); }\n}\n";

#[test]
fn a_new_test_added_ignored_is_not_a_disabled_test() {
    let dir = repo_with("newignored", &[("src/lib.rs", SLOW_BASE)]);
    write(
        &dir,
        "src/lib.rs",
        &SLOW_BASE.replace(
            "}\n}\n",
            "}\n\n    #[test]\n    #[ignore = \"slow; run with --release\"]\n    fn new_slow() { assert_eq!(super::f(), 1); }\n}\n",
        ),
    );
    let (code, text) = guard_text(&dir);
    assert_eq!(code, Some(0), "{text}");
    assert!(
        text.contains("new-test-ignored") && text.contains("new_slow"),
        "{text}"
    );
    assert!(!text.contains("block  tests-ignored"), "{text}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ignoring_an_existing_test_still_blocks() {
    let dir = repo_with("oldignored", &[("src/lib.rs", SLOW_BASE)]);
    write(
        &dir,
        "src/lib.rs",
        &SLOW_BASE.replace("    #[test]\n", "    #[test]\n    #[ignore]\n"),
    );
    let (code, text) = guard_text(&dir);
    assert_eq!(code, Some(1), "{text}");
    assert!(text.contains("block  tests-ignored"), "{text}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn removed_renamed_and_disabled_tests() {
    // Deleted outright: blocks.
    let dir = repo_with("deleted", &[("src/lib.rs", SLOW_BASE)]);
    write(&dir, "src/lib.rs", "pub fn f() -> u8 { 1 }\n");
    let (code, text) = guard_text(&dir);
    assert_eq!(code, Some(1), "{text}");
    assert!(
        text.contains("block  tests-removed") && text.contains("old_fast"),
        "{text}"
    );
    let _ = fs::remove_dir_all(&dir);
    // Only the function's name changed: the test still runs, so there is nothing to report.
    let dir = repo_with("renamed", &[("src/lib.rs", SLOW_BASE)]);
    write(
        &dir,
        "src/lib.rs",
        &SLOW_BASE.replace("old_fast", "old_quick"),
    );
    let (code, text) = guard_text(&dir);
    assert_eq!(code, Some(0), "{text}");
    assert!(!text.contains("tests-removed"), "{text}");
    let _ = fs::remove_dir_all(&dir);
    // One test removed and a different one added elsewhere: reported, not blocked, because the
    // count of tests is the same and a person can see which it was.
    let base = "pub fn f() -> u8 { 1 }\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn old_fast() { assert_eq!(super::f(), 1); }\n\n    fn helper() {}\n}\n";
    let dir = repo_with("replaced", &[("src/lib.rs", base)]);
    write(
        &dir,
        "src/lib.rs",
        "pub fn f() -> u8 { 1 }\n\n#[cfg(test)]\nmod tests {\n    fn helper() {}\n\n    #[test]\n    fn replacement() { assert_eq!(super::f(), 1); }\n}\n",
    );
    let (code, text) = guard_text(&dir);
    assert_eq!(code, Some(0), "{text}");
    assert!(
        text.contains("tests-renamed") && text.contains("old_fast") && text.contains("replacement"),
        "{text}"
    );
    let _ = fs::remove_dir_all(&dir);
    // The attribute removed, the function left behind: the test no longer runs.
    let dir = repo_with("disabled", &[("src/lib.rs", SLOW_BASE)]);
    write(&dir, "src/lib.rs", &SLOW_BASE.replace("    #[test]\n", ""));
    let (code, text) = guard_text(&dir);
    assert_eq!(code, Some(1), "{text}");
    assert!(text.contains("block  tests-removed"), "{text}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_file_over_the_limit_is_reported_only_when_the_change_grows_it() {
    let base = "pub fn a() -> u8 { 1 }\npub fn b() -> u8 { 2 }\npub fn c() -> u8 { 3 }\npub fn d() -> u8 { 4 }\n";
    let dir = repo_with(
        "size",
        &[
            ("src/lib.rs", base),
            ("anti-slop.toml", "max_file_lines = 3\n"),
        ],
    );
    write(&dir, "src/lib.rs", &base.replace("{ 4 }", "{ 5 }"));
    let (_, text) = guard_text(&dir);
    assert!(
        !text.contains("code lines, over the"),
        "unchanged size: {text}"
    );
    write(
        &dir,
        "src/lib.rs",
        &format!("{base}pub fn e() -> u8 {{ 6 }}\n"),
    );
    let (_, text) = guard_text(&dir);
    assert!(
        text.contains("code lines, over the 3-line limit"),
        "grew: {text}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn untracked_files_are_part_of_the_change() {
    let dir = repo_with("untracked", &[("src/lib.rs", "pub fn f() {}\n")]);
    write(
        &dir,
        "src/new.rs",
        "pub fn g(x: f64) -> bool { x == 0.1 }\n",
    );
    let (code, text) = guard_text(&dir);
    assert_eq!(code, Some(1), "{text}");
    assert!(
        text.contains("src/new.rs:1") && text.contains("float-eq-inexact"),
        "{text}"
    );
    let _ = fs::remove_dir_all(&dir);
}
