# Sensitivity analysis (not preregistered)

Added after the study review. The preregistered result is `study/replication.md` and stands as published. Model: repo + log(1+LOC) + log(1+prior churn) + pattern. OR per doubling of (1 + metric) for metrics and counts, or for having the pattern at all. p is unadjusted. ≈ marks an equivalence pass: the 90% CI lies inside [0.9, 1.1]. "Corrected miner" follows renames, ignores pure moves and whole-file deletions, credits an insertion only to a function it lands inside, and chooses snapshots on the mainline.

## as published

Discovery: 44961 functions, 3463 fixed. Replication: 26746 functions, 2010 fixed.

| | pattern | discovery, file-clustered | replication, file-clustered | pooled, file-clustered | pooled, repo-clustered |
|---|---|---|---|---|---|
| H1 | f_debt_comments | 1.48 [1.23, 1.79] p=<0.001 | 1.19 [0.85, 1.66] p=0.304 | 1.41 [1.20, 1.66] p=<0.001 | 1.41 [1.20, 1.67] p=<0.001 |
| H2 | max_nesting | 1.18 [1.07, 1.29] p=<0.001 | 1.11 [0.97, 1.27] p=0.116 | 1.15 [1.07, 1.24] p=<0.001 | 1.15 [1.01, 1.32] p=0.038 |
| H3 | cyclomatic | 1.01 [0.94, 1.08] p=0.841 ≈ | 1.07 [0.95, 1.20] p=0.289 | 1.03 [0.97, 1.09] p=0.342 ≈ | 1.03 [0.92, 1.15] p=0.600 |
| H4 | cognitive | 1.05 [1.00, 1.11] p=0.043 ≈ | 1.04 [0.97, 1.12] p=0.277 | 1.05 [1.01, 1.10] p=0.017 ≈ | 1.05 [0.97, 1.14] p=0.208 |
| H5 | long-function | 0.66 [0.53, 0.82] p=<0.001 | 0.74 [0.51, 1.08] p=0.116 | 0.69 [0.57, 0.84] p=<0.001 | 0.69 [0.56, 0.86] p=<0.001 |
| H6 | complex-function | 0.89 [0.75, 1.07] p=0.229 | 0.98 [0.74, 1.29] p=0.866 | 0.93 [0.80, 1.08] p=0.344 | 0.93 [0.77, 1.12] p=0.445 |
| H7 | unwrap | 0.97 [0.78, 1.20] p=0.750 | 1.01 [0.77, 1.32] p=0.935 | 0.98 [0.82, 1.16] p=0.781 | 0.98 [0.85, 1.12] p=0.737 |
|  | params | 1.13 [1.04, 1.22] p=0.004 | 1.06 [0.95, 1.19] p=0.282 | 1.10 [1.03, 1.18] p=0.004 | 1.10 [1.00, 1.21] p=0.048 |
|  | too-many-params | 1.06 [0.70, 1.61] p=0.772 | 0.77 [0.42, 1.41] p=0.394 | 0.95 [0.67, 1.34] p=0.765 | 0.95 [0.64, 1.41] p=0.796 |
|  | f_comment_lines | 1.13 [1.08, 1.19] p=<0.001 | 1.11 [1.02, 1.19] p=0.010 | 1.13 [1.08, 1.18] p=<0.001 | 1.13 [1.06, 1.21] p=<0.001 |
|  | f_magic_numbers | 0.87 [0.79, 0.96] p=0.006 | 1.02 [0.89, 1.16] p=0.824 | 0.92 [0.85, 1.00] p=0.039 | 0.92 [0.82, 1.04] p=0.170 |
|  | f_clones | 1.12 [1.03, 1.22] p=0.010 | 0.90 [0.79, 1.02] p=0.111 | 1.05 [0.98, 1.13] p=0.171 | 1.05 [0.96, 1.15] p=0.276 |
|  | f_bool_params | 1.24 [1.04, 1.48] p=0.016 | 1.02 [0.81, 1.29] p=0.871 | 1.15 [1.00, 1.32] p=0.056 | 1.15 [0.97, 1.36] p=0.111 |
|  | unsafe-without-safety-comment | 1.18 [0.75, 1.87] p=0.472 | 1.92 [1.11, 3.35] p=0.020 | 1.29 [0.87, 1.91] p=0.199 | 1.29 [0.85, 1.96] p=0.227 |

Replication power, if the discovery estimate were the true effect: H1: power 0.36 at the Holm threshold (α = 0.05/7), 0.65 at α = 0.05; discovery and replication estimates differ by z = 1.14; H2: power 0.39 at the Holm threshold (α = 0.05/7), 0.68 at α = 0.05; discovery and replication estimates differ by z = 0.69.

## corrected miner

Discovery: 44960 functions, 3350 fixed. Replication: 26728 functions, 2418 fixed.

| | pattern | discovery, file-clustered | replication, file-clustered | pooled, file-clustered | pooled, repo-clustered |
|---|---|---|---|---|---|
| H1 | f_debt_comments | 1.44 [1.20, 1.74] p=<0.001 | 1.33 [0.96, 1.85] p=0.089 | 1.42 [1.20, 1.67] p=<0.001 | 1.42 [1.21, 1.66] p=<0.001 |
| H2 | max_nesting | 1.09 [0.99, 1.20] p=0.066 | 1.12 [1.00, 1.26] p=0.056 | 1.10 [1.02, 1.19] p=0.009 | 1.10 [0.99, 1.23] p=0.068 |
| H3 | cyclomatic | 0.95 [0.88, 1.02] p=0.155 | 1.03 [0.92, 1.15] p=0.625 | 0.98 [0.92, 1.04] p=0.522 ≈ | 0.98 [0.89, 1.08] p=0.691 ≈ |
| H4 | cognitive | 1.01 [0.96, 1.06] p=0.652 ≈ | 1.04 [0.97, 1.11] p=0.299 ≈ | 1.02 [0.98, 1.07] p=0.289 ≈ | 1.02 [0.95, 1.10] p=0.542 ≈ |
| H5 | long-function | 0.57 [0.46, 0.71] p=<0.001 | 0.74 [0.49, 1.11] p=0.141 | 0.63 [0.51, 0.77] p=<0.001 | 0.63 [0.52, 0.77] p=<0.001 |
| H6 | complex-function | 0.81 [0.68, 0.97] p=0.025 | 0.90 [0.68, 1.20] p=0.480 | 0.85 [0.73, 1.00] p=0.045 | 0.85 [0.69, 1.05] p=0.132 |
| H7 | unwrap | 0.93 [0.75, 1.14] p=0.481 | 1.04 [0.83, 1.30] p=0.728 | 0.97 [0.83, 1.13] p=0.680 | 0.97 [0.82, 1.14] p=0.696 |
|  | params | 1.21 [1.12, 1.32] p=<0.001 | 1.08 [0.97, 1.20] p=0.141 | 1.16 [1.09, 1.24] p=<0.001 | 1.16 [1.01, 1.33] p=0.033 |
|  | too-many-params | 1.13 [0.74, 1.70] p=0.575 | 0.89 [0.50, 1.59] p=0.695 | 1.03 [0.73, 1.45] p=0.857 | 1.03 [0.73, 1.46] p=0.860 |
|  | f_comment_lines | 1.10 [1.05, 1.16] p=<0.001 | 1.10 [1.01, 1.19] p=0.026 | 1.10 [1.06, 1.15] p=<0.001 | 1.10 [1.04, 1.17] p=<0.001 |
|  | f_magic_numbers | 0.86 [0.78, 0.95] p=0.003 | 1.04 [0.92, 1.17] p=0.514 | 0.92 [0.86, 1.00] p=0.050 | 0.92 [0.82, 1.04] p=0.209 |
|  | f_clones | 1.10 [1.01, 1.20] p=0.031 | 1.01 [0.90, 1.13] p=0.877 | 1.07 [1.00, 1.14] p=0.066 | 1.07 [0.98, 1.16] p=0.131 |
|  | f_bool_params | 1.29 [1.09, 1.53] p=0.004 | 1.02 [0.78, 1.34] p=0.870 | 1.17 [1.01, 1.36] p=0.038 | 1.17 [1.01, 1.36] p=0.036 |
|  | unsafe-without-safety-comment | 1.32 [0.84, 2.06] p=0.231 | 2.11 [1.29, 3.43] p=0.003 | 1.44 [0.99, 2.09] p=0.055 | 1.44 [0.96, 2.15] p=0.077 |

Replication power, if the discovery estimate were the true effect: H1: power 0.31 at the Holm threshold (α = 0.05/7), 0.59 at α = 0.05; discovery and replication estimates differ by z = 0.42; H2: power 0.11 at the Holm threshold (α = 0.05/7), 0.32 at α = 0.05; discovery and replication estimates differ by z = -0.34.

