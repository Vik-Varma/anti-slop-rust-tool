# anti-slop: correctness review

Tool: `/home/user/anti-slop-rust-tool` at commit 411ba4c (syn 2.0.119). I built it with `cargo build --release` and the existing `cargo test` passed (35 tests). I did not modify the repository. All outputs below come from the release binary, with `--min-severity note` unless the entry says otherwise. A "debug" run means the existing `target/debug/anti-slop`. Language claims were checked with rustc 1.99.1, and with nightly 1.101 where frontmatter was involved. The proof programs are in `scratch/review-code/proof/`, and every reproduction file is under `scratch/review-code/`.

I also ran the tool over all 35 corpus projects. There were no crashes. Lines and columns were exact for all 10,119 token-bearing findings. The parse errors (exit 2) were all deliberately invalid fixtures or templates, except the frontmatter cases in entry 12.

## 1. Const-context exemption is wrong in both directions (High)

**Where:**
- `src/analyzer.rs:147` (`in_const: sig.constness.is_some()`)
- `:281-296` (`visit_item_const` and `visit_item_static` set `in_const` for the whole initializer)
- `:310-318` (closures inherit `in_const`)
- `:261` (associated consts go through the default visitor)
- Consumers: `src/checks/panics.rs:102` and `:166`
- `src/context.rs` documents the premise: "where a panic is a compile error".

**What's wrong:** that premise doesn't hold.
- A `const fn` called at run time is ordinary code.
- A closure in a `static` initializer (`LazyLock::new`, `Lazy::new`) runs at run time.
- A closure coerced to a `const` fn pointer runs at run time.

All three are exempt from `unwrap`, `panic-on-input` and `todo`. Meanwhile, code that really is const-evaluated (impl/trait associated consts, inline `const {}`) is still flagged.

**Input A (missed):**
```rust
pub const fn first(x: Option<u8>) -> u8 { x.unwrap() }
pub const fn later() -> u8 { todo!() }
use std::sync::LazyLock;
static CONFIG: LazyLock<String> = LazyLock::new(|| std::fs::read_to_string(path()).unwrap());
static PORT: LazyLock<u16> = LazyLock::new(|| std::env::var(key()).unwrap().parse().unwrap());
const HOOK: fn(Option<u8>) -> u8 = |x| x.unwrap();
```
- **Observed:** `0 finding(s)`.
- **Expected:** `unwrap`, `todo`, and `panic-on-input` on lines 1-6.
- **Proof** (`proof/constfn.rs`): `first(None)` panics with "called `Option::unwrap()` on a `None` value", `later()` panics with "not yet implemented", and a LazyLock closure panics in `V::{closure#0}`.

**Input B (false positives):**
```rust
struct S;
impl S { const X: u8 = match Some(1) { Some(_) => Some(3).unwrap(), None => 0 }; }
trait T { const Y: u8 = Some(3).unwrap(); }
fn f() -> u8 { const { Some(3).unwrap() } }
```
- **Observed:** three `note[unwrap]` findings (lines 2-4).
- **Expected:** none.

**Fix:**
- Don't derive `in_const` from `sig.constness`.
- Reset `in_const` in `visit_expr_closure` and `visit_expr_async`.
- Set `in_const` in `visit_impl_item_const`, `visit_trait_item_const` and `visit_expr_const`.

## 2. Walker silently skips source modules named `target` or `vendor` (High)

**Where:** `src/walk.rs:11` and `:55`. `SKIP_DIRS` is matched against every directory name at any depth.

**Input:**
- `t1/Cargo.toml`
- `t1/src/lib.rs`: `pub mod target;`
- `t1/src/target/mod.rs`: `pub fn triple() -> &'static str { todo!() }`

**Observed:** `0 finding(s) in 1 file(s)`, exit 0.
**Expected:** one `todo` finding, 2 files, exit 1.

**Corpus cases:**
- `tauri-codegen/src/lib.rs:31` declares `pub mod vendor;`, and the 389 lines in `src/vendor` are never analysed (the tool reports 4 files; there are 6 `.rs` files on disk).
- cargo's `tests/testsuite/cargo_add/target/mod.rs` is skipped the same way.

**Fix:**
- Skip `target` only next to a `Cargo.toml`, or when it holds `CACHEDIR.TAG`.
- Skip `vendor` only at a workspace root, or when it holds `.cargo-checksum.json`.

## 3. `unsafe-without-comment` accepts unrelated text as a justification (High)

**Where:** `src/checks/unsafety.rs`.
- `:105`: any line starting with `*` counts as a comment.
- `:111`: the check steps over one code line ending in `{` (a fn signature) and accepts the doc comment above it.
- Attribute lines are walked over freely.
- `has_comment` (`:87`) matches `//` inside string literals.

**Input:**
```rust
/// Reads one byte.
pub fn read(p: *const u8) -> u8 {
    unsafe { *p }
}
/// Writes one byte.
#[inline]
pub fn write(p: *mut u8) {
    unsafe { *p = 1 }
}
pub fn f(p: *mut u8, q: &mut u8) {
    *q = 1;
    unsafe { *p = 2 }
}
extern "C" { fn open_url(u: *const u8); }
pub fn g() { unsafe { open_url("https://example.com\0".as_ptr()) } }
pub fn undocumented(p: *const u8) -> u8 {
    unsafe { *p }
}
```
- **Observed:** only `undocumented` is flagged.
- **Expected:** all five blocks.

**Corpus:**
- 135 `unsafe` blocks are hidden by their fn's doc comment; 76 of them are in safe fns whose docs never mention safety. Example: tokio `util/sharded_list.rs:90`.
- 3 are hidden by a `*x = ..;` line above. Example: tokio `sync/once_cell.rs:446`.

**Fix:**
- Treat only real comment lines as comments.
- Exclude doc comments, or stop at an item header.
- Allow the one-line step-over only for lines ending in `=` or `(`, never `{`.
- Find comments with a lexer, not `find("//")`.

## 4. `lock-held-across-await` misses shadowed guards and async closures (Medium-High)

**Where:** `src/checks/concurrency.rs:44` and `:117` (`rebinds`); `src/checks/mod.rs:21` and `analyzer.rs:310`.

**What's wrong:**
- Shadowing doesn't drop the guard. Proof (`proof/shadow.rs`, rustc 1.99): after `let g = m.lock().unwrap(); let g = *g;`, `try_lock()` still fails.
- Async closures are treated as async for `blocking-sleep-in-async`, but their bodies are never scanned for guards.

**Input:**
```rust
pub async fn a(m: &std::sync::Mutex<u8>) -> u8 {
    let g = m.lock().unwrap(); let g = *g;
    other().await;                              // missed
    g
}
pub fn b(m: std::sync::Arc<std::sync::Mutex<u8>>) {
    let f = async move || {
        std::thread::sleep(std::time::Duration::from_millis(5)); // flagged
        let g = m.lock().unwrap(); other().await; drop(g);       // missed
    };
    let h = move || async move {
        let g = m.lock().unwrap(); other().await; drop(g);       // flagged
    };
}
async fn other() {}
```
- **Observed:** the shadowed guard and the async closure's guard are not reported.
- **Expected:** both reported, like the `async move` block.

**Fix:**
- A shadowing `let` should end tracking only when its initializer moves the guard.
- Run `async_body` on the body of a closure that has `asyncness`.

## 5. `#[cfg(test)] mod` resolution misses common layouts (Medium-High)

**Where:** `src/layout.rs:79-125` (a line-based text scan) and `:24`, `:36-75`.

Each case is a minimal crate whose target file holds `fn t() -> u8 { todo!() }`.

| Case | `src/lib.rs` | Target file | Observed | Expected |
|---|---|---|---|---|
| a | `#[cfg(test)]` / `mod unit; // unit tests` | `src/unit.rs` | `todo` | none |
| b | `#[cfg(test)]` / `mod unit {` / `mod helpers;` / `}` | `src/unit/helpers.rs` | `todo` | none |
| c | `mod a {` / `#[cfg(test)]` / `mod unit;` / `}` | `src/a/unit.rs` | `todo` | none |
| d | `#[cfg(all(feature = "latest", unix))]` / `mod backend;` | `src/backend.rs` | **0 findings** | `todo` (shipping code) |
| e | `#[cfg(test)] #[path = "spec/mod.rs"] mod spec;`, where `spec/mod.rs` has `mod helpers;` | `src/spec/helpers.rs` | `todo` | none |
| f | `#[cfg(all(` / `test,` / `unix` / `))]` / `mod unit;` | `src/unit.rs` | `todo` | none |

Causes:
- a: `strip_suffix(';')` fails because of the trailing comment.
- b, c: the scan has no inline-module stack.
- d: the attribute is matched with `contains("test")`.
- e: `with_extension("")` turns the root into `spec/mod`, which doesn't cover `spec/helpers.rs`.
- f: a `#[` line with no `]` breaks the attribute scan.

**Fix:** parse each file with syn, walk `ItemMod`s with a module path stack, reuse `is_test_item`, and let a `mod.rs` reached through `#[path]` own its directory.

## 6. Binary-crate cache gives order-dependent results (Medium)

**Where:** `src/layout.rs:133-137`. The cache is keyed by package, but the cached value includes the asking file's `starts_with(pkg/src)`.

**Input:** package `b1` with:
- `src/main.rs`: `mod cli; fn main() {}`
- `src/cli.rs`: `pub fn fail() -> ! { std::process::exit(1) }`
- `codegen/gen.rs`, which sorts before `src/`

**Observed:** `b1/src/cli.rs:1:22: note[process-exit-in-library]`. After deleting `codegen/gen.rs`, the same crate gives 0 findings.

**Fix:** cache only "has `main.rs` and no `lib.rs`", and test the `src/` prefix per file.

## 7. Single-file roots are classified by every ancestor directory (Medium)

**Where:** `src/walk.rs:82-86`. `base = ""` for a file root.

**Input:** `…/review-code/test/proj/src/lib.rs` containing `pub fn f() -> u8 { todo!() }`.

| Invocation | Result |
|---|---|
| `anti-slop /…/test/proj` | 1 finding |
| `anti-slop /…/test/proj/src/lib.rs` | 0 findings (classified as Test) |
| `cd proj; anti-slop src/lib.rs` | 1 finding |

**Fix:** for a file root, judge the path relative to its package's parent.

## 8. `uninit-memory` (error severity) flags the documented sound idiom (Medium)

**Where:** `src/checks/unsafety.rs:47-59`. Only turbofish arguments are inspected, not the `let` type annotation.

**Input:**
```rust
use std::mem::MaybeUninit;
pub fn arr() -> [MaybeUninit<Vec<u32>>; 4] {
    // SAFETY: an array of MaybeUninit needs no initialisation.
    let data: [MaybeUninit<Vec<u32>>; 4] = unsafe { MaybeUninit::uninit().assume_init() };
    data
}
```
- **Observed:** `4:75: error[uninit-memory]`, exit 1 even at the default severity.
- **Expected:** no finding. `rules.rs` says the sound form isn't flagged, and this is the form the std docs used for years.

**Fix:** consult the `Pat::Type` annotation of the enclosing `let`, and don't report at error severity when the type is unknown.

## 9. `panic-on-input` is downgraded to `unwrap` when arguments are literals (Medium)

**Where:** `src/checks/panics.rs:74`.

**Input:**
```rust
pub fn home() -> String { std::env::var("HOME").unwrap() }
pub fn cfg() -> std::fs::File { std::fs::File::open("/etc/app.toml").unwrap() }
pub fn conn() -> std::net::TcpStream { std::net::TcpStream::connect("10.0.0.1:5432").unwrap() }
pub fn home2(k: &str) -> String { std::env::var(k).unwrap() }
```
- **Observed:** lines 1-3 are `unwrap`; only line 4 is `panic-on-input`.
- **Expected:** all four are `panic-on-input`. A missing variable, a missing file and a refused connection are exactly the harms the rule lists.

**Corpus:** 10 cases, for example meilisearch `tracing-trace/src/main.rs:100`.

**Fix:** apply the literal shortcut only to parsers, never to `fs`, `File`, `env` or socket calls.

## 10. `ignored-io-error` misses `_ = expr;` and awaited writes (Medium)

**Where:** `src/checks/errors.rs:37-45` and `:56`.

**Input:**
```rust
use std::io::Write;
pub fn a(f: &mut std::fs::File, b: &[u8]) -> std::io::Result<()> {
    _ = f.write_all(b);
    _ = f.sync_all();
    let _ = f.flush();
    Ok(())
}
pub async fn b(f: &mut tokio::fs::File, buf: &[u8]) -> std::io::Result<()> {
    let _ = f.write_all(buf).await;
    f.flush().await.ok();
    Ok(())
}
```
- **Observed:** 1 finding (line 5).
- **Expected:** 5 findings.

**Fix:** handle `Expr::Assign` with an `Expr::Infer` left side, and peel `Expr::Await` and `Expr::Try` before matching.

## 11. `deny-warnings` misses `#![deny(warnings, reason = "..")]` (Medium)

**Where:** `src/checks/hygiene.rs:21-25`. The list is parsed as `Punctuated<Path>`, the `reason = ".."` element makes the parse fail, and the attribute is skipped.

**Input:** `#![deny(warnings, reason = "keep the build clean")]` followed by `pub fn a() {}`.
- **Observed:** 0 findings.
- **Expected:** `deny-warnings`.
- **Proof** (`proof/denyreason.rs`, rustc 1.99): an unused variable becomes a hard error, with "note: keep the build clean".

**Fix:** parse as `Punctuated<syn::Meta, Token![,]>`.

## 12. Valid cargo-script frontmatter fails to parse (Medium)

**Where:** `src/analyzer.rs:64-88`. `:74` requires `---` on the first line; `:80` closes at the first line that is exactly `---`, whatever the opening fence length.

| Input | rustc nightly `-Zcrate-attr=feature(frontmatter)` | anti-slop |
|---|---|---|
| `----` / `package.description = """` / `---` / `"""` / `----` / `fn main(){..}` | compiles and runs | `4:3: could not parse: cannot parse string into token stream`, exit 2 |
| a blank line, then `---` / `[dependencies]` / `---` / `fn main(){..}` | compiles | `2:1: could not parse`, exit 2 |
| a UTF-8 BOM, then `---` / `[dependencies]` / `---` / `fn main(){..}` | compiles | `1:1: could not parse`, exit 2 |

cargo's own passing fixtures fail the same way:
- `script/rustc_fixtures/escape-hyphens-leading.rs`
- `script/rustc_fixtures/fence-whitespace-trailing-1.rs`
- `cargo_add/script_escape/{in,out}/cargo-test-fixture.rs` (9-dash fence)

**Fix:** skip a BOM and leading blank lines, count the opening dashes (3 or more), and close only on the same number of dashes plus optional whitespace.

## 13. Suppressions leak to the next line and match inside strings (Medium)

**Where:** `src/suppress.rs:17` (the first `//` on the line, even inside a string) and `:53` (`*l + 1 == line` applies even when the comment trails code).

**Input:**
```rust
pub fn f(x: Option<u8>, y: Option<u8>) -> u8 {
    let a = x.unwrap(); // anti-slop: allow(unwrap) x is Some by construction
    let b = y.unwrap();
    a + b
}
pub fn g(x: Option<u8>) -> u8 {
    let help = "write // anti-slop: allow(unwrap) to silence a finding";
    x.unwrap()
}
```
- **Observed:** 0 findings.
- **Expected:** the unwraps on lines 3 and 8 are reported.

A related case: in a multi-line chain (`// anti-slop: allow(unwrap)` / `x.as_ref()` / `.unwrap()`), the comment doesn't reach the unwrap, which is still reported at 4:10.

**Fix:**
- Apply a suppression to the next line only when the comment stands alone on its line.
- Find comments with a lexer.
- Optionally, match a suppression against the line range of the enclosing statement.

## 14. `panic-in-drop` flags correctly guarded destructors (Low-Medium)

**Where:** `src/analyzer.rs:320-344` and `checks/mod.rs:116`.

**Input:**
```rust
pub struct Guard { armed: bool }
impl Drop for Guard {
    fn drop(&mut self) {
        if std::thread::panicking() { return; }
        panic!("guard dropped while armed");
    }
}
pub struct G2 { armed: bool }
impl Drop for G2 {
    fn drop(&mut self) {
        if self.armed && !std::thread::panicking() {
            panic!("guard dropped while armed");
        }
    }
}
```
- **Observed:** two `panic-in-drop` findings.
- **Expected:** none. Neither destructor can panic while unwinding, which is the only harm the rule describes.

**Fix:** treat an early return on `panicking()` as guarding the rest of the block, and treat `!panicking()` inside an `&&` chain as a guard on the then-branch.

## 15. `#[cfg(test)]` on a trait isn't test scope (Low-Medium)

**Where:** `src/analyzer.rs:268-279`. Unlike `impl` (`:250`) and `mod` (`:230`), `visit_item_trait` pushes no test scope.

**Input:** `#[cfg(test)]` / `trait Fixture { fn make() -> u8 { Some(1).unwrap() } }`
- **Observed:** `2:43: note[unwrap]`.
- **Expected:** none.

**Fix:** wrap `visit_item_trait` in `with_scope(.. test: parent.test || is_test_item(&node.attrs))`.

## 16. `float-eq-inexact`: long literals count as exact, and overflow panics (Low-Medium)

**Where:** `src/checks/numeric.rs:52` (`unwrap_or(0)`, and 0 means "exact") and `:53`, `:59` (unchecked i32 arithmetic).

**Input:**
- `pub fn tenth(x: f64) -> bool { x == 0.100000000000000000000000000000000000000001 }`
- `pub fn f(x: f64) -> bool { x == 1.5e-2147483648 }`

**Observed:**
- Line 1 gives 0 findings. The doc comment promises `None` for "too large", but the code returns `Some(true)`, so literals with 39 or more significant digits are never flagged.
- The debug binary panics on line 2 with "numeric.rs:53:19: attempt to subtract with overflow".
- `10e2147483647` under `#[allow(overflowing_literals)]` panics at `:59:9` ("attempt to add with overflow").
- Release builds wrap silently.

The literal on line 2 is valid: rustc 1.99 compiles it and evaluates it to 0.0.

**Fix:** return `None` when the parse fails (or strip trailing zeros first), and use `checked_sub`/`checked_add`.

## 17. Metrics: `let`-`else` and macro arguments uncounted, recursion ignored (Low-Medium)

**Where:** `src/metrics.rs:141-218`.

**Input:**
```rust
pub fn let_else(x: Option<u8>) -> u8 { let Some(v) = x else { return 0 }; v }
pub fn if_let(x: Option<u8>) -> u8 { if let Some(v) = x { v } else { return 0 } }
pub fn fmt_branch(x: u8) -> String { format!("{}", if x > 3 && x < 9 { "mid" } else { "edge" }) }
pub fn plain_branch(x: u8) -> String { let s = if x > 3 && x < 9 { "mid" } else { "edge" }; format!("{}", s) }
pub fn fact(n: u64) -> u64 { n * fact(n - 1) }
```

Observed cyclomatic / cognitive:

| Function | Cyclomatic | Cognitive |
|---|---|---|
| `let_else` | 1 | 0 |
| `if_let` | 2 | 2 |
| `fmt_branch` | 1 | 0 |
| `plain_branch` | 3 | 3 |
| `fact` | 1 | 0 |

Expected:
- `let_else` should match `if_let`: it is the same two-way branch.
- `fmt_branch` should equal `plain_branch`: the analyzer itself treats `format!` arguments as code.
- `fact` should have cognitive 1, per Campbell's B1 recursion increment.

**Fix:**
- Add `visit_local` for `diverge`.
- Add `visit_macro` that walks `macro_args`, as `features.rs` does.
- Add +1 cognitive for a self-call.

## 18. Per-function finding counts in metrics double-count (Low-Medium)

**Where:** `src/main.rs:250`. A finding is attributed to every function whose line range contains it.

**Input:**
- `pub fn a(x: Option<u8>) -> u8 { x.unwrap() } pub fn b() -> u8 { 0 }`
- `impl S { fn outer(&self) -> u8 { fn helper(x: Option<u8>) -> u8 { x.unwrap() } .. } }`

**Observed:** `b` gets `unwrap=1`, and both `S::outer` and `S::helper` get 1.
**Expected:** 0 for `b`, and 1 for `helper` only.

**Corpus:** per-function sums exceed the number of findings — tokio 482 vs 468, wgpu 2899 vs 2872, typst 304 vs 301. This skews the defect study's inputs.

**Fix:** record a function index on each `Finding` at emit time, and count by that index.

## 19. CSV output uses JSON escaping (Low)

**Where:** `src/main.rs:222`, `:261-277`.

**Input:** a file named `we"ird\x.rs`.
- **Observed:** the row is `"./we\"ird\\x.rs",…`, and Python's `csv` reads the field as `./we\ird\\x.rs"`. JSON output is correct.
- **Expected:** `"./we""ird\x.rs"`.

**Fix:** quote CSV fields by doubling `"`, without backslash escapes.

## 20. CLI exit codes and format handling (Low)

**Where:** `src/main.rs:54`, `:103`, `:148`, `:261`, `:297`.

- `metrics --format yaml` prints CSV and exits 0 (any value other than `json` is accepted), while `check --format csv` is rejected.
- A closed stdout (`anti-slop corpus/tokio | head -1`) prints "Broken pipe (os error 32)" plus the whole usage text, and exits 2.
- `--help` exits 2.
- `metrics` silently ignores `--only`, `--skip` and `--min-severity`.

**Fix:** validate the format per subcommand, give I/O errors their own variant (quiet on `BrokenPipe`), return exit 0 for `--help`, and apply or reject the filters in `metrics`.

## 21. Function names: nested fns get the impl's type; `dyn Trait` is mangled (Low)

**Where:** `src/analyzer.rs:138-141` and `:222`.

- `fn helper` nested in `S::outer` is named `S::helper` in findings and metrics.
- diesel's `impl dyn BoxableConnection<DB>` gives `dynBoxableConnection<DB>::downcast_ref`.

**Fix:** push a function frame (or clear `self_types`) on entering a body, and keep spaces in non-path types.

## 22. Overlapping path arguments duplicate findings (Low)

**Where:** `src/walk.rs:72-103`.

**Input:** `anti-slop --min-severity note . src src/lib.rs` on a crate with one `todo!()`.
- **Observed:** "3 finding(s) in 3 file(s)".
- **Expected:** 1 finding, 1 file.

**Fix:** canonicalize the walked files and deduplicate them.

## 23. Rule catalogue disagrees with the checks (Low)

**Where:** `src/rules.rs:82`, `:158` versus `checks/panics.rs:10`, `:114`.

- The `panic-on-input` summary says `.expect()` is flagged; it never is (`tests/rules.rs:347` asserts this).
- The `panic-in-drop` summary lists `assert!`; it is excluded (`tests/rules.rs:106` asserts this).

`rules` and `explain` therefore describe behaviour the tool doesn't have.

**Fix:** reword the two summaries to match the code.

### Checked and found correct
- **Line and column numbers:** exact on 10,119 corpus findings, and also with CRLF, a BOM, unicode identifiers and blanked frontmatter.
- **JSON escaping:** correct.
- **Edition-2024 syntax:** let chains, `unsafe extern` with `safe fn`, `use<..>`, `&raw`, `#[unsafe(..)]`, async closures, inline `const {}`, `r#try` and `become` all parse. Only nightly `gen {}` blocks fail.
- **Long expression chains:** anti-slop handles 8,000-term `+`/`&&` chains on which rustc itself segfaults, so its stack overflow at 12,000+ terms isn't reported.
- **Scope tracking:** push and pop are balanced in `with_scope` and `self_types`.

