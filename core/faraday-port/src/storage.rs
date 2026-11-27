//! The storage family's value types and asks (02 §3.5, §4.1, §4.2).

use alloc::vec::Vec;

use faraday_text::Text;

use crate::ask::{Answer, Ask, Failure, request};
use crate::bytes::Bytes;

/// The adapter's name for a stick, as `StickInfo::id` gives it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StickId(pub Text);

/// A SHA-256 hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sha256(pub [u8; 32]);

/// What a stick's device reports of itself; `None` where the adapter does not know.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StickMake {
    pub maker: Option<Text>,
    pub model: Option<Text>,
    pub size: Option<u64>,
}

/// A stick the adapter lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StickInfo {
    /// Where the adapter finds it; the app hands this back unchanged.
    pub id: StickId,
    /// The name a person knows it by: its volume label.
    pub label: Text,
    /// Whether it is the medium the machine booted from.
    pub boot: bool,
    /// Its FAT volume serial number, when the adapter can read one.
    pub serial: Option<u32>,
    /// The files at its top level, with their sizes.
    pub files: Vec<(Text, u64)>,
    /// The files among them that are vaults by their contents.
    pub vaults: Vec<Text>,
    pub make: StickMake,
}

/// Which medium the machine booted from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BootMedium {
    #[default]
    Known,
    Two,
    NotFound,
}

/// What a write does when the name is taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnTaken {
    Ask,
    Beside,
    Over(Sha256),
}

/// Why a rename was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unrenamed {
    Taken,
    SameVault,
    Failed(Failure),
}

/// Why naming a stick was refused: a label FAT cannot hold, or a failure (a folder stick's
/// is `Unsupported`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameRefused {
    Invalid,
    Failed(Failure),
}

pub enum StorageAsk {
    Read {
        stick: StickId,
        name: Text,
    },
    /// The codes in a picture on a stick; the adapter decodes it.
    ReadCodes {
        stick: StickId,
        name: Text,
    },
    Write {
        stick: StickId,
        name: Text,
        bytes: Bytes,
        /// Every copy of this vault the session has seen.
        copies: Vec<Sha256>,
        /// The person chose to replace.
        replace: bool,
        renamed: bool,
        on_taken: OnTaken,
    },
    Wipe {
        stick: StickId,
        name: Text,
    },
    Space {
        stick: StickId,
    },
    ZeroFree {
        stick: StickId,
        from: u32,
    },
    MakeEmpty {
        stick: StickId,
    },
    Rename {
        stick: StickId,
        from: Text,
        to: Text,
        vaults: Vec<Vec<Sha256>>,
    },
    Name {
        stick: StickId,
        label: Text,
    },
    Hash {
        stick: StickId,
        name: Text,
    },
}

pub enum StorageAnswer {
    Read(Result<Bytes, Failure>),
    /// The picture's bytes and each code's payload.
    Codes(Result<Codes, Failure>),
    Written(Result<Written, Failure>),
    Wiped(Result<Wiped, Failure>),
    /// Free and size; 0 and 0 for a folder.
    Space(Result<Room, Failure>),
    Zeroing(Result<Zeroing, Failure>),
    Emptied(Result<(), Failure>),
    Renamed(Result<(), Unrenamed>),
    Named(Result<(), NameRefused>),
    Hashed(Result<Sha256, Failure>),
}

/// A picture's bytes and each code read in it.
pub struct Codes {
    pub picture: Bytes,
    pub codes: Vec<Bytes>,
}

pub enum Written {
    Wrote { wrote_as: Text },
    NameTaken { on_stick: Text, sha: Sha256 },
}

pub enum Wiped {
    Zeroed,
    NameOnly,
}

/// A stick's free space and size, in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Room {
    pub free: u64,
    pub size: u64,
}

/// One step of zeroing a stick's free space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Zeroing {
    pub next: u32,
    pub total: u32,
    pub done: bool,
}

/// What one process leaves the next across a lock: named files in the app's own format, stored
/// unread, each written whole in the order given.
pub struct Carry {
    pub inbox: Files,
    pub outbox: Files,
    /// Settings, the signed-amount memory, the mode, the input counts, ...
    pub kept: Files,
}

pub type Files = Vec<(Text, Bytes)>;

/// `StorageAsk::Read`'s own type.
pub struct Read {
    pub stick: StickId,
    pub name: Text,
}
request!(Read => Result<Bytes, Failure>;
    into a => Ask::Storage(StorageAsk::Read { stick: a.stick, name: a.name });
    take r @ Answer::Storage(StorageAnswer::Read(r)),
    fault Err(Failure::Adapter));

/// `StorageAsk::ReadCodes`'s own type.
pub struct ReadCodes {
    pub stick: StickId,
    pub name: Text,
}
request!(ReadCodes => Result<Codes, Failure>;
    into a => Ask::Storage(StorageAsk::ReadCodes { stick: a.stick, name: a.name });
    take r @ Answer::Storage(StorageAnswer::Codes(r)),
    fault Err(Failure::Adapter));

/// `StorageAsk::Write`'s own type.
pub struct Write {
    pub stick: StickId,
    pub name: Text,
    pub bytes: Bytes,
    pub copies: Vec<Sha256>,
    pub replace: bool,
    pub renamed: bool,
    pub on_taken: OnTaken,
}
request!(Write => Result<Written, Failure>;
    into a => Ask::Storage(StorageAsk::Write { stick: a.stick, name: a.name, bytes: a.bytes, copies: a.copies, replace: a.replace, renamed: a.renamed, on_taken: a.on_taken });
    take r @ Answer::Storage(StorageAnswer::Written(r)),
    fault Err(Failure::Adapter));

/// `StorageAsk::Wipe`'s own type.
pub struct Wipe {
    pub stick: StickId,
    pub name: Text,
}
request!(Wipe => Result<Wiped, Failure>;
    into a => Ask::Storage(StorageAsk::Wipe { stick: a.stick, name: a.name });
    take r @ Answer::Storage(StorageAnswer::Wiped(r)),
    fault Err(Failure::Adapter));

/// `StorageAsk::Space`'s own type.
pub struct Space {
    pub stick: StickId,
}
request!(Space => Result<Room, Failure>;
    into a => Ask::Storage(StorageAsk::Space { stick: a.stick });
    take r @ Answer::Storage(StorageAnswer::Space(r)),
    fault Err(Failure::Adapter));

/// `StorageAsk::ZeroFree`'s own type.
pub struct ZeroFree {
    pub stick: StickId,
    pub from: u32,
}
request!(ZeroFree => Result<Zeroing, Failure>;
    into a => Ask::Storage(StorageAsk::ZeroFree { stick: a.stick, from: a.from });
    take r @ Answer::Storage(StorageAnswer::Zeroing(r)),
    fault Err(Failure::Adapter));

/// `StorageAsk::MakeEmpty`'s own type.
pub struct MakeEmpty {
    pub stick: StickId,
}
request!(MakeEmpty => Result<(), Failure>;
    into a => Ask::Storage(StorageAsk::MakeEmpty { stick: a.stick });
    take r @ Answer::Storage(StorageAnswer::Emptied(r)),
    fault Err(Failure::Adapter));

/// `StorageAsk::Rename`'s own type.
pub struct Rename {
    pub stick: StickId,
    pub from: Text,
    pub to: Text,
    pub vaults: Vec<Vec<Sha256>>,
}
request!(Rename => Result<(), Unrenamed>;
    into a => Ask::Storage(StorageAsk::Rename { stick: a.stick, from: a.from, to: a.to, vaults: a.vaults });
    take r @ Answer::Storage(StorageAnswer::Renamed(r)),
    fault Err(Unrenamed::Failed(Failure::Adapter)));

/// `StorageAsk::Name`'s own type.
pub struct Name {
    pub stick: StickId,
    pub label: Text,
}
request!(Name => Result<(), NameRefused>;
    into a => Ask::Storage(StorageAsk::Name { stick: a.stick, label: a.label });
    take r @ Answer::Storage(StorageAnswer::Named(r)),
    fault Err(NameRefused::Failed(Failure::Adapter)));

/// `StorageAsk::Hash`'s own type.
pub struct Hash {
    pub stick: StickId,
    pub name: Text,
}
request!(Hash => Result<Sha256, Failure>;
    into a => Ask::Storage(StorageAsk::Hash { stick: a.stick, name: a.name });
    take r @ Answer::Storage(StorageAnswer::Hashed(r)),
    fault Err(Failure::Adapter));
