//! A JSON value reader for the wallet files other wallets write
//! (`docs/INTEROP.md` §5, Single-key JSON files): RFC 8259's grammar and every string escape
//! (serde_json reads them), at most [`MAX_DEPTH`] arrays and objects deep
//! and [`MAX_SIZE`] bytes long.
//!
//! An object that names one key twice is refused: two readers could take
//! different values from it. A number is kept as the decimal text Rust
//! writes for the `u64`, `i64` or `f64` serde_json reads (`1e3` is
//! `1000`), and the text kept reads back as the same value; a reader asks
//! for the whole number it needs ([`Value::as_u64`]).
//!
//! Its strings, keys and numbers are wiping text ([`faraday_text::Text`]): a
//! wallet file may hold a private key where only public keys belong.
//!
//! It replaced the
//! scanners [`crate::coldcard`] and Faraday's readers used, which took
//! the first `"key"` anywhere in the text and undid no escapes.

pub use crate::json_quote::quote;
use alloc::vec::Vec;
use core::cell::Cell;
use core::fmt;
use faraday_text::{Text, text};
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};

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
    /// A number, as the decimal text of the `u64`, `i64` or `f64` read.
    Number(Text),
    /// A string, its escapes undone, in wiping text: a wallet file may
    /// hold a private key where only public keys belong.
    String(Text),
    /// An array.
    Array(Vec<Value>),
    /// An object, its members in the order written.
    Object(Vec<(Text, Value)>),
}

/// Why a text is not read as JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Longer than [`MAX_SIZE`].
    TooLarge,
    /// Nested deeper than [`MAX_DEPTH`].
    TooDeep,
    /// An object names a key twice.
    DuplicateKey,
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
            Value::String(s) => Some(s.as_ref()),
            _ => None,
        }
    }

    /// The whole number this value is, as decimal digits without a sign
    /// or fraction.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Number(n) if n.as_bytes().iter().all(u8::is_ascii_digit) => n.parse().ok(),
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

    /// The first member named `key` at any depth, in the order the text
    /// writes them (an object's members before what follows it): the
    /// field a wallet file names once wherever its writer put it.
    pub fn find(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(members) => members
                .iter()
                .find_map(|(k, v)| if k == key { Some(v) } else { v.find(key) }),
            Value::Array(items) => items.iter().find_map(|v| v.find(key)),
            _ => None,
        }
    }

    /// Every string member named `key` at any depth, in the order the
    /// text writes them.
    pub fn strings_named(&self, key: &str) -> Vec<&str> {
        let mut out = Vec::new();
        self.collect_strings(key, &mut out);
        out
    }

    /// Every string value at any depth, in the order the text writes
    /// them (member names are not values).
    pub fn all_strings(&self) -> Vec<&str> {
        let mut out = Vec::new();
        self.collect_all(&mut out);
        out
    }

    fn collect_all<'a>(&'a self, out: &mut Vec<&'a str>) {
        match self {
            Value::String(s) => out.push(s),
            Value::Object(members) => members.iter().for_each(|(_, v)| v.collect_all(out)),
            Value::Array(items) => items.iter().for_each(|v| v.collect_all(out)),
            _ => {}
        }
    }

    fn collect_strings<'a>(&'a self, key: &str, out: &mut Vec<&'a str>) {
        match self {
            Value::Object(members) => {
                for (k, v) in members {
                    if k == key
                        && let Value::String(s) = v
                    {
                        out.push(s);
                    }
                    v.collect_strings(key, out);
                }
            }
            Value::Array(items) => {
                for v in items {
                    v.collect_strings(key, out);
                }
            }
            _ => {}
        }
    }
}

/// Reads a text that is one JSON value, with white space (and a UTF-8
/// byte order mark) around it. Costs a pass over the text.
///
/// serde_json reads the grammar; a seed builds the [`Value`], counts
/// the depth and refuses a key named twice.
pub fn parse(text: &str) -> Result<Value, Error> {
    if text.len() > MAX_SIZE {
        return Err(Error::TooLarge);
    }
    let body = text.strip_prefix('\u{feff}').unwrap_or(text);
    let refused = Cell::new(None);
    let mut de = serde_json::Deserializer::from_str(body);
    let read = Seed {
        depth: 0,
        refused: &refused,
    }
    .deserialize(&mut de)
    .and_then(|v| de.end().map(|()| v));
    match read {
        Ok(v) => Ok(v),
        Err(e) => Err(refused.get().unwrap_or_else(|| {
            // serde_json says the line and column (1-based, in bytes).
            let before: usize = body
                .split_inclusive('\n')
                .take(e.line().saturating_sub(1))
                .map(str::len)
                .sum();
            Error::Syntax((before + e.column().saturating_sub(1)).min(body.len()))
        })),
    }
}

/// Builds a [`Value`] while serde_json reads, one level of nesting deep.
struct Seed<'a> {
    depth: usize,
    refused: &'a Cell<Option<Error>>,
}

impl Seed<'_> {
    fn refuse<E: de::Error>(&self, why: Error) -> E {
        self.refused.set(Some(why));
        E::custom("refused")
    }

    fn inside(&self) -> Seed<'_> {
        Seed {
            depth: self.depth + 1,
            refused: self.refused,
        }
    }
}

impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = Value;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        d.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed<'_> {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("json")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }

    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Value, E> {
        Ok(Value::Number(text!("{}", v)))
    }

    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Value, E> {
        Ok(Value::Number(text!("{}", v)))
    }

    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Value, E> {
        Ok(Value::Number(text!("{}", v)))
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Value, E> {
        Ok(Value::String(Text::of(v)))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        if self.depth + 1 > MAX_DEPTH {
            return Err(self.refuse(Error::TooDeep));
        }
        let mut items = Vec::new();
        while let Some(v) = seq.next_element_seed(self.inside())? {
            items.push(v);
        }
        Ok(Value::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        if self.depth + 1 > MAX_DEPTH {
            return Err(self.refuse(Error::TooDeep));
        }
        let mut members: Vec<(Text, Value)> = Vec::new();
        while let Some(key) = map.next_key_seed(KeyText)? {
            let v = map.next_value_seed(self.inside())?;
            members.push((key, v));
        }
        // Duplicates are found by sorting the positions by key and
        // comparing neighbours, so the check costs n log n and keeps no
        // second copy of a key.
        let mut order: Vec<usize> = (0..members.len()).collect();
        order.sort_unstable_by(|&a, &b| members[a].0.as_str().cmp(members[b].0.as_str()));
        if order
            .windows(2)
            .any(|w| members[w[0]].0.as_str() == members[w[1]].0.as_str())
        {
            return Err(self.refuse(Error::DuplicateKey));
        }
        Ok(Value::Object(members))
    }
}

/// Reads an object's key as wiping text.
struct KeyText;

impl<'de> DeserializeSeed<'de> for KeyText {
    type Value = Text;

    fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Text, D::Error> {
        d.deserialize_str(self)
    }
}

impl Visitor<'_> for KeyText {
    type Value = Text;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("key")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Text, E> {
        Ok(Text::of(v))
    }
}
