//! Specter DIY's wallet command, `addwallet {name}&{descriptor}`, which
//! Sparrow writes for it and Specter DIY scans (`docs/INTEROP.md` §5 I8):
//!
//! ```text
//! addwallet Sparrow Multisig&wsh(sortedmulti(2,[7fd1bbf4/48h/0h/0h/2h]xpub…,…))
//! ```
//!
//! Read as Specter DIY reads it (`Wallet.parse` in its
//! `src/apps/wallets/wallet.py`): the name is everything before the last
//! `&`, the descriptor everything after it; a descriptor none of whose
//! keys carries a derivation is the wallet's two chains, `/<0;1>/*` after
//! every key. Faraday is stricter than Specter DIY in one place: a
//! checksum, where the descriptor has one, is checked (Specter DIY drops
//! it unread). The descriptor itself goes to the reader every descriptor
//! goes to ([`crate::wallet::read_wallet`]).

use faraday_bip::policy::{DESCRIPTOR_SUFFIX, Template, WalletPolicy};

/// The command word Specter DIY reads a wallet after.
const PREFIX: &str = "addwallet ";

/// The name and the descriptor of an `addwallet` command, when `text` is
/// one: the name as written (it may be empty), the descriptor trimmed.
pub fn split(text: &str) -> Option<(&str, &str)> {
    let rest = text
        .trim_start_matches('\u{feff}')
        .trim()
        .strip_prefix(PREFIX)?;
    let (name, descriptor) = rest.rsplit_once('&')?;
    Some((name, descriptor.trim()))
}

/// The wallet's name an `addwallet` command carries, as one line of
/// text ([`crate::wallet::wallet_name`]); `None` when `text` is no such
/// command or its name is blank.
pub fn name(text: &str) -> Option<String> {
    let (name, _) = split(text)?;
    let name = crate::wallet::wallet_name(name);
    (!name.is_empty()).then(|| name.into_owned())
}

/// Whether [`command`] writes the wallet: every wallet with a descriptor
/// but a MuSig2 or silent payments one, which Specter DIY does not hold.
pub fn writes(policy: &WalletPolicy) -> bool {
    !matches!(policy.template(), Template::MuSig | Template::Silent)
}

/// The wallet as Specter DIY's `addwallet` command, as Sparrow writes it:
/// a multisig or single-key wallet's keys without their derivation, which
/// Specter DIY reads as the two chains, and no checksum; any other
/// wallet's descriptor as Faraday writes it, `/<0;1>/*` and all, which
/// Specter DIY reads too. The name is written as one line with no `&`
/// lost: Specter DIY splits at the last one. `None` for a wallet
/// Specter DIY cannot hold ([`writes`]).
pub fn command(name: &str, policy: &WalletPolicy) -> Option<String> {
    if !writes(policy) {
        return None;
    }
    let descriptor = policy.to_descriptor();
    let descriptor = match policy.template() {
        Template::Multi { .. } | Template::Single { .. } => {
            descriptor.replace(DESCRIPTOR_SUFFIX, "")
        }
        _ => descriptor,
    };
    Some(format!(
        "{PREFIX}{}&{descriptor}",
        crate::wallet::wallet_name(name)
    ))
}
