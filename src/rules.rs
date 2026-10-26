//! The rule catalogue: what each rule flags, why that is bad, and the evidence. Only claims that
//! survived the checks in docs/EVIDENCE.md are here; the ones that didn't are listed there too.

use crate::Severity;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    /// Wrong results, crashes or undefined behaviour.
    Correctness,
    /// Errors that vanish, so a failure goes unnoticed.
    ErrorHandling,
    /// Deadlocks and stalls.
    Concurrency,
    /// Code that measurably costs more to change safely.
    Maintainability,
}

impl Category {
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Correctness => "correctness",
            Category::ErrorHandling => "error-handling",
            Category::Concurrency => "concurrency",
            Category::Maintainability => "maintainability",
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
    pub sources: &'static [&'static str],
}

pub fn all() -> &'static [Rule] {
    RULES
}

pub fn get(id: &str) -> Option<&'static Rule> {
    RULES.iter().find(|r| r.id == id)
}

const BOOK_PANIC: &str = "Rust Book ch. 9.3, To panic! or Not to panic!: https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html";

static RULES: &[Rule] = &[
    Rule {
        id: "unwrap",
        category: Category::Correctness,
        severity: Severity::Warning,
        summary: "`.unwrap()` in shipping code",
        harm: "A failed unwrap kills the thread (and with it often the process) with a message that names \
               no cause: \"called `Option::unwrap()` on a `None` value\". A panic while holding a lock \
               poisons it for every other thread. The Rust Book reserves unwrap for tests, prototypes and \
               cases the compiler can't see, and says production code should prefer `expect` with the \
               reason. Cloudflare's 18 November 2025 global outage was an unwrap on an error.",
        false_positives: "Can't see an earlier `is_some()` check or other proof that the value is present. \
                          Not flagged: tests, examples, build scripts, `const` code, `lock().unwrap()` \
                          (poison propagation, as the std docs do it) and unwraps on literals such as \
                          `\"127.0.0.1\".parse().unwrap()`, which fail on the first run.",
        sources: &[
            BOOK_PANIC,
            "Cloudflare, 18 November 2025 outage: https://blog.cloudflare.com/18-november-2025-outage/",
            "Clippy unwrap_used: https://rust-lang.github.io/rust-clippy/master/index.html#unwrap_used",
            "experiments/tests/panics.rs: unwrap_panics_without_context, a_panic_poisons_shared_state",
        ],
    },
    Rule {
        id: "todo",
        category: Category::Correctness,
        severity: Severity::Warning,
        summary: "`todo!()` or `unimplemented!()` in shipping code",
        harm: "Both type-check as any type, so unfinished code compiles and ships, then panics with \"not yet \
               implemented\" when the path is reached.",
        false_positives: "None for the panic itself. `unimplemented!` is sometimes a deliberate \"unsupported\" \
                          answer; returning an error says that without crashing.",
        sources: &[
            "std::todo: https://doc.rust-lang.org/std/macro.todo.html",
            "Clippy todo / unimplemented: https://rust-lang.github.io/rust-clippy/master/index.html#todo",
            "experiments/tests/panics.rs: todo_compiles_and_panics_at_runtime",
        ],
    },
    Rule {
        id: "dbg-macro",
        category: Category::Correctness,
        severity: Severity::Warning,
        summary: "`dbg!` left in shipping code",
        harm: "`dbg!` is a debugging aid that behaves the same in release builds: it writes file, line and \
               value to stderr, which leaks internals and pollutes output that tools parse.",
        false_positives: "None; it is never needed in shipped code.",
        sources: &[
            "std::dbg: https://doc.rust-lang.org/std/macro.dbg.html",
            "Clippy dbg_macro: https://rust-lang.github.io/rust-clippy/master/index.html#dbg_macro",
        ],
    },
    Rule {
        id: "panic-in-drop",
        category: Category::Correctness,
        severity: Severity::Error,
        summary: "A panic (`unwrap`, `expect`, `panic!`, `assert!`) inside `Drop::drop`",
        harm: "Destructors run during unwinding. A panic in one while another panic unwinds aborts the whole \
               process: `catch_unwind` never gets control and no other destructor runs. Verified: SIGABRT \
               with \"panic in a destructor during cleanup\".",
        false_positives: "An assertion that truly can't fail. Rudra (SOSP '21) found panic-safety bugs in real \
                          crates of exactly this shape.",
        sources: &[
            "std::ops::Drop, panics: https://doc.rust-lang.org/std/ops/trait.Drop.html",
            "experiments/tests/panics.rs: panic_in_drop_during_unwind_aborts",
        ],
    },
    Rule {
        id: "static-mut",
        category: Category::Correctness,
        severity: Severity::Error,
        summary: "A `static mut` item",
        harm: "Every access is unsafe and nothing stops two threads, or two references, touching it at once: \
               a data race, which is undefined behaviour. Miri reports a data race on a two-thread counter. \
               Rust 2024 makes references to it a hard error (`static_mut_refs`).",
        false_positives: "Single-threaded embedded code that only uses raw pointers to it; an atomic, `Mutex`, \
                          `OnceLock` or a cell type still says that in the type.",
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
        sources: &[
            "std::mem::uninitialized: https://doc.rust-lang.org/std/mem/fn.uninitialized.html",
            "std::mem::MaybeUninit, initialization invariant: https://doc.rust-lang.org/std/mem/union.MaybeUninit.html",
            "Clippy uninit_assumed_init (correctness): https://rust-lang.github.io/rust-clippy/master/index.html#uninit_assumed_init",
            "experiments/miri: bad_assume_init, bad_mem_uninitialized; experiments/tests/memory.rs",
        ],
    },
    Rule {
        id: "unsafe-without-safety-comment",
        category: Category::Correctness,
        severity: Severity::Warning,
        summary: "An `unsafe` block or `unsafe impl` with no `// SAFETY:` comment",
        harm: "`unsafe` moves the proof obligation from the compiler to the author. Without the written \
               argument, a reviewer can't check it and a later edit can silently break it. Memory-safety \
               bugs in Rust CVEs come from unsafe code.",
        false_positives: "A justification written in a different style (e.g. `// Sound because ...`).",
        sources: &[
            "Clippy undocumented_unsafe_blocks: https://rust-lang.github.io/rust-clippy/master/index.html#undocumented_unsafe_blocks",
            "Standard library dev guide, safety comments: https://std-dev-guide.rust-lang.org/policy/safety-comments.html",
        ],
    },
    Rule {
        id: "mem-forget",
        category: Category::Correctness,
        severity: Severity::Note,
        summary: "`mem::forget(x)`",
        harm: "Skips the destructor: a forgotten `MutexGuard` keeps the lock forever, a forgotten `File` leaks \
               its descriptor (both verified). It is safe, so the compiler won't stop it.",
        false_positives: "FFI hand-offs where ownership moves to foreign code; `ManuallyDrop` says the same thing \
                          in the type and is preferred by the std docs.",
        sources: &[
            "std::mem::forget: https://doc.rust-lang.org/std/mem/fn.forget.html",
            "experiments/tests/memory.rs: forgetting_a_guard_keeps_the_lock, forgetting_a_file_leaks_its_descriptor",
        ],
    },
    Rule {
        id: "lock-held-across-await",
        category: Category::Concurrency,
        severity: Severity::Error,
        summary: "A sync lock guard (`lock()`, `read()`, `write()`) alive at an `.await`",
        harm: "While the task is suspended it still holds the lock; any other task on that thread that calls \
               `lock()` blocks the thread, so the holder can never resume to release it. Verified deadlock \
               on a current-thread Tokio runtime; the same code with `tokio::sync::Mutex` finishes.",
        false_positives: "A `lock()` method on a type that isn't a lock. Guards dropped by an inner scope \
                          before the await are handled; guards moved elsewhere aren't tracked.",
        sources: &[
            "Clippy await_holding_lock: https://rust-lang.github.io/rust-clippy/master/index.html#await_holding_lock",
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
               stalling every task scheduled on it. Measured: a 10 ms ticker stalled for the full 100 ms \
               sleep; with `tokio::time::sleep` it kept time.",
        false_positives: "None in async code. Sync closures (e.g. inside `spawn_blocking`) aren't flagged.",
        sources: &[
            "Tokio, spawn_blocking: https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html",
            "Alice Ryhl (Tokio maintainer), Async: What is blocking?: https://ryhl.io/blog/async-what-is-blocking/",
            "experiments/tests/concurrency.rs: blocking_sleep_stalls_other_tasks",
        ],
    },
    Rule {
        id: "ignored-io-error",
        category: Category::ErrorHandling,
        severity: Severity::Warning,
        summary: "`let _ =` or `.ok();` on a write, flush, sync, rename or copy",
        harm: "The error is the only signal that data didn't reach disk. Verified: on a full device \
               `write_all` fails with ENOSPC and the caller reports success. Yuan et al. (OSDI '14) found \
               most catastrophic failures in distributed systems came from mishandled errors.",
        false_positives: "Best-effort writes where loss is acceptable (diagnostics to a closed stderr). Removals \
                          and other cleanup aren't flagged.",
        sources: &[
            "CWE-252 Unchecked Return Value: https://cwe.mitre.org/data/definitions/252.html",
            "Yuan et al., Simple Testing Can Prevent Most Critical Failures, OSDI 2014: https://www.usenix.org/conference/osdi14/technical-sessions/presentation/yuan",
            "std::io::BufWriter (errors on drop are ignored): https://doc.rust-lang.org/std/io/struct.BufWriter.html",
            "experiments/tests/io.rs: ignored_write_error_loses_data_silently",
        ],
    },
    Rule {
        id: "unused-io-amount",
        category: Category::ErrorHandling,
        severity: Severity::Error,
        summary: "`.write(buf)` or `.read(buf)` whose byte count is thrown away",
        harm: "`Write::write` may write fewer bytes than given and `Read::read` may fill less of the buffer; \
               the count is the only way to know. Verified on a real Unix socket: one write of 8 MiB took \
               a fraction. `write_all`/`read_exact` loop for you.",
        false_positives: "A custom method named `write`/`read` with one argument that returns a `Result` but \
                          isn't I/O.",
        sources: &[
            "std::io::Write::write: https://doc.rust-lang.org/std/io/trait.Write.html#tymethod.write",
            "Clippy unused_io_amount (correctness): https://rust-lang.github.io/rust-clippy/master/index.html#unused_io_amount",
            "experiments/tests/io.rs: write_can_be_partial_on_a_real_socket",
        ],
    },
    Rule {
        id: "process-exit-in-library",
        category: Category::ErrorHandling,
        severity: Severity::Warning,
        summary: "`process::exit` outside a binary's entry files",
        harm: "No destructors run, on this thread or any other: buffered writers aren't flushed, temp files \
               stay, locks files stay locked. Verified: a `BufWriter`'s record never reached the file and the \
               process exited 0. A library also takes the decision away from its caller.",
        false_positives: "Library code that is really a CLI's internals and flushes first.",
        sources: &[
            "std::process::exit: https://doc.rust-lang.org/std/process/fn.exit.html",
            "Clippy exit: https://rust-lang.github.io/rust-clippy/master/index.html#exit",
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
        sources: &[
            "Goldberg, What Every Computer Scientist Should Know About Floating-Point Arithmetic (1991): https://docs.oracle.com/cd/E19957-01/806-3568/ncg_goldberg.html",
            "Clippy float_cmp: https://rust-lang.github.io/rust-clippy/master/index.html#float_cmp",
            "experiments/tests/numeric.rs: inexact_literals_break_equality",
        ],
    },
    Rule {
        id: "len-truncating-cast",
        category: Category::Correctness,
        severity: Severity::Note,
        summary: "`.len() as u32` (or any narrower integer)",
        harm: "`as` between integers truncates silently. Verified on a real 4 GiB + 1 buffer: \
               `len() as u32` is 1, so a length prefix or index would be wrong with no error.",
        false_positives: "Lengths bounded by construction (a fixed-size array). `try_from` documents and checks it.",
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
        summary: "`#![deny(warnings)]` in source",
        harm: "New compiler releases add lints by design (adding one is not a breaking change), so code that \
               builds today stops building for downstream users after a toolchain update. Verified: the same \
               file builds on Rust 1.76 and fails on 1.99.",
        false_positives: "None; `RUSTFLAGS=-D warnings` in CI gives the strictness without the breakage.",
        sources: &[
            "Rust Design Patterns, #![deny(warnings)] anti-pattern: https://rust-unofficial.github.io/patterns/anti_patterns/deny-warnings.html",
            "RFC 1105, API evolution (new lints are minor changes): https://rust-lang.github.io/rfcs/1105-api-evolution.html",
            "experiments/deny-warnings/run.sh",
        ],
    },
    Rule {
        id: "complex-function",
        category: Category::Maintainability,
        severity: Severity::Note,
        summary: "A function above the cognitive-complexity limit",
        harm: "PENDING STUDY",
        false_positives: "Large `match` tables score low here by design; long linear functions too.",
        sources: &["G. Ann Campbell, Cognitive Complexity (SonarSource, 2018): https://www.sonarsource.com/docs/CognitiveComplexity.pdf"],
    },
    Rule {
        id: "long-function",
        category: Category::Maintainability,
        severity: Severity::Note,
        summary: "A function above the code-line limit",
        harm: "PENDING STUDY",
        false_positives: "Generated or table-like code.",
        sources: &["Clippy too_many_lines: https://rust-lang.github.io/rust-clippy/master/index.html#too_many_lines"],
    },
    Rule {
        id: "deep-nesting",
        category: Category::Maintainability,
        severity: Severity::Note,
        summary: "A function nested deeper than the limit",
        harm: "PENDING STUDY",
        false_positives: "Closures count as a level.",
        sources: &["CWE-1124 Excessively Deep Nesting: https://cwe.mitre.org/data/definitions/1124.html"],
    },
    Rule {
        id: "too-many-params",
        category: Category::Maintainability,
        severity: Severity::Note,
        summary: "A function with more parameters than the limit",
        harm: "PENDING STUDY",
        false_positives: "Constructors for plain data.",
        sources: &["Clippy too_many_arguments: https://rust-lang.github.io/rust-clippy/master/index.html#too_many_arguments"],
    },
];
