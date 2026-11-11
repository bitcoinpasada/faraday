//! The Argon QR Vault's format: seal and open (`docs/AQRV.md`).
//!
//! One small secret under a passphrase, in 53 or 62 bytes (§3). [`seal`]
//! writes an envelope (§6 Writing), [`open`] reads one (§6 Reading),
//! [`recognise`] says from the bytes alone whether they are one (§9), and
//! [`workings`] gives every intermediate value of §6 for the test vectors.
//!
//! The library prints nothing, takes its random bytes from the caller, and
//! keeps no secret in a type that can be printed: an item, a key and the
//! passphrase's bytes are wiped when dropped.
//!
//! This is Faraday's own construction, not reviewed outside it, and it is
//! not part of any Faraday app (F245).

mod codex32;
mod envelope;
mod item;

pub use envelope::{Opened, Workings, open, seal, workings};
pub use item::Item;

/// The two sizes (§3), told apart by the envelope's total length.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    /// 53 bytes: a 29 x 29 code, a 4-byte tag, a room of 32.
    Small,
    /// 62 bytes: a 33 x 33 code, an 8-byte tag, a room of 37.
    Large,
}

impl Size {
    /// The envelope's total length, `N` in §6.
    pub const fn bytes(self) -> usize {
        match self {
            Size::Small => 53,
            Size::Large => 62,
        }
    }

    /// The room: the length of `C` and of `P` (§3).
    pub const fn room(self) -> usize {
        match self {
            Size::Small => 32,
            Size::Large => 37,
        }
    }

    /// The tag's length, `t` in §6.
    pub const fn tag_len(self) -> usize {
        match self {
            Size::Small => 4,
            Size::Large => 8,
        }
    }

    /// The size whose total length is `n`, if either (§9, first rule).
    pub fn of_len(n: usize) -> Option<Size> {
        match n {
            53 => Some(Size::Small),
            62 => Some(Size::Large),
            _ => None,
        }
    }
}

/// The memory steps (§5). Passes and lanes are the same for every step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Step 0, 64 MiB.
    Mib64,
    /// Step 1, 128 MiB.
    Mib128,
    /// Step 2, 256 MiB.
    Mib256,
    /// Step 3, 512 MiB.
    Mib512,
    /// Step 4, 1 GiB.
    Gib1,
}

impl Step {
    /// Every defined step, smallest first.
    pub const ALL: [Step; 5] = [
        Step::Mib64,
        Step::Mib128,
        Step::Mib256,
        Step::Mib512,
        Step::Gib1,
    ];

    /// The step §5 suggests a writer starts from.
    pub const DEFAULT: Step = Step::Mib256;

    /// Argon2id's memory `m` in KiB (§5).
    pub const fn memory_kib(self) -> u32 {
        match self {
            Step::Mib64 => 65_536,
            Step::Mib128 => 131_072,
            Step::Mib256 => 262_144,
            Step::Mib512 => 524_288,
            Step::Gib1 => 1_048_576,
        }
    }

    /// The header byte, 0xF8 to 0xFC: the marker and the step (§4).
    pub const fn header(self) -> u8 {
        match self {
            Step::Mib64 => 0xF8,
            Step::Mib128 => 0xF9,
            Step::Mib256 => 0xFA,
            Step::Mib512 => 0xFB,
            Step::Gib1 => 0xFC,
        }
    }

    /// The step a header byte names. None for any other marker and for
    /// the reserved steps 5 to 7 (§4, §5).
    pub fn of_header(h: u8) -> Option<Step> {
        Step::ALL.into_iter().find(|s| s.header() == h)
    }
}

/// The nine kinds (§3), as the byte that enters the tag (§6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    /// BIP-39 entropy, 12 words, 16 bytes.
    Words12 = 0x10,
    /// BIP-39 entropy, 15 words, 20 bytes.
    Words15 = 0x14,
    /// BIP-39 entropy, 18 words, 24 bytes.
    Words18 = 0x18,
    /// BIP-39 entropy, 21 words, 28 bytes.
    Words21 = 0x1C,
    /// BIP-39 entropy, 24 words, 32 bytes.
    Words24 = 0x20,
    /// A codex32 string (§3.1).
    Codex32 = 0x30,
    /// A BIP-39 passphrase, as text.
    Passphrase = 0x40,
    /// A note, as text.
    Note = 0x41,
    /// A raw 32-byte key.
    RawKey = 0x50,
}

impl Kind {
    /// Every kind. A reader tests them all (§6 Reading, step 2).
    pub const ALL: [Kind; 9] = [
        Kind::Words12,
        Kind::Words15,
        Kind::Words18,
        Kind::Words21,
        Kind::Words24,
        Kind::Codex32,
        Kind::Passphrase,
        Kind::Note,
        Kind::RawKey,
    ];
}

/// What §9 reads from the bytes alone, with no passphrase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    /// The size, from the total length.
    pub size: Size,
    /// The memory step, from the first byte.
    pub step: Step,
}

/// §9: bytes are treated as an AQRV code when they are 53 or 62 bytes
/// long and the first is 0xF8 to 0xFC. Only the tag proves a match.
pub fn recognise(bytes: &[u8]) -> Option<Header> {
    let size = Size::of_len(bytes.len())?;
    let step = Step::of_header(*bytes.first()?)?;
    Some(Header { size, step })
}

/// Why [`seal`] wrote nothing (§3, §6 Writing step 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// The passphrase is empty (§10).
    EmptyPassphrase,
    /// A passphrase or note with no text (§3).
    EmptyText,
    /// The text is longer than the size has room for (§3).
    TooLong,
    /// Entropy of a length no BIP-39 word count has (§3).
    EntropyLength,
    /// A string BIP 93 refuses, or one of a length no form here has (§3.1).
    Codex32(faraday_bip::codex32::Error),
    /// A codex32 string of 28 or 32 bytes of seed asked for at the small
    /// size, which it never moves from (§3, §3.1).
    Codex32TooLongForSmall,
    /// Argon2id's memory could not be had.
    NotEnoughMemory,
}

/// Why [`open`] or [`workings`] gave nothing (§6 Reading).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotOpened {
    /// Not treated as AQRV (§9): no passphrase is asked for.
    NotRecognised,
    /// Argon2id's memory could not be had: said in words of its own,
    /// not "does not open" (§6 Reading step 1).
    NotEnoughMemory,
    /// No kind's tag matches: a wrong passphrase or a damaged code.
    NoTag,
    /// More than one kind's tag matches, which no writer makes.
    TwoTags,
    /// One kind's tag matches and its item fails the kind's layout.
    Layout(Kind, Layout),
}

/// The rule an item fails after its tag matched (§3, §3.1, §6 Reading
/// step 4). A reader says all of these as "does not open".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// A text's `len` is 0.
    LenZero,
    /// A text's `len` is more than the room less one.
    LenPastRoom,
    /// The text is not valid UTF-8.
    NotUtf8,
    /// A codex32 `size` of 5 to 7.
    Codex32Size,
    /// A codex32 `size` whose item does not fit the room.
    Codex32TooLong,
    /// A codex32 threshold that is neither `0` nor `2` to `9`.
    Codex32Threshold,
    /// A codex32 threshold of `0` with an index other than `s`.
    Codex32ThresholdZeroNotS,
}
