# Crate size and later bug fixes (exploratory, not preregistered)

Does the size of the crate or file a function lives in predict that it is bug-fixed later, beyond its own size, its prior churn and its repository? Same outcome and controls as `replication.md`: logistic regression, a repository fixed effect, log(1 + function lines) and log(1 + prior churn). Size is in the study's units (code lines of non-test functions; types and constants aren't in the data), cross-checked against the crate's `.rs` bytes. OR is per doubling of (1 + size). Errors are clustered by crate, the unit the size varies over, and also by repository.

## Discovery: 44,961 functions in 181 crates of 20 repositories

Crate size (function lines): median 657, p90 10,498, p99 54,556, largest 55,729.

| size measure | OR per doubling [95% CI] (crate-clustered) | p | repo-clustered p |
|---|---|---|---|
| crate function lines | 0.96 [0.90, 1.04] | 0.318 | 0.432 |
| crate files | 0.90 [0.83, 0.99] | 0.022 | 0.085 |
| crate .rs KB (cross-check) | 0.96 [0.90, 1.03] | 0.232 | 0.327 |
| file function lines | 1.03 [0.95, 1.12] | 0.507 | 0.529 |
| crate lines, with file lines in the model | 0.96 [0.89, 1.03] | 0.260 | |
| file lines, with crate lines in the model | 1.04 [0.95, 1.13] | 0.412 | |

Crate size bands (function lines), adjusted odds of a later fix against the 1-3k band:

| band | crates | functions | fix rate | adjusted OR [95% CI] | p |
|---|---:|---:|---:|---|---|
| <1k | 99 | 2,841 | 6.4% | 0.83 [0.38, 1.80] | 0.634 |
| 1-3k | 39 | 5,440 | 8.8% | 1 (reference) | |
| 3-10k | 24 | 10,788 | 6.0% | 0.68 [0.36, 1.29] | 0.236 |
| 10-30k | 15 | 17,352 | 5.7% | 0.65 [0.30, 1.37] | 0.254 |
| >30k | 4 | 8,540 | 13.7% | 0.68 [0.35, 1.33] | 0.264 |

## Replication: 26,746 functions in 176 crates of 15 repositories

Crate size (function lines): median 496, p90 5,315, p99 25,569, largest 29,198.

| size measure | OR per doubling [95% CI] (crate-clustered) | p | repo-clustered p |
|---|---|---|---|
| crate function lines | 0.93 [0.86, 1.00] | 0.055 | <0.001 |
| crate files | 0.93 [0.85, 1.01] | 0.089 | <0.001 |
| crate .rs KB (cross-check) | 0.94 [0.86, 1.01] | 0.105 | 0.003 |
| file function lines | 1.01 [0.92, 1.10] | 0.906 | 0.872 |
| crate lines, with file lines in the model | 0.93 [0.86, 1.00] | 0.051 | |
| file lines, with crate lines in the model | 1.01 [0.92, 1.11] | 0.797 | |

Crate size bands (function lines), adjusted odds of a later fix against the 1-3k band:

| band | crates | functions | fix rate | adjusted OR [95% CI] | p |
|---|---:|---:|---:|---|---|
| <1k | 114 | 2,757 | 10.1% | 0.53 [0.27, 1.05] | 0.070 |
| 1-3k | 32 | 3,612 | 15.6% | 1 (reference) | |
| 3-10k | 22 | 8,373 | 6.5% | 0.68 [0.31, 1.48] | 0.330 |
| 10-30k | 8 | 12,004 | 5.2% | 0.47 [0.26, 0.86] | 0.014 |

## Both: 71,707 functions in 357 crates of 35 repositories

Crate size (function lines): median 613, p90 7,231, p99 34,705, largest 55,729.

| size measure | OR per doubling [95% CI] (crate-clustered) | p | repo-clustered p |
|---|---|---|---|
| crate function lines | 0.95 [0.90, 1.00] | 0.070 | 0.115 |
| crate files | 0.92 [0.86, 0.97] | 0.005 | 0.015 |
| crate .rs KB (cross-check) | 0.95 [0.91, 1.00] | 0.067 | 0.090 |
| file function lines | 1.02 [0.96, 1.09] | 0.510 | 0.520 |
| crate lines, with file lines in the model | 0.95 [0.90, 1.00] | 0.054 | |
| file lines, with crate lines in the model | 1.03 [0.97, 1.10] | 0.367 | |

Crate size bands (function lines), adjusted odds of a later fix against the 1-3k band:

| band | crates | functions | fix rate | adjusted OR [95% CI] | p |
|---|---:|---:|---:|---|---|
| <1k | 213 | 5,598 | 8.3% | 0.64 [0.38, 1.08] | 0.095 |
| 1-3k | 71 | 9,052 | 11.5% | 1 (reference) | |
| 3-10k | 46 | 19,161 | 6.2% | 0.67 [0.41, 1.12] | 0.129 |
| 10-30k | 23 | 29,356 | 5.5% | 0.56 [0.34, 0.92] | 0.022 |
| >30k | 4 | 8,540 | 13.7% | 0.63 [0.38, 1.05] | 0.075 |

## Where crates sit

Share of the corpus's crates above each size, and the repositories that hold them (function lines):

| threshold | crates above | of | repositories with one |
|---:|---:|---:|---|
| 5,000 | 52 | 357 | Rocket, actix-web, alacritty, cargo, clap, delta, diesel, eza... |
| 10,000 | 27 | 357 | alacritty, cargo, clap, diesel, gitoxide, helix, image, meilisearch... |
| 20,000 | 10 | 357 | cargo, meilisearch, nushell, tantivy, tokio, typst, wgpu, zellij |
| 50,000 | 2 | 357 | cargo, wgpu |
| 100,000 | 0 | 357 |  |

Largest crates:

| crate | function lines | files |
|---|---:|---:|
| wgpu:naga | 55,729 | 97 |
| cargo: | 54,556 | 207 |
| nushell:crates/nu-command | 47,106 | 335 |
| zellij:zellij-server | 34,705 | 61 |
| typst:crates/typst | 29,198 | 185 |
| tantivy: | 25,569 | 217 |
| wgpu:wgpu-hal | 24,530 | 42 |
| tokio:tokio | 23,182 | 269 |
| wgpu:wgpu-core | 22,068 | 53 |
| meilisearch:milli | 21,750 | 132 |
| regex:regex-automata | 19,686 | 59 |
| rustls:rustls | 17,685 | 84 |

## Sensitivity: the corrected miner

The same analysis on the corrected-miner outcomes (`data-corrected/`, `data-replication-corrected/`;
see `sensitivity.md` for what the correction changes). All 35 repositories, adjusted as above.

## Both (corrected miner): 71,688 functions in 357 crates of 35 repositories

Crate size (function lines): median 613, p90 7,231, p99 34,705, largest 55,729.

| size measure | OR per doubling [95% CI] (crate-clustered) | p | repo-clustered p |
|---|---|---|---|
| crate function lines | 0.98 [0.93, 1.03] | 0.426 | 0.435 |
| crate files | 0.95 [0.90, 1.02] | 0.141 | 0.195 |
| crate .rs KB (cross-check) | 0.98 [0.94, 1.03] | 0.509 | 0.519 |
| file function lines | 1.03 [0.98, 1.09] | 0.230 | 0.089 |
| crate lines, with file lines in the model | 0.97 [0.92, 1.03] | 0.352 | |
| file lines, with crate lines in the model | 1.04 [0.98, 1.10] | 0.201 | |

Crate size bands (function lines), adjusted odds of a later fix against the 1-3k band:

| band | crates | functions | fix rate | adjusted OR [95% CI] | p |
|---|---:|---:|---:|---|---|
| <1k | 213 | 5,603 | 8.3% | 0.88 [0.57, 1.36] | 0.563 |
| 1-3k | 71 | 9,049 | 9.2% | 1 (reference) | |
| 3-10k | 46 | 19,159 | 7.1% | 0.89 [0.57, 1.39] | 0.616 |
| 10-30k | 23 | 29,337 | 6.8% | 0.93 [0.61, 1.40] | 0.715 |
| >30k | 4 | 8,540 | 12.9% | 0.70 [0.42, 1.16] | 0.163 |

## Reading it

- No size measure shows more fixes in bigger crates or files once the function's own size, its
  prior churn and its repository are controlled. As published the crate-size odds ratio is 0.95
  per doubling (p = 0.07); with the corrected miner it is 0.98 (p = 0.43). The honest summary is
  no detectable effect, with the estimates sitting slightly below 1, not above.
- File size shows nothing either (1.02 to 1.03 per doubling, p about 0.2 to 0.5), which bears on
  any limit on lines per file.
- The size bands have wide intervals. Only 4 crates exceed 30,000 function lines and 2 exceed
  50,000 (naga and cargo's root crate), so the data cannot say anything about crates larger than
  the ones in these 35 projects.
- Cluster-robust errors with 15 repositories (replication, repo-clustered) are unreliable; the
  p < 0.001 there should not be read as strong evidence.
- Crate size is measured in function lines because that is what the study data holds. Types,
  constants and macros aren't counted, and the `.rs` bytes cross-check agrees.
- These are mature, reviewed projects. A crate larger or growing faster than any of these is
  outside what was measured.

