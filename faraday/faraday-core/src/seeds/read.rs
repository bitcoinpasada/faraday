//! A seed's passphrase, asked after the seed (owner, 2026-10-11): the
//! choice of loading it without a passphrase or with one, typed twice
//! here, each with the key it makes. A SeedQR holds no passphrase, so the
//! scan sheet turns into this wherever a SeedQR is scanned: Add a key,
//! Home's Scan, a slot of Create or Restore, the Spend tab's words. A
//! key already loaded, or a vault's seed, taken into a slot of Restore or
//! the Spend tab with words and no passphrase asks the same: a vault may
//! keep a seed apart from its passphrase (decision 49). A wallet's key
//! scanner (`scan_key`) opens it only for a seed whose key with no
//! passphrase is none of the wallet's, and then takes only a missing key
//! of that wallet.
//!
//! The Spend tab's What do you have? opens it on every seed its intake
//! scanner reads (with **Not this one** where the seed is none of a
//! described wallet's keys, back to the camera), and on a seed already
//! in for its passphrase: when Check the money asks for one, or after
//! every kind was tried (`docs/history/SCAN-TO-SIGN.md` §4, §5).
//!
//! The words wait in memory, never drawn, until a choice loads them or
//! the sheet closes, which wipes them.

use crate::drawing::View;
use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::Compute;
use faraday_draw::tokens::{BODY_M, BODY_S, GAP as T_GAP, LABEL_L};
use faraday_draw::widgets::Icon;
use faraday_shell_api::Key as KeyIn;

use crate::qr_view::ScanPurpose;
use crate::scan_key::{Found, key_of};
use crate::screens::pass_field;
use crate::secret_text::SecretText;
use crate::seeds::sweep::{Signed, Sweep, SweepAction};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::wallet::fp_text;
use crate::{Action, Faraday, Screen, Sheet};

/// What the sheet's presses do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// Load the seed with no passphrase.
    Plain,
    /// Show the two passphrase fields.
    AddPass,
    /// Typing goes to passphrase field 0 or 1.
    Field(u8),
    /// Load the seed with the passphrase typed twice.
    WithPass,
    /// It is a Seed XOR part: Add a key's Seed XOR form opens with these
    /// words in (`docs/SCANNING.md` §6).
    Xor,
    /// Not this one: the words go, and the intake scanner comes back.
    Skip,
}

/// What the Spend tab opened the sheet for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForFamily {
    /// A seed the intake scanner read: the camera comes back after.
    Intake,
    /// The seed's passphrase, asked by Check the money: the guess goes
    /// on with it.
    Guess,
    /// The seed's passphrase, typed after every kind was tried: the seed
    /// is its key with the passphrase from now on.
    Replace,
    /// A seed that is none of the described wallet's keys: with a
    /// passphrase that gives one of them, it is that key.
    Stray,
}

/// The words a passphrase is asked for: the ones the camera read, or
/// those of a key already loaded, which stay in the session's keys and
/// are used there.
pub(crate) enum Source {
    /// Words the camera read, held here until a choice loads them or the
    /// sheet closes.
    Words(SecretText),
    /// The words of the loaded key with this fingerprint.
    Loaded(Fingerprint),
}

impl Default for Source {
    fn default() -> Self {
        Source::Words(SecretText::new())
    }
}

impl Source {
    /// Costs what `fingerprint_of_words` costs. The key these words make
    /// with `pass`.
    pub(crate) fn fingerprint(
        &self,
        c: &Compute,
        keys: &faraday_keys::Keys,
        pass: &str,
    ) -> Option<Fingerprint> {
        match self {
            Source::Words(w) => key_of(c, w, pass),
            Source::Loaded(fp) => keys.fingerprint_with(c, keys.position(*fp)?, pass),
        }
    }

    /// Costs what `add_words_with` costs. Adds the key these words make
    /// with `pass`, under `label`.
    pub(crate) fn add(
        &self,
        c: &Compute,
        keys: &mut faraday_keys::Keys,
        pass: &str,
        label: &str,
    ) -> Result<Fingerprint, crate::wallet::Refusal> {
        match self {
            Source::Words(w) => keys.add_words_with(c, w, pass, label, None),
            Source::Loaded(fp) => match keys.position(*fp) {
                Some(k) => keys.add_with_passphrase(c, k, pass, label),
                None => Err(crate::wallet::Refusal::Words(String::new())),
            },
        }
    }
}

/// The seed read, waiting for its choice.
#[derive(Default)]
pub struct SeedRead {
    /// Its words.
    source: Source,
    /// A key already loaded from these words with no passphrase, taken
    /// into a slot; none for words the camera read, which load as Add a
    /// key loads typed words.
    loaded: Option<Fingerprint>,
    /// The key it makes with no passphrase.
    plain: Option<Fingerprint>,
    /// Add a passphrase was pressed: the fields show.
    pub adding: bool,
    /// The passphrase.
    pub pass: SecretText,
    /// The passphrase again.
    pub pass2: SecretText,
    /// Which field typing goes to.
    pub focus: Option<u8>,
    /// Why the last choice did not load.
    pub error: Option<String>,
    /// For a seed read by a wallet's key scanner: the wallet, whose
    /// missing keys alone it may load as.
    wallet: Option<usize>,
    /// Why the sheet asks: the seed with no passphrase is no key of that
    /// wallet.
    reason: Option<String>,
    /// Open by the Spend tab, and what for.
    pub family: Option<ForFamily>,
    /// The key the passphrase typed twice makes ([`SeedRead::with_key`]):
    /// worked out once typing that changes the fields has paused
    /// (`Faraday::seed_read_typed`), never while drawing, and none while
    /// the person types. A fingerprint: public.
    with: Option<Fingerprint>,
    /// The form the seed was read in, for the sheet's title.
    read: Option<crate::scan_read::Kind>,
    /// "Key 2 of Savings": the loaded wallet that has the key, worked out
    /// when the seed was read.
    key_of: Option<String>,
    /// Twelve or 24 words read: the sheet offers **It is a Seed XOR part**.
    xor: bool,
}

impl SeedRead {
    /// The two fields hold the same passphrase, and it is not empty.
    fn pass_ready(&self) -> bool {
        !self.pass.is_empty() && self.pass.as_str() == self.pass2.as_str()
    }

    /// The key the passphrase typed makes, once it is typed twice alike:
    /// worked out when it was typed. A lookup.
    pub(crate) fn with_key(&self) -> Option<Fingerprint> {
        self.with
    }

    /// Costs a PBKDF2 and an elliptic-curve multiplication once they match.
    /// Works out [`SeedRead::with_key`] from the fields as they are.
    fn with_key_now(&self, c: &Compute, keys: &faraday_keys::Keys) -> Option<Fingerprint> {
        self.pass_ready()
            .then(|| self.source.fingerprint(c, keys, &self.pass))
            .flatten()
    }

    /// The text typing goes to.
    pub(crate) fn field(&self) -> Option<&SecretText> {
        match self.focus? {
            0 => Some(&self.pass),
            _ => Some(&self.pass2),
        }
    }
}

impl Faraday {
    /// The seed reader's sheet, while it is open.
    pub fn seed_read(&self) -> Option<&SeedRead> {
        self.seed_read.state()
    }

    /// The camera read a seed's words: the camera closes and the sheet
    /// asks how it loads. Words that are not a BIP-39 seed go to Add a
    /// key as typed words, which says what is wrong with them.
    pub(crate) fn seed_read_open(&mut self, words: &str) {
        let c = self.compute();
        self.camera.close();
        self.out
            .commands
            .push_back(faraday_shell_api::Command::CameraOff);
        let Some(plain) = key_of(&c, words, "") else {
            self.sheet = None;
            self.seed_read_load(words, "");
            return;
        };
        self.seed_read.state = Some(SeedRead {
            source: Source::Words(SecretText::of(words)),
            plain: Some(plain),
            ..SeedRead::default()
        });
        self.sheet = Some(Sheet::SeedRead);
    }

    /// Wallet `w`'s key scanner read a seed whose key with no passphrase
    /// is none of the wallet's: the camera closes and the sheet asks for
    /// its passphrase, saying why.
    pub(crate) fn seed_read_for_wallet(&mut self, words: &str, w: usize, reason: String) {
        let c = self.compute();
        self.camera.close();
        self.out
            .commands
            .push_back(faraday_shell_api::Command::CameraOff);
        self.seed_read.state = Some(SeedRead {
            source: Source::Words(SecretText::of(words)),
            plain: key_of(&c, words, ""),
            wallet: Some(w),
            reason: Some(reason),
            ..SeedRead::default()
        });
        self.sheet = Some(Sheet::SeedRead);
    }

    /// Key `fp`, loaded, is being taken into a slot of Restore or the
    /// Spend tab: with words and no passphrase it asks first whether it
    /// has one. Returns whether it asked; else the slot takes it as is.
    pub(crate) fn seed_read_loaded(&mut self, fp: Fingerprint) -> bool {
        let Some(key) = self.session.keys.find(fp) else {
            return false;
        };
        if !key.has_words() || key.has_passphrase {
            return false;
        }
        self.seed_read.state = Some(SeedRead {
            source: Source::Loaded(fp),
            loaded: Some(fp),
            plain: Some(fp),
            ..SeedRead::default()
        });
        self.sheet = Some(Sheet::SeedRead);
        true
    }

    /// Key `fp`, loaded with its words, asked for its passphrase for the
    /// Spend tab.
    pub(crate) fn seed_read_family(&mut self, fp: Fingerprint, why: ForFamily) {
        if !self.session.keys.find(fp).is_some_and(|k| k.has_words()) {
            return;
        }
        let start_adding = why != ForFamily::Intake;
        self.seed_read.state = Some(SeedRead {
            source: Source::Loaded(fp),
            loaded: Some(fp),
            plain: Some(fp),
            adding: start_adding,
            focus: start_adding.then_some(0),
            family: Some(why),
            ..SeedRead::default()
        });
        self.sheet = Some(Sheet::SeedRead);
    }

    /// Key `fp`, loaded, is none of described wallet `w`'s keys: its
    /// passphrase is asked, and only a key of the wallet loads.
    pub(crate) fn seed_read_stray(&mut self, fp: Fingerprint, w: usize, reason: String) {
        self.seed_read_family(fp, ForFamily::Stray);
        if let Some(s) = self.seed_read.state.as_mut() {
            s.wallet = Some(w);
            s.reason = Some(reason);
        }
    }

    pub(crate) fn seed_read_act(&mut self, choice: Choice) {
        let Some(s) = self.seed_read.state.as_mut() else {
            return;
        };
        match choice {
            Choice::Skip => {
                self.seed_read.state = None;
                self.sheet = None;
                self.family_intake_again(None);
            }
            Choice::AddPass => {
                s.adding = true;
                s.focus = Some(0);
                s.error = None;
            }
            Choice::Field(f) => s.focus = Some(f.min(1)),
            Choice::Plain => self.seed_read_choose(false),
            Choice::Xor => self.seed_read_xor(),
            Choice::WithPass => {
                if s.pass.as_str() != s.pass2.as_str() {
                    s.error = Some(EN.seed_read_the_two_passphrases.to_string());
                } else if s.pass.is_empty() {
                    s.error = Some(EN.seed_read_type_passphrase.to_string());
                } else {
                    self.seed_read_choose(true);
                }
            }
        }
    }

    /// The seed sheet just opened on words read as `kind`: its title names
    /// the form, a loaded wallet that has the key is named, and twelve or
    /// 24 words offer **It is a Seed XOR part** (`docs/SCANNING.md` §6).
    /// A lookup per loaded wallet's keys.
    pub(crate) fn seed_read_marked(&mut self, kind: crate::scan_read::Kind) {
        let Some(s) = self.seed_read.state.as_ref() else {
            return;
        };
        let words = match &s.source {
            Source::Words(t) => t.as_str().split_whitespace().count(),
            Source::Loaded(_) => 0,
        };
        let key_of = s.plain.and_then(|fp| {
            (0..self.session.wallets.len()).find_map(|w| match self.wallet_key_found(w, fp) {
                Found::Missing(n) | Found::Here(n) => Some(fill(
                    EN.seed_read_key_of,
                    &[&(n + 1).to_string(), &self.session.wallets[w].name],
                )),
                Found::None => None,
            })
        });
        let plain_sheet = s.wallet.is_none() && s.family.is_none();
        if let Some(s) = self.seed_read.state.as_mut() {
            s.read = Some(kind);
            s.key_of = key_of;
            s.xor = plain_sheet && matches!(words, 12 | 24);
        }
    }

    /// Costs what adding a Seed XOR part costs. **It is a Seed XOR part**:
    /// the sheet closes and Add a key's Seed XOR form opens with the words
    /// in, the part's line under them.
    fn seed_read_xor(&mut self) {
        let Some(s) = self.seed_read.state.take() else {
            return;
        };
        let Source::Words(words) = &s.source else {
            return;
        };
        self.sheet = None;
        if self.screen != Screen::Entry {
            self.act(Action::Entry(None));
        }
        let xor = crate::forms::Form::ALL
            .iter()
            .position(|f| *f == crate::forms::Form::Xor);
        if let Some(i) = xor {
            self.act(Action::EntryForm(i as u8));
        }
        self.entry.state_mut().typed.set(words.as_str());
        self.form_add_part();
    }

    /// Loads the seed as chosen, unless Restore already knows the wallet
    /// and the key is not one of its keys: then the sheet says so and
    /// stays, for the passphrase to be typed again.
    fn seed_read_choose(&mut self, with_pass: bool) {
        let c = self.compute();
        let Some(s) = self.seed_read.state.as_ref() else {
            return;
        };
        let pass = if with_pass {
            s.pass.clone()
        } else {
            SecretText::new()
        };
        let Some(fp) = s.source.fingerprint(&c, &self.session.keys, &pass) else {
            return;
        };
        if let Some(w) = s.wallet {
            self.seed_read_wallet(w, fp, &pass);
            return;
        }
        // Check the money's passphrase, or one typed after every kind:
        // the seed in place becomes its key with it.
        if let (Some(why @ (ForFamily::Guess | ForFamily::Replace)), Some(plain)) =
            (s.family, s.loaded)
        {
            let Some(s) = self.seed_read.state.take() else {
                return;
            };
            self.sheet = None;
            if why == ForFamily::Guess {
                self.family_guess_pass(plain, &s.source, &pass);
            } else if !pass.is_empty() {
                self.family_seed_with(plain, &s.source, &pass);
            }
            self.family_settle();
            return;
        }
        let restoring = self.entry.state().back == Some(Screen::Restore)
            || (s.loaded.is_some() && self.screen == Screen::Restore);
        if restoring && let Some(why) = self.restore_refusal(fp, None) {
            if let Some(s) = self.seed_read.state.as_mut() {
                s.error = Some(why);
            }
            return;
        }
        let Some(s) = self.seed_read.state.take() else {
            return;
        };
        self.sheet = None;
        match (s.loaded, &s.source) {
            (None, Source::Words(w)) => self.seed_read_load(w, &pass),
            (None, Source::Loaded(_)) => {}
            (Some(plain), source) => self.seed_read_take(plain, source, &pass),
        }
        // Read by the intake scanner and loaded: the camera again.
        if s.family == Some(ForFamily::Intake) && self.screen == Screen::Family {
            self.family_intake_again(Some(fp));
        }
    }

    /// The choice made on a seed from wallet `w`'s key scanner, whose key
    /// is `fp`: loaded for a missing key of the wallet, else refused and
    /// the sheet stays.
    fn seed_read_wallet(&mut self, w: usize, fp: Fingerprint, pass: &str) {
        let c = self.compute();
        let refused = match self.wallet_key_found(w, fp) {
            Found::Missing(n) => {
                let Some(s) = self.seed_read.state.take() else {
                    return;
                };
                self.sheet = None;
                let label = self.wallet_key_label(&c, w, fp);
                match s.source.add(&c, &mut self.session.keys, pass, &label) {
                    Ok(_) => {
                        // A seed that came in on What do you have? is
                        // this key from now on.
                        if let Some(plain) = s.plain.filter(|_| s.family.is_some()) {
                            self.family.seed_became(plain.0, fp.0);
                            self.family_came(crate::family::Came::Seed(fp.0));
                        }
                        self.wallet_key_added(w, n);
                        if s.family == Some(ForFamily::Intake) {
                            self.family_intake_again(Some(fp));
                        }
                    }
                    Err(e) => self.toast(&e.text()),
                }
                return;
            }
            Found::Here(n) => fill(EN.seed_read_key_already, &[&(n + 1).to_string()]),
            Found::None => self.not_a_wallet_key(w, fp),
        };
        if let Some(s) = self.seed_read.state.as_mut() {
            s.error = Some(refused);
        }
    }

    /// A loaded key taken into the slot: itself, or with the passphrase
    /// the key its words and it make, added beside it.
    fn seed_read_take(&mut self, plain: Fingerprint, source: &Source, pass: &str) {
        let c = self.compute();
        let fp = if pass.is_empty() {
            plain
        } else {
            let label = self
                .session
                .keys
                .find(plain)
                .map(|k| fill(EN.seed_read_key_passphrase, &[&k.label.to_string()]))
                .unwrap_or_default();
            match source.add(&c, &mut self.session.keys, pass, &label) {
                Ok(fp) => fp,
                Err(crate::wallet::Refusal::Duplicate(_)) => {
                    match source.fingerprint(&c, &self.session.keys, pass) {
                        Some(fp) => fp,
                        None => return,
                    }
                }
                Err(e) => {
                    self.toast(&e.text());
                    return;
                }
            }
        };
        let back = Some(self.screen);
        self.seeds_took(fp, back);
        if self.screen == Screen::Family {
            self.family_key_added();
        }
    }

    /// The words and passphrase added as Add a key adds typed words, not
    /// through the word keyboard, which a small panel gets back if they
    /// are refused.
    fn seed_read_load(&mut self, words: &str, pass: &str) {
        self.entry.state_mut().typed.set(words);
        self.entry.state_mut().passphrase.set(pass);
        let keys = self.entry.state_mut().keys.take();
        self.entry_add();
        if self.screen == Screen::Entry && keys.is_some() {
            self.entry.state_mut().keys =
                Some(crate::forms::word_typer(self.entry.state().language()));
        }
    }

    /// Keys typed into the sheet's passphrase fields. Returns whether they
    /// took the key.
    pub(crate) fn seed_read_key(&mut self, key: KeyIn) -> bool {
        if self.sheet != Some(Sheet::SeedRead) {
            return false;
        }
        let Some(s) = self.seed_read.state.as_mut() else {
            return false;
        };
        let Some(f) = s.focus else {
            if key == KeyIn::Enter {
                self.seed_read_act(Choice::Plain);
                return true;
            }
            return false;
        };
        let field = if f == 0 { &mut s.pass } else { &mut s.pass2 };
        match key {
            // BIP-39 passphrases here are printable ASCII (BIP-39's seed derivation).
            KeyIn::Char(ch) if (' '..='~').contains(&ch) => field.push(ch),
            // Anything else is refused, and the sheet says so
            // (`docs/STANDARDS.md`).
            KeyIn::Char(c) if !c.is_control() => {
                s.error = Some(crate::state::ascii_refused());
                return true;
            }
            KeyIn::Backspace => {
                field.pop();
            }
            KeyIn::Tab | KeyIn::BackTab => s.focus = Some(1 - f.min(1)),
            KeyIn::Escape => s.focus = None,
            KeyIn::Enter => {
                self.seed_read_act(Choice::WithPass);
                return true;
            }
            _ => return false,
        }
        // The key the two fields make is worked out once typing pauses
        // (`Faraday::typed_tick`); none is shown until then.
        s.with = None;
        s.error = None;
        self.inbox
            .set_typed_at(s.pass_ready().then_some(self.now_ms));
        true
    }

    /// Field `i` of the seed read's passphrase pair was filled by a press
    /// (Practice's `faraday` chip): focused, and the key the two make is
    /// worked out once typing pauses, as when typed.
    pub(crate) fn seed_read_filled(&mut self, i: u8) {
        if let Some(s) = self.seed_read.state.as_mut() {
            let t = if i == 0 { &mut s.pass } else { &mut s.pass2 };
            t.set(crate::demo::PASSPHRASE);
            s.focus = Some(i);
            s.with = None;
            s.error = None;
            self.inbox
                .set_typed_at(s.pass_ready().then_some(self.now_ms));
        }
    }

    /// Works out the key the passphrase typed twice makes, typing having
    /// paused. Costs a PBKDF2 and an elliptic-curve multiplication when
    /// the two fields match.
    pub(crate) fn seed_read_typed(&mut self) {
        let c = self.compute();
        if let Some(s) = self.seed_read.state.as_mut() {
            s.with = s.with_key_now(&c, &self.session.keys);
        }
    }
}

impl Faraday {
    /// The Sweep sheet's state, while it is open (`seeds::sweep`).
    pub fn sweep(&self) -> Option<&Sweep> {
        self.seed_read.sweep()
    }

    /// Whether typing goes to the Sweep sheet's field.
    pub(crate) fn sweep_typing(&self) -> bool {
        self.sheet == Some(Sheet::Sweep) && self.sweep().is_some_and(|s| s.typing)
    }

    /// Whether the scanner up is the Sweep sheet's own.
    pub(crate) fn sweep_scanning(&self) -> bool {
        self.sheet == Some(Sheet::Scan)
            && self.seed_read.sweep.is_some()
            && self
                .camera
                .scan()
                .is_some_and(|s| s.purpose == ScanPurpose::Sweep)
    }

    /// The sweep is left: the sheet and its scanner are gone, so the
    /// key is erased and the sheet forgotten. Run after every press.
    pub(crate) fn sweep_tidy(&mut self) {
        if self.sheet == Some(Sheet::Sweep) || self.sweep_scanning() {
            return;
        }
        if self.seed_read.sweep.is_some() || self.session.keys.sweep().is_some() {
            self.seed_read.sweep = None;
            self.session.keys.sweep_forget();
        }
    }

    /// The camera closes and the Sweep sheet is up again.
    pub(crate) fn sweep_back(&mut self) {
        if self.camera.take().is_some() {
            self.out
                .commands
                .push_back(faraday_shell_api::Command::CameraOff);
        }
        self.sheet = Some(Sheet::Sweep);
    }

    /// Handles the Sweep sheet's presses (`seeds::sweep`).
    pub(crate) fn sweep_act(&mut self, a: SweepAction) {
        match a {
            SweepAction::Open => {
                if !self.makes_secrets() {
                    self.toast(EN.sweep_practice);
                    return;
                }
                self.session.keys.sweep_forget();
                self.seed_read.sweep = Some(Sweep {
                    typing: true,
                    ..Sweep::default()
                });
                self.sheet = Some(Sheet::Sweep);
            }
            SweepAction::Field => {
                if let Some(s) = self.seed_read.sweep.as_mut() {
                    s.typing = true;
                }
            }
            SweepAction::Use => {
                let Some(s) = self.seed_read.sweep.as_mut() else {
                    return;
                };
                let typed = std::mem::take(&mut s.typed);
                if !typed.is_empty() {
                    self.sweep_take(typed.as_str());
                }
            }
            SweepAction::Scan => {
                if self.seed_read.sweep.is_none() {
                    return;
                }
                let mut scan = crate::qr_view::ScanState::default();
                scan.purpose = ScanPurpose::Sweep;
                self.camera.open(scan);
                self.sheet = Some(Sheet::Scan);
                self.out
                    .commands
                    .push_back(faraday_shell_api::Command::CameraOn);
            }
            SweepAction::Back => self.sweep_back(),
            SweepAction::Hold => self.sweep_sign(),
            SweepAction::ShowQr => {
                let c = self.compute();
                let Some(done) = self
                    .seed_read
                    .sweep
                    .as_ref()
                    .and_then(|s| s.signed.as_ref())
                else {
                    return;
                };
                let view = match &done.hex {
                    Some(hex) => crate::QrView::text(&c, EN.sweep_qr_transaction, hex),
                    None => crate::QrView::psbt(&c, EN.sweep_qr_psbt, &done.psbt),
                };
                self.open_qr(view);
            }
            SweepAction::Done => {
                self.seed_read.sweep = None;
                self.session.keys.sweep_forget();
                if self.sheet == Some(Sheet::Sweep) {
                    self.sheet = None;
                }
            }
        }
    }

    /// Costs what reading a WIF costs. The WIF in `text`, typed or
    /// scanned, becomes the key the sheet sweeps, the sheet opening on
    /// its addresses; else the sheet, or the scanner, says why not.
    fn sweep_take(&mut self, text: &str) {
        let c = self.compute();
        let why = if self.makes_secrets() {
            match self.session.keys.sweep_read(&c, text) {
                Ok(()) => None,
                Err(r) => Some(crate::seeds::sweep::refusal_text(
                    r,
                    self.session.network(),
                    self.medium,
                )),
            }
        } else {
            Some(EN.sweep_practice.to_string())
        };
        if let Some(why) = why {
            match self
                .camera
                .scan_mut()
                .filter(|_| self.sheet == Some(Sheet::Scan))
            {
                Some(scan) => scan.note = Some(why),
                None => {
                    if let Some(s) = self.seed_read.sweep.as_mut() {
                        s.error = Some(why);
                    }
                }
            }
            return;
        }
        self.seed_read.sweep = Some(Sweep::default());
        self.sweep_back();
    }

    /// Costs what reading a WIF costs. The read card's **Sweep a private
    /// key** (`docs/SCANNING.md` §4.2): the Sweep sheet opens on the key
    /// read, as Home's Scan opens it.
    pub(crate) fn sweep_from_card(&mut self, text: &str) {
        self.seed_read.sweep = Some(Sweep::default());
        self.sweep_take(text);
        if self.session.keys.sweep().is_none() {
            self.seed_read.sweep = None;
        }
    }

    /// Costs a WIF's base58 checksum, and what reading the key or the
    /// PSBT costs. A code the camera read, or text pasted on its
    /// scanner, where a private key may be: the Sweep sheet's scanner
    /// takes the key, and once it is read the PSBT; Home's Scan opens
    /// the Sweep sheet on a key; every other scanner refuses one, and
    /// files nothing. Returns whether the code was the sweep's.
    pub(crate) fn sweep_scanned(&mut self, bytes: &zeroize::Zeroizing<Vec<u8>>) -> bool {
        let Some(purpose) = self.camera.scan().map(|s| s.purpose) else {
            return false;
        };
        let text = zeroize::Zeroizing::new(String::from_utf8_lossy(bytes).trim().to_string());
        let wif = faraday_keys::identity::is_wif(&text);
        let home = purpose == ScanPurpose::Files
            && self.screen == Screen::Home
            && self.sheet == Some(Sheet::Scan);
        match purpose {
            ScanPurpose::Sweep if wif => self.sweep_take(&text),
            ScanPurpose::Sweep if self.session.keys.sweep().is_some() => {
                self.sweep_psbt(bytes.clone());
            }
            ScanPurpose::Sweep => {
                if let Some(scan) = self.camera.scan_mut() {
                    scan.note = Some(EN.wif_not_a_key.to_string());
                }
            }
            _ if wif && home => {
                self.seed_read.sweep = Some(Sweep::default());
                self.sweep_take(&text);
                if self.session.keys.sweep().is_none() {
                    self.seed_read.sweep = None;
                }
            }
            // A scanner that shows the read card hands it over there
            // (`docs/SCANNING.md` §4.2).
            _ if wif && self.read_card_here() => return false,
            _ if wif => {
                if let Some(scan) = self.camera.scan_mut() {
                    scan.note = Some(EN.sweep_elsewhere.to_string());
                }
            }
            _ => return false,
        }
        true
    }

    /// Costs what reading the code and the review cost. A code read on
    /// the sweep's scanner once its key is in: a PSBT (in parts as a
    /// file of the scanner's is) closes the camera and its review opens
    /// on the sheet; anything else is named on the scanner.
    fn sweep_psbt(&mut self, bytes: zeroize::Zeroizing<Vec<u8>>) {
        let c = self.compute();
        let Some((_, _, data)) = self.read_code(bytes) else {
            return;
        };
        let Some(psbt) = crate::wallet::read_psbt_kept(&c, &data) else {
            if let Some(scan) = self.camera.scan_mut() {
                scan.note = Some(EN.sweep_not_psbt.to_string());
            }
            return;
        };
        let review = crate::seeds::sweep::review(&c, &self.session, psbt);
        if let Some(s) = self.seed_read.sweep.as_mut() {
            s.review = Some(review);
            s.signed = None;
            s.error = None;
        }
        self.sweep_back();
    }

    /// Costs what `Keys::sweep_sign` and finalizing cost. The hold: the
    /// key's inputs are signed past the holdable dangers its review
    /// shows; the signed PSBT, and the finished transaction when every
    /// input is signed, go to the Outbox.
    fn sweep_sign(&mut self) {
        let c = self.compute();
        let Some(r) = self
            .seed_read
            .sweep
            .as_ref()
            .and_then(|s| s.review.as_ref())
        else {
            return;
        };
        if r.stop.is_some() {
            return;
        }
        let (mut psbt, accepted, v2) = (r.psbt.clone(), r.accepted.clone(), r.v2.clone());
        let refs = self.session.key_refs(&c);
        let policies = self.session.policies();
        let ctx = faraday_psbt::Context {
            network: self.session.network(),
            keys: &refs,
            wallets: &policies,
            musig_session: None,
        };
        let signed = self
            .session
            .keys
            .sweep_sign(&c, &mut psbt, (ctx, &accepted));
        let result = match signed {
            Some(Ok(result)) => result,
            Some(Err(e)) => {
                if let Some(s) = self.seed_read.sweep.as_mut() {
                    s.error = Some(crate::review::sign_error_text(&e));
                }
                return;
            }
            None => return,
        };
        // In the version it came in, or nothing written (I10).
        let Ok(bytes) = crate::seeds::sweep::out_bytes(v2.as_ref(), &psbt) else {
            return self.toast(EN.spend_v2_not_written);
        };
        let n = result.signed_inputs.len().to_string();
        let mut done = psbt.clone();
        let hex = match faraday_psbt::finalize(&mut done) {
            Ok(Some(tx)) => Some(faraday_psbt::bitcoin::consensus::encode::serialize_hex(&tx)),
            _ => None,
        };
        self.put_outbox("sweep-signed.psbt", bytes.clone());
        let line = match &hex {
            Some(h) => {
                self.put_outbox("sweep-final.txn", h.clone().into_bytes());
                fill(EN.sweep_signed_final, &[&n])
            }
            None => fill(EN.sweep_signed_part, &[&n]),
        };
        if let Some(s) = self.seed_read.sweep.as_mut() {
            s.signed = Some(Signed {
                line,
                psbt: bytes,
                hex,
            });
            s.error = None;
        }
    }

    /// Keys typed into the Sweep sheet's field: the characters a WIF is
    /// written in, Backspace, Enter to read it, Escape to leave the
    /// field. Returns whether they took the key.
    pub(crate) fn sweep_key(&mut self, key: KeyIn) -> bool {
        if !self.sweep_typing() {
            return false;
        }
        let Some(s) = self.seed_read.sweep.as_mut() else {
            return false;
        };
        match key {
            KeyIn::Char(ch) if ch.is_ascii_alphanumeric() => s.typed.push(ch),
            KeyIn::Char(_) => {}
            KeyIn::Backspace => {
                s.typed.pop();
            }
            KeyIn::Escape => s.typing = false,
            KeyIn::Enter => {
                self.sweep_act(SweepAction::Use);
                return true;
            }
            _ => return false,
        }
        s.error = None;
        true
    }
}

/// The sheet: the key the seed makes without a passphrase and with the
/// one typed, and the way it loads.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(s) = app.seed_read.state() else {
        return;
    };
    let with = s.with_key();
    let differ = !s.pass.is_empty() && !s.pass2.is_empty() && s.pass.as_str() != s.pass2.as_str();
    let use_with = Style::Primary.when(s.pass_ready());
    let skip = s.family == Some(ForFamily::Intake) && s.wallet.is_some();
    let cancel = if skip {
        (
            EN.seed_read_not_this_one,
            Style::Ghost,
            Action::SeedRead(Choice::Skip),
        )
    } else {
        (EN.seed_read_cancel, Style::Ghost, Action::Cancel)
    };
    let mut items: Vec<(&str, Style, Action)> = if s.adding {
        vec![
            (
                EN.seed_read_use_passphrase,
                use_with,
                Action::SeedRead(Choice::WithPass),
            ),
            (
                EN.seed_read_no_passphrase,
                Style::Secondary,
                Action::SeedRead(Choice::Plain),
            ),
        ]
    } else {
        vec![
            (
                EN.seed_read_use_without,
                Style::Primary,
                Action::SeedRead(Choice::Plain),
            ),
            (
                EN.seed_read_add_a_passphrase,
                Style::Secondary,
                Action::SeedRead(Choice::AddPass),
            ),
        ]
    };
    // The dismissive action first, the confirming one last (Material's
    // dialog).
    items.reverse();
    if s.xor && !s.adding {
        items.insert(
            0,
            (EN.read_go_xor, Style::Ghost, Action::SeedRead(Choice::Xor)),
        );
    }
    items.insert(0, cancel);
    // A key's line: what it is made with, and its fingerprint.
    let key_line = |ui: &mut Ui, x: f32, y: f32, iw: f32, label: &str, fp: Fingerprint| -> f32 {
        ui.text_mid_as((x, y, 30.0), BODY_S, Role::OnSurfaceVariant, label);
        ui.text_right_as(
            (x + iw, y, 30.0),
            BODY_M.mono(),
            Role::OnSurface,
            &fp_text(fp),
        );
        34.0
    };
    let body = |ui: &mut Ui, x: f32, y: f32, iw: f32| -> f32 {
        let mut cy = y;
        if let Some(fp) = s.plain {
            cy += key_line(ui, x, cy, iw, EN.seed_read_no_passphrase, fp);
        }
        if let Some(line) = &s.key_of {
            cy += ui.wrap_as((x, cy, iw), BODY_M, Role::OnSurface, line) + T_GAP;
        }
        if s.adding {
            cy += 6.0;
            cy += pass_field(
                ui,
                x,
                cy,
                iw,
                EN.seed_read_passphrase_label,
                &s.pass,
                (
                    s.focus == Some(0),
                    app.pass_chip(crate::demo::PassField::SeedRead(0)),
                ),
                Action::SeedRead(Choice::Field(0)),
            );
            cy += pass_field(
                ui,
                x,
                cy,
                iw,
                EN.seed_read_passphrase,
                &s.pass2,
                (
                    s.focus == Some(1),
                    app.pass_chip(crate::demo::PassField::SeedRead(1)),
                ),
                Action::SeedRead(Choice::Field(1)),
            );
            cy += crate::screens::never_real_seed(app, ui, x, cy, iw);
            if let Some(fp) = with {
                cy += key_line(ui, x, cy, iw, EN.seed_read_with_passphrase, fp);
            } else if differ {
                cy += ui.wrap_as(
                    (x, cy, iw),
                    BODY_S,
                    Role::Warning,
                    EN.seed_read_the_two_passphrases,
                ) + 10.0;
            }
        }
        if let Some(e) = &s.error {
            cy += ui.wrap_as((x, cy + 4.0, iw), LABEL_L, Role::Error, e) + 14.0;
        } else if let Some(r) = &s.reason {
            cy += ui.wrap_as((x, cy + 4.0, iw), LABEL_L, Role::Warning, r) + 14.0;
        }
        cy - y
    };
    let title = match s.read {
        Some(crate::scan_read::Kind::SeedWords) => EN.seed_read_words_read,
        Some(crate::scan_read::Kind::CompactSeedQr) => EN.seed_read_compact_read,
        _ => EN.seed_read_seedqr_read,
    };
    let head = (Icon::Keys.into(), Role::Primary.paint(), title);
    crate::compact::dialog(
        ui,
        (w, h, faraday_draw::tokens::DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| body(ui, x, y, iw),
        &items,
    );
}
