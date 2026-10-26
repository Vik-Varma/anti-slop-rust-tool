use crate::lexer::{self, Comment, Lexed};
use proc_macro2::Span;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use syn::visit::Visit;

/// The role a file plays in a crate. Several rules only make sense for
/// production code and stay quiet in tests, benches, examples and build scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Library,
    Binary,
    Test,
    Bench,
    Example,
    BuildScript,
}

impl FileKind {
    pub fn from_path(path: &Path) -> FileKind {
        let comps: Vec<String> = path.components().map(|c| c.as_os_str().to_string_lossy().into_owned()).collect();
        let file_name = comps.last().map(String::as_str).unwrap_or("");
        let dirs = &comps[..comps.len().saturating_sub(1)];
        if file_name == "build.rs" {
            return FileKind::BuildScript;
        }
        // Test fixtures and fuzz targets are not production code either.
        if dirs.iter().any(|d| matches!(d.as_str(), "tests" | "test_data" | "testdata" | "fixtures" | "fuzz")) {
            return FileKind::Test;
        }
        if dirs.iter().any(|d| d == "benches") {
            return FileKind::Bench;
        }
        if dirs.iter().any(|d| d == "examples") {
            return FileKind::Example;
        }
        if file_name == "main.rs" || dirs.iter().any(|d| d == "bin") {
            return FileKind::Binary;
        }
        FileKind::Library
    }

    /// Code that ships to users (as opposed to tests, benches and examples).
    pub fn is_production(self) -> bool {
        matches!(self, FileKind::Library | FileKind::Binary)
    }
}

/// Everything a rule needs to know about one source file.
pub struct FileContext<'a> {
    pub path: &'a Path,
    pub display_path: PathBuf,
    pub source: &'a str,
    pub lines: Vec<&'a str>,
    pub ast: syn::File,
    pub lexed: Lexed,
    pub kind: FileKind,
    /// Inclusive line ranges covered by test-only code (`#[test]`, `#[cfg(test)]`, ...).
    test_ranges: Vec<(usize, usize)>,
    /// line -> rule ids suppressed on that line (`"*"` suppresses everything).
    suppressed_lines: HashMap<usize, HashSet<String>>,
    suppressed_file: HashSet<String>,
}

impl<'a> FileContext<'a> {
    pub fn new(path: &'a Path, display_path: PathBuf, source: &'a str) -> syn::Result<Self> {
        let kind = FileKind::from_path(&display_path);
        Self::with_kind(path, display_path, kind, source)
    }

    /// Like [`FileContext::new`], with an explicit file classification.
    pub fn with_kind(path: &'a Path, display_path: PathBuf, kind: FileKind, source: &'a str) -> syn::Result<Self> {
        let ast = syn::parse_file(source)?;
        let lexed = lexer::lex(source);
        let mut tests = TestRangeVisitor::default();
        tests.visit_file(&ast);
        let mut ctx = FileContext {
            path,
            display_path,
            source,
            lines: source.lines().collect(),
            ast,
            lexed,
            kind,
            test_ranges: tests.ranges,
            suppressed_lines: HashMap::new(),
            suppressed_file: HashSet::new(),
        };
        ctx.collect_suppressions();
        Ok(ctx)
    }

    pub fn comments(&self) -> &[Comment] {
        &self.lexed.comments
    }

    /// True if `line` lies in test-only code, or the whole file is a test/bench/example.
    pub fn in_test(&self, line: usize) -> bool {
        matches!(self.kind, FileKind::Test | FileKind::Bench)
            || self.test_ranges.iter().any(|&(s, e)| s <= line && line <= e)
    }

    /// True if `line` is production code: not a test, bench, example or build script.
    pub fn is_production_line(&self, line: usize) -> bool {
        self.kind.is_production() && !self.in_test(line)
    }

    pub fn is_suppressed(&self, rule: &str, line: usize) -> bool {
        if self.suppressed_file.contains(rule) || self.suppressed_file.contains("*") {
            return true;
        }
        self.suppressed_lines.get(&line).is_some_and(|ids| ids.contains(rule) || ids.contains("*"))
    }

    pub fn line_text(&self, line: usize) -> &str {
        line.checked_sub(1).and_then(|i| self.lines.get(i)).copied().unwrap_or("")
    }

    /// Comments that end on the lines directly above `line`, skipping
    /// attributes and blank lines, nearest first.
    pub fn comments_above(&self, line: usize) -> Vec<&Comment> {
        let mut out = Vec::new();
        let mut cur = line;
        while cur > 1 {
            cur -= 1;
            let text = self.line_text(cur).trim();
            if let Some(c) = self.comments().iter().find(|c| c.end_line == cur && c.own_line) {
                out.push(c);
                cur = c.line;
                continue;
            }
            if text.is_empty() || text.starts_with("#[") || text.starts_with("#![") {
                continue;
            }
            break;
        }
        out
    }

    fn collect_suppressions(&mut self) {
        for c in &self.lexed.comments {
            let Some(rest) = find_directive(&c.text) else { continue };
            if let Some(args) = rest.strip_prefix("allow-file") {
                self.suppressed_file.extend(parse_ids(args));
            } else if let Some(args) = rest.strip_prefix("allow") {
                let ids = parse_ids(args);
                let mut targets = vec![c.end_line];
                if c.own_line {
                    // An own-line directive also covers the next line holding code.
                    let mut next = c.end_line + 1;
                    while next <= self.lines.len() && !self.lexed.line_has_code(next) {
                        next += 1;
                    }
                    targets.push(next);
                }
                for t in targets {
                    self.suppressed_lines.entry(t).or_default().extend(ids.iter().cloned());
                }
            }
        }
    }
}

fn find_directive(text: &str) -> Option<&str> {
    let idx = text.find("anti-slop:")?;
    Some(text[idx + "anti-slop:".len()..].trim_start())
}

fn parse_ids(args: &str) -> HashSet<String> {
    let args = args.trim_start();
    let Some(inner) = args.strip_prefix('(').and_then(|a| a.split(')').next()) else {
        return ["*".to_string()].into_iter().collect();
    };
    inner.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

pub fn line_of(span: Span) -> usize {
    span.start().line
}

pub fn column_of(span: Span) -> usize {
    span.start().column + 1
}

/// Is this attribute one that marks test-only code?
pub fn is_test_attr(attr: &syn::Attribute) -> bool {
    let path = attr.path();
    let last = path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
    if last == "test" || last == "bench" || last == "rstest" || last == "quickcheck" || last == "proptest" {
        return true;
    }
    if path.is_ident("cfg") {
        // `cfg(test)`, `cfg(all(test, ..))` or `cfg(any(test, ..))`, but not `cfg(not(test))`
        // and not `cfg(feature = "test-utils")`.
        return attr.meta.require_list().is_ok_and(|l| cfg_mentions_test(l.tokens.clone(), false));
    }
    false
}

/// Does a cfg predicate mention the bare `test` flag outside a `not(..)`?
fn cfg_mentions_test(tokens: proc_macro2::TokenStream, negated: bool) -> bool {
    use proc_macro2::TokenTree;
    let mut prev_ident: Option<String> = None;
    for tt in tokens {
        match tt {
            TokenTree::Ident(i) => {
                if i == "test" && !negated {
                    return true;
                }
                prev_ident = Some(i.to_string());
            }
            TokenTree::Group(g) => {
                let inner_negated = negated || prev_ident.as_deref() == Some("not");
                if cfg_mentions_test(g.stream(), inner_negated) {
                    return true;
                }
                prev_ident = None;
            }
            _ => prev_ident = None,
        }
    }
    false
}

pub fn has_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(is_test_attr)
}

#[derive(Default)]
struct TestRangeVisitor {
    ranges: Vec<(usize, usize)>,
}

impl TestRangeVisitor {
    fn record(&mut self, attrs: &[syn::Attribute], span: Span) -> bool {
        if has_test_attr(attrs) {
            let start = attrs.first().map(|a| a.pound_token.span.start().line).unwrap_or(span.start().line);
            self.ranges.push((start, span.end().line));
            true
        } else {
            false
        }
    }
}

impl<'ast> Visit<'ast> for TestRangeVisitor {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        use syn::spanned::Spanned;
        let attrs: &[syn::Attribute] = match item {
            syn::Item::Fn(i) => &i.attrs,
            syn::Item::Mod(i) => &i.attrs,
            syn::Item::Impl(i) => &i.attrs,
            syn::Item::Struct(i) => &i.attrs,
            syn::Item::Enum(i) => &i.attrs,
            syn::Item::Use(i) => &i.attrs,
            syn::Item::Const(i) => &i.attrs,
            syn::Item::Static(i) => &i.attrs,
            syn::Item::Trait(i) => &i.attrs,
            syn::Item::Macro(i) => &i.attrs,
            _ => &[],
        };
        if !self.record(attrs, item.span()) {
            syn::visit::visit_item(self, item);
        }
    }

    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        use syn::spanned::Spanned;
        if !self.record(&f.attrs, f.span()) {
            syn::visit::visit_impl_item_fn(self, f);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(src: &str) -> FileContext<'_> {
        FileContext::new(Path::new("src/lib.rs"), PathBuf::from("src/lib.rs"), src).unwrap()
    }

    #[test]
    fn file_kinds() {
        assert_eq!(FileKind::from_path(Path::new("src/lib.rs")), FileKind::Library);
        assert_eq!(FileKind::from_path(Path::new("src/main.rs")), FileKind::Binary);
        assert_eq!(FileKind::from_path(Path::new("src/bin/tool.rs")), FileKind::Binary);
        assert_eq!(FileKind::from_path(Path::new("tests/it.rs")), FileKind::Test);
        assert_eq!(FileKind::from_path(Path::new("benches/b.rs")), FileKind::Bench);
        assert_eq!(FileKind::from_path(Path::new("examples/e.rs")), FileKind::Example);
        assert_eq!(FileKind::from_path(Path::new("build.rs")), FileKind::BuildScript);
    }

    #[test]
    fn test_ranges() {
        let src = "fn prod() {}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n#[cfg(not(test))]\nfn other() {}\n#[cfg(feature = \"test-utils\")]\nfn helper() {}\n#[cfg(all(test, feature = \"notify\"))]\nfn t2() {}\n";
        let c = ctx(src);
        assert!(!c.in_test(1));
        assert!(c.in_test(2));
        assert!(c.in_test(5));
        assert!(!c.in_test(8));
        assert!(!c.in_test(10), "feature named test-utils is not test code");
        assert!(c.in_test(12));
    }

    #[test]
    fn suppressions() {
        let src = "// anti-slop: allow(rule-a)\nfn a() {}\nfn b() {} // anti-slop: allow(rule-b, rule-c)\nfn c() {}\n";
        let c = ctx(src);
        assert!(c.is_suppressed("rule-a", 2));
        assert!(!c.is_suppressed("rule-a", 3));
        assert!(c.is_suppressed("rule-c", 3));
        assert!(!c.is_suppressed("rule-b", 4));
        let f = ctx("// anti-slop: allow-file(x)\nfn a() {}\n");
        assert!(f.is_suppressed("x", 2));
        assert!(!f.is_suppressed("y", 2));
    }

    #[test]
    fn comments_above_skips_attributes() {
        let src = "// SAFETY: fine\n#[inline]\nfn a() {}\n";
        let c = ctx(src);
        let above = c.comments_above(3);
        assert_eq!(above.len(), 1);
        assert!(above[0].text.contains("SAFETY"));
    }
}
