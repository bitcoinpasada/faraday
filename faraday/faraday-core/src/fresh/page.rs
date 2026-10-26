//! Add randomness (`docs/OWN-RANDOMNESS.md` §2.2): the page's chosen
//! source and the batch being entered on it, through `faraday-entropy`'s
//! accumulators as New key takes them. A batch of dice, coins or cards
//! goes into the session at its Add; a camera frame goes in as it is
//! taken. What a batch makes is its commitment, which `Fresh::own_add`
//! hashes into the person's accumulator; the batch itself is wiped then,
//! and at the lock.

use super::OwnSource;
use faraday_entropy::{CameraNoise, CardDraws, CoinFlips, DiceRolls, FrameStats};
use zeroize::Zeroize;

/// A press on Add randomness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    /// Open Add randomness from the screen on show, which Back returns
    /// to: Settings' Randomness row. New key's Generator card offers the
    /// sources on itself.
    Open,
    /// Choose the source shown.
    Source(OwnSource),
    /// A die's face, 1 to 6.
    Roll(u8),
    /// A coin: heads (`true`) or tails.
    Flip(bool),
    /// A card's rank, 0 (ace) to 12 (king), picked before its suit.
    Rank(u8),
    /// A card's suit, 0 to 3 (♠ ♥ ♦ ♣), which draws the card.
    Suit(u8),
    /// Take the camera's newest frame: it is added at once.
    Frame,
    /// Take back the batch's last entry.
    Undo,
    /// Forget the batch.
    Clear,
    /// Add the batch to the session.
    Add,
    /// New key's Generator card: on to the words with nothing of one's
    /// own added.
    Skip,
}

/// A batch the page hands to the session: its source, its commitment
/// and how many it holds.
pub(crate) type Batch = (OwnSource, [u8; 32], u32);

/// Add randomness while it is open. Its batches are the person's own
/// randomness and are wiped as they are added, as the page goes and at
/// the lock.
pub(crate) struct OwnPage {
    /// The source shown.
    source: OwnSource,
    /// The person has chosen `source`: until then no pad is live, no key
    /// is taken and no camera runs (owner, 2026-10-26: nothing is
    /// collected passively). New key's Generator card opens unchosen.
    chosen: bool,
    /// Rolls not yet added.
    dice: DiceRolls,
    /// Flips not yet added.
    coins: CoinFlips,
    /// Cards drawn from the deck being entered, not yet added.
    cards: CardDraws,
    /// A card's rank, picked before its suit.
    rank: Option<u8>,
    /// The camera's newest frame's luma, while the camera is on.
    frame: Option<Vec<u8>>,
    /// The last frame taken's statistics, taken or refused.
    last_frame: Option<FrameStats>,
    /// What the last press could not do.
    note: Option<&'static str>,
}

impl Drop for OwnPage {
    fn drop(&mut self) {
        if let Some(f) = self.frame.as_mut() {
            f.zeroize();
        }
    }
}

impl OwnPage {
    /// The page as it opens: on Camera where the shell has a camera, on
    /// Dice where it has none (§2.2).
    pub(crate) fn new(has_camera: bool) -> Self {
        let mut page = Self::unchosen();
        if !has_camera {
            page.source = OwnSource::Dice;
        }
        page.chosen = true;
        page
    }

    /// The sources as New key's Generator card offers them: none chosen
    /// until the person presses one.
    pub(crate) fn unchosen() -> Self {
        OwnPage {
            source: OwnSource::Camera,
            chosen: false,
            dice: DiceRolls::new(),
            coins: CoinFlips::new(),
            cards: CardDraws::new(),
            rank: None,
            frame: None,
            last_frame: None,
            note: None,
        }
    }

    /// The source shown.
    pub(crate) fn source(&self) -> OwnSource {
        self.source
    }

    /// Cards not yet added.
    pub(crate) fn cards(&self) -> &CardDraws {
        &self.cards
    }

    /// The rank picked, waiting for its suit.
    pub(crate) fn rank(&self) -> Option<u8> {
        self.rank
    }

    /// Whether a camera frame has arrived to take.
    pub(crate) fn frame_live(&self) -> bool {
        self.frame.is_some()
    }

    /// The last frame taken's statistics.
    pub(crate) fn last_frame(&self) -> Option<FrameStats> {
        self.last_frame
    }

    /// What the last press could not do.
    pub(crate) fn note(&self) -> Option<&'static str> {
        self.note
    }

    /// How many entries the shown source's batch holds.
    pub(crate) fn batch_len(&self) -> usize {
        match self.source {
            OwnSource::Dice => self.dice.len(),
            OwnSource::Coins => self.coins.len(),
            OwnSource::Cards => self.cards.len(),
            OwnSource::Camera | OwnSource::PointerKeys => 0,
        }
    }

    /// Whether a source has been chosen.
    pub(crate) fn chosen(&self) -> bool {
        self.chosen
    }

    /// Whether the camera is wanted: Camera is the source chosen.
    pub(crate) fn wants_camera(&self) -> bool {
        self.chosen && self.source == OwnSource::Camera
    }

    /// The camera's newest frame, in place of the one before.
    pub(crate) fn camera_frame(&mut self, luma: Vec<u8>) {
        if let Some(old) = self.frame.as_mut() {
            old.zeroize();
        }
        self.frame = Some(luma);
    }

    /// The camera went off: its last frame goes.
    pub(crate) fn camera_off(&mut self) {
        if let Some(old) = self.frame.as_mut() {
            old.zeroize();
        }
        self.frame = None;
    }

    /// The lock: every batch not yet added goes, and the last frame.
    pub(crate) fn wipe(&mut self) {
        self.dice.clear();
        self.coins.clear();
        self.cards.clear();
        self.rank = None;
        self.camera_off();
        self.last_frame = None;
        self.note = None;
    }

    /// A press. Returns the batch to add to the session, when the press
    /// makes one: Add with entries in, or a camera frame taken.
    pub(crate) fn act(&mut self, a: Act) -> Option<Batch> {
        self.note = None;
        if !self.chosen && !matches!(a, Act::Source(_)) {
            return None;
        }
        match a {
            Act::Source(s) if s != OwnSource::PointerKeys => {
                self.source = s;
                self.chosen = true;
                self.rank = None;
            }
            Act::Source(_) => {}
            Act::Roll(f) if self.source == OwnSource::Dice => {
                self.dice.push(f);
            }
            Act::Flip(h) if self.source == OwnSource::Coins => {
                self.coins.push(h);
            }
            Act::Rank(r) if self.source == OwnSource::Cards => self.rank = Some(r),
            Act::Suit(s) if self.source == OwnSource::Cards => self.suit(s),
            Act::Frame if self.source == OwnSource::Camera => return self.take_frame(),
            Act::Undo => self.undo(),
            Act::Clear => self.clear_shown(),
            Act::Add => return self.take_batch(),
            _ => {}
        }
        None
    }

    /// A suit after a rank draws the card; a card already out of this
    /// deck is refused, as New key refuses it.
    fn suit(&mut self, s: u8) {
        let Some(r) = self.rank.take() else {
            return;
        };
        if let Some(i) = faraday_entropy::card_index(r, s)
            && !self.cards.push(i)
        {
            self.note = Some(crate::strings::EN.keygen_card_already_drawn);
        }
    }

    /// The newest frame, hashed as New key hashes it, and refused as New
    /// key refuses one with too little variation.
    fn take_frame(&mut self) -> Option<Batch> {
        let Some(luma) = self.frame.as_ref() else {
            self.note = Some(crate::strings::EN.keygen_no_camera_picture);
            return None;
        };
        let mut camera = CameraNoise::new();
        let (stats, taken) = camera.push(luma);
        self.last_frame = Some(stats);
        if !taken {
            self.note = Some(crate::strings::EN.keygen_too_little_variation);
            return None;
        }
        Some((OwnSource::Camera, camera.commitment(), 1))
    }

    /// The shown source's last entry back, or the rank picked.
    fn undo(&mut self) {
        if self.rank.take().is_some() {
            return;
        }
        match self.source {
            OwnSource::Dice => {
                self.dice.pop();
            }
            OwnSource::Coins => {
                self.coins.pop();
            }
            OwnSource::Cards => {
                self.cards.pop();
            }
            OwnSource::Camera | OwnSource::PointerKeys => {}
        }
    }

    /// Forgets the shown source's batch.
    fn clear_shown(&mut self) {
        self.rank = None;
        match self.source {
            OwnSource::Dice => self.dice.clear(),
            OwnSource::Coins => self.coins.clear(),
            OwnSource::Cards => self.cards.clear(),
            OwnSource::Camera | OwnSource::PointerKeys => {}
        }
    }

    /// The shown source's batch as its commitment, wiped once taken; a
    /// next batch of cards starts from a full deck.
    fn take_batch(&mut self) -> Option<Batch> {
        let n = u32::try_from(self.batch_len()).ok().filter(|&n| n > 0)?;
        let commitment = match self.source {
            OwnSource::Dice => self.dice.commitment(),
            OwnSource::Coins => self.coins.commitment(),
            OwnSource::Cards => self.cards.commitment(),
            OwnSource::Camera | OwnSource::PointerKeys => return None,
        };
        let source = self.source;
        self.clear_shown();
        Some((source, commitment, n))
    }

    /// A key typed on the page: a die's face, H or T, a card's rank then
    /// its suit (S H D C), Space for a picture, Backspace and Enter.
    /// Returns the press it stands for.
    pub(crate) fn key(&self, key: faraday_shell_api::Key) -> Option<Act> {
        use faraday_shell_api::Key as K;
        // Keys count only once a source that takes them is chosen.
        if !self.chosen {
            return None;
        }
        match key {
            K::Backspace => Some(Act::Undo),
            K::Enter if self.source != OwnSource::Camera => Some(Act::Add),
            K::Char(' ') if self.source == OwnSource::Camera => Some(Act::Frame),
            K::Char(c) => self.typed(c.to_ascii_uppercase()),
            _ => None,
        }
    }

    /// A character typed, for the shown source.
    fn typed(&self, c: char) -> Option<Act> {
        match self.source {
            OwnSource::Dice => c
                .to_digit(10)
                .filter(|d| (1..=6).contains(d))
                .map(|d| Act::Roll(d as u8)),
            OwnSource::Coins => match c {
                'H' => Some(Act::Flip(true)),
                'T' => Some(Act::Flip(false)),
                _ => None,
            },
            OwnSource::Cards if self.rank.is_some() => ['S', 'H', 'D', 'C']
                .iter()
                .position(|s| *s == c)
                .map(|s| Act::Suit(s as u8)),
            OwnSource::Cards => faraday_entropy::RANKS
                .iter()
                .position(|r| *r == c)
                .map(|r| Act::Rank(r as u8)),
            OwnSource::Camera | OwnSource::PointerKeys => None,
        }
    }
}
