#!/usr/bin/env python3
"""Tests whether anti-slop metrics/rules predict later bug fixes, controlling for size.

Input: the per-repo CSVs written by mine.py.

For every predictor X (a rule firing in a function, or a metric threshold):
  * crude odds ratio of "function later touched by a bug-fix commit" for
    flagged vs. unflagged functions;
  * adjusted odds ratio from a logistic regression
        fixed ~ log(1 + LOC) + X + repository fixed effects
    with standard errors clustered by file;
  * Mantel-Haenszel odds ratio stratified by repository x LOC decile
    (a non-parametric size control);
  * Holm-adjusted p-values across all predictors.

A rule "survives" if its size-adjusted odds ratio is > 1 with Holm p < 0.05.

Usage: analyze.py DIR [--min-flagged 30] [--markdown out.md]
"""

import argparse
import glob
import os
import sys
import warnings

import numpy as np
import pandas as pd
import statsmodels.api as sm
import statsmodels.formula.api as smf
from statsmodels.stats.contingency_tables import StratifiedTable
from statsmodels.stats.multitest import multipletests

warnings.filterwarnings("ignore")

# Metric rules and the default thresholds anti-slop uses (src/config.rs).
METRIC_FLAGS = {
    "high-cognitive-complexity": ("cognitive", 25),
    "long-function": ("loc", 100),
    "deep-nesting": ("max_nesting", 5),
    "too-many-params": ("params", 5),
    "bool-params": ("bool_params", 1),
}


def load(d):
    frames = [pd.read_csv(f) for f in sorted(glob.glob(os.path.join(d, "*.csv")))]
    df = pd.concat(frames, ignore_index=True).fillna(0)
    prod = df[(df.file_kind.isin(["library", "binary"])) & (df.is_test == 0)].copy()
    prod["fixed"] = (prod.fixes > 0).astype(int)
    prod["changed"] = (prod.changes > 0).astype(int)
    prod["log_loc"] = np.log1p(prod["loc"])
    # Size strata for Mantel-Haenszel: quarter-steps of log2(LOC), so LOC varies
    # by less than ~19% within a stratum.
    prod["loc_bin"] = np.floor(4 * np.log2(prod["loc"] + 1)).astype(int)
    return prod


def predictors(df):
    preds = {}
    for rule, (col, thr) in METRIC_FLAGS.items():
        preds[rule] = (df[col] > thr).astype(int)
    for c in df.columns:
        if c.startswith("rule:"):
            name = c[5:]
            if name in METRIC_FLAGS:
                continue
            preds[name] = (df[c] > 0).astype(int)
    return preds


def odds_ratio(a, b, c, d):
    # a: flagged&fixed, b: flagged&not, c: unflagged&fixed, d: unflagged&not (Haldane correction)
    return ((a + 0.5) * (d + 0.5)) / ((b + 0.5) * (c + 0.5))


def analyze(df, outcome, min_flagged):
    rows = []
    for name, x in predictors(df).items():
        flagged = int(x.sum())
        row = {"predictor": name, "flagged_fns": flagged}
        if flagged < min_flagged or flagged == len(x):
            row["note"] = "too few flagged functions"
            rows.append(row)
            continue
        y = df[outcome]
        a = int(((x == 1) & (y == 1)).sum()); b = flagged - a
        c = int(((x == 0) & (y == 1)).sum()); d = int((x == 0).sum()) - c
        row["rate_flagged"] = a / flagged
        row["rate_unflagged"] = c / max(1, a + b + c + d - flagged)
        row["crude_or"] = odds_ratio(a, b, c, d)
        data = pd.DataFrame({"y": y, "x": x, "log_loc": df.log_loc, "repo": df.repo, "file": df.repo + "/" + df.file})
        try:
            m = smf.logit("y ~ log_loc + x + C(repo)", data=data).fit(
                disp=0, cov_type="cluster", cov_kwds={"groups": pd.factorize(data.file)[0]}, maxiter=200)
            row["adj_or"] = float(np.exp(m.params["x"]))
            lo, hi = m.conf_int().loc["x"]
            row["adj_ci"] = (float(np.exp(lo)), float(np.exp(hi)))
            row["p"] = float(m.pvalues["x"])
        except Exception as e:  # perfect separation etc.
            row["note"] = f"model failed: {type(e).__name__}"
        # Mantel-Haenszel over repo x LOC-decile strata.
        tables = []
        for _, g in df.assign(_x=x).groupby(["repo", "loc_bin"]):
            t = np.array([[((g._x == 1) & (g[outcome] == 1)).sum(), ((g._x == 1) & (g[outcome] == 0)).sum()],
                          [((g._x == 0) & (g[outcome] == 1)).sum(), ((g._x == 0) & (g[outcome] == 0)).sum()]])
            if t[0].sum() > 0 and t[1].sum() > 0:
                tables.append(t)
        if tables:
            try:
                st = StratifiedTable(tables)
                row["mh_or"] = float(st.oddsratio_pooled)
                row["mh_p"] = float(st.test_null_odds().pvalue)
            except Exception:
                pass
        rows.append(row)
    res = pd.DataFrame(rows)
    mask = res["p"].notna() if "p" in res else pd.Series(False, index=res.index)
    if mask.any():
        res.loc[mask, "p_holm"] = multipletests(res.loc[mask, "p"], method="holm")[1]
    return res


def verdict(r):
    if isinstance(r.get("note"), str) and r.get("note"):
        return "insufficient data"
    if pd.isna(r.get("p_holm")):
        return "n/a"
    if r["adj_or"] > 1 and r["p_holm"] < 0.05:
        return "SUPPORTED"
    if r["adj_or"] < 1 and r["p_holm"] < 0.05:
        return "CONTRADICTED"
    return "not significant"


def to_markdown(res, title, n_fns, n_pos):
    lines = [f"### {title}", "", f"{n_fns} functions, {n_pos} with the outcome.", "",
             "| predictor | flagged fns | rate flagged | rate unflagged | crude OR | size-adjusted OR (95% CI) | p | Holm p | MH OR | verdict |",
             "|---|---:|---:|---:|---:|---|---:|---:|---:|---|"]
    res = res.copy()
    res["verdict"] = res.apply(verdict, axis=1)
    order = {"SUPPORTED": 0, "not significant": 1, "CONTRADICTED": 2, "n/a": 3, "insufficient data": 4}
    res = res.sort_values(by=["verdict", "flagged_fns"], key=lambda s: s.map(order) if s.name == "verdict" else -s)
    for _, r in res.iterrows():
        if r["verdict"] == "insufficient data":
            lines.append(f"| {r.predictor} | {int(r.flagged_fns)} | | | | | | | | insufficient data |")
            continue
        ci = r.get("adj_ci")
        ci_s = f"{r.adj_or:.2f} ({ci[0]:.2f}-{ci[1]:.2f})" if isinstance(ci, tuple) else ""
        mh = f"{r.mh_or:.2f}" if not pd.isna(r.get("mh_or", np.nan)) else ""
        lines.append(f"| {r.predictor} | {int(r.flagged_fns)} | {r.rate_flagged:.1%} | {r.rate_unflagged:.1%} | "
                     f"{r.crude_or:.2f} | {ci_s} | {r.p:.2g} | {r.p_holm:.2g} | {mh} | {r.verdict} |")
    return "\n".join(lines) + "\n"


def continuous(df):
    """Per-SD effect of each continuous metric beyond log(LOC)."""
    lines = ["### Continuous metrics beyond size", "",
             "Logistic regression `fixed ~ log(1+LOC) + z(log(1+metric)) + repo`, standard errors clustered by file.", "",
             "| metric | OR per SD (95% CI) | p |", "|---|---|---:|"]
    groups = pd.factorize(df.repo + "/" + df.file)[0]
    for col in ["cognitive", "cyclomatic", "max_nesting", "params", "bool_params"]:
        z = np.log1p(df[col])
        z = (z - z.mean()) / z.std()
        data = pd.DataFrame({"y": df.fixed, "z": z, "log_loc": df.log_loc, "repo": df.repo})
        m = smf.logit("y ~ log_loc + z + C(repo)", data=data).fit(disp=0, cov_type="cluster", cov_kwds={"groups": groups})
        lo, hi = np.exp(m.conf_int().loc["z"])
        lines.append(f"| {col} | {np.exp(m.params['z']):.2f} ({lo:.2f}-{hi:.2f}) | {m.pvalues['z']:.2g} |")
    return "\n".join(lines) + "\n"


def single_test(df, x, title):
    data = pd.DataFrame({"y": df.fixed, "x": x, "log_loc": df.log_loc, "repo": df.repo})
    groups = pd.factorize(df.repo + "/" + df.file)[0]
    m = smf.logit("y ~ log_loc + x + C(repo)", data=data).fit(disp=0, cov_type="cluster", cov_kwds={"groups": groups})
    lo, hi = np.exp(m.conf_int().loc["x"])
    a = df.fixed[x == 1].mean(); b = df.fixed[x == 0].mean()
    return (f"### {title}\n\n{len(df)} functions ({int(x.sum())} flagged). Fix rate {a:.1%} flagged vs {b:.1%} "
            f"documented; size-adjusted OR {np.exp(m.params['x']):.2f} ({lo:.2f}-{hi:.2f}), p = {m.pvalues['x']:.2g}.\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("dir")
    ap.add_argument("--min-flagged", type=int, default=30)
    ap.add_argument("--markdown")
    a = ap.parse_args()
    df = load(a.dir)
    out = []
    # Size alone, for reference.
    base = smf.logit("fixed ~ log_loc + C(repo)", data=df).fit(disp=0)
    out.append(f"Size-only model: OR per unit log(1+LOC) = {np.exp(base.params['log_loc']):.2f} "
               f"(p = {base.pvalues['log_loc']:.2g}).\n")
    corr = df[["loc", "cyclomatic", "cognitive", "max_nesting", "params"]].corr(method="spearman")["loc"]
    out.append("Spearman correlation with LOC: " + ", ".join(f"{k} {v:.2f}" for k, v in corr.items() if k != "loc") + ".\n")
    res = analyze(df, "fixed", a.min_flagged)
    out.append(to_markdown(res, "Outcome: function later modified by a bug-fix commit", len(df), int(df.fixed.sum())))
    changed = df[df.changed == 1].copy()
    res2 = analyze(changed, "fixed", a.min_flagged)
    out.append(to_markdown(res2, "Outcome among changed functions only: was the change a bug fix?",
                           len(changed), int(changed.fixed.sum())))
    out.append(continuous(df))
    if "unsafe_blocks" in df.columns and "rule:undocumented-unsafe" in df.columns:
        uns = df[df.unsafe_blocks > 0].copy()
        x = (uns["rule:undocumented-unsafe"] > 0).astype(int)
        out.append(single_test(uns, x, "undocumented-unsafe, among functions that contain unsafe blocks"))
    text = "\n".join(out)
    print(text)
    if a.markdown:
        with open(a.markdown, "w") as fh:
            fh.write(text)


if __name__ == "__main__":
    main()
