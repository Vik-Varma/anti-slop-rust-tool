#!/usr/bin/env python3
"""The confirmatory test fixed in study/PREREGISTRATION.md, run on the replication corpus.

Model: repo + log LOC + log prior churn + pattern, file-clustered SEs, Holm across H1-H7.

Usage: confirm.py study/data-replication/*.csv > study/replication.md
"""

import math
import sys

import numpy as np
from scipy import stats

from analyze import fit_logit, holm, load

HYPOTHESES = [
    ("H1", "f_debt_comments", "log", "OR > 1, adjusted p < 0.05"),
    ("H2", "max_nesting", "log", "OR > 1, adjusted p < 0.05"),
    ("H3", "cyclomatic", "log", "CI contains 1, or OR < 1.10"),
    ("H4", "cognitive", "log", "CI contains 1, or OR < 1.10"),
    ("H5", "long-function", "any", "no effect, or OR < 1"),
    ("H6", "complex-function", "any", "CI contains 1"),
    ("H7", "unwrap", "any", "CI contains 1"),
]


def main():
    rows = load(sys.argv[1:])
    repos = sorted({r["repo"] for r in rows})
    y = np.array([int(r["later_fixes"]) > 0 for r in rows], dtype=float)
    loc = np.array([int(r["code_lines"]) for r in rows], dtype=float)
    prior = np.array([int(r["prior_changes"]) for r in rows], dtype=float)
    files = np.array([hash((r["repo"], r["path"])) for r in rows])
    D = np.zeros((len(rows), len(repos)))
    D[np.arange(len(rows)), [repos.index(r["repo"]) for r in rows]] = 1.0
    base = np.column_stack([D, np.log1p(loc), np.log1p(prior)])

    tests = []
    for hid, name, kind, pred in HYPOTHESES:
        x = np.array([float(r[name]) for r in rows])
        g = np.log1p(x) if kind == "log" else (x > 0).astype(float)
        beta, se, _, _ = fit_logit(np.column_stack([base, g]), y, files)
        b, s = beta[-1], se[-1]
        scale = math.log(2) if kind == "log" else 1.0
        orr, lo, hi = (math.exp(v * scale) for v in (b, b - 1.96 * s, b + 1.96 * s))
        tests.append(dict(hid=hid, name=name, pred=pred, exposed=int((x > 0).sum()), orr=orr, lo=lo, hi=hi,
                          p=2 * stats.norm.sf(abs(b / s))))
    for t, a in zip(tests, holm([t["p"] for t in tests])):
        t["adj"] = a
        contains_one = t["lo"] <= 1 <= t["hi"]
        t["held"] = {
            "H1": t["orr"] > 1 and a < 0.05,
            "H2": t["orr"] > 1 and a < 0.05,
            "H3": contains_one or t["orr"] < 1.10,
            "H4": contains_one or t["orr"] < 1.10,
            "H5": contains_one or t["orr"] < 1,
            "H6": contains_one,
            "H7": contains_one,
        }[t["hid"]]

    print("# Replication: the preregistered test\n")
    print(f"{len(rows)} non-test functions from {len(repos)} repositories not used in discovery; "
          f"{int(y.sum())} ({100 * y.mean():.1f}%) changed by a later bug fix.\n")
    print("| | pattern | functions with it | OR [95% CI] | Holm p | prediction | held? |")
    print("|---|---|---:|---|---:|---|---|")
    for t in tests:
        p = "<0.001" if t["adj"] < 0.001 else f"{t['adj']:.3f}"
        print(f"| {t['hid']} | {t['name']} | {t['exposed']} | {t['orr']:.2f} [{t['lo']:.2f}, {t['hi']:.2f}] | "
              f"{p} | {t['pred']} | {'yes' if t['held'] else '**no**'} |")


if __name__ == "__main__":
    main()
