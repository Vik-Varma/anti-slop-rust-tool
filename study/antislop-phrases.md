# Exploratory: antislop's deferral and hedging phrases

Not preregistered: added after the replication, when antislop (crates.io, 0.3.0) came up. The phrase lists are its `antislop-standard` profile, matched against comment lines inside each function (`hedge.py`). The counts are in `study/phrases-*.csv` (only functions with at least one phrase; every other function has zero); join them onto `study/data*/` by repo, path and function to re-run. Same models and columns as `results.md`; TODO/FIXME (`f_debt_comments`) repeated for comparison.

## data

| pattern | functions with it | fix rate with / without | ρ with LOC | naive OR | p | size-adjusted OR | p | + churn OR | p | ΔAUC | consistency |
|---|---:|---|---:|---|---:|---|---:|---|---:|---:|---:|
| f_debt_comments | 698 | 35.2% / 7.3% | 0.15 | 4.68 [3.90, 5.62] | <0.001 | 1.60 [1.34, 1.92] | <0.001 | 1.48 [1.23, 1.79] | <0.001 | +0.0003 | 8/11 |
| f_deferral | 135 | 30.4% / 7.6% | 0.07 | 3.64 [2.50, 5.31] | <0.001 | 0.99 [0.68, 1.46] | 1.000 | 1.00 [0.68, 1.47] | 1.000 | +0.0000 | 2/4 |
| f_hedging | 113 | 20.4% / 7.7% | 0.06 | 3.71 [2.37, 5.79] | <0.001 | 0.88 [0.52, 1.49] | 1.000 | 0.87 [0.50, 1.52] | 1.000 | +0.0000 | 2/5 |

## data-replication

| pattern | functions with it | fix rate with / without | ρ with LOC | naive OR | p | size-adjusted OR | p | + churn OR | p | ΔAUC | consistency |
|---|---:|---|---:|---|---:|---|---:|---|---:|---:|---:|
| f_debt_comments | 294 | 24.5% / 7.3% | 0.13 | 3.46 [2.57, 4.67] | <0.001 | 1.23 [0.89, 1.72] | 1.000 | 1.19 [0.85, 1.66] | 1.000 | +0.0003 | 5/8 |
| f_deferral | 50 | 26.0% / 7.5% | 0.06 | 4.28 [2.18, 8.41] | <0.001 | 1.25 [0.62, 2.52] | 1.000 | 1.15 [0.58, 2.31] | 1.000 | +0.0000 | 1/2 |
| f_hedging | 47 | 14.9% / 7.5% | 0.06 | 2.35 [1.03, 5.36] | 0.086 | 0.61 [0.29, 1.27] | 1.000 | 0.57 [0.25, 1.27] | 1.000 | +0.0001 | 0/0 |

Reading: all three are strongly associated with later fixes until function size is controlled for. After that, the deferral and hedging phrases are indistinguishable from no effect in both corpora; TODO/FIXME keeps an effect in discovery that didn't replicate (H1 in `replication.md`). Few functions contain the phrases, so the intervals are wide: a modest effect isn't ruled out, but there is no evidence for one.
