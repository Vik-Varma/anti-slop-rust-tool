//! Works out which files are compiled into which target by following
//! `mod name;` declarations from each crate's library and binary roots.

use crate::context::FileKind;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Default, Debug)]
pub struct Layout {
    /// Files only reachable through `#[cfg(test)] mod ..;`.
    test_files: HashSet<PathBuf>,
    /// Files reachable from a library root.
    lib_files: HashSet<PathBuf>,
    /// Files reachable from a binary root.
    bin_files: HashSet<PathBuf>,
}

fn canon(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

pub fn crate_root(abs: &Path) -> Option<PathBuf> {
    let mut dir = abs.parent();
    while let Some(d) = dir {
        if d.join("Cargo.toml").is_file() {
            return Some(d.to_path_buf());
        }
        dir = d.parent();
    }
    None
}

#[derive(Debug, PartialEq)]
pub struct ModDecl {
    pub name: String,
    pub path_attr: Option<String>,
    pub cfg_test: bool,
}

/// `mod x;` declarations in a source file (textual scan, attributes on the
/// preceding lines are taken into account).
pub fn mod_decls(src: &str) -> Vec<ModDecl> {
    let mut out = Vec::new();
    let mut cfg_test = false;
    let mut path_attr: Option<String> = None;
    for line in src.lines().map(str::trim) {
        if line.starts_with("#[") {
            if line.starts_with("#[cfg(") && line.contains("test") && !line.contains("not(") {
                cfg_test = true;
            }
            if let Some(rest) = line.strip_prefix("#[path") {
                path_attr = rest.split('"').nth(1).map(str::to_string);
            }
            continue;
        }
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let mut decl = line;
        for vis in ["pub(crate) ", "pub(super) ", "pub(self) ", "pub "] {
            decl = decl.trim_start_matches(vis);
        }
        if let Some(name) = decl.strip_prefix("mod ").and_then(|r| r.strip_suffix(';')) {
            let name = name.trim().trim_start_matches("r#");
            if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                out.push(ModDecl { name: name.to_string(), path_attr: path_attr.take(), cfg_test });
            }
        }
        cfg_test = false;
        path_attr = None;
    }
    out
}

fn child_files(file: &Path, decl: &ModDecl) -> Vec<PathBuf> {
    let Some(dir) = file.parent() else { return Vec::new() };
    if let Some(p) = &decl.path_attr {
        return vec![dir.join(p)];
    }
    let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let base = if matches!(stem.as_str(), "lib" | "main" | "mod") { dir.to_path_buf() } else { dir.join(&stem) };
    vec![base.join(format!("{}.rs", decl.name)), base.join(&decl.name).join("mod.rs")]
}

/// Library and binary root files of a crate.
fn roots(root: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut libs = Vec::new();
    let mut bins = Vec::new();
    let manifest =
        std::fs::read_to_string(root.join("Cargo.toml")).ok().and_then(|t| toml::from_str::<toml::Value>(&t).ok());
    if let Some(m) = &manifest {
        if let Some(p) = m.get("lib").and_then(|l| l.get("path")).and_then(|p| p.as_str()) {
            libs.push(root.join(p));
        }
        if let Some(bs) = m.get("bin").and_then(|b| b.as_array()) {
            for b in bs {
                if let Some(p) = b.get("path").and_then(|p| p.as_str()) {
                    bins.push(root.join(p));
                }
            }
        }
    }
    libs.push(root.join("src/lib.rs"));
    bins.push(root.join("src/main.rs"));
    if let Ok(entries) = std::fs::read_dir(root.join("src/bin")) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().is_some_and(|x| x == "rs") {
                bins.push(p);
            } else if p.join("main.rs").is_file() {
                bins.push(p.join("main.rs"));
            }
        }
    }
    (libs.into_iter().filter(|p| p.is_file()).collect(), bins.into_iter().filter(|p| p.is_file()).collect())
}

/// Walks the module tree from `start`, adding reachable files to `seen`;
/// files under a `#[cfg(test)]` declaration go to `tests` instead.
fn walk(
    start: &Path,
    seen: &mut HashSet<PathBuf>,
    tests: &mut HashSet<PathBuf>,
    cache: &mut HashMap<PathBuf, Vec<ModDecl>>,
) {
    let mut stack = vec![(canon(start), false)];
    while let Some((file, in_test)) = stack.pop() {
        let set = if in_test { &mut *tests } else { &mut *seen };
        if !set.insert(file.clone()) {
            continue;
        }
        let decls = cache
            .entry(file.clone())
            .or_insert_with(|| std::fs::read_to_string(&file).map(|s| mod_decls(&s)).unwrap_or_default());
        for d in decls.iter() {
            for child in child_files(&file, d) {
                if child.is_file() {
                    stack.push((canon(&child), in_test || d.cfg_test));
                    break;
                }
            }
        }
    }
}

pub fn compute(files: &[PathBuf]) -> Layout {
    let mut layout = Layout::default();
    let crate_roots: HashSet<PathBuf> = files.iter().filter_map(|f| crate_root(&canon(f))).collect();
    let mut cache = HashMap::new();
    for root in crate_roots {
        let (libs, bins) = roots(&root);
        for l in libs {
            walk(&l, &mut layout.lib_files, &mut layout.test_files, &mut cache);
        }
        for b in bins {
            walk(&b, &mut layout.bin_files, &mut layout.test_files, &mut cache);
        }
    }
    // A file reachable normally from some root is not test-only.
    let normal: HashSet<PathBuf> = layout.lib_files.union(&layout.bin_files).cloned().collect();
    layout.test_files.retain(|f| !normal.contains(f));
    layout
}

impl Layout {
    pub fn classify(&self, path: &Path) -> FileKind {
        let abs = canon(path);
        let root = crate_root(&abs);
        let rel = root.as_ref().and_then(|r| abs.strip_prefix(r).ok()).map(Path::to_path_buf).unwrap_or(abs.clone());
        let kind = FileKind::from_path(&rel);
        if kind != FileKind::Library && kind != FileKind::Binary {
            return kind;
        }
        if self.test_files.contains(&abs) {
            return FileKind::Test;
        }
        // Crates that exist to support tests (`foo-test-support`, `test-utils`, ...).
        let crate_name = root.as_ref().and_then(|r| r.file_name()).map(|n| n.to_string_lossy().to_ascii_lowercase());
        if crate_name.is_some_and(|n| {
            n.split(['-', '_']).any(|w| matches!(w, "test" | "tests" | "testing" | "fuzz" | "fuzzing"))
        }) {
            return FileKind::Test;
        }
        if self.bin_files.contains(&abs) && !self.lib_files.contains(&abs) {
            return FileKind::Binary;
        }
        kind
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mod_decls() {
        let src = "mod a;\n#[cfg(test)]\nmod tests;\n#[cfg(test)]\n#[path = \"x.rs\"]\npub(crate) mod unit;\n#[cfg(not(test))]\nmod real;\nmod inline { }\n";
        let d = mod_decls(src);
        let names: Vec<(&str, bool)> = d.iter().map(|m| (m.name.as_str(), m.cfg_test)).collect();
        assert_eq!(names, vec![("a", false), ("tests", true), ("unit", true), ("real", false)]);
        assert_eq!(d[2].path_attr.as_deref(), Some("x.rs"));
    }

    #[test]
    fn classifies_a_crate_on_disk() {
        let dir = std::env::temp_dir().join(format!("anti-slop-modtree-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src/cli")).unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        std::fs::write(dir.join("src/lib.rs"), "pub mod core;\n#[cfg(test)]\nmod tests;\n").unwrap();
        std::fs::write(dir.join("src/main.rs"), "mod cli;\n").unwrap();
        std::fs::write(dir.join("src/cli.rs"), "mod args;\n").unwrap();
        for f in ["src/core.rs", "src/tests.rs", "src/cli/args.rs"] {
            std::fs::write(dir.join(f), "").unwrap();
        }
        let files: Vec<PathBuf> =
            ["src/lib.rs", "src/main.rs", "src/cli.rs", "src/core.rs", "src/tests.rs", "src/cli/args.rs"]
                .iter()
                .map(|f| dir.join(f))
                .collect();
        let layout = compute(&files);
        assert_eq!(layout.classify(&dir.join("src/core.rs")), FileKind::Library);
        assert_eq!(layout.classify(&dir.join("src/tests.rs")), FileKind::Test);
        assert_eq!(layout.classify(&dir.join("src/cli.rs")), FileKind::Binary);
        assert_eq!(layout.classify(&dir.join("src/cli/args.rs")), FileKind::Binary);
        assert_eq!(layout.classify(&dir.join("src/main.rs")), FileKind::Binary);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
