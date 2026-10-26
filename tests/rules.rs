//! Each rule against code it must flag and code it must leave alone. A line ending in `//~ rule`
//! expects exactly that finding on that line; every other line expects none.

use std::path::Path;

use anti_slop::analyze_source;

fn check_at(path: &str, src: &str) {
    // The markers are comments, which some rules read, so analyse the source without them.
    let unmarked: Vec<&str> = src
        .lines()
        .map(|l| l.split("//~").next().unwrap_or(l))
        .collect();
    let report = analyze_source(Path::new(path), &unmarked.join("\n")).expect("fixture parses");
    let mut expected: Vec<(usize, String)> = Vec::new();
    for (i, line) in src.lines().enumerate() {
        if let Some((_, rules)) = line.split_once("//~") {
            for rule in rules.split_whitespace() {
                expected.push((i + 1, rule.to_string()));
            }
        }
    }
    let mut got: Vec<(usize, String)> = report
        .findings
        .iter()
        .map(|f| (f.line, f.rule.to_string()))
        .collect();
    expected.sort();
    got.sort();
    assert_eq!(got, expected, "\nfindings: {:#?}", report.findings);
}

fn check(src: &str) {
    check_at("src/lib.rs", src);
}

#[test]
fn unwrap() {
    check(
        r#"
        fn a(x: Option<u8>) -> u8 { x.unwrap() } //~ unwrap
        fn b(x: Option<u8>) -> u8 { x.expect("caller checked") }
        fn c(m: &std::sync::Mutex<u8>) -> u8 { *m.lock().unwrap() }
        fn d(m: &std::sync::RwLock<u8>) -> u8 { *m.read().unwrap() + *m.write().unwrap() }
        fn e() -> std::net::IpAddr { "127.0.0.1".parse().unwrap() }
        fn f(x: Result<u8, ()>) -> u8 { x.unwrap_or(0) }
        const fn g() -> u8 { match Some(1) { Some(v) => v, None => 0 } }
        #[cfg(test)]
        mod tests {
            fn h(x: Option<u8>) -> u8 { x.unwrap() }
        }
        #[test]
        fn i() { Some(1).unwrap(); }
        #[cfg(not(test))]
        fn j(x: Option<u8>) -> u8 { x.unwrap() } //~ unwrap
        "#,
    );
}

#[test]
fn unwrap_is_fine_outside_shipping_code() {
    let src = "fn main() { std::env::args().nth(1).unwrap(); }";
    check_at("tests/it.rs", src);
    check_at("examples/demo.rs", src);
    check_at("build.rs", src);
    check_at("crates/cargo-test-support/src/lib.rs", src);
    check_at("benchmarks/src/routing.rs", src);
    check_at("xtask/src/main.rs", src);
    check_at("crates/xtask-bump-check/src/main.rs", src);
    check_at("src/test_utils.rs", src);
}

#[test]
fn unwrap_inside_format_and_assert_macros() {
    check(
        r#"
        fn a(x: Option<u8>) { println!("{}", x.unwrap()); } //~ unwrap
        fn b(x: Option<u8>) { assert!(x.unwrap() > 0); } //~ unwrap
        "#,
    );
}

#[test]
fn todo_and_dbg() {
    check(
        r#"
        fn a() -> u8 { todo!() } //~ todo
        fn b() -> u8 { unimplemented!("not supported on this backend") }
        fn c(x: u8) -> u8 { dbg!(x) } //~ dbg-macro
        fn d() { let v = vec![dbg!(1), 2]; } //~ dbg-macro
        fn e() -> u8 { unreachable!() }
        #[cfg(test)]
        mod tests { fn f() { todo!(); dbg!(1); } }
        "#,
    );
}

#[test]
fn panic_in_drop() {
    check(
        r#"
        struct A(Option<std::fs::File>);
        impl Drop for A {
            fn drop(&mut self) {
                self.0.take().unwrap(); //~ panic-in-drop
                assert!(true);
                if !std::thread::panicking() {
                    panic!("dropped out of order");
                }
                if std::thread::panicking() {
                    eprintln!("closing failed");
                } else {
                    panic!("closing failed");
                }
                panic!("always"); //~ panic-in-drop
                let _ = self.0.take();
            }
        }
        impl A {
            fn drop(&mut self) { self.0.take().expect("not a destructor"); }
        }
        "#,
    );
}

#[test]
fn static_mut_and_uninit() {
    check(
        r#"
        static mut COUNTER: u32 = 0; //~ static-mut
        static OK: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        fn a() {
            // SAFETY: none, this is the bad example.
            let x: u32 = unsafe { std::mem::uninitialized() }; //~ uninit-memory
            // SAFETY: as above.
            let y: u32 = unsafe { std::mem::MaybeUninit::uninit().assume_init() }; //~ uninit-memory
            // SAFETY: an array of MaybeUninit needs no initialisation.
            let z: [std::mem::MaybeUninit<u8>; 4] = unsafe { std::mem::MaybeUninit::<[std::mem::MaybeUninit<u8>; 4]>::uninit().assume_init() };
            fn inner() { static mut LOCAL: u8 = 0; } //~ static-mut
        }
        "#,
    );
}

#[test]
fn unsafe_needs_a_safety_comment() {
    check(
        r#"
        fn a(p: *const u8) -> u8 {
            unsafe { *p } //~ unsafe-without-comment
        }
        fn b(p: *const u8) -> u8 {
            // SAFETY: the caller passes a pointer from `Box::into_raw`.
            unsafe { *p }
        }
        fn c(p: *const u8) -> u8 {
            let v = unsafe { *p }; // SAFETY: p is valid for reads.
            v
        }
        fn d(p: *const u8) -> u8 {
            // Safety: p came from a live reference.
            let v =
                unsafe { *p };
            v
        }
        fn e(p: *const u8) -> u8 {
            unsafe {
                // SAFETY: p is aligned and points at a live u8.
                *p
            }
        }
        struct S(*mut u8);
        unsafe impl Send for S {} //~ unsafe-without-comment
        // SAFETY: S owns its pointer exclusively.
        unsafe impl Sync for S {}
        "#,
    );
}

#[test]
fn lock_held_across_await() {
    check(
        r#"
        async fn a(m: &std::sync::Mutex<u8>) {
            let g = m.lock().unwrap();
            other().await; //~ lock-held-across-await
            drop(g);
        }
        async fn b(m: &std::sync::Mutex<u8>) {
            let g = m.lock().unwrap();
            let v = *g;
            drop(g);
            other().await;
        }
        async fn c(m: &tokio::sync::Mutex<u8>) {
            let g = m.lock().await;
            other().await;
        }
        async fn d(m: &std::sync::Mutex<u8>) {
            let v = *m.lock().unwrap();
            other().await;
        }
        async fn e(m: &std::sync::RwLock<u8>) {
            if true {
                let w = m.write().expect("not poisoned");
                async { 1 }.await; //~ lock-held-across-await
            }
        }
        fn f(m: std::sync::Arc<std::sync::Mutex<u8>>) {
            let fut = async move {
                let g = m.lock().unwrap();
                other().await; //~ lock-held-across-await
            };
        }
        async fn g(m: &std::sync::Mutex<u8>) {
            let g = m.lock().unwrap();
            let h = move || { let g = 1; };
        }
        async fn other() {}
        "#,
    );
}

#[test]
fn blocking_sleep_in_async() {
    check(
        r#"
        use std::time::Duration;
        async fn a() { std::thread::sleep(Duration::from_millis(5)); } //~ blocking-sleep-in-async
        fn b() { std::thread::sleep(Duration::from_millis(5)); }
        async fn c() { tokio::time::sleep(Duration::from_millis(5)).await; }
        async fn d() {
            tokio::task::spawn_blocking(|| std::thread::sleep(Duration::from_millis(5))).await;
        }
        fn e() -> impl std::future::Future<Output = ()> {
            async { std::thread::sleep(Duration::from_millis(5)) } //~ blocking-sleep-in-async
        }
        "#,
    );
}

#[test]
fn ignored_io_errors() {
    check(
        r#"
        use std::io::Write;
        fn callback(f: &mut std::fs::File, b: &[u8]) {
            let _ = f.write_all(b);
        }
        fn a(f: &mut std::fs::File, b: &[u8]) -> std::io::Result<()> {
            let _ = f.write_all(b); //~ ignored-io-error
            f.flush().ok(); //~ ignored-io-error
            let _ = std::fs::rename("a", "b"); //~ ignored-io-error
            let _ = std::fs::remove_file("tmp");
            let _ = tx_send();
            let _ = std::io::stdout().flush();
            let _ = stderr.write_all(b);
            f.write_all(b).expect("disk full is fatal here");
            Ok(())
        }
        fn tx_send() -> Result<(), ()> { Ok(()) }
        "#,
    );
}

#[test]
fn process_exit_only_in_libraries() {
    check("fn fail() -> ! { std::process::exit(1) } //~ process-exit-in-library");
    check_at("src/main.rs", "fn main() { std::process::exit(1) }");
    check_at("src/bin/tool.rs", "fn main() { std::process::exit(1) }");
}

#[test]
fn float_eq_inexact() {
    check(
        r#"
        fn a(x: f64) -> bool { x == 0.1 } //~ float-eq-inexact
        fn b(x: f64) -> bool { 0.3 != x } //~ float-eq-inexact
        fn c(x: f32) -> bool { x == -0.2f32 } //~ float-eq-inexact
        fn d(x: f64) -> bool { x == 0.0 || x == 0.5 || x == 1.0 }
        fn e(x: f64) -> bool { x < 0.1 }
        "#,
    );
}

#[test]
fn len_truncating_cast() {
    check(
        r#"
        fn a(v: &[u8]) -> u32 { v.len() as u32 } //~ len-truncating-cast
        fn b(v: &[u8]) -> u8 { (v.len()) as u8 } //~ len-truncating-cast
        fn c(v: &[u8]) -> u64 { v.len() as u64 }
        fn d(v: &[u8]) -> usize { v.len() as usize }
        fn e(x: u64) -> u32 { x as u32 }
        #[cfg(test)]
        mod tests { fn f(v: &[u8]) -> u32 { v.len() as u32 } }
        "#,
    );
}

#[test]
fn deny_warnings() {
    check_at("examples/demo.rs", "#![deny(warnings)]\nfn main() {}");
    check("#![deny(warnings)] //~ deny-warnings\nfn a() {}");
    check("#![forbid(unsafe_code, warnings)] //~ deny-warnings\nfn a() {}");
    check("#![deny(missing_docs)]\nfn a() {}");
}

#[test]
fn suppression_comment() {
    check(
        r#"
        fn a(x: Option<u8>) -> u8 {
            // anti-slop: allow(unwrap) x is Some by construction
            x.unwrap()
        }
        fn b(x: Option<u8>) -> u8 { x.unwrap() } // anti-slop: allow(todo) wrong rule //~ unwrap
        "#,
    );
}

#[test]
fn function_names_are_qualified() {
    let src = "struct S; impl S { fn go(&self, x: Option<u8>) { x.unwrap(); } }";
    let r = analyze_source(Path::new("src/lib.rs"), src).expect("parses");
    assert_eq!(r.findings[0].function.as_deref(), Some("S::go"));
    assert_eq!(r.functions[0].name, "S::go");
}

#[test]
fn unparseable_files_are_errors_not_clean() {
    assert!(analyze_source(Path::new("src/lib.rs"), "fn (").is_err());
}

#[test]
fn cargo_script_frontmatter() {
    check(
        "#!/usr/bin/env cargo\n---\n[package]\nedition = \"2024\"\n---\nfn main() { dbg!(1); } //~ dbg-macro\nfn f() { todo!() } //~ todo",
    );
}

#[test]
fn panic_on_input() {
    check(
        r#"
        fn a(s: &str) -> u16 { s.parse().unwrap() } //~ panic-on-input
        fn b(s: &str) -> u16 { s.parse().expect("validated by the CLI parser") }
        fn c() -> std::fs::File { std::fs::File::open(path()).unwrap() } //~ panic-on-input
        fn d(p: &std::path::Path) -> &str { p.to_str().unwrap() } //~ panic-on-input
        fn e(k: &str) -> String { std::env::var(k).unwrap() } //~ panic-on-input
        fn f(b: Vec<u8>) -> String { String::from_utf8(b).unwrap() } //~ panic-on-input
        fn g() -> std::process::Output { std::process::Command::new("git").arg("log").output().unwrap() } //~ panic-on-input
        fn h() -> std::net::IpAddr { "127.0.0.1".parse().unwrap() }
        fn i(x: Option<u8>) -> u8 { x.expect("checked by the caller") }
        fn j(s: &mut String) { use std::fmt::Write; write!(s, "{}", 1).unwrap(); }
        fn k(o: &Opener) { o.open().unwrap(); } //~ unwrap
        fn l(m: &Map) -> u8 { m.metadata(3).unwrap() } //~ unwrap
        fn n(p: &std::path::Path) -> u64 { p.metadata().unwrap().len() } //~ panic-on-input
        "#,
    );
}

fn temp_crate(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("anti-slop-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (path, body) in files {
        let p = dir.join(path);
        std::fs::create_dir_all(p.parent().expect("has a parent")).expect("temp dir");
        std::fs::write(p, body).expect("write");
    }
    dir
}

fn findings_in(dir: &std::path::Path) -> Vec<String> {
    let (reports, errors) = anti_slop::analyze_paths(&[dir.to_path_buf()]);
    assert!(errors.is_empty(), "{errors:?}");
    reports
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
        .collect()
}

#[test]
fn files_under_a_cfg_test_module_are_tests() {
    let dir = temp_crate(
        "layout",
        &[
            ("Cargo.toml", "[package]\nname = \"x\"\n"),
            (
                "src/lib.rs",
                "#[cfg(test)]\nmod unit;\nfn a(x: Option<u8>) -> u8 { x.unwrap() }\n",
            ),
            (
                "src/unit/mod.rs",
                "mod mocks;\nfn b(x: Option<u8>) -> u8 { x.unwrap() }\n",
            ),
            ("src/unit/mocks.rs", "fn c() -> u8 { todo!() }\n"),
        ],
    );
    assert_eq!(findings_in(&dir), ["lib.rs:unwrap"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn binary_crate_modules_may_exit() {
    let bin = temp_crate(
        "bin",
        &[
            ("Cargo.toml", "[package]\nname = \"x\"\n"),
            ("src/main.rs", "mod cli;\nfn main() {}\n"),
            (
                "src/cli.rs",
                "pub fn fail() -> ! { std::process::exit(1) }\n",
            ),
        ],
    );
    assert!(findings_in(&bin).is_empty());
    let lib = temp_crate(
        "lib",
        &[
            ("Cargo.toml", "[package]\nname = \"x\"\n"),
            (
                "src/lib.rs",
                "pub fn fail() -> ! { std::process::exit(1) }\n",
            ),
        ],
    );
    assert_eq!(findings_in(&lib), ["lib.rs:process-exit-in-library"]);
    let _ = std::fs::remove_dir_all(&bin);
    let _ = std::fs::remove_dir_all(&lib);
}

#[test]
fn blocking_sleep_in_async_tests_is_deliberate() {
    check(
        r#"
        #[tokio::test]
        async fn waits_for_expiry() { std::thread::sleep(std::time::Duration::from_secs(1)); }
        "#,
    );
}

#[test]
fn a_checkout_inside_a_tests_directory_is_still_shipping_code() {
    let dir = temp_crate(
        "tests/checkout",
        &[
            ("Cargo.toml", "[package]\nname = \"x\"\n"),
            ("src/lib.rs", "fn a() -> u8 { todo!() }\n"),
        ],
    );
    assert_eq!(findings_in(&dir), ["lib.rs:todo"]);
    let _ = std::fs::remove_dir_all(&dir);
}
