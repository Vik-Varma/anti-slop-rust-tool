# Replication: the preregistered test

26746 non-test functions from 15 repositories not used in discovery; 2010 (7.5%) changed by a later bug fix.

| | pattern | functions with it | OR [95% CI] | Holm p | prediction | held? |
|---|---|---:|---|---:|---|---|
| H1 | f_debt_comments | 294 | 1.19 [0.85, 1.66] | 1.000 | OR > 1, adjusted p < 0.05 | **no** |
| H2 | max_nesting | 12023 | 1.11 [0.97, 1.27] | 0.809 | OR > 1, adjusted p < 0.05 | **no** |
| H3 | cyclomatic | 26746 | 1.07 [0.95, 1.20] | 1.000 | CI contains 1, or OR < 1.10 | yes |
| H4 | cognitive | 9998 | 1.04 [0.97, 1.12] | 1.000 | CI contains 1, or OR < 1.10 | yes |
| H5 | long-function | 291 | 0.74 [0.51, 1.08] | 0.809 | no effect, or OR < 1 | yes |
| H6 | complex-function | 623 | 0.98 [0.74, 1.29] | 1.000 | CI contains 1 | yes |
| H7 | unwrap | 1147 | 1.01 [0.77, 1.32] | 1.000 | CI contains 1 | yes |
