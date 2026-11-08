//! The form that makes a Krux encrypted code (`docs/KEF.md` §15.3): the
//! passphrase, the dice that can fill it, the options and the press that
//! makes the code. The owner (a seed's item on the backup board, or the
//! tool in Tools) holds a [`KefForm`], passes it the app on each press,
//! key and tick, and wraps its [`KefAct`]s as its own `Action`s. The
//! drawing is `screen.rs`; the work is `worker::kef_make`'s.

use crate::Faraday;
use crate::dice_entry::{DiceEntry, Env, Part, Press, Pressed, Used};
use crate::strings::{EN, fill};
use crate::vault::TextBox;
use crate::worker::kef::KefStart;
use crate::worker::kef_make::{self, IdChoice, Made, MakeStart, Options};
use faraday_keys::kef::{DEFAULT_ID_LEN, ID_MAX, LONG_ID_LEN};
use faraday_shell_api::Key;
use zeroize::Zeroizing;

#[expect(dead_code, reason = "the owners that draw the form come after it")]
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
}

/// Where typing goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    /// The passphrase.
    Pass,
    /// The rounds, in Options.
    Rounds,
    /// A typed id, in Options.
    Label,
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
    /// The eye on the passphrase.
    Eye,
    /// Opens the dice entry.
    Dice,
    /// Opens Options.
    Options,
    /// Closes Options.
    Back,
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
}

/// The form that makes a code.
pub struct KefForm {
    source: Source,
    pass: TextBox,
    shown: bool,
    dice: DiceEntry,
    options: Options,
    label: TextBox,
    rounds: TextBox,
    paper: bool,
    focus: Option<Focus>,
    sheet: bool,
    legacy: bool,
    stage: Stage,
    error: Option<String>,
    made: Option<Made>,
}

impl KefForm {
    /// A form for `source`; `paper` where the code is also copied by hand,
    /// so that its side is shown before it is made.
    pub fn new(source: Source, paper: bool) -> Self {
        let options = Options::default();
        let mut rounds = TextBox::default();
        rounds.set(&options.count.to_string());
        KefForm {
            source,
            pass: TextBox::default(),
            shown: false,
            dice: DiceEntry::default(),
            options,
            label: TextBox::default(),
            rounds,
            paper,
            focus: Some(Focus::Pass),
            sheet: false,
            legacy: false,
            stage: Stage::Entry,
            error: None,
            made: None,
        }
    }

    /// Where the form stands.
    pub fn stage(&self) -> Stage {
        self.stage
    }

    /// The options as chosen now.
    pub fn options(&self) -> &Options {
        &self.options
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

    /// Whether Options is open.
    pub fn options_open(&self) -> bool {
        self.sheet
    }

    /// Where typing goes.
    pub fn focus(&self) -> Option<Focus> {
        self.focus
    }

    /// The side of the code in modules, worked out from the options with
    /// no seal; `None` for a compressed version.
    pub fn side(&self) -> Option<usize> {
        let len = match &self.source {
            Source::Seed { bytes, .. } => *bytes,
            Source::Text(t) => t.len(),
        };
        kef_make::side(&self.options, len)
    }

    /// The fingerprint offered as the id, on a seed.
    pub fn fingerprint(&self) -> Option<[u8; 4]> {
        match &self.source {
            Source::Seed { fingerprint, .. } => *fingerprint,
            Source::Text(_) => None,
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
        match (self.paper, self.side()) {
            (true, Some(s)) => fill(
                EN.kef_form_summary_side,
                &[&line, &s.to_string(), &s.to_string()],
            ),
            _ => line,
        }
    }

    /// A press.
    pub fn act(&mut self, app: &mut Faraday, a: KefAct) {
        if self.stage != Stage::Entry {
            return;
        }
        match a {
            KefAct::Field => self.focus = Some(Focus::Pass),
            KefAct::Eye => self.shown = !self.shown,
            KefAct::Dice => {
                if app.may_load_keys() {
                    let p = self.dice.start(DICE_TAG);
                    self.apply(app, p);
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
        if self.stage != Stage::Entry {
            return false;
        }
        match self.focus {
            Some(Focus::Dice(part)) => {
                let out = self.with_dice(app, |d, env| d.key(part, key, env));
                self.apply(app, out);
                true
            }
            Some(Focus::Pass) => match key {
                Key::Char(c) => {
                    // Printable ASCII, at most 200 (`docs/KEF.md` §9).
                    let room = self.pass.text.chars().count() < faraday_keys::kef::PASSPHRASE_MAX;
                    if (' '..='~').contains(&c) && room {
                        self.pass.text.push(c);
                        self.error = None;
                    }
                    true
                }
                Key::Backspace => {
                    self.pass.text.pop();
                    true
                }
                Key::Enter => {
                    self.make(app);
                    true
                }
                _ => false,
            },
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
        if self.stage != Stage::Making {
            return false;
        }
        match app.kef_take_made(&self.options) {
            None => false,
            Some(Ok(m)) => {
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
        if self.stage == Stage::Making {
            app.kef_make_cancel();
            self.stage = Stage::Entry;
        }
    }

    /// Takes the form back to its entry, as Make another does.
    pub fn again(&mut self) {
        if self.stage == Stage::Made {
            self.made = None;
            self.stage = Stage::Entry;
        }
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
            self.error = None;
        } else {
            self.error = Some(EN.kef_refuse_passphrase.to_string());
        }
    }

    fn make(&mut self, app: &mut Faraday) {
        if self.stage != Stage::Entry {
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
