//! The BIP-380 output descriptor checksum, and the facts a reader checks
//! in a descriptor someone else wrote.
//!
//! Descriptors themselves are produced by [`crate::account::AccountXpub`];
//! this module gives the 8-character BCH checksum that follows the
//! `#`, worked out by miniscript, and reads a
//! descriptor's text for its checksum verdict ([`checksum_facts`]), its
//! script type ([`script_type`]) and the masters its keys come from
//! ([`origin_fingerprints`]).
//!
//! ```
//! use faraday_bip::descriptor::{descriptor_checksum, verify_checksum};
//!
//! assert_eq!(descriptor_checksum("raw(deadbeef)"), Some(*b"89f8spxm"));
//! assert!(verify_checksum("raw(deadbeef)#89f8spxm"));
//! assert!(!verify_checksum("raw(deedbeef)#89f8spxm"));
//! ```

use alloc::string::String;
use alloc::vec::Vec;

use faraday_crypto::work::{Compute, Work};

use crate::keys::ScriptType;
use crate::policy::WalletPolicy;

/// The checksum for `descriptor` (which must not already carry one), as
/// eight ASCII characters, or `None` if it contains a character outside
/// the descriptor charset. It is miniscript's, which works on the text and
/// so serves BIP-392's `sp(..)` descriptors too.
pub fn descriptor_checksum(descriptor: &str) -> Option<[u8; 8]> {
    let sum = miniscript::descriptor::checksum::desc_checksum(descriptor).ok()?;
    sum.as_bytes().try_into().ok()
}

/// Whether `descriptor` ends in `#` plus a checksum that matches the text
/// before it.
pub fn verify_checksum(descriptor: &str) -> bool {
    let bytes = descriptor.as_bytes();
    if bytes.len() < 9 || bytes[bytes.len() - 9] != b'#' {
        return false;
    }
    let Some((body, checksum)) = descriptor.split_at_checked(bytes.len() - 9) else {
        return false;
    };
    descriptor_checksum(body).is_some_and(|expected| expected == checksum.as_bytes()[1..])
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

/// Costs a checksum and a parse of the descriptor: the checksum of
/// `text`, or `None` when it is empty or holds a character no descriptor
/// may.
pub fn checksum_facts(c: &Compute, text: &str) -> Option<ChecksumFacts> {
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
        wallet: WalletPolicy::parse_any(c, body).ok(),
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

/// Whether the part of one key text that reaches the base58 decoder is
/// short enough to decode: what follows a leading `[origin]`, up to the
/// first `/`. `miniscript` gives that part to the decoder, which is quadratic
/// in its valid start and sizes its scratch from the whole part, so an
/// outsized part costs far more than its length whatever it holds. The limit
/// is well over the longest thing a descriptor's text holds there (`{` and
/// wrappers, which the depth limit keeps to some hundreds of characters; an
/// uncompressed key in hex, 130) and small enough to decode at once. A path
/// never reaches the decoder, so it has no limit here.
pub(crate) fn token_fits(leaf: &str) -> bool {
    const LONGEST: usize = 1024;
    let key = leaf
        .strip_prefix('[')
        .and_then(|rest| rest.split_once(']'))
        .map_or(leaf, |(_, key)| key);
    key.split_once('/').map_or(key, |(head, _)| head).len() <= LONGEST
}

/// [`token_fits`] for every leaf of a descriptor's text, split where
/// `miniscript`'s tree reader splits it outside a `tr()` tree; a tree's `{`
/// and `}` count as part of a leaf, since in a key they are not divisions.
/// Every route that gives a whole descriptor to `miniscript` checks this
/// first, on the text it gives.
pub(crate) fn tokens_fit(text: &str) -> bool {
    text.split(['(', ')', ',']).all(token_fits)
}

/// Costs a parse of the descriptor: whether `text` reads as one output
/// descriptor of public keys, checksum and all, by `miniscript`'s own
/// reader, whatever wallet it is or is not.
///
/// `miniscript` has no `sortedmulti_a`: a `tr()` whose tree is one
/// `sortedmulti_a` leaf is read as its `multi_a` twin, as
/// [`WalletPolicy`] reads it, its `#checksum`, if present, checked first
/// over the text as written.
pub fn reads_as_descriptor(c: &Compute, text: &str) -> bool {
    use core::str::FromStr;
    c.count(Work::DescriptorParse);
    let read = |t: &str| {
        tokens_fit(t)
            && miniscript::Descriptor::<miniscript::DescriptorPublicKey>::from_str(t).is_ok()
    };
    let body = text.split_once('#').map_or(text, |(body, _)| body);
    match crate::tapmulti::unsorted_text(body) {
        Some(twin) => (body.len() == text.len() || verify_checksum(text)) && read(&twin),
        None => read(text),
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
