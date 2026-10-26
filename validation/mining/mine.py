#!/usr/bin/env python3
"""Function-level defect mining for Rust repositories.

For each repository:
  1. Take a snapshot S: the last first-parent commit before --snapshot.
  2. Compute per-function metrics and anti-slop findings at S.
  3. Walk every first-parent commit in (S, --window-end]. For each commit,
     map the changed lines (old side of the diff) onto the functions of the
     parent revision, and count, per function, how many commits changed it
     and how many of those were bug fixes (keyword heuristic, as in SZZ).
  4. Write one CSV row per function that exists at S.

The output is analysed by analyze.py, which tests whether each metric or
rule is associated with later bug fixes *after controlling for size*.

Usage:
  mine.py --tool target/release/anti-slop --repos-dir DIR --out DIR [repo ...]
"""

import argparse
import collections
import csv
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from concurrent.futures import ProcessPoolExecutor

FIX_RE = re.compile(
    r"\b(fix(e[sd]|ing)?|bug(s|fix(es)?)?|crash(es|ed|ing)?|panic(s|ked|king)?|regression|"
    r"segfault|deadlock(s|ed)?|hang(s|ing)?|incorrect(ly)?|wrong(ly)?|overflow(s|ed)?|ub|unsound(ness)?)\b",
    re.I,
)
# Subjects that mention "fix" but are not defect fixes.
NON_DEFECT_RE = re.compile(
    r"\b(typos?|docs?|documentation|readme|changelog|clippy|lints?|rustfmt|fmt|formatting|"
    r"ci|warnings?|deprecat\w*|bump|release|version|spelling|comments?|msrv)\b",
    re.I,
)
STRONG_DEFECT_RE = re.compile(r"\b(bug|crash|panic|regression|segfault|deadlock|hang|unsound)", re.I)
BOT_RE = re.compile(r"(dependabot|renovate|github-actions|bors)", re.I)
MAX_FILES_PER_COMMIT = 30


def is_fix(subject, body, author):
    if BOT_RE.search(author):
        return False
    text = subject + "\n" + body
    if not FIX_RE.search(text):
        return False
    if NON_DEFECT_RE.search(subject) and not STRONG_DEFECT_RE.search(text):
        return False
    return True


def git(repo, *args, check=True):
    r = subprocess.run(["git", "-C", repo, *args], capture_output=True, check=check)
    return r.stdout.decode("utf-8", "replace")


def run_tool(tool, args):
    r = subprocess.run([tool, *args], capture_output=True)
    return r.stdout.decode("utf-8", "replace")


def metrics_for_dir(tool, root):
    """{relpath: [fn dict]}"""
    out = collections.defaultdict(list)
    for line in run_tool(tool, ["metrics", root]).splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        rel = os.path.relpath(row["file"], root)
        row["file"] = rel
        out[rel].append(row)
    return out


def innermost(fns, line):
    best = None
    for f in fns:
        start = f.get("start_line", f["line"])
        if start <= line <= f["end_line"]:
            if best is None or (f["end_line"] - start) < (best["end_line"] - best.get("start_line", best["line"])):
                best = f
    return best


def parse_diff(diff):
    """{old_path: set of old-side line markers}. Insertions are recorded as
    (a, a+1) pairs so they can be attributed only if inside one function."""
    changes = collections.defaultdict(lambda: {"lines": set(), "inserts": set()})
    cur = None
    for line in diff.splitlines():
        if line.startswith("--- "):
            path = line[4:]
            cur = path[2:] if path.startswith("a/") else None
        elif line.startswith("@@") and cur:
            m = re.match(r"@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@", line)
            if not m:
                continue
            a = int(m.group(1))
            b = int(m.group(2)) if m.group(2) is not None else 1
            if b == 0:
                changes[cur]["inserts"].add(a)
            else:
                changes[cur]["lines"].update(range(a, a + b))
    return changes


def mine_repo(args_tuple):
    tool, repo_dir, name, snapshot_date, window_end, out_dir = args_tuple
    log = []
    head = git(repo_dir, "rev-parse", "HEAD").strip()
    snap = git(repo_dir, "rev-list", "-1", "--first-parent", f"--before={snapshot_date}", head).strip()
    end = git(repo_dir, "rev-list", "-1", "--first-parent", f"--before={window_end}", head).strip()
    if not snap or not end:
        return name, "no snapshot"
    work = tempfile.mkdtemp(prefix=f"mine-{name}-")
    try:
        snap_dir = os.path.join(work, "snap")
        os.makedirs(snap_dir)
        archive = subprocess.run(["git", "-C", repo_dir, "archive", snap], capture_output=True, check=True).stdout
        subprocess.run(["tar", "-x", "-C", snap_dir], input=archive, check=True)
        snap_fns = metrics_for_dir(tool, snap_dir)

        # Findings at the snapshot, attributed to the innermost enclosing function.
        findings = collections.defaultdict(collections.Counter)
        report = json.loads(run_tool(tool, ["check", snap_dir, "--format", "json"]) or '{"findings": []}')
        for f in report["findings"]:
            rel = os.path.relpath(f["file"], snap_dir)
            fn = innermost(snap_fns.get(rel, []), f["line"])
            if fn is not None:
                findings[(rel, fn["qual_name"])][f["rule"]] += 1
        rules_seen = sorted({r for c in findings.values() for r in c})

        # Commits in the window.
        raw = git(repo_dir, "log", "--first-parent", "--format=%H%x00%P%x00%an%x00%s%x00%b%x01", f"{snap}..{end}")
        changes = collections.Counter()
        fixes = collections.Counter()
        n_commits = n_fix = 0
        for rec in raw.split("\x01"):
            rec = rec.strip("\n")
            if not rec:
                continue
            parts = rec.split("\x00")
            if len(parts) < 5:
                continue
            sha, parents, author, subject, body = parts[:5]
            parent = parents.split()[0] if parents else None
            if not parent:
                continue
            diff = git(repo_dir, "diff", "-U0", "--no-renames", parent, sha, "--", "*.rs", check=False)
            ch = parse_diff(diff)
            ch = {p: v for p, v in ch.items() if p != "/dev/null"}
            if not ch or len(ch) > MAX_FILES_PER_COMMIT:
                continue
            fix = is_fix(subject, body, author)
            n_commits += 1
            n_fix += fix
            cdir = os.path.join(work, "c")
            shutil.rmtree(cdir, ignore_errors=True)
            os.makedirs(cdir)
            for p in ch:
                content = subprocess.run(["git", "-C", repo_dir, "show", f"{parent}:{p}"], capture_output=True)
                if content.returncode != 0:
                    continue
                dest = os.path.join(cdir, p)
                os.makedirs(os.path.dirname(dest), exist_ok=True)
                with open(dest, "wb") as fh:
                    fh.write(content.stdout)
            parent_fns = metrics_for_dir(tool, cdir)
            touched = set()
            for p, v in ch.items():
                fns = parent_fns.get(p, [])
                for l in v["lines"]:
                    fn = innermost(fns, l)
                    if fn:
                        touched.add((p, fn["qual_name"]))
                for a in v["inserts"]:
                    fa, fb = innermost(fns, a), innermost(fns, a + 1)
                    if fa and fb and fa["qual_name"] == fb["qual_name"]:
                        touched.add((p, fa["qual_name"]))
            for key in touched:
                changes[key] += 1
                if fix:
                    fixes[key] += 1

        out_path = os.path.join(out_dir, f"{name}.csv")
        base_cols = ["repo", "file", "file_kind", "qual_name", "line", "loc", "params", "bool_params",
                     "cyclomatic", "cognitive", "max_nesting", "is_test", "is_async", "is_unsafe",
                     "is_trait_impl", "changes", "fixes"]
        with open(out_path, "w", newline="") as fh:
            w = csv.writer(fh)
            w.writerow(base_cols + [f"rule:{r}" for r in rules_seen])
            for rel, fns in snap_fns.items():
                names = collections.Counter(f["qual_name"] for f in fns)
                for f in fns:
                    if names[f["qual_name"]] > 1:
                        continue  # ambiguous identity (cfg variants etc.)
                    key = (rel, f["qual_name"])
                    w.writerow([name, rel, f["file_kind"], f["qual_name"], f["line"], f["loc"], f["params"],
                                f["bool_params"], f["cyclomatic"], f["cognitive"], f["max_nesting"],
                                int(f["is_test"]), int(f["is_async"]), int(f["is_unsafe"]),
                                int(f["is_trait_impl"]), changes[key], fixes[key]]
                               + [findings[key][r] for r in rules_seen])
        meta = {"repo": name, "snapshot": snap, "end": end, "commits": n_commits, "fix_commits": n_fix,
                "functions": sum(len(v) for v in snap_fns.values())}
        with open(os.path.join(out_dir, f"{name}.meta.json"), "w") as fh:
            json.dump(meta, fh)
        return name, meta
    finally:
        shutil.rmtree(work, ignore_errors=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tool", required=True)
    ap.add_argument("--repos-dir", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--snapshot", default="2024-01-01")
    ap.add_argument("--window-end", default="2025-07-01")
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("repos", nargs="*")
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    repos = a.repos or sorted(d for d in os.listdir(a.repos_dir) if os.path.isdir(os.path.join(a.repos_dir, d, ".git")))
    tool = os.path.abspath(a.tool)
    jobs = [(tool, os.path.join(a.repos_dir, r), r, a.snapshot, a.window_end, a.out) for r in repos]
    with ProcessPoolExecutor(a.jobs) as ex:
        for name, meta in ex.map(mine_repo, jobs):
            print(name, meta, file=sys.stderr, flush=True)


if __name__ == "__main__":
    main()
