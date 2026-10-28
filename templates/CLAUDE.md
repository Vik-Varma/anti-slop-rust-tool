# Working in this repository

Rust code here is judged by `anti-slop`: a linter that reports only patterns with evidence behind
them, and a harness that judges what you changed, not the whole tree.

## Before you say a task is done

Run the harness on your change and read its first line:

```sh
anti-slop guard --since origin/main --format agent
```

`PASS` means nothing blocks. `FAIL` lists items, each with a rule and a `file:line`. Fix every
`block` item, then run it again. Leave `note` items for the reviewer. The hooks run the same
check before a commit, after each Rust edit, and when you stop; treat their output the same way.

## What it reports

- `warning` and `error` findings reproduce a real failure: a panic, undefined behaviour, a
  deadlock, lost data, a build that breaks. Fix them.
- `note` findings are real mechanisms that are usually deliberate. Hidden by default.
- `guard` items are about what your change did to the measuring stick: tests removed or ignored,
  benchmarks or build profiles edited, `target-cpu=native`, suppressions without a reason,
  `todo!()` left behind.

## Rules

1. Never edit a test, benchmark, profile or `anti-slop.toml` to turn `FAIL` into `PASS`. If the
   target can't be reached honestly, stop and say so.
2. A finding that is wrong for this code gets its reason where the code is:
   `// anti-slop: allow(<rule>) <reason of at least 12 characters>`.
   `anti-slop explain <rule>` lists the exceptions the source allows; cite one.
3. Keep changes small enough that `guard` can attribute them. Code lines count; comments don't.
4. Say what you ran and what you didn't. "Should work" is not a result.
