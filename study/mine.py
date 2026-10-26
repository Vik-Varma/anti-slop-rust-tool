#!/usr/bin/env python3
"""Mines one git repository for the defect study.

For every non-test function in the repository as of SNAPSHOT_DATE it records anti-slop's
metrics and findings, then counts:

  later_fixes    bug-fix commits after the snapshot (to END_DATE) that changed the function
  later_changes  all commits after the snapshot that changed it
  prior_changes  all commits in the PRIOR_DAYS before the snapshot that changed it

A function is identified by (file path, qualified name), as anti-slop names it. Functions
whose identity is ambiguous in a file (two `S::fmt` impls) are dropped. A commit changes a
function when a line it deletes or edits, or the point where it inserts, falls inside the
function's span in the commit's parent. Bug fixes are classified by commit message keywords
(Mockus & Votta 2000), minus housekeeping; see FIX and NOT_FIX.

Usage: mine.py REPO_DIR ANTI_SLOP_BINARY OUT_CSV
"""

import csv
import io
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections import defaultdict

SNAPSHOT_DATE = "2024-07-01"
END_DATE = "2026-10-01"
PRIOR_DAYS = 365

FIX = re.compile(
    r"\b(fix(e[sd]|ing)?|bug(s|fix)?|crash(es|ed)?|panic(s|ked)?|regression|incorrect(ly)?|wrong|"
    r"broken|segfault|deadlock(s|ed)?|hang(s)?|leak(s)?|overflow(s)?|race)\b",
    re.I,
)
NOT_FIX = re.compile(
    r"\b(typos?|docs?|documentation|readme|changelog|spelling|clippy|lints?|rustfmt|fmt|formatting|ci|"
    r"warnings?|bump|release|deps|dependabot|chore)\b",
    re.I,
)


def is_fix(subject, body, use_body=os.environ.get("FIX_USE_BODY") == "1"):
    """Subject-only by default: bodies cite "Fixes #123" for features too. FIX_USE_BODY=1 is the
    looser variant, kept as a sensitivity check."""
    text = subject + "\n" + body if use_body else subject
    return bool(FIX.search(text)) and not NOT_FIX.search(subject)


def git(repo, *args, binary=False):
    out = subprocess.run(["git", "-C", repo, *args], check=True, capture_output=True).stdout
    return out if binary else out.decode("utf-8", "replace")


def rev_before(repo, date):
    return git(repo, "rev-list", "-1", f"--before={date}", "HEAD").strip()


def write_blobs(repo, entries, root):
    """entries: list of (blob_sha, relpath, dest_dir_name). Writes each blob at root/dest/relpath."""
    proc = subprocess.Popen(
        ["git", "-C", repo, "cat-file", "--batch"], stdin=subprocess.PIPE, stdout=subprocess.PIPE
    )
    for sha, rel, dest in entries:
        proc.stdin.write(sha.encode() + b"\n")
        proc.stdin.flush()
        header = proc.stdout.readline().split()
        if len(header) < 3 or header[1] != b"blob":
            continue
        size = int(header[2])
        data = proc.stdout.read(size)
        proc.stdout.read(1)
        path = os.path.join(root, dest, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "wb") as f:
            f.write(data)
    proc.stdin.close()
    proc.wait()


def run_metrics(binary, root):
    """Runs `anti-slop metrics` over root; returns rows keyed by path relative to root."""
    out = subprocess.run([binary, "metrics", "."], cwd=root, capture_output=True)
    rows = list(csv.DictReader(io.StringIO(out.stdout.decode("utf-8", "replace"))))
    for r in rows:
        r["path"] = os.path.normpath(r["path"])
    return rows


def spans_by_blob(rows):
    """Groups metric rows written under blobs/<sha>/<path> into sha -> [(start, end, name, path)]."""
    out = defaultdict(list)
    for r in rows:
        sha, _, rel = r["path"].partition(os.sep)
        out[sha].append((int(r["start"]), int(r["end"]), r["function"], rel))
    return out


HUNK = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+\d+(?:,\d+)? @@")


def parse_log(text):
    """Parses `git log -p -U0 --full-index` into commits: (sha, [(old_blob, old_path, [lines])])."""
    commits = []
    cur = None
    fileinfo = None
    for line in text.split("\n"):
        if line.startswith("\x01COMMIT "):
            cur = (line.split()[1], [])
            commits.append(cur)
            fileinfo = None
        elif line.startswith("diff --git "):
            fileinfo = [None, None, []]
            cur[1].append(fileinfo)
        elif fileinfo is not None and line.startswith("index "):
            blobs = line.split()[1]
            fileinfo[0] = blobs.split("..")[0]
        elif fileinfo is not None and line.startswith("--- "):
            p = line[4:]
            fileinfo[1] = p[2:] if p.startswith("a/") else None
        elif fileinfo is not None and line.startswith("@@"):
            m = HUNK.match(line)
            if m:
                start, count = int(m.group(1)), int(m.group(2) or "1")
                if count == 0:
                    fileinfo[2].extend([start, start + 1])
                else:
                    fileinfo[2].extend(range(start, start + count))
    return commits


def touched_functions(repo, binary, since, until, tmp):
    """Returns {sha: set((path, name))} for commits in since..until, and the commit messages."""
    meta = git(repo, "log", "--no-merges", "--format=%H%x1f%s%x1f%b%x1e", f"{since}..{until}")
    messages = {}
    for rec in meta.split("\x1e"):
        parts = rec.strip("\n").split("\x1f")
        if len(parts) == 3:
            messages[parts[0]] = (parts[1], parts[2])
    log = git(
        repo, "log", "--no-merges", "-p", "-U0", "--full-index", "--no-renames", "--format=\x01COMMIT %H",
        f"{since}..{until}", "--", "*.rs",
    )
    commits = parse_log(log)
    blobs = {}
    for _, files in commits:
        for old_blob, old_path, _ in files:
            if old_blob and old_path and set(old_blob) != {"0"}:
                blobs[old_blob] = old_path
    root = os.path.join(tmp, "blobs")
    shutil.rmtree(root, ignore_errors=True)
    write_blobs(repo, [(sha, path, sha) for sha, path in blobs.items()], root)
    spans = spans_by_blob(run_metrics(binary, root)) if blobs else {}
    touched = {}
    for sha, files in commits:
        hit = set()
        for old_blob, old_path, lines in files:
            fns = spans.get(old_blob, [])
            for ln in lines:
                inside = [f for f in fns if f[0] <= ln <= f[1]]
                if inside:
                    s, e, name, _ = min(inside, key=lambda f: f[1] - f[0])
                    hit.add((old_path, name))
        touched[sha] = hit
    return touched, messages


def main():
    repo, binary, out_csv = sys.argv[1], os.path.abspath(sys.argv[2]), sys.argv[3]
    snap = rev_before(repo, SNAPSHOT_DATE)
    end = rev_before(repo, END_DATE)
    prior = git(repo, "rev-list", "-1", f"--before={SNAPSHOT_DATE} -{PRIOR_DAYS} days", "HEAD").strip()
    prior_date = subprocess.run(
        ["date", "-d", f"{SNAPSHOT_DATE} -{PRIOR_DAYS} days", "+%Y-%m-%d"], capture_output=True, text=True
    ).stdout.strip()
    prior = rev_before(repo, prior_date)
    with tempfile.TemporaryDirectory() as tmp:
        files = [
            l.split("\t") for l in git(repo, "ls-tree", "-r", snap).splitlines() if l.endswith(".rs")
        ]
        entries = [(meta.split()[2], path, "snap") for meta, path in files]
        write_blobs(repo, entries, tmp)
        snap_rows = run_metrics(binary, os.path.join(tmp, "snap"))

        later, later_msgs = touched_functions(repo, binary, snap, end, tmp)
        before, _ = touched_functions(repo, binary, prior, snap, tmp) if prior else ({}, {})

    fixes, changes, prior_changes = defaultdict(int), defaultdict(int), defaultdict(int)
    n_fix_commits = 0
    for sha, hit in later.items():
        subject, body = later_msgs.get(sha, ("", ""))
        fix = is_fix(subject, body)
        n_fix_commits += fix
        for ident in hit:
            changes[ident] += 1
            if fix:
                fixes[ident] += 1
    for hit in before.values():
        for ident in hit:
            prior_changes[ident] += 1

    seen = defaultdict(int)
    for r in snap_rows:
        seen[(r["path"], r["function"])] += 1
    kept = [r for r in snap_rows if seen[(r["path"], r["function"])] == 1 and r["test"] == "0"]
    name = os.path.basename(os.path.normpath(repo))
    with open(out_csv, "w", newline="") as f:
        fields = ["repo"] + list(kept[0].keys()) + ["later_fixes", "later_changes", "prior_changes"]
        w = csv.DictWriter(f, fieldnames=fields)
        w.writeheader()
        for r in kept:
            ident = (r["path"], r["function"])
            w.writerow({"repo": name, **r, "later_fixes": fixes[ident], "later_changes": changes[ident],
                        "prior_changes": prior_changes[ident]})
    fixed = sum(1 for r in kept if fixes[(r["path"], r["function"])])
    print(f"{name}: snapshot {snap[:10]}, {len(kept)} functions, {len(later)} later commits "
          f"({n_fix_commits} fixes), {fixed} functions fixed, {len(before)} prior commits", file=sys.stderr)


if __name__ == "__main__":
    main()
