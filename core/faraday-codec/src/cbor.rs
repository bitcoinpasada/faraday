//! Just enough CBOR (RFC 8949) for the UR types: definite lengths only, a
//! bounded depth, every length checked against what is left before
//! anything is allocated. One module for the UR registry's decoder, the
//! heads and byte strings the encoders write, the byte-string payload of
//! a single-part UR and the fountain part's five items.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

/// A decoded item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// Major type 0.
    Uint(u64),
    /// Major type 1, as `-1 - n`.
    Neg(u64),
    /// Major type 2.
    Bytes(Vec<u8>),
    /// Major type 3.
    Text(String),
    /// Major type 4.
    Array(Vec<Value>),
    /// Major type 5, in order.
    Map(Vec<(Value, Value)>),
    /// Major type 6.
    Tag(u64, Box<Value>),
    /// `true` or `false`.
    Bool(bool),
    /// `null`.
    Null,
}

/// Why CBOR did not decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Malformed;

const MAX_DEPTH: usize = 16;

impl Value {
    /// The one item `bytes` holds, with nothing after it.
    pub fn decode(bytes: &[u8]) -> Result<Value, Malformed> {
        let mut at = 0;
        let v = item(bytes, &mut at, 0)?;
        if at != bytes.len() {
            return Err(Malformed);
        }
        Ok(v)
    }

    /// The value under key `k` of a map with unsigned keys.
    pub fn get(&self, k: u64) -> Option<&Value> {
        match self {
            Value::Map(m) => m
                .iter()
                .find(|(key, _)| *key == Value::Uint(k))
                .map(|(_, v)| v),
            _ => None,
        }
    }

    /// The value inside any number of tags, and the outermost tag.
    pub fn untag(&self) -> (Option<u64>, &Value) {
        match self {
            Value::Tag(t, inner) => (Some(*t), inner.untag().1),
            v => (None, v),
        }
    }
}

fn take<'a>(b: &'a [u8], at: &mut usize, n: usize) -> Result<&'a [u8], Malformed> {
    let end = at.checked_add(n).ok_or(Malformed)?;
    let s = b.get(*at..end).ok_or(Malformed)?;
    *at = end;
    Ok(s)
}

fn arg(b: &[u8], at: &mut usize, info: u8) -> Result<u64, Malformed> {
    Ok(match info {
        0..=23 => u64::from(info),
        24 => u64::from(take(b, at, 1)?[0]),
        25 => {
            let s = take(b, at, 2)?;
            u64::from(u16::from_be_bytes([s[0], s[1]]))
        }
        26 => {
            let s = take(b, at, 4)?;
            u64::from(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
        }
        27 => {
            let s = take(b, at, 8)?;
            let mut a = [0u8; 8];
            a.copy_from_slice(s);
            u64::from_be_bytes(a)
        }
        _ => return Err(Malformed),
    })
}

fn item(b: &[u8], at: &mut usize, depth: usize) -> Result<Value, Malformed> {
    if depth > MAX_DEPTH {
        return Err(Malformed);
    }
    let head = take(b, at, 1)?[0];
    let (major, info) = (head >> 5, head & 0x1f);
    if major == 7 {
        return match info {
            20 => Ok(Value::Bool(false)),
            21 => Ok(Value::Bool(true)),
            22 => Ok(Value::Null),
            _ => Err(Malformed),
        };
    }
    let n = arg(b, at, info)?;
    // A count can never exceed the bytes left: every item takes one.
    let left = (b.len() - *at) as u64;
    match major {
        0 => Ok(Value::Uint(n)),
        1 => Ok(Value::Neg(n)),
        2 | 3 => {
            if n > left {
                return Err(Malformed);
            }
            let s = take(b, at, n as usize)?.to_vec();
            if major == 2 {
                Ok(Value::Bytes(s))
            } else {
                String::from_utf8(s).map(Value::Text).map_err(|_| Malformed)
            }
        }
        4 => {
            if n > left {
                return Err(Malformed);
            }
            (0..n)
                .map(|_| item(b, at, depth + 1))
                .collect::<Result<_, _>>()
                .map(Value::Array)
        }
        5 => {
            if n.saturating_mul(2) > left {
                return Err(Malformed);
            }
            let mut m = Vec::with_capacity(n as usize);
            for _ in 0..n {
                let k = item(b, at, depth + 1)?;
                let v = item(b, at, depth + 1)?;
                m.push((k, v));
            }
            Ok(Value::Map(m))
        }
        6 => Ok(Value::Tag(n, Box::new(item(b, at, depth + 1)?))),
        _ => Err(Malformed),
    }
}

/// A CBOR head: major type and argument, in the shortest form.
pub fn head(major: u8, n: u64) -> Vec<u8> {
    let mut v = Vec::with_capacity(9);
    push_head(&mut v, major, n);
    v
}

fn push_head(out: &mut Vec<u8>, major: u8, n: u64) {
    let m = major << 5;
    match n {
        0..=23 => out.push(m | n as u8),
        24..=0xff => out.extend_from_slice(&[m | 24, n as u8]),
        0x100..=0xffff => {
            out.push(m | 25);
            out.extend_from_slice(&(n as u16).to_be_bytes());
        }
        0x1_0000..=0xffff_ffff => {
            out.push(m | 26);
            out.extend_from_slice(&(n as u32).to_be_bytes());
        }
        _ => {
            out.push(m | 27);
            out.extend_from_slice(&n.to_be_bytes());
        }
    }
}

/// A byte string (major type 2) around `b`, with room for it made once.
pub fn bytes(b: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(b.len() + 9);
    push_head(&mut v, 2, b.len() as u64);
    v.extend_from_slice(b);
    v
}

/// The content of a byte string that is all of `cbor`: a head of at most
/// a four-byte length (not an eight-byte one), and nothing after it.
pub fn byte_string(cbor: &[u8]) -> Result<&[u8], Malformed> {
    let mut r = Reader::new(cbor);
    let head = r.read()?;
    if head >> 5 != 2 || head & 0x1f > 26 {
        return Err(Malformed);
    }
    let len = usize::try_from(r.argument_of(head)?).map_err(|_| Malformed)?;
    let s = r.take(len)?;
    if !r.is_done() {
        return Err(Malformed);
    }
    Ok(s)
}

/// An unsigned integer (major type 0) of at most 32 bits, in the
/// shortest form.
pub fn push_uint32(out: &mut Vec<u8>, value: u32) {
    push_head(out, 0, u64::from(value));
}

/// Reads heads and byte strings one at a time, for a fixed layout.
pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    /// A reader at the start of `bytes`.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    /// Whether every byte has been read.
    pub fn is_done(&self) -> bool {
        self.at == self.bytes.len()
    }

    /// The next byte.
    pub fn read(&mut self) -> Result<u8, Malformed> {
        Ok(take(self.bytes, &mut self.at, 1)?[0])
    }

    /// The next `n` bytes.
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], Malformed> {
        take(self.bytes, &mut self.at, n)
    }

    fn argument_of(&mut self, head: u8) -> Result<u64, Malformed> {
        arg(self.bytes, &mut self.at, head & 0x1f)
    }

    /// The argument of a head whose major type is `major`.
    pub fn argument(&mut self, major: u8) -> Result<u64, Malformed> {
        let head = self.read()?;
        if head >> 5 != major {
            return Err(Malformed);
        }
        self.argument_of(head)
    }

    /// An unsigned integer that fits 32 bits.
    pub fn uint32(&mut self) -> Result<u32, Malformed> {
        u32::try_from(self.argument(0)?).map_err(|_| Malformed)
    }

    /// A byte string, its length up to eight bytes wide.
    pub fn byte_string(&mut self) -> Result<Vec<u8>, Malformed> {
        let len = usize::try_from(self.argument(2)?).map_err(|_| Malformed)?;
        Ok(self.take(len)?.to_vec())
    }
}
