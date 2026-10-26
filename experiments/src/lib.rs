//! Helpers shared by the experiments in `tests/`. Each experiment is a test that passes when the
//! harm a rule claims is observed, so `cargo test` here re-checks every claim.

use std::process::{Command, Output};

/// Runs one of this crate's helper binaries, for experiments that kill or abort the process.
pub fn run_helper(path: &str, args: &[&str]) -> Output {
    Command::new(path).args(args).output().expect("helper binary runs")
}
