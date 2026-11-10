//! The form that makes a Krux encrypted code (`docs/KEF.md` §15.3): the
//! passphrase, the dice that can fill it, the options and the press that
//! makes the code. The owner (a seed's item on the backup board, or the
//! tool in Tools) holds a [`KefForm`], passes it the app on each press,
//! key and tick, and wraps its [`KefAct`]s as its own `Action`s. The
//! drawing is `screen.rs`; the work is `worker::kef_make`'s.

use crate::Faraday;
use crate::dice_entry::{DiceEntry, Env, Part, Press, Pressed, Used};
use crate::screens::pass_pair;
use crate::strings::{EN, fill};
use crate::vault::TextBox;
use crate::worker::kef::KefStart;
use crate::worker::kef_make::{self, IdChoice, Made, MakeStart, Options, Saved};
use faraday_keys::kef::{DEFAULT_ID_LEN, ID_MAX, LONG_ID_LEN};
use faraday_shell_api::Key;
use zeroize::Zeroizing;

pub(crate) mod screen;

/// The dice entry's tag: the form has one passphrase.
const DICE_TAG: usize = 0;

/// What the code holds.
pub enum Source {
    /// Seed `index`'s entropy, `bytes` long (16 or 32); `fingerprint` is
    /// the wallet's, offered as the id.
    Seed {
        /// The seed's place in the session's keys.
        index: usize,
        /// The entropy's length.
        bytes: usize,
        /// The wallet's fingerprint.
        fingerprint: Option<[u8; 4]>,
    },
    /// A text: a passphrase, a share, a note.
    Text(Zeroizing<String>),
}

/// Where the form stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Asking for the passphrase and the options.
    Entry,
    /// The work runs.
    Making,
    /// The code is made.
    Made,
    /// Check my code asks for the passphrase.
    Check,
    /// The code is being opened to compare.
    Comparing,
}

/// Where typing goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    /// The passphrase.
    Pass,
    /// The passphrase again.
    PassAgain,
    /// The rounds, in Options.
    Rounds,
    /// A typed id, in Options.
    Label,
    /// The passphrase of Check my code.
    Check,
    /// The dice entry.
    Dice(Part),
}

/// The choices of the id in Options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdKind {
    /// 8 random bytes.
    Random8,
    /// 16 random bytes.
    Random16,
    /// A typed label.
    Label,
    /// The wallet's fingerprint.
    Fingerprint,
}

/// A press on the form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KefAct {
    /// Typing goes to the passphrase.
    Field,
    /// Typing goes to the passphrase again.
    FieldAgain,
    /// The eye on the passphrase.
    Eye,
    /// Opens the dice entry.
    Dice,
    /// Typing goes to the dice rolls.
    DiceField,
    /// Opens Options.
    Options,
    /// Closes Options.
    Back,
    /// Chooses a code of this side, in modules (`kef_make::sizes`).
    Size(usize),
    /// Chooses a version.
    Version(u8),
    /// Shows or hides the legacy versions.
    Legacy,
    /// Low error correction.
    Low,
    /// Typing goes to the rounds.
    Rounds,
    /// Chooses the id.
    Id(IdKind),
    /// Typing goes to the label.
    Label,
    /// Makes the code.
    Make,
    /// Opens Check my code, on the Code card.
    Check,
    /// Typing goes to the check's passphrase.
    CheckField,
    /// The eye on the check's passphrase.
    CheckEye,
    /// Opens the code under the check's passphrase and compares.
    Compare,
    /// Closes Check my code.
    CheckBack,
    /// Make another: back to the first step.
    Another,
    /// On Paper: marks block (row) `n` of the grid to copy.
    Pin(usize),
    /// On Paper: marks the grid row by row (`true`) or block by block.
    Rows(bool),
    /// In the Krux code tool: shows the grid to copy the code by hand from
    /// in place of its picture, or the picture again.
    ByHand,
    /// On Paper: takes the code off every place (`Faraday::kef_not_on_paper`).
    NotOnPaper,
    /// On Paper: the camera on the drawn copy (`ScanPurpose::CheckKef`).
    ScanCopy,
}

/// The form that makes a code.
pub struct KefForm {
    source: Source,
    pass: TextBox,
    pass2: TextBox,
    shown: bool,
    dice: DiceEntry,
    options: Options,
    label: TextBox,
    rounds: TextBox,
    paper: bool,
    pin: usize,
    rows: bool,
    /// The tool's Code card: offers **Copy by hand**.
    by_hand_offer: bool,
    /// The tool's Code card shows the grid in place of the picture.
    by_hand: bool,
    scanned: bool,
    from_scan: bool,
    focus: Option<Focus>,
    sheet: bool,
    legacy: bool,
    stage: Stage,
    error: Option<String>,
    made: Option<Made>,
    saved: Option<Saved>,
    check: TextBox,
    check_shown: bool,
    checked: bool,
    offered: Option<[u8; 4]>,
    /// The sheet sizes the contents can take, worked out when the options
    /// or the label change, so that drawing only reads them.
    sizes: Vec<(usize, Options)>,
}

impl KefForm {
    /// Closes the passphrase's eye and the check's: the idle warning.
    pub(crate) fn hide_eyes(&mut self) {
        self.shown = false;
        self.check_shown = false;
    }

    /// Whether the form holds something typed that is secret: the
    /// passphrase, the passphrase typed again to check, or dice rolls
    /// still being entered (for the clean state, `Faraday::clean`).
    pub(crate) fn holds_secret(&self) -> bool {
        !self.pass.text.is_empty()
            || !self.pass2.text.is_empty()
            || !self.check.text.is_empty()
            || self
                .dice
                .open()
                .is_some_and(|(_, rolls, _)| !rolls.text.is_empty())
    }

    /// A form for `source`; `paper` where the code is also copied by hand,
    /// so that its side is shown before it is made.
    pub fn new(source: Source, paper: bool) -> Self {
        let options = Options::default();
        let mut rounds = TextBox::default();
        rounds.set(&options.count.to_string());
        let mut form = KefForm {
            source,
            pass: TextBox::default(),
            pass2: TextBox::default(),
            shown: false,
            dice: DiceEntry::default(),
            options,
            label: TextBox::default(),
            rounds,
            paper,
            pin: 0,
            rows: false,
            by_hand_offer: false,
            by_hand: false,
            scanned: false,
            from_scan: false,
            focus: Some(Focus::Pass),
            sheet: false,
            legacy: false,
            stage: Stage::Entry,
            error: None,
            made: None,
            saved: None,
            check: TextBox::default(),
            check_shown: false,
            checked: false,
            offered: None,
            sizes: Vec::new(),
        };
        form.refresh_sizes();
        form
    }

    /// Works out the sheet sizes again after a press or a key, which may
    /// have changed the options or the label (`kef_make::sizes`).
    fn refresh_sizes(&mut self) {
        self.sizes = kef_make::sizes(&self.options, self.contents_len());
    }

    /// Offers a fingerprint as the id on a text: a loaded wallet's.
    pub(crate) fn offer_fingerprint(&mut self, fingerprint: Option<[u8; 4]>) {
        self.offered = fingerprint;
    }

    /// Whether the code was opened and held what it was made from.
    pub fn checked(&self) -> bool {
        self.checked
    }

    /// Where the form stands.
    pub fn stage(&self) -> Stage {
        self.stage
    }

    /// The options as chosen now.
    pub fn options(&self) -> &Options {
        &self.options
    }

    /// The field typing goes to, with the press that focuses it and its
    /// text; none for the dice entry, which has its own keys.
    pub(crate) fn field(&self) -> Option<(KefAct, &str)> {
        let entry = self.stage == Stage::Entry;
        match self.focus? {
            Focus::Pass | Focus::PassAgain | Focus::Rounds | Focus::Label if !entry => None,
            Focus::Check if self.stage != Stage::Check => None,
            Focus::Pass => Some((KefAct::Field, self.pass.text.as_str())),
            Focus::PassAgain => Some((KefAct::FieldAgain, self.pass2.text.as_str())),
            Focus::Check => Some((KefAct::CheckField, self.check.text.as_str())),
            Focus::Rounds => Some((KefAct::Rounds, self.rounds.text.as_str())),
            Focus::Label => Some((KefAct::Label, self.label.text.as_str())),
            Focus::Dice(_) => None,
        }
    }

    /// The passphrase typed twice is the same, so Make is on.
    pub(crate) fn passphrases_agree(&self) -> bool {
        !pass_pair::differ(self.pass.text.as_str(), self.pass2.text.as_str())
    }

    /// The passphrase as typed.
    pub fn passphrase(&self) -> &str {
        self.pass.text.as_str()
    }

    /// The refusal or problem line, if any.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// The code, once made.
    pub fn made(&self) -> Option<&Made> {
        self.made.as_ref()
    }

    /// The names the Outbox gave the code's files.
    pub fn saved(&self) -> Option<&Saved> {
        self.saved.as_ref()
    }

    /// Whether the drawn copy on Paper was scanned and held the code.
    pub fn scanned(&self) -> bool {
        self.scanned
    }

    /// Whether the code is on Paper at a side the drawn copy is scanned
    /// back for (25 to 37), and not scanned yet: **Scan my copy** shows.
    pub(crate) fn scan_offered(&self) -> bool {
        self.paper && !self.scanned && self.stage == Stage::Made && self.by_hand_side().is_some()
    }

    /// Offers **Copy by hand** on the Code card (the Krux code tool).
    pub fn offer_by_hand(&mut self) {
        self.by_hand_offer = true;
    }

    fn by_hand_side(&self) -> Option<usize> {
        let q = self.made.as_ref()?.view.as_ref().ok()?.frames.first()?;
        (q.size() <= faraday_draw::tokens::KEF_BY_HAND_MAX_SIDE).then(|| q.size())
    }

    /// The code as its picture holds it: the base43 text of its bytes.
    pub(crate) fn code_text(&self) -> Option<String> {
        self.by_hand_side()?;
        Some(faraday_codec::kef::base43_encode(
            &self.made.as_ref()?.bytes,
        ))
    }

    /// The drawn copy was read and is the code: one field and Compare,
    /// run on the bytes scanned, which are the code's.
    pub(crate) fn scan_matched(&mut self) {
        if self.made.is_none() {
            return;
        }
        self.from_scan = true;
        self.check.clear();
        self.error = None;
        self.focus = Some(Focus::Check);
        self.stage = Stage::Check;
    }

    /// Where the code is copied by hand now: the plan's map has it on
    /// Paper at a place. A code checked while it was not on Paper, and not
    /// scanned, goes back to Not checked when it is put on Paper while a
    /// camera is attached: its drawn copy is yet to be scanned
    /// (`docs/KEF.md` §15.1). With no camera it stays checked.
    pub(crate) fn set_paper(&mut self, paper: bool, camera: bool) {
        if paper && !self.paper && camera && !self.scanned {
            self.checked = false;
        }
        self.paper = paper;
    }

    /// Takes up a code made before, read back from its file: the form
    /// stands at the code, `checked`, `scanned` and `paper` as they were, so a
    /// code put on Paper since reads Not checked when the map is followed.
    pub(crate) fn restore(
        &mut self,
        made: Made,
        saved: Saved,
        checked: bool,
        scanned: bool,
        paper: bool,
    ) {
        self.paper = paper;
        self.made = Some(made);
        self.saved = Some(saved);
        self.checked = checked;
        self.scanned = scanned;
        self.stage = Stage::Made;
    }

    /// Whether Options is open.
    pub(crate) fn options_open(&self) -> bool {
        self.sheet
    }

    /// Where typing goes.
    pub fn focus(&self) -> Option<Focus> {
        self.focus
    }

    /// The side of the code in modules, worked out from the options with
    /// no seal; `None` for a compressed version.
    pub fn side(&self) -> Option<usize> {
        kef_make::side(&self.options, self.contents_len())
    }

    fn contents_len(&self) -> usize {
        match &self.source {
            Source::Seed { bytes, .. } => *bytes,
            Source::Text(t) => t.len(),
        }
    }

    /// The sheet sizes the contents can take, each with the options that
    /// give it (`kef_make::sizes`), as last worked out.
    pub fn sizes(&self) -> &[(usize, Options)] {
        &self.sizes
    }

    /// The fingerprint offered as the id, on a seed.
    pub fn fingerprint(&self) -> Option<[u8; 4]> {
        match &self.source {
            Source::Seed { fingerprint, .. } => *fingerprint,
            Source::Text(_) => self.offered,
        }
    }

    /// Whether the code is also copied by hand.
    pub fn on_paper(&self) -> bool {
        self.paper
    }

    /// The options button's line.
    pub fn summary(&self) -> String {
        let o = &self.options;
        let id = match &o.id {
            IdChoice::Random(n) if *n == LONG_ID_LEN => EN.kef_id_random16_fact,
            IdChoice::Random(_) => EN.kef_id_random8_fact,
            IdChoice::Label(_) => EN.kef_id_label_fact,
            IdChoice::Fingerprint(_) => EN.kef_id_fingerprint_fact,
        };
        let line = fill(
            EN.kef_form_summary,
            &[&o.version.to_string(), &group(o.count), id],
        );
        match self.side() {
            Some(s) => fill(
                EN.kef_form_summary_side,
                &[&line, &s.to_string(), &s.to_string()],
            ),
            _ => line,
        }
    }

    /// A press.
    pub fn act(&mut self, app: &mut Faraday, a: KefAct) {
        self.press(app, a);
        self.refresh_sizes();
    }

    fn press(&mut self, app: &mut Faraday, a: KefAct) {
        match (self.stage, a) {
            // On Paper with a camera the drawn copy is scanned instead.
            (Stage::Made, KefAct::Check) if self.paper && !app.cameras().is_empty() => {}
            (Stage::Made, KefAct::Check) => {
                self.from_scan = false;
                self.stage = Stage::Check;
                self.error = None;
                self.focus = Some(Focus::Check);
            }
            (Stage::Made, KefAct::Another) => self.again(),
            (Stage::Made, KefAct::ByHand) if self.by_hand_offer => {
                self.by_hand = !self.by_hand && self.by_hand_side().is_some();
                self.pin = 0;
                self.rows = false;
            }
            (Stage::Made, KefAct::Pin(n)) => self.pin = n,
            (Stage::Made, KefAct::Rows(r)) => {
                self.rows = r;
                self.pin = 0;
            }
            (Stage::Check, KefAct::CheckField) => self.focus = Some(Focus::Check),
            (Stage::Check, KefAct::CheckEye) => self.check_shown = !self.check_shown,
            (Stage::Check, KefAct::Compare) => self.compare(app),
            (Stage::Check, KefAct::CheckBack) => {
                self.from_scan = false;
                self.check.clear();
                self.error = None;
                self.stage = Stage::Made;
            }
            _ => {}
        }
        if self.stage == Stage::Entry {
            self.entry_press(app, a);
        }
    }

    fn entry_press(&mut self, app: &mut Faraday, a: KefAct) {
        match a {
            KefAct::Field => self.focus = Some(Focus::Pass),
            KefAct::FieldAgain => self.focus = Some(Focus::PassAgain),
            KefAct::Eye => self.shown = !self.shown,
            KefAct::Dice => {
                if app.may_load_keys() {
                    let p = self.dice.start(DICE_TAG);
                    self.apply(app, p);
                }
            }
            KefAct::DiceField => {
                if self.dice.open().is_some() {
                    self.focus = Some(Focus::Dice(Part::Dice));
                }
            }
            KefAct::Options => {
                self.sheet = true;
                self.focus = None;
            }
            KefAct::Back => {
                self.sheet = false;
                self.focus = Some(Focus::Pass);
            }
            KefAct::Size(side) => {
                if let Some((_, o)) = self.sizes.iter().find(|(s, _)| *s == side) {
                    self.options = o.clone();
                    self.error = None;
                }
            }
            KefAct::Version(v) => {
                if faraday_codec::kef::spec_of(v).is_some() {
                    self.options.version = v;
                    if !self.options.low_offered() {
                        self.options.low = false;
                    }
                    self.error = None;
                }
            }
            KefAct::Legacy => self.legacy = !self.legacy,
            KefAct::Low => {
                if self.options.low_offered() {
                    self.options.low = !self.options.low;
                }
            }
            KefAct::Rounds => self.focus = Some(Focus::Rounds),
            KefAct::Id(k) => {
                self.options.id = match k {
                    IdKind::Random8 => IdChoice::Random(DEFAULT_ID_LEN),
                    IdKind::Random16 => IdChoice::Random(LONG_ID_LEN),
                    IdKind::Label => IdChoice::Label(self.label.text.as_str().to_string()),
                    IdKind::Fingerprint => match self.fingerprint() {
                        Some(f) => IdChoice::Fingerprint(f),
                        None => return,
                    },
                };
                self.focus = (k == IdKind::Label).then_some(Focus::Label);
            }
            KefAct::Label => {
                self.options.id = IdChoice::Label(self.label.text.as_str().to_string());
                self.focus = Some(Focus::Label);
            }
            KefAct::Make => self.make(app),
            KefAct::Check
            | KefAct::CheckField
            | KefAct::CheckEye
            | KefAct::Compare
            | KefAct::CheckBack
            | KefAct::Another
            | KefAct::Pin(_)
            | KefAct::Rows(_)
            | KefAct::ByHand
            | KefAct::NotOnPaper
            | KefAct::ScanCopy => {}
        }
    }

    /// A press on the dice entry or the roller. The roller is Faraday
    /// Demo's.
    pub fn dice_press(&mut self, app: &mut Faraday, p: Press) {
        if self.stage != Stage::Entry || matches!(p, Press::Roller(_)) && !app.roller_offered() {
            return;
        }
        let out = self.with_dice(app, |d, env| d.press(p, env));
        self.apply(app, out);
    }

    /// The roller's tick. Returns whether the screen needs a frame.
    pub fn tick(&mut self, app: &mut Faraday) -> bool {
        self.with_dice(app, |d, env| d.tick(env))
    }

    /// Typing. Returns whether the key was taken.
    pub fn key(&mut self, app: &mut Faraday, key: Key) -> bool {
        let taken = self.typed(app, key);
        if taken {
            self.refresh_sizes();
        }
        taken
    }

    fn typed(&mut self, app: &mut Faraday, key: Key) -> bool {
        if self.stage == Stage::Check {
            return self.check_key(app, key);
        }
        if self.stage != Stage::Entry {
            return false;
        }
        match self.focus {
            Some(Focus::Dice(part)) => {
                let out = self.with_dice(app, |d, env| d.key(part, key, env));
                self.apply(app, out);
                true
            }
            Some(f @ (Focus::Pass | Focus::PassAgain)) => match key {
                Key::Char(c) => {
                    let again = f == Focus::PassAgain;
                    let text = if again {
                        &mut self.pass2.text
                    } else {
                        &mut self.pass.text
                    };
                    // Printable ASCII, at most 200 (`docs/KEF.md` §9).
                    let room = text.chars().count() < faraday_keys::kef::PASSPHRASE_MAX;
                    if (' '..='~').contains(&c) && room {
                        text.push(c);
                        self.error = None;
                    }
                    true
                }
                Key::Backspace => {
                    if f == Focus::PassAgain {
                        self.pass2.text.pop();
                    } else {
                        self.pass.text.pop();
                    }
                    true
                }
                Key::Tab if f == Focus::Pass => {
                    self.focus = Some(Focus::PassAgain);
                    true
                }
                Key::Enter => {
                    if f == Focus::Pass {
                        self.focus = Some(Focus::PassAgain);
                    } else {
                        self.make(app);
                    }
                    true
                }
                _ => false,
            },
            Some(Focus::Check) => false,
            Some(Focus::Rounds) => match key {
                Key::Char(c) if c.is_ascii_digit() => {
                    if self.rounds.text.len() < 7 {
                        self.rounds.text.push(c);
                        self.sync_rounds();
                    }
                    true
                }
                Key::Backspace => {
                    self.rounds.text.pop();
                    self.sync_rounds();
                    true
                }
                _ => false,
            },
            Some(Focus::Label) => match key {
                Key::Char(c) if !c.is_control() => {
                    if self.label.text.len() + c.len_utf8() <= ID_MAX {
                        self.label.text.push(c);
                        self.sync_label();
                    }
                    true
                }
                Key::Backspace => {
                    self.label.text.pop();
                    self.sync_label();
                    true
                }
                _ => false,
            },
            None => false,
        }
    }

    /// Costs a QR encoding when the work has ended. Takes the answer into
    /// the form: the code made, or what was refused. Returns whether
    /// anything changed.
    pub fn poll(&mut self, app: &mut Faraday) -> bool {
        match self.stage {
            Stage::Making => self.poll_made(app),
            Stage::Comparing => self.poll_compared(app),
            _ => false,
        }
    }

    fn poll_made(&mut self, app: &mut Faraday) -> bool {
        match app.kef_take_made(&self.options) {
            None => false,
            Some(Ok(m)) => {
                // The job has the passphrase; the field is wiped.
                self.pass.clear();
                self.pass2.clear();
                let saved = app.kef_save_files(&m);
                app.toast_out(saved.toast_name());
                self.saved = Some(saved);
                self.made = Some(m);
                self.stage = Stage::Made;
                true
            }
            Some(Err(e)) => {
                self.error = Some(e);
                self.stage = Stage::Entry;
                true
            }
        }
    }

    /// Cancel while the work runs: it stops and its answer is dropped.
    pub fn cancel(&mut self, app: &mut Faraday) {
        match self.stage {
            Stage::Making => {
                app.kef_make_cancel();
                self.stage = Stage::Entry;
            }
            Stage::Comparing => {
                app.kef_make_cancel();
                self.stage = Stage::Check;
            }
            _ => {}
        }
    }

    /// Takes the form back to its first step, as Make another does: the
    /// passphrase and the check's are empty.
    pub fn again(&mut self) {
        if matches!(self.stage, Stage::Made | Stage::Check) {
            self.made = None;
            self.saved = None;
            self.scanned = false;
            self.from_scan = false;
            self.pin = 0;
            self.by_hand = false;
            self.pass.clear();
            self.pass2.clear();
            self.check.clear();
            self.checked = false;
            self.error = None;
            self.focus = Some(Focus::Pass);
            self.stage = Stage::Entry;
        }
    }

    /// Puts the form away where it is held between visits: the work stops,
    /// the passphrase, the check's and the dice rolls are wiped and the
    /// sheets close. What was made and whether it was checked stay, and a
    /// Check my code left open is back at the code.
    pub fn close(&mut self, app: &mut Faraday) {
        self.cancel(app);
        self.pass.clear();
        self.pass2.clear();
        self.check.clear();
        self.dice.close();
        self.shown = false;
        self.check_shown = false;
        self.sheet = false;
        self.error = None;
        if self.stage == Stage::Check {
            self.stage = Stage::Made;
        }
        if self.stage == Stage::Entry {
            self.focus = Some(Focus::Pass);
        }
    }

    fn check_key(&mut self, app: &mut Faraday, key: Key) -> bool {
        match key {
            Key::Char(c) => {
                let room = self.check.text.chars().count() < faraday_keys::kef::PASSPHRASE_MAX;
                if (' '..='~').contains(&c) && room {
                    self.check.text.push(c);
                    self.error = None;
                }
                true
            }
            Key::Backspace => {
                self.check.text.pop();
                true
            }
            Key::Enter => {
                self.compare(app);
                true
            }
            _ => false,
        }
    }

    /// Costs a PBKDF2 of the code's rounds, on the worker. Opens the code
    /// under the check's passphrase.
    fn compare(&mut self, app: &mut Faraday) {
        let Some(made) = &self.made else { return };
        let Ok(envelope) = faraday_keys::kef::unwrap(&made.bytes) else {
            return;
        };
        self.error = None;
        let pass = Zeroizing::new(self.check.text.as_str().as_bytes().to_vec());
        app.kef_open(pass, envelope, EN.kef_comparing);
        self.stage = Stage::Comparing;
    }

    /// Costs a parse of the seed's words. Takes the open's answer and
    /// compares what it holds with what the code was made from.
    fn poll_compared(&mut self, app: &mut Faraday) -> bool {
        let Some(answer) = app.kef_take_opened() else {
            return false;
        };
        match answer {
            Ok(opened) => {
                let same = match &self.source {
                    Source::Seed { index, .. } => {
                        let c = app.compute();
                        faraday_keys::kef::seed_matches(
                            &app.session.keys,
                            &c,
                            *index,
                            &opened.plaintext,
                        )
                    }
                    Source::Text(t) => opened.plaintext.as_slice() == t.as_bytes(),
                };
                if same {
                    // The status is fixed here: on Paper with a camera the
                    // code counts once its drawn copy was scanned, else
                    // once it opened.
                    self.scanned |= self.from_scan;
                    self.checked = !self.paper || self.from_scan || app.cameras().is_empty();
                    self.from_scan = false;
                    self.check.clear();
                    self.error = None;
                    self.stage = Stage::Made;
                } else {
                    self.no_match();
                }
            }
            Err(faraday_keys::kef::OpenError::Cancelled) => self.stage = Stage::Check,
            Err(_) => self.no_match(),
        }
        true
    }

    fn no_match(&mut self) {
        self.error = Some(EN.kef_check_no_match.to_string());
        self.stage = Stage::Check;
    }

    fn sync_rounds(&mut self) {
        self.options.count = self.rounds.text.as_str().parse().unwrap_or(0);
        self.error = None;
    }

    fn sync_label(&mut self) {
        if matches!(self.options.id, IdChoice::Label(_)) {
            self.options.id = IdChoice::Label(self.label.text.as_str().to_string());
        }
        self.error = None;
    }

    fn with_dice<R>(
        &mut self,
        app: &mut Faraday,
        f: impl FnOnce(&mut DiceEntry, &mut Env) -> R,
    ) -> R {
        let reduce = app.reduce_motion();
        let now_ms = app.now_ms();
        let mut draw = || app.fresh(b"faraday dice roller");
        f(
            &mut self.dice,
            &mut Env {
                now_ms,
                reduce,
                draw: &mut draw,
            },
        )
    }

    fn apply(&mut self, app: &Faraday, out: Pressed) {
        if let Some(f) = out.focus {
            self.focus = Some(f.map_or(Focus::Pass, Focus::Dice));
        }
        if let Some(u) = out.used {
            self.use_words(app, u);
        }
    }

    /// The words the rolls made become the passphrase; Practice puts its
    /// public passphrase in their place (`docs/DEMO.md` §5.2).
    fn use_words(&mut self, app: &Faraday, u: Used) {
        if u.phrase.is_empty() {
            return;
        }
        let phrase = if app.makes_secrets() {
            u.phrase
        } else {
            Zeroizing::new(crate::demo::PASSPHRASE.to_string())
        };
        if faraday_keys::kef::writable_passphrase(&phrase) {
            self.pass.set(&phrase);
            self.pass2.set(&phrase);
            self.error = None;
        } else {
            self.error = Some(EN.kef_refuse_passphrase.to_string());
        }
    }

    fn make(&mut self, app: &mut Faraday) {
        if self.stage != Stage::Entry || !self.passphrases_agree() {
            return;
        }
        self.error = None;
        let pass = Zeroizing::new(self.pass.text.as_str().as_bytes().to_vec());
        let started = match &self.source {
            Source::Seed { index, .. } => app.kef_make_seed(*index, pass, &self.options),
            Source::Text(t) => app.kef_make_text(pass, t.clone(), &self.options),
        };
        match started {
            Ok(()) => self.stage = Stage::Making,
            Err(MakeStart::Refused(line)) => self.error = Some(line.to_string()),
            Err(MakeStart::NotOffered) => {
                self.error = Some(kef_make::not_offered_line().to_string())
            }
            Err(MakeStart::Not(KefStart::NoFreshBytes)) => {
                self.error = Some(EN.kef_form_need_random.to_string());
            }
            Err(MakeStart::Not(KefStart::Refused(e))) => {
                self.error = Some(kef_make::refusal_text(e));
            }
        }
    }
}

/// A count with thousands separated: 100,000.
fn group(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}
