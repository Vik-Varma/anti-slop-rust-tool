//! What a file is for can depend on files around it: `#[cfg(test)] mod unit;` makes `unit.rs` (and
//! everything under `unit/`) test code, and every module of a crate with a `main.rs` but no `lib.rs`
//! belongs to a binary. This resolves both from the file tree, without running Cargo.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::analyzer::{blank_frontmatter, is_test_item};
use crate::context::FileKind;

/// The kind of each file in `files`, found under `root`, using the crate layout where it says more
/// than the path. Paths are judged relative to `root`, so a checkout that happens to live in a
/// directory called `tests` isn't all test code.
pub fn kinds(root: &Path, files: &[PathBuf]) -> HashMap<PathBuf, FileKind> {
    let tests = test_modules(files);
    let mut bin_crates: HashMap<PathBuf, bool> = HashMap::new();
    files
        .iter()
        .map(|f| {
            let mut kind = FileKind::of(f.strip_prefix(root).unwrap_or(f));
            if kind.ships() && tests.iter().any(|t| t.covers(f)) {
                kind = FileKind::Test;
            } else if kind == FileKind::Library && in_binary_crate(f, &mut bin_crates) {
                kind = FileKind::Binary;
            }
            (f.clone(), kind)
        })
        .collect()
}

/// A module declared only for tests: its own file, and the directory its submodules live in.
#[derive(Debug, PartialEq)]
struct TestModule {
    file: PathBuf,
    dir: PathBuf,
}

impl TestModule {
    fn at(file: PathBuf) -> TestModule {
        // `x/mod.rs` keeps its submodules in `x/`; `x.rs` keeps them in `x/` too.
        let dir = if file.file_name().is_some_and(|n| n == "mod.rs") {
            file.parent().map(Path::to_path_buf).unwrap_or_default()
        } else {
            file.with_extension("")
        };
        TestModule { file, dir }
    }

    fn covers(&self, f: &Path) -> bool {
        f == self.file || f.starts_with(&self.dir)
    }
}

/// Every out-of-line module that only exists under `cfg(test)`: declared with a test-only `cfg`,
/// or nested (inline or not) inside one. Each file is parsed with `syn`, as the analyzer does.
fn test_modules(files: &[PathBuf]) -> Vec<TestModule> {
    let parsed: Vec<(&PathBuf, syn::File)> = files
        .iter()
        .filter_map(|f| {
            let src = fs::read_to_string(f).ok()?;
            Some((f, syn::parse_file(&blank_frontmatter(&src)).ok()?))
        })
        .collect();
    let mut out: Vec<TestModule> = Vec::new();
    // A test module's own submodules are test code too, whichever order the files come in, so
    // repeat until nothing new is found.
    loop {
        let before = out.len();
        for (file, ast) in &parsed {
            let parent = file.parent().unwrap_or(Path::new(""));
            let stem = file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            // A file's out-of-line submodules live beside it if it's `mod.rs`/`lib.rs`/`main.rs`,
            // else in a directory named after it.
            let dir = if matches!(stem, "mod" | "lib" | "main") {
                parent.to_path_buf()
            } else {
                parent.join(stem)
            };
            let in_test = tests_cover(&out, file);
            let mut found = Vec::new();
            walk_items(&ast.items, &dir, parent, false, in_test, &mut found);
            for t in found {
                if !out.contains(&t) {
                    out.push(t);
                }
            }
        }
        if out.len() == before {
            return out;
        }
    }
}

fn tests_cover(tests: &[TestModule], f: &Path) -> bool {
    tests.iter().any(|t| t.covers(f))
}

/// Walks `items` of a module whose out-of-line submodules live in `dir`. `#[path]` on a module
/// that isn't inside an inline block is relative to the file's own directory (`file_dir`).
fn walk_items(
    items: &[syn::Item],
    dir: &Path,
    file_dir: &Path,
    inline: bool,
    test: bool,
    out: &mut Vec<TestModule>,
) {
    for item in items {
        let syn::Item::Mod(m) = item else { continue };
        let test = test || is_test_item(&m.attrs);
        let name = m.ident.to_string();
        let path_attr = m.attrs.iter().find_map(|a| {
            let syn::Meta::NameValue(nv) = &a.meta else {
                return None;
            };
            if !nv.path.is_ident("path") {
                return None;
            }
            match &nv.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) => Some(s.value()),
                _ => None,
            }
        });
        match (&m.content, path_attr) {
            (Some((_, inner)), _) => walk_items(inner, &dir.join(&name), file_dir, true, test, out),
            (None, Some(p)) if test => {
                let base = if inline { dir } else { file_dir };
                out.push(TestModule::at(base.join(p)));
            }
            (None, None) if test => {
                for candidate in [
                    dir.join(format!("{name}.rs")),
                    dir.join(&name).join("mod.rs"),
                ] {
                    if candidate.is_file() {
                        out.push(TestModule::at(candidate));
                    }
                }
            }
            _ => {}
        }
    }
}

/// Whether `file` is under `src/` of a package that has `src/main.rs` and no `src/lib.rs`.
fn in_binary_crate(file: &Path, cache: &mut HashMap<PathBuf, bool>) -> bool {
    let mut dir = file.parent();
    while let Some(d) = dir {
        if d.join("Cargo.toml").is_file() {
            let binary_only = *cache.entry(d.to_path_buf()).or_insert_with(|| {
                d.join("src/main.rs").is_file() && !d.join("src/lib.rs").is_file()
            });
            return binary_only && file.starts_with(d.join("src"));
        }
        dir = d.parent();
    }
    false
}

/// The directory a file's crate-relative path should be judged from: the parent of its package,
/// so `crates/foo-test-support/src/lib.rs` keeps its telling directory but not the checkout's.
pub fn judging_base(file: &Path) -> PathBuf {
    let mut dir = file.parent();
    while let Some(d) = dir {
        if d.join("Cargo.toml").is_file() {
            return d.parent().map(Path::to_path_buf).unwrap_or_default();
        }
        dir = d.parent();
    }
    PathBuf::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decls(src: &str, file: &str) -> Vec<TestModule> {
        let parsed = syn::parse_file(src).expect("test source parses");
        let mut out = Vec::new();
        let file = Path::new(file);
        let parent = file.parent().unwrap_or(Path::new(""));
        walk_items(&parsed.items, parent, parent, false, false, &mut out);
        out
    }

    #[test]
    fn path_attributes_and_cfg_forms() {
        let src = "#[cfg(test)]\n#[path = \"spec/mod.rs\"]\nmod spec;\n\
                   #[cfg(all(feature = \"latest\", unix))]\nmod backend;\n\
                   #[cfg(all(\n    test,\n    unix\n))]\n#[path = \"u.rs\"]\nmod unit; // unit tests\n";
        let got = decls(src, "src/lib.rs");
        assert_eq!(
            got,
            vec![
                TestModule {
                    file: "src/spec/mod.rs".into(),
                    dir: "src/spec".into()
                },
                TestModule {
                    file: "src/u.rs".into(),
                    dir: "src/u".into()
                },
            ]
        );
    }
}
