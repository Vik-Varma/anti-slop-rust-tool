# Validation

Each default rule in anti-slop had to survive at least one of the checks below.
This directory holds the code that runs each check and the results it produced.
The per-rule verdicts are in [`docs/EVIDENCE.md`](../docs/EVIDENCE.md).

| Check | What it shows | Code | Results |
|---|---|---|---|
| Demonstrations | The failure a rule predicts really happens (deadlock, stall, abort, crash, data loss, broken build), and the recommended fix avoids it | [`tests/demonstrations.rs`](../tests/demonstrations.rs) | [`results/demonstrations.txt`](results/demonstrations.txt) |
| Miri | The `unsafe` patterns are undefined behaviour or a data race, and the fixed versions are clean | [`ub/`](ub) | [`results/miri.txt`](results/miri.txt) |
| Benchmarks | Performance rules have a measurable cost; some popular claims do not | [`benches/`](benches) | [`results/benchmarks.md`](results/benchmarks.md) |
| Mining | Whether a rule or metric predicts later bug fixes once function size is controlled for | [`mining/`](mining) | [`results/mining.md`](results/mining.md), [`results/mining_repos.md`](results/mining_repos.md), [`results/fix_commit_sample.md`](results/fix_commit_sample.md) |
| Precision review | How often a finding on real code is correct | [`precision/`](precision) | [`docs/EVIDENCE.md`](../docs/EVIDENCE.md#precision-on-real-code) |

## Reproducing

```sh
# Demonstrations (part of the normal test suite)
cargo test --test demonstrations -- --nocapture

# Undefined-behaviour checks (needs a nightly toolchain with Miri)
rustup toolchain install nightly --component miri
validation/ub/run.sh

# Benchmarks
cargo bench --manifest-path validation/benches/Cargo.toml

# Mining: clone repositories with full history into one directory, then
cargo build --release
python3 validation/mining/mine.py --tool target/release/anti-slop --repos-dir REPOS --out OUT \
    --snapshot 2023-07-01 --window-end 2025-07-01
python3 -m venv venv && venv/bin/pip install pandas numpy scipy statsmodels
venv/bin/python validation/mining/analyze.py OUT --markdown mining.md
python3 validation/mining/fix_commit_sample.py REPOS   # sample for checking the fix classifier
```

The 22 repositories mined are listed in
[`results/mining_repos.md`](results/mining_repos.md).

## Mining method

The design follows the size-controlled approach that the empirical literature
recommends (El Emam et al., IEEE TSE 2001). Without it, any metric that grows
with code size looks like a defect predictor.

1. **Snapshot.** For each repository, take the last commit on the default
   branch's first-parent history before 2023-07-01. Compute per-function
   metrics and anti-slop findings at that snapshot.
2. **Outcome.** Walk every first-parent commit up to 2025-07-01, skipping
   commits that touch more than 30 Rust files. Map the changed lines (the
   parent side of the diff) onto the functions of the parent revision.
   Record whether each function was changed, and whether it was changed by a
   **bug-fix commit**. A bug-fix commit is identified by keywords (fix, bug,
   crash, panic, regression, deadlock, …), excluding docs/CI/typo/release
   commits and bots. Functions are matched across revisions by file path and
   qualified name.
3. **Population.** Production functions only: not tests, benches or examples,
   not in `#[cfg(test)]` code, and not ambiguous names (e.g. `cfg` variants).
   This gives 40,320 functions, of which 4,388 were later touched by a fix.
4. **Models.** For each rule or threshold flag X, fit a logistic regression
   `fixed ~ log(1+LOC) + X + repository` with standard errors clustered by
   file. Also compute a Mantel-Haenszel odds ratio stratified by repository ×
   LOC bin (quarter-steps of log2 LOC), as a non-parametric check. Correct
   p-values with Holm's method across all predictors. Continuous metrics are
   also tested per standard deviation.
5. **Secondary analysis.** Repeat among functions that changed at all, which
   asks whether a change to a flagged function is more likely to be a fix.
   This separates "buggy" from "frequently edited".
6. **Sharper test for `undocumented-unsafe`.** Compare undocumented with
   documented functions, among only the functions that contain `unsafe`.

### Threats to validity

- **Noisy outcome.** In a manual check of 30 commits per class, 60% of
  keyword-classified fix commits were defect fixes and 10% of non-fix commits
  were ([`results/fix_commit_sample.md`](results/fix_commit_sample.md)). This
  noise is unrelated to the code metrics, so it pulls odds ratios towards 1.
  Weak real effects may therefore be missed. The effects we report as
  supported are, if anything, underestimated.
- **Identity by name.** Functions that are renamed or moved between the
  snapshot and the fix are lost. This also biases results towards no effect.
- **Mature projects only.** These are well-maintained, popular crates. Some
  patterns are rare there (e.g. `lock-in-scrutinee`, `set-len-uninit`), so the
  mining cannot test them. Those rules rely on demonstrations instead.
- **Association, not causation.** For example, TODO-marked functions are
  partly fixed more often because they change more often (see the secondary
  analysis).
- **Syntactic detection.** Rules work without type information. The precision
  review measures how often this misleads them.

## Precision review

To check precision, findings of the default rules were sampled at random
(fixed seeds) from the 22 repositories' current code. Reviewers labelled each
one TP, TP-intentional or FP against the rule's written definition. Each
reviewer saw only the rule text and the code, never the other reviewer's
labels. These reviewers were independent model sessions, not humans: they had
no access to this tool's design discussion.

- **Round 1:** 155 findings, two reviewers. They agreed on 151/155 labels
  (Cohen's κ = 0.95). The false-positive classes they found were then fixed in
  the tool.
- **Round 2:** 64 *new* findings drawn after the fixes, one reviewer. This
  estimates precision on findings the fixes were not tuned against.

Items and labels are in [`precision/`](precision).
