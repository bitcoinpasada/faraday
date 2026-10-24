//! Make codex32 shares: the main flow (`docs/CODEX32.md` §6), its cards'
//! state and what each press does. The screen is [`screen`].
//!
//! The person chooses the scheme, types the identifier, and puts in the
//! entropy a character at a time, rolled on the booklet's dice
//! de-biasing worksheet or typed; the sheets fill as each character
//! arrives. The secret work is `faraday-keys`' [`WorkSet`]: the
//! characters go to it as each is complete, and it keeps every
//! worksheet's cells, lent to drawing only as a paper form. What is kept
//! here is the transient input (the rolls of the character in progress,
//! as the booklet's sheet shows them) and how each character was made,
//! for the worksheets' review and every cell's working (§7.4).
//!
//! **Kept in step.** `made[s]` holds one record per data character
//! initial share `s` has in the work set, in box order: a push or a pop
//! of a character changes both, and nothing else changes either. The
//! rolls of the character in progress are `seq`; `dice` and `rows` are
//! what `seq` gives, worked again from it at every press
//! ([`Codex32Make::replay`]). `strings`, `fingerprint` and `qr` are
//! taken from the finished set when its last character arrives and
//! dropped when any character is taken back. On Faraday Desktop the
//! five-dice roller (`docs/CODEX32.md` §5.5) stands beside the pad: its
//! faces enter through [`Codex32Make::roll`] as the pad's keys do, it is
//! told of every face taken back ([`FiveRoller::trim`]) and starts over
//! with the entry; `rolled_dice` is what `seq` and the roller's marks
//! give, worked again with `dice` at every replay.
//!
//! **The dice** (§5.1): the person says how many faces their dice have
//! (six unless told, up to the booklet's twenty) and each die's colour
//! ([`DiceChoice`]). The faces are read 1 to N on the pad and the roller
//! rolls N; a change of dice is taken only between characters, so that
//! `seq` is always of one die, and each character keeps the faces it
//! was rolled with ([`Made`]) for its rolls to come back on Backspace.

pub(crate) mod dice_screen;
mod pan;
pub(crate) mod pane_screen;
pub(crate) mod screen;
mod walk;
pub(crate) mod walk_screen;
#[cfg(test)]
mod walk_tests;
pub(crate) mod worksheet_screen;

use pan::SheetFit;
pub use pan::SheetPan;
pub use walk::{SPEEDS, Speed, Step, Walk, WalkAct, WalkChar};

/// Making codex32 shares, and the wallet kind of §6.9, are offered. False
/// hides them until the backup plan carries codex32 strings: New key's
/// Start card then does not offer Make a new key › Codex32. Typing codex32
/// strings in Add a key stays either way.
pub const MAKE_OFFERED: bool = true;

use faraday_bip::codex32::{Length, Scheme};
use faraday_codec::qr::QrMatrix;
use faraday_crypto::work::Compute;
use faraday_entropy::booklet::{self, BOOKLET_DICE, BookletChar};
use faraday_keys::{Held, Making, Reveal, WorkSet};
pub use faraday_ui::keyboard::OskPress;
use zeroize::{Zeroize, Zeroizing};

use crate::fresh::roller::Sim;
use crate::fresh::roller::five::FiveRoller;
use crate::secret_text::SecretText;
use crate::strings::{EN, fill};

/// The bech32 alphabet in value order: a symbol's character.
pub const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// A character's symbol value, either case; `None` outside the alphabet.
pub fn symbol_of(c: char) -> Option<u8> {
    let c = c.to_ascii_lowercase();
    CHARSET
        .iter()
        .position(|&x| char::from(x) == c)
        .map(|p| p as u8)
}

/// A symbol's character, lowercase.
pub fn char_of(v: u8) -> char {
    char::from(CHARSET[usize::from(v & 31)])
}

/// The flow's cards, in order (`docs/CODEX32.md` §6.1 to §6.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Card {
    /// Length and split (§6.1).
    Scheme,
    /// The identifier, typed (§6.2).
    Identifier,
    /// Dice or characters typed (§6.3).
    Entropy,
    /// The sheets on screen, filling live (§6.4).
    Enter,
    /// The worksheets read back, a tab each (§6.5).
    Sheets,
    /// One string at a time, typed back (§6.6).
    Strings,
    /// Strings made: the Result, and Lock in (§6.7).
    Made,
}

/// The cards in order. Actions carry a card by its place here.
pub const CARDS: [Card; 7] = [
    Card::Scheme,
    Card::Identifier,
    Card::Entropy,
    Card::Enter,
    Card::Sheets,
    Card::Strings,
    Card::Made,
];

impl Card {
    /// Its place in [`CARDS`].
    pub fn index(self) -> u8 {
        CARDS.iter().position(|c| *c == self).unwrap_or(0) as u8
    }
}

/// The Scheme card's split (§6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    /// Any 2 of 3 strings.
    TwoOfThree,
    /// Any 3 of 5.
    ThreeOfFive,
    /// The threshold and the strings by steppers.
    Other,
    /// No threshold: the secret alone (k = 0).
    Alone,
}

/// The splits in the order the card lists them.
pub const SPLITS: [Split; 4] = [
    Split::TwoOfThree,
    Split::ThreeOfFive,
    Split::Other,
    Split::Alone,
];

/// Where the entropy comes from (§6.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Way {
    /// Five dice by the booklet's de-biasing worksheet (§5.1).
    Dice,
    /// Characters made on paper, typed (§5.2).
    Typed,
    /// Shares made by hand, each string typed whole (§5.3, §8.1): the
    /// flow hands over to Add a key's codex32 form, Make the rest on.
    Shares,
}

/// The ways in the order the card lists them.
pub const WAYS: [Way; 3] = [Way::Dice, Way::Typed, Way::Shares];

/// The dice offered, by their faces (§5.1): the booklet's sheet has pads
/// 1 to 6 and, for larger dice, 7 to 20, so every die of up to twenty
/// faces fits; these are the dice sold in sets.
pub const DIE_SIZES: [u8; 6] = [4, 6, 8, 10, 12, 20];

/// The columns of keys of the pad of a die of `n` faces: five where
/// they divide it (d10, d20), six (d12), else four (d4, d8).
pub fn pad_columns(n: usize) -> usize {
    if n.is_multiple_of(5) {
        5
    } else if n.is_multiple_of(6) {
        6
    } else {
        4
    }
}

/// A die's colour (§5.1, owner 2026-10-24): the booklet asks for five
/// distinct dice, and a die's marker takes its colour. Drawn with its
/// name and the die's number, so colour is never the only cue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DieColour {
    /// Red.
    Red,
    /// Orange.
    Orange,
    /// Yellow.
    Yellow,
    /// Green.
    Green,
    /// Blue.
    Blue,
    /// Purple.
    Purple,
    /// White.
    White,
    /// Black.
    Black,
}

/// The colours, in the order a press steps through them.
pub const DIE_COLOURS: [DieColour; 8] = [
    DieColour::Red,
    DieColour::Orange,
    DieColour::Yellow,
    DieColour::Green,
    DieColour::Blue,
    DieColour::Purple,
    DieColour::White,
    DieColour::Black,
];

impl DieColour {
    /// The colour after this one, round the list.
    pub fn next(self) -> DieColour {
        let i = DIE_COLOURS.iter().position(|c| *c == self).unwrap_or(0);
        DIE_COLOURS[(i + 1) % DIE_COLOURS.len()]
    }
}

/// The dice the person rolls: their faces, and each die's colour, die 1
/// first. Kept for the session (`KeyGenArea`), not in the settings file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiceChoice {
    /// The faces of each die, one of [`DIE_SIZES`].
    pub faces: u8,
    /// Die 1's colour to die 5's.
    pub colours: [DieColour; BOOKLET_DICE],
}

impl Default for DiceChoice {
    /// Six-faced dice in five distinct colours.
    fn default() -> Self {
        DiceChoice {
            faces: booklet::BOOKLET_FACES,
            colours: [
                DieColour::Red,
                DieColour::Blue,
                DieColour::Green,
                DieColour::Yellow,
                DieColour::Purple,
            ],
        }
    }
}

/// A set made of strings typed into Add a key's codex32 form
/// (`docs/CODEX32.md` §8.1, §8.2): the flow opens on its first card with
/// the strings in, and the identifier, the entropy and the entry are the
/// strings'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Given {
    /// **Make the rest** (§8.1 step 2): how many strings in all is asked,
    /// then every string not typed is made.
    Rest,
    /// **Add a share** (§8.2): the index is asked, then its string made.
    Add,
}

/// A cell of a worksheet, as a press names it to open its working
/// (`docs/CODEX32.md` §7.4). `sheet` is an initial share's place for a
/// checksum worksheet, a target's place ([`WorkSet::targets`]) for a
/// translation worksheet; `b` a box, `j` a step, `i` a cell from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    /// A given character of a checksum worksheet's string.
    Given {
        /// The worksheet.
        sheet: u8,
        /// The box.
        b: u8,
    },
    /// The residue of `ms`, under the first row.
    Hrp {
        /// The worksheet.
        sheet: u8,
        /// The cell, from 0.
        i: u8,
    },
    /// A sum row's cell (the row past the last step is the final row).
    Sum {
        /// The worksheet.
        sheet: u8,
        /// The step.
        j: u8,
        /// The cell, from 0.
        i: u8,
    },
    /// A lookup row's cell.
    Lookup {
        /// The worksheet.
        sheet: u8,
        /// The step.
        j: u8,
        /// The cell, from 0.
        i: u8,
    },
    /// A translation worksheet's symbol for input `i`.
    Symbol {
        /// The worksheet.
        sheet: u8,
        /// The input.
        i: u8,
    },
    /// Input `i`'s character at box `b`, translated.
    Translated {
        /// The worksheet.
        sheet: u8,
        /// The input.
        i: u8,
        /// The box.
        b: u8,
    },
    /// The sum of the translated characters at box `b`.
    TSum {
        /// The worksheet.
        sheet: u8,
        /// The box.
        b: u8,
    },
}

/// A press inside Make codex32 shares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum C32 {
    /// A card's header: opens it, or closes it while open.
    Card(u8),
    /// The open card's Continue.
    Next,
    /// The length, by its place: 48 or 74 characters.
    Length(u8),
    /// The split, by its place in [`SPLITS`].
    Split(u8),
    /// Other's threshold stepped.
    K(i8),
    /// Other's strings stepped.
    N(i8),
    /// A key of the codex32 keyboard or the dice pad, to the open card.
    Key(OskPress),
    /// Four characters from this device's randomness for the identifier.
    Suggest,
    /// The way, by its place in [`WAYS`].
    Way(u8),
    /// A worksheet tab (§6.5), by its place.
    Tab(u8),
    /// The tab before or after.
    TabStep(i8),
    /// Which worksheet the entry shows beside the dice: 0 the share's
    /// checksum worksheet, `n` the `n`-th translation worksheet.
    View(u8),
    /// The small panel's context: 0 the dice, 1 the checksum, 2 the
    /// string (§6.4.7).
    Panel(u8),
    /// A cell's working opened (§7.4).
    Cell(Cell),
    /// The working closed.
    CellClose,
    /// The Codex32QR code in place of the string, and back (§6.6).
    Qr,
    /// The string's typing back skipped: a second press skips it.
    Skip,
    /// Check on another device, opened or closed (§6.8).
    Check,
    /// Lock in: the key loaded from the set (§6.7).
    Lock,
    /// The eye in the app bar: the sheets masked, or shown (§6.4.6).
    Eye,
    /// The open cell's working moved to the neighbouring cell of its row,
    /// before (-1) or after (1) (§7.4).
    CellStep(i8),
    /// A press on the step-through of the "Demo only" card: the computer's
    /// five dice, a step at a time (Faraday Desktop, §5.5).
    Walk(WalkAct),
    /// On a narrow window, the "Demo only" card opened from its strip at
    /// the foot, or closed back to it (§5.5).
    Pane,
    /// The dice's faces, by their place in [`DIE_SIZES`] (§5.1): in the
    /// middle of a character, its rolls so far are cleared.
    Faces(u8),
    /// Die `n`'s colour (from 0) steps to the next of [`DIE_COLOURS`].
    Colour(u8),
    /// A face pressed on the pad of dice other than six-faced ones.
    Face(u8),
    /// **Use a sample** on the entry, in Demo (§12): the characters left,
    /// from the sample's shares.
    Sample,
}

/// One die's two values for one character, as entered: struck where
/// they are equal and the die was rolled again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Zeroize)]
pub struct DieRow {
    /// The die, from 0.
    pub die: u8,
    /// Its first value.
    pub first: u8,
    /// Its second value.
    pub second: u8,
}

impl DieRow {
    /// Its bit: 0 lower, 1 higher; `None` for an equal pair, struck.
    pub fn bit(self) -> Option<u8> {
        booklet::bit(self.first, self.second)
    }
}

/// A rolled character's faces, in order, and the pairs they completed.
type Rolled = (Zeroizing<Vec<u8>>, Zeroizing<Vec<DieRow>>);

/// How one data character was made: the dice rows that made it, struck
/// pairs included, in the order they were completed, or none where it
/// was typed.
#[derive(Default)]
pub struct Made {
    /// Every face entered for it, in order.
    seq: Zeroizing<Vec<u8>>,
    /// The completed pairs, in order.
    rows: Zeroizing<Vec<DieRow>>,
    /// The symbol it gave.
    value: u8,
    /// It was typed, not rolled.
    typed: bool,
    /// The faces of the dice it was rolled with.
    faces: u8,
}

impl Drop for Made {
    fn drop(&mut self) {
        self.value.zeroize();
    }
}

impl Made {
    /// The completed pairs, struck ones included, in order.
    pub fn rows(&self) -> &[DieRow] {
        &self.rows
    }

    /// The symbol.
    pub fn value(&self) -> u8 {
        self.value
    }

    /// It was typed.
    pub fn typed(&self) -> bool {
        self.typed
    }

    /// The faces of the dice it was rolled with (6 for a typed one).
    pub fn faces(&self) -> u8 {
        self.faces
    }

    /// The five bits, die 1's first.
    pub fn bits(&self) -> [u8; BOOKLET_DICE] {
        let v = self.value;
        [v >> 4 & 1, v >> 3 & 1, v >> 2 & 1, v >> 1 & 1, v & 1]
    }
}

/// Where one 48- or 74-character set's strings stand on the Strings card.
#[derive(Default)]
struct Back {
    /// What is typed back of the string shown, from box 4.
    typed: SecretText,
    /// The first box that differs, once the whole string is typed.
    differs: Option<usize>,
    /// Each string typed back and found the same, or skipped.
    checked: Vec<bool>,
    /// The person asked to skip; a second press skips.
    skip_ask: bool,
}

/// The flow, open inside New key while its Start card's choice is
/// Codex32 (`docs/CODEX32.md` §6). See the module documentation.
pub struct Codex32Make {
    open: Option<Card>,
    done: [bool; 7],
    length: Length,
    split: Split,
    other_k: u8,
    other_n: u8,
    /// The scheme came from Make a new wallet's Quorum (§6.9): the Scheme
    /// card is closed on it and offers no change.
    fixed: bool,
    id: [u8; 4],
    id_len: u8,
    /// Identifiers already used on this device, with where.
    taken: Vec<([u8; 4], String)>,
    way: Way,
    work: Option<WorkSet>,
    /// The faces entered for the character in progress, in order.
    seq: Zeroizing<Vec<u8>>,
    /// What `seq` gives on the booklet's sheet.
    dice: BookletChar,
    /// The pairs `seq` completed, struck ones included.
    rows: Zeroizing<Vec<DieRow>>,
    /// The die whose pair just came out equal, until the next face.
    equal: Option<u8>,
    /// The dice the person rolls: faces and colours.
    dice_choice: DiceChoice,
    /// A tens digit typed on the keyboard for dice of ten faces or more,
    /// waiting for its units digit, Space or Enter.
    pending: Option<u8>,
    made: Vec<Vec<Made>>,
    rolled: bool,
    typed: bool,
    tab: u8,
    seen: Vec<bool>,
    view: u8,
    panel: u8,
    cell: Option<Cell>,
    strings: Option<Held<Vec<Zeroizing<String>>>>,
    fingerprint: Option<[u8; 4]>,
    at: usize,
    qr: Option<QrMatrix>,
    qr_on: bool,
    back: Back,
    check_open: bool,
    note: Option<String>,
    locked: bool,
    /// The sheets are masked (§6.4.6): every character, roll and working
    /// drawn as dots. Set when the flow is left backwards, and by the eye.
    hidden: bool,
    /// Faraday Desktop: never masked, and no eye (§6.4.6).
    unmasked: bool,
    /// The five-dice roller beside the pad (Faraday Desktop, §5.5).
    roller: FiveRoller,
    /// Which dice of the character in progress show a value the roller
    /// made: worked out from `seq` at every replay.
    rolled_dice: [bool; BOOKLET_DICE],
    /// Whether the set's faces came from the roller, in part or all:
    /// taken when the set completes, for Strings made.
    sim: Sim,
    /// A set made of strings typed in, and what it makes (§8.1, §8.2).
    given: Option<Given>,
    /// The strings typed in, until the set is made of them.
    inputs: Vec<faraday_bip::codex32::Codex32>,
    /// Add a share's index, lowercase.
    add_at: u8,
    /// The strongest mark the strings typed in carry (§8.4), for the key
    /// Lock in loads.
    mark: Option<crate::codex32_correct::Mark>,
    /// The Entropy card's Continue on shares made by hand: New key hands
    /// over to Add a key's codex32 form.
    hand: bool,
    /// Demo put its sample in place of the set entered (§12): every card
    /// after the entry shows the sample's.
    sample: bool,
    /// In Demo, the sample's published strings, which stand in place of
    /// the set entered (none where the shell gave no sample); `None`
    /// outside Demo.
    demo: Option<Vec<faraday_bip::codex32::Codex32>>,
    /// How far the checksum worksheet shown is panned: the entry's, or
    /// the review's tab's, which starts at its top left.
    pan: SheetPan,
    /// Whether the checksum worksheet is drawn whole and at which cell
    /// (`docs/LAYOUT.md` §16.1): kept in step with the window and the
    /// sheet's length by [`Codex32Make::refit`].
    fit: SheetFit,
    /// The page's own scroll following the work while the worksheet is
    /// drawn whole (`docs/LAYOUT.md` §16.4): its `y` is the page's
    /// offset while it moves.
    page: SheetPan,
    /// The step-through of the "Demo only" card (§5.5).
    walk: Walk,
    /// How far the "Demo only" card is scrolled within itself, units.
    pane_y: f32,
    /// On a narrow window, the card opened from its strip at the foot.
    pane_open: bool,
}

impl Codex32Make {
    /// A fresh flow, on the Scheme card. `taken`: the identifiers other
    /// sets on this device carry, each with where (§6.4.5).
    pub fn new(taken: Vec<([u8; 4], String)>) -> Codex32Make {
        Codex32Make {
            open: Some(Card::Scheme),
            done: [false; 7],
            length: Length::Chars48,
            split: Split::TwoOfThree,
            other_k: 2,
            other_n: 3,
            fixed: false,
            id: [0; 4],
            id_len: 0,
            taken,
            way: Way::Dice,
            work: None,
            seq: Zeroizing::new(Vec::new()),
            dice: BookletChar::new(),
            rows: Zeroizing::new(Vec::new()),
            equal: None,
            dice_choice: DiceChoice::default(),
            pending: None,
            made: Vec::new(),
            rolled: false,
            typed: false,
            tab: 0,
            seen: Vec::new(),
            view: 0,
            panel: 0,
            cell: None,
            strings: None,
            fingerprint: None,
            at: 0,
            qr: None,
            qr_on: false,
            back: Back::default(),
            check_open: false,
            note: None,
            locked: false,
            hidden: false,
            unmasked: false,
            roller: FiveRoller::default(),
            rolled_dice: [false; BOOKLET_DICE],
            sim: Sim::No,
            given: None,
            inputs: Vec::new(),
            add_at: b'a',
            mark: None,
            hand: false,
            sample: false,
            demo: None,
            pan: SheetPan::default(),
            fit: SheetFit::default(),
            page: SheetPan::default(),
            walk: Walk::default(),
            pane_y: 0.0,
            pane_open: false,
        }
    }

    /// The flow in Demo (§12, `docs/DEMO.md` §5): `sample` is the
    /// published set that stands in place of the strings entered, the
    /// shares of BIP 93's vector 2. Cheap.
    pub(crate) fn in_demo(mut self, sample: Vec<faraday_bip::codex32::Codex32>) -> Codex32Make {
        self.demo = Some(sample);
        self
    }

    /// Whether the flow runs in Demo.
    pub fn demo(&self) -> bool {
        self.demo.is_some()
    }

    /// The flow on `strings`, `k` strings of one set typed into Add a
    /// key's codex32 form (§8.1, §8.2), `mark` the strongest they carry:
    /// open on its first card, the identifier, the entropy and the entry
    /// being the strings'. Cheap.
    pub fn given(
        kind: Given,
        strings: Vec<faraday_bip::codex32::Codex32>,
        mark: Option<crate::codex32_correct::Mark>,
    ) -> Codex32Make {
        let mut m = Codex32Make::new(Vec::new());
        let k = strings.first().map_or(2, |s| s.threshold());
        let chars = strings.first().map_or(48, |s| s.encode().as_str().len());
        m.length = Length::of_chars(chars).unwrap_or(Length::Chars48);
        if let Some(first) = strings.first() {
            m.id = first
                .identifier()
                .map(|c| symbol_of(char::from(c)).unwrap_or(0));
            m.id_len = 4;
        }
        m.split = Split::Other;
        m.other_k = k;
        m.other_n = (k + 1).min(faraday_bip::codex32::MAX_SHARES);
        let held: Vec<u8> = strings.iter().map(|s| s.share_index()).collect();
        m.add_at = faraday_bip::codex32::INDEX_ORDER
            .iter()
            .copied()
            .find(|i| !held.contains(i))
            .unwrap_or(b'a');
        for c in [Card::Identifier, Card::Entropy, Card::Enter] {
            m.done[usize::from(c.index())] = true;
        }
        m.way = Way::Typed;
        m.given = Some(kind);
        m.inputs = strings;
        m.mark = mark;
        m
    }

    /// The flow with the dice the person chose earlier this session.
    pub(crate) fn with_dice(mut self, choice: DiceChoice) -> Codex32Make {
        if faraday_entropy::booklet::faces_ok(choice.faces) {
            self.dice_choice = choice;
            self.dice = BookletChar::with_faces(choice.faces).unwrap_or_default();
            self.roller.set_faces(choice.faces);
        }
        self
    }

    /// The flow on Faraday Desktop, where the sheets are never masked
    /// (§6.4.6).
    pub(crate) fn desktop(mut self) -> Codex32Make {
        self.unmasked = true;
        self
    }

    /// The flow for a wallet whose Quorum chose `k` of `n` (§6.9): the
    /// Scheme card closed on it, the identifier open.
    pub fn with_scheme(k: u8, n: u8, taken: Vec<([u8; 4], String)>) -> Codex32Make {
        let mut m = Codex32Make::new(taken);
        m.split = Split::Other;
        m.other_k = k.clamp(2, faraday_bip::codex32::MAX_THRESHOLD);
        m.other_n = n.clamp(m.other_k, faraday_bip::codex32::MAX_SHARES);
        m.fixed = true;
        m.done[usize::from(Card::Scheme.index())] = true;
        m.open = Some(Card::Identifier);
        m
    }

    // ----- What drawing reads -----

    /// A set made of strings typed in, and what it makes.
    pub fn given_kind(&self) -> Option<Given> {
        self.given
    }

    /// Add a share's index, lowercase.
    pub fn add_at(&self) -> u8 {
        self.add_at
    }

    /// Whether Add a share's index is one of the strings typed in, or the
    /// secret's: the index then makes nothing.
    pub fn add_at_taken(&self) -> bool {
        self.add_at == b's'
            || self.inputs.iter().any(|s| s.share_index() == self.add_at)
            || self
                .work
                .as_ref()
                .is_some_and(|w| w.inputs().contains(&self.add_at))
    }

    /// The share indices of the strings typed in, lowercase; none for a
    /// fresh set. Cheap.
    pub fn input_indices(&self) -> Vec<u8> {
        match self.work.as_ref() {
            Some(w) if w.is_given() => w.inputs().to_vec(),
            _ => self.inputs.iter().map(|s| s.share_index()).collect(),
        }
    }

    /// The Entropy card went on with shares made by hand (§5.3): they are
    /// typed on Add a key's codex32 form, Make the rest on.
    pub fn by_hand(&self) -> bool {
        self.hand
    }

    /// Make the rest's strings in all set to `n`, as the Scheme card of a
    /// set to be made by hand chose (§8.1 step 2): one more than the
    /// threshold at least.
    pub fn rest_of(mut self, n: u8) -> Codex32Make {
        self.other_n = n.clamp(
            (self.other_k + 1).min(faraday_bip::codex32::MAX_SHARES),
            faraday_bip::codex32::MAX_SHARES,
        );
        self
    }

    /// Demo's sample stands in place of the set entered (§12).
    pub fn sample(&self) -> bool {
        self.sample
    }

    /// The strongest mark the strings typed in carry (§8.4).
    pub fn mark(&self) -> Option<crate::codex32_correct::Mark> {
        self.mark
    }

    /// The open card.
    pub fn open(&self) -> Option<Card> {
        self.open
    }

    /// Whether card `c` is done.
    pub fn done(&self, c: Card) -> bool {
        self.done[usize::from(c.index())]
    }

    /// The sheets are masked now: what the person typed or rolled, and
    /// all that it becomes, is drawn as dots (§6.4.6, DESIGN §4.10).
    pub fn masked(&self) -> bool {
        self.hidden && !self.unmasked
    }

    /// The app bar offers the eye: sheets exist, on a device (not on
    /// Faraday Desktop).
    pub fn eye_offered(&self) -> bool {
        !self.unmasked && self.work.is_some()
    }

    /// The scheme was fixed by the wallet's Quorum.
    pub fn fixed(&self) -> bool {
        self.fixed
    }

    /// The length chosen.
    pub fn length(&self) -> Length {
        self.length
    }

    /// The split chosen.
    pub fn split(&self) -> Split {
        self.split
    }

    /// The threshold and the strings the split makes: (0, 1) alone.
    pub fn k_n(&self) -> (u8, u8) {
        match self.split {
            Split::TwoOfThree => (2, 3),
            Split::ThreeOfFive => (3, 5),
            Split::Other => (self.other_k, self.other_n),
            Split::Alone => (0, 1),
        }
    }

    /// The identifier typed so far, lowercase.
    pub fn identifier(&self) -> String {
        self.id[..usize::from(self.id_len)]
            .iter()
            .map(|&v| char_of(v))
            .collect()
    }

    /// Why the identifier typed is refused: where it is used already.
    pub fn id_refused(&self) -> Option<&str> {
        if self.id_len < 4 {
            return None;
        }
        let id = self.id.map(|v| CHARSET[usize::from(v)]);
        self.taken
            .iter()
            .find(|(t, _)| *t == id)
            .map(|(_, w)| w.as_str())
    }

    /// The way chosen.
    pub fn way(&self) -> Way {
        self.way
    }

    /// The work set, once the entry has begun.
    pub fn work(&self) -> Option<&WorkSet> {
        self.work.as_ref()
    }

    /// The initial share being entered, and how many characters it has:
    /// `None` once every one is complete.
    pub fn front(&self) -> Option<(usize, usize)> {
        let w = self.work.as_ref()?;
        let data = w.scheme().length().data_len();
        (0..w.inputs().len())
            .map(|s| (s, w.entered(s)))
            .find(|&(_, n)| n < data)
    }

    /// How data character `n` of initial share `share` was made.
    pub fn made_of(&self, share: usize, n: usize) -> Option<&Made> {
        self.made.get(share)?.get(n)
    }

    /// The rolls of the character in progress on the booklet's sheet.
    pub fn dice(&self) -> &BookletChar {
        &self.dice
    }

    /// The pairs the character in progress has completed, struck ones
    /// included.
    pub fn rows(&self) -> &[DieRow] {
        &self.rows
    }

    /// The die whose pair just came out equal, to be rolled again.
    pub fn equal(&self) -> Option<u8> {
        self.equal
    }

    /// The dice the person rolls: faces and colours.
    pub fn dice_choice(&self) -> DiceChoice {
        self.dice_choice
    }

    /// Whether the dice can be changed now: no character in progress.
    pub fn dice_open(&self) -> bool {
        self.seq.is_empty() && self.pending.is_none()
    }

    /// The tens digit typed and waiting, if any.
    pub fn pending(&self) -> Option<u8> {
        self.pending
    }

    /// How much taller than the six-key pad the pad of the dice chosen
    /// is, in dp: its rows past two, and the "0 is 10" line of ten-faced
    /// dice. A small panel's entry scrolls by it more, so the whole pad
    /// stays in view (§6.4.7). A lookup.
    pub fn pad_extra(&self) -> f32 {
        let n = usize::from(self.dice_choice.faces);
        if n == 6 || self.way != Way::Dice {
            return 0.0;
        }
        let rows = n.div_ceil(pad_columns(n));
        let label = if n == 10 {
            faraday_draw::tokens::PAD + faraday_draw::tokens::GAP_SMALL
        } else {
            0.0
        };
        rows.saturating_sub(2) as f32 * faraday_draw::tokens::CODEX32_KEY_ROW + label
    }

    /// Where the entropy came from, for Strings made: rolled, typed, or
    /// both.
    pub fn sources(&self) -> (bool, bool) {
        (self.rolled, self.typed)
    }

    /// Whether the set's faces came from the five-dice roller, in part
    /// or all (§5.5): what Strings made's entropy row says.
    pub fn simulated(&self) -> Sim {
        self.sim
    }

    /// The five-dice roller beside the pad (Faraday Desktop, §5.5).
    pub fn roller(&self) -> &FiveRoller {
        &self.roller
    }

    /// Whether die `die` (from 0) of the character in progress shows a
    /// value the roller made.
    pub fn die_rolled(&self, die: usize) -> bool {
        self.rolled_dice.get(die).copied().unwrap_or(false)
    }

    /// Whether data character `n` of initial share `share` has a face the
    /// roller made (a lookup).
    pub fn char_rolled(&self, share: usize, n: usize) -> bool {
        let data = self.length.data_len();
        self.roller.char_rolled(share * data + n)
    }

    /// Whether the roller has a press to take: the entry open on dice
    /// with a character left to enter.
    pub fn roller_can_roll(&self) -> bool {
        self.open == Some(Card::Enter) && self.way == Way::Dice && self.front().is_some()
    }

    /// The review's tab shown, and which have been shown.
    pub fn tab(&self) -> (u8, &[bool]) {
        (self.tab, &self.seen)
    }

    /// The worksheet beside the dice on the entry (see [`C32::View`]).
    pub fn view(&self) -> u8 {
        self.view
    }

    /// How far the checksum worksheet shown is panned.
    pub fn pan(&self) -> SheetPan {
        self.pan
    }

    /// How far the "Demo only" card is scrolled within itself.
    pub fn pane_offset(&self) -> f32 {
        self.pane_y
    }

    /// The "Demo only" card's own scroll, which the scroll machinery
    /// moves (`ui::Slot::DemoPane`).
    pub(crate) fn pane_offset_mut(&mut self) -> &mut f32 {
        &mut self.pane_y
    }

    /// On a narrow window, whether the "Demo only" card is open over the
    /// page rather than its strip alone.
    pub fn pane_open(&self) -> bool {
        self.pane_open
    }

    /// The worksheet's pan, which the scroll machinery moves
    /// (`Faraday::scroll_slot`, `ui::Slot::Worksheet`).
    pub(crate) fn pan_mut(&mut self) -> &mut SheetPan {
        &mut self.pan
    }

    /// Where the entry's work stands on its checksum worksheet as drawn
    /// on a window of class `cls`: the cells the step-through's step
    /// touches, or the front cell (left, top, wide, tall, from the sheet's
    /// corner), and the cell's side. Arithmetic on the cells' boxes.
    fn work_spot(
        &self,
        cls: faraday_draw::geom::WindowClass,
    ) -> Option<((f32, f32, f32, f32), f32)> {
        let (share, front) = self.entry_share()?;
        let sh = self
            .work
            .as_ref()
            .and_then(|w| w.worksheet(share, Reveal::PaperForm))?;
        let g = worksheet_screen::Geom::drawn(self.fit.cell, cls);
        let marks = self.walk_cells(share);
        let spot = worksheet_screen::paper!(sh, |p| {
            let mut spots = marks.iter().filter_map(|&(cell, _)| g.cell_spot(p, cell));
            match spots.next() {
                Some(first) => Some(spots.fold(first, worksheet_screen::union)),
                None => front.and_then(|b| g.box_spot(p, b)),
            }
        });
        spot.map(|s| (s, g.cell))
    }

    /// Sends the checksum worksheet's view, `view` units that pan as far
    /// as `max`, to where the entry's work is: the cells the step-through's
    /// step touches, or the front cell, moving the least
    /// (`docs/CODEX32.md` §6.4.7, §7.5). The place is worked out here,
    /// once, as the front moves; the frames after only interpolate
    /// ([`SheetPan::tick`]); under `reduce` the view is placed at once.
    /// Arithmetic on the cells' boxes.
    pub(crate) fn follow_front(
        &mut self,
        cls: faraday_draw::geom::WindowClass,
        view: (f32, f32),
        max: (f32, f32),
        reduce: bool,
    ) {
        if let Some((spot, cell)) = self.work_spot(cls) {
            let to = self.pan.target_for(spot, view, max, 2.0 * cell);
            self.pan.go_to(to, reduce);
        }
    }

    /// The worksheet drawn whole has no view to move: the page's scroll,
    /// now at `offset` and going as far as `max`, makes the least move
    /// that brings the work into the page's room `room` units tall, the
    /// sheet's top `top` units down the page's content
    /// (`docs/LAYOUT.md` §16.4), in the same timed move as the view's;
    /// under `reduce` at once. Returns where the page is to stand now.
    /// Arithmetic on the cells' boxes.
    pub(crate) fn follow_page(
        &mut self,
        cls: faraday_draw::geom::WindowClass,
        (top, room, max): (f32, f32, f32),
        (offset, reduce): (f32, bool),
    ) -> Option<f32> {
        let ((sx, sy, sw, sh), cell) = self.work_spot(cls)?;
        if !self.page.moving() {
            self.page = SheetPan::default();
            self.page.y = offset;
        }
        let spot = (sx, top + sy, sw, sh);
        let to = self
            .page
            .target_for(spot, (f32::MAX, room), (0.0, max), 2.0 * cell);
        self.page.go_to((0.0, to.1), reduce);
        Some(self.page.y)
    }

    /// One frame of the page following a whole worksheet, at `now_ms`,
    /// `f` pixels to the unit: where the page stands, while it moves.
    /// Arithmetic.
    pub(crate) fn page_tick(&mut self, now_ms: u64, f: f32) -> Option<f32> {
        self.page.tick(now_ms, f).then_some(self.page.y)
    }

    /// Whether the page is moving to follow a whole worksheet.
    pub(crate) fn page_moving(&self) -> bool {
        self.page.moving()
    }

    /// The person moved the page: its following move stops.
    pub(crate) fn page_stop(&mut self) {
        self.page.stop();
    }

    /// Keeps the checksum worksheet's fitted cell in step with `room`,
    /// the work width and the view's height it is drawn in (none on
    /// one-column pages), and with the sheet's length
    /// (`docs/LAYOUT.md` §16.1): worked out again only when either has
    /// changed. Arithmetic.
    pub(crate) fn refit(&mut self, room: Option<(f32, f32)>) {
        let sheet = self
            .work
            .as_ref()
            .and_then(|w| w.worksheet(0, Reveal::PaperForm));
        let chars = sheet.as_ref().map_or(0, |sh| sh.chars());
        if self.fit.room == room && self.fit.chars == chars {
            return;
        }
        let cell = match (sheet, room) {
            (Some(sh), Some(room)) => {
                worksheet_screen::paper!(sh, |p| worksheet_screen::fit_cell(p, room))
            }
            _ => None,
        };
        self.fit = SheetFit { room, chars, cell };
    }

    /// The checksum worksheet's fitted cell, where it is drawn whole
    /// (`docs/LAYOUT.md` §16.1); none where it pans in its view.
    pub(crate) fn sheet_cell(&self) -> Option<f32> {
        self.fit.cell
    }

    /// One frame of the worksheet's view following the work, at `now_ms`,
    /// `f` pixels to the unit. Returns whether it moved. Arithmetic.
    pub(crate) fn pan_tick(&mut self, now_ms: u64, f: f32) -> bool {
        self.pan.tick(now_ms, f)
    }

    /// The small panel's context (see [`C32::Panel`]).
    pub fn panel(&self) -> u8 {
        self.panel
    }

    /// The cell whose working is open.
    pub fn cell(&self) -> Option<Cell> {
        self.cell
    }

    /// The strings to write, once the set is made.
    pub fn strings(&self) -> Option<&[Zeroizing<String>]> {
        self.strings.as_ref().map(|s| s.as_slice())
    }

    /// The fingerprint of the key the set makes, once made.
    pub fn fingerprint(&self) -> Option<[u8; 4]> {
        self.fingerprint
    }

    /// The string on the Strings card, by its place.
    pub fn at(&self) -> usize {
        self.at
    }

    /// The string shown's Codex32QR code, while it is shown in place of
    /// the string.
    pub fn qr(&self) -> Option<&QrMatrix> {
        self.qr.as_ref().filter(|_| self.qr_on)
    }

    /// What is typed back of the string shown, and the first box that
    /// differs.
    pub fn typed_back(&self) -> (&str, Option<usize>) {
        (self.back.typed.as_str(), self.back.differs)
    }

    /// Each string checked or skipped, and whether a skip was asked.
    pub fn checked(&self) -> (&[bool], bool) {
        (&self.back.checked, self.back.skip_ask)
    }

    /// Check on another device is open.
    pub fn check_open(&self) -> bool {
        self.check_open
    }

    /// The last thing worth saying.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    /// The key is loaded: nothing changes after this.
    pub fn locked(&self) -> bool {
        self.locked
    }

    /// The review's tabs, in the booklet's order (§6.5): (name, what).
    pub fn tabs(&self) -> Vec<(String, Tab)> {
        let Some(w) = self.work.as_ref() else {
            return Vec::new();
        };
        let initial = w.inputs();
        let mut out = Vec::new();
        let name = |s: u8| {
            if w.scheme().threshold() == 0 || s == b's' {
                EN.codex32_the_secret.to_string()
            } else {
                char::from(s.to_ascii_uppercase()).to_string()
            }
        };
        // The dice are the person's; with Demo's sample in place they made
        // no string shown (§12).
        if self.rolled && !self.sample {
            for (s, &i) in initial.iter().enumerate() {
                out.push((fill(EN.codex32_tab_dice, &[&name(i)]), Tab::Dice(s)));
            }
        }
        // Each input's checksum worksheet: of a set typed in, the strings
        // typed, in the verification form (§8.1 step 3).
        for (s, &i) in initial.iter().enumerate() {
            out.push((fill(EN.codex32_tab_checksum, &[&name(i)]), Tab::Checksum(s)));
        }
        for (n, &t) in w.targets().iter().enumerate() {
            let label = if t == b's' {
                EN.codex32_the_secret.to_string()
            } else {
                fill(
                    EN.codex32_tab_share,
                    &[&char::from(t.to_ascii_uppercase()).to_string()],
                )
            };
            out.push((label, Tab::Translation(n)));
        }
        out
    }

    // ----- Presses -----

    /// Costs what the press does: a worksheet column (see `roll`), the work
    /// set's headers, or a QR encoding. One press. `fresh` is this
    /// device's randomness where the press asks for it (Suggest), drawn
    /// before the flow is borrowed.
    pub(crate) fn act(&mut self, c: &Compute, press: C32, fresh: Option<[u8; 32]>) {
        let before = self.entry_pos();
        self.act_press(c, press, fresh);
        // The entry changed by another way than the step-through's own:
        // its steps are forgotten.
        self.walk_check(before);
    }

    /// Costs what [`Self::act`] costs. The press itself.
    fn act_press(&mut self, c: &Compute, press: C32, fresh: Option<[u8; 32]>) {
        if self.locked && !matches!(press, C32::Card(_) | C32::Tab(_) | C32::TabStep(_)) {
            if matches!(
                press,
                C32::Check | C32::Cell(_) | C32::CellClose | C32::CellStep(_) | C32::Eye
            ) {
                self.view_press(press);
            }
            return;
        }
        match press {
            C32::Card(i) => self.toggle(i),
            C32::Next => self.next(c),
            C32::Length(i) if self.scheme_open() => {
                self.length = if i == 0 {
                    Length::Chars48
                } else {
                    Length::Chars74
                };
            }
            C32::Split(i) if self.scheme_open() => {
                if let Some(&s) = SPLITS.get(usize::from(i)) {
                    self.split = s;
                }
            }
            C32::K(d) if self.scheme_open() => {
                self.other_k = step(self.other_k, d, 2, faraday_bip::codex32::MAX_THRESHOLD);
                self.other_n = self.other_n.max(self.other_k);
            }
            // Make the rest asks how many strings in all: one more than
            // those typed, at least (§8.1 step 2).
            C32::N(d) if self.work.is_none() && self.given == Some(Given::Rest) => {
                self.other_n = step(
                    self.other_n,
                    d,
                    self.other_k + 1,
                    faraday_bip::codex32::MAX_SHARES,
                );
            }
            C32::N(d) if self.scheme_open() => {
                self.other_n = step(
                    self.other_n,
                    d,
                    self.other_k,
                    faraday_bip::codex32::MAX_SHARES,
                );
            }
            C32::Key(k) => self.key(c, k),
            C32::Suggest if self.work.is_none() => {
                if let Some(r) = fresh {
                    for (slot, b) in self.id.iter_mut().zip(r) {
                        *slot = b & 31;
                    }
                    self.id_len = 4;
                    self.note = None;
                }
            }
            C32::Way(i) if self.work.is_none() => {
                if let Some(&w) = WAYS.get(usize::from(i)) {
                    self.way = w;
                }
            }
            C32::Faces(i) => {
                if let Some(&n) = DIE_SIZES.get(usize::from(i)) {
                    self.change_faces(n);
                }
            }
            C32::Colour(d) => {
                if let Some(col) = self.dice_choice.colours.get_mut(usize::from(d)) {
                    *col = col.next();
                }
            }
            C32::Face(f) if self.open == Some(Card::Enter) && self.way == Way::Dice => {
                self.pending = None;
                if (1..=self.dice_choice.faces).contains(&f) {
                    self.roll(c, f);
                }
            }
            C32::Qr => self.qr_on = !self.qr_on && self.qr.is_some(),
            C32::Skip => self.skip(c),
            C32::Sample if self.open == Some(Card::Enter) => self.use_sample(c),
            _ => self.view_press(press),
        }
    }

    /// Whether the Scheme card's length and split take a press: before
    /// the entry, not fixed by a wallet's Quorum, not a set typed in.
    fn scheme_open(&self) -> bool {
        self.work.is_none() && !self.fixed && self.given.is_none()
    }

    /// The presses that only change what is shown.
    fn view_press(&mut self, press: C32) {
        match press {
            C32::Tab(i) => self.show_tab(usize::from(i)),
            C32::TabStep(d) => {
                let n = self.seen.len() as i64;
                let to = (i64::from(self.tab) + i64::from(d)).clamp(0, (n - 1).max(0));
                self.show_tab(to as usize);
            }
            C32::View(v) => self.view = v,
            C32::Panel(p) => self.panel = p.min(2),
            C32::Cell(cell) if !self.masked() => self.cell = Some(cell),
            C32::CellClose => self.cell = None,
            C32::CellStep(d) => {
                if let Some(n) = self.cell.and_then(|c| self.neighbour(c, d)) {
                    self.cell = Some(n);
                }
            }
            C32::Eye if self.eye_offered() => {
                self.hidden = !self.hidden;
                // A working shows values: it closes when they are masked.
                if self.hidden {
                    self.cell = None;
                }
            }
            C32::Check => self.check_open = !self.check_open,
            C32::Pane => {
                self.pane_open = !self.pane_open;
                self.pane_y = 0.0;
            }
            _ => {}
        }
    }

    fn show_tab(&mut self, i: usize) {
        if let Some(s) = self.seen.get_mut(i) {
            *s = true;
            self.tab = i as u8;
            self.pan = SheetPan::default();
        }
    }

    /// A card's header: the open one closes; a later one opens once the
    /// ones before it are done.
    fn toggle(&mut self, i: u8) {
        let Some(&card) = CARDS.get(usize::from(i)) else {
            return;
        };
        if self.open == Some(card) {
            // Closing the entry puts it aside.
            if card == Card::Enter {
                self.mask();
            }
            self.open = None;
            return;
        }
        let reachable = CARDS
            .iter()
            .take_while(|&&c| c != card)
            .all(|&c| self.done(c));
        // Once the entry has begun, the cards before it only show.
        if reachable {
            // Back past the first entry card: the sheets are masked
            // again (§6.4.6).
            if self.work.is_some() && card.index() < Card::Enter.index() {
                self.mask();
            }
            self.open = Some(card);
            self.cell = None;
        }
    }

    /// The flow is left or put aside: the sheets are masked (§6.4.6).
    pub(crate) fn mask(&mut self) {
        self.hidden = true;
        self.cell = None;
    }

    /// The cell `d` places along the row of `cell`, if there is one.
    fn neighbour(&self, cell: Cell, d: i8) -> Option<Cell> {
        let work = self.work.as_ref()?;
        let step = |v: u8, lo: usize, len: usize| -> Option<u8> {
            let n = i32::from(v) + i32::from(d);
            (n >= lo as i32 && n < (lo + len) as i32).then_some(n as u8)
        };
        let chars = work.scheme().length().chars();
        let cols = chars - 3;
        Some(match cell {
            Cell::Given { sheet, b } => Cell::Given {
                sheet,
                b: step(b, 4, cols)?,
            },
            Cell::Translated { sheet, i, b } => Cell::Translated {
                sheet,
                i,
                b: step(b, 4, cols)?,
            },
            Cell::TSum { sheet, b } => Cell::TSum {
                sheet,
                b: step(b, 4, cols)?,
            },
            Cell::Hrp { sheet, i } => Cell::Hrp {
                sheet,
                i: step(
                    i,
                    0,
                    work.worksheet(usize::from(sheet), Reveal::PaperForm)?
                        .hrp_row()
                        .len(),
                )?,
            },
            Cell::Sum { sheet, j, i } => Cell::Sum {
                sheet,
                j,
                i: step(
                    i,
                    0,
                    work.worksheet(usize::from(sheet), Reveal::PaperForm)?
                        .checksum_len(),
                )?,
            },
            Cell::Lookup { sheet, j, i } => Cell::Lookup {
                sheet,
                j,
                i: step(
                    i,
                    0,
                    work.worksheet(usize::from(sheet), Reveal::PaperForm)?
                        .checksum_len(),
                )?,
            },
            Cell::Symbol { .. } => return None,
        })
    }

    fn finish(&mut self, card: Card, next: Card) {
        self.done[usize::from(card.index())] = true;
        self.open = Some(next);
        self.note = None;
        self.cell = None;
    }

    /// Costs the work set's headers (Entropy's Continue) or a QR encoding
    /// (a string shown). The open card's Continue.
    fn next(&mut self, c: &Compute) {
        match self.open {
            Some(Card::Scheme) if self.given.is_some() => self.make_given(c),
            Some(Card::Scheme) => self.finish(Card::Scheme, Card::Identifier),
            Some(Card::Identifier) => {
                if self.id_len < 4 {
                    self.note = Some(EN.codex32_four_characters.to_string());
                } else if self.id_refused().is_none() {
                    self.finish(Card::Identifier, Card::Entropy);
                }
            }
            // Shares made by hand are typed whole on Add a key's form
            // (§5.3): New key hands over to it (`Codex32Make::by_hand`).
            Some(Card::Entropy) if self.way == Way::Shares && self.work.is_none() => {
                self.hand = true;
            }
            Some(Card::Entropy) => {
                if self.work.is_none() && !self.begin(c) {
                    return;
                }
                self.finish(Card::Entropy, Card::Enter);
            }
            Some(Card::Enter) if self.strings.is_some() => {
                // In Demo the strings made never leave the entry: the
                // sample stands in their place (§12).
                if !self.sample && self.demo.is_some() && !self.put_sample(c) {
                    return;
                }
                self.seen = vec![false; self.tabs().len()];
                self.show_tab(0);
                self.finish(Card::Enter, Card::Sheets);
            }
            Some(Card::Sheets) => {
                self.at = 0;
                self.back = Back {
                    checked: vec![false; self.strings().map_or(0, <[_]>::len)],
                    ..Back::default()
                };
                self.show_string(c);
                self.finish(Card::Sheets, Card::Strings);
            }
            Some(Card::Strings) => self.string_next(c),
            _ => {}
        }
    }

    /// Costs what `WorkSet::given` costs: a worksheet of each string typed
    /// in and an interpolation for each string made. The first card's
    /// Continue for a set typed in (§8.1, §8.2): the strings made, the
    /// worksheets read back next.
    fn make_given(&mut self, c: &Compute) {
        let making = match self.given {
            Some(Given::Add) => {
                if self.add_at_taken() {
                    self.note = Some(EN.c32r_index_taken.to_string());
                    return;
                }
                Making::Add(self.add_at)
            }
            _ => Making::Rest(self.other_n),
        };
        match WorkSet::given(c, &self.inputs, making) {
            Ok(w) => {
                self.work = Some(w);
                self.inputs.clear();
                self.made.clear();
                self.complete(c);
                self.seen = vec![false; self.tabs().len()];
                self.show_tab(0);
                self.finish(Card::Scheme, Card::Sheets);
            }
            Err(e) => self.note = Some(fill(EN.codex32_refused, &[&e.to_string()])),
        }
    }

    /// Costs what `WorkSet::given` costs for the sample. Demo's sample in
    /// place of the set entered (§12): the sample's own scheme, its
    /// strings and every worksheet of it; the characters entered and how
    /// each was made are dropped. False, with the note, where Demo has no
    /// sample.
    fn put_sample(&mut self, c: &Compute) -> bool {
        let Some(set) = self.demo.as_ref() else {
            return true;
        };
        let n = set.first().map_or(0, |s| s.threshold()) + 1;
        match WorkSet::given(c, set, Making::All(n)) {
            Ok(w) => {
                self.length = w.scheme().length();
                self.id = w
                    .scheme()
                    .identifier()
                    .map(|ch| symbol_of(char::from(ch)).unwrap_or(0));
                self.id_len = 4;
                self.split = Split::Other;
                self.other_k = w.scheme().threshold();
                self.other_n = w.scheme().shares();
                self.work = Some(w);
                self.made.clear();
                self.seq.clear();
                self.replay();
                self.sample = true;
                self.complete(c);
                true
            }
            Err(_) => {
                self.note = Some(EN.c32r_demo_no_sample.to_string());
                false
            }
        }
    }

    /// Costs what [`Self::put`] costs for each character. **Use a sample**
    /// in Demo (§12): the characters left of the entry, as typed, from the
    /// sample's own shares (BIP 93's vector 2), share by share.
    fn use_sample(&mut self, c: &Compute) {
        let Some(set) = self.demo.as_ref() else {
            return;
        };
        let data: Vec<Zeroizing<Vec<u8>>> = set
            .iter()
            .map(|s| Zeroizing::new(s.payload().to_vec()))
            .collect();
        if data.is_empty() || data.iter().any(|d| d.is_empty()) {
            return;
        }
        self.seq.clear();
        self.replay();
        while let Some((share, n)) = self.front() {
            let from = &data[share % data.len()];
            let v = from[n % from.len()];
            let before = self.entered_all();
            self.put(c, v, None);
            if self.entered_all() == before {
                break;
            }
        }
    }

    /// Costs the header's columns of every worksheet. The work set for
    /// the scheme and identifier chosen.
    fn begin(&mut self, c: &Compute) -> bool {
        let (k, n) = self.k_n();
        let id = self.id.map(|v| CHARSET[usize::from(v)]);
        match Scheme::new(k, n, id, self.length).and_then(|s| WorkSet::new(c, s)) {
            Ok(w) => {
                self.made = w
                    .scheme()
                    .initial_indices()
                    .iter()
                    .map(|_| Vec::new())
                    .collect();
                self.work = Some(w);
                // A new entry: nothing on it is the roller's.
                self.roller.reset();
                true
            }
            Err(e) => {
                self.note = Some(fill(EN.codex32_refused, &[&e.to_string()]));
                false
            }
        }
    }

    /// Costs what [`Self::roll`] or [`Self::next`] costs. A key of the
    /// keyboard or the pad, to the open card's field.
    fn key(&mut self, c: &Compute, k: OskPress) {
        match (self.open, k) {
            // Add a share's index, typed over the one offered (§8.2).
            (Some(Card::Scheme), OskPress::Char(ch))
                if self.given == Some(Given::Add) && self.work.is_none() =>
            {
                if let Some(v) = symbol_of(ch) {
                    self.add_at = CHARSET[usize::from(v)];
                    self.note = None;
                }
            }
            (Some(Card::Identifier), OskPress::Char(ch)) if self.work.is_none() => {
                if let Some(v) = symbol_of(ch)
                    && self.id_len < 4
                {
                    self.id[usize::from(self.id_len)] = v;
                    self.id_len += 1;
                    self.note = None;
                }
            }
            (Some(Card::Identifier), OskPress::Back) if self.work.is_none() => {
                self.id_len = self.id_len.saturating_sub(1);
                self.note = None;
            }
            (Some(Card::Enter), OskPress::Char(ch)) => match self.way {
                Way::Dice => {
                    if let Some(d) = ch.to_digit(10) {
                        self.digit(c, d as u8);
                    } else if ch == ' ' {
                        self.take_pending(c);
                    }
                }
                Way::Typed => {
                    if let Some(v) = symbol_of(ch) {
                        self.put(c, v, None);
                    }
                }
                Way::Shares => {}
            },
            (Some(Card::Enter), OskPress::Back) if self.pending.is_some() => self.pending = None,
            (Some(Card::Enter), OskPress::Done) if self.pending.is_some() => self.take_pending(c),
            (Some(Card::Enter), OskPress::Back) => self.undo(),
            (Some(Card::Strings), OskPress::Char(ch)) => self.type_back(ch),
            (Some(Card::Strings), OskPress::Back) => {
                self.back.typed.pop();
                self.back.differs = None;
            }
            (_, OskPress::Done) => self.next(c),
            _ => {}
        }
    }

    // ----- The entry -----

    /// The dice's faces become `n`, the roller's too; an earlier
    /// character keeps the faces it was rolled with. Cheap.
    fn set_faces(&mut self, n: u8) {
        if faraday_entropy::booklet::faces_ok(n) {
            self.dice_choice.faces = n;
            self.roller.set_faces(n);
            self.replay();
        }
    }

    /// Other dice chosen: between characters they are simply taken; in
    /// the middle of one, its rolls so far are cleared (they were of the
    /// other dice), the roller stops, and the note says so. Cheap.
    fn change_faces(&mut self, n: u8) {
        if n == self.dice_choice.faces {
            return;
        }
        if !self.dice_open() {
            self.seq.clear();
            self.pending = None;
            self.roller.stop();
            let done = self.entered_all();
            self.roller.trim(done, 0);
            self.note = Some(EN.codex32_dice_changed.to_string());
        }
        self.set_faces(n);
    }

    /// Costs what [`Self::roll`] costs. A digit typed on the keyboard:
    /// a face at once where it cannot begin a face of two digits on
    /// these dice; else it waits for the units digit (10 to 20), Space
    /// or Enter. A tens digit followed by a digit that would pass the
    /// dice's faces is a face alone, and the new digit starts over. On
    /// ten-faced dice a 0 alone is 10, as a die numbered 0 to 9 reads.
    fn digit(&mut self, c: &Compute, d: u8) {
        let n = self.dice_choice.faces;
        if let Some(tens) = self.pending.take() {
            let v = tens * 10 + d;
            if v <= n {
                self.roll(c, v);
                return;
            }
            self.roll(c, tens);
        }
        if d >= 1 && d * 10 <= n {
            self.pending = Some(d);
        } else if (1..=n).contains(&d) {
            self.roll(c, d);
        } else if d == 0 && n == 10 {
            // A ten-faced die numbered 0 to 9: its 0 is 10.
            self.roll(c, 10);
        }
    }

    /// Costs what [`Self::roll`] costs. The tens digit waiting, taken as
    /// a face of one digit.
    fn take_pending(&mut self, c: &Compute) {
        if let Some(d) = self.pending.take() {
            self.roll(c, d);
        }
    }

    /// Characters entered so far, of every initial share (a lookup over
    /// at most 31 shares).
    fn entered_all(&self) -> usize {
        self.work
            .as_ref()
            .map_or(0, |w| (0..w.inputs().len()).map(|s| w.entered(s)).sum())
    }

    /// Costs a worksheet column of each sheet the character is in, and
    /// at a share's last character its checksum, and at the set's last
    /// the set, its key's fingerprint and the first string's Codex32QR.
    /// One die's face, in the booklet's order.
    fn roll(&mut self, c: &Compute, face: u8) {
        if self.front().is_none() {
            return;
        }
        // A character the step-through holds whole goes into its box
        // before a face of the next.
        self.put_whole(c);
        self.push_face(face);
        self.put_whole(c);
    }

    /// A face onto the character in progress, the sheet worked again;
    /// a character it makes whole stays out of its box. Cheap.
    fn push_face(&mut self, face: u8) {
        self.seq.push(face);
        self.replay();
    }

    /// Costs what [`Self::put`] costs. The character in progress, once
    /// whole, into its box; nothing while it is not.
    fn put_whole(&mut self, c: &Compute) {
        if self.seq.is_empty() {
            return;
        }
        if let Ok(ch) = self.dice.character() {
            let value = symbol_of(char::from(ch)).unwrap_or(0);
            let seq = std::mem::replace(&mut self.seq, Zeroizing::new(Vec::new()));
            let rows = std::mem::replace(&mut self.rows, Zeroizing::new(Vec::new()));
            self.replay();
            self.put(c, value, Some((seq, rows)));
        }
    }

    /// Works `seq` out on the booklet's sheet again: the rolls, the pairs
    /// completed, an equal pair struck and its die rolled again. Cheap: a
    /// few dozen faces at most.
    fn replay(&mut self) {
        self.dice = BookletChar::with_faces(self.dice_choice.faces).unwrap_or_default();
        self.rows.clear();
        self.equal = None;
        self.rolled_dice = [false; BOOKLET_DICE];
        let done = self.entered_all();
        for (i, &face) in self.seq.iter().enumerate() {
            let place = (0..2 * BOOKLET_DICE).find(|&p| {
                if p < BOOKLET_DICE {
                    self.dice.first(p).is_none()
                } else {
                    self.dice.second(p - BOOKLET_DICE).is_none()
                }
            });
            if self.dice.push(face).is_err() {
                continue;
            }
            self.equal = None;
            if let Some(p) = place {
                let die = p % BOOKLET_DICE;
                let rolled = self.roller.is_rolled(done, i);
                // A die's first value starts its mark over; its second
                // adds to it.
                self.rolled_dice[die] = rolled || (p >= BOOKLET_DICE && self.rolled_dice[die]);
            }
            if let Some(p) = place.filter(|&p| p >= BOOKLET_DICE) {
                let die = p - BOOKLET_DICE;
                let row = DieRow {
                    die: die as u8,
                    first: self.dice.first(die).unwrap_or(0),
                    second: face,
                };
                self.rows.push(row);
                if row.bit().is_none() {
                    self.dice.redo(die);
                    self.equal = Some(die as u8);
                    self.rolled_dice[die] = false;
                }
            }
        }
    }

    /// Costs what [`Self::roll`] costs. A character, rolled (`seq`) or
    /// typed, into the share being entered.
    fn put(&mut self, c: &Compute, value: u8, rolled: Option<Rolled>) {
        let Some((share, _)) = self.front() else {
            return;
        };
        let Some(work) = self.work.as_mut() else {
            return;
        };
        if let Err(e) = work.push(c, share, value) {
            self.note = Some(fill(EN.codex32_refused, &[&e.to_string()]));
            return;
        }
        let typed = rolled.is_none();
        let (seq, rows) = rolled.unwrap_or_default();
        self.rolled |= !typed;
        self.typed |= typed;
        let faces = if typed {
            booklet::BOOKLET_FACES
        } else {
            self.dice_choice.faces
        };
        if let Some(list) = self.made.get_mut(share) {
            list.push(Made {
                seq,
                rows,
                value,
                typed,
                faces,
            });
        }
        self.note = None;
        if work.is_complete() {
            self.complete(c);
        }
    }

    /// Costs an HMAC-SHA512 and an elliptic-curve multiplication (the
    /// key's fingerprint) and a QR encoding. Takes the finished set's
    /// strings, as a paper form, and its key's fingerprint.
    fn complete(&mut self, c: &Compute) {
        let Some(work) = self.work.as_ref() else {
            return;
        };
        self.strings = work.strings(Reveal::PaperForm);
        self.fingerprint = work.fingerprint(c).map(|f| f.0);
        // Every face entered, a character typed counting one.
        let faces: usize = self
            .made
            .iter()
            .flatten()
            .map(|m| if m.typed { 1 } else { m.seq.len() })
            .sum();
        self.sim = self.roller.sim(faces);
        self.at = 0;
        self.encode_qr(c);
    }

    /// Takes back the last face of the character in progress, or, with
    /// none, the last character, its rolls back on the sheet less the
    /// last. Cheap but for the work set's own pop.
    fn undo(&mut self) {
        // A set typed in, or Demo's sample, has no entry to take back.
        if self.work.as_ref().is_some_and(WorkSet::is_given) {
            return;
        }
        self.undo_entry();
        // Faces taken back are not the roller's any more.
        let done = self.entered_all();
        self.roller.trim(done, self.seq.len());
        self.replay();
    }

    /// [`Self::undo`] on the entry alone.
    fn undo_entry(&mut self) {
        if !self.seq.is_empty() {
            self.seq.pop();
            self.replay();
            return;
        }
        let share = match self.front() {
            Some((s, 0)) if s > 0 => s - 1,
            Some((s, _)) => s,
            None => self.made.len().saturating_sub(1),
        };
        let Some(work) = self.work.as_mut() else {
            return;
        };
        if work.entered(share) == 0 {
            return;
        }
        work.pop(share);
        self.strings = None;
        self.fingerprint = None;
        self.qr = None;
        self.qr_on = false;
        if let Some(m) = self.made.get_mut(share).and_then(Vec::pop)
            && !m.typed
        {
            // Its rolls come back on the dice they were rolled with.
            self.dice_choice.faces = m.faces;
            self.roller.set_faces(m.faces);
            self.seq = Zeroizing::new(m.seq.to_vec());
            self.seq.pop();
            self.replay();
        }
    }

    // ----- The strings -----

    /// Costs a QR encoding. The string shown's Codex32QR code, kept: at
    /// 48 characters only (§10.2), upper case, byte mode, version 3,
    /// level L.
    fn encode_qr(&mut self, c: &Compute) {
        self.qr = None;
        let Some(s) = self.strings().and_then(|s| s.get(self.at)) else {
            return;
        };
        self.qr = faraday_codec::qr::codex32qr(c, s);
    }

    /// Costs a QR encoding. Shows string `at`, nothing typed back.
    fn show_string(&mut self, c: &Compute) {
        self.back.typed.clear();
        self.back.differs = None;
        self.back.skip_ask = false;
        self.qr_on = false;
        self.encode_qr(c);
    }

    /// A character typed back, from box 4; the whole string typed
    /// compared box by box.
    fn type_back(&mut self, ch: char) {
        let Some(s) = self.strings().and_then(|s| s.get(self.at)) else {
            return;
        };
        let room = s.len() - 3;
        if symbol_of(ch).is_none() || self.back.typed.len() >= room {
            return;
        }
        let want: Zeroizing<String> = Zeroizing::new(s[3..].to_string());
        self.back.typed.push(ch.to_ascii_lowercase());
        self.back.skip_ask = false;
        if self.back.typed.len() == room {
            self.back.differs = want
                .bytes()
                .zip(self.back.typed.as_str().bytes())
                .position(|(a, b)| a != b)
                .map(|p| p + 4);
            if self.back.differs.is_none()
                && let Some(done) = self.back.checked.get_mut(self.at)
            {
                *done = true;
            }
        }
    }

    /// Costs a QR encoding. The Strings card's Continue: the next string
    /// once this one is typed back the same (or skipped), and Strings
    /// made after the last.
    fn string_next(&mut self, c: &Compute) {
        if !self.back.checked.get(self.at).copied().unwrap_or(false) {
            self.note = Some(EN.codex32_type_it_back.to_string());
            return;
        }
        let n = self.strings().map_or(0, <[_]>::len);
        if self.at + 1 < n {
            self.at += 1;
            self.note = None;
            self.show_string(c);
        } else {
            self.finish(Card::Strings, Card::Made);
        }
    }

    /// Costs what [`Self::string_next`] costs. Skip: a first press asks,
    /// a second goes on without the typing back.
    fn skip(&mut self, c: &Compute) {
        if self.open != Some(Card::Strings) {
            return;
        }
        if self.back.skip_ask {
            if let Some(done) = self.back.checked.get_mut(self.at) {
                *done = true;
            }
            self.string_next(c);
        } else {
            self.back.skip_ask = true;
        }
    }

    /// The key is loaded (Lock in): nothing changes after this.
    pub(crate) fn set_locked(&mut self) {
        self.locked = true;
        self.done[usize::from(Card::Made.index())] = true;
        self.note = None;
    }

    /// Lock in was refused, with why.
    pub(crate) fn refused(&mut self, why: String) {
        self.note = Some(why);
    }

    /// Whether Lock in may load the key: the strings were shown and
    /// Strings made is open.
    pub(crate) fn ready_to_lock(&self) -> bool {
        !self.locked
            && self.open == Some(Card::Made)
            && self.work.as_ref().is_some_and(WorkSet::is_complete)
    }

    /// The Check on another device record's sets to try (§6.8): subsets
    /// of `k` share indices in which every string is at least once and
    /// the initial shares are not in every subset; `ceil(n / k)` of
    /// them. Cheap.
    pub fn sets_to_try(&self) -> Vec<String> {
        let (k, n) = self.k_n();
        if k == 0 {
            return Vec::new();
        }
        let order = &faraday_bip::codex32::INDEX_ORDER[..usize::from(n)];
        let (k, n) = (usize::from(k), usize::from(n));
        (0..n.div_ceil(k))
            .map(|t| {
                (0..k)
                    .map(|i| char::from(order[(t * k + i) % n].to_ascii_uppercase()).to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .collect()
    }
}

/// The identifiers the codex32 strings typed into Add a key's form
/// carry, each with where (`docs/CODEX32.md` §6.4.5).
pub(crate) fn taken(form: &[faraday_bip::codex32::Codex32]) -> Vec<([u8; 4], String)> {
    form.iter()
        .map(|s| (s.identifier(), EN.codex32_used_in_add_a_key.to_string()))
        .collect()
}

/// `v` moved by `d` within `lo..=hi`.
fn step(v: u8, d: i8, lo: u8, hi: u8) -> u8 {
    (i16::from(v) + i16::from(d)).clamp(i16::from(lo), i16::from(hi)) as u8
}

/// What a review tab shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    /// The dice rows of initial share `s`'s characters.
    Dice(usize),
    /// Initial share `s`'s checksum worksheet.
    Checksum(usize),
    /// The `n`-th translation worksheet ([`WorkSet::targets`]).
    Translation(usize),
}
