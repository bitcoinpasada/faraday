//! Convert a seed (`docs/SEED-CONVERSION.md`): a loaded key written in
//! another standard, the same wallet. Every conversion is `faraday-keys`'s
//! and reads its result back first; this area holds the flow, and the
//! result until the flow is left.

pub mod screen;

use crate::copy_check::{Typed, WordCheck};
use crate::flow;
use crate::strings::{EN, fill};
use crate::wallet::fp_text;
use crate::{Action, Faraday, Screen};
use faraday_bip::keys::Fingerprint;
use faraday_keys::{KeyInfo, Reveal, operations::Convert};
use zeroize::Zeroizing;

/// The cards, in order. A target with settings would have its card
/// between [`cstep::TO`] and [`cstep::RESULT`].
pub mod cstep {
    /// Which key.
    pub const FROM: u8 = 0;
    /// Which standard.
    pub const TO: u8 = 1;
    /// The value and its facts.
    pub const RESULT: u8 = 2;
    /// How many cards.
    pub const COUNT: usize = 3;
}

/// Everything the flow can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConvertAction {
    /// Open the flow.
    Open,
    /// Open or close a card.
    Step(u8),
    /// Choose a loaded key, by fingerprint.
    Key([u8; 4]),
    /// Load a key from an open vault, by that vault's place among those
    /// open and the record's in it, and choose it.
    VaultKey(usize, usize),
    /// Press a row of [`ROWS`], by its place there.
    To(u8),
    /// Show or hide the value.
    Show,
    /// Open or close Check my copy.
    Check,
    /// Compare what is typed with the value.
    Compare,
    /// Clear what is typed.
    CheckClear,
    /// Offer the value as a file, on the secret sheet.
    Save,
    /// Leave the flow.
    Done,
}

/// What a row that is offered writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The BIP-32 seed, as hex.
    SeedHex,
    /// The BIP-32 master extended private key.
    Xprv,
}

impl Target {
    /// The Standard fact.
    fn standard(self) -> &'static str {
        match self {
            Target::SeedHex => EN.convert_seed_line,
            Target::Xprv => EN.convert_xprv_line,
        }
    }

    /// The Restores in fact.
    fn restores_in(self) -> &'static str {
        match self {
            Target::SeedHex => EN.convert_seed_restores,
            Target::Xprv => EN.convert_xprv_restores,
        }
    }

    /// What the secret sheet calls the value.
    fn secret(self) -> &'static str {
        match self {
            Target::SeedHex => EN.convert_secret_seed,
            Target::Xprv => EN.convert_secret_xprv,
        }
    }

    /// The end of the file's name.
    fn file(self) -> &'static str {
        match self {
            Target::SeedHex => "seed-hex",
            Target::Xprv => "xprv",
        }
    }
}

/// A row under To.
pub struct Row {
    /// The target's name.
    pub name: &'static str,
    /// Its standard, or why it is not offered.
    pub line: &'static str,
    /// What a press writes; `None` for a dimmed row, which takes no press.
    pub target: Option<Target>,
    /// Whether the row applies to the key.
    applies: fn(&KeyInfo) -> bool,
}

impl Row {
    /// Whether the row is listed for `key`.
    pub fn applies(&self, key: &KeyInfo) -> bool {
        (self.applies)(key)
    }
}

/// Every row under To: the targets, then what is not offered, each with
/// the sources it applies to.
pub const ROWS: [Row; 6] = [
    Row {
        name: EN.convert_seed,
        line: EN.convert_seed_line,
        target: Some(Target::SeedHex),
        applies: |_| true,
    },
    Row {
        name: EN.convert_xprv,
        line: EN.convert_xprv_line,
        target: Some(Target::Xprv),
        applies: |_| true,
    },
    Row {
        name: EN.convert_no_words,
        line: EN.convert_no_words_line,
        target: None,
        applies: |k| !k.has_words(),
    },
    Row {
        name: EN.convert_no_shares,
        line: EN.convert_no_shares_line,
        target: None,
        applies: KeyInfo::has_words,
    },
    Row {
        name: EN.convert_no_list,
        line: EN.convert_no_list_line,
        target: None,
        applies: KeyInfo::has_words,
    },
    Row {
        name: EN.convert_no_shamir39,
        line: EN.convert_no_shamir39_line,
        target: None,
        applies: |_| true,
    },
];

/// The flow's state. None of it is a secret: the value is on the area.
pub struct ConvertState {
    /// The card open.
    pub open: Option<u8>,
    /// Which cards are done.
    pub done: [bool; cstep::COUNT],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The key chosen.
    pub key: Option<[u8; 4]>,
    /// The target pressed.
    pub target: Option<Target>,
    /// Whether the value shows.
    pub shown: bool,
    /// Where Back and Done go.
    pub back: Screen,
    /// The line in place of the value, when it was not given.
    pub refused: Option<&'static str>,
    /// The Paths fact: the loaded wallets that hold the key, worked out on
    /// the press. Public text. It stays right because every way to change
    /// the wallets or keys leaves the flow, which drops it.
    pub paths: Vec<String>,
}

/// The flow and its result.
#[derive(Default)]
pub struct ConvertArea {
    state: Option<ConvertState>,
    /// The value, wiped on drop: dropped when the Result card or the flow
    /// is left, or the session locks. Drawing only reads it.
    held: Option<faraday_keys::Held<Zeroizing<String>>>,
    /// Check my copy, while it is open: what is typed, wiped on drop.
    check: Option<WordCheck>,
    /// What the last comparison found, until the next key or press.
    found: Option<bool>,
}

impl ConvertArea {
    /// The flow's state, while it is open.
    pub(crate) fn state(&self) -> Option<&ConvertState> {
        self.state.as_ref()
    }

    /// Forgets the flow and its result.
    pub(crate) fn clear(&mut self) {
        self.state = None;
        self.drop_value();
    }

    /// Drops the value and what is typed to check it.
    fn drop_value(&mut self) {
        self.held = None;
        self.check = None;
        self.found = None;
    }

    /// Check my copy, while it is open.
    pub(crate) fn check(&self) -> Option<&WordCheck> {
        self.check.as_ref()
    }

    /// Opens Check my copy with nothing typed, or closes it.
    fn toggle_check(&mut self) {
        self.found = None;
        self.check = match (&self.check, &self.held) {
            (None, Some(_)) => Some(WordCheck::passphrase()),
            _ => None,
        };
    }

    /// Compares what is typed with the value and says which. A match
    /// closes the check; a difference wipes what was typed, as the backup's
    /// passphrase check does. Nothing typed compares nothing.
    fn compare(&mut self) {
        let (Some(check), Some(held)) = (self.check.as_mut(), self.held.as_ref()) else {
            return;
        };
        if check.typed.is_empty() {
            return;
        }
        let same = check.typed == *held.as_str();
        check.typed.clear();
        self.found = Some(same);
        if same {
            self.check = None;
        }
    }

    /// Clears what is typed.
    fn clear_check(&mut self) {
        self.found = None;
        if let Some(c) = self.check.as_mut() {
            c.typed.clear();
        }
    }

    /// A key typed into Check my copy. Returns whether the key was taken.
    fn check_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        let Some(check) = self.check.as_mut() else {
            return false;
        };
        match key {
            // Every value here is printable ASCII: anything else cannot
            // be part of a right copy, and is not typed.
            K::Char(ch) if !(' '..='~').contains(&ch) => return true,
            K::Char(_) | K::Backspace | K::Delete | K::Enter | K::Escape => {}
            _ => return false,
        }
        match check.key(key) {
            Typed::Compare => self.compare(),
            Typed::Close => self.toggle_check(),
            Typed::Nothing | Typed::Words => self.found = None,
        }
        true
    }

    /// The value, while its eye is open: a borrow, no work.
    pub(crate) fn held(&self) -> Option<&faraday_keys::Held<Zeroizing<String>>> {
        self.held
            .as_ref()
            .filter(|_| self.state.as_ref().is_some_and(|s| s.shown))
    }

    /// The column's scroll, while the flow is open.
    pub(crate) fn scroll_mut(&mut self) -> Option<&mut flow::Scroll> {
        self.state.as_mut().map(|s| &mut s.scroll)
    }

    /// Takes the column's scroll as the pass left it.
    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        if let Some(s) = self.state.as_mut() {
            s.scroll = to;
        }
    }

    /// Opens the flow on `key` (the only key loaded, if there is one), to
    /// return to `back`.
    fn open(&mut self, key: Option<[u8; 4]>, back: Screen) {
        self.drop_value();
        self.state = Some(ConvertState {
            open: Some(if key.is_some() {
                cstep::TO
            } else {
                cstep::FROM
            }),
            done: [key.is_some(), false, false],
            scroll: flow::Scroll::default(),
            key,
            target: None,
            shown: false,
            back,
            refused: None,
            paths: Vec::new(),
        });
    }

    /// Drops the result, as leaving its card does.
    fn drop_result(&mut self) {
        self.drop_value();
        if let Some(s) = self.state.as_mut() {
            s.target = None;
            s.shown = false;
            s.refused = None;
            s.paths.clear();
            s.done[usize::from(cstep::TO)] = false;
            s.done[usize::from(cstep::RESULT)] = false;
        }
    }

    /// Opens or closes card `step`. A card opens when those before it are
    /// done; any other than Result drops the result.
    fn step(&mut self, step: u8) {
        let Some(s) = self.state.as_mut() else {
            return;
        };
        if s.open == Some(step) {
            s.open = None;
        } else if (0..step).all(|i| s.done[usize::from(i)]) {
            s.open = Some(step);
            s.scroll.follow = true;
        }
        if s.open != Some(cstep::RESULT) {
            self.drop_result();
        }
    }

    /// Chooses `key`, and To opens.
    fn choose(&mut self, key: [u8; 4]) {
        self.drop_result();
        if let Some(s) = self.state.as_mut() {
            s.key = Some(key);
            s.done[usize::from(cstep::FROM)] = true;
            s.open = Some(cstep::TO);
            s.scroll.follow = true;
        }
    }

    /// What the press gave for `target`, and Result opens.
    fn made(
        &mut self,
        target: Target,
        given: Result<faraday_keys::Held<Zeroizing<String>>, Convert>,
        paths: Vec<String>,
    ) {
        self.drop_result();
        let (held, refused) = match given {
            Ok(value) => (Some(value), None),
            Err(Convert::NotSameWallet) => (None, Some(EN.convert_not_same_wallet)),
            Err(_) => (None, Some(EN.convert_not_given)),
        };
        self.held = held;
        if let Some(s) = self.state.as_mut() {
            s.target = Some(target);
            s.refused = refused;
            s.paths = paths;
            s.done[usize::from(cstep::TO)] = true;
            s.done[usize::from(cstep::RESULT)] = true;
            s.open = Some(cstep::RESULT);
            s.scroll.follow = true;
        }
    }
}

/// The flow's state, while it is open.
pub fn state(app: &Faraday) -> Option<&ConvertState> {
    app.tools.convert().state()
}

/// The value, while its eye is open: a borrow, no work.
pub fn shown(app: &Faraday) -> Option<&str> {
    app.tools.convert().held().map(|v| v.as_str())
}

/// What Check my copy has typed, while it is open.
pub fn typed(app: &Faraday) -> Option<&str> {
    app.tools.convert().check().map(|c| &*c.typed)
}

/// Whether the last comparison found the copy right, until the next key.
pub fn found(app: &Faraday) -> Option<bool> {
    app.tools.convert().found
}

/// The row at `place` of [`ROWS`], if it applies to the key chosen. A
/// lookup.
fn row_for(app: &Faraday, place: u8) -> Option<(usize, Target)> {
    let area = app.tools.convert();
    let fp = area.state()?.key?;
    let i = app.session.keys.position_of(fp)?;
    let row = ROWS.get(usize::from(place))?;
    (row.applies(&app.session.keys[i]))
        .then_some(row.target)
        .flatten()
        .map(|target| (i, target))
}

/// What a loaded key was loaded from, as far as its facts say.
pub fn source(key: &KeyInfo) -> String {
    if let Some(n) = key.word_count {
        return fill(EN.convert_from_words, &[&n.to_string()]);
    }
    if let Some(set) = &key.codex32 {
        return fill(EN.convert_from_codex32, &[&set.length.to_string()]);
    }
    EN.convert_from_seed.to_string()
}

/// The Paths fact's lines: each path the loaded wallets use from `fp`. A
/// walk of the wallets' keys.
fn paths_of(app: &Faraday, fp: Fingerprint) -> Vec<String> {
    let mut out = Vec::new();
    for w in &app.session.wallets {
        for k in w.policy().keys() {
            let Some(path) = k.path().filter(|_| k.fingerprint() == Some(fp)) else {
                continue;
            };
            let line = fill(EN.convert_wallet_path, &[&w.name, &format!("m/{path}")]);
            if !out.contains(&line) {
                out.push(line);
            }
        }
    }
    out
}

/// Costs what `master_seed_hex` or `master_xprv` costs: a PBKDF2 on a key
/// of words. The press on the row at `place`; a row not offered does
/// nothing.
fn make(app: &mut Faraday, place: u8) {
    let Some((i, target)) = row_for(app, place) else {
        return;
    };
    let c = app.compute();
    let keys = &app.session.keys;
    let given = match target {
        Target::SeedHex => keys.master_seed_hex(&c, i, Reveal::PaperForm),
        Target::Xprv => keys.master_xprv(&c, i, Reveal::PaperForm),
    };
    let paths = paths_of(app, keys[i].fingerprint);
    app.tools.convert_mut().made(target, given, paths);
}

/// Offers the value on the secret sheet: nothing is written until the
/// sheet is answered, and what it writes is the value alone.
fn save(app: &mut Faraday) {
    let area = app.tools.convert();
    let (Some(held), Some(s)) = (area.held.as_ref(), area.state()) else {
        return;
    };
    let (Some(target), Some(fp)) = (s.target, s.key) else {
        return;
    };
    let name = format!(
        "{}-{}.txt",
        fp_text(Fingerprint(fp)).to_lowercase(),
        target.file()
    );
    let bytes = Zeroizing::new(held.as_str().as_bytes().to_vec());
    let out = crate::secrets::SecretOut::new(name, bytes, target.secret(), EN.convert_secret_gives);
    app.offer_secret(out);
}

/// Costs what `vault_key_load` costs: the key's seed and master key. The
/// press on an open vault's key under From: it loads, stays loaded, and
/// the flow goes on as for a loaded key.
fn from_vault(app: &mut Faraday, v: usize, r: usize) {
    if let Some(fp) = app.vault_key_load(v, r) {
        app.tools.convert_mut().choose(fp);
    }
}

/// A press in the flow.
pub(crate) fn act(app: &mut Faraday, action: ConvertAction) {
    match action {
        ConvertAction::Open => {
            let key = (app.session.keys.len() == 1).then(|| app.session.keys[0].fingerprint.0);
            let back = app.screen;
            app.tools.convert_mut().open(key, back);
            app.screen = Screen::Convert;
        }
        ConvertAction::Step(s) => app.tools.convert_mut().step(s),
        ConvertAction::Key(fp) => {
            if app.session.keys.position_of(fp).is_some() {
                app.tools.convert_mut().choose(fp);
            }
        }
        ConvertAction::VaultKey(v, r) => from_vault(app, v, r),
        ConvertAction::To(place) => make(app, place),
        ConvertAction::Show => {
            let area = app.tools.convert_mut();
            if area.held.is_some()
                && let Some(s) = area.state.as_mut()
            {
                s.shown = !s.shown;
            }
        }
        ConvertAction::Check => app.tools.convert_mut().toggle_check(),
        ConvertAction::Compare => app.tools.convert_mut().compare(),
        ConvertAction::CheckClear => app.tools.convert_mut().clear_check(),
        ConvertAction::Save => save(app),
        ConvertAction::Done => {
            if let Some(back) = state(app).map(|s| s.back) {
                app.act(Action::Nav(back));
            }
        }
    }
}

/// A key on the flow: typing while Check my copy is open, where Escape
/// closes it; otherwise Escape is Back. Returns whether it was taken.
pub(crate) fn key(app: &mut Faraday, key: faraday_shell_api::Key) -> bool {
    if app.screen != Screen::Convert || app.sheet.is_some() || state(app).is_none() {
        return false;
    }
    if app.tools.convert_mut().check_key(key) {
        return true;
    }
    if key == faraday_shell_api::Key::Escape {
        app.back();
        return true;
    }
    false
}

/// After a press: the flow ends, its result with it, unless it is on
/// show or Add a key, opened from it, is and will come back.
pub(crate) fn tidy(app: &mut Faraday) {
    let kept = app.screen == Screen::Convert
        || (matches!(app.screen, Screen::KeyGen | Screen::Randomness)
            && app
                .keygen
                .state()
                .is_some_and(|k| k.back == Screen::Convert));
    if !kept {
        app.tools.convert_mut().clear();
    }
}

/// The fingerprint fact: the key's, and that it is the source's.
fn same_as_source(fp: Fingerprint) -> String {
    fill(EN.convert_same_as_source, &[&fp_text(fp)])
}

/// The flow's steps, plugged into Back, Forward and the choices that
/// move on (F103, F104).
pub(crate) struct ConvertSteps;

/// The one [`ConvertSteps`].
pub(crate) static CONVERT_STEPS: ConvertSteps = ConvertSteps;

impl crate::trail::StepFlow for ConvertSteps {
    fn steps(&self, app: &Faraday) -> Option<crate::trail::Steps> {
        let s = state(app)?;
        let order = (0..cstep::COUNT as u8).collect();
        Some(crate::trail::Steps::new(order, s.open))
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        Action::Convert(ConvertAction::Step(k))
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        screen::TITLES[usize::from(k)]
    }
}
