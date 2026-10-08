//! The Backup flow opened from Key detail (UX.md §7.2 "Backup quiz", D1,
//! D2, D4, D5, D6, K1): the quiz for any loaded key, with the helper
//! variant, the words screen (masked until held or until the app bar's
//! eye is on, with the wordlist numbers as an option beside them), and
//! the SeedQR and CompactSeedQR secret frames (blank until held). The
//! words themselves stay in the key's [`Mnemonic`]; the flow holds the
//! key's index and the quiz, and the QR is encoded from the mnemonic
//! when the frame is drawn.
//!
//! This file is listed in `tools/lint-secrets.sh` and may hold no heap
//! text.
//!
//! [`Mnemonic`]: osk_bip::bip39::Mnemonic

use alloc::vec::Vec;

use osk_bip::bip39::Language;
use osk_bip::keys::Fingerprint;
use osk_crypto::{Zeroize, ZeroizeOnDrop};
use osk_entropy::MAX_XOR_PARTS;
use osk_ui::widgets::keyboard::KeyInput;

use crate::create::CreateWizard;
use crate::ids::{self, Id};
use crate::load::EntryList;
use crate::pass_entry::PassEntry;
use crate::quiz::{Quiz, QuizState};
use crate::sign::Save;
use osk_entropy::{SOURCE_ROWS, Source};

/// The part counts a Seed XOR split offers, in the order the Choice
/// lists them.
pub const XOR_COUNTS: [u8; 3] = [2, 3, 4];

/// Where the flow is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupStep {
    /// The helper toggle and "Start quiz".
    QuizStart,
    /// Questions.
    Quiz,
    /// Pass or fail (skipped).
    QuizResult,
    /// The word grid, masked until it is revealed.
    Words,
    /// The SeedQR, blank until held (D4).
    SeedQr,
    /// The CompactSeedQR, blank until held (D4).
    CompactSeedQr,
    /// "Which code?": the grid draws a SeedQR or a CompactSeedQR.
    GridChoice,
    /// The seed code as a grid to copy by hand, blank until held.
    Grid,
    /// The words as a numbered steel plate takes them, blank until held.
    Steel,
    /// "How many parts?": 2, 3 or 4.
    XorCount,
    /// "Random parts from?": the same sources Create a key offers.
    XorSource,
    /// The chosen source's own steps, once per random part.
    XorGather,
    /// One Seed XOR part's words, blank until held.
    XorPart,
    /// The parts' fingerprints, once the split is made.
    XorResult,
    /// SLIP-39 shares: the passphrase, the plan, the randomness, the
    /// shares and the result, which [`crate::shares::SharePlan`] holds
    /// (`docs/PLANNING.md` §16.107 rule 5).
    Shares,
    /// Codex32 strings: the plan, the randomness, the strings with the
    /// entry that types each back, and the result, which
    /// [`crate::codex32::Codex32Plan`] holds (`docs/PLANNING.md`
    /// §16.109 rule 5).
    Codex32,
    /// The backup passphrase, typed once.
    Passphrase,
    /// The same passphrase, typed again.
    PassphraseRepeat,
    /// The encrypted backup that was made, with the two ways to take it
    /// off the device.
    Encrypted,
    /// That backup as one QR. The bytes are ciphertext, so the screen
    /// is a QR and not a secret panel.
    EncryptedQr,
}

/// The flow's state.
pub struct BackupFlow {
    key: usize,
    step: BackupStep,
    helper: bool,
    quiz: Option<Quiz>,
    page: u8,
    /// Whether each word's place in the wordlist is beside it.
    numbers: bool,
    /// Whether the words were opened from a wrong quiz answer, and the
    /// last page returns to the question (UX review 2026-09-07, §3.3).
    from_quiz: bool,
    /// The backup passphrase being typed.
    pass: PassEntry,
    /// What was typed the first time, while the second is being typed.
    first: PassEntry,
    /// Whether the two typings differed and the second is being asked
    /// for again.
    mismatch: bool,
    /// The encrypted backup, once it is made. Ciphertext, so it is a
    /// plain vector; it lives only while the result is on screen.
    backup: Vec<u8>,
    /// What became of "Save to file".
    save: Save,
    /// Whether the grid draws the CompactSeedQR rather than the SeedQR.
    compact: bool,
    /// Quadrant the grid is on, on a class that pages it.
    grid_page: u8,
    /// The Seed XOR parts, as entropy. Every part is the whole key's
    /// length, and the XOR of all of them is the key.
    parts: [[u8; 32]; MAX_XOR_PARTS],
    /// Bytes of each part, which is the key's own length: 16 for a
    /// 12-word key, 20, 24 or 28 in between, 32 for a 24-word one.
    part_len: u8,
    /// Parts the split was asked for.
    part_count: u8,
    /// The part on screen, 0-based.
    part_at: u8,
    /// Each part's fingerprint, which is what a person checks a part by
    /// later without recombining. Public data.
    part_fingerprints: [Fingerprint; MAX_XOR_PARTS],
    /// Where the split's random parts come from, which the person
    /// chooses and the result names (`docs/PLANNING.md` §16.92).
    source: Source,
    /// The Create wizard gathering the random part on screen. It holds
    /// the accumulators, so it is zeroized with the rest of the flow.
    gather: Option<CreateWizard>,
    /// Which random part that wizard is making, 0-based.
    gather_at: u8,
}

impl Zeroize for BackupFlow {
    fn zeroize(&mut self) {
        self.quiz = None;
        self.page = 0;
        self.numbers = false;
        self.from_quiz = false;
        self.pass.zeroize();
        self.first.zeroize();
        self.mismatch = false;
        self.backup.zeroize();
        self.save = Save::Idle;
        self.compact = false;
        self.grid_page = 0;
        self.parts.zeroize();
        self.part_len = 0;
        self.part_at = 0;
        self.gather = None;
        self.gather_at = 0;
    }
}

impl Drop for BackupFlow {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for BackupFlow {}

impl BackupFlow {
    /// A flow for the key at index `key`, starting at `step`.
    pub fn new(key: usize, step: BackupStep) -> Self {
        BackupFlow {
            key,
            step,
            helper: false,
            quiz: None,
            page: 0,
            numbers: false,
            from_quiz: false,
            pass: PassEntry::new(),
            first: PassEntry::new(),
            mismatch: false,
            backup: Vec::new(),
            save: Save::Idle,
            compact: false,
            grid_page: 0,
            parts: [[0u8; 32]; MAX_XOR_PARTS],
            part_len: 0,
            part_count: XOR_COUNTS[0],
            part_at: 0,
            part_fingerprints: [Fingerprint([0u8; 4]); MAX_XOR_PARTS],
            source: SOURCE_ROWS[0],
            gather: None,
            gather_at: 0,
        }
    }

    /// The index of the loaded key being backed up.
    pub fn key(&self) -> usize {
        self.key
    }

    /// The step.
    pub fn step(&self) -> BackupStep {
        self.step
    }

    /// Whether the helper variant is on.
    pub fn helper(&self) -> bool {
        self.helper
    }

    /// The quiz, while running or just finished.
    pub fn quiz(&self) -> Option<&Quiz> {
        self.quiz.as_ref()
    }

    /// Word-grid page.
    pub fn page(&self) -> u8 {
        self.page
    }

    /// Whether the words screen shows each word's wordlist number.
    pub fn numbers(&self) -> bool {
        self.numbers
    }

    /// Whether the quiz on the result step passed.
    pub fn passed(&self) -> bool {
        self.quiz
            .as_ref()
            .is_some_and(|q| q.state() == QuizState::Passed)
    }

    /// Goes one step back. `false` means leaving the flow.
    pub fn back(&mut self) -> bool {
        self.step = match self.step {
            // The words opened from a wrong answer go back to it.
            BackupStep::Words if self.from_quiz => {
                self.from_quiz = false;
                self.page = 0;
                BackupStep::Quiz
            }
            BackupStep::QuizStart
            | BackupStep::Words
            | BackupStep::SeedQr
            | BackupStep::CompactSeedQr
            | BackupStep::GridChoice
            | BackupStep::Steel
            | BackupStep::XorCount
            | BackupStep::Passphrase
            | BackupStep::Encrypted => return false,
            BackupStep::Grid => {
                self.grid_page = 0;
                BackupStep::GridChoice
            }
            // Leaving a part or the result drops every part: nothing a
            // split made is kept, and coming back makes a new one.
            BackupStep::XorPart | BackupStep::XorResult => {
                self.clear_parts();
                BackupStep::XorCount
            }
            BackupStep::XorSource => {
                self.clear_parts();
                BackupStep::XorCount
            }
            // The plans are `SharePlan`'s and `Codex32Plan`'s; the
            // caller steps them back and only leaves the flow when they
            // have nothing behind them.
            BackupStep::Shares | BackupStep::Codex32 => return false,
            // The gatherer's own steps come first, and where it has
            // none left the caller decides what stands behind it: the
            // source choice, or the part before this one.
            BackupStep::XorGather => BackupStep::XorGather,
            // The repeat goes back to the first typing, and both
            // entries start again.
            BackupStep::PassphraseRepeat => {
                self.pass.zeroize();
                self.first.zeroize();
                self.mismatch = false;
                BackupStep::Passphrase
            }
            BackupStep::EncryptedQr => BackupStep::Encrypted,
            // Leaving the questions is ending the quiz, so the chevron
            // lands on where the run stands rather than dropping it
            // silently (`docs/DESIGN.md` §4.1: the chevron is the only
            // cancel).
            BackupStep::Quiz => BackupStep::QuizResult,
            BackupStep::QuizResult => return false,
        };
        true
    }

    /// Handles a tap at time `now_ms` over the key's `indices` in `list`.
    /// Returns `true` when the flow is finished and should close.
    pub fn tap(
        &mut self,
        id: Id,
        indices: &[u16],
        list: EntryList,
        now_ms: u64,
        per_page: usize,
        grid_pages: usize,
    ) -> bool {
        match self.step {
            BackupStep::QuizStart => {
                if id == ids::QUIZ_HELPER {
                    self.helper = !self.helper;
                } else if id == ids::QUIZ_START {
                    let seed = Quiz::seed(now_ms, indices);
                    self.quiz = Some(Quiz::new(indices, list, self.helper, seed));
                    self.step = BackupStep::Quiz;
                }
            }
            BackupStep::Quiz => {
                // A wrong answer never names the word; it offers the
                // words themselves instead, so the whole backup is
                // checked (UX review 2026-09-07, §3.3).
                if id == ids::QUIZ_SHOW_WORDS {
                    if self.quiz.as_ref().is_some_and(|q| q.wrong_slot().is_some()) {
                        self.from_quiz = true;
                        self.page = 0;
                        self.step = BackupStep::Words;
                    }
                    return false;
                }
                let Some(q) = &mut self.quiz else {
                    return false;
                };
                if let Some(slot) = ids::index_in(id, ids::QUIZ_CHOICE_BASE, crate::quiz::CHOICES) {
                    q.pick(slot);
                } else if id == ids::QUIZ_CONTINUE {
                    q.confirm(indices, list);
                } else if id == ids::QUIZ_RETRY {
                    q.retry(indices, list);
                }
                if q.state() == QuizState::Passed {
                    self.step = BackupStep::QuizResult;
                }
            }
            BackupStep::QuizResult => {
                if id == ids::QUIZ_AGAIN {
                    self.quiz = None;
                    self.step = BackupStep::QuizStart;
                    return false;
                }
                return id == ids::QUIZ_DONE;
            }
            BackupStep::Words => {
                if id == ids::WORDS_NUMBERS {
                    self.numbers = !self.numbers;
                } else if id == ids::WORDS_PREV {
                    self.page = self.page.saturating_sub(1);
                } else if id == ids::WORDS_NEXT {
                    let pages = indices.len().div_ceil(per_page.max(1)).max(1) as u8;
                    if u16::from(self.page) + 1 < u16::from(pages) {
                        self.page += 1;
                    }
                } else if id == ids::QUIZ_DONE {
                    self.page = 0;
                    if self.from_quiz {
                        self.from_quiz = false;
                        self.step = BackupStep::Quiz;
                        if let Some(q) = &mut self.quiz {
                            q.retry(indices, list);
                        }
                    } else {
                        return true;
                    }
                }
            }
            BackupStep::SeedQr | BackupStep::CompactSeedQr => return id == ids::QUIZ_DONE,
            BackupStep::GridChoice => {
                if id == ids::BACKUP_GRID_SEEDQR {
                    self.compact = false;
                } else if id == ids::BACKUP_GRID_COMPACT {
                    self.compact = true;
                } else if id == ids::BACKUP_GRID_CONTINUE {
                    self.grid_page = 0;
                    self.step = BackupStep::Grid;
                }
            }
            BackupStep::Grid => {
                if id == ids::WORDS_PREV {
                    self.grid_page = self.grid_page.saturating_sub(1);
                } else if id == ids::WORDS_NEXT
                    && usize::from(self.grid_page) + 1 < grid_pages.max(1)
                {
                    self.grid_page += 1;
                }
            }
            BackupStep::Steel => {
                if id == ids::WORDS_PREV {
                    self.page = self.page.saturating_sub(1);
                } else if id == ids::WORDS_NEXT {
                    let pages = indices.len().div_ceil(per_page.max(1)).max(1) as u8;
                    if u16::from(self.page) + 1 < u16::from(pages) {
                        self.page += 1;
                    }
                } else if id == ids::QUIZ_DONE {
                    self.page = 0;
                    return true;
                }
            }
            BackupStep::XorCount => {
                if let Some(i) = ids::index_in(id, ids::XOR_COUNT_BASE, XOR_COUNTS.len()) {
                    self.part_count = XOR_COUNTS[i];
                }
                // Continue opens the source choice, which the caller
                // does (`lib.rs`).
            }
            BackupStep::XorSource => {
                if let Some(i) = ids::index_in(id, ids::XOR_SOURCE_BASE, SOURCE_ROWS.len()) {
                    self.source = SOURCE_ROWS[i];
                }
                // Continue needs the key's word count and wordlist, so
                // the caller opens the first part's entry (`lib.rs`).
            }
            // The gatherer's steps are the Create wizard's, routed to it
            // by the caller.
            BackupStep::XorGather => {}
            BackupStep::XorPart => {
                if id == ids::WORDS_PREV {
                    self.page = self.page.saturating_sub(1);
                } else if id == ids::WORDS_NEXT {
                    let pages = indices.len().div_ceil(per_page.max(1)).max(1) as u8;
                    if u16::from(self.page) + 1 < u16::from(pages) {
                        self.page += 1;
                    }
                } else if id == ids::WORDS_NUMBERS {
                    self.numbers = !self.numbers;
                } else if id == ids::CREATE_CONTINUE {
                    self.page = 0;
                    if usize::from(self.part_at) + 1 < usize::from(self.part_count) {
                        self.part_at += 1;
                    } else {
                        self.step = BackupStep::XorResult;
                    }
                }
            }
            BackupStep::XorResult => return id == ids::QUIZ_DONE,
            // The shares' screens are the plan's, driven by the caller.
            BackupStep::Shares | BackupStep::Codex32 => {}
            // The passphrase steps are the keyboard's, and what the
            // result's two actions do needs the shell, so the caller
            // handles both (`lib.rs`).
            BackupStep::Passphrase
            | BackupStep::PassphraseRepeat
            | BackupStep::Encrypted
            | BackupStep::EncryptedQr => {}
        }
        false
    }

    // ----- the grid, the steel rows and Seed XOR -----

    /// Whether the grid draws the CompactSeedQR rather than the SeedQR.
    pub fn compact(&self) -> bool {
        self.compact
    }

    /// The quadrant the grid is on.
    pub fn grid_page(&self) -> u8 {
        self.grid_page
    }

    /// Parts the split was asked for.
    pub fn part_count(&self) -> usize {
        usize::from(self.part_count)
    }

    /// The part on screen, 0-based.
    pub fn part_at(&self) -> usize {
        usize::from(self.part_at)
    }

    /// Part `i`'s entropy, while a split is on screen.
    pub fn part(&self, i: usize) -> Option<&[u8]> {
        (i < usize::from(self.part_count) && self.part_len > 0)
            .then(|| &self.parts[i][..usize::from(self.part_len)])
    }

    /// Part `i`'s fingerprint.
    pub fn part_fingerprint(&self, i: usize) -> Option<Fingerprint> {
        (i < usize::from(self.part_count) && self.part_len > 0).then(|| self.part_fingerprints[i])
    }

    /// Keeps one part of the split being made and the fingerprint a key
    /// loaded from it would have.
    pub fn set_part(&mut self, i: usize, entropy: &[u8], fingerprint: Fingerprint) {
        if i >= usize::from(self.part_count) || entropy.len() > 32 {
            return;
        }
        self.parts[i][..entropy.len()].copy_from_slice(entropy);
        self.part_fingerprints[i] = fingerprint;
        self.part_len = entropy.len() as u8;
    }

    /// Where the random parts come from.
    pub fn xor_source(&self) -> Source {
        self.source
    }

    /// "How many parts?" is confirmed: ask where the random parts come
    /// from.
    pub fn go_to_source(&mut self) {
        self.clear_parts();
        self.step = BackupStep::XorSource;
    }

    /// The wizard gathering the random part on screen.
    pub fn gather(&self) -> Option<&CreateWizard> {
        self.gather.as_ref()
    }

    /// That wizard, to act on.
    pub fn gather_mut(&mut self) -> Option<&mut CreateWizard> {
        self.gather.as_mut()
    }

    /// Which random part is being gathered, 0-based.
    pub fn gather_at(&self) -> usize {
        usize::from(self.gather_at)
    }

    /// Opens random part `at`'s entry, at the key's word `count` and
    /// wordlist, from the source the person chose.
    pub fn start_gather(
        &mut self,
        at: usize,
        count: u8,
        lang: Language,
        has_camera: bool,
        tier_d: bool,
    ) {
        let mut w = CreateWizard::gathering(self.source, count, lang, at, self.part_count());
        w.set_device(has_camera, tier_d);
        self.gather = Some(w);
        self.gather_at = at as u8;
        self.step = BackupStep::XorGather;
    }

    /// The gatherer's part is finished: its entropy is kept as that
    /// part and the wizard is dropped, which zeroizes what it held.
    /// Returns which part was kept.
    pub fn take_gathered(&mut self) -> Option<usize> {
        let g = self.gather.take()?;
        let at = usize::from(self.gather_at);
        let m = g.mnemonic()?;
        let entropy = m.entropy();
        let bytes = entropy.expose().as_bytes();
        if at + 1 >= self.part_count() || bytes.len() > 32 {
            return None;
        }
        self.parts[at][..bytes.len()].copy_from_slice(bytes);
        self.part_len = bytes.len() as u8;
        Some(at)
    }

    /// Puts a gatherer in, which a SLIP-39 split's randomness uses as a
    /// Seed XOR split's does.
    pub fn set_gather(&mut self, gather: CreateWizard) {
        self.gather = Some(gather);
    }

    /// Drops it, zeroizing what it held.
    pub fn drop_gather(&mut self) {
        self.gather = None;
    }

    /// Back out of the gatherer to the source choice, with every part
    /// gathered so far forgotten.
    pub fn leave_gather(&mut self) {
        self.gather = None;
        self.gather_at = 0;
        self.clear_parts();
        self.step = BackupStep::XorSource;
    }

    /// The split is made: show its first part.
    pub fn show_parts(&mut self) {
        self.part_at = 0;
        self.page = 0;
        self.numbers = false;
        self.step = BackupStep::XorPart;
    }

    /// Forgets every part.
    pub fn clear_parts(&mut self) {
        self.parts.zeroize();
        self.part_len = 0;
        self.part_at = 0;
        self.page = 0;
    }

    // ----- the encrypted backup -----

    /// Characters typed on the passphrase step now on screen.
    pub fn pass_len(&self) -> usize {
        self.pass.len()
    }

    /// Whether ✓ is live: the passphrase is long enough.
    pub fn pass_ready(&self) -> bool {
        self.pass.long_enough()
    }

    /// Whether the last repeat did not match the first typing.
    pub fn pass_mismatch(&self) -> bool {
        self.mismatch
    }

    /// The last typed character while it is still unmasked at `now_ms`.
    pub fn pass_visible_char(&self, now_ms: u64) -> Option<char> {
        self.pass.visible_char(now_ms)
    }

    /// When that character masks, if one is showing.
    pub fn mask_deadline(&self) -> Option<u64> {
        self.pass.mask_deadline()
    }

    /// The passphrase itself, for the one call that stretches it.
    pub fn pass_bytes(&self) -> &[u8] {
        self.pass.expose()
    }

    /// Handles the passphrase keyboard at time `now_ms`. Returns `true`
    /// when the second typing matched the first and the caller should
    /// make the backup.
    pub fn pass_key(&mut self, input: KeyInput, now_ms: u64) -> bool {
        match input {
            KeyInput::Char(c) => {
                self.pass.push(c, now_ms);
                self.mismatch = false;
            }
            KeyInput::Backspace => self.pass.pop(),
            KeyInput::Done => {
                if !self.pass.long_enough() {
                    return false;
                }
                match self.step {
                    BackupStep::Passphrase => {
                        self.first = self.pass.take();
                        self.mismatch = false;
                        self.step = BackupStep::PassphraseRepeat;
                    }
                    BackupStep::PassphraseRepeat => {
                        if self.pass.same_as(&self.first) {
                            return true;
                        }
                        // A mismatch asks for the second typing again
                        // and says so under the field.
                        self.pass.zeroize();
                        self.mismatch = true;
                    }
                    _ => {}
                }
            }
            KeyInput::Shift | KeyInput::Symbols => {}
        }
        false
    }

    /// Keeps the backup that was made and moves to its result. Both
    /// passphrase entries are wiped: what they were for is done.
    pub fn set_backup(&mut self, bytes: Vec<u8>) {
        self.pass.zeroize();
        self.first.zeroize();
        self.mismatch = false;
        self.backup = bytes;
        self.save = Save::Idle;
        self.step = BackupStep::Encrypted;
    }

    /// The encrypted backup, empty before one is made.
    pub fn backup(&self) -> &[u8] {
        &self.backup
    }

    /// What became of "Save to file".
    pub fn save(&self) -> Save {
        self.save
    }

    /// Marks a write as out, waiting for the shell's answer.
    pub fn mark_saving(&mut self) {
        self.save = Save::Waiting;
    }

    /// The shell answered: `written` says whether the bytes are on its
    /// storage.
    pub fn mark_saved(&mut self, written: bool) {
        self.save = if written { Save::Written } else { Save::Failed };
    }

    /// Shows the backup as one QR.
    pub fn show_qr(&mut self) {
        self.step = BackupStep::EncryptedQr;
    }
}
