//! Silent payments (BIP-352): a key's scan and spend halves.

use faraday_bip::bitcoin::Transaction;
use faraday_bip::bitcoin::secp256k1::Secp256k1;
use faraday_bip::keys::MasterKey;
use faraday_bip::silent as bip352;
use faraday_crypto::work::Compute;
use zeroize::Zeroizing;

pub use faraday_bip::silent::{Payment, Receiver};

/// Costs two key derivations. The scan and spend keys of `master`, at
/// `account`.
pub fn receiver(c: &Compute, master: &MasterKey, account: u32) -> Receiver {
    Receiver::derive(c, master, account)
}

/// Costs an address (point additions). The `sp1…` address of `receiver`,
/// unlabelled for `label` 0 and labelled otherwise; `None` when the
/// address cannot be made.
pub fn address(receiver: &Receiver, label: u32) -> Option<String> {
    let secp = Secp256k1::new();
    let a = if label == 0 {
        receiver.address(&secp)
    } else {
        receiver.labelled_address(&secp, label)
    };
    a.ok().map(|t| t.as_str().to_string())
}

/// The scan key as text, BIP-392's `spscan1q…`.
pub fn scan_key(receiver: &Receiver) -> Option<Zeroizing<String>> {
    let k = receiver.scan_key_text().ok()?;
    Some(Zeroizing::new(k.as_str().to_string()))
}

/// Costs a scan of the transaction's outputs. The payments `tx` makes to
/// `receiver` under `labels`, its inputs' scripts being `scripts`.
pub fn payments_in(
    receiver: &Receiver,
    tx: &Transaction,
    scripts: &[&[u8]],
    labels: &[u32],
) -> Result<Vec<Payment>, bip352::Error> {
    let secp = Secp256k1::new();
    bip352::find_payments(&secp, receiver, tx, scripts, labels)
}
