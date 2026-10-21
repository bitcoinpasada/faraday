//! Public key texts and records, derived from a master key: the account
//! keys a wallet's descriptor names, and the BSMS record a signer writes.

use faraday_bip::bitcoin::bip32::{DerivationPath, Xpub};
use faraday_bip::keys::{Fingerprint, MasterKey, MultisigScriptType, ScriptType};
use faraday_crypto::work::Compute;

/// An account key: public, with the origin it is derived at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountKey {
    /// The extended public key.
    pub xpub: Xpub,
    /// The master key's fingerprint.
    pub fingerprint: Fingerprint,
    /// The path below the master key.
    pub path: DerivationPath,
}

/// Costs a key derivation. The account key of `master` for a single-key
/// wallet of `script`, at `account`.
pub fn single_account(
    c: &Compute,
    master: &MasterKey,
    script: ScriptType,
    account: u32,
) -> Result<AccountKey, String> {
    let a = master
        .account_xpub(c, script, account)
        .map_err(|e| e.to_string())?;
    Ok(AccountKey {
        xpub: *a.xpub(),
        fingerprint: a.master_fingerprint(),
        path: a.path().clone(),
    })
}

/// Costs a key derivation. The account key of `master` for a multisig
/// cosigner of `script`, at `account`.
pub fn multisig_account(
    c: &Compute,
    master: &MasterKey,
    script: MultisigScriptType,
    account: u32,
) -> Result<AccountKey, String> {
    let a = master
        .multisig_account_xpub(c, script, account)
        .map_err(|e| e.to_string())?;
    Ok(AccountKey {
        xpub: *a.xpub(),
        fingerprint: a.master_fingerprint(),
        path: a.path().clone(),
    })
}

/// Costs a key derivation. The key of `master` at any `path`.
pub fn account_at(c: &Compute, master: &MasterKey, path: &DerivationPath) -> AccountKey {
    let d = master.derive(c, path);
    AccountKey {
        xpub: d.to_xpub(c),
        fingerprint: master.fingerprint(),
        path: path.clone(),
    }
}

/// Costs a key derivation and a signature. A key's BIP 129 key record,
/// signed by the account key itself, for the multisig script `script` at
/// `account`: the record's text.
pub fn bsms_record(
    c: &Compute,
    master: &MasterKey,
    script: MultisigScriptType,
    account: u32,
    description: &str,
) -> Option<String> {
    let record = master
        .with_multisig_account_secret(c, script, account, |secret, view| {
            faraday_bip::bsms::signer_record(master.secp(), view, secret, "00", description)
        })
        .ok()?
        .ok()?;
    Some(record.to_text())
}
