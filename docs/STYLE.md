# Style rules

These rules are a different kind of thing from the rest of the tool. Each is a preference stated
by [Google's Rust style guide](https://google.github.io/styleguide/rustguide.html), not a failure
that was reproduced, so by [DEFINITION.md](DEFINITION.md) none of them identifies bad code. They
exist because teams that follow the guide asked for them, and they are off unless asked for:

```sh
anti-slop --min-severity style path       # everything, including these
anti-slop --only glob-import,rc-arc path  # just some
anti-slop explain glob-import             # the guide's reason, and the exceptions it lists
```

The guide was read claim by claim ([research/rust-official.md](research/rust-official.md), §6).
One claim names a failure that reproduces, and became the warning `catch-unwind-under-panic-abort`
([EVIDENCE.md](EVIDENCE.md)). Three agree with rules already here. Rules the guide tags as one
codebase's policy ([chrome]: small crates, flat file layout, `chromium::import!`, no `async`, no
std threads; [Fuchsia]: `fuchsia_sync::Mutex`, test frameworks) aren't included: the guide itself
scopes them to one codebase. Everything else that syntax can decide is below.

## How the guide's exceptions are handled

Every rule's entry (`anti-slop explain <rule>`) ends with the exceptions the guide lists for it,
in three kinds:

- **Honoured by the check.** Where syntax can tell, the exception is built in: `use super::*` in
  an inline module isn't a `glob-import`; a `bool` parameter with a comment on its line isn't a
  `bool-parameter`; a two-state enum isn't `non-exhaustive-missing`.
- **Needs a suppression.** Where the exception is a judgment ("unreasonably complex", "important
  `From` impls", "an existing ontology"), the finding is reported and the exception is named, so
  the judgment can be recorded where the code is:
  `// anti-slop: allow(rc-arc) the alternative is a four-way borrow; agreed in review`.
- **Not checkable.** Some of the guide's rules need types or intent (iterators versus owned
  collections, `#[derive]` over a manual impl, `const fn` stability); they have no rule, and are
  listed at the end.

A few thresholds are this tool's, not the guide's, and each rule says so: five adapters for a
"long" combinator chain, 50 lines of definitions in a non-leaf module, `Error` and `Result` as the
"common names" that may keep their module path.

## The rules


Counts are findings over the same 37 codebases as the audit (35 study projects, Ferrock and Dataminer), shipping code only. They say how often each preference is departed from in real Rust, not whether a finding is "right": a preference has nothing to be right about.

| Rule | Guide section | Flags | Findings | In how many codebases |
|---|---|---|---:|---:|
| `glob-import` | Don't use glob imports | `use path::*` | 1,503 | 33 |
| `use-mixes-modules` | A `use` statement should import items from one module | `use a::{B, c::D}`: one `use` reaching into more than one module | 4,857 | 35 |
| `unqualified-function-call` | Qualify calls to top-level functions with a module name | a free function imported by name and called bare (`transmute(x)` for `mem::transmute(x)`) | 11,677 | 37 |
| `qualified-type-name` | Type names should be imported and used unqualified | a type or trait written with its module path (`regex::Regex`, `fmt::Display`) | 75,799 | 37 |
| `wrapper-reference-param` | Accept flexible references for immutable data | a parameter typed `&Vec<T>`, `&String`, `&Box<T>` or `&Option<T>` | 406 | 28 |
| `bool-parameter` | Prefer `enum` over `bool` in APIs | a `bool` parameter on a public function | 2,367 | 35 |
| `into-parameter-in-application` | Accepting `Into` | `impl Into<T>` (or `T: Into<U>`) as a parameter in application code | 30 | 6 |
| `rc-arc` | Avoid `Rc` and `Arc` | `Rc<T>` or `Arc<T>` in a field or signature | 3,280 | 33 |
| `tuple-fields` | Prefer named fields over tuples and non-newtype tuple structs | a tuple struct with two or more fields, a tuple-typed field, or a public function returning a tuple | 675 | 35 |
| `lifetime-name` | Naming lifetimes | a single-letter lifetime on a type, or more than two on a function | 2,590 | 36 |
| `must-use-missing` | `#[must_use]` on functions and on types | a public query-like function, or an error type, without `#[must_use]` | 9,768 | 34 |
| `non-exhaustive-missing` | Apply `#[non_exhaustive]` to publicly-exposed enums and to public structs with public fields | a public enum of three or more variants, or a public struct with all fields public, without `#[non_exhaustive]` | 6,811 | 33 |
| `len-without-is-empty` | Implement expected API sets together | a public `len()` with no `is_empty()` on the same type | 36 | 12 |
| `getter-prefix` | Getters and setters | a getter named `get_foo()` | 440 | 24 |
| `constructor-name` | Naming initialization functions | a constructor named outside the guide's scheme, or `new()` returning `Result`/`Option` | 2,697 | 37 |
| `from-impl-without-constructor` | Pair important `From` impls with explicit constructor methods | `impl From<T> for X` with no explicit constructor on `X` | 1,156 | 35 |
| `std-trait-as-method` | Implement known standard library traits instead of custom methods | a public method with a standard trait's name (`from_str`, `eq`, `next`, ...) on a type without that trait | 81 | 22 |
| `new-without-default` | Specific guidance for important traits: `Default` | `pub fn new()` with no arguments on a type without `Default` | 136 | 22 |
| `options-struct-default` | Options structs should implement Default | an all-public-field options struct (`FooOptions`, `FooConfig`, ...) without `Default` | 229 | 19 |
| `trait-as-organization` | When to define a trait versus an inherent implementation | a private trait with one implementation and no use as a bound | 44 | 16 |
| `ext-trait-in-bounds` | Extension traits | an extension trait (`FooExt`) used as a bound, `impl Trait` or `dyn Trait` | 46 | 6 |
| `macro-definition` | Macros | a `macro_rules!` or procedural macro definition | 1,172 | 34 |
| `code-before-main` | Do not run code before `main()` | a function or static arranged to run before `main()` (`#[ctor]`, an `.init_array` section) | 0 | 0 |
| `unstable-feature` | No unstable features | `#![feature(..)]` | 114 | 23 |
| `rustfmt-skip` | Opting out of automatic formatting | `#[rustfmt::skip]` without a stated reason | 68 | 12 |
| `api-in-non-leaf-module` | Don't implement APIs in non-leaf modules | a module that declares submodules and also holds more than 50 lines of definitions | 671 | 37 |
| `file-named-after-item` | Name source files and leaf modules after the primary user-defined item | a file whose one public type isn't what it is named after | 1,584 | 37 |
| `check-then-unwrap` | Prefer pattern matching over check-then-unwrap | `if x.is_some() { .. x.unwrap() .. }` (or `is_ok`, `is_none`, `is_err`) | 31 | 8 |
| `assert-without-message` | Use context strings to clarify assertions | `assert!`, `assert_eq!` or `assert_ne!` with no message | 1,547 | 33 |
| `combinator-chain` | Don't use iterator combinator chains when direct iteration is clearer | `for_each`, a combinator chain in a `for` header, or a chain of five or more adapters | 484 | 32 |
| `string-concat-plus` | Prefer `format!` / `write!` for concatenation and substitution | three or more strings joined with `+`, or `replace` on a string literal | 15 | 10 |
| `inferred-conversion-type` | Don't overuse type inference | `let x = y.into();` or `.parse()` with no type, or a turbofish on `collect` instead of a type on the `let` | 738 | 36 |
| `poisoned-mutex-recovery` | Handling poison | taking the data out of a poisoned lock (`PoisonError::into_inner`) | 66 | 10 |
| `catch-unwind` | Don't call `panic::catch_unwind` | `catch_unwind` in first-party code | 59 | 10 |
| `missing-docs` | When and how to write documentation | a public item with no doc comment | 28,147 | 35 |
| `doc-comment-style` | When and how to write documentation | a doc comment that restates the item's name, or lacks capitalisation and punctuation | 6,155 | 35 |
| `panic-undocumented` | When and how to write documentation | a public function that can panic on its arguments, with no `# Panics` section | 935 | 32 |
| `safety-undocumented` | When and how to write documentation | a public `unsafe fn` or `unsafe trait` with no `# Safety` section | 511 | 11 |
| `safety-comment-prefix` | Unsafe code | an `unsafe` block or impl whose justification doesn't start with `SAFETY:` | 808 | 28 |

## Each rule, with the guide's reason and exceptions

### `glob-import`

**Guide:** Don't use glob imports. **Flags:** `use path::*`.

**The guide's reason:** Globs "increase toil for code janitors running LSCs, and make it harder to know where a type comes from in contexts without xrefs"; "defining preludes should be avoided".

**Exceptions:** Honoured by the check: `use super::*` in an inline module; `use Enum::*` inside a function (a glob whose last segment is CamelCase); `pub use leaf::*` from a module that declares submodules; a prelude (`..::prelude::*`), which the guide leaves to the reviewer. Not visible to syntax: merging C++ namespaces into a private module; cite it in a suppression.

**Limits:** Whether a glob names an enum or a module is read from the case of its last segment.

**On the corpus:** 1,503 finding(s) in 33 codebase(s); most in nushell 600, diesel 192, wgpu 144.

### `use-mixes-modules`

**Guide:** A `use` statement should import items from one module. **Flags:** `use a::{B, c::D}`: one `use` reaching into more than one module.

**The guide's reason:** "there should not be any `::`'s inside the `{..}` block of a `use` statement", which is rustfmt's `imports_granularity = Module`.

**Exceptions:** None listed.

**Limits:** None in shape.

**On the corpus:** 4,857 finding(s) in 35 codebase(s); most in nushell 1,080, gitoxide 915, wgpu 457.

### `unqualified-function-call`

**Guide:** Qualify calls to top-level functions with a module name. **Flags:** a free function imported by name and called bare (`transmute(x)` for `mem::transmute(x)`).

**The guide's reason:** "import their module and call them with `modulename::functionname()`"; the benefit is "mostly consistency and a minor edge on reading code without xrefs".

**Exceptions:** The guide allows functions whose names are sufficiently disambiguated (its example is functions from C++), functions from small single-purpose first-party crates (Chromium), and modules whose names are prone to collisions. None is visible to syntax: cite the one that applies in a suppression.

**Limits:** A snake-case `use` leaf is taken for a function; it could be a module or a lower-case constant. Only calls in the same file as the import are seen.

**On the corpus:** 11,677 finding(s) in 37 codebase(s); most in nushell 2,480, zellij 2,011, gitoxide 1,746.

### `qualified-type-name`

**Guide:** Type names should be imported and used unqualified. **Flags:** a type or trait written with its module path (`regex::Regex`, `fmt::Display`).

**The guide's reason:** "types and traits should be referenced in code without using their module path"; "This rule largely exists to avoid reviewer disagreements".

**Exceptions:** Honoured: names the guide calls ambiguous (`io::Error`; here `Error` and `Result`), and `Self`. The guide also allows a large group of types from one module (its example is rustc's `hir`), macros and generated code, and files with so many dependencies that an import block would be very large: cite one in a suppression.

**Limits:** Only `Error` and `Result` count as common names; the guide leaves the list open.

**On the corpus:** 75,799 finding(s) in 37 codebase(s); most in wgpu 23,900, gitoxide 12,556, zellij 5,779.

### `wrapper-reference-param`

**Guide:** Accept flexible references for immutable data. **Flags:** a parameter typed `&Vec<T>`, `&String`, `&Box<T>` or `&Option<T>`.

**The guide's reason:** "Pass references to the data itself instead of references to smart pointers, containers, or wrappers": `&[T]`, `&str`, `&T` and `Option<&T>` take every caller the wrapper does, and more (Clippy `ptr_arg`, `borrowed_box`).

**Exceptions:** The guide excepts non-linear character data such as a Rope or Cord, which isn't one of these four types, so nothing is exempted.

**Limits:** A function that needs the container's capacity or identity can say so in a suppression.

**On the corpus:** 406 finding(s) in 28 codebase(s); most in zellij 174, cargo 33, meilisearch 33.

### `bool-parameter`

**Guide:** Prefer `enum` over `bool` in APIs. **Flags:** a `bool` parameter on a public function.

**The guide's reason:** "Use `enum` function parameters instead of `bool` in cases where the purpose of the parameter is not obvious." The defect study found no replicated link between bool parameters and later bug fixes (docs/EVIDENCE.md), so this is readability.

**Exceptions:** Honoured, from the guide's fallback: a parameter with an inline comment on its line (`dry_run: bool, // skip writes`) isn't reported. Whether a purpose is "obvious" is a judgment for a suppression.

**Limits:** Only public functions are checked, since the rule is about APIs.

**On the corpus:** 2,367 finding(s) in 35 codebase(s); most in zellij 693, tauri 235, nushell 199.

### `into-parameter-in-application`

**Guide:** Accepting `Into`. **Flags:** `impl Into<T>` (or `T: Into<U>`) as a parameter in application code.

**The guide's reason:** "Application code should almost never use `impl Into<T>` parameters"; the convenience is weighed "against the cost of generics".

**Exceptions:** Honoured: library code isn't checked, as the guide allows `impl Into<T>` there "if call-site ergonomics are paramount". A file is application code when it belongs to a binary (`main.rs`, `src/bin/`, or a package with `main.rs` and no `lib.rs`).

**Limits:** A binary that is a thin shell over its own library modules is still application code to the guide.

**On the corpus:** 30 finding(s) in 6 codebase(s); most in alacritty 8, zellij 7, diesel 6.

### `rc-arc`

**Guide:** Avoid `Rc` and `Arc`. **Flags:** `Rc<T>` or `Arc<T>` in a field or signature.

**The guide's reason:** "Prefer unique ownership and borrowing over `Rc` and `Arc` for managing memory"; "These pointers add runtime overhead and their own unique complexity."

**Exceptions:** The guide allows them "when the alternative ownership and borrowing structure would be unreasonably complex to write, understand, or maintain", as "a judgment call to be discussed during code review". That can't be read from a type, so every use is reported and the judgment goes in a suppression.

**Limits:** Shared-state code (async runtimes, threads) uses `Arc` by design; this rule reports all of it.

**On the corpus:** 3,280 finding(s) in 33 codebase(s); most in wgpu 861, zellij 429, nushell 334.

### `tuple-fields`

**Guide:** Prefer named fields over tuples and non-newtype tuple structs. **Flags:** a tuple struct with two or more fields, a tuple-typed field, or a public function returning a tuple.

**The guide's reason:** "Prefer types with named fields over tuples, as tuple indexes (`.0`, `.1`, ...) quickly become confusing."

**Exceptions:** Honoured: newtypes (one field) aren't reported; nor are "very local contexts", taken as `let` bindings and private functions.

**Limits:** A tuple returned by a public function and used once nearby is still reported.

**On the corpus:** 675 finding(s) in 35 codebase(s); most in zellij 116, nushell 57, wgpu 54.

### `lifetime-name`

**Guide:** Naming lifetimes. **Flags:** a single-letter lifetime on a type, or more than two on a function.

**The guide's reason:** "Lifetimes should have descriptive names, especially type lifetimes."

**Exceptions:** Honoured: `'_`; up to two single-character lifetimes on a function, "simply expressing the relationship between a method's input and output"; `for<'a>` bounds aren't counted. "Extremely generic abstractions like `Cow`" are a judgment for a suppression.

**Limits:** None in shape.

**On the corpus:** 2,590 finding(s) in 36 codebase(s); most in wgpu 304, gitoxide 285, meilisearch 205.

### `must-use-missing`

**Guide:** `#[must_use]` on functions and on types. **Flags:** a public query-like function, or an error type, without `#[must_use]`.

**The guide's reason:** Apply `#[must_use]` when "The caller cannot use the function correctly if they ignore the return value. This includes pure functions", and to a type that "is an error type".

**Exceptions:** Honoured: functions returning `Result` are exempt, as the guide says the type already carries it. Only functions that look pure are reported: `&self` or no receiver, no `&mut` parameter, a return type of `bool`, `Option`, a reference or a primitive, and a name that isn't a verb whose result is commonly ignored (`insert`, `send`, ...).

**Limits:** Purity is guessed from the signature; a `&self` method with interior mutability can have a result that is fine to ignore. A type is an error type by its `Error` suffix.

**On the corpus:** 9,768 finding(s) in 34 codebase(s); most in zellij 1,581, gitoxide 775, nushell 740.

### `non-exhaustive-missing`

**Guide:** Apply `#[non_exhaustive]` to publicly-exposed enums and to public structs with public fields. **Flags:** a public enum of three or more variants, or a public struct with all fields public, without `#[non_exhaustive]`.

**The guide's reason:** "If an enum is part of a public API used by other teams, mark it `#[non_exhaustive]`"; adding a field to an all-public struct "is a breaking change, because callers can no longer name all fields in a struct literal".

**Exceptions:** Honoured: two-state enums (`Visible`/`Hidden`), the guide's example of an enum that never changes, aren't reported; only library crates and `pub` items are checked. The guide's other exceptions are judgments: an enum that is an existing ontology (`Red`, `Green`, `Blue`), a struct whose fields won't change (a `Rectangle`), a commitment to fix breakages with an LSC, and wanting functional record update syntax. Name one in a suppression.

**Limits:** Whether another team uses the item isn't visible.

**On the corpus:** 6,811 finding(s) in 33 codebase(s); most in zellij 1,579, meilisearch 953, wgpu 665.

### `len-without-is-empty`

**Guide:** Implement expected API sets together. **Flags:** a public `len()` with no `is_empty()` on the same type.

**The guide's reason:** "If you implement `len()`, also implement `is_empty()`" (Clippy `len_without_is_empty`).

**Exceptions:** None listed.

**Limits:** The two can be in different files; only this one is read.

**On the corpus:** 36 finding(s) in 12 codebase(s); most in regex 12, tantivy 5, alacritty 3.

### `getter-prefix`

**Guide:** Getters and setters. **Flags:** a getter named `get_foo()`.

**The guide's reason:** "`foo()` for a getter that returns `&T`, or `T` by value if `T: Copy`"; the Rust API Guidelines (C-GETTER) agree.

**Exceptions:** None listed. `get()`, `get_mut()` and `get_or_*()` aren't field getters and aren't reported.

**Limits:** A `get_foo(&self)` that computes rather than reads a field is still reported.

**On the corpus:** 440 finding(s) in 24 codebase(s); most in regex 89, clap 80, zellij 61.

### `constructor-name`

**Guide:** Naming initialization functions. **Flags:** a constructor named outside the guide's scheme, or `new()` returning `Result`/`Option`.

**The guide's reason:** "Constructors: Use `new()`, or `new_with_foo()` if there is more than one. If the constructor returns a `Result`, use `try_new()`." "Value-to-value conversions: Use `from_foo()`".

**Exceptions:** Honoured, as names the guide or the standard library use: `new`, `new_*`, `try_new`, `try_new_*`, `from_*`, `try_from_*`, `with_*`, `default`, and the std verbs `open`, `create`, `connect`, `bind`, `spawn`, `parse`, `load`, `build`, `empty`, `now`, `zeroed`, `random`.

**Limits:** A constructor is any inherent associated function without `self` returning `Self`, `Result<Self, _>` or `Option<Self>`.

**On the corpus:** 2,697 finding(s) in 37 codebase(s); most in typst 236, wgpu 225, zellij 217.

### `from-impl-without-constructor`

**Guide:** Pair important `From` impls with explicit constructor methods. **Flags:** `impl From<T> for X` with no explicit constructor on `X`.

**The guide's reason:** "add an inherent `::new()`, `::new_from_foo()`, or `::from_foo()` method" beside a `From` impl that is the primary way to build the type.

**Exceptions:** The guide says "important" `From` impls; which are important is a judgment for a suppression. Only types defined in the same file are checked.

**Limits:** A constructor in another file isn't seen.

**On the corpus:** 1,156 finding(s) in 35 codebase(s); most in tantivy 151, helix 132, gitoxide 88.

### `std-trait-as-method`

**Guide:** Implement known standard library traits instead of custom methods. **Flags:** a public method with a standard trait's name (`from_str`, `eq`, `next`, ...) on a type without that trait.

**The guide's reason:** "If a method matches the purpose and behavior of a standard library trait, implement that trait" (Clippy `should_implement_trait`).

**Exceptions:** Honoured: "both the method and the trait impl for discoverability" is fine, so a type with the trait implemented or derived in the same file isn't reported.

**Limits:** The trait impl can be in another file.

**On the corpus:** 81 finding(s) in 22 codebase(s); most in zellij 14, typst 11, meilisearch 6.

### `new-without-default`

**Guide:** Specific guidance for important traits: `Default`. **Flags:** `pub fn new()` with no arguments on a type without `Default`.

**The guide's reason:** "Implement if the type implements `new()` or otherwise has a default state" (Clippy `new_without_default`).

**Exceptions:** None listed.

**Limits:** The `Default` impl can be in another file.

**On the corpus:** 136 finding(s) in 22 codebase(s); most in wgpu 22, regex 15, zellij 14.

### `options-struct-default`

**Guide:** Options structs should implement Default. **Flags:** an all-public-field options struct (`FooOptions`, `FooConfig`, ...) without `Default`.

**The guide's reason:** "Structs with all-public fields that are primarily used as a 'bag of options' for a function should implement `Default`", so callers can write `Struct { a: 0, ..Default::default() }`.

**Exceptions:** None listed. A struct is a bag of options by its name: `Options`, `Opts`, `Config`, `Settings` or `Params`.

**Limits:** An options struct with another name isn't seen; one with a field that has no default can't derive it.

**On the corpus:** 229 finding(s) in 19 codebase(s); most in helix 70, gitoxide 30, meilisearch 29.

### `trait-as-organization`

**Guide:** When to define a trait versus an inherent implementation. **Flags:** a private trait with one implementation and no use as a bound.

**The guide's reason:** "Define a trait only" for generic programming, for `impl Trait`/`dyn Trait`, or as an extension trait; "Don't use traits merely to organize code."

**Exceptions:** Honoured: public traits and extension traits (`..Ext`) aren't reported, since other modules may use them as bounds.

**Limits:** A bound in another file of the same crate isn't seen.

**On the corpus:** 44 finding(s) in 16 codebase(s); most in wgpu 8, regex 7, image 5.

### `ext-trait-in-bounds`

**Guide:** Extension traits. **Flags:** an extension trait (`FooExt`) used as a bound, `impl Trait` or `dyn Trait`.

**The guide's reason:** An extension trait's "sole purpose is to add methods to the type; it is not intended for use in generics (`T: Trait`, `impl Trait`) or dynamic dispatch (`dyn Trait`)".

**Exceptions:** None listed. A trait is an extension trait by the `Ext` suffix the guide requires.

**Limits:** A trait that ends in `Ext` without being an extension trait is reported.

**On the corpus:** 46 finding(s) in 6 codebase(s); most in gitoxide 21, sqlx 15, clap 6.

### `macro-definition`

**Guide:** Macros. **Flags:** a `macro_rules!` or procedural macro definition.

**The guide's reason:** The guide ranks "Regular Rust functions, types, traits, objects, and modules" first: "Avoid macros, code generation, and metaprogramming where possible"; "IDEs, Code Search and other developer tools will work less well"; "Large scale changes are more difficult and expensive to apply."

**Exceptions:** None listed; the guide orders the alternatives (`macro_rules!` before other code generation, procedural macros last) rather than excepting any.

**Limits:** None in shape.

**On the corpus:** 1,172 finding(s) in 34 codebase(s); most in diesel 124, tokio 116, pyo3 115.

### `code-before-main`

**Guide:** Do not run code before `main()`. **Flags:** a function or static arranged to run before `main()` (`#[ctor]`, an `.init_array` section).

**The guide's reason:** "Do not cause code to run before `main()`, for example by using techniques such as the ctor crate."

**Exceptions:** None listed.

**Limits:** Only the `ctor`/`dtor` attributes and the known initialiser link sections are recognised.

**On the corpus:** 0 finding(s) in 0 codebase(s).

### `unstable-feature`

**Guide:** No unstable features. **Flags:** `#![feature(..)]`.

**The guide's reason:** "Don't use unstable features."

**Exceptions:** The guide's exceptions are organisational: features Android Embedded needs to build an artifact, a short list blanket-approved for Chrome, and Fuchsia's own policy. Cite the approval in a suppression.

**Limits:** None in shape.

**On the corpus:** 114 finding(s) in 23 codebase(s); most in gitoxide 35, actix-web 9, tracing 9.

### `rustfmt-skip`

**Guide:** Opting out of automatic formatting. **Flags:** `#[rustfmt::skip]` without a stated reason.

**The guide's reason:** Opt out of formatting only for "Manual alignment" or "Generated code"; "Formatting should not be skipped for personal style choices."

**Exceptions:** Honoured: both listed exceptions, when a comment on the attribute's line or the line above mentions alignment (`align`) or generated code (`generat`).

**Limits:** A reason worded differently isn't recognised.

**On the corpus:** 68 finding(s) in 12 codebase(s); most in eza 41, typst 10, alacritty 4.

### `api-in-non-leaf-module`

**Guide:** Don't implement APIs in non-leaf modules. **Flags:** a module that declares submodules and also holds more than 50 lines of definitions.

**The guide's reason:** "The only items in non-leaf modules should be module declarations (not definitions) and re-exports."

**Exceptions:** Honoured: `#[cfg(test)] mod` and `#[cxx::bridge]` modules don't make a module non-leaf, and "a small amount of core 'ties things together' implementation code" is allowed, taken here as up to 50 lines of code. The number is this tool's, not the guide's.

**Limits:** The 50-line allowance is a guess at "a small amount".

**On the corpus:** 671 finding(s) in 37 codebase(s); most in gitoxide 103, meilisearch 56, nushell 51.

### `file-named-after-item`

**Guide:** Name source files and leaf modules after the primary user-defined item. **Flags:** a file whose one public type isn't what it is named after.

**The guide's reason:** "When a leaf module has a primary user-defined item (e.g., `Foo`), the module should be named after that item (e.g., `mod foo` and `foo.rs`)."

**Exceptions:** Honoured: a module "not primarily about any particular abstraction" may be named as desired, so files with no public type, more than one, or two or more public functions beside it aren't reported; nor are `mod.rs`, `lib.rs`, `main.rs` and `build.rs`.

**Limits:** The primary item is guessed as the file's only public type.

**On the corpus:** 1,584 finding(s) in 37 codebase(s); most in nushell 551, gitoxide 112, starship 109.

### `check-then-unwrap`

**Guide:** Prefer pattern matching over check-then-unwrap. **Flags:** `if x.is_some() { .. x.unwrap() .. }` (or `is_ok`, `is_none`, `is_err`).

**The guide's reason:** "instead of using `Option::unwrap` on a value known to be valid, use pattern matching to prove correctness to the compiler".

**Exceptions:** None listed.

**Limits:** The checked value and the unwrapped one are matched by their text.

**On the corpus:** 31 finding(s) in 8 codebase(s); most in meilisearch 8, zellij 6, clap 5.

### `assert-without-message`

**Guide:** Use context strings to clarify assertions. **Flags:** `assert!`, `assert_eq!` or `assert_ne!` with no message.

**The guide's reason:** "Use the optional format string argument for `assert!` to convey this context": why it should hold, a value worth logging, or the API misuse it catches.

**Exceptions:** The guide adds "Don't use the context string merely to repeat the condition", which can't be checked. Tests are exempt here, which is this tool's choice, not the guide's: a test's assertion is its own context.

**Limits:** None in shape.

**On the corpus:** 1,547 finding(s) in 33 codebase(s); most in tantivy 228, tokio 181, wgpu 169.

### `combinator-chain`

**Guide:** Don't use iterator combinator chains when direct iteration is clearer. **Flags:** `for_each`, a combinator chain in a `for` header, or a chain of five or more adapters.

**The guide's reason:** "Avoid long and complex combinator chains"; "Don't construct combinator chains in the header of a `for` loop"; "Avoid `Iterator::for_each` unless you're convinced you have a good reason. A `for` loop is usually easier to read."

**Exceptions:** Honoured: the guide prefers combinators "for shorter chains and for cases where the combinator itself provides a lot of logic", so chains under five adapters and `for` headers with a single adapter (`.iter().enumerate()`) aren't reported. The thresholds are this tool's.

**Limits:** Length in lines, not adapter count, may be what makes a chain hard to read.

**On the corpus:** 484 finding(s) in 32 codebase(s); most in gitoxide 71, zellij 62, image 44.

### `string-concat-plus`

**Guide:** Prefer `format!` / `write!` for concatenation and substitution. **Flags:** three or more strings joined with `+`, or `replace` on a string literal.

**The guide's reason:** "To concatenate more than two strings... use `format!(\"{a}{b}{c}\")` instead of `a + b + c`"; "To perform substitutions on a literal string (instead of using `str::replace`)"; "`format!` / `write!` require fewer allocations".

**Exceptions:** None listed.

**Limits:** Operands count as strings when one is a literal, `format!`, `to_string()`, `to_owned()` or `String::from`; numeric sums aren't reported.

**On the corpus:** 15 finding(s) in 10 codebase(s); most in clap 3, cargo 2, mdBook 2.

### `inferred-conversion-type`

**Guide:** Don't overuse type inference. **Flags:** `let x = y.into();` or `.parse()` with no type, or a turbofish on `collect` instead of a type on the `let`.

**The guide's reason:** "Add enough explicit types so that someone familiar with the codebase can figure out what the inferred types are", especially "When calling methods with a generic return type, such as `.into()` or `.parse()`"; "Prefer to add the types to variable declarations, rather than as turbofishes."

**Exceptions:** Honoured: a turbofish with a `_` inference marker (`collect::<Vec<_>>()`) is fine when the compiler forces one, as the guide says.

**Limits:** None in shape.

**On the corpus:** 738 finding(s) in 36 codebase(s); most in nushell 161, gitoxide 66, tauri 50.

### `poisoned-mutex-recovery`

**Guide:** Handling poison. **Flags:** taking the data out of a poisoned lock (`PoisonError::into_inner`).

**The guide's reason:** "Never reach inside a poisoned mutex to get at its insides, no matter how tempting the use case may seem." The standard library calls poisoning "only advisory" and documents `into_inner` for exactly this, and the corpus uses it deliberately, which is why this is a style rule and not a defect (docs/EVIDENCE.md).

**Exceptions:** None listed.

**Limits:** Only `unwrap_or_else(|e| e.into_inner())`, `unwrap_or_else(PoisonError::into_inner)` and `Err(e) => e.into_inner()` on `lock()`, `read()`, `write()` and `try_lock()` are recognised.

**On the corpus:** 66 finding(s) in 10 codebase(s); most in Ferrock 26, nushell 12, zellij 12.

### `catch-unwind`

**Guide:** Don't call `panic::catch_unwind`. **Flags:** `catch_unwind` in first-party code.

**The guide's reason:** "Use of `panic::catch_unwind` is prohibited in first-party Rust code." Where the build profile aborts on panic it can't work at all; that is the warning `catch-unwind-under-panic-abort`, and this rule covers the rest.

**Exceptions:** The guide accepts it "as a defense in depth mechanism as long as it is never relied upon for any properties", with the code doing "most of what it can to avoid panics", and only where `panic=unwind` is possible. Reliance can't be read from the call: say so in a suppression.

**Limits:** A local function that happens to be named `catch_unwind` is taken for std's.

**On the corpus:** 59 finding(s) in 10 codebase(s); most in diesel 15, tokio 13, nushell 9.

### `missing-docs`

**Guide:** When and how to write documentation. **Flags:** a public item with no doc comment.

**The guide's reason:** "Add documentation comments to public APIs, potentially using the `missing_docs` lint to help with this."

**Exceptions:** Honoured: enum variants needn't be documented, as the guide says. `#[doc(hidden)]` items, `pub(crate)` items, test modules and non-library crates aren't checked.

**Limits:** A re-export or a macro-generated item isn't seen.

**On the corpus:** 28,147 finding(s) in 35 codebase(s); most in zellij 7,595, meilisearch 3,672, nushell 3,061.

### `doc-comment-style`

**Guide:** When and how to write documentation. **Flags:** a doc comment that restates the item's name, or lacks capitalisation and punctuation.

**The guide's reason:** "Comments should say something that the code does not already say" (the guide's example: `/// Removes an element.` on `remove_element`); "Use capitalization and punctuation."

**Exceptions:** None listed. A first line that starts with code, a heading, a list or a link isn't checked for its capital or its full stop.

**Limits:** Restating is judged by word overlap, so a short comment on a well-named item can be reported even when it adds a nuance.

**On the corpus:** 6,155 finding(s) in 35 codebase(s); most in nushell 644, meilisearch 604, diesel 592.

### `panic-undocumented`

**Guide:** When and how to write documentation. **Flags:** a public function that can panic on its arguments, with no `# Panics` section.

**The guide's reason:** "Always document panic cases for functions which may panic due to malformed input."

**Exceptions:** None listed. Only panics that involve a parameter (`assert!(n > 0)`, `input.parse().unwrap()`) and explicit `panic!`-family macros count; a panic on internal state isn't "malformed input".

**Limits:** A panic inside a callee isn't seen; indexing isn't counted.

**On the corpus:** 935 finding(s) in 32 codebase(s); most in gitoxide 104, wgpu 104, cargo 91.

### `safety-undocumented`

**Guide:** When and how to write documentation. **Flags:** a public `unsafe fn` or `unsafe trait` with no `# Safety` section.

**The guide's reason:** "Clearly document invariants for unsafe functions and traits." (Clippy `missing_safety_doc`)

**Exceptions:** None listed.

**Limits:** A contract documented under another heading is reported.

**On the corpus:** 511 finding(s) in 11 codebase(s); most in pyo3 251, wgpu 247, cargo 2.

### `safety-comment-prefix`

**Guide:** Unsafe code. **Flags:** an `unsafe` block or impl whose justification doesn't start with `SAFETY:`.

**The guide's reason:** "Add a safety comment starting with `SAFETY:` that explains why the code is correct." The standard library's own policy uses the same marker. The note `unsafe-without-comment` covers having no justification at all; this is the wording.

**Exceptions:** None listed.

**Limits:** None in shape.

**On the corpus:** 808 finding(s) in 28 codebase(s); most in tokio 178, pyo3 159, wgpu 134.

## Rules the guide states that syntax can't check

| Guide rule | Why there is no rule |
|---|---|
| Iterators vs. owned collections in return types; iterators vs. slices in parameters | six listed exceptions turn on what callers need and whether the function builds a vector anyway |
| Prefer borrows over deep copies (`.clone()`) | which clones are deep, and which could be borrows, needs types; the defect study also found no effect of clone counts |
| Use `#[derive()]` if possible | whether a manual impl "can be expressed as a derive" needs the impl's semantics |
| Generics vs. `dyn Trait` | a code-size judgment |
| Add trait implementations when they become necessary; `Copy`, `Clone`, `Hash`+`Eq`, `Ord` guidance | "whenever possible" and "likely to be used as a map key" are about intent |
| Prefer locally-scoped trait imports | which imports are traits, and how widely they're used, needs name resolution |
| Use `const` functions responsibly | whether a function will stay `const` is a promise, not a property of the code |
| Do not access items marked `#[doc(hidden)]` that you don't own | the attribute is in another crate |
| Avoid overloading for non-mathematical purposes | what an `Add` impl means needs the type's semantics |
| Don't omit patterns on `non_exhaustive` enums when matching | the enum's attribute is in another crate; inside its own crate the attribute has no effect |
| Avoid abbreviations | needs a dictionary of what "someone outside your project is unlikely to know" |
| Naming generic parameters; naming methods (`as_`/`to_`/`into_`) | "somewhat indicative" is a judgment; the method prefixes need the receiver's semantics (Clippy's type-aware `wrong_self_convention` covers them) |
| Reporting errors: `Option` vs. `Result` vs. `panic!` | the API contract |
