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
    /// Make codex32 shares' own cards (`docs/CODEX32.md` §6), drawn as
    /// a card each by [`crate::codex32`].
    pub const CODEX32: u8 = 9;
    /// How many cards.
    pub const COUNT: usize = 10;
}

/// SHA-256("faraday device own" ‖ `bytes` ‖ `acc`): This device's
/// bytes with what the person added of their own mixed in
/// (`docs/OWN-RANDOMNESS.md` §2.3). Someone who lacks either part cannot
/// predict it.
pub(crate) fn device_own(bytes: &[u8; 32], acc: &[u8; 32]) -> zeroize::Zeroizing<[u8; 32]> {
    use faraday_bip::bitcoin::hashes::{Hash, HashEngine, sha256};
    let mut e = sha256::Hash::engine();
    e.input(b"faraday device own");
    e.input(bytes);
    e.input(acc);
    zeroize::Zeroizing::new(sha256::Hash::from_engine(e).to_byte_array())
}

mod area;
mod entries;
mod start;
pub(crate) use area::KeyGenArea;
pub(crate) mod steps;
pub use start::{STARTS, Start};

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
    /// Make codex32 shares is offered: a key on its own, or for a
    /// single-key wallet's slot (`docs/CODEX32.md` §6.9).
    pub codex32_offered: bool,
    /// Make codex32 shares, while it is the Start card's choice and its
    /// cards are open (`docs/CODEX32.md` §6).
    pub codex32: Option<crate::codex32::Codex32Make>,
    /// Add a key's own state, kept here while the flow types into the
    /// app's (`Faraday::entry`), and put back when the flow ends.
    pub entry_kept: crate::EntryState,
    /// What This device's randomness is read from, copied from
    /// `Faraday::entropy_source` when the flow opens and kept in step by
    /// `Event::EntropySource`; `None` when the shell named none.
    pub entropy_source: Option<&'static str>,
    /// The person has added randomness of their own this session, which
    /// This device mixes in: copied from `Faraday::fresh` when the flow
    /// opens and kept in step at every addition and at the lock
    /// (`KeyGenArea::own_changed`).
    pub with_own: bool,
    /// The one-die roller beside the dice pad (Faraday Desktop,
    /// `docs/CODEX32.md` §5.6): which of the rolls it made.
    pub roller: crate::fresh::roller::Roller,
    /// The roller's number field has the keys.
    pub roller_typing: bool,
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
    pub fn new(slot: Option<u8>, back: Screen) -> KeyGen {
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
            words: 12,
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
            codex32_offered: slot.is_none(),
            codex32: None,
            entry_kept: crate::EntryState::default(),
            entropy_source: None,
            with_own: false,
            roller: crate::fresh::roller::Roller::default(),
            roller_typing: false,
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
        if self.start == Start::MakeCodex32 {
            vec![START, CODEX32]
        } else if self.start == Start::Scan {
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
        self.starts_with(crate::codex32::MAKE_OFFERED)
    }

    /// [`KeyGen::starts`] with making codex32 shares switched on or off
    /// (`crate::codex32::MAKE_OFFERED`).
    pub fn starts_with(&self, make_codex32: bool) -> Vec<Start> {
        STARTS
            .iter()
            .copied()
            .filter(|s| *s != Start::MakeCodex32 || (make_codex32 && self.codex32_offered))
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
        if self.slip39 == on {
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

    /// The Randomness card's way on: Enter opens once a source is chosen
    /// (a mix, once it has two), else the card says what is missing.
    fn source_next(&mut self) {
        let ok = match self.source {
            Some(Source::Mix) => self.mix_enough(),
            Some(_) => true,
            None => false,
        };
        if ok {
            self.done[usize::from(kstep::SOURCE)] = true;
            self.open = Some(kstep::ENTER);
            self.scroll.follow = true;
        } else {
            self.note = Some(if self.source.is_some() {
                EN.keygen_choose_two_sources.to_string()
            } else {
                EN.keygen_choose_a_source.to_string()
            });
        }
    }

    /// Whether a mix has the two sources it needs.
    pub fn mix_enough(&self) -> bool {
        self.mix.iter().filter(|on| **on).count() >= MIN_MIX
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
        let back = self.screen;
        let mut k = KeyGen::new(slot, back);
        k.entropy_source = self.keygen.entropy_source;
        k.with_own = self.fresh.own_acc().is_some();
        // Codex32 is made for single-key wallets only (`docs/CODEX32.md`
        // §6.9).
        k.codex32_offered = slot.is_none() || self.create.state().is_some_and(|c| !c.kind.multi());
        // A small panel is a touch screen with no keyboard of its own:
        // the faces are pressed, not typed.
        k.typing = !self.compact;
        // Add a key's own state waits in the flow, which types into the
        // app's; it comes back when the flow ends.
        k.entry_kept = self.entry.take();
        // Start opens closed on its default, Make a new key: Length is
        // open, and Change on Start reopens it (F103, `docs/FLOWS.md`,
        // New key).
        k.done[usize::from(kstep::START)] = true;
        k.started = Some(Start::Make);
        k.open = Some(kstep::LENGTH);
        // Single key, backup in shares: its slot opens Make codex32
        // shares on the identifier, the Quorum's scheme fixed
        // (`docs/CODEX32.md` §6.9).
        if let Some((sk, sn)) = slot.and_then(|_| self.create_codex32_scheme()) {
            k.start = Start::MakeCodex32;
            k.started = Some(Start::MakeCodex32);
            k.open = Some(kstep::CODEX32);
            let m = crate::codex32::Codex32Make::with_scheme(
                sk,
                sn,
                crate::codex32::taken(&k.entry_kept.parts.codex32),
            );
            k.codex32 = Some(self.codex32_here(m));
        }
        self.keygen.state = Some(k);
        self.screen = Screen::KeyGen;
    }

    /// Make codex32 shares as this device runs it: on Faraday Desktop the
    /// sheets are never masked (§6.4.6); in Demo the sample stands in
    /// place of the strings entered (`docs/CODEX32.md` §12). Cheap: the
    /// sample's two strings parsed.
    pub(crate) fn codex32_here(
        &self,
        m: crate::codex32::Codex32Make,
    ) -> crate::codex32::Codex32Make {
        let m = m.with_dice(self.keygen.dice);
        let m = if self.online { m.desktop() } else { m };
        if self.makes_secrets() {
            return m;
        }
        // BIP 93's vector 2, as the shell's samples carry it: the codex32
        // sample of shares (`docs/MODES.md` §7).
        let sample = self
            .samples
            .and_then(|s| {
                s.keys
                    .iter()
                    .find(|k| k.form == crate::samples::SampleForm::Codex32 && k.parts.len() >= 2)
            })
            .map(|k| {
                k.parts
                    .iter()
                    .filter_map(|p| faraday_bip::codex32::Codex32::parse(p).ok())
                    .collect()
            })
            .unwrap_or_default();
        m.in_demo(sample)
    }

    /// Costs a parse of each string typed in, and in Demo the seed they
    /// carry and its fingerprint. **Make the rest** or **Add a share** on
    /// Add a key's codex32 form (`docs/CODEX32.md` §8.1, §8.2): New key
    /// opens on Make codex32 shares with the `k` strings typed in; the
    /// form's strings stay as they are. In Demo strings of a set that is
    /// not public are refused as any key is (§12).
    pub(crate) fn keygen_codex32_given(&mut self, kind: crate::codex32::Given) {
        let c = self.compute();
        let e = self.entry.state();
        let k = e
            .parts
            .codex32
            .first()
            .map_or(0, |s| usize::from(s.threshold()));
        if k < 2 || e.parts.codex32.len() < k {
            return;
        }
        let strings: Vec<faraday_bip::codex32::Codex32> = e.parts.codex32[..k]
            .iter()
            .filter_map(|s| faraday_bip::codex32::Codex32::parse(s.encode().as_str()).ok())
            .collect();
        let (mark, rest_n) = (e.correction.key_mark(), e.codex32.rest_n);
        if !self.makes_secrets() {
            let fp = faraday_keys::codex32_seed(&strings)
                .ok()
                .and_then(|seed| faraday_keys::identity::fingerprint_of_seed(&c, &seed));
            if let Some(fp) = fp.filter(|&fp| !self.session.keys.is_public(fp)) {
                self.entry.state_mut().error =
                    Some(crate::wallet::Refusal::NotAllowed(crate::wallet::fp_text(fp)).text());
                return;
            }
        }
        // The form's strings stay; it makes the key again if pressed.
        self.entry.state_mut().codex32.making = None;
        if self.screen != Screen::KeyGen || self.keygen.state().is_none() {
            self.keygen_open(None);
        }
        let m = crate::codex32::Codex32Make::given(kind, strings, mark);
        let m = self.codex32_here(if rest_n > 0 { m.rest_of(rest_n) } else { m });
        let Some(k) = self.keygen.state.as_mut() else {
            return;
        };
        k.start = Start::MakeCodex32;
        k.started = Some(Start::MakeCodex32);
        k.done[usize::from(kstep::START)] = true;
        k.open = Some(kstep::CODEX32);
        k.scroll.follow = true;
        k.codex32 = Some(m);
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
            if matches!(start, Start::Make | Start::MakeShares) {
                k.set_slip39(start == Start::MakeShares);
            }
            let fresh = (start == Start::MakeCodex32).then(|| {
                crate::codex32::Codex32Make::new(crate::codex32::taken(&k.entry_kept.parts.codex32))
            });
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
            let flow = fresh.map(|m| self.codex32_here(m));
            if let Some(k) = self.keygen.state.as_mut() {
                k.codex32 = flow;
            }
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
            Start::MakeCodex32 => kstep::CODEX32,
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
        let mut words = None;
        if form == Form::Words {
            let m = match self.entry.mnemonic() {
                Ok(m) => m,
                Err(e) => {
                    self.entry.state_mut().error = Some(e);
                    return;
                }
            };
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
        if let A::KRoller(r) = action {
            self.keygen_roller(r);
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
        // A choice on Start moves on, and so does a press on the one
        // already chosen (F103).
        if let A::KStart(i) = action {
            let Some(&s) = STARTS.get(usize::from(i)) else {
                return;
            };
            if k.open != Some(kstep::START) || !k.starts().contains(&s) {
                return;
            }
            if let Some(k) = self.keygen.state.as_mut() {
                k.start = s;
                k.note = None;
            }
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
                    && let Some(c) = self.create.state_mut()
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
                .state()
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
        // Lock in moves on by itself (F103).
        self.keygen_continue();
    }

    /// A key locked in for Create slot `slot`, if any, fills it; on a
    /// single-key Create it makes the wallet too, as Keys' Continue
    /// would, and Check is what the Key card shows (`docs/history/NEW-WALLET.md`
    /// §3.3). `made`: the key was made here, not typed.
    fn keygen_fill(&mut self, slot: Option<u8>, fp: faraday_bip::keys::Fingerprint, made: bool) {
        let Some(slot) = slot else {
            return;
        };
        let Some(c) = self.create.state_mut() else {
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
            if let Some(c) = self.create.state_mut()
                && c.built.is_some()
            {
                c.done[usize::from(crate::cstep::KEYS)] = true;
                c.done[usize::from(crate::cstep::CHECK)] = true;
                c.open = Some(crate::cstep::BACKUP);
                c.scroll.follow = true;
            }
            if let Some(w) = self.create.state().and_then(|c| c.built)
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
                .state()
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
        // Lock in moves on by itself (F103).
        self.keygen_continue();
    }

    /// Where Lock in goes on to: back to Create for a
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
            && let Some(c) = self.create.state_mut()
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
                    (if k.slip39 || k.codex32.is_some() {
                        EN.keygen_shares_inline
                    } else {
                        EN.keygen_words_inline
                    }),
                ],
            ));
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
        // What the person added of their own is mixed in, never in place
        // of the system's (`docs/OWN-RANDOMNESS.md` §2.3).
        let own = self.fresh.own_acc().map(|a| zeroize::Zeroizing::new(*a));
        self.keygen.entropy(bytes, own.as_deref())
    }

    /// The camera is on while camera noise is being taken, and off
    /// otherwise.
    pub(crate) fn keygen_camera(&mut self) {
        self.keygen
            .camera(self.screen == Screen::KeyGen, &mut self.out);
    }

    /// One press inside Make codex32 shares (`docs/CODEX32.md` §6): Lock
    /// in loads the key, Suggest draws this device's randomness first.
    pub(crate) fn keygen_codex32(&mut self, press: crate::codex32::C32) {
        use crate::codex32::{C32, Card};
        let locked_next = self.keygen.state().is_some_and(|k| {
            k.codex32
                .as_ref()
                .is_some_and(|m| m.locked() && m.open() == Some(Card::Made))
        });
        match press {
            C32::Lock => return self.keygen_codex32_lock(),
            C32::Walk(w) => return self.keygen_codex32_walk(w),
            C32::Next | C32::Key(faraday_ui::keyboard::OskPress::Done) if locked_next => {
                return self.keygen_done();
            }
            _ => {}
        }
        let c = self.compute();
        let fresh = (press == C32::Suggest)
            .then(|| self.fresh(b"codex32 identifier"))
            .flatten();
        let compact = self.compact;
        let Some(k) = self.keygen.state.as_mut() else {
            return;
        };
        if matches!(press, C32::Card(_)) {
            k.open = Some(kstep::CODEX32);
            k.scroll.follow = true;
        }
        if let Some(m) = k.codex32.as_mut() {
            let before = m.open();
            m.act(&c, press, fresh);
            // The dice chosen are kept for the session's next flow.
            self.keygen.dice = m.dice_choice();
            k.scroll.follow |= m.open() != before;
            // On a small panel the entry stays scrolled to its foot at every
            // press, so the worksheet tabs, the panel and the pad are all in view
            // (`docs/CODEX32.md` §6.4.7).
            if compact && m.open() == Some(Card::Enter) {
                k.scroll.follow = false;
                k.scroll.y = faraday_draw::tokens::CODEX32_ENTER_SCROLL + m.pad_extra();
            }
        }
        if k.codex32
            .as_ref()
            .is_some_and(crate::codex32::Codex32Make::by_hand)
        {
            self.keygen_codex32_by_hand();
        }
        self.worksheet_follow();
    }

    /// The Entropy card's Continue on **Type shares made by hand**
    /// (`docs/CODEX32.md` §6.3, §5.3, §8.1): New key goes on to the
    /// codex32 form, Make the rest pressed and the strings in all the
    /// Scheme card's; once `k` shares are in, the make flow opens on them.
    fn keygen_codex32_by_hand(&mut self) {
        let Some(k) = self.keygen.state.as_mut() else {
            return;
        };
        let n = k.codex32.take().map_or(3, |m| m.k_n().1);
        k.start = Start::Codex32;
        k.started = Some(Start::Codex32);
        k.done[usize::from(kstep::START)] = true;
        k.done[usize::from(kstep::TYPED)] = false;
        k.open = Some(kstep::TYPED);
        k.scroll.follow = true;
        let mut e = crate::EntryState::default();
        e.form = Form::Codex32;
        e.codex32.making = Some(crate::codex32::Given::Rest);
        e.codex32.rest_n = n;
        self.entry.put(e);
    }

    /// Costs what a step costs: a worksheet column for a character put
    /// in its box, a fresh answer where Next rolls a character. A press on
    /// the step-through of the "Demo only" card, on Faraday Desktop only
    /// (`docs/CODEX32.md` §5.5): the view follows the step.
    fn keygen_codex32_walk(&mut self, w: crate::codex32::WalkAct) {
        if !self.roller_offered() {
            return;
        }
        let c = self.compute();
        let now = self.now_ms();
        let mut m = self.keygen.state.as_mut().and_then(|k| k.codex32.take());
        if let Some(m) = m.as_mut() {
            m.walk_act(&c, w, now, &mut || {
                self.fresh(b"faraday codex32 dice roller")
            });
        }
        if let Some(k) = self.keygen.state.as_mut() {
            k.codex32 = m;
        }
        self.worksheet_follow();
    }

    /// ← and → step the codex32 step-through while its card is on the
    /// entry (Faraday Desktop, `docs/CODEX32.md` §5.5), the worksheet
    /// under the pointer included; Up and Down still pan it. True when
    /// taken.
    pub(crate) fn keygen_walk_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use crate::codex32::WalkAct;
        use faraday_shell_api::Key;
        let act = match key {
            Key::Left => WalkAct::Previous,
            Key::Right => WalkAct::Next,
            _ => return false,
        };
        let open = self.screen == Screen::KeyGen
            && self.sheet.is_none()
            && self.roller_offered()
            && self
                .keygen
                .state()
                .and_then(|k| k.codex32.as_ref())
                .is_some_and(crate::codex32::Codex32Make::walk_open);
        if open {
            self.keygen_codex32_walk(act);
        }
        open
    }

    /// A press on the one-die roller beside the dice pad, on Faraday
    /// Desktop only (`docs/CODEX32.md` §5.6). Under Reduce motion a
    /// click's values land at once.
    fn keygen_roller(&mut self, r: crate::fresh::roller::Act) {
        use crate::fresh::roller::Act;
        if !self.roller_offered() {
            return;
        }
        let now = self.now_ms();
        let Some(k) = self.keygen.state.as_mut().filter(|k| !k.locked) else {
            return;
        };
        let room = k.roller_room();
        k.roller.act(r, room, now);
        k.roller_typing = r == Act::Count;
        if r == Act::Roll && self.reduce_motion() {
            self.keygen_tick(now);
        }
    }

    /// The one die's tick: the values it made land on the rolls as the
    /// pad's presses would. Returns whether the screen needs a frame.
    fn keygen_roller_tick(&mut self, now_ms: u64, reduce: bool) -> bool {
        let Some(mut roller) = self
            .keygen
            .state
            .as_mut()
            .map(|k| std::mem::take(&mut k.roller))
        else {
            return false;
        };
        let (len, room) = self
            .keygen
            .state()
            .map_or((0, 0), |k| (k.progress().0, k.roller_room()));
        let (values, redraw) = roller.tick(now_ms, reduce, len, room, &mut || {
            self.fresh(b"faraday dice roller")
        });
        let Some(k) = self.keygen.state.as_mut() else {
            return redraw;
        };
        k.roller = roller;
        for v in values {
            k.roll(v);
        }
        // A value the procedure did not take is not on the rolls.
        let len = k.progress().0;
        k.roller.trim(len);
        redraw
    }

    /// Costs what the faces and values landed cost and a fresh answer for
    /// every 31 of them at most; nothing at rest. The tick of New key's
    /// moving parts: the codex32 step-through's Play (Faraday Desktop,
    /// `docs/CODEX32.md` §5.5), the codex32 worksheet's view following the
    /// work (§7.5), and the one die beside the dice pad (§5.6). Returns
    /// whether the screen needs a frame.
    pub(crate) fn keygen_tick(&mut self, now_ms: u64) -> bool {
        let Some(k) = self.keygen.state.as_ref() else {
            return false;
        };
        let (walk, pan) = k
            .codex32
            .as_ref()
            .map_or((false, false), |m| (m.walk().active(), m.pan().moving()));
        let one = k.roller.active();
        if !walk && !pan && !one {
            return false;
        }
        let reduce = self.reduce_motion();
        let mut redraw = false;
        if walk && self.roller_offered() {
            let c = self.compute();
            let mut m = self.keygen.state.as_mut().and_then(|k| k.codex32.take());
            let stepped = m.as_mut().is_some_and(|m| {
                let playing = m.walk().playing();
                let moved = m.walk_tick(&c, now_ms, &mut || {
                    self.fresh(b"faraday codex32 dice roller")
                });
                // Play ending changes its button too.
                moved || playing != m.walk().playing()
            });
            if let Some(k) = self.keygen.state.as_mut() {
                k.codex32 = m;
            }
            if stepped {
                // The step moved the work: the worksheet's view follows.
                self.worksheet_follow();
                redraw = true;
            }
        }
        let f = self.f;
        if let Some(m) = self.keygen.state.as_mut().and_then(|k| k.codex32.as_mut()) {
            redraw |= m.pan_tick(now_ms, f);
        }
        if one {
            redraw |= self.keygen_roller_tick(now_ms, reduce);
        }
        redraw
    }

    /// Costs what `add_seed` costs, and the key's account text. Lock in on
    /// Strings made: the key the set carries, loaded as a key made here
    /// (`docs/CODEX32.md` §6.7); a Create slot is filled with it.
    fn keygen_codex32_lock(&mut self) {
        let c = self.compute();
        if !self.selftest_passed() || !self.may_load_keys() {
            return;
        }
        let label = fill(
            EN.keygen_new_key_n,
            &[&(self.keygen.new_keys + 1).to_string()],
        );
        let Some(k) = self.keygen.state.as_ref() else {
            return;
        };
        let Some(m) = k.codex32.as_ref().filter(|m| m.ready_to_lock()) else {
            return;
        };
        let Some(work) = m.work() else {
            return;
        };
        // In Demo only a set of strings already public loads: the sample
        // in place of the strings entered, or strings typed in (§12).
        if !self.makes_secrets() && !work.is_given() {
            return;
        }
        let (made_here, mark) = (m.given_kind().is_none(), m.mark());
        let slot = k.slot;
        let fp = match self.session.keys.add_codex32(&c, work, &label) {
            Ok(fp) => fp,
            Err(e) => {
                if let Some(m) = self.keygen.state.as_mut().and_then(|k| k.codex32.as_mut()) {
                    m.refused(e.text());
                }
                return;
            }
        };
        // A key of strings typed in was made on paper, not here; it
        // carries the mark of any string that went in by a correction
        // (§8.4).
        if made_here {
            self.mark_made_here(fp);
        }
        if let Some(mark) = mark {
            self.session.key_marks.note(fp.0, mark);
        }
        self.keygen.new_keys += 1;
        let (kind, account) = match slot {
            Some(_) => self
                .create
                .state()
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
            k.fingerprint = Some(fp.0);
            k.key_text = key_text;
            if let Some(m) = k.codex32.as_mut() {
                m.set_locked();
            }
        }
        // The set's facts go to Create first, for Check and Back up.
        if let Some(set) = self
            .keygen
            .state
            .as_ref()
            .and_then(|k| k.codex32.as_ref())
            .filter(|_| slot.is_some())
            .map(|m| crate::create::SharesSet::of(fp.0, m))
        {
            self.create_codex32_made(set);
        }
        self.keygen_fill(slot, fp, true);
        self.refresh_spend();
        // A Create slot goes back to Create at once; on its own the
        // flow ends on Done (F103).
        if slot.is_some() {
            self.keygen_done();
        }
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
        if k.open == Some(kstep::KEY) && !k.locked && k.asks_passphrase() && k.focus.is_some() {
            if key == KeyIn::Enter {
                self.keygen_lock();
                return true;
            }
            return k.passphrase_key(key);
        }
        if k.roller_typing {
            return k.roller_key(key);
        }
        // Escape is Back: the step before, or out of the flow from its
        // first (F104).
        if key == KeyIn::Escape {
            self.act(crate::Action::Back);
            return true;
        }
        // Make codex32 shares: the keyboard's keys to its open card.
        if k.open == Some(kstep::CODEX32) && k.codex32.is_some() {
            let press = match key {
                KeyIn::Char(c) => faraday_ui::keyboard::OskPress::Char(c),
                KeyIn::Backspace => faraday_ui::keyboard::OskPress::Back,
                KeyIn::Enter => faraday_ui::keyboard::OskPress::Done,
                _ => return false,
            };
            self.keygen_codex32(crate::codex32::C32::Key(press));
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
