//! What leaves this device, by who may read it (`docs/FLOWS.md`, Public
//! and secret). Public files go to the Outbox as they are. A secret goes
//! into the open vault, sealed under its passphrase, and leaves only
//! inside the vault file. Only once the person has held **Hold to save
//! without encryption** on the sheet that says it goes out as a plain
//! file does one go to the Outbox unprotected, where the Outbox lists it
//! apart, under Unprotected secrets (F129). The same hold comes before a
//! backup plan keeps a secret as a plain file on a stick at a place, and
//! before Transfer's Receive saves a secret.
//!
//! What goes into a vault depends on the secret. Most go in as a note.
//! A wallet's seed, from the backup's checklist,
//! goes in as the key record Vaults saves (`kind::KEY`, loaded at
//! unlock), never as a note; as a file it is the words or the SeedQR as
//! a labelled PNG, one of the two, picked on the sheet, and never holds
//! a BIP-39 passphrase.

use crate::strings::EN;
use faraday_text::{Text, text};
use faraday_vault::Record;
use faraday_vault::records::{field, kind};

use crate::wallet::FileKind;
use crate::{Action, Faraday, Sheet};

/// Who may read a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exposure {
    /// Anyone: public keys, PSBTs, signed messages, certificates.
    Public,
    /// Only who has the passphrase: a vault, a QR vault or a KDBX database.
    Sealed,
    /// Anyone who has the file can use the secret in it.
    Secret,
}

impl FileKind {
    /// Who may read a file of this kind.
    pub fn exposure(self) -> Exposure {
        match self {
            FileKind::Vault | FileKind::Kdbx | FileKind::KdbxOther | FileKind::QrVault => {
                Exposure::Sealed
            }
            FileKind::Entries
            | FileKind::Words
            | FileKind::SeedPart
            | FileKind::SecretWallet
            | FileKind::CoreWallet => Exposure::Secret,
            _ => Exposure::Public,
        }
    }

    /// A file of this kind may hold a secret: the secret kinds, and text
    /// or a file Faraday does not read, which it cannot tell. A lock drops
    /// these from the Inbox (`docs/VAULT.md`).
    pub(crate) fn may_be_secret(self) -> bool {
        matches!(
            self,
            FileKind::Words
                | FileKind::SeedPart
                | FileKind::SecretWallet
                | FileKind::Entries
                | FileKind::CoreWallet
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
    /// A vault's key file Faraday made at Create.
    KeyFile,
}

/// A secret on its way out, waiting for the person to say where.
pub struct SecretOut {
    /// The file name it would go out under.
    pub name: Text,
    /// The file.
    pub bytes: zeroize::Zeroizing<Vec<u8>>,
    /// What it is, as the sheet names it.
    pub what: Text,
    /// What someone holding it can do.
    pub gives: Text,
    /// What it becomes in a vault.
    pub keep: Keep,
    /// The forms the file can take, when it can take more than one: the
    /// person picks one on the sheet, and `name` and `bytes` are the one
    /// picked. Empty for a secret of one form.
    pub forms: Vec<SecretFile>,
    /// The form picked, by index into `forms`.
    pub form: usize,
    /// What it is to Faraday: a seed's, another secret, or text Faraday
    /// cannot tell about (the sheet's head says it may be a secret).
    pub ack: Ack,
    /// An Inbox file a stick visit would write to another stick: past the
    /// warning it is ticked on the visit, and goes nowhere else.
    pub to_visit: bool,
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
        /// The fields (codex32 set, BIP-85 origin) the person unticked on
        /// the sheet: they hold for this secret's save and end with it.
        left: Vec<u8>,
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
    pub name: Text,
    /// The file.
    pub bytes: zeroize::Zeroizing<Vec<u8>>,
}

/// What kind of secret a sheet offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ack {
    /// Every secret but a seed's.
    Any,
    /// A seed's.
    Seed,
    /// Text or a file Faraday does not read: it cannot tell.
    Unknown,
}

impl SecretOut {
    /// A secret of one form that goes into a vault as a note.
    pub fn new(
        name: impl AsRef<str>,
        bytes: zeroize::Zeroizing<Vec<u8>>,
        what: &str,
        gives: &str,
    ) -> Self {
        Self {
            name: Text::of(name.as_ref()),
            bytes,
            what: Text::of(what),
            gives: Text::of(gives),
            keep: Keep::Note,
            forms: Vec::new(),
            form: 0,
            ack: Ack::Any,
            to_visit: false,
        }
    }
}

/// A secret about to be saved as a plain file, outside any vault: the
/// **Save without encryption?** sheet's, confirmed only by its hold
/// (F129).
pub(crate) struct Plain {
    /// What goes: a file's name, or several.
    pub name: Text,
    /// Where it goes, as the sheet's first line names it.
    pub on: Text,
    /// What keeps it, as the sheet's second line names it.
    pub keep: Text,
    /// What the hold does.
    pub then: PlainThen,
}

/// What a held **Hold to save without encryption** does.
pub(crate) enum PlainThen {
    /// The secret sheet's secret goes out unprotected.
    Secret,
    /// Back up's plan keeps these files on a stick at a place: the plan
    /// as it was before, the files' names and their seeds' fingerprints,
    /// marked past the warning.
    Plan(Box<crate::plan::Answers>, Vec<Text>, Vec<Text>),
    /// **Lock and write** makes this file of Back up's plan on a stick at
    /// a place: held for, the lock goes on (or asks for the next file).
    Lock(Text),
    /// Transfer's Receive saves this file into the folder.
    Transfer(Text, zeroize::Zeroizing<Vec<u8>>),
    /// A vault record's **Save to a stick** writes its ticked files, a
    /// key's words or SeedQR among them.
    VaultSave,
}

/// The secret sheet's area: a secret waiting for the person to say where
/// it goes, and the hold that sends one out as a plain file.
#[derive(Default)]
pub(crate) struct SecretsArea {
    out: Option<SecretOut>,
    plain: Option<Plain>,
}

impl SecretsArea {
    /// The secret waiting, if one is.
    pub(crate) fn out(&self) -> Option<&SecretOut> {
        self.out.as_ref()
    }

    /// Unticks field `number` of the waiting key's record, or ticks it
    /// again.
    pub(crate) fn toggle_left_field(&mut self, number: u8) {
        if let Some(out) = self.out.as_mut()
            && let Keep::Key { left, .. } = &mut out.keep
        {
            if left.contains(&number) {
                left.retain(|n| *n != number);
            } else {
                left.push(number);
            }
        }
    }

    /// The secret about to be saved as a plain file, waiting for its
    /// hold.
    pub(crate) fn plain(&self) -> Option<&Plain> {
        self.plain.as_ref()
    }

    /// Asks where a secret goes: a vault, or, after the warning, the
    /// Outbox.
    pub(crate) fn offer(&mut self, out: SecretOut, sheet: &mut Option<Sheet>) {
        self.out = Some(out);
        self.plain = None;
        *sheet = Some(Sheet::SecretOut);
    }

    /// Takes the secret away, to be put somewhere or dropped.
    pub(crate) fn take(&mut self) -> Option<SecretOut> {
        self.out.take()
    }

    /// Drops the secret, its bytes wiped.
    pub(crate) fn clear(&mut self) {
        self.out = None;
        self.plain = None;
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
        let fps = Text::of(&crate::wallet::fp_text(fp));
        let name = crate::file_stem(&wallet.name);
        let stem = text!("{}-{}", name.as_str(), &*Text::lowercase(&fps));
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
            name: text!("{}-words.txt", &*stem),
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
            let title = Text::fill(EN.secrets_seed, &[&fps]);
            let line = Text::fill(EN.secrets_words_secret, &[&Text::num(word_count as u64)]);
            let png =
                zeroize::Zeroizing::new(crate::picture::labelled_png(&c, &mx, &title, &[line]));
            let (label, name) = if compact {
                (
                    EN.secrets_compact_seedqr,
                    text!("{}-compactseedqr.png", &*stem),
                )
            } else {
                (EN.secrets_seedqr_png, text!("{}-seedqr.png", &*stem))
            };
            forms.push(SecretFile {
                label,
                name,
                bytes: png,
            });
        }
        let (need, _) = crate::wallet::Session::quorum(wallet);
        let gives = if need <= 1 {
            Text::of(EN.secrets_seed_warning)
        } else {
            let more = need - 1;
            Text::fill(
                EN.secrets_whoever_has,
                &[
                    &fps,
                    &Text::num(more as u64),
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
            what: Text::of(EN.secrets_a_wallet_s),
            gives,
            keep: Keep::Key {
                key: k,
                fp,
                left: Vec::new(),
            },
            forms,
            form: 0,
            ack: Ack::Seed,
            to_visit: false,
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
        self.secrets.plain = None;
        let Some(out) = self.secrets.take() else {
            return;
        };
        let record = match &out.keep {
            Keep::Note => faraday_keys::vault::Staged::Made(
                Record::new(kind::NOTE).with(field::NOTE, &out.bytes),
            ),
            Keep::Key { key, fp, left } => {
                let same = self
                    .session
                    .keys
                    .info(*key)
                    .is_some_and(|k| k.fingerprint == *fp);
                match same.then(|| self.key_record(*key, false, left)).flatten() {
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
                self.toast(&Text::fill(EN.secrets_saved_to_vault, &[&*label, &vault]));
            }
            Keep::Note => self.toast(&Text::fill(EN.secrets_saved_to_vault, &[&out.name, &vault])),
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

    /// **Unprotected** on the secret sheet: the **Save without
    /// encryption?** sheet, whose hold sends the secret out (F129).
    pub(crate) fn secret_unprotected(&mut self) {
        let Some(out) = self.secrets.out() else {
            return;
        };
        let noun = self.medium.noun();
        let plain = Plain {
            name: out.name.clone(),
            on: Text::of(noun),
            keep: Text::of(noun),
            then: PlainThen::Secret,
        };
        self.plain_ask(plain);
    }

    /// Asks for the hold that saves a secret as a plain file.
    pub(crate) fn plain_ask(&mut self, plain: Plain) {
        self.secrets.plain = Some(plain);
        self.sheet = Some(Sheet::SecretOut);
        self.scroll.reset_sheet();
    }

    /// **Hold to save without encryption**, held to its end: what the
    /// sheet asked for is done.
    pub(crate) fn secret_hold(&mut self) {
        let Some(plain) = self.secrets.plain.take() else {
            return;
        };
        self.sheet = None;
        match plain.then {
            PlainThen::Secret => self.secret_unprotected_now(),
            PlainThen::Plan(before, names, fps) => {
                for f in names.into_iter().chain(fps) {
                    self.backup.files_warned(Text::of(&f));
                }
                self.board_sees_ask(*before);
            }
            PlainThen::Lock(name) => {
                self.backup.files_warned(Text::of(&name));
                self.wallet_lock_write();
            }
            PlainThen::Transfer(name, bytes) => self.transfer_save(name, bytes),
            PlainThen::VaultSave => self.vault_stick_save_write(),
        }
    }

    /// **Save** on a vault record's Save to a stick: a key's words or
    /// SeedQR ticked asks **Save without encryption?**'s hold first
    /// (F129); public files alone are written at once.
    pub(crate) fn vault_stick_save_ask(&mut self) {
        let names: Vec<Text> = self
            .vaults
            .stick_save
            .as_ref()
            .map(|s| {
                s.files
                    .iter()
                    .filter(|f| f.ticked && f.what != crate::vault::out::SaveWhat::Public)
                    .flat_map(|f| f.names.iter().map(|n| Text::of(n)))
                    .collect()
            })
            .unwrap_or_default();
        if names.is_empty() {
            return self.vault_stick_save_write();
        }
        let noun = self.medium.noun();
        self.plain_ask(Plain {
            name: Text::joined(&names, ", "),
            on: Text::of(noun),
            keep: Text::of(noun),
            then: PlainThen::VaultSave,
        });
    }

    /// Costs what `secret_hold`, `register_act` and `power_act` cost. A
    /// hold held to its end, from the tick: the secret's, the register
    /// sheet's, or a lock's or power's.
    pub(crate) fn hold_done(&mut self, held: Action, c: &faraday_crypto::work::Compute) {
        match held {
            Action::SecretHold => self.secret_hold(),
            Action::RegisterHold => self.register_act(held),
            Action::Sweep(a) => self.sweep_act(a),
            Action::ReviewHold => self.wallet_act(held),
            Action::DevicesHold => self.devices_act(held),
            Action::NameTakenOverwrite => self.visit_act(held),
            _ => self.power_act(held, c),
        }
    }

    /// Costs two maps of the plan. Back up's plan changed by `f`. Where
    /// the change keeps a secret as a plain file on a stick at a place
    /// that place did not keep before, and the hold has not been held for
    /// that file, **Save without encryption?** asks for it, once per file,
    /// with the plan as it was for **Cancel** (F129).
    pub(crate) fn backup_change_plain(
        &mut self,
        shape: &crate::plan::Shape,
        f: &dyn Fn(&mut crate::plan::Answers, &crate::plan::Shape),
    ) {
        let c = self.compute();
        let Some(was) = self.backup.state().map(|b| b.answers.clone()) else {
            return;
        };
        self.backup.change_answers(shape, f);
        let Some(b) = self.backup.state() else {
            return;
        };
        let (w, now) = (b.wallet, b.answers.clone());
        let had = plain_files(&crate::plan::map(&c, shape, &was));
        let added: Vec<(usize, crate::plan::What)> =
            plain_files(&crate::plan::map(&c, shape, &now))
                .into_iter()
                .filter(|x| !had.contains(x))
                .collect();
        let mut names: Vec<Text> = Vec::new();
        let mut fps: Vec<Text> = Vec::new();
        for &(_, what) in &added {
            for n in self.stick_file_names_now(w, shape, &now, what) {
                let n = Text::of(&n);
                if !b.files_warned.iter().any(|x| x.as_str() == n.as_str()) && !names.contains(&n) {
                    names.push(n);
                }
            }
            if let crate::plan::What::SeedFile(i)
            | crate::plan::What::SeedQrFile(i)
            | crate::plan::What::PassphraseFile(i) = what
                && let Some(seed) = shape.seeds.get(i)
            {
                fps.push(Text::of(&seed.name));
            }
        }
        let Some(&(p, _)) = added.first().filter(|_| !names.is_empty()) else {
            return;
        };
        // A second file asked about before the first is answered joins it:
        // **Cancel** goes back to the plan before either.
        let before = match self.secrets.plain.take() {
            Some(Plain {
                then: PlainThen::Plan(before, n, f),
                ..
            }) => {
                names.splice(0..0, n);
                fps.extend(f);
                before
            }
            _ => Box::new(was),
        };
        let noun = self.medium.noun();
        let on = Text::fill(EN.plain_stick_at, &[noun, &self.board_place_name(p)]);
        self.plain_ask(Plain {
            name: Text::joined(&names, ", "),
            on,
            keep: Text::of(noun),
            then: PlainThen::Plan(before, names, fps),
        });
    }

    /// Costs a map of the plan. The first file Back up's plan keeps on a
    /// stick at a place that holds a secret as a plain file (a seed's
    /// words, SeedQR or passphrase, a codex32 string) and that has not been
    /// held for, at a drop or at an earlier ask: **Lock and write** asks for
    /// its hold, one file at a time, before anything is made (F129). A file
    /// from a preset or a plan saved before is asked here. Computes from
    /// the state as it is.
    pub(crate) fn backup_lock_plain(&self, c: &faraday_crypto::work::Compute) -> Option<Plain> {
        let b = self.backup.state()?;
        let shape = self.plan_shape_now(b.wallet);
        for (p, what) in plain_files(&crate::plan::map(c, &shape, &b.answers)) {
            let names = self.stick_file_names_now(b.wallet, &shape, &b.answers, what);
            if let Some(n) = names
                .into_iter()
                .find(|n| !b.files_warned.iter().any(|x| x.as_str() == n.as_str()))
            {
                let noun = self.medium.noun();
                return Some(Plain {
                    name: n.clone(),
                    on: Text::fill(EN.plain_stick_at, &[noun, &self.board_place_name(p)]),
                    keep: Text::of(noun),
                    then: PlainThen::Lock(n),
                });
            }
        }
        None
    }

    /// Back up's plan as it was before the drop **Cancel** answered.
    fn board_undo(&mut self, before: crate::plan::Answers) {
        let Some(w) = self.backup.state().map(|b| b.wallet) else {
            return;
        };
        let shape = self.plan_shape_now(w);
        self.backup
            .change_answers(&shape, &|a, _| *a = before.clone());
    }

    /// The secret to the Outbox unprotected, past its hold.
    fn secret_unprotected_now(&mut self) {
        let c = self.compute();
        let Some(out) = self.secrets.take() else {
            return;
        };
        self.sheet = None;
        // From the Inbox to another stick: ticked on the visit, written
        // from the Inbox when the person presses Write.
        if out.to_visit {
            self.visit.inbox_pick(&out.name);
            return;
        }
        let name = out.name.clone();
        self.outbox.retain(|i| i.name != *name);
        let item = crate::Item::new(&c, &name, out.bytes.to_vec());
        self.outbox_secret(OutReason::Unprotected, item);
        self.save_boxes();
        self.toast(&Text::fill(
            EN.secrets_waits_unprotected,
            &[&name, self.medium.for_the()],
        ));
    }

    /// Nothing chosen: the secret is dropped, its bytes wiped. A plan's
    /// file asked about leaves the plan, which is as it was before; a
    /// received file is not saved.
    pub(crate) fn secret_cancel(&mut self) {
        match self.secrets.plain.take().map(|p| p.then) {
            Some(PlainThen::Plan(before, ..)) => self.board_undo(*before),
            Some(PlainThen::Transfer(..)) => self.transfer_skip(),
            Some(PlainThen::VaultSave) => self.sheet = Some(Sheet::VaultSave),
            Some(PlainThen::Secret | PlainThen::Lock(_)) | None => {}
        }
        drop(self.secrets.take());
    }
}

impl Faraday {
    /// Handles the secret sheet's actions (`Secret*`).
    pub(crate) fn secret_act(&mut self, action: Action) {
        match action {
            Action::SecretVault => self.secret_to_vault(),
            Action::SecretHold => self.secret_hold(),
            Action::SecretUnprotected => self.secret_unprotected(),
            Action::SecretForm(i) => self.secrets.pick_form(usize::from(i)),
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}

/// The secrets a map keeps as plain files on a stick at a place, by
/// place: a seed's words, SeedQR or passphrase, a codex32 string.
fn plain_files(m: &[crate::plan::Spot]) -> Vec<(usize, crate::plan::What)> {
    m.iter()
        .filter_map(|s| match s.at {
            crate::plan::At::Stick(p, _) => Some((p, s)),
            _ => None,
        })
        .flat_map(|(p, s)| s.holds.iter().map(move |(w, _)| (p, *w)))
        .filter(|(_, w)| crate::backup_flow::wallet_files::is_secret_file(*w))
        .collect()
}
