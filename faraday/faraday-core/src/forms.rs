//! The other forms a key comes in, beside BIP-39 words, as Add a key
//! takes them (`docs/OPENSIGNER-PARITY.md` plan B2): SLIP-39 shares,
//! codex32 strings (BIP 93) and Seed XOR parts. Each is typed one at a
//! time, collected, and put together by `osk-bip` or `osk-entropy`, the
//! crates OpenSigner's own Load reads them with. A SLIP-39 or codex32 key
//! is a master seed and has no words; a Seed XOR key is BIP-39 words again.

use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::codex32::Codex32;
use osk_bip::slip39;
use osk_entropy::SeedXor;

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
            Form::Words => "BIP-39 words",
            Form::Slip39 => "SLIP-39 shares",
            Form::Codex32 => "codex32",
            Form::Xor => "Seed XOR parts",
        }
    }
}

/// The parts collected so far, erased on drop.
#[derive(Default)]
pub struct Parts {
    /// SLIP-39 shares.
    pub slip39: Vec<slip39::Share>,
    /// codex32 strings.
    pub codex32: Vec<Codex32>,
    /// Seed XOR parts.
    pub xor: SeedXor,
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
                    format!(
                        "Group {} of {}, member {} · {} of the group needed · id {:04x}",
                        s.group_index() + 1,
                        s.group_count(),
                        s.member_index() + 1,
                        s.member_threshold(),
                        s.identifier()
                    )
                })
                .collect(),
            Form::Codex32 => self
                .codex32
                .iter()
                .map(|c| {
                    let id = String::from_utf8_lossy(&c.identifier()).into_owned();
                    if c.is_secret() {
                        format!("The secret · id {id}")
                    } else {
                        format!(
                            "Share {} · id {id} · {} needed",
                            char::from(c.share_index()),
                            c.threshold()
                        )
                    }
                })
                .collect(),
            Form::Xor => (1..=self.xor.parts())
                .map(|i| {
                    let words = self.xor.strength().map_or(0, |s| s.words());
                    format!("Part {i} · {words} words")
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
            Form::Xor => self.xor.parts() >= osk_entropy::MIN_XOR_PARTS,
            Form::Words => false,
        }
    }
}

impl Faraday {
    /// The typed text as one more part. Returns the refusal, if any.
    pub(crate) fn form_add_part(&mut self) {
        let form = self.entry.form;
        let text = zeroize::Zeroizing::new(self.entry.typed.trim().to_lowercase());
        let result: Result<(), String> = match form {
            Form::Slip39 => slip39::Share::parse(&text)
                .map_err(|e| format!("Not a SLIP-39 share: {e:?}"))
                .and_then(|s| {
                    if self
                        .entry
                        .parts
                        .slip39
                        .iter()
                        .any(|t| t.indices() == s.indices())
                    {
                        return Err("That share is in already".to_string());
                    }
                    self.entry.parts.slip39.push(s);
                    Ok(())
                }),
            Form::Codex32 => Codex32::parse(&text)
                .map_err(|e| format!("Not a codex32 string: {e}"))
                .and_then(|c| {
                    if let Some(first) = self.entry.parts.codex32.first()
                        && first.identifier() != c.identifier()
                    {
                        return Err("That string is of another seed: its id differs".to_string());
                    }
                    if self
                        .entry
                        .parts
                        .codex32
                        .iter()
                        .any(|d| d.share_index() == c.share_index())
                    {
                        return Err("That share is in already".to_string());
                    }
                    self.entry.parts.codex32.push(c);
                    Ok(())
                }),
            Form::Xor => Mnemonic::parse(Language::English, &text)
                .map_err(|e| format!("Not BIP-39 words: {e}"))
                .and_then(|m| {
                    let e = m.entropy();
                    self.entry
                        .parts
                        .xor
                        .push(e.expose().as_bytes())
                        .map_err(|_| "Every part has the same number of words".to_string())
                }),
            Form::Words => Ok(()),
        };
        match result {
            Ok(()) => {
                zeroize::Zeroize::zeroize(&mut self.entry.typed);
                self.entry.typed.clear();
                self.entry.error = None;
                // A codex32 secret is the key already.
                if form == Form::Codex32 && self.entry.parts.ready(form) {
                    self.form_recover();
                }
            }
            Err(e) => self.entry.error = Some(e),
        }
    }

    /// Puts the parts together and adds the key.
    pub(crate) fn form_recover(&mut self) {
        if !self.may_load_keys() {
            return;
        }
        let form = self.entry.form;
        let label = match form {
            Form::Slip39 => "SLIP-39 key",
            Form::Codex32 => "codex32 key",
            _ => "Seed XOR key",
        };
        let added = match form {
            Form::Slip39 => {
                slip39::recover(&self.entry.parts.slip39, self.entry.passphrase.as_bytes())
                    .map_err(|e| format!("The shares do not make a key: {e:?}"))
                    .and_then(|secret| {
                        self.session
                            .add_seed(secret.expose().as_bytes(), label)
                            .map_err(|e| e.text())
                    })
            }
            Form::Codex32 => {
                let parts = &self.entry.parts.codex32;
                let seed = match parts.iter().find(|c| c.is_secret()) {
                    Some(s) => s.to_seed(),
                    None => Codex32::recover(parts).and_then(|s| s.to_seed()),
                };
                seed.map_err(|e| format!("The strings do not make a key: {e}"))
                    .and_then(|seed| {
                        self.session
                            .add_seed(seed.expose().bytes(), label)
                            .map_err(|e| e.text())
                    })
            }
            Form::Xor => self
                .entry
                .parts
                .xor
                .entropy()
                .map_err(|_| "Seed XOR needs two parts or more".to_string())
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
                        .add_words_with(&phrase, &self.entry.passphrase, label, None)
                        .map_err(|e| e.text())
                }),
            Form::Words => return,
        };
        match added {
            Ok(fp) => {
                self.toast(&format!("Key {} added", fp_text(fp)));
                let back = self.entry.back;
                self.entry = crate::EntryState::default();
                self.refresh_spend();
                self.screen = back.unwrap_or(crate::Screen::Wallets);
            }
            Err(e) => self.entry.error = Some(e),
        }
    }
}

/// The BIP-39 lists a laptop keyboard types: those written in Latin
/// letters, whose words are typed as their ASCII fold (`niño` as `nino`).
pub const LATIN: [Language; 6] = [
    Language::English,
    Language::Spanish,
    Language::French,
    Language::Italian,
    Language::Czech,
    Language::Portuguese,
];

/// A list's name.
pub fn language_name(lang: Language) -> &'static str {
    match lang {
        Language::English => "English",
        Language::Spanish => "Spanish",
        Language::French => "French",
        Language::Italian => "Italian",
        Language::Czech => "Czech",
        Language::Portuguese => "Portuguese",
        Language::Japanese => "Japanese",
        Language::Korean => "Korean",
        Language::ChineseSimplified => "Chinese (Simplified)",
        Language::ChineseTraditional => "Chinese (Traditional)",
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

/// The words in a file of them, as a person or a backup tool writes one:
/// lines starting `#` are notes, and a number before a word (`1.`, `2)`,
/// `03`) is its place, not a word.
pub fn file_words(text: &str) -> zeroize::Zeroizing<String> {
    let mut out = zeroize::Zeroizing::new(String::new());
    for line in text.lines().map(str::trim) {
        if line.starts_with('#') {
            continue;
        }
        for t in line.split_whitespace() {
            let place = t
                .trim_end_matches(['.', ')', ':'])
                .bytes()
                .all(|b| b.is_ascii_digit());
            if place {
                continue;
            }
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(t);
        }
    }
    out
}

/// The typed words as a mnemonic in `lang`, when every one is a word.
pub fn typed_mnemonic(lang: Language, typed: &str) -> Result<Mnemonic, String> {
    let mut idx = Vec::new();
    for (k, w) in typed.split_whitespace().enumerate() {
        idx.push(
            typed_index(lang, w).ok_or_else(|| {
                format!("Word {} is not on the {} list", k + 1, language_name(lang))
            })?,
        );
    }
    let m = Mnemonic::from_indices(lang, &idx).map_err(|e| e.to_string());
    zeroize::Zeroize::zeroize(&mut idx);
    m
}
