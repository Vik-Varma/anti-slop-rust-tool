#!/usr/bin/env python3
"""Draws a random sample of commits classified as bug fixes / non-fixes by mine.py, for manual checking.

Usage: fix_commit_sample.py REPOS_DIR
"""
import os
import sys, subprocess, random, importlib.util
spec = importlib.util.spec_from_file_location("mine", os.path.join(os.path.dirname(os.path.abspath(__file__)), "mine.py")); mine = importlib.util.module_from_spec(spec); spec.loader.exec_module(mine)
random.seed(42)
recs=[]
import os, glob
for repo in sorted(os.listdir(sys.argv[1])):
    d=os.path.join(sys.argv[1],repo)
    if not os.path.isdir(os.path.join(d,'.git')): continue
    raw=subprocess.run(["git","-C",d,"log","--first-parent","--since=2023-07-01","--until=2025-07-01","--format=%an%x00%s%x00%b%x01"],capture_output=True).stdout.decode('utf-8','replace')
    for r in raw.split('\x01'):
        p=r.strip('\n').split('\x00')
        if len(p)>=3: recs.append((repo,p[0],p[1],p[2]))
fix=[r for r in recs if mine.is_fix(r[2],r[3],r[1])]; non=[r for r in recs if not mine.is_fix(r[2],r[3],r[1])]
print(len(recs), len(fix))
print("--- classified FIX"); [print(' ', r[0][:12], '|', r[2][:110]) for r in random.sample(fix,30)]
print("--- classified NON-FIX"); [print(' ', r[0][:12], '|', r[2][:110]) for r in random.sample(non,30)]
