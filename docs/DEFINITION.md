# What "bad code" means here

"Bad code" is used loosely, for anything from undefined behaviour to a style someone dislikes. A
tool can't be useful, or honest, on that basis. This one uses a definition that can be checked.

## The definition

A pattern of Rust code is **bad** when both of these hold:

1. **It has a failure mechanism.** There is a concrete way it makes a program do the wrong thing:
   undefined behaviour, a panic or abort, a deadlock or stall, data silently lost, or a build that
   breaks without the code changing. Either the mechanism is reproduced by a program in
   `experiments/`, or (for the one rule where it can't be) the Rust project requires it of its own
   compiler and standard library. Folklore, taste and "it looks complicated" don't count.
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
| style | with `--min-severity style` | Not bad code by this definition: a preference stated by Google's Rust style guide, provided for teams that follow it. Each rule quotes the guide's reason and encodes the exceptions the guide lists ([STYLE.md](STYLE.md)) |

The 50% bar is a floor, not a target. Google's Tricorder (Sadowski et al., ICSE 2015,
<https://research.google/pubs/pub43322/>) enforces "a very low effective false positive rate
(< 10%)", puts an analysis whose not-useful rate reaches 10% on probation, and may disable it
above 25%. The five default rules are exact by construction: each flags only the precise
construct its experiment shows failing (`uninit-memory`, `float-eq-inexact`, `deny-warnings`,
`blocking-sleep-in-async`, `catch-unwind-under-panic-abort`). Four had no instances in shipping
code to audit, since mature projects don't contain them; the fifth had one, judged real. Every
rule with audit data fell short of the 50% bar once a second,
independent rater traced the code: `todo` (6/15 reachable, census) and `lock-held-across-await` (7/7 real
patterns, 0/7 able to deadlock as written) included. So all of them are notes.

What that means in practice: the default run reports almost nothing on mature, well-reviewed
code, because almost nothing in it is demonstrably bad by a check that can be trusted without type
information. The notes are where review effort goes; each is a real failure mechanism that, in the
audit, was usually guarded by something a syntax check can't see.

## What is deliberately not "bad code" here

These are widely called bad. They fail the definition, and [EVIDENCE.md](EVIDENCE.md) has the
numbers for each:

- **High cyclomatic or cognitive complexity, long functions, complexity thresholds.** Each
  strongly predicts later bug fixes before adjustment (odds ratios of 1.7 to 12), mostly because
  each measures size. Once function size is controlled for, no effect is detectable: the
  preregistered replication on 15 new projects held for all of them, and with the corrected miner
  the pooled interval for cyclomatic and cognitive complexity excludes anything above about 1.1
  per doubling. Functions over 100 lines even came out less fix-prone than their size predicts,
  which says the size control fits badly at the top, not that long functions are good. (This is
  about defects. Cognitive complexity is meant to measure understandability, which Muñoz Barón et
  al., ESEM 2020, found it partly does; that question isn't tested here.)
- **Deep nesting, many parameters, TODO/FIXME/HACK comments.** These kept a small effect beyond
  size. Pooled over all 35 projects: TODO comments odds ratio 1.41, nesting 1.15 and parameter
  count 1.10 per doubling, with standard errors clustered by repository. Comment lines in a
  function behaved the same way (1.13). The preregistered replication failed to confirm TODO
  comments (H1) and nesting (H2), but it had only about 37% power to, so that result is
  inconclusive, not a refutation. Even taken at face value the effects are small (a TODO comment
  counts for about as much as 40% more lines), comments may simply mark code already known to be
  tricky, and none has a failure to reproduce. The "deferral" and "hedging" phrases that LLM-slop
  linters look for showed nothing detectable, but they were too rare to rule much out.
- **Magic numbers, `.clone()` calls, boolean parameters, `as` casts in general.** There was no
  consistent effect after size, and magic numbers pointed the other way.
- **`unimplemented!()`**: unlike `todo!()`, which "conveys an intent of implementing the
  functionality later", the standard library says it "makes no such claims", and the audit found
  it marking unsupported features and mocks.
- **`mem::forget`**: it can leak, but in the audit every use was a deliberate hand-off of
  ownership.

- **Style-guide rules with no failure behind them.** Google's Rust style guide was read claim by
  claim ([EVIDENCE.md](EVIDENCE.md)). The one rule with a reproducible failure became
  `catch-unwind-under-panic-abort`. Its import, naming, macro, `Rc`/`Arc`, documentation and
  formatting rules are preferences; they exist here as opt-in style rules that say so
  ([STYLE.md](STYLE.md)), not as findings of bad code.

None of this says such code is pleasant to read. It says the evidence doesn't show it causes
defects, so a tool shouldn't call it bad.

## Limits

- The checks read syntax only: no types, no macro expansion, no cross-crate analysis. Where that
  matters, each rule says what it can mistake (`anti-slop explain <rule>`).
- "Later bug fix" is inferred from commit messages. It misses fixes with unhelpful messages and
  counts some non-fixes; the precision check is in [EVIDENCE.md](EVIDENCE.md).
- Findings that are real but rare in mature projects (undefined behaviour, deadlocks) can't be
  tested for fix-proneness; their evidence is the reproduced failure.
- The audit judged code that had already passed review and shipped, which flatters no rule: what
  survives review is what reviewers decided was fine. Google found developers rated 74% of
  issues flagged at compile time "real problems", against 21% of those found in checked-in code
  (Sadowski et al., CACM 2018). The notes' precision on new, unreviewed code is likely higher than
  the audit shows; that hasn't been measured.
- The 50% bar and Google's 10% aren't the same measure. Google counts developers' "not useful"
  clicks on fresh results; this audit is an auditor's judgment of whether old findings can fail.
