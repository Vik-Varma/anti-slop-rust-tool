# Review of the anti-slop defect study and write-up

Repository: /home/user/anti-slop-rust-tool at 411ba4c. Nothing in it was edited. Every number below was recomputed from the committed data or from the corpus clones, using scripts in scratch/review-study/:

- **mine_variant.py** re-mines all 35 repos with the 18c520c build. For every repo it first reproduces `later_fixes` exactly (orig_mismatch=0 in all 35), then computes corrected outcomes. Output is in `var/data*/*.csv` and `var/log.txt`.
- **rerun.py** fits the preregistered model on each corrected outcome. Output is in `rerun.txt`.
- **pooled.py** does the pooled fit, the repo- and directory-clustered fits, and the power calculation.
- **verify_fit.py** compares IRLS with scipy BFGS and an independent sandwich estimator.
- **inspect_attr.py, summ_attr.py, subjects.py, fixsample.py** are the commit-level spot checks.

Findings are ranked by how much each could change a stated conclusion.

## 1. "No significant effect" is reported as "no effect", and the replication could not have confirmed H1/H2

**Where.** README.md ("TODO/FIXME comments all predict bug fixes only because they measure size. Once size is controlled for, none of them adds anything, and that held up in a preregistered replication"). DEFINITION.md ("none adds anything, and the result held…"). EVIDENCE.md verdicts ("no effect", "no effect beyond size").

**Power.** Take the replication's own standard errors and suppose the discovery effect is exactly true. Holm over 7 tests means roughly α = 0.05/7 for the smallest p.

| Hypothesis | Discovery b | Replication SE | Power at α = 0.05/7 | Power at α = 0.05 |
|---|---|---|---|---|
| H1 `f_debt_comments` | 0.570 | 0.244 | 0.36 | 0.65 |
| H2 `max_nesting` | 0.234 | 0.097 | 0.39 | 0.68 |

Winner's curse makes the true power lower still.

**Agreement.** The discovery and replication estimates differ by z = 1.14 (p = 0.25) for H1 and z = 0.69 (p = 0.49) for H2.

**Pooled 35 repos** (same model, file-clustered):

| Pattern | OR [95% CI] | p |
|---|---|---|
| TODO/FIXME comments | 1.41 [1.20, 1.66] | 2.7e-5 |
| max_nesting | 1.15 [1.07, 1.24] | 2.4e-4 |
| params | 1.10 [1.03, 1.18] | 0.004 |
| f_comment_lines | 1.13 [1.08, 1.18] | 5e-9 |
| cognitive | 1.05 [1.01, 1.10] | 0.017 |

In discovery alone:
- TODO is significant after size and churn (Holm p < 0.001).
- Nesting is significant after size and churn (Holm p = 0.014).
- params is significant after size (Holm p = 0.014).

**The null criteria could hardly fail.**
- H3 and H4 pass if "the CI contains 1 or OR < 1.10". So a significant 1.08 [1.05, 1.09] would pass, and so would any wide interval.
- H5 to H7 pass whenever the CI contains 1.

The replication's upper bounds are:
- cyclomatic: 1.20 per doubling
- complex-function: 1.29
- unwrap: 1.32

An equivalence test (90% CI inside 0.9 to 1.1) fails for every one; cyclomatic's 90% CI is [0.96, 1.18].

**Asymmetry.** The positive hypotheses needed Holm p < 0.05. The nulls only needed an unadjusted CI containing 1.

**Fix.**
- Call H1/H2 "inconclusive (power about 37%), consistent with discovery" and report the pooled estimates.
- Write "no detectable effect, upper bound X" instead of "no effect".
- Use equivalence testing and a power analysis in future preregistrations.
- Remove TODO, nesting and params from the README's "only measure size" sentence.

## 2. File moves erase fixes and invent others; about 47% of the replication outcome changes

**Where.** mine.py `touched_functions` uses `--no-renames` and identifies functions by (path, name). EVIDENCE.md's threat says "biases every effect towards zero equally… Meilisearch… clearest case."

**tauri.** Commit 36eee3722, "Restructure the repository (#10796)", 2024-08-27, moved `core/` to `crates/`, which holds 2,020 of the 2,569 snapshot functions.
- Example: `ExitRequestApi::prevent_exit` was changed by b05f82d35, "fix: ensure RunEvent::Exit is triggered on restart (#12313)", 2025-03-15 (verified with `git log -L`). The data has later_fixes=0.
- Following renames, tauri goes from 85 fixed functions (3.3%) to 521 (20.3%).

**Other repos** when renames are followed:
- meilisearch: 40 to 176
- typst: 168 to 236
- mdBook: 6 to 16

**The opposite error: clap.**
- Commit 6727c1537b, "fix(complete): Section off existing completions", is a pure move (`clap_complete/src/{ => aot}/…`, 0 changed lines on 9 of 11 files).
- With rename detection off, every line of the old files counts as deleted, so 60 functions are recorded as fixed: 47% of clap's 128.
- Whole-file deletions in fix-titled commits do the same: dropping them takes gitoxide from 996 to 832 fixed functions.

**Scale.** With renames followed, boundary inserts dropped and whole-file deletes dropped:
- replication: 946 functions change (691 gained, 255 lost), 47% of the 2,010 published;
- discovery: 418 functions change (12%).

The table at the top shows the effect on the preregistered tests.

**Fix.**
- Use `git log -M`, map each path through its rename chain, and never count a whole-file deletion or rename as a change.
- Re-mine both corpora.
- Report the result as a sensitivity analysis next to the preregistered one.

## 3. Pure insertions just outside a function are attributed to it

**Where.** mine.py `parse_log` adds lines [s, s+1] for `@@ -s,0`. So code appended after a closing brace, or inserted just before the next function, counts as a change to that neighbour.

**Case.** fd d3a500af7a, "fix: handle invalid working directories gracefully…". The hunk `@@ -686,0 +713,26 @@` appends a `#[cfg(test)] mod tests` after `scan` (lines 684–686), and `scan` is recorded as fixed.

**Scale.**
- Functions fixed only through such an insertion: 214 in discovery (6.2% of fixed), 99 in replication (4.9%).
- zellij alone: 717 to 624.
- Correcting this alone moves discovery H2 from 1.18 (Holm 0.003) to 1.14 (Holm 0.042), and long-function from 0.66 to 0.59.

**Fix.** Count an insertion only when start ≤ s and s+1 ≤ end.

## 4. The clustered standard errors ignore correlation across files within a commit

Discovery, churn model:

| Clustering | Nesting p | params p | TODO p |
|---|---|---|---|
| By file (as published) | 0.00065 | 0.0038 | |
| By directory | 0.011 | 0.015 | |
| By repo (G = 20) | 0.078 | 0.088 | 7e-5 |

In the replication, clustering by repo moves nesting to p = 0.052.

This cuts both ways. Some discovery "hits" may be chance, and the replication's "no effect" intervals are too narrow.

**Fix.** Add a wild-cluster bootstrap by repo, or two-way clustering by file and commit.

## 5. The most consistent positive result is never reported: comment lines inside a function

| | OR [95% CI] | p | Repos with a positive effect |
|---|---|---|---|
| Discovery | 1.13 [1.08, 1.19] | Holm < 0.001 | 17/20 |
| Replication | 1.11 [1.02, 1.19] | unadjusted 0.010 | 11/14 |
| Replication, changed functions only | 1.19 [1.10, 1.29] | Holm < 0.001 | |
| Pooled | | 5e-9 | |

It survives every outcome correction, and none of README.md, DEFINITION.md or EVIDENCE.md mentions it.

**Fix.** Report it as exploratory and make it a candidate for the next confirmatory test.

## 6. Exploratory numbers are labelled preregistered

- EVIDENCE.md's "Replication (preregistered)" column includes parameter count (1.06) and more than 7 parameters (0.77). Neither was among H1–H7.
- README.md and DEFINITION.md say the null "held up" in the replication for many parameters and magic numbers, which it never tested, and for deep nesting, where H2 predicted an effect.

**Fix.** Relabel those rows as exploratory, and limit the "preregistered" claim to H3–H7.

## 7. The size control doesn't fit

**Long functions come out significantly protective:**
- discovery 0.66 [0.53, 0.82], Holm 0.005
- pooled 0.69, p = 1.6e-4
- corrected outcome 0.59

EVIDENCE.md calls this "no effect".

**Why.** The outcome is yes/no and saturates: P(fixed) is 0.71 for 200+ LOC in discovery (0.50 in replication), against 0.02 under 5 LOC. A single log-LOC term mis-fits the top of the range, so the threshold rules pick up residual size.

**Per-line claim.** "Not more fixes per line" (DEFINITION.md) is never estimated. Raw fixes per 100 lines are 0.62–0.84 in discovery and fall from 1.08 to 0.32 in replication.

**Fix.** Use a LOC spline, or a count model with offset log(LOC).

## 8. The snapshot isn't always on the mainline

`rev_before` picks the snapshot without `--first-parent`.

**meilisearch.**
- The snapshot 0df84bbba ("Merge #4746") is on a release branch: 126 commits ahead of, and 54 behind, the mainline commit at that date (809e742253).
- The "later" window therefore counts 31 mainline `.rs` commits dated before the snapshot, including "Fix the missing geo distance…" (2024-06-26).

**Smaller cases:** fd (4 behind, 5 ahead), diesel (7 ahead), gitoxide (1 ahead).

**Fix.** Add `--first-parent` when choosing the snapshot, end and prior commits.

## 9. The fix classifier

- **No artifact.** EVIDENCE.md's "hand check of 50 found 33 real defect fixes (66%)" has no file behind it; 33/50 has a 95% CI of 51% to 79%.
- **A random 40** of the fix-classified subjects includes at least 6 non-fixes:
  - "Fix review remarks"
  - "fix tests"
  - "Fix old test"
  - "fix(tests): rename…"
  - "Fix the insta snapshots"
  - "fix(ui): add … shortcut hints" (a feature)
- **Tangled change.** clap 0e6ba89ff2 ("Improve invalid attribute errors") only lowercases an error string, yet counts as a fix of the 477-line `Item::push_attrs`.
- **Exclusion words apply to the whole subject.** 1,088 matching subjects are excluded, including real fixes:
  - `fix(builtin-deps): Reject builtin…` (cargo)
  - `fix(diag): Don't report unused normal deps…` (cargo)
  - `fmt: fix ANSI escape sequence injection vulnerability` (tracing)
- **Fix-titled PR merges are skipped.** First-parent merges with fix subjects: cargo 334, gitoxide 157, meilisearch 82, diesel 74, mdBook 45. Counting them changes little (replication H1 1.37, H2 1.15).

**Fix.**
- Commit the hand-check sample.
- Apply the exclusion words only to the commit type, not the scope.
- Add the merge-title variant as a sensitivity check.
- Try excluding test-only commits.

## 10. Test-code classification

- **Study data.** The 18c520c build predates the audit fix for test modules in their own files. The current build calls 1,171 discovery and 812 replication "non-test" rows test code, for example:
  - actix-web: 167
  - cargo: 388
  - tauri: 190
  - rustup: 131

  Dropping them barely moves the estimates. But it contradicts EVIDENCE.md's statement that the study excludes test code.
- **Audit.** 6 of the 7 `lock-held-across-await` findings are in zellij's `web_client/unit/web_client_tests.rs`. That file is included via `#[cfg(test)] #[path = "./unit/web_client_tests.rs"] mod web_client_tests;` (mod.rs:489–491), and the final build still doesn't honour `#[path]`. Findings #2 and #5 are the same line. So "0/7 could deadlock" rests on one shipping finding (rustup).

**Fix.** Handle `#[path]` on `#[cfg(test)]` modules, and re-audit that rule.

## 11. The audit samples are too small for the 50% severity gate

Clopper-Pearson 95% intervals:

| Rule | Settled | 95% CI |
|---|---|---|
| todo | 4/10 | 12% to 74% |
| ignored-io-error | 4/10 | 12% to 74% |
| panic-on-input | 5/20 | 9% to 49% |
| panic-in-drop | 1/10 | 0.3% to 44% |

- For `todo` and `ignored-io-error` the sample can't tell which side of the 50% gate the rule is on, so their severity is set by noise.
- EVIDENCE.md's "roughly 10% to 45%" for 5/20 understates the upper bound.
- Rater 2 is a Claude agent, like the author, so the raters aren't independent. The agreement figures themselves reproduce: 80%, κ 0.54, and per rule 0.67, 1.00, 0.41, 0.44 and 0.00.
- Settling disagreements for rater 2, with one exception chosen by rater 1, is itself an unblinded judgement.

**Fix.** At least 30 findings per rule for rules near the gate, a human second rater, and the intervals printed next to each severity.

## 12. Numeric and labelling slips in the docs

- **"Every metric above correlates with size (Spearman ρ 0.40 to 0.73)".** The threshold rules in that table are 0.08 to 0.28 (too-many-params 0.08/0.09, long-function 0.21/0.18, complex-function 0.28/0.26). Parameter count is 0.50/0.47. 0.40 is `try_ops`, which isn't in the table.
- **"Odds ratios of 2 to 16".** Cognitive complexity's naive OR is 1.74 (1.66 in replication).
- **"Each doubling of a function's lines doubles its odds (OR 2.13)".** It is per doubling of 1+LOC, and it is the discovery value; replication is 1.92. analyze.py labels it "per doubling of LOC".
- **AUDIT.md's "Findings in corpus" mixes builds.** panic-in-drop is listed as 42 (the current build finds 39), and len-truncating-cast as 442 (from the round-1 build; the current build finds 323). The other counts match the current build.
- **The study's `unwrap` and `unsafe-without-safety-comment` columns** come from the pre-audit rule definitions, when `unwrap` still included what is now `panic-on-input`. EVIDENCE.md attaches those ORs to the current rules without saying so.

## 13. Minor

- **Uncommitted data.** The `f_deferral` and `f_hedging` columns behind antislop-phrases.md aren't committed. hedge.py rewrites `study/data*/*.csv` in place, which then grows analyze.py's Holm family and shifts other p-values (TODO changed-only: 0.390 to 0.431). The tables otherwise reproduce.
- **Duplicate-name drops.** 2% to 11% of non-test functions are dropped for duplicate names (clap 11.3%, tokio 4.9%). They are mostly small trait impls (`from`, `fmt`, `eq`) and cfg-gated platform variants, averaging 4 to 10 LOC against 8 to 24 for kept functions. The threats section doesn't mention this.
- **Nested functions.** They are named as if they were siblings (`S::helper`). Their lines count toward the outer function's metrics, but edits to them are credited only to the inner function.
- **Dead code.** In mine.py `main`, the first `prior = …` assignment is overwritten on the next lines.
