#!/usr/bin/env python3
"""Tests which code patterns predict later bug fixes once size is accounted for.

Outcome: a function was changed by at least one bug-fix commit after the snapshot.

For each candidate pattern X, three logistic models, all with a fixed effect per repository:

  naive     y ~ repo + g(X)                      what a "bad code" claim usually rests on
  sized     y ~ repo + log(1+LOC) + g(X)         El Emam et al. (2001): control for size
  churn     y ~ repo + log(1+LOC) + log(1+prior changes) + g(X)

g(X) is log(1+X) for counts and metrics, or an indicator for rules and thresholds. Standard
errors are cluster-robust by file (functions in one file share fate). p-values are Holm-adjusted
across all patterns within each model. Also reported: Spearman correlation of X with LOC, and in
how many repositories the sized-model coefficient is positive (consistency across projects).

Usage: analyze.py study/data/*.csv > study/results.md
"""

import csv
import math
import sys
from collections import defaultdict

import numpy as np
from scipy import stats

METRICS = ["cyclomatic", "cognitive", "max_nesting", "params", "try_ops"]
FEATURES = [
    "f_clones", "f_expects", "f_panics", "f_as_casts", "f_magic_numbers", "f_unsafe_blocks",
    "f_bool_params", "f_debt_comments", "f_comment_lines", "f_deferral", "f_hedging",
]
RULES = [
    "unwrap", "todo", "dbg-macro", "panic-in-drop", "static-mut", "uninit-memory",
    "unsafe-without-safety-comment", "mem-forget", "lock-held-across-await", "blocking-sleep-in-async",
    "ignored-io-error", "unused-io-amount", "process-exit-in-library", "float-eq-inexact",
    "len-truncating-cast", "complex-function", "long-function", "deep-nesting", "too-many-params",
]
MIN_EXPOSED = 30  # below this many functions with the pattern, there's nothing to estimate


def load(paths):
    rows = []
    for p in paths:
        with open(p) as f:
            rows.extend(r for r in csv.DictReader(f) if r["test"] == "0")
    return rows


def fit_logit(X, y, clusters, ridge=1e-6, iters=50):
    """IRLS logistic regression. Returns (beta, robust_se, loglik, fitted)."""
    n, k = X.shape
    beta = np.zeros(k)
    for _ in range(iters):
        eta = X @ beta
        p = 1 / (1 + np.exp(-eta))
        w = p * (1 - p)
        H = X.T @ (X * w[:, None]) + ridge * np.eye(k)
        g = X.T @ (y - p) - ridge * beta
        step = np.linalg.solve(H, g)
        beta += step
        if np.max(np.abs(step)) < 1e-9:
            break
    p = 1 / (1 + np.exp(-(X @ beta)))
    p = np.clip(p, 1e-12, 1 - 1e-12)
    loglik = float(np.sum(y * np.log(p) + (1 - y) * np.log(1 - p)))
    Hinv = np.linalg.inv(X.T @ (X * (p * (1 - p))[:, None]) + ridge * np.eye(k))
    scores = X * (y - p)[:, None]
    meat = np.zeros((k, k))
    order = np.argsort(clusters, kind="stable")
    cs = clusters[order]
    bounds = np.flatnonzero(np.r_[True, cs[1:] != cs[:-1], True])
    for a, b in zip(bounds[:-1], bounds[1:]):
        s = scores[order[a:b]].sum(axis=0)
        meat += np.outer(s, s)
    G = len(bounds) - 1
    V = Hinv @ meat @ Hinv * (G / (G - 1))
    return beta, np.sqrt(np.diag(V)), loglik, p


def auc(y, score):
    r = stats.rankdata(score)
    pos = y == 1
    n1, n0 = pos.sum(), (~pos).sum()
    return (r[pos].sum() - n1 * (n1 + 1) / 2) / (n1 * n0)


def holm(ps):
    order = sorted(range(len(ps)), key=lambda i: ps[i])
    adj = [0.0] * len(ps)
    running = 0.0
    for rank, i in enumerate(order):
        running = max(running, min(1.0, (len(ps) - rank) * ps[i]))
        adj[i] = running
    return adj


def main():
    rows = load(sys.argv[1:])
    repos = sorted({r["repo"] for r in rows})
    y = np.array([int(r["later_fixes"]) > 0 for r in rows], dtype=float)
    loc = np.array([int(r["code_lines"]) for r in rows], dtype=float)
    prior = np.array([int(r["prior_changes"]) for r in rows], dtype=float)
    files = np.array([hash((r["repo"], r["path"])) for r in rows])
    repo_idx = np.array([repos.index(r["repo"]) for r in rows])
    D = np.zeros((len(rows), len(repos)))
    D[np.arange(len(rows)), repo_idx] = 1.0
    size = np.log1p(loc)
    churn = np.log1p(prior)
    # Flexible size control: one dummy per LOC decile (drop the first), in case log-linear is wrong.
    edges = np.quantile(loc, np.linspace(0, 1, 11)[1:-1])
    decile = np.searchsorted(edges, loc, side="right")
    S = np.zeros((len(rows), 10))
    S[np.arange(len(rows)), decile] = 1.0
    S = S[:, 1:]
    later = np.array([int(r["later_changes"]) for r in rows], dtype=float)
    changed = later > 0

    out = []
    w = out.append
    w("# Which patterns predict bug fixes?\n")
    w(f"{len(rows)} non-test functions from {len(repos)} repositories; {int(y.sum())} "
      f"({100 * y.mean():.1f}%) were changed by a bug-fix commit after the snapshot.\n")
    w("| repo | functions | fixed | rate |\n|---|---:|---:|---:|")
    for i, name in enumerate(repos):
        m = repo_idx == i
        w(f"| {name} | {m.sum()} | {int(y[m].sum())} | {100 * y[m].mean():.1f}% |")

    base_sized = np.column_stack([D, size])
    base_churn = np.column_stack([D, size, churn])
    b, se, _, p_sized = fit_logit(base_sized, y, files)
    _, _, _, p_churn = fit_logit(base_churn, y, files)
    _, _, _, p_repo = fit_logit(D, y, files)
    w(f"\nSize alone: odds ratio per doubling of LOC = {math.exp(b[-1] * math.log(2)):.2f} "
      f"(robust SE of log-odds {se[-1]:.3f}). AUC: repo only {auc(y, p_repo):.3f}, "
      f"+ size {auc(y, p_sized):.3f}, + prior churn {auc(y, p_churn):.3f}.\n")

    results = []
    candidates = [(m, "log") for m in METRICS + FEATURES] + [(r, "any") for r in RULES]
    for name, kind in candidates:
        if name not in rows[0]:
            continue  # data mined by a build without this rule
        x = np.array([float(r[name]) for r in rows])
        exposed = int((x > 0).sum())
        if exposed < MIN_EXPOSED:
            results.append(dict(name=name, kind=kind, exposed=exposed, skip=True))
            continue
        g = np.log1p(x) if kind == "log" else (x > 0).astype(float)
        rho = stats.spearmanr(x, loc).statistic
        res = dict(name=name, kind=kind, exposed=exposed, rho=rho, skip=False)
        res["rate1"] = y[x > 0].mean()
        res["rate0"] = y[x == 0].mean() if (x == 0).any() else float("nan")
        for label, base in [("naive", D), ("sized", base_sized), ("churn", base_churn),
                            ("flex", np.column_stack([D, S, size, churn]))]:
            beta, se, _, p = fit_logit(np.column_stack([base, g]), y, files)
            z = beta[-1] / se[-1]
            res[label] = (beta[-1], se[-1], 2 * stats.norm.sf(abs(z)))
            if label == "sized":
                res["dauc"] = auc(y, p) - auc(y, p_sized)
        # Among functions that changed at all: is a change to one with X more often a fix?
        Xc = np.column_stack([D[changed], size[changed], np.log1p(later[changed]), g[changed]])
        beta, se, _, _ = fit_logit(Xc, y[changed], files[changed])
        res["cond"] = (beta[-1], se[-1], 2 * stats.norm.sf(abs(beta[-1] / se[-1])))
        positive = 0
        fitted = 0
        for i in range(len(repos)):
            m = repo_idx == i
            if (g[m] > 0).sum() < 10 or y[m].sum() < 5:
                continue
            beta, _, _, _ = fit_logit(np.column_stack([np.ones(m.sum()), size[m], g[m]]), y[m], files[m])
            fitted += 1
            positive += beta[-1] > 0
        res["consistency"] = (positive, fitted)
        results.append(res)

    live = [r for r in results if not r["skip"]]
    for label in ["naive", "sized", "churn", "flex", "cond"]:
        adj = holm([r[label][2] for r in live])
        for r, a in zip(live, adj):
            r[label + "_adj"] = a

    def fmt_or(r, label):
        beta, se, _ = r[label]
        scale = math.log(2) if r["kind"] == "log" else 1.0
        lo, hi = math.exp((beta - 1.96 * se) * scale), math.exp((beta + 1.96 * se) * scale)
        return f"{math.exp(beta * scale):.2f} [{lo:.2f}, {hi:.2f}]"

    def fmt_p(p):
        return "<0.001" if p < 0.001 else f"{p:.3f}"

    w("## Results\n")
    w("OR: odds ratio for being bug-fixed later, per doubling of (1+X) for metrics and counts, or "
      "for having any finding for rules. 95% CI from file-clustered SEs. p is Holm-adjusted. "
      "Consistency: repositories where the size-adjusted effect is positive / repositories with "
      "enough data.\n")
    w("| pattern | functions with it | fix rate with / without | ρ with LOC | naive OR | p | "
      "size-adjusted OR | p | + churn OR | p | ΔAUC | consistency |")
    w("|---|---:|---|---:|---|---:|---|---:|---|---:|---:|---:|")
    for r in results:
        if r["skip"]:
            w(f"| {r['name']} | {r['exposed']} | too rare to estimate | | | | | | | | | |")
            continue
        pos, fit = r["consistency"]
        w(f"| {r['name']} | {r['exposed']} | {100 * r['rate1']:.1f}% / {100 * r['rate0']:.1f}% | "
          f"{r['rho']:.2f} | {fmt_or(r, 'naive')} | {fmt_p(r['naive_adj'])} | {fmt_or(r, 'sized')} | "
          f"{fmt_p(r['sized_adj'])} | {fmt_or(r, 'churn')} | {fmt_p(r['churn_adj'])} | "
          f"{r['dauc']:+.4f} | {pos}/{fit} |")
    w("\n## Robustness\n")
    w("flexible size: repo + LOC-decile dummies + log LOC + log prior churn. changed only: among "
      "functions changed at all after the snapshot, controlling for size and log(number of later "
      "changes): is a change to a function with the pattern more often a bug fix?\n")
    w("| pattern | flexible-size OR | p | changed-only OR | p |")
    w("|---|---|---:|---|---:|")
    for r in live:
        w(f"| {r['name']} | {fmt_or(r, 'flex')} | {fmt_p(r['flex_adj'])} | {fmt_or(r, 'cond')} | "
          f"{fmt_p(r['cond_adj'])} |")
    print("\n".join(out))


if __name__ == "__main__":
    main()
