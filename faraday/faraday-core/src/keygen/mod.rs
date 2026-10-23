//! Making a new key: the randomness, the word count, and the words
//! (`docs/FLOWS.md`, New key).
//!
//! The options: the sources
//! and their order ([`SOURCE_ROWS`]), what a mix may combine
//! ([`MIX_SOURCES`]), the word counts and their names, and the dice
//! procedures' names. Every cryptographic step is `faraday-entropy`'s and `faraday-bip`'s,
//! the crates it calls: the rolls, flips, draws,
//! digits and frames accumulate in their fixed buffers, the procedures
//! hash or spell them, the sanity checks are theirs, a mix combines their
//! commitments, the checksum leaves the last word, and a [`Mnemonic`]
//! spells the entropy. Faraday adds the flow and the screen.
//!
//! The accumulators and the words are erased on drop; leaving the flow
//! drops them all.

pub(crate) mod screen;

use crate::strings::EN;
use crate::strings::fill;
use crate::wallet::KeyTexts;
use faraday_bip::bip39::{Language, Mnemonic};
use faraday_entropy::{
    CameraNoise, CardDraws, CoinFlips, DiceProcedure, DiceRolls, MIN_MIX, Mixed, RawHex, Strength,
    Warnings,
};

pub use crate::quiz::QuizState;
pub use faraday_entropy::{MIX_SOURCES, SOURCE_ROWS, Source};

use faraday_bip::bitcoin::hashes::{Hash, HashEngine, sha256};
use faraday_crypto::work::Compute;

use crate::forms::Form;
use crate::{Screen, flow};

/// The word counts on offer.
pub const COUNTS: [u8; 5] = faraday_entropy::WORD_COUNTS;

/// A source's name.
pub fn source_name(s: Source) -> &'static str {
    match s {
        Source::Dice => EN.create_source_dice,
        Source::Coins => EN.create_source_coins,
        Source::Hex => EN.create_source_hex,
        Source::Cards => EN.create_source_cards,
        Source::Camera => EN.create_source_camera,
        Source::Mix => EN.create_source_mix,
        Source::Device => EN.create_source_device,
        Source::SeedXor => EN.keygen_seed_xor,
    }
}

/// A dice procedure's name.
pub fn procedure_name(p: DiceProcedure) -> &'static str {
    match p {
        DiceProcedure::Hashed => EN.dice_procedure_hashed,
        DiceProcedure::SixAsZero => EN.dice_procedure_six_as_zero,
        DiceProcedure::Words => EN.keygen_words_chosen,
    }
}

/// One option on the Randomness card: a source and, for dice, how the
/// rolls are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Way {
    /// Each roll read as a flip, as Coins reads a die.
    DiceFlips,
    /// Coin flips, or a die's faces read as flips.
    Coins,
    /// Rolls that name words directly (BitBox), the last word too, its
    /// low bits replaced by the checksum.
    DiceWords,
    /// Rolls hashed (Coldcard, SeedSigner).
    DiceHashed,
    /// Rolls hashed with every 6 written 0 (Keystone).
    DiceSixAsZero,
    /// Hex digits made elsewhere.
    Hex,
    /// Cards from a shuffled deck.
    Cards,
    /// Pictures from the camera.
    Camera,
    /// Two or more sources, combined.
    Mix,
    /// This device's generator.
    Device,
}

/// The options, in the order the card lists them. Actions carry an
/// option by its place here.
pub const WAYS: [Way; 10] = [
    Way::DiceWords,
    Way::DiceFlips,
    Way::Coins,
    Way::DiceHashed,
    Way::DiceSixAsZero,
    Way::Hex,
    Way::Cards,
    Way::Camera,
    Way::Mix,
    Way::Device,
];

/// The groups the Randomness card lists the options in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// The person's own entropy, every word of which can be worked out
    /// by hand from the entries as they come in.
    ByHand,
    /// The person's own entropy, which the device hashes or splits into
    /// the words.
    Computed,
    /// This device's generator.
    Device,
}

impl Way {
    /// Its place in [`WAYS`].
    pub fn index(self) -> u8 {
        WAYS.iter().position(|w| *w == self).unwrap_or(0) as u8
    }

    /// The source it enters.
    pub fn source(self) -> Source {
        match self {
            Way::DiceFlips | Way::DiceWords | Way::DiceHashed | Way::DiceSixAsZero => Source::Dice,
            Way::Coins => Source::Coins,
            Way::Hex => Source::Hex,
            Way::Cards => Source::Cards,
            Way::Camera => Source::Camera,
            Way::Mix => Source::Mix,
            Way::Device => Source::Device,
        }
    }

    /// The group it is listed in. SLIP-39 shares are made from the
    /// entropy by the device, so no option makes words a person can
    /// check by hand.
    pub fn group(self, slip39: bool) -> Group {
        match self {
            Way::Device => Group::Device,
            Way::DiceFlips | Way::Coins | Way::DiceWords if !slip39 => Group::ByHand,
            _ => Group::Computed,
        }
    }

    /// Whether it is offered: rolls that name words make BIP-39 words,
    /// not a SLIP-39 secret.
    pub fn offered(self, slip39: bool) -> bool {
        !(slip39 && self == Way::DiceWords)
    }
}

/// The cards of the flow, in order.
pub mod kstep {
    /// How the key comes in: made here, typed, or scanned
    /// (`docs/history/NEW-WALLET.md` §15.1).
    pub const START: u8 = 0;
    /// 12 or 24 words.
    pub const LENGTH: u8 = 1;
    /// Which source.
    pub const SOURCE: u8 = 2;
    /// Rolling, flipping, drawing, typing or taking frames; for BIP-39
    /// words the randomness check too, once the last entry is in.
    pub const ENTER: u8 = 3;
    /// The words, a passphrase for a Create slot or a key typed, and Lock
    /// in, which adds the key; then its fingerprint and what it makes
    /// (`docs/history/NEW-WALLET.md` §3.3, §15.2). Not for SLIP-39 shares made
    /// here.
    pub const KEY: u8 = 4;
    /// Counts, statistics and cautions: SLIP-39 shares only.
    pub const CHECK: u8 = 5;
    /// The words.
    pub const WORDS: u8 = 6;
    /// The backup quiz over them.
    pub const QUIZ: u8 = 7;
    /// The words, shares, codex32 strings or Seed XOR parts typed: Add a
    /// key's form, drawn by the same code (`docs/history/NEW-WALLET.md` §15.2).
    pub const TYPED: u8 = 8;
    /// How many cards.
    pub const COUNT: usize = 9;
}

/// The choices on the Start card (`docs/history/NEW-WALLET.md` §15.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// Make a new key from randomness, as BIP-39 words.
    Make,
    /// Type the words of a key already held.
    Words,
    /// Scan a SeedQR.
    Scan,
    /// Make a new key as SLIP-39 shares.
    MakeShares,
    /// Type SLIP-39 shares.
    Shares,
    /// Type a codex32 secret, or its shares.
    Codex32,
    /// Type Seed XOR parts.
    Xor,
}

/// The choices, in the order the card lists them. Actions carry a
/// choice by its place here.
pub const STARTS: [Start; 7] = [
    Start::Make,
    Start::Words,
    Start::Scan,
    Start::MakeShares,
    Start::Shares,
    Start::Codex32,
    Start::Xor,
];

impl Start {
    /// Its place in [`STARTS`].
    pub fn index(self) -> u8 {
        STARTS.iter().position(|s| *s == self).unwrap_or(0) as u8
    }

    /// Listed under Other forms, closed at first.
    pub fn other(self) -> bool {
        matches!(
            self,
            Start::MakeShares | Start::Shares | Start::Codex32 | Start::Xor
        )
    }

    /// The key is made here from randomness.
    pub fn makes(self) -> bool {
        matches!(self, Start::Make | Start::MakeShares)
    }

    /// The form typed on the Words card, for a key typed.
    pub fn form(self) -> Option<Form> {
        match self {
            Start::Words => Some(Form::Words),
            Start::Shares => Some(Form::Slip39),
            Start::Codex32 => Some(Form::Codex32),
            Start::Xor => Some(Form::Xor),
            _ => None,
        }
    }

    /// The choice that types `form`.
    pub fn typing(form: Form) -> Start {
        match form {
            Form::Words => Start::Words,
            Form::Slip39 => Start::Shares,
            Form::Codex32 => Start::Codex32,
            Form::Xor => Start::Xor,
        }
    }

    /// Whether a threshold wallet's slot offers it: its shares are 24
    /// BIP-39 words, made here or typed.
    pub fn for_threshold(self) -> bool {
        matches!(self, Start::Make | Start::Words)
    }
}

/// A new key being made. See the module documentation.
pub struct KeyGen {
    /// The open card.
    pub open: Option<u8>,
    /// Cards closed as done.
    pub done: [bool; kstep::COUNT],
    /// The column's scroll.
    pub scroll: flow::Scroll,
    /// 12, 15, 18, 21 or 24.
    pub words: usize,
    /// Only 24 words will do (a threshold wallet's shares are 32 bytes).
    pub only_24: bool,
    /// The chosen source.
    pub source: Option<Source>,
    /// Which dice procedure, when the source is dice.
    pub procedure: DiceProcedure,
    /// Which of [`MIX_SOURCES`] a mix takes.
    pub mix: [bool; MIX_SOURCES.len()],
    /// Which of the mix's sources is being entered.
    pub mix_at: usize,
    /// The commitments a mix has taken.
    pub mixed: Mixed,
    /// Rolls.
    pub dice: DiceRolls,
    /// Flips.
    pub coins: CoinFlips,
    /// Draws.
    pub cards: CardDraws,
    /// Digits.
    pub hex: RawHex,
    /// Frames taken.
    pub camera: CameraNoise,
    /// The 32 bytes the shell answered with, asked for when this
    /// device's generator is the source being entered.
    pub device: faraday_entropy::DeviceRandom,
    /// Those bytes have been asked for and not yet answered.
    pub device_asked: bool,
    /// The newest camera frame's luma, while the camera is on.
    pub frame: Option<Vec<u8>>,
    /// A card's rank, picked before its suit.
    pub rank: Option<u8>,
    /// Which of the Randomness card's groups are open: the options
    /// verifiable by hand, and the ones the device computes.
    pub groups_open: [bool; 3],
    /// The words, once made.
    pub mnemonic: Option<Mnemonic>,
    /// The words are on screen.
    pub shown: bool,
    /// The key's fingerprint, once the words are made.
    pub fingerprint: Option<[u8; 4]>,
    /// The backup quiz, once the words have been shown.
    pub quiz: Option<crate::quiz::Quiz>,
    /// The person asked to skip the quiz; a second press skips it.
    pub skip_ask: bool,
    /// The quiz was skipped.
    pub skipped: bool,
    /// Shares, strings or parts typed in, once the Words card of a form
    /// other than BIP-39 words is done.
    pub parts_in: usize,
    /// The last thing worth saying.
    pub note: Option<String>,
    /// The Create slot the key fills, if it was made for one.
    pub slot: Option<u8>,
    /// The key is made as SLIP-39 shares rather than BIP-39 words: the
    /// entropy is the master secret, written only as shares.
    pub slip39: bool,
    /// Shares needed to restore.
    pub slip_m: u8,
    /// Shares dealt.
    pub slip_n: u8,
    /// The master secret, once the entropy is in.
    pub secret: Option<zeroize::Zeroizing<Vec<u8>>>,
    /// The shares, once dealt.
    pub shares: Vec<faraday_bip::slip39::Share>,
    /// The share on screen.
    pub share_at: usize,
    /// The share the quiz is on.
    pub quiz_share: usize,
    /// Coin flips are entered as die faces: 1, 2 or 3 is a 0 (tails),
    /// 4, 5 or 6 a 1 (heads). The flips, and so the key, are the same as
    /// flipping a coin.
    pub by_die: bool,
    /// Entries are typed into a box, rather than pressed on buttons. On
    /// from the start on a desktop, which has a keyboard; off on a small
    /// panel. Either way each entry is taken as it comes.
    pub typing: bool,
    /// Every roll or flip taken, as it was entered (a face, or H or T),
    /// in order: what the box shows.
    pub entered: crate::secret_text::SecretText,
    /// Where the flow returns to.
    pub back: Screen,
    /// The passphrase typed on the Key card, for a Create slot.
    pub passphrase: crate::secret_text::SecretText,
    /// The same passphrase typed again.
    pub passphrase2: crate::secret_text::SecretText,
    /// Which of the two passphrase fields typing goes to, if either.
    pub focus: Option<u8>,
    /// The key is added: nothing about it changes after this
    /// (`docs/history/NEW-WALLET.md` §3.4).
    pub locked: bool,
    /// It was locked in with a passphrase.
    pub with_passphrase: bool,
    /// The sample the demo added in place of the key made, as its word
    /// and its word count ([`faraday_keys::Keys::next_handout`]).
    pub handed: Option<(&'static str, u8)>,
    /// The key as `[fingerprint/path]xpub` at the kind's path, once
    /// locked in: the Create kind's for a slot, native SegWit on its own.
    pub key_text: Option<String>,
    /// The Start card's choice (`docs/history/NEW-WALLET.md` §15.1).
    pub start: Start,
    /// The choice the cards after Start were opened for: a different
    /// one chosen with Change starts them over.
    pub started: Option<Start>,
    /// The Start card's Other forms are shown.
    pub others_open: bool,
    /// Add a key's own state, kept here while the flow types into the
    /// app's (`Faraday::entry`), and put back when the flow ends.
    pub entry_kept: crate::EntryState,
    /// What This device's randomness is read from, copied from
    /// `Faraday::entropy_source` when the flow opens and kept in step by
    /// `Event::EntropySource`; `None` when the shell named none.
    pub entropy_source: Option<&'static str>,
}

impl Drop for KeyGen {
    fn drop(&mut self) {
        if let Some(f) = self.frame.as_mut() {
            zeroize::Zeroize::zeroize(f);
        }
    }
}

impl KeyGen {
    /// A fresh flow.
    pub fn new(slot: Option<u8>, back: Screen, only_24: bool) -> KeyGen {
        KeyGen {
            // New key opens on Start, Make a new key ticked
            // (`docs/history/NEW-WALLET.md` §15.3); its Continue opens Length
            // (§2.1, reversing `docs/history/SIMPLIFY.md` §2.1 for this flow). Length
            // and Randomness still carry defaults and show Change once
            // closed (`DESIGN.md` §4.14), but nothing is done until its
            // own Continue is pressed.
            open: Some(kstep::START),
            done: [false; kstep::COUNT],
            scroll: flow::Scroll::default(),
            words: if only_24 { 24 } else { 12 },
            only_24,
            // Dice that name words: every word checkable by hand as it
            // comes in.
            source: Some(Source::Dice),
            procedure: DiceProcedure::Words,
            mix: [false; MIX_SOURCES.len()],
            mix_at: 0,
            mixed: Mixed::new(),
            dice: DiceRolls::new(),
            coins: CoinFlips::new(),
            cards: CardDraws::new(),
            hex: RawHex::new(),
            camera: CameraNoise::new(),
            device: faraday_entropy::DeviceRandom::new(),
            device_asked: false,
            frame: None,
            rank: None,
            groups_open: [true, false, false],
            mnemonic: None,
            shown: false,
            fingerprint: None,
            quiz: None,
            skip_ask: false,
            skipped: false,
            parts_in: 0,
            note: None,
            slot,
            back,
            slip39: false,
            slip_m: 2,
            slip_n: 3,
            secret: None,
            shares: Vec::new(),
            share_at: 0,
            quiz_share: 0,
            by_die: false,
            typing: true,
            entered: crate::secret_text::SecretText::new(),
            passphrase: crate::secret_text::SecretText::new(),
            passphrase2: crate::secret_text::SecretText::new(),
            focus: None,
            locked: false,
            with_passphrase: false,
            handed: None,
            key_text: None,
            start: Start::Make,
            started: None,
            others_open: false,
            entry_kept: crate::EntryState::default(),
            entropy_source: None,
        }
    }

    /// Whether the key is typed or scanned rather than made here.
    pub fn typed(&self) -> bool {
        !self.start.makes()
    }

    /// Whether the Key card has the two passphrase fields: for a key
    /// typed or scanned (none for codex32, which has no passphrase), and
    /// for one made for a Create slot; a key made on its own has none
    /// (decision 4).
    pub fn asks_passphrase(&self) -> bool {
        if self.typed() {
            self.start != Start::Codex32
        } else {
            self.slot.is_some()
        }
    }

    /// The cards this flow shows, in order (`docs/history/NEW-WALLET.md` §3.1):
    /// BIP-39 words for a Create slot end on Key, since Back up shows
    /// and checks the words; on their own they go on to Words and the
    /// Quiz. SLIP-39 shares keep their Check card and add the key at the
    /// end of the Quiz.
    ///
    /// Every flow opens with Start; a key typed is Start · Words · Key,
    /// one scanned Start · Key (§15.2).
    pub fn steps(&self) -> Vec<u8> {
        use kstep::*;
        if self.start == Start::Scan {
            vec![START, KEY]
        } else if self.typed() {
            vec![START, TYPED, KEY]
        } else if self.slip39 {
            vec![START, LENGTH, SOURCE, ENTER, CHECK, WORDS, QUIZ]
        } else if self.slot.is_some() {
            vec![START, LENGTH, SOURCE, ENTER, KEY]
        } else {
            vec![START, LENGTH, SOURCE, ENTER, KEY, WORDS, QUIZ]
        }
    }

    /// The Start card's choices this flow offers: a threshold wallet's
    /// slot only Make a new key and Type the words (§15.3).
    pub fn starts(&self) -> Vec<Start> {
        STARTS
            .iter()
            .copied()
            .filter(|s| !self.only_24 || s.for_threshold())
            .collect()
    }

    /// Whether the entries have counts the randomness check reads: not
    /// this device's generator, the camera or a mix as a whole.
    pub fn has_check(&self) -> bool {
        !matches!(
            self.active(),
            None | Some(Source::Device | Source::Camera | Source::Mix | Source::SeedXor)
        )
    }

    /// The word counts the Length card offers: BIP-39's, or a SLIP-39
    /// share's (20 words for 128 bits, 33 for 256).
    pub fn counts(&self) -> &'static [u8] {
        if self.slip39 {
            &crate::word_entry::SLIP39_COUNTS
        } else {
            &COUNTS
        }
    }

    /// Switches between words and shares, forgetting what was entered.
    pub fn set_slip39(&mut self, on: bool) {
        if self.slip39 == on || (on && self.only_24) {
            return;
        }
        self.slip39 = on;
        self.words = usize::from(self.counts()[0]);
        self.clear_entries();
        // Start's choice follows: shares or words, made here.
        let start = self.done[usize::from(kstep::START)];
        self.done = [false; kstep::COUNT];
        self.done[usize::from(kstep::START)] = start;
        if self.start.makes() {
            self.start = if on { Start::MakeShares } else { Start::Make };
        }
        self.open = Some(kstep::LENGTH);
        // A share's words are a fixed scheme: no direct word choice. Back
        // to words, the dice name them again, as at the start.
        if on {
            self.procedure = DiceProcedure::Hashed;
        } else if self.way() == Some(Way::DiceHashed) {
            self.procedure = DiceProcedure::Words;
        }
        // Shares are not words anyone checks by hand: the options are
        // all in the group the device computes.
        self.groups_open[0] = !on;
        self.groups_open[1] = on;
    }

    /// The share whose words are on screen or quizzed, as SLIP-39 word
    /// indices.
    pub fn share_indices(&self, at: usize) -> Option<&[u16]> {
        self.shares.get(at).map(|s| s.indices())
    }

    /// Costs a PBKDF2 (through `split`). Deals the master secret into
    /// `slip_m` of `slip_n` shares, with `random` for the split's own
    /// randomness (a fresh draw from the system's generator).
    pub fn deal(&mut self, c: &Compute, random: [u8; 32]) -> Result<(), String> {
        use faraday_bip::slip39;
        let Some(secret) = self.secret.as_ref() else {
            return Err(EN.keygen_no_secret_yet.to_string());
        };
        // The draw, stretched: SHA-256 over the draw and a counter, as
        // much as the split asks for.
        let mut counter: u32 = 0;
        let mut fill = |out: &mut [u8]| {
            for chunk in out.chunks_mut(32) {
                let mut e = sha256::Hash::engine();
                e.input(b"faraday slip39 split");
                e.input(&random);
                e.input(&counter.to_le_bytes());
                counter += 1;
                let h = sha256::Hash::from_engine(e).to_byte_array();
                chunk.copy_from_slice(&h[..chunk.len()]);
            }
        };
        let mut id = [0u8; 2];
        fill(&mut id);
        let identifier = u16::from_le_bytes(id) & 0x7fff;
        let mut out: Vec<slip39::Share> = Vec::new();
        out.resize_with(usize::from(self.slip_n), slip39::Share::default);
        let n = faraday_keys::words::slip39_split(
            c,
            secret,
            identifier,
            self.slip_m,
            self.slip_n,
            &mut fill,
            &mut out,
        )
        .map_err(|e| crate::strings::fill(EN.keygen_the_shares_could, &[&format!("{e:?}")]))?;
        out.truncate(n);
        self.shares = out;
        self.share_at = 0;
        Ok(())
    }

    /// The strength the word count takes.
    pub fn strength(&self) -> Strength {
        if self.slip39 {
            return if self.words <= faraday_bip::slip39::MIN_WORDS {
                Strength::Bits128
            } else {
                Strength::Bits256
            };
        }
        Strength::for_words(self.words).unwrap_or(Strength::Bits256)
    }

    /// The procedure the rolls follow: the chosen one for dice alone, the
    /// hashed one inside a mix.
    pub fn procedure(&self) -> DiceProcedure {
        if self.source == Some(Source::Dice) {
            self.procedure
        } else {
            DiceProcedure::Hashed
        }
    }

    /// The option chosen on the Randomness card.
    pub fn way(&self) -> Option<Way> {
        Some(match self.source? {
            Source::Dice if self.by_die => Way::DiceFlips,
            Source::Dice => match self.procedure {
                DiceProcedure::Hashed => Way::DiceHashed,
                DiceProcedure::SixAsZero => Way::DiceSixAsZero,
                DiceProcedure::Words => Way::DiceWords,
            },
            Source::Coins => Way::Coins,
            Source::Hex => Way::Hex,
            Source::Cards => Way::Cards,
            Source::Camera => Way::Camera,
            Source::Mix => Way::Mix,
            Source::Device | Source::SeedXor => Way::Device,
        })
    }

    /// Chooses an option, forgetting what was entered.
    pub fn choose(&mut self, way: Way) {
        if !way.offered(self.slip39) {
            return;
        }
        self.source = Some(way.source());
        self.by_die = way == Way::DiceFlips;
        self.procedure = match way {
            Way::DiceWords => DiceProcedure::Words,
            Way::DiceSixAsZero => DiceProcedure::SixAsZero,
            _ => DiceProcedure::Hashed,
        };
        self.clear_entries();
        self.done[usize::from(kstep::SOURCE)] = false;
    }

    /// The high bits of the last rolled word the key keeps under direct
    /// selection: what the checksum leaves, 11 less a bit for every three
    /// words (7 at 12 words, 3 at 24). The checksum replaces the rest
    /// (`faraday_entropy::DiceRolls::entropy_under`).
    pub fn last_bits(&self) -> usize {
        11 - self.words / 3
    }

    /// Rolls the chosen dice procedure takes. Direct selection stops as
    /// soon as the key's bits are in: six rolls for every word but the
    /// last, and for the last only the faces that cover its kept high
    /// bits, two bits a face (4 at 12 words, 2 at 24). The rest of that
    /// word would be replaced by the checksum, so it is not rolled.
    pub fn dice_needed(&self) -> usize {
        self.rolls_for(self.procedure())
    }

    /// Rolls procedure `p` takes at this length (see
    /// [`KeyGen::dice_needed`]).
    pub fn rolls_for(&self, p: DiceProcedure) -> usize {
        if p.direct() {
            faraday_entropy::DICE_WORD_ROLLS * (self.words - 1) + self.last_bits().div_ceil(2)
        } else {
            p.needed(self.strength())
        }
    }

    /// The dice's entropy under the chosen procedure. Direct selection's
    /// last word, rolled only as far as its kept bits, is filled out
    /// with faces that stand for zero bits, which the checksum replaces.
    fn dice_entropy(
        &self,
        strength: Strength,
    ) -> Result<faraday_entropy::RawEntropy, faraday_entropy::Error> {
        let p = self.procedure();
        if !p.direct() || self.dice.len() < self.dice_needed() {
            return self.dice.entropy_under(strength, p);
        }
        let mut full = DiceRolls::new();
        for &r in self.dice.rolls() {
            full.push(r);
        }
        while full.len() < p.needed(strength) {
            full.push(1);
        }
        full.entropy_under(strength, p)
    }

    /// The sources a mix takes, in order.
    pub fn mix_list(&self) -> Vec<Source> {
        MIX_SOURCES
            .iter()
            .zip(self.mix)
            .filter(|(_, on)| *on)
            .map(|(s, _)| *s)
            .collect()
    }

    /// The source being entered now: the chosen one, or a mix's current
    /// one.
    pub fn active(&self) -> Option<Source> {
        match self.source? {
            Source::Mix => self.mix_list().get(self.mix_at).copied(),
            s => Some(s),
        }
    }

    /// Entries in and entries needed for the active source.
    pub fn progress(&self) -> (usize, usize) {
        let s = self.strength();
        match self.active() {
            Some(Source::Dice) if self.by_die => (self.coins.len(), CoinFlips::needed(s)),
            Some(Source::Dice) => (self.dice.len(), self.dice_needed()),
            Some(Source::Coins) => (self.coins.len(), CoinFlips::needed(s)),
            Some(Source::Cards) => (self.cards.len(), CardDraws::needed(s)),
            Some(Source::Hex) => (self.hex.len(), s.hex_digits()),
            Some(Source::Camera) => (self.camera.len(), CameraNoise::needed(s)),
            Some(Source::Device) => (usize::from(self.device.is_enough(s)), 1),
            _ => (0, 1),
        }
    }

    /// Whether the active source has what the strength needs.
    pub fn ready(&self) -> bool {
        let (have, need) = self.progress();
        match self.active() {
            Some(Source::Hex) => have == need,
            _ => have >= need,
        }
    }

    /// The cautions on the active source.
    pub fn warnings(&self) -> Warnings {
        let s = self.strength();
        match self.active() {
            Some(Source::Dice) if self.by_die => self.coins.warnings(s),
            Some(Source::Dice) => Warnings {
                too_few: !self.ready(),
                ..self.dice.warnings_under(s, self.procedure())
            },
            Some(Source::Coins) => self.coins.warnings(s),
            Some(Source::Cards) => self.cards.warnings(s),
            Some(Source::Camera) => self.camera.warnings(s),
            _ => Warnings {
                too_few: !self.ready(),
                ..Warnings::default()
            },
        }
    }

    /// The cautions, as lines.
    pub fn caution_lines(&self) -> Vec<&'static str> {
        let w = self.warnings();
        let mut out = Vec::new();
        if w.long_run {
            out.push(EN.keygen_caution_long_run);
        }
        if w.skewed {
            out.push(EN.keygen_far_from_even);
        }
        if w.sequential {
            out.push(EN.keygen_a_long_run_counting);
        }
        out
    }

    /// Forgets every entry, the mix and the words.
    pub fn clear_entries(&mut self) {
        self.clear_entered();
        self.dice.clear();
        self.coins.clear();
        self.cards.clear();
        self.hex.clear();
        self.camera.clear();
        self.device.clear();
        self.device_asked = false;
        self.mixed.clear();
        self.mix_at = 0;
        self.rank = None;
        self.mnemonic = None;
        self.shown = false;
        self.fingerprint = None;
        self.quiz = None;
        self.skip_ask = false;
        self.skipped = false;
        self.note = None;
        for k in [
            kstep::ENTER,
            kstep::KEY,
            kstep::CHECK,
            kstep::WORDS,
            kstep::QUIZ,
        ] {
            self.done[usize::from(k)] = false;
        }
    }

    /// Forgets the active source's entries only.
    pub fn clear_active(&mut self) {
        self.clear_entered();
        match self.active() {
            Some(Source::Dice) if self.by_die => self.coins.clear(),
            Some(Source::Dice) => self.dice.clear(),
            Some(Source::Coins) => self.coins.clear(),
            Some(Source::Cards) => self.cards.clear(),
            Some(Source::Hex) => self.hex.clear(),
            Some(Source::Camera) => self.camera.clear(),
            Some(Source::Device) => {
                self.device.clear();
                self.device_asked = false;
            }
            _ => {}
        }
        self.rank = None;
        self.note = None;
    }

    /// Takes back the last entry.
    pub fn undo(&mut self) {
        if self.rank.take().is_some() {
            return;
        }
        let had = self.progress().0;
        match self.active() {
            Some(Source::Dice) if self.by_die => {
                self.coins.pop();
            }
            Some(Source::Dice) => {
                self.dice.pop();
            }
            Some(Source::Coins) => {
                self.coins.pop();
            }
            Some(Source::Cards) => {
                self.cards.pop();
            }
            Some(Source::Hex) => {
                self.hex.pop();
            }
            _ => {}
        }
        if self.progress().0 < had && self.flip_or_roll() {
            self.entered.pop();
        }
        self.note = None;
    }

    /// Whether the active source's entries are rolls or flips, which
    /// the box shows.
    pub(crate) fn flip_or_roll(&self) -> bool {
        matches!(self.active(), Some(Source::Dice | Source::Coins))
    }

    /// Notes an entry taken, for the box.
    fn record(&mut self, c: char) {
        if self.entered.len() < 1024 {
            self.entered.push(c);
        }
    }

    /// Whether a press lands in the flip accumulator: Coins, with either
    /// its own sides or a die's faces, or Dice in its own Flip mode.
    fn flip_active(&self) -> bool {
        match self.active() {
            Some(Source::Coins) => true,
            Some(Source::Dice) => self.by_die,
            _ => false,
        }
    }

    /// A roll of 1 to 6. In Dice's Flip mode this is read as a flip
    /// instead, the same reading Coins gives a die's face.
    pub fn roll(&mut self, face: u8) {
        if self.active() != Some(Source::Dice) {
            return;
        }
        if self.by_die {
            self.die_flip(face);
            return;
        }
        let (have, need) = self.progress();
        if have >= need {
            return;
        }
        let taken = self.dice.push_under(face, self.procedure());
        if taken {
            self.record(char::from(b'0' + face));
        }
        self.note = (!taken).then(|| {
            if self.procedure().direct() {
                EN.keygen_reroll_rule.to_string()
            } else {
                EN.keygen_a_die_has_six.to_string()
            }
        });
    }

    /// A flip: heads or tails.
    pub fn flip(&mut self, heads: bool) {
        if self.flip_in(heads) {
            self.record(if heads { 'H' } else { 'T' });
        }
    }

    /// Takes a flip into the accumulator. Returns whether it was taken.
    fn flip_in(&mut self, heads: bool) -> bool {
        let (have, need) = self.progress();
        self.flip_active() && have < need && self.coins.push(heads)
    }

    /// A die's face read as a flip: 1, 2 or 3 is tails (0), 4, 5 or 6
    /// heads (1).
    pub fn die_flip(&mut self, face: u8) {
        if !self.flip_active() {
            return;
        }
        if !(1..=6).contains(&face) {
            self.note = Some(EN.keygen_a_die_has_six.to_string());
            return;
        }
        self.note = None;
        if self.flip_in(face >= 4) {
            self.record(char::from(b'0' + face));
        }
    }

    /// Whether the box takes `c`: a die's face for dice or for flips
    /// read from a die, H, T, 1 or 0 for coin flips.
    pub fn typable(&self, c: char) -> bool {
        match self.active() {
            Some(Source::Dice) => ('1'..='6').contains(&c),
            Some(Source::Coins) if self.by_die => ('1'..='6').contains(&c),
            Some(Source::Coins) => matches!(c, '0' | '1' | 'h' | 'H' | 't' | 'T'),
            _ => false,
        }
    }

    /// A character typed into the box, taken at once as the entry it
    /// names: a roll, a die's face read as a flip, or a flip. One the
    /// source does not take is left out.
    pub fn type_char(&mut self, c: char) {
        if !self.typable(c) {
            return;
        }
        if self.progress().0 >= self.progress().1 {
            self.note = Some(EN.keygen_no_more_are.to_string());
            return;
        }
        match (self.active(), c) {
            (Some(Source::Dice), _) => self.roll(c as u8 - b'0'),
            (Some(Source::Coins), _) if self.by_die => self.die_flip(c as u8 - b'0'),
            (Some(Source::Coins), '1' | 'h' | 'H') => self.flip(true),
            (Some(Source::Coins), _) => self.flip(false),
            _ => {}
        }
    }

    /// Forgets the record of entries.
    pub fn clear_entered(&mut self) {
        self.entered.clear();
    }

    /// Whether the entries are the key's own bits as they come in, so
    /// the words they name can be shown as they are entered: coin flips
    /// (as they are or read from a die), packed as they are, or rolls
    /// that name words directly. Not inside a mix, which hashes its
    /// sources, nor for SLIP-39 shares, which are not BIP-39 words.
    pub fn reveals(&self) -> bool {
        !self.slip39
            && match self.source {
                Some(Source::Coins) => true,
                Some(Source::Dice) if self.by_die => true,
                Some(Source::Dice) => self.procedure().direct(),
                _ => false,
            }
    }

    /// The words the entries name so far: one for each whole 11 flips,
    /// first flip the highest bit, or one for each whole group of rolls
    /// under direct selection. Never the last word, whose last bits are
    /// the checksum of all the others and are known only once every
    /// other bit is in.
    pub fn live_words(&self) -> zeroize::Zeroizing<Vec<u16>> {
        let mut out = zeroize::Zeroizing::new(Vec::new());
        if !self.reveals() {
            return out;
        }
        let most = self.words.saturating_sub(1);
        // Flips always land in `coins`, whether Coins or Dice's Flip mode
        // took them.
        if self.source == Some(Source::Coins) || self.by_die {
            let (mut index, mut n) = (0u16, 0);
            for heads in self.coins.flips() {
                if out.len() >= most {
                    break;
                }
                index = index << 1 | u16::from(heads);
                n += 1;
                if n == 11 {
                    out.push(index);
                    index = 0;
                    n = 0;
                }
            }
            zeroize::Zeroize::zeroize(&mut index);
        } else {
            out.extend(self.dice.word_indices().take(most));
        }
        out
    }

    /// The last word, once every entry is in: what the checksum makes
    /// of the rest, from the words made or, before they are, worked out
    /// on the spot.
    pub fn checksum_word(&self) -> Option<u16> {
        if !self.reveals() {
            return None;
        }
        if let Some(m) = self.mnemonic.as_ref() {
            return m.indices().get(self.words - 1).copied();
        }
        if !self.ready() {
            return None;
        }
        let strength = self.strength();
        let mut bytes = if self.procedure().direct() && !self.by_die {
            self.dice_entropy(strength).ok()?.as_bytes().to_vec()
        } else {
            self.coins.entropy(strength).ok()?.as_bytes().to_vec()
        };
        let m = faraday_keys::words::from_entropy(Language::English, &bytes).ok();
        zeroize::Zeroize::zeroize(&mut bytes);
        m?.indices().get(self.words - 1).copied()
    }

    /// A card's rank, then its suit, makes a draw.
    pub fn card(&mut self, rank: Option<u8>, suit: Option<u8>) {
        if self.active() != Some(Source::Cards) {
            return;
        }
        if let Some(r) = rank {
            self.rank = Some(r);
        }
        if let (Some(r), Some(s)) = (self.rank, suit) {
            let (have, need) = self.progress();
            if have >= need {
                return;
            }
            self.note = match faraday_entropy::card_index(r, s) {
                Some(i) if self.cards.push(i) => None,
                Some(_) => Some(EN.keygen_card_already_drawn.to_string()),
                None => None,
            };
            self.rank = None;
        }
    }

    /// A hex digit.
    pub fn hex_digit(&mut self, c: char) {
        let (have, need) = self.progress();
        if self.active() == Some(Source::Hex) && have < need {
            self.hex.push(c.to_ascii_lowercase());
        }
    }

    /// Takes the newest camera frame.
    pub fn take_frame(&mut self) {
        if self.active() != Some(Source::Camera) {
            return;
        }
        let Some(luma) = self.frame.as_ref() else {
            self.note = Some(EN.keygen_no_camera_picture.to_string());
            return;
        };
        let (_, taken) = self.camera.push(luma);
        self.note = (!taken).then(|| EN.keygen_too_little_variation.to_string());
    }

    /// SHA-256 of the active source's input, which a mix combines.
    fn commitment(&self) -> [u8; 32] {
        match self.active() {
            Some(Source::Coins) => self.coins.commitment(),
            Some(Source::Dice) if self.by_die => self.coins.commitment(),
            Some(Source::Cards) => self.cards.commitment(),
            Some(Source::Camera) => self.camera.commitment(),
            Some(Source::Device) => self.device.commitment(),
            Some(Source::Hex) => self.hex.commitment(),
            _ => self.dice.commitment(),
        }
    }

    /// Done with the active source: the next source of a mix, or the
    /// words. Returns whether the words were made.
    pub fn finish_entry(&mut self) -> bool {
        if !self.ready() {
            let (have, need) = self.progress();
            self.note = Some(fill(
                EN.keygen_entry_count,
                &[&have.to_string(), &need.to_string()],
            ));
            return false;
        }
        self.note = None;
        let strength = self.strength();
        if self.source == Some(Source::Mix) {
            self.mixed.push(self.commitment());
            self.clear_active();
            self.mix_at += 1;
            if self.mix_at < self.mix_list().len() {
                return false;
            }
            return match self.mixed.entropy(strength) {
                Ok(e) => self.words_from(e.as_bytes()),
                Err(_) => false,
            };
        }
        let entropy = match self.source {
            Some(Source::Dice) if self.by_die => self.coins.entropy(strength),
            Some(Source::Dice) => self.dice_entropy(strength),
            Some(Source::Coins) => self.coins.entropy(strength),
            Some(Source::Cards) => self.cards.entropy(strength),
            Some(Source::Hex) => self.hex.entropy(),
            Some(Source::Camera) => self.camera.entropy(strength),
            Some(Source::Device) => self.device.entropy(strength),
            _ => return false,
        };
        match entropy {
            Ok(e) => self.words_from(e.as_bytes()),
            Err(_) => false,
        }
    }

    fn words_from(&mut self, entropy: &[u8]) -> bool {
        // A SLIP-39 key's entropy is its master secret.
        if self.slip39 {
            self.secret = Some(zeroize::Zeroizing::new(entropy.to_vec()));
            return true;
        }
        match faraday_keys::words::from_entropy(Language::English, entropy) {
            Ok(m) => {
                self.mnemonic = Some(m);
                true
            }
            Err(_) => false,
        }
    }

    /// How many words the key has, which is the length of the sample the
    /// demo hands out for it: the words made, else the count chosen.
    pub fn handout_words(&self) -> u8 {
        let n = self
            .mnemonic
            .as_ref()
            .map_or(self.words, faraday_bip::bip39::Mnemonic::word_count);
        u8::try_from(n).unwrap_or(24)
    }

    /// The words, space-separated, for the session to load.
    pub fn phrase(&self) -> Option<zeroize::Zeroizing<String>> {
        let m = self.mnemonic.as_ref()?;
        let mut s = crate::secret_text::room();
        for (k, &i) in m.indices().iter().enumerate() {
            if k > 0 {
                s.push(' ');
            }
            s.push_str(Language::English.word(i));
        }
        Some(s)
    }

    /// Whether a mix has the two sources it needs.
    pub fn mix_enough(&self) -> bool {
        self.mix.iter().filter(|on| **on).count() >= MIN_MIX
    }
}

/// New key's area: the flow being made, the keys made this session, the
/// camera and the shell's randomness source.
#[derive(Default)]
pub(crate) struct KeyGenArea {
    state: Option<KeyGen>,
    /// New keys made this session, which number their labels.
    new_keys: u32,
    /// The camera is on for camera noise.
    camera_on: bool,
    /// The shell's randomness source (`Faraday::entropy_source_named`).
    entropy_source: Option<&'static str>,
}

impl KeyGenArea {
    /// The flow being made, while it is open.
    pub(crate) fn state(&self) -> Option<&KeyGen> {
        self.state.as_ref()
    }

    /// Forgets the flow, with everything in it.
    pub(crate) fn forget(&mut self) {
        self.state = None;
    }

    /// Whether the camera is on for camera noise.
    pub(crate) fn camera_on(&self) -> bool {
        self.camera_on
    }

    /// The column's scroll, while the flow is open.
    pub(crate) fn scroll_mut(&mut self) -> Option<&mut flow::Scroll> {
        self.state.as_mut().map(|k| &mut k.scroll)
    }

    /// Takes the column's scroll as the pass left it.
    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        if let Some(k) = self.state.as_mut() {
            k.scroll = to;
        }
    }

    /// Takes a camera frame as noise, forgetting the one before.
    pub(crate) fn camera_frame(&mut self, luma: Vec<u8>) {
        if let Some(k) = self.state.as_mut() {
            if let Some(old) = k.frame.as_mut() {
                zeroize::Zeroize::zeroize(old);
            }
            k.frame = Some(luma);
        }
    }

    /// Demo's passphrase typed into field `i` of the Key card.
    pub(crate) fn demo_passphrase(&mut self, i: u8, text: &str) {
        if let Some(k) = self.state.as_mut() {
            let t = if i == 0 {
                &mut k.passphrase
            } else {
                &mut k.passphrase2
            };
            t.set(text);
            k.focus = Some(i);
            k.note = None;
        }
    }

    /// The shell named its randomness source: kept, and copied into an
    /// open flow, as a new one copies it when it opens.
    pub(crate) fn entropy_source_named(&mut self, name: &'static str) {
        self.entropy_source = Some(name);
        if let Some(k) = self.state.as_mut() {
            k.entropy_source = Some(name);
        }
    }

    /// The shell is asked for fresh bytes when this device's generator
    /// is the source being entered; they go to the key and nowhere else.
    pub(crate) fn device(&mut self, out: &mut crate::out::Out) {
        let Some(k) = self.state.as_mut() else {
            return;
        };
        if k.open == Some(kstep::ENTER)
            && k.active() == Some(Source::Device)
            && !k.device.is_enough(k.strength())
            && !k.device_asked
        {
            k.device_asked = true;
            out.command(faraday_shell_api::Command::RequestEntropy);
        }
    }

    /// Takes the shell's answer when the flow asked for it. Returns
    /// whether it did.
    pub(crate) fn entropy(&mut self, bytes: &mut [u8; 32]) -> bool {
        let Some(k) = self.state.as_mut().filter(|k| k.device_asked) else {
            return false;
        };
        k.device.set(bytes);
        k.device_asked = false;
        zeroize::Zeroize::zeroize(bytes);
        true
    }

    /// The camera is on while camera noise is being taken, and off
    /// otherwise.
    pub(crate) fn camera(&mut self, on_screen: bool, out: &mut crate::out::Out) {
        let want = on_screen
            && self.state.as_ref().is_some_and(|k| {
                k.open == Some(kstep::ENTER) && k.active() == Some(Source::Camera)
            });
        if want != self.camera_on {
            self.camera_on = want;
            out.command(if want {
                faraday_shell_api::Command::CameraOn
            } else {
                faraday_shell_api::Command::CameraOff
            });
        }
        if !want
            && let Some(k) = self.state.as_mut()
            && let Some(f) = k.frame.as_mut()
        {
            zeroize::Zeroize::zeroize(f);
            k.frame = None;
        }
    }

    /// A press that changes only the flow. `draw` is the fresh draw
    /// dealing shares takes, asked for before the flow is borrowed.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    pub(crate) fn act(
        &mut self,
        action: crate::Action,
        c: &Compute,
        draw: Option<[u8; 32]>,
        now_ms: u64,
    ) {
        use crate::Action as A;
        let Some(k) = self.state.as_mut() else {
            return;
        };
        let open = |k: &mut KeyGen, step: u8| {
            k.open = Some(step);
            k.scroll.follow = true;
        };
        match action {
            A::KStart(i) => {
                if let Some(&s) = STARTS.get(usize::from(i))
                    && k.open == Some(kstep::START)
                    && k.starts().contains(&s)
                {
                    k.start = s;
                    k.note = None;
                }
            }
            A::KOthers => k.others_open = !k.others_open,
            A::KPassField(f) => {
                if k.open == Some(kstep::KEY) && k.asks_passphrase() && f < 2 {
                    k.focus = Some(f);
                    k.note = None;
                }
            }
            A::KStep(s) => {
                // A later card opens once the ones before it in this
                // flow are done.
                let steps = k.steps();
                let reachable = steps
                    .iter()
                    .take_while(|&&i| i != s)
                    .all(|&i| k.done[usize::from(i)])
                    && steps.contains(&s);
                if k.open == Some(s) {
                    k.open = None;
                } else if reachable {
                    open(k, s);
                }
            }
            A::KForm(on) => k.set_slip39(on),
            A::KSlipWords(n) => {
                if k.slip39 && k.counts().contains(&n) && usize::from(n) != k.words {
                    k.words = usize::from(n);
                    k.clear_entries();
                }
            }
            A::KSlipM(d) => {
                k.slip_m = (i16::from(k.slip_m) + i16::from(d)).clamp(2, i16::from(k.slip_n)) as u8;
            }
            A::KSlipN(d) => {
                let max = faraday_bip::slip39::MAX_COUNT as i16;
                k.slip_n = (i16::from(k.slip_n) + i16::from(d)).clamp(2, max) as u8;
                k.slip_m = k.slip_m.min(k.slip_n);
            }
            A::KShare(d) => {
                let last = k.shares.len().saturating_sub(1) as i64;
                k.share_at = (k.share_at as i64 + i64::from(d)).clamp(0, last) as usize;
                k.shown = false;
            }
            A::KWords(n) => {
                let n = usize::from(n);
                if k.counts().iter().any(|c| usize::from(*c) == n) && !(k.only_24 && n != 24) {
                    k.words = n;
                    k.clear_entries();
                    k.done[usize::from(kstep::LENGTH)] = true;
                    open(k, kstep::SOURCE);
                }
            }
            A::KWay(i) => {
                if let Some(&w) = WAYS.get(usize::from(i)) {
                    k.choose(w);
                }
            }
            A::KGroup(i) => {
                if let Some(open) = k.groups_open.get_mut(usize::from(i)) {
                    *open = !*open;
                }
            }
            A::KMix(i) => {
                if let Some(on) = k.mix.get_mut(usize::from(i)) {
                    *on = !*on;
                    k.clear_entries();
                }
            }
            A::KNext => match k.open {
                Some(kstep::SOURCE) => {
                    let ok = match k.source {
                        Some(Source::Mix) => k.mix_enough(),
                        Some(_) => true,
                        None => false,
                    };
                    if ok {
                        k.done[usize::from(kstep::SOURCE)] = true;
                        open(k, kstep::ENTER);
                    } else {
                        k.note = Some(if k.source.is_some() {
                            EN.keygen_choose_two_sources.to_string()
                        } else {
                            EN.keygen_choose_a_source.to_string()
                        });
                    }
                }
                Some(kstep::ENTER) => {
                    if k.slip39 && draw.is_none() {
                        k.note = Some(EN.keygen_no_randomness.to_string());
                        return;
                    }
                    if k.finish_entry() {
                        if k.slip39 {
                            if let Err(e) = draw.map_or(Ok(()), |r| k.deal(c, r)) {
                                k.note = Some(e);
                                return;
                            }
                            k.fingerprint = k.secret.as_ref().and_then(|s| {
                                faraday_keys::identity::fingerprint_of_seed(c, s).map(|f| f.0)
                            });
                        } else {
                            k.fingerprint = k.phrase().and_then(|w| {
                                faraday_keys::identity::fingerprint_of_words(c, &w, "")
                                    .ok()
                                    .map(|f| f.0)
                            });
                        }
                        k.done[usize::from(kstep::ENTER)] = true;
                        if k.slip39 {
                            open(k, kstep::CHECK);
                        } else {
                            // Words that showed as they came in stay on
                            // screen; hashed ones wait for Show words.
                            k.shown = k.reveals();
                            open(k, kstep::KEY);
                        }
                    }
                }
                Some(kstep::CHECK) => {
                    k.done[usize::from(kstep::CHECK)] = true;
                    open(k, kstep::WORDS);
                }
                Some(kstep::WORDS) if k.slip39 => {
                    // Each share is quizzed in turn, from the first.
                    if let Some(ind) = k.share_indices(0) {
                        let seed = crate::quiz::Quiz::seed(now_ms, ind);
                        k.quiz = Some(crate::quiz::Quiz::new(
                            ind,
                            crate::word_entry::EntryList::Slip39,
                            seed,
                        ));
                        k.quiz_share = 0;
                        k.shown = false;
                        k.done[usize::from(kstep::WORDS)] = true;
                        open(k, kstep::QUIZ);
                    }
                }
                Some(kstep::WORDS) => {
                    // The quiz asks every word in a random order; the
                    // words are hidden while it runs.
                    if let Some(m) = k.mnemonic.as_ref() {
                        let seed = crate::quiz::Quiz::seed(now_ms, m.indices());
                        k.quiz = Some(crate::quiz::Quiz::new(
                            m.indices(),
                            crate::word_entry::EntryList::Bip39(Language::English),
                            seed,
                        ));
                        k.shown = false;
                        k.done[usize::from(kstep::WORDS)] = true;
                        open(k, kstep::QUIZ);
                    }
                }
                _ => {}
            },
            A::KRoll(f) => k.roll(f),
            A::KFlip(h) => k.flip(h),
            A::KDie(f) => k.die_flip(f),
            A::KByDie(on) => k.by_die = on,
            A::KTyping(on) => k.typing = on,
            A::KRank(r) => k.card(Some(r), None),
            A::KSuit(s) => k.card(None, Some(s)),
            A::KHex(v) => {
                if let Some(c) = char::from_digit(u32::from(v), 16) {
                    k.hex_digit(c);
                }
            }
            A::KFrame => k.take_frame(),
            A::KUndo => k.undo(),
            A::KClear => k.clear_active(),
            A::KShow => {
                if matches!(k.open, Some(kstep::WORDS | kstep::KEY)) {
                    k.shown = !k.shown;
                }
            }
            A::KQuiz(slot) if k.slip39 => {
                let list = crate::word_entry::EntryList::Slip39;
                let at = k.quiz_share;
                let ind: Vec<u16> = k.share_indices(at).map(<[u16]>::to_vec).unwrap_or_default();
                let passed = match k.quiz.as_mut() {
                    Some(q) => {
                        q.pick(usize::from(slot));
                        q.confirm(&ind, list);
                        q.state() == crate::quiz::QuizState::Passed
                    }
                    None => false,
                };
                if passed {
                    // The next share, or done.
                    match k.share_indices(at + 1).map(<[u16]>::to_vec) {
                        Some(next) => {
                            let seed = crate::quiz::Quiz::seed(now_ms, &next);
                            k.quiz = Some(crate::quiz::Quiz::new(&next, list, seed));
                            k.quiz_share = at + 1;
                        }
                        None => k.done[usize::from(kstep::QUIZ)] = true,
                    }
                }
            }
            A::KQuizRetry if k.slip39 => {
                let ind: Vec<u16> = k
                    .share_indices(k.quiz_share)
                    .map(<[u16]>::to_vec)
                    .unwrap_or_default();
                if let Some(q) = k.quiz.as_mut() {
                    q.retry(&ind, crate::word_entry::EntryList::Slip39);
                }
            }
            A::KQuiz(slot) => {
                if let (Some(q), Some(m)) = (k.quiz.as_mut(), k.mnemonic.as_ref()) {
                    let list = crate::word_entry::EntryList::Bip39(Language::English);
                    q.pick(usize::from(slot));
                    q.confirm(m.indices(), list);
                    if q.state() == crate::quiz::QuizState::Passed {
                        k.done[usize::from(kstep::QUIZ)] = true;
                    }
                }
            }
            A::KQuizRetry => {
                if let (Some(q), Some(m)) = (k.quiz.as_mut(), k.mnemonic.as_ref()) {
                    q.retry(
                        m.indices(),
                        crate::word_entry::EntryList::Bip39(Language::English),
                    );
                }
            }
            A::KSkip => {
                if k.skip_ask {
                    k.skipped = true;
                    k.done[usize::from(kstep::QUIZ)] = true;
                } else {
                    k.skip_ask = true;
                }
            }
            A::KAgain => {
                k.clear_entries();
                open(k, kstep::ENTER);
            }
            _ => {}
        }
    }
}

impl crate::Faraday {
    /// The flow being made, while it is open.
    pub fn keygen(&self) -> Option<&KeyGen> {
        self.keygen.state()
    }

    /// Opens the flow, for a Create slot or for the session.
    pub(crate) fn keygen_open(&mut self, slot: Option<u8>) {
        if !self.may_load_keys() {
            self.toast(&fill(EN.keygen_remove_the_first, &[self.medium.noun()]));
            return;
        }
        // A flow already open ends first, giving back what it kept.
        self.keygen_drop();
        let only_24 = slot.is_some() && self.create.as_ref().is_some_and(|c| c.kind.threshold());
        let back = self.screen;
        let mut k = KeyGen::new(slot, back, only_24);
        k.entropy_source = self.keygen.entropy_source;
        // A small panel is a touch screen with no keyboard of its own:
        // the faces are pressed, not typed.
        k.typing = !self.compact;
        // Add a key's own state waits in the flow, which types into the
        // app's; it comes back when the flow ends.
        k.entry_kept = self.entry.take();
        // Add a key's Make a new key chose already: Start is closed on it
        // and Length is open, as before Start was (`docs/history/NEW-WALLET.md`
        // §15.3).
        if back == Screen::Entry {
            k.done[usize::from(kstep::START)] = true;
            k.started = Some(Start::Make);
            k.open = Some(kstep::LENGTH);
        }
        self.keygen.state = Some(k);
        self.screen = Screen::KeyGen;
    }

    /// Opens the flow for the session with `start` chosen and Start
    /// closed on it, as the Tools tiles for one form do (§15.3).
    pub(crate) fn keygen_open_on(&mut self, start: Start) {
        self.keygen_open(None);
        if let Some(k) = self.keygen.state.as_mut() {
            k.start = start;
            k.others_open = start.other();
        }
        self.keygen_start_next();
    }

    /// Start's Continue: the cards for the choice open. A choice other
    /// than the one the later cards were opened for starts them over.
    fn keygen_start_next(&mut self) {
        let compact = self.compact;
        let Some(k) = self.keygen.state.as_mut() else {
            return;
        };
        if k.locked || !k.starts().contains(&k.start) {
            return;
        }
        let start = k.start;
        if k.started != Some(start) {
            k.clear_entries();
            k.started = None;
            k.parts_in = 0;
            k.passphrase.clear();
            k.passphrase2.clear();
            k.focus = None;
            for s in [kstep::TYPED, kstep::KEY] {
                k.done[usize::from(s)] = false;
            }
            if start.makes() {
                k.set_slip39(start == Start::MakeShares);
            }
            // What was typed for another choice goes.
            let mut e = crate::EntryState::default();
            if let Some(f) = start.form() {
                e.form = f;
            }
            // A small panel has no keyboard of its own: the words go on
            // the word keyboard, as on Add a key.
            if compact && e.form == Form::Words {
                e.keys = Some(crate::forms::word_typer(e.language()));
            }
            self.entry.put(e);
        }
        let Some(k) = self.keygen.state.as_mut() else {
            return;
        };
        let next = match start {
            // The scan sheet; the card after it opens once a code is read.
            Start::Scan => {
                self.act(crate::Action::ScanSeed);
                return;
            }
            Start::Make | Start::MakeShares => kstep::LENGTH,
            _ => kstep::TYPED,
        };
        k.done[usize::from(kstep::START)] = true;
        k.started = Some(start);
        k.open = Some(next);
        k.scroll.follow = true;
    }

    /// A SeedQR read while the flow waits on Scan a SeedQR: the camera
    /// closes and Key opens with its words. Returns whether it took them.
    pub(crate) fn keygen_scanned(&mut self, words: &str) -> bool {
        let c = self.compute();
        if self.screen != Screen::KeyGen {
            return false;
        }
        let Some(k) = self
            .keygen
            .state
            .as_mut()
            .filter(|k| k.start == Start::Scan && !k.locked)
        else {
            return false;
        };
        let Ok(m) = crate::wallet::parse_words(&c, Language::English, words) else {
            return false;
        };
        k.mnemonic = Some(m);
        k.shown = false;
        k.done[usize::from(kstep::START)] = true;
        k.started = Some(Start::Scan);
        k.open = Some(kstep::KEY);
        k.scroll.follow = true;
        self.camera.close();
        self.sheet = None;
        self.out
            .commands
            .push_back(faraday_shell_api::Command::CameraOff);
        true
    }

    /// The Words card's Continue for a key typed: Key opens once the
    /// words are whole and their checksum holds, or the parts are enough,
    /// with Add a key's own messages when they are not (§15.2).
    fn keygen_typed_next(&mut self) {
        let Some(k) = self.keygen.state.as_ref() else {
            return;
        };
        let Some(form) = k.start.form() else {
            return;
        };
        let only_24 = k.only_24;
        let mut words = None;
        if form == Form::Words {
            let m = match self.entry.mnemonic() {
                Ok(m) => m,
                Err(e) => {
                    self.entry.state_mut().error = Some(e);
                    return;
                }
            };
            if only_24 && m.word_count() != 24 {
                self.entry.state_mut().error = Some(EN.keygen_a_threshold.to_string());
                return;
            }
            words = Some(m);
        } else if !self.entry.state().parts.ready(form) {
            return;
        }
        let parts = self.entry.state().parts.lines(form).len();
        self.entry.state_mut().error = None;
        let Some(k) = self.keygen.state.as_mut() else {
            return;
        };
        k.mnemonic = words;
        k.parts_in = parts;
        k.shown = false;
        k.done[usize::from(kstep::TYPED)] = true;
        k.open = Some(kstep::KEY);
        k.scroll.follow = true;
    }

    /// One press inside the flow.
    pub(crate) fn keygen_act(&mut self, action: crate::Action) {
        let c = self.compute();
        use crate::Action as A;
        if let A::KeyGen(slot) = action {
            self.keygen_open(slot);
            return;
        }
        // In the demo SLIP-39's sources and its deal do nothing
        // (`docs/DEMO.md` §4), however the press arrives.
        if self.demo_refuses(action) {
            return;
        }
        if action == A::KeyGenSlip39 {
            self.keygen_open_on(Start::MakeShares);
            return;
        }
        // Dealing shares takes a fresh draw of its own from the system's
        // generator; asked for before the flow is borrowed.
        let dealing = action == A::KNext
            && self
                .keygen
                .state()
                .is_some_and(|k| k.slip39 && k.open == Some(kstep::ENTER));
        let draw = if dealing {
            self.fresh(b"slip39 split")
        } else {
            None
        };
        if action == A::KAdd {
            self.keygen_add();
            return;
        }
        let Some(k) = self.keygen.state.as_ref() else {
            return;
        };
        // After lock-in nothing about the key changes: the cards before
        // Key open to be read, and take no input (`docs/history/NEW-WALLET.md`
        // §3.3).
        let inputs_closed = k.locked
            && match action {
                A::KStep(_) | A::KShow | A::KQuiz(_) | A::KQuizRetry | A::KSkip => false,
                A::KNext => matches!(
                    k.open,
                    Some(
                        kstep::START | kstep::LENGTH | kstep::SOURCE | kstep::ENTER | kstep::TYPED
                    ) | None
                ),
                _ => true,
            };
        if inputs_closed {
            return;
        }
        if action == A::KNext && k.open == Some(kstep::START) {
            self.keygen_start_next();
            return;
        }
        if action == A::KNext && k.open == Some(kstep::TYPED) {
            self.keygen_typed_next();
            return;
        }
        if action == A::KLock || (action == A::KNext && k.open == Some(kstep::KEY)) {
            if k.locked {
                self.keygen_continue();
            } else {
                self.keygen_lock();
            }
            return;
        }
        self.keygen.act(action, &c, draw, self.now_ms);
    }

    /// The Quiz's last button. SLIP-39 shares: loads the key and leaves
    /// the flow. BIP-39 words, added at Lock in already: leaves the flow
    /// (**Done**).
    fn keygen_add(&mut self) {
        let c = self.compute();
        // No key is accepted before the self-test has passed.
        if !self.selftest_passed() {
            return;
        }
        let Some(k) = self.keygen.state.as_ref() else {
            return;
        };
        // The words were checked by the quiz, or the person said twice to
        // go without it.
        let passed = k
            .quiz
            .as_ref()
            .is_some_and(|q| q.state() == crate::quiz::QuizState::Passed);
        if !(passed || k.skipped) {
            return;
        }
        if !k.slip39 {
            if k.locked {
                self.keygen_done();
            }
            return;
        }
        let slot = k.slot;
        let label = fill(
            EN.keygen_new_key_n,
            &[&(self.keygen.new_keys + 1).to_string()],
        );
        // The demo adds its next sample of this length in place of the key
        // just made, which goes unused (`Keys::next_handout`).
        let handed = self
            .session
            .keys
            .next_handout(k.handout_words())
            .map(|(w, _)| (w, k.handout_words()));
        let added = if let Some((word, n)) = handed {
            self.session
                .keys
                .add_words(&c, &crate::samples::repeated(word, n), &label, None)
        } else {
            match k.secret.as_ref() {
                Some(s) => {
                    let s = s.clone();
                    self.session.keys.add_seed(&c, &s, &label)
                }
                None => return,
            }
        };
        match added {
            Ok(fp) => {
                self.keygen.new_keys += 1;
                if let Some(slot) = slot
                    && let Some(c) = self.create.as_mut()
                {
                    c.made += 1;
                    if let Some(s) = c.slots.get_mut(usize::from(slot)) {
                        *s = crate::create::Source::Here(fp.0);
                    }
                }
                if let Some(k) = self.keygen.state.as_mut() {
                    k.fingerprint = Some(fp.0);
                    k.handed = handed;
                }
                self.refresh_spend();
                self.keygen_done();
            }
            Err(e) => {
                if let Some(k) = self.keygen.state.as_mut() {
                    k.note = Some(e.text());
                }
            }
        }
    }

    /// Lock in, on the Key card: adds the key the words make, with the
    /// passphrase typed for a Create slot, under the self-test gate. For
    /// a slot it fills the slot, and on a single-key Create it makes the
    /// wallet too (`docs/history/NEW-WALLET.md` §3.3).
    fn keygen_lock(&mut self) {
        let c = self.compute();
        if !self.selftest_passed() || self.demo_refuses(crate::Action::KLock) {
            return;
        }
        if self.keygen.state().is_some_and(KeyGen::typed) {
            self.keygen_lock_typed();
            return;
        }
        let Some(k) = self.keygen.state.as_mut() else {
            return;
        };
        if k.slip39 || k.locked || !k.done[usize::from(kstep::ENTER)] {
            return;
        }
        let Some(words) = k.phrase() else {
            return;
        };
        // The demo adds its next sample of this length in place of the key
        // just made, which goes unused, with the passphrase chosen on the
        // Key card, which is `faraday` (`Keys::next_handout`, F20,
        // `docs/MODES.md` §7).
        let handed = self
            .session
            .keys
            .next_handout(k.handout_words())
            .map(|(w, _)| (w, k.handout_words()));
        if k.slot.is_some() && k.passphrase.as_str() != k.passphrase2.as_str() {
            k.note = Some(EN.keygen_the_two_passphrases.to_string());
            return;
        }
        // New key on its own makes the key alone: no passphrase
        // (decision 4).
        let mut pass = crate::secret_text::room();
        if k.slot.is_some() {
            pass.push_str(k.passphrase.as_str());
        }
        let slot = k.slot;
        let label = fill(
            EN.keygen_new_key_n,
            &[&(self.keygen.new_keys + 1).to_string()],
        );
        let added = if let Some((word, n)) = handed {
            self.session.keys.add_words_with(
                &c,
                &crate::samples::repeated(word, n),
                &pass,
                &label,
                None,
            )
        } else {
            self.session
                .keys
                .add_words_with(&c, &words, &pass, &label, None)
        };
        let fp = match added {
            Ok(fp) => fp,
            Err(e) => {
                if let Some(k) = self.keygen.state.as_mut() {
                    k.note = Some(e.text());
                }
                return;
            }
        };
        // Made here: it signs once its backup is done (`docs/history/NEW-WALLET.md`
        // §14.3).
        self.mark_made_here(fp);
        self.keygen.new_keys += 1;
        // A Create slot's key at the account Create uses (NEW-WALLET §16.2).
        let (kind, account) = match slot {
            Some(_) => self
                .create
                .as_ref()
                .map(|c| (c.kind, c.account))
                .unwrap_or_default(),
            None => (crate::create::NewKind::NativeSegwit, 0),
        };
        let key_text = self
            .session
            .keys
            .position(fp)
            .and_then(|key| self.session.keys.account_text(&c, key, kind, account).ok());
        if let Some(k) = self.keygen.state.as_mut() {
            k.locked = true;
            k.with_passphrase = !pass.is_empty();
            k.handed = handed;
            k.fingerprint = Some(fp.0);
            k.key_text = key_text;
            k.passphrase.clear();
            k.passphrase2.clear();
            k.focus = None;
            k.note = None;
            k.scroll.follow = true;
        }
        self.keygen_fill(slot, fp, true);
        self.refresh_spend();
    }

    /// A key locked in for Create slot `slot`, if any, fills it; on a
    /// single-key Create it makes the wallet too, as Keys' Continue
    /// would, and Check is what the Key card shows (`docs/history/NEW-WALLET.md`
    /// §3.3). `made`: the key was made here, not typed.
    fn keygen_fill(&mut self, slot: Option<u8>, fp: faraday_bip::keys::Fingerprint, made: bool) {
        let Some(slot) = slot else {
            return;
        };
        let Some(c) = self.create.as_mut() else {
            return;
        };
        if made {
            c.made += 1;
        }
        if let Some(s) = c.slots.get_mut(usize::from(slot)) {
            *s = crate::create::Source::Here(fp.0);
        }
        if !c.kind.multi() && c.built.is_none() {
            self.create_make();
            if let Some(c) = self.create.as_mut()
                && c.built.is_some()
            {
                c.done[usize::from(crate::cstep::KEYS)] = true;
                c.done[usize::from(crate::cstep::CHECK)] = true;
                c.open = Some(crate::cstep::BACKUP);
                c.scroll.follow = true;
            }
            if let Some(w) = self.create.as_ref().and_then(|c| c.built)
                && let Some(wallet) = self.session.wallets.get(w)
            {
                let checksum = wallet.checksum().to_string();
                self.mark_checked(&checksum);
            }
        }
    }

    /// Lock in for a key typed or scanned: loaded as Add a key loads it
    /// (`Faraday::typed_load`, or the form's own for shares, codex32 and
    /// Seed XOR), with the passphrase typed on the Key card, not marked
    /// made here; for a Create slot it fills the slot (§15.2).
    fn keygen_lock_typed(&mut self) {
        let c = self.compute();
        if !self.may_load_keys() {
            return;
        }
        let Some(k) = self.keygen.state.as_mut() else {
            return;
        };
        if k.locked {
            return;
        }
        let form = k.start.form().unwrap_or(Form::Words);
        let ready = match k.start {
            Start::Scan => k.mnemonic.is_some(),
            _ => k.done[usize::from(kstep::TYPED)],
        };
        if !ready {
            return;
        }
        let asks = k.asks_passphrase();
        if asks && k.passphrase.as_str() != k.passphrase2.as_str() {
            k.note = Some(EN.keygen_the_two_passphrases.to_string());
            return;
        }
        let mut pass = crate::secret_text::room();
        if asks {
            pass.push_str(k.passphrase.as_str());
        }
        let slot = k.slot;
        let words = match k.mnemonic.as_ref() {
            Some(m) => faraday_keys::words::from_indices(m.language(), m.indices()).ok(),
            None => None,
        };
        let added = match words {
            Some(m) => self
                .typed_load(&m, &pass, "", None, None, |_, _| {})
                .map_err(|e| e.text()),
            None if form != Form::Words => {
                self.entry.state_mut().passphrase.set(&pass);
                self.form_key()
            }
            None => return,
        };
        let fp = match added {
            Ok(fp) => fp,
            Err(e) => {
                if let Some(k) = self.keygen.state.as_mut() {
                    k.note = Some(e);
                }
                return;
            }
        };
        // A Create slot's key at the account Create uses (NEW-WALLET §16.2).
        let (kind, account) = match slot {
            Some(_) => self
                .create
                .as_ref()
                .map(|c| (c.kind, c.account))
                .unwrap_or_default(),
            None => (crate::create::NewKind::NativeSegwit, 0),
        };
        let key_text = self
            .session
            .keys
            .position(fp)
            .and_then(|key| self.session.keys.account_text(&c, key, kind, account).ok());
        // What was typed goes; the Key card keeps the words to show.
        self.entry.reset();
        if let Some(k) = self.keygen.state.as_mut() {
            k.locked = true;
            k.with_passphrase = !pass.is_empty();
            k.fingerprint = Some(fp.0);
            k.key_text = key_text;
            k.passphrase.clear();
            k.passphrase2.clear();
            k.focus = None;
            k.note = None;
            k.scroll.follow = true;
        }
        self.keygen_fill(slot, fp, false);
        self.refresh_spend();
    }

    /// The Key card's Continue, once locked in: back to Create for a
    /// slot, else on to Words (`docs/history/NEW-WALLET.md` §3.4).
    fn keygen_continue(&mut self) {
        let Some(k) = self.keygen.state.as_mut() else {
            return;
        };
        k.done[usize::from(kstep::KEY)] = true;
        // A key typed or scanned has its words already: no Words or Quiz
        // (§15.2).
        if k.slot.is_some() || k.typed() {
            self.keygen_done();
            return;
        }
        k.open = Some(kstep::WORDS);
        k.shown = false;
        k.scroll.follow = true;
    }

    /// Leaves the flow with its key added: back where it was opened, a
    /// multisig Create on Keys, with the toast that says to back it up.
    fn keygen_done(&mut self) {
        let Some(k) = self.keygen_drop() else {
            return;
        };
        self.return_back(k.back);
        if k.slot.is_some()
            && let Some(c) = self.create.as_mut()
            && c.built.is_none()
        {
            c.open = Some(crate::cstep::KEYS);
            c.scroll.follow = true;
        }
        if let Some(fp) = k.fingerprint.filter(|_| k.typed()) {
            // A key typed: Add a key's own toast.
            self.toast(&fill(
                EN.entry_add_key_added,
                &[&crate::wallet::fp_text(faraday_bip::keys::Fingerprint(fp)).to_string()],
            ));
        } else if let Some(fp) = k.fingerprint {
            self.toast(&fill(
                EN.keygen_key_added_back,
                &[
                    &crate::wallet::fp_text(faraday_bip::keys::Fingerprint(fp)).to_string(),
                    (if k.slip39 {
                        EN.keygen_shares_inline
                    } else {
                        EN.keygen_words_inline
                    }),
                ],
            ));
        }
    }

    /// Leaves the flow, forgetting everything in it.
    pub(crate) fn keygen_leave(&mut self) {
        if let Some(k) = self.keygen_drop() {
            self.return_back(k.back);
        }
    }

    /// Ends the flow: Add a key's own state comes back in place of what
    /// the flow typed, which goes. Returns the flow, to be dropped.
    pub(crate) fn keygen_drop(&mut self) -> Option<KeyGen> {
        let mut k = self.keygen.state.take()?;
        self.entry.put(std::mem::take(&mut k.entry_kept));
        Some(k)
    }

    /// The shell is asked for fresh bytes when this device's generator
    /// is the source being entered; they go to the key and nowhere else.
    pub(crate) fn keygen_device(&mut self) {
        self.keygen.device(&mut self.out);
    }

    /// The shell named what it reads its randomness from
    /// (`Event::EntropySource`).
    pub(crate) fn entropy_source_named(&mut self, name: &'static str) {
        self.keygen.entropy_source_named(name);
    }

    /// The shell's answer to a `RequestEntropy`. Asked for by New key,
    /// the bytes are the key's, and the session's own seed stays as it
    /// was; otherwise they are the session's fresh seed.
    pub(crate) fn entropy_arrived(&mut self, e: faraday_shell_api::EntropyBytes) {
        let mut bytes = e.into_bytes();
        if !self.keygen_entropy(&mut bytes) {
            self.fresh_arrived(bytes);
            zeroize::Zeroize::zeroize(&mut bytes);
        }
    }

    /// Takes the shell's answer when the flow asked for it. Returns
    /// whether it did.
    pub(crate) fn keygen_entropy(&mut self, bytes: &mut [u8; 32]) -> bool {
        self.keygen.entropy(bytes)
    }

    /// The camera is on while camera noise is being taken, and off
    /// otherwise.
    pub(crate) fn keygen_camera(&mut self) {
        self.keygen
            .camera(self.screen == Screen::KeyGen, &mut self.out);
    }

    /// Keys typed while the flow is on screen. Returns whether it took
    /// the key.
    pub(crate) fn keygen_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as KeyIn;
        if self.screen != Screen::KeyGen {
            return false;
        }
        let Some(k) = self.keygen.state.as_mut() else {
            return false;
        };
        // A passphrase field on the Key card takes printable ASCII, as
        // BIP-39's seed derivation reads it; Tab moves to the other field and
        // Enter locks in.
        if k.open == Some(kstep::KEY)
            && !k.locked
            && k.asks_passphrase()
            && let Some(f) = k.focus
        {
            let field = if f == 0 {
                &mut k.passphrase
            } else {
                &mut k.passphrase2
            };
            match key {
                KeyIn::Char(c) if (' '..='~').contains(&c) => field.push(c),
                KeyIn::Backspace => {
                    field.pop();
                }
                KeyIn::Tab => k.focus = Some(1 - f.min(1)),
                KeyIn::Escape => k.focus = None,
                KeyIn::Enter => {
                    self.keygen_lock();
                    return true;
                }
                _ => return false,
            }
            k.note = None;
            return true;
        }
        if key == KeyIn::Escape {
            self.keygen_leave();
            return true;
        }
        // The Words card of a key typed: Add a key's own typing reads the
        // keys (`Faraday::key`), and its Enter is the card's Continue.
        if k.open == Some(kstep::TYPED) && !k.locked {
            return false;
        }
        // Typing into the box: each character is taken as it comes, and
        // the box keeps every one, so nothing else on the page reads it.
        // Backspace and Enter are the page's, as with the buttons.
        if k.open == Some(kstep::ENTER)
            && !k.locked
            && k.typing
            && k.flip_or_roll()
            && let KeyIn::Char(c) = key
        {
            k.type_char(c);
            return true;
        }
        if key == KeyIn::Enter {
            if k.open == Some(kstep::QUIZ) {
                self.keygen_add();
            } else {
                self.keygen_act(crate::Action::KNext);
            }
            return true;
        }
        if k.open == Some(kstep::QUIZ) {
            if let KeyIn::Char(c @ '1'..='4') = key {
                self.keygen_act(crate::Action::KQuiz(c as u8 - b'1'));
                return true;
            }
            return false;
        }
        if k.open != Some(kstep::ENTER) || k.locked {
            return false;
        }
        match (k.active(), key) {
            (_, KeyIn::Backspace) => k.undo(),
            (Some(Source::Dice), KeyIn::Char(c @ '1'..='6')) => k.roll(c as u8 - b'0'),
            (Some(Source::Coins), KeyIn::Char(c @ '1'..='6')) if k.by_die => {
                k.die_flip(c as u8 - b'0')
            }
            (Some(Source::Coins), KeyIn::Char('h' | 'H' | '1')) => k.flip(true),
            (Some(Source::Coins), KeyIn::Char('t' | 'T' | '0')) => k.flip(false),
            (Some(Source::Hex), KeyIn::Char(c)) if c.is_ascii_hexdigit() => k.hex_digit(c),
            (Some(Source::Cards), KeyIn::Char(c)) => {
                let c = c.to_ascii_uppercase();
                if let Some(r) = faraday_entropy::RANKS.iter().position(|x| *x == c) {
                    k.card(Some(r as u8), None);
                } else if let Some(s) = ['S', 'H', 'D', 'C'].iter().position(|x| *x == c) {
                    k.card(None, Some(s as u8));
                } else {
                    return false;
                }
            }
            (Some(Source::Camera), KeyIn::Char(' ')) => k.take_frame(),
            _ => return false,
        }
        true
    }
}
