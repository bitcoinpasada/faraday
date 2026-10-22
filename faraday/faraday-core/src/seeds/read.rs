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
//! every kind was tried (`docs/SCAN-TO-SIGN.md` §4, §5).
//!
//! The words wait in memory, never drawn, until a choice loads them or
//! the sheet closes, which wipes them.

use crate::drawing::View;
use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::Compute;
use faraday_draw::widgets::Icon;
use faraday_shell_api::Key as KeyIn;

use crate::scan_key::{Found, key_of};
use crate::screens::{pass_field, sheet_box, wrap_buttons};
use crate::secret_text::SecretText;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
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
    /// The camera read a seed's words: the camera closes and the sheet
    /// asks how it loads. Words that are not a BIP-39 seed go to Add a
    /// key as typed words, which says what is wrong with them.
    pub(crate) fn seed_read_open(&mut self, words: &str) {
        let c = self.compute();
        self.scan = None;
        self.commands
            .push_back(faraday_shell_api::Command::CameraOff);
        let Some(plain) = key_of(&c, words, "") else {
            self.sheet = None;
            self.seed_read_load(words, "");
            return;
        };
        self.seed_read = Some(SeedRead {
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
        self.scan = None;
        self.commands
            .push_back(faraday_shell_api::Command::CameraOff);
        self.seed_read = Some(SeedRead {
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
        self.seed_read = Some(SeedRead {
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
        self.seed_read = Some(SeedRead {
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
        if let Some(s) = self.seed_read.as_mut() {
            s.wallet = Some(w);
            s.reason = Some(reason);
        }
    }

    pub(crate) fn seed_read_act(&mut self, choice: Choice) {
        let Some(s) = self.seed_read.as_mut() else {
            return;
        };
        match choice {
            Choice::Skip => {
                self.seed_read = None;
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
            Choice::WithPass => {
                if s.pass.as_str() != s.pass2.as_str() {
                    s.error = Some(EN.seed_read_the_two_passphrases.to_string());
                } else if s.pass.is_empty() {
                    s.error = Some(EN.seed_read_type_the.to_string());
                } else {
                    self.seed_read_choose(true);
                }
            }
        }
    }

    /// Loads the seed as chosen, unless Restore already knows the wallet
    /// and the key is not one of its keys: then the sheet says so and
    /// stays, for the passphrase to be typed again.
    fn seed_read_choose(&mut self, with_pass: bool) {
        let c = self.compute();
        let Some(s) = self.seed_read.as_ref() else {
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
            let Some(s) = self.seed_read.take() else {
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
        let restoring = self.entry.back == Some(Screen::Restore)
            || (s.loaded.is_some() && self.screen == Screen::Restore);
        if restoring && let Some(why) = self.restore_refusal(fp) {
            if let Some(s) = self.seed_read.as_mut() {
                s.error = Some(why);
            }
            return;
        }
        let Some(s) = self.seed_read.take() else {
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
                let Some(s) = self.seed_read.take() else {
                    return;
                };
                self.sheet = None;
                let label = self.wallet_key_label(&c, w, fp);
                match s.source.add(&c, &mut self.session.keys, pass, &label) {
                    Ok(_) => {
                        // A seed that came in on What do you have? is
                        // this key from now on.
                        if let Some(plain) = s.plain.filter(|_| s.family.is_some()) {
                            for c in self.family.came.iter_mut() {
                                if *c == crate::family::Came::Seed(plain.0) {
                                    *c = crate::family::Came::Seed(fp.0);
                                }
                            }
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
        if let Some(s) = self.seed_read.as_mut() {
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
                .map(|k| fill(EN.seed_read_passphrase_3, &[&k.label.to_string()]))
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
        self.entry.typed.set(words);
        self.entry.passphrase.set(pass);
        let keys = self.entry.keys.take();
        self.entry_add();
        if self.screen == Screen::Entry && keys.is_some() {
            self.entry.keys = Some(crate::forms::word_typer(self.entry.language()));
        }
    }

    /// Keys typed into the sheet's passphrase fields. Returns whether they
    /// took the key.
    pub(crate) fn seed_read_key(&mut self, key: KeyIn) -> bool {
        if self.sheet != Some(Sheet::SeedRead) {
            return false;
        }
        let Some(s) = self.seed_read.as_mut() else {
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
        self.typed_at = s.pass_ready().then_some(self.now_ms);
        true
    }

    /// Works out the key the passphrase typed twice makes, typing having
    /// paused. Costs a PBKDF2 and an elliptic-curve multiplication when
    /// the two fields match.
    pub(crate) fn seed_read_typed(&mut self) {
        let c = self.compute();
        if let Some(s) = self.seed_read.as_mut() {
            s.with = s.with_key_now(&c, &self.session.keys);
        }
    }
}

/// The sheet: the key the seed makes without a passphrase and with the
/// one typed, and the way it loads.
pub(crate) fn sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(s) = app.seed_read.as_ref() else {
        return;
    };
    let with = s.with_key();
    let differ = !s.pass.is_empty() && !s.pass2.is_empty() && s.pass.as_str() != s.pass2.as_str();
    let use_with = if s.pass_ready() {
        Style::Primary
    } else {
        Style::Disabled
    };
    let skip = s.family == Some(ForFamily::Intake) && s.wallet.is_some();
    let cancel = if skip {
        (
            EN.seed_read_not_this,
            Style::Ghost,
            Action::SeedRead(Choice::Skip),
        )
    } else {
        (EN.seed_read_cancel, Style::Ghost, Action::Cancel)
    };
    let mut items: Vec<(&str, Style, Action)> = if s.adding {
        vec![
            (
                EN.seed_read_use_with,
                use_with,
                Action::SeedRead(Choice::WithPass),
            ),
            (
                EN.seed_read_without_a,
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
    items.push(cancel);
    // A key's line: what it is made with, and its fingerprint.
    let key_line = |ui: &mut Ui, x: f32, y: f32, iw: f32, label: &str, fp: Fingerprint| -> f32 {
        ui.text_mid(x, y, 30.0, 13.0, W::R, MUTED, label);
        ui.text_right(x + iw, y, 30.0, 14.0, W::M, TEXT, &fp_text(fp));
        34.0
    };
    let body = |ui: &mut Ui, x: f32, y: f32, iw: f32| -> f32 {
        let mut cy = y;
        if let Some(fp) = s.plain {
            cy += key_line(ui, x, cy, iw, EN.seed_read_without_a, fp);
        }
        if s.adding {
            cy += 6.0;
            cy += pass_field(
                ui,
                x,
                cy,
                iw,
                EN.seed_read_passphrase_2,
                &s.pass,
                s.focus == Some(0),
                Action::SeedRead(Choice::Field(0)),
            );
            cy += pass_field(
                ui,
                x,
                cy,
                iw,
                EN.seed_read_passphrase,
                &s.pass2,
                s.focus == Some(1),
                Action::SeedRead(Choice::Field(1)),
            );
            if let Some(fp) = with {
                cy += key_line(ui, x, cy, iw, EN.seed_read_with_this, fp);
            } else if differ {
                cy += ui.wrap(
                    x,
                    cy,
                    iw,
                    13.0,
                    W::R,
                    WARN,
                    EN.seed_read_the_two_passphrases,
                ) + 10.0;
            }
        }
        if let Some(e) = &s.error {
            cy += ui.wrap(x, cy + 4.0, iw, 13.0, W::S, ERR, e) + 14.0;
        } else if let Some(r) = &s.reason {
            cy += ui.wrap(x, cy + 4.0, iw, 13.0, W::S, WARN, r) + 14.0;
        }
        cy - y
    };
    let title = EN.seed_read_seedqr_read;
    if ui.compact {
        crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Keys, ACCENT, title);
            cy += body(ui, x, cy, iw) + 8.0;
            cy += crate::compact::buttons(ui, x, cy, iw, &items);
            cy - y
        });
        return;
    }
    let sw = 600.0;
    let (ix, iw) = (32.0, sw - 64.0);
    // Measured first, out of sight, for the sheet's height.
    ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
    let body_h = body(ui, 0.0, 0.0, iw);
    let buttons_h = wrap_buttons(ui, 0.0, 0.0, iw, 44.0, &items);
    ui.c.pop_clip();
    let sh = 30.0 + 30.0 + 22.0 + body_h + 12.0 + buttons_h + 24.0;
    let (x, y) = sheet_box(ui, w, h, sw, sh);
    let x = x + ix;
    ui.icon(x, y + 30.0, 30.0, Icon::Keys, 18.0, ACCENT);
    ui.text_mid(x + 42.0, y + 30.0, 30.0, 20.0, W::S, TEXT, title);
    let mut cy = y + 82.0;
    cy += body(ui, x, cy, iw) + 12.0;
    wrap_buttons(ui, x, cy, iw, 44.0, &items);
}
