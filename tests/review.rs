//! Regression tests for the defects found in the code review (docs/reviews/code-review.md). Each
//! test is the review's reproduction, numbered as there, and failed before its fix.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anti_slop::{analyze_paths, analyze_source};

fn rules_in(path: &str, src: &str) -> Vec<(usize, &'static str)> {
    let r = analyze_source(Path::new(path), src).expect("parses");
    r.findings.iter().map(|f| (f.line, f.rule)).collect()
}

fn crate_dir(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("anti-slop-review-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    for (path, body) in files {
        let p = dir.join(path);
        fs::create_dir_all(p.parent().expect("parent")).expect("dir");
        fs::write(p, body).expect("write");
    }
    dir
}

fn rules_under(paths: &[PathBuf]) -> Vec<String> {
    let (reports, errors) = analyze_paths(paths);
    assert!(errors.is_empty(), "{errors:?}");
    let mut out: Vec<String> = reports
        .iter()
        .flat_map(|(_, r)| &r.findings)
        .map(|f| {
            format!(
                "{}:{}",
                f.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default(),
                f.rule
            )
        })
        .collect();
    out.sort();
    out
}

const TODO: &str = "pub fn t() -> u8 { todo!() }\n";

#[test]
fn r01_const_fn_and_lazy_closures_run_at_run_time() {
    let src = "pub const fn first(x: Option<u8>) -> u8 { x.unwrap() }\n\
               pub const fn later() -> u8 { todo!() }\n\
               static C: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| std::fs::read_to_string(path()).unwrap());\n\
               struct S;\n\
               impl S { const X: u8 = match Some(1) { Some(_) => Some(3).unwrap(), None => 0 }; }\n\
               trait T { const Y: u8 = Some(3).unwrap(); }\n\
               fn f() -> u8 { const { Some(3).unwrap() } }\n";
    assert_eq!(
        rules_in("src/lib.rs", src),
        [(1, "unwrap"), (2, "todo"), (3, "panic-on-input")]
    );
}

#[test]
fn r02_modules_named_target_or_vendor_are_read() {
    let dir = crate_dir(
        "target",
        &[
            ("Cargo.toml", "[package]\nname = \"x\"\n"),
            ("src/lib.rs", "pub mod target;\npub mod vendor;\n"),
            ("src/target/mod.rs", TODO),
            ("src/vendor/mod.rs", TODO),
            ("target/debug/build.rs", TODO),
        ],
    );
    fs::write(
        dir.join("target/CACHEDIR.TAG"),
        "Signature: 8a477f597d28d172789f06886806bc55",
    )
    .expect("tag");
    assert_eq!(
        rules_under(std::slice::from_ref(&dir)),
        ["mod.rs:todo", "mod.rs:todo"]
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn r03_doc_comments_code_and_strings_are_not_justifications() {
    let src = "/// Reads one byte.\npub fn read(p: *const u8) -> u8 {\n    unsafe { *p }\n}\n\
               pub fn f(p: *mut u8, q: &mut u8) {\n    *q = 1;\n    unsafe { *p = 2 }\n}\n\
               pub fn g() { unsafe { open(\"https://example.com\\0\".as_ptr()) } }\n\
               pub fn ok(p: *const u8) -> u8 {\n    // SAFETY: p is valid.\n    unsafe { *p }\n}\n";
    assert_eq!(
        rules_in("src/lib.rs", src),
        [
            (3, "unsafe-without-comment"),
            (7, "unsafe-without-comment"),
            (9, "unsafe-without-comment")
        ]
    );
}

#[test]
fn r04_shadowing_keeps_the_guard_and_async_closures_are_scanned() {
    let src = "pub async fn a(m: &std::sync::Mutex<u8>) -> u8 {\n    let g = m.lock().unwrap(); let g = *g;\n    other().await;\n    g\n}\n\
               pub fn b(m: std::sync::Arc<std::sync::Mutex<u8>>) {\n    let f = async move || {\n        let g = m.lock().unwrap();\n        other().await;\n        drop(g);\n    };\n}\n";
    assert_eq!(
        rules_in("src/lib.rs", src),
        [(3, "lock-held-across-await"), (9, "lock-held-across-await")]
    );
}

#[test]
fn r05_cfg_test_module_layouts() {
    let dir = crate_dir(
        "layout",
        &[
            ("Cargo.toml", "[package]\nname = \"x\"\n"),
            (
                "src/lib.rs",
                "#[cfg(test)]\nmod unit; // unit tests\n\
                 #[cfg(test)]\nmod inline {\n    mod helpers;\n}\n\
                 mod a {\n    #[cfg(test)]\n    mod deep;\n}\n\
                 #[cfg(all(feature = \"latest\", unix))]\nmod backend;\n\
                 #[cfg(test)]\n#[path = \"spec/mod.rs\"]\nmod spec;\n\
                 #[cfg(all(\n    test,\n    unix\n))]\nmod multi;\n",
            ),
            ("src/unit.rs", TODO),
            ("src/inline/helpers.rs", TODO),
            ("src/a/deep.rs", TODO),
            ("src/backend.rs", TODO),
            ("src/spec/mod.rs", "mod helpers;\n"),
            ("src/spec/helpers.rs", TODO),
            ("src/multi.rs", TODO),
        ],
    );
    assert_eq!(rules_under(std::slice::from_ref(&dir)), ["backend.rs:todo"]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn r06_binary_crate_detection_doesnt_depend_on_file_order() {
    let dir = crate_dir(
        "bincache",
        &[
            ("Cargo.toml", "[package]\nname = \"b1\"\n"),
            ("src/main.rs", "mod cli;\nfn main() {}\n"),
            (
                "src/cli.rs",
                "pub fn fail() -> ! { std::process::exit(1) }\n",
            ),
            ("codegen/gen.rs", "pub fn g() {}\n"),
        ],
    );
    assert!(rules_under(std::slice::from_ref(&dir)).is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn r07_a_file_given_by_absolute_path_isnt_judged_by_its_ancestors() {
    let dir = crate_dir(
        "abs/test",
        &[
            ("proj/Cargo.toml", "[package]\nname = \"p\"\n"),
            ("proj/src/lib.rs", TODO),
        ],
    );
    let file = dir.join("proj/src/lib.rs");
    assert!(file.is_absolute());
    assert_eq!(rules_under(&[file]), ["lib.rs:todo"]);
    let _ = fs::remove_dir_all(dir.parent().expect("parent"));
}

#[test]
fn r08_annotated_maybe_uninit_array_is_sound() {
    let src = "use std::mem::MaybeUninit;\npub fn arr() -> [MaybeUninit<Vec<u32>>; 4] {\n    \
               // SAFETY: an array of MaybeUninit needs no initialisation.\n    \
               let data: [MaybeUninit<Vec<u32>>; 4] = unsafe { MaybeUninit::uninit().assume_init() };\n    data\n}\n\
               pub fn bad() -> u32 {\n    // SAFETY: none.\n    let x: u32 = unsafe { MaybeUninit::uninit().assume_init() };\n    x\n}\n";
    assert_eq!(rules_in("src/lib.rs", src), [(9, "uninit-memory")]);
}

#[test]
fn r09_literal_paths_and_addresses_are_still_input() {
    let src = "pub fn home() -> String { std::env::var(\"HOME\").unwrap() }\n\
               pub fn cfg() -> std::fs::File { std::fs::File::open(\"/etc/app.toml\").unwrap() }\n\
               pub fn conn() -> std::net::TcpStream { std::net::TcpStream::connect(\"10.0.0.1:5432\").unwrap() }\n\
               pub fn lit() -> serde_json::Value { serde_json::from_str(\"{}\").unwrap() }\n";
    assert_eq!(
        rules_in("src/lib.rs", src),
        [
            (1, "panic-on-input"),
            (2, "panic-on-input"),
            (3, "panic-on-input"),
            (4, "unwrap")
        ]
    );
}

#[test]
fn r10_underscore_assignments_and_awaited_writes() {
    let src = "use std::io::Write;\npub fn a(f: &mut std::fs::File, b: &[u8]) -> std::io::Result<()> {\n    \
               _ = f.write_all(b);\n    _ = f.sync_all();\n    let _ = f.flush();\n    Ok(())\n}\n\
               pub async fn b(f: &mut tokio::fs::File, buf: &[u8]) -> std::io::Result<()> {\n    \
               let _ = f.write_all(buf).await;\n    f.flush().await.ok();\n    Ok(())\n}\n";
    let got: Vec<usize> = rules_in("src/lib.rs", src)
        .iter()
        .map(|(l, _)| *l)
        .collect();
    assert_eq!(got, [3, 4, 5, 9, 10]);
}

#[test]
fn r11_deny_warnings_with_a_reason() {
    assert_eq!(
        rules_in(
            "src/lib.rs",
            "#![deny(warnings, reason = \"keep the build clean\")]\npub fn a() {}\n"
        ),
        [(1, "deny-warnings")]
    );
}

#[test]
fn r12_frontmatter_fences_blank_lines_and_bom() {
    let long = "----\npackage.description = \"\"\"\n---\n\"\"\"\n----\nfn main() { todo!() }\n";
    let blank = "\n---\n[dependencies]\n---\nfn main() { todo!() }\n";
    let bom = "\u{feff}---\n[dependencies]\n---\nfn main() { todo!() }\n";
    assert_eq!(rules_in("src/main.rs", long), [(6, "todo")]);
    assert_eq!(rules_in("src/main.rs", blank), [(5, "todo")]);
    assert_eq!(rules_in("src/main.rs", bom), [(4, "todo")]);
}

#[test]
fn r13_trailing_suppression_covers_only_its_line_and_strings_arent_comments() {
    let src = "pub fn f(x: Option<u8>, y: Option<u8>) -> u8 {\n    let a = x.unwrap(); // anti-slop: allow(unwrap) by construction\n    \
               let b = y.unwrap();\n    a + b\n}\npub fn g(x: Option<u8>) -> u8 {\n    \
               let help = \"write // anti-slop: allow(unwrap) to silence\";\n    x.unwrap()\n}\n";
    assert_eq!(rules_in("src/lib.rs", src), [(3, "unwrap"), (8, "unwrap")]);
}

#[test]
fn r14_guarded_destructors() {
    let src = "pub struct G { armed: bool }\nimpl Drop for G {\n    fn drop(&mut self) {\n        \
               if std::thread::panicking() { return; }\n        panic!(\"armed\");\n    }\n}\n\
               pub struct H { armed: bool }\nimpl Drop for H {\n    fn drop(&mut self) {\n        \
               if self.armed && !std::thread::panicking() {\n            panic!(\"armed\");\n        }\n    }\n}\n";
    assert!(rules_in("src/lib.rs", src).is_empty());
}

#[test]
fn r15_cfg_test_trait_is_test_code() {
    assert!(
        rules_in(
            "src/lib.rs",
            "#[cfg(test)]\ntrait Fixture { fn make() -> u8 { Some(1).unwrap() } }\n"
        )
        .is_empty()
    );
}

#[test]
fn r18_findings_count_once_per_function() {
    let src = "pub fn a(x: Option<u8>) -> u8 { x.unwrap() } pub fn b() -> u8 { 0 }\n\
               pub struct S;\nimpl S { fn outer(&self) -> u8 { fn helper(x: Option<u8>) -> u8 { x.unwrap() } helper(None) } }\n";
    let r = analyze_source(Path::new("src/lib.rs"), src).expect("parses");
    let per: Vec<(String, usize)> = r
        .functions
        .iter()
        .enumerate()
        .map(|(i, m)| {
            (
                m.name.clone(),
                r.findings
                    .iter()
                    .filter(|f| f.function_index == Some(i))
                    .count(),
            )
        })
        .collect();
    assert_eq!(
        per,
        [
            ("a".into(), 1),
            ("b".into(), 0),
            ("S::outer".into(), 0),
            ("helper".into(), 1)
        ]
    );
}

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_anti-slop"))
}

#[test]
fn r19_csv_quotes_by_doubling() {
    let dir = crate_dir("csv", &[("we\"ird.rs", "fn f() {}\n")]);
    let out = bin().arg("metrics").arg(&dir).output().expect("runs");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("we\"\"ird.rs\""), "{text}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn r20_cli_exit_codes_and_options() {
    assert_eq!(
        bin().arg("--help").output().expect("runs").status.code(),
        Some(0)
    );
    let yaml = bin()
        .args(["metrics", "--format", "yaml", "src"])
        .output()
        .expect("runs");
    assert_eq!(yaml.status.code(), Some(2));
    let only = bin()
        .args(["metrics", "--only", "todo", "src"])
        .output()
        .expect("runs");
    assert_eq!(only.status.code(), Some(2));
}

#[test]
fn r21_names_of_nested_fns_and_dyn_impls() {
    let src = "pub struct S;\nimpl S { fn outer(&self) { fn helper() { None::<u8>.unwrap(); } } }\n\
               pub trait B {}\nimpl dyn B { fn down(&self) { None::<u8>.unwrap(); } }\n";
    let r = analyze_source(Path::new("src/lib.rs"), src).expect("parses");
    let names: Vec<&str> = r.functions.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["S::outer", "helper", "dyn B::down"]);
}

#[test]
fn r22_overlapping_paths_report_once() {
    let dir = crate_dir(
        "overlap",
        &[
            ("Cargo.toml", "[package]\nname = \"x\"\n"),
            ("src/lib.rs", TODO),
        ],
    );
    let paths = [dir.clone(), dir.join("src"), dir.join("src/lib.rs")];
    assert_eq!(rules_under(&paths), ["lib.rs:todo"]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn r23_catalogue_says_what_the_checks_do() {
    let poi = anti_slop::rules::get("panic-on-input").expect("rule");
    assert!(!poi.summary.contains("`.expect()` on"), "{}", poi.summary);
    let pid = anti_slop::rules::get("panic-in-drop").expect("rule");
    assert!(!pid.summary.contains("assert!"), "{}", pid.summary);
}
