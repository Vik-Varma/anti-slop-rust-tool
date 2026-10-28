//! The rule catalogue: what each rule flags, why that is bad, and the evidence. Only claims that
//! survived the checks in docs/EVIDENCE.md are here; the ones that didn't are listed there too.

mod style;

use crate::Severity;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    /// Wrong results, crashes or undefined behaviour.
    Correctness,
    /// Errors that vanish, so a failure goes unnoticed.
    ErrorHandling,
    /// Deadlocks and stalls.
    Concurrency,
    /// Code that breaks other people's builds.
    Maintainability,
    /// A published style guide's preference; no failure behind it.
    Style,
}

impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Correctness => "correctness",
            Category::ErrorHandling => "error-handling",
            Category::Concurrency => "concurrency",
            Category::Maintainability => "maintainability",
            Category::Style => "style",
        }
    }
}

/// How the claim behind a rule was established (docs/DEFINITION.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evidence {
    /// The failure was reproduced in `experiments/`, and an authoritative source describes it.
    Demonstrated,
    /// Authoritative sources agree on it, but it is a review practice: there is no failure to
    /// reproduce, and the defect study found no measurable effect either way.
    Consensus,
    /// A style guide says so (Google's Rust style guide). Not "bad code" by docs/DEFINITION.md:
    /// off by default, and each rule records the guide's own exceptions.
    Style,
}

impl Evidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Evidence::Demonstrated => "demonstrated",
            Evidence::Consensus => "consensus",
            Evidence::Style => "style",
        }
    }
}

#[derive(Debug)]
pub struct Rule {
    pub id: &'static str,
    pub category: Category,
    pub severity: Severity,
    /// One line: what it flags.
    pub summary: &'static str,
    /// The failure it causes, as demonstrated (see `experiments/`).
    pub harm: &'static str,
    /// What the check can get wrong without type information.
    pub false_positives: &'static str,
    pub evidence: Evidence,
    /// Hand-checked findings from the corpus audit that were what the rule says they are.
    pub precision: &'static str,
    /// The exceptions the source lists, and whether the check honours each by itself or needs a
    /// suppression comment naming it. Empty for rules whose source lists none.
    pub exceptions: &'static str,
    pub sources: &'static [&'static str],
}

/// Every rule: the evidence-backed ones, then the style rules.
pub fn all() -> impl Iterator<Item = &'static Rule> {
    RULES.iter().chain(style::STYLE_RULES.iter())
}

pub fn get(id: &str) -> Option<&'static Rule> {
    all().find(|r| r.id == id)
}

const BOOK_PANIC: &str = "Rust Book ch. 9.3, To panic! or Not to panic!: https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html";
const BOOK_EXPECT: &str = "Rust Book ch. 9.2 (\"In production-quality code, most Rustaceans choose expect rather than unwrap\"): https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html";
const API_DTOR: &str = "Rust API Guidelines C-DTOR-FAIL, destructors never fail: https://rust-lang.github.io/api-guidelines/dependability.html#c-dtor-fail";
const GOOGLE_PANIC: &str = "Google Rust style guide, Reporting errors (\"Use `panic!` to report bugs and violated invariants\"; recoverable failures are `Result`): https://google.github.io/styleguide/rustguide.html";
const GOOGLE_EXPECT: &str = "Google Rust style guide, Prefer `expect` over `unwrap` (\"to document the reason\"): https://google.github.io/styleguide/rustguide.html";

static RULES: &[Rule] = &[
    Rule {
        id: "panic-on-input",
        category: Category::Correctness,
        severity: Severity::Note,
        summary: "`.unwrap()` on parsing, decoding, file, env or socket results (`.expect(\"why\")` states a check and isn't flagged)",
        harm: "These fail because of something outside the program: a malformed file, a missing \
               variable, a non-UTF-8 path, a refused connection. Unwrapping turns that into a crash \
               instead of an error the caller can report. Cloudflare's 18 November 2025 outage was an \
               unwrap on the result of loading a generated feature file that had grown past a limit.",
        false_positives: "Input the program wrote itself moments before, or that a previous step validated.",
        evidence: Evidence::Demonstrated,
        precision: "validation sample, two raters (docs/AUDIT.md): 5 of 20 crash on real external input (non-UTF-8 paths and arguments, env vars, dump files, process output); the rest were validated data or dev tooling",
        exceptions: "",
        sources: &[
            BOOK_PANIC,
            BOOK_EXPECT,
            GOOGLE_PANIC,
            "Cloudflare, 18 November 2025 outage: https://blog.cloudflare.com/18-november-2025-outage/",
            "experiments/tests/panics.rs: unwrap_panics_without_context, a_panic_poisons_shared_state",
        ],
    },
    Rule {
        id: "unwrap",
        category: Category::Correctness,
        severity: Severity::Note,
        summary: "Any other `.unwrap()` in shipping code",
        harm: "A failed unwrap panics with a message that names no cause (\"called `Option::unwrap()` \
               on a `None` value\"), and a panic while a lock is held poisons it for every thread. Where \
               the code knows more than the compiler, the Rust Book's advice is `expect` with the reason \
               written out, and it says most Rustaceans choose `expect` over `unwrap` in production code. \
               Most unwraps in the audit were such invariants, and the defect study found unwraps don't \
               mark fix-prone functions, so this is a note, not a warning.",
        false_positives: "An invariant the reader can see. Not flagged: tests, examples, build scripts, `const` \
                          code, `lock().unwrap()`, unwraps of literals and of `write!` (formatting into a \
                          `String` can't fail).",
        evidence: Evidence::Demonstrated,
        precision: "validation sample, two raters (docs/AUDIT.md): 2 of 20 can fail at run time; the rest asserted invariants the code maintains, or were dev-only",
        exceptions: "",
        sources: &[
            BOOK_PANIC,
            BOOK_EXPECT,
            GOOGLE_EXPECT,
            "Clippy unwrap_used (restriction): https://rust-lang.github.io/rust-clippy/master/index.html#unwrap_used",
            "study/results.md, study/replication.md: H7",
        ],
    },
    Rule {
        id: "todo",
        category: Category::Correctness,
        severity: Severity::Note,
        summary: "`todo!()` in shipping code",
        harm: "It type-checks as any type, so unfinished code compiles and ships, then panics with \"not \
               yet implemented\" when the path is reached. `unimplemented!()` isn't flagged: the std \
               docs contrast it with `todo!()`, which \"conveys an intent of implementing the functionality \
               later\", while `unimplemented!` \"makes no such claims\". In the audit it marked unsupported \
               backend features and mocks, not unfinished work.",
        false_positives: "None for the panic itself.",
        evidence: Evidence::Demonstrated,
        precision: "census, two raters (docs/AUDIT.md): 6 of 15 reachable; the rest sat on branches nothing can reach, i.e. `todo!()` used as `unreachable!()`",
        exceptions: "",
        sources: &[
            "std::todo: https://doc.rust-lang.org/std/macro.todo.html",
            "experiments/tests/panics.rs: todo_compiles_and_panics_at_runtime",
        ],
    },
    Rule {
        id: "dbg-macro",
        category: Category::Correctness,
        severity: Severity::Note,
        summary: "`dbg!` in shipping code",
        harm: "`dbg!` behaves the same in release builds: it writes file, line and value to stderr. Left \
               in by accident, that leaks internals into output that tools parse. The audit found it \
               mostly used on purpose, behind a debug flag, so this is a note.",
        false_positives: "Deliberate debug output, e.g. gated on an environment variable.",
        evidence: Evidence::Demonstrated,
        precision: "first audit: 1 of 10 accidental; 9 deliberate debug output behind a flag",
        exceptions: "",
        sources: &[
            "std::dbg: https://doc.rust-lang.org/std/macro.dbg.html",
            "Clippy dbg_macro (restriction): https://rust-lang.github.io/rust-clippy/master/index.html#dbg_macro",
        ],
    },
    Rule {
        id: "panic-in-drop",
        category: Category::Correctness,
        severity: Severity::Note,
        summary: "`unwrap`, `expect`, `panic!` or `unreachable!` inside `Drop::drop`, unless guarded by `thread::panicking()`",
        harm: "Destructors run during unwinding. A panic in one while another panic unwinds aborts the whole \
               process: `catch_unwind` never gets control and no other destructor runs. Verified: SIGABRT \
               with \"panic in a destructor during cleanup\".",
        false_positives: "An assertion of an internal invariant that only fails if the program is already broken.",
        evidence: Evidence::Demonstrated,
        precision: "validation sample, two raters (docs/AUDIT.md): 1 of 10 can realistically fire (an io_uring syscall); the rest asserted invariants or were deliberate",
        exceptions: "",
        sources: &[
            API_DTOR,
            "std::ops::Drop, panics: https://doc.rust-lang.org/std/ops/trait.Drop.html",
            "experiments/tests/panics.rs: panic_in_drop_during_unwind_aborts",
        ],
    },
    Rule {
        id: "catch-unwind-under-panic-abort",
        category: Category::Correctness,
        severity: Severity::Warning,
        summary: "`catch_unwind` in a package whose workspace profile sets `panic = \"abort\"`",
        harm: "With `panic = \"abort\"` the process ends at the panic, so `catch_unwind` never returns `Err` and whatever that arm was for (an error report, a fallback, a cleanup) doesn't happen. A test suite can't show it: Cargo builds tests with unwinding whatever the profile says, so the arm runs under `cargo test` and never in the shipped binary.",
        false_positives: "The strategy is read as Cargo reads it, from the workspace root's Cargo.toml and a `.cargo/config.toml` beside it; a config file elsewhere, a `CARGO_PROFILE_*_PANIC` variable or a `--config` flag can change it and isn't seen. Proc-macro crates, build scripts and tests are exempt, as Cargo ignores the setting for them. A `catch_unwind` kept only as defence in depth that nothing relies on is still flagged, since it still can't run. A local function that happens to be named `catch_unwind` is taken for std's.",
        evidence: Evidence::Demonstrated,
        precision: "exact by construction given the profile; 1 instance in the corpus (tauri's `packages/cli` reports a CLI panic to Node through a callback, in a workspace whose release profile aborts first), judged real",
        exceptions: "",
        sources: &[
            "Cargo Book, Profiles, `panic` (\"Tests, benchmarks, build scripts, and proc macros ignore the `panic` setting\"; \"Cargo only looks at the profile settings in the `Cargo.toml` manifest at the root of the workspace\"): https://doc.rust-lang.org/cargo/reference/profiles.html#panic",
            "std::panic::catch_unwind, Notes (\"This function only catches unwinding panics, not those that abort the process\"): https://doc.rust-lang.org/std/panic/fn.catch_unwind.html#notes",
            "Google Rust style guide, Don't call `panic::catch_unwind` (\"will only function correctly in environments with `panic=unwind`\"; Chrome, Android and Fuchsia build with `panic=abort`): https://google.github.io/styleguide/rustguide.html",
            "experiments/panic-abort, run by experiments/tests/panics.rs: catch_unwind_does_not_catch_under_panic_abort",
        ],
    },
    Rule {
        id: "static-mut",
        category: Category::Correctness,
        severity: Severity::Note,
        summary: "A `static mut` item",
        harm: "Every access is unsafe and nothing stops two threads, or two references, touching it at once: \
               a data race, which is undefined behaviour. Miri reports a data race on a two-thread counter. \
               Taking a reference to it while it can be written \"has always been instantaneous undefined \
               behavior, even if the reference is never read from or written to\" (Edition Guide), and \
               Rust 2024 denies such references by default (`static_mut_refs`).",
        false_positives: "FFI tables a C library writes to (Python module definitions); a cell type or \
                          `&raw mut` access still says that in the code.",
        evidence: Evidence::Demonstrated,
        precision: "10 of 10 deliberate (Python FFI tables, an atomic-guarded global); no data race found",
        exceptions: "",
        sources: &[
            "Edition Guide, static mut references: https://doc.rust-lang.org/edition-guide/rust-2024/static-mut-references.html",
            "experiments/miri: bad_static_mut_race (UB) vs good_atomic_counter (clean)",
        ],
    },
    Rule {
        id: "uninit-memory",
        category: Category::Correctness,
        severity: Severity::Error,
        summary: "`mem::uninitialized()` or `MaybeUninit::uninit().assume_init()`",
        harm: "Reading uninitialised memory is undefined behaviour for integers, references, bools, enums: \
               nearly every type. Miri reports UB for both forms on a `u32`; std aborts the process for a \
               reference.",
        false_positives: "`MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init()`, the documented sound \
                          form, isn't flagged.",
        evidence: Evidence::Demonstrated,
        precision: "no instances in the corpus",
        exceptions: "",
        sources: &[
            "std::mem::uninitialized: https://doc.rust-lang.org/std/mem/fn.uninitialized.html",
            "std::mem::MaybeUninit, initialization invariant: https://doc.rust-lang.org/std/mem/union.MaybeUninit.html",
            "experiments/miri: bad_assume_init, bad_mem_uninitialized; experiments/tests/memory.rs",
        ],
    },
    Rule {
        id: "unsafe-without-comment",
        category: Category::Correctness,
        severity: Severity::Note,
        summary: "An `unsafe` block or `unsafe impl` with no comment justifying it",
        harm: "`unsafe` moves the proof obligation from the compiler to the author. Without the written \
               argument a reviewer can't check it, and a later edit can silently break it. Rust's compiler \
               and standard library require it of their own code (\"each unsafe block should have a SAFETY: \
               comment\"), and Clippy offers it as a restriction lint. It is a review practice: the defect \
               study found no significant effect, so it is a note.",
        false_positives: "A justification further away than the lines just above (e.g. one comment for a \
                          group of `unsafe impl`s).",
        evidence: Evidence::Consensus,
        precision: "20 of 20 had no justification nearby; none was shown to be unsound",
        exceptions: "",
        sources: &[
            "Clippy undocumented_unsafe_blocks (restriction): https://rust-lang.github.io/rust-clippy/master/index.html#undocumented_unsafe_blocks",
            "Standard library dev guide, safety comments: https://std-dev-guide.rust-lang.org/policy/safety-comments.html",
            "Google Rust style guide, Unsafe code (\"Add a safety comment starting with `SAFETY:` that explains why the code is correct\"): https://google.github.io/styleguide/rustguide.html",
        ],
    },
    Rule {
        id: "lock-held-across-await",
        category: Category::Concurrency,
        severity: Severity::Note,
        summary: "A sync lock guard (`lock()`, `read()`, `write()`) alive at an `.await`",
        harm: "While the task is suspended it still holds the lock; any other task on that thread that calls \
               `lock()` blocks the thread, so the holder can never resume to release it. Verified deadlock \
               on a current-thread Tokio runtime; the same code with `tokio::sync::Mutex` finishes.",
        false_positives: "A `lock()` method on a type that isn't a lock. Guards moved elsewhere aren't tracked.",
        evidence: Evidence::Demonstrated,
        precision: "census, two raters (docs/AUDIT.md): 7 of 7 really hold a sync guard across `.await`, 0 of 7 can deadlock as written (nothing else takes the lock meanwhile); 6 were in tests",
        exceptions: "",
        sources: &[
            "Clippy await_holding_lock (suspicious): https://rust-lang.github.io/rust-clippy/master/index.html#await_holding_lock",
            "Tokio Mutex docs, which mutex to use: https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html",
            "experiments/tests/concurrency.rs: std_lock_across_await_deadlocks",
        ],
    },
    Rule {
        id: "blocking-sleep-in-async",
        category: Category::Concurrency,
        severity: Severity::Warning,
        summary: "`thread::sleep` inside async code",
        harm: "An async task must yield at `.await`; `thread::sleep` parks the whole worker thread instead, \
               stalling every task scheduled on it. Measured on a current-thread runtime: a 5 ms ticker \
               ran 0 times during a 200 ms `thread::sleep`, and kept ticking through `tokio::time::sleep`.",
        false_positives: "A sleep of a few microseconds does little harm; Tokio's guidance is a rule of thumb \
                          (no more than 10 to 100 microseconds between `.await`s). Tests, and sync closures \
                          (e.g. inside `spawn_blocking`), aren't flagged.",
        evidence: Evidence::Demonstrated,
        precision: "no instances outside tests in the corpus; all 10 in tests were deliberate, hence tests are exempt",
        exceptions: "",
        sources: &[
            "Tokio, CPU-bound tasks and blocking code (\"blocking code that would otherwise block other tasks from running\"): https://docs.rs/tokio/latest/tokio/index.html#cpu-bound-tasks-and-blocking-code",
            "Alice Ryhl (Tokio maintainer), Async: What is blocking?: https://ryhl.io/blog/async-what-is-blocking/",
            "experiments/tests/concurrency.rs: blocking_sleep_stalls_other_tasks",
        ],
    },
    Rule {
        id: "ignored-io-error",
        category: Category::ErrorHandling,
        severity: Severity::Note,
        summary: "`let _ =` or `.ok();` on a write, flush, sync, rename or copy",
        harm: "The error is the only signal that data didn't reach disk. Verified: on a full device \
               `write_all` fails with ENOSPC and the caller reports success. In five distributed systems, \
               Yuan et al. (OSDI '14) traced 92% of catastrophic failures to incorrect handling of \
               non-fatal errors that the software had signalled; in 25%, the handler simply ignored the error.",
        false_positives: "Best-effort writes where loss is acceptable. Writes to stdout/stderr and removals \
                          aren't flagged.",
        evidence: Evidence::Demonstrated,
        precision: "census, two raters (docs/AUDIT.md): 7 of 16 hid a failed flush, write or sync from a caller expecting an error; the rest were deliberate best-effort writes, writes on an error path, or test doubles",
        exceptions: "",
        sources: &[
            "CWE-252 Unchecked Return Value: https://cwe.mitre.org/data/definitions/252.html",
            "Yuan et al., Simple Testing Can Prevent Most Critical Failures, OSDI 2014: https://www.usenix.org/conference/osdi14/technical-sessions/presentation/yuan",
            "std::io::BufWriter (errors on drop are ignored): https://doc.rust-lang.org/std/io/struct.BufWriter.html",
            "experiments/tests/io.rs: ignored_write_error_loses_data_silently",
        ],
    },
    Rule {
        id: "process-exit-in-library",
        category: Category::ErrorHandling,
        severity: Severity::Note,
        summary: "`process::exit` in a library crate",
        harm: "No destructors run, on this thread or any other: buffered writers aren't flushed and temp \
               files stay. Verified: a `BufWriter`'s record never reached the file and the process exited \
               0. A library also takes the decision away from its caller. Binary crates (a `main.rs` and \
               no `lib.rs`) aren't flagged.",
        false_positives: "A library that is really a CLI's internals, exiting before any buffered state exists.",
        evidence: Evidence::Demonstrated,
        precision: "10 of 10 a library crate exiting for its caller; none lost data (all CLI paths before any buffered state)",
        exceptions: "",
        sources: &[
            "std::process::exit: https://doc.rust-lang.org/std/process/fn.exit.html",
            "experiments/tests/io.rs: process_exit_skips_destructors",
        ],
    },
    Rule {
        id: "float-eq-inexact",
        category: Category::Correctness,
        severity: Severity::Warning,
        summary: "`==` or `!=` against a float literal with no exact binary value (0.1, 0.3, 1e-3)",
        harm: "Those literals are rounded, and so is the arithmetic that produced the other side: \
               `0.1 + 0.2 == 0.3` is false. Literals that are exact (0.0, 0.5, 1.0, 2.5) aren't flagged, \
               since equality with them is well defined.",
        false_positives: "Comparing against a value stored from the same literal (a sentinel).",
        evidence: Evidence::Demonstrated,
        precision: "no instances in the corpus",
        exceptions: "",
        sources: &[
            "Goldberg, What Every Computer Scientist Should Know About Floating-Point Arithmetic (1991): https://docs.oracle.com/cd/E19957-01/806-3568/ncg_goldberg.html",
            "experiments/tests/numeric.rs: inexact_literals_break_equality",
        ],
    },
    Rule {
        id: "len-truncating-cast",
        category: Category::Correctness,
        severity: Severity::Note,
        summary: "`.len() as u32` (or any narrower integer)",
        harm: "`as` between integers truncates silently. Verified on a real 4 GiB + 1 buffer: \
               `len() as u32` is 1. Most lengths in the audit were bounded by their domain (a count of \
               render targets), so this is a note.",
        false_positives: "Lengths bounded by construction. `try_from` documents and checks it.",
        evidence: Evidence::Demonstrated,
        precision: "first audit: 0 of 20 shown reachable; lengths were bounded by their domain",
        exceptions: "",
        sources: &[
            "Rust Reference, numeric cast semantics: https://doc.rust-lang.org/reference/expressions/operator-expr.html#numeric-cast",
            "CWE-197 Numeric Truncation Error: https://cwe.mitre.org/data/definitions/197.html",
            "experiments/tests/numeric.rs: len_as_u32_wraps_for_a_real_buffer",
        ],
    },
    Rule {
        id: "deny-warnings",
        category: Category::Maintainability,
        severity: Severity::Warning,
        summary: "`#![deny(warnings)]` in a library or binary crate root",
        harm: "New compiler releases add lints by design (adding one is not a breaking change), so code that \
               builds today stops building for downstream users after a toolchain update. Verified: the same \
               file builds on Rust 1.76 and fails on 1.99.",
        false_positives: "None; `RUSTFLAGS=-D warnings` in CI gives the strictness without the breakage. \
                          Examples, tests and benches aren't flagged.",
        evidence: Evidence::Demonstrated,
        precision: "no instances in shipping crate roots; all 31 were in examples, tests or benches, which are exempt",
        exceptions: "",
        sources: &[
            "Rust Design Patterns, #![deny(warnings)] anti-pattern: https://rust-unofficial.github.io/patterns/anti_patterns/deny-warnings.html",
            "RFC 1105, API evolution (new lints are minor changes): https://rust-lang.github.io/rfcs/1105-api-evolution.html",
            "experiments/deny-warnings/run.sh",
        ],
    },
];
