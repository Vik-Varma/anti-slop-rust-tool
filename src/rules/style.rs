//! Rules from Google's Rust style guide (https://google.github.io/styleguide/rustguide.html).
//! None rests on a reproduced failure: each is a preference the guide states, with the reason
//! the guide gives, and `exceptions` says which of the guide's own exceptions the check honours
//! by itself and which need a suppression comment naming them. See docs/STYLE.md.

use super::{Category, Evidence, Rule};
use crate::Severity;

macro_rules! style_rule {
    ($id:literal, $section:literal, $summary:literal, $harm:literal, $exceptions:literal, $fp:literal) => {
        Rule {
            id: $id,
            category: Category::Style,
            severity: Severity::Style,
            summary: $summary,
            harm: $harm,
            false_positives: $fp,
            evidence: Evidence::Style,
            precision: "not measured: a preference has no false positives in the audit's sense; docs/STYLE.md gives how often each fires on the 37-codebase corpus",
            exceptions: $exceptions,
            sources: &[concat!("Google Rust style guide, ", $section, ": ", "https://google.github.io/styleguide/rustguide.html")],
        }
    };
}

pub(super) static STYLE_RULES: &[Rule] = &[
    style_rule!(
        "glob-import",
        "Don't use glob imports",
        "`use path::*`",
        "Globs \"increase toil for code janitors running LSCs, and make it harder to know where a type comes from in contexts without xrefs\"; \"defining preludes should be avoided\".",
        "Honoured by the check: `use super::*` in an inline module; `use Enum::*` inside a function (a glob whose last segment is CamelCase); `pub use leaf::*` from a module that declares submodules; a prelude (`..::prelude::*`), which the guide leaves to the reviewer. Not visible to syntax: merging C++ namespaces into a private module; cite it in a suppression.",
        "Whether a glob names an enum or a module is read from the case of its last segment."
    ),
    style_rule!(
        "use-mixes-modules",
        "A `use` statement should import items from one module",
        "`use a::{B, c::D}`: one `use` reaching into more than one module",
        "\"there should not be any `::`'s inside the `{..}` block of a `use` statement\", which is rustfmt's `imports_granularity = Module`.",
        "None listed.",
        "None in shape."
    ),
    style_rule!(
        "unqualified-function-call",
        "Qualify calls to top-level functions with a module name",
        "a free function imported by name and called bare (`transmute(x)` for `mem::transmute(x)`)",
        "\"import their module and call them with `modulename::functionname()`\"; the benefit is \"mostly consistency and a minor edge on reading code without xrefs\".",
        "The guide allows functions whose names are sufficiently disambiguated (its example is functions from C++), functions from small single-purpose first-party crates (Chromium), and modules whose names are prone to collisions. None is visible to syntax: cite the one that applies in a suppression.",
        "A snake-case `use` leaf is taken for a function; it could be a module or a lower-case constant. Only calls in the same file as the import are seen."
    ),
    style_rule!(
        "qualified-type-name",
        "Type names should be imported and used unqualified",
        "a type or trait written with its module path (`regex::Regex`, `fmt::Display`)",
        "\"types and traits should be referenced in code without using their module path\"; \"This rule largely exists to avoid reviewer disagreements\".",
        "Honoured: names the guide calls ambiguous (`io::Error`; here `Error` and `Result`), and `Self`. The guide also allows a large group of types from one module (its example is rustc's `hir`), macros and generated code, and files with so many dependencies that an import block would be very large: cite one in a suppression.",
        "Only `Error` and `Result` count as common names; the guide leaves the list open."
    ),
    style_rule!(
        "wrapper-reference-param",
        "Accept flexible references for immutable data",
        "a parameter typed `&Vec<T>`, `&String`, `&Box<T>` or `&Option<T>`",
        "\"Pass references to the data itself instead of references to smart pointers, containers, or wrappers\": `&[T]`, `&str`, `&T` and `Option<&T>` take every caller the wrapper does, and more (Clippy `ptr_arg`, `borrowed_box`).",
        "The guide excepts non-linear character data such as a Rope or Cord, which isn't one of these four types, so nothing is exempted.",
        "A function that needs the container's capacity or identity can say so in a suppression."
    ),
    style_rule!(
        "bool-parameter",
        "Prefer `enum` over `bool` in APIs",
        "a `bool` parameter on a public function",
        "\"Use `enum` function parameters instead of `bool` in cases where the purpose of the parameter is not obvious.\" The defect study found no replicated link between bool parameters and later bug fixes (docs/EVIDENCE.md), so this is readability.",
        "Honoured, from the guide's fallback: a parameter with an inline comment on its line (`dry_run: bool, // skip writes`) isn't reported. Whether a purpose is \"obvious\" is a judgment for a suppression.",
        "Only public functions are checked, since the rule is about APIs."
    ),
    style_rule!(
        "into-parameter-in-application",
        "Accepting `Into`",
        "`impl Into<T>` (or `T: Into<U>`) as a parameter in application code",
        "\"Application code should almost never use `impl Into<T>` parameters\"; the convenience is weighed \"against the cost of generics\".",
        "Honoured: library code isn't checked, as the guide allows `impl Into<T>` there \"if call-site ergonomics are paramount\". A file is application code when it belongs to a binary (`main.rs`, `src/bin/`, or a package with `main.rs` and no `lib.rs`).",
        "A binary that is a thin shell over its own library modules is still application code to the guide."
    ),
    style_rule!(
        "rc-arc",
        "Avoid `Rc` and `Arc`",
        "`Rc<T>` or `Arc<T>` in a field or signature",
        "\"Prefer unique ownership and borrowing over `Rc` and `Arc` for managing memory\"; \"These pointers add runtime overhead and their own unique complexity.\"",
        "The guide allows them \"when the alternative ownership and borrowing structure would be unreasonably complex to write, understand, or maintain\", as \"a judgment call to be discussed during code review\". That can't be read from a type, so every use is reported and the judgment goes in a suppression.",
        "Shared-state code (async runtimes, threads) uses `Arc` by design; this rule reports all of it."
    ),
    style_rule!(
        "tuple-fields",
        "Prefer named fields over tuples and non-newtype tuple structs",
        "a tuple struct with two or more fields, a tuple-typed field, or a public function returning a tuple",
        "\"Prefer types with named fields over tuples, as tuple indexes (`.0`, `.1`, ...) quickly become confusing.\"",
        "Honoured: newtypes (one field) aren't reported; nor are \"very local contexts\", taken as `let` bindings and private functions.",
        "A tuple returned by a public function and used once nearby is still reported."
    ),
    style_rule!(
        "lifetime-name",
        "Naming lifetimes",
        "a single-letter lifetime on a type, or more than two on a function",
        "\"Lifetimes should have descriptive names, especially type lifetimes.\"",
        "Honoured: `'_`; up to two single-character lifetimes on a function, \"simply expressing the relationship between a method's input and output\"; `for<'a>` bounds aren't counted. \"Extremely generic abstractions like `Cow`\" are a judgment for a suppression.",
        "None in shape."
    ),
    style_rule!(
        "must-use-missing",
        "`#[must_use]` on functions and on types",
        "a public query-like function, or an error type, without `#[must_use]`",
        "Apply `#[must_use]` when \"The caller cannot use the function correctly if they ignore the return value. This includes pure functions\", and to a type that \"is an error type\".",
        "Honoured: functions returning `Result` are exempt, as the guide says the type already carries it. Only functions that look pure are reported: `&self` or no receiver, no `&mut` parameter, a return type of `bool`, `Option`, a reference or a primitive, and a name that isn't a verb whose result is commonly ignored (`insert`, `send`, ...).",
        "Purity is guessed from the signature; a `&self` method with interior mutability can have a result that is fine to ignore. A type is an error type by its `Error` suffix."
    ),
    style_rule!(
        "non-exhaustive-missing",
        "Apply `#[non_exhaustive]` to publicly-exposed enums and to public structs with public fields",
        "a public enum of three or more variants, or a public struct with all fields public, without `#[non_exhaustive]`",
        "\"If an enum is part of a public API used by other teams, mark it `#[non_exhaustive]`\"; adding a field to an all-public struct \"is a breaking change, because callers can no longer name all fields in a struct literal\".",
        "Honoured: two-state enums (`Visible`/`Hidden`), the guide's example of an enum that never changes, aren't reported; only library crates and `pub` items are checked. The guide's other exceptions are judgments: an enum that is an existing ontology (`Red`, `Green`, `Blue`), a struct whose fields won't change (a `Rectangle`), a commitment to fix breakages with an LSC, and wanting functional record update syntax. Name one in a suppression.",
        "Whether another team uses the item isn't visible."
    ),
    style_rule!(
        "len-without-is-empty",
        "Implement expected API sets together",
        "a public `len()` with no `is_empty()` on the same type",
        "\"If you implement `len()`, also implement `is_empty()`\" (Clippy `len_without_is_empty`).",
        "None listed.",
        "The two can be in different files; only this one is read."
    ),
    style_rule!(
        "getter-prefix",
        "Getters and setters",
        "a getter named `get_foo()`",
        "\"`foo()` for a getter that returns `&T`, or `T` by value if `T: Copy`\"; the Rust API Guidelines (C-GETTER) agree.",
        "None listed. `get()`, `get_mut()` and `get_or_*()` aren't field getters and aren't reported.",
        "A `get_foo(&self)` that computes rather than reads a field is still reported."
    ),
    style_rule!(
        "constructor-name",
        "Naming initialization functions",
        "a constructor named outside the guide's scheme, or `new()` returning `Result`/`Option`",
        "\"Constructors: Use `new()`, or `new_with_foo()` if there is more than one. If the constructor returns a `Result`, use `try_new()`.\" \"Value-to-value conversions: Use `from_foo()`\".",
        "Honoured, as names the guide or the standard library use: `new`, `new_*`, `try_new`, `try_new_*`, `from_*`, `try_from_*`, `with_*`, `default`, and the std verbs `open`, `create`, `connect`, `bind`, `spawn`, `parse`, `load`, `build`, `empty`, `now`, `zeroed`, `random`.",
        "A constructor is any inherent associated function without `self` returning `Self`, `Result<Self, _>` or `Option<Self>`."
    ),
    style_rule!(
        "from-impl-without-constructor",
        "Pair important `From` impls with explicit constructor methods",
        "`impl From<T> for X` with no explicit constructor on `X`",
        "\"add an inherent `::new()`, `::new_from_foo()`, or `::from_foo()` method\" beside a `From` impl that is the primary way to build the type.",
        "The guide says \"important\" `From` impls; which are important is a judgment for a suppression. Only types defined in the same file are checked.",
        "A constructor in another file isn't seen."
    ),
    style_rule!(
        "std-trait-as-method",
        "Implement known standard library traits instead of custom methods",
        "a public method with a standard trait's name (`from_str`, `eq`, `next`, ...) on a type without that trait",
        "\"If a method matches the purpose and behavior of a standard library trait, implement that trait\" (Clippy `should_implement_trait`).",
        "Honoured: \"both the method and the trait impl for discoverability\" is fine, so a type with the trait implemented or derived in the same file isn't reported.",
        "The trait impl can be in another file."
    ),
    style_rule!(
        "new-without-default",
        "Specific guidance for important traits: `Default`",
        "`pub fn new()` with no arguments on a type without `Default`",
        "\"Implement if the type implements `new()` or otherwise has a default state\" (Clippy `new_without_default`).",
        "None listed.",
        "The `Default` impl can be in another file."
    ),
    style_rule!(
        "options-struct-default",
        "Options structs should implement Default",
        "an all-public-field options struct (`FooOptions`, `FooConfig`, ...) without `Default`",
        "\"Structs with all-public fields that are primarily used as a 'bag of options' for a function should implement `Default`\", so callers can write `Struct { a: 0, ..Default::default() }`.",
        "None listed. A struct is a bag of options by its name: `Options`, `Opts`, `Config`, `Settings` or `Params`.",
        "An options struct with another name isn't seen; one with a field that has no default can't derive it."
    ),
    style_rule!(
        "trait-as-organization",
        "When to define a trait versus an inherent implementation",
        "a private trait with one implementation and no use as a bound",
        "\"Define a trait only\" for generic programming, for `impl Trait`/`dyn Trait`, or as an extension trait; \"Don't use traits merely to organize code.\"",
        "Honoured: public traits and extension traits (`..Ext`) aren't reported, since other modules may use them as bounds.",
        "A bound in another file of the same crate isn't seen."
    ),
    style_rule!(
        "ext-trait-in-bounds",
        "Extension traits",
        "an extension trait (`FooExt`) used as a bound, `impl Trait` or `dyn Trait`",
        "An extension trait's \"sole purpose is to add methods to the type; it is not intended for use in generics (`T: Trait`, `impl Trait`) or dynamic dispatch (`dyn Trait`)\".",
        "None listed. A trait is an extension trait by the `Ext` suffix the guide requires.",
        "A trait that ends in `Ext` without being an extension trait is reported."
    ),
    style_rule!(
        "macro-definition",
        "Macros",
        "a `macro_rules!` or procedural macro definition",
        "The guide ranks \"Regular Rust functions, types, traits, objects, and modules\" first: \"Avoid macros, code generation, and metaprogramming where possible\"; \"IDEs, Code Search and other developer tools will work less well\"; \"Large scale changes are more difficult and expensive to apply.\"",
        "None listed; the guide orders the alternatives (`macro_rules!` before other code generation, procedural macros last) rather than excepting any.",
        "None in shape."
    ),
    style_rule!(
        "code-before-main",
        "Do not run code before `main()`",
        "a function or static arranged to run before `main()` (`#[ctor]`, an `.init_array` section)",
        "\"Do not cause code to run before `main()`, for example by using techniques such as the ctor crate.\"",
        "None listed.",
        "Only the `ctor`/`dtor` attributes and the known initialiser link sections are recognised."
    ),
    style_rule!(
        "unstable-feature",
        "No unstable features",
        "`#![feature(..)]`",
        "\"Don't use unstable features.\"",
        "The guide's exceptions are organisational: features Android Embedded needs to build an artifact, a short list blanket-approved for Chrome, and Fuchsia's own policy. Cite the approval in a suppression.",
        "None in shape."
    ),
    style_rule!(
        "rustfmt-skip",
        "Opting out of automatic formatting",
        "`#[rustfmt::skip]` without a stated reason",
        "Opt out of formatting only for \"Manual alignment\" or \"Generated code\"; \"Formatting should not be skipped for personal style choices.\"",
        "Honoured: both listed exceptions, when a comment on the attribute's line or the line above mentions alignment (`align`) or generated code (`generat`).",
        "A reason worded differently isn't recognised."
    ),
    style_rule!(
        "api-in-non-leaf-module",
        "Don't implement APIs in non-leaf modules",
        "a module that declares submodules and also holds more than 50 lines of definitions",
        "\"The only items in non-leaf modules should be module declarations (not definitions) and re-exports.\"",
        "Honoured: `#[cfg(test)] mod` and `#[cxx::bridge]` modules don't make a module non-leaf, and \"a small amount of core 'ties things together' implementation code\" is allowed, taken here as up to 50 lines of code. The number is this tool's, not the guide's.",
        "The 50-line allowance is a guess at \"a small amount\"."
    ),
    style_rule!(
        "file-named-after-item",
        "Name source files and leaf modules after the primary user-defined item",
        "a file whose one public type isn't what it is named after",
        "\"When a leaf module has a primary user-defined item (e.g., `Foo`), the module should be named after that item (e.g., `mod foo` and `foo.rs`).\"",
        "Honoured: a module \"not primarily about any particular abstraction\" may be named as desired, so files with no public type, more than one, or two or more public functions beside it aren't reported; nor are `mod.rs`, `lib.rs`, `main.rs` and `build.rs`.",
        "The primary item is guessed as the file's only public type."
    ),
    style_rule!(
        "check-then-unwrap",
        "Prefer pattern matching over check-then-unwrap",
        "`if x.is_some() { .. x.unwrap() .. }` (or `is_ok`, `is_none`, `is_err`)",
        "\"instead of using `Option::unwrap` on a value known to be valid, use pattern matching to prove correctness to the compiler\".",
        "None listed.",
        "The checked value and the unwrapped one are matched by their text."
    ),
    style_rule!(
        "assert-without-message",
        "Use context strings to clarify assertions",
        "`assert!`, `assert_eq!` or `assert_ne!` with no message",
        "\"Use the optional format string argument for `assert!` to convey this context\": why it should hold, a value worth logging, or the API misuse it catches.",
        "The guide adds \"Don't use the context string merely to repeat the condition\", which can't be checked. Tests are exempt here, which is this tool's choice, not the guide's: a test's assertion is its own context.",
        "None in shape."
    ),
    style_rule!(
        "combinator-chain",
        "Don't use iterator combinator chains when direct iteration is clearer",
        "`for_each`, a combinator chain in a `for` header, or a chain of five or more adapters",
        "\"Avoid long and complex combinator chains\"; \"Don't construct combinator chains in the header of a `for` loop\"; \"Avoid `Iterator::for_each` unless you're convinced you have a good reason. A `for` loop is usually easier to read.\"",
        "Honoured: the guide prefers combinators \"for shorter chains and for cases where the combinator itself provides a lot of logic\", so chains under five adapters and `for` headers with a single adapter (`.iter().enumerate()`) aren't reported. The thresholds are this tool's.",
        "Length in lines, not adapter count, may be what makes a chain hard to read."
    ),
    style_rule!(
        "string-concat-plus",
        "Prefer `format!` / `write!` for concatenation and substitution",
        "three or more strings joined with `+`, or `replace` on a string literal",
        "\"To concatenate more than two strings... use `format!(\\\"{a}{b}{c}\\\")` instead of `a + b + c`\"; \"To perform substitutions on a literal string (instead of using `str::replace`)\"; \"`format!` / `write!` require fewer allocations\".",
        "None listed.",
        "Operands count as strings when one is a literal, `format!`, `to_string()`, `to_owned()` or `String::from`; numeric sums aren't reported."
    ),
    style_rule!(
        "inferred-conversion-type",
        "Don't overuse type inference",
        "`let x = y.into();` or `.parse()` with no type, or a turbofish on `collect` instead of a type on the `let`",
        "\"Add enough explicit types so that someone familiar with the codebase can figure out what the inferred types are\", especially \"When calling methods with a generic return type, such as `.into()` or `.parse()`\"; \"Prefer to add the types to variable declarations, rather than as turbofishes.\"",
        "Honoured: a turbofish with a `_` inference marker (`collect::<Vec<_>>()`) is fine when the compiler forces one, as the guide says.",
        "None in shape."
    ),
    style_rule!(
        "poisoned-mutex-recovery",
        "Handling poison",
        "taking the data out of a poisoned lock (`PoisonError::into_inner`)",
        "\"Never reach inside a poisoned mutex to get at its insides, no matter how tempting the use case may seem.\" The standard library calls poisoning \"only advisory\" and documents `into_inner` for exactly this, and the corpus uses it deliberately, which is why this is a style rule and not a defect (docs/EVIDENCE.md).",
        "None listed.",
        "Only `unwrap_or_else(|e| e.into_inner())`, `unwrap_or_else(PoisonError::into_inner)` and `Err(e) => e.into_inner()` on `lock()`, `read()`, `write()` and `try_lock()` are recognised."
    ),
    style_rule!(
        "catch-unwind",
        "Don't call `panic::catch_unwind`",
        "`catch_unwind` in first-party code",
        "\"Use of `panic::catch_unwind` is prohibited in first-party Rust code.\" Where the build profile aborts on panic it can't work at all; that is the warning `catch-unwind-under-panic-abort`, and this rule covers the rest.",
        "The guide accepts it \"as a defense in depth mechanism as long as it is never relied upon for any properties\", with the code doing \"most of what it can to avoid panics\", and only where `panic=unwind` is possible. Reliance can't be read from the call: say so in a suppression.",
        "A local function that happens to be named `catch_unwind` is taken for std's."
    ),
    style_rule!(
        "missing-docs",
        "When and how to write documentation",
        "a public item with no doc comment",
        "\"Add documentation comments to public APIs, potentially using the `missing_docs` lint to help with this.\"",
        "Honoured: enum variants needn't be documented, as the guide says. `#[doc(hidden)]` items, `pub(crate)` items, test modules and non-library crates aren't checked.",
        "A re-export or a macro-generated item isn't seen."
    ),
    style_rule!(
        "doc-comment-style",
        "When and how to write documentation",
        "a doc comment that restates the item's name, or lacks capitalisation and punctuation",
        "\"Comments should say something that the code does not already say\" (the guide's example: `/// Removes an element.` on `remove_element`); \"Use capitalization and punctuation.\"",
        "None listed. A first line that starts with code, a heading, a list or a link isn't checked for its capital or its full stop.",
        "Restating is judged by word overlap, so a short comment on a well-named item can be reported even when it adds a nuance."
    ),
    style_rule!(
        "panic-undocumented",
        "When and how to write documentation",
        "a public function that can panic on its arguments, with no `# Panics` section",
        "\"Always document panic cases for functions which may panic due to malformed input.\"",
        "None listed. Only panics that involve a parameter (`assert!(n > 0)`, `input.parse().unwrap()`) and explicit `panic!`-family macros count; a panic on internal state isn't \"malformed input\".",
        "A panic inside a callee isn't seen; indexing isn't counted."
    ),
    style_rule!(
        "safety-undocumented",
        "When and how to write documentation",
        "a public `unsafe fn` or `unsafe trait` with no `# Safety` section",
        "\"Clearly document invariants for unsafe functions and traits.\" (Clippy `missing_safety_doc`)",
        "None listed.",
        "A contract documented under another heading is reported."
    ),
    style_rule!(
        "safety-comment-prefix",
        "Unsafe code",
        "an `unsafe` block or impl whose justification doesn't start with `SAFETY:`",
        "\"Add a safety comment starting with `SAFETY:` that explains why the code is correct.\" The standard library's own policy uses the same marker. The note `unsafe-without-comment` covers having no justification at all; this is the wording.",
        "None listed.",
        "None in shape."
    ),
];
