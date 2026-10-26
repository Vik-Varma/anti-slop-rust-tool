# Fact-checking popular "bad code" claims against the empirical literature

Prepared 2026-10-26 for a Rust linter project. Every number or quote below was taken from a page
fetched during this session. The source line for each item says how it was fetched:

- **PDF**: the full-text PDF was downloaded and searched with `pdftotext`.
- **abs**: the abstract was read from the publisher page, arXiv, or an institutional repository page.
- **CR**: the Crossref record (`api.crossref.org`), which confirms only that the paper exists, plus
  its title, venue, year and DOI. It says nothing about the content.
- **SE**: the text came from a web-search extract of the named page, because the page itself
  blocked direct download. Treat it as weaker than abs.
- **UNVERIFIED**: I could not fetch the content. I give the claim from background knowledge only
  so it can be checked later. Do not cite it as checked.

Access problems during this session: the Semantic Scholar API answered HTTP 429 to every request
(more than 20 attempts). OpenAlex's shared daily budget was used up. Springer (`link.springer.com`)
and the ACM DL returned bot walls or 403. As a result, the Springer EMSE papers (Khomh 2012,
Palomba 2018, Kapser & Godfrey 2008, Gil & Lalouche 2017) are verified only through Crossref or
secondary pages.

Verdict scale: SUPPORTED / SUPPORTED BUT CONFOUNDED BY SIZE / WEAK / CONTRADICTED / OPINION ONLY.

---

## 1. Does cyclomatic complexity (McCabe 1976) predict defects beyond size?

**Claim.** High cyclomatic complexity (CC) marks defect-prone code, independently of how long the
code is.

**Origin.** McCabe, "A Complexity Measure", IEEE TSE 2(4), 1976, doi:10.1109/TSE.1976.233837 (CR).
McCabe proposed the metric to measure testability and control-flow complexity. It was not
validated against defect data.

**Evidence that size explains the effect**

- El Emam, Benlarbi, Goel, Rai, "The confounding effect of class size on the validity of
  object-oriented metrics", IEEE TSE 27(7), 2001, doi:10.1109/32.935855 (CR). Abstract text (SE,
  via the summary at <https://neverworkintheory.org/2011/07/07/the-confounding-effect-of-class-size-on-the-validity-of-object-oriented-metrics.html>):
  "before controlling for size, the results are very similar to previous studies... After
  controlling for size none of the metrics we studied were associated with fault-proneness
  anymore."
  - Caveat: this study covers OO class metrics in one C++ telecom system, not CC on functions.
  - Counterpoint: a published rebuttal exists. Evanco, "Comments on 'The confounding effect of class
    size on the validity of object-oriented metrics'", IEEE TSE 2003, doi:10.1109/TSE.2003.1214331
    (CR). The rebuttal's argument is UNVERIFIED. From memory, it objects to treating size as a
    confounder rather than a mediator.
- Gil & Lalouche, "On the correlation between size and metric validity", Empirical Software
  Engineering, 2017, doi:10.1007/s10664-017-9513-5 (CR only; content UNVERIFIED). A Technion seminar
  listing for Lalouche's thesis work (SE, <https://www.cs.technion.ac.il/events/2017/2684/>) says
  "most metrics are correlated with the size of the code module. However, it isn't clear whether or
  not these metrics offer any substantial benefit over a simple size measurement."
- Herraiz & Hassan, "Beyond lines of code: do we need more complexity metrics?", ch. 8 of *Making
  Software*, O'Reilly 2010. UNVERIFIED: it has no Crossref record and I did not fetch the chapter.
  From memory, it reports very high correlations between CC and SLOC once the data are
  log-transformed.

**Evidence that CC is not just size**

- Landman, Serebrenik, Bouwers, Vinju, "Empirical analysis of the relationship between CC and SLOC
  in a large corpus of Java methods and C functions", J. Software: Evolution and Process, 2016
  (online December 2015), doi:10.1002/smr.1760 (abs, via the Crossref record). The study covers
  17.6 M Java methods and 6.3 M C functions. Quote: "linear correlation between SLOC and CC is only
  moderate as a result of increasingly high variance... the observed linear correlation between CC
  and SLOC of Java methods or C functions is not strong enough to conclude that CC is redundant
  with SLOC. This conclusion contradicts earlier claims from literature."
  - Note: this paper tests whether CC is redundant with size. It does not test defects. It argues
    against redundancy, so it is evidence against the "CC = size" position, not for it.

**Verdict: SUPPORTED BUT CONFOUNDED BY SIZE.**

- CC correlates with defects. The best-known size-controlled study (El Emam) found that the effect
  vanishes once size is controlled.
- CC is not literally redundant with SLOC (Landman). Whether the leftover variation predicts
  defects is not established by any source I could fetch.

## 2. Cognitive complexity (Campbell 2018) and its validation

**Claim.** SonarSource's Cognitive Complexity measures understandability better than CC.

**Origin.** Campbell, "Cognitive complexity: an overview and evaluation", Proc. TechDebt 2018,
doi:10.1145/3194164.3194186 (CR). It is a design rationale written by the tool vendor. The metric's
increments were chosen by argument, not fitted to data. In itself it is OPINION ONLY.

**Evidence for**

- Muñoz Barón, Wyrich, Wagner, "An Empirical Validation of Cognitive Complexity as a Measure of
  Source Code Understandability", ESEM 2020, doi:10.1145/3382494.3410636 (CR). The abstract is on
  arXiv: <https://arxiv.org/abs/2007.12520> (abs).
  - Method: a meta-analysis of "about 24,000 understandability evaluations of 427 code snippets".
  - Results, quoted: "Cognitive Complexity positively correlates with comprehension time and
    subjective ratings of understandability. The metric showed mixed results for the correlation
    with the correctness of comprehension tasks and with physiological measures."
  - Conclusion, quoted: "the first validated and solely code-based metric which is able to reflect
    at least some aspects of code understandability."

**Evidence against or limits**

- The same abstract reports mixed results for correctness and physiological measures.
- The studies it pools use small snippets. I could not verify whether the meta-analysis controlled
  for snippet size (UNVERIFIED).
- I found no fetched study linking Cognitive Complexity to defects beyond size.
- Lenarduzzi, Kilamo, Janes, "Does Cyclomatic or Cognitive Complexity Better Represents Code
  Understandability?", SSRN preprint 2023, doi:10.2139/ssrn.4397231 (CR only; content UNVERIFIED).

**Verdict.**

- **WEAK** (partially validated) as an understandability measure, which is what it claims to be.
- **OPINION ONLY** as a defect predictor: no fetched evidence either way.

## 3. Long methods, large files, long parameter lists, deep nesting and fault- or change-proneness

**Claim.** Code smells such as Long Method, Large/God/Complex Class and Long Parameter List make
code more fault- and change-prone.

**Evidence for**

- Palomba, Bavota, Di Penta, Fasano, Oliveto, De Lucia, "On the diffuseness and the impact on
  maintainability of code smells: a large scale empirical investigation", EMSE 23, 2018 (online
  August 2017), doi:10.1007/s10664-017-9535-z (CR). Abstract from the TU Delft repository page
  (abs, <https://research.tudelft.nl/en/publications/on-the-diffuseness-and-the-impact-on-maintainability-of-code-smel>):
  - Scope: "395 releases of 30 open source projects and considering 17,350 manually validated
    instances of 13 different code smell kinds."
  - Results: "smells characterized by long and/or complex code (e.g., Complex Class) are highly
    diffused, and that smelly classes have a higher change- and fault-proneness than smell-free
    classes."
  - The task asks two further questions: what fraction of smells related to faults, and whether size
    was controlled. Both are UNVERIFIED, because I could not reach the full text (Springer
    blocked, and the IRIS UniSA and TU Delft PDF mirrors returned 503 or reset the connection).
    The abstract mentions no size control. The smells it reports as most diffused are
    size-defined, so size confounding is likely and should be checked in the full paper.
- Khomh, Di Penta, Guéhéneuc, Antoniol, "An exploratory study of the impact of antipatterns on class
  change- and fault-proneness", EMSE 17, 2012 (online August 2011), doi:10.1007/s10664-011-9171-y
  (CR only). Content UNVERIFIED. From memory, the study covers 13 antipatterns over 54 releases of
  four Java systems and finds classes with antipatterns more change- and fault-prone. It also
  includes some check on whether size alone explains this. Verify before citing.

**Evidence against or limits**

- The El Emam size-confound result (Q1) applies directly, because Long Method, Large Class, Long
  Parameter List and Complex Class are all size proxies.
- I found no fetched study that isolates nesting depth from size.

**Verdict: SUPPORTED BUT CONFOUNDED BY SIZE.** This rests on Palomba's abstract plus the general
size-confound literature. Whether these studies controlled for size is UNVERIFIED.

## 4. Self-admitted technical debt (TODO/FIXME)

**Claim.** TODO/FIXME/HACK comments mark defect-prone code.

**Evidence**

- Potdar & Shihab, "An exploratory study on self-admitted technical debt", ICSME 2014. Content
  UNVERIFIED (no fetch succeeded). Wehaibi et al. confirm the paper's role in their PDF: "a study by
  Potdar and Shihab [5] introduced a new way to identify technical debt through source code
  comments". From memory: SATD appears in about 2.4% to 31% of files, and 26% to 63% of it is
  removed later.
- Wehaibi, Shihab, Guerrouj, "Examining the impact of self-admitted technical debt on software
  quality", SANER 2016, pp. 179–188, doi:10.1109/SANER.2016.72. The author PDF was fetched (PDF,
  <https://faculty.ksu.edu.sa/sites/default/files/examining_the_impact_of_self-admitted_technical_debt_on_software_quality.pdf>).
  - Scope: five projects (Hadoop, Chromium, Cassandra, Spark, Tomcat).
  - Defects, quoted: "there is no clear trend when it comes to defects and self-admitted technical
    debt, although the defectiveness of the technical debt files increases after the introduction
    of technical debt". Also quoted: "In some of the studied projects, self-admitted technical debt
    files have more bug-fixing changes, while in other projects, files without it had more defects."
  - Future defects, quoted: "self-admitted technical debt changes induce less future defects than
    none technical debt changes". The same summary also reports that SATD changes are harder to make.
- Later and stronger, but size-confounded: "Evidence is All We Need: Do Self-Admitted Technical
  Debts Impact Method-Level Maintenance?", MSR 2025, arXiv 2411.13777 (abs,
  <https://arxiv.org/abs/2411.13777>).
  - Scope: "774,051 methods from 49 open-source projects".
  - Quoted: "methods containing SATD are not only larger and more complex but also exhibit lower
    readability and a higher tendency for bugs and changes."
  - The abstract itself says that SATD methods are larger, which is the same size confound as Q1.
- Yuan et al. 2014 (Q8) found TODO/FIXME inside error handlers in only "2%" of catastrophic
  failures (Figure 5 of the PDF).

**Verdict: WEAK.** The file-level result is inconsistent (Wehaibi). The method-level link exists
but the SATD methods are bigger, and I found no fetched result that adjusts for size.

## 5. Code clones

**Claim.** Duplicated code causes bugs.

**Evidence for**

- Juergens, Deissenboeck, Hummel, Wagner, "Do Code Clones Matter?", ICSE 2009, pp. 485–495,
  doi:10.1109/ICSE.2009.5070547. Full text fetched from the arXiv reposting (PDF,
  <https://arxiv.org/pdf/1701.05472>).
  - Scope: three commercial C# systems, one Cobol system and one open-source Java system. The
    authors "manually inspected about 900 clone groups".
  - Results, quoted: "inconsistent changes to clones are very frequent" and "nearly every second
    unintentionally inconsistent change to a clone leads to a fault." Elsewhere the paper says
    "about every second to third".
  - Note: the claim is about unintentionally inconsistent changes to clones, not about clones as
    such. The systems were not randomly sampled ("we relied on our connections with the developers
    of the systems").

**Evidence against**

- Kapser & Godfrey, "'Cloning considered harmful' considered harmful: patterns of cloning in
  software", EMSE 13, 2008, doi:10.1007/s10664-008-9076-6 (CR only). Content UNVERIFIED. From
  memory, the paper catalogues deliberate cloning patterns (forking, templating, customisation) and
  finds that a large share of clones, around 71% in their sample, were judged to have a positive or
  neutral effect on maintainability. Verify before citing.

**Verdict.**

- **SUPPORTED** for *inconsistently changed* clones.
- **WEAK or contested** for "clones are bad" in general.
- This needs clone-genealogy tracking, not a single-snapshot syntax check.

## 6. Magic numbers, commented-out code, dead code

- **Magic numbers.** An arXiv API search ("magic numbers empirical study code quality") returned
  nothing relevant. I found no fetched empirical study of defects or comprehension.
  **OPINION ONLY.** My search was limited, so this is "none found", not "none exists".
- **Commented-out code.** The same arXiv search returned nothing, and I found no fetched study.
  **OPINION ONLY** (none found).
- **Dead code.** Romano, Vendome, Scanniello, Poshyvanyk, "A Multi-Study Investigation into Dead
  Code", IEEE TSE 46(1), 2020, doi:10.1109/TSE.2018.2842781 (SE: abstract via a search extract of
  the publisher and repository listings; full text not fetched).
  - Method: interviews with practitioners plus four controlled experiments.
  - Result: dead code is "harmful in the maintenance and evolution phases" (comprehensibility and
    maintainability). Effect sizes are UNVERIFIED.
  - Eder et al., ICSE 2012 ("How much does unused code matter for maintenance?") was not fetched
    (UNVERIFIED).
  - **WEAK** (an experimental comprehension effect, with no defect link shown).
  - In Rust, rustc's `dead_code` lint already warns on unused items by default.

## 7. Rust-specific bug studies

- Qin, Chen, Yu, Song, Zhang, "Understanding Memory and Thread Safety Practices and Issues in
  Real-World Rust Programs", PLDI 2020, doi:10.1145/3385412.3386036. Full text fetched (PDF,
  <https://songlh.github.io/paper/rust-study.pdf>).
  - Scope: "850 unsafe code usages and 170 bugs".
  - Where blocking bugs live, quoted: "all blocking bugs we studied are in safe code" and "55 out of
    59 blocking bugs are caused by operations of synchronization primitives, like Mutex and Condvar."
  - Double locks, quoted: "Failing to acquire lock (for Mutex) or read/write (for RwLock) results in
    thread blocking for 38 bugs, with 30 of them caused by double locking".
  - Guard lifetimes, quoted: "a lot of the concurrency bugs in Rust are caused by programmers'
    misunderstanding of Rust's (complex) lifetime and safety rules". The paper's worked example is
    `match connect(client.read().unwrap().m)`, where the read guard lives for the whole `match`. The
    fix binds the result to a variable first.
  - Suggestion 7, quoted: "Rust should add an explicit unlock API of Mutex, since programmers may
    not save the return value of lock() in a variable".
  - The authors' static double-lock and use-after-free detectors "found ten previously unknown bugs".
- Xu et al., "Memory-Safety Challenge Considered Solved? An In-Depth Study with All Rust CVEs", ACM
  TOSEM 2021. Abstract from arXiv 2003.03296 (abs, <https://arxiv.org/abs/2003.03296>); the TOSEM
  venue and DOI were not fetched.
  - Scope: "186 real-world bug reports... all existing Rust CVEs... of memory-safety issues by
    2020-12-31".
  - Results, quoted: "all memory-safety bugs require unsafe code, and many memory-safety bugs in our
    dataset are mild soundness issues". The bug categories are "automatic memory reclaim, unsound
    function, and unsound generic or trait".
- Bae et al., "Rudra: Finding Memory Safety Bugs in Rust at the Ecosystem Scale", SOSP 2021,
  doi:10.1145/3477132.3483570. Full text fetched (PDF, <https://taesoo.kim/pubs/2021/bae:rudra.pdf>).
  - Results, quoted: "264 previously unknown memory safety bugs—leading to 76 CVEs and 112 RustSec
    advisories", "51.6% of memory safety bugs reported to RustSec since 2016", and the bugs are
    "often made by Rust experts".
  - Precision: Rudra "generated 2,390 reports and inspected them all at a rough rate of 150 reports
    per man-hour. Most false positives were filtered out at a glance". The precision was therefore
    low, but triage was cheap.
- Astrauskas, Matheja, Poli, Müller, Summers, "How Do Programmers Use Unsafe Rust?", OOPSLA 2020
  (PACMPL 4, art. 136). The abstract was read from the SPLASH page (abs,
  <https://2020.splashcon.org/details/splash-2020-oopsla/12/How-Do-Programmers-Use-Unsafe-Rust->):
  "Our study supports the Rust hypothesis partially: While most unsafe code is simple and
  well-encapsulated, unsafe features are used extensively, especially for interoperability with
  other languages."
- Evans, Campbell, Soffa, "Is Rust Used Safely by Software Developers?", ICSE 2020. Abstract from
  arXiv 2007.00752 (abs, <https://arxiv.org/abs/2007.00752>). Quoted: "software engineers use the
  keyword unsafe in less than 30% of Rust libraries, but more than half cannot be entirely
  statically checked by the Rust compiler because of Unsafe Rust hidden somewhere in a library's
  call chain."

**Verdict: SUPPORTED** for two claims:

- Memory-safety bugs in Rust are concentrated in `unsafe` code (Xu, Rudra).
- Blocking bugs in Rust are mostly double locks in safe code, driven by misunderstood guard
  lifetimes (Qin).

None of these studies tests whether `// SAFETY:` comments reduce bugs. That remains **OPINION
ONLY**: it is a project convention.

## 8. Ignored errors

- Yuan et al., "Simple Testing Can Prevent Most Critical Failures", OSDI 2014. Full text fetched
  (PDF, <https://www.usenix.org/system/files/conference/osdi14/osdi14-paper-yuan.pdf>).
  - Scope: 198 sampled user-reported failures in Cassandra, HBase, HDFS, MapReduce and Redis, of
    which 48 (24%) were catastrophic (Table 2).
  - The 92% figure, quoted: "almost all (92%) of the catastrophic system failures are the result of
    incorrect handling of non-fatal errors explicitly signaled in software."
  - The 35% figure, quoted: "in 35% of the catastrophic failures, the faults in the error handling
    code fall into three trivial patterns: (i) the error handler is simply empty or only contains a
    log printing statement, (ii) the error handler aborts the cluster on an overly-general
    exception, and (iii) the error handler contains expressions like 'FIXME' or 'TODO'".
  - Figure 5 breakdown of catastrophic failures: errors ignored 25%, abort in over-caught
    exceptions 8%, "TODO" in handler 2%.
  - Also quoted: "an error handler that only logs the error is also considered as ignoring the
    error"; "in 58% of the catastrophic failures, the underlying faults could easily have been
    detected through simple testing of error handling code"; "In all but one case, the errors were
    checked by the developers."
  - The developers did check errors and then mishandled them. That makes the finding about
    *handlers that discard*, not unchecked errors. Rust's `let _ = result;` and `.ok();` are direct
    analogues of an empty handler.
  - Limits: five Java/C distributed data systems; catastrophic failures only (n = 48). For
    comparison, "only 25% of the non-catastrophic failures in our study involve incorrect error
    handling".
- CWE-252 (Unchecked Return Value) and CWE-391 (Unchecked Error Condition): MITRE pages not
  fetched, so UNVERIFIED. These are taxonomy entries, not empirical evidence.

**Verdict: SUPPORTED** for catastrophic failures in distributed systems. Its transfer to Rust is
plausible but untested.

## 9. Static-analysis false-positive tolerance

- Sadowski, van Gogh, Jaspan, Söderberg, Winter, "Tricorder: Building a Program Analysis Ecosystem",
  ICSE 2015, doi:10.1109/ICSE.2015.76. Full text fetched (PDF,
  <https://static.googleusercontent.com/media/research.google.com/en//pubs/archive/43322.pdf>).
  - Definition, quoted: "We define an effective false positive as any report from the tool where a
    user chooses not to take action to resolve the report."
  - Threshold, quoted: "We still enforce a very low effective false positive rate here (< 10%)."
  - Footnote, quoted: "This 10% false positive threshold matches that used by other analysis
    platforms such as Coverity."
  - Escalation: an analyzer above the threshold goes "on probation", and "If the rate goes above
    25%, we may decide to tu[rn it off]".
  - Scope: results are shown only "on changed lines by default".
- Sadowski, Aftandilian, Eagle, Miller-Cushon, Jaspan, "Lessons from Building Static Analysis Tools
  at Google", CACM 61(4), 2018, doi:10.1145/3188720. The CACM site returned 403; the full text was
  read from an MIT course copy (PDF, <https://6826.csail.mit.edu/2020/papers/google-analysis-cacm.pdf>).
  - Code review, quoted: results shown during code review "are allowed to include up to 10%
    effective false positives".
  - Compiler checks must "Produce less than 10% effective false positives. Developers should feel
    the check is pointing out an actual issue at least 90% of the time".
  - Disabling, quoted: "If the ratio for an analyzer goes above 10%, the Tricorder team disables the
    analyzer until the author(s) improve it."
  - Survivor effect, quoted: developers rated "74% of the issues flagged at compile time as 'real
    problems,' compared to 21% of those found in checked-in code."
  - FindBugs fixit: "a significant fraction were not important enough to fix in practice."
  - Counting fixes, from the paper's definition: a fix made only for readability is not an
    effective false positive, but a correct report the developer misunderstands is one.
- Johnson, Song, Murphy-Hill, Bowdidge, "Why Don't Software Developers Use Static Analysis Tools to
  Find Bugs?", ICSE 2013. The abstract was read on Google Research (abs,
  <https://research.google/pubs/why-dont-software-developers-use-static-analysis-tools-to-find-bugs/>).
  The author PDF returned 404. Quoted: "We conducted interviews with 20 developers and found that
  although all of our participants felt that use is beneficial, false positives and the way in
  which the warnings are presented, among other things, are barriers to use."

**Verdict: SUPPORTED** that high false-positive rates kill adoption. The 10% figure is Google's
operating policy (matched by Coverity), not a measured threshold of user tolerance. Johnson et al.
is qualitative (n = 20).

---

## Summary table

| # | Claim | Verdict |
|---|---|---|
| 1 | Cyclomatic complexity predicts defects | SUPPORTED BUT CONFOUNDED BY SIZE (CC is not redundant with SLOC per Landman, but no fetched evidence of defect prediction beyond size) |
| 2 | Cognitive complexity measures understandability | WEAK (partially validated: time and ratings yes, correctness and physiology mixed); OPINION ONLY for defects |
| 3 | Long/large/many-parameter/deeply nested code is fault-prone | SUPPORTED BUT CONFOUNDED BY SIZE (size control in Palomba and Khomh UNVERIFIED) |
| 4 | TODO/FIXME marks defect-prone code | WEAK |
| 5 | Clones cause bugs | SUPPORTED for unintentionally inconsistent clone changes; WEAK for clones in general |
| 6 | Magic numbers / commented-out code / dead code | OPINION ONLY / OPINION ONLY / WEAK |
| 7 | Rust memory bugs need `unsafe`; Rust deadlocks come from guard lifetimes | SUPPORTED |
| 8 | Ignored errors cause severe failures | SUPPORTED (distributed Java systems) |
| 9 | Developers abandon tools with many false positives; aim for under 10% | SUPPORTED (policy and qualitative evidence) |

## Disagreements with the repository

I read `/home/user/anti-slop-rust-tool/docs/EVIDENCE.md` and `docs/DEFINITION.md` after collecting
all of the evidence above. I did not modify them.

1. **Landman et al. is misread.** EVIDENCE.md says Landman et al. (2016) "found" the size confound
   "between McCabe complexity and lines of code". Their abstract says the reverse: the correlation
   is "only moderate" and "not strong enough to conclude that CC is redundant with SLOC. This
   conclusion contradicts earlier claims from literature." Landman should be cited as
   counter-evidence to "CC is just size", or dropped. Herraiz & Hassan is the redundancy source;
   I could not verify that chapter.

2. **"None adds anything" overstates the null.** DEFINITION.md says that once size is controlled,
   "none adds anything". The repo's own table disagrees for two metrics:
   - Nesting depth: OR 1.18 [1.07, 1.29] in discovery and 1.11 [0.97, 1.27] in replication.
   - Parameter count: OR 1.13 [1.04, 1.22] in discovery and 1.06 [0.95, 1.19] in replication.

   The point estimates are above 1 in both corpora. "Not replicated" or "small and uncertain" is
   accurate; "nothing" is not. The repo itself warns that "'No evidence' is not 'evidence of no
   effect'", but only applies that to rare patterns.

3. **"Odds ratios of 2 to 16" doesn't match the table.** The naive ORs in EVIDENCE.md run from 1.74
   (cognitive complexity) to 12.07 (more than 100 lines). No 16 appears anywhere.

4. **The adjustment may be over-adjustment, and the counter-argument is missing.** The models
   adjust for log(LOC) *and* log(prior churn). Complexity plausibly drives churn, and size may
   mediate complexity rather than confound it. If so, adjusting for either can absorb a real effect.
   - Evanco's published rebuttal of El Emam (IEEE TSE 2003, doi:10.1109/TSE.2003.1214331, existence
     verified via Crossref) is not cited.
   - The ">100 lines" OR of 0.66, adjusted for log(LOC), is a threshold of size adjusted for size.
     It is hard to interpret, and should not be read as "protective".

5. **Cognitive complexity is dismissed on the wrong outcome.** The repo tests fix-proneness, but
   Cognitive Complexity claims to measure understandability. On understandability, Muñoz Barón,
   Wyrich & Wagner (ESEM 2020) found positive correlations with comprehension time and subjective
   ratings across about 24,000 evaluations. The repo doesn't cite this. "Deliberately not bad" is
   justified for defect claims, but not as a blanket statement. The Clippy quote is about Clippy's
   implementation, not the metric's validity.

6. **The TODO lead from Yuan is thinner than presented.** EVIDENCE.md says Yuan's 35% "suggests a
   narrower hypothesis worth testing" for TODOs. In Yuan's Figure 5, a "TODO" in a handler accounts
   for only 2% of 48 catastrophic failures, which is about one failure. Ignored errors account for
   25%. The repo also leaves out the closer SATD literature, which supports its null:
   - Wehaibi et al. 2016 found "no clear trend".
   - The method-level MSR 2025 study found SATD methods "larger and more complex", i.e. size-confounded.

7. **Yuan is unused where it is strongest.** Yuan's 25% "errors ignored" figure, counting log-only
   handlers as ignoring, is direct evidence for `ignored-io-error`. That rule cites only its own
   experiment and CWE-252. Yuan also defines an error handler that only logs as ignoring the error,
   which could widen the rule to `if let Err(e) = … { log(e) }` when the code then continues.

8. **The audit measures the survivor population.** Sadowski et al. (CACM 2018) found developers
   rated 74% of compile-time findings as real problems but only 21% of findings in checked-in code.
   The repo's audit samples mature, checked-in code. Its precision numbers therefore systematically
   understate a rule's value at authoring time, which is when a linter runs. This is a threat to
   validity that EVIDENCE.md doesn't list, and it bears on demoting every audited rule to "note".

9. **The 50% bar is compared with Google's 10% on different definitions.**
   - Google's "effective false positive" means the developer took no action. A readability fix
     counts as useful.
   - The repo's audit counts a finding as true only if the failure can really occur.
   - The two percentages are not comparable, and the citation implies more alignment than exists.
   - The CACM paper is stricter than the 2015 Tricorder description the repo quotes: "If the ratio
     for an analyzer goes above 10%, the Tricorder team disables the analyzer". It is not cited.

10. **The repo has no rule for the double-lock pattern, which has strong Rust-specific evidence.**
    Qin et al. (PLDI 2020) found 30 of 38 lock-related blocking bugs were double locks, often from
    guard temporaries living through a `match` scrutinee. All of the blocking bugs were in safe code.
    The repo's `lock-held-across-await` audit found 0/7 deadlock-able cases. A same-scope re-lock
    or scrutinee-temporary rule is better supported by bug data, and Qin's static double-lock
    detector found previously unknown bugs. I did not check whether Clippy already has a lint for
    this.

11. **The repo cites no clone evidence.** Juergens et al. (ICSE 2009) found that "nearly every
    second unintentionally inconsistent change to a clone leads to a fault". This is one of the
    better-supported "bad code" findings in the literature. It needs history, so DEFINITION.md's
    syntax-only scope excludes it, but EVIDENCE.md should list it among the claims it considered.

12. **Unsafe-related evidence is uncited.** Xu et al. found "all memory-safety bugs require unsafe
    code". Rudra found 264 bugs, 51.6% of RustSec memory-safety bugs. Evans et al. found unsafe
    reachable in "more than half" of crates. These justify concentrating rules on `unsafe`.
    `unsafe-without-comment` is correctly labelled consensus, since no study tests whether SAFETY
    comments help.

13. **Minor provenance issues.**
    - DEFINITION.md cites Tricorder via `research.google/pubs/pub43322/`. The quoted text is in the
      PDF (archive/43322.pdf), which I confirmed.
    - Herraiz & Hassan has no DOI or Crossref record, so the claim that "papers were checked against
      their publisher records (Crossref DOIs)" can't cover it.
