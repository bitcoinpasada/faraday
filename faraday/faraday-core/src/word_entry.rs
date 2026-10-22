//! A list's words typed on its own on-screen keyboard: kana, jamo,
//! pinyin and 注音, and the Latin keyboard on a small panel. The keys a
//! word allows, the words a prefix can still become, the readings of the
//! two Chinese lists and their tones, and BIP-39's last-word filter.
//!
//! Everything secret is inline and fixed-size: word indices are `u16`
//! and the prefix is a fixed array of keys.
//! The whole struct is zeroized on drop. This file is listed in
//! `tools/lint-secrets.sh` and may hold no heap text.

use crate::{Action, Command, EntryState, Faraday, Screen, forms};
use faraday_bip::bip39::{self, Language, Script, Typed};
use faraday_bip::kana;
use faraday_bip::slip39;
use faraday_draw::widgets::keyboard::{self, KeyMask, KeyboardKind};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// The list a word entry runs over: one of BIP-39's ten, or SLIP-39's
/// own 1024 words. The prefix
/// search, the candidate strip, the keyboard mask and the quiz's decoys
/// are written once over this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryList {
    /// A BIP-39 wordlist.
    Bip39(Language),
    /// The SLIP-39 wordlist: English, spelled on the Latin keyboard, and
    /// a word is identified by its first four letters.
    Slip39,
}

impl EntryList {
    /// The keyboard this list is typed on.
    pub fn keyboard(self) -> KeyboardKind {
        match self {
            EntryList::Bip39(lang) => keyboard_for(lang),
            EntryList::Slip39 => KeyboardKind::Bip39,
        }
    }

    /// Characters in the longest word as a reader sees it.
    pub fn max_display_chars(self) -> usize {
        match self {
            EntryList::Bip39(lang) => lang.max_display_chars(),
            EntryList::Slip39 => slip39::MAX_DISPLAY_CHARS,
        }
    }

    /// How many words the list holds.
    pub fn word_count(self) -> u16 {
        match self {
            EntryList::Bip39(_) => faraday_bip::wordlists::WORDLIST_LEN as u16,
            EntryList::Slip39 => slip39::WORDLIST_LEN as u16,
        }
    }

    /// The word at `idx` in its published form.
    pub fn word(self, idx: u16) -> &'static str {
        match self {
            EntryList::Bip39(lang) => lang.word(idx),
            EntryList::Slip39 => slip39::word(idx),
        }
    }

    /// Every word whose published form starts with `prefix`, in list
    /// order. The quiz draws its decoys from this.
    pub fn candidates(self, prefix: &str) -> impl Iterator<Item = u16> + '_ {
        (0..self.word_count()).filter(move |&i| self.word(i).starts_with(prefix))
    }

    /// The keys that type the word at `idx`.
    pub fn typed(self, idx: u16) -> Option<Typed> {
        match self {
            EntryList::Bip39(lang) => lang.typed(idx),
            EntryList::Slip39 => bip39::fold(slip39::word(idx)),
        }
    }

    /// Runs `f` over every word whose typed form starts with `prefix`,
    /// in list order, stopping when it returns `false`.
    fn each_typed(self, prefix: &[char], mut f: impl FnMut(u16) -> bool) {
        match self {
            EntryList::Bip39(lang) => {
                for i in lang.candidates_typed(prefix) {
                    if !f(i) {
                        return;
                    }
                }
            }
            EntryList::Slip39 => {
                for i in 0..slip39::WORDLIST_LEN as u16 {
                    if self.typed(i).is_some_and(|t| t.starts_with(prefix)) && !f(i) {
                        return;
                    }
                }
            }
        }
    }

    /// Whether any word's typed form starts with `prefix`.
    fn any_typed(self, prefix: &[char]) -> bool {
        let mut any = false;
        self.each_typed(prefix, |_| {
            any = true;
            false
        });
        any
    }
}

/// The keyboard a wordlist is typed on. The two Chinese lists are
/// looked up by a Mandarin reading rather than spelled, so they have
/// keyboards of their own: pinyin for Simplified, 注音 for Traditional.
fn keyboard_for(lang: Language) -> KeyboardKind {
    match lang.script() {
        Script::Latin => KeyboardKind::Bip39,
        Script::Kana => KeyboardKind::Kana,
        Script::Jamo => KeyboardKind::Jamo,
        Script::Pinyin => KeyboardKind::Pinyin,
        Script::Zhuyin => KeyboardKind::Zhuyin,
    }
}

/// Most candidates the strip lists on a spelled list, whose cell is as
/// wide as the widest word it can hold.
const MAX_CANDIDATES: usize = 8;

/// Most it lists on a list whose words are one character: two rows of
/// ten at the keyboard's key pitch, which is the largest tone group
/// either Chinese list has.
const MAX_CANDIDATES_HANZI: usize =
    faraday_draw::tokens::CANDIDATE_ROWS_HANZI * faraday_draw::tokens::CANDIDATE_CELLS_HANZI;

/// A SLIP-39 share's two word counts, in the order a screen lists
/// them: 20 words carry a 128-bit master secret, 33 a 256-bit one.
pub const SLIP39_COUNTS: [u8; 2] = [slip39::MIN_WORDS as u8, slip39::MAX_WORDS as u8];

/// Words the entry holds at once: a SLIP-39 share's 33, which is more
/// than BIP-39's 24.
const ENTRY_WORDS: usize = slip39::MAX_WORDS;

/// Bytes a syllable's spelling takes at most: sixteen keys, and a 注音
/// key is three bytes of UTF-8.
const SPELLING_BYTES: usize = bip39::MAX_TYPED * 3;

/// What the keys typed mean on a keyboard that looks a character up by
/// its Mandarin reading: the syllable's spelling in the list's script,
/// and the tone once the tone key has been pressed. The tone key is
/// always the last one — a digit on the pinyin keyboard, one of the
/// five marks on the 注音 one — so the split needs no state.
#[derive(Clone, Copy)]
struct Reading {
    buf: [u8; SPELLING_BYTES],
    len: usize,
    tone: Option<u8>,
}

impl Reading {
    fn of(kind: KeyboardKind, keys: &[char]) -> Self {
        let mut r = Reading {
            buf: [0; SPELLING_BYTES],
            len: 0,
            tone: None,
        };
        for (i, &c) in keys.iter().enumerate() {
            if i + 1 == keys.len()
                && let Some(tone) = tone_of(kind, c)
            {
                r.tone = Some(tone);
                break;
            }
            let mut bytes = [0u8; 4];
            let encoded = c.encode_utf8(&mut bytes);
            if r.len + encoded.len() <= SPELLING_BYTES {
                r.buf[r.len..r.len + encoded.len()].copy_from_slice(encoded.as_bytes());
                r.len += encoded.len();
            }
        }
        r
    }

    fn spelling(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

/// The tone a key stands for, or `None` for a key that spells.
fn tone_of(kind: KeyboardKind, c: char) -> Option<u8> {
    match kind {
        KeyboardKind::Pinyin => ('1'..='5').contains(&c).then(|| c as u8 - b'0'),
        KeyboardKind::Zhuyin => keyboard::ZHUYIN_MARKS
            .iter()
            .position(|&m| m == c)
            .map(|i| i as u8 + 1),
        _ => None,
    }
}

/// The mask bit of the key that gives `tone`. Every tone has a key on
/// both Chinese keyboards: a digit on the pinyin one, a mark on the
/// 注音 one, ˉ included.
fn tone_bit(kind: KeyboardKind, tone: u8) -> KeyMask {
    match kind {
        KeyboardKind::Pinyin if (1..=5).contains(&tone) => {
            keyboard::key_bit(kind, char::from(b'0' + tone))
        }
        KeyboardKind::Zhuyin if (1..=5).contains(&tone) => {
            keyboard::key_bit(kind, keyboard::ZHUYIN_MARKS[usize::from(tone) - 1])
        }
        _ => 0,
    }
}

/// Whether `kind` looks a word up by its reading rather than spelling it.
fn reads(kind: KeyboardKind) -> bool {
    matches!(kind, KeyboardKind::Pinyin | KeyboardKind::Zhuyin)
}

/// A word entry over one BIP-39 list. See the module documentation.
pub struct WordEntry {
    count: u8,
    lang: Language,
    words: [u16; ENTRY_WORDS],
    committed: u8,
    prefix: Typed,
    /// Bitmask over the wordlist of valid final words, computed when the
    /// penultimate word is committed.
    allowed_last: Option<[u64; 32]>,
}

impl Zeroize for WordEntry {
    fn zeroize(&mut self) {
        self.words.zeroize();
        self.committed = 0;
        self.prefix.zeroize();
        if let Some(mask) = &mut self.allowed_last {
            mask.zeroize();
        }
        self.allowed_last = None;
    }
}

impl Drop for WordEntry {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for WordEntry {}

impl WordEntry {
    /// An empty entry of `count` words of `lang`: 12, 15, 18, 21 or 24.
    /// Any other count is 12.
    pub fn new(lang: Language, count: u8) -> Self {
        WordEntry {
            count: if faraday_entropy::WORD_COUNTS.contains(&count) {
                count
            } else {
                12
            },
            lang,
            words: [0; ENTRY_WORDS],
            committed: 0,
            prefix: Typed::new(),
            allowed_last: None,
        }
    }

    /// The list the words are typed from.
    fn list(&self) -> EntryList {
        EntryList::Bip39(self.lang)
    }

    /// Number of committed words.
    pub fn committed(&self) -> usize {
        usize::from(self.committed)
    }

    /// The committed words' wordlist indices, for the masked panel of
    /// the words accepted so far (`docs/DESIGN.md` §4.3).
    pub fn committed_indices(&self) -> impl Iterator<Item = u16> + '_ {
        self.words[..self.committed()].iter().copied()
    }

    /// The keys typed so far for the current word.
    pub fn prefix(&self) -> &[char] {
        self.prefix.as_chars()
    }

    /// The keyboard this entry's wordlist is typed on.
    pub fn keyboard(&self) -> KeyboardKind {
        self.list().keyboard()
    }

    /// Whether the current word is the last one.
    fn typing_last(&self) -> bool {
        self.committed + 1 == self.count
    }

    /// Whether `i` is a valid final word for the committed words.
    fn last_allowed(&self, i: u16) -> bool {
        match self.allowed_last {
            Some(mask) if self.typing_last() => mask[usize::from(i >> 6)] >> (i & 63) & 1 == 1,
            _ => true,
        }
    }

    /// Runs `f` over the words that match what has been typed, in list
    /// order, stopping when it returns `false`. One place decides which
    /// prefix search a wordlist uses: the key sequence for the eight
    /// spelled lists, the syllable and its tone for the two read ones.
    fn for_each_candidate(&self, mut f: impl FnMut(u16) -> bool) {
        let kind = self.keyboard();
        if reads(kind) {
            let r = Reading::of(kind, self.prefix.as_chars());
            let tone = r.tone;
            if kind == KeyboardKind::Pinyin {
                for i in self.lang.candidates_pinyin(r.spelling(), tone) {
                    if !f(i) {
                        return;
                    }
                }
            } else {
                for i in self.lang.candidates_zhuyin(r.spelling(), tone) {
                    if !f(i) {
                        return;
                    }
                }
            }
        } else {
            self.list().each_typed(self.prefix.as_chars(), f);
        }
    }

    /// Candidate indices, in list order, and how many were found: one
    /// more than the strip holds means more follow. Empty until a key is
    /// pressed, and on the two read lists until the tone completes the
    /// reading ([`awaiting_tone`](Self::awaiting_tone)). For the final
    /// word, only the words whose checksum bits fit are offered while any
    /// match the prefix; when none do, every matching word is offered so
    /// that a wrong earlier word still leads somewhere rather than to a
    /// dead end.
    fn candidate_page(&self) -> ([u16; MAX_CANDIDATES_HANZI + 1], usize) {
        let mut out = [0u16; MAX_CANDIDATES_HANZI + 1];
        // One more than the strip holds is what says more follow, so the
        // search stops there.
        let cap = (self.capacity() + 1).min(out.len());
        let mut n = 0;
        if self.prefix.is_empty() || self.awaiting_tone() {
            return (out, 0);
        }
        let mut filtered = false;
        self.for_each_candidate(|i| {
            filtered = self.last_allowed(i);
            !filtered
        });
        self.for_each_candidate(|i| {
            if filtered && !self.last_allowed(i) {
                return true;
            }
            out[n] = i;
            n += 1;
            n < cap
        });
        (out, n)
    }

    /// Candidates the strip holds on this entry's list.
    fn capacity(&self) -> usize {
        if self.list().max_display_chars() == 1 {
            MAX_CANDIDATES_HANZI
        } else {
            MAX_CANDIDATES
        }
    }

    /// Candidate indices the strip offers now.
    pub fn candidates(&self) -> impl Iterator<Item = u16> {
        let (buf, n) = self.candidate_page();
        buf.into_iter().take(n.min(self.capacity()))
    }

    /// Whether one candidate is left, which the strip outlines.
    pub fn accepting(&self) -> bool {
        self.candidate_page().1 == 1
    }

    /// Whether a reading is being typed and its tone has not been
    /// pressed yet. A syllable without a tone leaves far too many
    /// characters to choose between, so the strip stays empty until the
    /// reading is complete — and an incomplete reading is not an error,
    /// so the screen says nothing either.
    pub fn awaiting_tone(&self) -> bool {
        let kind = self.keyboard();
        reads(kind) && Reading::of(kind, self.prefix.as_chars()).tone.is_none()
    }

    /// The keys some reading still allows: the character keys that
    /// extend the spelling, the tone keys that end it, and whether what
    /// has been typed is already a whole syllable of some word.
    fn reading_keys(&self, kind: KeyboardKind, spelling: &str) -> (KeyMask, KeyMask, bool) {
        let mut chars = 0;
        let mut tones = 0;
        let mut whole = false;
        for i in 0..2048u16 {
            for (reading, tone) in self.lang.word_readings(i) {
                let Some(rest) = reading.strip_prefix(spelling) else {
                    continue;
                };
                match rest.chars().next() {
                    Some(c) => chars |= keyboard::key_bit(kind, c),
                    None => {
                        whole = true;
                        tones |= tone_bit(kind, tone);
                    }
                }
            }
        }
        (chars, tones, whole)
    }

    /// The keys that would type the first reading of the word at `idx`,
    /// for the field a backspace re-opens.
    fn reading_typed(&self, idx: u16) -> Option<Typed> {
        let kind = self.keyboard();
        let (spelling, tone) = self.lang.word_readings(idx).next()?;
        let mut out = Typed::new();
        for c in spelling.chars() {
            out.push(c);
        }
        match kind {
            KeyboardKind::Pinyin => {
                out.push(char::from(b'0' + tone.clamp(1, 5)));
            }
            _ => {
                out.push(keyboard::ZHUYIN_MARKS[usize::from(tone.clamp(1, 5)) - 1]);
            }
        }
        Some(out)
    }

    /// Whether some word of the list continues the prefix with `c`.
    fn continues_with(&self, c: char) -> bool {
        let n = self.prefix.len();
        let list = self.list();
        let mut found = false;
        list.each_typed(self.prefix.as_chars(), |i| {
            found = list
                .typed(i)
                .is_some_and(|t| t.as_chars().get(n) == Some(&c));
            !found
        });
        found
    }

    /// The prefix with the last kana swapped between its full-size and
    /// its small form, which is what the 小 key does.
    fn small_toggled(&self) -> Option<Typed> {
        let last = self.prefix.last()?;
        let other = kana::small(last).or_else(|| kana::large(last))?;
        let mut swapped = self.prefix;
        swapped.set_last(other);
        Some(swapped)
    }

    /// Keys that extend the prefix towards some word of the list. The
    /// final word is not restricted here, for the reason given on
    /// [`candidates`](Self::candidates).
    ///
    /// A key is live when a candidate's next character is typed by it: on
    /// the kana keyboard a small kana lights its full-size key, since 小
    /// is what makes it small, and the two voicing marks light ゛ and ゜.
    /// 小 itself is live when swapping the last kana for its other form
    /// leads somewhere.
    pub fn enabled_keys(&self) -> KeyMask {
        let kind = self.keyboard();
        if reads(kind) {
            // A tone ends the syllable: nothing follows it but a
            // backspace, or a tap on one of the characters it left.
            let r = Reading::of(kind, self.prefix.as_chars());
            if r.tone.is_some() {
                return 0;
            }
            let (chars, tones, _) = self.reading_keys(kind, r.spelling());
            return chars | tones;
        }
        let n = self.prefix.len();
        let list = self.list();
        let mut mask = 0;
        list.each_typed(self.prefix.as_chars(), |i| {
            if let Some(c) = list.typed(i).and_then(|t| t.as_chars().get(n).copied()) {
                let key = if kind == KeyboardKind::Kana {
                    kana::key_for(c)
                } else {
                    c
                };
                mask |= keyboard::key_bit(kind, key);
            }
            true
        });
        if kind == KeyboardKind::Kana
            && let Some(swapped) = self.small_toggled()
            && list.any_typed(swapped.as_chars())
        {
            mask |= keyboard::key_bit(kind, kana::SMALL_KEY);
        }
        mask
    }

    /// Swaps the last kana typed for its small form, or back. False when
    /// there is nothing to swap or nothing follows from it.
    fn toggle_small(&mut self) -> bool {
        let Some(swapped) = self.small_toggled() else {
            return false;
        };
        if !self.list().any_typed(swapped.as_chars()) {
            return false;
        }
        self.prefix = swapped;
        true
    }

    /// Types one key. Ignored when no candidate word continues with it.
    /// Nothing a key does commits a word: a word is taken by a tap on
    /// its candidate.
    pub fn type_char(&mut self, c: char) -> bool {
        let kind = self.keyboard();
        let c = if matches!(kind, KeyboardKind::Bip39 | KeyboardKind::Pinyin) {
            c.to_ascii_lowercase()
        } else {
            c
        };
        if reads(kind) {
            let bit = keyboard::key_bit(kind, c);
            return bit != 0 && self.enabled_keys() & bit != 0 && self.prefix.push(c);
        }
        if kind == KeyboardKind::Kana && c == kana::SMALL_KEY {
            return self.toggle_small();
        }
        // The two mark keys carry the standalone marks; a word carries
        // the combining ones.
        let typed = match (kind, c) {
            (KeyboardKind::Kana, kana::DAKUTEN_KEY) => kana::DAKUTEN,
            (KeyboardKind::Kana, kana::HANDAKUTEN_KEY) => kana::HANDAKUTEN,
            _ => c,
        };
        let bit = keyboard::key_bit(kind, c);
        let allowed = if bit == 0 {
            // A character with no key of its own: a small kana from a
            // physical keyboard. It stands if a candidate wants it.
            self.continues_with(typed)
        } else {
            self.enabled_keys() & bit != 0
        };
        allowed && self.prefix.push(typed)
    }

    /// Deletes the last key typed. On an empty field it steps back to
    /// the previous word, which returns to the field for editing:
    /// the screen shows one word and nothing
    /// else, so the way back to the one before it is the backspace.
    pub fn backspace(&mut self) {
        if self.prefix.pop().is_some() {
            return;
        }
        if self.committed == 0 {
            return;
        }
        let previous = self.words[self.committed() - 1];
        self.uncommit();
        if let Some(typed) = self
            .list()
            .typed(previous)
            .or_else(|| self.reading_typed(previous))
        {
            self.prefix = typed;
        }
    }

    /// Takes the `n`-th candidate.
    pub fn commit_candidate(&mut self, n: usize) -> bool {
        let Some(picked) = self.candidates().nth(n) else {
            return false;
        };
        self.commit(picked);
        true
    }

    fn commit(&mut self, index: u16) {
        self.words[self.committed()] = index;
        self.committed += 1;
        self.prefix.zeroize();
        // The final-word filter is BIP-39's checksum.
        if self.committed + 1 == self.count {
            self.allowed_last = Some(last_word_mask(&self.words[..self.committed()]));
        }
    }

    fn uncommit(&mut self) {
        if self.committed > 0 {
            self.committed -= 1;
            self.words[self.committed()] = 0;
        }
        if self.committed + 1 < self.count {
            self.allowed_last = None;
        }
    }
}

/// Bitmask of the valid final words after `first_words`.
fn last_word_mask(first_words: &[u16]) -> [u64; 32] {
    let mut mask = [0u64; 32];
    if let Ok(candidates) = bip39::last_word_candidates(first_words) {
        for i in candidates {
            mask[usize::from(i >> 6)] |= 1 << (i & 63);
        }
    }
    mask
}

impl Faraday {
    /// Handles seed-word entry's actions (`Action::Entry*`).
    pub(crate) fn entry_act(&mut self, action: Action) {
        match action {
            Action::Entry(wanted) => {
                if self.may_load_keys() {
                    let back = matches!(
                        self.screen,
                        Screen::Restore | Screen::Family | Screen::Spend
                    )
                    .then_some(self.screen);
                    self.entry = EntryState::default();
                    self.entry.wanted = wanted;
                    self.entry.back = back;
                    // A small panel has no keyboard of its own: every
                    // list's words go on the word keyboard.
                    if self.compact {
                        self.entry.keys = Some(forms::word_typer(self.entry.language()));
                    }
                    self.screen = Screen::Entry;
                }
            }
            Action::EntryWordCount(n) => {
                if EntryState::COUNTS.contains(&n) {
                    self.entry.word_count = n;
                }
            }
            Action::EntryClear => {
                self.entry.word_count = 0;
                self.entry.typed.clear();
                if self.entry.keys.is_some() {
                    self.entry.keys = Some(forms::word_typer(self.entry.language()));
                }
                self.entry.label.clear();
                self.entry.error = None;
            }
            Action::EntryAdd => self.entry_add(),
            Action::EntryLanguages => self.entry.languages = !self.entry.languages,
            Action::EntryLanguage(i) => {
                if let Some(&lang) = forms::LANGUAGES.get(usize::from(i)) {
                    self.entry.language_at = i;
                    self.entry.error = None;
                    self.entry.shift = false;
                    // A list with a keyboard of its own starts its words
                    // over; one typed in Latin letters keeps them.
                    self.entry.keys =
                        (forms::on_screen(lang) || self.compact).then(|| forms::word_typer(lang));
                    if self.compact {
                        self.entry.languages = false;
                    }
                }
            }
            Action::EntryKey(c) => {
                let w = self.words_use();
                let (lang, in_check) = (w.lang, w.in_check);
                if let Some(k) = w.keys.as_mut() {
                    k.type_char(c);
                    *w.shift = false;
                    *w.error = None;
                    if forms::word_whole(k, lang) {
                        forms::take_typed(k, lang);
                    }
                }
                if in_check {
                    self.copy_check_after();
                }
            }
            Action::EntryKeyBack => {
                let w = self.words_use();
                if let Some(k) = w.keys.as_mut() {
                    k.backspace();
                    *w.error = None;
                }
            }
            Action::EntryShift => {
                let w = self.words_use();
                *w.shift = !*w.shift;
            }
            Action::EntryCandidate(n) => {
                let w = self.words_use();
                let in_check = w.in_check;
                if let Some(k) = w.keys.as_mut() {
                    k.commit_candidate(usize::from(n));
                    *w.error = None;
                }
                if in_check {
                    self.copy_check_after();
                }
            }
            Action::EntryForm(i) => {
                if let Some(&f) = forms::Form::ALL.get(usize::from(i)) {
                    let (wanted, back) = (self.entry.wanted, self.entry.back);
                    self.entry = EntryState::default();
                    self.entry.wanted = wanted;
                    self.entry.back = back;
                    self.entry.form = f;
                    // Chosen from the chips, or opened by a Tools tile
                    // for this form: the chips stay in view.
                    self.entry.other_forms = true;
                }
            }
            Action::EntryOtherForms => self.entry.other_forms = true,
            Action::EntryPart => self.form_add_part(),
            Action::EntryRecover => self.form_recover(),
            Action::EntryPassphrase => self.entry.on_passphrase = !self.entry.on_passphrase,
            Action::EntryWallet(w) => {
                if self.may_load_keys() && w < self.session.wallets.len() {
                    if self.scan.take().is_some() {
                        self.commands.push_back(Command::CameraOff);
                    }
                    self.sheet = None;
                    self.act(Action::Entry(None));
                    self.entry.wallet = Some(w);
                }
            }
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_word(w: &mut WordEntry, word: &str) {
        for c in word.chars() {
            assert!(w.type_char(c), "{word}: {c}");
        }
        // §16.118: nothing the letters do takes the word. A word typed
        // out in full is the first candidate, and a tap on it accepts.
        assert!(w.commit_candidate(0), "{word}");
        assert!(w.prefix().is_empty(), "{word} was not taken");
    }

    fn keys(word: &str) -> Vec<char> {
        word.chars().collect()
    }

    /// The kana keyboard: the voicing key is live after a kana that can
    /// be voiced and dead after one that cannot, and 小 is live where a
    /// small kana follows.
    #[test]
    fn the_kana_keys_follow_what_the_words_allow() {
        let voiced = keyboard::key_bit(KeyboardKind::Kana, kana::DAKUTEN_KEY);
        let small = keyboard::key_bit(KeyboardKind::Kana, kana::SMALL_KEY);

        let mut w = WordEntry::new(Language::Japanese, 12);
        assert!(w.type_char('\u{304b}'), "か");
        assert!(w.enabled_keys() & voiced != 0, "か can take a voicing mark");

        let mut w = WordEntry::new(Language::Japanese, 12);
        assert!(w.type_char('\u{3042}'), "あ");
        assert!(w.type_char('\u{3093}'), "ん");
        assert_eq!(w.enabled_keys() & voiced, 0, "ん cannot be voiced");
        assert!(w.type_char('\u{304b}'), "か");
        assert!(w.enabled_keys() & voiced != 0, "あんがい");

        // きゃ: the や key types the full-size kana and 小 makes it small.
        let mut w = WordEntry::new(Language::Japanese, 12);
        assert!(w.type_char('\u{304d}'), "き");
        assert!(w.type_char('\u{3084}'), "や");
        assert!(w.enabled_keys() & small != 0, "小 makes it きゃ");
        assert!(w.type_char(kana::SMALL_KEY));
        assert!(w.candidates().count() > 0, "きゃ starts words");
        assert_eq!(w.prefix(), ['\u{304d}', '\u{3083}']);
        // And back again.
        assert!(w.type_char(kana::SMALL_KEY));
        assert_eq!(w.prefix(), ['\u{304d}', '\u{3084}']);
    }

    /// The jamo keyboard: a consonant that ends one syllable and a vowel
    /// that starts the next are both live after `ㄱㅏㄱ`, because the
    /// keys say nothing about which syllable they belong to.
    #[test]
    fn the_jamo_keys_leave_both_readings_of_a_final_open() {
        let mut w = WordEntry::new(Language::Korean, 12);
        for key in ['\u{3131}', '\u{314F}', '\u{3131}'] {
            assert!(w.type_char(key), "ㄱㅏㄱ");
        }
        let enabled = w.enabled_keys();
        // 가격 continues with a vowel, 각자 with a consonant.
        assert!(
            enabled & keyboard::key_bit(KeyboardKind::Jamo, '\u{3155}') != 0,
            "ㅕ, which makes 가격"
        );
        assert!(
            enabled & keyboard::key_bit(KeyboardKind::Jamo, '\u{3148}') != 0,
            "ㅈ, which makes 각자"
        );
        assert_eq!(
            enabled & keyboard::key_bit(KeyboardKind::Jamo, '\u{3142}'),
            0,
            "ㅂ starts no word from here"
        );
    }

    #[test]
    fn typing_narrows_and_a_tap_takes_the_candidate() {
        let mut w = WordEntry::new(Language::English, 12);
        assert_eq!(w.candidates().count(), 0, "nothing until a letter");
        assert!(w.type_char('a'));
        assert!(w.candidates().count() > 0);
        assert!(!w.type_char('q'), "no word starts with aq");
        assert!(w.type_char('c'));
        assert!(w.type_char('t'));
        assert_eq!(w.prefix(), keys("act"));
        // "act", "action", "actor", "actress" all start with it, so the
        // strip is how one of them is taken (§16.118).
        assert!(w.candidates().count() > 1);
        assert!(!w.accepting());
        assert!(w.commit_candidate(0));
        assert_eq!(w.committed(), 1);
        assert!(w.prefix().is_empty());
        // Backspace on an empty prefix un-commits, and the word comes
        // back to the field.
        w.backspace();
        assert_eq!(w.committed(), 0);
        assert_eq!(w.prefix(), keys("act"));
        // One candidate left: it waits in the strip, and a tap takes it.
        let mut w = WordEntry::new(Language::English, 12);
        for c in "abando".chars() {
            assert!(w.type_char(c));
        }
        assert_eq!(w.candidates().count(), 1);
        assert!(w.accepting());
        assert_eq!(w.committed(), 0);
        assert!(w.commit_candidate(0));
        assert_eq!(w.committed(), 1);
    }

    /// A word typed out in full is the one candidate left and stays in
    /// the strip until it is tapped.
    #[test]
    fn a_fully_typed_word_is_taken_by_a_tap_and_by_nothing_else() {
        let mut w = WordEntry::new(Language::English, 12);
        for c in "abandon".chars() {
            assert!(w.type_char(c));
        }
        assert_eq!(w.candidates().count(), 1);
        assert_eq!(w.committed(), 0, "the last letter took the word");
        assert_eq!(w.prefix(), keys("abandon"));
        assert!(w.commit_candidate(0));
        assert_eq!(w.committed(), 1);
        assert!(w.prefix().is_empty());
    }

    /// The checksum leaves 128 of the 2048 words for the last place, so
    /// three letters can leave one word there. It is a candidate like
    /// any other: it waits for its tap.
    #[test]
    fn the_last_word_the_checksum_leaves_waits_for_its_tap() {
        let mut w = WordEntry::new(Language::English, 12);
        for _ in 0..11 {
            type_word(&mut w, "abandon");
        }
        assert!(w.typing_last());
        for c in "abo".chars() {
            assert!(w.type_char(c));
        }
        assert_eq!(w.candidates().count(), 1, "one word can end this key");
        assert_eq!(w.committed(), 11);
        assert!(w.commit_candidate(0));
        let taken: Vec<u16> = w.committed_indices().collect();
        assert!(faraday_keys::words::from_indices(Language::English, &taken).is_ok());
    }

    #[test]
    fn a_typed_word_waits_for_its_tap_and_the_last_word_is_filtered() {
        let mut w = WordEntry::new(Language::English, 12);
        for _ in 0..11 {
            type_word(&mut w, "abandon");
        }
        assert_eq!(w.committed(), 11);
        assert!(w.typing_last());
        // Only 128 of 2048 words are valid last words.
        w.type_char('a');
        let with_a = w.candidates().count();
        assert!(with_a > 0 && with_a <= MAX_CANDIDATES);
        assert_eq!(
            w.enabled_keys() & keyboard::letter_bit('b'),
            keyboard::letter_bit('b')
        );
        w.backspace();
        type_word(&mut w, "about");
        let taken: Vec<u16> = w.committed_indices().collect();
        assert_eq!(taken[11], 3, "about");
        assert!(faraday_keys::words::from_indices(Language::English, &taken).is_ok());
    }

    #[test]
    fn the_final_word_prefers_valid_words_but_never_blocks() {
        let mut w = WordEntry::new(Language::English, 12);
        for _ in 0..11 {
            type_word(&mut w, "abandon");
        }
        w.type_char('a');
        // Every candidate shown is a valid final word.
        let mask = last_word_mask(&w.words[..11]);
        assert!(
            w.candidates()
                .all(|i| mask[usize::from(i >> 6)] >> (i & 63) & 1 == 1)
        );
        // Every letter of the list stays typable, so an invalid final word
        // can still be entered.
        w.backspace();
        for c in "abandon".chars() {
            assert!(w.type_char(c) || w.prefix().is_empty(), "{c}");
        }
    }
}
