//! Where the comments are, line by line. `syn` drops comments, and searching for `//` finds them
//! inside string literals too, so this is a small lexer that knows strings, raw strings, character
//! literals (as opposed to lifetimes) and nested block comments.

/// What one source line holds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line {
    /// The text of every comment on the line, joined by a space.
    pub comment: String,
    /// Whether the line has any code (anything outside comments and whitespace).
    pub code: bool,
    /// Whether every comment on the line is a doc comment (`///`, `//!`, `/** */`, `/*! */`).
    pub doc: bool,
}

impl Line {
    pub fn has_comment(&self) -> bool {
        !self.comment.is_empty()
    }

    /// A line holding a comment and nothing else.
    pub fn only_comment(&self) -> bool {
        self.has_comment() && !self.code
    }
}

pub fn lines(source: &str) -> Vec<Line> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = vec![Line::default()];
    let mut doc_flags: Vec<Vec<bool>> = vec![Vec::new()];
    let mut i = 0;
    let push_comment = |out: &mut Vec<Line>, flags: &mut Vec<Vec<bool>>, text: &str, doc: bool| {
        let line = out.last_mut().expect("at least one line");
        if !line.comment.is_empty() {
            line.comment.push(' ');
        }
        line.comment.push_str(text);
        flags.last_mut().expect("at least one line").push(doc);
    };
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match c {
            '\n' => {
                out.push(Line::default());
                doc_flags.push(Vec::new());
                i += 1;
            }
            '/' if next == Some('/') => {
                let start = i;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                let doc = (text.starts_with("///") && !text.starts_with("////"))
                    || text.starts_with("//!");
                push_comment(&mut out, &mut doc_flags, &text, doc);
            }
            '/' if next == Some('*') => {
                let doc = matches!(chars.get(i + 2), Some('*') | Some('!'))
                    && chars.get(i + 3) != Some(&'/');
                let mut depth = 0;
                let mut segment = String::new();
                while i < chars.len() {
                    if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                        depth += 1;
                        segment.push_str("/*");
                        i += 2;
                    } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                        depth -= 1;
                        segment.push_str("*/");
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    } else if chars[i] == '\n' {
                        push_comment(&mut out, &mut doc_flags, &segment, doc);
                        segment.clear();
                        out.push(Line::default());
                        doc_flags.push(Vec::new());
                        i += 1;
                    } else {
                        segment.push(chars[i]);
                        i += 1;
                    }
                }
                if !segment.trim().is_empty() {
                    push_comment(&mut out, &mut doc_flags, &segment, doc);
                }
            }
            '"' => {
                out.last_mut().expect("line").code = true;
                i = skip_string(&chars, i + 1, &mut out, &mut doc_flags, 0);
            }
            'r' | 'b' | 'c' if is_raw_string_start(&chars, i) => {
                out.last_mut().expect("line").code = true;
                let mut j = i;
                while chars[j] != 'r' {
                    j += 1;
                }
                j += 1;
                let mut hashes = 0;
                while chars.get(j) == Some(&'#') {
                    hashes += 1;
                    j += 1;
                }
                i = skip_string(&chars, j + 1, &mut out, &mut doc_flags, hashes);
            }
            '\'' => {
                out.last_mut().expect("line").code = true;
                // A character literal is `'x'` or `'\..'`; anything else is a lifetime or label.
                if next == Some('\\') {
                    i += 2;
                    while i < chars.len() && chars[i] != '\'' && chars[i] != '\n' {
                        i += 1;
                    }
                    i += 1;
                } else if chars.get(i + 2) == Some(&'\'') {
                    i += 3;
                } else {
                    i += 1;
                }
            }
            c if c.is_whitespace() => i += 1,
            _ => {
                out.last_mut().expect("line").code = true;
                i += 1;
            }
        }
    }
    for (line, flags) in out.iter_mut().zip(doc_flags) {
        line.doc = !flags.is_empty() && flags.iter().all(|d| *d);
    }
    out
}

/// `r"`, `r#"`, `br"`, `cr#"`: the start of a raw string, not an identifier like `rand`.
fn is_raw_string_start(chars: &[char], i: usize) -> bool {
    if i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_') {
        return false;
    }
    let mut j = i;
    if matches!(chars[j], 'b' | 'c') {
        j += 1;
    }
    if chars.get(j) != Some(&'r') {
        return false;
    }
    j += 1;
    while chars.get(j) == Some(&'#') {
        j += 1;
    }
    chars.get(j) == Some(&'"')
}

/// Skips to just past the closing quote of a string whose body starts at `i`, closed by `"` and
/// `hashes` `#`s; escapes count only in ordinary strings. Newlines inside still start new lines.
fn skip_string(
    chars: &[char],
    mut i: usize,
    out: &mut Vec<Line>,
    flags: &mut Vec<Vec<bool>>,
    hashes: usize,
) -> usize {
    while i < chars.len() {
        match chars[i] {
            '\\' if hashes == 0 => i += 2,
            '\n' => {
                out.push(Line {
                    code: true,
                    ..Line::default()
                });
                flags.push(Vec::new());
                i += 1;
            }
            '"' if (1..=hashes).all(|k| chars.get(i + k) == Some(&'#')) => return i + 1 + hashes,
            _ => i += 1,
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_chars_lifetimes_and_comments() {
        let src = "let u = \"https://x.com\"; // real\n/// doc\nfn f<'a>(c: char) -> bool { c == '/' }\n\
                   let r = r#\"no // comment\"#;\n/* a\n b */ x();\n/** outer doc */\n";
        let l = lines(src);
        assert_eq!(l[0].comment, "// real");
        assert!(l[0].code && !l[0].doc);
        assert!(l[1].only_comment() && l[1].doc);
        assert!(!l[2].has_comment() && l[2].code);
        assert!(!l[3].has_comment());
        assert!(l[4].only_comment());
        assert!(l[5].has_comment() && l[5].code);
        assert!(l[6].doc);
    }
}
