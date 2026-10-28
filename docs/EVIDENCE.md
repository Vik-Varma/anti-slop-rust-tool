# Evidence ledger

Every claim about "bad code" that was tested, what it was tested with, and the verdict. Claims
that failed are kept, because knowing what *isn't* bad is half the definition
([DEFINITION.md](DEFINITION.md)).

Three kinds of test were used:

- **Experiment**: a program in `experiments/` that reproduces the claimed failure. `cargo test`
  in `experiments/` re-runs them; `experiments/miri/run.sh` and `experiments/deny-warnings/run.sh`
  cover the UB and compiler-version cases. Results below are from Linux x86-64, Rust 1.99.1, and
  nightly 1.101 for Miri.
- **Defect study**: does the pattern mark functions that later needed a bug fix, beyond what their
  size explains? 44,961 non-test functions from 20 projects (discovery), then 26,746 from 15 other
  projects (replication), with the hypotheses fixed in `study/PREREGISTRATION.md` first. Method:
  the snapshot is 2024-07-01, and fixes are bug-fix commits up to 2026-10-01 that touched the
  function. The models are logistic regressions with a repository fixed effect, log(LOC) and
  log(prior churn), using file-clustered standard errors and a Holm correction.
  Tables: `study/results.md`, `study/results-replication.md`, `study/replication.md`. A review
  of the study (`docs/reviews/study-review.md`) led to a sensitivity analysis that isn't
  preregistered: pooled fits, repository-clustered errors and a corrected miner
  (`study/sensitivity.md`).
- **Audit**: each rule run over 37 codebases (the 35 study projects, Ferrock and Dataminer), with
  random samples hand-checked ([AUDIT.md](AUDIT.md)).

OR = odds ratio of being bug-fixed later: per doubling of (1 + metric) for metrics, or for having
the pattern at all.

## Rules: claims that passed

| Rule | Claim | Experiment | Audit | Verdict |
|---|---|---|---|---|
| `uninit-memory` | Reading uninitialised memory is UB | Miri: UB for `MaybeUninit::<u32>::uninit().assume_init()` and `mem::uninitialized::<u32>()`; the `[MaybeUninit<T>; N]` form is clean. `mem::uninitialized::<&u8>()` aborts the process | no instances | **error** |
| `lock-held-across-await` | A sync guard held over `.await` deadlocks | Current-thread Tokio, two futures: std `Mutex` hangs (2 s timeout); `tokio::sync::Mutex` completes | 7/7 really hold a guard across `.await`; 0/7 can deadlock as written, since nothing else takes the lock meanwhile (6 in tests) | **note** |
| `todo` | `todo!()` ships and panics | panics "not yet implemented" | 6/15 reachable (census); the rest are `todo!()` used as `unreachable!()` on paths no caller reaches | **note** |
| `blocking-sleep-in-async` | `thread::sleep` stalls the executor | a 5 ms ticker runs 0 times during a 200 ms `thread::sleep` and keeps ticking during `tokio::time::sleep` (current-thread runtime) | 0 outside tests; all 10 in tests deliberate, so tests exempt | **warning** |
| `float-eq-inexact` | `==` with an inexact literal is unreliable | `0.1 + 0.2 != 0.3`; `0.25 + 0.25 == 0.5` | no instances | **warning** (exact literals not flagged) |
| `deny-warnings` | `#![deny(warnings)]` breaks on newer compilers | same file builds on 1.76, fails on 1.99 (`static_mut_refs`) | 0 in crate roots; 31 in examples/tests/benches, now exempt | **warning** |
| `catch-unwind-under-panic-abort` | `catch_unwind` can't catch when the profile aborts on panic (Google's reason for banning it; Cargo and std document the mechanism) | `experiments/panic-abort`: the same program exits 134 (SIGABRT) with `panic = "abort"` and prints `recovered` with `unwind`; Cargo ignores the setting for tests, so `cargo test` can't show it | 1 instance in shipping code (tauri's CLI: a panic report to Node that its release profile never delivers), judged real; a second in a proc-macro crate is exempt, since Cargo builds those with unwinding | **warning** (exact given the profile) |
| `panic-on-input` | unwrap on parse/IO/env results crashes on bad input | `unwrap` panics without context, and a panic poisons shared locks; Cloudflare's 18 Nov 2025 outage (unwrap on an `Err` after a generated file exceeded 200 features) | 5/20 crash on real external input | **note** |
| `panic-in-drop` | Panicking in `Drop` during unwinding aborts | SIGABRT, "panic in a destructor during cleanup"; `catch_unwind` never runs | 1/10 can realistically fire | **note** |
| `ignored-io-error` | `let _ =` on a write loses data silently | `/dev/full`: `write_all` returns ENOSPC, the ignoring caller reports success; `BufWriter` drop swallows the error | 7/16 hid a failure from a caller expecting errors (census) | **note** |
| `process-exit-in-library` | `exit` skips destructors | `BufWriter` record lost, process exits 0 | 10/10 libraries exiting; 0/10 lost data | **note** |
| `static-mut` | `static mut` invites data races | Miri: data race on a two-thread counter; atomic version clean | 0/10 races; 10/10 deliberate (FFI, guarded) | **note** |
| `len-truncating-cast` | `len() as u32` wraps silently | a real 4 GiB + 1 `Vec`: `len() as u32 == 1` | 0/20 shown reachable | **note** |
| `unwrap` | unwrap panics without context | as above | 2/20 can fail at run time | **note** |
| `dbg-macro` | `dbg!` leaks to stderr in release | (behaviour documented in std) | 1/10 accidental | **note** |
| `unsafe-without-comment` | unsafe needs a written justification | none possible: it is the rule for rustc's and std's own code (std-dev guide) and a Clippy restriction lint (`undocumented_unsafe_blocks`) | 20/20 lacked one; none shown unsound. Study: OR 1.18 [0.75, 1.87] discovery, 1.92 [1.11, 3.35] replication, neither significant after Holm correction | **note** (consensus) |

## Claims that failed

### Complexity and size metrics (defect study)

Only H2 to H6 were preregistered; rows without an H were explored after the discovery data and
weren't confirmed by anything. "Pooled" fits all 35 projects together, with standard errors
clustered by repository. "Corrected" re-mines both corpora with the fixes the study review asked
for (renames followed, pure moves and whole-file deletions ignored, insertions credited only to a
function they land inside, mainline snapshots); see `study/sensitivity.md`. Neither is
preregistered.

| Pattern | Naive OR | Discovery, adjusted | Replication, adjusted | Pooled | Pooled, corrected | Verdict |
|---|---|---|---|---|---|---|
| Cyclomatic complexity (H3) | 2.16 | 1.01 [0.94, 1.08] | 1.07 [0.95, 1.20] (held) | 1.03 [0.92, 1.15] | 0.98 [0.89, 1.08] | no detectable effect beyond size; corrected, the 90% CI lies inside 0.9 to 1.1 |
| Cognitive complexity (H4) | 1.74 | 1.05 [1.00, 1.11] | 1.04 [0.97, 1.12] (held) | 1.05 [0.97, 1.14] | 1.02 [0.95, 1.10] | the same |
| Cognitive > 15 (H6) | 8.36 | 0.89 [0.75, 1.07] | 0.98 [0.74, 1.29] (held) | 0.93 [0.77, 1.12] | 0.85 [0.69, 1.05] | no detectable effect; an effect up to about 1.1 isn't ruled out |
| > 100 lines (H5) | 12.07 | 0.66 [0.53, 0.82] | 0.74 [0.51, 1.08] (held) | 0.69 [0.56, 0.86] | 0.63 [0.52, 0.77] | *fewer* fixes than size predicts: a misfit of the single log-size term, not a benefit of long functions |
| Nesting depth (H2) | 2.86 | 1.18 [1.07, 1.29] | 1.11 [0.97, 1.27] (**failed**) | 1.15 [1.01, 1.32] | 1.10 [0.99, 1.23] | small effect, if any; the replication is inconclusive (39% power) |
| Parameter count | 2.03 | 1.13 [1.04, 1.22] | 1.06 [0.95, 1.19] | 1.10 [1.00, 1.21] | 1.16 [1.01, 1.33] | small effect, exploratory only |
| > 7 parameters (Clippy) | 4.46 | 1.06 [0.70, 1.61] | 0.77 [0.42, 1.41] | 0.95 [0.64, 1.41] | 1.03 [0.73, 1.46] | no detectable effect; wide intervals |

Size itself is the strongest signal: each doubling of (1 + lines of code) multiplies the odds of
a later fix by 2.13 in discovery and 1.92 in replication. The continuous metrics above correlate
with size (Spearman ρ 0.69 to 0.73; parameter count 0.47 to 0.50), the threshold rules much less
(0.08 to 0.28). This is the confound El Emam et al. (IEEE TSE 2001) showed for object-oriented
metrics; a published comment disputes how they treated size (Evanco, IEEE TSE 2003). Herraiz &
Hassan (2010) found complexity metrics track lines of code closely in C. Landman et al. (2016),
on 24 million Java methods and C functions, found the correlation "only moderate" and concluded
CC is not redundant with size. The data here agrees the two differ, but the part of cyclomatic
and cognitive complexity that isn't size didn't predict fixes. Results were the same with size
alone as the control (the "size-adjusted" column of `study/results.md`), so adjusting for prior
churn as well isn't what removed the effects.

The long-function result shows the size control's limit. The outcome is yes or no, and it
saturates: 71% of functions over 200 lines were fixed later in discovery, against 2% of those
under 5. One log-size term can't follow that curve, so threshold rules pick up what it misses.
That makes "> 100 lines" look protective, and it may hide or inflate other small effects too.

Clippy reached the same place from experience: its `cognitive_complexity` lint now sits in the
restriction group because "the true Cognitive Complexity of a method is not something we can
calculate using modern technology", and it was kept there "so as to not mislead users into using
this lint as a measurement tool".

### Comments and other "smells" (defect study)

| Pattern | Naive OR | Discovery, adjusted | Replication, adjusted | Pooled | Pooled, corrected | Verdict |
|---|---|---|---|---|---|---|
| TODO/FIXME/HACK/XXX comment (H1) | 4.68 | 1.48 [1.23, 1.79] | 1.19 [0.85, 1.66] (**failed**) | 1.41 [1.20, 1.67] | 1.42 [1.21, 1.66] | a real but small effect, probably; the replication is inconclusive (36% power) |
| Comment lines in the function | 1.88 | 1.13 [1.08, 1.19] | 1.11 [1.02, 1.19] | 1.13 [1.06, 1.21] | 1.10 [1.04, 1.17] | small, consistent effect; exploratory only |
| antislop "deferral" phrases | 3.64 | 1.00 [0.68, 1.47] | 1.15 [0.58, 2.31] | | | no detectable effect; too rare to say more |
| antislop "hedging" phrases | 3.71 | 0.87 [0.50, 1.52] | 0.57 [0.25, 1.27] | | | the same |
| Magic numbers | 1.45 | 0.87 [0.79, 0.96] | 1.02 [0.89, 1.16] | 0.92 [0.82, 1.04] | 0.92 [0.82, 1.04] | no detectable effect |
| `.clone()` calls | 2.51 | 1.12 [1.03, 1.22] | 0.90 [0.79, 1.02] | 1.05 [0.96, 1.15] | 1.07 [0.98, 1.16] | no detectable effect |
| Boolean parameters | 2.26 | 1.24 [1.04, 1.48] | 1.02 [0.81, 1.29] | 1.15 [0.97, 1.36] | 1.17 [1.01, 1.36] | not replicated; small if real |
| `unwrap` as fix-proneness (H7) | 2.45 | 0.97 [0.78, 1.20] | 1.01 [0.77, 1.32] (held) | 0.98 [0.85, 1.12] | 0.97 [0.82, 1.14] | no detectable effect; still a crash mechanism |
| `expect`, `panic!`/`assert!` | 2.19, 2.03 | 1.03 [0.88, 1.22], 0.95 [0.83, 1.10] | 0.95 [0.80, 1.14], 0.93 [0.74, 1.15] | | | no detectable effect |

Every row but H1 and H7 is exploratory. The `unwrap` column is the rule as it stood when the study
was mined, before `panic-on-input` was split out of it; likewise the `unsafe` column below.

None of the effects that survived is large. A TODO comment raises the odds of a later fix about as
much as 40% more lines does, and a doubling of comment lines by about 10%. Comments may well mark
code that is already known to be tricky rather than cause anything. Neither has a failure
mechanism to reproduce, so neither is "bad code" by this tool's definition, and the tool doesn't
flag them; the result is a candidate for the next preregistered test, not a rule.

One pattern leans the other way: functions with an `unsafe` block lacking a `SAFETY:` comment
had point estimates above 1 in both corpora (1.18, then 1.92; pooled 1.29 [0.85, 1.96], and 1.44
[0.96, 2.15] corrected), but none of these survives the multiple-comparison correction, it wasn't
a preregistered hypothesis, and only 209 replication functions had one. That is a lead, not
evidence.

Other studies agree the evidence on TODO comments is weak. Wehaibi et al. (SANER 2016) found "no
clear trend when it comes to defects and self-admitted technical debt" across five projects. A
2025 method-level study (arXiv 2411.13777) links such methods to bugs, but notes they are also
larger, which is the confound controlled for here. Yuan et al. counted a "TODO" in an error
handler in only 2% of the catastrophic failures they studied.

Full tables, including the flexible-size and changed-functions-only robustness checks, are in
`study/results.md`, `study/sensitivity.md` and `study/antislop-phrases.md`.

### Google's Rust style guide, claim by claim

The guide (https://google.github.io/styleguide/rustguide.html, undated; binding for Android,
Chromium and Fuchsia library code) was read in full and each rule sorted by the reason it gives.
One names a failure that reproduces; three agree with rules already here; the rest are
preferences, which `docs/STYLE.md` covers.

| Guide rule | Reason the guide gives | Here |
|---|---|---|
| Don't call `panic::catch_unwind` | "will only function correctly in environments with `panic=unwind`"; Chrome, Android and Fuchsia build with `panic=abort` | **adopted** as `catch-unwind-under-panic-abort` where the profile is visible (table above); the rest of the prohibition is the style rule `catch-unwind` |
| Unsafe code: a `SAFETY:` comment and a justification | none beyond review | already `unsafe-without-comment`; the guide is a third source. The `SAFETY:` wording itself is the style rule `safety-comment-prefix` |
| Reporting errors: `panic!` for bugs and violated invariants, `Result` for recoverable failures | the API contract | agrees with `panic-on-input`; added as a source |
| Prefer `expect` over `unwrap` | "to document the reason" | agrees with the Book; a source on `unwrap` |
| Never reach inside a poisoned mutex | none given | **not a defect rule**: std calls poisoning "only advisory" and documents `PoisonError::into_inner` as the way to proceed; of 55 corpus uses, a sample of 10 were all deliberate (caches, counters, a lock released during cleanup, test fixtures). Kept as the style rule `poisoned-mutex-recovery` |
| Prefer `enum` over `bool` parameters | readability | the study measured bool parameters: OR 1.24 [1.04, 1.48] discovery, 1.02 [0.81, 1.29] replication, not replicated. Style rule `bool-parameter` |
| Avoid `.clone()` as a substitute for borrowing | cost | the study found no detectable effect of `.clone()` counts; needs types to check, so no rule |
| `#[non_exhaustive]`, `const fn` only if it stays `const`, `#[must_use]` | API evolution: a later change breaks callers | library-design rules about who the callers are; the syntactic parts are style rules (`non-exhaustive-missing`, `must-use-missing`) |
| No unstable features; no code before `main()` | none given | no failure to reproduce; style rules `unstable-feature` and `code-before-main` (0 and 0 instances in the corpus's shipping code) |
| Imports, naming, macros, `Rc`/`Arc`, iterators, documentation, formatting, assertions, type inference | readability, consistency, tooling, allocation cost | style rules, each with the guide's listed exceptions, in `docs/STYLE.md` |
| Rules tagged [chrome] or [Fuchsia] (small crates, flat layout, no `async`, no std threads, `fuchsia_sync::Mutex`, test frameworks) | one codebase's build policy | not rules: the guide itself scopes them to one codebase |

### Rules dropped after the audit

| Candidate | Why it was dropped |
|---|---|
| `unused-io-amount` (`w.write(buf)?` dropping the count) | The failure is real: one 8 MiB `write` to a non-blocking Unix socket took only part of it. But 0/10 audit findings were `io::Write`; all were custom `write` methods, which syntax can't tell apart. Clippy's type-aware `unused_io_amount` (deny by default) already covers it. |
| `mem-forget` | Leaks verified: a forgotten guard holds its lock forever and forgotten files keep their descriptors. But 10/10 audit uses were deliberate ownership hand-offs in expert code. |
| `unimplemented!()` as unfinished code | The std docs distinguish it from `todo!()`: it makes no claim the code will be written. In the audit it was used that way (unsupported features, mocks). antislop flags it as critical; this tool doesn't. |
| Complexity, length, nesting, parameter rules | The defect study, above. `anti-slop metrics` still reports the numbers. |

## Threats to validity

- **Fix classification.** Fixes are commits whose subject says fix, bug, crash, panic,
  regression, wrong and so on, minus docs, CI and typo commits. A hand check of 50 found 33 real
  defect fixes (66%, 95% CI 51% to 79%); that sample wasn't kept, so it can't be re-checked. The
  review's own random 40 included at least 6 non-fixes ("fix tests", a feature). Exclusion words
  apply to the whole subject, so some real fixes are dropped, and fix-titled merge commits are
  skipped. Most misses touched only tests, examples or docs, which the study excludes.
  Among commits touching other code, about 80% were fixes. For comparison, Herzig et al. (ICSE
  2013) hand-checked over 7,000 issue reports and found about a third weren't bugs.
- **Function identity** is (file path, qualified name), and the preregistered miner didn't
  follow renames. That isn't a uniform bias: tauri's repository restructure hid most of its
  fixes (85 fixed functions, 521 with renames followed), while clap's pure moves counted every
  moved function as fixed. Pure insertions just after a function were also credited to it, and
  snapshots weren't always on the mainline (meilisearch's was on a release branch). Correcting
  all of these changes 47% of the replication's outcomes and 12% of discovery's. The corrected
  results are in `study/sensitivity.md`; the conclusions above hold under both. Functions with a
  duplicate name in a file (2% to 11%, mostly small trait impls) are dropped, and a nested
  function's edits are credited only to itself.
- **Test code.** The study was mined with a build from before the audit fix for test modules in
  their own files, so about 2,000 rows it treats as shipping code are tests. Dropping them barely
  moves the estimates.
- **Model.** Standard errors clustered by file ignore correlation within a repository; clustered
  by repository, discovery's nesting and parameter-count effects fall to p ≈ 0.08. One log-size
  term doesn't fit the saturating outcome, which is why long functions look protective.
- **Outcome.** "Touched by a later fix" is fix-proneness, not proof of fault. A function can be
  edited to fix a bug that lives elsewhere.
- **Power.** The replication had about 37% power to confirm H1 and H2 at the Holm threshold if
  the discovery effects were true, and the null hypotheses H3 to H7 passed whenever an interval
  contained 1, which a small sample makes easy. So "held" means no detectable effect, not
  evidence of none; only cyclomatic and cognitive complexity pass an equivalence test (90% CI
  inside 0.9 to 1.1), and only pooled with the corrected miner. Rare patterns (deferral and
  hedging phrases, `panic-in-drop`) have wide intervals.
- **Audit population.** The audit judged mature, already-reviewed code. Google found developers
  rated 74% of compile-time findings real problems against 21% of findings in checked-in code
  (Sadowski et al., CACM 2018), so precision on new code is likely higher than measured here.
- **Audit.** The validation sample was judged by two raters: the author, and an independent agent
  blind to these docs that traced callers. Agreement was moderate (80%, κ = 0.54). Where they
  disagreed, the rater who traced the code was usually right, and its verdicts were taken. Samples
  are 10 to 20 findings per rule, so a figure like 5/20 means 9% to 49%; `todo`,
  `ignored-io-error` and `lock-held-across-await` were judged in full instead. Rater 2 is a Claude
  agent like rater 1, so they aren't fully independent. Neither rater
  ran the code. Everything needed to re-judge is in [AUDIT.md](AUDIT.md).

## Sources and how they were checked

Every web page cited by a rule or in these docs was fetched on 2026-10-26 (2026-10-27 for Google's
guide, the Cargo Book and the `catch_unwind` and `Mutex` pages), and the claim it is cited for was
found in its text. `study/check_sources.py` repeats the check (a regex per claim)
and exits non-zero if any claim has gone missing: 38 claims on 25 pages plus the six Clippy lint groups,
none missing. Two corrections came out of that check. The Book's "in production-quality code,
most Rustaceans choose `expect` rather than `unwrap`" is in chapter 9.2, not 9.3. And Rust 2024
makes `static_mut_refs` *deny by default*, a lint that can be allowed, not a hard error.
Papers were checked against their publisher records (Crossref DOIs below). The Yuan et al. and
Tricorder figures were checked against the papers' PDFs, and the Cloudflare quote against the
post-mortem.

## Candidates not yet tested

Well-sourced patterns that a syntax check might catch, raised by the source reviews
(`docs/research/`). None has been through the experiments or the audit, so none is a rule yet.

- **Double locking** (`m.lock()` twice in one scope, or a guard kept alive by a `match` or
  `if let` scrutinee while the same lock is taken again). Qin et al. (PLDI 2020) found "30 of
  [38]" lock-related blocking bugs in real Rust programs were double locks.
- **Inconsistent clone edits.** Juergens et al. (ICSE 2009) found unintentionally inconsistent
  changes to cloned code frequently led to faults. Needs history, not one snapshot.
- **`pub unsafe fn` without a `# Safety` doc section** (Clippy `missing_safety_doc`, on by default).
- **`let _ = <future>`** (Clippy `let_underscore_future`): the future is dropped unpolled.
- **References taken to a `static mut`**, the precise construct the Edition Guide calls instant UB.
- **`pub fn` returning `Result<_, ()>`** (API Guidelines C-GOOD-ERR).

## References

- McCabe, T. J. A Complexity Measure. IEEE TSE 2(4), 1976. doi:10.1109/TSE.1976.233837
- Campbell, G. A. Cognitive Complexity (SonarSource white paper, 2018). <https://www.sonarsource.com/docs/CognitiveComplexity.pdf>
- El Emam, K., Benlarbi, S., Goel, N., Rai, S. N. The confounding effect of class size on the validity of object-oriented metrics. IEEE TSE 27(7), 2001. doi:10.1109/32.935855
- Herraiz, I., Hassan, A. E. Beyond lines of code: do we need more complexity metrics? Ch. 8 of *Making Software*, O'Reilly 2010. <https://www.oreilly.com/library/view/making-software/9780596808310/ch08.html>
- Landman, D., Serebrenik, A., Bouwers, E., Vinju, J. Empirical analysis of the relationship between CC and SLOC in a large corpus of Java methods and C functions. J. Software: Evolution and Process 28(7), 2016. doi:10.1002/smr.1760
- Mockus, A., Votta, L. Identifying reasons for software changes using historic databases. ICSM 2000. doi:10.1109/ICSM.2000.883028
- Herzig, K., Just, S., Zeller, A. It's not a bug, it's a feature: how misclassification impacts bug prediction. ICSE 2013. doi:10.1109/ICSE.2013.6606585
- Evanco, W. M. Comments on "The confounding effect of class size on the validity of object-oriented metrics". IEEE TSE 29(7), 2003. doi:10.1109/TSE.2003.1214331
- Muñoz Barón, M., Wyrich, M., Wagner, S. An Empirical Validation of Cognitive Complexity as a Measure of Source Code Understandability. ESEM 2020. doi:10.1145/3382494.3410636
- Wehaibi, S., Shihab, E., Guerrouj, L. Examining the impact of self-admitted technical debt on software quality. SANER 2016. doi:10.1109/SANER.2016.72
- Juergens, E., Deissenboeck, F., Hummel, B., Wagner, S. Do code clones matter? ICSE 2009.
- Qin, B. et al. Understanding Memory and Thread Safety Practices and Issues in Real-World Rust Programs. PLDI 2020.
- Sadowski, C., Aftandilian, E., Eagle, A., Miller-Cushon, L., Jaspan, C. Lessons from Building Static Analysis Tools at Google. CACM 61(4), 2018. doi:10.1145/3188720
- Yuan, D. et al. Simple Testing Can Prevent Most Critical Failures. OSDI 2014. <https://www.usenix.org/conference/osdi14/technical-sessions/presentation/yuan>
- Sadowski, C., van Gogh, J., Jaspan, C., Söderberg, E., Winter, C. Tricorder: Building a Program Analysis Ecosystem. ICSE 2015. doi:10.1109/ICSE.2015.76
- Cloudflare. Cloudflare outage on November 18, 2025. <https://blog.cloudflare.com/18-november-2025-outage/>
- The Rust Book, ch. 9.2 and 9.3; the Rust Reference; std docs for `MaybeUninit`, `mem::uninitialized`, `process::exit`, `BufWriter`, `Drop`, `todo!`, `unimplemented!`, `dbg!`; the Edition Guide on `static mut` references; Rust API Guidelines C-DTOR-FAIL; RFC 1105; Rust Design Patterns, `#![deny(warnings)]`; Tokio's `Mutex` docs and its section on blocking code; the std-dev guide on safety comments; CWE-252 and CWE-197; the Clippy lint list. Links are on each rule: `anti-slop explain <rule>`.
