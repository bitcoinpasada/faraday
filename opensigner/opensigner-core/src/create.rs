//! The Create wizard's state (UX.md §7.2 Create; `docs/PLANNING.md` §8.1):
//! the entropy source, word count and language, the rolls, flips or hex
//! digits typed so far, the mnemonic built from them, the backup quiz,
//! and the shared finishing steps ([`Finish`]).
//!
//! Everything secret is inline and fixed-size: the entry buffers are
//! `osk-entropy` accumulators, the words a [`Mnemonic`], the quiz's
//! candidates a small array; all are zeroized on drop, so cancelling the
//! wizard erases everything. This file is listed in
//! `tools/lint-secrets.sh` and may hold no heap text; the views in
//! `views/create.rs` receive only what the screen shows.

use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{Fingerprint, MasterKey, Network};
use osk_bip::slip39;
use osk_crypto::{Zeroize, ZeroizeOnDrop};
use osk_entropy::{
    CameraNoise, CardDraws, CoinFlips, DeviceRandom, DiceProcedure, DiceRolls, FrameStats,
    MAX_XOR_PARTS, MIN_MIX, MIN_XOR_PARTS, MIX_SOURCES, Mixed, RawHex, SOURCE_ROWS, SeedXor,
    Source, Strength, Warnings,
};
use osk_ui::geom::SizeClass;
use osk_ui::widgets::keyboard::KeyInput;

use crate::finish::{Finish, FinishStep, MASK_MS, Material};
use crate::ids::{self, Id};
use crate::load::{EntryList, LoadWizard, LoadedKey};
use crate::quiz::{Quiz, QuizState};

/// Where the wizard is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Dice, coins, hex, or a source coming later.
    Source,
    /// Which of the published dice procedures the rolls follow
    /// (`docs/PLANNING.md` §16.115). Only the dice source has it.
    Procedure,
    /// 12 or 24 words.
    Count,
    /// Which wordlist.
    Language,
    /// Rolling, flipping or typing.
    Entropy,
    /// Typing one Seed XOR part's words, and the fingerprint it comes
    /// to once they are all in.
    XorPart,
    /// Pointing the camera and taking the frames.
    Camera,
    /// What the device's own generator answered.
    Device,
    /// Which sources a mix combines.
    MixChoose,
    /// The commitments a mix combined, before the words.
    MixResult,
    /// Counts, runs and cautions.
    Sanity,
    /// Entropy → checksum → first word, as a record of secret rows.
    Math,
    /// The entropy hex on its own Secret screen.
    MathEntropy,
    /// The checksum bits on their own Secret screen.
    MathBits,
    /// The first word on its own Secret screen.
    MathWord,
    /// The words (secret panel).
    Words,
    /// The SLIP-39 plan, its randomness and its shares, which
    /// [`crate::shares::SharePlan`] holds (`docs/PLANNING.md` §16.107
    /// rule 4).
    Shares,
    /// The codex32 plan, its randomness and its strings, which
    /// [`crate::codex32::Codex32Plan`] holds (`docs/PLANNING.md`
    /// §16.109 rule 4).
    Codex32,
    /// The helper toggle and "Start quiz".
    QuizStart,
    /// The backup quiz.
    Quiz,
    /// The caution behind "Skip quiz".
    QuizSkip,
    /// "Add a passphrase?"
    PassphraseOffer,
    /// Typing the passphrase.
    Passphrase,
    /// Fingerprints without and with the passphrase.
    PassphraseConfirm,
    /// The final fingerprint and the hold to add.
    Confirm,
    /// Setting the session PIN (first key of the session only).
    Pin,
    /// Repeating the session PIN.
    PinConfirm,
}

impl Step {
    /// The shared finishing step this is, if any.
    pub fn finish(self) -> Option<FinishStep> {
        match self {
            Step::PassphraseOffer => Some(FinishStep::PassphraseOffer),
            Step::Passphrase => Some(FinishStep::Passphrase),
            Step::PassphraseConfirm => Some(FinishStep::PassphraseConfirm),
            Step::Confirm => Some(FinishStep::Confirm),
            Step::Pin => Some(FinishStep::Pin),
            Step::PinConfirm => Some(FinishStep::PinConfirm),
            _ => None,
        }
    }

    fn from_finish(f: FinishStep) -> Self {
        match f {
            FinishStep::PassphraseOffer => Step::PassphraseOffer,
            FinishStep::Passphrase => Step::Passphrase,
            FinishStep::PassphraseConfirm => Step::PassphraseConfirm,
            FinishStep::Confirm => Step::Confirm,
            FinishStep::Pin => Step::Pin,
            FinishStep::PinConfirm => Step::PinConfirm,
        }
    }
}

/// Wizard state. See the module documentation.
pub struct CreateWizard {
    step: Step,
    source: Source,
    count: u8,
    lang: Language,
    dice: DiceRolls,
    /// Which published procedure the rolls follow
    /// (`docs/PLANNING.md` §16.115).
    dice_proc: DiceProcedure,
    /// Whether the last key pressed was a face the direct-selection
    /// procedure rerolls, which the caption line under the pad says.
    reroll: bool,
    coins: CoinFlips,
    hex: RawHex,
    /// When the last roll, flip or digit was entered, for masking.
    typed_at: u64,
    /// Whether ✓ came before the entries the word count needs, which
    /// the reserved caption line under the field says (§4.3).
    too_short: bool,
    mnemonic: Option<Mnemonic>,
    quiz: Option<Quiz>,
    /// K1: someone else runs the quiz; nothing is revealed.
    helper: bool,
    backup_verified: bool,
    /// Word-grid page on `small`.
    page: u8,
    /// Whether each word's place in the wordlist is beside it.
    numbers: bool,
    /// Whether the words were opened from a wrong quiz answer, and the
    /// last page returns to the question (UX review 2026-09-07, §3.3).
    from_quiz: bool,
    /// The Seed XOR parts taken so far, accumulated by XOR.
    xor: SeedXor,
    /// The word entry the part on screen is typed into: the Load
    /// wizard's own, run once per part.
    entry: LoadWizard,
    /// Parts taken so far.
    xor_parts: u8,
    /// The fingerprint of the part on screen, once its words are in.
    xor_print: Option<Fingerprint>,
    /// Cards drawn so far.
    cards: CardDraws,
    /// The half of a card already tapped: a rank, or a suit.
    card_rank: Option<u8>,
    /// The suit already tapped, where the suit came first.
    card_suit: Option<u8>,
    /// Whether the last tap named a card already out of the deck, which
    /// the caption line under the strip says.
    card_repeat: bool,
    /// The frames taken so far.
    camera: CameraNoise,
    /// The bytes the shell answered with.
    device: DeviceRandom,
    /// The sources a mix has combined so far.
    mix: Mixed,
    /// Which of [`MIX_SOURCES`] the mix takes.
    mix_chosen: [bool; MIX_SOURCES.len()],
    /// How far through [`MIX_SOURCES`] the mix has got.
    mix_at: u8,
    /// The source whose own steps are running, while a mix runs them.
    /// `None` everywhere else, where the source is the whole of it.
    sub: Option<Source>,
    /// Whether the shell has a camera, which dims the camera row.
    has_camera: bool,
    /// Whether this is a Tier D build, which dims the device row.
    tier_d: bool,
    /// Whether the wizard is making a SLIP-39 key: the same source and
    /// entropy steps, no language, no BIP-39 math and no words, and the
    /// entropy is the master secret the shares carry
    /// (`docs/PLANNING.md` §16.107 rule 4).
    slip39: bool,
    /// Whether the wizard is making a codex32 key: the same source and
    /// entropy steps, a seed length in place of a word count, no
    /// language, no BIP-39 math, no words and no passphrase, and the
    /// entropy is the master seed the strings carry
    /// (`docs/PLANNING.md` §16.109 rule 4).
    codex32: bool,
    /// The seed length that mode was asked for.
    bits: Strength,
    /// Whether the strings have been written, so that Confirm stands
    /// after them rather than before.
    codex32_done: bool,
    /// That master secret, once the entropy is in.
    secret: [u8; osk_bip::slip39::MAX_STRENGTH_BYTES],
    secret_len: u8,
    /// Whether the shares have been dealt, so that Confirm stands after
    /// them rather than before.
    shares_done: bool,
    /// While one of a split's random values is being gathered on this
    /// wizard's own screens: the 1-based group it belongs to (0 at the
    /// group level), which value it is there and how many that level
    /// needs. It is what the title carries (§16.92's scheme).
    share_gather: Option<(u8, u8, u8)>,
    /// Whether the wizard is here to combine Seed XOR parts, which the
    /// Load wizard's own source row starts. It has no source step: the
    /// source is the row that opened it, and Back leaves for that row.
    combining: bool,
    /// Whether the wizard is here to gather one random part of a Seed
    /// XOR split rather than to make a key: the source's own steps at
    /// the key's strength, ending at [`Step::Words`] with the part's
    /// entropy in the mnemonic (`docs/PLANNING.md` §16.92).
    gathering: bool,
    /// Which part of the split is being gathered, 0-based.
    part_at: u8,
    /// How many parts the split has.
    part_total: u8,
    finish: Finish,
}

impl Zeroize for CreateWizard {
    fn zeroize(&mut self) {
        self.dice.zeroize();
        self.reroll = false;
        self.coins.zeroize();
        self.hex.zeroize();
        self.typed_at = 0;
        self.too_short = false;
        self.mnemonic = None;
        self.quiz = None;
        self.helper = false;
        self.backup_verified = false;
        self.page = 0;
        self.numbers = false;
        self.from_quiz = false;
        self.xor.zeroize();
        self.entry.zeroize();
        self.xor_parts = 0;
        self.xor_print = None;
        self.cards.zeroize();
        self.card_rank = None;
        self.card_suit = None;
        self.card_repeat = false;
        self.camera.zeroize();
        self.device.zeroize();
        self.mix.zeroize();
        self.mix_chosen = [false; MIX_SOURCES.len()];
        self.mix_at = 0;
        self.sub = None;
        self.secret.zeroize();
        self.secret_len = 0;
        self.shares_done = false;
        self.codex32_done = false;
        self.share_gather = None;
        self.finish.zeroize();
    }
}

impl Drop for CreateWizard {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for CreateWizard {}

impl Default for CreateWizard {
    fn default() -> Self {
        Self::new()
    }
}

impl CreateWizard {
    /// A wizard at its first step: dice, 12 English words.
    pub fn new() -> Self {
        CreateWizard {
            step: Step::Source,
            source: Source::Dice,
            count: 12,
            lang: Language::English,
            dice: DiceRolls::new(),
            dice_proc: DiceProcedure::Hashed,
            reroll: false,
            coins: CoinFlips::new(),
            hex: RawHex::new(),
            typed_at: 0,
            too_short: false,
            mnemonic: None,
            quiz: None,
            helper: false,
            backup_verified: false,
            page: 0,
            numbers: false,
            from_quiz: false,
            xor: SeedXor::new(),
            entry: LoadWizard::new(),
            xor_parts: 0,
            xor_print: None,
            cards: CardDraws::new(),
            card_rank: None,
            card_suit: None,
            card_repeat: false,
            camera: CameraNoise::new(),
            device: DeviceRandom::new(),
            mix: Mixed::new(),
            mix_chosen: [false; MIX_SOURCES.len()],
            mix_at: 0,
            sub: None,
            slip39: false,
            codex32: false,
            bits: Strength::Bits128,
            codex32_done: false,
            secret: [0; osk_bip::slip39::MAX_STRENGTH_BYTES],
            secret_len: 0,
            shares_done: false,
            share_gather: None,
            has_camera: true,
            tier_d: false,
            combining: false,
            gathering: false,
            part_at: 0,
            part_total: 0,
            finish: Finish::new(),
        }
    }

    /// The wizard Load › "Seed XOR parts" opens: the word count, the
    /// wordlist, then one part after another.
    pub fn combining() -> Self {
        let mut w = Self::new();
        w.source = Source::SeedXor;
        w.combining = true;
        w.step = Step::Count;
        w
    }

    /// Whether this wizard is combining Seed XOR parts.
    pub fn is_combining(&self) -> bool {
        self.combining
    }

    /// The wizard Add a key › "Create SLIP-39 shares" opens: the source
    /// Choice as Create a key's, then 20 or 33 words, no language step,
    /// and a master secret that is only ever written as shares.
    pub fn shares() -> Self {
        let mut w = Self::new();
        w.slip39 = true;
        w.count = crate::load::SLIP39_COUNTS[0];
        w
    }

    /// Whether this wizard is making a SLIP-39 key.
    pub fn is_slip39(&self) -> bool {
        self.slip39
    }

    /// The wizard Add a key › "Create Codex32 shares" opens: the source
    /// Choice as Create a key's, then "How long a seed?", no language
    /// step, and a master seed that is only ever written as codex32
    /// strings (`docs/PLANNING.md` §16.109 rule 4).
    pub fn codex32() -> Self {
        let mut w = Self::new();
        w.codex32 = true;
        w
    }

    /// Whether this wizard is making a codex32 key.
    pub fn is_codex32(&self) -> bool {
        self.codex32
    }

    /// The seed lengths "How long a seed?" offers. BIP 93 also allows
    /// 512 bits, which no entropy source here makes and which the step
    /// therefore does not offer (§16.109 rule 4).
    pub fn strengths(&self) -> &'static [Strength] {
        &Strength::ALL
    }

    /// The strings are written: Confirm stands after them, over the
    /// seed the entropy gave.
    pub fn codex32_made(&mut self, verified: bool, network: Network) -> bool {
        self.codex32_done = true;
        self.backup_verified = verified;
        let CreateWizard {
            finish,
            secret,
            secret_len,
            ..
        } = self;
        if !finish.build(
            &Material::Seed(&secret[..usize::from(*secret_len)]),
            network,
        ) {
            return false;
        }
        self.step = Step::Confirm;
        true
    }

    /// Whether the strings have been written.
    pub fn codex32_done(&self) -> bool {
        self.codex32_done
    }

    /// The word counts the count step offers, in the order it lists
    /// them.
    pub fn counts(&self) -> &'static [u8] {
        if self.slip39 {
            &crate::load::SLIP39_COUNTS
        } else {
            &osk_entropy::WORD_COUNTS
        }
    }

    /// The master secret a SLIP-39 key is, once the entropy is in.
    pub fn secret<R>(&self, f: impl FnOnce(&[u8]) -> R) -> Option<R> {
        (self.secret_len > 0).then(|| f(&self.secret[..usize::from(self.secret_len)]))
    }

    /// The shares are dealt: Confirm stands after them.
    pub fn shares_made(&mut self, verified: bool) {
        self.shares_done = true;
        self.backup_verified = verified;
        self.step = Step::Confirm;
    }

    /// Whether the shares have been dealt.
    pub fn shares_done(&self) -> bool {
        self.shares_done
    }

    /// Opens this wizard's own entropy screens for one of a split's
    /// random values, titled by where it belongs in the plan.
    pub fn start_share_gather(&mut self, group: u8, at: u8, of: u8) {
        self.clear_entry();
        self.mnemonic = None;
        self.share_gather = Some((group, at, of));
        self.gathering = true;
        self.step = self.entry_step();
    }

    /// Ends a run of gathering, whether because the values are all in or
    /// because the chevron left it.
    pub fn end_share_gather(&mut self) {
        self.clear_entry();
        self.mnemonic = None;
        self.share_gather = None;
        self.gathering = false;
    }

    /// Where in the plan the random value now on screen belongs, which
    /// is what the title carries.
    pub fn share_gather(&self) -> Option<(u8, u8, u8)> {
        self.share_gather
    }

    /// The wizard one random part of a Seed XOR split is made in: the
    /// chosen `source`'s own steps at the key's word `count` and
    /// wordlist, with no source, count or language step of its own.
    /// It ends at [`Step::Words`], where the part's entropy is the
    /// mnemonic the caller takes.
    pub fn gathering(source: Source, count: u8, lang: Language, at: usize, of: usize) -> Self {
        let mut w = Self::new();
        w.source = source;
        w.count = count;
        w.lang = lang;
        w.gathering = true;
        w.part_at = at as u8;
        w.part_total = of as u8;
        w.start_source();
        w
    }

    /// Which part of a Seed XOR split this wizard is making, 0-based,
    /// and how many parts the split has. `None` in a wizard that is
    /// making a key.
    pub fn part_of(&self) -> Option<(usize, usize)> {
        (self.gathering && self.share_gather.is_none())
            .then(|| (usize::from(self.part_at), usize::from(self.part_total)))
    }

    /// What this build can take: the camera row is dimmed where the
    /// shell has none, and the device row where the tier is D.
    pub fn set_device(&mut self, has_camera: bool, tier_d: bool) {
        self.has_camera = has_camera;
        self.tier_d = tier_d;
    }

    /// Whether `source` can be chosen on this build.
    pub fn available(&self, source: Source) -> bool {
        match source {
            Source::Camera => self.has_camera,
            Source::Device => !self.tier_d,
            _ => true,
        }
    }

    /// The source whose own step is on screen: the one being collected
    /// for a mix, or the chosen source itself.
    pub fn active(&self) -> Source {
        self.sub.unwrap_or(self.source)
    }

    /// The current step.
    pub fn step(&self) -> Step {
        self.step
    }

    /// Moves to `step` without touching any state.
    pub fn go(&mut self, step: Step) {
        self.step = step;
    }

    /// The source.
    pub fn source(&self) -> Source {
        self.source
    }

    // ----- the dice procedure (`docs/PLANNING.md` §16.115) -----

    /// The procedure the rolls on screen follow. A mix takes each
    /// source's commitment, which is SHA-256 of the rolls as ASCII
    /// whatever a key made from dice alone would do with them, so a
    /// mix's dice step is always the hashed procedure.
    pub fn procedure(&self) -> DiceProcedure {
        if self.sub.is_some() {
            return DiceProcedure::Hashed;
        }
        self.dice_proc
    }

    /// The procedure the Choice has checked.
    pub fn chosen_procedure(&self) -> DiceProcedure {
        self.dice_proc
    }

    /// Checks `procedure` on the Choice, which starts the rolls over: a
    /// run read one way names nothing when read another.
    pub fn set_procedure(&mut self, procedure: DiceProcedure) {
        if procedure != self.dice_proc && self.procedure_available(procedure) {
            self.dice_proc = procedure;
            self.dice.clear();
            self.reroll = false;
        }
    }

    /// Whether a procedure can be taken here. Direct selection names
    /// BIP-39 words, which a SLIP-39 master secret and a codex32 master
    /// seed are not spelled in.
    pub fn procedure_available(&self, procedure: DiceProcedure) -> bool {
        !procedure.direct() || !(self.slip39 || self.codex32)
    }

    /// Rolls `procedure` needs at the chosen length, which is the second
    /// line of its row.
    pub fn procedure_rolls(&self, procedure: DiceProcedure) -> usize {
        procedure.needed(self.strength())
    }

    /// Whether the last key pressed was rerolled (§4.3's caption line).
    pub fn rerolled(&self) -> bool {
        self.reroll
    }

    /// The words the rolls have named so far under direct selection.
    pub fn dice_word_indices(&self) -> impl Iterator<Item = u16> + '_ {
        self.dice.word_indices()
    }

    /// How many words direct selection rolls: every word of the key.
    pub fn dice_words_needed(&self) -> usize {
        self.strength().words()
    }

    /// Word count: a BIP-39 length, or a SLIP-39 share's 20 or 33.
    pub fn count(&self) -> u8 {
        self.count
    }

    /// The strength for the word count. A SLIP-39 share of 20 words
    /// carries a 128-bit master secret and one of 33 a 256-bit one.
    pub fn strength(&self) -> Strength {
        if self.codex32 {
            return self.bits;
        }
        if self.slip39 {
            return if usize::from(self.count) >= osk_bip::slip39::MAX_WORDS {
                Strength::Bits256
            } else {
                Strength::Bits128
            };
        }
        Strength::for_words(usize::from(self.count)).unwrap_or(Strength::Bits128)
    }

    /// Wordlist.
    pub fn language(&self) -> Language {
        self.lang
    }

    /// Sets the wordlist.
    pub fn set_language(&mut self, lang: Language) {
        self.lang = lang;
    }

    /// Whether the quiz will be run with a helper holding the device.
    pub fn helper(&self) -> bool {
        self.helper
    }

    /// Whether the entry step's reserved caption line says the entries
    /// are short of the count (§4.3).
    pub fn entry_too_short(&self) -> bool {
        self.too_short
    }

    /// The shared finishing state.
    pub fn finish(&self) -> &Finish {
        &self.finish
    }

    /// Whether the quiz was passed (as opposed to skipped).
    pub fn backup_verified(&self) -> bool {
        self.backup_verified
    }

    /// Goes one step back. Returns `false` at the first step, where going
    /// back means cancelling the wizard. State that belongs to the step
    /// being left is zeroized.
    pub fn back(&mut self) -> bool {
        self.step = match self.step {
            Step::Source => return false,
            // The count is the first step of a combine, and the row
            // that opened it is what Back goes to.
            Step::Count if self.combining => return false,
            Step::Count => Step::Source,
            Step::Language => Step::Count,
            Step::Procedure if self.slip39 || self.codex32 => Step::Count,
            Step::Procedure => Step::Language,
            Step::Entropy if self.sub.is_some() => {
                self.clear_entry();
                self.clear_mix();
                Step::MixChoose
            }
            // The dice source's rolls stand behind the procedure that
            // reads them (`docs/PLANNING.md` §16.115).
            Step::Entropy if self.source == Source::Dice && !self.gathering => {
                self.clear_entry();
                Step::Procedure
            }
            // A gathered part has no step of its own behind its
            // entry: Back leaves the gatherer, and the split decides
            // what stands behind it.
            Step::Entropy if self.gathering => {
                self.clear_entry();
                return false;
            }
            Step::Entropy if self.slip39 || self.codex32 => {
                self.clear_entry();
                Step::Count
            }
            Step::Entropy => {
                self.clear_entry();
                Step::Language
            }
            // Leaving a part drops every part: nothing half-combined
            // is kept, and coming back starts the run again.
            Step::XorPart => {
                self.clear_parts();
                Step::Language
            }
            Step::MixChoose if self.gathering => {
                self.clear_mix();
                return false;
            }
            Step::MixChoose if self.slip39 || self.codex32 => {
                self.clear_mix();
                Step::Count
            }
            Step::MixChoose => {
                self.clear_mix();
                Step::Language
            }
            // Leaving a mix's step drops the whole mix: a commitment
            // half of whose sources were thrown away commits to
            // nothing.
            Step::Camera | Step::Device if self.sub.is_some() => {
                self.clear_entry();
                self.clear_mix();
                Step::MixChoose
            }
            Step::Camera | Step::Device if self.gathering => {
                self.clear_entry();
                return false;
            }
            Step::Camera | Step::Device if self.slip39 || self.codex32 => {
                self.clear_entry();
                Step::Count
            }
            Step::Camera | Step::Device => {
                self.clear_entry();
                Step::Language
            }
            Step::MixResult => {
                self.mnemonic = None;
                self.clear_mix();
                Step::MixChoose
            }
            Step::Sanity => {
                self.mnemonic = None;
                self.secret.zeroize();
                self.secret_len = 0;
                self.entry_step()
            }
            Step::Math => Step::Sanity,
            Step::MathEntropy | Step::MathBits | Step::MathWord => Step::Math,
            // The words opened from a wrong answer go back to it.
            Step::Words if self.from_quiz => {
                self.from_quiz = false;
                self.page = 0;
                Step::Quiz
            }
            Step::Words if self.source == Source::Mix => {
                self.page = 0;
                Step::MixResult
            }
            Step::Words if self.source == Source::Device => {
                self.page = 0;
                Step::Device
            }
            // The plan is `SharePlan`'s; the caller steps it back and
            // only asks the wizard when the plan has nothing behind it.
            Step::Shares => {
                self.shares_done = false;
                match self.finish.back(FinishStep::Confirm) {
                    Some(f) => Step::from_finish(f),
                    None => Step::PassphraseOffer,
                }
            }
            // The plan is `Codex32Plan`'s; the caller steps it back and
            // only asks the wizard when the plan has nothing behind it.
            Step::Codex32 => {
                self.codex32_done = false;
                match self.source {
                    Source::Device => Step::Device,
                    Source::Mix => Step::MixResult,
                    _ => Step::Sanity,
                }
            }
            // Confirm stands after the shares for a SLIP-39 key.
            Step::Confirm if self.slip39 && self.shares_done => Step::Shares,
            // And after the strings for a codex32 key.
            Step::Confirm if self.codex32 && self.codex32_done => Step::Codex32,
            Step::Words => {
                self.page = 0;
                Step::Sanity
            }
            Step::QuizStart => Step::Words,
            Step::Quiz => {
                self.quiz = None;
                self.backup_verified = false;
                Step::QuizStart
            }
            Step::QuizSkip => Step::QuizStart,
            other => match self.finish.back(other.finish().expect("a finishing step")) {
                Some(f) => Step::from_finish(f),
                None => {
                    self.backup_verified = false;
                    self.quiz = None;
                    Step::QuizStart
                }
            },
        };
        true
    }

    // ----- taps and keys -----

    /// Handles a tap at time `now_ms` for `network`. Returns whether the
    /// key should be taken (the hold on the last step is handled by the
    /// caller).
    pub fn tap(&mut self, id: Id, network: Network, now_ms: u64, per_page: usize) {
        match self.step {
            // A choice is checked by the tap and confirmed by Continue
            // (`docs/DESIGN.md` §2.7).
            Step::Source => {
                if let Some(i) = ids::index_in(id, ids::CREATE_SOURCE_BASE, SOURCE_ROWS.len())
                    && self.available(SOURCE_ROWS[i])
                {
                    self.source = SOURCE_ROWS[i];
                } else if id == ids::CREATE_SOURCE_CONTINUE {
                    self.step = Step::Count;
                }
            }
            Step::Count if self.codex32 => {
                if let Some(i) = ids::index_in(id, ids::CREATE_COUNT_BASE, Strength::ALL.len()) {
                    self.bits = Strength::ALL[i];
                } else if id == ids::CREATE_COUNT_CONTINUE {
                    // A codex32 string has no words, so there is no
                    // wordlist to choose.
                    self.start_source();
                }
            }
            Step::Count => {
                let counts = self.counts();
                if let Some(i) = ids::index_in(id, ids::CREATE_COUNT_BASE, counts.len())
                    && (self.slip39 || Strength::for_words(usize::from(counts[i])).is_some())
                {
                    self.count = counts[i];
                } else if id == ids::CREATE_COUNT_CONTINUE {
                    // A SLIP-39 share is English on the Latin keyboard,
                    // so there is no language to choose.
                    if self.slip39 {
                        self.start_source();
                    } else {
                        self.step = Step::Language;
                    }
                }
            }
            Step::Language => {
                if let Some(i) = ids::index_in(id, ids::CREATE_LANG_BASE, Language::ALL.len()) {
                    self.lang = Language::ALL[i];
                } else if id == ids::CREATE_LANG_CONTINUE {
                    self.start_source();
                }
            }
            Step::Procedure => {
                if let Some(i) =
                    ids::index_in(id, ids::CREATE_PROCEDURE_BASE, DiceProcedure::ALL.len())
                {
                    self.set_procedure(DiceProcedure::ALL[i]);
                } else if id == ids::CREATE_PROCEDURE_CONTINUE {
                    self.step = self.entry_step();
                }
            }
            Step::Entropy | Step::Camera => {
                if id == ids::CREATE_CONTINUE {
                    self.continue_from_entry();
                }
            }
            Step::Device => {
                if id == ids::CREATE_CONTINUE {
                    self.continue_from_entry();
                }
            }
            Step::MixChoose => self.tap_mix(id),
            Step::MixResult => {
                if id == ids::CREATE_CONTINUE {
                    self.step = if self.codex32 && self.share_gather.is_none() {
                        Step::Codex32
                    } else {
                        Step::Words
                    };
                }
            }
            Step::XorPart => self.tap_part(id, network),
            Step::Sanity => {
                if id == ids::CREATE_CONTINUE {
                    self.step = self.after_sanity();
                } else if id == ids::CREATE_AGAIN {
                    self.clear_entry();
                    self.mnemonic = None;
                    self.step = self.entry_step();
                } else if id == ids::CREATE_MATH && !self.gathering && !self.slip39 {
                    self.step = Step::Math;
                }
            }
            Step::Math => {
                if id == ids::CREATE_CONTINUE {
                    self.step = Step::Sanity;
                } else if id == ids::CREATE_MATH_ENTROPY {
                    self.step = Step::MathEntropy;
                } else if id == ids::CREATE_MATH_BITS {
                    self.step = Step::MathBits;
                } else if id == ids::CREATE_MATH_WORD {
                    self.step = Step::MathWord;
                }
            }
            // The Secret screens the math opens have one way out: the
            // app bar's chevron.
            Step::MathEntropy | Step::MathBits | Step::MathWord => {}
            Step::Words => {
                if id == ids::WORDS_NUMBERS {
                    self.numbers = !self.numbers;
                } else if id == ids::WORDS_PREV {
                    self.page = self.page.saturating_sub(1);
                } else if id == ids::WORDS_NEXT {
                    if u16::from(self.page) + 1 < u16::from(self.word_pages(per_page)) {
                        self.page += 1;
                    }
                } else if id == ids::CREATE_CONTINUE {
                    self.page = 0;
                    if self.from_quiz {
                        self.return_to_quiz();
                    } else {
                        self.step = Step::QuizStart;
                    }
                }
            }
            Step::QuizStart => {
                if id == ids::QUIZ_HELPER {
                    self.helper = !self.helper;
                } else if id == ids::QUIZ_START {
                    self.start_quiz(now_ms);
                } else if id == ids::QUIZ_SKIP {
                    self.step = Step::QuizSkip;
                }
            }
            Step::QuizSkip => {
                if id == ids::QUIZ_SKIP_CANCEL {
                    self.step = Step::QuizStart;
                } else if id == ids::QUIZ_SKIP_CONFIRM {
                    self.backup_verified = false;
                    self.quiz = None;
                    self.step = Step::PassphraseOffer;
                }
            }
            Step::Quiz => self.tap_quiz(id),
            // The plan, the randomness and the shares or strings are
            // `crate::shares::SharePlan`'s and
            // `crate::codex32::Codex32Plan`'s, driven by the caller.
            Step::Shares | Step::Codex32 => {}
            other => {
                let step = other.finish().expect("a finishing step");
                let next = if self.codex32 {
                    let CreateWizard {
                        finish,
                        secret,
                        secret_len,
                        ..
                    } = self;
                    let material = Material::Seed(&secret[..usize::from(*secret_len)]);
                    finish.tap(step, id, &material, network)
                } else if self.slip39 {
                    let CreateWizard {
                        finish,
                        secret,
                        secret_len,
                        ..
                    } = self;
                    let material = Material::Secret(&secret[..usize::from(*secret_len)]);
                    finish.tap(step, id, &material, network)
                } else if let Some(m) = &self.mnemonic {
                    self.finish.tap(step, id, &Material::Words(m), network)
                } else {
                    None
                };
                if let Some(next) = next {
                    self.step = self.after_finish(next);
                }
            }
        }
    }

    /// Handles keyboard input at time `now_ms` for `network`: the entry
    /// pad on the entropy step, the passphrase keyboard on its step.
    pub fn key(&mut self, id: Id, input: KeyInput, network: Network, now_ms: u64) {
        if id == ids::LOAD_KEYBOARD && self.step == Step::XorPart {
            self.entry.key(id, input, network, now_ms);
            self.part_landed(network);
            return;
        }
        if id == ids::CREATE_PAD && self.step == Step::Entropy && self.active() == Source::Cards {
            match input {
                KeyInput::Char(c) => {
                    self.card_key(c, now_ms);
                }
                KeyInput::Backspace => self.entry_pop(),
                KeyInput::Done => self.continue_from_entry(),
                KeyInput::Shift | KeyInput::Symbols => {}
            }
            return;
        }
        if id == ids::CREATE_PAD && self.step == Step::Entropy {
            match input {
                KeyInput::Char(c) => {
                    self.entry_push(c, now_ms);
                }
                KeyInput::Backspace => self.entry_pop(),
                KeyInput::Done => self.continue_from_entry(),
                KeyInput::Shift | KeyInput::Symbols => {}
            }
        } else if id == ids::LOAD_PASS_KEYBOARD
            && let Some(step) = self.step.finish()
        {
            let next = if self.codex32 {
                let CreateWizard {
                    finish,
                    secret,
                    secret_len,
                    ..
                } = self;
                let material = Material::Seed(&secret[..usize::from(*secret_len)]);
                finish.key(step, input, &material, network, now_ms)
            } else if self.slip39 {
                let CreateWizard {
                    finish,
                    secret,
                    secret_len,
                    ..
                } = self;
                let material = Material::Secret(&secret[..usize::from(*secret_len)]);
                finish.key(step, input, &material, network, now_ms)
            } else if let Some(m) = &self.mnemonic {
                self.finish
                    .key(step, input, &Material::Words(m), network, now_ms)
            } else {
                None
            };
            if let Some(next) = next {
                self.step = self.after_finish(next);
            }
        } else if id == ids::LOAD_PIN_KEYBOARD
            && let Some(step) = self.step.finish()
            && let Some(next) = self.finish.pin_key(step, input)
        {
            self.step = Step::from_finish(next);
        }
    }

    /// The session PIN, once typed twice the same on the PIN steps.
    pub fn take_pin(&mut self) -> Option<crate::session::PinEntry> {
        self.finish.take_pin()
    }

    /// When the last typed roll, flip, digit or passphrase character
    /// masks, if one is showing.
    pub fn mask_deadline(&self) -> Option<u64> {
        match self.step {
            Step::Entropy => (self.entry_len() > 0).then_some(self.typed_at + MASK_MS),
            Step::Passphrase => self.finish.mask_deadline(),
            _ => None,
        }
    }

    // ----- entropy entry -----

    /// What stands after the sanity step: the words, or, for a SLIP-39
    /// key, the passphrase the shares will be written under.
    fn after_sanity(&self) -> Step {
        if self.share_gather.is_some() {
            return Step::Words;
        }
        if self.codex32 {
            // BIP 93 has no passphrase, so the plan stands where the
            // passphrase steps do in every other flow.
            return Step::Codex32;
        }
        if self.slip39 {
            return Step::PassphraseOffer;
        }
        Step::Words
    }

    /// Where a finishing step leads. A SLIP-39 key's shares are made
    /// between the passphrase and Confirm, so the first time the
    /// finishing steps reach Confirm they reach the plan instead.
    fn after_finish(&self, next: FinishStep) -> Step {
        if next == FinishStep::Confirm && self.slip39 && !self.shares_done {
            return Step::Shares;
        }
        Step::from_finish(next)
    }

    /// The step the active source collects on.
    fn entry_step(&self) -> Step {
        match self.active() {
            Source::Camera => Step::Camera,
            Source::Device => Step::Device,
            _ => Step::Entropy,
        }
    }

    /// Leaves the language step for the source's own first step.
    fn start_source(&mut self) {
        self.step = match self.source {
            Source::SeedXor => {
                self.start_part();
                Step::XorPart
            }
            Source::Mix => Step::MixChoose,
            // A gathered part is one value of a split and has no
            // procedure step: its rolls are hashed, as §16.92's rule
            // takes them.
            Source::Dice if !self.gathering => Step::Procedure,
            _ => self.entry_step(),
        };
    }

    /// Adds a roll (`'1'`–`'6'`), a flip (`'H'`/`'T'`) or a hex digit,
    /// depending on the source, at time `now_ms`.
    pub fn entry_push(&mut self, c: char, now_ms: u64) -> bool {
        let ok = match self.active() {
            Source::Dice => {
                let procedure = self.procedure();
                let face = c.to_digit(10).filter(|d| (1..=6).contains(d));
                // §4.3's caption line: a 5 or a 6 in a word's first five
                // places is rolled again under direct selection, and the
                // pad says so rather than swallowing the key.
                self.reroll = face.is_some_and(|d| !procedure.accepts(self.dice.len(), d as u8));
                face.is_some_and(|d| self.dice.push_under(d as u8, procedure))
            }
            Source::Coins => match c.to_ascii_uppercase() {
                'H' => self.coins.push(true),
                'T' => self.coins.push(false),
                _ => false,
            },
            Source::Hex => self.hex.len() < self.entry_needed() && self.hex.push(c),
            // Cards are two taps, not one; the camera and the device
            // are not typed at all.
            _ => false,
        };
        if ok {
            self.typed_at = now_ms;
            self.too_short = false;
        }
        ok
    }

    /// A key of the card pad: a rank or a suit. The card lands once
    /// both halves are in, in whichever order they were tapped.
    pub fn card_key(&mut self, c: char, now_ms: u64) -> bool {
        self.too_short = false;
        if let Some(suit) = osk_entropy::SUITS.iter().position(|&s| s == c) {
            self.card_suit = Some(suit as u8);
        } else if let Some(rank) = osk_entropy::RANKS
            .iter()
            .position(|&r| r == c.to_ascii_uppercase())
        {
            self.card_rank = Some(rank as u8);
        } else {
            return false;
        }
        let (Some(rank), Some(suit)) = (self.card_rank, self.card_suit) else {
            self.card_repeat = false;
            return true;
        };
        self.card_rank = None;
        self.card_suit = None;
        let Some(index) = osk_entropy::card_index(rank, suit) else {
            return false;
        };
        // The deck tracker: a card already out of this deck is refused,
        // and the caption line under the strip says so (§4.3).
        self.card_repeat = !self.cards.push(index);
        if !self.card_repeat {
            self.typed_at = now_ms;
        }
        true
    }

    /// The rank tapped and not yet paired with a suit.
    pub fn card_rank(&self) -> Option<u8> {
        self.card_rank
    }

    /// The suit tapped and not yet paired with a rank.
    pub fn card_suit(&self) -> Option<u8> {
        self.card_suit
    }

    /// Whether the last card named was already out of the deck.
    pub fn card_repeat(&self) -> bool {
        self.card_repeat
    }

    /// The cards drawn, for the strip and the sanity table.
    pub fn cards(&self) -> &CardDraws {
        &self.cards
    }

    /// The frames taken, for the sanity table.
    pub fn camera(&self) -> &CameraNoise {
        &self.camera
    }

    /// The last frame the shell delivered, taken or refused.
    pub fn frame_stats(&self) -> Option<FrameStats> {
        self.camera.last()
    }

    /// Whether the camera step is waiting for the device's answer to
    /// its own entropy request.
    pub fn device_ready(&self) -> bool {
        self.device.is_enough(self.strength())
    }

    /// The shell answered the entropy request.
    pub fn set_device_entropy(&mut self, bytes: &[u8; 32]) {
        self.device.set(bytes);
    }

    /// Whether the camera should be running: the camera step of this
    /// wizard, and nothing else.
    pub fn wants_camera(&self) -> bool {
        self.step == Step::Camera
    }

    /// Takes one camera frame's luma. Returns whether it was taken; a
    /// frame with too little variation is refused and the caption line
    /// says so.
    pub fn take_frame(&mut self, luma: &[u8], now_ms: u64) -> bool {
        if self.step != Step::Camera {
            return false;
        }
        self.too_short = false;
        let (_, taken) = self.camera.push(luma);
        if taken {
            self.typed_at = now_ms;
            // The shutter is the only action here, so the last frame is
            // the way on: there is nothing to confirm about a count the
            // person reached one picture at a time.
            if self.entry_ready() {
                self.continue_from_entry();
            }
        }
        taken
    }

    /// Removes the last entry.
    pub fn entry_pop(&mut self) {
        self.too_short = false;
        match self.active() {
            Source::Dice => {
                self.reroll = false;
                self.dice.pop();
            }
            Source::Coins => {
                self.coins.pop();
            }
            Source::Hex => {
                self.hex.pop();
            }
            // The half-typed card goes first, then the last whole one.
            Source::Cards => {
                self.card_repeat = false;
                if self.card_rank.take().is_some() || self.card_suit.take().is_some() {
                    return;
                }
                self.cards.pop();
            }
            _ => {}
        }
    }

    fn clear_entry(&mut self) {
        self.dice.clear();
        self.reroll = false;
        self.coins.clear();
        self.hex.clear();
        self.cards.clear();
        self.camera.clear();
        self.device.clear();
        self.card_rank = None;
        self.card_suit = None;
        self.card_repeat = false;
        self.typed_at = 0;
        self.too_short = false;
    }

    /// Entries so far.
    pub fn entry_len(&self) -> usize {
        match self.active() {
            Source::Dice => self.dice.len(),
            Source::Coins => self.coins.len(),
            Source::Hex => self.hex.len(),
            Source::Cards => self.cards.len(),
            Source::Camera => self.camera.len(),
            _ => 0,
        }
    }

    /// Entries needed for the word count.
    pub fn entry_needed(&self) -> usize {
        let s = self.strength();
        match self.active() {
            Source::Dice => self.procedure().needed(s),
            Source::Coins => CoinFlips::needed(s),
            Source::Hex => s.hex_digits(),
            Source::Cards => CardDraws::needed(s),
            Source::Camera => CameraNoise::needed(s),
            _ => 0,
        }
    }

    /// Bits gathered so far (four per hex digit).
    pub fn entry_bits(&self) -> f32 {
        match self.active() {
            // Direct selection is worth eleven bits a completed word,
            // not log2 6 a roll: the rerolled faces carry nothing. The
            // last word's low bits become the checksum, so the count
            // stops at the strength.
            Source::Dice if self.procedure().direct() => {
                (self.dice.word_count() * 11).min(self.strength().bits()) as f32
            }
            Source::Dice => self.dice.bits_collected(),
            Source::Coins => self.coins.bits_collected(),
            Source::Hex => self.hex.len() as f32 * 4.0,
            Source::Cards => self.cards.bits_collected(),
            _ => 0.0,
        }
    }

    /// Whether the entry step can continue.
    pub fn entry_ready(&self) -> bool {
        let s = self.strength();
        match self.active() {
            Source::Dice => self.dice.is_enough_under(s, self.procedure()),
            Source::Coins => self.coins.is_enough(s),
            Source::Hex => self.hex.len() == self.entry_needed(),
            Source::Cards => self.cards.is_enough(s),
            Source::Camera => self.camera.is_enough(s),
            Source::Device => self.device.is_enough(s),
            _ => false,
        }
    }

    /// The entries as characters: digits, `H`/`T`, or hex digits.
    pub fn entry_chars(&self) -> impl Iterator<Item = char> + '_ {
        let dice = self.dice.rolls().iter().map(|&r| (b'0' + r) as char);
        let coins = self.coins.flips().map(|h| if h { 'H' } else { 'T' });
        let hex = self.hex.digits().chars();
        let source = self.active();
        dice.filter(move |_| source == Source::Dice)
            .chain(coins.filter(move |_| source == Source::Coins))
            .chain(hex.filter(move |_| source == Source::Hex))
    }

    /// The last entry while it is still unmasked at `now_ms`.
    pub fn entry_visible_last(&self, now_ms: u64) -> Option<char> {
        if self.entry_len() == 0 || now_ms >= self.typed_at + MASK_MS {
            return None;
        }
        self.entry_chars().last()
    }

    /// Whether the newest entry is still unmasked at `now_ms`, which is
    /// what the strip flashes.
    pub fn entry_fresh(&self, now_ms: u64) -> bool {
        self.entry_len() > 0 && now_ms < self.typed_at + MASK_MS
    }

    /// The dice accumulator, for the sanity table.
    pub fn dice(&self) -> &DiceRolls {
        &self.dice
    }

    /// The coin accumulator, for the sanity table.
    pub fn coins(&self) -> &CoinFlips {
        &self.coins
    }

    /// The sanity flags for the source (none for hex).
    pub fn warnings(&self) -> Warnings {
        let s = self.strength();
        match self.active() {
            Source::Dice => self.dice.warnings_under(s, self.procedure()),
            Source::Coins => self.coins.warnings(s),
            Source::Cards => self.cards.warnings(s),
            Source::Camera => self.camera.warnings(s),
            _ => Warnings {
                too_few: !self.entry_ready(),
                ..Warnings::default()
            },
        }
    }

    /// SHA-256 of the active source's raw input, which is what a mix
    /// commits to and what the mixed result lists.
    fn commitment(&self) -> [u8; 32] {
        match self.active() {
            Source::Coins => self.coins.commitment(),
            Source::Hex => self.hex.commitment(),
            Source::Cards => self.cards.commitment(),
            Source::Camera => self.camera.commitment(),
            Source::Device => self.device.commitment(),
            _ => self.dice.commitment(),
        }
    }

    /// Builds the mnemonic from the entries and moves to the sanity step.
    fn continue_from_entry(&mut self) {
        if !self.entry_ready() {
            // §4.3: the caption line under the field says why ✓ did
            // nothing.
            self.too_short = true;
            return;
        }
        self.too_short = false;
        // Inside a mix the source's own entropy is never made: only its
        // commitment, which the combination is taken over.
        if self.sub.is_some() {
            self.mix_take();
            return;
        }
        let strength = self.strength();
        let entropy = match self.source {
            Source::Dice => self.dice.entropy_under(strength, self.procedure()),
            Source::Coins => self.coins.entropy(strength),
            Source::Hex => self.hex.entropy(),
            Source::Cards => self.cards.entropy(strength),
            Source::Camera => self.camera.entropy(strength),
            Source::Device => self.device.entropy(strength),
            Source::SeedXor | Source::Mix => return,
        };
        let Ok(entropy) = entropy else {
            return;
        };
        // A codex32 key is its master seed: the entropy is kept as it
        // is and never spelled into words.
        if self.codex32 && self.share_gather.is_none() {
            let bytes = entropy.as_bytes();
            if !osk_bip::codex32::SEED_LENGTHS.contains(&bytes.len()) {
                return;
            }
            self.secret.zeroize();
            self.secret[..bytes.len()].copy_from_slice(bytes);
            self.secret_len = bytes.len() as u8;
            self.step = match self.source {
                Source::Device => Step::Codex32,
                _ => Step::Sanity,
            };
            return;
        }
        // A SLIP-39 key is its master secret: the entropy is kept as it
        // is and never spelled into words.
        if self.slip39 && self.share_gather.is_none() {
            let bytes = entropy.as_bytes();
            if slip39::SecretBytes::new(bytes).is_err() {
                return;
            }
            self.secret.zeroize();
            self.secret[..bytes.len()].copy_from_slice(bytes);
            self.secret_len = bytes.len() as u8;
            self.step = match self.source {
                Source::Device => Step::PassphraseOffer,
                _ => Step::Sanity,
            };
            return;
        }
        let Ok(m) = Mnemonic::from_entropy(self.lang, entropy.as_bytes()) else {
            return;
        };
        self.mnemonic = Some(m);
        // The device's own result is its sanity statement: there is no
        // count to check and no statistic that would say anything.
        self.step = match self.source {
            Source::Device => Step::Words,
            _ => Step::Sanity,
        };
    }

    /// The mnemonic, once built.
    pub fn mnemonic(&self) -> Option<&Mnemonic> {
        self.mnemonic.as_ref()
    }

    // ----- mixing (`docs/PLANNING.md` §8.1 item 5) -----

    /// Whether row `i` of [`MIX_SOURCES`] is checked.
    pub fn mix_chosen(&self, i: usize) -> bool {
        self.mix_chosen.get(i).copied().unwrap_or(false)
    }

    /// How many sources the mix takes.
    pub fn mix_count(&self) -> usize {
        self.mix_chosen.iter().filter(|c| **c).count()
    }

    /// The commitments the mix has taken so far, in order.
    pub fn mix_commitments(&self) -> &[[u8; 32]] {
        self.mix.commitments()
    }

    /// The sources those commitments came from, in the same order.
    pub fn mix_taken(&self) -> impl Iterator<Item = Source> + '_ {
        MIX_SOURCES
            .iter()
            .enumerate()
            .filter(|(i, _)| self.mix_chosen(*i))
            .map(|(_, s)| *s)
            .take(self.mix.len())
    }

    fn tap_mix(&mut self, id: Id) {
        if let Some(i) = ids::index_in(id, ids::CREATE_MIX_BASE, MIX_SOURCES.len()) {
            if self.available(MIX_SOURCES[i]) {
                self.mix_chosen[i] = !self.mix_chosen[i];
                self.too_short = false;
            }
        } else if id == ids::CREATE_MIX_CONTINUE {
            // Continue is dimmed below two sources (§4.3), so this is
            // only ever reached with enough of them.
            if self.mix_count() < MIN_MIX {
                return;
            }
            self.too_short = false;
            self.mix.clear();
            self.mix_at = 0;
            self.mix_next();
        }
    }

    /// The source on screen is finished: its commitment joins the mix
    /// and the next chosen source's step opens.
    fn mix_take(&mut self) {
        let c = self.commitment();
        if !self.mix.push(c) {
            return;
        }
        self.clear_entry();
        self.mix_at += 1;
        self.mix_next();
    }

    /// Opens the next chosen source's step, or combines what is in hand.
    fn mix_next(&mut self) {
        while usize::from(self.mix_at) < MIX_SOURCES.len() {
            let i = usize::from(self.mix_at);
            if self.mix_chosen[i] {
                self.sub = Some(MIX_SOURCES[i]);
                self.step = self.entry_step();
                return;
            }
            self.mix_at += 1;
        }
        self.sub = None;
        self.combine_mix();
    }

    /// SHA-256 over the commitments in the order the result lists them.
    fn combine_mix(&mut self) {
        let Ok(entropy) = self.mix.entropy(self.strength()) else {
            return;
        };
        if self.codex32 && self.share_gather.is_none() {
            let bytes = entropy.as_bytes();
            if !osk_bip::codex32::SEED_LENGTHS.contains(&bytes.len()) {
                return;
            }
            self.secret.zeroize();
            self.secret[..bytes.len()].copy_from_slice(bytes);
            self.secret_len = bytes.len() as u8;
            self.step = Step::MixResult;
            return;
        }
        let Ok(m) = Mnemonic::from_entropy(self.lang, entropy.as_bytes()) else {
            return;
        };
        self.mnemonic = Some(m);
        self.step = Step::MixResult;
    }

    fn clear_mix(&mut self) {
        self.mix.clear();
        self.mix_at = 0;
        self.sub = None;
        self.clear_entry();
    }

    // ----- Seed XOR parts (`docs/PLANNING.md` §8.2 item 14) -----

    /// The word entry the part on screen is typed into.
    pub fn entry(&self) -> &LoadWizard {
        &self.entry
    }

    /// §16.50: the class decides whether a candidate takes one tap or
    /// two, and the part entry types under the same rule the Load
    /// wizard's does.
    pub fn set_two_tap(&mut self, two_tap: bool) {
        self.entry.set_two_tap(two_tap);
    }

    /// §16.123 rule 1: a tap on a candidate of the part entry's strip
    /// takes the word there, and the part's last word lands as one
    /// typed with Enter does.
    pub fn candidate(&mut self, class: SizeClass, second_row: bool, n: u8, network: Network) {
        if self.step != Step::XorPart {
            return;
        }
        crate::candidate_tap(&mut self.entry, class, second_row, n);
        self.part_landed(network);
    }

    /// Parts taken so far, not counting the one on screen.
    pub fn xor_parts(&self) -> usize {
        usize::from(self.xor_parts)
    }

    /// The fingerprint of the part on screen, once its words are in.
    pub fn xor_print(&self) -> Option<Fingerprint> {
        self.xor_print
    }

    /// Whether one more part can be typed after this one.
    pub fn xor_room(&self) -> bool {
        usize::from(self.xor_parts) + 1 < MAX_XOR_PARTS
    }

    /// Whether the parts in hand are enough to combine.
    pub fn xor_ready(&self) -> bool {
        usize::from(self.xor_parts) + 1 >= MIN_XOR_PARTS
    }

    /// One Seed XOR part read from a seed code: the part's words go
    /// into the entry whole, as a typed part's do once the last word
    /// lands, and the fingerprint the screen names is worked out from
    /// them.
    pub fn set_part_words(&mut self, indices: &[u16], lang: Language, network: Network) {
        if self.step != Step::XorPart {
            return;
        }
        self.entry = crate::load::LoadWizard::from_words(indices, lang);
        self.count = self.entry.count();
        self.lang = lang;
        self.xor_print = None;
        self.part_landed(network);
    }

    /// A fresh word entry for the next part, at this wizard's count and
    /// wordlist.
    fn start_part(&mut self) {
        self.entry.zeroize();
        self.entry.set_count(self.count);
        self.entry.set_language(self.lang);
        self.entry.go(crate::load::Step::Words);
        self.xor_print = None;
    }

    /// Forgets every part and the entry.
    fn clear_parts(&mut self) {
        self.xor.zeroize();
        self.entry.zeroize();
        self.xor_parts = 0;
        self.xor_print = None;
    }

    /// A tap on the part step: the word entry's own, and then the two
    /// ways on once the words are in.
    fn tap_part(&mut self, id: Id, network: Network) {
        if self.entry.step() == crate::load::Step::Words {
            self.entry.tap(id, network);
            self.part_landed(network);
            return;
        }
        let add = id == ids::XOR_ADD_ANOTHER && self.xor_room();
        let combine = id == ids::XOR_COMBINE && self.xor_ready();
        if !add && !combine {
            return;
        }
        if !self.take_part() {
            return;
        }
        if add {
            self.start_part();
            return;
        }
        self.combine(network);
    }

    /// The last word of a part has just been accepted, whichever way it
    /// was typed: work out the fingerprint a key loaded from this part
    /// would have, which is what names it on the screen.
    fn part_landed(&mut self, network: Network) {
        if self.entry.step() != crate::load::Step::Checksum || self.xor_print.is_some() {
            return;
        }
        self.xor_print = self
            .entry
            .mnemonic()
            .and_then(|m| m.to_seed(b"").ok())
            .map(|seed| MasterKey::from_seed(&seed, network).fingerprint());
    }

    /// XORs the part on screen into the accumulator.
    fn take_part(&mut self) -> bool {
        let Some(m) = self.entry.mnemonic() else {
            return false;
        };
        let entropy = m.entropy();
        if self.xor.push(entropy.expose().as_bytes()).is_err() {
            return false;
        }
        self.xor_parts += 1;
        true
    }

    /// The parts XORed back into a key, which then finishes like any
    /// other: a passphrase may be added, and the confirm adds it.
    fn combine(&mut self, network: Network) {
        let Ok(entropy) = self.xor.entropy() else {
            return;
        };
        let Ok(m) = Mnemonic::from_entropy(self.lang, entropy.as_bytes()) else {
            return;
        };
        self.mnemonic = Some(m);
        self.clear_parts();
        // A recombined key is a key that already existed: there is
        // nothing to check the randomness of and no quiz to pass, so it
        // goes straight to the steps every wizard ends with.
        self.backup_verified = false;
        self.step = Step::PassphraseOffer;
        let _ = network;
    }

    // ----- words -----

    /// Number of word pages at `per_page` words each.
    pub fn word_pages(&self, per_page: usize) -> u8 {
        let count = self.mnemonic.as_ref().map_or(0, Mnemonic::word_count);
        count.div_ceil(per_page.max(1)).max(1) as u8
    }

    /// Word-grid page.
    pub fn page(&self) -> u8 {
        self.page
    }

    /// Whether the words screen shows each word's wordlist number.
    pub fn numbers(&self) -> bool {
        self.numbers
    }

    // ----- quiz -----

    fn start_quiz(&mut self, now_ms: u64) {
        let Some(m) = &self.mnemonic else {
            return;
        };
        let seed = Quiz::seed(now_ms, m.indices());
        self.quiz = Some(Quiz::new(
            m.indices(),
            EntryList::Bip39(self.lang),
            self.helper,
            seed,
        ));
        self.backup_verified = false;
        self.step = Step::Quiz;
    }

    /// Goes back to the question the words were opened from, with fresh
    /// decoys for the same word.
    fn return_to_quiz(&mut self) {
        self.from_quiz = false;
        self.page = 0;
        self.step = Step::Quiz;
        if let (Some(q), Some(m)) = (self.quiz.as_mut(), self.mnemonic.as_ref()) {
            q.retry(m.indices(), EntryList::Bip39(self.lang));
        }
    }

    /// The quiz, while on its step.
    pub fn quiz(&self) -> Option<&Quiz> {
        self.quiz.as_ref()
    }

    fn tap_quiz(&mut self, id: Id) {
        // A wrong answer never names the word; it offers the words
        // themselves instead (UX review 2026-09-07, §3.3).
        if id == ids::QUIZ_SHOW_WORDS {
            if self.quiz.as_ref().is_some_and(|q| q.wrong_slot().is_some()) {
                self.from_quiz = true;
                self.page = 0;
                self.step = Step::Words;
            }
            return;
        }
        let Some(m) = &self.mnemonic else {
            return;
        };
        let Some(q) = &mut self.quiz else {
            return;
        };
        let list = EntryList::Bip39(self.lang);
        if let Some(slot) = ids::index_in(id, ids::QUIZ_CHOICE_BASE, crate::quiz::CHOICES) {
            q.pick(slot);
        } else if id == ids::QUIZ_CONTINUE {
            q.confirm(m.indices(), list);
        } else if id == ids::QUIZ_RETRY {
            q.retry(m.indices(), list);
        }
        if q.state() == QuizState::Passed {
            self.backup_verified = true;
            self.quiz = None;
            self.step = Step::PassphraseOffer;
        }
    }

    // ----- key -----

    /// Takes the built key out; the rest of the wizard state is zeroized
    /// when `self` drops.
    pub fn into_key(mut self) -> Option<LoadedKey> {
        // A codex32 key is its master seed and has no words.
        if self.codex32 {
            return self
                .finish
                .take_key(None, self.backup_verified)
                .map(LoadedKey::codex32);
        }
        // A SLIP-39 key is its master secret and has no words.
        if self.slip39 {
            return self.finish.take_key(None, self.backup_verified);
        }
        let m = self.mnemonic.take()?;
        self.finish.take_key(Some(m), self.backup_verified)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_words(w: &mut CreateWizard) {
        // A choice is checked by the tap and confirmed by Continue.
        w.tap(ids::at(ids::CREATE_SOURCE_BASE, 0), Network::Mainnet, 0, 12);
        assert_eq!(w.step(), Step::Source, "the tap only checks");
        w.tap(ids::CREATE_SOURCE_CONTINUE, Network::Mainnet, 0, 12);
        assert_eq!(w.step(), Step::Count);
        w.tap(ids::at(ids::CREATE_COUNT_BASE, 0), Network::Mainnet, 0, 12);
        w.tap(ids::CREATE_COUNT_CONTINUE, Network::Mainnet, 0, 12);
        assert_eq!(w.step(), Step::Language);
        w.tap(ids::at(ids::CREATE_LANG_BASE, 0), Network::Mainnet, 0, 12);
        w.tap(ids::CREATE_LANG_CONTINUE, Network::Mainnet, 0, 12);
        // The dice source asks which published procedure reads the
        // rolls; the hashed one is checked when the Choice opens.
        assert_eq!(w.step(), Step::Procedure);
        assert_eq!(w.chosen_procedure(), DiceProcedure::Hashed);
        w.tap(ids::CREATE_PROCEDURE_CONTINUE, Network::Mainnet, 0, 12);
        assert_eq!(w.step(), Step::Entropy);
        for c in "32461151351521144121541512665155412152342515356215".chars() {
            assert!(w.entry_push(c, 10));
        }
        assert!(w.entry_ready());
        w.tap(ids::CREATE_CONTINUE, Network::Mainnet, 0, 12);
        assert_eq!(w.step(), Step::Sanity);
        assert!(!w.warnings().any());
        w.tap(ids::CREATE_CONTINUE, Network::Mainnet, 0, 12);
        assert_eq!(w.step(), Step::Words);
    }

    #[test]
    fn dice_entry_counts_masks_and_builds_the_mnemonic() {
        let mut w = CreateWizard::new();
        to_words(&mut w);
        let m = w.mnemonic().unwrap();
        assert_eq!(m.word_count(), 12);
        let e = m.entropy();
        assert_eq!(&e.expose().as_bytes()[..4], &[0xb4, 0xb7, 0xbf, 0x15]);
        // Masking: the last roll is visible for half a second.
        let mut w = CreateWizard::new();
        w.go(Step::Entropy);
        assert!(!w.entry_push('7', 0) && !w.entry_push('0', 0));
        assert!(w.entry_push('3', 1000));
        assert_eq!(w.entry_visible_last(1200), Some('3'));
        assert_eq!(w.entry_visible_last(1500), None);
        assert_eq!(w.mask_deadline(), Some(1500));
        assert_eq!(w.entry_needed(), 50);
        assert!(!w.entry_ready());
        w.entry_pop();
        assert_eq!(w.entry_len(), 0);
        assert_eq!(w.mask_deadline(), None);
        // Continue does nothing before the count is reached.
        w.tap(ids::CREATE_CONTINUE, Network::Mainnet, 0, 12);
        assert_eq!(w.step(), Step::Entropy);
    }

    #[test]
    fn quiz_pass_leads_to_the_passphrase_offer_and_marks_verified() {
        let mut w = CreateWizard::new();
        to_words(&mut w);
        w.tap(ids::CREATE_CONTINUE, Network::Mainnet, 5000, 12);
        assert_eq!(w.step(), Step::QuizStart);
        w.tap(ids::QUIZ_START, Network::Mainnet, 5000, 12);
        assert_eq!(w.step(), Step::Quiz);
        for _ in 0..12 {
            let slot = w.quiz().unwrap().correct_slot();
            w.tap(
                ids::at(ids::QUIZ_CHOICE_BASE, slot),
                Network::Mainnet,
                0,
                12,
            );
            w.tap(ids::QUIZ_CONTINUE, Network::Mainnet, 0, 12);
        }
        assert_eq!(w.step(), Step::PassphraseOffer);
        assert!(w.backup_verified());
        w.tap(ids::LOAD_SKIP, Network::Mainnet, 0, 12);
        w.tap(ids::LOAD_PASS_CONTINUE, Network::Mainnet, 0, 12);
        assert_eq!(w.step(), Step::Confirm);
        let key = w.into_key().unwrap();
        assert!(key.backup_verified);
        assert!(key.has_mnemonic());
        assert_eq!(key.language, Language::English);
    }

    #[test]
    fn skipping_the_quiz_needs_the_caution_and_leaves_unverified() {
        let mut w = CreateWizard::new();
        to_words(&mut w);
        w.tap(ids::CREATE_CONTINUE, Network::Mainnet, 5000, 12);
        assert_eq!(w.step(), Step::QuizStart);
        w.tap(ids::QUIZ_SKIP_CONFIRM, Network::Mainnet, 0, 12);
        assert_eq!(
            w.step(),
            Step::QuizStart,
            "the confirm lives on the caution, not on the start"
        );
        w.tap(ids::QUIZ_SKIP, Network::Mainnet, 0, 12);
        assert_eq!(w.step(), Step::QuizSkip);
        w.tap(ids::QUIZ_SKIP_CANCEL, Network::Mainnet, 0, 12);
        assert_eq!(w.step(), Step::QuizStart);
        w.tap(ids::QUIZ_SKIP, Network::Mainnet, 0, 12);
        w.tap(ids::QUIZ_SKIP_CONFIRM, Network::Mainnet, 0, 12);
        assert_eq!(w.step(), Step::PassphraseOffer);
        assert!(!w.backup_verified());
    }

    /// §4.3: ✓ before the digits the word count needs does nothing and
    /// says so on the reserved caption line under the field.
    #[test]
    fn the_hex_field_says_when_the_entries_are_short() {
        let mut w = CreateWizard::new();
        w.tap(ids::at(ids::CREATE_SOURCE_BASE, 2), Network::Mainnet, 0, 12);
        w.tap(ids::CREATE_SOURCE_CONTINUE, Network::Mainnet, 0, 12);
        w.go(Step::Entropy);
        assert!(!w.entry_too_short());
        assert!(!w.entry_push('z', 0), "the field takes hex only");
        assert!(w.entry_push('a', 0));
        w.tap(ids::CREATE_CONTINUE, Network::Mainnet, 0, 12);
        assert_eq!(w.step(), Step::Entropy);
        assert!(w.entry_too_short());
        assert!(w.entry_push('b', 0));
        assert!(!w.entry_too_short(), "a digit clears the line");
    }

    #[test]
    fn back_zeroizes_what_the_step_owned() {
        let mut w = CreateWizard::new();
        to_words(&mut w);
        assert!(w.back());
        assert_eq!(w.step(), Step::Sanity);
        assert!(w.back());
        assert_eq!(w.step(), Step::Entropy);
        assert!(w.mnemonic().is_none());
        assert_eq!(w.entry_len(), 50, "rolls survive a look at the sanity step");
        assert!(w.back());
        assert_eq!(w.step(), Step::Procedure);
        assert_eq!(w.entry_len(), 0);
        assert!(w.back());
        assert_eq!(w.step(), Step::Language);
        // The quiz's own steps step back one at a time.
        w.go(Step::QuizSkip);
        assert!(w.back());
        assert_eq!(w.step(), Step::QuizStart);
        assert!(w.back());
        assert_eq!(w.step(), Step::Words);
        w.go(Step::Source);
        assert!(!w.back(), "back at the first step cancels");
    }

    #[test]
    fn coins_and_hex_sources_count_their_own_units() {
        let mut w = CreateWizard::new();
        w.tap(ids::at(ids::CREATE_SOURCE_BASE, 1), Network::Mainnet, 0, 12);
        assert_eq!(w.source(), Source::Coins);
        w.tap(ids::CREATE_SOURCE_CONTINUE, Network::Mainnet, 0, 12);
        w.tap(ids::at(ids::CREATE_COUNT_BASE, 1), Network::Mainnet, 0, 12);
        assert_eq!(w.count(), 24);
        assert_eq!(w.entry_needed(), 256);
        w.go(Step::Entropy);
        assert!(w.entry_push('h', 0) && w.entry_push('T', 0) && !w.entry_push('x', 0));
        assert_eq!(w.entry_chars().collect::<alloc::vec::Vec<_>>(), ['H', 'T']);
        let mut w = CreateWizard::new();
        w.tap(ids::at(ids::CREATE_SOURCE_BASE, 2), Network::Mainnet, 0, 12);
        assert_eq!(w.source(), Source::Hex);
        w.tap(ids::CREATE_SOURCE_CONTINUE, Network::Mainnet, 0, 12);
        w.tap(ids::at(ids::CREATE_COUNT_BASE, 0), Network::Mainnet, 0, 12);
        assert_eq!(w.entry_needed(), 32);
        w.go(Step::Entropy);
        for _ in 0..32 {
            assert!(w.entry_push('0', 0));
        }
        assert!(!w.entry_push('0', 0), "exactly 32 digits");
        assert_eq!(w.entry_bits(), 128.0);
        w.key(ids::CREATE_PAD, KeyInput::Done, Network::Mainnet, 0);
        assert_eq!(w.step(), Step::Sanity);
        assert_eq!(
            osk_bip::bip39::Language::English.word(w.mnemonic().unwrap().indices()[11]),
            "about"
        );
        // A source this build cannot take leaves the check where it
        // was; every word count is live.
        let mut w = CreateWizard::new();
        w.set_device(false, true);
        w.tap(ids::at(ids::CREATE_SOURCE_BASE, 4), Network::Mainnet, 0, 12);
        assert_eq!(w.source(), Source::Dice, "no camera");
        w.tap(ids::at(ids::CREATE_SOURCE_BASE, 6), Network::Mainnet, 0, 12);
        assert_eq!(w.source(), Source::Dice, "Tier D");
        w.tap(ids::at(ids::CREATE_SOURCE_BASE, 3), Network::Mainnet, 0, 12);
        assert_eq!(w.source(), Source::Cards);
        w.go(Step::Count);
        w.tap(ids::at(ids::CREATE_COUNT_BASE, 2), Network::Mainnet, 0, 12);
        assert_eq!(w.count(), 15);
        assert_eq!(w.entry_needed(), CardDraws::needed(Strength::Bits160));
    }
}
