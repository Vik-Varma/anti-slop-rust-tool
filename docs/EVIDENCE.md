# Evidence catalogue

This file records, for each rule, what the outside sources claim, how we checked
the claim, and what we found. The criteria are in [DEFINITION.md](DEFINITION.md).
How each check works and how to reproduce it is in
[`validation/README.md`](../validation/README.md).

Legend for "our check":
- **demo**: reproduced in `tests/demonstrations.rs`.
- **miri**: undefined behaviour or a data race reported by Miri (`validation/ub`).
- **bench**: measured with criterion (`validation/benches`).
- **mining**: size-adjusted association with later bug fixes across 40,320
  functions in 22 repositories (`validation/mining`).

Odds ratios (OR) are size-adjusted unless stated otherwise. OR > 1 means
flagged functions were more likely to be modified by a later bug fix than
unflagged functions of the same size.

## Summary

| rule | default | evidence | outside sources | our check | result |
|---|---|---|---|---|---|
| `lock-across-await` | on | strong | Tokio docs; Clippy `await_holding_lock` | demo | deadlock; control completes |
| `lock-in-scrutinee` | on | strong | Qin et al. PLDI'20 (30 of 38 Mutex/RwLock blocking bugs were double locks) | demo | deadlock; control completes |
| `blocking-in-async` | on | strong | Tokio docs; Ryhl | demo | 10 ms timer fires after 300 ms (11 ms with async sleep) |
| `unbounded-send-sync` | on | strong | Xu et al. TOSEM'21 (most common CVE pattern, 30 of 185 bugs); Rudra SOSP'21 | miri | data race; bounded control clean |
| `uninit-memory` | on | strong | std docs; Clippy `uninit_assumed_init` (deny) | miri | UB; control clean |
| `set-len-uninit` | on | strong | Rudra SOSP'21; Clippy `uninit_vec` (deny) | miri | UB; control clean |
| `unused-io-amount` | on | strong | std `Write` docs; Clippy `unused_io_amount` (deny) | demo | std `Cursor` performs a short write; bytes lost |
| `todo-in-production` | on | strong | Clippy `todo` | demo | panics when reached |
| `panic-in-drop` | on | strong | API Guidelines C-DTOR-FAIL | demo | process aborts (SIGABRT) |
| `float-equality` | on | strong | Goldberg 1991 | demo | `0.1 + 0.2 != 0.3`; representable literals compare exactly |
| `swallowed-error` | on | strong | Yuan et al. OSDI'14 (92% of catastrophic failures from mishandled errors, 25% from ignored errors); de Pádua & Shang MSR'18 (size-controlled) | literature | — (too rare in mined code to test: 12 functions) |
| `exit-in-library` | on | strong | std `process::exit` docs; Clippy `exit` | demo | buffered output lost |
| `deny-warnings` | on | strong | Rust Design Patterns anti-pattern | demo | downstream build breaks when a dependency deprecates a function |
| `borrowed-clone` | on | strong | Design Patterns "clone to satisfy the borrow checker"; Perf Book | bench | 13x (String), 74x (Vec) slower |
| `needless-collect` | on | strong | Perf Book; Clippy `needless_collect` | bench | 1.9–4.3x slower (`len`), up to 88x (`is_empty`) |
| `format-push-string` | on | strong | Clippy `format_push_string` | bench | 2.1x slower |
| `redundant-box` | on | moderate | Clippy `box_collection` (perf) | bench | 1.9x slower to build, 1.18x to read |
| `eager-fallback-alloc` | on | moderate | Perf Book | bench | about 12 ns wasted per call |
| `unwrap-in-result-fn` | on | moderate | Cloudflare outage 2025-11-18; Clippy `unwrap_in_result`; Rust Book ch. 9 | demo, mining | crash instead of `Err`; no frequency association (OR 1.06, p = 0.87) |
| `dbg-macro` | on | moderate | Clippy `dbg_macro` | — | debug output on stderr, by definition |
| `placeholder-comment` | on | moderate | self-declared incompleteness; OpenAI on model "laziness" | precision review | rare in mature code (2 findings in 9,755 files) |
| `satd-comment` | on (info) | moderate | Potdar & Shihab '14; Wehaibi et al. '16; Yuan et al. '14 | mining | **OR 1.58 (1.29–1.93), Holm p = 0.0001** |
| `too-many-params` | on (info) | moderate | Rice et al. OOPSLA'17 (argument mix-ups rise sharply above 5 params) | mining | **OR 1.11 per SD (1.04–1.18), p = 0.002** (threshold flag: OR 1.29, p = 0.069) |
| `undocumented-unsafe` | opt-in | weak | std-dev-guide policy; Clippy; Effective Rust | mining | documented vs undocumented: OR 0.79, p = 0.65 |
| `unwrap-used` | opt-in | weak | Clippy (restriction); disputed by BurntSushi | mining | OR 1.17, p = 0.24 |
| `unit-error-type` | opt-in | weak | API Guidelines C-GOOD-ERR | — | no concrete failure demonstrated |
| `ptr-arg` | opt-in | weak | Clippy `ptr_arg`; Design Patterns idiom | bench, mining | no speed difference; OR 0.93 |
| `high-cognitive-complexity` | opt-in | weak | SonarSource; Muñoz Barón et al. ESEM'20 (reading time, not defects) | mining | OR 1.07, p = 0.60; per SD 0.98 |
| `long-function` | opt-in | weak | Fowler; Clippy `too_many_lines` | mining | crude OR 12.2 → adjusted OR 1.02 |
| `deep-nesting` | opt-in | weak | Johnson et al. ICSME'19 (confounded by snippet length) | mining | OR 1.38, p = 0.094; per SD 1.00 |
| `bool-params` | opt-in | weak | API Guidelines C-CUSTOM-TYPE | mining | OR 1.22, p = 0.45 |
| `duplicate-code` | opt-in | weak | Juergens et al. ICSE'09 (inconsistent clones); Kapser & Godfrey '08 and Rahman et al. '12 (clones not harmful) | mining | OR 0.67 (0.50–0.88), Holm p = 0.052: if anything *less* buggy |
| `commented-out-code` | opt-in | weak | Pham & Yang ICPC'20; Huang et al. FSE'26 (harms AI code generation) | mining | OR 1.01, p = 0.97 |
| `redundant-comment` | opt-in | weak | Steidl et al. ICPC'13 | mining | OR 1.14, p = 0.67 |

`swallowed-error`, the lock rules, the `unsafe` rules and the performance rules
fire too rarely in these mature crates (fewer than 30 functions each) for the
mining to test them. They rest on their demonstrations and benchmarks.

## Correctness

**`lock-in-scrutinee`.** Temporaries in a `match`, `if let` or `while let`
scrutinee live until the end of the construct. So
`match m.lock().unwrap().get(k) { _ => m.lock() }` re-locks a mutex the thread
already holds.
- Qin, Chen, Yu, Song and Zhang (PLDI 2020, doi:10.1145/3385412.3386036)
  studied 59 blocking bugs. 38 involved Mutex/RwLock, and 30 of those were
  double locks. Six had the first lock in a `match` scrutinee. 21 were fixed by
  shortening the guard's lifetime, and Clippy caught none.
- Our demonstration deadlocks, and the `let`-first version completes.
- A plain `if` condition drops its temporaries before the body, so the rule
  ignores it.
- `stdout()`/`stderr()` locks are reentrant (found by our precision sampling on
  nushell), so they are exempt.

**`lock-across-await`.** A std or parking_lot guard held across `.await` blocks
the executor thread when another task on that thread takes the same lock.
- Tokio's documentation: "virtually never leads to correct concurrent code".
- Our demonstration uses a single-threaded runtime with two tasks. It
  deadlocks, and dropping the guard first fixes it.
- Tokio also says std `Mutex` is *preferred* in async code when not held across
  `.await`, so that alone is not flagged.

**`unbounded-send-sync`.**
- Xu et al. (TOSEM 2021, doi:10.1145/3466642) found missing Send/Sync bounds to
  be the most common pattern among memory-safety bugs in Rust CVEs.
- Rudra (Bae et al., SOSP 2021, doi:10.1145/3477132.3483570) found 178 such
  bugs on crates.io, at 48.5% precision.
- Our Miri run reports a data race on an `Rc` refcount through
  `unsafe impl<T> Send for Wrapper<T>`, and none with `T: Send`.

Precision adjustments, from sampling tokio and crossbeam:
- `T: Send` is accepted for `Sync` impls, as for `Mutex<T>`.
- Bounds on associated types are accepted.
- Bounds on crate-specific traits are assumed to possibly carry `Send` as a
  supertrait.

All 12 remaining findings in the 22 repositories were documented, deliberate
impls (TP-intentional).

**`uninit-memory`, `set-len-uninit`.** Clippy correctness lints (deny by
default), and `set_len` after `with_capacity` was a Rudra bug class. Miri
reports undefined behaviour for both, and the initialised controls are clean.
`set-len-uninit` follows Clippy's shape: `set_len` *immediately* after the
allocation, with `MaybeUninit` buffers skipped. An earlier, looser version
flagged rayon code that initialises the buffer through a consumer.

**`unused-io-amount`.** `Write::write` may write only part of the buffer. Our
demonstration uses only std: `Cursor<&mut [u8]>` returns `Ok(4)` for a 6-byte
write.
- To avoid false positives without type information, the argument must look
  like a buffer.
- `self.write(..)` is skipped, since it is usually the type's own helper.
- So is a bare `x.write(..);` statement: for real I/O it already warns through
  `#[must_use]`, and `Hasher::write` returns `()`.

**`todo-in-production`.** `todo!()` always panics when reached (demo). Our
precision sampling found that `unimplemented!()` in a match arm or a trait impl
usually means "deliberately unsupported" (the std docs also distinguish the
two macros). So `unimplemented!()` is flagged only as the whole body of an
inherent or free function.

**`panic-in-drop`.** A panic in `drop` during unwinding aborts the process. Our
demonstration observes SIGABRT. API Guidelines C-DTOR-FAIL. A body that checks
`std::thread::panicking()` is exempt.

**`float-equality`.** `0.1 + 0.2 != 0.3` (demo). The rule flags only literals
that binary floating point cannot represent exactly. It decides this by exact
decimal-to-dyadic analysis, so `x == 0.0`, `x == 0.5` and `x == 1.0` are
allowed. Clippy's broader `float_cmp` is among its most-suppressed lints
(dtolnay's count, rust-clippy#5418).

**`placeholder-comment`.** Matches phrases in which the code describes itself
as incomplete, such as "... rest of the code", "in a real implementation",
"simplified for brevity" or "your code here". Doc comments are skipped, because
their examples legitimately elide code. Our precision sampling pruned several
phrases that humans use in other senses: "for brevity", "you may want to",
"dummy implementation", "existing code", "implementation here", "TODO:
implement" (the last is an ordinary TODO, covered by `satd-comment`). Across
9,755 files of mature crates, only 2 findings remain.

## Robustness

**`swallowed-error`.** Yuan et al. (OSDI 2014) analysed 198 user-reported
failures in five distributed systems:
- 92% of the catastrophic failures came from incorrect handling of
  explicitly signalled errors;
- 25% came from errors that were simply ignored.

De Pádua and Shang (MSR 2018, doi:10.1145/3196398.3196435) found empty "dummy
handlers" predicted post-release defects *on top of* size and other metrics.

The rule flags:
- empty `Err(_)`/`Err(e)` arms;
- empty `if let Err(..)` bodies;
- `.ok();` statements.

Exemptions, from precision sampling:
- arms that match a specific error variant;
- empty handlers that contain an explanatory comment;
- `write!(string, ..).ok()`, which cannot fail;
- fire-and-forget channel sends and `OnceCell::set`.

**`unwrap-in-result-fn`.** In a function that already returns
`Result`/`Option`, `.unwrap()` turns a recoverable error into a crash. This is
the shape of Cloudflare's 18 November 2025 outage, where an oversized config
file made the FL2 proxy panic on `Result::unwrap()`.
- Our demonstration shows the crash where `?` would have returned the error.
- Mining found no link with *how often* such functions need fixing (OR 1.06).
  The cost is the severity of a failure, not its frequency.
- `.lock().unwrap()`, `.join().unwrap()`, unwraps of calls whose arguments are
  all literals, and closures are exempt.
- `.expect()` is not flagged, because the Rust Book recommends it for
  documenting why failure is impossible.

**`exit-in-library`.** `process::exit` skips destructors. Our demonstration
loses a `BufWriter`'s data. Files that only a binary target reaches are
classified as binary code by following the module tree, so CLI helper modules
are not reported.

## Performance

All measured with criterion; full numbers are in
[`validation/results/benchmarks.md`](../validation/results/benchmarks.md).

| rule | flagged | fix | slowdown |
|---|---|---|---:|
| `borrowed-clone` | `f(&s.clone())` (64 B String) | `f(&s)` | 13x |
| `borrowed-clone` | `f(&v.clone())` (1000 × u64) | `f(&v)` | 74x |
| `needless-collect` | `.collect::<Vec<_>>().len()` | `.count()` | 1.9–4.3x |
| `needless-collect` | `.collect::<Vec<_>>().is_empty()` | `.next().is_none()` | up to 88x |
| `format-push-string` | `s.push_str(&format!(..))` | `write!(s, ..)` | 2.1x |
| `redundant-box` | `Box<Vec<_>>` | `Vec<_>` | 1.9x build, 1.18x read |
| `eager-fallback-alloc` | `.unwrap_or(format!(..))` | `.unwrap_or_else(..)` | about 12 ns per call |

Detector choices shaped by the evidence:
- `needless-collect` flags only `len`, `is_empty`, `contains`, `first` and
  `last` after `collect::<Vec<_>>()`. Clippy demoted its broader lint for false
  positives, and `.collect().into_iter()` can be needed for `.rev()`.
- `borrowed-clone` flags only `.clone()`. In sampling, `&x.to_owned()` and
  `&x.to_vec()` were often required to build a `&String`/`&Vec` argument.
- `eager-fallback-alloc` ignores `"".to_string()`, `String::new()` and
  `Vec::new()`, none of which allocate. The precision review found the empty
  string case.

`blocking-in-async`: in our demonstration, a 10 ms timer task fired after
300 ms when another task called `std::thread::sleep(300 ms)`, and after 11 ms
with `tokio::time::sleep`.
- Closures passed to free functions (`spawn_blocking`, `thread::spawn`,
  tokio's `asyncify`) are skipped.
- So are futures handed to `block_on` from synchronous code.
- `blocking_*`/`block_on` inside a runtime are flagged, because they panic.

## Maintainability

The mining replicated the classic size-confounding result.

| metric (threshold) | crude OR | size-adjusted OR (95% CI) | Holm p |
|---|---:|---|---:|
| `long-function` (> 100 LOC) | 12.15 | 1.02 (0.75–1.38) | 1 |
| `high-cognitive-complexity` (> 25) | 9.16 | 1.07 (0.82–1.41) | 1 |
| `deep-nesting` (> 5) | 10.71 | 1.38 (0.95–2.02) | 0.94 |
| `bool-params` (≥ 2) | 5.74 | 1.22 (0.73–2.05) | 1 |
| `too-many-params` (> 5) | 4.83 | 1.29 (0.98–1.69) | 0.76 |
| `satd-comment` | 4.77 | **1.58 (1.29–1.93)** | **0.0001** |
| `duplicate-code` | 2.92 | 0.67 (0.50–0.88) | 0.052 |
| `commented-out-code` | 3.74 | 1.01 (0.66–1.54) | 1 |

Every metric looks predictive until size is controlled. Afterwards only TODO
markers remain significant. Among continuous metrics, only parameter count adds
information (OR 1.11 per SD, p = 0.002).

That agrees with Rice et al. (OOPSLA 2017): mining 200 MLOC at Google, they
found argument-selection defects rise sharply above five parameters. The
default threshold is therefore 5, not Clippy's 7.

Cyclomatic and cognitive complexity correlate with LOC at Spearman 0.72 and
0.71 and add nothing beyond it. This matches:
- El Emam et al. (TSE 2001);
- Lavazza et al. (JSS 2023), who found cognitive complexity predicts no better
  than LOC;
- Clippy's own history: `cognitive_complexity` was demoted to nursery in Rust
  1.44 and to restriction in 1.91, and its docs now say "We used to think it
  measured how hard a method is to understand."

The complexity rules remain available as opt-in tools for reading code. They
are not reported as bad code.

## Precision on real code

To estimate precision, findings of the default rules were sampled at random
from the current code of the 22 repositories and labelled by independent
reviewers (separate model sessions that saw only the rule text and the code).
Each finding was labelled **TP** (pattern present and the harm can occur), **TP-intentional**
(pattern present but evidently deliberate, e.g. an invariant `assert!` in `Drop`),
or **FP** (detector wrong, exempt case, or test code). Method details are in
[`validation/README.md`](../validation/README.md#precision-review); items and
labels are in [`validation/precision`](../validation/precision).

**Round 1** used 155 findings and two reviewers. They agreed on 151/155 labels
(Cohen's κ = 0.95).

| rule | n | TP | TP-intentional | FP |
|---|---:|---:|---:|---:|
| blocking-in-async | 10 | 8 | 1 | 1 |
| borrowed-clone | 4 | 3 | 0 | 1 |
| dbg-macro | 10 | 1 | 9 | 0 |
| eager-fallback-alloc | 12 | 7 | 0 | 5 |
| exit-in-library | 12 | 9.5 | 2.5 | 0 |
| format-push-string | 12 | 12 | 0 | 0 |
| panic-in-drop | 12 | 5 | 7 | 0 |
| placeholder-comment | 4 | 0 | 2 | 2 |
| redundant-box | 3 | 2 | 1 | 0 |
| satd-comment | 12 | 12 | 0 | 0 |
| swallowed-error | 12 | 5.5 | 3.5 | 3 |
| todo-in-production | 10 | 2 | 7 | 1 |
| too-many-params | 12 | 12 | 0 | 0 |
| unbounded-send-sync | 12 | 0 | 12 | 0 |
| unused-io-amount | 6 | 1 | 3 | 2 |
| unwrap-in-result-fn | 12 | 5 | 7 | 0 |
| **all** | **155** | **85** | **55** | **15 (9.7%)** |

(Half counts are where the two reviewers disagreed.)

The false-positive classes were then fixed:
- empty-string allocations;
- futures passed to `block_on` from synchronous code;
- a placeholder phrase used as prose;
- fuzz targets and `test_data`/`fixtures` directories treated as production code.

Re-running on the same 155 findings removed exactly the 11 findings both
reviewers called FP, plus the 2 disputed ones, and no true positives.
Precision on that sample rose to 96.5%. Because the fixes were made with this
sample in view, that figure is optimistic.

**Round 2** drew 64 *new* findings after the fixes, labelled by a third
reviewer. Every finding was correct:

| rule | n | TP | TP-intentional | FP |
|---|---:|---:|---:|---:|
| eager-fallback-alloc | 8 | 8 | 0 | 0 |
| exit-in-library | 8 | 2 | 6 | 0 |
| format-push-string | 8 | 8 | 0 | 0 |
| panic-in-drop | 8 | 2 | 6 | 0 |
| satd-comment | 8 | 8 | 0 | 0 |
| swallowed-error | 8 | 6 | 2 | 0 |
| too-many-params | 8 | 6 | 2 | 0 |
| unwrap-in-result-fn | 8 | 4 | 4 | 0 |
| **all** | **64** | **44** | **20** | **0** |

That gives a precision of 64/64, with a 95% Clopper-Pearson interval of
94.4–100%. The other default rules had 12 or fewer findings in total, so round 1
already reviewed every one of them.

**What precision does not tell you.** Detector precision is high: the flagged
pattern is almost always really there. But 31% of round-2 findings, and 35% of
round-1 findings, were judged deliberate in context. The unwraps cannot fail by
construction, the `Drop` asserts guard invariants, the `unsafe impl`s are
documented. Mature, heavily reviewed crates are where such deliberate cases
concentrate. For new or machine-generated code, the share of actionable
findings should be higher. Use the suppression comments
(`// anti-slop: allow(rule)`) to record deliberate cases.

## Thresholds

| setting | default | basis |
|---|---:|---|
| `max_params` | 5 | Rice et al. 2017: defects rise markedly above five parameters |
| `max_cognitive_complexity` | 25 | Clippy's default (opt-in rule; Sonar's 15 is a convention) |
| `max_function_lines` | 100 | Clippy `too_many_lines` default (opt-in rule) |
| `max_nesting` | 5 | convention (opt-in rule) |
| `min_duplicate_tokens` | 120 | conservative, so only long runs match (opt-in rule) |

No study derives an empirically optimal threshold for function length,
complexity or nesting. El Emam et al. (2002) found no threshold effect for size.
The opt-in thresholds are therefore conventions, and are configurable in
`anti-slop.toml`.
