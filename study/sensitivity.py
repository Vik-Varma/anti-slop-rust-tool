#!/usr/bin/env python3
"""Sensitivity analysis, added after the study review (docs/reviews/study-review.md). Not
preregistered. The preregistered result stays study/replication.md; this asks how much it
depends on choices the review questioned.

For each data version (as published, corrected miner),
each corpus (discovery, replication, both pooled) and each clustering (by file, by repository),
it fits the preregistered model (repo + log(1+LOC) + log(1+prior churn) + pattern) and reports
the odds ratio, its 95% CI, the unadjusted p-value, and whether the 90% CI lies inside
[0.9, 1.1], an equivalence test for "no effect". It also reports the power the replication had
to confirm H1 and H2 if the discovery estimate were true.

Usage: sensitivity.py > study/sensitivity.md   (run from the repository root)
"""

import glob
import math
import os

import numpy as np
from scipy import stats

from analyze import fit_logit, load

VERSIONS = [
    ("as published", "study/data", "study/data-replication"),
    ("corrected miner", "study/data-corrected", "study/data-replication-corrected"),
]
PATTERNS = [
    ("H1", "f_debt_comments", "log"),
    ("H2", "max_nesting", "log"),
    ("H3", "cyclomatic", "log"),
    ("H4", "cognitive", "log"),
    ("H5", "long-function", "any"),
    ("H6", "complex-function", "any"),
    ("H7", "unwrap", "any"),
    ("", "params", "log"),
    ("", "too-many-params", "any"),
    ("", "f_comment_lines", "log"),
    ("", "f_magic_numbers", "log"),
    ("", "f_clones", "log"),
    ("", "f_bool_params", "log"),
    ("", "unsafe-without-safety-comment", "any"),
]


def design(rows, name, kind):
    repos = sorted({r["repo"] for r in rows})
    y = np.array([int(r["later_fixes"]) > 0 for r in rows], dtype=float)
    loc = np.array([int(r["code_lines"]) for r in rows], dtype=float)
    prior = np.array([int(r["prior_changes"]) for r in rows], dtype=float)
    D = np.zeros((len(rows), len(repos)))
    D[np.arange(len(rows)), [repos.index(r["repo"]) for r in rows]] = 1.0
    x = np.array([float(r.get(name) or 0) for r in rows])
    g = np.log1p(x) if kind == "log" else (x > 0).astype(float)
    X = np.column_stack([D, np.log1p(loc), np.log1p(prior), g])
    files = np.array([hash((r["repo"], r["path"])) for r in rows])
    repo_ids = np.array([repos.index(r["repo"]) for r in rows])
    return X, y, files, repo_ids


def fit(rows, name, kind, cluster):
    if not rows or name not in rows[0]:
        return None
    X, y, files, repo_ids = design(rows, name, kind)
    if (X[:, -1] > 0).sum() < 30:
        return None
    beta, se, _, _ = fit_logit(X, y, files if cluster == "file" else repo_ids)
    b, s = beta[-1], se[-1]
    scale = math.log(2) if kind == "log" else 1.0
    orr, lo, hi = (math.exp(v * scale) for v in (b, b - 1.96 * s, b + 1.96 * s))
    lo90, hi90 = (math.exp(v * scale) for v in (b - 1.645 * s, b + 1.645 * s))
    return dict(b=b, se=s, orr=orr, lo=lo, hi=hi, p=2 * stats.norm.sf(abs(b / s)), equiv=0.9 < lo90 and hi90 < 1.1)


def cell(r):
    if r is None:
        return "n/a"
    p = "<0.001" if r["p"] < 0.001 else f"{r['p']:.3f}"
    return f"{r['orr']:.2f} [{r['lo']:.2f}, {r['hi']:.2f}] p={p}{' ≈' if r['equiv'] else ''}"


def main():
    print("# Sensitivity analysis (not preregistered)\n")
    print("Added after the study review. The preregistered result is `study/replication.md` and "
          "stands as published. Model: repo + log(1+LOC) + log(1+prior churn) + pattern. OR per "
          "doubling of (1 + metric) for metrics and counts, or for having the pattern at all. "
          "p is unadjusted. ≈ marks an equivalence pass: the 90% CI lies inside [0.9, 1.1]. "
          "\"Corrected miner\" follows renames, ignores pure moves and whole-file deletions, credits "
          "an insertion only to a function it lands inside, and chooses snapshots on the mainline.\n")
    for label, disc, rep in VERSIONS:
        if not glob.glob(os.path.join(disc, "*.csv")):
            continue
        d = load(sorted(glob.glob(os.path.join(disc, "*.csv"))))
        r = load(sorted(glob.glob(os.path.join(rep, "*.csv"))))
        print(f"## {label}\n")
        print(f"Discovery: {len(d)} functions, {sum(int(x['later_fixes']) > 0 for x in d)} fixed. "
              f"Replication: {len(r)} functions, {sum(int(x['later_fixes']) > 0 for x in r)} fixed.\n")
        print("| | pattern | discovery, file-clustered | replication, file-clustered | pooled, file-clustered | pooled, repo-clustered |")
        print("|---|---|---|---|---|---|")
        for hid, name, kind in PATTERNS:
            cells = [fit(d, name, kind, "file"), fit(r, name, kind, "file"), fit(d + r, name, kind, "file"),
                     fit(d + r, name, kind, "repo")]
            print(f"| {hid} | {name} | " + " | ".join(cell(c) for c in cells) + " |")
        print()
        # Power of the replication to confirm the positive hypotheses, if discovery were exactly right.
        lines = []
        for hid, name, kind in PATTERNS[:2]:
            dd, rr = fit(d, name, kind, "file"), fit(r, name, kind, "file")
            if dd and rr:
                z = dd["b"] / rr["se"]
                strict = stats.norm.ppf(1 - 0.05 / 7 / 2)
                power_holm = stats.norm.sf(strict - z) + stats.norm.cdf(-strict - z)
                power_05 = stats.norm.sf(1.96 - z) + stats.norm.cdf(-1.96 - z)
                diff = (dd["b"] - rr["b"]) / math.hypot(dd["se"], rr["se"])
                lines.append(f"{hid}: power {power_holm:.2f} at the Holm threshold (α = 0.05/7), {power_05:.2f} at α = 0.05; "
                             f"discovery and replication estimates differ by z = {diff:.2f}")
        if lines:
            print("Replication power, if the discovery estimate were the true effect: " + "; ".join(lines) + ".\n")


if __name__ == "__main__":
    main()
