//! A vault's key and wallet, shown and saved (`docs/BACKUP-FLOW.md`
//! §19.7): a key record's SeedQR behind the hold that shows its words,
//! a wallet record's descriptor as the Wallets page shows it, and Save to
//! a stick for either, so that the picture and text forms need not be
//! kept in the vault. Nothing here is stored in the vault: what is made
//! goes to the Outbox, for the stick, as Files' files do.

use crate::strings::EN;
use crate::strings::fill;
use faraday_vault::records::{field, kind};

use crate::vaults::VaultAction;
use crate::{Action, Code, Faraday, Sheet};

/// What a file of the popup is made from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveWhat {
    /// The key's words, one line.
    Words,
    /// The key's SeedQR, as a labelled picture.
    SeedQr,
    /// A wallet's public files, already made.
    Public,
}

/// One line of the popup: a form, the files it is written as, ticked or
/// not.
pub struct SaveFile {
    /// The form, as the popup names it.
    pub label: String,
    /// What it is made from.
    pub what: SaveWhat,
    /// The files' names.
    pub names: Vec<String>,
    /// A wallet's files, made when the popup opened: public, so kept
    /// until it closes. Empty for a seed's, which are made at Save.
    pub made: Vec<(String, Vec<u8>)>,
    /// To be saved.
    pub ticked: bool,
}

/// Save to a stick's popup.
pub struct StickSave {
    /// The vault record it is for, by index into the open vault's records.
    pub record: usize,
    /// The record is a key: the popup says what the stick gives whoever
    /// finds it.
    pub seed: bool,
    /// Its lines.
    pub files: Vec<SaveFile>,
}

impl Faraday {
    /// The selected vault record, when it is of kind `k`.
    fn vault_record_of(&self, k: u8) -> Option<(usize, &faraday_vault::Record)> {
        let at = self.vault_selected_index()?;
        let v = self.vaults.open.get(self.vaults.current)?;
        v.contents
            .records
            .get(at)
            .filter(|r| r.kind == k)
            .map(|r| (at, r))
    }

    /// Makes a SeedQR when a press lands on Show SeedQR: the selected key
    /// record's code, kept while the press is held (`docs/SPEED.md`
    /// §3.3: drawing makes no code). Any other press drops one kept.
    pub(crate) fn vault_seedqr_press(&mut self, pressed: Option<Action>) {
        let c = self.compute();
        self.vaults.seedqr = None;
        if pressed != Some(Action::Vault(VaultAction::ShowSeedQr)) {
            return;
        }
        let Some((at, r)) = self.vault_record_of(kind::KEY) else {
            return;
        };
        let Some(m) = crate::wallet::record_words(&c, r) else {
            return;
        };
        if m.language() != osk_bip::bip39::Language::English {
            return;
        }
        if let Ok(mx) = osk_codec::seedqr::encode_seedqr(&c, &m) {
            self.vaults.seedqr = Some((at, mx));
        }
    }

    /// Runs `f` with the selected wallet record as a session wallet, by
    /// its index: the session's own when it holds the same wallet, else
    /// the record read in for `f` alone and taken out again, so the
    /// session is as it was. Reads the descriptor.
    fn with_vault_wallet<T>(&mut self, f: impl FnOnce(&mut Self, usize) -> T) -> Option<T> {
        let c = self.compute();
        let (_, r) = self.vault_record_of(kind::WALLET)?;
        let text = r.text(field::WALLET)?.to_string();
        let name = r
            .text(field::WALLET_NAME)
            .unwrap_or(EN.vaults_wallet)
            .to_string();
        let policy = crate::wallet::read_wallet(&c, &text).ok()?;
        let same = crate::wallet::same_wallet(&policy);
        if let Some(i) = self
            .session
            .wallets
            .iter()
            .position(|w| crate::wallet::same_wallet(w.policy()) == same)
        {
            return Some(f(self, i));
        }
        self.session.wallets.push(crate::wallet::Wallet::new(
            name,
            policy,
            EN.vaults_vault.to_string(),
            crate::wallet::Origin::Vault,
        ));
        let i = self.session.wallets.len() - 1;
        let out = f(self, i);
        self.session.wallets.truncate(i);
        Some(out)
    }

    /// Makes the selected wallet record's code and shows it: the QR view
    /// the Wallets page uses, several codes for a long descriptor.
    pub(crate) fn vault_wallet_qr(&mut self) {
        let c = self.compute();
        if let Some(view) = self.with_vault_wallet(|app, i| app.code_view(&c, Code::Descriptor(i)))
        {
            self.open_qr(view);
        }
    }

    /// Opens Save to a stick for the selected record, writing out the
    /// names of its files: for a key its words and its SeedQR, both
    /// ticked; for a wallet its descriptor, its QR pictures and the
    /// software files Back up makes, which are made now (QR pictures
    /// drawn), the descriptor alone ticked.
    pub(crate) fn vault_stick_save_open(&mut self) {
        let c = self.compute();
        let Some(at) = self.vault_selected_index() else {
            return;
        };
        let save = if self.vault_record_of(kind::KEY).is_some() {
            let Some((_, r)) = self.vault_record_of(kind::KEY) else {
                return;
            };
            let Some(m) = crate::wallet::record_words(&c, r) else {
                return;
            };
            let fp = crate::vault_screens::key_fingerprint(&c, self, r).unwrap_or_default();
            let mut files = Vec::new();
            for (label, what, form) in [
                (
                    EN.board_words,
                    SaveWhat::Words,
                    crate::plan::What::SeedFile(0),
                ),
                (
                    EN.board_seedqr,
                    SaveWhat::SeedQr,
                    crate::plan::What::SeedQrFile(0),
                ),
            ] {
                if what == SaveWhat::SeedQr && m.language() != osk_bip::bip39::Language::English {
                    continue;
                }
                if let Some(name) = crate::backup_flow_wallet::seed_stick_name(form, &fp) {
                    files.push(SaveFile {
                        label: label.to_string(),
                        what,
                        names: vec![name],
                        made: Vec::new(),
                        ticked: true,
                    });
                }
            }
            StickSave {
                record: at,
                seed: true,
                files,
            }
        } else {
            let Some(files) = self.with_vault_wallet(|app, i| app.wallet_save_files(i)) else {
                return;
            };
            StickSave {
                record: at,
                seed: false,
                files,
            }
        };
        self.vaults.stick_save = Some(save);
        self.sheet = Some(Sheet::VaultSave);
    }

    /// Makes session wallet `i`'s public files for the popup: the
    /// descriptor's text, its QR pictures, and each file of the software
    /// Back up writes for (Sparrow, the hardware signers, Nunchuk, Bitcoin
    /// Core) that is not the descriptor.
    fn wallet_save_files(&mut self, i: usize) -> Vec<SaveFile> {
        let Some(w) = self.session.wallets.get(i) else {
            return Vec::new();
        };
        let every = crate::plan::Answers {
            software: [true, true, true, true, false],
            ..Default::default()
        };
        let software = Self::backup_public_for(w, &every);
        let mut files = Vec::new();
        let mut add = |label: String, made: Vec<(String, Vec<u8>)>, ticked: bool| {
            if !made.is_empty() {
                files.push(SaveFile {
                    label,
                    what: SaveWhat::Public,
                    names: made.iter().map(|(n, _)| n.clone()).collect(),
                    made,
                    ticked,
                });
            }
        };
        add(
            EN.vault_descriptor.to_string(),
            self.public_made(i, 1),
            true,
        );
        add(
            EN.board_qr_picture.to_string(),
            self.public_made(i, 8),
            false,
        );
        for x in software.into_iter().filter(|x| *x != 1) {
            let name = match x {
                5 => EN.app_sparrow,
                2 => EN.app_coldcard_keystone,
                6 => EN.app_nunchuk,
                _ => EN.app_bitcoin_core,
            };
            add(
                fill(EN.board_software_file, &[name]),
                self.public_made(i, x),
                false,
            );
        }
        files
    }

    /// Writes the popup's ticked files to the Outbox, for the stick: a
    /// wallet's as they were made, a key's words and SeedQR picture made
    /// now from the record (a SeedQR drawn), as secrets let out
    /// unprotected. The popup closes.
    pub(crate) fn vault_stick_save_write(&mut self) {
        let c = self.compute();
        let Some(save) = self.vaults.stick_save.take() else {
            return;
        };
        self.sheet = None;
        let mut out: Vec<String> = Vec::new();
        let words = self
            .vaults
            .open
            .get(self.vaults.current)
            .and_then(|v| v.contents.records.get(save.record))
            .filter(|r| r.kind == kind::KEY)
            .and_then(|r| {
                let fp = crate::vault_screens::key_fingerprint(&c, self, r).unwrap_or_default();
                crate::wallet::record_words(&c, r).map(|m| (m, fp))
            });
        for f in save.files.into_iter().filter(|f| f.ticked) {
            match f.what {
                SaveWhat::Public => {
                    for (name, bytes) in f.made {
                        self.put_outbox(&name, bytes);
                        out.push(name);
                    }
                }
                SaveWhat::Words | SaveWhat::SeedQr => {
                    let (Some((m, fp)), Some(name)) = (words.as_ref(), f.names.first()) else {
                        continue;
                    };
                    let bytes = if f.what == SaveWhat::Words {
                        // The words, one line, as Add a key reads them.
                        // Sized first so that no copy is left behind by a
                        // reallocation.
                        let lang = m.language();
                        let mut v =
                            zeroize::Zeroizing::new(Vec::with_capacity(m.indices().len() * 9 + 1));
                        for (k, i) in m.indices().iter().enumerate() {
                            if k > 0 {
                                v.push(b' ');
                            }
                            v.extend_from_slice(lang.word(*i).as_bytes());
                        }
                        v.push(b'\n');
                        Some(v)
                    } else {
                        crate::backup_flow_wallet::seedqr_png(&c, m, fp)
                    };
                    let Some(bytes) = bytes else {
                        continue;
                    };
                    self.outbox.retain(|it| it.name != *name);
                    let mut item = crate::Item::new(&c, name, bytes.to_vec());
                    item.secret = true;
                    self.outbox.push(item);
                    out.push(name.clone());
                }
            }
        }
        self.save_boxes();
        let what = match out.len() {
            0 => return,
            1 => out[0].clone(),
            k => fill(EN.public_out_files, &[&k.to_string()]),
        };
        if save.seed {
            self.toast(&fill(
                EN.secrets_waits_unprotected,
                &[&what, self.medium.for_the()],
            ));
        } else {
            self.toast_out(&what);
        }
    }
}
