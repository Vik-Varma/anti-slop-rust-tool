# Redundant code from AI agents: what is known, and what the tool can do

Researched 2026-11-04, prompted by a real case. An AI-assisted pull request to SteelMC (#763)
added `data.motion = [0.0; 3];` to saved player data "like a teleport", but every path ended in a
`spawn()` that overwrites velocity before anything reads it. The line had no effect. It also read
as if it enforced the doc's promise that velocity "is always cleared", though it covered one of
three cases and the guarantee lived in `spawn()`. It survived because a reviewer's mutation test
(delete the line) failed nothing and was noted, not acted on; the finding was parked as "optional
cleanup"; and when asked "are you sure?" the agent defended it as a "safeguard" without listing
which cases it covered.

Three research runs and one prototype. Tags: **measured** = the source reports numbers from a
run; **opinion** = guidance without data. "Own runs" are the research agent's, on rustc 1.97 and
Clippy 0.1.97; they are reproducible but not re-run here.

## What is known about AI-written redundancy

- **Measured, indirect.** Successful patches in SWE-bench Verified are larger than developers':
  the median of 28 repair approaches makes 121.78% more total changes, and prompting for
  minimality "can hardly" reduce it; a separate refinement pass after generation cut the excess
  from +242% to +4% while keeping the fix rate (arXiv 2608.13292). Agent pull requests carry about
  1.87x the semantic redundancy of human ones in one repository, crewAI, 617 PRs (arXiv
  2601.21276, MSR 2026). 36% of agent commits add mocks to tests against 26% for non-agents (arXiv
  2602.00409), a proxy for tests that cannot fail. GitClear (a vendor) reports copy/pasted lines
  rising from 8.3% to 12.3% over 2020 to 2024; its counts track the era of AI adoption, not
  AI-authored lines.
- **Not found.** No study measures an ineffectual write (a value overwritten before any read) in
  agent code, and none measures *over*-defensive checks against a human baseline. The measured
  direction for robustness is the opposite: about 35% of LLM outputs were *less* robust than
  reference code, mostly from missing checks (arXiv 2503.20197, snippet only). Over-defensiveness
  is anecdote.
- **Harm of dead stores, in general.** CWE-563 says a dead store "could be an indication of a bug
  in the program or a deprecated variable that was not removed"; CERT MSC13-C: "The presence of
  unused values may indicate significant logic errors". Those are warnings, not measurements. Xie
  & Engler (TSE 2003) found redundant operations correlate with hard errors (snippet only; the
  numbers need checking before use). That the *misleading-guarantee* harm is real is argument.

## Can static analysis see it?

- **rustc and Clippy cannot, on stable (own runs).** `unused_assignments` fires for a dead local or
  a whole-struct overwrite, and is silent for `d.x = 1; d.x = 2;`, for tuple fields and array
  elements, and for a write that a callee overwrites (`self.x = 5; self.reset();`). Clippy has no
  overwritten-store lint; the nearest, `field_reassign_with_default`, targets only
  `Default::default()` and for a style reason. rust-lang/rust#49256 (field dead stores, open since
  2018) and #65467 (arrays and fields, open) record the gap; neither has a decision.
- **Other languages mostly stop at locals.** Go `ineffassign`: "assignments to struct fields are
  never marked as ineffectual". staticcheck's SA4005 source says it deliberately does not flag
  `a.x = 1; a.x = 2`. clang's `DeadStores` is locals only and exempts defensive constants and
  nulling ("This is a common form of defensive programming"). SpotBugs and Sonar are locals plus
  unread fields. PMD's `UnusedAssignment` tracks fields of `this` within a class and warns of
  false positives in concurrent code.
- **Closest prior art: CodeQL `js/useless-assignment-to-property`** (precision: high): same base
  and property, the second write post-dominating the first, and no read or impure expression
  between them. That is what this tool's `overwritten-assignment` does for Rust.
- **The callee case has no mainstream tool.** It needs a per-function must-write summary composed
  at call sites (LLVM's `initializes` attribute is exactly that). CodeQL and others avoid it by
  treating any call as a possible read. No published false-positive rate exists.

## Mutation testing

- **`cargo-mutants` 27.1.0 cannot delete a plain statement or assignment.** Its operators are
  function-body replacement, binary and unary operator swaps, match-arm deletion, guard
  replacement, and deleting struct-literal fields that have a base expression (read from its book
  and source). Statement deletion is an open request (issues #180, #487). So `--in-diff` would not
  have flagged `data.motion = [0.0; 3];`. Its docs treat a missed mutant as a test gap or an
  "undistinguishable" equivalent; they never say it may mean redundant code.
- **Who does delete statements.** Google's system has a statement-block-removal operator, runs on
  changed lines only, and shows mutants in code review: developers first judged 85% of reported
  mutants unproductive; more than a hundred suppression rules (logging, allocation sizes, caches,
  sleeps) raised the productive share from 15% to 89% (Petrović et al., TSE 2022). Findings are
  not blocking unless a reviewer marks them. PIT's optional member-variable mutator removes
  assignments and warns it "is likely to create equivalent mutations" when a member holds its
  default, which is this case.
- **Equivalent mutants are most survivors.** In one study of C programs, 946 of 1,230 survivors
  (77%) were equivalent; none were due to unreachable code (Yao, Harman, Jia, ICSE 2014). A dead
  write like this one is an equivalent mutant of the "state never infected" or "unobservable" kind.
- **Rust practice.** rust-bitcoin, rust-payjoin and ebman fail a pull request on any missed mutant
  in the diff, with a commented exclusion list; most others run it advisory. "Every surviving
  mutant is killed or justified" is the exception, not the norm. Pitfalls recorded in their
  workflows: a wrong diff prefix gives exit 0 with no mutants (a silent pass), empty and test-only
  diffs also pass, and skip attributes let an agent silence the gate.
- **For agent patches**, nothing evaluates a mutation gate; the evidence is about LLM-written
  *tests* (MuTAP, MUTGEN; coverage 100% with a mutation score of 4% in one case).

## Agents defending their own claims

- **Measured.** Models change a correct answer when challenged ("wrongly admits mistakes on 98% of
  questions", Sharma et al. 2023) and also fail to see their own errors: a 64.5% "self-correction
  blind spot", cut 89.3% by appending "Wait" (arXiv 2507.02778, synthetic errors); a bias that
  boosts confidence in an answer already given (arXiv 2507.03120). Without external feedback,
  self-correction does not help and changes right answers to wrong ones about as often as the
  reverse (Huang et al., ICLR 2024; Kamoi et al. 2024). With external tools (a code interpreter,
  a test) it works (CRITIC).
- **So** a verbal "are you sure?" reply is unreliable either way, and the fix the evidence
  supports is an experiment: delete the line and run the tests, or write the failing test. That
  protocol itself has not been tested.

## Rules and prompts

- **Measured, on SWE-bench pass rate, not on redundancy.** Across 679 rule files, negative
  constraints ("do not refactor unrelated code") were beneficial and positive directives harmful,
  but random rules did as well as curated ones, and context files did not generally improve
  success and raised cost by over 20% (arXiv 2604.11088; arXiv 2602.11988). Prompt-only minimality
  was weak and a post-generation pass worked (above).
- **Opinion.** Anthropic's prompting guide warns that Opus 4.5/4.6 "overengineer" and suggests
  "Don't add error handling, fallbacks, or validation for scenarios that can't happen", and its
  Claude Code guide notes that "A reviewer that's asked to find problems will always find
  *something*" and recommends a fresh reviewer that "hasn't seen your reasoning".

## One place for a guarantee

Meyer (1992): put a condition in the precondition or in an `if` in the body "but never in both";
redundant "just in case" code "only contributes to the software's complexity". Hunt and Thomas'
DRY is about knowledge, not text. "Parse, don't validate" and the Rust API Guidelines (C-VALIDATE:
static, then dynamic, then `debug_assert!`, then unchecked) say the same for types. The only
measured drift evidence is about code clones (Juergens et al., ICSE 2009: inconsistent changes to
clones caused "a significant number of faults"), not duplicated enforcement of an invariant.

## What the tool can do

| Option | Catches | Cost and limits | Evidence |
|---|---|---|---|
| `overwritten-assignment` (built) | adjacent overwrite or whole-value replace, in one block | zero hits in 38 codebases; exact but rare; would not have caught the motivating case | CWE-563; CodeQL JS precedent |
| Statement-deletion probe (not built): delete each added assignment in a scratch copy, run the tests, report survivors | the motivating case, if the suite can see velocity | one test run per statement; Google needed 100+ suppression rules to get from 15% to 89% productive; equivalents are common; runs project code | Google SBR (measured on human code); nothing on agent patches |
| `cargo mutants --in-diff` | changed logic: swapped operators, deleted arms, replaced bodies | misses statement deletion; ~2 GB and minutes per run; silent-pass pitfalls | used and gated by several Rust projects |
| Same-crate must-write lint (not built) | `x.f = e;` then a uniquely-named method that writes `f` first | no precedent or false-positive data; blind to data moved into another struct, as in the motivating case | none published |
| Write-only private fields (not built) | a private field only ever assigned | rustc is silent; derive, serde and macros read fields invisibly | SpotBugs `URF_UNREAD_FIELD` |
| Process rules for the agent | "decide every finding", "list the cases before calling code a safeguard", "answer a challenge with an experiment" | free; effect on redundancy unmeasured | partly measured, mostly opinion |

Not recommended: a lint on guarantee wording in doc comments ("always", "never" must name an
enforcer). It is opinion-backed and noisy on prose.

## Sources

Papers and pages: arXiv 2608.13292, 2601.21276, 2602.00409, 2503.20197, 2507.02778, 2507.03120,
2310.13548, 2310.01798, 2406.01297, 2305.11738, 2604.11088, 2602.11988, 2102.11378 (Google), 2103.07189;
Yao, Harman & Jia, ICSE 2014; Meyer, "Applying Design by Contract", IEEE Computer 1992;
https://cwe.mitre.org/data/definitions/563.html ;
https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html ;
https://github.com/rust-lang/rust/issues/49256 ; https://github.com/rust-lang/rust/issues/65467 ;
https://mutants.rs/mutants.html ; https://github.com/sourcefrog/cargo-mutants/issues/180 ;
https://github.com/gordonklaus/ineffassign ;
https://github.com/github/codeql/blob/main/javascript/ql/src/Declarations/DeadStoreOfProperty.ql ;
https://pitest.org/quickstart/mutators/ ;
https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices ;
https://code.claude.com/docs/en/best-practices ;
https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/ ;
https://rust-lang.github.io/api-guidelines/dependability.html .

Not verified: Xie & Engler's numbers and Romano et al.'s dead-code study (publisher pages were
unreachable); Sonar's S1854 text (the rule pages did not load); SpotBugs, PMD and clang details
were read from their docs by a research agent and not re-fetched here; the rustc/Clippy results
are the agent's own runs.
