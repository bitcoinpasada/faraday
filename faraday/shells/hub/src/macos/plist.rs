//! A reader for the XML property lists macOS's tools print (`diskutil
//! … -plist`, a bundle's `Info.plist`), with no macOS call in it.
//!
//! The format is Apple's `PropertyList-1.0.dtd`: one `<plist>` holding
//! one value, a value being `<dict>` (`<key>` then a value, repeated),
//! `<array>`, `<string>`, `<integer>`, `<real>`, `<true/>`, `<false/>`,
//! `<data>` (base64) or `<date>` (`YYYY-MM-DDTHH:MM:SSZ`). Text carries
//! XML's five named entities and numeric character references.
//!
//! What it does not understand is an error, never a partial answer: an
//! unknown element, an attribute on anything but `<plist>`, text where a
//! value belongs, a key twice in one dictionary, a list cut short, CDATA,
//! a binary property list, anything after `</plist>` but white space and
//! comments.

use std::collections::BTreeMap;

/// A property list value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `<dict>`.
    Dict(BTreeMap<String, Value>),
    /// `<array>`.
    Array(Vec<Value>),
    /// `<string>`.
    String(String),
    /// `<integer>`.
    Integer(i128),
    /// `<real>`.
    Real(f64),
    /// `<true/>`, `<false/>`.
    Bool(bool),
    /// `<data>`, decoded.
    Data(Vec<u8>),
    /// `<date>`, as written.
    Date(String),
}

/// The largest number a `<real>` may carry and still be read as a
/// whole number of bytes: every integer up to 2^53 is exact in an `f64`.
const EXACT: f64 = 9_007_199_254_740_992.0;

/// How deep values may nest: far more than any answer the hub reads.
const DEPTH: usize = 64;

impl Value {
    /// The value under `key`, when this is a dictionary that has it.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Dict(d) => d.get(key),
            _ => None,
        }
    }

    /// The text of a `<string>`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// A count of bytes: an `<integer>` from 0 to `u64::MAX`, or a
    /// `<real>` that is a whole number no larger than 2^53 (`diskutil`
    /// prints some sizes as reals).
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Integer(n) => u64::try_from(*n).ok(),
            Value::Real(r) if r.is_finite() && *r >= 0.0 && *r <= EXACT && r.fract() == 0.0 => {
                Some(*r as u64)
            }
            _ => None,
        }
    }

    /// A `<true/>` or `<false/>`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The items of an `<array>`.
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }
}

/// Reads a property list: its one value, or why not.
pub fn parse(text: &str) -> Result<Value, String> {
    let mut r = Reader { s: text, at: 0 };
    r.prolog()?;
    let open = r.tag()?;
    if open.name != "plist" || open.close || open.empty {
        return Err(r.err("no <plist>"));
    }
    if !open.attrs.iter().all(|(k, _)| k == "version") {
        return Err(r.err("an attribute <plist> does not have"));
    }
    let value = r.value(0)?;
    let close = r.tag()?;
    if close.name != "plist" || !close.close {
        return Err(r.err("more than one value in <plist>"));
    }
    r.misc()?;
    if r.at != r.s.len() {
        return Err(r.err("text after </plist>"));
    }
    Ok(value)
}

/// One tag: `<name a="v">`, `</name>` or `<name/>`.
struct Tag {
    name: String,
    attrs: Vec<(String, String)>,
    close: bool,
    empty: bool,
}

struct Reader<'a> {
    s: &'a str,
    at: usize,
}

impl Reader<'_> {
    fn err(&self, what: &str) -> String {
        format!("property list, byte {}: {what}", self.at)
    }

    fn rest(&self) -> &str {
        &self.s[self.at..]
    }

    fn skip_space(&mut self) {
        let n = self.rest().len() - self.rest().trim_start().len();
        self.at += n;
    }

    /// Past `until`, or an error when it never comes.
    fn skip_past(&mut self, until: &str) -> Result<(), String> {
        match self.rest().find(until) {
            Some(i) => {
                self.at += i + until.len();
                Ok(())
            }
            None => Err(self.err("cut short")),
        }
    }

    /// White space and comments.
    fn misc(&mut self) -> Result<(), String> {
        loop {
            self.skip_space();
            if self.rest().starts_with("<!--") {
                self.skip_past("-->")?;
            } else {
                return Ok(());
            }
        }
    }

    /// The XML declaration, the document type and comments before
    /// `<plist>`.
    fn prolog(&mut self) -> Result<(), String> {
        if self.rest().starts_with("bplist") {
            return Err(self.err("a binary property list"));
        }
        let bom = '\u{feff}';
        if self.rest().starts_with(bom) {
            self.at += bom.len_utf8();
        }
        loop {
            self.misc()?;
            if self.rest().starts_with("<?xml") {
                self.skip_past("?>")?;
            } else if self.rest().starts_with("<!DOCTYPE") {
                self.skip_past(">")?;
            } else {
                return Ok(());
            }
        }
    }

    /// The next tag, after white space and comments.
    fn tag(&mut self) -> Result<Tag, String> {
        self.misc()?;
        if !self.rest().starts_with('<') {
            return Err(self.err(if self.rest().is_empty() {
                "cut short"
            } else {
                "text where a value belongs"
            }));
        }
        if self.rest().starts_with("<![CDATA[") || self.rest().starts_with("<!") {
            return Err(self.err("markup a property list does not have"));
        }
        let end = match self.rest().find('>') {
            Some(i) => self.at + i,
            None => return Err(self.err("cut short")),
        };
        let mut inner = &self.s[self.at + 1..end];
        self.at = end + 1;
        let close = inner.starts_with('/');
        if close {
            inner = &inner[1..];
        }
        let empty = inner.ends_with('/');
        if empty {
            inner = &inner[..inner.len() - 1];
        }
        if close && empty {
            return Err(self.err("a malformed tag"));
        }
        let mut parts = inner.trim_end().splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or("").to_string();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(self.err("a malformed tag"));
        }
        let attrs = attributes(parts.next().unwrap_or(""))
            .ok_or_else(|| self.err("a malformed attribute"))?;
        if close && !attrs.is_empty() {
            return Err(self.err("a malformed tag"));
        }
        Ok(Tag {
            name,
            attrs,
            close,
            empty,
        })
    }

    /// The text up to the next `<`, its entities read, then the closing
    /// tag `name`.
    fn text(&mut self, name: &str) -> Result<String, String> {
        let end = match self.rest().find('<') {
            Some(i) => self.at + i,
            None => return Err(self.err("cut short")),
        };
        let raw = &self.s[self.at..end];
        let text = entities(raw).ok_or_else(|| self.err("an unknown entity"))?;
        self.at = end;
        let close = self.tag()?;
        if !close.close || close.name != name {
            return Err(self.err("markup inside a text value"));
        }
        Ok(text)
    }

    /// One value.
    fn value(&mut self, depth: usize) -> Result<Value, String> {
        if depth > DEPTH {
            return Err(self.err("nested too deep"));
        }
        let tag = self.tag()?;
        if tag.close {
            return Err(self.err("a value missing"));
        }
        if !tag.attrs.is_empty() {
            return Err(self.err("an attribute on a value"));
        }
        let name = tag.name.as_str();
        if tag.empty {
            return match name {
                "true" => Ok(Value::Bool(true)),
                "false" => Ok(Value::Bool(false)),
                "dict" => Ok(Value::Dict(BTreeMap::new())),
                "array" => Ok(Value::Array(Vec::new())),
                "string" => Ok(Value::String(String::new())),
                "data" => Ok(Value::Data(Vec::new())),
                _ => Err(self.err("an element a property list does not have")),
            };
        }
        match name {
            "true" | "false" => {
                let t = self.tag()?;
                if t.close && t.name == name {
                    Ok(Value::Bool(name == "true"))
                } else {
                    Err(self.err("markup inside a boolean"))
                }
            }
            "string" => Ok(Value::String(self.text(name)?)),
            "integer" => {
                let t = self.text(name)?;
                let t = t.trim();
                let digits = t.strip_prefix(['-', '+']).unwrap_or(t);
                if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(self.err("not an integer"));
                }
                t.parse()
                    .map(Value::Integer)
                    .map_err(|_| self.err("not an integer"))
            }
            "real" => {
                let t = self.text(name)?;
                t.trim()
                    .parse()
                    .map(Value::Real)
                    .map_err(|_| self.err("not a real"))
            }
            "data" => {
                let t = self.text(name)?;
                base64(&t)
                    .map(Value::Data)
                    .ok_or_else(|| self.err("not base64"))
            }
            "date" => {
                let t = self.text(name)?;
                if date(&t) {
                    Ok(Value::Date(t))
                } else {
                    Err(self.err("not a date"))
                }
            }
            "array" => {
                let mut items = Vec::new();
                loop {
                    self.misc()?;
                    if self.rest().starts_with("</") {
                        let t = self.tag()?;
                        if t.name != "array" {
                            return Err(self.err("an array closed as something else"));
                        }
                        return Ok(Value::Array(items));
                    }
                    items.push(self.value(depth + 1)?);
                }
            }
            "dict" => {
                let mut map = BTreeMap::new();
                loop {
                    let t = self.tag()?;
                    if t.close {
                        if t.name != "dict" {
                            return Err(self.err("a dictionary closed as something else"));
                        }
                        return Ok(Value::Dict(map));
                    }
                    if t.name != "key" || !t.attrs.is_empty() {
                        return Err(self.err("a value without its key"));
                    }
                    let key = if t.empty {
                        String::new()
                    } else {
                        self.text("key")?
                    };
                    let v = self.value(depth + 1)?;
                    if map.insert(key, v).is_some() {
                        return Err(self.err("a key twice in one dictionary"));
                    }
                }
            }
            _ => Err(self.err("an element a property list does not have")),
        }
    }
}

/// `a="v" b='w'`, or `None` when malformed.
fn attributes(mut s: &str) -> Option<Vec<(String, String)>> {
    let mut out = Vec::new();
    loop {
        s = s.trim_start();
        if s.is_empty() {
            return Some(out);
        }
        let (name, rest) = s.split_once('=')?;
        let name = name.trim();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric()) {
            return None;
        }
        let rest = rest.trim_start();
        let quote = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        let rest = &rest[1..];
        let end = rest.find(quote)?;
        out.push((name.to_string(), entities(&rest[..end])?));
        s = &rest[end + 1..];
    }
}

/// `raw` with XML's entities and character references read, or `None`
/// for one it does not know.
fn entities(raw: &str) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i + 1..];
        let end = rest.find(';')?;
        let name = &rest[..end];
        let c = match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let n = if let Some(hex) = name.strip_prefix("#x") {
                    u32::from_str_radix(hex, 16).ok()?
                } else {
                    name.strip_prefix('#')?.parse().ok()?
                };
                char::from_u32(n)?
            }
        };
        out.push(c);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    Some(out)
}

/// Standard base64 with white space anywhere and `=` padding.
fn base64(text: &str) -> Option<Vec<u8>> {
    let mut bits = 0u32;
    let mut n = 0;
    let mut out = Vec::new();
    let mut pad = 0;
    for c in text.bytes().filter(|b| !b.is_ascii_whitespace()) {
        if c == b'=' {
            pad += 1;
            continue;
        }
        if pad > 0 {
            return None;
        }
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        bits = (bits << 6) | u32::from(v);
        n += 6;
        if n >= 8 {
            n -= 8;
            out.push((bits >> n) as u8);
            bits &= (1 << n) - 1;
        }
    }
    (pad <= 2 && bits == 0).then_some(out)
}

/// `YYYY-MM-DDTHH:MM:SSZ`.
fn date(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 20
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            10 => *c == b'T',
            13 | 16 => *c == b':',
            19 => *c == b'Z',
            _ => c.is_ascii_digit(),
        })
}
