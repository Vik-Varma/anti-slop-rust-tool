use std::path::Path;

/// What a file is for. Tests, examples and build scripts may panic on purpose; libraries may not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Library,
    /// `src/main.rs` or `src/bin/**`: the process entry point owns process-wide decisions.
    Binary,
    /// `tests/**`, `benches/**`.
    Test,
    Example,
    BuildScript,
}

impl FileKind {
    pub fn of(path: &Path) -> FileKind {
        let parts: Vec<&str> = path.iter().filter_map(|p| p.to_str()).collect();
        let file = parts.last().copied().unwrap_or_default();
        let stem = file.strip_suffix(".rs").unwrap_or(file);
        let dirs = &parts[..parts.len().saturating_sub(1)];
        if file == "build.rs" {
            FileKind::BuildScript
        } else if dirs.iter().any(|d| is_test_dir(d)) || TEST_FILE_STEMS.contains(&stem) {
            FileKind::Test
        } else if dirs.contains(&"examples") {
            FileKind::Example
        } else if file == "main.rs" || parts.windows(2).any(|w| w[0] == "src" && w[1] == "bin") {
            FileKind::Binary
        } else {
            FileKind::Library
        }
    }

    /// Whether a panic here crashes a user's program rather than a test, demo or build.
    pub fn ships(self) -> bool {
        matches!(self, FileKind::Library | FileKind::Binary)
    }
}

/// Directories of code that only runs in development: tests, test-support crates, benchmarks,
/// fuzz targets and `xtask` build helpers.
const TEST_DIRS: &[&str] = &[
    "tests",
    "test",
    "testing",
    "benches",
    "bench",
    "benchmarks",
    "fuzz",
    "xtask",
];
const TEST_DIR_SUFFIXES: &[&str] = &[
    "-test",
    "-tests",
    "_test",
    "_tests",
    "-test-support",
    "-testing",
    "-bench",
    "-fuzz",
];
const TEST_FILE_STEMS: &[&str] = &[
    "test",
    "tests",
    "test_utils",
    "test_helpers",
    "testing",
    "test_support",
];

fn is_test_dir(d: &str) -> bool {
    TEST_DIRS.contains(&d)
        || TEST_DIR_SUFFIXES.iter().any(|s| d.ends_with(s))
        || d.starts_with("xtask-")
}

/// Where the visitor is right now. Pushed on entering a function, closure, block or module.
#[derive(Clone, Debug, Default)]
pub struct Scope {
    /// Inside `#[test]`, `#[cfg(test)]` or a test-only file.
    pub test: bool,
    /// Inside an `async fn` or `async` block, and not in a nested sync closure or fn.
    pub in_async: bool,
    /// Inside `impl Drop for T { fn drop }`.
    pub in_drop: bool,
    /// Inside a `const fn` or a `const`/`static` initialiser, where a panic is a compile error.
    pub in_const: bool,
    /// The enclosing function returns a `Result`, so an error could have been propagated with `?`.
    pub returns_result: bool,
    /// The enclosing function's display name.
    pub function: Option<String>,
}
