//! The standard accounts of a loaded key that has no wallet made from it,
//! searched beside the loaded wallets by the address check (`docs/QR.md`
//! §2, `docs/WALLETS.md`): account 0 of BIP-84, BIP-86, BIP-49 and BIP-44,
//! on the network in use.

use faraday_bip::bitcoin::bip32::DerivationPath;
use faraday_bip::keys::{Fingerprint, ScriptType};
use faraday_bip::policy::{Template, WalletPolicy};
use faraday_crypto::work::Compute;
use faraday_text::Text;

use crate::create::NewKind;
use crate::wallet::Session;

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
    /// The account's path as a derivation path, to make its wallet at.
    pub(crate) account: DerivationPath,
    /// The script the account's addresses are of.
    pub(crate) script_type: ScriptType,
    /// The account's number.
    pub(crate) number: u32,
    /// The single-key wallet the account is.
    pub(crate) policy: WalletPolicy,
}

/// Whether two single-key wallets are of one account: the same master
/// fingerprint, the same account path, the same extended key and the same
/// script, read from the parsed descriptors, so one written with `h` for
/// `'` or with another checksum is the same one. A wallet whose key has
/// no origin, and a wallet of more than one key, is of no account; a
/// wallet naming the key's origin over another extended key is not the
/// key's account, so that account is still searched.
pub(crate) fn same_account(a: &WalletPolicy, b: &WalletPolicy) -> bool {
    let shape = |p: &WalletPolicy| {
        let [key] = p.keys() else {
            return None;
        };
        if !matches!(p.template(), Template::Single { .. }) {
            return None;
        }
        Some((
            key.fingerprint()?,
            key.path()?.clone(),
            *key.xpub(),
            p.script_type(),
        ))
    };
    match (shape(a), shape(b)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

/// Costs a key derivation the first time for each account of each loaded
/// key (kept on the key after), and a descriptor parse for each. The
/// standard accounts of the session's keys that no loaded wallet is the
/// same as: worked out once for a check, never while drawing.
pub(crate) fn unmade(c: &Compute, session: &Session) -> Vec<Account> {
    let net = session.network();
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
            if session
                .wallets
                .iter()
                .any(|w| same_account(w.policy(), &policy))
            {
                continue;
            }
            let Ok(account) = kind.path(net).parse::<DerivationPath>() else {
                continue;
            };
            out.push(Account {
                label: info.label.clone(),
                fingerprint: info.fingerprint,
                account,
                script_type: policy.script_type(),
                number: 0,
                policy,
            });
        }
    }
    out
}
