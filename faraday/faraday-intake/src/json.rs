//! A JSON value reader for the wallet files other wallets write
//! (`docs/INTEROP.md` §5 I2): RFC 8259's grammar, every string escape
//! including `\uXXXX` and its surrogate pairs, at most [`MAX_DEPTH`]
//! arrays and objects deep and [`MAX_SIZE`] bytes long.
//!
//! An object that names one key twice is refused: two readers could take
//! different values from it. Numbers are kept as written; a reader asks
//! for the whole number it needs ([`Value::as_u64`]).
//!
//! The quotation mark is written `'\u{22}'` and `b'\x22'` here, so that
//! `tools/lint-strings.sh` does not read it as the start of a string.

/// The deepest nesting of arrays and objects read.
pub const MAX_DEPTH: usize = 32;

/// The longest text read, bytes.
pub const MAX_SIZE: usize = 1 << 20;

/// A JSON value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number, as written.
    Number(String),
    /// A string, its escapes undone.
    String(String),
    /// An array.
    Array(Vec<Value>),
    /// An object, its members in the order written.
    Object(Vec<(String, Value)>),
}

/// Why a text is not read as JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Longer than [`MAX_SIZE`].
    TooLarge,
    /// Nested deeper than [`MAX_DEPTH`].
    TooDeep,
    /// An object names a key twice.
    Duplicate,
    /// Not JSON, at this byte.
    Syntax(usize),
}

impl Value {
    /// The member `key` of an object; `None` for a value that is no
    /// object or has no such member.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The string this value is.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// The whole number this value is, written without a sign, fraction
    /// or exponent.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Number(n) if n.bytes().all(|b| b.is_ascii_digit()) => n.parse().ok(),
            _ => None,
        }
    }

    /// The items of an array.
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }
}

/// Reads a text that is one JSON value, with white space (and a UTF-8
/// byte order mark) around it. Costs a pass over the text.
pub fn parse(text: &str) -> Result<Value, Error> {
    if text.len() > MAX_SIZE {
        return Err(Error::TooLarge);
    }
    let body = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut p = Parser {
        bytes: body.as_bytes(),
        at: 0,
    };
    p.space();
    let v = p.value(0)?;
    p.space();
    if p.at != p.bytes.len() {
        return Err(Error::Syntax(p.at));
    }
    Ok(v)
}

/// A string written as a JSON string, quotes and escapes included.
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\u{22}');
    for c in s.chars() {
        match c {
            '\u{22}' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('\u{22}');
    out
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn fail<T>(&self) -> Result<T, Error> {
        Err(Error::Syntax(self.at))
    }

    fn space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn word(&mut self, w: &[u8], v: Value) -> Result<Value, Error> {
        if self.bytes[self.at..].starts_with(w) {
            self.at += w.len();
            Ok(v)
        } else {
            self.fail()
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, Error> {
        match self.peek() {
            Some(b'{') => self.object(depth + 1),
            Some(b'[') => self.array(depth + 1),
            Some(b'\x22') => self.string().map(Value::String),
            Some(b't') => self.word(b"true", Value::Bool(true)),
            Some(b'f') => self.word(b"false", Value::Bool(false)),
            Some(b'n') => self.word(b"null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => self.fail(),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::TooDeep);
        }
        self.at += 1;
        let mut members: Vec<(String, Value)> = Vec::new();
        self.space();
        if self.peek() == Some(b'}') {
            self.at += 1;
            return Ok(Value::Object(members));
        }
        loop {
            self.space();
            if self.peek() != Some(b'\x22') {
                return self.fail();
            }
            let key = self.string()?;
            self.space();
            if self.peek() != Some(b':') {
                return self.fail();
            }
            self.at += 1;
            self.space();
            let v = self.value(depth)?;
            if members.iter().any(|(k, _)| *k == key) {
                return Err(Error::Duplicate);
            }
            members.push((key, v));
            self.space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Value::Object(members));
                }
                _ => return self.fail(),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, Error> {
        if depth > MAX_DEPTH {
            return Err(Error::TooDeep);
        }
        self.at += 1;
        let mut items = Vec::new();
        self.space();
        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.space();
            items.push(self.value(depth)?);
            self.space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(Value::Array(items));
                }
                _ => return self.fail(),
            }
        }
    }

    fn digits(&mut self) -> usize {
        let from = self.at;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.at += 1;
        }
        self.at - from
    }

    fn number(&mut self) -> Result<Value, Error> {
        let from = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        match self.peek() {
            Some(b'0') => self.at += 1,
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return self.fail(),
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            if self.digits() == 0 {
                return self.fail();
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if self.digits() == 0 {
                return self.fail();
            }
        }
        // The bytes read are ASCII.
        let text = String::from_utf8_lossy(&self.bytes[from..self.at]).into_owned();
        Ok(Value::Number(text))
    }

    fn hex4(&mut self) -> Result<u32, Error> {
        let Some(h) = self.bytes.get(self.at..self.at + 4) else {
            return self.fail();
        };
        let mut v = 0u32;
        for &b in h {
            let d = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return self.fail(),
            };
            v = v * 16 + u32::from(d);
        }
        self.at += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, Error> {
        self.at += 1;
        let mut out = String::new();
        loop {
            // Copy the run up to the next quote, escape or control byte
            // whole: the text is UTF-8 already.
            let from = self.at;
            while matches!(self.peek(), Some(b) if b != b'\x22' && b != b'\\' && b >= 0x20) {
                self.at += 1;
            }
            match core::str::from_utf8(&self.bytes[from..self.at]) {
                Ok(s) => out.push_str(s),
                Err(_) => return self.fail(),
            }
            match self.peek() {
                Some(b'\x22') => {
                    self.at += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.at += 1;
                    let Some(e) = self.peek() else {
                        return self.fail();
                    };
                    self.at += 1;
                    match e {
                        b'\x22' => out.push('\u{22}'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let c = if (0xd800..0xdc00).contains(&hi) {
                                if !self.bytes[self.at..].starts_with(b"\\u") {
                                    return self.fail();
                                }
                                self.at += 2;
                                let lo = self.hex4()?;
                                if !(0xdc00..0xe000).contains(&lo) {
                                    return self.fail();
                                }
                                0x10000 + ((hi - 0xd800) << 10) + (lo - 0xdc00)
                            } else {
                                hi
                            };
                            match char::from_u32(c) {
                                Some(c) => out.push(c),
                                None => return self.fail(),
                            }
                        }
                        _ => return self.fail(),
                    }
                }
                // A control byte, or the end of the text.
                _ => return self.fail(),
            }
        }
    }
}
