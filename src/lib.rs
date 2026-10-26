//! anti-slop: finds Rust code that is bad by an evidence-backed definition.
//!
//! "Bad code", here, is code with a demonstrated mechanism for failing (a panic, undefined
//! behaviour, a lost error, a stall) or a measured link to defects that size alone doesn't
//! explain. docs/DEFINITION.md has the full definition and docs/EVIDENCE.md the evidence for, and
//! against, every rule.

mod analyzer;
mod checks;
mod comments;
mod context;
mod features;
mod finding;
mod layout;
pub mod metrics;
pub mod rules;
mod suppress;
mod walk;

pub use analyzer::{FileReport, analyze_source, analyze_source_as};
pub use context::FileKind;
pub use features::Features;
pub use finding::{Finding, Severity};
pub use walk::{AnalyzeError, analyze_paths, rust_files};
