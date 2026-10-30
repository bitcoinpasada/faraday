//! Silent payments (BIP-352): a key's scan and spend halves.

use faraday_bip::bitcoin::Transaction;
use faraday_bip::bitcoin::secp256k1::{PublicKey, Secp256k1};
use faraday_bip::keys::{MasterKey, Network};
use faraday_bip::silent as bip352;
use faraday_crypto::work::Compute;
use zeroize::Zeroizing;

use crate::{Held, Part, Reveal};

pub use faraday_bip::silent::Payment;

/// A key's scan and spend keys at one account. It holds the scan private
/// key, so it is neither `Clone` nor `Debug` and gives out public values;
/// the scan key leaves only through [`Receiver::scan_key`], for a reason
/// that names it.
pub struct Receiver(bip352::Receiver);

/// Costs two key derivations. The scan and spend keys of `master`, at
/// `account`.
pub(crate) fn receiver(c: &Compute, master: &MasterKey, account: u32) -> Receiver {
    Receiver(bip352::Receiver::derive(c, master, account))
}

impl Receiver {
    /// The network the keys are for.
    pub fn network(&self) -> Network {
        self.0.network()
    }

    /// Costs an elliptic-curve multiplication. The scan public key.
    pub fn scan_public_key(&self) -> PublicKey {
        self.0.scan_public_key(&Secp256k1::new())
    }

    /// The spend public key.
    pub fn spend_public_key(&self) -> PublicKey {
        self.0.spend_public_key()
    }

    /// Costs an address (point additions). The `sp1…` address, unlabelled
    /// for `label` 0 and labelled otherwise; `None` when the address cannot
    /// be made.
    pub fn address(&self, label: u32) -> Option<String> {
        let secp = Secp256k1::new();
        let a = if label == 0 {
            self.0.address(&secp)
        } else {
            self.0.labelled_address(&secp, label)
        };
        a.ok().map(|t| t.as_str().to_string())
    }

    /// The scan key as text, BIP-392's `spscan1q…`, for `why`: into a
    /// vault record (`Reveal::VaultEntry`) or a file to the Outbox
    /// (`Reveal::OutboxFile`), of `Part::ScanKey`. `None` for another
    /// reason.
    pub fn scan_key(&self, why: Reveal) -> Option<Held<Zeroizing<String>>> {
        if !matches!(
            why,
            Reveal::VaultEntry(Part::ScanKey) | Reveal::OutboxFile(Part::ScanKey)
        ) {
            return None;
        }
        let k = self.0.scan_key_text().ok()?;
        Some(Held::of(Zeroizing::new(k.as_str().to_string())))
    }

    /// The scan key as BIP-392's descriptor, `sp([fp/352h/coinh/accounth]spscan1q…)`,
    /// with the origin of the key it came from, for the same reasons as
    /// [`Receiver::scan_key`]: it carries the scan key. `None` for another
    /// reason.
    pub fn descriptor(
        &self,
        why: Reveal,
        fingerprint: faraday_bip::keys::Fingerprint,
        account: u32,
    ) -> Option<Held<Zeroizing<String>>> {
        if !matches!(
            why,
            Reveal::VaultEntry(Part::ScanKey) | Reveal::OutboxFile(Part::ScanKey)
        ) {
            return None;
        }
        let d = self.0.descriptor(Some((fingerprint, account))).ok()?;
        Some(Held::of(Zeroizing::new(d.as_str().to_string())))
    }

    /// Costs a scan of the transaction's outputs. The payments `tx` makes
    /// to these keys under `labels`, its inputs' scripts being `scripts`.
    pub fn payments_in(
        &self,
        tx: &Transaction,
        scripts: &[&[u8]],
        labels: &[u32],
    ) -> Result<Vec<Payment>, bip352::Error> {
        let secp = Secp256k1::new();
        bip352::find_payments(&secp, &self.0, tx, scripts, labels)
    }
}
