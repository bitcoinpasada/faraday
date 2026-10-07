//! The BIP-380 output descriptor checksum.
//!
//! Descriptors themselves are produced by [`crate::account::AccountXpub`];
//! this module only implements the 8-character BCH checksum that follows
//! the `#`, ported from the reference Python in BIP-380.
//!
//! ```
//! use osk_bip::descriptor::{descriptor_checksum, verify_checksum};
//!
//! assert_eq!(descriptor_checksum("raw(deadbeef)"), Some(*b"89f8spxm"));
//! assert!(verify_checksum("raw(deadbeef)#89f8spxm"));
//! assert!(!verify_checksum("raw(deedbeef)#89f8spxm"));
//! ```

/// Every character a descriptor may contain, in symbol order.
const INPUT_CHARSET: &[u8] =
    b"0123456789()[],'/*abcdefgh@:$%{}IJKLMNOPQRSTUVWXYZ&+-.;<=>?!^_|~ijklmnopqrstuvwxyzABCDEFGH`#\"\\ ";
/// The bech32 alphabet, used for the checksum characters.
const CHECKSUM_CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";
const GENERATOR: [u64; 5] = [
    0x00f5_dee5_1989,
    0x00a9_fdca_3312,
    0x001b_ab10_e32d,
    0x0037_06b1_677a,
    0x0064_4d62_6ffd,
];

/// Feeds descriptor characters into the BCH polymod, inserting the group
/// symbol after every third character as BIP-380 specifies.
struct Engine {
    chk: u64,
    groups: [u64; 3],
    group_len: usize,
}

impl Engine {
    fn new() -> Self {
        Self {
            chk: 1,
            groups: [0; 3],
            group_len: 0,
        }
    }

    fn polymod(&mut self, value: u64) {
        let top = self.chk >> 35;
        self.chk = ((self.chk & 0x7_ffff_ffff) << 5) ^ value;
        for (i, g) in GENERATOR.iter().enumerate() {
            if (top >> i) & 1 == 1 {
                self.chk ^= g;
            }
        }
    }

    /// Returns `false` if `c` is not in the descriptor charset.
    fn input(&mut self, c: u8) -> bool {
        let Some(pos) = INPUT_CHARSET.iter().position(|&x| x == c) else {
            return false;
        };
        let v = pos as u64;
        self.polymod(v & 31);
        self.groups[self.group_len] = v >> 5;
        self.group_len += 1;
        if self.group_len == 3 {
            self.polymod(self.groups[0] * 9 + self.groups[1] * 3 + self.groups[2]);
            self.group_len = 0;
        }
        true
    }

    fn finish_groups(&mut self) {
        match self.group_len {
            1 => self.polymod(self.groups[0]),
            2 => self.polymod(self.groups[0] * 3 + self.groups[1]),
            _ => {}
        }
        self.group_len = 0;
    }
}

/// The checksum for `descriptor` (which must not already carry one), as
/// eight ASCII characters, or `None` if it contains a character outside
/// the descriptor charset.
pub fn descriptor_checksum(descriptor: &str) -> Option<[u8; 8]> {
    let mut eng = Engine::new();
    for &c in descriptor.as_bytes() {
        if !eng.input(c) {
            return None;
        }
    }
    eng.finish_groups();
    for _ in 0..8 {
        eng.polymod(0);
    }
    let chk = eng.chk ^ 1;
    let mut out = [0u8; 8];
    for (i, o) in out.iter_mut().enumerate() {
        *o = CHECKSUM_CHARSET[((chk >> (5 * (7 - i))) & 31) as usize];
    }
    Some(out)
}

/// Whether `descriptor` ends in `#` plus a checksum that matches the text
/// before it.
pub fn verify_checksum(descriptor: &str) -> bool {
    let bytes = descriptor.as_bytes();
    if bytes.len() < 9 || bytes[bytes.len() - 9] != b'#' {
        return false;
    }
    let (body, checksum) = descriptor.split_at(bytes.len() - 9);
    let checksum = &checksum[1..];
    if !checksum.bytes().all(|c| CHECKSUM_CHARSET.contains(&c)) {
        return false;
    }
    descriptor_checksum(body).is_some_and(|expected| expected == *checksum.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bip380_examples() {
        assert_eq!(descriptor_checksum("raw(deadbeef)"), Some(*b"89f8spxm"));
        assert!(verify_checksum("raw(deadbeef)#89f8spxm"));
        assert!(!verify_checksum("raw(deadbeef)#89f8spxmx"), "too long");
        assert!(!verify_checksum("raw(deadbeef)#89f8spx"), "too short");
        assert!(!verify_checksum("raw(deedbeef)#89f8spxm"), "payload error");
        assert!(!verify_checksum("raw(deadbeef)#89f8spxn"), "checksum error");
        assert_eq!(descriptor_checksum("raw(\u{dc})"), None, "non-charset char");
        assert!(!verify_checksum("raw(\u{dc})#00000000"));
        assert!(
            !verify_checksum("raw(deadbeef)#89f8spxb"),
            "b is not bech32"
        );
        assert!(!verify_checksum(""));
        assert!(!verify_checksum("#89f8spxm"));
    }
}
