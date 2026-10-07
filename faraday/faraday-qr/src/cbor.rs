//! Just enough CBOR (RFC 8949) for the UR registry's types: definite
//! lengths only, a bounded depth, every length checked against what is
//! left before anything is allocated.

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

/// A CBOR head: major type and argument.
pub fn head(major: u8, n: u64) -> Vec<u8> {
    let m = major << 5;
    match n {
        0..=23 => vec![m | n as u8],
        24..=0xff => vec![m | 24, n as u8],
        0x100..=0xffff => {
            let mut v = vec![m | 25];
            v.extend_from_slice(&(n as u16).to_be_bytes());
            v
        }
        0x1_0000..=0xffff_ffff => {
            let mut v = vec![m | 26];
            v.extend_from_slice(&(n as u32).to_be_bytes());
            v
        }
        _ => {
            let mut v = vec![m | 27];
            v.extend_from_slice(&n.to_be_bytes());
            v
        }
    }
}

/// A byte string.
pub fn bytes(b: &[u8]) -> Vec<u8> {
    let mut v = head(2, b.len() as u64);
    v.extend_from_slice(b);
    v
}
