#!/usr/bin/env python3
"""Fetches every web page the rule catalogue and docs cite, and checks that each says what it is
cited for (a regex per claim). Run it to re-verify the sources; docs/EVIDENCE.md records the last
run. Papers that are PDFs or behind DOIs are checked separately (see EVIDENCE.md, Sources).

Usage: check_sources.py
"""
import re, html, subprocess, sys
CHECKS = [
 ("https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html", [r"more information than the compiler", r"prototype", r"tests"]),
 ("https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html", [r"production-quality code[^.]*expect"]),
 ("https://doc.rust-lang.org/edition-guide/rust-2024/static-mut-references.html", [r"[^.]*static_mut_refs lint level is now deny by default[^.]*\."]),
 ("https://doc.rust-lang.org/reference/expressions/operator-expr.html", [r"truncate"]),
 ("https://doc.rust-lang.org/std/io/struct.BufWriter.html", [r"[^.]*errors[^.]*drop[^.]*\.", r"[^.]*flush[^.]*before[^.]*drop[^.]*\."]),
 ("https://doc.rust-lang.org/std/macro.dbg.html", [r"[^.]*release builds[^.]*\."]),
 ("https://doc.rust-lang.org/std/macro.todo.html", [r"[^.]*unimplemented![^.]*\."]),
 ("https://doc.rust-lang.org/std/macro.unimplemented.html", [r"[^.]*todo![^.]*\."]),
 ("https://doc.rust-lang.org/std/mem/fn.uninitialized.html", [r"[^.]*undefined behavior[^.]*\.", r"Deprecated"]),
 ("https://doc.rust-lang.org/std/mem/union.MaybeUninit.html", [r"[^.]*initialization invariant[^.]*\.", r"array"]),
 ("https://doc.rust-lang.org/std/ops/trait.Drop.html", [r"[^.]*double panic[^.]*abort[^.]*\."]),
 ("https://doc.rust-lang.org/std/process/fn.exit.html", [r"[^.]*destructors[^.]*\."]),
 ("https://docs.oracle.com/cd/E19957-01/806-3568/ncg_goldberg.html", [r"What Every Computer Scientist"]),
 ("https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html", [r"[^.]*ability to keep it locked across an \.await point[^.]*\."]),
 ("https://docs.rs/tokio/latest/tokio/index.html", [r"[^.]*blocking code that would otherwise block other tasks from running[^.]*\."]),
 ("https://rust-lang.github.io/api-guidelines/dependability.html", [r"C-DTOR-FAIL", r"[^.]*Destructors[^.]*panicking[^.]*\.", r"[^.]*abort[^.]*\."]),
 ("https://rust-lang.github.io/rfcs/1105-api-evolution.html", [r"[^.]*[Ll]int[^.]*minor[^.]*\.", r"[^.]*new lint[^.]*\."]),
 ("https://rust-unofficial.github.io/patterns/anti_patterns/deny-warnings.html", [r"[^.]*new[^.]*lint[^.]*\."]),
 ("https://ryhl.io/blog/async-what-is-blocking/", [r"[^.]*thread::sleep[^.]*\.", r"[^.]*10 to 100 microseconds[^.]*\."]),
 ("https://std-dev-guide.rust-lang.org/policy/safety-comments.html", [r"SAFETY", r"[^.]*unsafe block[^.]*\."]),
 ("https://www.usenix.org/conference/osdi14/technical-sessions/presentation/yuan", [r"[^.]*simple testing on error handling code[^.]*\."]),
 ("https://cwe.mitre.org/data/definitions/252.html", [r"Unchecked Return Value", r"[^.]*does not check the return value[^.]*\."]),
 ("https://cwe.mitre.org/data/definitions/197.html", [r"Numeric Truncation Error", r"[^.]*truncat[^.]*\."]),
 ("https://research.google/pubs/pub43322/", [r"Tricorder"]),
 ("https://blog.cloudflare.com/18-november-2025-outage/", [r"called Result::unwrap\(\) on an Err value", r"[^.]*more than 200 features[^.]*\."]),
]
missing = 0
for url, pats in CHECKS:
    r = subprocess.run(["curl","-sSL","-A","Mozilla/5.0 (anti-slop source check)","--max-time","40",url], capture_output=True)
    t = r.stdout.decode("utf-8","replace")
    t = re.sub(r"<script.*?</script>|<style.*?</style>", " ", t, flags=re.S)
    t = re.sub(r"\s+", " ", html.unescape(re.sub(r"<[^>]+>", " ", t)))
    print(f"\n=== {url}  ({len(t)} chars, exit {r.returncode})")
    for p in pats:
        m = re.search(p, t, re.I)
        missing += not m
        print(f"  [{'OK ' if m else 'MISSING'}] {p[:40]}: {m.group(0)[:300] if m else ''}")
# Clippy's lint list is one page; each lint's section names its group.
CLIPPY = {"unwrap_used": "restriction", "dbg_macro": "restriction", "undocumented_unsafe_blocks": "restriction",
          "await_holding_lock": "suspicious", "unused_io_amount": "correctness", "cognitive_complexity": "restriction"}
r = subprocess.run(["curl", "-sSL", "--max-time", "60", "https://rust-lang.github.io/rust-clippy/master/index.html"],
                   capture_output=True)
page = r.stdout.decode("utf-8", "replace")
print("\n=== Clippy lint groups")
for lint, group in CLIPPY.items():
    at = page.find(f'id="{lint}"')
    section = re.sub(r"\s+", " ", re.sub(r"<[^>]+>", " ", page[at:at + 600])) if at >= 0 else ""
    ok = at >= 0 and group in section.split("What it does")[0]
    missing += not ok
    print(f"  [{'OK ' if ok else 'MISSING'}] {lint}: {group}")
print(f"\n{missing} claim(s) not found")
sys.exit(1 if missing else 0)
