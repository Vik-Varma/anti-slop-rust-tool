//! `// anti-slop: allow(rule-id) because ...` on the line of a finding, or on the line above it,
//! silences that rule there. `// anti-slop: allow-file(rule-id)` silences it for the whole file.

const MARKER: &str = "anti-slop:";

#[derive(Debug, Default)]
pub struct Suppressions {
    /// (1-based line, rule id) pairs.
    lines: Vec<(usize, String)>,
    file: Vec<String>,
}

impl Suppressions {
    pub fn parse(lines: &[&str]) -> Suppressions {
        let mut s = Suppressions::default();
        for (i, line) in lines.iter().enumerate() {
            let Some(comment) = line.find("//").map(|at| &line[at + 2..]) else {
                continue;
            };
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
                        s.lines.push((i + 1, rule.to_string()));
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
                .any(|(l, r)| r == rule && (*l == line || *l + 1 == line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_line_and_line_above() {
        let lines = [
            "let a = x.unwrap(); // anti-slop: allow(unwrap) checked above",
            "",
            "// anti-slop: allow(a, b)",
            "f();",
        ];
        let s = Suppressions::parse(&lines);
        assert!(s.allows("unwrap", 1));
        assert!(s.allows("unwrap", 2));
        assert!(!s.allows("unwrap", 3));
        assert!(s.allows("b", 4));
        assert!(!s.allows("c", 4));
    }

    #[test]
    fn file_wide() {
        let s = Suppressions::parse(&["//! anti-slop: allow-file(dbg-macro)"]);
        assert!(s.allows("dbg-macro", 900));
    }
}
