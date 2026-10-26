//! anti-slop: finds evidence-backed "bad code" patterns in Rust source.
//!
//! See `docs/DEFINITION.md` for what counts as bad code and
//! `docs/EVIDENCE.md` for the evidence behind each rule.

pub mod analyze;
pub mod ast_util;
pub mod config;
pub mod context;
pub mod duplicates;
pub mod finding;
pub mod lexer;
pub mod metrics;
pub mod rules;
