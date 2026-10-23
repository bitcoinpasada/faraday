//! Hex: digits typed as they are.

use crate::{Error, MAX_HEX, RawEntropy, Strength};
use faraday_crypto::{Zeroize, ZeroizeOnDrop, sha256};

/// Accumulates hex digits typed by the user.
pub struct RawHex {
    digits: [u8; MAX_HEX],
    len: u8,
}

impl Default for RawHex {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for RawHex {
    fn zeroize(&mut self) {
        self.digits.zeroize();
        self.len.zeroize();
    }
}

impl Drop for RawHex {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for RawHex {}

impl RawHex {
    /// No digits yet.
    pub fn new() -> Self {
        RawHex {
            digits: [0u8; MAX_HEX],
            len: 0,
        }
    }

    /// Parses `text`, which must be exactly 32, 40, 48, 56 or 64 hex
    /// digits.
    pub fn parse(text: &str) -> Result<RawEntropy, Error> {
        let mut h = RawHex::new();
        if text.len() > MAX_HEX {
            return Err(Error::BadLength);
        }
        for c in text.chars() {
            if !h.push(c) {
                return Err(Error::BadLength);
            }
        }
        h.entropy()
    }

    /// Adds a hex digit (either case). Returns `false` for anything else
    /// or when full.
    pub fn push(&mut self, c: char) -> bool {
        if !c.is_ascii_hexdigit() || usize::from(self.len) >= MAX_HEX {
            return false;
        }
        self.digits[usize::from(self.len)] = c.to_ascii_lowercase() as u8;
        self.len += 1;
        true
    }

    /// Removes the last digit.
    pub fn pop(&mut self) -> Option<char> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        let c = self.digits[usize::from(self.len)] as char;
        self.digits[usize::from(self.len)] = 0;
        Some(c)
    }

    /// Forgets every digit.
    pub fn clear(&mut self) {
        self.zeroize();
    }

    /// Number of digits.
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether there are no digits.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The digits so far, lower-case.
    pub fn digits(&self) -> &str {
        core::str::from_utf8(&self.digits[..self.len()]).expect("ascii")
    }

    /// The last digit, if any.
    pub fn last(&self) -> Option<char> {
        self.digits().chars().last()
    }

    /// The strength the digit count corresponds to, two digits a byte:
    /// 32, 40, 48, 56 or 64 digits.
    pub fn strength(&self) -> Option<Strength> {
        self.len()
            .is_multiple_of(2)
            .then(|| Strength::for_bytes(self.len() / 2))
            .flatten()
    }

    /// SHA-256 of the digits as typed, lower-case.
    pub fn commitment(&self) -> [u8; 32] {
        sha256(self.digits().as_bytes())
    }

    /// The bytes the digits spell.
    pub fn entropy(&self) -> Result<RawEntropy, Error> {
        let strength = self.strength().ok_or(Error::BadLength)?;
        let mut bytes = [0u8; 32];
        for (i, pair) in self.digits().as_bytes().chunks_exact(2).enumerate() {
            bytes[i] = (nibble(pair[0]) << 4) | nibble(pair[1]);
        }
        let e = RawEntropy::new(&bytes[..strength.bytes()]);
        bytes.zeroize();
        Ok(e)
    }
}

fn nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Error, Strength};

    #[test]
    fn hex_parsing() {
        let e = RawHex::parse("00000000000000000000000000000000").unwrap();
        assert_eq!(e.as_bytes(), &[0u8; 16]);
        let e = RawHex::parse("FFfF00000000000000000000000000000000000000000000000000000000ab1C")
            .unwrap();
        assert_eq!(e.as_bytes().len(), 32);
        assert_eq!(e.as_bytes()[0], 0xff);
        assert_eq!(e.as_bytes()[31], 0x1c);
        assert_eq!(RawHex::parse("0000").err(), Some(Error::BadLength));
        assert_eq!(
            RawHex::parse("0000000000000000000000000000000g").err(),
            Some(Error::BadLength)
        );
        assert_eq!(RawHex::parse("").err(), Some(Error::BadLength));
        let mut h = RawHex::new();
        assert!(!h.push('g'));
        assert!(h.push('A'));
        assert_eq!(h.digits(), "a");
        assert_eq!(h.strength(), None);
        assert_eq!(h.entropy().err(), Some(Error::BadLength));
        assert_eq!(h.pop(), Some('a'));
        assert!(h.is_empty());
        for _ in 0..64 {
            assert!(h.push('0'));
        }
        assert!(!h.push('0'));
        assert_eq!(h.strength(), Some(Strength::Bits256));
    }
}
