# anti-slop

A Rust linter that only calls code bad when there's evidence it is.

Most "bad code" advice is taste or folklore. This tool started from the opposite end. It gathered
the claims trusted sources make (the Rust Book, std docs, Clippy, Tokio, CWE, the research
literature), then tested each one:

- **Does the failure actually happen?** Each claim has a program in `experiments/` that
  reproduces it: panics, process aborts, a deadlock, a stalled executor, data silently lost,
  undefined behaviour caught by Miri, and a build that breaks on a newer compiler.
- **Is the pattern bad where it actually appears?** Each check ran over 37 real codebases,
  and random samples of its findings were judged by hand.
- **Do "maintainability" smells predict bugs?** Functions with each smell were tracked through
  the git history of 35 projects (about 72,000 functions) to see whether they needed more
  bug fixes than functions of the same size without it. The hypotheses were fixed in advance
  of the confirming data.

Many popular claims failed. High cyclomatic or cognitive complexity, long functions, deep
nesting, many parameters, magic numbers, and TODO/FIXME comments all predict bug fixes only
because they measure size. Once size is controlled for, none of them adds anything, and that
held up in a preregistered replication. So this tool doesn't flag them.

The definition, decision rules and limits are in [docs/DEFINITION.md](docs/DEFINITION.md).
Every claim, kept or rejected, is in [docs/EVIDENCE.md](docs/EVIDENCE.md), and the hand audit is
in [docs/AUDIT.md](docs/AUDIT.md).

## Use

```sh
cargo install --path .
anti-slop path/to/crate                 # warnings and errors; exit 1 if any
anti-slop --min-severity note path      # also the notes: real mechanisms, usually deliberate
anti-slop --format json path
anti-slop rules                         # the catalogue
anti-slop explain lock-held-across-await
anti-slop metrics --format csv path     # per-function size, complexity and finding counts
```

Silence a finding with a reason, on its line or the line above:

```rust
// anti-slop: allow(todo) tracked in #123, unreachable until the v2 protocol ships
```

It reads syntax only (via `syn`), so it needs no build and works on code that doesn't compile.
It doesn't replace Clippy, and overlaps with it in only a few places. Clippy's type-aware lints
cover several patterns better than syntax can, so those aren't duplicated here (`unused-io-amount`,
for example; see EVIDENCE.md).

## Rules

Shown by default:

| Rule | Severity | Flags | Evidence |
|---|---|---|---|
| `uninit-memory` | error | `mem::uninitialized()`, `MaybeUninit::uninit().assume_init()` | Miri reports UB |
| `lock-held-across-await` | error | a std/parking_lot guard alive at an `.await` | deadlock reproduced; 7/7 in the audit |
| `todo` | warning | `todo!()` in shipping code | 10/10 real in the audit |
| `blocking-sleep-in-async` | warning | `thread::sleep` in async code | other tasks get 0 turns during the sleep |
| `float-eq-inexact` | warning | `x == 0.1` (literals with no exact binary value) | `0.1 + 0.2 != 0.3`; exact literals are left alone |
| `deny-warnings` | warning | `#![deny(warnings)]` in a crate root | builds on Rust 1.76, fails on 1.99 |

Notes (`--min-severity note`). Each mechanism is real, but most audited findings were deliberate
or safe, so they're review prompts rather than verdicts:

| Rule | Flags | Audit |
|---|---|---|
| `panic-on-input` | `.unwrap()` on parse/decode/file/env/`to_str` results | 8/20 crash on real input |
| `ignored-io-error` | `let _ =`/`.ok();` on write/flush/sync/rename in a fn returning `Result` | 4/10 |
| `panic-in-drop` | `unwrap`/`expect`/`panic!` in `Drop::drop`, unless guarded by `thread::panicking()` | 3/10 |
| `unwrap` | other `.unwrap()` in shipping code | 2/20 |
| `static-mut` | `static mut` | 0/10 races |
| `process-exit-in-library` | `process::exit` in a library crate | 0/10 lost data |
| `unsafe-without-comment` | `unsafe` with no justifying comment | consensus, not measured |
| `len-truncating-cast` | `.len() as u32` and narrower | 0/20 reachable |
| `dbg-macro` | `dbg!` in shipping code | 1/10 accidental |

Tests, examples, benches, build scripts, `#[cfg(test)]` modules (including those in their own
files), test-support crates and `xtask` crates aren't judged as shipping code.

## Reproduce

```sh
cargo test                                   # the tool
(cd experiments && cargo test)               # the failure experiments
experiments/miri/run.sh                      # needs nightly + miri
experiments/deny-warnings/run.sh             # needs `rustup toolchain install 1.76`
python3 study/check_sources.py               # every cited page still says what it's cited for
```

CI runs the tool's tests and the experiments on Linux, Windows and macOS, and re-checks the
sources weekly.

The study (`study/`) needs Python 3 with numpy and scipy, and clones of the repositories in
`study/repos.txt` and `study/repos-replication.txt`. `study/corpus-heads.txt` records the commits
used. Run `study/mine_all.sh`, then `study/analyze.py`, then `study/confirm.py`. The mined data is
in `study/data*/`, mined by this repository's build at commit `18c520c`.

## License

Not chosen yet.
