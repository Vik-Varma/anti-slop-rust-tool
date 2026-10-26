# What counts as "bad code"

"Bad code" is used loosely. People apply it to everything from undefined
behaviour to brace placement. This tool uses a narrow, testable definition, and
each rule is held to it.

## Definition

A code pattern is **bad code** when all three of these hold:

1. **It does harm.** The pattern causes, or reliably predicts, at least one of:
   - **Correctness**: wrong results, crashes, deadlocks, or undefined behaviour.
   - **Robustness**: failures that go unnoticed, or recoverable errors turned into
     crashes, aborts or lost data.
   - **Performance**: avoidable work with a measurable cost.
   - **Maintainability**: more defects later than the size of the code alone
     would predict.

2. **The harm is backed by evidence** of at least one of these kinds:
   - **Reproduced**: an executable test, a [Miri](https://github.com/rust-lang/miri)
     run or a benchmark in this repository shows the harm. A control version
     using the recommended fix does not show it.
   - **Associated**: a size-controlled empirical study links the pattern to
     defects. That means either our own mining of 22 Rust repositories (68k
     functions, 2.6k bug-fix commits) or peer-reviewed research.
   - **Self-declared**: the code says itself that it is unfinished, as `todo!()`,
     placeholder comments and TODO markers do.

   The mechanism must also be described by an authoritative source: the official
   Rust documentation, the Rust API Guidelines, Clippy, a peer-reviewed study,
   or a documented production incident.

3. **It can be detected precisely.** The pattern can be recognised from source
   code alone (no compilation or type inference). Independent reviewers labelled
   a sample of findings on real code to estimate how often the detector is right
   ([EVIDENCE.md](EVIDENCE.md#precision-on-real-code)).

Patterns that meet all three are **enabled by default**. Patterns that are
widely recommended but failed step 2 are still implemented, as **opt-in**
rules (`--all-rules` or `--enable <id>`), and are labelled with their weak
evidence grade.

## What is *not* bad code here

The fact-checking rejected several popular claims:

| Claim | Why it is not treated as bad code |
|---|---|
| High cyclomatic or cognitive complexity causes bugs | In our data, complexity correlates strongly with size (Spearman 0.71–0.72). Once function size is controlled it predicts nothing (OR per SD 0.98, p = 0.69). Clippy reached the same conclusion and demoted its lint. |
| Long functions are bug-prone | Long functions are 12× more likely to be bug-fixed, but no more likely than their size predicts (size-adjusted OR 1.02). |
| Deep nesting causes bugs | No significant association beyond size (OR per SD 1.00). |
| Duplicated code is bug-prone | Duplicated functions were if anything *less* often bug-fixed (OR 0.67). This agrees with Rahman et al. 2012. |
| `unwrap()` everywhere is a defect | No association with later fixes once size is controlled (OR 1.17, p = 0.24). The narrower case, unwrap inside a function that already returns `Result`, *is* a rule, because there the crash is avoidable. |
| Undocumented `unsafe` blocks are more buggy | Strong consensus (std library policy), but among functions containing `unsafe`, undocumented ones were not more often fixed (OR 0.79, p = 0.65). Opt-in. |
| `.iter().count()` is O(n), use `.len()` | Measured identical (406 ps vs 404 ps). Slice iterators compute `count` from their length. |
| `&Vec<T>` parameters are slower than `&[T]` | No measurable difference. Kept only as an opt-in API-flexibility rule. |
| `rc.clone()` vs `Rc::clone(&rc)` | Identical code. A style preference. |
| Magic numbers, short names, boolean flags | The literature found no direct evidence of harm (see `validation/README.md`). Boolean flags showed no association in our data either. |

## Evidence grades

Every rule carries one of three grades, shown by `anti-slop rules` and
`anti-slop explain <rule>`:

- **strong**: harm reproduced in this repository *and* documented by an
  authoritative source or a peer-reviewed study.
- **moderate**: harm reproduced or documented, but it depends on context. One
  example is a small (≈1.2–1.6×) slowdown. Another is a defect association that
  is real but modest.
- **weak**: recommended by style guides or experts, but not verified. All
  opt-in rules are weak.

## Severity

- **error**: the code is wrong whenever it runs. Examples: a deadlock, undefined
  behaviour, lost data, or `todo!()` reached.
- **warning**: the code is wrong under some conditions, or measurably wasteful.
- **info**: worth a look, with modest expected impact.

## Scope rules

The sources consistently exempt test code from robustness rules. The Rust Book
says that in tests, panicking "is exactly what should happen". So most rules
ignore:
- `#[test]` functions and `#[cfg(test)]` modules, including test modules in
  their own files;
- files in `tests/`, `benches/` and `examples/`, and build scripts;
- crates whose name marks them as test support (e.g. `foo-test-support`).

`float-equality` and `unbounded-send-sync` apply everywhere: an exact float
comparison or an unsound `Send` impl is just as wrong in a test.
