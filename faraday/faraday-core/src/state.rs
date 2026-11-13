//! The state of screens that belong to no area folder: seed entry, signing a
//! message and the stick visit.

use std::collections::BTreeSet;

use crate::codex32_correct::{Offered, typable};
use crate::forms::{Form, completion, slip39_completion};
use crate::strings::{EN, fill};
use crate::wallet::fp_text;
use crate::{
    Action, Code, Column, Faraday, Screen, VisitBarAt, Writing, flow, forms, mstep, secret_text,
    visit_taken,
};
use faraday_bip::bip39::Language;
use faraday_bip::codex32::Codex32;
use faraday_bip::slip39;
use faraday_crypto::work::Compute;

/// Seed entry.
#[derive(Default)]
pub struct EntryState {
    /// The words as typed.
    pub typed: secret_text::SecretText,
    /// The slot fingerprint the key is meant for.
    pub wanted: Option<[u8; 4]>,
    /// The codex32 string the round after a lock takes, typed here in
    /// place of a key (`docs/CODEX32.md` §9.5): the codex32 form checks
    /// each string against it and keeps the one that is it.
    pub c32_round: Option<crate::plan::codex32::Wanted>,
    /// The loaded wallet any of whose missing keys the words may give.
    pub wallet: Option<usize>,
    /// The label for the key.
    pub label: String,
    /// The last refusal.
    pub error: Option<faraday_text::Text>,
    /// `error` is the passphrase field's refusal of the key just typed:
    /// it stays until the next key.
    pub refused: bool,
    /// Where to go once the key is added.
    pub back: Option<Screen>,
    /// The BIP-39 passphrase, when one is typed.
    pub passphrase: secret_text::SecretText,
    /// The passphrase typed again, where it is asked twice ([`EntryState::pair`]).
    pub again: secret_text::SecretText,
    /// Typing goes to the passphrase rather than the words.
    pub on_passphrase: bool,
    /// With `on_passphrase`: typing goes to the passphrase typed again.
    pub on_again: bool,
    /// Which form of key is being typed.
    pub form: forms::Form,
    /// The BIP-39 list the words are typed from, by its place in
    /// `forms::LANGUAGES`.
    pub language_at: u8,
    /// The word entry, for a list typed on an on-screen
    /// keyboard ([`forms::on_screen`]); `None` for a Latin list, whose
    /// words are typed into `typed`.
    pub keys: Option<Box<crate::word_entry::WordEntry>>,
    /// The on-screen keyboard's shift is on.
    pub shift: bool,
    /// SLIP-39 shares, codex32 strings or Seed XOR parts collected.
    pub parts: forms::Parts,
    /// The lists other than English are shown to choose from.
    pub languages: bool,
    /// The chips for SLIP-39, codex32 and Seed XOR are shown: pressed
    /// directly, or set already by a Tools tile for one of those forms
    /// (`docs/history/SIMPLIFY.md` §2.4).
    pub other_forms: bool,
    /// How many words were chosen, 0 until one is: 12 boxes.
    pub word_count: u8,
    /// Use a sample's chip and cursor for this visit (`docs/MODES.md` §6.1).
    pub(crate) sample: crate::samples::put::Use,
    /// The codex32 form's error correction (`docs/CODEX32.md` §8.4):
    /// fresh, the tick off, whenever the form opens.
    pub correction: crate::codex32_correct::Correction,
    /// The codex32 form's own state beside the strings in: a string at an
    /// index already in, "Already in", Check a string (§8.3, §8.5).
    pub codex32: forms::Codex32Form,
    /// The status line's key: what the words typed make with the
    /// passphrase, the fingerprint or why there is none, public alone.
    /// Worked out by the first frame that asks for it and kept until the
    /// next event that can change the words, list or passphrase clears it
    /// ([`EntryState::probed`], [`EntryState::forget_probe`]).
    probe: std::cell::RefCell<Option<Result<faraday_bip::keys::Fingerprint, String>>>,
}

impl EntryState {
    /// Whether the BIP-39 passphrase is typed twice: nothing checks it, and
    /// a typo makes another key. Not where it is checked against a wallet
    /// (Restore, Spend from a backup, a wallet's missing key).
    pub fn pair(&self) -> bool {
        self.wallet.is_none()
            && !matches!(
                self.back,
                Some(crate::Screen::Restore | crate::Screen::SpendBackup)
            )
    }

    /// The passphrase is asked twice and the two differ: the key waits.
    pub fn differs(&self) -> bool {
        self.pair()
            && crate::screens::pass_pair::differ(self.passphrase.as_str(), self.again.as_str())
    }

    /// The word counts a BIP-39 key can have.
    pub const COUNTS: [u8; 5] = [12, 15, 18, 21, 24];

    /// How many word boxes the choice asks for: 12 until another count
    /// is chosen.
    pub(crate) fn words_chosen(&self) -> usize {
        if Self::COUNTS.contains(&self.word_count) {
            usize::from(self.word_count)
        } else {
            12
        }
    }

    /// How many word boxes to show with `filled` words in: the count
    /// chosen, or all 24 once the words run past it.
    pub(crate) fn word_boxes(&self, filled: usize) -> usize {
        let chosen = self.words_chosen();
        if filled > chosen { 24 } else { chosen }
    }

    /// The BIP-39 list the words are typed from.
    pub fn language(&self) -> faraday_bip::bip39::Language {
        forms::LANGUAGES[usize::from(self.language_at) % forms::LANGUAGES.len()]
    }

    /// Costs a PBKDF2 and an elliptic-curve multiplication when nothing is
    /// kept, the first frame after an event; then it is a lookup until
    /// [`EntryState::forget_probe`]. The key words `m` make with the
    /// passphrase typed, or why they make none, for the status line. `m`
    /// must be the words typed now. Decisions (Add, Continue, Lock in) load
    /// the words themselves and never read this.
    pub(crate) fn probed(
        &self,
        c: &Compute,
        m: &faraday_bip::bip39::Mnemonic,
    ) -> Result<faraday_bip::keys::Fingerprint, String> {
        if let Some(r) = self.probe.borrow().as_ref() {
            return r.clone();
        }
        // Kept until an event forgets it: worked out once, by the frame
        // that asks first.
        let r =
            faraday_keys::identity::fingerprint_of_mnemonic(c, m, &self.passphrase).map_err(|e| {
                String::from(&*crate::wallet::Refusal::Words(faraday_text::Text::of(&e)).text())
            });
        *self.probe.borrow_mut() = Some(r.clone());
        r
    }

    /// Forgets the status line's key, for an event that may have changed
    /// the words, list or passphrase.
    pub(crate) fn forget_probe(&self) {
        self.probe.borrow_mut().take();
    }

    /// The status line's key as kept, if it is: a lookup.
    pub(crate) fn probe_kept(&self) -> Option<Result<faraday_bip::keys::Fingerprint, String>> {
        self.probe.borrow().clone()
    }
}

/// What a key on Add a key leaves to the caller.
pub(crate) enum Keyed {
    /// The area took it.
    Taken,
    /// Add the words typed as a key.
    Add,
    /// Put the parts together and add the key.
    Recover,
    /// Take the text typed as one more part.
    AddPart,
    /// Leave Add a key.
    Leave,
}

/// Seed entry's area: its state, which only its own methods replace and
/// which other areas reach through [`EntryArea::state`] to read; they
/// change it only through the area's own methods.
#[derive(Default)]
pub(crate) struct EntryArea {
    state: EntryState,
}

impl EntryArea {
    /// The entry's state.
    pub(crate) fn state(&self) -> &EntryState {
        &self.state
    }

    /// Forgets everything typed and chosen.
    pub(crate) fn reset(&mut self) {
        self.state = EntryState::default();
    }

    /// Takes the state, leaving a fresh one.
    pub(crate) fn take(&mut self) -> EntryState {
        std::mem::take(&mut self.state)
    }

    /// Puts a state back in place of the current one.
    pub(crate) fn put(&mut self, state: EntryState) {
        self.state = state;
    }

    /// Opens a fresh entry: what is wanted and the screen Back goes to
    /// kept, and, on a small panel, every list's words on the word
    /// keyboard.
    pub(crate) fn begin(&mut self, wanted: Option<[u8; 4]>, back: Option<Screen>, compact: bool) {
        self.reset();
        self.state.wanted = wanted;
        self.state.back = back;
        if compact {
            self.restart_keys();
        }
    }

    /// Forgets what the form holds but keeps what is wanted and where
    /// Back goes, and shows `form` with the chips of the other forms.
    pub(crate) fn choose_form(&mut self, form: Form) {
        let (wanted, back) = (self.state.wanted, self.state.back);
        self.reset();
        self.state.wanted = wanted;
        self.state.back = back;
        self.state.form = form;
        self.state.other_forms = true;
    }

    /// Shows the chips of the other forms.
    pub(crate) fn show_other_forms(&mut self) {
        self.state.other_forms = true;
    }

    /// Opens for a round of the backup (`EntryState::c32_round`): Back
    /// goes to the backup flow, the string wanted for the round.
    pub(crate) fn open_round(&mut self, want: crate::plan::codex32::Wanted) {
        self.state.other_forms = false;
        self.state.back = Some(Screen::BackupFlow);
        self.state.c32_round = Some(want);
    }

    /// Refuses a string typed for the round: the parts dropped, `line`
    /// shown.
    pub(crate) fn refuse_round(&mut self, line: String) {
        self.state.parts.codex32.clear();
        self.state.error = Some(faraday_text::Text::of(&line));
    }

    /// Sets the screen Back goes to.
    pub(crate) fn set_back(&mut self, back: Option<Screen>) {
        self.state.back = back;
    }

    /// Sets the wallet the key is added for.
    pub(crate) fn set_wallet(&mut self, wallet: usize) {
        self.state.wallet = Some(wallet);
    }

    /// Shows `error` as the last refusal.
    pub(crate) fn set_error(&mut self, error: impl AsRef<str>) {
        self.state.error = Some(faraday_text::Text::of(error.as_ref()));
    }

    /// Clears the last refusal, whatever it was.
    pub(crate) fn forget_error(&mut self) {
        self.state.error = None;
    }

    /// Takes the last refusal away.
    pub(crate) fn take_error(&mut self) -> Option<String> {
        self.state.error.take().map(|t| String::from(&*t))
    }

    /// Puts `text` in as typed.
    pub(crate) fn set_typed(&mut self, text: &str) {
        self.state.typed.set(text);
    }

    /// Wipes what was typed.
    pub(crate) fn wipe_typed(&mut self) {
        zeroize::Zeroize::zeroize(&mut self.state.typed);
    }

    /// Changes the text typed through `f` and clears the last refusal.
    pub(crate) fn edit_typed(&mut self, f: impl FnOnce(&mut secret_text::SecretText)) {
        f(&mut self.state.typed);
        self.state.error = None;
    }

    /// Puts `pass` in as the passphrase.
    pub(crate) fn set_passphrase(&mut self, pass: &str) {
        self.state.passphrase.set(pass);
    }

    /// Puts `pass` in as the passphrase's repeat.
    pub(crate) fn set_again(&mut self, pass: &str) {
        self.state.again.set(pass);
    }

    /// Takes the on-screen keyboard's words away.
    pub(crate) fn take_keys(&mut self) -> Option<Box<crate::word_entry::WordEntry>> {
        self.state.keys.take()
    }

    /// Starts the on-screen keyboard's words over on the list chosen.
    pub(crate) fn restart_keys(&mut self) {
        self.state.keys = Some(forms::word_typer(self.state.language()));
    }

    /// `f` on the on-screen keyboard's words, if it has any.
    pub(crate) fn with_keys<T>(
        &mut self,
        f: impl FnOnce(&mut crate::word_entry::WordEntry) -> Option<T>,
    ) -> Option<T> {
        f(self.state.keys.as_deref_mut()?)
    }

    /// The words typed on the computer's keyboard or the word keyboard,
    /// for the shared helper that types a key into them.
    pub(crate) fn typing_words(&mut self, numbers: bool) -> forms::WordsMut<'_> {
        let e = &mut self.state;
        let lang = e.language();
        forms::WordsMut {
            typed: &mut e.typed,
            keys: &mut e.keys,
            error: &mut e.error,
            lang,
            numbers,
        }
    }

    /// The state the on-screen word keys press into.
    pub(crate) fn typing_use(&mut self) -> crate::copy_check::WordsUse<'_> {
        let e = &mut self.state;
        let lang = e.language();
        crate::copy_check::WordsUse {
            keys: &mut e.keys,
            shift: &mut e.shift,
            error: &mut e.error,
            lang,
            in_check: false,
        }
    }

    /// Sets how many words the form is for.
    pub(crate) fn set_word_count(&mut self, n: u8) {
        self.state.word_count = n;
    }

    /// Clears the words form: the words, the count, the label and the
    /// refusal, with the chip's use forgotten.
    pub(crate) fn clear_words(&mut self) {
        self.state.sample.emptied();
        self.state.word_count = 0;
        self.state.typed.clear();
        if self.state.keys.is_some() {
            self.restart_keys();
        }
        self.state.label.clear();
        self.state.error = None;
    }

    /// Opens or closes the list of languages.
    pub(crate) fn toggle_languages(&mut self) {
        self.state.languages = !self.state.languages;
    }

    /// Chooses the word list `at`, `lang`: a list with a keyboard of its
    /// own, or any on a small panel, starts its words over; one typed in
    /// Latin letters keeps them.
    pub(crate) fn choose_language(&mut self, at: u8, lang: Language, compact: bool) {
        self.state.language_at = at;
        self.state.error = None;
        self.state.shift = false;
        self.state.keys = (forms::on_screen(lang) || compact).then(|| forms::word_typer(lang));
        if compact {
            self.state.languages = false;
        }
    }

    /// Chooses the word list `at` without touching the words.
    pub(crate) fn set_language_at(&mut self, at: u8) {
        self.state.language_at = at;
    }

    /// A field of the form pressed: neither passphrase field has the
    /// focus and the chip is gone.
    pub(crate) fn focus_words(&mut self) {
        self.state.on_passphrase = false;
        self.state.on_again = false;
        self.state.sample.dismiss();
    }

    /// Leaves the passphrase field.
    pub(crate) fn leave_passphrase(&mut self) {
        self.state.on_passphrase = false;
    }

    /// The passphrase's chip pressed: the field opens, or closes where it
    /// was open and not on its repeat.
    pub(crate) fn toggle_passphrase(&mut self) {
        let e = &mut self.state;
        e.on_passphrase = !(e.on_passphrase && !e.on_again);
        e.on_again = false;
    }

    /// The passphrase's repeat has the focus.
    pub(crate) fn focus_again(&mut self) {
        self.state.on_passphrase = true;
        self.state.on_again = true;
    }

    /// The chip inside the field is gone for the visit.
    pub(crate) fn dismiss_sample(&mut self) {
        self.state.sample.dismiss();
    }

    /// Notes that the chip gave words: its next pick at `at`, and the
    /// count.
    pub(crate) fn sampled_words(&mut self, at: usize, next: u8, count: u8) {
        self.state.sample.took_words(at, next);
        self.state.word_count = count;
    }

    /// Notes that the chip gave the part of set `begun`, its next pick
    /// `next`.
    pub(crate) fn sampled_part(&mut self, next: u8, begun: Option<u8>) {
        self.state.sample.took_part(next, begun);
    }

    /// Puts the chip's `text` in as typed words, on the keyboard's words
    /// where it has them.
    pub(crate) fn put_sample_words(&mut self, text: &str) {
        if self.state.keys.is_some() {
            let mut w = forms::word_typer(Language::English);
            forms::type_words(&mut w, Language::English, text);
            self.state.keys = Some(w);
        } else {
            self.state.typed.set(text);
            self.state.typed.push(' ');
        }
    }

    /// The chip's text is in: the refusal is gone and the chip is used.
    pub(crate) fn sample_filled(&mut self) {
        self.state.error = None;
        self.state.sample.fill();
    }

    /// The codex32 form's strings stay; it makes the key again if
    /// pressed.
    pub(crate) fn keep_strings(&mut self) {
        self.state.codex32.making = None;
    }

    /// Notes how many of the open vaults' codex32 strings count and how
    /// many the checked string has. Returns whether the strings and the
    /// vaults' now make `k` of the set where they did not (so the key
    /// may load).
    pub(crate) fn set_vault_counts(&mut self, n: usize, checked: usize) -> bool {
        let e = &mut self.state;
        let before = e.parts.codex32_vaults;
        e.parts.codex32_vaults = n;
        e.codex32.check_in_vaults = checked;
        let k = e
            .parts
            .codex32
            .first()
            .map_or(0, |s| usize::from(s.threshold()));
        let typed = e.parts.codex32.len();
        n != before && e.codex32.making.is_none() && k >= 2 && typed < k && typed + n >= k
    }

    /// Toggles making `kind` (the rest or a share) on the codex32 form.
    /// `None` where the strings offer no more; else whether they make it
    /// now.
    pub(crate) fn toggle_making(&mut self, kind: crate::codex32::Given) -> Option<bool> {
        let e = &mut self.state;
        if !forms::offers_more(&e.parts) {
            return None;
        }
        e.codex32.making = (e.codex32.making != Some(kind)).then_some(kind);
        Some(e.codex32.making.is_some() && forms::makes_more(&e.parts))
    }

    /// The codex32 correction's caret put in box `i`.
    pub(crate) fn correction_caret_to_box(&mut self, i: usize) {
        self.state.correction.caret_to_box(i);
    }

    /// The press on the codex32 correction.
    pub(crate) fn correction_press(
        &mut self,
        p: crate::codex32_correct::Press,
    ) -> crate::codex32_correct::Outcome {
        self.state.correction.press(p)
    }

    /// The worksheet's pan, for the motion that moves it.
    pub(crate) fn worksheet_pan(&self) -> crate::codex32::SheetPan {
        self.state.codex32.pan
    }

    /// The worksheet's pan put as `to`, once the motion has moved it.
    pub(crate) fn set_worksheet_pan(&mut self, to: crate::codex32::SheetPan) {
        self.state.codex32.pan = to;
    }

    /// Puts the words pasted in, `idx` their indices on list `lang`: on
    /// the keyboard's words where it has them, else as typed, with the
    /// count set to theirs.
    pub(crate) fn put_pasted_words(&mut self, lang: Language, idx: &[u16]) {
        let e = &mut self.state;
        if e.keys.is_some() {
            let mut w = forms::word_typer(lang);
            for &i in idx.iter() {
                w.put(i);
            }
            e.keys = Some(w);
        } else {
            let mut typed = zeroize::Zeroizing::new(String::new());
            for &i in idx.iter() {
                if let Some(t) = lang.typed(i) {
                    typed.extend(t.as_chars());
                }
                typed.push(' ');
            }
            e.typed.set(&typed);
        }
        e.word_count = idx.len() as u8;
        e.on_passphrase = false;
        e.error = None;
        e.sample.dismiss();
        e.forget_probe();
    }

    /// The word just typed into `typed`, the way Tab completes it: the
    /// one word of the list this form's words come from that its letters
    /// can still become, if only one. `Codex32` has none, since it is
    /// typed as a string, not words.
    pub(crate) fn tab_word(&self) -> Option<faraday_text::Text> {
        let e = &self.state;
        match e.form {
            Form::Words => completion(e.language(), &e.typed),
            Form::Xor => completion(faraday_bip::bip39::Language::English, &e.typed),
            Form::Slip39 => slip39_completion(&e.typed),
            Form::Codex32 => None,
        }
    }

    /// Completes the word just typed into `typed`, the way Tab does for
    /// every form that types BIP-39 or SLIP-39 words one at a time.
    pub(crate) fn complete_word(&mut self) {
        if let Some(word) = self.tab_word() {
            let cut = self.state.typed.rfind(' ').map(|i| i + 1).unwrap_or(0);
            self.state.typed.truncate(cut);
            self.state.typed.push_str(&word);
            self.state.typed.push(' ');
        }
    }

    /// Whether the word just typed into `typed` is already whole and the
    /// only one it can be: the point at which it moves on by itself, the
    /// same move Tab makes for you.
    pub(crate) fn word_whole(&self) -> bool {
        let prefix = self.state.typed.rsplit(' ').next().unwrap_or("");
        !prefix.is_empty() && self.tab_word().as_deref() == Some(prefix)
    }

    /// The words typed on Add a key's words form, the word being typed
    /// on a list's own keyboard taken first, as a key's words: whole and
    /// with a checksum that holds, or what is wrong with them.
    pub(crate) fn mnemonic(&mut self) -> Result<faraday_bip::bip39::Mnemonic, String> {
        let e = &mut self.state;
        let lang = e.language();
        if let Some(w) = e.keys.as_mut() {
            forms::take_typed(w, lang);
        }
        match e.keys.as_ref() {
            Some(w) => {
                let mut idx: Vec<u16> = w.committed_indices().collect();
                let m = faraday_keys::words::from_indices(lang, &idx)
                    .map_err(|e| String::from(&*faraday_strings::bip::bip39_error(&e)));
                zeroize::Zeroize::zeroize(&mut idx);
                m
            }
            None => forms::typed_mnemonic(lang, &e.typed).map_err(|t| String::from(&*t)),
        }
    }

    /// A key while the passphrase field has the keyboard. Enter says
    /// what to do with the words or parts typed.
    pub(crate) fn passphrase_key(&mut self, key: faraday_shell_api::Key) -> Keyed {
        use faraday_shell_api::Key as K;
        let e = &mut self.state;
        let mut to = Keyed::Taken;
        let (pair, second) = (e.pair(), e.on_again);
        match key {
            // BIP-39 passphrases here are printable ASCII (BIP-39's seed derivation).
            K::Char(c) if (' '..='~').contains(&c) && second => e.again.push(c),
            K::Char(c) if (' '..='~').contains(&c) => e.passphrase.push(c),
            // Anything else is refused, and the field says so
            // (`docs/STANDARDS.md`).
            K::Char(c) if !c.is_control() => {
                e.error = Some(faraday_text::Text::of(&ascii_refused()));
                e.refused = true;
            }
            K::Backspace if second => {
                e.again.pop();
            }
            K::Backspace => {
                e.passphrase.pop();
            }
            // The first field's Enter and Tab move to the second.
            K::Enter | K::Tab if pair && !second => e.on_again = true,
            K::Enter if e.form == Form::Words => to = Keyed::Add,
            K::Enter => to = Keyed::Recover,
            K::Escape | K::Tab => {
                e.on_passphrase = false;
                e.on_again = false;
            }
            _ => {}
        }
        to
    }

    /// Forgets the last refusal, but the passphrase field's refusal of
    /// the key just typed, which stays until the next key.
    pub(crate) fn clear_error(&mut self) {
        if !std::mem::take(&mut self.state.refused) {
            self.state.error = None;
        }
    }

    /// Costs what a correction's outcome costs (a codex32 string checked,
    /// its worksheet), nothing else. A key typed into a form other than
    /// words: SLIP-39 shares, a codex32 string or Seed XOR parts. Letters complete a word that is whole; Tab
    /// completes the one it can only be.
    pub(crate) fn part_key(&mut self, c: &Compute, key: faraday_shell_api::Key) -> Keyed {
        use faraday_shell_api::Key as K;
        let codex32 = self.state.form == Form::Codex32;
        // The correction's screen, while it stands in place of the field,
        // takes every key (`docs/CODEX32.md` §8.4).
        if codex32 && self.state.correction.is_open() {
            let out = self.state.correction.key(key);
            return match self.codex32_outcome(c, out) {
                true => Keyed::Recover,
                false => Keyed::Taken,
            };
        }
        // A string at an index already in waits on its answer; Escape
        // keeps the one held.
        if codex32 && self.state.codex32.clash.is_some() {
            if key == K::Escape {
                self.codex32_press(forms::C32Press::Keep);
            }
            return Keyed::Taken;
        }
        let unreadable = |c: char| codex32 && typable(&self.state.correction, c);
        match key {
            K::Char(c)
                if c.is_ascii_alphabetic() || (codex32 && c.is_ascii_digit()) || unreadable(c) =>
            {
                self.state.typed.push(c.to_ascii_lowercase());
                self.state.error = None;
                self.state.codex32.already = false;
                self.state.codex32.forget_check();
                if !codex32 && self.word_whole() {
                    self.complete_word();
                }
            }
            K::Char(' ') if !codex32 => {
                if !self.state.typed.is_empty() && !self.state.typed.ends_with(' ') {
                    self.state.typed.push(' ');
                }
            }
            K::Tab if !codex32 => self.complete_word(),
            K::Backspace => {
                self.state.typed.pop();
                self.state.error = None;
                self.state.codex32.forget_check();
            }
            K::Enter if self.state.codex32.checking && self.state.typed.trim().is_empty() => {}
            K::Enter if self.state.typed.trim().is_empty() => return Keyed::Recover,
            K::Enter => return Keyed::AddPart,
            K::Escape => return Keyed::Leave,
            _ => {}
        }
        Keyed::Taken
    }

    /// Costs what reading a Seed XOR part's words costs, a checksum and a
    /// hash, and nothing for the other forms. Takes the typed text as one more part of the form
    /// being read; a refusal is left in `error`. Returns whether a codex32
    /// string just completed the key (it is the key already).
    pub(crate) fn add_part(&mut self, c: &Compute) -> bool {
        if self.state.form == Form::Codex32 {
            return self.codex32_part(c);
        }
        let e = &mut self.state;
        let form = e.form;
        let text = zeroize::Zeroizing::new(e.typed.trim().to_lowercase());
        let result: Result<(), String> = match form {
            Form::Slip39 => add_slip39(&mut e.parts, &text),
            Form::Xor => add_xor(c, &mut e.parts, &text),
            Form::Codex32 | Form::Words => Ok(()),
        };
        match result {
            Ok(()) => {
                e.sample.emptied();
                e.typed.clear();
                e.error = None;
                false
            }
            Err(msg) => {
                e.error = Some(faraday_text::Text::of(&msg));
                false
            }
        }
    }

    /// Costs a decode of one string where the tick is on and the string
    /// does not parse (`Correction::offer`); a parse otherwise, and a
    /// checksum worksheet while checking a string. The
    /// codex32 form's Enter: the string typed is checked (Check a
    /// string) or added; with the tick on, one that does not parse is
    /// offered for correction (`docs/CODEX32.md` §8.1, §8.4, §8.5).
    /// Returns whether the strings in now make the key.
    fn codex32_part(&mut self, c: &Compute) -> bool {
        let e = &mut self.state;
        // Read as typed: BIP 93 refuses mixed case.
        let given = zeroize::Zeroizing::new(e.typed.trim().to_string());
        e.codex32.already = false;
        e.codex32.forget_check();
        let parses = Codex32::parse(&given).is_ok();
        if !parses {
            let held = e.parts.codex32.first();
            if e.correction.offer(c, &given, held) == Offered::Shown {
                e.error = None;
                return false;
            }
        }
        if e.codex32.checking {
            self.codex32_check(c, &given, None);
            return false;
        }
        self.codex32_add(&given, None)
    }

    /// Costs a checksum worksheet of one string. Check a string (§8.3):
    /// what `given` is, whether its checksum holds, and its worksheet in
    /// the verification form; nothing is kept or loaded.
    fn codex32_check(
        &mut self,
        c: &Compute,
        given: &str,
        mark: Option<crate::codex32_correct::Mark>,
    ) {
        let e = &mut self.state;
        e.error = None;
        e.codex32.sheet = faraday_keys::StringCheck::of(c, given);
        e.codex32.sheet_open = false;
        e.codex32.checked = Some(match Codex32::parse(given) {
            Ok(s) => forms::Checked {
                holds: true,
                what: forms::codex32_what(&s),
                of_the_set: forms::of_the_set(&s, &e.parts.codex32),
                mark,
            },
            Err(faraday_bip::codex32::Error::BadChecksum) => forms::Checked {
                holds: false,
                what: forms::codex32_refusal(given, &faraday_bip::codex32::Error::BadChecksum),
                of_the_set: None,
                mark: None,
            },
            Err(err) => {
                e.error = Some(forms::codex32_refusal(given, &err));
                e.codex32.sheet = None;
                return;
            }
        });
    }

    /// One codex32 string more, `mark` where it went in by a correction:
    /// the same string again is "Already in"; another at an index already
    /// in waits on Keep or Replace (§8.5). Returns whether the strings in
    /// now make the key.
    fn codex32_add(&mut self, given: &str, mark: Option<crate::codex32_correct::Mark>) -> bool {
        let e = &mut self.state;
        match add_codex32(&mut e.parts, given) {
            Ok(Added::New) => {
                if let (Some(m), Some(s)) = (mark, e.parts.codex32.last()) {
                    e.correction.note(s, m);
                }
                e.sample.emptied();
                e.typed.clear();
                e.error = None;
                e.parts.ready(Form::Codex32)
            }
            Ok(Added::Already) => {
                e.typed.clear();
                e.error = None;
                e.codex32.already = true;
                false
            }
            Ok(Added::Clash(string, at)) => {
                e.typed.clear();
                e.error = None;
                e.codex32.clash = Some(forms::Clash::of(string, &e.parts.codex32[at], mark));
                false
            }
            Err(msg) => {
                e.error = Some(faraday_text::Text::of(&msg));
                false
            }
        }
    }

    /// Costs a checksum worksheet of one string where it is checked. A
    /// correction's outcome: a string taken goes in (or is checked) as
    /// one typed, with its mark. Returns whether the strings in now make
    /// the key.
    pub(crate) fn codex32_outcome(
        &mut self,
        c: &Compute,
        out: crate::codex32_correct::Outcome,
    ) -> bool {
        let crate::codex32_correct::Outcome::Take(t) = out else {
            return false;
        };
        if self.state.codex32.checking {
            self.state.typed.set(t.text());
            self.codex32_check(c, t.text(), Some(t.mark()));
            return false;
        }
        self.codex32_add(t.text(), Some(t.mark()))
    }

    /// A press on the codex32 form's own rows. Returns whether the
    /// strings in now make the key.
    pub(crate) fn codex32_press(&mut self, p: forms::C32Press) -> bool {
        let e = &mut self.state;
        match p {
            forms::C32Press::Keep => e.codex32.clash = None,
            forms::C32Press::Replace => {
                if let Some(clash) = e.codex32.clash.take()
                    && let Some(at) = e
                        .parts
                        .codex32
                        .iter()
                        .position(|d| d.share_index() == clash.string.share_index())
                {
                    if let Some(m) = clash.mark {
                        e.correction.note(&clash.string, m);
                    }
                    e.parts.codex32[at] = clash.string;
                    return e.parts.ready(Form::Codex32);
                }
            }
            forms::C32Press::Check => {
                // The string checked is dropped on leaving, and so is one
                // typed to add on going to check.
                e.codex32.checking = !e.codex32.checking;
                e.codex32.forget_check();
                e.codex32.already = false;
                e.typed.clear();
                e.error = None;
                if e.correction.is_open() {
                    e.correction
                        .press(crate::codex32_correct::Press::CheckBoxes);
                }
            }
            forms::C32Press::Sheet => {
                e.codex32.sheet_open = !e.codex32.sheet_open && e.codex32.sheet.is_some();
                e.codex32.pan = crate::codex32::SheetPan::default();
            }
            forms::C32Press::CheckIt | forms::C32Press::Rest | forms::C32Press::AddShare => {}
        }
        false
    }
}

/// What one codex32 string more did to the strings in.
enum Added {
    /// It went in.
    New,
    /// The same string is in already.
    Already,
    /// Another string is in at its index, at this place among them.
    Clash(Codex32, usize),
}

/// What a passphrase field says of a letter it refuses: a passphrase
/// here is printable ASCII (`docs/STANDARDS.md`). It names no letter: the
/// field is masked.
pub(crate) fn ascii_refused() -> String {
    EN.passphrase_ascii_only.to_string()
}

/// One SLIP-39 share more, refused when it is not one or is a repeat.
fn add_slip39(parts: &mut forms::Parts, text: &str) -> Result<(), String> {
    let s = slip39::Share::parse(text).map_err(|e| {
        fill(
            EN.forms_not_slip39_share,
            &[&faraday_strings::bip::slip39_error(&e)],
        )
    })?;
    if parts.slip39.iter().any(|t| t.indices() == s.indices()) {
        return Err(EN.forms_share_duplicate.to_string());
    }
    // Never past the room made at the start: a list that grows leaves a
    // copy of every share behind.
    if parts.slip39.len() >= parts.slip39.capacity().min(forms::SLIP39_ROOM) {
        return Err(EN.forms_shares_full.to_string());
    }
    parts.slip39.push(s);
    Ok(())
}

/// One codex32 string more, refused when it is not one or is of another
/// seed. The same string again changes nothing; another at an index
/// already in is handed back to ask about (`docs/CODEX32.md` §8.5).
fn add_codex32(parts: &mut forms::Parts, given: &str) -> Result<Added, String> {
    let c = Codex32::parse(given).map_err(|e| String::from(&*forms::codex32_refusal(given, &e)))?;
    if let Some(first) = parts.codex32.first()
        && first.identifier() != c.identifier()
    {
        return Err(EN.forms_other_seed_string.to_string());
    }
    if let Some(at) = parts
        .codex32
        .iter()
        .position(|d| d.share_index() == c.share_index())
    {
        let same = parts.codex32[at].encode().as_str() == c.encode().as_str();
        return Ok(if same {
            Added::Already
        } else {
            Added::Clash(c, at)
        });
    }
    parts.codex32.push(c);
    Ok(Added::New)
}

/// Costs what reading the words costs, a checksum and a hash. One Seed
/// XOR part more, its words checked.
fn add_xor(c: &Compute, parts: &mut forms::Parts, text: &str) -> Result<(), String> {
    let m = crate::wallet::parse_words(c, Language::English, text).map_err(|e| {
        fill(
            EN.forms_not_bip_39_words,
            &[&faraday_strings::bip::bip39_error(&e)],
        )
    })?;
    let ent = m.entropy();
    parts
        .xor
        .push(ent.expose().as_bytes())
        .map_err(|_| EN.forms_every_part_has.to_string())
}

/// Signing a message: the cards' state.
#[derive(Default)]
pub struct MessageState {
    /// The open card.
    pub open: Option<u8>,
    /// Cards closed as done.
    pub done: [bool; 4],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// The wallet, by index into the session.
    pub wallet: Option<usize>,
    /// The receive address's index.
    pub index: u32,
    /// The message.
    pub text: String,
    /// Typing goes to the message.
    pub typing: bool,
    /// BIP-322 rather than BIP-137.
    pub bip322: bool,
    /// The signature, once made.
    pub signed: Option<faraday_psbt::message::Signed>,
    /// The name the signed message's file went out under.
    pub out_name: Option<String>,
    /// The last refusal.
    pub error: Option<faraday_text::Text>,
}

/// The message-signing area: the cards while they are open, and the
/// Inbox file of a signed message being checked.
#[derive(Default)]
pub(crate) struct MessageArea {
    state: Option<MessageState>,
    /// The Inbox file of the signed message being checked.
    checking: Option<usize>,
}

impl MessageArea {
    /// The cards' state, while the flow is open.
    pub(crate) fn state(&self) -> Option<&MessageState> {
        self.state.as_ref()
    }

    /// The Inbox file being checked, by index.
    pub(crate) fn checking(&self) -> Option<usize> {
        self.checking
    }

    /// Forgets the cards, leaving the screen to the caller.
    pub(crate) fn clear_state(&mut self) {
        self.state = None;
    }

    /// Forgets the cards and the file being checked.
    pub(crate) fn clear(&mut self) {
        self.state = None;
        self.checking = None;
    }

    /// How far the column is scrolled, units, while the flow is open.
    pub(crate) fn scroll_y(&self) -> Option<f32> {
        self.state.as_ref().map(|x| x.scroll.y)
    }

    /// The column scrolled to `y` units, while the flow is open.
    pub(crate) fn set_scroll_y(&mut self, y: f32) {
        if let Some(x) = self.state.as_mut() {
            x.scroll.y = y;
        }
    }

    /// Takes the scroll a frame drew the column with.
    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        if let Some(m) = self.state.as_mut() {
            m.scroll = to;
        }
    }

    /// The actions that change only the cards: whether `action` was one.
    pub(crate) fn edit(&mut self, action: &Action) -> bool {
        let Some(m) = self.state.as_mut() else {
            return matches!(
                action,
                Action::MStep(_)
                    | Action::MNext(_)
                    | Action::MWallet(_)
                    | Action::MIndex(_)
                    | Action::MType
                    | Action::MFormat(_)
            );
        };
        match *action {
            Action::MStep(n) => {
                m.open = if m.open == Some(n) { None } else { Some(n) };
                m.scroll.follow = true;
                m.typing = m.open == Some(mstep::TEXT);
            }
            Action::MNext(n) => {
                m.done[n as usize] = true;
                m.open = (0..4u8).find(|&i| !m.done[i as usize]);
                m.scroll.follow = true;
                m.typing = m.open == Some(mstep::TEXT);
            }
            Action::MWallet(i) => {
                m.wallet = Some(i);
                m.index = 0;
                m.signed = None;
            }
            Action::MIndex(d) => {
                m.index = (i64::from(m.index) + i64::from(d)).clamp(0, 9999) as u32;
                m.signed = None;
            }
            Action::MType => m.typing = !m.typing,
            Action::MFormat(b) => {
                m.bip322 = b;
                m.signed = None;
            }
            _ => return false,
        }
        true
    }

    /// A key while the message is being typed: whether it was taken.
    pub(crate) fn key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        let Some(m) = self.state.as_mut().filter(|m| m.typing) else {
            return false;
        };
        match key {
            K::Char(c) => {
                m.text.push(c);
                m.signed = None;
            }
            K::Enter => {
                m.text.push('\n');
                m.signed = None;
            }
            K::Backspace => {
                m.text.pop();
                m.signed = None;
            }
            K::Escape => m.typing = false,
            _ => {}
        }
        true
    }

    /// Types a text into the message, as a test does.
    #[cfg(feature = "testkit")]
    pub(crate) fn set_text(&mut self, text: &str) {
        if let Some(m) = self.state.as_mut() {
            m.text = text.to_string();
        }
    }
}

/// The visit screen's choices.
#[derive(Default)]
pub struct VisitState {
    /// The stick shown, by index.
    pub stick: usize,
    /// On a small window, whether Files' tab On the stick is the one
    /// shown (else In Faraday).
    pub tab_stick: bool,
    /// Outbox files chosen to write, by name.
    pub out: BTreeSet<String>,
    /// Stick files chosen to copy, by name.
    pub inn: BTreeSet<String>,
    /// Stick files asked for and not yet answered, by name: a picture's
    /// codes are read off the shell's thread, one picture at a time.
    pub reading: BTreeSet<String>,
    /// The settings file's row as the person set it; `None` for its
    /// default (`Faraday::visit_settings_on`).
    pub settings: Option<bool>,
    /// What the last write or copy did, one line per file.
    pub log: Vec<(String, bool)>,
    /// Names from an "Import and load" copy, by name: loaded as keys the
    /// moment the stick is pulled, no further press needed.
    pub load_after: BTreeSet<String>,
    /// How far the Outbox list is scrolled, design units. The stick's
    /// files scroll by [`Faraday::list_offset`].
    pub out_offset: f32,
    /// Where each column's scrollbar was drawn, by [`VisitState::bar`].
    pub bars: [std::cell::Cell<Option<VisitBarAt>>; 2],
    /// Inbox files chosen to write, by name: a copy from one stick to
    /// another. One that may be a secret is chosen only on the secret
    /// sheet.
    pub from_inbox: BTreeSet<String>,
    /// Inbox files sent to be written and not yet answered, by name.
    pub writing_inbox: BTreeSet<String>,
    /// The write under way: the stick's label, and each file sent with
    /// the SHA-256 of its bytes. Its first answer starts a new receipt.
    pub writing: Option<Writing>,
    /// The files the write would write whose names are taken on the
    /// stick, while the Already on the stick sheet asks.
    pub taken: Vec<visit_taken::Taken>,
}

impl VisitState {
    /// Where `column`'s scrollbar was drawn, `None` when its list fits.
    pub fn bar(&self, column: Column) -> &std::cell::Cell<Option<VisitBarAt>> {
        &self.bars[match column {
            Column::Outbox => 0,
            Column::Stick => 1,
        }]
    }
}

/// Why a message cannot be signed or its signature does not verify, as
/// the screen says it.
pub(crate) fn message_error_text(e: faraday_psbt::message::Error) -> &'static str {
    use faraday_psbt::message::Error as E;
    match e {
        E::Invalid => EN.message_error_invalid,
        E::Malformed => EN.message_error_malformed,
        E::UnsupportedAddress => EN.message_error_unsupported_address,
        E::UnsupportedFormat => EN.message_error_unsupported_format,
    }
}

impl Faraday {
    /// Costs what the actions below cost, the keys' and wallets' work among
    /// it: handles the message-signing flow's actions (`SignMessage`, `M*`, `CheckMessage`).
    pub(crate) fn message_act(&mut self, action: Action, c: &Compute) {
        if self.message.edit(&action) {
            if matches!(action, Action::MWallet(_)) {
                self.message_format_fit();
            }
            return;
        }
        match action {
            Action::SignMessage => {
                let first = self.session.message_wallets().first().copied();
                self.message.state = Some(MessageState {
                    open: Some(mstep::ADDRESS),
                    wallet: first,
                    bip322: true,
                    ..MessageState::default()
                });
                self.message_format_fit();
                self.screen = Screen::Message;
            }
            Action::MScan => {
                if let Some(m) = self.message.state.as_mut() {
                    // Keys go to the scanner's sheet, not the message behind it.
                    m.typing = false;
                    let mut scan = crate::qr_view::ScanState::default();
                    scan.purpose = crate::ScanPurpose::MessageText;
                    self.camera.open(scan);
                    self.sheet = Some(crate::Sheet::Scan);
                    self.out
                        .commands
                        .push_back(faraday_shell_api::Command::CameraOn);
                }
            }
            Action::MSign => {
                if let Some(m) = self.message.state.as_mut()
                    && let Some(w) = m.wallet
                {
                    let format = if m.bip322 {
                        faraday_psbt::message::Format::Bip322
                    } else {
                        faraday_psbt::message::Format::Bip137
                    };
                    match self.session.sign_message(c, w, m.index, &m.text, format) {
                        Ok(signed) => {
                            m.signed = Some(signed);
                            m.out_name = None;
                            m.error = None;
                            m.done[mstep::SIGN as usize] = true;
                        }
                        Err(e) => m.error = Some(faraday_text::Text::of(&e)),
                    }
                }
                // The signed message goes For the stick as it is made.
                if self
                    .message
                    .state
                    .as_ref()
                    .is_some_and(|m| m.signed.is_some())
                {
                    self.act(Action::MOut);
                }
            }
            Action::MOut | Action::MQr => {
                if let Some(m) = self.message.state.as_ref()
                    && let Some(sig) = m.signed.as_ref()
                {
                    let text =
                        faraday_keys::message::signed_text(&sig.address, &sig.signature, &m.text);
                    if action == Action::MOut {
                        let name = format!(
                            "message-{}.txt",
                            &sig.address[sig.address.len().saturating_sub(6)..]
                        );
                        let name = self.put_outbox_beside(&name, text.as_bytes().to_vec());
                        if let Some(m) = self.message.state.as_mut() {
                            m.out_name = Some(String::from(&*name));
                        }
                        self.toast_out(&name);
                    } else {
                        let view = self.code_view(c, Code::Message);
                        self.open_qr(view);
                    }
                }
            }
            Action::CheckMessage(i) => {
                if i < self.inbox.len() {
                    self.message.checking = Some(i);
                    self.screen = Screen::CheckMessage;
                }
            }
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }

    /// Costs what a code read as a file costs and, for a secret, what
    /// [`scan_read::secret_kind`] costs. A code read on Sign a message's
    /// scanner: only plain text becomes the message, and nothing is
    /// filed. A secret of any kind and every other kind of code is named
    /// on the scanner and refused (`docs/SCANNING.md` §4.3). Returns
    /// whether the scanner on screen is that one.
    pub(crate) fn message_scanned(&mut self, bytes: &zeroize::Zeroizing<Vec<u8>>) -> bool {
        use crate::scan_read::{self, Kind};
        if self.camera.scan().map(|s| s.purpose) != Some(crate::ScanPurpose::MessageText) {
            return false;
        }
        let c = self.compute();
        // Plain text is the message here, so 16 or 32 bytes that are valid
        // UTF-8 are text, not a CompactSeedQR.
        let secret = scan_read::message_secret_kind;
        if let Some(kind) = secret(&c, bytes) {
            self.scan_say(kind.not_here());
            return true;
        }
        // Parts are put together first; what comes out is looked at again,
        // since a secret may be what the parts make.
        let Some((named, _, data)) = self.read_code(bytes.clone()) else {
            return true;
        };
        if let Some(kind) = secret(&c, &data) {
            self.scan_say(kind.not_here());
            return true;
        }
        let kind = scan_read::public_kind(
            crate::wallet::classify(&c, "", &data),
            named.is_some(),
            &data,
        );
        if kind != Kind::Text {
            self.scan_say(kind.not_here());
            return true;
        }
        // The text as scanned: a single code's own bytes, not the trimmed
        // text filed for it, and refused when they are not UTF-8.
        let Ok(text) = String::from_utf8(self.scanned_own(&data)) else {
            self.scan_say(Kind::Unreadable.not_here());
            return true;
        };
        if let Some(m) = self.message.state.as_mut() {
            m.text = text;
            m.typing = false;
            m.signed = None;
            m.error = None;
            self.scan_close();
        } else {
            self.scan_say(Kind::Unreadable.not_here());
        }
        true
    }

    /// Opens Sign a message with `text` as the message: the read card's
    /// **Sign this message**. Nothing opens for a code that is no UTF-8 or
    /// when no single-key wallet is loaded.
    pub(crate) fn message_open_with(&mut self, text: &[u8]) {
        let Ok(text) = std::str::from_utf8(text) else {
            return;
        };
        if self.session.message_wallets().is_empty() {
            return;
        }
        self.act(Action::SignMessage);
        if let Some(m) = self.message.state.as_mut() {
            m.text = text.to_string();
        }
    }

    /// Sign a message's format, kept to one its wallet allows: where the
    /// chosen one is greyed for the wallet, the first it allows is
    /// chosen.
    fn message_format_fit(&mut self) {
        use faraday_psbt::message::Format;
        let Some(m) = self.message.state.as_mut() else {
            return;
        };
        let allowed = m
            .wallet
            .and_then(|w| self.session.wallets.get(w))
            .and_then(|w| match crate::wallet::Kind::of(w.policy()) {
                crate::wallet::Kind::Single(t) => Some(Format::for_script(t)),
                _ => None,
            })
            .unwrap_or(&[]);
        let chosen = if m.bip322 {
            Format::Bip322
        } else {
            Format::Bip137
        };
        if !allowed.contains(&chosen)
            && let Some(first) = allowed.first()
        {
            m.bip322 = *first == Format::Bip322;
        }
    }
}

impl Faraday {
    /// The typed text as one more part; a codex32 secret is the key
    /// already: Add a key adds it, New key opens its Key card.
    pub(crate) fn form_add_part(&mut self) {
        let c = self.compute();
        let correcting = self.entry.state().correction.is_open();
        let done = self.entry.add_part(&c);
        self.correction_from_top(correcting);
        if done {
            self.form_complete();
            return;
        }
        self.form_codex32_vaults();
    }

    /// The strings or shares in make the key: Add a key adds it, New key
    /// opens its Key card.
    fn form_complete(&mut self) {
        if self.entry.state().c32_round.is_some() {
            return self.c32_round_take();
        }
        // Make the rest or Add a share: the strings go on into the make
        // flow, not into a key (`docs/CODEX32.md` §8.1, §8.2).
        if let Some(kind) = self.form_makes_more() {
            self.keygen_codex32_given(kind);
            return;
        }
        if self.screen == Screen::KeyGen {
            self.act(Action::KNext);
        } else {
            self.form_recover();
        }
    }

    /// Costs what `add_seed` costs, an interpolation of the strings and,
    /// where the open vaults' strings make up `k`, a parse of each of
    /// theirs. The key the codex32 strings typed make, with the open
    /// vaults' strings of their set where those typed are fewer than `k`
    /// (`docs/CODEX32.md` §9.5). It carries the strings' mark (§8.4), and
    /// its set's plain facts and strings are kept with it (§9.4, §9.5):
    /// k, the identifier, the length and the indices typed; the plan asks
    /// how many strings there are.
    fn form_key_codex32(
        &mut self,
        c: &Compute,
        label: &str,
    ) -> Result<faraday_bip::keys::Fingerprint, String> {
        let typed = &self.entry.state().parts.codex32;
        let fp =
            match self
                .vaults
                .store()
                .codex32_with_typed(c, &mut self.session.keys, typed, label)
            {
                Some(r) => r.map_err(|e| String::from(&*e.text()))?,
                None => {
                    let fp = faraday_keys::codex32_seed(typed)
                        .map_err(|e| {
                            fill(
                                EN.forms_the_strings_do,
                                &[&faraday_strings::bip::codex32_error(&e)],
                            )
                        })
                        .and_then(|seed| {
                            self.session
                                .keys
                                .add_seed(c, &seed, label)
                                .map_err(|e| String::from(&*e.text()))
                        })?;
                    let typed = &self.entry.state().parts.codex32;
                    let texts: Vec<zeroize::Zeroizing<String>> = typed
                        .iter()
                        .map(|s| zeroize::Zeroizing::new(s.encode().as_str().to_string()))
                        .collect();
                    let refs: Vec<&str> = texts.iter().map(|t| t.as_str()).collect();
                    if let Some(i) = self.session.keys.position(fp) {
                        self.session.keys.set_codex32(i, forms::set_facts(typed));
                        self.session.keys.keep_codex32(c, i, &refs);
                    }
                    fp
                }
            };
        if let Some(m) = self.entry.state().correction.key_mark() {
            self.session.key_marks.note(fp.0, m);
        }
        Ok(fp)
    }

    /// A parse of each codex32 share string the open vaults keep. The
    /// codex32 form's strings counted with the open vaults' strings of
    /// their set (`docs/CODEX32.md` §9.5), and the string checked with
    /// its set's: the counts kept on the form for drawing; once the
    /// strings typed and the vaults' make `k`, the key loads.
    pub(crate) fn form_codex32_vaults(&mut self) {
        let e = self.entry.state();
        if e.form != Form::Codex32 {
            return;
        }
        if e.c32_round.is_some() {
            return self.c32_round_take();
        }
        let n = self.vaults.store().codex32_count(&e.parts.codex32);
        let checked = e
            .codex32
            .checked
            .as_ref()
            .filter(|ch| ch.holds)
            .and_then(|_| Codex32::parse(e.typed.trim()).ok())
            .map_or(0, |s| self.vaults.store().codex32_count(&[s]));
        if self.entry.set_vault_counts(n, checked) {
            self.form_complete();
        }
    }

    /// Whether Add a key's form takes the keys typed: on Add a key, or on
    /// New key's Words card for a key typed (`docs/history/NEW-WALLET.md` §15.2).
    pub(crate) fn entry_typing_here(&self) -> bool {
        self.screen == Screen::Entry
            || (self.screen == Screen::KeyGen
                && self
                    .keygen
                    .state()
                    .is_some_and(|k| k.open == Some(crate::keygen::kstep::TYPED) && !k.locked))
    }

    /// A key typed into Add a key's form other than words, with the
    /// compute token a codex32 string checked takes (see
    /// [`EntryArea::part_key`]).
    pub(crate) fn entry_part_key(&mut self, key: faraday_shell_api::Key) -> Keyed {
        let c = self.compute();
        let out = self.entry.part_key(&c, key);
        // A correction taken may have put a string in (§9.5).
        if self.entry.state().form == Form::Codex32 && matches!(out, Keyed::Taken) {
            self.form_codex32_vaults();
        }
        out
    }

    /// A press on the codex32 form's correction (`docs/CODEX32.md`
    /// §8.4): a correction taken goes in as a string typed, with its mark.
    pub(crate) fn form_correct(&mut self, p: crate::codex32_correct::Press) {
        // A group of the re-entry's boxes pressed: its box nearest the
        // press, as the last frame drew it, takes the caret.
        if let crate::codex32_correct::Press::Group(_) = p {
            if let Some(i) = self.pressed_index(Action::Codex32Correct(p)) {
                self.entry.correction_caret_to_box(i + 1);
            }
            return;
        }
        let c = self.compute();
        let out = self.entry.correction_press(p);
        if self.entry.codex32_outcome(&c, out) {
            self.form_complete();
            return;
        }
        self.form_codex32_vaults();
    }

    /// A press on the codex32 form's own rows (§8.1 to §8.3, §8.5).
    pub(crate) fn form_codex32(&mut self, p: forms::C32Press) {
        use crate::codex32::Given;
        let kind = match p {
            forms::C32Press::CheckIt => return self.form_add_part(),
            forms::C32Press::Rest => Given::Rest,
            forms::C32Press::AddShare => Given::Add,
            _ => {
                if self.entry.codex32_press(p) {
                    self.form_complete();
                } else {
                    self.form_codex32_vaults();
                }
                return;
            }
        };
        // Pressed again it is off; on, the strings make the rest or a
        // share once k are in, at once where they are in already.
        if self.entry.toggle_making(kind) == Some(true) {
            self.form_complete();
        }
    }

    /// The codex32 form's strings make more than the key: Make the rest or
    /// Add a share was pressed and `k` are in (§8.1, §8.2).
    fn form_makes_more(&self) -> Option<crate::codex32::Given> {
        let e = self.entry.state();
        (e.form == Form::Codex32 && forms::makes_more(&e.parts))
            .then_some(e.codex32.making)
            .flatten()
    }

    /// Puts the parts together and adds the key.
    pub(crate) fn form_recover(&mut self) {
        if !self.may_load_keys()
            || self.entry.state().form == Form::Words
            || self.entry.state().differs()
        {
            return;
        }
        if let Some(kind) = self.form_makes_more() {
            return self.keygen_codex32_given(kind);
        }
        match self.form_key() {
            Ok(fp) => {
                self.toast(&fill(EN.forms_key_added, &[&fp_text(fp)]));
                let back = self.entry.state().back;
                self.seeds_took(fp, back);
                self.entry.reset();
                self.refresh_spend();
                self.screen = back.unwrap_or(crate::Screen::Wallets);
                if back == Some(crate::Screen::SpendBackup) {
                    self.spend_backup_key_added();
                }
            }
            Err(e) => self.entry.set_error(e),
        }
    }

    /// Puts the parts typed together and loads the key they make, with
    /// the passphrase typed (not for codex32); what is wrong otherwise.
    /// Add a key's Make the key and New key's Lock in for a form other
    /// than words both load through here (`docs/history/NEW-WALLET.md` §15.2).
    pub(crate) fn form_key(&mut self) -> Result<faraday_bip::keys::Fingerprint, String> {
        let c = self.compute();
        let form = self.entry.state().form;
        let label = match form {
            Form::Slip39 => EN.forms_slip_39_key,
            Form::Codex32 => EN.forms_codex32_key,
            _ => EN.forms_seed_xor_key,
        };
        match form {
            Form::Slip39 => faraday_keys::slip39_seed(
                &c,
                &self.entry.state().parts.slip39,
                self.entry.state().passphrase.as_bytes(),
            )
            .map_err(|e| {
                fill(
                    EN.forms_the_shares_do,
                    &[&faraday_strings::bip::slip39_error(&e)],
                )
            })
            .and_then(|seed| {
                self.session
                    .keys
                    .add_seed(&c, &seed, label)
                    .map_err(|e| String::from(&*e.text()))
            }),
            Form::Codex32 => self.form_key_codex32(&c, label),
            Form::Xor => self
                .entry
                .state()
                .parts
                .xor
                .entropy()
                .map_err(|_| EN.forms_seed_xor_needs.to_string())
                .and_then(|e| {
                    faraday_keys::words::from_entropy(Language::English, e.as_bytes())
                        .map_err(|e| String::from(&*faraday_strings::bip::bip39_error(&e)))
                })
                .and_then(|m| {
                    let words: Vec<&str> = m
                        .indices()
                        .iter()
                        .map(|&i| Language::English.word(i))
                        .collect();
                    let phrase = zeroize::Zeroizing::new(words.join(" "));
                    self.session
                        .keys
                        .add_words_with(&c, &phrase, &self.entry.state().passphrase, label, None)
                        .map_err(|e| String::from(&*e.text()))
                }),
            // Words are loaded as words (`Faraday::typed_load`), never here.
            Form::Words => Err(String::new()),
        }
    }
}
