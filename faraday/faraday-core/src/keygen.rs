//! Making a new key: the randomness, the word count, and the words
//! (`docs/FLOWS.md`, New key).
//!
//! The options are OpenSigner's, taken from its Create wizard rather than
//! copied: the sources and their order ([`SOURCE_ROWS`]), what a mix may
//! combine ([`MIX_SOURCES`]), the word counts (`opensigner_core::load::
//! COUNTS`) and their names and the dice procedures' in OpenSigner's
//! strings. Every cryptographic step is `osk-entropy`'s and `osk-bip`'s,
//! the crates OpenSigner's wizard itself calls: the rolls, flips, draws,
//! digits and frames accumulate in their fixed buffers, the procedures
//! hash or spell them, the sanity checks are theirs, a mix combines their
//! commitments, the checksum leaves the last word, and a [`Mnemonic`]
//! spells the entropy. Faraday adds the flow and the screen.
//!
//! The accumulators and the words are erased on drop; leaving the flow
//! drops them all.

use osk_bip::bip39::{Language, Mnemonic};
use osk_entropy::{
    CameraNoise, CardDraws, CoinFlips, DiceProcedure, DiceRolls, MIN_MIX, Mixed, RawHex, Strength,
    Warnings,
};

pub use opensigner_core::quiz::QuizState;
use opensigner_core::strings::EN;
pub use osk_entropy::{MIX_SOURCES, SOURCE_ROWS, Source};

use osk_bip::bitcoin::hashes::{Hash, HashEngine, sha256};

use crate::{Screen, flow};

/// The word counts on offer, in OpenSigner's order.
pub const COUNTS: [u8; 5] = osk_entropy::WORD_COUNTS;

/// A source's name, as OpenSigner writes it.
pub fn source_name(s: Source) -> &'static str {
    match s {
        Source::Dice => EN.create_source_dice,
        Source::Coins => EN.create_source_coins,
        Source::Hex => EN.create_source_hex,
        Source::Cards => EN.create_source_cards,
        Source::Camera => EN.create_source_camera,
        Source::Mix => EN.create_source_mix,
        Source::Device => EN.create_source_device,
        Source::SeedXor => "Seed XOR",
    }
}

/// A dice procedure's name, as OpenSigner writes it.
pub fn procedure_name(p: DiceProcedure) -> &'static str {
    match p {
        DiceProcedure::Hashed => EN.dice_procedure_hashed,
        DiceProcedure::SixAsZero => EN.dice_procedure_six_as_zero,
        // OpenSigner's name less its "(BitBox)".
        DiceProcedure::Words => "Words chosen by the dice",
    }
}

/// Its place in [`SOURCE_ROWS`], which actions carry.
pub fn source_index(s: Source) -> u8 {
    SOURCE_ROWS.iter().position(|x| *x == s).unwrap_or(0) as u8
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
    /// 12 or 24 words.
    pub const LENGTH: u8 = 0;
    /// Which source.
    pub const SOURCE: u8 = 1;
    /// Rolling, flipping, drawing, typing or taking frames.
    pub const ENTER: u8 = 2;
    /// Counts, statistics and cautions.
    pub const CHECK: u8 = 3;
    /// The words.
    pub const WORDS: u8 = 4;
    /// OpenSigner's backup quiz over them, and adding the key.
    pub const QUIZ: u8 = 5;
    /// How many cards.
    pub const COUNT: usize = 6;
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
    pub device: osk_entropy::DeviceRandom,
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
    /// OpenSigner's backup quiz, once the words have been shown.
    pub quiz: Option<opensigner_core::quiz::Quiz>,
    /// The person asked to skip the quiz; a second press skips it.
    pub skip_ask: bool,
    /// The quiz was skipped.
    pub skipped: bool,
    /// The last thing worth saying.
    pub note: Option<String>,
    /// The Create slot the key fills, if it was made for one.
    pub slot: Option<u8>,
    /// The key is made as SLIP-39 shares rather than BIP-39 words: the
    /// entropy is the master secret, written only as shares
    /// (`docs/PLANNING.md` §16.107 rule 4, as OpenSigner's own wizard).
    pub slip39: bool,
    /// Shares needed to restore.
    pub slip_m: u8,
    /// Shares dealt.
    pub slip_n: u8,
    /// The master secret, once the entropy is in.
    pub secret: Option<zeroize::Zeroizing<Vec<u8>>>,
    /// The shares, once dealt.
    pub shares: Vec<osk_bip::slip39::Share>,
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
            open: Some(kstep::LENGTH),
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
            device: osk_entropy::DeviceRandom::new(),
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
        }
    }

    /// The word counts the Length card offers: BIP-39's, or a SLIP-39
    /// share's (20 words for 128 bits, 33 for 256).
    pub fn counts(&self) -> &'static [u8] {
        if self.slip39 {
            &opensigner_core::load::SLIP39_COUNTS
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
        self.done = [false; kstep::COUNT];
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

    /// Deals the master secret into `slip_m` of `slip_n` shares, with
    /// `random` for the split's own randomness (a fresh draw from the
    /// system's generator).
    pub fn deal(&mut self, random: [u8; 32]) -> Result<(), String> {
        use osk_bip::slip39;
        let Some(secret) = self.secret.as_ref() else {
            return Err("No secret yet".to_string());
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
        let groups = [slip39::GroupSpec {
            threshold: self.slip_m,
            count: self.slip_n,
        }];
        let n = slip39::split(
            secret, b"", identifier, true, 1, 1, &groups, &mut fill, &mut out,
        )
        .map_err(|e| format!("The shares could not be made: {e:?}"))?;
        out.truncate(n);
        self.shares = out;
        self.share_at = 0;
        Ok(())
    }

    /// The strength the word count takes.
    pub fn strength(&self) -> Strength {
        if self.slip39 {
            return if self.words <= osk_bip::slip39::MIN_WORDS {
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
    /// (`osk_entropy::DiceRolls::entropy_under`).
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
            osk_entropy::DICE_WORD_ROLLS * (self.words - 1) + self.last_bits().div_ceil(2)
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
    ) -> Result<osk_entropy::RawEntropy, osk_entropy::Error> {
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
            out.push("A long run of one result");
        }
        if w.skewed {
            out.push("Far from even: one result came up much more than the others");
        }
        if w.sequential {
            out.push("A long run counting up or down");
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
        for k in [kstep::ENTER, kstep::CHECK, kstep::WORDS, kstep::QUIZ] {
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
    fn flip_or_roll(&self) -> bool {
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
                "A 5 or a 6 is rolled again in a word's first five rolls".to_string()
            } else {
                "A die has six faces".to_string()
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
            self.note = Some("A die has six faces".to_string());
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
            self.note = Some("No more are needed".to_string());
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
        let m = Mnemonic::from_entropy(Language::English, &bytes).ok();
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
            self.note = match osk_entropy::card_index(r, s) {
                Some(i) if self.cards.push(i) => None,
                Some(_) => Some("That card is already out of this deck".to_string()),
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
            self.note = Some("No picture from the camera yet".to_string());
            return;
        };
        let (_, taken) = self.camera.push(luma);
        self.note = (!taken).then(|| "Too little variation in that picture".to_string());
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
            self.note = Some(format!("{have} of {need}"));
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
        match Mnemonic::from_entropy(Language::English, entropy) {
            Ok(m) => {
                self.mnemonic = Some(m);
                true
            }
            Err(_) => false,
        }
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

impl crate::Faraday {
    /// Opens the flow, for a Create slot or for the session.
    pub(crate) fn keygen_open(&mut self, slot: Option<u8>) {
        if !self.may_load_keys() {
            self.toast("Remove the stick first");
            return;
        }
        let only_24 = slot.is_some() && self.create.as_ref().is_some_and(|c| c.kind.threshold());
        let back = self.screen;
        let mut k = KeyGen::new(slot, back, only_24);
        // A small panel is a touch screen with no keyboard of its own:
        // the faces are pressed, not typed.
        k.typing = !self.compact;
        self.keygen = Some(k);
        self.screen = Screen::KeyGen;
    }

    /// One press inside the flow.
    pub(crate) fn keygen_act(&mut self, action: crate::Action) {
        use crate::Action as A;
        if let A::KeyGen(slot) = action {
            self.keygen_open(slot);
            return;
        }
        if action == A::KeyGenSlip39 {
            self.keygen_open(None);
            if let Some(k) = self.keygen.as_mut() {
                k.set_slip39(true);
            }
            return;
        }
        // Dealing shares takes a fresh draw of its own from the system's
        // generator; asked for before the flow is borrowed.
        let dealing = action == A::KNext
            && self
                .keygen
                .as_ref()
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
        let Some(k) = self.keygen.as_mut() else {
            return;
        };
        let open = |k: &mut KeyGen, step: u8| {
            k.open = Some(step);
            k.scroll.follow = true;
        };
        match action {
            A::KStep(s) => {
                // A later card opens once the ones before it are done.
                let reachable = (0..s).all(|i| k.done[usize::from(i)]);
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
                let max = osk_bip::slip39::MAX_COUNT as i16;
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
                            "Choose two sources or more".to_string()
                        } else {
                            "Choose a source".to_string()
                        });
                    }
                }
                Some(kstep::ENTER) => {
                    if k.slip39 && draw.is_none() {
                        k.note = Some("No randomness from the system yet. Try again".to_string());
                        return;
                    }
                    if k.finish_entry() {
                        if k.slip39 {
                            if let Err(e) = draw.map_or(Ok(()), |r| k.deal(r)) {
                                k.note = Some(e);
                                return;
                            }
                            k.fingerprint = k.secret.as_ref().and_then(|s| {
                                crate::wallet::Session::default()
                                    .add_seed(s, "")
                                    .ok()
                                    .map(|f| f.0)
                            });
                        } else {
                            k.fingerprint = k.phrase().and_then(|w| {
                                crate::wallet::Session::default()
                                    .add_words_with(&w, "", "", None)
                                    .ok()
                                    .map(|f| f.0)
                            });
                        }
                        k.done[usize::from(kstep::ENTER)] = true;
                        open(k, kstep::CHECK);
                    }
                }
                Some(kstep::CHECK) => {
                    k.done[usize::from(kstep::CHECK)] = true;
                    open(k, kstep::WORDS);
                }
                Some(kstep::WORDS) if k.slip39 => {
                    // Each share is quizzed in turn, from the first.
                    if let Some(ind) = k.share_indices(0) {
                        let seed = opensigner_core::quiz::Quiz::seed(self.now_ms, ind);
                        k.quiz = Some(opensigner_core::quiz::Quiz::new(
                            ind,
                            opensigner_core::load::EntryList::Slip39,
                            false,
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
                        let seed = opensigner_core::quiz::Quiz::seed(self.now_ms, m.indices());
                        k.quiz = Some(opensigner_core::quiz::Quiz::new(
                            m.indices(),
                            opensigner_core::load::EntryList::Bip39(Language::English),
                            false,
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
                if k.open == Some(kstep::WORDS) {
                    k.shown = !k.shown;
                }
            }
            A::KQuiz(slot) if k.slip39 => {
                let list = opensigner_core::load::EntryList::Slip39;
                let at = k.quiz_share;
                let ind: Vec<u16> = k.share_indices(at).map(<[u16]>::to_vec).unwrap_or_default();
                let passed = match k.quiz.as_mut() {
                    Some(q) => {
                        q.pick(usize::from(slot));
                        q.confirm(&ind, list);
                        q.state() == opensigner_core::quiz::QuizState::Passed
                    }
                    None => false,
                };
                if passed {
                    // The next share, or done.
                    match k.share_indices(at + 1).map(<[u16]>::to_vec) {
                        Some(next) => {
                            let seed = opensigner_core::quiz::Quiz::seed(self.now_ms, &next);
                            k.quiz =
                                Some(opensigner_core::quiz::Quiz::new(&next, list, false, seed));
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
                    q.retry(&ind, opensigner_core::load::EntryList::Slip39);
                }
            }
            A::KQuiz(slot) => {
                if let (Some(q), Some(m)) = (k.quiz.as_mut(), k.mnemonic.as_ref()) {
                    let list = opensigner_core::load::EntryList::Bip39(Language::English);
                    q.pick(usize::from(slot));
                    q.confirm(m.indices(), list);
                    if q.state() == opensigner_core::quiz::QuizState::Passed {
                        k.done[usize::from(kstep::QUIZ)] = true;
                    }
                }
            }
            A::KQuizRetry => {
                if let (Some(q), Some(m)) = (k.quiz.as_mut(), k.mnemonic.as_ref()) {
                    q.retry(
                        m.indices(),
                        opensigner_core::load::EntryList::Bip39(Language::English),
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

    /// Loads the key the words spell and leaves the flow.
    fn keygen_add(&mut self) {
        // No key is accepted before the self-test has passed.
        if !self.selftest_passed() {
            return;
        }
        let Some(k) = self.keygen.as_ref() else {
            return;
        };
        // The words were checked by the quiz, or the person said twice to
        // go without it.
        let passed = k
            .quiz
            .as_ref()
            .is_some_and(|q| q.state() == opensigner_core::quiz::QuizState::Passed);
        if !(passed || k.skipped) {
            return;
        }
        let (slot, back, slip39) = (k.slot, k.back, k.slip39);
        let label = format!("New key {}", self.new_keys + 1);
        let added = if k.slip39 {
            match k.secret.as_ref() {
                Some(s) => {
                    let s = s.clone();
                    self.session.add_seed(&s, &label)
                }
                None => return,
            }
        } else {
            let Some(words) = k.phrase() else {
                return;
            };
            self.session.add_words(&words, &label, None)
        };
        match added {
            Ok(fp) => {
                self.new_keys += 1;
                if let Some(slot) = slot
                    && let Some(c) = self.create.as_mut()
                {
                    c.made += 1;
                    if let Some(s) = c.slots.get_mut(usize::from(slot)) {
                        *s = crate::create::Source::Here(fp.0);
                    }
                }
                self.keygen = None;
                self.screen = back;
                self.refresh_spend();
                self.toast(&format!(
                    "Key {} added; back up its {} before you rely on it",
                    crate::wallet::fp_text(fp),
                    if slip39 { "shares" } else { "words" }
                ));
            }
            Err(e) => {
                if let Some(k) = self.keygen.as_mut() {
                    k.note = Some(e.text());
                }
            }
        }
    }

    /// Leaves the flow, forgetting everything in it.
    pub(crate) fn keygen_leave(&mut self) {
        if let Some(k) = self.keygen.take() {
            self.screen = k.back;
        }
    }

    /// The shell is asked for fresh bytes when this device's generator
    /// is the source being entered; they go to the key and nowhere else.
    pub(crate) fn keygen_device(&mut self) {
        let Some(k) = self.keygen.as_mut() else {
            return;
        };
        if k.open == Some(kstep::ENTER)
            && k.active() == Some(Source::Device)
            && !k.device.is_enough(k.strength())
            && !k.device_asked
        {
            k.device_asked = true;
            self.commands
                .push_back(osk_shell_api::Command::RequestEntropy);
        }
    }

    /// Takes the shell's answer when the flow asked for it. Returns
    /// whether it did.
    pub(crate) fn keygen_entropy(&mut self, bytes: &mut [u8; 32]) -> bool {
        let Some(k) = self.keygen.as_mut().filter(|k| k.device_asked) else {
            return false;
        };
        k.device.set(bytes);
        k.device_asked = false;
        zeroize::Zeroize::zeroize(bytes);
        true
    }

    /// The camera is on while camera noise is being taken, and off
    /// otherwise.
    pub(crate) fn keygen_camera(&mut self) {
        let want = self.screen == Screen::KeyGen
            && self.keygen.as_ref().is_some_and(|k| {
                k.open == Some(kstep::ENTER) && k.active() == Some(Source::Camera)
            });
        if want != self.keygen_camera_on {
            self.keygen_camera_on = want;
            self.commands.push_back(if want {
                osk_shell_api::Command::CameraOn
            } else {
                osk_shell_api::Command::CameraOff
            });
        }
        if !want
            && let Some(k) = self.keygen.as_mut()
            && let Some(f) = k.frame.as_mut()
        {
            zeroize::Zeroize::zeroize(f);
            k.frame = None;
        }
    }

    /// Keys typed while the flow is on screen. Returns whether it took
    /// the key.
    pub(crate) fn keygen_key(&mut self, key: osk_shell_api::Key) -> bool {
        use osk_shell_api::Key as KeyIn;
        if self.screen != Screen::KeyGen {
            return false;
        }
        let Some(k) = self.keygen.as_mut() else {
            return false;
        };
        if key == KeyIn::Escape {
            self.keygen_leave();
            return true;
        }
        // Typing into the box: each character is taken as it comes, and
        // the box keeps every one, so nothing else on the page reads it.
        // Backspace and Enter are the page's, as with the buttons.
        if k.open == Some(kstep::ENTER)
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
        if k.open != Some(kstep::ENTER) {
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
                if let Some(r) = osk_entropy::RANKS.iter().position(|x| *x == c) {
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
