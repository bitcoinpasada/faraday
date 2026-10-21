//! Counts of the costly work the app does, by kind, so a test can say
//! that drawing a frame twice, or moving the pointer, did none it should
//! not (`docs/SPEED.md` §3.3 and §7).
//!
//! Each costly function calls [`count`] on its first line. A count is a
//! thread-local add, about a nanosecond, always compiled: not a cargo
//! feature and not `cfg(test)`, so faraday-core is built once. The counts
//! are per thread so that tests running side by side in one process do
//! not see each other's work; drawing and events run on the thread that
//! calls the app. Nothing reads them outside a test.

use std::cell::Cell;

/// A kind of costly work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Work {
    /// A master key made from a seed (BIP-39's PBKDF2 or a seed's HMAC)
    /// and its fingerprint (an elliptic-curve multiplication).
    MasterKey,
    /// An account or child key derived from a master key.
    KeyDerive,
    /// A wallet's address derived.
    Address,
    /// A wallet description read (`read_wallet`): parsed, checked and
    /// its keys read.
    DescriptorParse,
    /// A file's or a transaction's bytes hashed whole.
    FileHash,
    /// A backup plan mapped (`plan::map`) or checked (`plan::check`).
    PlanMap,
    /// A QR code, a SeedQR or a BBQr run encoded.
    QrEncode,
    /// Seed words parsed (`Mnemonic::parse`, a vault key record's
    /// words): each word looked up in its list and the checksum hashed.
    WordsParse,
    /// A PNG or a PDF made.
    Picture,
}

/// Every kind, in the order the counts keep them.
pub const KINDS: [Work; 9] = [
    Work::MasterKey,
    Work::KeyDerive,
    Work::Address,
    Work::DescriptorParse,
    Work::FileHash,
    Work::PlanMap,
    Work::QrEncode,
    Work::WordsParse,
    Work::Picture,
];

thread_local! {
    static COUNTS: [Cell<u64>; KINDS.len()] = const { [const { Cell::new(0) }; KINDS.len()] };
}

/// Notes one piece of work of kind `w` on this thread. Costs an add.
#[inline]
pub fn count(w: Work) {
    COUNTS.with(|c| {
        let cell = &c[w as usize];
        cell.set(cell.get().wrapping_add(1));
    });
}

/// The counts so far on this thread, by kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts([u64; KINDS.len()]);

impl Counts {
    /// The count of kind `w`.
    pub fn of(&self, w: Work) -> u64 {
        self.0[w as usize]
    }

    /// The work done since `before` was taken.
    pub fn since(&self, before: &Counts) -> Counts {
        let mut out = Counts::default();
        for (i, o) in out.0.iter_mut().enumerate() {
            *o = self.0[i].wrapping_sub(before.0[i]);
        }
        out
    }
}

/// The counts so far on this thread.
pub fn snapshot() -> Counts {
    COUNTS.with(|c| {
        let mut out = Counts::default();
        for (o, cell) in out.0.iter_mut().zip(c.iter()) {
            *o = cell.get();
        }
        out
    })
}

/// The work done on this thread since `before` was taken.
pub fn since(before: &Counts) -> Counts {
    snapshot().since(before)
}

#[cfg(test)]
mod budget;
