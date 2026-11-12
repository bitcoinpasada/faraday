//! Convert a seed (`docs/SEED-CONVERSION.md`): a loaded key written in
//! another standard, the same wallet. Every conversion is `faraday-keys`'s
//! and reads its result back first; this area holds the flow, and the
//! result until the flow is left.

pub mod screen;
pub mod set;

pub use set::{SetAct, SetState};

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

/// The most SLIP-39 groups, and shares in a group: SLIP-39's own limit.
pub(crate) const MAX_SLIP: usize = faraday_bip::slip39::MAX_COUNT;

/// The cards, in order. A target with settings has its card between
/// [`cstep::TO`] and [`cstep::RESULT`]; a target without goes from To to
/// Result.
pub mod cstep {
    /// Which key.
    pub const FROM: u8 = 0;
    /// Which standard.
    pub const TO: u8 = 1;
    /// The target's settings, where it has any.
    pub const SETTINGS: u8 = 2;
    /// The value and its facts.
    pub const RESULT: u8 = 3;
    /// How many cards.
    pub const COUNT: usize = 4;
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
    /// Read an LND aezeed as the source: the entry opens, and returns here.
    Aezeed,
    /// Press a row of [`ROWS`], by its place there.
    To(u8),
    /// More (1) or fewer (-1) Seed XOR parts.
    Parts(i8),
    /// A higher (1) or lower (-1) codex32 threshold: 0, then 2 to 9.
    K(i8),
    /// More (1) or fewer (-1) codex32 shares made.
    N(i8),
    /// More (1) or fewer (-1) SLIP-39 groups, 1 to 16.
    SlipGroups(i8),
    /// A higher (1) or lower (-1) number of SLIP-39 groups needed.
    SlipGroupK(i8),
    /// A higher (1) or lower (-1) threshold of a SLIP-39 group, by its place.
    SlipK(u8, i8),
    /// More (1) or fewer (-1) shares in a SLIP-39 group, by its place.
    SlipN(u8, i8),
    /// The source A SLIP-39 set: its entry opens.
    SetSource,
    /// Hold the share typed.
    SetHold,
    /// Forget the set held and start again.
    SetAgain,
    /// Press one of the set's three actions.
    SetAct(SetAct),
    /// The next (1) or previous (-1) group with its quorum met.
    SetGroup(i8),
    /// A higher (1) or lower (-1) member number.
    SetMember(i8),
    /// More (1) or fewer (-1) shares lost or exposed in a group.
    Lost(u8, i8),
    /// A typed share of a group and member pressed destroyed.
    Destroy(u8, u8),
    /// Every other share held pressed destroyed.
    DestroyOthers,
    /// Make the result from the settings.
    Make,
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
    /// The words as Seed XOR parts.
    SeedXor,
    /// The seed as SLIP-39, 1 of 1.
    SlipOne,
    /// The seed as a SLIP-39 set: one group of k of n shares, or groups.
    SlipSet,
    /// The seed as codex32: the secret, or a set.
    Codex32,
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
            Target::SeedXor => EN.convert_xor_line,
            Target::SlipOne => EN.convert_slip39_line,
            Target::SlipSet => EN.convert_slip39_set_line,
            Target::Codex32 => EN.convert_codex32_line,
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
            Target::SeedXor => EN.convert_xor_restores,
            Target::SlipOne => EN.convert_slip39_restores,
            Target::SlipSet => EN.convert_slip39_restores,
            Target::Codex32 => EN.convert_codex32_restores,
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
            Target::SeedXor => EN.convert_secret_xor,
            Target::SlipOne => EN.convert_secret_slip39,
            Target::SlipSet => EN.convert_secret_slip39,
            Target::Codex32 => EN.convert_secret_codex32,
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
            Target::SeedXor => "seed-xor",
            Target::SlipOne => "slip39",
            Target::SlipSet => "slip39-set",
            Target::Codex32 => "codex32",
        }
    }

    /// Whether the target has a Settings card: the parts of Seed XOR, the
    /// threshold and count of a codex32 set, the groups and shares of a
    /// SLIP-39 set.
    pub(crate) fn has_settings(self) -> bool {
        matches!(self, Target::SeedXor | Target::Codex32 | Target::SlipSet)
    }

    /// Whether every press draws randomness: Seed XOR's random parts and
    /// a SLIP-39 set's identifier and shares. A codex32 set does only for
    /// a threshold above 0.
    pub(crate) fn draws_randomness(self) -> bool {
        matches!(self, Target::SeedXor | Target::SlipSet)
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

    /// Whether Check my copy is offered for a key on `lang`: a code's copy
    /// is scanned back, any other value's is typed. Another list's words
    /// may not be ASCII and are not compared: a copy typed by hand may be
    /// composed otherwise, and the keyboard may not hold them.
    fn checks(self, lang: Language) -> bool {
        !matches!(self, Target::Words | Target::SeedXor) || lang == Language::English
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
pub const ROWS: [Row; 17] = [
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
        name: EN.convert_xor,
        line: EN.convert_xor_line,
        target: Some(Target::SeedXor),
        applies: english_words,
    },
    Row {
        name: EN.convert_slip39,
        line: EN.convert_slip39_line,
        target: Some(Target::SlipOne),
        applies: |k| matches!(k.seed_len, Some(16 | 32)),
    },
    Row {
        name: EN.convert_slip39_set,
        line: EN.convert_slip39_set_line,
        target: Some(Target::SlipSet),
        applies: |k| matches!(k.seed_len, Some(16 | 32)),
    },
    Row {
        name: EN.convert_codex32,
        line: EN.convert_codex32_line,
        target: Some(Target::Codex32),
        applies: |k| k.seed_len.is_some(),
    },
    Row {
        name: EN.convert_no_slip39_64,
        line: EN.convert_no_slip39_64_line,
        target: None,
        applies: |k| k.seed_len == Some(64),
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
    /// The key chosen, by fingerprint; for an aezeed, the fingerprint of
    /// its BIP-32 seed.
    pub key: Option<[u8; 4]>,
    /// Whether the source is the aezeed `faraday-keys` holds for this
    /// flow (`Keys::aezeed_hold`), not a loaded key.
    pub aezeed: bool,
    /// The set source's state, where the source is A SLIP-39 set held by
    /// `faraday-keys` (`Keys::slip39_set_hold`).
    pub set: Option<SetState>,
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
    /// Whether the target pressed has a Settings card in this session.
    pub settings: bool,
    /// Seed XOR's parts, 2 to 4.
    pub parts: u8,
    /// SLIP-39 groups needed to recover: 1 for one group, else 2 to the
    /// groups' count.
    pub slip_t: u8,
    /// Each SLIP-39 group's shares needed and shares made, 1 to 16 groups.
    pub slip: Vec<(u8, u8)>,
    /// A codex32 set's threshold: 0 for the secret alone, else 2 to 9.
    pub k: u8,
    /// How many codex32 shares are made: 1 with k 0, else k to 31.
    pub n: u8,
}

/// What a press gave: text, or a code with its word count. Wiped on drop.
pub enum Value {
    /// A written form.
    Text(Held<Zeroizing<String>>),
    /// A SeedQR or CompactSeedQR, for a seed of this many words.
    Code(Box<Held<QrMatrix>>, usize),
    /// A result of several strings, each one a secret, numbered when shown.
    Set(Held<Vec<Zeroizing<String>>>),
}

impl Value {
    /// The text: a borrow, no work.
    pub(crate) fn text(&self) -> Option<&str> {
        match self {
            Value::Text(t) => Some(t.as_str()),
            Value::Code(..) | Value::Set(_) => None,
        }
    }

    /// What the field shows: the text, or a set's strings a line each,
    /// numbered when there are several, by group and place in it where
    /// `groups` (each group's share count) has several. A copy, wiped on
    /// drop; `None` for a code.
    pub(crate) fn display(&self, groups: &[u8]) -> Option<Zeroizing<String>> {
        match self {
            Value::Text(t) => Some(Zeroizing::new(t.to_string())),
            Value::Set(parts) => {
                let mut out = Zeroizing::new(String::new());
                for (i, part) in parts.iter().enumerate() {
                    if i > 0 {
                        out.push('\n');
                    }
                    if groups.len() > 1 {
                        let (g, m) = place_in(groups, i);
                        out.push_str(&fill(
                            EN.convert_group_share,
                            &[&g.to_string(), &m.to_string()],
                        ));
                        out.push(' ');
                    } else if parts.len() > 1 {
                        out.push_str(&(i + 1).to_string());
                        out.push_str(EN.convert_part_sep);
                        out.push(' ');
                    }
                    out.push_str(part);
                }
                Some(out)
            }
            Value::Code(..) => None,
        }
    }

    /// The code: a borrow, no work.
    pub(crate) fn code(&self) -> Option<&QrMatrix> {
        match self {
            Value::Code(m, _) => Some(m),
            Value::Text(_) | Value::Set(_) => None,
        }
    }
}

/// The group and the place in it, both from 1, of share `i` (from 0) of a
/// set whose groups have these share counts, in order.
pub(crate) fn place_in(groups: &[u8], i: usize) -> (usize, usize) {
    let mut left = i;
    for (g, &n) in groups.iter().enumerate() {
        if left < usize::from(n) {
            return (g + 1, left + 1);
        }
        left -= usize::from(n);
    }
    (groups.len(), left + 1)
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
    /// What the field shows of a text or set held (a set's strings
    /// joined and numbered), made once by the press that opens the eye
    /// and held while it is open (F195): wiped when the eye closes, the
    /// value is dropped, the flow is left and at a lock.
    shown_text: Option<Zeroizing<String>>,
    /// Check my copy, while it is open: what is typed, wiped on drop.
    check: Option<WordCheck>,
    /// The set's share being typed, wiped on drop and with the flow.
    set_typed: crate::secret_text::SecretText,
    /// What the last comparison found, until the next key or press.
    found: Option<bool>,
    /// Which part of a result of several the last copy matched, by number.
    found_part: Option<usize>,
}

impl ConvertArea {
    /// The text the open eye shows, held since the press: a borrow, no
    /// work. `None` with the eye closed or for a code.
    pub(crate) fn shown_text(&self) -> Option<&str> {
        self.shown_text.as_deref().map(String::as_str)
    }

    /// The flow's state, while it is open.
    pub(crate) fn state(&self) -> Option<&ConvertState> {
        self.state.as_ref()
    }

    /// Forgets the flow and its result.
    pub(crate) fn clear(&mut self) {
        self.state = None;
        self.set_typed.clear();
        self.drop_value();
    }

    /// Drops the value and what is typed to check it.
    fn drop_value(&mut self) {
        self.held = None;
        self.shown_text = None;
        self.check = None;
        self.found = None;
        self.found_part = None;
    }

    /// Forgets what the last comparison found.
    pub(crate) fn forget_found(&mut self) {
        self.found = None;
        self.found_part = None;
    }

    /// A copy scanned back: it matched or it differed.
    pub(crate) fn set_found(&mut self, same: bool) {
        self.found = Some(same);
        self.found_part = None;
        if let Some(s) = self.state.as_mut() {
            // The answer is at the top of the Result card.
            s.scroll.follow = true;
        }
    }

    /// Check my copy, while it is open.
    pub(crate) fn check(&self) -> Option<&WordCheck> {
        self.check.as_ref()
    }

    /// Opens Check my copy with nothing typed, or closes it.
    fn toggle_check(&mut self) {
        self.found = None;
        self.found_part = None;
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
        let (Some(check), Some(value)) = (self.check.as_mut(), self.held.as_ref()) else {
            return;
        };
        if check.typed.is_empty() {
            return;
        }
        // Against the text, or every part of a set; a set of several says
        // which part by its number.
        let (same, part) = match value {
            Value::Text(t) => (copy_is(target, &check.typed, t), None),
            Value::Set(parts) => {
                let hit = parts.iter().position(|p| copy_is(target, &check.typed, p));
                (
                    hit.is_some(),
                    hit.filter(|_| parts.len() > 1).map(|i| i + 1),
                )
            }
            Value::Code(..) => return,
        };
        check.typed.clear();
        self.found = Some(same);
        self.found_part = part;
        if let Some(s) = self.state.as_mut() {
            // The answer is at the top of the Result card.
            s.scroll.follow = true;
        }
        if same {
            self.check = None;
        }
    }

    /// A share typed and what is typed to check are wiped.
    fn wipe_typed(&mut self) {
        self.set_typed.clear();
        self.clear_check();
    }

    /// Clears what is typed.
    fn clear_check(&mut self) {
        self.found = None;
        self.found_part = None;
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
            Typed::Nothing | Typed::Words => {
                self.found = None;
                self.found_part = None;
            }
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

    /// How far the column is scrolled, units, while it is open.
    pub(crate) fn scroll_y(&self) -> Option<f32> {
        self.state.as_ref().map(|x| x.scroll.y)
    }

    /// The column scrolled to `y` units, while it is open.
    pub(crate) fn set_scroll_y(&mut self, y: f32) {
        if let Some(x) = self.state.as_mut() {
            x.scroll.y = y;
        }
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
            done: [key.is_some(), false, false, false],
            scroll: flow::Scroll::default(),
            key,
            aezeed: false,
            set: None,
            target: None,
            shown: false,
            back,
            checkable: false,
            refused: None,
            paths: Vec::new(),
            settings: false,
            parts: 2,
            slip_t: 1,
            slip: vec![(2, 3)],
            k: 0,
            n: 1,
        });
    }

    /// Drops the result, as leaving its card does; with `keep_target` the
    /// target pressed stays, for its Settings card.
    fn drop_result(&mut self, keep_target: bool) {
        self.drop_value();
        if let Some(s) = self.state.as_mut() {
            s.shown = false;
            s.checkable = false;
            s.refused = None;
            s.paths.clear();
            s.done[usize::from(cstep::SETTINGS)] = false;
            s.done[usize::from(cstep::RESULT)] = false;
            if !keep_target {
                s.target = None;
                s.settings = false;
                if let Some(x) = s.set.as_mut() {
                    x.act = None;
                }
                s.done[usize::from(cstep::TO)] = false;
            }
        }
    }

    /// Opens the Settings card of `target`, pressed under To.
    fn open_settings(&mut self, target: Target) {
        self.drop_result(false);
        if let Some(s) = self.state.as_mut() {
            s.target = Some(target);
            s.settings = true;
            s.done[usize::from(cstep::TO)] = true;
            s.open = Some(cstep::SETTINGS);
            s.scroll.follow = true;
        }
    }

    /// Changes Seed XOR's parts by `by`, within 2 to 4.
    fn parts(&mut self, by: i8) {
        if let Some(s) = self.state.as_mut() {
            s.parts = s.parts.saturating_add_signed(by).clamp(2, 4);
        }
    }

    /// Steps a codex32 set's threshold through 0 (the secret alone), 2 to
    /// 9, and keeps the count in step with it.
    fn threshold(&mut self, by: i8) {
        if let Some(s) = self.state.as_mut() {
            s.k = match (s.k, by.signum()) {
                (0, 1) => 2,
                (0, _) | (2, -1) => 0,
                (k, d) => k.saturating_add_signed(d).clamp(2, 9),
            };
            s.n = if s.k == 0 {
                1
            } else {
                s.n.clamp(s.k, 31).max(s.k)
            };
        }
    }

    /// Changes a codex32 set's count by `by`, from k to 31; the secret
    /// alone is one string.
    fn count(&mut self, by: i8) {
        if let Some(s) = self.state.as_mut()
            && s.k > 0
        {
            s.n = s.n.saturating_add_signed(by).clamp(s.k, 31);
        }
    }

    /// A press on the SLIP-39 shape (`action` one of the four Slip
    /// presses), for a key of `len` seed bytes: made only when the shape
    /// it gives is one the randomness can pay for ([`slip_fits`]); else it
    /// does nothing.
    fn slip_press(&mut self, action: ConvertAction, len: usize) {
        if let Some(s) = self.state.as_mut() {
            let (t, slip) = slip_after(s.slip_t, &s.slip, action);
            if slip_need(len, t, &slip) <= crate::fresh::TAKE_MAX {
                (s.slip_t, s.slip) = (t, slip);
            }
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
        match s.open {
            Some(cstep::RESULT) => {}
            // The target stays for its settings; the result does not.
            Some(cstep::SETTINGS) => self.drop_result(true),
            _ => self.drop_result(false),
        }
    }

    /// Chooses `key`, and To opens. Another key starts a SLIP-39 shape
    /// afresh: one the last key's length could pay for may be past this
    /// key's, and the card only takes shapes the randomness can pay for.
    fn choose(&mut self, key: [u8; 4]) {
        self.drop_result(false);
        if let Some(s) = self.state.as_mut() {
            if s.key != Some(key) {
                s.slip_t = 1;
                s.slip = vec![(2, 3)];
            }
            s.key = Some(key);
            s.aezeed = false;
            s.set = None;
            self.set_typed.clear();
            s.done[usize::from(cstep::FROM)] = true;
            s.open = Some(cstep::TO);
            s.scroll.follow = true;
        }
    }

    /// Chooses the aezeed `faraday-keys` now holds, whose BIP-32 seed has
    /// fingerprint `fp`, and To opens.
    pub(crate) fn choose_aezeed(&mut self, fp: Fingerprint) {
        self.choose(fp.0);
        if let Some(s) = self.state.as_mut() {
            s.aezeed = true;
        }
    }

    /// What the press gave for `target`, and Result opens.
    fn made(
        &mut self,
        target: Target,
        given: Result<Value, Convert>,
        checkable: bool,
        (paths, settings): (Vec<String>, bool),
    ) {
        self.drop_result(false);
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
            s.settings = settings;
            s.done[usize::from(cstep::TO)] = true;
            s.done[usize::from(cstep::SETTINGS)] = true;
            s.done[usize::from(cstep::RESULT)] = true;
            s.open = Some(cstep::RESULT);
            s.scroll.follow = true;
        }
    }
}

/// The SLIP-39 shape after `action` (a Slip press), kept to what the split
/// takes: groups 1 to 16, a group added 2 of 3; groups needed 1 for one
/// group, else 2 to the count; shares needed 1 for one share, else 2 to
/// the count; shares made 1 to 16. A lookup.
pub(crate) fn slip_after(t: u8, slip: &[(u8, u8)], action: ConvertAction) -> (u8, Vec<(u8, u8)>) {
    let (mut t, mut slip) = (t, slip.to_vec());
    match action {
        ConvertAction::SlipGroups(by) => {
            let n = slip.len().saturating_add_signed(isize::from(by));
            slip.resize(n.clamp(1, MAX_SLIP), (2, 3));
        }
        ConvertAction::SlipGroupK(by) => t = t.saturating_add_signed(by),
        ConvertAction::SlipK(g, by) => {
            if let Some(group) = slip.get_mut(usize::from(g)) {
                group.0 = group.0.saturating_add_signed(by);
            }
        }
        ConvertAction::SlipN(g, by) => {
            if let Some(group) = slip.get_mut(usize::from(g)) {
                group.1 = group.1.saturating_add_signed(by).clamp(1, MAX_SLIP as u8);
            }
        }
        _ => {}
    }
    let fit = |t: u8, of: u8| if of <= 1 { 1 } else { t.clamp(2, of) };
    t = fit(t, slip.len() as u8);
    for g in &mut slip {
        g.0 = fit(g.0, g.1);
    }
    (t, slip)
}

/// The fresh bytes a SLIP-39 set of this shape takes for a key of `len`
/// seed bytes. A lookup.
pub(crate) fn slip_need(len: usize, t: u8, slip: &[(u8, u8)]) -> usize {
    let groups: Vec<faraday_bip::slip39::GroupSpec> = slip
        .iter()
        .map(|&(threshold, count)| faraday_bip::slip39::GroupSpec { threshold, count })
        .collect();
    faraday_keys::slip39_random_len(len, t, &groups)
}

/// Whether `action` leaves a shape the randomness can pay for
/// ([`crate::fresh::TAKE_MAX`]), for a key of `len` seed bytes: a press
/// that does not is not taken, and its button shows disabled. A lookup.
pub(crate) fn slip_fits(len: usize, t: u8, slip: &[(u8, u8)], action: ConvertAction) -> bool {
    let (t, slip) = slip_after(t, slip, action);
    slip_need(len, t, &slip) <= crate::fresh::TAKE_MAX
}

/// The seed bytes of the key chosen: 32 where it has none, the longest a
/// SLIP-39 set takes. A lookup.
pub(crate) fn seed_len_of(keys: &faraday_keys::Keys, state: &ConvertState) -> usize {
    if state.set.is_some() {
        return keys.slip39_set_facts().map_or(32, |f| f.secret_len);
    }
    source_info(keys, state)
        .and_then(|k| k.seed_len)
        .unwrap_or(32)
}

/// The source chosen, if it is still there: a loaded key, or the aezeed
/// held for the flow. A lookup.
pub(crate) fn source_info<'a>(
    keys: &'a faraday_keys::Keys,
    state: &ConvertState,
) -> Option<&'a KeyInfo> {
    if state.aezeed {
        return keys.source_info(faraday_keys::Keys::AEZEED_SOURCE);
    }
    let fp = state.key?;
    keys.all().find(|k| k.fingerprint.0 == fp)
}

/// The flow's state, while it is open.
pub fn state(app: &Faraday) -> Option<&ConvertState> {
    app.tools.convert.state()
}

/// The value, while its eye is open and it is text: a borrow, no work.
pub fn shown(app: &Faraday) -> Option<&str> {
    app.tools.convert.held().and_then(Value::text)
}

/// The code, while its eye is open and it is one: a borrow, no work.
pub fn shown_code(app: &Faraday) -> Option<&QrMatrix> {
    app.tools.convert.held().and_then(Value::code)
}

/// The strings of a result of several, while its eye is open: a borrow, no
/// work.
pub fn shown_parts(app: &Faraday) -> Option<Vec<&str>> {
    match app.tools.convert.held()? {
        Value::Set(parts) => Some(parts.iter().map(|p| p.as_str()).collect()),
        _ => None,
    }
}

/// What Check my copy has typed, while it is open.
pub fn typed(app: &Faraday) -> Option<&str> {
    app.tools.convert.check().map(|c| &*c.typed)
}

/// Whether the last comparison found the copy right, until the next key.
pub fn found(app: &Faraday) -> Option<bool> {
    app.tools.convert.found
}

/// Each group's share count of the SLIP-39 set made, in order; empty for
/// any other result. A read of the settings, which stay as made until the
/// card is opened.
pub(crate) fn groups_made(s: &ConvertState) -> Vec<u8> {
    let rotated = s
        .set
        .as_ref()
        .is_some_and(|x| x.act == Some(SetAct::Rotate));
    if s.target == Some(Target::SlipSet) || rotated {
        s.slip.iter().map(|g| g.1).collect()
    } else {
        Vec::new()
    }
}

/// The cards shown, in order: Settings only where the target pressed has
/// any.
pub fn order(s: &ConvertState) -> Vec<u8> {
    let mut order = vec![cstep::FROM, cstep::TO];
    if s.settings && (s.target.is_some() || s.set.as_ref().is_some_and(|x| x.act.is_some())) {
        order.push(cstep::SETTINGS);
    }
    order.push(cstep::RESULT);
    order
}

/// Which part of a result of several the last copy matched, by number.
pub fn found_part(app: &Faraday) -> Option<usize> {
    app.tools.convert.found_part
}

/// Whether `target` is dimmed in this session: a target that draws
/// randomness makes a secret, which the demo does not.
pub fn dimmed(app: &Faraday, target: Target) -> bool {
    target.draws_randomness() && !app.makes_secrets()
}

/// Whether the Settings card is shown for `target`: Seed XOR's parts, or a
/// codex32 set's threshold and count where this session makes secrets.
fn has_settings(app: &Faraday, target: Target) -> bool {
    target.has_settings() && app.makes_secrets()
}

/// The place of the key chosen, if `target` is a row that applies to it. A
/// lookup.
fn key_for(app: &Faraday, target: Target) -> Option<usize> {
    let s = app.tools.convert.state()?;
    let i = if s.aezeed {
        faraday_keys::Keys::AEZEED_SOURCE
    } else {
        app.session.keys.position_of(s.key?)?
    };
    let info = app.session.keys.source_info(i)?;
    ROWS.iter()
        .any(|r| r.target == Some(target) && r.applies(info))
        .then_some(i)
}

/// Whether Check my copy on the result open is scanned, not typed.
pub(crate) fn checks_by_scan(app: &Faraday) -> bool {
    state(app).is_some_and(|s| s.checkable && s.target.is_some_and(Target::is_code))
}

/// Whether the open result is still the SeedQR (or, `compact`, the
/// CompactSeedQR) of the key `fp`, the one Check my copy's camera was
/// opened for. A lookup.
pub(crate) fn scan_is_for(
    app: &Faraday,
    fp: faraday_bip::keys::Fingerprint,
    compact: bool,
) -> bool {
    state(app)
        .filter(|s| s.checkable)
        .and_then(|s| s.target)
        .is_some_and(|t| {
            t.is_code()
                && (t == Target::CompactSeedQr) == compact
                && key_for(app, t)
                    .and_then(|i| app.session.keys.info(i))
                    .is_some_and(|k| k.fingerprint == fp)
        })
}

/// Check my copy on a SeedQR or CompactSeedQR result: the camera opens
/// to read the copy back, compared with the key's words
/// ([`Faraday::convert_copy_scanned`]). A lookup.
fn scan_open(app: &mut Faraday) {
    let Some(target) = state(app).filter(|s| s.checkable).and_then(|s| s.target) else {
        return;
    };
    let Some(fp) = key_for(app, target)
        .and_then(|i| app.session.keys.info(i))
        .map(|k| k.fingerprint)
    else {
        return;
    };
    app.tools.convert.forget_found();
    let mut scan = crate::ScanState::default();
    scan.purpose = crate::ScanPurpose::CheckConvert(fp, target == Target::CompactSeedQr);
    app.camera.open(scan);
    app.sheet = Some(crate::Sheet::Scan);
    app.out
        .commands
        .push_back(faraday_shell_api::Command::CameraOn);
}

/// The row at `place` of [`ROWS`], if it applies to the key chosen. A
/// lookup.
fn row_for(app: &Faraday, place: u8) -> Option<Target> {
    let target = ROWS.get(usize::from(place))?.target?;
    key_for(app, target).map(|_| target)
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

/// The Standard fact of a result: the target's, with what the settings and
/// the key made it, as made (the settings stay as they were until the card
/// is opened, which drops the result).
pub(crate) fn standard_text(s: &ConvertState, key: &KeyInfo, target: Target) -> String {
    match target {
        Target::SeedXor => fill(EN.convert_xor_standard, &[&s.parts.to_string()]),
        Target::Codex32 if s.settings && s.k > 0 => fill(
            EN.convert_codex32_set,
            &[&s.k.to_string(), &s.n.to_string()],
        ),
        Target::Codex32 => EN.convert_codex32_secret.to_string(),
        Target::SlipSet => slip_set_standard(s),
        // 16 bytes are 20 words, 32 bytes 33.
        Target::SlipOne => fill(
            EN.convert_slip39_standard,
            &[if key.seed_len == Some(16) { "20" } else { "33" }],
        ),
        _ => target.standard().to_string(),
    }
}

/// The Standard fact of a SLIP-39 set of the shape in the settings.
pub(crate) fn slip_set_standard(s: &ConvertState) -> String {
    if s.slip.len() > 1 {
        let shapes: Vec<String> = s
            .slip
            .iter()
            .map(|(k, n)| fill(EN.convert_slip39_shape, &[&k.to_string(), &n.to_string()]))
            .collect();
        return fill(
            EN.convert_slip39_groups_standard,
            &[
                &s.slip_t.to_string(),
                &s.slip.len().to_string(),
                &shapes.join(&format!("{} ", EN.convert_slip39_shape_sep)),
            ],
        );
    }
    let (k, n) = s.slip.first().copied().unwrap_or((2, 3));
    fill(
        EN.convert_slip39_set_standard,
        &[&k.to_string(), &n.to_string()],
    )
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

/// The Paths fact of an aezeed: the paths LND uses on this network.
fn aezeed_paths(app: &Faraday) -> Vec<String> {
    let coin = if app.session.keys.network() == faraday_bip::keys::Network::Mainnet {
        "0"
    } else {
        "1"
    };
    vec![fill(EN.convert_aezeed_paths, &[coin, coin, coin])]
}

/// The press on the row at `place`: a target with settings opens its
/// card, any other is made; a row not offered, or dimmed, does nothing.
fn press(app: &mut Faraday, place: u8) {
    let Some(target) = row_for(app, place) else {
        return;
    };
    if dimmed(app, target) {
        return;
    }
    if has_settings(app, target) {
        app.tools.convert.open_settings(target);
    } else {
        make(app, target);
    }
}

/// Costs what the target's operation costs: a PBKDF2 on a key of words for
/// `master_seed_hex` or `master_xprv`, a parse of the words or two for a
/// written form, a parse and a QR encoding and a scan of it for a code, a
/// parse of each part for Seed XOR, a master key from the seed read back
/// for SLIP-39 and codex32, and for a SLIP-39 set a PBKDF2 for the split
/// and two for the read back. The press on a row without settings, or Make;
/// a target that draws randomness waits for it, as New key's split does
/// ([`Faraday::fresh_bytes`]), and says so.
fn make(app: &mut Faraday, target: Target) {
    let Some(i) = key_for(app, target) else {
        return;
    };
    if dimmed(app, target) {
        return;
    }
    let settings = has_settings(app, target);
    let Some((parts, k, n, slip_t, slip)) = app
        .tools
        .convert
        .state()
        .map(|s| (s.parts, s.k, s.n, s.slip_t, s.slip.clone()))
    else {
        return;
    };
    let (parts, k, n) = if settings { (parts, k, n) } else { (2, 0, 1) };
    let groups: Vec<faraday_bip::slip39::GroupSpec> = slip
        .iter()
        .map(|&(threshold, count)| faraday_bip::slip39::GroupSpec { threshold, count })
        .collect();
    let need = match target {
        Target::SeedXor => faraday_keys::seed_xor_random_len(parts),
        Target::Codex32 => faraday_keys::codex32_random_len(k),
        Target::SlipSet => faraday_keys::slip39_random_len(
            app.session
                .keys
                .source_info(i)
                .and_then(|k| k.seed_len)
                .unwrap_or(32),
            slip_t,
            &groups,
        ),
        _ => 0,
    };
    let random = if need == 0 {
        Zeroizing::new(Vec::new())
    } else if let Some(draw) = app.fresh_bytes(b"convert split", need) {
        draw
    } else {
        return app.toast(EN.no_randomness);
    };
    let c = app.compute();
    let keys = &app.session.keys;
    // The identifiers other sets on this device carry: a codex32 set made
    // here draws one that is none of them.
    let taken_ids = crate::codex32::taken_ids(&crate::codex32::taken(
        &app.entry.state().parts.codex32,
        keys,
        app.backup.state().map(|b| &b.answers),
        &app.vaults,
    ));
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
        Target::SeedXor => keys
            .seed_xor_converted(&c, i, parts, &random, why)
            .map(Value::Set),
        Target::SlipOne => keys.slip39_one_of_one(&c, i, why).map(Value::Text),
        Target::SlipSet => keys
            .slip39_converted(&c, i, slip_t, &groups, &random, why)
            .map(Value::Set),
        Target::Codex32 => keys
            .codex32_converted(&c, i, k, n, (&random, &taken_ids), why)
            .map(Value::Set),
    };
    let Some(info) = keys.source_info(i) else {
        return;
    };
    let paths = if i == faraday_keys::Keys::AEZEED_SOURCE {
        aezeed_paths(app)
    } else {
        paths_of(app, info.fingerprint)
    };
    let checkable = target.checks(info.language);
    let area = &mut app.tools.convert;
    area.made(target, given, checkable, (paths, settings));
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
    let area = &app.tools.convert;
    let (Some(held), Some(s)) = (area.held.as_ref(), area.state()) else {
        return;
    };
    let (Some(target), Some(fp)) = (s.target, s.key) else {
        return;
    };
    let Some(k) = source_info(&app.session.keys, s) else {
        return;
    };
    let (lang, apart) = (
        k.language,
        k.has_passphrase && target.keeps_passphrase_apart(),
    );
    let fp = fp_text(Fingerprint(fp));
    let lower = fp.to_lowercase();
    let (name, bytes, keep) = match held {
        Value::Set(parts) => {
            let mut all = Zeroizing::new(Vec::new());
            for part in parts.iter() {
                all.extend_from_slice(part.as_bytes());
                all.push(b'\n');
            }
            (
                format!("{lower}-{}.txt", target.file_for(lang)),
                all,
                Keep::Note,
            )
        }
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
        forget_held(app);
        app.tools.convert.choose(fp);
    }
}

/// Wipes what faraday-keys holds for the flow: an aezeed and a typed set.
fn forget_held(app: &mut Faraday) {
    app.session.keys.aezeed_forget();
    app.session.keys.slip39_set_forget();
}

/// A stick is attached: the share entry and Check my copy close, and the
/// share and the copy typed go (whether or not `wipe` asks: a closed field
/// keeps nothing).
pub(crate) fn stick_attached(app: &mut Faraday, _wipe: bool) {
    // The share entry and Check my copy are the secret fields: they close,
    // and what they held is wiped with them.
    let area = &mut app.tools.convert;
    area.wipe_typed();
    area.check = None;
    area.close_share_entry();
}

/// A press in the flow.
pub(crate) fn act(app: &mut Faraday, action: ConvertAction) {
    match action {
        ConvertAction::Aezeed => app.lightning_for_convert(),
        ConvertAction::Open => {
            forget_held(app);
            let key = (app.session.keys.len() == 1).then(|| app.session.keys[0].fingerprint.0);
            let back = app.screen;
            app.tools.convert.open(key, back);
            app.screen = Screen::Convert;
        }
        ConvertAction::OpenOn(fp) => {
            if app.session.keys.position_of(fp).is_none() {
                return;
            }
            forget_held(app);
            let back = app.screen;
            // Explore is kept for the way back; what its eyes show is not.
            app.explore_hide();
            app.tools.convert.open(Some(fp), back);
            app.screen = Screen::Convert;
        }
        ConvertAction::Step(s) => app.tools.convert.step(s),
        ConvertAction::Key(fp) => {
            if app.session.keys.position_of(fp).is_some() {
                forget_held(app);
                app.tools.convert.choose(fp);
            }
        }
        ConvertAction::VaultKey(v, r) => from_vault(app, v, r),
        ConvertAction::To(place) => press(app, place),
        ConvertAction::Parts(by) => app.tools.convert.parts(by),
        ConvertAction::SlipGroups(_)
        | ConvertAction::SlipGroupK(_)
        | ConvertAction::SlipK(..)
        | ConvertAction::SlipN(..) => {
            let len = state(app).map_or(32, |s| seed_len_of(&app.session.keys, s));
            app.tools.convert.slip_press(action, len);
        }
        ConvertAction::K(by) => app.tools.convert.threshold(by),
        ConvertAction::N(by) => app.tools.convert.count(by),
        ConvertAction::SetSource
        | ConvertAction::SetHold
        | ConvertAction::SetAgain
        | ConvertAction::SetAct(_)
        | ConvertAction::SetGroup(_)
        | ConvertAction::SetMember(_)
        | ConvertAction::Lost(..)
        | ConvertAction::Destroy(..)
        | ConvertAction::DestroyOthers => set::act(app, action),
        ConvertAction::Make if state(app).is_some_and(|s| s.set.is_some()) => {
            set::act(app, action);
        }
        ConvertAction::Make => {
            let target = state(app).filter(|s| s.settings).and_then(|s| s.target);
            if let Some(target) = target {
                make(app, target);
            }
        }
        ConvertAction::Show => {
            let area = &mut app.tools.convert;
            if area.held.is_some()
                && let Some(s) = area.state.as_mut()
            {
                s.shown = !s.shown;
                let groups = groups_made(s);
                area.shown_text = if s.shown {
                    area.held.as_ref().and_then(|v| v.display(&groups))
                } else {
                    None
                };
            }
        }
        ConvertAction::Check if checks_by_scan(app) => {
            scan_open(app);
        }
        ConvertAction::Check => {
            let opening = app.tools.convert.check().is_none();
            if !(opening && crate::stick::refuses_secret(app)) {
                app.tools.convert.toggle_check();
            }
        }
        ConvertAction::Compare => {
            let typed = app
                .tools
                .convert
                .check()
                .is_some_and(|c| !c.typed.is_empty());
            if !(typed && crate::stick::refuses_secret(app)) {
                app.tools.convert.compare();
            }
        }
        ConvertAction::CheckClear => app.tools.convert.clear_check(),
        ConvertAction::Save if state(app).is_some_and(|s| s.set.is_some()) => set::save(app),
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
    let typing = app.tools.convert.entering_share() || app.tools.convert.check().is_some();
    if typing && crate::stick::refuses_secret_key(app, key) {
        return true;
    }
    if set::key(app, key) {
        return true;
    }
    if app.tools.convert.check_key(key) {
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
    let area = &mut app.tools.convert;
    area.shown_text = None;
    if let Some(s) = area.state.as_mut() {
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
                .is_some_and(|k| k.back == Screen::Convert))
        || (app.screen == Screen::Lightning && app.lightning.for_convert());
    if !kept {
        forget_held(app);
        app.tools.convert.clear();
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
        Some(crate::trail::Steps::new(order(s), s.open))
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        Action::Convert(ConvertAction::Step(k))
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        screen::TITLES[usize::from(k)]
    }
}
