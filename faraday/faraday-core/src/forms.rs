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

use crate::Faraday;
use crate::wallet::fp_text;

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
                                &char::from(c.share_index()).to_string(),
                                &id.to_string(),
                                &c.threshold().to_string(),
                            ],
                        )
                    }
                })
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
            Form::Codex32 => self
                .codex32
                .first()
                .is_some_and(|c| c.is_secret() || self.codex32.len() >= usize::from(c.threshold())),
            Form::Xor => self.xor.parts() >= faraday_entropy::MIN_XOR_PARTS,
            Form::Words => false,
        }
    }
}

impl Faraday {
    /// The typed text as one more part. Returns the refusal, if any.
    pub(crate) fn form_add_part(&mut self) {
        let c = self.compute();
        let form = self.entry.form;
        let text = zeroize::Zeroizing::new(self.entry.typed.trim().to_lowercase());
        let result: Result<(), String> = match form {
            Form::Slip39 => slip39::Share::parse(&text)
                .map_err(|e| fill(EN.forms_not_a_slip_39, &[&format!("{e:?}")]))
                .and_then(|s| {
                    if self
                        .entry
                        .parts
                        .slip39
                        .iter()
                        .any(|t| t.indices() == s.indices())
                    {
                        return Err(EN.forms_that_share_is.to_string());
                    }
                    self.entry.parts.slip39.push(s);
                    Ok(())
                }),
            Form::Codex32 => Codex32::parse(&text)
                .map_err(|e| fill(EN.forms_not_a_codex32, &[&e.to_string()]))
                .and_then(|c| {
                    if let Some(first) = self.entry.parts.codex32.first()
                        && first.identifier() != c.identifier()
                    {
                        return Err(EN.forms_that_string_is.to_string());
                    }
                    if self
                        .entry
                        .parts
                        .codex32
                        .iter()
                        .any(|d| d.share_index() == c.share_index())
                    {
                        return Err(EN.forms_that_share_is.to_string());
                    }
                    self.entry.parts.codex32.push(c);
                    Ok(())
                }),
            Form::Xor => crate::wallet::parse_words(&c, Language::English, &text)
                .map_err(|e| fill(EN.forms_not_bip_39_words, &[&e.to_string()]))
                .and_then(|m| {
                    let e = m.entropy();
                    self.entry
                        .parts
                        .xor
                        .push(e.expose().as_bytes())
                        .map_err(|_| EN.forms_every_part_has.to_string())
                }),
            Form::Words => Ok(()),
        };
        match result {
            Ok(()) => {
                self.entry.typed.clear();
                self.entry.error = None;
                // A codex32 secret is the key already: Add a key adds it,
                // New key opens its Key card.
                if form == Form::Codex32 && self.entry.parts.ready(form) {
                    if self.screen == crate::Screen::KeyGen {
                        self.act(crate::Action::KNext);
                    } else {
                        self.form_recover();
                    }
                }
            }
            Err(e) => self.entry.error = Some(e),
        }
    }

    /// Puts the parts together and adds the key.
    pub(crate) fn form_recover(&mut self) {
        if !self.may_load_keys() || self.entry.form == Form::Words {
            return;
        }
        match self.form_key() {
            Ok(fp) => {
                self.toast(&fill(EN.forms_key_added, &[&fp_text(fp).to_string()]));
                let back = self.entry.back;
                self.seeds_took(fp, back);
                self.entry = crate::EntryState::default();
                self.refresh_spend();
                self.screen = back.unwrap_or(crate::Screen::Wallets);
                if back == Some(crate::Screen::Family) {
                    self.family_key_added();
                }
            }
            Err(e) => self.entry.error = Some(e),
        }
    }

    /// Puts the parts typed together and loads the key they make, with
    /// the passphrase typed (not for codex32); what is wrong otherwise.
    /// Add a key's Make the key and New key's Lock in for a form other
    /// than words both load through here (`docs/history/NEW-WALLET.md` §15.2).
    pub(crate) fn form_key(&mut self) -> Result<faraday_bip::keys::Fingerprint, String> {
        let c = self.compute();
        let form = self.entry.form;
        let label = match form {
            Form::Slip39 => EN.forms_slip_39_key,
            Form::Codex32 => EN.forms_codex32_key,
            _ => EN.forms_seed_xor_key,
        };
        match form {
            Form::Slip39 => slip39::recover(
                &c,
                &self.entry.parts.slip39,
                self.entry.passphrase.as_bytes(),
            )
            .map_err(|e| fill(EN.forms_the_shares_do, &[&format!("{e:?}")]))
            .and_then(|secret| {
                self.session
                    .add_seed(&c, secret.expose().as_bytes(), label)
                    .map_err(|e| e.text())
            }),
            Form::Codex32 => {
                let parts = &self.entry.parts.codex32;
                let seed = match parts.iter().find(|c| c.is_secret()) {
                    Some(s) => s.to_seed(),
                    None => Codex32::recover(parts).and_then(|s| s.to_seed()),
                };
                seed.map_err(|e| fill(EN.forms_the_strings_do, &[&e.to_string()]))
                    .and_then(|seed| {
                        self.session
                            .add_seed(&c, seed.expose().bytes(), label)
                            .map_err(|e| e.text())
                    })
            }
            Form::Xor => self
                .entry
                .parts
                .xor
                .entropy()
                .map_err(|_| EN.forms_seed_xor_needs.to_string())
                .and_then(|e| {
                    Mnemonic::from_entropy(Language::English, e.as_bytes())
                        .map_err(|e| e.to_string())
                })
                .and_then(|m| {
                    let words: Vec<&str> = m
                        .indices()
                        .iter()
                        .map(|&i| Language::English.word(i))
                        .collect();
                    let phrase = zeroize::Zeroizing::new(words.join(" "));
                    self.session
                        .add_words_with(&c, &phrase, &self.entry.passphrase, label, None)
                        .map_err(|e| e.text())
                }),
            // Words are loaded as words (`Faraday::typed_load`), never here.
            Form::Words => Err(String::new()),
        }
    }
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
                EN.forms_word_is_not_on,
                &[&(k + 1).to_string(), language_name(lang)],
            )
        })?);
    }
    let m = Mnemonic::from_indices(lang, &idx).map_err(|e| e.to_string());
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
                            EN.forms_word_is_not_on,
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
