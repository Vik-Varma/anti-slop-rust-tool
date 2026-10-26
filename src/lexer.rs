//! A small lexer that recovers the information `syn` throws away:
//! comments, and which lines contain code.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// 1-based line on which the comment starts.
    pub line: usize,
    /// 1-based line on which the comment ends.
    pub end_line: usize,
    /// 1-based column of the comment opener.
    pub column: usize,
    /// Comment text without the `//`, `///`, `//!`, `/*`, `*/` delimiters.
    pub text: String,
    pub is_doc: bool,
    pub is_block: bool,
    /// True when nothing but whitespace precedes the comment on its line.
    pub own_line: bool,
}

#[derive(Debug, Default)]
pub struct Lexed {
    pub comments: Vec<Comment>,
    /// `has_code[i]` is true when line `i + 1` contains a token that is not
    /// whitespace or part of a comment.
    pub has_code: Vec<bool>,
}

impl Lexed {
    pub fn line_has_code(&self, line: usize) -> bool {
        line >= 1 && self.has_code.get(line - 1).copied().unwrap_or(false)
    }
}

pub fn lex(src: &str) -> Lexed {
    let chars: Vec<char> = src.chars().collect();
    let line_count = src.lines().count().max(1);
    let mut out = Lexed { comments: Vec::new(), has_code: vec![false; line_count + 1] };

    let mut i = 0;
    let mut line = 1;
    let mut col = 1;
    // Whether a code token has already been seen on the current line.
    let mut code_on_line = false;

    macro_rules! mark_code {
        () => {{
            if let Some(slot) = out.has_code.get_mut(line - 1) {
                *slot = true;
            }
            code_on_line = true;
        }};
    }

    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();

        if c == '\n' {
            line += 1;
            col = 1;
            code_on_line = false;
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            col += 1;
            continue;
        }

        // Line comment.
        if c == '/' && next == Some('/') {
            let start_col = col;
            let own_line = !code_on_line;
            let mut j = i + 2;
            let (is_doc, skip) = match (chars.get(j), chars.get(j + 1)) {
                // `////` is a regular comment, `///` is doc, `//!` is inner doc.
                (Some('/'), Some('/')) => (false, 0),
                (Some('/'), _) | (Some('!'), _) => (true, 1),
                _ => (false, 0),
            };
            j += skip;
            let text_start = j;
            while j < chars.len() && chars[j] != '\n' {
                j += 1;
            }
            let text: String = chars[text_start..j].iter().collect();
            out.comments.push(Comment {
                line,
                end_line: line,
                column: start_col,
                text,
                is_doc,
                is_block: false,
                own_line,
            });
            col += j - i;
            i = j;
            continue;
        }

        // Block comment (Rust block comments nest).
        if c == '/' && next == Some('*') {
            let start_line = line;
            let start_col = col;
            let own_line = !code_on_line;
            let is_doc = matches!(chars.get(i + 2), Some('*') | Some('!'))
                && chars.get(i + 3) != Some(&'/')
                && !(chars.get(i + 2) == Some(&'*') && chars.get(i + 3) == Some(&'*'));
            let mut depth = 1;
            let mut j = i + 2;
            let text_start = j;
            let mut text_end = chars.len();
            while j < chars.len() {
                if chars[j] == '/' && chars.get(j + 1) == Some(&'*') {
                    depth += 1;
                    j += 2;
                    col += 2;
                    continue;
                }
                if chars[j] == '*' && chars.get(j + 1) == Some(&'/') {
                    depth -= 1;
                    if depth == 0 {
                        text_end = j;
                        j += 2;
                        col += 2;
                        break;
                    }
                    j += 2;
                    col += 2;
                    continue;
                }
                if chars[j] == '\n' {
                    line += 1;
                    col = 1;
                    code_on_line = false;
                } else {
                    col += 1;
                }
                j += 1;
            }
            let text: String = chars[text_start..text_end.min(chars.len())].iter().collect();
            let text = if is_doc { text[1..].to_string() } else { text };
            out.comments.push(Comment {
                line: start_line,
                end_line: line,
                column: start_col,
                text,
                is_doc,
                is_block: true,
                own_line,
            });
            i = j;
            continue;
        }

        // Raw strings: r"..", r#".."#, br"..", cr"..".
        if let Some((hashes, prefix_len)) = raw_string_start(&chars, i) {
            mark_code!();
            let mut j = i + prefix_len + hashes + 1;
            col += prefix_len + hashes + 1;
            loop {
                if j >= chars.len() {
                    break;
                }
                if chars[j] == '"' && (0..hashes).all(|k| chars.get(j + 1 + k) == Some(&'#')) {
                    j += 1 + hashes;
                    col += 1 + hashes;
                    break;
                }
                if chars[j] == '\n' {
                    line += 1;
                    col = 1;
                    mark_code!();
                } else {
                    col += 1;
                }
                j += 1;
            }
            i = j;
            continue;
        }

        // Normal and byte/C strings.
        if c == '"' || ((c == 'b' || c == 'c') && next == Some('"') && !prev_is_ident(&chars, i)) {
            mark_code!();
            let mut j = if c == '"' { i + 1 } else { i + 2 };
            col += j - i;
            while j < chars.len() {
                match chars[j] {
                    '\\' => {
                        // An escaped newline still advances the line counter.
                        if chars.get(j + 1) == Some(&'\n') {
                            line += 1;
                            col = 1;
                            mark_code!();
                        } else {
                            col += 2;
                        }
                        j += 2;
                    }
                    '"' => {
                        j += 1;
                        col += 1;
                        break;
                    }
                    '\n' => {
                        line += 1;
                        col = 1;
                        mark_code!();
                        j += 1;
                    }
                    _ => {
                        j += 1;
                        col += 1;
                    }
                }
            }
            i = j;
            continue;
        }

        // Char literal vs lifetime.
        if c == '\'' {
            mark_code!();
            if next == Some('\\') {
                // Escaped char literal: skip to closing quote.
                let mut j = i + 2;
                while j < chars.len() && chars[j] != '\'' && chars[j] != '\n' {
                    j += 1;
                }
                col += j + 1 - i;
                i = j + 1;
                continue;
            }
            if chars.get(i + 2) == Some(&'\'') {
                col += 3;
                i += 3;
                continue;
            }
            // Lifetime or label.
            i += 1;
            col += 1;
            continue;
        }

        mark_code!();
        i += 1;
        col += 1;
    }

    out.has_code.truncate(line_count);
    out
}

fn prev_is_ident(chars: &[char], i: usize) -> bool {
    i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_')
}

/// If a raw string literal starts at `i`, returns (number of `#`, prefix length).
fn raw_string_start(chars: &[char], i: usize) -> Option<(usize, usize)> {
    if prev_is_ident(chars, i) {
        return None;
    }
    let prefix_len = match (chars.get(i), chars.get(i + 1)) {
        (Some('r'), _) => 1,
        (Some('b'), Some('r')) | (Some('c'), Some('r')) => 2,
        _ => return None,
    };
    let mut j = i + prefix_len;
    let mut hashes = 0;
    while chars.get(j) == Some(&'#') {
        hashes += 1;
        j += 1;
    }
    if chars.get(j) == Some(&'"') { Some((hashes, prefix_len)) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_line_and_block_comments() {
        let src = "fn a() {} // trailing\n// own\n/* block\n spans */ let x = 1;\n";
        let l = lex(src);
        assert_eq!(l.comments.len(), 3);
        assert_eq!(l.comments[0].text, " trailing");
        assert!(!l.comments[0].own_line);
        assert!(l.comments[1].own_line);
        assert_eq!(l.comments[2].line, 3);
        assert_eq!(l.comments[2].end_line, 4);
        assert_eq!(l.has_code, vec![true, false, false, true]);
    }

    #[test]
    fn ignores_comment_markers_in_strings() {
        let src = "let s = \"// not a comment\";\nlet r = r#\"/* nope */\"#;\nlet c = '/';\n";
        let l = lex(src);
        assert!(l.comments.is_empty(), "{:?}", l.comments);
    }

    #[test]
    fn lifetimes_are_not_char_literals() {
        let src = "fn f<'a>(x: &'a str) -> &'a str { x } // real\n";
        let l = lex(src);
        assert_eq!(l.comments.len(), 1);
        assert_eq!(l.comments[0].text, " real");
    }

    #[test]
    fn doc_comments_are_flagged() {
        let l = lex("/// doc\n//! inner\n//// plain\n/** block doc */\n");
        let docs: Vec<bool> = l.comments.iter().map(|c| c.is_doc).collect();
        assert_eq!(docs, vec![true, true, false, true]);
    }

    #[test]
    fn nested_block_comments() {
        let l = lex("/* a /* b */ c */ x\n");
        assert_eq!(l.comments.len(), 1);
        assert!(l.has_code[0]);
    }

    #[test]
    fn multiline_string_lines_count_as_code() {
        let l = lex("let s = \"a\nb\";\n\n");
        assert_eq!(l.has_code, vec![true, true, false]);
    }
}
