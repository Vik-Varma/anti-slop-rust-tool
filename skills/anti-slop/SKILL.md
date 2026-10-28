---
name: anti-slop
description: >
  Check your Rust changes before you say they are done. Use after editing Rust, before a commit,
  and when a review asks whether code is "bad". anti-slop reports only patterns with evidence
  behind them, and reports what a change did to the tests and benchmarks that judge it.
---

# anti-slop

Run it on what you changed, not on the whole repository:

```sh
anti-slop guard --since <base> --format agent    # <base>: the branch you started from
anti-slop guard --staged --format agent          # what the next commit contains
```

Read the first line. `PASS` means nothing blocks. `FAIL` lists, per line, a rule and a
`file:line`; `block` items must be fixed or justified before you finish, `note` items are for the
person who reviews your work.

What it reports, and why it is trusted:

- `warning`/`error` findings reproduce a failure (a panic, UB, a deadlock, data loss, a broken
  build). They are rare in reviewed code and common in new code. Fix them.
- `note` findings are real mechanisms that are often deliberate. Shown with `--min-severity note`.
- `guard` items are not about the code. They are what your change did to the measuring stick:
  tests removed or ignored, benchmarks or profiles edited, `target-cpu=native`, suppressions
  added without a reason, `todo!()` left behind. Each came from a way agents have been seen to
  pass a check without doing the work.

When a finding is wrong for this code, say why where the code is, and the reason will be read:

```rust
// anti-slop: allow(<rule>) <reason of at least 12 characters>
```

`anti-slop explain <rule>` gives the evidence behind a rule and the exceptions its source allows;
cite one of those exceptions in the reason when it applies. Do not suppress to get to `PASS`;
do not edit a test or benchmark to get to `PASS`; if a target can't be reached honestly, stop and
say so.

Do not change `anti-slop.toml` (the policy) unless the person asked for that change.
