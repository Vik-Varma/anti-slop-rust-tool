# anti-slop

A fast static analyser for Rust that flags **bad code**, where "bad" has a
concrete, testable definition. Every default rule has had its claim of harm
checked. Each check is one of:
- a reproduced failure (deadlock, abort, crash, data loss, broken build);
- undefined behaviour caught by Miri;
- a measured slowdown;
- a size-controlled link to later bug fixes, mined from 22 real Rust projects.

Popular claims that failed these checks, such as cyclomatic complexity predicting
bugs or `.iter().count()` being slow, are documented as rejected. Their rules are
off by default.

anti-slop works on source text alone. It needs no compilation, type information
or Cargo build, so it can check a crate, a single file, or a snippet pasted from
a code assistant in milliseconds. It is meant to complement Clippy, not replace
it.

```text
$ anti-slop check src/lib.rs
src/lib.rs:5:5: warning[placeholder-comment] placeholder comment ("in a real implementation"): the code here is likely incomplete
src/lib.rs:6:38: warning[unwrap-in-result-fn] `.unwrap()` in a function that returns Result/Option; propagate with `?` instead
src/lib.rs:11:19: error[lock-across-await] guard `g` from `.lock()` is still held at the `.await` on line 14; drop it first
src/lib.rs:13:5: warning[blocking-in-async] blocking `std::thread::sleep` inside async code; use `tokio::time::sleep(..).await`
src/lib.rs:20:9: warning[swallowed-error] `Err` arm discards the error without handling or logging it
src/lib.rs:22:39: warning[needless-collect] collected into a Vec only to call `.len()`; use `.count()`

1 files checked, 6 findings (1 error, 5 warning, 0 info)
```

## What counts as bad code

A pattern is reported by default only if all three of these hold:
1. **It does harm** to correctness, robustness, performance, or (measurably)
   maintainability.
2. **The harm is backed by evidence**: reproduced in this repository, shown in
   size-controlled empirical data, or declared by the code itself (`todo!()`,
   "rest of the code here"). An authoritative source must also describe the
   mechanism.
3. **It can be detected precisely** from source code. Independent review of
   219 findings on real crates measured this.

The full definition is in [`docs/DEFINITION.md`](docs/DEFINITION.md). The
per-rule evidence is in [`docs/EVIDENCE.md`](docs/EVIDENCE.md). How to
reproduce each check is in [`validation/`](validation/README.md).

## Install and run

```sh
cargo install --path .
anti-slop check                 # current directory
anti-slop check src/ other.rs   # files and directories (.gitignore is respected)
anti-slop check --format json   # machine-readable output
anti-slop check --fail-on warning   # exit 1 for CI
anti-slop rules                 # list rules with evidence grades
anti-slop explain lock-in-scrutinee # rationale and sources for one rule
anti-slop metrics src/          # per-function size/complexity as JSON lines
```

## Rules

**On by default (verified).**

| rule | severity | what it catches | evidence |
|---|---|---|---|
| `lock-across-await` | error | std/parking_lot guard (or `RefCell` borrow) held across `.await` | deadlock reproduced |
| `lock-in-scrutinee` | error | `match m.lock()...` re-locked inside the match | deadlock reproduced; PLDI'20 study of real Rust blocking bugs |
| `uninit-memory` | error | `MaybeUninit::uninit().assume_init()`, `mem::uninitialized()` | UB under Miri |
| `set-len-uninit` | error | `Vec::set_len` right after allocating | UB under Miri |
| `unused-io-amount` | error | `w.write(buf)?` with the byte count ignored | short write reproduced with std types |
| `todo-in-production` | error | `todo!()`; functions whose body is only `unimplemented!()` | panics |
| `unbounded-send-sync` | warning | `unsafe impl<T> Send/Sync` without `T: Send/Sync` | data race under Miri; most common CVE pattern |
| `blocking-in-async` | warning | `thread::sleep`, `std::fs`, `std::net`, `block_on`, `blocking_*` in async code | 30x timer delay reproduced |
| `panic-in-drop` | warning | `unwrap`/`expect`/`panic!`/`assert!` in `Drop::drop` | abort reproduced |
| `float-equality` | warning | `x == 0.1` (literal not exactly representable) | `0.1 + 0.2 != 0.3` |
| `swallowed-error` | warning | `Err(_) => {}`, `result.ok();` | 25% of catastrophic failures in an OSDI'14 study |
| `unwrap-in-result-fn` | warning | `.unwrap()` in a function that already returns `Result`/`Option` | crash reproduced; Cloudflare's Nov 2025 outage |
| `deny-warnings` | warning | `#![deny(warnings)]` | broken downstream build reproduced |
| `placeholder-comment` | warning | "... rest of the code", "in a real implementation", "simplified for brevity" | self-declared stub |
| `dbg-macro` | warning | leftover `dbg!` | Clippy `dbg_macro` |
| `borrowed-clone` | warning | `f(&x.clone())` | 13–74x slower |
| `needless-collect` | warning | `.collect::<Vec<_>>().len()` / `.is_empty()` | 2–88x slower |
| `redundant-box` | warning | `Box<Vec<_>>`, `Box<String>`, `Box<HashMap<..>>` | 1.9x slower |
| `format-push-string` | info | `s.push_str(&format!(..))` | 2.1x slower |
| `eager-fallback-alloc` | info | `.unwrap_or(format!(..))`, `.ok_or(x.to_string())` | wasted allocation per call |
| `exit-in-library` | info | `process::exit` in library code | lost buffered output reproduced |
| `satd-comment` | info | TODO / FIXME / HACK / XXX | 1.58x odds of a later bug fix at equal size |
| `too-many-params` | info | more than 5 parameters | defects rise above 5 params (OOPSLA'17); 1.11x odds per SD in our data |

**Opt-in (unverified).** Enable with `--all-rules` or `--enable <id>`.
These are widely recommended, but our checks found no harm:
`undocumented-unsafe`, `unwrap-used`, `unit-error-type`, `ptr-arg`,
`high-cognitive-complexity`, `long-function`, `deep-nesting`, `bool-params`,
`duplicate-code`, `commented-out-code` and `redundant-comment`.

Test code is exempt from most rules: `#[test]`, `#[cfg(test)]` modules
(including ones in separate files), `tests/`, `benches/`, `examples/`, build
scripts, and fuzz and test-support crates. This follows the Rust Book's
guidance that panicking is how tests fail.

## Suppressing findings

```rust
// anti-slop: allow(unwrap-in-result-fn) -- the regex is a literal
let re = Regex::new(PATTERN).unwrap();

let n = parse(x).unwrap(); // anti-slop: allow(unwrap-in-result-fn)

// anti-slop: allow-file(too-many-params)    (anywhere in the file)
```

An own-line directive covers itself and the next line of code. A trailing
directive covers its own line. `allow` with no list suppresses every rule.

## Configuration

`anti-slop.toml` in the current directory, or `--config <file>`:

```toml
max_params = 5                 # too-many-params threshold
max_cognitive_complexity = 25  # opt-in rules' thresholds
max_function_lines = 100
max_nesting = 5
min_duplicate_tokens = 120
enable = ["undocumented-unsafe"]
disable = ["satd-comment"]
# only = ["lock-across-await"]
# all_rules = true
```

## How it works

- **Parsing.** Files are parsed with [`syn`](https://docs.rs/syn). A small
  lexer recovers comments, which `syn` discards.
- **Macros.** Macro bodies that look like expression lists (`println!`,
  `assert!`, `vec!`, `write!`, …) are parsed too, so code inside them is
  checked.
- **File roles.** Each file is classified as library, binary, test, bench,
  example or build script. The classifier uses its location and follows `mod`
  declarations from each target's root, so a module reached only through
  `#[cfg(test)] mod tests;` counts as test code, and one reached only from
  `main.rs` counts as binary code.
- **Speed.** Rules run in parallel per file. The 22 repositories used for
  validation (9,755 files) take about 20 seconds in total.
- **Type information.** There is none. Where a rule could be fooled by
  same-named methods (`write`, `lock`), it uses conservative shape checks.
  These were tuned against real code, and every heuristic is documented in
  [`docs/EVIDENCE.md`](docs/EVIDENCE.md).

## Development

```sh
cargo test                     # unit tests + executable demonstrations
cargo clippy --all-targets
cargo run -- check src tests   # the tool checks itself (0 findings)
validation/ub/run.sh           # Miri checks (nightly)
cargo bench --manifest-path validation/benches/Cargo.toml
```
