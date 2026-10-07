//! The DER this crate writes, and the little it reads back: tag, length,
//! value. Only what certificates, CMS and Authenticode need.

use alloc::vec::Vec;

/// Tags.
pub mod tag {
    /// BOOLEAN.
    pub const BOOLEAN: u8 = 0x01;
    /// INTEGER.
    pub const INTEGER: u8 = 0x02;
    /// BIT STRING.
    pub const BIT_STRING: u8 = 0x03;
    /// OCTET STRING.
    pub const OCTET_STRING: u8 = 0x04;
    /// NULL.
    pub const NULL: u8 = 0x05;
    /// OBJECT IDENTIFIER.
    pub const OID: u8 = 0x06;
    /// UTF8String.
    pub const UTF8_STRING: u8 = 0x0c;
    /// UTCTime.
    pub const UTC_TIME: u8 = 0x17;
    /// GeneralizedTime.
    pub const GENERALIZED_TIME: u8 = 0x18;
    /// SEQUENCE, constructed.
    pub const SEQUENCE: u8 = 0x30;
    /// SET, constructed.
    pub const SET: u8 = 0x31;
}

/// A tag, its length and its value.
pub fn tlv(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut out = alloc::vec![tag];
    let n = value.len();
    if n < 0x80 {
        out.push(n as u8);
    } else {
        let bytes = (n as u32).to_be_bytes();
        let skip = bytes.iter().position(|&b| b != 0).unwrap_or(3);
        out.push(0x80 | (4 - skip) as u8);
        out.extend_from_slice(&bytes[skip..]);
    }
    out.extend_from_slice(value);
    out
}

/// A SEQUENCE of already-encoded parts.
pub fn seq(parts: &[&[u8]]) -> Vec<u8> {
    tlv(tag::SEQUENCE, &parts.concat())
}

/// A SET of already-encoded parts, sorted as DER requires.
pub fn set(parts: &[&[u8]]) -> Vec<u8> {
    let mut sorted: Vec<&[u8]> = parts.to_vec();
    sorted.sort();
    tlv(tag::SET, &sorted.concat())
}

/// An unsigned big-endian INTEGER: leading zeros dropped, a zero added
/// where the top bit is set.
pub fn uint(bytes: &[u8]) -> Vec<u8> {
    let start = bytes
        .iter()
        .position(|&b| b != 0)
        .unwrap_or(bytes.len().saturating_sub(1));
    let b = &bytes[start..];
    let mut v = Vec::with_capacity(b.len() + 1);
    if b.first().is_some_and(|x| x & 0x80 != 0) {
        v.push(0);
    }
    v.extend_from_slice(b);
    if v.is_empty() {
        v.push(0);
    }
    tlv(tag::INTEGER, &v)
}

/// A small INTEGER.
pub fn int(n: u32) -> Vec<u8> {
    uint(&n.to_be_bytes())
}

/// An OBJECT IDENTIFIER from its arcs.
pub fn oid(arcs: &[u64]) -> Vec<u8> {
    let mut v = Vec::new();
    let mut put = |mut n: u64| {
        // Base 128, most significant group first, the high bit set on
        // every group but the last.
        let mut groups = alloc::vec![(n & 0x7f) as u8];
        n >>= 7;
        while n > 0 {
            groups.push((n & 0x7f) as u8 | 0x80);
            n >>= 7;
        }
        groups.reverse();
        v.extend(groups);
    };
    put(arcs[0] * 40 + arcs[1]);
    for &a in &arcs[2..] {
        put(a);
    }
    tlv(tag::OID, &v)
}

/// NULL.
pub fn null() -> Vec<u8> {
    alloc::vec![tag::NULL, 0]
}

/// An OCTET STRING.
pub fn octets(v: &[u8]) -> Vec<u8> {
    tlv(tag::OCTET_STRING, v)
}

/// A BIT STRING of whole bytes.
pub fn bits(v: &[u8]) -> Vec<u8> {
    let mut b = alloc::vec![0];
    b.extend_from_slice(v);
    tlv(tag::BIT_STRING, &b)
}

/// A context-specific constructed tag, `[n]`, around encoded parts.
pub fn explicit(n: u8, inner: &[u8]) -> Vec<u8> {
    tlv(0xa0 | n, inner)
}

/// One TLV read from `bytes` at `at`: its tag, its value's range, and
/// where the next begins.
pub fn read(bytes: &[u8], at: usize) -> Option<(u8, core::ops::Range<usize>, usize)> {
    let t = *bytes.get(at)?;
    let first = *bytes.get(at + 1)? as usize;
    let (len, head) = if first < 0x80 {
        (first, 2)
    } else {
        let k = first & 0x7f;
        if k == 0 || k > 4 {
            return None;
        }
        let mut n = 0usize;
        for i in 0..k {
            n = (n << 8) | *bytes.get(at + 2 + i)? as usize;
        }
        (n, 2 + k)
    };
    let start = at + head;
    let end = start.checked_add(len)?;
    if end > bytes.len() {
        return None;
    }
    Some((t, start..end, end))
}

/// The TLVs directly inside a constructed value.
pub fn children(bytes: &[u8]) -> Option<Vec<(u8, core::ops::Range<usize>)>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let (t, r, next) = read(bytes, at)?;
        out.push((t, r));
        at = next;
    }
    Some(out)
}
