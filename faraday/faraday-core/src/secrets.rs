//! What leaves this device, by who may read it (`docs/FLOWS.md`, Public
//! and secret). Public files go to the Outbox as they are. A secret goes
//! into the open vault, sealed under its passphrase, and leaves only
//! inside the vault file. Only when the person has read what the secret
//! gives away and said so does one go to the Outbox unprotected, where
//! the Outbox lists it apart, under Unprotected secrets.
//!
//! What goes into a vault depends on the secret. Most go in as a note.
//! A FROST spend's carry file, the secret nonce the next share signs
//! with, goes in as a signing-round record (`docs/VAULT.md` §7, type 9),
//! and the public PSBT with this device's nonce goes to the Outbox; the
//! next device, with the vault unlocked, finds the round by the
//! transaction when it opens that PSBT, and the round is removed from
//! the vault once used. A wallet's seed, from the backup's checklist,
//! goes in as the key record Vaults saves (`kind::KEY`, loaded at
//! unlock), never as a note; as a file it is the words or the SeedQR as
//! a labelled PNG, one of the two, picked on the sheet, and never holds
//! a BIP-39 passphrase.

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
    pub what: String,
    /// What someone holding it can do.
    pub gives: String,
    /// What it becomes in a vault.
    pub keep: Keep,
    /// The forms the file can take, when it can take more than one: the
    /// person picks one on the sheet, and `name` and `bytes` are the one
    /// picked. Empty for a secret of one form.
    pub forms: Vec<SecretFile>,
    /// The form picked, by index into `forms`.
    pub form: usize,
    /// The line the person ticks before it may go out unprotected.
    pub ack: &'static str,
}

/// What a secret becomes in a vault.
pub enum Keep {
    /// A note holding the file's text.
    Note,
    /// A FROST signing round.
    Round(Round),
    /// Session key `key`, whose fingerprint is `fp`, as the key record
    /// Vaults' Save writes (`VaultAction::SaveKey`).
    Key {
        /// Its index in the session.
        key: usize,
        /// Its fingerprint, checked before it is saved.
        fp: osk_bip::keys::Fingerprint,
    },
}

/// One form a secret's file can take.
pub struct SecretFile {
    /// The form, as the sheet's button names it.
    pub label: &'static str,
    /// The file name.
    pub name: String,
    /// The file.
    pub bytes: zeroize::Zeroizing<Vec<u8>>,
}

/// The acknowledgement every secret but a seed asks for.
pub const ACK: &str = "Anyone who copies the stick or sees the code can read it";

/// A seed's acknowledgement.
pub const SEED_ACK: &str =
    "I understand: anyone who copies the stick or sees this file can spend these coins";

impl SecretOut {
    /// A secret of one form that goes into a vault as a note.
    pub fn new(name: String, bytes: zeroize::Zeroizing<Vec<u8>>, what: &str, gives: &str) -> Self {
        Self {
            name,
            bytes,
            what: what.to_string(),
            gives: gives.to_string(),
            keep: Keep::Note,
            forms: Vec::new(),
            form: 0,
            ack: ACK,
        }
    }
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

    /// The seed shown on the backup's checklist, offered as a secret:
    /// into a vault as its key record, or as a file, its words or its
    /// SeedQR in the form picked on the step, after the warning.
    pub(crate) fn offer_seed(&mut self) {
        use osk_bip::bip39::{Language, Mnemonic};
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let (k, compact) = (b.key, b.compact);
        let (Some(key), Some(wallet)) =
            (self.session.keys.get(k), self.session.wallets.get(b.wallet))
        else {
            return;
        };
        let Some(words) = key.words.as_ref() else {
            return;
        };
        let Ok(m) = Mnemonic::parse(key.language, words) else {
            return;
        };
        let fp = key.master.fingerprint();
        let fps = crate::wallet::fp_text(fp);
        let stem = format!("{}-{}", crate::file_stem(&wallet.name), fps.to_lowercase());
        // The words, one line, as Add a key and other signers read them.
        // Sized first so that no copy is left behind by a reallocation.
        let mut text = Vec::with_capacity(words.len() + 1);
        text.extend_from_slice(words.as_bytes());
        text.push(b'\n');
        let mut forms = vec![SecretFile {
            label: "Words",
            name: format!("{stem}-words.txt"),
            bytes: zeroize::Zeroizing::new(text),
        }];
        // SeedQR numbers the English list's words only.
        if key.language == Language::English {
            let matrix = if compact {
                osk_codec::seedqr::encode_compact(&m)
            } else {
                osk_codec::seedqr::encode_seedqr(&m)
            };
            if let Ok(mx) = matrix {
                let title = format!("Seed {fps}");
                let line = format!("{} words · secret: spends", m.word_count());
                let png =
                    zeroize::Zeroizing::new(crate::picture::labelled_png(&mx, &title, &[line]));
                let (label, name) = if compact {
                    ("Compact SeedQR PNG", format!("{stem}-compactseedqr.png"))
                } else {
                    ("SeedQR PNG", format!("{stem}-seedqr.png"))
                };
                forms.push(SecretFile {
                    label,
                    name,
                    bytes: png,
                });
            }
        }
        let (need, _) = crate::wallet::Session::quorum(wallet);
        let gives = if need <= 1 {
            "Whoever has it spends this wallet's coins".to_string()
        } else {
            let more = need - 1;
            format!(
                "Whoever has it signs as key {fps}; with {more} more {}, spends",
                if more == 1 { "key" } else { "keys" }
            )
        };
        let first = &forms[0];
        let out = SecretOut {
            name: first.name.clone(),
            bytes: first.bytes.clone(),
            what: "A wallet's seed".to_string(),
            gives,
            keep: Keep::Key { key: k, fp },
            forms,
            form: 0,
            ack: SEED_ACK,
        };
        self.offer_secret(out);
    }

    /// The file's form `i`, picked on the sheet.
    pub(crate) fn secret_form(&mut self, i: usize) {
        if let Some(out) = self.secret_out.as_mut()
            && let Some(f) = out.forms.get(i)
        {
            out.name = f.name.clone();
            out.bytes = f.bytes.clone();
            out.form = i;
        }
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
        let record = match &out.keep {
            Keep::Round(r) => Record::new(kind::ROUND)
                .with(field::ROUND_SCHEME, &[2])
                .with(field::ROUND_TX, &r.tx)
                .with(field::ROUND_KEY, &r.key)
                .with(field::ROUND_STATE, &out.bytes),
            Keep::Note => Record::new(kind::NOTE).with(field::NOTE, &out.bytes),
            Keep::Key { key, fp } => {
                let same = self
                    .session
                    .keys
                    .get(*key)
                    .is_some_and(|k| k.master.fingerprint() == *fp);
                match same.then(|| self.key_record(*key, false)).flatten() {
                    Some((r, _)) => r,
                    None => {
                        self.sheet = None;
                        return;
                    }
                }
            }
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
        if matches!(out.keep, Keep::Round(_))
            && let Some(s) = self.spend.as_mut()
        {
            s.out_signed = true;
        }
        match out.keep {
            Keep::Round(r) => {
                self.put_outbox(&r.psbt_name, r.psbt);
                self.toast(&format!(
                    "The nonce is in {vault}; {} is in the Outbox",
                    r.psbt_name
                ));
            }
            Keep::Key { key, .. } => {
                let label = self
                    .session
                    .keys
                    .get(key)
                    .map(|k| k.label.clone())
                    .unwrap_or_default();
                self.toast(&format!("{label} is in {vault}"));
            }
            Keep::Note => self.toast(&format!("{} is in {vault}", out.name)),
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
        if matches!(out.keep, Keep::Round(_))
            && let Some(s) = self.spend.as_mut()
        {
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

    /// Nothing chosen: a carry file goes back to the spend it came from;
    /// any other secret is dropped, its bytes wiped.
    pub(crate) fn secret_cancel(&mut self) {
        if let Some(out) = self.secret_out.take() {
            self.secret_back(out);
        }
    }

    fn secret_back(&mut self, out: SecretOut) {
        if matches!(out.keep, Keep::Round(_))
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
