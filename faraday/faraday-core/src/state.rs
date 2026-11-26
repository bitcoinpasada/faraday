//! The state of screens that belong to no area folder: seed entry, signing a
//! message, and the lock, power and idle-warning sheets with
//! their timers.

use crate::codex32_correct::{Offered, typable};
use crate::forms::{Form, completion, slip39_completion};
use crate::idle;
use crate::strings::EN;
#[cfg(feature = "demo")]
use crate::transfer;
use crate::wallet::fp_text;
use crate::{Action, Code, Faraday, Screen, Sheet, flow, forms, mstep, secret_text};
use crate::{
    Medium, SecureBootAtStart, Session, backup_flow, backups, bip85, boot_import, compact, create,
    decode, devices, drawing, edit, explore, fade, fresh, glance, guide, in_progress, inbox,
    inputs, keygen, lightning, memory, modes, motion, out, pointer, qr_view, restore, samples,
    scan_key, secrets, seeds, set_time, silent, spend_backup, state, stick, tools, trail, vanity,
    vault, wallet, wallet_backup, wordlist, worker,
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
    pub label: faraday_text::Text,
    /// The last refusal.
    pub error: Option<faraday_text::Text>,
    /// `error` is the passphrase field's refusal of the key just typed:
    /// it stays until the next key.
    pub refused: bool,
    /// Where to go once the key is added.
    pub back: Option<Screen>,
    /// The BIP-39 passphrase, when one is typed, and the one typed again
    /// where it is asked twice ([`EntryState::pair`]).
    pub(crate) passphrase: crate::pass_field::PassField,
    /// Typing goes to the passphrase rather than the words.
    pub on_passphrase: bool,
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
    /// directly, or set already by a Tools tile for one of those forms.
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
    probe: std::cell::RefCell<Option<Result<faraday_bip::keys::Fingerprint, faraday_text::Text>>>,
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
            && crate::pass_field::pair::differ(
                self.passphrase.as_str(),
                self.passphrase.again_str(),
            )
    }

    /// The passphrase typed.
    pub fn passphrase(&self) -> &str {
        self.passphrase.as_str()
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
    ) -> Result<faraday_bip::keys::Fingerprint, faraday_text::Text> {
        if let Some(r) = self.probe.borrow().as_ref() {
            return r.clone();
        }
        // Kept until an event forgets it: worked out once, by the frame
        // that asks first.
        let r = faraday_keys::identity::fingerprint_of_mnemonic(c, m, self.passphrase.borrow())
            .map_err(|e| crate::wallet::Refusal::Words(faraday_text::Text::of(&e)).text());
        *self.probe.borrow_mut() = Some(r.clone());
        r
    }

    /// Forgets the status line's key, for an event that may have changed
    /// the words, list or passphrase.
    pub(crate) fn forget_probe(&self) {
        self.probe.borrow_mut().take();
    }

    /// The status line's key as kept, if it is: a lookup.
    pub(crate) fn probe_kept(
        &self,
    ) -> Option<Result<faraday_bip::keys::Fingerprint, faraday_text::Text>> {
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
    /// Wallet `gone` left the session list: the place kept moves up, or
    /// goes when it is the one gone.
    pub(crate) fn wallet_left(&mut self, gone: usize) {
        crate::wallet::kept_place_after_removal(&mut self.state.wallet, gone);
    }

    /// The entry's state.
    pub(crate) fn state(&self) -> &EntryState {
        &self.state
    }

    /// Runs `f` on the passphrase field.
    pub(crate) fn with_pass(&mut self, f: &mut dyn FnMut(&mut crate::pass_field::PassField)) {
        f(&mut self.state.passphrase);
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

    /// Forgets what the form holds but keeps what is wanted, the wallet
    /// it is for and where Back goes, and shows `form` with the chips of the other forms.
    pub(crate) fn choose_form(&mut self, form: Form) {
        let (wanted, back, wallet) = (self.state.wanted, self.state.back, self.state.wallet);
        self.reset();
        self.state.wanted = wanted;
        self.state.back = back;
        self.state.wallet = wallet;
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
    pub(crate) fn refuse_round(&mut self, line: &str) {
        self.state.parts.codex32.clear();
        self.state.error = Some(faraday_text::Text::of(line));
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
        self.state.passphrase.fill_first(pass);
    }

    /// Puts `pass` in as the passphrase's repeat.
    pub(crate) fn set_again(&mut self, pass: &str) {
        self.state.passphrase.fill_again(pass);
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

    /// A field of the form pressed: neither passphrase field has the
    /// focus and the chip is gone.
    pub(crate) fn focus_words(&mut self) {
        self.state.on_passphrase = false;
        self.state.passphrase.set_on_again(false);
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
        e.on_passphrase = !(e.on_passphrase && !e.passphrase.on_again());
        e.passphrase.set_on_again(false);
    }

    /// The passphrase's repeat has the focus.
    pub(crate) fn focus_again(&mut self) {
        self.state.on_passphrase = true;
        self.state.passphrase.set_on_again(true);
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
            let mut typed = faraday_text::Text::default();
            for &i in idx.iter() {
                if let Some(t) = lang.typed(i) {
                    typed.extend(t.as_chars().iter().copied());
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
            Form::Codex32 | Form::Krux | Form::Xprv | Form::HexSeed => None,
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
    pub(crate) fn mnemonic(&mut self) -> Result<faraday_bip::bip39::Mnemonic, faraday_text::Text> {
        let e = &mut self.state;
        let lang = e.language();
        if let Some(w) = e.keys.as_mut() {
            forms::take_typed(w, lang);
        }
        match e.keys.as_ref() {
            Some(w) => {
                let mut idx: Vec<u16> = w.committed_indices().collect();
                let m = faraday_keys::words::from_indices(lang, &idx)
                    .map_err(|e| faraday_strings::bip::bip39_error(&e));
                zeroize::Zeroize::zeroize(&mut idx);
                m
            }
            None => forms::typed_mnemonic(lang, &e.typed),
        }
    }

    /// A key while the passphrase field has the keyboard. Enter says
    /// what to do with the words or parts typed. A character, Backspace
    /// and Shift-Tab are the field's, with the stick rule (`gate`); where one is refused
    /// the field says why.
    pub(crate) fn passphrase_key(
        &mut self,
        key: faraday_shell_api::Key,
        gate: crate::pass_field::Gate,
        medium: crate::Medium,
    ) -> Keyed {
        use crate::pass_field::PassKey;
        use faraday_shell_api::Key as K;
        let e = &mut self.state;
        let (pair, second) = (e.pair(), e.passphrase.on_again());
        match key {
            // Shift-Tab goes to the field too: the stick rule refuses it
            // as it refuses a character, and in a pair it moves back.
            K::Char(_) | K::Backspace | K::BackTab => {
                if let PassKey::Refused(why) = e.passphrase.key(key, gate) {
                    e.error = Some(crate::pass_field::refused_text(medium, why));
                    e.refused = true;
                }
            }
            // The first field's Enter and Tab move to the second.
            K::Enter | K::Tab if pair && !second => e.passphrase.set_on_again(true),
            K::Enter if e.form == Form::Words => return Keyed::Add,
            K::Enter => return Keyed::Recover,
            K::Escape | K::Tab => {
                e.on_passphrase = false;
                e.passphrase.set_on_again(false);
            }
            _ => {}
        }
        Keyed::Taken
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
        // One line of text: what is typed is kept as it is, with its case
        // (`docs/KEY-FORMS.md` §6).
        if self.state.form.typing() == faraday_forms::Typing::OneLine {
            match key {
                K::Char(c) if !c.is_control() => {
                    self.state.typed.push(c);
                    self.state.error = None;
                }
                K::Backspace => {
                    self.state.typed.pop();
                    self.state.error = None;
                }
                K::Enter => return Keyed::Recover,
                K::Escape => return Keyed::Leave,
                _ => {}
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
        let text: faraday_text::Text = e
            .typed
            .trim()
            .chars()
            .flat_map(char::to_lowercase)
            .collect();
        let result: Result<(), faraday_text::Text> = match form {
            Form::Slip39 => add_slip39(&mut e.parts, &text),
            Form::Xor => add_xor(c, &mut e.parts, &text),
            Form::Codex32 | Form::Words | Form::Krux | Form::Xprv | Form::HexSeed => Ok(()),
        };
        match result {
            Ok(()) => {
                e.sample.emptied();
                e.typed.clear();
                e.error = None;
                false
            }
            Err(msg) => {
                e.error = Some(msg);
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
        let given = faraday_text::Text::of(e.typed.trim());
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
                e.error = Some(msg);
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
pub(crate) fn ascii_refused() -> &'static str {
    EN.passphrase_ascii_only
}

/// One SLIP-39 share more, refused when it is not one or is a repeat.
fn add_slip39(parts: &mut forms::Parts, text: &str) -> Result<(), faraday_text::Text> {
    let s = slip39::Share::parse(text).map_err(|e| {
        faraday_text::Text::fill(
            EN.forms_not_slip39_share,
            &[&faraday_strings::bip::slip39_error(&e)],
        )
    })?;
    if parts.slip39.iter().any(|t| t.indices() == s.indices()) {
        return Err(faraday_text::Text::of(EN.forms_share_duplicate));
    }
    // Never past the room made at the start: a list that grows leaves a
    // copy of every share behind.
    if parts.slip39.len() >= parts.slip39.capacity().min(forms::SLIP39_ROOM) {
        return Err(faraday_text::Text::of(EN.forms_shares_full));
    }
    parts.slip39.push(s);
    Ok(())
}

/// One codex32 string more, refused when it is not one or is of another
/// seed. The same string again changes nothing; another at an index
/// already in is handed back to ask about (`docs/CODEX32.md` §8.5).
fn add_codex32(parts: &mut forms::Parts, given: &str) -> Result<Added, faraday_text::Text> {
    let c = Codex32::parse(given).map_err(|e| forms::codex32_refusal(given, &e))?;
    if let Some(first) = parts.codex32.first()
        && first.identifier() != c.identifier()
    {
        return Err(faraday_text::Text::of(EN.forms_other_seed_string));
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
fn add_xor(c: &Compute, parts: &mut forms::Parts, text: &str) -> Result<(), faraday_text::Text> {
    let m = crate::wallet::parse_words(c, Language::English, text).map_err(|e| {
        faraday_text::Text::fill(
            EN.forms_not_bip_39_words,
            &[&faraday_strings::bip::bip39_error(&e)],
        )
    })?;
    let ent = m.entropy();
    parts
        .xor
        .push(ent.expose().as_bytes())
        .map_err(|_| faraday_text::Text::of(EN.forms_every_part_has))
}

impl MessageState {
    /// Drops the signature and what tells the key that made it.
    fn drop_signature(&mut self) {
        self.signed = None;
        self.signer = None;
    }

    /// The name the signed message's file is made under, from the last
    /// six characters of the address.
    pub(crate) fn file_name(&self) -> Option<faraday_text::Text> {
        let address = &self.signed.as_ref()?.address;
        Some(faraday_text::text!(
            "message-{}.txt",
            &address[address.len().saturating_sub(6)..]
        ))
    }
}

/// The receive address Sign a message signs for, kept beside the wallet
/// and the number it was worked out for.
#[derive(Clone)]
pub struct ShownReceive {
    /// The wallet it is of.
    pub wallet: usize,
    /// The receive number it is of.
    pub index: u32,
    /// The address in groups of four, or the line said in place of an
    /// address held back until its key's backup.
    pub text: faraday_text::Text,
    /// Held back: `text` is the line, not an address.
    pub held: bool,
    /// What the session said of the wallet when it was worked out; it is
    /// drawn only while the session still says the same
    /// ([`Faraday::message_shown`]).
    pub basis: ShownBasis,
}

/// What an address kept for Sign a message depends on beyond the wallet's
/// place and the number: the wallet itself (a place names another wallet
/// once one is removed or added), its name, the network and whether its
/// key waits for its backup.
#[derive(Clone, PartialEq)]
pub struct ShownBasis {
    id: faraday_bip::policy::WalletId,
    name: faraday_text::Text,
    network: faraday_bip::keys::Network,
    waits: bool,
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
    /// Opened from this key's details: the wallet step lists its wallets
    /// alone.
    pub of_key: Option<[u8; 4]>,
    /// The receive address's index.
    pub index: u32,
    /// An address of a loaded key by its path, which the address check
    /// found: signed for instead of a wallet's receive address.
    pub at: Option<crate::wallet::MessageAt>,
    /// The wallet's receive address at `index` as the address row and the
    /// card say it, worked out where the wallet or the number changes
    /// ([`Faraday::message_address`]); drawing only reads it.
    pub shown: Option<ShownReceive>,
    /// The message.
    pub text: faraday_text::Text,
    /// Typing goes to the message.
    pub typing: bool,
    /// BIP-322 rather than BIP-137.
    pub bip322: bool,
    /// The signature, once made.
    pub signed: Option<faraday_psbt::message::Signed>,
    /// The fingerprint of the key that made `signed`, and all 32 bytes of
    /// what tells it from another, taken as it was made: its files are
    /// recorded with both, whatever wallet or key is there by then. Public.
    signer: Option<(faraday_bip::keys::Fingerprint, faraday_keys::KeyId)>,
    /// The name the signed message's file went out under.
    pub out_name: Option<faraday_text::Text>,
    /// The signed text is itself a secret (a seed, a key or a share): its
    /// file is refused, so none is offered; its code still is.
    pub out_refused: bool,
    /// The SHA-256 of that file's bytes, taken when it was made, so that
    /// "written and checked" holds only for these bytes.
    pub out_sha: Option<[u8; 32]>,
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
    /// The signed message just made in Sign a message, checked without
    /// being filed: its file's name and text.
    made: Option<(faraday_text::Text, faraday_text::Text)>,
    /// The screen Check a signed message was opened from, and returns to.
    check_back: Option<crate::Screen>,
    /// The message as drawn, kept while it is the same.
    shown: crate::visible::Shown,
    /// How far Check a signed message's flow card is scrolled, where it is
    /// drawn as one.
    check_scroll: crate::flow::Scroll,
    /// Which key signed each signed message's file put For the stick
    /// ([`Faraday::message_file_waits`]). Outlives the flow and a lock, as
    /// the file does.
    files: Vec<SignedFile>,
}

/// A signed message's file and the key that signed it. The file is told by
/// its name and SHA-256: another file under the name is not it.
struct SignedFile {
    name: faraday_text::Text,
    sha: [u8; 32],
    key: faraday_bip::keys::Fingerprint,
    /// All 32 bytes of what tells that key from another: another key with
    /// the same fingerprint does not clear the hold. Public.
    id: Option<faraday_keys::KeyId>,
    /// The key was found waiting for its backup, and has not been seen
    /// loaded and released since: the file stays held back while the key
    /// is not loaded, after a lock too ([`MessageArea::sync_holds`]).
    held: bool,
}

impl MessageArea {
    /// Notes that `key`, told whole by `id`, signed the file `name` of
    /// SHA-256 `sha`.
    pub(crate) fn file_signed(
        &mut self,
        name: &str,
        sha: [u8; 32],
        key: faraday_bip::keys::Fingerprint,
        id: Option<faraday_keys::KeyId>,
    ) {
        if !self
            .signed_as(name, Some(sha))
            .any(|f| f.key == key && f.id == id)
        {
            self.files.push(SignedFile {
                name: faraday_text::Text::of(name),
                sha,
                key,
                id,
                held: false,
            });
        }
    }

    /// Takes the keys held now and the keys loaded: a file whose key is
    /// held is held back from here on, and one whose very key is loaded
    /// and not held, its backup done, is not. A file with no id noted has
    /// no hold to keep.
    pub(crate) fn sync_holds(
        &mut self,
        held: &[faraday_bip::keys::Fingerprint],
        loaded: impl Fn(faraday_keys::KeyId) -> bool,
    ) {
        for f in &mut self.files {
            if held.contains(&f.key) {
                f.held = f.id.is_some();
            } else if f.id.is_some_and(&loaded) {
                f.held = false;
            }
        }
    }

    /// The keys that signed the file `name` of SHA-256 `sha`.
    pub(crate) fn signers(
        &self,
        name: &str,
        sha: Option<[u8; 32]>,
    ) -> impl Iterator<Item = faraday_bip::keys::Fingerprint> {
        self.signed_as(name, sha).map(|f| f.key)
    }

    /// What is noted of the file `name` of SHA-256 `sha`, once for each
    /// key that signed it.
    fn signed_as(&self, name: &str, sha: Option<[u8; 32]>) -> impl Iterator<Item = &SignedFile> {
        self.files
            .iter()
            .filter(move |f| *f.name == *name && sha == Some(f.sha))
    }

    /// The file `from` of SHA-256 `sha` is renamed `to` in Files.
    pub(crate) fn file_renamed(&mut self, from: &str, to: &str, sha: Option<[u8; 32]>) {
        for f in &mut self.files {
            if *f.name == *from && sha == Some(f.sha) {
                f.name = faraday_text::Text::of(to);
            }
        }
    }

    /// What a lock keeps of the signed files: a line of SHA-256, key, the
    /// key's id and name each, an `h` after the key of a file held back.
    /// Nothing in it is secret.
    pub(crate) fn files_encode(&self) -> Vec<u8> {
        let mut out = faraday_text::Text::new();
        for f in &self.files {
            out.push_str(&faraday_text::text!(
                "{}\t{}{}\t{}\t{}\n",
                &*faraday_crypto::hex::encode_string(&f.sha),
                &*faraday_crypto::hex::encode_string(&f.key.0),
                if f.held { "h" } else { "" },
                &*f.id.map_or_else(Default::default, |id| {
                    faraday_crypto::hex::encode_string(&id.0)
                }),
                &*f.name
            ));
        }
        out.as_bytes().to_vec()
    }

    /// Reads [`MessageArea::files_encode`] back.
    pub(crate) fn files_restore(&mut self, bytes: &[u8]) {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return;
        };
        for line in text.lines() {
            let mut parts = line.splitn(4, '\t');
            let (Some(sha), Some(key), Some(id), Some(name)) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            else {
                continue;
            };
            let (key, held) = key.strip_suffix('h').map_or((key, false), |k| (k, true));
            if let (Some(s), Some(k)) = (
                faraday_crypto::hex::decode_exact::<32>(sha),
                faraday_crypto::hex::decode_exact::<4>(key),
            ) {
                let k = faraday_bip::keys::Fingerprint(k);
                // With no id, no key could clear a hold: read as not held.
                let id = faraday_crypto::hex::decode_exact::<32>(id).map(faraday_keys::KeyId);
                self.file_signed(name, s, k, id);
                for f in &mut self.files {
                    if held && id.is_some() && f.sha == s && f.id == id && *f.name == *name {
                        f.held = true;
                    }
                }
            }
        }
    }

    /// The key that made the open flow's signature: its fingerprint and id.
    pub(crate) fn signer(&self) -> Option<(faraday_bip::keys::Fingerprint, faraday_keys::KeyId)> {
        self.state.as_ref()?.signer
    }

    /// For the testkit: a signature put in the open flow, no step run.
    #[cfg(feature = "testkit")]
    pub(crate) fn put_signed(
        &mut self,
        signed: faraday_psbt::message::Signed,
        signer: Option<(faraday_bip::keys::Fingerprint, faraday_keys::KeyId)>,
    ) {
        if let Some(m) = self.state.as_mut() {
            m.signed = Some(signed);
            m.signer = signer;
        }
    }

    /// For the testkit: whether a message is kept for **Check it**.
    #[cfg(feature = "testkit")]
    pub(crate) fn made_kept(&self) -> bool {
        self.made.is_some()
    }

    /// How far Check a signed message's card is scrolled.
    pub(crate) fn check_scroll(&self) -> crate::flow::Scroll {
        self.check_scroll
    }

    pub(crate) fn set_check_scroll(&mut self, to: crate::flow::Scroll) {
        self.check_scroll = to;
    }

    /// The card's scroll, units.
    pub(crate) fn check_scroll_y(&self) -> f32 {
        self.check_scroll.y
    }

    pub(crate) fn set_check_scroll_y(&mut self, y: f32) {
        self.check_scroll.y = y;
    }

    /// The Files index of the signed message being checked, none for one
    /// just made.
    pub(crate) fn checking(&self) -> Option<usize> {
        self.checking
    }

    /// Wallet `gone` left the session list: the place kept moves up, or
    /// goes when it is the one gone. True when it was the flow's own.
    pub(crate) fn wallet_left(&mut self, gone: usize) -> bool {
        let Some(s) = self.state.as_mut() else {
            return false;
        };
        let own = s.wallet == Some(gone);
        crate::wallet::kept_place_after_removal(&mut s.wallet, gone);
        own
    }

    /// The message as drawn: its visible form and lines, kept.
    pub(crate) fn shown(&self) -> &crate::visible::Shown {
        &self.shown
    }

    /// The cards' state, while the flow is open.
    pub(crate) fn state(&self) -> Option<&MessageState> {
        self.state.as_ref()
    }

    /// The screen Check a signed message returns to: where it was opened
    /// from, Wallets when that is not known.
    pub(crate) fn check_back(&self) -> crate::Screen {
        self.check_back.unwrap_or(crate::Screen::WalletsStart)
    }

    /// Whether Check a signed message is open with no message chosen: the
    /// files to choose from.
    pub(crate) fn landing(&self) -> bool {
        self.checking.is_none() && self.made.is_none() && self.check_back.is_some()
    }

    /// Whether a signed message is being checked.
    pub(crate) fn has_check(&self) -> bool {
        self.checking.is_some() || self.made.is_some()
    }

    /// The signed message being checked: its file's name and its text,
    /// from the Inbox file or from the one just made.
    pub(crate) fn check_text<'a>(
        &'a self,
        inbox: &'a [crate::Item],
    ) -> Option<(&'a faraday_text::Text, &'a [u8])> {
        match (self.checking.and_then(|i| inbox.get(i)), &self.made) {
            (Some(item), _) => Some((&item.name, &item.bytes)),
            (None, Some((name, text))) => Some((name, text.as_bytes())),
            (None, None) => None,
        }
    }

    /// Opens Check a signed message with no message chosen, returning to
    /// `back`.
    pub(crate) fn open_landing(&mut self, back: crate::Screen) {
        self.checking = None;
        self.made = None;
        self.check_back = Some(back);
    }

    /// Forgets the cards, leaving the screen to the caller.
    pub(crate) fn clear_state(&mut self) {
        self.state = None;
    }

    /// Forgets the cards and the file being checked.
    pub(crate) fn clear(&mut self) {
        self.state = None;
        self.checking = None;
        self.made = None;
        self.check_back = None;
        self.check_scroll = crate::flow::Scroll::default();
        self.shown = crate::visible::Shown::default();
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
            // A step is always open: a press on the open one does nothing.
            Action::MStep(n) if m.open != Some(n) => {
                m.open = Some(n);
                m.scroll.follow = true;
                m.typing = m.open == Some(mstep::TEXT);
            }
            Action::MStep(_) => {}
            Action::MNext(n) => {
                m.done[n as usize] = true;
                m.open = (0..4u8).find(|&i| !m.done[i as usize]).or(m.open);
                m.scroll.follow = true;
                m.typing = m.open == Some(mstep::TEXT);
            }
            Action::MWallet(i) => {
                m.wallet = Some(i);
                m.at = None;
                m.index = 0;
                m.drop_signature();
            }
            Action::MIndex(d) if m.at.is_none() => {
                m.index = (i64::from(m.index) + i64::from(d)).clamp(0, 9999) as u32;
                m.drop_signature();
            }
            Action::MIndex(_) => {}
            Action::MType => m.typing = !m.typing,
            Action::MFormat(b) => {
                m.bip322 = b;
                m.drop_signature();
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
                m.drop_signature();
            }
            K::Enter => {
                m.text.push('\n');
                m.drop_signature();
            }
            K::Backspace => {
                m.text.pop();
                m.drop_signature();
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
            m.text = faraday_text::Text::of(text);
        }
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
        E::UnsupportedVariant => EN.message_error_unsupported_variant,
    }
}

impl Faraday {
    /// Costs what the actions below cost, the keys' and wallets' work among
    /// it: handles the message-signing flow's actions (`SignMessage`, `M*`, `CheckMessage`).
    pub(crate) fn message_act(&mut self, action: Action, c: &Compute) {
        self.message_do(action, c);
        self.message_address(c);
    }

    /// Costs an address derivation, the first time for a wallet and a
    /// number. Works the receive address of the chosen wallet and number out
    /// once, through the function the card uses ([`Faraday::receive_line`],
    /// which reads the address `Session::sign_message` checks its signature
    /// against), and keeps it until either changes. A wallet that cannot
    /// sign a message, or an address picked, has none.
    ///
    /// First, where what a message was signed with now waits for its key's
    /// backup, what was made of it is dropped: the signature, the file's
    /// name and the message kept for **Check it**, each holding the address.
    pub(crate) fn message_address(&mut self, c: &Compute) {
        let Some(m) = self.message.state.as_ref() else {
            return;
        };
        if m.signed.is_some() && self.message_waits(m.at.as_ref(), m.wallet) {
            self.message_drop_made();
        }
        let Some(m) = self.message.state.as_ref() else {
            return;
        };
        let want = match (m.at.is_some(), m.wallet) {
            (false, Some(w)) if self.session.message_wallets().contains(&w) => {
                self.message_basis(w).map(|b| (w, m.index, b))
            }
            _ => None,
        };
        let have = m
            .shown
            .as_ref()
            .map(|s| (s.wallet, s.index, s.basis.clone()));
        if want == have {
            return;
        }
        let shown = want.and_then(|(wallet, index, basis)| {
            let wl = self.session.wallets.get(wallet)?;
            let (text, held) = self.receive_line(c, wl, false, index);
            Some(ShownReceive {
                wallet,
                index,
                text,
                held,
                basis,
            })
        });
        if let Some(m) = self.message.state.as_mut() {
            m.shown = shown;
        }
    }

    /// Drops what Sign a message made: the signature, the file's name, the
    /// message kept for **Check it**, and its code on show with the sheet.
    /// Files already made keep their records.
    pub(crate) fn message_drop_made(&mut self) {
        self.message.made = None;
        // Its code on show goes with it: no picture is made of it.
        if self
            .qr
            .view()
            .is_some_and(|q| q.title == EN.code_view_signed_message)
        {
            self.qr.close();
            if self.sheet == Some(Sheet::Qr) {
                self.sheet = None;
            }
        }
        if let Some(m) = self.message.state.as_mut() {
            m.drop_signature();
            m.out_name = None;
            m.out_sha = None;
            m.out_refused = false;
            m.done[mstep::SIGN as usize] = false;
        }
    }

    /// Costs what `message_address` costs, when Sign a message is open:
    /// the end of any action, which may have changed the session.
    pub(crate) fn message_address_after(&mut self) {
        if self.message.state.is_some() {
            let c = self.compute();
            self.message_address(&c);
        }
    }

    /// A lookup. What the session says now of wallet `w`, which a kept
    /// address is drawn only while it still matches.
    fn message_basis(&self, w: usize) -> Option<ShownBasis> {
        let wl = self.session.wallets.get(w)?;
        Some(ShownBasis {
            id: wl.id(),
            name: wl.name.clone(),
            network: self.session.network(),
            waits: self.wallet_waits_kept(w),
        })
    }

    /// A lookup. The wallets Sign a message offers, from the keys kept as
    /// waiting: those with their key loaded ([`wallet::Session::message_wallets`])
    /// whose key does not wait for its backup. For drawing and for what is
    /// offered; a press asks [`Faraday::message_waits_now`].
    pub(crate) fn message_offered(&self) -> Vec<usize> {
        let mut wallets = self.session.message_wallets();
        wallets.retain(|&w| !self.wallet_waits_kept(w));
        wallets
    }

    /// A lookup. [`Faraday::message_offered`], those of key `fp` alone.
    pub(crate) fn message_offered_of(&self, fp: [u8; 4]) -> Vec<usize> {
        let mut wallets = self.message_offered();
        wallets.retain(|&w| self.wallet_of_key(w, fp));
        wallets
    }

    /// A lookup. Whether key `fp` is the first key of wallet `w`.
    fn wallet_of_key(&self, w: usize, fp: [u8; 4]) -> bool {
        self.session
            .wallets
            .get(w)
            .and_then(|wl| wl.policy().keys().first()?.fingerprint())
            .is_some_and(|f| f.0 == fp)
    }

    /// A lookup. Whether the open message's wallet step lists wallet `w`:
    /// every wallet, or those of the key it was opened for.
    pub(crate) fn message_lists(&self, w: usize) -> bool {
        self.message
            .state()
            .and_then(|m| m.of_key)
            .is_none_or(|fp| self.wallet_of_key(w, fp))
    }

    /// A lookup. Whether what a message would be signed with, the address
    /// picked (`at`) or else `wallet`, waits for its key's backup, from the
    /// keys kept as waiting: for drawing.
    pub(crate) fn message_waits(
        &self,
        at: Option<&crate::wallet::MessageAt>,
        wallet: Option<usize>,
    ) -> bool {
        match at {
            Some(at) => self.session.waiting().contains(&at.fingerprint),
            None => wallet.is_some_and(|w| self.wallet_waits_kept(w)),
        }
    }

    /// Costs what `receive_held` or `key_held` costs.
    /// [`Faraday::message_waits`] for a press, worked out now.
    pub(crate) fn message_waits_now(
        &self,
        c: &Compute,
        at: Option<&crate::wallet::MessageAt>,
        wallet: Option<usize>,
    ) -> bool {
        match at {
            Some(at) => self.key_held(c, at.fingerprint),
            None => wallet
                .and_then(|w| self.session.wallets.get(w))
                .is_some_and(|wl| self.receive_held(c, wl)),
        }
    }

    /// A lookup. Whether the Outbox file `item` is a signed message whose
    /// key waits for its backup, from the keys kept as waiting: for
    /// drawing and the visit's ticks.
    pub(crate) fn message_file_waits(&self, item: &crate::Item) -> bool {
        let waiting = self.session.waiting();
        self.message
            .signed_as(&item.name, item.sha)
            .any(|f| waiting.contains(&f.key) || self.hold_stands(f))
    }

    /// A lookup. Whether `f` was held back and its key is not loaded to say
    /// that its backup is done.
    fn hold_stands(&self, f: &SignedFile) -> bool {
        f.held && !f.id.is_some_and(|id| self.session.keys.holds(id))
    }

    /// Costs what `key_held` costs for each key that signed `item`, and
    /// nothing for any other file. [`Faraday::message_file_waits`] for a
    /// write or a press, worked out now.
    pub(crate) fn message_file_waits_now(&self, c: &Compute, item: &crate::Item) -> bool {
        self.message
            .signed_as(&item.name, item.sha)
            .any(|f| self.hold_stands(f) || self.key_held(c, f.key))
    }

    /// A lookup. The key the open flow's signature is made with: the picked
    /// address's, or else the one key of the picked wallet.
    pub(crate) fn message_signer(&self) -> Option<faraday_bip::keys::Fingerprint> {
        let m = self.message.state.as_ref()?;
        match &m.at {
            Some(at) => Some(at.fingerprint),
            None => self
                .session
                .wallets
                .get(m.wallet?)?
                .policy()
                .keys()
                .first()?
                .fingerprint(),
        }
    }

    /// A lookup. [`Faraday::message_signer`] with the id of the loaded key
    /// of that fingerprint.
    pub(crate) fn message_signer_loaded(
        &self,
    ) -> Option<(faraday_bip::keys::Fingerprint, faraday_keys::KeyId)> {
        let fp = self.message_signer()?;
        Some((fp, self.loaded_key_id(fp)?))
    }

    /// A lookup. The kept receive address of Sign a message while the
    /// session still says what it said when it was worked out; nothing
    /// otherwise (a wallet removed, added, changed or renamed, the network
    /// or the key's backup changed): it is worked out again where the
    /// session changes ([`Faraday::message_address`]).
    pub(crate) fn message_shown(&self) -> Option<&ShownReceive> {
        let s = self.message.state()?.shown.as_ref()?;
        (self.message_basis(s.wallet).as_ref() == Some(&s.basis)).then_some(s)
    }

    /// Costs what `message_waits_now` costs. Refuses a press of Sign a
    /// message whose choice waits for its key's backup, saying so where
    /// the message's errors stand: nothing is signed or made of it.
    fn message_refused(&mut self, c: &Compute) -> bool {
        let waits = self
            .message
            .state
            .as_ref()
            .is_some_and(|m| self.message_waits_now(c, m.at.as_ref(), m.wallet));
        if waits && let Some(m) = self.message.state.as_mut() {
            m.error = Some(faraday_text::Text::of(EN.message_waits_backup));
        }
        waits
    }

    fn message_do(&mut self, action: Action, c: &Compute) {
        if self.message.edit(&action) {
            if matches!(action, Action::MWallet(_)) {
                self.message_format_fit();
            }
            return;
        }
        match action {
            Action::SignMessage => {
                let first = self.message_offered().first().copied();
                self.message.state = Some(MessageState {
                    open: Some(mstep::ADDRESS),
                    wallet: first,
                    bip322: true,
                    ..MessageState::default()
                });
                self.message_format_fit();
                self.screen = Screen::Message;
            }
            // From a wallet's card: that wallet is chosen, the address
            // card done, and the message is next.
            Action::SignMessageWith(wallet) => {
                if self.session.message_wallets().contains(&wallet)
                    && !self.message_waits_now(c, None, Some(wallet))
                {
                    self.message_open_at(c, wallet, 0);
                } else {
                    self.act(Action::SignMessage);
                }
            }
            // From a key's details: its wallets alone, the first picked.
            Action::SignMessageKey(fp) => {
                let mut wallets = self.message_offered_of(fp);
                wallets.retain(|&w| !self.message_waits_now(c, None, Some(w)));
                self.act(Action::SignMessage);
                if let Some((m, &first)) = self.message.state.as_mut().zip(wallets.first()) {
                    m.of_key = Some(fp);
                    m.wallet = Some(first);
                    self.message_format_fit();
                }
            }
            Action::MDone => {
                self.message.state = None;
                self.back();
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
                if self.message_refused(c) {
                    return;
                }
                // While the key is loaded to sign: only one of its
                // fingerprint can be.
                let signer = self.message_signer_loaded();
                if let Some(m) = self.message.state.as_mut()
                    && (m.at.is_some() || m.wallet.is_some())
                {
                    let format = if m.bip322 {
                        faraday_psbt::message::Format::Bip322
                    } else {
                        faraday_psbt::message::Format::Bip137
                    };
                    let made = match (&m.at, m.wallet) {
                        (Some(at), _) => self.session.sign_message_at(c, at, &m.text, format),
                        (None, Some(w)) => {
                            self.session.sign_message(c, w, m.index, &m.text, format)
                        }
                        (None, None) => return,
                    };
                    match made {
                        Ok(signed) => {
                            m.signed = Some(signed);
                            m.signer = signer;
                            m.out_name = None;
                            m.out_sha = None;
                            m.out_refused = false;
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
                if self.message_refused(c) {
                    return;
                }
                if let Some(m) = self.message.state.as_ref()
                    && let Some(sig) = m.signed.as_ref()
                {
                    let text =
                        faraday_keys::message::signed_text(&sig.address, &sig.signature, &m.text);
                    if action == Action::MOut {
                        let name = faraday_text::text!(
                            "message-{}.txt",
                            &sig.address[sig.address.len().saturating_sub(6)..]
                        );
                        // A message that is itself a secret never becomes a
                        // file (`put_item` refuses it): no name, no toast
                        // that it waits, the refusal said as a scanner says it.
                        let held = crate::Item::new(c, &name, text.as_bytes().to_vec());
                        if held.exposure() == crate::secrets::Exposure::Secret {
                            let why = match held.holds() {
                                crate::wallet::FileKind::Words => EN.secret_text_those_are,
                                crate::wallet::FileKind::KeyLine => EN.secret_text_private_key,
                                _ => EN.secret_text_file_holds,
                            };
                            if let Some(m) = self.message.state.as_mut() {
                                m.out_name = None;
                                m.out_sha = None;
                                m.out_refused = true;
                            }
                            self.toast(why);
                            return;
                        }
                        let sha = crate::sha256_of(c, text.as_bytes());
                        let name = self.put_outbox_beside(&name, text.as_bytes().to_vec());
                        // Kept on the item, so the take-away's row and
                        // press tell this file from another under its name.
                        if let Some(item) = self.outbox.iter_mut().find(|i| i.name == name) {
                            item.sha = Some(sha);
                        }
                        // With the key that signed it: the file is held
                        // back while that key waits for its backup.
                        let signer = self.message.signer();
                        if let Some((key, id)) = signer {
                            self.message.file_signed(&name, sha, key, Some(id));
                        }
                        if let Some(m) = self.message.state.as_mut() {
                            m.out_name = Some(name.duplicate());
                            m.out_sha = Some(sha);
                        }
                        // Ticked to write, as the take-away shows it.
                        self.visit.out_extend([&*name]);
                        self.toast_out(&name);
                        // The record is new: a hold already on its key
                        // takes it, and its tick goes.
                        self.sync_waiting();
                    } else {
                        let view = self.code_view(c, Code::Message);
                        self.open_qr(view);
                    }
                }
            }
            Action::CheckMessageAnother => {
                self.message.checking = None;
                self.message.made = None;
                self.list_offset = 0.0;
            }
            Action::CheckMessageMade => {
                if self.message_refused(c) {
                    return;
                }
                if let Some(m) = self.message.state.as_ref()
                    && let Some(sig) = m.signed.as_ref()
                {
                    let text =
                        faraday_keys::message::signed_text(&sig.address, &sig.signature, &m.text);
                    let name = faraday_text::text!(
                        "message-{}.txt",
                        &sig.address[sig.address.len().saturating_sub(6)..]
                    );
                    self.message.check_back = Some(self.screen);
                    self.message.checking = None;
                    self.message.made = Some((name, text));
                    self.list_offset = 0.0;
                    self.screen = Screen::CheckMessage;
                }
            }
            Action::CheckMessage(i) => {
                if i < self.inbox.len() {
                    self.message.made = None;
                    // Where it was opened from, unless it is open already.
                    if self.screen != Screen::CheckMessage {
                        self.message.check_back = Some(self.screen);
                    }
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
        if self.camera.scan().map(|s| s.purpose) != Some(crate::ScanPurpose::MessageText) {
            return false;
        }
        // Plain text is the message here, so 16 or 32 bytes that are valid
        // UTF-8 are text, not a CompactSeedQR.
        let Some(text) = self.scanned_plain_text(
            bytes,
            crate::scan_read::message_secret_kind,
            &[crate::scan_read::Kind::Text],
        ) else {
            return true;
        };
        if let Some(m) = self.message.state.as_mut() {
            m.text = text;
            m.typing = false;
            m.drop_signature();
            m.error = None;
            self.scan_close();
        } else {
            self.scan_say(crate::scan_read::Kind::Unreadable.not_here());
        }
        true
    }

    /// Costs what [`scan_read::secret_kind`] costs and a read of the code:
    /// the plain text of a code on a scanner that takes text into a field.
    /// A secret of any kind and every other kind of code is named on the
    /// scanner and gives `None`. `secret` says which kind of code is a
    /// secret here and `accept` which public kinds the field takes.
    pub(crate) fn scanned_plain_text(
        &mut self,
        bytes: &zeroize::Zeroizing<Vec<u8>>,
        secret: fn(&Compute, &[u8]) -> Option<crate::scan_read::Kind>,
        accept: &[crate::scan_read::Kind],
    ) -> Option<faraday_text::Text> {
        use crate::scan_read::{self, Kind};
        let c = self.compute();
        if let Some(kind) = secret(&c, bytes) {
            self.scan_say(kind.not_here());
            return None;
        }
        // Parts are put together first; what comes out is looked at again,
        // since a secret may be what the parts make.
        let (named, _, data) = self.read_code(bytes.clone())?;
        if let Some(kind) = secret(&c, &data) {
            self.scan_say(kind.not_here());
            return None;
        }
        let kind = scan_read::public_kind(
            crate::wallet::classify(&c, "", &data),
            named.is_some(),
            &data,
        );
        if !accept.contains(&kind) {
            self.scan_say(kind.not_here());
            return None;
        }
        // The text as scanned: a single code's own bytes, not the trimmed
        // text filed for it, and refused when they are not UTF-8.
        let text = std::str::from_utf8(&self.scanned_own(&data)).map(faraday_text::Text::of);
        if text.is_err() {
            self.scan_say(Kind::Unreadable.not_here());
        }
        text.ok()
    }

    /// Opens Sign a message on receive address `index` of the wallet at
    /// `wallet`, the address card done: the address check's **Sign a
    /// message with this address**. Nothing opens for a wallet that is no
    /// single-key wallet with its key loaded.
    pub(crate) fn message_open_at(&mut self, c: &Compute, wallet: usize, index: u32) {
        if !self.session.message_wallets().contains(&wallet) {
            return;
        }
        if self.message_waits_now(c, None, Some(wallet)) {
            self.toast(EN.message_waits_backup);
            return;
        }
        self.act(Action::SignMessage);
        if let Some(m) = self.message.state.as_mut() {
            m.wallet = Some(wallet);
            m.index = index;
            m.done[mstep::ADDRESS as usize] = true;
            m.open = Some(mstep::TEXT);
            m.typing = true;
            m.scroll.follow = true;
        }
        self.message_format_fit();
        self.message_address(c);
    }

    /// Opens Sign a message on the address `at` of a loaded key, the
    /// address card done: the address check's **Sign a message with this
    /// address** for a find outside a wallet's receive chain. Nothing opens
    /// for a key that is not loaded, nor for one that waits for its backup,
    /// which is said.
    pub(crate) fn message_open_path(&mut self, at: crate::wallet::MessageAt) {
        if self.session.keys.position(at.fingerprint).is_none() {
            return;
        }
        let c = self.compute();
        if self.message_waits_now(&c, Some(&at), None) {
            self.toast(EN.message_waits_backup);
            return;
        }
        self.act(Action::SignMessage);
        if let Some(m) = self.message.state.as_mut() {
            m.at = Some(at);
            m.wallet = None;
            m.done[mstep::ADDRESS as usize] = true;
            m.open = Some(mstep::TEXT);
            m.typing = true;
            m.scroll.follow = true;
        }
        self.message_format_fit();
    }

    /// Opens Sign a message with `text` as the message: the read card's
    /// **Sign this message**. Nothing opens for a code that is no UTF-8 or
    /// when Sign a message offers no wallet ([`Faraday::message_offered`]).
    pub(crate) fn message_open_with(&mut self, text: &[u8]) {
        let Ok(text) = std::str::from_utf8(text) else {
            return;
        };
        if self.message_offered().is_empty() {
            return;
        }
        self.act(Action::SignMessage);
        if let Some(m) = self.message.state.as_mut() {
            m.text = faraday_text::Text::of(text);
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
        let allowed =
            m.at.as_ref()
                .map(|at| Format::for_script(at.script))
                .or_else(|| {
                    m.wallet
                        .and_then(|w| self.session.wallets.get(w))
                        .and_then(|w| match crate::wallet::Kind::of(w.policy()) {
                            crate::wallet::Kind::Single(t) => Some(Format::for_script(t)),
                            _ => None,
                        })
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
        if crate::seed_check::here(self) {
            return crate::seed_check::form_done(self);
        }
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
    ) -> Result<faraday_bip::keys::Fingerprint, faraday_text::Text> {
        let typed = &self.entry.state().parts.codex32;
        let fp =
            match self
                .vaults
                .store()
                .codex32_with_typed(c, &mut self.session.keys, typed, label)
            {
                Some(r) => r.map_err(|e| e.text())?,
                None => {
                    let fp = faraday_keys::codex32_seed(typed)
                        .map_err(|e| {
                            faraday_text::Text::fill(
                                EN.forms_the_strings_do,
                                &[&faraday_strings::bip::codex32_error(&e)],
                            )
                        })
                        .and_then(|seed| {
                            self.session
                                .keys
                                .add_seed(c, &seed, label)
                                .map_err(|e| e.text())
                        })?;
                    let typed = &self.entry.state().parts.codex32;
                    let texts: Vec<faraday_text::Text> = typed
                        .iter()
                        .map(|s| faraday_text::Text::of(s.encode().as_str()))
                        .collect();
                    let refs: Vec<&str> = texts.iter().map(|t| t.as_str()).collect();
                    if let Some(i) = self.session.keys.position(fp) {
                        self.session.keys.set_codex32(i, forms::set_facts(typed));
                        self.session.keys.keep_codex32(c, i, &refs);
                    }
                    fp
                }
            };
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

    /// Whether My seed makes my wallet's Seed card shows the words form.
    pub(crate) fn seed_words_typing(&self) -> bool {
        self.tools
            .seed_check
            .state()
            .is_some_and(|s| s.typing_words())
    }

    /// Add a key was on screen (`was_on`) and is not now: what was typed
    /// into it is dropped, however it was left (the arrow, Escape, the
    /// mouse's back, a key loaded).
    pub(crate) fn entry_left(&mut self, was_on: bool) {
        if was_on && self.screen != Screen::Entry {
            self.entry.reset();
            self.pointer.forget_drawn();
        }
    }

    /// Whether Add a key's form takes the keys typed: on Add a key, on
    /// New key's Words card for a key typed
    /// or on My seed makes my wallet's Seed card.
    pub(crate) fn entry_typing_here(&self) -> bool {
        self.screen == Screen::Entry
            || (crate::seed_check::here(self) && self.seed_words_typing())
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
        if crate::seed_check::here(self) {
            return crate::seed_check::form_done(self);
        }
        if !self.may_load_keys()
            || self.entry.state().form == Form::Words
            || self.entry.state().differs()
            || self.typing.pass_eye.waits()
        {
            return;
        }
        if let Some(kind) = self.form_makes_more() {
            return self.keygen_codex32_given(kind);
        }
        // Restore takes only a key it fills a slot with: refused before
        // anything is loaded, the parts and passphrase stay on the form.
        if self.entry.state().back == Some(crate::Screen::Restore)
            && let Some(why) = self.form_restore_refusal()
        {
            self.entry.set_error(why);
            return;
        }
        let mark = self.entry.state().correction.key_mark();
        match self.form_key() {
            Ok(fp) => {
                if let Some(m) = mark {
                    self.session.key_marks.note(fp.0, m);
                }
                self.toast(&faraday_text::Text::fill(
                    EN.forms_key_added,
                    &[&fp_text(fp)],
                ));
                let back = self.entry.state().back;
                self.seeds_took(fp, back);
                self.entry.reset();
                crate::pass_field::eye::done(self);
                self.refresh_spend();
                self.screen = back.unwrap_or(crate::Screen::Wallets);
                if back == Some(crate::Screen::SpendBackup) {
                    self.spend_backup_key_added();
                }
            }
            Err(e) => self.entry.set_error(e),
        }
    }

    /// Costs what making the key of the parts costs. Why Restore does not
    /// take the key the parts typed make, if it does not; `None` too when
    /// the parts make no key yet (loading says why).
    fn form_restore_refusal(&mut self) -> Option<faraday_text::Text> {
        use faraday_keys::identity::Material;
        let c = self.compute();
        let st = self.entry.state();
        let pass = st.passphrase.as_str();
        match st.form {
            Form::Slip39 => {
                let seed = faraday_keys::slip39_seed(&c, &st.parts.slip39, pass.as_bytes()).ok()?;
                self.restore_refusal_of(&c, || Material::Seed(&seed), "")
            }
            Form::Codex32 => {
                let seed = faraday_keys::codex32_seed(&st.parts.codex32).ok()?;
                self.restore_refusal_of(&c, || Material::Seed(&seed), "")
            }
            Form::Xor => {
                let e = st.parts.xor.entropy().ok()?;
                let m = faraday_keys::words::from_entropy(Language::English, e.as_bytes()).ok()?;
                self.restore_refusal_of(&c, || Material::Words(&m), pass)
            }
            Form::Xprv => self.restore_refusal_of(&c, || Material::Xprv(st.typed.trim()), ""),
            Form::HexSeed => {
                let seed = faraday_keys::xprv::parse_hex_seed(st.typed.trim()).ok()?;
                self.restore_refusal_of(&c, || Material::Seed(&seed), "")
            }
            Form::Words | Form::Krux => None,
        }
    }

    /// Puts the parts typed together and loads the key they make, with
    /// the passphrase typed (not for codex32); what is wrong otherwise.
    /// Add a key's Make the key and New key's Lock in for a form other
    /// than words both load through here.
    pub(crate) fn form_key(
        &mut self,
    ) -> Result<faraday_bip::keys::Fingerprint, faraday_text::Text> {
        let c = self.compute();
        let form = self.entry.state().form;
        let label = match form {
            Form::Slip39 => EN.forms_slip_39_key,
            Form::Codex32 => EN.forms_codex32_key,
            Form::Xprv => EN.forms_xprv_key,
            Form::HexSeed => EN.forms_hex_seed_key,
            _ => EN.forms_seed_xor_key,
        };
        match form {
            Form::Slip39 => faraday_keys::slip39_seed(
                &c,
                &self.entry.state().parts.slip39,
                self.entry.state().passphrase.as_str().as_bytes(),
            )
            .map_err(|e| {
                faraday_text::Text::fill(
                    EN.forms_the_shares_do,
                    &[&faraday_strings::bip::slip39_error(&e)],
                )
            })
            .and_then(|seed| {
                // The seed these shares make is dropped (wiped) here.
                match self.session.keys.add_seed(&c, &seed, label) {
                    // A key made here and added without its shares quizzed:
                    // a quorum typed back with the same fingerprint is its
                    // copy checked, and it is released.
                    Err(crate::wallet::Refusal::Duplicate(fp))
                        if self.flows.made_here().contains(&fp) =>
                    {
                        self.flows.release_made(fp, &[]);
                        if let Some(k) = self.session.keys.position(fp) {
                            self.held_clear_marks(fp, k);
                        }
                        self.refresh_text_files();
                        Ok(fp)
                    }
                    other => other.map_err(|e| e.text()),
                }
            }),
            Form::Codex32 => self.form_key_codex32(&c, label),
            Form::Xor => self
                .entry
                .state()
                .parts
                .xor
                .entropy()
                .map_err(|_| faraday_text::Text::of(EN.forms_seed_xor_needs))
                .and_then(|e| {
                    faraday_keys::words::from_entropy(Language::English, e.as_bytes())
                        .map_err(|e| faraday_strings::bip::bip39_error(&e))
                })
                .and_then(|m| {
                    let words: Vec<&str> = m
                        .indices()
                        .iter()
                        .map(|&i| Language::English.word(i))
                        .collect();
                    let mut phrase = faraday_text::Text::default();
                    for (n, w) in words.iter().enumerate() {
                        if n > 0 {
                            phrase.push(' ');
                        }
                        phrase.push_str(w);
                    }
                    self.session
                        .keys
                        .add_words_with(
                            &c,
                            &phrase,
                            self.entry.state().passphrase.borrow(),
                            label,
                            None,
                        )
                        .map_err(|e| e.text())
                }),
            // Words are loaded as words (`Faraday::typed_load`), a Krux code
            // as the sheet that opens it loads it, never here.
            Form::Xprv => self
                .session
                .keys
                .add_xprv(&c, self.entry.state().typed.trim(), label)
                .map_err(|e| crate::one_line::xprv_error(&e)),
            Form::HexSeed => faraday_keys::xprv::parse_hex_seed(self.entry.state().typed.trim())
                .map_err(crate::one_line::hex_refusal)
                .and_then(|seed| {
                    self.session
                        .keys
                        .add_seed(&c, &seed, label)
                        .map_err(|e| e.text())
                }),
            Form::Words | Form::Krux => Err(faraday_text::Text::default()),
        }
    }
}

impl Faraday {
    /// Costs what the actions below cost, the keys' and wallets' work among
    /// it: handles lock, power and idle-timer actions (`Lock*`, `NotNow`, `Power*`, `Restart`, `Hold*`, `Idle*`).
    pub(crate) fn power_act(&mut self, action: Action, c: &Compute) {
        match action {
            Action::Lock => {
                // Scan again's Lock answered: the read card goes in any
                // case, even where the lock stops at its own ask.
                if matches!(self.visit.pull(), Some(Action::ReadAgain)) {
                    self.read_again_cancel();
                    self.visit.clear_pull();
                }
                self.lock();
            }
            Action::LockUnsealed => self.lock_now(true),
            Action::LockAsk => self.lock_ask(false),
            Action::NotNow => {
                self.visit.set_lock_to_write(None);
                // Scan again's Lock declined: the read card is wiped and
                // the press that waited for the pull is dropped.
                if matches!(self.visit.pull(), Some(Action::ReadAgain)) {
                    self.visit.clear_pull();
                }
                // No Lock is put over a card that is still to be drawn.
                if self.camera.scan().is_some_and(|s| s.card.is_some()) {
                    self.camera.close();
                }
                self.next_mode.clear();
                self.sheet = None;
                self.stick.set_not_now(true);
                self.import_resume();
            }
            Action::PowerAsk => {
                // An open vault's changes are never sealed at power-off:
                // the sheet says they are lost.
                let changed = self.vaults.open.iter().any(|v| v.changes > 0);
                if self.outbox.is_empty()
                    && !self.secure_boot_reminder()
                    && !changed
                    && !self.offers_restart()
                    && !self.lock_held(c)
                {
                    self.exit_forgetting();
                } else {
                    self.sheet = Some(Sheet::Power);
                }
            }
            Action::WalletHold => self.confirm_act(action),
            Action::PowerOff | Action::HoldPowerOff => {
                self.sheet = None;
                self.exit_forgetting();
            }
            Action::Restart | Action::HoldRestart => {
                self.sheet = None;
                self.restart_computer = self.offers_restart();
                self.exit_forgetting();
            }
            Action::IdleLock(m) => {
                self.settings.set_idle_lock_min(m);
                self.save_boxes();
            }
            Action::IdleOff(m) => {
                self.settings.set_idle_off_min(m);
                self.save_boxes();
            }
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }

    /// The idle timers. Returns whether the screen
    /// changed.
    pub(crate) fn idle_tick(&mut self, now_ms: u64) -> bool {
        // A seal that never returns must not keep the device open.
        if crate::vault::seal::seal_overdue(self, now_ms) {
            crate::vault::seal::seal_given_up(self);
            return true;
        }
        if self.hold_idle(now_ms) {
            return false;
        }
        let c = self.compute();
        let idle = self.idle.ms_starting(now_ms);
        let min = 60_000u64;
        // The warning, with its countdown, before the lock.
        // Make a new wallet before its keys are kept anywhere: no idle
        // lock at all.
        let held = self.lock_held(&c);
        // Held: no warning and no lock, but words on show are put away
        // once at the warning time; the flow, its key and the Seed XOR
        // parts stay.
        if held
            && let Some((warn, _)) = idle::lock_at(self.idle_lock_min())
            && idle >= warn
            && self.idle.mask_held_once()
        {
            self.secrets_off_screen();
            return true;
        }
        if let Some((warn, lock)) = idle::lock_at(self.idle_lock_min())
            && idle >= warn
            && idle < lock
            && self.lockable()
            && self.idle.locking().is_none()
            && !held
        {
            let changed = self.sheet != Some(Sheet::IdleWarn);
            // Over any sheet but a lock's own: a code on the QR sheet
            // goes too, and the warning counts down over it.
            if changed
                && !matches!(
                    self.sheet,
                    Some(Sheet::Lock | Sheet::LockAsk | Sheet::Locked)
                )
            {
                self.idle.set_under(self.sheet);
                self.sheet = Some(Sheet::IdleWarn);
                self.idle.set_warn_min(idle / min);
                self.secrets_off_screen();
            }
            let left = (lock - idle).div_ceil(1000);
            if self.sheet == Some(Sheet::IdleWarn) && self.idle.shown() != Some(left) {
                self.idle.set_shown(Some(left));
                return true;
            }
            return false;
        }
        // The Locked sheet counts down to power-off.
        if self.sheet == Some(Sheet::Locked)
            && let Some(off) = self.idle_off_at()
        {
            let left = off.saturating_sub(idle).div_ceil(1000);
            if self.idle.shown() != Some(left) {
                self.idle.set_shown(Some(left));
                if idle < off {
                    return true;
                }
            }
        }
        if self.idle_lock_min() > 0
            && idle >= u64::from(self.idle_lock_min()) * min
            && self.lockable()
            && self.idle.locking().is_none()
            && !held
        {
            self.idle.set_locking(Some(idle));
            // No one is there to ask: the lock goes ahead, and an open
            // vault's changes with no seed to seal them are lost, as the
            // warning before it said. It is no mode change: the sheet that
            // asked for one ends with it.
            self.next_mode.clear();
            // A write is a press: one that waited for this lock to be
            // pressed is given up.
            self.visit.set_lock_to_write(None);
            self.lock_now(true);
            return true;
        }
        if self.idle_off_min() > 0
            && idle >= u64::from(self.idle_off_min()) * min
            && !self.online
            && !self.lockable()
            && !self.flow_screen()
        {
            if self.outbox.is_empty() {
                self.exit_forgetting();
            } else if self.sheet.is_none() {
                // Powering off would lose what waits: the lock screen
                // stays, saying what it is.
                self.sheet = Some(Sheet::Locked);
                return true;
            }
        }
        false
    }
}

impl Faraday {
    /// A fresh app: no keys, empty boxes, Home.
    pub fn new() -> Self {
        Faraday {
            canvas: None,
            f: 1.0,
            w: 1280.0,
            h: 800.0,
            compact: false,
            pointer: pointer::Pointer::new(),
            out: out::Out::new(),
            clock_drawn: None,
            restart: false,
            restart_computer: false,
            last_display: None,
            settings: stick::settings::Settings::default(),
            set_time: set_time::SetTimeArea::default(),
            idle: idle::Idle::new(),
            tainted: false,
            inputs: inputs::InputsArea::default(),
            drawing: drawing::Drawing::default(),
            kept: drawing::Kept::default(),
            vaults: vault::VaultArea::default(),
            memory: memory::MemoryArea::default(),
            backups_area: backups::BackupsArea::default(),
            fresh: fresh::Fresh::default(),
            now_ms: 0,
            screen: Screen::Home,
            sheet: None,
            sheet_offset: 0.0,
            panel: compact::CompactArea::default(),
            session: Session::default(),
            decode: decode::DecodeArea::default(),
            inbox: inbox::InboxArea::default(),
            stick: stick::StickArea::default(),
            outbox: Vec::new(),
            entry: state::EntryArea::default(),
            wallets: wallet::WalletsArea::default(),
            visit: stick::visit_taken::VisitArea::default(),
            import: boot_import::BootImportArea::default(),
            pin_h: std::cell::Cell::new(0.0),
            list_offset: 0.0,
            glance: glance::GlanceArea::default(),
            scroll: motion::Scroll::default(),
            qr: qr_view::QrArea::default(),
            spend_backup: spend_backup::SpendBackupArea::default(),
            vanity: vanity::VanityArea::default(),
            backup: wallet_backup::WalletBackupArea::default(),
            message: state::MessageArea::default(),
            create: create::CreateArea::default(),
            restore: restore::RestoreArea::default(),
            keygen: keygen::KeyGenArea::default(),
            guide: guide::GuideArea::default(),
            wordlist: wordlist::WordListArea::default(),
            bip85: bip85::Bip85Area::default(),
            silent: silent::SilentArea::default(),
            explore: explore::ExploreArea::default(),
            lightning: lightning::LightningArea::default(),
            tools: tools::ToolsArea::default(),
            secrets: secrets::SecretsArea::default(),
            camera: scan_key::Camera::default(),
            #[cfg(feature = "demo")]
            transfer: transfer::TransferArea::default(),
            typing: edit::TypingArea::new(),
            trail: trail::Trail::default(),
            seed_read: seeds::SeedsArea::default(),
            later: None,
            worker: worker::Worker::new(),
            disclosed: None,
            fade: fade::Fade::default(),
            selftest: None,
            online: false,
            seed_line: false,
            held: false,
            shell_sticks: false,
            clipboard: false,
            medium: Medium::Stick,
            secure_boot: SecureBootAtStart::Unknown,
            maker: faraday_text::Text::default(),
            maker_family: faraday_text::Text::default(),
            maker_model: faraday_text::Text::default(),
            devices: devices::DeviceCheck::default(),
            next_mode: modes::ModesArea::default(),
            samples: None,
            sample_area: samples::SamplesArea::default(),
            flows: backup_flow::BackupFlowArea::default(),
            in_progress: in_progress::InProgress::default(),
        }
    }
}

// Words, parts and passphrases typed.
impl crate::at_lock::AtLock for EntryArea {
    fn holds(&self) -> crate::at_lock::Holds {
        let e = &self.state;
        crate::at_lock::Holds::typed(
            !e.typed.is_empty()
                || e.parts.holds_any()
                || e.passphrase.holds_any()
                || e.keys.as_deref().is_some_and(|k| k.holds_any()),
        )
    }
    fn lock(&mut self) {
        self.reset();
    }
}

// A message to sign and its signature are public; a lock drops them.
impl crate::at_lock::AtLock for MessageArea {
    fn holds(&self) -> crate::at_lock::Holds {
        crate::at_lock::Holds::NOTHING
    }
    fn lock(&mut self) {
        self.clear();
    }
}
