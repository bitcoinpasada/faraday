//! Handles, tickets and the enums two composites name (03 §3.3, §8.1; 09 §4.1).

use crate::message::Public;

/// A key, by id minted by the session; never reused within a build of the lock scope.
#[derive(Public, Copy, PartialEq, Eq, Hash)]
pub struct KeyId(u32);

impl KeyId {
    /// The `n`th handle of a build of the lock scope. Only the session owner counts, and never
    /// gives one number twice (03 §3.3).
    pub fn minted(n: u32) -> KeyId {
        KeyId(n)
    }
}

#[derive(Public, Copy, PartialEq, Eq, Hash)]
pub struct WalletId(u32);

#[derive(Public, Copy, PartialEq, Eq, Hash)]
pub struct FileId(u32);

impl FileId {
    /// The `n`th handle of a build of the lock scope. Only the session owner counts, and never
    /// gives one number twice (03 §3.3); a test names its own.
    pub fn minted(n: u32) -> FileId {
        FileId(n)
    }
}

#[derive(Public, Copy, PartialEq, Eq)]
pub struct Ticket(u32);

/// One awaited answer of a feature; a part of its state.
#[derive(Public, Default)]
pub struct Waiting {
    next: u32,
    live: Option<Ticket>,
}

impl Waiting {
    /// A new ticket; any earlier one is stale.
    pub fn begin(&mut self) -> Ticket {
        todo!()
    }

    /// True once, for the live ticket.
    pub fn take(&mut self, _t: Ticket) -> bool {
        todo!()
    }

    /// Back, Cancel, a stick that moves the screen.
    pub fn forget(&mut self) {
        todo!()
    }

    /// The step is running.
    pub fn live(&self) -> bool {
        todo!()
    }
}

/// What a scan asks for; the scan answers along the path the composites record from the
/// `Out::Scan` that opened it, as the camera's holder (06 §6.2).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum ScanFor {
    Psbt,
    Key,
    Wallet,
    Address,
    Message,
    Text,
    File,
    Any,
}

/// A feature's ask to show a secret; the device composite routes it to devices, and back
/// (06 §7).
#[derive(Public, Copy, PartialEq, Eq)]
pub struct Gate(pub u32);

/// The tools a menu page opens (06 §3.3).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Tool {}

/// A published sample a device feature asks for: the online composite routes it to the strip,
/// which loads it and answers the feature that asked; the device app drops it (09 §2.1).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum SampleAsk {
    /// The codex32 sample (BIP 93's second vector), for the share worksheet.
    Codex32,
    /// The loaded sample wallet's spend, read as a scanned PSBT.
    Spend,
    /// The open tool's sample text, typed into its field.
    ToolText(Tool),
}
