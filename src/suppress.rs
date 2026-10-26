//! `// anti-slop: allow(rule-id) because ...` at the end of a finding's line, or alone on the line
//! above it, silences that rule there. `// anti-slop: allow-file(rule-id)` silences it for the whole file.

use crate::comments::Line;

const MARKER: &str = "anti-slop:";

#[derive(Debug, Default)]
pub struct Suppressions {
    /// (1-based line, rule id, whether it also covers the next line).
    lines: Vec<(usize, String, bool)>,
    file: Vec<String>,
}

impl Suppressions {
    pub fn parse(lines: &[Line]) -> Suppressions {
        let mut s = Suppressions::default();
        for (i, line) in lines.iter().enumerate() {
            let comment = line.comment.as_str();
            let Some(at) = comment.find(MARKER) else {
                continue;
            };
            let directive = comment[at + MARKER.len()..].trim_start();
            for (keyword, file_wide) in [("allow-file(", true), ("allow(", false)] {
                let Some(rest) = directive.strip_prefix(keyword) else {
                    continue;
                };
                let Some(close) = rest.find(')') else {
                    continue;
                };
                for rule in rest[..close]
                    .split(',')
                    .map(str::trim)
                    .filter(|r| !r.is_empty())
                {
                    if file_wide {
                        s.file.push(rule.to_string());
                    } else {
                        // A comment on a line of its own covers the next line; one trailing code
                        // covers only that code.
                        s.lines.push((i + 1, rule.to_string(), line.only_comment()));
                    }
                }
                break;
            }
        }
        s
    }

    pub fn allows(&self, rule: &str, line: usize) -> bool {
        self.file.iter().any(|r| r == rule)
            || self
                .lines
                .iter()
                .any(|(l, r, next)| r == rule && (*l == line || *next && *l + 1 == line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Suppressions {
        Suppressions::parse(&crate::comments::lines(src))
    }

    #[test]
    fn own_line_covers_the_next_trailing_covers_only_its_own() {
        let s = parse(
            "let a = x.unwrap(); // anti-slop: allow(unwrap) checked\nlet b = y.unwrap();\n// anti-slop: allow(a, b)\nf();\n",
        );
        assert!(s.allows("unwrap", 1));
        assert!(!s.allows("unwrap", 2));
        assert!(s.allows("b", 4));
        assert!(!s.allows("c", 4));
    }

    #[test]
    fn a_marker_in_a_string_is_not_a_comment() {
        let s = parse("let help = \"// anti-slop: allow(unwrap)\";\nx.unwrap();\n");
        assert!(!s.allows("unwrap", 1) && !s.allows("unwrap", 2));
    }

    #[test]
    fn file_wide() {
        let s = parse("//! anti-slop: allow-file(dbg-macro)");
        assert!(s.allows("dbg-macro", 900));
    }
}
