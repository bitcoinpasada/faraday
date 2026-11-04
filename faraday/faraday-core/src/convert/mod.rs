//! Convert a seed (`docs/SEED-CONVERSION.md`): a loaded key written in
//! another standard, the same wallet. Every conversion is `faraday-keys`'s
//! and reads its result back first; this area holds the flow, and the
//! result until the flow is left.

pub mod screen;

use crate::copy_check::{Typed, WordCheck};
use crate::flow;
use crate::secrets::{Keep, SecretOut};
use crate::strings::{EN, fill};
use crate::wallet::fp_text;
use crate::{Action, Faraday, Screen};
use faraday_bip::bip39::Language;
use faraday_bip::keys::Fingerprint;
use faraday_codec::qr::QrMatrix;
use faraday_crypto::work::Compute;
use faraday_keys::operations::WordsForm;
use faraday_keys::{Held, KeyInfo, Reveal, operations::Convert};
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
    /// Open the flow on a loaded key, by fingerprint, to return to the
    /// screen pressed from. A key not loaded does nothing.
    OpenOn([u8; 4]),
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
    /// The key's words.
    Words,
    /// The words as a SeedQR.
    SeedQr,
    /// The words as a CompactSeedQR.
    CompactSeedQr,
    /// The words' entropy, as hex.
    HexEntropy,
    /// Each word's number on the list.
    WordNumbers,
    /// Each word's first four letters.
    Starts,
}

impl Target {
    /// The Standard fact.
    fn standard(self) -> &'static str {
        match self {
            Target::SeedHex => EN.convert_seed_line,
            Target::Xprv => EN.convert_xprv_line,
            Target::Words => EN.convert_words_line,
            Target::SeedQr => EN.convert_seedqr_line,
            Target::CompactSeedQr => EN.convert_compact_line,
            Target::HexEntropy => EN.convert_entropy_line,
            Target::WordNumbers => EN.convert_numbers_line,
            Target::Starts => EN.convert_starts_line,
        }
    }

    /// The Restores in fact.
    fn restores_in(self) -> &'static str {
        match self {
            Target::SeedHex => EN.convert_seed_restores,
            Target::Xprv => EN.convert_xprv_restores,
            Target::Words => EN.convert_words_restores,
            Target::SeedQr => EN.convert_seedqr_restores,
            Target::CompactSeedQr => EN.convert_compact_restores,
            Target::HexEntropy => EN.convert_entropy_restores,
            Target::WordNumbers => EN.convert_numbers_restores,
            Target::Starts => EN.convert_starts_restores,
        }
    }

    /// What the secret sheet calls the value.
    fn secret(self) -> &'static str {
        match self {
            Target::SeedHex => EN.convert_secret_seed,
            Target::Xprv => EN.convert_secret_xprv,
            Target::Words => EN.convert_secret_words,
            Target::SeedQr | Target::CompactSeedQr => EN.convert_secret_code,
            Target::HexEntropy => EN.convert_secret_entropy,
            Target::WordNumbers => EN.convert_secret_numbers,
            Target::Starts => EN.convert_secret_starts,
        }
    }

    /// Whether the value carries no list: numbers and entropy are the
    /// words' own, written without it, so another list's must be named.
    pub(crate) fn lacks_list(self) -> bool {
        matches!(self, Target::HexEntropy | Target::WordNumbers)
    }

    /// What the file's name holds beside the fingerprint, the list's name
    /// after it for a key not on the English list whose value carries none.
    fn file_for(self, lang: Language) -> String {
        if self.lacks_list() && lang != Language::English {
            format!("{}-{}", self.file(), lang.name())
        } else {
            self.file().to_string()
        }
    }

    /// What the file's name holds beside the fingerprint.
    fn file(self) -> &'static str {
        match self {
            Target::SeedHex => "seed-hex",
            Target::Xprv => "xprv",
            Target::Words => "words",
            Target::SeedQr => "seedqr",
            Target::CompactSeedQr => "compactseedqr",
            Target::HexEntropy => "entropy",
            Target::WordNumbers => "word-numbers",
            Target::Starts => "starts",
        }
    }

    /// Whether the value is a code, drawn and saved as a picture.
    pub(crate) fn is_code(self) -> bool {
        matches!(self, Target::SeedQr | Target::CompactSeedQr)
    }

    /// Whether the value leaves the key's passphrase out: another writing
    /// of the words, restored with the passphrase.
    pub(crate) fn keeps_passphrase_apart(self) -> bool {
        !matches!(self, Target::SeedHex | Target::Xprv)
    }

    /// Whether Check my copy is offered for a key on `lang`. A code's copy
    /// would be scanned back, which is not built. Another list's words may
    /// not be ASCII and are not compared: a copy typed by hand may be
    /// composed otherwise, and the keyboard may not hold them.
    fn checks(self, lang: Language) -> bool {
        !self.is_code() && (self != Target::Words || lang == Language::English)
    }
}

/// Whether `key` has words on the English list, which SeedQR and the
/// 4-letter starts are defined on.
fn english_words(key: &KeyInfo) -> bool {
    key.has_words() && key.language == Language::English
}

/// Whether `key` has words SeedQR is defined on: the English list, 12 or 24
/// of them.
fn seedqr_words(key: &KeyInfo) -> bool {
    english_words(key) && matches!(key.word_count, Some(12 | 24))
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
pub const ROWS: [Row; 12] = [
    Row {
        name: EN.convert_words,
        line: EN.convert_words_line,
        target: Some(Target::Words),
        applies: KeyInfo::has_words,
    },
    Row {
        name: EN.convert_seedqr,
        line: EN.convert_seedqr_line,
        target: Some(Target::SeedQr),
        applies: seedqr_words,
    },
    Row {
        name: EN.convert_compact,
        line: EN.convert_compact_line,
        target: Some(Target::CompactSeedQr),
        applies: seedqr_words,
    },
    Row {
        name: EN.convert_entropy,
        line: EN.convert_entropy_line,
        target: Some(Target::HexEntropy),
        applies: KeyInfo::has_words,
    },
    Row {
        name: EN.convert_numbers,
        line: EN.convert_numbers_line,
        target: Some(Target::WordNumbers),
        applies: KeyInfo::has_words,
    },
    Row {
        name: EN.convert_starts,
        line: EN.convert_starts_line,
        target: Some(Target::Starts),
        applies: english_words,
    },
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
    /// Whether the value has a Check my copy.
    pub checkable: bool,
    /// The line in place of the value, when it was not given.
    pub refused: Option<&'static str>,
    /// The Paths fact: the loaded wallets that hold the key, worked out on
    /// the press. Public text. It stays right because every way to change
    /// the wallets or keys leaves the flow, which drops it.
    pub paths: Vec<String>,
}

/// What a press gave: text, or a code with its word count. Wiped on drop.
pub enum Value {
    /// A written form.
    Text(Held<Zeroizing<String>>),
    /// A SeedQR or CompactSeedQR, for a seed of this many words.
    Code(Box<Held<QrMatrix>>, usize),
}

impl Value {
    /// The text: a borrow, no work.
    pub(crate) fn text(&self) -> Option<&str> {
        match self {
            Value::Text(t) => Some(t.as_str()),
            Value::Code(..) => None,
        }
    }

    /// The code: a borrow, no work.
    pub(crate) fn code(&self) -> Option<&QrMatrix> {
        match self {
            Value::Code(m, _) => Some(m),
            Value::Text(_) => None,
        }
    }
}

/// Whether a typed copy is `value`. The value's own text exactly, single
/// spaces being what separate words and starts. Word numbers read in lines
/// with a second number beside each, so a copy is the 1 to 2048 numbers
/// alone, or every pair as shown, whose two numbers agree; either with any
/// white space between them. Anything else differs.
fn copy_is(target: Option<Target>, typed: &str, value: &str) -> bool {
    if target == Some(Target::WordNumbers) {
        let mut numbers = value.split_whitespace().filter(|t| !t.starts_with('('));
        let mut shown = value.split_whitespace();
        typed.split_whitespace().eq(&mut shown) || typed.split_whitespace().eq(&mut numbers)
    } else {
        typed == value
    }
}

/// The flow and its result.
#[derive(Default)]
pub struct ConvertArea {
    state: Option<ConvertState>,
    /// The value, wiped on drop: dropped when the Result card or the flow
    /// is left, or the session locks. Drawing only reads it.
    held: Option<Value>,
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
        let checkable = self.state.as_ref().is_some_and(|s| s.checkable);
        self.check = match (&self.check, &self.held) {
            (None, Some(_)) if checkable => Some(WordCheck::passphrase()),
            _ => None,
        };
    }

    /// Compares what is typed with the value and says which. A match
    /// closes the check; a difference wipes what was typed, as the backup's
    /// passphrase check does. Nothing typed compares nothing.
    fn compare(&mut self) {
        let target = self.state.as_ref().and_then(|s| s.target);
        let (Some(check), Some(value)) = (
            self.check.as_mut(),
            self.held.as_ref().and_then(Value::text),
        ) else {
            return;
        };
        if check.typed.is_empty() {
            return;
        }
        let same = copy_is(target, &check.typed, value);
        check.typed.clear();
        self.found = Some(same);
        if let Some(s) = self.state.as_mut() {
            // The answer is at the top of the Result card.
            s.scroll.follow = true;
        }
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
            // Every value with a check is printable ASCII (another list's
            // words have none, `Target::checks`): anything else cannot be
            // part of a right copy, and is not typed.
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
    pub(crate) fn held(&self) -> Option<&Value> {
        self.held
            .as_ref()
            .filter(|_| self.state.as_ref().is_some_and(|s| s.shown))
    }

    /// Whether a value is held, masked or not.
    #[cfg(test)]
    pub(crate) fn holds_value(&self) -> bool {
        self.held.is_some()
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
            checkable: false,
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
            s.checkable = false;
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
        given: Result<Value, Convert>,
        checkable: bool,
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
            s.checkable = checkable && refused.is_none();
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

/// The value, while its eye is open and it is text: a borrow, no work.
pub fn shown(app: &Faraday) -> Option<&str> {
    app.tools.convert().held().and_then(Value::text)
}

/// The code, while its eye is open and it is one: a borrow, no work.
pub fn shown_code(app: &Faraday) -> Option<&QrMatrix> {
    app.tools.convert().held().and_then(Value::code)
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

/// Costs what the target's operation costs: a PBKDF2 on a key of words for
/// `master_seed_hex` or `master_xprv`, a parse of the words or two for a
/// written form, a parse and a QR encoding and a scan of it for a code. The press on the row
/// at `place`; a row not offered does nothing.
fn make(app: &mut Faraday, place: u8) {
    let Some((i, target)) = row_for(app, place) else {
        return;
    };
    let c = app.compute();
    let keys = &app.session.keys;
    let why = Reveal::PaperForm;
    let written = |form| keys.words_written(&c, i, form, why).map(Value::Text);
    let code = |compact| {
        let (grid, words) = keys.seedqr_written(&c, i, compact, why)?;
        if !scan_reads(app, &c, i, &grid, compact) {
            return Err(Convert::NotSameWallet);
        }
        Ok(Value::Code(Box::new(grid), words))
    };
    let given = match target {
        Target::SeedHex => keys.master_seed_hex(&c, i, why).map(Value::Text),
        Target::Xprv => keys.master_xprv(&c, i, why).map(Value::Text),
        Target::Words => written(WordsForm::Words),
        Target::SeedQr => code(false),
        Target::CompactSeedQr => code(true),
        Target::HexEntropy => written(WordsForm::HexEntropy),
        Target::WordNumbers => written(WordsForm::Numbers),
        Target::Starts => written(WordsForm::Starts),
    };
    let paths = paths_of(app, keys[i].fingerprint);
    let checkable = target.checks(keys[i].language);
    let area = app.tools.convert_mut();
    area.made(target, given, checkable, paths);
}

/// Costs a scan of the code and a parse of the words. Whether a scanner
/// reading `grid`'s modules gets key `i`'s own SeedQR: the payload is read
/// by [`faraday_codec::decode::decode_luma`] and checked against the key's
/// words, in the digits' form, whichever the code is (a CompactSeedQR's
/// entropy bytes are written as the words' digits first). The picture
/// of the code and the payload are wiped when done.
fn scan_reads(app: &Faraday, c: &Compute, i: usize, grid: &Held<QrMatrix>, compact: bool) -> bool {
    let (w, h, luma) = grid.to_luma(6, 4);
    let luma = Zeroizing::new(luma);
    let read = faraday_codec::decode::decode_luma(c, w, h, &luma);
    let [one] = read.as_slice() else {
        return false;
    };
    let digits;
    let payload: &[u8] = if compact {
        let Ok(m) = faraday_keys::words::from_compact_seedqr(&one.bytes, Language::English) else {
            return false;
        };
        digits = faraday_codec::seedqr::to_digits(&m);
        digits.expose().as_bytes()
    } else {
        &one.bytes
    };
    app.seedqr_reads_as(c, i, payload) == Some(true)
}

/// Costs a picture for a code. Offers the value on the secret sheet: nothing
/// is written until the sheet is answered, and what it writes is the value
/// alone, or the code's labelled picture, which no vault is offered.
fn save(app: &mut Faraday) {
    let c = app.compute();
    let area = app.tools.convert();
    let (Some(held), Some(s)) = (area.held.as_ref(), area.state()) else {
        return;
    };
    let (Some(target), Some(fp)) = (s.target, s.key) else {
        return;
    };
    let Some(k) = app.session.keys.all().find(|k| k.fingerprint.0 == fp) else {
        return;
    };
    let (lang, apart) = (
        k.language,
        k.has_passphrase && target.keeps_passphrase_apart(),
    );
    let fp = fp_text(Fingerprint(fp));
    let lower = fp.to_lowercase();
    let (name, bytes, keep) = match held {
        Value::Text(text) => (
            format!("{lower}-{}.txt", target.file_for(lang)),
            Zeroizing::new(text.as_str().as_bytes().to_vec()),
            Keep::Note,
        ),
        Value::Code(grid, words) => (
            format!("{}-{lower}.png", target.file()),
            crate::backup_flow::seedqr_picture(&c, grid, *words, &fp),
            Keep::None,
        ),
    };
    let gives = if apart {
        EN.convert_secret_gives_apart
    } else {
        EN.convert_secret_gives
    };
    let mut out = SecretOut::new(name, bytes, target.secret(), gives);
    out.keep = keep;
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
        ConvertAction::OpenOn(fp) => {
            if app.session.keys.position_of(fp).is_none() {
                return;
            }
            let back = app.screen;
            // Explore is kept for the way back; what its eyes show is not.
            app.explore_hide();
            app.tools.convert_mut().open(Some(fp), back);
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

/// Closes the eye: the idle warning, so that nothing secret stays on
/// screen while no one is there.
pub(crate) fn hide(app: &mut Faraday) {
    if let Some(s) = app.tools.convert_mut().state.as_mut() {
        s.shown = false;
    }
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
