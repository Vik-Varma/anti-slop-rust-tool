# Crate size: what it does to a project, and whether to cap it

Researched 2026-11-04 for a possible guard rail on how big a crate (or file, or change) may get.
Two literature reviews, a measurement of the study data (`study/crate-size.md`), and the Rust
build-time evidence. Grades: **measured** means the source reports numbers from a run;
**opinion** means an expert or a policy asserts it without data.

## The answer

| Question | Finding |
|---|---|
| Do bigger crates have more bugs per line? | No detectable effect in 71,707 functions across 357 crates: odds ratio 0.95 per doubling of crate size as published (p = 0.07), 0.98 with the corrected miner (p = 0.43). File size: 1.02 to 1.03. |
| Is there an optimal component size? | The literature says no. The one famous claim (Hatton's U-shape, "about 300 LOC") was withdrawn by its author in 2009; El Emam et al. (2002) show the other claimed optima are a mathematical artifact and find "no threshold effect". The claimed optima differ by an order of magnitude (83 to 877 lines). |
| Does a cap on size improve outcomes? | No study tests a cap as an intervention. The closest test (Yamashita et al., QRS 2016, Eclipse, Netbeans and industrial systems) found the biggest files had the *lowest* defect density and that "redistribution of Java code into smaller and less complex files may be counterproductive". |
| Does crate size affect build time? | Yes, but through the dependency graph's shape, not the line count. A crate is the unit of coarse parallelism and of single-threaded LLVM work, so one huge crate on the critical path is a bottleneck; splitting a chain gains nothing. |
| Is there a size number to adopt? | None with evidence. Chromium says "prefer small crates" for its own GN/Ninja build graph, with no published data. A forum thread says 1,000 lines is a sweet spot and was disputed. |
| Is there a tool or lint? | No Clippy, Cargo or rustc check limits crate or file size. No public tool enforces a crate-size or compile-time budget; `cargo build --timings` is a report. |

## Defects and size (literature)

- Absolute defect counts rise with size at file and class level; size is the strongest simple
  covariate and has to be controlled (El Emam et al., IEEE TSE 2001).
- Defect *density* is flat or falls as modules get bigger in most studies: Basili & Perricone
  (CACM 1984: 16.0 errors per 1,000 lines at 50 lines, 6.4 above 200, "the larger the module the
  less error prone it was"), Koru et al. (EMSE 2008, TSE 2009), Yamashita et al. (2016),
  Olbrich et al. (ICSM 2010). Fenton & Ohlsson (TSE 2000) and its replication (Galinac Grbac et
  al., TSE 2013) found "no evidence to support previous claims relating module size to fault
  density".
- Process beats size for prediction: change history, ownership and dependency structure predict
  failures better than code metrics (Rahman & Devanbu ICSE 2013; Bird et al. FSE 2011; Zimmermann
  & Nagappan ICSE 2008).
- No study at crate, package or library level tests size against defect density with controls.
  The npm and PyPI micro-package studies measure dependency overhead and breakage risk, not
  defects. There is no crates.io study of crate size.
- Not found: any 2023 to 2026 replication of the size-defect relationship.

## Build and tooling effects (Rust)

- **Parallelism and the critical path** (opinion, expert): "The most important property of a crate
  is which crates it doesn't (transitively) depend on" (matklad, 2021). Cargo's timings guide says
  "Split large crates into smaller pieces" and, for many crates waiting on one, "focus your
  attention on improving that one crate".
- **Measured, splitting helped enormously, once**: a 100k-line *generated* crate took about 27
  minutes, one thread pinned in LLVM; split into 1,106 crates it took about 2.5 minutes (Feldera,
  2025). Hand-written code is not like that.
- **Measured, splitting did not help**: splitting `rustc_mir` slightly hurt bootstrap time on
  perf.rust-lang.org (2021); a bin/lib split made one cold build slightly slower. No clean
  "split X into N, clean build went from A to B" result exists for a mainstream hand-written
  project.
- **Costs of boundaries**: a changed crate reruns every transitive dependent ("Relink, don't
  rebuild" goal, 2025); generics are instantiated in the consumer; each integration test is a
  separate binary and relinks (matklad: consolidating cut test compile time 3x); LTO is off by
  default, so "two small crates may be less optimized than one large one".
- **Costs of many small crates**: release and version overhead (tokio merged most of its crates
  in 2019: "Maintaining many separate crates requires overhead"), the orphan rule forces some
  impls into one crate, and items shared across crates must be `pub`.
- **The community's own view** (Rust Compiler Performance Survey 2025, 3,700 responses): the three
  named bottlenecks are unnecessary rebuilds of dependent workspace crates, slow linking and slow
  single-crate incremental rebuilds. The survey's reading of what respondents said they tried:
  "the most popular (and effective) mechanisms for improving build performance are reducing the
  number of dependencies and their activated features, and splitting larger crates into smaller
  crates". That is self-report, not measurement.
- **Parallel front end**: up to 50% faster on `-Zthreads=8` in the best cases; still not stable in
  the sources found, so one large crate is still processed largely single-threaded.

## What sizes real projects keep

Counted by two methods, which agree. In the 35 study projects (function lines): median crate 613,
p90 7,231, largest 55,729 (naga); cargo's root crate is 54,556. In seven well-known workspaces
counted from a fresh clone (non-blank, non-comment lines): median crate 330 to 1,900 across
tokio, rust-analyzer, bevy, wgpu, cargo, ripgrep and servo, yet each keeps a few crates of
50,000 to 215,000 lines (servo's `script` is over 200k). Files over 1,000 lines are 6 to 17% of
the files in those workspaces.

## Change size is different

The evidence that supports anything numeric is about the size of a *change*, not of a component:

- Google reviewers are "strongly encouraged to make small, incremental changes"; median change
  24 lines, about 90% touch fewer than 10 files (Sadowski et al., ICSE-SEIP 2018). Google's
  engineering practices call "100 lines" reasonable and "1000 lines" usually too large (opinion).
- Review effectiveness was significantly higher for small changes in a 50-participant controlled
  experiment (Baum et al., EMSE 2019). SmartBear's Cisco study suggests 200 to 400 lines per
  review (vendor-published, not peer reviewed).
- Contested: across 845,316 pull requests, size did not affect time-to-merge (Kudrjavets et al.,
  MSR 2022).

## Verdict for the harness

1. **No crate-size or file-size defect rule.** Nothing supports it, and our data and the best
   threshold study point the other way. It belongs in the evidence ledger as a rejected claim.
2. **No fixed crate-size cap on build-time grounds either.** The cost is real but depends on the
   shape of the graph and what is on the critical path, which is measured with
   `cargo build --timings`, not with a line count. A cap would be arbitrary: the median is under
   1,000 lines and well-run projects keep several crates fifty times that.
3. **Change size is the defensible guard.** The harness already reports code lines added per
   change and files over 1,000 lines. Their basis is review practice (Ferrock's `AGENTS.md`, the
   agentic-iteration post's 1,000-line file rule, Google's change-size guidance), not defect
   data, and `docs/HARNESS.md` says so.
4. **If a build-time budget is wanted**, measure time, not lines: record `cargo build --timings`
   or `cargo build --release` wall time in CI and alert on a regression, accepting run-to-run
   noise. No existing tool does this as a gate.

## Sources

Defect literature: Hatton, IEEE Software 1997 (author's page https://leshatton.org/IEEE_Soft_97b.html,
carrying the 2009 disavowal); Basili & Perricone, CACM 1984
(https://ntrs.nasa.gov/api/citations/19870015469/downloads/19870015469.pdf); Fenton & Ohlsson,
TSE 2000; Galinac Grbac et al., TSE 2013; El Emam et al., TSE 2001 and 2002; Yamashita et al.,
QRS 2016; Koru et al., EMSE 2008 and TSE 2009; Olbrich et al., ICSM 2010; Sjøberg et al., TSE
2013; Rahman & Devanbu, ICSE 2013; Nagappan, Murphy & Basili, ICSE 2008; Zimmermann & Nagappan,
ICSE 2008; Abdalkareem et al., FSE 2017 and EMSE 2020; Kula et al., https://arxiv.org/abs/1709.04638;
Zimmermann et al., USENIX Security 2019; Sadowski et al., ICSE-SEIP 2018; Baum et al., EMSE 2019;
Kudrjavets et al., https://arxiv.org/abs/2203.05045.

Rust build evidence: https://doc.rust-lang.org/cargo/reference/timings.html ;
https://matklad.github.io/2021/09/04/fast-rust-builds.html ;
https://matklad.github.io/2021/08/22/large-rust-workspaces.html ;
https://matklad.github.io/2021/02/27/delete-cargo-integration-tests.html ;
https://blog.rust-lang.org/2025/09/10/rust-compiler-performance-survey-2025-results ;
https://blog.rust-lang.org/2023/11/09/parallel-rustc/ ;
https://goals.rust-lang.org/2025h2/relink-dont-rebuild.html ;
https://github.com/rust-lang/rust/issues/80522 ;
https://www.feldera.com/blog/cutting-down-rust-compile-times-from-30-to-2-minutes-with-one-thousand-crates ;
https://github.com/tokio-rs/tokio/issues/1264 ;
https://google.github.io/eng-practices/review/developer/small-cls.html ;
https://rust-unofficial.github.io/patterns/patterns/structural/small-crates.html.

Not re-checkable by script (the page is rendered by JavaScript): the tokio issue quote, which the
research agent read. Not verified: the contents of Koru & Liu 2007 and Syer et al. 2015; the exact USENIX figures; the
Cisco data behind the 200 to 400 line advice; a crates.io-wide size distribution; the
fasterthanli.me numbers (search summary only).
