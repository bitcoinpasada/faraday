//! Counts of the costly work the app does, by kind, so a test can say
//! that drawing a frame twice, or moving the pointer, did none it should
//! not (`docs/SPEED.md` §3.3 and §7).
//!
//! Each costly function counts itself on its first line. The core's
//! count through the token they take (`osk_crypto::work::Compute`, which
//! this crate mints with [`core_count`]); a canvas notes its pixel work
//! with the counter this crate sets on it (`osk_ui::Canvas::set_counter`);
//! this crate's own kinds call [`count`], in functions that take the
//! token as their gate. A
//! count is a thread-local add, about a nanosecond, always compiled: not
//! a cargo feature and not `cfg(test)`, so faraday-core is built once. The counts
//! are per thread so that tests running side by side in one process do
//! not see each other's work; drawing and events run on the thread that
//! calls the app. Nothing reads them outside a test.
//!
//! Costly work drawing may reach goes through `drawing::View`'s token
//! (`docs/COMPUTE.md` §3 and §4); the type, the costs lint and the
//! doctests keep the rest off a frame, so nothing here checks it at run
//! time.

use std::cell::Cell;

/// A kind of costly work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Work {
    /// A master key made from a seed (an HMAC-SHA-512) or an `xprv`,
    /// and its fingerprint (an elliptic-curve multiplication)
    /// (`osk-bip`).
    MasterKey,
    /// An account or child key derived from a master key, or an extended
    /// public key made from a private one (`osk-bip`).
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
    /// A PSBT parsed from its bytes or its base64 (`osk-psbt`).
    PsbtParse,
    /// A PSBT inspected against the session's keys and wallets
    /// (`osk_psbt::inspect`, also inside every signing).
    PsbtInspect,
    /// A PSBT or a message signed (`osk-psbt`).
    Sign,
    /// A wallet's script derived to find which wallet a PSBT spends from
    /// ([`crate::wallet::Wallet::script_at`]).
    WalletMatch,
    /// A PNG's pixels decoded to look for QR codes (`faraday-files`).
    PngDecode,
    /// QR codes looked for in a picture's pixels (`osk-codec`).
    QrDecode,
    /// A key stretched: PBKDF2, scrypt or Argon2 (`osk-crypto`,
    /// `osk-backup`, `faraday-vault`).
    Kdf,
    /// A canvas reduced and blurred: a sheet's frosted backdrop, made
    /// once when the sheet opens (`osk-ui`).
    Blur,
    /// A whole canvas copied: a snapshot taken, put back or blended
    /// (`osk-ui`): a sheet's kept backdrop each frame, a fade's.
    FullCanvas,
}

/// Every kind, in the order the counts keep them.
pub const KINDS: [Work; 18] = [
    Work::MasterKey,
    Work::KeyDerive,
    Work::Address,
    Work::DescriptorParse,
    Work::FileHash,
    Work::PlanMap,
    Work::QrEncode,
    Work::WordsParse,
    Work::Picture,
    Work::PsbtParse,
    Work::PsbtInspect,
    Work::Sign,
    Work::WalletMatch,
    Work::PngDecode,
    Work::QrDecode,
    Work::Kdf,
    Work::Blur,
    Work::FullCanvas,
];

/// The kinds that are drawing's own pixel work: a frame does them (a
/// sheet's backdrop put back, a fade blended), so the budget test counts
/// them per frame instead of expecting none.
pub const PIXELS: [Work; 2] = [Work::Blur, Work::FullCanvas];

/// The core's kind as this crate counts it.
fn of_core(w: osk_crypto::work::Work) -> Work {
    use osk_crypto::work::Work as C;
    match w {
        C::PsbtParse => Work::PsbtParse,
        C::PsbtInspect => Work::PsbtInspect,
        C::Sign => Work::Sign,
        C::QrDecode => Work::QrDecode,
        C::QrEncode => Work::QrEncode,
        C::FileHash => Work::FileHash,
        C::PngDecode => Work::PngDecode,
        C::Kdf => Work::Kdf,
        C::WordsParse => Work::WordsParse,
        C::MasterKey => Work::MasterKey,
        C::KeyDerive => Work::KeyDerive,
        C::DescriptorParse => Work::DescriptorParse,
        C::Address => Work::Address,
        C::Blur => Work::Blur,
        C::FullCanvas => Work::FullCanvas,
    }
}

/// The counter the core's costly functions call through the token, and
/// a canvas with its pixel work (`osk_crypto::work`): the Starter's
/// canvas counts with it too. Costs an add.
pub fn core_count(w: osk_crypto::work::Work) {
    count(of_core(w));
}

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
#[cfg(test)]
mod kept;
