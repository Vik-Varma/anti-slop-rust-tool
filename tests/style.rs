//! Each style rule against code it must flag and code it must leave alone. Markers as in
//! tests/rules.rs: `//~ rule` on a line expects that finding there, once per mention. Only the
//! rule under test is compared, since one fixture can trip several style rules at once.

use std::path::Path;

use anti_slop::{FileKind, Options, Severity, analyze_source_in};

fn check_at(rule: &str, kind: FileKind, path: &str, src: &str) {
    let unmarked: Vec<&str> = src
        .lines()
        .map(|l| l.split("//~").next().unwrap_or(l))
        .collect();
    let options = Options {
        style: true,
        ..Options::default()
    };
    let report = analyze_source_in(Path::new(path), &unmarked.join("\n"), kind, options)
        .expect("fixture parses");
    let mut expected: Vec<usize> = Vec::new();
    for (i, line) in src.lines().enumerate() {
        if let Some((_, marks)) = line.split_once("//~") {
            expected.extend(
                marks
                    .split_whitespace()
                    .filter(|m| *m == rule)
                    .map(|_| i + 1),
            );
        }
    }
    let found: Vec<_> = report.findings.iter().filter(|f| f.rule == rule).collect();
    let mut got: Vec<usize> = found.iter().map(|f| f.line).collect();
    expected.sort();
    got.sort();
    assert_eq!(got, expected, "{rule}\nfindings: {found:#?}");
    assert!(found.iter().all(|f| f.severity == Severity::Style));
}

fn lib(rule: &str, src: &str) {
    check_at(rule, FileKind::Library, "src/lib.rs", src);
}

fn bin(rule: &str, src: &str) {
    check_at(rule, FileKind::Binary, "src/main.rs", src);
}

#[test]
fn off_by_default() {
    let src = "use std::collections::*;\npub struct B;\nfn f(a: &Vec<u8>) {}\n";
    let r = anti_slop::analyze_source(Path::new("src/lib.rs"), src).expect("parses");
    assert!(
        r.findings.iter().all(|f| f.severity != Severity::Style),
        "{:?}",
        r.findings
    );
}

#[test]
fn glob_import() {
    lib(
        "glob-import",
        r#"
use std::collections::*; //~ glob-import
use rayon::prelude::*;
pub use leaf::*;
mod leaf;
fn f() { use Color::*; use bar::*; } //~ glob-import
mod tests { use super::*; }
mod other { use crate::x::*; } //~ glob-import
"#,
    );
}

#[test]
fn use_mixes_modules() {
    lib(
        "use-mixes-modules",
        r#"
use std::{collections::HashMap, io}; //~ use-mixes-modules
use std::io::{Read, Write};
use a::{b::{C, D}}; //~ use-mixes-modules
"#,
    );
}

#[test]
fn unqualified_function_call() {
    lib(
        "unqualified-function-call",
        r#"
use std::mem::swap;
use std::mem;
use crate::local;
fn local() {}
fn f(a: &mut u8, b: &mut u8) {
    swap(a, b); //~ unqualified-function-call
    mem::swap(a, b);
    local();
}
"#,
    );
}

#[test]
fn qualified_type_name() {
    lib(
        "qualified-type-name",
        r#"
use std::fmt;
fn f(r: regex::Regex, e: io::Error, s: fmt::Result) -> Option<u8> { None } //~ qualified-type-name
impl fmt::Display for X {} //~ qualified-type-name
fn g() { let m = std::collections::HashMap::<u8, u8>::new(); } //~ qualified-type-name
fn h(o: std::cmp::Ordering) -> bool { o == Ordering::Less } //~ qualified-type-name
"#,
    );
}

#[test]
fn wrapper_reference_param() {
    lib(
        "wrapper-reference-param",
        "fn f(a: &Vec<u8>, b: &String, c: &mut Vec<u8>, d: &[u8], e: &Box<u8>, g: &Option<u8>) {} //~ wrapper-reference-param wrapper-reference-param wrapper-reference-param wrapper-reference-param\n",
    );
}

#[test]
fn bool_parameter() {
    lib(
        "bool-parameter",
        r#"
pub fn f(verbose: bool) {} //~ bool-parameter
pub fn g(verbose: bool /* print each step */) {}
fn h(verbose: bool) {}
pub struct S;
impl S { pub fn m(&self, dry_run: bool) {} } //~ bool-parameter
"#,
    );
}

#[test]
fn into_parameter_in_application() {
    bin(
        "into-parameter-in-application",
        r#"
fn f(name: impl Into<String>) {} //~ into-parameter-in-application
fn g<T: Into<String>>(name: T) {} //~ into-parameter-in-application
fn h<T>(name: T) where T: Into<String> {} //~ into-parameter-in-application
fn k(name: String) {}
fn main() {}
"#,
    );
    lib(
        "into-parameter-in-application",
        "pub fn f(name: impl Into<String>) {}\n",
    );
}

#[test]
fn rc_arc() {
    lib(
        "rc-arc",
        r#"
use std::sync::Arc;
pub struct S { a: Arc<u8>, b: Vec<Rc<u8>> } //~ rc-arc rc-arc
fn f(x: Arc<u8>) -> Rc<u8> { todo!() } //~ rc-arc rc-arc
fn g() { let x = Arc::new(1); }
"#,
    );
}

#[test]
fn tuple_fields() {
    lib(
        "tuple-fields",
        r#"
pub struct Pair(u8, u8); //~ tuple-fields
pub struct Meters(u32);
pub struct S { pub at: (u8, u8) } //~ tuple-fields
pub fn f() -> (u8, u8) { (0, 0) } //~ tuple-fields
fn g() -> (u8, u8) { let t = (1, 2); t }
"#,
    );
}

#[test]
fn lifetime_name() {
    lib(
        "lifetime-name",
        r#"
pub struct S<'a> { x: &'a str } //~ lifetime-name
pub struct T<'src> { x: &'src str }
fn f<'a, 'b>(x: &'a str, y: &'b str) {}
fn g<'a, 'b, 'c>(x: &'a str, y: &'b str, z: &'c str) {} //~ lifetime-name
impl<'a> S<'a> { fn h(&self) -> &'_ str { self.x } }
"#,
    );
}

#[test]
fn must_use_missing() {
    lib(
        "must-use-missing",
        r#"
pub struct S { n: usize }
impl S {
    pub fn count(&self) -> usize { self.n } //~ must-use-missing
    #[must_use]
    pub fn len(&self) -> usize { self.n }
    pub fn is_empty(&self) -> bool { self.n == 0 } //~ must-use-missing
    pub fn insert(&self, n: usize) -> bool { true }
    pub fn bump(&mut self) -> usize { 0 }
    pub fn load(&self) -> Result<u8, u8> { Ok(0) }
    fn private(&self) -> bool { true }
}
pub struct ParseError; //~ must-use-missing
#[must_use]
pub struct IoError;
"#,
    );
}

#[test]
fn non_exhaustive_missing() {
    lib(
        "non-exhaustive-missing",
        r#"
pub enum Three { A, B, C } //~ non-exhaustive-missing
pub enum Two { On, Off }
#[non_exhaustive]
pub enum Four { A, B, C, D }
pub struct Open { pub a: u8, pub b: u8 } //~ non-exhaustive-missing
pub struct Closed { pub a: u8, b: u8 }
pub(crate) enum Internal { A, B, C }
"#,
    );
}

#[test]
fn len_without_is_empty() {
    lib(
        "len-without-is-empty",
        r#"
pub struct S;
impl S { pub fn len(&self) -> usize { 0 } } //~ len-without-is-empty
pub struct T;
impl T { pub fn len(&self) -> usize { 0 } pub fn is_empty(&self) -> bool { true } }
"#,
    );
}

#[test]
fn getter_prefix() {
    lib(
        "getter-prefix",
        r#"
pub struct S { name: String }
impl S {
    pub fn get_name(&self) -> &str { &self.name } //~ getter-prefix
    pub fn name(&self) -> &str { &self.name }
    pub fn get(&self) -> &str { &self.name }
    pub fn get_or_default(&self) -> &str { &self.name }
    pub fn get_name_len(&self, x: u8) -> usize { 0 }
}
"#,
    );
}

#[test]
fn constructor_name() {
    lib(
        "constructor-name",
        r#"
pub struct S;
impl S {
    pub fn new() -> Result<S, u8> { Ok(S) } //~ constructor-name
    pub fn create() -> Self { S }
    pub fn make() -> Self { S } //~ constructor-name
    pub fn new_with_cap(n: usize) -> Self { S }
    pub fn try_new() -> Option<Self> { Some(S) }
    pub fn from_bytes(b: &[u8]) -> Self { S }
    pub fn helper() -> u8 { 0 }
}
"#,
    );
}

#[test]
fn from_impl_without_constructor() {
    lib(
        "from-impl-without-constructor",
        r#"
pub struct A(u8);
impl From<u8> for A { fn from(x: u8) -> A { A(x) } } //~ from-impl-without-constructor
pub struct B(u8);
impl B { pub fn new(x: u8) -> B { B(x) } }
impl From<u8> for B { fn from(x: u8) -> B { B(x) } }
impl From<u8> for Vec<u8> { fn from(x: u8) -> Vec<u8> { vec![x] } }
"#,
    );
}

#[test]
fn std_trait_as_method() {
    lib(
        "std-trait-as-method",
        r#"
pub struct S;
impl S {
    pub fn from_str(s: &str) -> S { S } //~ std-trait-as-method
    pub fn eq(&self, other: &S) -> bool { true } //~ std-trait-as-method
    pub fn next(&mut self) -> Option<u8> { None } //~ std-trait-as-method
    pub fn to_string(&self) -> String { String::new() }
    fn hash(&self, h: &mut u8) {}
}
impl std::fmt::Display for S {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) }
}
"#,
    );
}

#[test]
fn new_without_default() {
    lib(
        "new-without-default",
        r#"
pub struct S { n: u8 }
impl S { pub fn new() -> Self { S { n: 0 } } } //~ new-without-default
#[derive(Default)]
pub struct T { n: u8 }
impl T { pub fn new() -> Self { T { n: 0 } } }
pub struct U { n: u8 }
impl U { pub fn new() -> Self { U { n: 0 } } }
impl Default for U { fn default() -> Self { U::new() } }
pub struct V;
impl V { pub fn new(n: u8) -> V { V } }
"#,
    );
}

#[test]
fn options_struct_default() {
    lib(
        "options-struct-default",
        r#"
pub struct RunOptions { pub verbose: bool, pub jobs: u8 } //~ options-struct-default
#[derive(Default)]
pub struct BuildConfig { pub verbose: bool, pub jobs: u8 }
pub struct Point { pub x: u8, pub y: u8 }
pub struct HalfOptions { pub a: u8, b: u8 }
"#,
    );
}

#[test]
fn trait_as_organization() {
    lib(
        "trait-as-organization",
        r#"
trait Helper { fn help(&self); } //~ trait-as-organization
struct S;
impl Helper for S { fn help(&self) {} }
trait Shared { fn go(&self); }
struct A; struct B;
impl Shared for A { fn go(&self) {} }
impl Shared for B { fn go(&self) {} }
trait Bound { fn b(&self); }
struct C;
impl Bound for C { fn b(&self) {} }
fn use_bound<T: Bound>(t: T) {}
pub trait Public { fn p(&self); }
struct D;
impl Public for D { fn p(&self) {} }
trait ReadExt { fn r(&self); }
impl ReadExt for D { fn r(&self) {} }
"#,
    );
}

#[test]
fn ext_trait_in_bounds() {
    lib(
        "ext-trait-in-bounds",
        r#"
pub trait ReadExt { fn r(&self); }
fn f<T: ReadExt>(t: T) {} //~ ext-trait-in-bounds
fn g(t: &dyn ReadExt) {} //~ ext-trait-in-bounds
fn h(t: impl Iterator<Item = u8>) {}
"#,
    );
}

#[test]
fn macro_definition() {
    lib(
        "macro-definition",
        r#"
macro_rules! sq { ($x:expr) => { $x * $x }; } //~ macro-definition
#[proc_macro]
pub fn derive_thing(input: TokenStream) -> TokenStream { input } //~ macro-definition
fn f() -> u8 { sq!(2) }
"#,
    );
}

#[test]
fn code_before_main() {
    lib(
        "code-before-main",
        r#"
#[ctor::ctor] //~ code-before-main
fn init() {}
#[used]
#[link_section = ".init_array"] //~ code-before-main
static INIT: fn() = init;
#[link_section = ".text.hot"]
static OTHER: u8 = 0;
"#,
    );
}

#[test]
fn unstable_feature() {
    lib(
        "unstable-feature",
        "#![feature(let_chains)] //~ unstable-feature\n#![cfg_attr(nightly, feature(doc_cfg))] //~ unstable-feature\n#![allow(dead_code)]\n",
    );
}

#[test]
fn rustfmt_skip() {
    lib(
        "rustfmt-skip",
        r#"
#[rustfmt::skip] //~ rustfmt-skip
const A: [u8; 4] = [1, 2, 3, 4];
// Manually aligned table.
#[rustfmt::skip]
const B: [u8; 4] = [1, 2, 3, 4];
#[rustfmt::skip] // generated by build.rs
const C: [u8; 4] = [1, 2, 3, 4];
"#,
    );
}

#[test]
fn api_in_non_leaf_module() {
    let body: Vec<String> = (0..60).map(|i| format!("    let x{i} = {i};")).collect();
    let big = format!(
        "mod leaf;\npub use leaf::Thing;\nfn glue() {{ //~ api-in-non-leaf-module\n{}\n}}\n",
        body.join("\n")
    );
    lib("api-in-non-leaf-module", &big);
    lib(
        "api-in-non-leaf-module",
        "mod leaf;\npub use leaf::Thing;\nfn small() {}\n#[cfg(test)]\nmod tests {}\n",
    );
    let leaf = format!("fn alone() {{\n{}\n}}\n", body.join("\n"));
    lib("api-in-non-leaf-module", &leaf);
}

#[test]
fn file_named_after_item() {
    check_at(
        "file-named-after-item",
        FileKind::Library,
        "src/widget.rs",
        "pub struct Gadget; //~ file-named-after-item\n",
    );
    check_at(
        "file-named-after-item",
        FileKind::Library,
        "src/gadget.rs",
        "pub struct Gadget;\n",
    );
    check_at(
        "file-named-after-item",
        FileKind::Library,
        "src/util.rs",
        "pub struct Gadget;\npub fn a() {}\npub fn b() {}\n",
    );
}

#[test]
fn check_then_unwrap() {
    lib(
        "check-then-unwrap",
        r#"
fn f(x: Option<u8>, r: Result<u8, u8>) {
    if x.is_some() { let v = x.unwrap(); } //~ check-then-unwrap
    if x.is_none() { } else { let v = x.unwrap(); } //~ check-then-unwrap
    if r.is_ok() && x.is_some() { let v = r.unwrap(); } //~ check-then-unwrap
    if !x.is_none() { let v = x.unwrap(); } //~ check-then-unwrap
    if x.is_none() { let v = x.unwrap(); }
    if let Some(v) = x { }
}
"#,
    );
}

#[test]
fn assert_without_message() {
    lib(
        "assert-without-message",
        r#"
fn f(n: u8) {
    assert!(n > 0); //~ assert-without-message
    assert!(n > 0, "n is a count");
    assert_eq!(n, 1); //~ assert-without-message
    debug_assert_ne!(n, 2, "two is reserved");
}
#[test]
fn t() { assert!(true); }
"#,
    );
}

#[test]
fn combinator_chain() {
    lib(
        "combinator-chain",
        r#"
fn f(v: &[u8]) {
    v.iter().for_each(|x| {}); //~ combinator-chain
    for x in v.iter().filter(|x| **x > 0).map(|x| x + 1) {} //~ combinator-chain
    for (i, x) in v.iter().enumerate() {}
    let n = v.iter().map(|x| x + 1).filter(|x| *x > 1).map(|x| x * 2).take(3).skip(1).count(); //~ combinator-chain
    let m = v.iter().map(|x| x + 1).filter(|x| *x > 1).count();
}
"#,
    );
}

#[test]
fn string_concat_plus() {
    lib(
        "string-concat-plus",
        r#"
fn f(a: &str, b: &str, n: u8) -> String {
    let s = a.to_string() + b + "!"; //~ string-concat-plus
    let t = a.to_string() + b;
    let u = n + 1 + 2;
    let w = "Hello NAME".replace("NAME", a); //~ string-concat-plus
    s + &t + "." //~ string-concat-plus
}
"#,
    );
}

#[test]
fn inferred_conversion_type() {
    lib(
        "inferred-conversion-type",
        r#"
fn f(s: &str) -> Result<(), std::num::ParseIntError> {
    let n = s.parse()?; //~ inferred-conversion-type
    let m: u32 = s.parse()?;
    let k = s.parse::<u64>()?;
    let o = s.to_string().into(); //~ inferred-conversion-type
    let v = s.chars().collect::<Vec<char>>(); //~ inferred-conversion-type
    let w = s.chars().collect::<Vec<_>>();
    let z: Vec<char> = s.chars().collect();
    Ok(())
}
"#,
    );
}

#[test]
fn poisoned_mutex_recovery() {
    lib(
        "poisoned-mutex-recovery",
        r#"
use std::sync::{Mutex, PoisonError};
fn f(m: &Mutex<u8>) {
    let a = m.lock().unwrap_or_else(|e| e.into_inner()); //~ poisoned-mutex-recovery
    let b = m.lock().unwrap_or_else(PoisonError::into_inner); //~ poisoned-mutex-recovery
    let c = match m.lock() { Ok(g) => g, Err(p) => p.into_inner() }; //~ poisoned-mutex-recovery
    let d = m.lock().unwrap();
    let e = m.lock().unwrap_or_else(|e| panic!("poisoned: {e}"));
}
"#,
    );
}

#[test]
fn catch_unwind() {
    lib(
        "catch-unwind",
        "fn f() { let r = std::panic::catch_unwind(|| ()); } //~ catch-unwind\n",
    );
}

#[test]
fn missing_docs() {
    lib(
        "missing-docs",
        r#"
//! Crate docs.
/// Documented.
pub struct A;
pub struct B; //~ missing-docs
pub(crate) struct C;
#[doc(hidden)]
pub struct D;
pub enum E { X, Y } //~ missing-docs
impl A {
    pub fn m(&self) {} //~ missing-docs
    /// Does the thing carefully.
    pub fn n(&self) {}
    fn p(&self) {}
}
pub trait T { //~ missing-docs
    fn q(&self); //~ missing-docs
}
#[cfg(test)]
mod tests { pub fn x() {} }
pub mod inner { pub fn y() {} } //~ missing-docs missing-docs
"#,
    );
    bin("missing-docs", "pub struct B;\n");
}

#[test]
fn doc_comment_style() {
    lib(
        "doc-comment-style",
        r#"
/// Removes an element.
pub fn remove_element() {} //~ doc-comment-style
/// returns the number of live entries.
pub fn count() -> usize { 0 } //~ doc-comment-style
/// Counts the entries that survived the last compaction
pub fn survivors() -> usize { 0 } //~ doc-comment-style
/// Counts the entries that survived the last compaction.
pub fn survived() -> usize { 0 }
/// `Config` for the run.
pub struct Config;
"#,
    );
}

#[test]
fn panic_undocumented() {
    lib(
        "panic-undocumented",
        r#"
/// Divides.
pub fn div(a: u8, b: u8) -> u8 { assert!(b != 0, "divisor"); a / b } //~ panic-undocumented
/// Divides.
///
/// # Panics
/// If `b` is zero.
pub fn div2(a: u8, b: u8) -> u8 { assert!(b != 0, "divisor"); a / b }
/// Parses.
pub fn parse(s: &str) -> u8 { s.parse().unwrap() } //~ panic-undocumented
/// Internal.
pub fn internal(n: u8) -> u8 { let x: Option<u8> = None; x.unwrap() }
/// Always.
pub fn boom(n: u8) { panic!("no") } //~ panic-undocumented
fn private(n: u8) { assert!(n > 0, "n"); }
"#,
    );
}

#[test]
fn safety_undocumented() {
    lib(
        "safety-undocumented",
        r#"
/// Reads.
pub unsafe fn read(p: *const u8) -> u8 { *p } //~ safety-undocumented
/// Reads.
///
/// # Safety
/// `p` must be valid.
pub unsafe fn read2(p: *const u8) -> u8 { *p }
/// Marker.
pub unsafe trait Zeroable {} //~ safety-undocumented
unsafe fn private(p: *const u8) -> u8 { *p }
"#,
    );
}

#[test]
fn safety_comment_prefix() {
    lib(
        "safety-comment-prefix",
        r#"
fn f(p: *const u8) -> u8 {
    // The pointer comes from a live Vec.
    let a = unsafe { *p }; //~ safety-comment-prefix
    // SAFETY: the pointer comes from a live Vec.
    let b = unsafe { *p };
    let c = unsafe { *p };
    a + b + c
}
"#,
    );
}
