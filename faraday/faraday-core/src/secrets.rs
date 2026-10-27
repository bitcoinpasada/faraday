//! What leaves this device, by who may read it (`docs/FLOWS.md`, Public
//! and secret). Public files go to the Outbox as they are. A secret goes
//! into the open vault, sealed under its passphrase, and leaves only
//! inside the vault file. Only when the person has read what the secret
//! gives away and said so does one go to the Outbox unprotected, where
//! the Outbox lists it apart, under Unprotected secrets.
//!
//! What goes into a vault depends on the secret. Most go in as a note.
//! A wallet's seed, from the backup's checklist,
//! goes in as the key record Vaults saves (`kind::KEY`, loaded at
//! unlock), never as a note; as a file it is the words or the SeedQR as
//! a labelled PNG, one of the two, picked on the sheet, and never holds
//! a BIP-39 passphrase.

use crate::strings::EN;
use crate::strings::fill;
use faraday_vault::Record;
use faraday_vault::records::{field, kind};

use crate::wallet::FileKind;
use crate::{Action, Faraday, Sheet};

/// Who may read a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exposure {
    /// Anyone: public keys, PSBTs, signed messages, certificates.
    Public,
    /// Only who has the passphrase: a vault or a KDBX database.
    Sealed,
    /// Anyone who has the file can use the secret in it.
    Secret,
}

impl FileKind {
    /// Who may read a file of this kind.
    pub fn exposure(self) -> Exposure {
        match self {
            FileKind::Vault | FileKind::Kdbx => Exposure::Sealed,
            FileKind::Entries | FileKind::Words | FileKind::SeedPart | FileKind::SecretWallet => {
                Exposure::Secret
            }
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

/// The secret sheet's area: a secret waiting for the person to say where
/// it goes, and whether they have read what it gives away.
#[derive(Default)]
pub(crate) struct SecretsArea {
    out: Option<SecretOut>,
    ack: bool,
}

impl SecretsArea {
    /// The secret waiting, if one is.
    pub(crate) fn out(&self) -> Option<&SecretOut> {
        self.out.as_ref()
    }

    /// The person has read what that secret gives away.
    pub(crate) fn ack(&self) -> bool {
        self.ack
    }

    /// Asks where a secret goes: a vault, or, after the warning, the
    /// Outbox.
    pub(crate) fn offer(&mut self, out: SecretOut, sheet: &mut Option<Sheet>) {
        self.out = Some(out);
        self.ack = false;
        *sheet = Some(Sheet::SecretOut);
    }

    /// Takes the secret away, to be put somewhere or dropped.
    pub(crate) fn take(&mut self) -> Option<SecretOut> {
        self.out.take()
    }

    /// Drops the secret, its bytes wiped.
    pub(crate) fn clear(&mut self) {
        self.out = None;
    }

    /// Ticks or unticks the warning.
    pub(crate) fn toggle_ack(&mut self) {
        self.ack = !self.ack;
    }

    /// The file's form `i`, picked on the sheet.
    pub(crate) fn pick_form(&mut self, i: usize) {
        if let Some(out) = self.out.as_mut()
            && let Some(f) = out.forms.get(i)
        {
            out.name = f.name.clone();
            out.bytes = f.bytes.clone();
            out.form = i;
        }
    }
}

impl Faraday {
    /// Asks where a secret goes: a vault, or, after the warning, the
    /// Outbox.
    pub(crate) fn offer_secret(&mut self, out: SecretOut) {
        self.secrets.offer(out, &mut self.sheet);
    }

    /// The seed shown on the backup's checklist, offered as a secret:
    /// into a vault as its key record, or as a file, its words or its
    /// SeedQR in the form picked on the step, after the warning.
    pub(crate) fn offer_seed(&mut self) {
        let c = self.compute();
        use faraday_bip::bip39::Language;
        let Some(b) = self.backup.state() else {
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
            EN.secrets_seed_warning.to_string()
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

    /// The secret into the open vault; its public half, if it has one, to
    /// the Outbox.
    pub(crate) fn secret_to_vault(&mut self) {
        let Some(v) = self.vaults.open.get(self.vaults.current) else {
            self.toast(EN.secrets_no_vault_open);
            return;
        };
        let vault = v.name.clone();
        let Some(out) = self.secrets.take() else {
            return;
        };
        let record = match &out.keep {
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
            // It did not fit: the secret is dropped, its bytes wiped.
            drop(out);
            return;
        }
        self.sheet = None;
        match out.keep {
            Keep::Key { key, .. } => {
                let label = self
                    .session
                    .keys
                    .info(key)
                    .map(|k| k.label.clone())
                    .unwrap_or_default();
                self.toast(&fill(
                    EN.secrets_saved_to_vault,
                    &[&label.to_string(), &vault.to_string()],
                ));
            }
            Keep::Note => self.toast(&fill(
                EN.secrets_saved_to_vault,
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
        if !self.secrets.ack() {
            return;
        }
        let Some(out) = self.secrets.take() else {
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
            self.backup.files_warned(fp);
            self.wallet_lock_write();
            return;
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

    /// Nothing chosen: the secret is dropped, its bytes wiped.
    pub(crate) fn secret_cancel(&mut self) {
        drop(self.secrets.take());
    }

    /// Costs a PSBT parse. The PSBT Inbox item `index` holds, to sign;
    /// `None`, after a toast saying why, when it holds none.
    pub(crate) fn spend_source(
        &mut self,
        c: &faraday_crypto::work::Compute,
        index: usize,
    ) -> Option<faraday_psbt::Psbt> {
        let item = self.inbox.get(index)?;
        let Some(psbt) = crate::wallet::read_psbt(c, &item.bytes) else {
            self.toast(EN.start_spend_not_a_psbt);
            return None;
        };
        Some(psbt)
    }
}

impl Faraday {
    /// Handles the secret sheet's actions (`Secret*`).
    pub(crate) fn secret_act(&mut self, action: Action) {
        match action {
            Action::SecretVault => self.secret_to_vault(),
            Action::SecretAck => self.secrets.toggle_ack(),
            Action::SecretUnprotected => self.secret_unprotected(),
            Action::SecretForm(i) => self.secrets.pick_form(usize::from(i)),
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}
