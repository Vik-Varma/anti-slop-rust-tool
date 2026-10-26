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

MINE_CORRECTED=1 applies the corrections from the study review (docs/reviews/study-review.md):
follow renames (git -M) and map each later path back to its snapshot path, never count a pure move
or a whole-file deletion as a change, credit a pure insertion only to a function it lands inside,
and choose the snapshot, end and prior commits on the mainline (--first-parent). Without it the
original, preregistered method runs unchanged.

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

CORRECTED = os.environ.get("MINE_CORRECTED") == "1"

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
    mainline = ["--first-parent"] if CORRECTED else []
    return git(repo, "rev-list", "-1", *mainline, f"--before={date}", "HEAD").strip()


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
    """Parses `git log -p -U0 --full-index` into commits: (sha, [file]), where each file is
    [old_blob, old_path, lines, new_path, inserts]. `lines` are the old-side lines a hunk deletes
    or edits; `inserts` are the old-side lines after which a pure insertion goes. (The original
    method adds both `s` and `s + 1` of an insertion to `lines` instead.)"""
    commits = []
    cur = None
    fileinfo = None
    for line in text.split("\n"):
        if line.startswith("\x01COMMIT "):
            cur = (line.split()[1], [])
            commits.append(cur)
            fileinfo = None
        elif line.startswith("diff --git "):
            fileinfo = [None, None, [], None, []]
            cur[1].append(fileinfo)
        elif fileinfo is not None and line.startswith("index "):
            blobs = line.split()[1]
            fileinfo[0] = blobs.split("..")[0]
        elif fileinfo is not None and line.startswith("rename from "):
            fileinfo[1] = line[len("rename from "):]
        elif fileinfo is not None and line.startswith("rename to "):
            fileinfo[3] = line[len("rename to "):]
        elif fileinfo is not None and line.startswith("--- "):
            p = line[4:]
            fileinfo[1] = p[2:] if p.startswith("a/") else None
        elif fileinfo is not None and line.startswith("+++ "):
            p = line[4:]
            fileinfo[3] = p[2:] if p.startswith("b/") else None
        elif fileinfo is not None and line.startswith("@@"):
            m = HUNK.match(line)
            if m:
                start, count = int(m.group(1)), int(m.group(2) or "1")
                if count == 0:
                    if CORRECTED:
                        fileinfo[4].append(start)
                    else:
                        fileinfo[2].extend([start, start + 1])
                else:
                    fileinfo[2].extend(range(start, start + count))
    return commits


def touched_functions(repo, binary, since, until, tmp, forward=True):
    """Returns {sha: set((path, name))} for commits in since..until, and the commit messages.
    Paths are as of the snapshot: with renames followed, a later commit's path is mapped back
    through the renames after the snapshot (forward), or a prior commit's path forward through
    the renames before it (not forward)."""
    meta = git(repo, "log", "--no-merges", "--format=%H%x1f%s%x1f%b%x1e", f"{since}..{until}")
    messages = {}
    for rec in meta.split("\x1e"):
        parts = rec.strip("\n").split("\x1f")
        if len(parts) == 3:
            messages[parts[0]] = (parts[1], parts[2])
    renames = ["-M"] if CORRECTED else ["--no-renames"]
    log = git(
        repo, "log", "--no-merges", "-p", "-U0", "--full-index", *renames, "--format=\x01COMMIT %H",
        f"{since}..{until}", "--", "*.rs",
    )
    commits = parse_log(log)
    # git log lists newest first. Going forward from the snapshot, map paths oldest first.
    if CORRECTED and forward:
        commits.reverse()
    blobs = {}
    for _, files in commits:
        for old_blob, old_path, *_ in files:
            if old_blob and old_path and set(old_blob) != {"0"}:
                blobs[old_blob] = old_path
    root = os.path.join(tmp, "blobs")
    shutil.rmtree(root, ignore_errors=True)
    write_blobs(repo, [(sha, path, sha) for sha, path in blobs.items()], root)
    spans = spans_by_blob(run_metrics(binary, root)) if blobs else {}
    touched = {}
    alias = {}  # path in some commit -> path at the snapshot
    for sha, files in commits:
        hit = set()
        if CORRECTED and not forward:
            # Newest first: a rename A -> B seen here means A is B (or later) at the snapshot.
            for _, old_path, _, new_path, _ in files:
                if old_path and new_path and old_path != new_path:
                    alias[old_path] = alias.get(new_path, new_path)
        for old_blob, old_path, lines, new_path, inserts in files:
            if CORRECTED and new_path is None:
                continue  # deleting a whole file isn't a change to its functions
            fns = spans.get(old_blob, [])
            ident_path = alias.get(old_path, old_path) if CORRECTED else old_path
            for ln in lines:
                inside = [f for f in fns if f[0] <= ln <= f[1]]
                if inside:
                    s, e, name, _ = min(inside, key=lambda f: f[1] - f[0])
                    hit.add((ident_path, name))
            for ln in inserts:
                # Inserted after line `ln`: inside a function only if both neighbours are.
                inside = [f for f in fns if f[0] <= ln and ln + 1 <= f[1]]
                if inside:
                    s, e, name, _ = min(inside, key=lambda f: f[1] - f[0])
                    hit.add((ident_path, name))
        if CORRECTED and forward:
            for _, old_path, _, new_path, _ in files:
                if old_path and new_path and old_path != new_path:
                    alias[new_path] = alias.get(old_path, old_path)
        touched[sha] = hit
    return touched, messages


def main():
    repo, binary, out_csv = sys.argv[1], os.path.abspath(sys.argv[2]), sys.argv[3]
    snap = rev_before(repo, SNAPSHOT_DATE)
    end = rev_before(repo, END_DATE)
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
        before, _ = touched_functions(repo, binary, prior, snap, tmp, forward=False) if prior else ({}, {})

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
