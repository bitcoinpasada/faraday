//! A copy of a seed by hand is checked by its form (`docs/NEW-WALLET.md`
//! §14.7): the words are typed back, a SeedQR is scanned. This is the
//! typed check, which types through the word entry Add a key uses
//! ([`crate::forms::word_key`]), and the rules for which checks a copy
//! item asks for.

use crate::backup::{self, CopyCheck};
use crate::forms::{self, WordKey, WordsMut};
use crate::plan::{self, CopyChecks, CopyForm, CopyForms};
use crate::secret_text::SecretText;
use crate::strings::EN;
use crate::strings::fill;
use crate::word_entry::WordEntry;
use crate::{BackupState, Faraday, Screen, wallet};
use osk_bip::bip39::Language;
use osk_crypto::work::Compute;
use osk_shell_api::Key as KeyIn;
use zeroize::Zeroize;

/// The typed check of a copy of the words while it is open: the words
/// typed so far, wiped whenever it is dropped.
pub struct WordCheck {
    /// The words, for a list typed on the computer's keyboard.
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
}

impl WordCheck {
    /// A check with nothing typed, for a seed of `count` words from
    /// `lang`; on an on-screen keyboard when `keyed`.
    fn new(lang: Language, count: usize, keyed: bool) -> WordCheck {
        WordCheck {
            typed: SecretText::new(),
            keys: keyed.then(|| forms::word_typer(lang)),
            shift: false,
            error: None,
            lang,
            count,
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
    fn indices(&self) -> Vec<u16> {
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

impl Faraday {
    /// The word entry the on-screen keys and the candidates press into.
    pub(crate) fn words_use(&mut self) -> WordsUse<'_> {
        if self.screen == Screen::Backup
            && let Some(c) = self.backup.as_mut().and_then(|b| b.check.as_mut())
        {
            return WordsUse {
                keys: &mut c.keys,
                shift: &mut c.shift,
                error: &mut c.error,
                lang: c.lang,
                in_check: true,
            };
        }
        let lang = self.entry.language();
        WordsUse {
            keys: &mut self.entry.keys,
            shift: &mut self.entry.shift,
            error: &mut self.entry.error,
            lang,
            in_check: false,
        }
    }

    /// The forms of the copies of seed `i` the backup's map keeps, read
    /// from the plan's map ([`Faraday::plan_mapped`]: once a frame while
    /// drawing).
    pub(crate) fn copy_forms(&self, c: &Compute, b: &BackupState, i: usize) -> CopyForms {
        let shape = self.plan_shape_now(b.wallet);
        plan::copy_forms_on(&self.plan_mapped(c, &shape, &b.answers), i)
    }

    /// What checking seed `i`'s copies asks for with the cameras listed
    /// now.
    pub(crate) fn copy_checks(
        &self,
        c: &Compute,
        b: &BackupState,
        i: usize,
        key: &wallet::Key,
    ) -> CopyChecks {
        self.copy_forms(c, b, i)
            .checks(!self.cameras.is_empty(), key.language == Language::English)
    }

    /// Whether every check seed `i`'s copy item asks for has matched in
    /// this backup. A scan of the drawn grid also stands for the words of
    /// a seed with no copy of its words.
    pub(crate) fn copy_checked(
        &self,
        c: &Compute,
        b: &BackupState,
        i: usize,
        key: &wallet::Key,
    ) -> bool {
        let fp = key.fingerprint().0;
        let checks = self.copy_checks(c, b, i, key);
        let has = |f: CopyForm| b.checked.contains(&(fp, f));
        (checks.typed || checks.scan)
            && (!checks.typed
                || has(CopyForm::Words)
                || (!self.copy_forms(c, b, i).words && has(CopyForm::SeedQr)))
            && (!checks.scan || has(CopyForm::SeedQr))
    }

    /// Whether seed `fp`'s copy item is done in the backup under way; a
    /// seed the backup has no item for is checked by any match.
    pub(crate) fn seed_copy_checked(&self, c: &Compute, fp: [u8; 4]) -> bool {
        let Some(b) = self.backup.as_ref() else {
            return false;
        };
        let at = self
            .backup_seed_list_now(b.wallet)
            .iter()
            .enumerate()
            .find_map(|(i, (_, k))| {
                let key = self.session.keys.get((*k)?)?;
                (key.fingerprint().0 == fp).then_some((i, key))
            });
        match at {
            Some((i, key)) => self.copy_checked(c, b, i, key),
            None => b.checked.iter().any(|(f, _)| *f == fp),
        }
    }

    /// Check my copy: opens the typed check, with nothing typed and the
    /// seed hidden.
    pub(crate) fn copy_check_open(&mut self) {
        if self.backup.as_ref().is_none_or(|b| b.check.is_some()) {
            return;
        }
        let check = self.copy_check_new();
        if let Some(b) = self.backup.as_mut() {
            b.reveal = false;
            b.check = check;
        }
    }

    /// Clear: the check starts again from word 1.
    pub(crate) fn copy_check_clear(&mut self) {
        let check = self.copy_check_new();
        if let Some(b) = self.backup.as_mut()
            && b.check.is_some()
        {
            b.check = check;
        }
    }

    fn copy_check_new(&self) -> Option<WordCheck> {
        let b = self.backup.as_ref()?;
        let key = self.session.keys.get(b.key)?;
        let count = key.words.as_ref()?.split_whitespace().count();
        let keyed = forms::on_screen(key.language) || self.compact;
        Some(WordCheck::new(key.language, count, keyed))
    }

    /// One key of the open check, typed through Add a key's word entry,
    /// with a word's four-digit number accepted.
    pub(crate) fn copy_check_key(&mut self, key: KeyIn) {
        let Some(c) = self.backup.as_mut().and_then(|b| b.check.as_mut()) else {
            return;
        };
        match forms::word_key(c.entry(), key) {
            // The word as typed is taken, as a space takes it.
            WordKey::Enter => {
                forms::word_key(c.entry(), KeyIn::Char(' '));
            }
            WordKey::Escape => {
                if let Some(b) = self.backup.as_mut() {
                    b.check = None;
                }
                return;
            }
            _ => {}
        }
        self.copy_check_after();
    }

    /// Compares the words with the seed once the last is taken, not
    /// before. A match is the item's words check, done: the words are
    /// wiped and the check closes. A word that differs is named; the
    /// words before it stay and typing resumes at it.
    pub(crate) fn copy_check_after(&mut self) {
        let c = self.compute();
        let Some(b) = self.backup.as_ref() else {
            return;
        };
        let Some(chk) = b.check.as_ref() else {
            return;
        };
        if chk.taken() < chk.count {
            return;
        }
        let Some(key) = self.session.keys.get(b.key) else {
            return;
        };
        let fp = key.fingerprint().0;
        let seed = key
            .words
            .as_ref()
            .and_then(|w| crate::wallet::parse_words(&c, key.language, w.as_str()).ok());
        let Some(seed) = seed else {
            return;
        };
        let mut typed = chk.indices();
        let found = backup::compare_words(&typed, seed.indices());
        let off_list = match found {
            CopyCheck::WrongWord(k) => typed.get(k - 1) == Some(&u16::MAX),
            CopyCheck::Matches => false,
        };
        typed.zeroize();
        drop(seed);
        match found {
            CopyCheck::Matches => {
                if let Some(b) = self.backup.as_mut() {
                    b.check = None;
                }
                self.copy_matched(fp, CopyForm::Words);
            }
            CopyCheck::WrongWord(k) => {
                let line = if off_list {
                    fill(
                        EN.forms_word_is_not_on,
                        &[&k.to_string(), forms::language_name(key_language(self))],
                    )
                } else {
                    fill(EN.copy_check_word_differs, &[&k.to_string()])
                };
                if let Some(c) = self.backup.as_mut().and_then(|b| b.check.as_mut()) {
                    c.keep(k - 1);
                    c.error = Some(line);
                }
            }
        }
    }
}

/// The list of the seed under check.
fn key_language(app: &Faraday) -> Language {
    app.backup
        .as_ref()
        .and_then(|b| b.check.as_ref())
        .map_or(Language::English, |c| c.lang)
}
