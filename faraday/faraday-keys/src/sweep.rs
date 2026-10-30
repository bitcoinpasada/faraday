//! A private key swept (`docs/INTEROP.md` §5 I11, F171): a WIF read
//! from a scan or typed, held in memory for one sweep and never kept.
//!
//! [`SweepKey`] holds the key ([`faraday_bip::wif::SingleKey`]) and what
//! the sweep shows of it, worked out once when it is read: the
//! addresses it pays, legacy first, then native and nested SegWit for a
//! compressed key. It has no reveal: the key leaves only as signatures
//! ([`crate::Keys::sweep_sign`]). The store holds at most one
//! ([`crate::Keys::sweep_read`]) and drops it, erasing it, at
//! [`crate::Keys::sweep_forget`] and with the session's keys.

use faraday_bip::bitcoin::{Address, NetworkKind};
use faraday_bip::keys::{Network, ScriptType};
use faraday_bip::wif::SingleKey;
use faraday_crypto::work::Compute;

/// Why a WIF is not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The text is not a WIF private key.
    NotAKey,
    /// A WIF for the other kind of network: a mainnet key on a test
    /// network, or a test network's key on mainnet.
    Network {
        /// Whether the key is mainnet's.
        mainnet: bool,
    },
    /// A stick is attached: no key joins while one is
    /// (`docs/SECURITY-MODEL.md`).
    StickAttached,
}

/// The key being swept, with the addresses it pays.
pub struct SweepKey {
    key: SingleKey,
    /// Each address the key pays, with its script type: legacy, then for
    /// a compressed key native SegWit and nested SegWit.
    addresses: Vec<(ScriptType, String)>,
}

impl SweepKey {
    /// Costs what `SingleKey::decode` costs and an address encoding per
    /// script. The WIF in `text` (surrounding white space dropped) read
    /// for a store on `network`.
    pub(crate) fn read(c: &Compute, text: &str, network: Network) -> Result<Self, Refusal> {
        let key = SingleKey::decode(c, text.trim()).map_err(|_| Refusal::NotAKey)?;
        let mainnet = key.network() == NetworkKind::Main;
        if mainnet != (network == Network::Mainnet) {
            return Err(Refusal::Network { mainnet });
        }
        let kinds = [
            ScriptType::Legacy,
            ScriptType::NativeSegwit,
            ScriptType::NestedSegwit,
        ];
        let addresses = kinds
            .into_iter()
            .zip(key.scripts())
            .filter_map(|(kind, script)| {
                Address::from_script(&script, faraday_bip::bitcoin::Network::from(network))
                    .ok()
                    .map(|a| (kind, a.to_string()))
            })
            .collect();
        Ok(SweepKey { key, addresses })
    }

    /// The addresses the key pays, each with its script type: legacy
    /// first, then native and nested SegWit for a compressed key. Worked
    /// out when the key was read; a lookup.
    pub fn addresses(&self) -> &[(ScriptType, String)] {
        &self.addresses
    }

    /// The address the sweep names the key by: its legacy one, which
    /// every WIF pays and a paper wallet prints beside it. A lookup.
    pub fn name(&self) -> &str {
        self.addresses.first().map_or("", |(_, a)| a.as_str())
    }

    /// Whether the WIF states a compressed public key; an uncompressed
    /// one pays a legacy address only.
    pub fn compressed(&self) -> bool {
        self.key.compressed()
    }

    /// The inputs of `psbt` that spend one of the key's scripts, not
    /// finalized. A lookup over the inputs.
    pub fn inputs_of(&self, psbt: &faraday_psbt::Psbt) -> Vec<usize> {
        faraday_psbt::sweep::inputs_of(psbt, &self.key)
    }

    /// The key itself, for the store's signer and blinding.
    pub(crate) fn key_mut(&mut self) -> &mut SingleKey {
        &mut self.key
    }

    /// The key itself, for the store's signer.
    pub(crate) fn key(&self) -> &SingleKey {
        &self.key
    }
}
