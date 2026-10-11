//! The names a person gives the wallets on Home.
//!
//! A wallet is named by its shape — "2 of 3 · SegWit", "Miniscript ·
//! SegWit", "Taproot tree" — and two wallets of one shape read alike.
//! A name replaces that label on Home and states itself on the wallet's
//! page; the page title stays the shape or the fingerprint, so what the
//! wallet is is never only a name someone typed.
//!
//! A name is public data: it is kept with the wallets in the blob on a
//! device that keeps keys, and it never appears in an export.
//!
//! A wallet over one key is named by the key's fingerprint, because the
//! key is what it is; a policy is named by its descriptor checksum,
//! which no two wallets in use share.

use alloc::string::String;
use alloc::vec::Vec;

use osk_bip::keys::Fingerprint;
use osk_bip::policy::WalletPolicy;

/// Characters a name takes. A name is a label on one row, so it is
/// short enough to be read whole beside the glyph on a 268 dp panel.
pub const MAX_NAME: usize = 24;

/// The names given this session.
#[derive(Debug, Clone, Default)]
pub struct WalletNames {
    keys: Vec<(Fingerprint, String)>,
    policies: Vec<(String, String)>,
}

impl WalletNames {
    /// No names.
    pub fn new() -> Self {
        WalletNames::default()
    }

    /// Forgets every name.
    pub fn clear(&mut self) {
        self.keys.clear();
        self.policies.clear();
    }

    /// The name of the wallet over the key `fp`.
    pub fn key_name(&self, fp: Fingerprint) -> Option<&str> {
        self.keys
            .iter()
            .find(|(k, _)| *k == fp)
            .map(|(_, n)| n.as_str())
    }

    /// The name of `policy`.
    pub fn policy_name(&self, policy: &WalletPolicy) -> Option<&str> {
        self.by_checksum(&policy.checksum())
    }

    /// The name kept under a checksum.
    pub fn by_checksum(&self, checksum: &str) -> Option<&str> {
        self.policies
            .iter()
            .find(|(c, _)| c == checksum)
            .map(|(_, n)| n.as_str())
    }

    /// Names the wallet over the key `fp`; an empty name clears it.
    pub fn set_key_name(&mut self, fp: Fingerprint, name: &str) {
        let name = tidy(name);
        self.keys.retain(|(k, _)| *k != fp);
        if !name.is_empty() {
            self.keys.push((fp, name));
        }
    }

    /// Names `policy`; an empty name clears it.
    pub fn set_policy_name(&mut self, policy: &WalletPolicy, name: &str) {
        self.set_checksum_name(&policy.checksum(), name);
    }

    /// The same, under a checksum already computed.
    pub fn set_checksum_name(&mut self, checksum: &str, name: &str) {
        let name = tidy(name);
        self.policies.retain(|(c, _)| c != checksum);
        if !name.is_empty() {
            self.policies.push((String::from(checksum), name));
        }
    }

    /// The named keys, for the record kept on the device.
    pub fn key_names(&self) -> impl Iterator<Item = (Fingerprint, &str)> {
        self.keys.iter().map(|(k, n)| (*k, n.as_str()))
    }
}

/// A name as it is kept: the spaces at its ends gone and no more than
/// [`MAX_NAME`] characters.
pub fn tidy(name: &str) -> String {
    name.trim().chars().take(MAX_NAME).collect()
}
