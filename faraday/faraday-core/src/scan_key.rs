//! Scan a key (`docs/history/SCAN-TO-SIGN.md` §2): one way in for any key a
//! loaded wallet is short of. The camera takes any of the wallet's
//! missing keys and Faraday works out which; a seed whose key with no
//! passphrase is one of them loads at once, since the match proves it
//! has none, and one that matches nothing goes to the passphrase sheet
//! (`seeds::read`), which takes it only for a missing key of the wallet.
//! Here too: the scanner's Paste, and what it refuses (F75).

use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::bip39::Language;
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::Compute;
use faraday_shell_api::Command;

use crate::plan::{At, What};
use crate::scan_read::{self, Kind};
use crate::wallet::fp_text;
use crate::{Action, Faraday, ScanPurpose, ScanState, Sheet};
use faraday_intake::seed_in_code;

/// Costs what `add_words_with` costs. The key `words` make with `pass`,
/// if they are BIP-39 words.
pub(crate) fn key_of(c: &Compute, words: &str, pass: &str) -> Option<Fingerprint> {
    faraday_keys::identity::fingerprint_of_words(c, words, pass).ok()
}

/// What a read or a choice found of a wallet's keys.
pub(crate) enum Found {
    /// Key n of the wallet, not here.
    Missing(usize),
    /// Key n of the wallet, here already.
    Here(usize),
    /// No key of the wallet.
    None,
}

/// The camera's area: the scan sheet's state, a transfer in parts kept
/// across closing the camera, and the cameras the shell can open.
#[derive(Default)]
pub(crate) struct Camera {
    /// The camera sheet.
    scan: Option<ScanState>,
    /// A transfer in parts the camera was closed on, kept for the next
    /// scan.
    parts: Option<(faraday_codec::ur::Decoder, faraday_qr::Assembler)>,
    /// The cameras the shell can open (id, name). With none, the
    /// backup's seeds step checks a copy by its typed numbers instead of
    /// the camera.
    cameras: Vec<(String, String)>,
    /// The one chosen, by id; the shell's first when none is.
    chosen: Option<String>,
    /// A camera chosen that the shell has not opened yet.
    change: Option<String>,
    /// Files made of codes read, for their names.
    scanned: u32,
}

impl Camera {
    /// The scan sheet's state, while it is open.
    pub(crate) fn scan(&self) -> Option<&ScanState> {
        self.scan.as_ref()
    }

    /// The scan sheet's state to change: the one door the code routing
    /// writes the sheet's note, frame and decoders through.
    pub(crate) fn scan_mut(&mut self) -> Option<&mut ScanState> {
        self.scan.as_mut()
    }

    /// Opens the scan sheet's state (the caller shows the sheet and
    /// turns the camera on).
    pub(crate) fn open(&mut self, scan: ScanState) {
        self.scan = Some(scan);
    }

    /// Drops the scan sheet's state.
    pub(crate) fn close(&mut self) {
        self.scan = None;
    }

    /// Takes the scan sheet's state out, as closing it.
    pub(crate) fn take(&mut self) -> Option<ScanState> {
        self.scan.take()
    }

    /// Puts `scan` in place and gives back what was there.
    pub(crate) fn swap(&mut self, scan: Option<ScanState>) -> Option<ScanState> {
        std::mem::replace(&mut self.scan, scan)
    }

    /// Closing the camera: a transfer in parts under way, but not
    /// Transfer's own, waits for the next scan.
    pub(crate) fn keep_parts(&mut self) {
        if let Some(s) = self.scan.as_mut()
            && s.in_parts()
            && s.purpose != ScanPurpose::Transfer
        {
            self.parts = Some((
                std::mem::take(&mut s.decoder),
                std::mem::take(&mut s.assembler),
            ));
        }
    }

    /// The transfer in parts kept, once.
    pub(crate) fn take_parts(
        &mut self,
    ) -> Option<(faraday_codec::ur::Decoder, faraday_qr::Assembler)> {
        self.parts.take()
    }

    /// The cameras the shell can open.
    pub(crate) fn cameras(&self) -> &[(String, String)] {
        &self.cameras
    }

    /// Takes the shell's list; whether it differs from the last.
    pub(crate) fn set_cameras(&mut self, list: Vec<(String, String)>) -> bool {
        if self.cameras == list {
            return false;
        }
        self.cameras = list;
        true
    }

    /// The camera chosen, by id.
    pub(crate) fn chosen(&self) -> Option<&str> {
        self.chosen.as_deref()
    }

    /// Chooses the `i`th camera for the shell to open; whether there is
    /// one.
    pub(crate) fn choose(&mut self, i: usize) -> bool {
        let Some((id, _)) = self.cameras.get(i) else {
            return false;
        };
        self.chosen = Some(id.clone());
        self.change = Some(id.clone());
        true
    }

    /// The camera chosen since the last call.
    pub(crate) fn take_change(&mut self) -> Option<String> {
        self.change.take()
    }

    /// The number the next file made of a code read is named by.
    pub(crate) fn next_number(&mut self) -> u32 {
        self.scanned += 1;
        self.scanned
    }
}

impl Faraday {
    /// The scan sheet's state, while the camera sheet is open.
    pub fn scan(&self) -> Option<&ScanState> {
        self.camera.scan()
    }

    /// The cameras the shell can open.
    pub fn cameras(&self) -> &[(String, String)] {
        self.camera.cameras()
    }

    /// The camera chosen, by id.
    pub fn camera(&self) -> Option<&str> {
        self.camera.chosen()
    }

    /// Whether loaded wallet `w` has enough keys here to sign.
    pub fn can_spend(&self, w: usize) -> bool {
        self.session
            .wallets
            .get(w)
            .is_some_and(|wl| crate::screens::sign_readiness(self, wl) == 2)
    }

    /// Keys loaded wallet `w` still needs here before it can spend.
    pub(crate) fn keys_short(&self, w: usize) -> usize {
        let Some(wl) = self.session.wallets.get(w) else {
            return 0;
        };
        let here = self
            .session
            .slots(wl)
            .iter()
            .filter(|s| s.held_by.is_some())
            .count();
        crate::wallet::needed(wl).saturating_sub(here)
    }

    /// Which of wallet `w`'s keys `fp` is.
    pub(crate) fn wallet_key_found(&self, w: usize, fp: Fingerprint) -> Found {
        let Some(wl) = self.session.wallets.get(w) else {
            return Found::None;
        };
        let slots = self.session.slots(wl);
        match slots.iter().position(|s| s.fingerprint == Some(fp)) {
            Some(n) if slots[n].held_by.is_some() => Found::Here(n),
            Some(n) => Found::Missing(n),
            None => Found::None,
        }
    }

    /// Costs what `plan_notes` costs. The name a key of wallet `w` loads
    /// under: who holds it, as the backup's plan keeps it, else none.
    pub(crate) fn wallet_key_label(&self, c: &Compute, w: usize, fp: Fingerprint) -> String {
        let Some(wl) = self.session.wallets.get(w) else {
            return String::new();
        };
        self.plan_notes(c, wl, None)
            .into_iter()
            .find_map(|n| match n {
                crate::plan::Note::Holder(f, name) if f == fp.0 => Some(name),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// The line a seed that is no key of wallet `w` is refused with.
    pub(crate) fn not_a_wallet_key(&self, w: usize, fp: Fingerprint) -> String {
        let name = self
            .session
            .wallets
            .get(w)
            .map(|wl| wl.name.as_str())
            .unwrap_or_default();
        fill(EN.scan_key_is_not_one, &[&fp_text(fp).to_string(), name])
    }

    /// Costs what `of_now` costs. The keys the key scanner for wallet `w`
    /// takes, a line each: "Key 2 · 1A2B3C4D · Place 2", the place where
    /// the backup's plan keeps that seed.
    pub(crate) fn wallet_key_lines(&self, c: &Compute, w: usize) -> Vec<String> {
        let Some(wl) = self.session.wallets.get(w) else {
            return Vec::new();
        };
        let glance = crate::glance::of_now(c, self, w);
        let place = |n: usize| -> Option<String> {
            let g = glance.as_ref()?;
            let seed = g.keys.get(n)?.seed?;
            let crate::glance::Backup::Plan { nodes, .. } = &g.backup else {
                return None;
            };
            nodes
                .iter()
                .filter(|b| matches!(b.at, At::Place(_)))
                .find(|b| {
                    b.whats.iter().any(|x| {
                        matches!(*x, What::Words(i) | What::SeedQr(i) | What::Seed(i)
                            | What::SeedPassphrase(i) | What::SeedFile(i)
                            | What::SeedQrFile(i) if i == seed)
                    })
                })
                .map(|b| b.name.clone())
        };
        self.session
            .slots(wl)
            .iter()
            .enumerate()
            .filter(|(_, s)| s.held_by.is_none())
            .filter_map(|(n, s)| {
                let fp = s.fingerprint?;
                let line = fill(
                    EN.scan_key_key,
                    &[&(n + 1).to_string(), &fp_text(fp).to_string()],
                );
                Some(match place(n) {
                    Some(p) => format!("{line} · {p}"),
                    None => line,
                })
            })
            .collect()
    }

    /// A seed's words read by wallet `w`'s key scanner.
    pub(crate) fn wallet_key_scanned(&mut self, w: usize, words: &str) {
        let c = self.compute();
        let Some(fp) = key_of(&c, words, "") else {
            if let Some(scan) = self.camera.scan_mut() {
                scan.note = Some(EN.scan_key_not_a_seedqr.to_string());
            }
            return;
        };
        match self.wallet_key_found(w, fp) {
            Found::Here(n) => {
                if let Some(scan) = self.camera.scan_mut() {
                    scan.note = Some(fill(EN.scan_key_key_already, &[&(n + 1).to_string()]));
                }
            }
            Found::Missing(n) => {
                let label = self.wallet_key_label(&c, w, fp);
                match self
                    .session
                    .keys
                    .add_words_with(&c, words, "", &label, None)
                {
                    Ok(_) => self.wallet_key_added(w, n),
                    Err(e) => {
                        if let Some(scan) = self.camera.scan_mut() {
                            scan.note = Some(e.text());
                        }
                    }
                }
            }
            // Perhaps a seed with a passphrase: the sheet asks for it.
            Found::None => {
                let reason = self.not_a_wallet_key(w, fp);
                self.seed_read_for_wallet(words, w, reason);
            }
        }
    }

    /// Key n of wallet `w` loaded from the key scanner or the passphrase
    /// sheet: the spend and the Spend tab take it, and the camera says
    /// how many more, or closes once the wallet can spend.
    pub(crate) fn wallet_key_added(&mut self, w: usize, n: usize) {
        self.refresh_spend();
        // Scanned from Add a key: back where Add a key was opened from.
        if self.screen == crate::Screen::Entry {
            let back = self.entry.state().back;
            self.entry.reset();
            self.screen = match back {
                Some(s) => s,
                None if self.wallets.spend().is_some() => crate::Screen::Spend,
                None => crate::Screen::Wallets,
            };
        }
        if self.screen == crate::Screen::Family {
            self.family_key_added();
        }
        self.wallet_key_said(w, n);
    }

    /// What loading key n of wallet `w` says: with the camera on and the
    /// wallet still short, the camera's note; else a toast, the camera
    /// closed.
    pub(crate) fn wallet_key_said(&mut self, w: usize, n: usize) {
        let short = self.keys_short(w);
        if short > 0
            && let Some(scan) = self.camera.scan_mut()
        {
            scan.note = Some(fill(
                EN.scan_key_key_added_more,
                &[&(n + 1).to_string(), &short.to_string()],
            ));
            return;
        }
        if self.camera.take().is_some() {
            self.out.commands.push_back(Command::CameraOff);
            if self.sheet == Some(Sheet::Scan) {
                self.sheet = None;
            }
        }
        if short > 0 {
            self.toast(&fill(
                EN.scan_key_key_added_more,
                &[&(n + 1).to_string(), &short.to_string()],
            ));
        } else {
            self.toast(&fill(EN.scan_key_key_added, &[&(n + 1).to_string()]));
        }
    }
}

impl Faraday {
    /// Handles the scan sheet's actions (`ScanPaste`, `ScanCamera`, `ScanPart`, `ScanSeed`, `ScanKey`, `Scan`), and Paste words.
    pub(crate) fn scan_act(&mut self, action: Action) {
        match action {
            Action::ScanPaste => self.scan_paste(),
            Action::PasteWords => self.entry_paste(),
            Action::ScanCamera(i) => {
                if self.camera.choose(usize::from(i))
                    && let Some(s) = self.camera.scan_mut()
                {
                    s.seen = None;
                    s.too_fine = 0;
                }
            }
            Action::ScanPart => {
                if self.may_load_keys() {
                    let mut scan = ScanState::default();
                    scan.purpose = ScanPurpose::KeyPart;
                    self.camera.open(scan);
                    self.sheet = Some(Sheet::Scan);
                    self.out.commands.push_back(Command::CameraOn);
                }
            }
            Action::ScanSeed => {
                if self.may_load_keys() {
                    let mut scan = ScanState::default();
                    scan.purpose = ScanPurpose::Seed;
                    self.camera.open(scan);
                    self.sheet = Some(Sheet::Scan);
                    self.out.commands.push_back(Command::CameraOn);
                }
            }
            Action::ScanKey(w) => {
                if self.may_load_keys() && w < self.session.wallets.len() {
                    let mut scan = ScanState::default();
                    scan.purpose = ScanPurpose::WalletKey(w);
                    self.camera.open(scan);
                    self.sheet = Some(Sheet::Scan);
                    self.out.commands.push_back(Command::CameraOn);
                }
            }
            Action::Scan => {
                let mut scan = ScanState::default();
                if let Some((decoder, assembler)) = self.camera.take_parts() {
                    scan.decoder = decoder;
                    scan.assembler = assembler;
                    scan.note = Some(EN.act_carrying_on_parts.to_string());
                }
                self.camera.open(scan);
                self.sheet = Some(Sheet::Scan);
                self.out.commands.push_back(Command::CameraOn);
            }
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}

/// Costs a mnemonic's checksum. The words of `text`, pasted onto the
/// words form typed from `lang`, as the list's places: split at any
/// white space and matched in any case, as typing them would be; 12, 15,
/// 18, 21 or 24 of them, each on the list, with a checksum that holds.
/// Else what is wrong, in the words a typed key is refused with.
fn pasted_words(lang: Language, text: &str) -> Result<zeroize::Zeroizing<Vec<u16>>, String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return Err(EN.pasted_nothing.to_string());
    }
    if !crate::EntryState::COUNTS
        .iter()
        .any(|n| usize::from(*n) == words.len())
    {
        return Err(fill(EN.paste_words_count, &[&words.len().to_string()]));
    }
    let mut idx = zeroize::Zeroizing::new(Vec::with_capacity(words.len()));
    for (k, w) in words.iter().enumerate() {
        let lower = zeroize::Zeroizing::new(w.to_lowercase());
        let i = crate::forms::typed_index(lang, &lower)
            .or_else(|| lang.index_of(&lower))
            .ok_or_else(|| {
                fill(
                    EN.forms_word_not_in_list,
                    &[&(k + 1).to_string(), crate::forms::language_name(lang)],
                )
            })?;
        idx.push(i);
    }
    faraday_keys::words::from_indices(lang, &idx)
        .map_err(|e| faraday_strings::bip::bip39_error(&e))?;
    Ok(idx)
}

/// Costs what [`scan_read::secret_kind`] costs. What a code that is a
/// secret is named as by a scanner for anything (F167: no scanner files
/// one), with where Faraday takes it: a seed in any of its forms, a GPG
/// key's paperkey, a private key, or words of a BIP-39 list that are no
/// English BIP-39 seed. `None` for anything else, a share of a key and a
/// TOTP setup code among them, which such a scanner routes.
pub(crate) fn secret_code(c: &Compute, bytes: &[u8]) -> Option<&'static str> {
    Some(match scan_read::secret_kind(c, bytes)? {
        Kind::Paperkey => EN.read_code_is_paperkey,
        Kind::SeedQr => EN.read_code_is_seedqr,
        Kind::CompactSeedQr => EN.read_code_that_may,
        Kind::SeedWords => EN.secret_text_those_are,
        Kind::Wif | Kind::Xprv => EN.secret_text_private_key,
        Kind::Nostr => EN.secret_text_nostr,
        Kind::BadChecksum => EN.secret_text_not_a_seed,
        _ => return None,
    })
}

/// Whether every word is on one of the BIP-39 lists Faraday reads, in any
/// case. A lookup per word.
fn on_a_word_list(words: &[&str]) -> bool {
    Language::ALL.iter().any(|lang| {
        words.iter().all(|w| {
            lang.index_of(&zeroize::Zeroizing::new(w.to_lowercase()))
                .is_some()
        })
    })
}

/// Whether `text` starts as a part of `form` does, so the form's parser
/// says what is wrong with it: a codex32 string's `ms1`, a SLIP-39
/// share's first word, Seed XOR's words of a BIP-39 list. A lookup.
fn starts_as_form(form: crate::forms::Form, text: &str) -> bool {
    use crate::forms::Form;
    let first = text.split_whitespace().next().unwrap_or("");
    match form {
        Form::Codex32 => text.get(..3).is_some_and(|p| p.eq_ignore_ascii_case("ms1")),
        Form::Slip39 => {
            faraday_bip::slip39::index_of(&zeroize::Zeroizing::new(first.to_lowercase())).is_some()
        }
        Form::Xor | Form::Words => {
            let words: Vec<&str> = text.split_whitespace().collect();
            !words.is_empty() && scan_read::on_a_word_list(&words)
        }
    }
}
/// Costs a mnemonic's checksum (through `secret_text`), a codex32 and a
/// SLIP-39 parse and a WIF's checksum. Why pasted text is refused for
/// holding a secret (`docs/DESIGN.md` §4.10): a seed's words, checksum or
/// not, a SeedQR's digits, an extended private key, a WIF private key, a
/// codex32 string or a SLIP-39 share. A scanner that loads a seed
/// (`seeds`) takes the seed's own forms, as typing them would (F75): it
/// refuses only a private key.
pub(crate) fn pasted_secret(c: &Compute, text: &str, seeds: bool) -> Option<&'static str> {
    let t = text.trim();
    let words: Vec<&str> = t.split_whitespace().collect();
    // Words of any of the word lists Faraday reads, checksum or not.
    if !seeds && matches!(words.len(), 12 | 15 | 18 | 21 | 24) && on_a_word_list(&words) {
        return Some(EN.pasted_words);
    }
    match faraday_intake::secret_text(c, t) {
        Some(faraday_intake::TextSecret::Words) if !seeds => return Some(EN.pasted_words),
        Some(faraday_intake::TextSecret::ExtendedPrivateKey) => {
            return Some(EN.pasted_private_key);
        }
        _ => {}
    }
    if faraday_keys::identity::is_wif(t) {
        return Some(EN.pasted_private_key);
    }
    if seeds {
        return None;
    }
    if matches!(t.len(), 48 | 96) && t.bytes().all(|b| b.is_ascii_digit()) {
        return Some(EN.pasted_seedqr);
    }
    let lower = zeroize::Zeroizing::new(t.to_lowercase());
    if faraday_bip::codex32::Codex32::parse(&lower).is_ok()
        || faraday_bip::slip39::Share::parse(&lower).is_ok()
    {
        return Some(EN.pasted_share);
    }
    None
}

impl Faraday {
    /// Whether the words form takes words pasted (F75): Add a key's, a
    /// Restore slot's and New key's typed key's, with no sheet over it and
    /// no stick in, on a shell with a clipboard (Faraday Demo; never the
    /// device) and off Mainnet. A lookup.
    pub fn entry_pastes(&self) -> bool {
        self.clipboard
            && self.mode() != crate::Mode::Mainnet
            && self.sheet.is_none()
            && self.entry_typing_here()
            && self.entry.state().form == crate::forms::Form::Words
            && self.may_load_keys()
    }

    /// Whether the page hands Faraday Demo's strip Paste words: wherever
    /// the words form takes words pasted, in every mode Faraday Demo has
    /// (Practice and the test networks; owner, 2026-10-26). A lookup.
    pub fn entry_paste_offered(&self) -> bool {
        self.entry_pastes()
    }

    /// Paste words: the shell is asked for the clipboard's text, where the
    /// words form takes words pasted.
    pub(crate) fn entry_paste(&mut self) {
        if self.entry_pastes() {
            self.out.commands.push_back(Command::RequestClipboard {
                kind: faraday_shell_api::FileKind::Text,
            });
        }
    }

    /// Costs a mnemonic's checksum (`pasted_words`). The clipboard's text
    /// asked for by Paste words, where no scanner asked: a whole key's
    /// words go into the words form in place of what it held, as typed,
    /// the word count set to theirs; anything else leaves the form as it
    /// was and says why. Returns whether the form took the text.
    pub(crate) fn entry_pasted(&mut self, text: &str) -> bool {
        if self.scan_pastes() || !self.entry_pastes() {
            return false;
        }
        let lang = self.entry.state().language();
        let idx = match pasted_words(lang, text) {
            Ok(idx) => idx,
            Err(why) => {
                self.entry.state_mut().error = Some(why);
                return true;
            }
        };
        let e = self.entry.state_mut();
        if e.keys.is_some() {
            let mut w = crate::forms::word_typer(lang);
            for &i in idx.iter() {
                w.put(i);
            }
            e.keys = Some(w);
        } else {
            let mut typed = zeroize::Zeroizing::new(String::new());
            for &i in idx.iter() {
                if let Some(t) = lang.typed(i) {
                    typed.extend(t.as_chars());
                }
                typed.push(' ');
            }
            e.typed.set(&typed);
        }
        e.word_count = idx.len() as u8;
        e.on_passphrase = false;
        e.error = None;
        e.sample.dismiss();
        e.forget_probe();
        true
    }
}

impl Faraday {
    /// Whether the scanner on screen offers Paste: only where the shell
    /// has a clipboard (the desktop app), and then on one that takes
    /// anything or loads a seed (F75). Not a TOTP code's, nor a copy
    /// check's (`docs/QR.md` §8 item 5). Without a clipboard no scanner
    /// has it.
    pub fn scan_pastes(&self) -> bool {
        self.clipboard
            && self.sheet == Some(Sheet::Scan)
            && self.camera.scan().is_some_and(|s| match s.purpose {
                ScanPurpose::Files
                | ScanPurpose::Transfer
                | ScanPurpose::Intake
                | ScanPurpose::Seed
                | ScanPurpose::KeyPart
                | ScanPurpose::WalletKey(_)
                | ScanPurpose::GpgPublic
                | ScanPurpose::Sweep => true,
                ScanPurpose::VaultEntry
                | ScanPurpose::CheckCopy(_)
                | ScanPurpose::CheckCodex32
                | ScanPurpose::GpgPaperkey => false,
            })
    }

    /// Whether the scanner on screen loads a seed and takes one pasted
    /// (F75): Add a key's, a part of a key's, a wallet's key's, What do
    /// you have?, and Home's Scan (owner, 2026-10-29), on a shell with a
    /// clipboard. Home's Scan is the Files purpose opened on Home; Files'
    /// own scanner stays refused.
    fn scan_takes_pasted_seed(&self) -> bool {
        self.clipboard
            && self.camera.scan().is_some_and(|s| {
                matches!(
                    s.purpose,
                    ScanPurpose::Seed
                        | ScanPurpose::KeyPart
                        | ScanPurpose::WalletKey(_)
                        | ScanPurpose::Intake
                ) || (s.purpose == ScanPurpose::Files && self.screen == crate::Screen::Home)
            })
    }

    /// Ctrl+V: the scanner's Paste where a scanner is up, else Paste words
    /// where the words form has the keyboard (not its passphrase).
    pub(crate) fn paste(&mut self) {
        if self.scan_pastes() {
            self.scan_paste();
        } else if !self.entry.state().on_passphrase {
            self.entry_paste();
        }
    }

    /// Paste on the scanner: the shell is asked for the clipboard's text,
    /// where it has a clipboard.
    pub(crate) fn scan_paste(&mut self) {
        if self.clipboard && self.scan_pastes() {
            self.out.commands.push_back(Command::RequestClipboard {
                kind: faraday_shell_api::FileKind::Text,
            });
        }
    }

    /// Costs what `pasted_secret` and `seed_in_code` cost, then what a
    /// code read costs (through `scanned`). The clipboard's text, asked
    /// for by Paste: a secret is refused and named, anything else goes
    /// where a QR the camera read goes, and nothing at all says so for a
    /// moment (`docs/QR.md` §8 item 5). A scanner that loads a seed takes
    /// its words, SeedQR digits and shares as typed or scanned (F75); the
    /// words go on as one space between each, in lower case.
    pub(crate) fn pasted(&mut self, text: zeroize::Zeroizing<String>) {
        if self.entry_pasted(&text) || self.explore_pasted(&text) || !self.scan_pastes() {
            return;
        }
        let c = self.compute();
        let seeds = self.scan_takes_pasted_seed();
        let until = self.now_ms + faraday_draw::tokens::NOTE_FLASH_MS;
        let t = text.trim();
        // The Sweep sheet's scanner takes a private key pasted, as typed
        // or scanned (F171); every other scanner refuses one.
        let sweep_key = self
            .camera
            .scan()
            .is_some_and(|s| s.purpose == ScanPurpose::Sweep)
            && faraday_keys::identity::is_wif(t);
        if let Some(scan) = self.camera.scan_mut() {
            if t.is_empty() {
                scan.nothing_to_paste = Some(until);
                return;
            }
            scan.nothing_to_paste = None;
            if let Some(why) = pasted_secret(&c, t, seeds).filter(|_| !sweep_key) {
                scan.note = Some(why.to_string());
                return;
            }
        }
        if seeds && t.contains(char::is_whitespace) {
            let lower = zeroize::Zeroizing::new(t.to_lowercase());
            if let Some(words) = seed_in_code(&c, lower.as_bytes()) {
                return self.scanned(words.as_bytes().to_vec());
            }
        }
        self.scanned(t.as_bytes().to_vec());
    }

    /// Costs a codex32 and a SLIP-39 parse, and what adding a part costs.
    /// A codex32 string (a Codex32QR code's) or a SLIP-39 share read by a
    /// scanner for anything (Home's Scan, Files', Restore's, Create's and
    /// the Spend tab's; F167): Add a key opens on its form with the part
    /// in, as the intake scanner sends one, never into Files. Any other
    /// scanner's code is left to it. Returns whether it was one.
    pub(crate) fn scan_part_routed(&mut self, bytes: &[u8]) -> bool {
        if self.sheet != Some(Sheet::Scan)
            || !self
                .camera
                .scan()
                .is_some_and(|s| s.purpose == ScanPurpose::Files)
        {
            return false;
        }
        self.intake_key_part(bytes)
    }

    /// Costs what adding entries to a vault costs. A code read as a file,
    /// kept from Files when it is a secret (F167): TOTP setup codes and
    /// vault entries go to the open vault (`scanned_entries`), and a
    /// seed's words or a part of one that reach here (keys not taken now)
    /// are named, their bytes wiped. A file sent in Faraday's envelope is
    /// filed as sent. Returns the item when it is to be filed.
    pub(crate) fn scanned_secret(
        &mut self,
        mut item: crate::Item,
        enveloped: bool,
        intake: bool,
    ) -> Option<crate::Item> {
        use crate::wallet::FileKind;
        // A picture read from a stick is copied in as the visit's rules say.
        if self.sheet != Some(Sheet::Scan)
            || enveloped
            || !matches!(
                item.kind,
                FileKind::Entries | FileKind::Words | FileKind::SeedPart
            )
        {
            return Some(item);
        }
        let text = zeroize::Zeroizing::new(String::from_utf8_lossy(&item.bytes).trim().to_string());
        zeroize::Zeroize::zeroize(&mut item.bytes);
        if item.kind == FileKind::Entries {
            self.scanned_entries(text, intake);
            return None;
        }
        drop(text);
        let why = if self.may_load_keys() {
            EN.secret_text_those_are.to_string()
        } else {
            crate::stick::keys_load(self.medium)
        };
        if let Some(scan) = self.camera.scan_mut() {
            scan.note = Some(why);
        }
        None
    }

    /// Costs what adding entries to a vault costs. TOTP setup codes or
    /// vault entries read by a scanner for anything (F167), never filed:
    /// into the open vault, a TOTP code's new entry filled in as the
    /// vault's own Scan fills it; with no vault open the scanner says to
    /// unlock one, and the intake, which adds to no vault, says it is not
    /// used there. The camera stays on unless the vault took them.
    fn scanned_entries(&mut self, text: zeroize::Zeroizing<String>, intake: bool) {
        let said = if intake {
            Some(EN.scanned_totp_not_here.to_string())
        } else if self.vaults.current_id().is_none() {
            Some(EN.scanned_totp_needs_vault.to_string())
        } else {
            self.vault_scanned_entry(&text).err()
        };
        drop(text);
        match said {
            None => {
                self.camera.close();
                self.sheet = None;
                self.out.commands.push_back(Command::CameraOff);
            }
            Some(why) => {
                if let Some(scan) = self.camera.scan_mut() {
                    scan.note = Some(why);
                }
            }
        }
    }

    /// Costs a codex32 parse, and what adding the string costs. A codex32
    /// string (a Codex32QR code's, `docs/CODEX32.md` §10.1) read where a
    /// SeedQR was asked for: the camera closes and the codex32 form opens
    /// with the string in, as Home's Scan opens it; nothing loads until
    /// enough strings are in. Any other scanner's code, or one read while
    /// a stick is in, is left to the scanner. Read as given: mixed case is refused, as
    /// BIP 93 says. Returns whether it was one. A Create slot's New key
    /// keeps its own scanner, which takes SeedQRs only.
    pub(crate) fn seed_scan_codex32(&mut self, bytes: &[u8]) -> bool {
        use crate::forms::Form;
        // Only Scan a SeedQR's scanner, and never while a stick is in
        // (the scanner says why itself).
        if !self
            .camera
            .scan()
            .is_some_and(|s| s.purpose == ScanPurpose::Seed)
            || (self.sheet == Some(Sheet::Scan) && !self.stick.is_empty())
        {
            return false;
        }
        let given = zeroize::Zeroizing::new(String::from_utf8_lossy(bytes).trim().to_string());
        if faraday_bip::codex32::Codex32::parse(&given).is_err() {
            return false;
        }
        let in_flow = self.screen == crate::Screen::KeyGen;
        if in_flow && self.keygen.state().is_some_and(|k| k.slot.is_some()) {
            return false;
        }
        self.camera.close();
        self.sheet = None;
        self.out.commands.push_back(Command::CameraOff);
        let codex32 = Form::ALL.iter().position(|f| *f == Form::Codex32);
        if in_flow {
            self.keygen_open_on(crate::keygen::Start::Codex32);
        } else {
            if self.screen != crate::Screen::Entry {
                self.act(Action::Entry(None));
            }
            if let Some(i) = codex32
                && self.entry.state().form != Form::Codex32
            {
                self.act(Action::EntryForm(i as u8));
            }
        }
        self.entry.state_mut().typed.set(&given);
        self.form_add_part();
        true
    }
}

impl Faraday {
    /// Costs what reading the code costs (a mnemonic's checksum, a
    /// codex32 and a SLIP-39 parse, a PSBT and a wallet read) and what
    /// the scanner that takes it does with it. One code the camera read,
    /// or Paste gave (`docs/SCANNING.md` §4): the scanner on screen takes
    /// what it is for, and names anything else (§4.6).
    pub(crate) fn scanned(&mut self, bytes: Vec<u8>) {
        let bytes = zeroize::Zeroizing::new(bytes);
        if self.seed_scan_codex32(&bytes)
            || self.gpg_restore_scanned(&bytes)
            || self.sweep_scanned(&bytes)
        {
            return;
        }
        let camera = self.sheet == Some(Sheet::Scan);
        let Some(purpose) = self.camera.scan().map(|s| s.purpose) else {
            return;
        };
        // Transfer's Receive takes anything and files nothing.
        if purpose == ScanPurpose::Transfer {
            return self.transfer_scanned(bytes);
        }
        // A stick plugged in while the camera is on is held back, and no
        // key loads while it is attached: what loads one is not read.
        if camera
            && !self.stick.is_empty()
            && matches!(
                purpose,
                ScanPurpose::Seed | ScanPurpose::KeyPart | ScanPurpose::WalletKey(_)
            )
        {
            return self.scan_say(crate::stick::keys_load(self.medium));
        }
        match purpose {
            ScanPurpose::VaultEntry => self.entry_code_scanned(bytes),
            ScanPurpose::KeyPart => self.part_scanned(bytes),
            ScanPurpose::CheckCopy(fp) => self.copy_scanned(fp, bytes),
            ScanPurpose::CheckCodex32 => self.c32_copy_scanned(bytes),
            ScanPurpose::WalletKey(w) => self.wallet_key_code(w, bytes),
            ScanPurpose::Seed => self.seed_code_scanned(bytes),
            _ => self.files_scanned(bytes, camera),
        }
    }

    /// The scanner's line says `why`; the camera stays on.
    pub(crate) fn scan_say(&mut self, why: String) {
        if let Some(scan) = self.camera.scan_mut() {
            scan.note = Some(why);
        }
    }

    /// The camera closes and the scanner with it.
    pub(crate) fn scan_close(&mut self) {
        self.camera.close();
        self.sheet = None;
        self.out.commands.push_back(Command::CameraOff);
    }

    /// Costs what [`scan_read::secret_kind`] costs, and a code read as a
    /// file (`read_code`). A code the scanner on screen does not take:
    /// named on its line by what it is (`docs/SCANNING.md` §4.1), the
    /// code wiped; a transfer in parts is put together first.
    pub(crate) fn scan_not_taken(&mut self, bytes: zeroize::Zeroizing<Vec<u8>>) {
        let c = self.compute();
        if let Some(kind) = scan_read::secret_kind(&c, &bytes) {
            drop(bytes);
            return self.scan_say(kind.not_here());
        }
        let Some((named, _, data)) = self.read_code(bytes) else {
            return;
        };
        let data = zeroize::Zeroizing::new(data);
        let kind = scan_read::public_kind(
            crate::wallet::classify(&c, "", &data),
            named.is_some(),
            &String::from_utf8_lossy(&data),
        );
        self.scan_say(kind.not_here());
    }

    /// Costs what adding entries to a vault costs. A code read by a
    /// vault's Scan a setup code: a TOTP setup code or vault entries go
    /// into the open vault; anything else is named.
    fn entry_code_scanned(&mut self, bytes: zeroize::Zeroizing<Vec<u8>>) {
        let text = zeroize::Zeroizing::new(String::from_utf8_lossy(&bytes).into_owned());
        let refused = self.vault_scanned_entry(&text).err();
        drop(text);
        let Some(why) = refused else {
            return self.scan_close();
        };
        let c = self.compute();
        match scan_read::secret_kind(&c, &bytes) {
            Some(Kind::Totp | Kind::Entries) => self.scan_say(why),
            _ => self.scan_not_taken(bytes),
        }
    }

    /// Costs what adding a part costs, and a codex32 and a SLIP-39 parse.
    /// A code read by an Add a key form's Scan (codex32, SLIP-39, Seed
    /// XOR; `docs/SCANNING.md` §4.3): its own form's part goes in; the
    /// other share form's switches to that form with it in; a whole seed
    /// goes to the seed sheet; a code that starts as the form's does keeps
    /// the parser's line, and anything else is named.
    fn part_scanned(&mut self, bytes: zeroize::Zeroizing<Vec<u8>>) {
        use crate::forms::Form;
        let c = self.compute();
        let kind = scan_read::secret_kind(&c, &bytes);
        let form = self.entry.state().form;
        let kind_form = match kind {
            Some(Kind::Codex32Share | Kind::Codex32Secret) => Some(Form::Codex32),
            Some(Kind::Slip39Share) => Some(Form::Slip39),
            _ => None,
        };
        if let Some(to) = kind_form.filter(|f| *f != form) {
            if let Some(i) = Form::ALL.iter().position(|f| *f == to) {
                self.act(Action::EntryForm(i as u8));
            }
        } else if form != Form::Xor
            && kind.is_some_and(Kind::seed)
            && let Some(words) = seed_in_code(&c, &bytes)
        {
            drop(bytes);
            return self.seed_scanned(&words);
        }
        let text = match faraday_intake::seedqr_words(&bytes) {
            Some(w) => w,
            None => zeroize::Zeroizing::new(String::from_utf8_lossy(&bytes).trim().to_string()),
        };
        let ours = kind_form.is_some() || starts_as_form(self.entry.state().form, &text);
        self.entry.state_mut().typed.set(&text);
        drop(text);
        self.entry.state_mut().error = None;
        self.form_add_part();
        match self.entry.state().error.clone() {
            None => self.scan_close(),
            Some(why) => {
                zeroize::Zeroize::zeroize(&mut self.entry.state_mut().typed);
                self.entry.state_mut().error = None;
                if ours {
                    self.scan_say(why);
                } else {
                    self.scan_not_taken(bytes);
                }
            }
        }
    }

    /// Costs a mnemonic's checksum. A code read by a wallet's Scan a key:
    /// a seed is matched to the wallet's missing keys; anything else is
    /// named.
    fn wallet_key_code(&mut self, w: usize, bytes: zeroize::Zeroizing<Vec<u8>>) {
        let c = self.compute();
        match seed_in_code(&c, &bytes) {
            Some(words) => {
                drop(bytes);
                self.wallet_key_scanned(w, &words);
            }
            None => self.scan_not_taken(bytes),
        }
    }

    /// Costs a mnemonic's checksum, a SeedQR's decode and a SLIP-39 parse.
    /// A code read by Scan a SeedQR: a seed's words (pasted on the
    /// desktop, F75, or in a QR), a SeedQR or a CompactSeedQR load as the
    /// SeedQR's would; a SLIP-39 share goes to its form, as a codex32
    /// string does (`seed_scan_codex32`); anything else is named.
    fn seed_code_scanned(&mut self, mut bytes: zeroize::Zeroizing<Vec<u8>>) {
        let c = self.compute();
        if std::str::from_utf8(&bytes).is_ok_and(|t| t.contains(' '))
            && let Some(words) = seed_in_code(&c, &bytes)
        {
            drop(bytes);
            return self.seed_scanned(&words);
        }
        if self.screen != crate::Screen::KeyGen
            && scan_read::secret_kind(&c, &bytes) == Some(Kind::Slip39Share)
            && self.intake_key_part(&bytes)
        {
            return;
        }
        let lang = faraday_bip::bip39::Language::English;
        let digits = bytes.iter().all(u8::is_ascii_digit);
        let read = if digits {
            faraday_keys::words::from_seedqr_digits(&bytes, lang)
        } else {
            faraday_keys::words::from_compact_seedqr(&bytes, lang)
        };
        let Ok(m) = read else {
            return self.scan_not_taken(bytes);
        };
        zeroize::Zeroize::zeroize(&mut *bytes);
        let words = lang.words();
        let mut typed = crate::secret_text::room();
        for (k, &i) in m.indices().iter().enumerate() {
            if k > 0 {
                typed.push(' ');
            }
            typed.push_str(words[usize::from(i)]);
        }
        self.seed_scanned(&typed);
    }

    /// Costs a mnemonic's checksum, a key part's parse and what a code
    /// read as a file costs. A code read by a scanner for anything (Home's
    /// Scan, Files', Restore's, Create's, Spend's) or What do you have?:
    /// a seed and a part of a key go to Add a key, a secret is named and
    /// never filed (F167), anything else is filed and goes to the flow
    /// waiting for it.
    fn files_scanned(&mut self, bytes: zeroize::Zeroizing<Vec<u8>>, camera: bool) {
        let c = self.compute();
        let sticks = !self.stick.is_empty();
        // What do you have?: a seed asks how it loads, a share of a key
        // goes to Add a key's form for it, any other secret is named, and
        // anything else goes to Files below and the camera stays on.
        let intake = self
            .camera
            .scan()
            .is_some_and(|s| s.purpose == ScanPurpose::Intake);
        if intake {
            if let Some(words) = seed_in_code(&c, &bytes) {
                if sticks {
                    return self.scan_say(crate::stick::keys_load(self.medium));
                }
                return self.family_intake_seed(&words);
            }
            if self.intake_key_part(&bytes) {
                return;
            }
            if let Some(kind) = scan_read::secret_kind(&c, &bytes) {
                drop(bytes);
                return self.scan_say(kind.not_here());
            }
        }
        // A part of a key goes to Add a key's form, never Files (F167).
        if self.scan_part_routed(&bytes) {
            return;
        }
        // Home's Scan takes a seed too: a SeedQR, a CompactSeedQR or the
        // words go to Add a key, as its own Scan does, and never into a
        // box (`docs/QR.md` §2).
        if camera && let Some(words) = seed_in_code(&c, &bytes) {
            if self.may_load_keys() {
                self.act(Action::Entry(None));
                return self.seed_scanned(&words);
            }
            if sticks {
                return self.scan_say(crate::stick::keys_load(self.medium));
            }
        }
        let Some((named, ext, data)) = self.read_code(bytes) else {
            return;
        };
        self.file_scanned(&c, named, ext, data, intake);
    }

    /// Costs a file's classification and what the flow that takes it
    /// costs. A whole code read as a file: filed (the same file again is
    /// the one already in Files), then taken by the flow waiting for it,
    /// or left in Files with its toast. A secret is never filed (F167).
    fn file_scanned(
        &mut self,
        c: &Compute,
        named: Option<String>,
        ext: &str,
        data: Vec<u8>,
        intake: bool,
    ) {
        let number = self.camera.next_number();
        // A file in the Faraday file envelope is a file sent as one, of
        // whatever kind (`docs/QR.md` §2): it is filed under its name.
        let enveloped = named.is_some();
        let name = match named {
            Some(n) => self.free_inbox_name(&n),
            None => format!("scanned-{number}.{ext}"),
        };
        let item = crate::Item::new(c, &name, data);
        if item.kind == crate::wallet::FileKind::Other && !enveloped {
            if let Some(scan) = self.camera.scan_mut() {
                scan.note = Some(EN.scanned_not_a_psbt.to_string());
                scan.decoder = faraday_codec::ur::Decoder::new();
            }
            return;
        }
        let Some(item) = self.scanned_secret(item, enveloped, intake) else {
            return;
        };
        let is_key = item.kind == crate::wallet::FileKind::Key;
        let at = self.inbox_file_in(item);
        let name = self.inbox[at].name.clone();
        if intake {
            return self.family_intake_file(at);
        }
        self.scan_close();
        // A cosigner's account key scanned while a wallet is made from
        // seeds fills the box waiting for one.
        if is_key && self.seeds_take_file(at) {
            self.toast(&fill(EN.scanned_is_a_cosigner, &[&name.to_string()]));
            return;
        }
        // A cosigner's key, or a descriptor of one, scanned while a wallet
        // is being made goes to the slot waiting for one.
        if self.create_scan_arrived(c, at, &name)
            || self.family_arrived(at)
            || (self.screen == crate::Screen::Restore && self.restore_arrived(at))
        {
            return;
        }
        self.scan_landed(c, at, &name);
    }

    /// A seed's words read by the camera: the camera closes and the
    /// seed waits on its sheet for how it loads (`seed_read`).
    pub(crate) fn seed_scanned(&mut self, words: &str) {
        // A scan fills the words from a camera frame, which keeps the
        // status line's probe: it is worked out again for these.
        self.entry.state().forget_probe();
        // New key's Scan a SeedQR: its Key card takes the words and asks
        // the passphrase there (`docs/history/NEW-WALLET.md` §15.2).
        if self.keygen_scanned(words) {
            return;
        }
        self.seed_read_open(words);
    }

    /// A code read while the backup's copy item scans the copy of the
    /// seed `fp`: decoded as a SeedQR and compared word by word with that
    /// seed, never loaded. A match closes the camera; a word that differs
    /// is named and the camera stays open for the copy fixed.
    pub(crate) fn copy_scanned(
        &mut self,
        fp: faraday_bip::keys::Fingerprint,
        bytes: zeroize::Zeroizing<Vec<u8>>,
    ) {
        let c = self.compute();
        let copy = crate::copy_check::copy_indices(&bytes);
        drop(bytes);
        let Some(copy) = copy else {
            if let Some(scan) = self.camera.scan_mut() {
                scan.note = Some(EN.copy_scanned_not_seedqr.to_string());
            }
            return;
        };
        let found = self
            .session
            .keys
            .position(fp)
            .and_then(|k| self.session.keys.words_match(&c, k, &copy));
        let Some(found) = found else {
            // The seed is no longer here: nothing to compare with.
            self.camera.close();
            self.sheet = None;
            self.out.commands.push_back(Command::CameraOff);
            return;
        };
        drop(copy);
        let matched = found == crate::wallet_backup::CopyCheck::Matches;
        if matched {
            self.copy_matched(fp.0, crate::plan::CopyForm::SeedQr);
        }
        self.backup.set_scanned(found.clone());
        if matched {
            self.camera.close();
            self.sheet = None;
            self.out.commands.push_back(Command::CameraOff);
        } else if let Some(scan) = self.camera.scan_mut() {
            scan.note = Some(crate::wallet_backup::copy_scan_line(&found));
        }
    }

    /// What one code read as a file holds: its name when it carries one,
    /// its extension and its bytes. `None` while a transfer in parts waits
    /// for more, and for a code that is refused, which leaves its reason
    /// as the scan's note.
    /// What it read is wiped once done with: it may be a seed.
    pub(crate) fn read_code(
        &mut self,
        bytes: zeroize::Zeroizing<Vec<u8>>,
    ) -> Option<(Option<String>, &'static str, Vec<u8>)> {
        let c = self.compute();
        let scan = self.camera.scan_mut()?;
        scan.reads += 1;
        let text = zeroize::Zeroizing::new(String::from_utf8_lossy(&bytes).trim().to_string());
        // A wrong arrival is named, not refused generically (`docs/QR.md`
        // §1): a seed in any form, a private key or a paperkey never goes
        // in a box.
        if let Some(why) = secret_code(&c, &bytes) {
            scan.note = Some(why.to_string());
            return None;
        }
        // BBQr and numbered parts are put together here; anything else is
        // read as before.
        let mut item: Option<(Option<String>, &str, Vec<u8>)> = None;
        match scan.assembler.feed(&text) {
            faraday_qr::Step::Part {
                what,
                have,
                total,
                missing,
            } => {
                scan.note = Some(crate::qr_view::part_note(
                    crate::transfer_name(what),
                    have,
                    total,
                    &missing,
                ));
                return None;
            }
            faraday_qr::Step::Refused(why) => {
                scan.note = Some(crate::qr_view::scan_refusal(why));
                return None;
            }
            faraday_qr::Step::Done(arrived) => item = Some(faraday_intake::arrival(arrived)),
            faraday_qr::Step::NotMine => {}
        }
        if item.is_none() && faraday_codec::ur::is_ur(&text) {
            let message = match crate::ur_feed(&mut scan.decoder, &text) {
                Ok(crate::UrFed::Whole(m)) => m,
                Ok(crate::UrFed::More { restarted }) => {
                    scan.note = crate::ur_note(&scan.decoder, restarted);
                    return None;
                }
                Err(why) => {
                    scan.note = Some(why.to_string());
                    return None;
                }
            };
            match message {
                faraday_codec::ur::Message::Psbt(p) => item = Some((None, "psbt", p)),
                faraday_codec::ur::Message::Bytes(b) => {
                    item = Some(faraday_intake::arrival(faraday_qr::classify_text(
                        &String::from_utf8_lossy(&b),
                    )));
                }
                faraday_codec::ur::Message::Other { ur_type, cbor } => {
                    match faraday_qr::registry::read(&ur_type, &cbor) {
                        Ok(Some(faraday_qr::registry::Read::Psbt(p))) => {
                            item = Some((None, "psbt", p));
                        }
                        Ok(Some(faraday_qr::registry::Read::Keys(t)))
                        | Ok(Some(faraday_qr::registry::Read::Descriptor(t))) => {
                            item = Some((None, "txt", t.into_bytes()));
                        }
                        Ok(None) => {
                            scan.note = Some(fill(EN.read_code_ur_unknown, &[&ur_type]));
                            return None;
                        }
                        Err(why) => {
                            scan.note = Some(crate::qr_view::registry_text(&why));
                            return None;
                        }
                    }
                }
            }
        }
        let (named, ext, data) = match item {
            Some(i) => i,
            None if crate::wallet::read_psbt(&c, &bytes).is_some() => {
                (None, "psbt", bytes.to_vec())
            }
            None => {
                // An address is checked against the loaded wallets, not
                // filed.
                if let Some(said) = self.address_note(&c, &text) {
                    if let Some(scan) = self.camera.scan_mut() {
                        scan.note = Some(said);
                    }
                    return None;
                }
                faraday_intake::arrival(faraday_qr::classify_text(&text))
            }
        };
        Some((named, ext, data))
    }
}
