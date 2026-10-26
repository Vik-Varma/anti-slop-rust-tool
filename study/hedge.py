#!/usr/bin/env python3
"""Exploratory, not preregistered: do antislop's "deferral" and "hedging" comment phrases mark
fix-prone functions? Adds two count columns to a mined CSV, from the snapshot's sources.

The phrase lists are antislop 0.3.0's `antislop-standard` profile (https://crates.io/crates/antislop),
applied to comment text only, as antislop applies them.

Usage: hedge.py REPO_DIR DATA_CSV OUT_CSV  (writes DATA_CSV plus f_deferral and f_hedging to OUT_CSV;
never in place, so the study's own data files keep their columns)
"""

import csv
import re
import subprocess
import sys

from mine import SNAPSHOT_DATE, rev_before

DEFERRAL = [r"for\s+now", r"temporary", r"for\s+later", r"quick\s*implement", r"workaround", r"\bWIP\b",
            r"shortcut", r"prototype"]
HEDGING = [r"hopefully.*work", r"should\s+work", r"seems\s+to\s+work", r"probably\s+work", r"good\s*enough",
           r"appears\s+to\s+work", r"\b(obviously|clearly|simply|easy|trivial)\b", r"\b(sorry|hacky|ugly|terrible)\b"]
DEFERRAL_RE = re.compile("|".join(DEFERRAL), re.I)
HEDGING_RE = re.compile("|".join(HEDGING), re.I)


def main():
    repo, data, out_csv = sys.argv[1], sys.argv[2], sys.argv[3]
    snap = rev_before(repo, SNAPSHOT_DATE)
    with open(data) as f:
        rows = list(csv.DictReader(f))
    cache = {}
    for r in rows:
        path = r["path"]
        if path not in cache:
            out = subprocess.run(["git", "-C", repo, "show", f"{snap}:{path}"], capture_output=True)
            cache[path] = out.stdout.decode("utf-8", "replace").split("\n")
        lines = cache[path][int(r["start"]) - 1 : int(r["end"])]
        comments = [l[l.index("//"):] for l in lines if "//" in l]
        r["f_deferral"] = sum(bool(DEFERRAL_RE.search(c)) for c in comments)
        r["f_hedging"] = sum(bool(HEDGING_RE.search(c)) for c in comments)
    with open(out_csv, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)


if __name__ == "__main__":
    main()
