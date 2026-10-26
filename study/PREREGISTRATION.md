# Replication plan, fixed before the replication data was mined

Written after the discovery analysis (`study/results.md`, 20 repositories in `study/repos.txt`)
and before any data from the replication corpus (`study/repos-replication.txt`) was collected.
The commit that adds this file precedes the commit that adds `study/data-replication/`.

## Method (unchanged from discovery)

Same `mine.py` (subject-only fix classification, snapshot 2024-07-01, outcome window to
2026-10-01, prior churn over the year before the snapshot) and same `analyze.py`. Same anti-slop
binary build. No new variables, thresholds or transforms.

## Hypotheses

Primary test for each: the **+ churn** model (repo + log LOC + log prior churn + pattern),
two-sided, file-clustered SEs, Holm-adjusted across the hypotheses below only (7 tests).

| # | Pattern | Prediction in the replication corpus |
|---|---|---|
| H1 | `f_debt_comments` (TODO/FIXME/HACK/XXX inside the function) | OR > 1, adjusted p < 0.05 |
| H2 | `max_nesting` | OR > 1, adjusted p < 0.05 |
| H3 | `cyclomatic` | no effect: the 95% CI contains 1, or OR < 1.10 |
| H4 | `cognitive` | no effect: the 95% CI contains 1, or OR < 1.10 |
| H5 | `long-function` (> 100 code lines) | no effect or OR < 1 |
| H6 | `complex-function` (cognitive > 15) | no effect: the 95% CI contains 1 |
| H7 | `unwrap` | no effect on fix-proneness: the 95% CI contains 1 |

H7 does not test whether `unwrap` is bad (that rests on the panic mechanism). It tests whether it
also marks fix-prone code, which the discovery data says it doesn't.

## What follows from each outcome

- H1 holds: `debt-comment` becomes a rule (note severity). Fails: it is reported as not replicated
  and does not become a rule.
- H2 holds: `deep-nesting` stays a rule (note). Fails: it is removed.
- H3 to H6 hold: the size and complexity rules are removed from the default rule set, with the
  numbers in docs/EVIDENCE.md. Any that fails (a real effect appears) is kept as a note, and the
  discrepancy is reported.
