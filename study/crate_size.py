#!/usr/bin/env python3
"""Exploratory, not preregistered: does the size of the crate (or file) a function lives in
predict later bug fixes, beyond the function's own size, its prior churn and its repository?

Crate size is measured in the study's own units: the code lines of all non-test functions in the
crate at the snapshot (types, constants and macros are not in the study data). A cross-check uses
the bytes of every .rs file in the crate at the snapshot, from `git ls-tree -l`. Crates are found
from the Cargo.toml files that have a [package] table, at the snapshot commit; a file belongs to
the deepest such crate above it. File size is likewise the code lines of the file's non-test
functions.

Usage: crate_size.py CORPUS_DIR[,CORPUS_DIR..] DATA_DIR[,DATA_DIR..]   > study/crate-size.md
  e.g. crate_size.py ~/corpus,~/corpus-rep study/data,study/data-replication
"""

import collections
import glob
import math
import os
import subprocess
import sys

import numpy as np
from scipy import stats

from analyze import fit_logit, load
from mine import SNAPSHOT_DATE, rev_before

BINS = [(0, 1000, "<1k"), (1000, 3000, "1-3k"), (3000, 10000, "3-10k"), (10000, 30000, "10-30k"), (30000, 10**9, ">30k")]


def git(repo, *args):
    return subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True, errors="replace").stdout


def crates_at(repo, snap):
    """{crate_dir: rs_bytes} for the crates in the tree at `snap`; '' is the repository root."""
    tree = git(repo, "ls-tree", "-r", "-l", snap).splitlines()
    manifests = []
    rs = []
    for line in tree:
        meta, _, path = line.partition("\t")
        parts = meta.split()
        size = int(parts[3]) if parts[3].isdigit() else 0
        if path.endswith("Cargo.toml"):
            manifests.append(path)
        elif path.endswith(".rs"):
            rs.append((path, size))
    dirs = []
    for m in manifests:
        text = git(repo, "show", f"{snap}:{m}")
        if "[package]" in text:
            dirs.append(os.path.dirname(m))
    dirs.sort(key=len, reverse=True)
    out = {d: 0 for d in dirs}
    for path, size in rs:
        d = owner(dirs, path)
        if d is not None:
            out[d] += size
    return dirs, out


def owner(dirs, path):
    for d in dirs:  # longest first
        if d == "" or path == d or path.startswith(d + "/"):
            return d
    return None


def clone_for(name, corpora):
    for c in corpora:
        p = os.path.join(c, name)
        if os.path.isdir(os.path.join(p, ".git")):
            return p
    return None


def build(corpora, data_dirs):
    rows = []
    crate_bytes = {}
    for dd in data_dirs:
        for f in sorted(glob.glob(os.path.join(dd, "*.csv"))):
            name = os.path.basename(f)[:-4]
            clone = clone_for(name, corpora)
            if clone is None:
                print(f"skip {name}: no clone", file=sys.stderr)
                continue
            snap = rev_before(clone, SNAPSHOT_DATE)
            dirs, sizes = crates_at(clone, snap)
            for r in load([f]):
                d = owner(dirs, r["path"])
                if d is None:
                    continue
                r["crate"] = f"{name}:{d}"
                r["corpus"] = "replication" if "replication" in dd else "discovery"
                rows.append(r)
                crate_bytes[r["crate"]] = sizes[d]
    by_crate = collections.defaultdict(int)
    by_file = collections.defaultdict(int)
    n_files = collections.defaultdict(set)
    for r in rows:
        n = int(r["code_lines"])
        by_crate[r["crate"]] += n
        by_file[(r["repo"], r["path"])] += n
        n_files[r["crate"]].add(r["path"])
    for r in rows:
        r["crate_lines"] = by_crate[r["crate"]]
        r["file_lines"] = by_file[(r["repo"], r["path"])]
        r["crate_files"] = len(n_files[r["crate"]])
        r["crate_kb"] = crate_bytes[r["crate"]] / 1024
    return rows, by_crate, n_files, crate_bytes


def design(rows, extra):
    repos = sorted({r["repo"] for r in rows})
    idx = {r: i for i, r in enumerate(repos)}
    y = np.array([int(r["later_fixes"]) > 0 for r in rows], dtype=float)
    D = np.zeros((len(rows), len(repos)))
    D[np.arange(len(rows)), [idx[r["repo"]] for r in rows]] = 1.0
    base = [np.log1p([float(r["code_lines"]) for r in rows]), np.log1p([float(r["prior_changes"]) for r in rows])]
    cols = base + [c(rows) for c in extra]
    return np.column_stack([D] + cols), y


def fit(rows, extra, cluster):
    X, y = design(rows, extra)
    ids = {}
    cl = np.array([ids.setdefault(cluster(r), len(ids)) for r in rows])
    beta, se, _, _ = fit_logit(X, y, cl)
    k = len(extra)
    out = []
    for j in range(k):
        b, s = beta[-k + j], se[-k + j]
        z = b / s
        out.append((b, s, 2 * stats.norm.sf(abs(z))))
    return out, len(ids)


def doubling(b, s):
    f = math.log(2)
    return f"{math.exp(b * f):.2f} [{math.exp((b - 1.96 * s) * f):.2f}, {math.exp((b + 1.96 * s) * f):.2f}]"


def pfmt(p):
    return "<0.001" if p < 0.001 else f"{p:.3f}"


def logcol(key):
    return lambda rows: np.log1p([float(r[key]) for r in rows])


def main():
    corpora = sys.argv[1].split(",")
    data_dirs = sys.argv[2].split(",")
    rows, by_crate, n_files, crate_bytes = build(corpora, data_dirs)
    print("# Crate size and later bug fixes (exploratory, not preregistered)\n")
    print("Does the size of the crate or file a function lives in predict that it is bug-fixed later, "
          "beyond its own size, its prior churn and its repository? Same outcome and controls as "
          "`replication.md`: logistic regression, a repository fixed effect, log(1 + function lines) "
          "and log(1 + prior churn). Size is in the study's units (code lines of non-test functions; "
          "types and constants aren't in the data), cross-checked against the crate's `.rs` bytes. "
          "OR is per doubling of (1 + size). Errors are clustered by crate, the unit the size varies "
          "over, and also by repository.\n")
    for label, sel in [("Discovery", "discovery"), ("Replication", "replication"), ("Both", None)]:
        sub = [r for r in rows if sel is None or r["corpus"] == sel]
        crates = {r["crate"] for r in sub}
        sizes = sorted({by_crate[c] for c in crates})
        pcs = sorted(by_crate[c] for c in crates)
        q = lambda p: pcs[min(len(pcs) - 1, int(p * len(pcs)))]
        print(f"## {label}: {len(sub):,} functions in {len(crates)} crates of {len({r['repo'] for r in sub})} repositories\n")
        print(f"Crate size (function lines): median {q(0.5):,}, p90 {q(0.9):,}, p99 {q(0.99):,}, largest {pcs[-1]:,}.\n")
        print("| size measure | OR per doubling [95% CI] (crate-clustered) | p | repo-clustered p |")
        print("|---|---|---|---|")
        for name, key in [("crate function lines", "crate_lines"), ("crate files", "crate_files"), ("crate .rs KB (cross-check)", "crate_kb"), ("file function lines", "file_lines")]:
            (b, s, p), _ = (lambda o, n: (o[0], n))(*fit(sub, [logcol(key)], lambda r: r["crate"]))
            (_, _, p_repo), _ = (lambda o, n: (o[0], n))(*fit(sub, [logcol(key)], lambda r: r["repo"]))
            print(f"| {name} | {doubling(b, s)} | {pfmt(p)} | {pfmt(p_repo)} |")
        both, _ = fit(sub, [logcol("crate_lines"), logcol("file_lines")], lambda r: r["crate"])
        print(f"| crate lines, with file lines in the model | {doubling(both[0][0], both[0][1])} | {pfmt(both[0][2])} | |")
        print(f"| file lines, with crate lines in the model | {doubling(both[1][0], both[1][1])} | {pfmt(both[1][2])} | |")
        print()
        # Size bands against the 1-3k band, same controls.
        print("Crate size bands (function lines), adjusted odds of a later fix against the 1-3k band:\n")
        print("| band | crates | functions | fix rate | adjusted OR [95% CI] | p |")
        print("|---|---:|---:|---:|---|---|")
        band_of = lambda r: next(l for lo, hi, l in BINS if lo <= r["crate_lines"] < hi)
        cols = []
        names = []
        for lo, hi, l in BINS:
            if l == "1-3k":
                continue
            names.append(l)
            cols.append(lambda rows, l=l: np.array([1.0 if band_of(r) == l else 0.0 for r in rows]))
        res, _ = fit(sub, cols, lambda r: r["crate"])
        stats_by = {}
        for r in sub:
            stats_by.setdefault(band_of(r), []).append(int(r["later_fixes"]) > 0)
        crates_by = collections.defaultdict(set)
        for r in sub:
            crates_by[band_of(r)].add(r["crate"])
        for lo, hi, l in BINS:
            xs = stats_by.get(l, [])
            if not xs:
                continue
            if l == "1-3k":
                print(f"| {l} | {len(crates_by[l])} | {len(xs):,} | {sum(xs) / len(xs):.1%} | 1 (reference) | |")
                continue
            if sum(xs) < 20:
                print(f"| {l} | {len(crates_by[l])} | {len(xs):,} | {sum(xs) / len(xs):.1%} | too few | |")
                continue
            b, s, p = res[names.index(l)]
            print(f"| {l} | {len(crates_by[l])} | {len(xs):,} | {sum(xs) / len(xs):.1%} | {math.exp(b):.2f} [{math.exp(b - 1.96 * s):.2f}, {math.exp(b + 1.96 * s):.2f}] | {pfmt(p)} |")
        print()
    # The distribution that a cap would act on.
    crates = {}
    for r in rows:
        crates[r["crate"]] = (r["repo"], by_crate[r["crate"]], len(n_files[r["crate"]]))
    print("## Where crates sit\n")
    print("Share of the corpus's crates above each size, and the repositories that hold them "
          "(function lines):\n")
    print("| threshold | crates above | of | repositories with one |")
    print("|---:|---:|---:|---|")
    for t in (5_000, 10_000, 20_000, 50_000, 100_000):
        above = [v for v in crates.values() if v[1] >= t]
        repos = sorted({v[0] for v in above})
        print(f"| {t:,} | {len(above)} | {len(crates)} | {', '.join(repos[:8])}{'...' if len(repos) > 8 else ''} |")
    print()
    print("Largest crates:\n")
    print("| crate | function lines | files |")
    print("|---|---:|---:|")
    for c, (repo, n, f) in sorted(crates.items(), key=lambda kv: -kv[1][1])[:12]:
        print(f"| {c or repo} | {n:,} | {f} |")


if __name__ == "__main__":
    main()
