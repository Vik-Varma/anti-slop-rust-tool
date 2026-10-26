# What "bad code" means here

"Bad code" is used loosely, for anything from undefined behaviour to a style someone dislikes. A
tool can't be useful, or honest, on that basis. This one uses a definition that can be checked.

## The definition

A pattern of Rust code is **bad** when both of these hold:

1. **It has a failure mechanism.** There is a concrete way it makes a program do the wrong thing:
   undefined behaviour, a panic or abort, a deadlock or stall, data silently lost, or a build that
   breaks without the code changing. Either the mechanism is reproduced by a program in
   `experiments/`, or (for the one rule where it can't be) the claim is the Rust project's own
   stated policy. Folklore, taste and "it looks complicated" don't count.
2. **The mechanism is a real risk where the pattern appears.** A check that mostly flags safe
   code teaches people to ignore it. So each rule was run over 37 real codebases and a random
   sample of its findings was checked by hand ([AUDIT.md](AUDIT.md)).

A claimed maintainability problem (long functions, high complexity, TODO comments) can only meet
condition 1 by **measurement**: functions with the pattern must need more later bug fixes than
functions of the same size without it. That was tested on the history of 35 projects, with the
hypotheses written down before the confirming data was collected ([EVIDENCE.md](EVIDENCE.md),
`study/`).

## How a rule earns its severity

| Severity | Shown by default | Requires |
|---|---|---|
| error | yes | A reproduced failure that is undefined behaviour or a deadlock, and audit precision ≥ 50% (or no false positive possible by construction) |
| warning | yes | A reproduced failure, and audit precision ≥ 50% on a validation sample the rule was not tuned on (or no false positive possible by construction) |
| note | with `--min-severity note` | A reproduced failure or Rust-project consensus, but most findings in the audit were deliberate or safe |

The 50% bar is a floor, not a target. Google's Tricorder (Sadowski et al., ICSE 2015,
<https://research.google/pubs/pub43322/>) enforces "a very low effective false positive rate
(< 10%)", puts an analysis whose not-useful rate reaches 10% on probation, and may disable it
above 25%. The four default rules are exact by construction: each flags only the precise
construct its experiment shows failing (`uninit-memory`, `float-eq-inexact`, `deny-warnings`,
`blocking-sleep-in-async`). They also had no instances in shipping code to audit, since mature
projects don't contain them. Every rule with audit data fell short of the 50% bar once a second,
independent rater traced the code: `todo` (4/10 reachable) and `lock-held-across-await` (7/7 real
patterns, 0/7 able to deadlock as written) included. So all of them are notes.

What that means in practice: the default run reports almost nothing on mature, well-reviewed
code, because almost nothing in it is demonstrably bad by a check that can be trusted without type
information. The notes are where review effort goes; each is a real failure mechanism that, in the
audit, was usually guarded by something a syntax check can't see.

## What is deliberately not "bad code" here

These are widely called bad. They fail the definition, and [EVIDENCE.md](EVIDENCE.md) has the
numbers for each:

- **High cyclomatic or cognitive complexity, long functions, deep nesting, many parameters.**
  Each strongly predicts later bug fixes (odds ratios of 2 to 16), but that is because each
  measures size. Once function size is controlled for, none adds anything, and the result held
  in a preregistered replication on 15 new projects. Bigger functions do get more fixes in
  total, but not more fixes per line.
- **TODO/FIXME/HACK comments**, and the "deferral" and "hedging" phrases that LLM-slop linters
  look for. No size-independent link to bug fixes replicated.
- **Magic numbers, `.clone()` calls, boolean parameters, `as` casts in general.** There was no
  consistent effect after size, and magic numbers pointed the other way.
- **`unimplemented!()`**: the standard library documents it as a statement that code won't be
  written, and the audit found it used that way.
- **`mem::forget`**: it can leak, but in the audit every use was a deliberate hand-off of
  ownership.

None of this says such code is pleasant to read. It says the evidence doesn't show it causes
defects, so a tool shouldn't call it bad.

## Limits

- The checks read syntax only: no types, no macro expansion, no cross-crate analysis. Where that
  matters, each rule says what it can mistake (`anti-slop explain <rule>`).
- "Later bug fix" is inferred from commit messages. It misses fixes with unhelpful messages and
  counts some non-fixes; the precision check is in [EVIDENCE.md](EVIDENCE.md).
- Findings that are real but rare in mature projects (undefined behaviour, deadlocks) can't be
  tested for fix-proneness; their evidence is the reproduced failure.
