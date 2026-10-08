//! What a scanned descriptor or extended public key says, apart from
//! how it is drawn (`docs/DESIGN.md` §5 Record).
//!
//! The document itself. The three readings its rows are built from are
//! in `osk-bip` (`docs/PLANNING.md` §16.139): the script type a
//! descriptor's function names (`descriptor::script_type`), the master
//! fingerprint of every key origin in it
//! (`descriptor::origin_fingerprints`), and the chain an extended public
//! key belongs to (`slip132::network_kind`).

use alloc::string::String;

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
