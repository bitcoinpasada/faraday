//! New key's entries as they come in: a roll, a flip, a card, a digit,
//! taken back or cleared, and what the box takes. The flow's state is
//! [`KeyGen`] in `mod.rs`.

use super::{KeyGen, Source, kstep};
use crate::strings::EN;

impl KeyGen {
    /// Forgets every entry, the mix and the words.
    pub fn clear_entries(&mut self) {
        self.clear_entered();
        self.roller.reset();
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
        self.roller.reset();
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
        // A roll taken back is not the roller's any more.
        let len = self.progress().0;
        self.roller.trim(len);
        self.note = None;
    }

    /// A key in the Key card's passphrase field that has the keys:
    /// printable ASCII, as BIP-39's seed derivation reads it; Tab moves
    /// to the other field. Returns whether it took the key.
    pub(crate) fn passphrase_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        let Some(f) = self.focus else {
            return false;
        };
        let field = if f == 0 {
            &mut self.passphrase
        } else {
            &mut self.passphrase2
        };
        match key {
            K::Char(c) if (' '..='~').contains(&c) => field.push(c),
            // Anything else is refused, and the card says so.
            K::Char(c) if !c.is_control() => {
                self.note = Some(crate::state::ascii_refused());
                return true;
            }
            K::Backspace => {
                field.pop();
            }
            K::Tab => self.focus = Some(1 - f.min(1)),
            K::Escape => self.focus = None,
            _ => return false,
        }
        self.note = None;
        true
    }

    /// A key while the roller's number field has the keys: digits until
    /// Enter or Escape. Returns whether it took the key.
    pub(crate) fn roller_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        match key {
            K::Char(c) => self.roller.digit(c),
            K::Backspace => self.roller.backspace(),
            K::Enter | K::Escape => self.roller_typing = false,
            _ => return false,
        }
        true
    }

    /// How many more rolls the dice pad takes now, for the roller beside
    /// it (`docs/CODEX32.md` §5.6): none unless dice are being entered
    /// (a lookup).
    pub(crate) fn roller_room(&self) -> usize {
        if self.active() != Some(Source::Dice) || self.locked {
            return 0;
        }
        let (have, need) = self.progress();
        need.saturating_sub(have)
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
}
