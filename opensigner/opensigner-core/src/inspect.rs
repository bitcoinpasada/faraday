//! What a scanned descriptor or extended public key says, apart from
//! how it is drawn (`docs/DESIGN.md` §5 Record).
//!
//! The document itself, and the three readings its rows are built from:
//! which script type a descriptor's function names, the master
//! fingerprint of every key origin in it, and the chain an extended
//! public key belongs to. The strings here are descriptor notation, not
//! wording, so they stay out of `strings/en.rs`.

use alloc::string::String;
use alloc::vec::Vec;

use osk_bip::keys::ScriptType;
use osk_bip::policy::WalletPolicy;

/// A read-only document for something scanned: a descriptor, an extended
/// public key, or bytes treated as text.
pub(crate) struct InspectDoc {
    pub(crate) title: &'static str,
    pub(crate) text: String,
    /// Whether the text is a descriptor, which is compared as structure
    /// rather than chunked in fours (§4.5).
    pub(crate) descriptor: bool,
    /// Whether a descriptor's checksum holds; `None` when it carries
    /// none.
    pub(crate) checksum: Option<bool>,
    /// The wallet the text describes, when it is a BIP-388 policy or a
    /// descriptor one can be built from. The review then offers to use
    /// it (`docs/PLANNING.md` §15 item 8).
    pub(crate) policy: Option<WalletPolicy>,
    /// The name the wallet arrived with: a coordinator config's `Name:`
    /// line, or an export's own. It becomes the wallet's name when the
    /// wallet is put in use.
    pub(crate) name: Option<String>,
    /// Whether the review was reached from the wallet's own menu, where
    /// Forget is a row of its own, so the document carries Done instead.
    pub(crate) from_menu: bool,
    /// What the review found when the wallet is nearly one this device
    /// already has, or when the one key claims a master this device
    /// holds (UX.md E2).
    pub(crate) swap: Option<Swap>,
    /// What a BIP 129 descriptor record states beside the wallet: the
    /// chains it restricts derivation to, its first receive address,
    /// and which key of the list is this device's.
    pub(crate) bsms: Option<Bsms>,
}

/// The rows BIP 129 asks every signer to check before a wallet is
/// registered.
pub(crate) struct Bsms {
    /// The derivation paths the record restricts the wallet to.
    pub(crate) paths: String,
    /// The wallet's first receive address, as the record states it.
    pub(crate) first_address: String,
    /// Which key of the wallet is this device's, by its place in the
    /// list, or `None` when this device holds none of them.
    pub(crate) ours: Option<usize>,
}

/// A reviewed wallet that is one key away from a wallet in use, or a
/// single key that claims a master this device holds and is not the key
/// that master derives.
///
/// A coordinator can hand back the wallet that was set up with one
/// cosigner's key replaced by the attacker's. Every fingerprint a person
/// recognises is still there when the origins are kept and only the xpub
/// is swapped, so the review says which key changed.
pub(crate) struct Swap {
    /// Which key of the reviewed wallet differs: the row that takes the
    /// caution tone.
    pub(crate) key: usize,
    /// The card's title: what was compared.
    pub(crate) title: &'static str,
    /// What the card states under its title: the wallet in use, and the
    /// key that changed.
    pub(crate) value: String,
}

/// The script type a descriptor's function names, in the one vocabulary
/// §4.6 allows.
pub(crate) fn descriptor_script(text: &str) -> Option<ScriptType> {
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

/// The master fingerprint of every key origin a descriptor carries,
/// `[73c5da0a/84h/0h/0h]` giving `73c5da0a`.
pub(crate) fn descriptor_origins(text: &str) -> Vec<String> {
    osk_ui::descriptor::tokens(text)
        .into_iter()
        .filter(|t| t.kind == osk_ui::descriptor::Kind::Origin)
        .map(|t| {
            t.full
                .trim_start_matches('[')
                .chars()
                .take_while(|c| *c != '/' && *c != ']')
                .collect()
        })
        .collect()
}

/// The chain an extended public key belongs to, by its version bytes.
pub(crate) fn xpub_network(text: &str) -> Option<&'static str> {
    use osk_bip::bitcoin::NetworkKind;
    use osk_bip::keys::Network;
    let kind = osk_bip::xkey::decode_xpub(text)
        .map(|x| x.network)
        .or_else(|_| osk_bip::slip132::decode_xpub(text).map(|(x, _)| x.network))
        .ok()?;
    Some(match kind {
        NetworkKind::Main => Network::Mainnet.name(),
        NetworkKind::Test => Network::Testnet.name(),
    })
}
