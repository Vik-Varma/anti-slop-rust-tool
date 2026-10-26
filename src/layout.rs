//! What a file is for can depend on files around it: `#[cfg(test)] mod unit;` makes `unit.rs` (and
//! everything under `unit/`) test code, and every module of a crate with a `main.rs` but no `lib.rs`
//! belongs to a binary. This resolves both from the file tree, without running Cargo.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::context::FileKind;

/// The kind of each file in `files`, found under `root`, using the crate layout where it says more
/// than the path. Paths are judged relative to `root`, so a checkout that happens to live in a
/// directory called `tests` isn't all test code.
pub fn kinds(root: &Path, files: &[PathBuf]) -> HashMap<PathBuf, FileKind> {
    let test_roots = test_modules(files);
    let mut bin_crates: HashMap<PathBuf, bool> = HashMap::new();
    files
        .iter()
        .map(|f| {
            let mut kind = FileKind::of(f.strip_prefix(root).unwrap_or(f));
            if kind.ships()
                && test_roots
                    .iter()
                    .any(|t| f == t || f.starts_with(t.with_extension("")))
            {
                kind = FileKind::Test;
            } else if kind == FileKind::Library && in_binary_crate(f, &mut bin_crates) {
                kind = FileKind::Binary;
            }
            (f.clone(), kind)
        })
        .collect()
}

/// Files declared as `#[cfg(test)] mod name;` by another file in the set.
fn test_modules(files: &[PathBuf]) -> HashSet<PathBuf> {
    let mut out = HashSet::new();
    for file in files {
        let Ok(src) = fs::read_to_string(file) else {
            continue;
        };
        for (name, path_attr) in cfg_test_mod_decls(&src) {
            let parent = file.parent().unwrap_or(Path::new(""));
            let stem = file
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let owns_dir = matches!(stem, "mod" | "lib" | "main");
            if let Some(p) = path_attr {
                out.insert(parent.join(p));
                continue;
            }
            let dir = if owns_dir {
                parent.to_path_buf()
            } else {
                parent.join(stem)
            };
            for candidate in [
                dir.join(format!("{name}.rs")),
                dir.join(&name).join("mod.rs"),
            ] {
                if candidate.is_file() {
                    // `unit/mod.rs` stands for the whole `unit/` directory.
                    let root = if candidate.ends_with("mod.rs") {
                        dir.join(&name).with_extension("rs")
                    } else {
                        candidate
                    };
                    out.insert(root);
                }
            }
        }
    }
    out
}

/// `(module name, #[path] value)` for each out-of-line `mod name;` whose attributes include a
/// `cfg` that only holds under `test`. A text scan: it runs before parsing, over every file.
fn cfg_test_mod_decls(src: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    let mut attrs: Vec<&str> = Vec::new();
    for line in src.lines() {
        let mut t = line.trim();
        // Attributes, possibly several on one line ahead of the item: `#[cfg(test)] mod unit;`.
        while t.starts_with("#[") {
            let Some(end) = t.find(']') else { break };
            attrs.push(&t[..=end]);
            t = t[end + 1..].trim_start();
        }
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        let decl = t
            .trim_start_matches("pub(crate) ")
            .trim_start_matches("pub(super) ")
            .trim_start_matches("pub ");
        if let Some(name) = decl.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')) {
            let is_test = attrs.iter().any(|a| {
                let a: String = a.chars().filter(|c| !c.is_whitespace()).collect();
                a == "#[cfg(test)]"
                    || (a.starts_with("#[cfg(all(")
                        && a.contains("test")
                        && !a.contains("not(test"))
            });
            if is_test {
                let path = attrs.iter().find_map(|a| {
                    let rest = a
                        .strip_prefix("#[path")?
                        .trim_start()
                        .strip_prefix('=')?
                        .trim();
                    Some(
                        rest.trim_end_matches(']')
                            .trim()
                            .trim_matches('"')
                            .to_string(),
                    )
                });
                out.push((name.trim().to_string(), path));
            }
        }
        attrs.clear();
    }
    out
}

/// Whether `file` is under `src/` of a package that has `src/main.rs` and no `src/lib.rs`.
fn in_binary_crate(file: &Path, cache: &mut HashMap<PathBuf, bool>) -> bool {
    let mut dir = file.parent();
    while let Some(d) = dir {
        if d.join("Cargo.toml").is_file() {
            let pkg = d.to_path_buf();
            return *cache.entry(pkg.clone()).or_insert_with(|| {
                file.starts_with(pkg.join("src"))
                    && pkg.join("src/main.rs").is_file()
                    && !pkg.join("src/lib.rs").is_file()
            });
        }
        dir = d.parent();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::cfg_test_mod_decls;

    #[test]
    fn finds_test_module_declarations() {
        let src = "mod a;\n#[cfg(test)]\nmod unit;\n#[cfg(test)]\n#[path = \"t/x.rs\"]\nmod x;\n#[cfg(not(test))]\nmod real;\n#[cfg(test)] mod inline { }\n#[cfg(test)] mod one_line;\n";
        assert_eq!(
            cfg_test_mod_decls(src),
            vec![
                ("unit".into(), None),
                ("x".into(), Some("t/x.rs".into())),
                ("one_line".into(), None)
            ]
        );
    }
}
