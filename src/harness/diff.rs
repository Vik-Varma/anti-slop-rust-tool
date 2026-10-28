//! What changed, from `git diff`: which lines of which files, and the added and removed text, so
//! the harness can judge only what an agent (or a person) touched. Needs `git` on the path; the
//! analysis itself never does.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// What to compare the working tree (or the index) against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Base {
    /// `git diff <rev>`: everything since that commit, committed or not.
    Rev(String),
    /// `git diff --cached`: what is staged for the next commit.
    Staged,
}

/// One changed file, as it is now (after renames).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changed {
    /// Path relative to the repository root, on the new side.
    pub path: PathBuf,
    /// The old path when the file was renamed or copied.
    pub old_path: Option<PathBuf>,
    pub deleted: bool,
    pub binary: bool,
    /// New-side line ranges that were added or edited, inclusive.
    pub ranges: Vec<(usize, usize)>,
    /// Added lines, with their new-side line numbers.
    pub added: Vec<(usize, String)>,
    /// Removed lines (old-side text only).
    pub removed: Vec<String>,
}

impl Changed {
    pub fn touches(&self, line: usize) -> bool {
        self.ranges.iter().any(|(s, e)| (*s..=*e).contains(&line))
    }

    pub fn is_rust(&self) -> bool {
        self.path.extension().is_some_and(|e| e == "rs")
    }
}

fn git(root: &Path, args: &[&str]) -> io::Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The repository that holds `dir`.
pub fn repo_root(dir: &Path) -> io::Result<PathBuf> {
    let out = git(dir, &["rev-parse", "--show-toplevel"])?;
    Ok(PathBuf::from(out.trim()))
}

/// Every file changed against `base`, with its changed lines.
pub fn changes(root: &Path, base: &Base) -> io::Result<Vec<Changed>> {
    let mut args = vec!["diff", "--no-color", "--no-ext-diff", "-U0", "-M"];
    match base {
        Base::Rev(rev) => args.push(rev),
        Base::Staged => args.push("--cached"),
    }
    args.push("--");
    Ok(parse(&git(root, &args)?))
}

/// A file's content at `base` (before the change), if it existed there.
pub fn old_content(root: &Path, base: &Base, path: &Path) -> Option<String> {
    let spec = match base {
        Base::Rev(rev) => format!("{rev}:{}", path.display()),
        Base::Staged => format!("HEAD:{}", path.display()),
    };
    git(root, &["show", &spec]).ok()
}

/// Parses `git diff -U0` output. Each `diff --git` header starts a file; `+++ b/<path>` names
/// its new side; `@@ -a,b +c,d @@` gives the new-side start and count.
pub fn parse(text: &str) -> Vec<Changed> {
    let mut out: Vec<Changed> = Vec::new();
    let mut new_line = 0usize;
    for raw in text.lines() {
        if let Some(rest) = raw.strip_prefix("diff --git ") {
            // `a/old b/new`; paths with spaces are quoted, which is rare enough to read naively.
            let new = rest.rsplit(" b/").next().unwrap_or_default();
            out.push(Changed {
                path: PathBuf::from(new),
                ..Changed::default()
            });
            continue;
        }
        let Some(cur) = out.last_mut() else { continue };
        if let Some(p) = raw.strip_prefix("rename from ") {
            cur.old_path = Some(PathBuf::from(p));
        } else if let Some(p) = raw.strip_prefix("copy from ") {
            cur.old_path = Some(PathBuf::from(p));
        } else if raw.starts_with("deleted file mode") {
            cur.deleted = true;
        } else if raw.starts_with("Binary files") || raw.starts_with("GIT binary patch") {
            cur.binary = true;
        } else if let Some(p) = raw.strip_prefix("+++ ") {
            if let Some(p) = p.strip_prefix("b/") {
                cur.path = PathBuf::from(p.trim_end());
            }
        } else if let Some(h) = raw.strip_prefix("@@ ") {
            // `-a,b +c,d @@ ...`
            let plus = h.split_whitespace().nth(1).unwrap_or_default();
            let plus = plus.trim_start_matches('+');
            let (start, count) = match plus.split_once(',') {
                Some((s, c)) => (s.parse().unwrap_or(0), c.parse().unwrap_or(1)),
                None => (plus.parse().unwrap_or(0), 1),
            };
            new_line = start;
            if count > 0 {
                cur.ranges.push((start, start + count - 1));
            }
        } else if let Some(l) = raw.strip_prefix('+') {
            cur.added.push((new_line, l.to_string()));
            new_line += 1;
        } else if let Some(l) = raw.strip_prefix('-') {
            if !raw.starts_with("--- ") {
                cur.removed.push(l.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIFF: &str = "\
diff --git a/src/a.rs b/src/a.rs
index 1..2 100644
--- a/src/a.rs
+++ b/src/a.rs
@@ -3,0 +4,2 @@ fn f() {
+    let x = 1;
+    let y = 2;
@@ -10 +12 @@
-old
+new
diff --git a/old.rs b/new.rs
similarity index 90%
rename from old.rs
rename to new.rs
--- a/old.rs
+++ b/new.rs
@@ -1 +1 @@
-a
+b
diff --git a/gone.rs b/gone.rs
deleted file mode 100644
--- a/gone.rs
+++ /dev/null
@@ -1,2 +0,0 @@
-x
-y
";

    #[test]
    fn hunks_renames_and_deletions() {
        let c = parse(DIFF);
        assert_eq!(c.len(), 3);
        assert_eq!(c[0].path, PathBuf::from("src/a.rs"));
        assert_eq!(c[0].ranges, vec![(4, 5), (12, 12)]);
        assert!(c[0].touches(5) && !c[0].touches(6) && c[0].touches(12));
        assert_eq!(c[0].added[0], (4, "    let x = 1;".to_string()));
        assert_eq!(c[0].removed, vec!["old".to_string()]);
        assert_eq!(c[1].path, PathBuf::from("new.rs"));
        assert_eq!(c[1].old_path, Some(PathBuf::from("old.rs")));
        assert!(c[2].deleted && c[2].ranges.is_empty());
        assert_eq!(c[2].removed.len(), 2);
    }
}
