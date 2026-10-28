//! Enough JSON to read a hook's input and write a hook's output, so the tool keeps its two
//! dependencies. Strings, numbers, booleans, null, arrays and objects; no streaming.

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Value>),
    Obj(Vec<(String, Value)>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

pub fn parse(text: &str) -> Result<Value, String> {
    let mut p = Parser {
        chars: text.chars().collect(),
        i: 0,
    };
    let v = p.value()?;
    p.ws();
    if p.i != p.chars.len() {
        return Err(format!("trailing input at {}", p.i));
    }
    Ok(v)
}

/// A JSON string literal for `s`, with quotes.
pub fn string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

struct Parser {
    chars: Vec<char>,
    i: usize,
}

impl Parser {
    fn ws(&mut self) {
        while self.i < self.chars.len() && self.chars[self.i].is_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.i).copied()
    }

    fn expect(&mut self, c: char) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected `{c}` at {}", self.i))
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        self.ws();
        match self.peek() {
            Some('{') => {
                self.i += 1;
                let mut pairs = Vec::new();
                self.ws();
                if self.peek() == Some('}') {
                    self.i += 1;
                    return Ok(Value::Obj(pairs));
                }
                loop {
                    self.ws();
                    let key = self.string()?;
                    self.ws();
                    self.expect(':')?;
                    let v = self.value()?;
                    pairs.push((key, v));
                    self.ws();
                    match self.peek() {
                        Some(',') => self.i += 1,
                        Some('}') => {
                            self.i += 1;
                            return Ok(Value::Obj(pairs));
                        }
                        _ => return Err(format!("expected `,` or `}}` at {}", self.i)),
                    }
                }
            }
            Some('[') => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.peek() == Some(']') {
                    self.i += 1;
                    return Ok(Value::Arr(items));
                }
                loop {
                    items.push(self.value()?);
                    self.ws();
                    match self.peek() {
                        Some(',') => self.i += 1,
                        Some(']') => {
                            self.i += 1;
                            return Ok(Value::Arr(items));
                        }
                        _ => return Err(format!("expected `,` or `]` at {}", self.i)),
                    }
                }
            }
            Some('"') => Ok(Value::Str(self.string()?)),
            Some('t') => self.literal("true", Value::Bool(true)),
            Some('f') => self.literal("false", Value::Bool(false)),
            Some('n') => self.literal("null", Value::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => {
                let start = self.i;
                while self
                    .peek()
                    .is_some_and(|c| c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E'))
                {
                    self.i += 1;
                }
                let s: String = self.chars[start..self.i].iter().collect();
                s.parse()
                    .map(Value::Num)
                    .map_err(|_| format!("bad number `{s}`"))
            }
            _ => Err(format!("unexpected input at {}", self.i)),
        }
    }

    fn literal(&mut self, word: &str, v: Value) -> Result<Value, String> {
        let end = self.i + word.len();
        let got: String = self.chars.get(self.i..end).unwrap_or(&[]).iter().collect();
        if got == word {
            self.i = end;
            Ok(v)
        } else {
            Err(format!("expected `{word}` at {}", self.i))
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect('"')?;
        let mut out = String::new();
        loop {
            let c = self.peek().ok_or("unterminated string")?;
            self.i += 1;
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let e = self.peek().ok_or("unterminated escape")?;
                    self.i += 1;
                    match e {
                        'n' => out.push('\n'),
                        't' => out.push('\t'),
                        'r' => out.push('\r'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'u' => {
                            let hex: String = self
                                .chars
                                .get(self.i..self.i + 4)
                                .unwrap_or(&[])
                                .iter()
                                .collect();
                            self.i += 4;
                            let code =
                                u32::from_str_radix(&hex, 16).map_err(|_| "bad \\u escape")?;
                            // Surrogate pairs come as two escapes; join them.
                            let ch = if (0xD800..0xDC00).contains(&code) {
                                let low: String = self
                                    .chars
                                    .get(self.i + 2..self.i + 6)
                                    .unwrap_or(&[])
                                    .iter()
                                    .collect();
                                self.i += 6;
                                let low =
                                    u32::from_str_radix(&low, 16).map_err(|_| "bad surrogate")?;
                                0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00)
                            } else {
                                code
                            };
                            out.push(char::from_u32(ch).unwrap_or('\u{FFFD}'));
                        }
                        other => out.push(other),
                    }
                }
                c => out.push(c),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let v = parse(r#"{"tool_name": "Bash", "tool_input": {"command": "git commit -m \"x\"", "n": [1, 2.5, -3]}, "ok": true, "none": null}"#)
            .expect("parses");
        assert_eq!(v.get("tool_name").and_then(Value::as_str), Some("Bash"));
        assert_eq!(
            v.get("tool_input")
                .and_then(|t| t.get("command"))
                .and_then(Value::as_str),
            Some("git commit -m \"x\"")
        );
        assert_eq!(v.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(string("a\"b\n"), "\"a\\\"b\\n\"");
        assert!(parse("{\"a\": }").is_err());
        assert_eq!(parse(r#""é😀""#), Ok(Value::Str("é😀".into())));
    }
}
