//! What leaves this device, by who may read it (`docs/FLOWS.md`, Public
//! and secret). Public files go to the Outbox as they are. A secret goes
//! into the open vault, sealed under its passphrase, and leaves only
//! inside the vault file. Only when the person has read what the secret
//! gives away and said so does one go to the Outbox unprotected, where
//! the Outbox lists it apart, under Unprotected secrets.
//!
//! The one secret Faraday makes to carry today is a FROST spend's carry
//! file: the secret nonce the next share signs with. In a vault it is a
//! signing-round record (`docs/VAULT.md` §7, type 9), and the public PSBT
//! with this device's nonce goes to the Outbox; the next device, with the
//! vault unlocked, finds the round by the transaction when it opens that
//! PSBT, and the round is removed from the vault once used.

use faraday_vault::Record;
use faraday_vault::records::{field, kind};
use osk_bip::bitcoin::hashes::{Hash, sha256};

use crate::wallet::FileKind;
use crate::{Faraday, Sheet};

/// Who may read a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exposure {
    /// Anyone: public keys, PSBTs, signed messages, certificates.
    Public,
    /// Only who has the passphrase: a vault or an encrypted backup.
    Sealed,
    /// Anyone who has the file can use the secret in it.
    Secret,
}

impl FileKind {
    /// Who may read a file of this kind.
    pub fn exposure(self) -> Exposure {
        match self {
            FileKind::Vault | FileKind::Backup | FileKind::Kdbx => Exposure::Sealed,
            FileKind::Carry | FileKind::Entries | FileKind::Words | FileKind::SeedPart => {
                Exposure::Secret
            }
            _ => Exposure::Public,
        }
    }
}

/// A secret on its way out, waiting for the person to say where.
pub struct SecretOut {
    /// The file name it would go out under.
    pub name: String,
    /// The file.
    pub bytes: zeroize::Zeroizing<Vec<u8>>,
    /// What it is, as the sheet names it.
    pub what: &'static str,
    /// What someone holding it can do.
    pub gives: &'static str,
    /// For a signing round: what binds it, for the vault record.
    pub round: Option<Round>,
}

/// A FROST round's binding, for its vault record.
pub struct Round {
    /// SHA-256 of the unsigned transaction.
    pub tx: [u8; 32],
    /// The fingerprint of the key that signed with it.
    pub key: [u8; 4],
    /// The public PSBT, with this device's nonce, that goes to the Outbox
    /// beside the vault.
    pub psbt: Vec<u8>,
    /// Its file name.
    pub psbt_name: String,
}

/// SHA-256 of a PSBT's unsigned transaction: what a round is bound to.
pub fn tx_hash(psbt: &osk_psbt::Psbt) -> [u8; 32] {
    let tx = osk_bip::bitcoin::consensus::serialize(&psbt.inner().unsigned_tx);
    sha256::Hash::hash(&tx).to_byte_array()
}

impl Faraday {
    /// Asks where a secret goes: a vault, or, after the warning, the
    /// Outbox.
    pub(crate) fn offer_secret(&mut self, out: SecretOut) {
        self.secret_out = Some(out);
        self.secret_ack = false;
        self.sheet = Some(Sheet::SecretOut);
    }

    /// The secret into the open vault; its public half, if it has one, to
    /// the Outbox.
    pub(crate) fn secret_to_vault(&mut self) {
        let Some(v) = self.vaults.open.get(self.vaults.current) else {
            self.toast("No vault is open");
            return;
        };
        let vault = v.name.clone();
        let Some(out) = self.secret_out.take() else {
            return;
        };
        let record = match &out.round {
            Some(r) => Record::new(kind::ROUND)
                .with(field::ROUND_SCHEME, &[2])
                .with(field::ROUND_TX, &r.tx)
                .with(field::ROUND_KEY, &r.key)
                .with(field::ROUND_STATE, &out.bytes),
            None => Record::new(kind::NOTE).with(field::NOTE, &out.bytes),
        };
        let before = self
            .vaults
            .open
            .get(self.vaults.current)
            .map_or(0, |v| v.changes);
        self.vault_push(record, "");
        let after = self
            .vaults
            .open
            .get(self.vaults.current)
            .map_or(0, |v| v.changes);
        if after == before {
            // It did not fit: the secret stays where it came from.
            self.secret_back(out);
            return;
        }
        self.sheet = None;
        if let Some(s) = self.spend.as_mut() {
            s.out_signed = true;
        }
        match out.round {
            Some(r) => {
                self.put_outbox(&r.psbt_name, r.psbt);
                self.toast(&format!(
                    "The nonce is in {vault}; {} is in the Outbox",
                    r.psbt_name
                ));
            }
            None => self.toast(&format!("{} is in {vault}", out.name)),
        }
    }

    /// The secret to the Outbox unprotected, once the person has said
    /// they understand what it gives away.
    pub(crate) fn secret_unprotected(&mut self) {
        if !self.secret_ack {
            return;
        }
        let Some(out) = self.secret_out.take() else {
            return;
        };
        self.sheet = None;
        if let Some(s) = self.spend.as_mut() {
            s.out_signed = true;
        }
        let name = out.name.clone();
        self.outbox.retain(|i| i.name != name);
        let mut item = crate::Item::new(&name, out.bytes.to_vec());
        item.secret = true;
        self.outbox.push(item);
        self.save_boxes();
        self.toast(&format!("{name} is in the Outbox, unprotected"));
    }

    /// Nothing chosen: a carry file goes back to the spend it came from.
    pub(crate) fn secret_cancel(&mut self) {
        if let Some(out) = self.secret_out.take() {
            self.secret_back(out);
        }
    }

    fn secret_back(&mut self, out: SecretOut) {
        if out.round.is_some()
            && let Some(s) = self.spend.as_mut()
            && s.carry_out.is_none()
        {
            s.carry_out = Some(out.bytes);
        }
    }

    /// The carry section a vault holds for this transaction, if an open
    /// vault has an unused round bound to it.
    pub(crate) fn vault_round(
        &self,
        psbt: &osk_psbt::Psbt,
    ) -> Option<osk_psbt::threshold::CarrySection> {
        let tx = tx_hash(psbt);
        for v in &self.vaults.open {
            for (_, r) in v.contents.of(kind::ROUND) {
                if r.field(field::ROUND_TX) == Some(&tx[..])
                    && r.field(field::ROUND_USED).is_none()
                    && let Some(state) = r.field(field::ROUND_STATE)
                    && let Ok(c) = osk_psbt::threshold::Carry::parse(state)
                {
                    return Some(c.section);
                }
            }
        }
        None
    }

    /// A round has signed: it leaves every open vault at the next seal.
    pub(crate) fn vault_round_used(&mut self, psbt: &osk_psbt::Psbt) {
        let tx = tx_hash(psbt);
        for v in &mut self.vaults.open {
            let before = v.contents.records.len();
            v.contents
                .records
                .retain(|r| !(r.kind == kind::ROUND && r.field(field::ROUND_TX) == Some(&tx[..])));
            if v.contents.records.len() != before {
                v.changes += 1;
            }
        }
    }
}
