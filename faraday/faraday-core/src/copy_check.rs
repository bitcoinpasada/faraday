//! A copy of a seed by hand is checked by its form (`docs/history/NEW-WALLET.md`
//! §14.7): the words are typed back, a SeedQR is scanned. This is the
//! typed check, which types through the word entry Add a key uses
//! ([`crate::forms::word_key`]), and the rules for which checks a copy
//! item asks for. A passphrase on paper is checked the same way, typed
//! back whole with the passphrase hidden (§14.7 item 5). The presses
//! that open, type into and compare the check are the wallet backup's
//! (`crate::wallet_backup`).

use crate::BackupState;
use crate::forms::{self, WordKey, WordsMut};
use crate::secret_text::SecretText;
use crate::word_entry::WordEntry;
use faraday_bip::bip39::Language;
use faraday_shell_api::Key as KeyIn;

/// The typed check of a copy of the words, or of a passphrase on paper,
/// while it is open: what is typed so far, wiped whenever it is dropped.
pub struct WordCheck {
    /// The words, for a list typed on the computer's keyboard; or the
    /// passphrase.
    pub typed: SecretText,
    /// The word entry, for a list typed on an on-screen
    /// keyboard and on the small panel.
    pub keys: Option<Box<WordEntry>>,
    /// The on-screen keyboard's shift is on.
    pub shift: bool,
    /// What the last comparison found, until the next key.
    pub error: Option<String>,
    /// The seed's own list.
    pub lang: Language,
    /// The seed's word count.
    pub count: usize,
    /// A passphrase is typed, whole, not the words.
    pub passphrase: bool,
    /// The person pressed in the grid or typed: **Fill words** is gone.
    pub sample_gone: bool,
}

impl WordCheck {
    /// A check with nothing typed, for a seed of `count` words from
    /// `lang`; on an on-screen keyboard when `keyed`.
    pub(crate) fn new(lang: Language, count: usize, keyed: bool) -> WordCheck {
        WordCheck {
            typed: SecretText::new(),
            keys: keyed.then(|| forms::word_typer(lang)),
            shift: false,
            error: None,
            lang,
            count,
            passphrase: false,
            sample_gone: false,
        }
    }

    /// A check of a passphrase on paper, with nothing typed.
    pub(crate) fn passphrase() -> WordCheck {
        WordCheck {
            passphrase: true,
            ..WordCheck::new(Language::English, 0, false)
        }
    }

    /// How many words are taken: all but the one being typed.
    pub fn taken(&self) -> usize {
        match &self.keys {
            Some(w) => w.committed(),
            None => {
                let n = self.typed.split_whitespace().count();
                if self.typed.ends_with(' ') {
                    n
                } else {
                    n.saturating_sub(1)
                }
            }
        }
    }

    /// The word being typed so far.
    pub fn typing(&self) -> String {
        match &self.keys {
            Some(w) => w.prefix().iter().collect(),
            None if self.typed.ends_with(' ') => String::new(),
            None => self.typed.rsplit(' ').next().unwrap_or("").to_string(),
        }
    }

    /// The words taken, as the list's numbers; a word not on the list as
    /// `u16::MAX`. Costs a lookup a word. The caller wipes them.
    pub(crate) fn indices(&self) -> Vec<u16> {
        match &self.keys {
            Some(w) => w.committed_indices().collect(),
            None => self
                .typed
                .split_whitespace()
                .take(self.taken())
                .map(|t| forms::typed_index(self.lang, t).unwrap_or(u16::MAX))
                .collect(),
        }
    }

    /// The word entry's state, for the keys typed.
    fn entry(&mut self) -> WordsMut<'_> {
        WordsMut {
            typed: &mut self.typed,
            keys: &mut self.keys,
            error: &mut self.error,
            lang: self.lang,
            numbers: true,
        }
    }

    /// Keeps the first `keep` words taken and wipes the rest.
    fn keep(&mut self, keep: usize) {
        match self.keys.as_mut() {
            Some(w) => {
                while w.committed() > keep {
                    w.backspace();
                    while !w.prefix().is_empty() {
                        w.backspace();
                    }
                }
            }
            None => {
                let mut cut = 0;
                for (n, t) in self.typed.split(' ').enumerate() {
                    if n == keep {
                        break;
                    }
                    cut += t.len() + 1;
                }
                self.typed.truncate(cut);
            }
        }
    }
    /// One key typed into the check, through Add a key's word entry,
    /// with a word's four-digit number accepted: what it asks of the
    /// backup.
    pub(crate) fn key(&mut self, key: KeyIn) -> Typed {
        self.sample_gone = true;
        if self.passphrase {
            match key {
                // BIP-39 passphrases here are printable ASCII.
                KeyIn::Char(ch) if (' '..='~').contains(&ch) => {
                    self.typed.push(ch);
                    self.error = None;
                }
                // Anything else is refused, and the check says so.
                KeyIn::Char(ch) if !ch.is_control() => {
                    self.error = Some(crate::state::ascii_refused());
                }
                KeyIn::Backspace | KeyIn::Delete => {
                    self.typed.pop();
                    self.error = None;
                }
                KeyIn::Enter => return Typed::Compare,
                KeyIn::Escape => return Typed::Close,
                _ => {}
            }
            return Typed::Nothing;
        }
        match forms::word_key(self.entry(), key) {
            // The word as typed is taken, as a space takes it.
            WordKey::Enter => {
                forms::word_key(self.entry(), KeyIn::Char(' '));
            }
            WordKey::Escape => return Typed::Close,
            _ => {}
        }
        Typed::Words
    }

    /// Word `k` differs, as `line` says: the words before it stay and
    /// typing resumes at it.
    pub(crate) fn wrong(&mut self, k: usize, line: String) {
        self.keep(k - 1);
        self.error = Some(line);
    }
}

/// What a key typed into the check asks of the backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Typed {
    /// Nothing more.
    Nothing,
    /// The check closes.
    Close,
    /// The passphrase typed is compared.
    Compare,
    /// The words typed are compared once the last is taken.
    Words,
}

/// The state a word entry on screen types into: Check my copy's while it
/// is open on Back up, else Add a key's.
pub(crate) struct WordsUse<'a> {
    pub keys: &'a mut Option<Box<WordEntry>>,
    pub shift: &'a mut bool,
    pub error: &'a mut Option<String>,
    pub lang: Language,
    pub in_check: bool,
}

/// The scans without a match after which **Keep the words only** moves
/// to the item's foot (`docs/history/NEW-WALLET.md` §14.7 item 3).
pub const MISSES: u8 = 3;

impl BackupState {
    /// How many scans of seed `fp`'s SeedQR copy closed without a match
    /// in this backup.
    pub fn misses_of(&self, fp: [u8; 4]) -> u8 {
        self.misses
            .iter()
            .find(|(f, _)| *f == fp)
            .map_or(0, |(_, n)| *n)
    }
}

// ---------------------------------------------------------------------
// The SeedQR's copy grid, marked by blocks or rows
// (`docs/BACKUP-FLOW.md` §17.24)
// ---------------------------------------------------------------------

/// The last mark on a SeedQR grid copied by hand: block 35 of the
/// largest, 29 × 29 (six blocks a side); a smaller grid's last is less,
/// and [`pin_on`] keeps the mark on it.
pub const PIN_MAX: usize = 35;

/// The side of the SeedQR of a seed of `words` words, standard or
/// compact: 25 or 29 squares standard, 21 or 25 compact.
pub fn seedqr_side(words: u8, compact: bool) -> usize {
    match (words > 12, compact) {
        (false, false) | (true, true) => 25,
        (true, false) => 29,
        (false, true) => 21,
    }
}

/// Whether `key` moves the copy grid's mark on backup `b`: on a copy
/// item, Up and Down always, Left and Right only block by block.
pub(crate) fn copy_key(b: &BackupState, key: KeyIn) -> bool {
    b.open.is_some_and(crate::bstep::is_copy) && (matches!(key, KeyIn::Up | KeyIn::Down) || !b.rows)
}

/// Backup `b`'s grid mark kept on a SeedQR of side `n`: at most its last
/// row, or its last block.
pub(crate) fn pin_on(b: &BackupState, n: usize) -> usize {
    let count = if b.rows {
        n
    } else {
        crate::screens::copy_blocks(n).pow(2)
    };
    b.pin.min(count.saturating_sub(1))
}

/// The words' BIP-39 numbers a drawn SeedQR holds: a Standard SeedQR's
/// digits four to a word, or a Compact SeedQR's 16 or 32 bytes. The
/// digits are read without the checksum, so a copy with one word wrong
/// still reads and the word is named. Wiped when dropped.
pub(crate) fn copy_indices(bytes: &[u8]) -> Option<zeroize::Zeroizing<Vec<u16>>> {
    if !bytes.is_empty() && bytes.iter().all(u8::is_ascii_digit) {
        if !matches!(bytes.len(), 48 | 96) {
            return None;
        }
        let mut out = zeroize::Zeroizing::new(Vec::with_capacity(bytes.len() / 4));
        for chunk in bytes.chunks(4) {
            let v = chunk
                .iter()
                .fold(0u16, |v, &c| v * 10 + u16::from(c - b'0'));
            if v >= 2048 {
                return None;
            }
            out.push(v);
        }
        return Some(out);
    }
    let m = faraday_keys::words::from_compact_seedqr(bytes, faraday_bip::bip39::Language::English)
        .ok()?;
    Some(zeroize::Zeroizing::new(m.indices().to_vec()))
}
