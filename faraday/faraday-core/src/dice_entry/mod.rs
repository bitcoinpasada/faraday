//! Generate with dice on Create a vault: the rolls typed, and beside
//! them, on Faraday Demo, the roller (`docs/CODEX32.md` §5.6). A
//! rolled value lands on the rolls as a key pressed would, and is marked
//! as this computer's, so that the passphrase's strength line says
//! "simulated dice".
//!
//! [`DiceEntry`] holds the rolls, the list and the roller and knows
//! nothing of who opened it: the owner passes its focus, the time, the
//! randomness for the roller and how a [`Press`] is wrapped as an
//! `Action`, and takes back from Use the phrase, its bits and the
//! simulated marks.

use crate::fresh::roller::{Act, PerClick, Roller, Sim};
use crate::strings::{EN, fill};
use crate::vault::TextBox;
use faraday_shell_api::Key as K;
use zeroize::Zeroizing;

pub(crate) mod screen;

/// Bits of a passphrase from dice counted strong: six words of the EFF
/// long list (`docs/VAULT.md` §3.1).
pub const STRONG_BITS: u32 = 77;

/// The words this list needs for a strong passphrase, said with where the
/// lists come from: text, since Faraday is offline.
pub fn dice_aim(list: faraday_bip::diceware::List) -> String {
    let n = (1..=20)
        .find(|&n| list.bits(n) >= STRONG_BITS)
        .unwrap_or(20);
    fill(
        EN.vaults_aim_for_words,
        &[&n.to_string(), &STRONG_BITS.to_string()],
    )
}

/// The words a list's dice rolls spell, for as many whole words as the
/// rolls make (`faraday-bip`'s diceware lists, in roll order).
pub fn dice_words(rolls: &str, list: faraday_bip::diceware::List) -> Vec<&'static str> {
    let per = list.dice_per_word();
    let digits: Vec<u8> = rolls
        .bytes()
        .filter(|b| (b'1'..=b'6').contains(b))
        .map(|b| b - b'0')
        .collect();
    digits
        .chunks_exact(per)
        .filter_map(|w| list.word(w))
        .collect()
}

/// The place in its list of each whole word the rolls make, in order:
/// the dice read as a number in base 6, first die highest.
pub fn dice_word_indices(rolls: &str, list: faraday_bip::diceware::List) -> Vec<u16> {
    let digits: Vec<u16> = rolls
        .bytes()
        .filter(|b| (b'1'..=b'6').contains(b))
        .map(|b| u16::from(b - b'1'))
        .collect();
    digits
        .chunks_exact(list.dice_per_word())
        .map(|w| w.iter().fold(0u16, |n, d| n * 6 + d))
        .collect()
}

/// The most words the roller adds, one click or many.
const MAX_WORDS: usize = 20;

/// Where typing goes inside the entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The dice rolls.
    Dice,
    /// The roller's typed number of rolls per click.
    Rolls,
}

/// A press on the dice entry or the roller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// A key of the pad: a face, 1 to 6, or 0 for Backspace.
    Face(u8),
    /// The list's place in `faraday-bip`'s lists.
    List(u8),
    /// Use these words.
    Use,
    /// Put the dice away.
    Close,
    /// A press on the roller (Faraday Demo).
    Roller(Act),
}

/// What the owner gives a press: the time, whether Reduce motion is on
/// and the randomness for the roller (32 fresh bytes, or none yet).
pub struct Env<'a> {
    /// The clock, in milliseconds.
    pub now_ms: u64,
    /// Whether Reduce motion is on.
    pub reduce: bool,
    /// Asks for 32 fresh bytes.
    pub draw: &'a mut dyn FnMut() -> Option<[u8; 32]>,
}

/// What a press did for the owner.
#[derive(Default)]
pub struct Pressed {
    /// `Some(f)`: the focus is now `f` (and none when the entry is put
    /// away). `None`: the focus is as it was.
    pub focus: Option<Option<Part>>,
    /// Use: what the rolls made.
    pub used: Option<Used>,
}

/// What Use hands back: the words the rolls made.
pub struct Used {
    /// The owner's tag the entry was opened with.
    pub tag: usize,
    /// The words joined; empty when the rolls made none.
    pub phrase: Zeroizing<String>,
    /// Their bits.
    pub bits: f32,
    /// Which of the rolls this computer made.
    pub sim: Sim,
}

/// Generate with dice: the rolls typed, the list they are read in and
/// the roller, while the entry is open.
#[derive(Default)]
pub struct DiceEntry {
    /// The owner's tag, the rolls and the list, while open.
    open: Option<(usize, TextBox, faraday_bip::diceware::List)>,
    /// The roller beside the rolls (Faraday Demo).
    roller: Roller,
}

impl DiceEntry {
    /// The tag, rolls and list while the entry is open.
    pub fn open(&self) -> Option<&(usize, TextBox, faraday_bip::diceware::List)> {
        self.open.as_ref()
    }

    /// The rolls, to type into, while open.
    pub fn typing(&mut self) -> Option<&mut TextBox> {
        self.open.as_mut().map(|o| &mut o.1)
    }

    /// The roller.
    pub fn roller(&self) -> &Roller {
        &self.roller
    }

    /// Opens the entry for `tag`; pressed again for the same tag, the
    /// rolls box keeps what is rolled. The focus is the rolls.
    pub fn start(&mut self, tag: usize) -> Pressed {
        if self.open.as_ref().is_none_or(|d| d.0 != tag) {
            self.open = Some((tag, TextBox::default(), faraday_bip::diceware::List::Large));
            self.roller.reset();
        }
        Pressed {
            focus: Some(Some(Part::Dice)),
            used: None,
        }
    }

    /// Puts the entry away.
    pub fn close(&mut self) {
        self.open = None;
        self.roller.reset();
    }

    /// How many more rolls the entry takes now: up to [`MAX_WORDS`]
    /// words' worth, and for To the end only as far as a strong
    /// passphrase (a lookup).
    pub fn room(&self) -> usize {
        let Some((_, rolls, list)) = self.open.as_ref() else {
            return 0;
        };
        let per = list.dice_per_word();
        let end = if self.roller.per() == PerClick::ToEnd {
            (STRONG_BITS as f32 / list.bits_per_word()).ceil() as usize
        } else {
            MAX_WORDS
        };
        (end * per).saturating_sub(rolls.text.len())
    }

    /// A press. The roller's is the owner's to allow (Faraday Demo).
    pub fn press(&mut self, p: Press, env: &mut Env) -> Pressed {
        let mut out = Pressed::default();
        match p {
            Press::Face(0) => {
                if let Some((_, rolls, _)) = self.open.as_mut() {
                    rolls.text.pop();
                    let len = rolls.text.len();
                    self.roller.trim(len);
                }
            }
            Press::Face(f @ 1..=6) => {
                if let Some((_, rolls, _)) = self.open.as_mut() {
                    rolls.text.push(char::from(b'0' + f));
                }
            }
            Press::Face(_) => {}
            Press::List(k) => {
                if let Some((_, rolls, list)) = self.open.as_mut()
                    && let Some(&l) = faraday_bip::diceware::List::ALL.get(usize::from(k))
                    && l != *list
                {
                    *list = l;
                    rolls.clear();
                    self.roller.reset();
                }
            }
            Press::Use => {
                out.used = self.use_words();
                if out.used.is_some() {
                    out.focus = Some(None);
                }
            }
            Press::Close => {
                self.close();
                out.focus = Some(None);
            }
            Press::Roller(r) => {
                let (room, now) = (self.room(), env.now_ms);
                self.roller.act(r, room, now);
                out.focus = Some(Some(if r == Act::Count {
                    Part::Rolls
                } else {
                    Part::Dice
                }));
                if r == Act::Roll {
                    self.tick(env);
                }
            }
        }
        out
    }

    /// Takes the words the rolls made and puts the entry away.
    fn use_words(&mut self) -> Option<Used> {
        let (tag, rolls, list) = self.open.take()?;
        let words = dice_words(&rolls.text, list);
        let sim = self.roller.sim(words.len() * list.dice_per_word());
        self.roller.reset();
        Some(Used {
            tag,
            phrase: Zeroizing::new(words.join("")),
            bits: words.len() as f32 * list.bits_per_word(),
            sim,
        })
    }

    /// The roller's tick: the values it made land on the rolls. Returns
    /// whether the screen needs a frame.
    pub fn tick(&mut self, env: &mut Env) -> bool {
        if !self.roller.active() {
            return false;
        }
        let room = self.room();
        let Some(len) = self.open.as_ref().map(|d| d.1.text.len()) else {
            self.roller.reset();
            return false;
        };
        let (values, redraw) = self
            .roller
            .tick(env.now_ms, env.reduce, len, room, &mut *env.draw);
        if let Some((_, rolls, _)) = self.open.as_mut() {
            for v in values {
                rolls.text.push(char::from(b'0' + v));
            }
        }
        redraw
    }

    /// Typing while the rolls or the roller's number has the keys.
    pub fn key(&mut self, part: Part, key: K, env: &mut Env) -> Pressed {
        let mut out = Pressed::default();
        if part == Part::Rolls {
            match key {
                K::Char(c) => self.roller.digit(c),
                K::Backspace => self.roller.backspace(),
                K::Enter => out.focus = Some(Some(Part::Dice)),
                _ => {}
            }
            return out;
        }
        if let Some((_, rolls, _)) = self.open.as_mut() {
            match key {
                K::Char(c) if ('1'..='6').contains(&c) => rolls.text.push(c),
                K::Backspace => {
                    rolls.text.pop();
                    let len = rolls.text.len();
                    self.roller.trim(len);
                }
                K::Enter => return self.press(Press::Use, env),
                _ => {}
            }
        }
        out
    }
}
