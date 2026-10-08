//! The BIP-380 output descriptor checksum, and the facts a reader checks
//! in a descriptor someone else wrote.
//!
//! Descriptors themselves are produced by [`crate::account::AccountXpub`];
//! this module implements the 8-character BCH checksum that follows the
//! `#`, ported from the reference Python in BIP-380, and reads a
//! descriptor's text for its checksum verdict ([`checksum_facts`]), its
//! script type ([`script_type`]) and the masters its keys come from
//! ([`origin_fingerprints`]).
//!
//! ```
//! use osk_bip::descriptor::{descriptor_checksum, verify_checksum};
//!
//! assert_eq!(descriptor_checksum("raw(deadbeef)"), Some(*b"89f8spxm"));
//! assert!(verify_checksum("raw(deadbeef)#89f8spxm"));
//! assert!(!verify_checksum("raw(deedbeef)#89f8spxm"));
//! ```

use alloc::string::String;
use alloc::vec::Vec;

use crate::keys::ScriptType;
use crate::policy::WalletPolicy;

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

/// What the checksum of a typed descriptor comes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecksumFacts {
    /// The descriptor with the checksum computed here.
    pub with_checksum: String,
    /// The checksum computed here, on its own.
    pub checksum: String,
    /// The checksum that arrived with the text, and whether it holds.
    pub given: Option<(String, bool)>,
    /// The wallet the descriptor is, where it is one
    /// [`WalletPolicy::parse_any`] reads.
    pub wallet: Option<WalletPolicy>,
}

/// The checksum of `text`, or `None` when it is empty or holds a
/// character no descriptor may.
pub fn checksum_facts(text: &str) -> Option<ChecksumFacts> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let (body, given) = match text.split_once('#') {
        Some((body, tail)) => (body, Some(String::from(tail))),
        None => (text, None),
    };
    let sum = descriptor_checksum(body)?;
    let sum = String::from_utf8(sum.to_vec()).ok()?;
    let with_checksum = alloc::format!("{body}#{sum}");
    Some(ChecksumFacts {
        given: given.map(|g| {
            let holds = g == sum;
            (g, holds)
        }),
        wallet: WalletPolicy::parse_any(body).ok(),
        checksum: sum,
        with_checksum,
    })
}

/// The script type a descriptor's outer function names: `tr(` and
/// `rawtr(` Taproot, `sh(wpkh(` and `sh(wsh(` nested Segwit, `wpkh(` and
/// `wsh(` native Segwit, `pkh(`, `sh(` and `pk(` legacy.
pub fn script_type(text: &str) -> Option<ScriptType> {
    let head: String = text
        .chars()
        .take_while(|c| *c != '[' && *c != ')')
        .collect();
    if head.starts_with("tr(") || head.starts_with("rawtr(") {
        Some(ScriptType::Taproot)
    } else if head.starts_with("sh(wpkh(") || head.starts_with("sh(wsh(") {
        Some(ScriptType::NestedSegwit)
    } else if head.starts_with("wpkh(") || head.starts_with("wsh(") {
        Some(ScriptType::NativeSegwit)
    } else if head.starts_with("pkh(") || head.starts_with("sh(") || head.starts_with("pk(") {
        Some(ScriptType::Legacy)
    } else {
        None
    }
}

/// The master fingerprint of every key origin a descriptor carries, in
/// the order it writes them: `[73c5da0a/84h/0h/0h]` gives `73c5da0a`.
pub fn origin_fingerprints(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let origin = &rest[open + 1..];
        let end = origin.find(']').unwrap_or(origin.len());
        out.push(origin[..end].chars().take_while(|c| *c != '/').collect());
        rest = &origin[end..];
    }
    out
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
