//! The standard accounts of a loaded key that has no wallet made from it,
//! searched beside the loaded wallets by the address check (`docs/QR.md`
//! §2, `docs/WALLETS.md`): account 0 of BIP-84, BIP-86, BIP-49 and BIP-44,
//! on the network in use.

use faraday_bip::keys::Fingerprint;
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;
use faraday_text::Text;

use crate::create::NewKind;
use crate::wallet::{Session, same_wallet};

/// The single-key kinds searched, in BIP number order of preference:
/// native SegWit first, as most wallets are.
const KINDS: [NewKind; 4] = [
    NewKind::NativeSegwit,
    NewKind::Taproot,
    NewKind::NestedSegwit,
    NewKind::Legacy,
];

/// One standard account of a loaded key, as a search target.
pub(crate) struct Account {
    /// The key's name.
    pub(crate) label: Text,
    /// The key's fingerprint.
    pub(crate) fingerprint: Fingerprint,
    /// The account's path, `m/84'/0'/0'`.
    pub(crate) path: Text,
    /// The script kind's name, as the New wallet screen writes it.
    pub(crate) script: &'static str,
    /// The single-key wallet the account is.
    pub(crate) policy: WalletPolicy,
}

/// Costs a key derivation the first time for each account of each loaded
/// key (kept on the key after), and a descriptor parse for each. The
/// standard accounts of the session's keys that no loaded wallet is the
/// same as: worked out once for a check, never while drawing.
pub(crate) fn unmade(c: &Compute, session: &Session) -> Vec<Account> {
    let net = session.network();
    let made: Vec<Text> = session
        .wallets
        .iter()
        .map(|w| same_wallet(w.policy()))
        .collect();
    let mut out = Vec::new();
    for (i, info) in session.keys.all().enumerate() {
        for kind in KINDS {
            let Ok(spec) = kind.spec(net, 0) else {
                continue;
            };
            let Some(Ok(key)) = session.keys.key_text(c, i, &spec) else {
                continue;
            };
            let descriptor = kind.descriptor(1, &[key.as_str()]);
            let Ok(policy) = WalletPolicy::parse_any(c, descriptor.as_str()) else {
                continue;
            };
            if made.contains(&same_wallet(&policy)) {
                continue;
            }
            out.push(Account {
                label: info.label.clone(),
                fingerprint: info.fingerprint,
                path: kind.path(net),
                script: kind.name(),
                policy,
            });
        }
    }
    out
}
