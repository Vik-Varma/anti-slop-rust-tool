Size-only model: OR per unit log(1+LOC) = 2.79 (p = 0).

Spearman correlation with LOC: cyclomatic 0.72, cognitive 0.71, max_nesting 0.72, params 0.49.

### Outcome: function later modified by a bug-fix commit

40320 functions, 4388 with the outcome.

| predictor | flagged fns | rate flagged | rate unflagged | crude OR | size-adjusted OR (95% CI) | p | Holm p | MH OR | verdict |
|---|---:|---:|---:|---:|---|---:|---:|---:|---|
| satd-comment | 883 | 35.4% | 10.3% | 4.77 | 1.58 (1.29-1.93) | 1e-05 | 0.00014 | 1.50 | SUPPORTED |
| unwrap-used | 1374 | 19.9% | 10.6% | 2.10 | 1.17 (0.90-1.53) | 0.24 | 1 | 1.18 | not significant |
| undocumented-unsafe | 643 | 19.3% | 10.7% | 1.99 | 3.87 (0.78-19.19) | 0.098 | 0.94 | 3.10 | not significant |
| high-cognitive-complexity | 634 | 51.1% | 10.2% | 9.16 | 1.07 (0.82-1.41) | 0.6 | 1 | 1.09 | not significant |
| long-function | 505 | 58.2% | 10.3% | 12.15 | 1.02 (0.75-1.38) | 0.9 | 1 | 1.05 | not significant |
| duplicate-code | 463 | 25.9% | 10.7% | 2.92 | 0.67 (0.50-0.88) | 0.004 | 0.052 | 0.66 | not significant |
| too-many-params | 409 | 36.4% | 10.6% | 4.83 | 1.29 (0.98-1.69) | 0.069 | 0.76 | 1.28 | not significant |
| unwrap-in-result-fn | 364 | 20.9% | 10.8% | 2.19 | 1.06 (0.52-2.16) | 0.87 | 1 | 1.09 | not significant |
| deep-nesting | 169 | 56.2% | 10.7% | 10.71 | 1.38 (0.95-2.02) | 0.094 | 0.94 | 1.32 | not significant |
| commented-out-code | 161 | 31.1% | 10.8% | 3.74 | 1.01 (0.66-1.54) | 0.97 | 1 | 1.01 | not significant |
| bool-params | 110 | 40.9% | 10.8% | 5.74 | 1.22 (0.73-2.05) | 0.45 | 1 | 1.16 | not significant |
| redundant-comment | 74 | 35.1% | 10.8% | 4.49 | 1.14 (0.63-2.08) | 0.67 | 1 | 1.37 | not significant |
| ptr-arg | 73 | 32.9% | 10.8% | 4.07 | 0.93 (0.36-2.41) | 0.87 | 1 | 0.89 | not significant |
| format-push-string | 46 | 17.4% | 10.9% | 1.81 | 0.27 (0.09-0.78) | 0.016 | 0.19 | 0.19 | not significant |
| exit-in-library | 27 | | | | | | | | insufficient data |
| panic-in-drop | 24 | | | | | | | | insufficient data |
| swallowed-error | 12 | | | | | | | | insufficient data |
| eager-fallback-alloc | 12 | | | | | | | | insufficient data |
| borrowed-clone | 8 | | | | | | | | insufficient data |
| todo-in-production | 7 | | | | | | | | insufficient data |
| unused-io-amount | 5 | | | | | | | | insufficient data |
| set-len-uninit | 4 | | | | | | | | insufficient data |
| uninit-memory | 3 | | | | | | | | insufficient data |
| unit-error-type | 3 | | | | | | | | insufficient data |
| dbg-macro | 2 | | | | | | | | insufficient data |
| blocking-in-async | 1 | | | | | | | | insufficient data |
| needless-collect | 1 | | | | | | | | insufficient data |
| redundant-box | 1 | | | | | | | | insufficient data |

### Outcome among changed functions only: was the change a bug fix?

14890 functions, 4388 with the outcome.

| predictor | flagged fns | rate flagged | rate unflagged | crude OR | size-adjusted OR (95% CI) | p | Holm p | MH OR | verdict |
|---|---:|---:|---:|---:|---|---:|---:|---:|---|
| unwrap-used | 742 | 36.8% | 29.1% | 1.42 | 0.92 (0.67-1.27) | 0.61 | 1 | 0.92 | not significant |
| satd-comment | 683 | 45.8% | 28.7% | 2.10 | 1.29 (1.05-1.58) | 0.016 | 0.21 | 1.18 | not significant |
| high-cognitive-complexity | 533 | 60.8% | 28.3% | 3.92 | 1.24 (0.94-1.64) | 0.13 | 1 | 1.11 | not significant |
| long-function | 452 | 65.0% | 28.4% | 4.69 | 1.33 (0.97-1.81) | 0.077 | 0.85 | 1.36 | not significant |
| too-many-params | 330 | 45.2% | 29.1% | 2.01 | 1.10 (0.81-1.50) | 0.53 | 1 | 1.08 | not significant |
| undocumented-unsafe | 268 | 46.3% | 29.2% | 2.09 | 1.81 (0.81-4.01) | 0.15 | 1 | 1.51 | not significant |
| duplicate-code | 254 | 47.2% | 29.2% | 2.18 | 0.66 (0.46-0.96) | 0.028 | 0.34 | 0.64 | not significant |
| unwrap-in-result-fn | 201 | 37.8% | 29.4% | 1.47 | 1.02 (0.45-2.28) | 0.97 | 1 | 1.03 | not significant |
| deep-nesting | 148 | 64.2% | 29.1% | 4.34 | 1.42 (0.95-2.11) | 0.087 | 0.87 | 1.22 | not significant |
| commented-out-code | 108 | 46.3% | 29.3% | 2.08 | 1.09 (0.69-1.73) | 0.7 | 1 | 1.06 | not significant |
| bool-params | 78 | 57.7% | 29.3% | 3.27 | 1.13 (0.64-1.98) | 0.67 | 1 | 1.04 | not significant |
| ptr-arg | 51 | 47.1% | 29.4% | 2.14 | 0.52 (0.24-1.12) | 0.095 | 0.87 | 0.53 | not significant |
| redundant-comment | 46 | 56.5% | 29.4% | 3.11 | 1.07 (0.50-2.26) | 0.87 | 1 | 1.34 | not significant |
| format-push-string | 26 | | | | | | | | insufficient data |
| exit-in-library | 15 | | | | | | | | insufficient data |
| swallowed-error | 10 | | | | | | | | insufficient data |
| eager-fallback-alloc | 10 | | | | | | | | insufficient data |
| panic-in-drop | 9 | | | | | | | | insufficient data |
| borrowed-clone | 5 | | | | | | | | insufficient data |
| todo-in-production | 5 | | | | | | | | insufficient data |
| unused-io-amount | 4 | | | | | | | | insufficient data |
| set-len-uninit | 4 | | | | | | | | insufficient data |
| uninit-memory | 3 | | | | | | | | insufficient data |
| dbg-macro | 2 | | | | | | | | insufficient data |
| unit-error-type | 2 | | | | | | | | insufficient data |
| blocking-in-async | 1 | | | | | | | | insufficient data |
| needless-collect | 1 | | | | | | | | insufficient data |
| redundant-box | 1 | | | | | | | | insufficient data |

### Continuous metrics beyond size

Logistic regression `fixed ~ log(1+LOC) + z(log(1+metric)) + repo`, standard errors clustered by file.

| metric | OR per SD (95% CI) | p |
|---|---|---:|
| cognitive | 0.98 (0.87-1.10) | 0.69 |
| cyclomatic | 0.99 (0.88-1.12) | 0.89 |
| max_nesting | 1.00 (0.89-1.12) | 0.96 |
| params | 1.11 (1.04-1.18) | 0.0023 |
| bool_params | 1.03 (0.99-1.07) | 0.15 |

### undocumented-unsafe, among functions that contain unsafe blocks

840 functions (642 flagged). Fix rate 19.3% flagged vs 8.1% documented; size-adjusted OR 0.79 (0.29-2.16), p = 0.65.
