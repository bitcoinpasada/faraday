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

use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::bitcoin::hashes::{Hash, sha256};
use faraday_vault::Record;
use faraday_vault::records::{field, kind};

use crate::wallet::FileKind;
use crate::{Action, Faraday, Sheet};

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
            FileKind::Carry
            | FileKind::Entries
            | FileKind::Words
            | FileKind::SeedPart
            | FileKind::SecretWallet => Exposure::Secret,
            _ => Exposure::Public,
        }
    }

    /// A file of this kind may hold a secret: the secret kinds, and text
    /// or a file Faraday does not read, which it cannot tell. A lock drops
    /// these from the Inbox (`docs/history/PLAN.md` §5.3).
    pub fn may_be_secret(self) -> bool {
        matches!(
            self,
            FileKind::Words
                | FileKind::SeedPart
                | FileKind::SecretWallet
                | FileKind::Carry
                | FileKind::Entries
                | FileKind::Text
                | FileKind::Other
        )
    }
}

/// Why a secret file goes to the Outbox, named after the key boundary's
/// reveal list (docs/KEYS.md §9).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum OutReason {
    /// A file the person let out unprotected.
    Unprotected,
    /// The backup flow writing a key's words or SeedQR picture to a stick.
    StickBackup,
    /// A vault key's words or SeedQR picture saved to a stick.
    VaultStick,
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
    pub ack: Ack,
    /// An Inbox file a stick visit would write to another stick: past the
    /// warning it is ticked on the visit, and goes nowhere else.
    pub to_visit: bool,
    /// The warning **Lock and write** asks for before it makes a seed's
    /// files on a stick (`docs/BACKUP-FLOW.md` §17.12 item 6), for the
    /// seed of this fingerprint (as text): past it, the seed is let out
    /// and the lock goes on. Such a sheet holds no bytes.
    pub to_lock: Option<String>,
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
        fp: faraday_bip::keys::Fingerprint,
    },
    /// Nothing: a picture or a file Faraday does not read is not offered
    /// to a vault.
    None,
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

/// The acknowledgement a secret asks for before it goes out unprotected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ack {
    /// Every secret but a seed's.
    Any,
    /// A seed's.
    Seed,
    /// Text or a file Faraday does not read: it cannot tell.
    Unknown,
}

impl Ack {
    /// Its line, naming the medium the file is carried on.
    pub fn text(self, medium: crate::Medium) -> String {
        let noun = medium.noun();
        match self {
            Ack::Any => fill(EN.secrets_anyone_who, &[noun]),
            Ack::Seed => fill(EN.secrets_i_understand, &[noun]),
            Ack::Unknown => fill(EN.secrets_faraday_cannot, &[noun]),
        }
    }
}

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
            ack: Ack::Any,
            to_visit: false,
            to_lock: None,
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
pub fn tx_hash(psbt: &faraday_psbt::Psbt) -> [u8; 32] {
    let tx = faraday_bip::bitcoin::consensus::serialize(&psbt.inner().unsigned_tx);
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
        let c = self.compute();
        use faraday_bip::bip39::Language;
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let (k, compact) = (b.key, b.compact);
        let (Some(key), Some(wallet)) = (
            self.session.keys.info(k),
            self.session.wallets.get(b.wallet),
        ) else {
            return;
        };
        if !key.has_words() {
            return;
        }
        let fp = key.fingerprint;
        let fps = crate::wallet::fp_text(fp);
        let stem = format!("{}-{}", crate::file_stem(&wallet.name), fps.to_lowercase());
        // The words, one line, as Add a key and other signers read them.
        // Sized first so that no copy is left behind by a reallocation.
        let words = self.session.keys.reveal(
            k,
            faraday_keys::Reveal::OutboxFile(faraday_keys::Part::Words),
        );
        let mut text = Vec::with_capacity(words.len() + 1);
        text.extend_from_slice(words.as_bytes());
        text.push(b'\n');
        let mut forms = vec![SecretFile {
            label: EN.secrets_words,
            name: format!("{stem}-words.txt"),
            bytes: zeroize::Zeroizing::new(text),
        }];
        // SeedQR numbers the English list's words only, and a SeedQR
        // picture is offered only where the plan copies the seed as a
        // SeedQR (`docs/history/NEW-WALLET.md` §14.7).
        let seedqr = self
            .backup_seed_list_now(b.wallet)
            .iter()
            .position(|(_, i)| *i == Some(k))
            .is_none_or(|i| self.copy_forms(&c, b, i).seedqr);
        if key.language == Language::English
            && seedqr
            && let Some((mx, word_count)) = self.session.keys.seedqr_matrix(
                &c,
                k,
                compact,
                faraday_keys::Reveal::OutboxFile(faraday_keys::Part::SeedQr),
            )
        {
            let title = fill(EN.secrets_seed, &[&fps.to_string()]);
            let line = fill(EN.secrets_words_secret, &[&word_count.to_string()]);
            let png =
                zeroize::Zeroizing::new(crate::picture::labelled_png(&c, &mx, &title, &[line]));
            let (label, name) = if compact {
                (
                    EN.secrets_compact_seedqr,
                    format!("{stem}-compactseedqr.png"),
                )
            } else {
                (EN.secrets_seedqr_png, format!("{stem}-seedqr.png"))
            };
            forms.push(SecretFile {
                label,
                name,
                bytes: png,
            });
        }
        let (need, _) = crate::wallet::Session::quorum(wallet);
        let gives = if need <= 1 {
            EN.secrets_whoever_has_it.to_string()
        } else {
            let more = need - 1;
            fill(
                EN.secrets_whoever_has,
                &[
                    &fps.to_string(),
                    &more.to_string(),
                    (if more == 1 {
                        EN.secrets_key
                    } else {
                        EN.secrets_keys
                    }),
                ],
            )
        };
        let first = &forms[0];
        let out = SecretOut {
            name: first.name.clone(),
            bytes: first.bytes.clone(),
            what: EN.secrets_a_wallet_s.to_string(),
            gives,
            keep: Keep::Key { key: k, fp },
            forms,
            form: 0,
            ack: Ack::Seed,
            to_visit: false,
            to_lock: None,
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
            self.toast(EN.secrets_no_vault_is);
            return;
        };
        let vault = v.name.clone();
        let Some(out) = self.secret_out.take() else {
            return;
        };
        let record = match &out.keep {
            Keep::Round(r) => faraday_keys::vault::Staged::Made(
                Record::new(kind::ROUND)
                    .with(field::ROUND_SCHEME, &[2])
                    .with(field::ROUND_TX, &r.tx)
                    .with(field::ROUND_KEY, &r.key)
                    .with(field::ROUND_STATE, &out.bytes),
            ),
            Keep::Note => faraday_keys::vault::Staged::Made(
                Record::new(kind::NOTE).with(field::NOTE, &out.bytes),
            ),
            Keep::Key { key, fp } => {
                let same = self
                    .session
                    .keys
                    .info(*key)
                    .is_some_and(|k| k.fingerprint == *fp);
                match same.then(|| self.key_record(*key, false)).flatten() {
                    Some((r, _)) => faraday_keys::vault::Staged::Held(r),
                    None => {
                        self.sheet = None;
                        return;
                    }
                }
            }
            Keep::None => {
                self.sheet = None;
                return;
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
                self.toast(&fill(
                    EN.secrets_the_nonce_is,
                    &[
                        &vault.to_string(),
                        &r.psbt_name.to_string(),
                        self.medium.for_the(),
                    ],
                ));
            }
            Keep::Key { key, .. } => {
                let label = self
                    .session
                    .keys
                    .info(key)
                    .map(|k| k.label.clone())
                    .unwrap_or_default();
                self.toast(&fill(
                    EN.secrets_is_in,
                    &[&label.to_string(), &vault.to_string()],
                ));
            }
            Keep::Note => self.toast(&fill(
                EN.secrets_is_in,
                &[&out.name.to_string(), &vault.to_string()],
            )),
            Keep::None => {}
        }
    }

    /// The one way a secret file enters the Outbox: marks the item
    /// secret and puts it there. Cheap. The reason names which of the
    /// key boundary's reveals this is (docs/KEYS.md); it changes nothing
    /// the person sees.
    pub(crate) fn outbox_secret(&mut self, why: OutReason, mut item: crate::Item) {
        let _ = why;
        item.secret = true;
        self.outbox.push(item);
    }

    /// The secret to the Outbox unprotected, once the person has said
    /// they understand what it gives away.
    pub(crate) fn secret_unprotected(&mut self) {
        let c = self.compute();
        if !self.secret_ack {
            return;
        }
        let Some(out) = self.secret_out.take() else {
            return;
        };
        self.sheet = None;
        // From the Inbox to another stick: ticked on the visit, written
        // from the Inbox when the person presses Write.
        if out.to_visit {
            self.visit.from_inbox.insert(out.name.clone());
            return;
        }
        // The seed's files on a stick, let out: Lock and write goes on.
        if let Some(fp) = out.to_lock {
            if let Some(b) = self.backup.as_mut()
                && !b.files_warned.contains(&fp)
            {
                b.files_warned.push(fp);
            }
            self.wallet_lock_write();
            return;
        }
        if matches!(out.keep, Keep::Round(_))
            && let Some(s) = self.spend.as_mut()
        {
            s.out_signed = true;
        }
        let name = out.name.clone();
        self.outbox.retain(|i| i.name != name);
        let item = crate::Item::new(&c, &name, out.bytes.to_vec());
        self.outbox_secret(OutReason::Unprotected, item);
        self.save_boxes();
        self.toast(&fill(
            EN.secrets_waits_unprotected,
            &[&name.to_string(), self.medium.for_the()],
        ));
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
        psbt: &faraday_psbt::Psbt,
    ) -> Option<faraday_psbt::threshold::CarrySection> {
        // The round's state is read inside faraday-keys: the section it
        // holds goes to the spend, which signs with its nonces.
        self.vaults
            .store()
            .round_section(&tx_hash(psbt))
            .map(faraday_keys::Held::release)
    }

    /// A round has signed: it leaves every open vault at the next seal.
    pub(crate) fn vault_round_used(&mut self, psbt: &faraday_psbt::Psbt) {
        let tx = tx_hash(psbt);
        let ids: Vec<faraday_keys::vault::VaultId> =
            self.vaults.open.iter().map(|v| v.id).collect();
        for id in ids {
            let gone = self.vaults.edit(id, |s| {
                s.remove_where(id, |r| {
                    r.kind() == kind::ROUND && r.public(field::ROUND_TX) == Some(&tx[..])
                })
            });
            if gone > 0 {
                self.vaults.changed(id);
            }
        }
    }
}

impl Faraday {
    /// Handles the secret sheet's actions (`Secret*`).
    pub(crate) fn secret_act(&mut self, action: Action) {
        match action {
            Action::SecretVault => self.secret_to_vault(),
            Action::SecretAck => self.secret_ack = !self.secret_ack,
            Action::SecretUnprotected => self.secret_unprotected(),
            Action::SecretForm(i) => self.secret_form(usize::from(i)),
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}
