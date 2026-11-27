//! Playing cards: draws from a shuffled deck, tracked so a card already out is refused.

use crate::{CARDS_IN_ORDER, Error, RawEntropy, Strength, Warnings};
use faraday_crypto::{Zeroize, ZeroizeOnDrop, sha256};

/// Cards in one deck.
pub const DECK: usize = 52;

/// Most cards an accumulator holds: two decks, which is what 256 bits
/// needs ([`CardDraws::needed`]), and more than any shorter length does.
pub const MAX_CARDS: usize = 2 * DECK;

/// The ranks, in the order [`CardDraws`] numbers them. `T` is the ten,
/// so that every rank is one character and the strip of cards entered
/// keeps one width.
pub const RANKS: [char; 13] = [
    'A', '2', '3', '4', '5', '6', '7', '8', '9', 'T', 'J', 'Q', 'K',
];

/// The suits, in the order [`CardDraws`] numbers them: ♠ ♥ ♦ ♣.
pub const SUITS: [char; 4] = ['\u{2660}', '\u{2665}', '\u{2666}', '\u{2663}'];

/// log2 of 0..=52, for the bits a draw from a deck of that many carries.
/// `LOG2[0]` is a placeholder: no draw is made from an empty deck.
const LOG2: [f32; 53] = [
    0.0,
    0.0,
    1.0,
    1.5849625,
    2.0,
    2.321928,
    2.5849626,
    2.807355,
    3.0,
    3.169925,
    core::f32::consts::LOG2_10,
    3.4594316,
    3.5849626,
    3.7004397,
    3.807355,
    3.9068906,
    4.0,
    4.087463,
    4.169925,
    4.2479277,
    4.321928,
    4.3923173,
    4.4594316,
    4.523562,
    4.5849624,
    4.643856,
    4.70044,
    4.7548876,
    4.807355,
    4.857981,
    4.9068904,
    4.9541965,
    5.0,
    5.044394,
    5.087463,
    5.129283,
    5.169925,
    5.2094536,
    5.2479277,
    5.2854023,
    5.321928,
    5.357552,
    5.3923173,
    5.426265,
    5.4594316,
    5.491853,
    5.523562,
    5.554589,
    5.5849624,
    5.61471,
    5.643856,
    5.6724253,
    5.70044,
];

/// The card index a rank and a suit make, 0..52.
pub fn card_index(rank: u8, suit: u8) -> Option<u8> {
    (usize::from(rank) < RANKS.len() && usize::from(suit) < SUITS.len())
        .then(|| suit * RANKS.len() as u8 + rank)
}

/// The rank and the suit of a card index.
pub fn card_parts(index: u8) -> (u8, u8) {
    (index % RANKS.len() as u8, index / RANKS.len() as u8)
}

/// Bits the first `draws` cards of a shuffled deck carry: the first is
/// one of 52, the second one of the 51 left, and a fresh deck follows
/// the fifty-second. The fifty-second card of a deck carries none — it
/// is the one the other fifty-one left behind.
pub fn card_bits(draws: usize) -> f32 {
    let mut bits = 0.0;
    for i in 0..draws {
        bits += LOG2[DECK - i % DECK];
    }
    bits
}

/// Per-suit counts of a run of draws, and where the run stands in the
/// deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardStats {
    /// How often each suit came up, `suits[0]` for ♠.
    pub suits: [u32; 4],
    /// Which deck the next card is drawn from, counting from 1.
    pub deck: u8,
    /// Cards already drawn from that deck.
    pub in_deck: u32,
}

/// Accumulates cards drawn from a shuffled deck, refusing one that is
/// already out of it.
///
/// The entropy is SHA-256 of the draws' indices, one byte each, as dice
/// are the SHA-256 of their digits. The index is `suit * 13 + rank` with
/// the suits in [`SUITS`] order and the ranks in [`RANKS`] order, so a
/// person can reproduce it from a written-down list of cards.
pub struct CardDraws {
    cards: [u8; MAX_CARDS],
    len: u8,
}

impl Default for CardDraws {
    fn default() -> Self {
        Self::new()
    }
}

impl Zeroize for CardDraws {
    fn zeroize(&mut self) {
        self.cards.zeroize();
        self.len.zeroize();
    }
}

impl Drop for CardDraws {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for CardDraws {}

impl CardDraws {
    /// No draws yet.
    pub fn new() -> Self {
        CardDraws {
            cards: [0u8; MAX_CARDS],
            len: 0,
        }
    }

    /// The cards drawn so far, as indices.
    pub fn cards(&self) -> &[u8] {
        &self.cards[..self.len()]
    }

    /// Number of draws.
    pub fn len(&self) -> usize {
        usize::from(self.len)
    }

    /// Whether nothing has been drawn.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Draws from the current deck, counting from 1.
    pub fn in_deck(&self) -> usize {
        self.len() % DECK
    }

    /// Which deck the next card comes from, counting from 1: a deck is
    /// spent after fifty-two draws and the next card is the first of a
    /// freshly shuffled one.
    pub fn deck(&self) -> usize {
        self.len() / DECK + 1
    }

    /// Whether `index` is already out of the deck being drawn from.
    pub fn drawn(&self, index: u8) -> bool {
        let start = self.len() - self.in_deck();
        self.cards[start..self.len()].contains(&index)
    }

    /// Adds a draw. Returns `false` for an index past the deck, for a
    /// card already drawn from this deck, or when full.
    pub fn push(&mut self, index: u8) -> bool {
        if usize::from(index) >= DECK || self.len() >= MAX_CARDS || self.drawn(index) {
            return false;
        }
        self.cards[self.len()] = index;
        self.len += 1;
        true
    }

    /// Removes the last draw, putting it back in its deck.
    pub fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        let c = self.cards[self.len()];
        self.cards[self.len()] = 0;
        Some(c)
    }

    /// Forgets every draw.
    pub fn clear(&mut self) {
        self.zeroize();
    }

    /// The last card, if any.
    pub fn last(&self) -> Option<u8> {
        self.cards().last().copied()
    }

    /// Bits gathered so far ([`card_bits`]).
    pub fn bits_collected(&self) -> f32 {
        card_bits(self.len())
    }

    /// Draws needed for `strength`: the fewest whose [`card_bits`] reach
    /// it — 25, 31, 39, 50 and 58. The count climbs faster than the bits
    /// do because a draw is worth less as the deck empties. 58 is more
    /// than a deck holds, so 256 bits asks for a second shuffled deck.
    pub fn needed(strength: Strength) -> usize {
        let mut n = 0;
        while card_bits(n) < strength.bits() as f32 {
            n += 1;
        }
        n
    }

    /// Whether enough cards are in for `strength`.
    pub fn is_enough(&self, strength: Strength) -> bool {
        self.len() >= Self::needed(strength)
    }

    /// SHA-256 of the draws' indices, one byte each.
    pub fn commitment(&self) -> [u8; 32] {
        sha256(self.cards())
    }

    /// The commitment truncated to the strength.
    pub fn entropy(&self, strength: Strength) -> Result<RawEntropy, Error> {
        if !self.is_enough(strength) {
            return Err(Error::TooFew {
                have: self.len(),
                need: Self::needed(strength),
            });
        }
        let mut hash = self.commitment();
        let e = RawEntropy::new(&hash[..strength.bytes()]);
        hash.zeroize();
        Ok(e)
    }

    /// Per-suit counts and where the run stands in the deck.
    pub fn stats(&self) -> CardStats {
        let mut suits = [0u32; 4];
        for &c in self.cards() {
            suits[usize::from(card_parts(c).1)] += 1;
        }
        CardStats {
            suits,
            deck: self.deck() as u8,
            in_deck: self.in_deck() as u32,
        }
    }

    /// The longest run of draws each one after the last in the deck's
    /// own order (index + 1), or each one before it (index − 1): a deck
    /// in the order it left its box shows the whole suit.
    pub fn longest_in_order(&self) -> u32 {
        let (mut best, mut up, mut down) = (u32::from(!self.is_empty()), 1u32, 1u32);
        for w in self.cards().windows(2) {
            up = if w[1] == w[0].wrapping_add(1) {
                up + 1
            } else {
                1
            };
            down = if w[1].wrapping_add(1) == w[0] {
                down + 1
            } else {
                1
            };
            best = best.max(up).max(down);
        }
        best
    }

    /// The sanity flags for `strength`. A shuffled deck is not a die:
    /// its suits come out even by construction, so the count is tested,
    /// and a run of [`CARDS_IN_ORDER`] cards in the deck's own order,
    /// which a deck out of its box or one barely shuffled gives. The
    /// bits counted assume a well-shuffled deck (F158).
    pub fn warnings(&self, strength: Strength) -> Warnings {
        Warnings {
            too_few: !self.is_enough(strength),
            in_order: self.longest_in_order() >= CARDS_IN_ORDER,
            ..Warnings::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::known::{self, bytes, count, strength};
    use crate::{Error, Strength};

    /// The known answer of `n` cards (`tools/vectors/cards/`, whose README
    /// writes out the procedure and a few lines of Python that reproduce
    /// it): the cards as their rank and suit characters, their indices,
    /// the bits they carry to two places, the strength asked for, its
    /// commitment and entropy, and for 25 cards the 256 bits they are too
    /// few for. The file's ranks and suits are checked to be the crate's.
    fn known(n: usize) -> serde_json::Value {
        let v = known::json(known::CARDS);
        let chars = |k: &str| v[k].as_str().expect("characters").chars();
        assert!(chars("ranks").eq(RANKS.iter().copied()));
        assert!(chars("suits").eq(SUITS.iter().copied()));
        v["vectors"]
            .as_array()
            .expect("the vectors")
            .iter()
            .find(|c| c["indices"].as_array().map(|a| a.len()) == Some(n))
            .expect("a case of that many cards")
            .clone()
    }

    fn cards(k: &serde_json::Value) -> &str {
        k["cards"].as_str().expect("the cards")
    }

    /// The bits a run of `n` draws carries, worked out here rather than
    /// read from the table the crate ships: the i-th draw is one of 52 −
    /// (i mod 52).
    fn bits_by_hand(n: usize) -> f64 {
        let mut bits = 0.0f64;
        for i in 0..n {
            bits += log2(f64::from((52 - i % 52) as u32));
        }
        bits
    }

    /// log2 by repeated squaring, so that the expected bit counts are
    /// worked out here rather than read from the table the crate ships.
    fn log2(v: f64) -> f64 {
        let (mut x, mut exp) = (v, 0f64);
        while x >= 2.0 {
            x /= 2.0;
            exp += 1.0;
        }
        let (mut frac, mut w, mut y) = (0f64, 0.5f64, x);
        for _ in 0..52 {
            y *= y;
            if y >= 2.0 {
                y /= 2.0;
                frac += w;
            }
            w /= 2.0;
        }
        exp + frac
    }

    fn deal(spec: &str) -> CardDraws {
        let mut d = CardDraws::new();
        let mut chars = spec.chars();
        while let (Some(r), Some(u)) = (chars.next(), chars.next()) {
            let rank = RANKS.iter().position(|&c| c == r).unwrap() as u8;
            let suit = SUITS.iter().position(|&c| c == u).unwrap() as u8;
            assert!(d.push(card_index(rank, suit).unwrap()), "{r}{u}");
        }
        d
    }

    #[test]
    fn cards_hash_their_indices_and_count_the_deck_down() {
        // Twenty-five cards with no repeat: the whole spade suit and the
        // first twelve hearts.
        let k = known(25);
        let d = deal(cards(&k));
        assert_eq!(d.len(), 25);
        assert!(d.is_enough(Strength::Bits128));
        assert!(!d.is_enough(Strength::Bits256));
        assert_eq!(d.stats().suits, [13, 12, 0, 0]);
        assert_eq!(d.commitment()[..], bytes(&k["commitment"])[..]);
        let bits = bits_by_hand(25);
        assert!((bits - k["bits"].as_f64().unwrap()).abs() < 0.01, "{bits}");
        assert!(
            (f64::from(d.bits_collected()) - bits).abs() < 0.01,
            "{bits}"
        );
        assert!(bits > 128.0);
        let at = strength(&k["strength"]);
        assert_eq!(at, Strength::Bits128);
        let e = d.entropy(at).unwrap();
        assert_eq!(e.as_bytes(), &bytes(&k["entropy"])[..]);
        let few = &k["too_few"];
        assert_eq!(
            d.entropy(strength(&few["strength"])).err(),
            Some(Error::TooFew {
                have: count(&few["have"]),
                need: count(&few["need"]),
            })
        );
        // The counts are where the definition crosses the two targets.
        assert!(card_bits(24) < 128.0 && card_bits(25) >= 128.0);
        assert!(card_bits(57) < 256.0 && card_bits(58) >= 256.0);
    }

    /// A deck in the order it left its box is cautioned, counting up or
    /// down, and across a suit; a shuffled one is not, and a run one card
    /// short of the caution is not.
    #[test]
    fn a_deck_in_new_deck_order_is_cautioned() {
        let w = deal(cards(&known(25))).warnings(Strength::Bits128);
        assert!(w.in_order && !w.too_few && w.any());
        // King to ace of clubs, as some decks come.
        let mut down = CardDraws::new();
        for i in (39..52u8).rev() {
            assert!(down.push(i));
        }
        assert_eq!(down.longest_in_order(), 13);
        assert!(down.warnings(Strength::Bits128).in_order);
        // The queen and king of spades, then the ace to three of hearts.
        let mut across = CardDraws::new();
        for i in [11u8, 12, 13, 14, 15] {
            assert!(across.push(i));
        }
        assert!(across.warnings(Strength::Bits128).in_order);
        // Every other card, then a run of four in order.
        let mut shuffled = CardDraws::new();
        for i in 0..21u8 {
            assert!(shuffled.push(i * 2));
        }
        for i in [45u8, 46, 47, 48] {
            assert!(shuffled.push(i));
        }
        assert_eq!(shuffled.len(), 25);
        assert_eq!(shuffled.longest_in_order(), 4);
        let w = shuffled.warnings(Strength::Bits128);
        assert!(!w.in_order && !w.any());
    }

    #[test]
    fn the_deck_tracker_refuses_a_card_already_out() {
        let mut d = CardDraws::new();
        let ace_of_spades = card_index(0, 0).unwrap();
        assert!(d.push(ace_of_spades));
        assert!(d.drawn(ace_of_spades));
        assert!(!d.push(ace_of_spades), "already drawn");
        assert_eq!(d.len(), 1);
        assert!(!d.push(52), "no such card");
        // Putting it back frees it again.
        assert_eq!(d.pop(), Some(ace_of_spades));
        assert!(!d.drawn(ace_of_spades));
        assert!(d.push(ace_of_spades));
        d.clear();
        assert!(d.is_empty());
    }

    #[test]
    fn twenty_four_words_from_cards_asks_for_a_second_deck() {
        assert!(CardDraws::needed(Strength::Bits256) > DECK);
        let mut d = CardDraws::new();
        for i in 0..DECK as u8 {
            assert!(d.push(i));
        }
        assert_eq!(d.deck(), 2, "the fifty-third card is the second deck's");
        assert_eq!(d.in_deck(), 0);
        // The whole first deck is back in play.
        assert!(!d.drawn(0));
        assert!(d.push(0));
        assert_eq!(d.deck(), 2);
        assert!(!d.push(0), "and out of the second deck now");
        for i in 1..6u8 {
            assert!(d.push(i));
        }
        assert_eq!(d.len(), 58);
        assert!(d.is_enough(Strength::Bits256));
        assert_eq!(d.entropy(Strength::Bits256).unwrap().as_bytes().len(), 32);
        assert_eq!(d.stats().suits.iter().sum::<u32>(), 58);
    }

    /// A hand of cards reaches 160 bits at the thirty-first draw and not
    /// before, by the bits this file works out for itself.
    #[test]
    fn cards_reach_160_bits_at_the_thirty_first_draw() {
        // The spades, the hearts and five diamonds: indices 0 to 30.
        let k = known(31);
        // The first thirty, two characters each.
        let spec = cards(&k);
        let end = spec.char_indices().nth(60).map_or(spec.len(), |(i, _)| i);
        let mut d = deal(&spec[..end]);
        assert!(!d.is_enough(Strength::Bits160), "thirty draws is short");
        assert!(d.push(30));
        assert!(d.is_enough(Strength::Bits160));
        assert_eq!(d.commitment()[..], bytes(&k["commitment"])[..]);

        // The bits a run of draws from a fresh deck carries, summed here:
        // the first card is one of 52, the thirty-first one of 22.
        let mut thirty = 0f64;
        for i in 0..30 {
            thirty += log2(f64::from(52 - i as u32));
        }
        let all = thirty + log2(22.0);
        assert!(thirty < 160.0 && all >= 160.0, "{thirty} then {all}");
        assert!((bits_by_hand(31) - k["bits"].as_f64().unwrap()).abs() < 0.01);

        let at = strength(&k["strength"]);
        assert_eq!(at, Strength::Bits160);
        assert_eq!(d.entropy(at).unwrap().as_bytes(), &bytes(&k["entropy"])[..]);
    }
}
