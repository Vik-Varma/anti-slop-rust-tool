#!/usr/bin/env python3
"""Draws a fixed random sample of findings per rule for hand-checking (docs/AUDIT.md).

Usage: audit_sample.py SEED FINDINGS_JSON... > sample.txt
  where each FINDINGS_JSON is `anti-slop --format json --min-severity note <repo>` output.
"""

import collections
import json
import random
import sys


def main():
    seed, paths = int(sys.argv[1]), sys.argv[2:]
    by_rule = collections.defaultdict(list)
    for p in sorted(paths):
        with open(p) as f:
            for x in json.load(f):
                by_rule[x["rule"]].append(x)
    rng = random.Random(seed)
    for rule in sorted(by_rule):
        xs = by_rule[rule]
        k = 20 if len(xs) > 200 else 10
        for i, x in enumerate(rng.sample(xs, min(k, len(xs)))):
            with open(x["path"], errors="replace") as f:
                lines = f.read().split("\n")
            n = x["line"]
            print(f"### {rule} #{i} of {len(xs)} {x['path']}:{n} fn={x['function']}")
            for j in range(max(0, n - 5), min(len(lines), n + 2)):
                print((">> " if j + 1 == n else "   ") + lines[j][:150])


if __name__ == "__main__":
    main()
