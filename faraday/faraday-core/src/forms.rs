//! The other forms a key comes in, beside BIP-39 words, as Add a key
//! takes them: SLIP-39 shares,
//! codex32 strings (BIP 93) and Seed XOR parts. Each is typed one at a
//! time, collected, and put together by `faraday-bip` or `faraday-entropy`, the
//! crates that read them. A SLIP-39 or codex32 key
//! is a master seed and has no words; a Seed XOR key is BIP-39 words again.

use crate::strings::EN;
use crate::strings::fill;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::codex32::Codex32;
use faraday_bip::slip39;
use faraday_entropy::SeedXor;

/// Which form Add a key is reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Form {
    /// BIP-39 words.
    #[default]
    Words,
    /// SLIP-39 shares.
    Slip39,
    /// codex32 strings: the secret, or shares of it.
    Codex32,
    /// Seed XOR parts, each BIP-39 words.
    Xor,
}

impl Form {
    /// Every form, in the order the row lists them.
    pub const ALL: [Form; 4] = [Form::Words, Form::Slip39, Form::Codex32, Form::Xor];

    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Form::Words => EN.forms_bip_39_words,
            Form::Slip39 => EN.forms_slip_39_shares,
            Form::Codex32 => EN.forms_codex32,
            Form::Xor => EN.forms_seed_xor_parts,
        }
    }
}

/// The parts collected so far, erased on drop.
pub struct Parts {
    /// SLIP-39 shares.
    pub slip39: Vec<slip39::Share>,
    /// codex32 strings.
    pub codex32: Vec<Codex32>,
    /// Seed XOR parts.
    pub xor: SeedXor,
    /// How many strings of the codex32 strings' set the open vaults keep
    /// at other indices (`docs/CODEX32.md` §9.5): they count towards
    /// `k`. A count, kept in step at every string taken.
    pub codex32_vaults: usize,
}

/// Room for every share a backup can have, made at the start: a list
/// that grows moves its shares to a larger buffer and frees the old one
/// as it is, which would leave a copy of each share behind.
const PART_ROOM: usize = 32;

impl Default for Parts {
    fn default() -> Self {
        Parts {
            slip39: Vec::with_capacity(PART_ROOM),
            codex32: Vec::with_capacity(PART_ROOM),
            xor: SeedXor::default(),
            codex32_vaults: 0,
        }
    }
}

impl Parts {
    /// One line per part collected, for the list on screen: what it is,
    /// never its words.
    pub fn lines(&self, form: Form) -> Vec<String> {
        match form {
            Form::Slip39 => self
                .slip39
                .iter()
                .map(|s| {
                    fill(
                        EN.forms_group_of_member,
                        &[
                            &(s.group_index() + 1).to_string(),
                            &s.group_count().to_string(),
                            &(s.member_index() + 1).to_string(),
                            &s.member_threshold().to_string(),
                            &format!("{:04x}", s.identifier()),
                        ],
                    )
                })
                .collect(),
            Form::Codex32 => self
                .codex32
                .iter()
                .map(|c| {
                    let id = String::from_utf8_lossy(&c.identifier()).into_owned();
                    if c.is_secret() {
                        fill(EN.forms_the_secret_id, &[&id.to_string()])
                    } else {
                        fill(
                            EN.forms_share_id_needed,
                            &[
                                &char::from(c.share_index().to_ascii_uppercase()).to_string(),
                                &id.to_string(),
                                &c.threshold().to_string(),
                            ],
                        )
                    }
                })
                .chain(
                    (self.codex32_vaults > 0)
                        .then(|| fill(EN.c32r_in_vaults, &[&self.codex32_vaults.to_string()])),
                )
                .collect(),
            Form::Xor => (1..=self.xor.parts())
                .map(|i| {
                    let words = self.xor.strength().map_or(0, |s| s.words());
                    fill(EN.forms_part_words, &[&i.to_string(), &words.to_string()])
                })
                .collect(),
            Form::Words => Vec::new(),
        }
    }

    /// Whether enough is in to put the key together.
    pub fn ready(&self, form: Form) -> bool {
        match form {
            // Enough members of the first share's group; a backup of several
            // groups is checked whole when the key is made.
            Form::Slip39 => self
                .slip39
                .first()
                .is_some_and(|s| self.slip39.len() >= usize::from(s.member_threshold())),
            Form::Codex32 => self.codex32.first().is_some_and(|c| {
                c.is_secret()
                    || self.codex32.len() + self.codex32_vaults >= usize::from(c.threshold())
            }),
            Form::Xor => self.xor.parts() >= faraday_entropy::MIN_XOR_PARTS,
            Form::Words => false,
        }
    }
}

/// A press on the codex32 form's own rows (`docs/CODEX32.md` §8.3,
/// §8.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum C32Press {
    /// **Keep the one held**: the string typed at an index already in is
    /// dropped.
    Keep,
    /// **Replace it**: the string typed takes the place of the one held.
    Replace,
    /// **Check a string**, or back to adding strings: the field checks
    /// one string rather than adding it.
    Check,
    /// **Check it**: the string in the field is checked.
    CheckIt,
    /// **Show the worksheet** of the string checked, or its final row
    /// alone again (§8.3).
    Sheet,
    /// **Make the rest** from the `k` strings in (§8.1 step 2).
    Rest,
    /// **Add a share** from the `k` strings in (§8.2).
    AddShare,
}

/// A string typed or scanned at a share index already in, which
/// differs from the one held (§8.5): both shown, their differing boxes
/// marked, until **Keep the one held** or **Replace it**. Both strings
/// are kept as text once, for drawing, and wiped on drop.
pub struct Clash {
    /// The string typed.
    pub string: Codex32,
    /// Its text, lowercase.
    pub text: faraday_bip::codex32::Codex32Ascii,
    /// The string held at that index, as text.
    pub held: faraday_bip::codex32::Codex32Ascii,
    /// The boxes the two differ in, counted from 1 at `m`.
    pub differ: Vec<usize>,
    /// The mark the string typed carries, where it went in by a
    /// correction.
    pub mark: Option<crate::codex32_correct::Mark>,
}

impl Clash {
    /// The clash between `string` and `held`, the strings' boxes
    /// compared once.
    pub fn of(
        string: Codex32,
        held: &Codex32,
        mark: Option<crate::codex32_correct::Mark>,
    ) -> Clash {
        let text = string.encode();
        let held = held.encode();
        let differ = text
            .as_str()
            .bytes()
            .zip(held.as_str().bytes())
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, _)| i + 1)
            .collect();
        Clash {
            string,
            text,
            held,
            differ,
            mark,
        }
    }
}

/// What **Check it** found of one string (§8.3). Public: what the string
/// is, never its data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// The checksum holds.
    pub holds: bool,
    /// What the string is, where its checksum holds ("Share C · id
    /// w0za · 2 needed"); where it fails, the refusal.
    pub what: String,
    /// Whether it is of the set typed in, where strings are in and its
    /// checksum holds.
    pub of_the_set: Option<bool>,
    /// The mark it carries, where it went in by a correction.
    pub mark: Option<crate::codex32_correct::Mark>,
}

/// The codex32 form's state beside the strings in: a clash waiting on
/// its answer, "Already in", and Check a string.
#[derive(Default)]
pub struct Codex32Form {
    /// A string at an index already in, waiting on Keep or Replace.
    pub clash: Option<Clash>,
    /// The last string typed was in already, the same.
    pub already: bool,
    /// Check a string: the field checks one string; nothing is kept or
    /// loaded.
    pub checking: bool,
    /// What the last Check it found.
    pub checked: Option<Checked>,
    /// The checksum worksheet of the string last checked, in the
    /// verification form, kept with what Check it found and dropped with
    /// it (§8.3).
    pub sheet: Option<faraday_keys::StringCheck>,
    /// **Show the worksheet** is open: every row of it, not only the
    /// final row.
    pub sheet_open: bool,
    /// Make the rest or Add a share was pressed: once `k` strings are in
    /// the make flow opens on them, in place of loading the key.
    pub making: Option<crate::codex32::Given>,
    /// How many strings of the set of the string last checked the open
    /// vaults keep (§9.5).
    pub check_in_vaults: usize,
    /// The strings in all Make the rest starts on, where Make codex32
    /// shares' Scheme card chose it for shares made by hand (§5.3); 0
    /// where none was chosen.
    pub rest_n: u8,
}

impl Codex32Form {
    /// Forgets what Check it found, its worksheet with it.
    pub fn forget_check(&mut self) {
        self.checked = None;
        self.sheet = None;
        self.sheet_open = false;
    }
}

/// The plain facts of the set the codex32 strings `typed` are of, for the
/// key they load (`docs/CODEX32.md` §9.4, "Loaded from strings"): k, the
/// identifier, the length and each index typed. How many strings exist
/// besides is the plan's question (§9.4), so none is counted here. `None`
/// with no string in.
pub fn set_facts(typed: &[Codex32]) -> Option<faraday_vault::codex32::SetFacts> {
    let first = typed.first()?;
    Some(faraday_vault::codex32::SetFacts {
        threshold: first.threshold(),
        identifier: first.identifier(),
        length: u8::try_from(first.encode().as_str().len()).unwrap_or(0),
        unknown: 0,
        strings: typed
            .iter()
            .map(|s| (s.share_index(), faraday_vault::codex32::State::Made))
            .collect(),
    })
}

/// Whether Make the rest and Add a share are offered: a string of a split
/// set is in (§8.1, §8.2). Pressed before the last of the `k` strings, the
/// strings then make the rest or a share in place of loading the key.
pub fn offers_more(parts: &Parts) -> bool {
    parts.codex32.first().is_some_and(|c| c.threshold() >= 2)
}

/// Whether `k` strings of a split set are in, the secret counting as one.
pub fn makes_more(parts: &Parts) -> bool {
    parts
        .codex32
        .first()
        .is_some_and(|c| c.threshold() >= 2 && parts.codex32.len() >= usize::from(c.threshold()))
}

/// The line a string that does not parse is refused with: a share or
/// the secret whose checksum fails is named by its index ("Share C ·
/// checksum fails", §8.1), anything else by what is wrong.
pub fn codex32_refusal(given: &str, e: &faraday_bip::codex32::Error) -> String {
    if *e == faraday_bip::codex32::Error::BadChecksum
        && let Some(&i) = given.as_bytes().get(8)
    {
        let i = i.to_ascii_lowercase();
        if i == b's' {
            return EN.codex32_form_secret_fails.to_string();
        }
        return fill(
            EN.codex32_form_share_fails,
            &[&char::from(i.to_ascii_uppercase()).to_string()],
        );
    }
    fill(EN.forms_not_a_codex32, &[&e.to_string()])
}

/// What a codex32 string is, by its header: "Share C · id w0za · 2
/// needed", or "The secret · id w0za".
pub fn codex32_what(c: &Codex32) -> String {
    let id = String::from_utf8_lossy(&c.identifier()).into_owned();
    if c.is_secret() {
        fill(EN.codex32_form_checked_secret, &[&id])
    } else {
        fill(
            EN.codex32_form_checked_share,
            &[
                &char::from(c.share_index().to_ascii_uppercase()).to_string(),
                &id,
                &c.threshold().to_string(),
            ],
        )
    }
}

/// Whether something stands in place of the codex32 form's field: a
/// clash, or the correction's screen.
pub fn codex32_replaced(e: &crate::EntryState) -> bool {
    e.codex32.clash.is_some() || e.correction.is_open()
}

/// Whether the codex32 form takes presses only: a clash, or the
/// correction's screen at any step but the boxes typed again from the
/// paper, stands in place of the field, so no keyboard is up for it.
pub fn codex32_pressed_only(e: &crate::EntryState) -> bool {
    e.form == Form::Codex32
        && (e.codex32.clash.is_some()
            || (e.correction.is_open()
                && e.correction.step() != Some(crate::codex32_correct::Step::Retype)))
}

/// Whether `c` is of the set `set` holds, where strings are in: the same
/// identifier, threshold and length as its first string.
pub fn of_the_set(c: &Codex32, set: &[Codex32]) -> Option<bool> {
    let first = set.first()?;
    Some(
        first.identifier() == c.identifier()
            && first.threshold() == c.threshold()
            && first.payload().len() == c.payload().len(),
    )
}

/// Every BIP-39 list words are typed from, in the order they are
/// offered. The six written in Latin letters are typed on the computer's
/// keyboard as their ASCII fold (`niño` as `nino`); the other four on an
/// on-screen keyboard of their own ([`on_screen`]).
pub const LANGUAGES: [Language; 10] = [
    Language::English,
    Language::Spanish,
    Language::French,
    Language::Italian,
    Language::Czech,
    Language::Portuguese,
    Language::Japanese,
    Language::Korean,
    Language::ChineseSimplified,
    Language::ChineseTraditional,
];

/// Whether a list's words are typed on an on-screen keyboard: kana for
/// Japanese, jamo for Korean, pinyin and 注音 readings for the two
/// Chinese lists. [`crate::word_entry::WordEntry`] does the typing, with
/// the keys it allows and the words it offers.
pub fn on_screen(lang: Language) -> bool {
    lang.script() != faraday_bip::bip39::Script::Latin
}

/// A word entry for `lang`. Up to 24 words; the count is however many
/// are taken when the key is added.
pub fn word_typer(lang: Language) -> Box<crate::word_entry::WordEntry> {
    Box::new(crate::word_entry::WordEntry::new(lang, 24))
}

/// Types `text`, words of `lang`, into `w` as a person does on the
/// keyboard: each word's letters, then the word taken.
pub(crate) fn type_words(w: &mut crate::word_entry::WordEntry, lang: Language, text: &str) {
    for word in text.split_whitespace() {
        for c in word.chars() {
            w.type_char(c);
        }
        take_typed(w, lang);
    }
}

/// A list's name.
pub fn language_name(lang: Language) -> &'static str {
    match lang {
        Language::English => EN.forms_english,
        Language::Spanish => EN.forms_spanish,
        Language::French => EN.forms_french,
        Language::Italian => EN.forms_italian,
        Language::Czech => EN.forms_czech,
        Language::Portuguese => EN.forms_portuguese,
        Language::Japanese => EN.forms_japanese,
        Language::Korean => EN.forms_korean,
        Language::ChineseSimplified => EN.forms_chinese_simplified,
        Language::ChineseTraditional => EN.forms_chinese_traditional,
    }
}

/// The list's word a typed word is, matched on its ASCII fold.
pub fn typed_index(lang: Language, typed: &str) -> Option<u16> {
    let chars: Vec<char> = typed.chars().collect();
    lang.candidates_typed(&chars).find(|&i| {
        lang.typed(i)
            .is_some_and(|t| t.as_chars() == chars.as_slice())
    })
}

/// The words a typed prefix can still become.
pub fn typed_candidates(lang: Language, prefix: &str) -> Vec<u16> {
    let chars: Vec<char> = prefix.chars().collect();
    lang.candidates_typed(&chars).collect()
}

/// The typed form of the one word a prefix can still become, for Tab.
pub fn completion(lang: Language, typed: &str) -> Option<String> {
    let prefix = typed.rsplit(' ').next().unwrap_or("");
    if prefix.is_empty() {
        return None;
    }
    let c = typed_candidates(lang, prefix);
    match c.as_slice() {
        [only] => lang.typed(*only).map(|t| t.as_chars().iter().collect()),
        _ => None,
    }
}

/// The one SLIP-39 word a prefix can still become, for Tab: the same
/// rule as [`completion`], over the 1024-word SLIP-39 list instead of a
/// BIP-39 one.
pub fn slip39_completion(typed: &str) -> Option<String> {
    let prefix = typed.rsplit(' ').next().unwrap_or("");
    if prefix.is_empty() {
        return None;
    }
    let mut c = slip39::candidates(prefix);
    let only = c.next()?;
    if c.next().is_some() {
        return None;
    }
    Some(slip39::word(only).to_string())
}

pub use faraday_intake::file_words;

/// The typed words as a mnemonic in `lang`, when every one is a word.
pub fn typed_mnemonic(lang: Language, typed: &str) -> Result<Mnemonic, String> {
    let mut idx = Vec::new();
    for (k, w) in typed.split_whitespace().enumerate() {
        idx.push(typed_index(lang, w).ok_or_else(|| {
            fill(
                EN.forms_word_not_in_list,
                &[&(k + 1).to_string(), language_name(lang)],
            )
        })?);
    }
    let m = faraday_keys::words::from_indices(lang, &idx).map_err(|e| e.to_string());
    zeroize::Zeroize::zeroize(&mut idx);
    m
}

/// Whether the keys typed for the word in progress on `w` already type
/// one word whole, with no other word on the list starting the same
/// way: the point at which typing takes it as Tab would. A read list's
/// word is whole once its tone leaves one character.
pub fn word_whole(w: &crate::word_entry::WordEntry, lang: Language) -> bool {
    let prefix = w.prefix();
    if prefix.is_empty() || !w.accepting() {
        return false;
    }
    let Some(only) = w.candidates().next() else {
        return false;
    };
    match lang.typed(only) {
        Some(t) => t.as_chars() == prefix,
        None => !w.awaiting_tone(),
    }
}

/// Takes the word typed whole on `w`: the one its letters spell, or the
/// only one they can be. Space, Tab, Enter and Add key end a word so on
/// a physical keyboard, as a press on its pill does on the screen.
pub fn take_typed(w: &mut crate::word_entry::WordEntry, lang: Language) {
    let prefix: String = w.prefix().iter().collect();
    if prefix.is_empty() {
        return;
    }
    let cands: Vec<u16> = w.candidates().collect();
    let n = cands
        .iter()
        .position(|&i| lang.word(i) == prefix)
        .or((cands.len() == 1).then_some(0));
    if let Some(n) = n {
        w.commit_candidate(n);
    }
}

/// What a key did in the word entry that the screen it is on acts on
/// (`docs/history/NEW-WALLET.md` §14.7 item 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WordKey {
    /// A word, or part of one, was typed or taken back.
    Edited,
    /// Enter.
    Enter,
    /// Escape.
    Escape,
    /// Nothing the entry takes.
    Ignored,
}

/// The state a word entry types into: Add a key's, and Check my copy's.
pub(crate) struct WordsMut<'a> {
    /// The words as typed, for a list typed on the computer's keyboard.
    pub typed: &'a mut crate::secret_text::SecretText,
    /// The word entry, for a list typed on an on-screen
    /// keyboard.
    pub keys: &'a mut Option<Box<crate::word_entry::WordEntry>>,
    /// The last refusal.
    pub error: &'a mut Option<String>,
    /// The list the words are from.
    pub lang: Language,
    /// A word may be typed as its four-digit number.
    pub numbers: bool,
}

/// The one word of the list a Latin entry's last word can still become,
/// as it is typed, when it is not that word already.
fn latin_take(typed: &mut crate::secret_text::SecretText, lang: Language) {
    if let Some(word) = completion(lang, typed) {
        let cut = typed.rfind(' ').map(|i| i + 1).unwrap_or(0);
        typed.truncate(cut);
        typed.push_str(&word);
    }
}

/// Handles one key of a word entry, typed on the computer's keyboard
/// into `typed` (a Latin list) or into the word entry `keys`:
/// letters; Space and Tab take the word as it is or as the one word its
/// letters can still become; a word typed whole and alone on its prefix
/// is taken at once; Backspace; with `numbers`, a word as its four-digit
/// number. Add a key and Check my copy both type through here.
pub(crate) fn word_key(w: WordsMut<'_>, key: faraday_shell_api::Key) -> WordKey {
    use faraday_shell_api::Key;
    let lang = w.lang;
    if let Some(wizard) = w.keys.as_mut() {
        let out = match key {
            Key::Char(' ') | Key::Tab => {
                take_typed(wizard, lang);
                WordKey::Edited
            }
            Key::Char(c) => {
                wizard.type_char(c);
                if word_whole(wizard, lang) {
                    take_typed(wizard, lang);
                }
                WordKey::Edited
            }
            Key::Backspace => {
                wizard.backspace();
                WordKey::Edited
            }
            Key::Enter => WordKey::Enter,
            Key::Escape => WordKey::Escape,
            _ => WordKey::Ignored,
        };
        *w.error = None;
        return out;
    }
    let typed = w.typed;
    match key {
        Key::Char(c) if c.is_ascii_alphabetic() => {
            // A number being typed takes no letter.
            if typed
                .rsplit(' ')
                .next()
                .is_some_and(|t| t.bytes().any(|b| b.is_ascii_digit()))
            {
                return WordKey::Ignored;
            }
            typed.push(c.to_ascii_lowercase());
            *w.error = None;
            let last = typed.rsplit(' ').next().unwrap_or("");
            if !last.is_empty() && completion(lang, typed).as_deref() == Some(last) {
                latin_take(typed, lang);
                typed.push(' ');
            }
            WordKey::Edited
        }
        Key::Char(c) if w.numbers && c.is_ascii_digit() => {
            let cut = typed.rfind(' ').map(|i| i + 1).unwrap_or(0);
            if typed[cut..].bytes().any(|b| b.is_ascii_alphabetic()) {
                return WordKey::Ignored;
            }
            typed.push(c);
            *w.error = None;
            if typed.len() - cut == 4 {
                let number: u16 = typed[cut..].parse().unwrap_or(u16::MAX);
                let at = typed[..cut].split_whitespace().count() + 1;
                typed.truncate(cut);
                match (number < 2048).then(|| lang.typed(number)).flatten() {
                    Some(t) => {
                        typed.push_str(&t.as_chars().iter().collect::<String>());
                        typed.push(' ');
                    }
                    None => {
                        *w.error = Some(fill(
                            EN.forms_word_not_in_list,
                            &[&at.to_string(), language_name(lang)],
                        ));
                    }
                }
            }
            WordKey::Edited
        }
        Key::Char(' ') => {
            if !typed.is_empty() && !typed.ends_with(' ') {
                let last = typed.rsplit(' ').next().unwrap_or("");
                if last.bytes().any(|b| b.is_ascii_digit()) {
                    return WordKey::Ignored;
                }
                if typed_index(lang, last).is_none() {
                    latin_take(typed, lang);
                }
                typed.push(' ');
            }
            WordKey::Edited
        }
        Key::Tab => {
            if completion(lang, typed).is_some() {
                latin_take(typed, lang);
                typed.push(' ');
            }
            WordKey::Edited
        }
        Key::Backspace => {
            typed.pop();
            *w.error = None;
            WordKey::Edited
        }
        Key::Enter => WordKey::Enter,
        Key::Escape => WordKey::Escape,
        _ => WordKey::Ignored,
    }
}
