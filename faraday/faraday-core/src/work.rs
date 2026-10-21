//! Counts of the costly work the app does, by kind, so a test can say
//! that drawing a frame twice, or moving the pointer, did none it should
//! not (`docs/SPEED.md` §3.3 and §7).
//!
//! Each costly function calls [`count`] on its first line; the core's
//! own count themselves: seed words parsed, a key stretched, a master key
//! made and a key derived through the token they take
//! (`osk_crypto::work::Compute`, minted with [`core_count`]), and a PSBT
//! parsed, inspected or signed, a QR code read, a canvas blurred or
//! copied through `osk_crypto::work`'s process-wide counter, which is
//! this crate's ([`install`]). A
//! count is a thread-local add, about a nanosecond, always compiled: not
//! a cargo feature and not `cfg(test)`, so faraday-core is built once. The counts
//! are per thread so that tests running side by side in one process do
//! not see each other's work; drawing and events run on the thread that
//! calls the app. Nothing reads them outside a test.
//!
//! In debug and test builds [`count`] is also the guard of `docs/SPEED.md`
//! §3.3: costly work while a frame is drawn panics unless the drawing
//! memo is filling one of its cells ([`filling`]), a value kept on the
//! state is worked out the first time ([`keeping`]), or an [`Allowed`]
//! entry names it. `render` marks the frame ([`Frame`]). Release builds
//! compile the guard out.

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
/// sheet's backdrop put back, a fade blended), so the guard lets them
/// through and the budget test counts them per frame instead.
pub const PIXELS: [Work; 2] = [Work::Blur, Work::FullCanvas];

/// The core's kind as this crate counts it.
fn of_core(w: osk_crypto::work::Work) -> Work {
    use osk_crypto::work::Work as C;
    match w {
        C::PsbtParse => Work::PsbtParse,
        C::PsbtInspect => Work::PsbtInspect,
        C::Sign => Work::Sign,
        C::QrDecode => Work::QrDecode,
        C::PngDecode => Work::PngDecode,
        C::Kdf => Work::Kdf,
        C::WordsParse => Work::WordsParse,
        C::MasterKey => Work::MasterKey,
        C::KeyDerive => Work::KeyDerive,
        C::Blur => Work::Blur,
        C::FullCanvas => Work::FullCanvas,
    }
}

/// The counter the core's costly functions call (`osk_crypto::work`).
pub(crate) fn core_count(w: osk_crypto::work::Work) {
    count(of_core(w));
}

/// Has the core count its costly work here, for the process: once, at
/// the app's start and at the first count a test takes. A lookup after.
pub(crate) fn install() {
    osk_crypto::work::set_counter(core_count);
}

thread_local! {
    static COUNTS: [Cell<u64>; KINDS.len()] = const { [const { Cell::new(0) }; KINDS.len()] };
}

#[cfg(debug_assertions)]
thread_local! {
    /// True while a frame is drawn on this thread.
    static DRAWING: Cell<bool> = const { Cell::new(false) };
    /// How many drawing-memo fills are under way on this thread.
    static FILLING: Cell<u32> = const { Cell::new(0) };
}

/// Marks a frame being drawn on this thread until it is dropped, however
/// the drawing returns. Debug and test builds only; release has nothing
/// to mark.
pub(crate) struct Frame {
    #[cfg(debug_assertions)]
    was: bool,
}

impl Frame {
    /// Marks the frame begun.
    pub(crate) fn begin() -> Self {
        #[cfg(debug_assertions)]
        {
            let was = DRAWING.with(|d| d.replace(true));
            Frame { was }
        }
        #[cfg(not(debug_assertions))]
        Frame {}
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        DRAWING.with(|d| d.set(self.was));
    }
}

/// Runs `f`, a fill of one of the drawing memo's cells: the costly work
/// it does while a frame is drawn is the memo's once-a-frame work, which
/// the guard in [`count`] lets through.
#[inline]
pub(crate) fn filling<T>(f: impl FnOnce() -> T) -> T {
    #[cfg(debug_assertions)]
    {
        struct Fill;
        impl Drop for Fill {
            fn drop(&mut self) {
                FILLING.with(|n| n.set(n.get().saturating_sub(1)));
            }
        }
        FILLING.with(|n| n.set(n.get() + 1));
        let _fill = Fill;
        f()
    }
    #[cfg(not(debug_assertions))]
    f()
}

/// Runs `f`, the work of a value kept on the state that whoever asks
/// first works out, a frame included: a wallet's address, a key record's
/// fingerprint, Explore's reading. It is kept with what it describes, by
/// the inputs it is worked out from, so it is worked out once and never
/// stale (`docs/SPEED.md` §2 rules 2 and 4); the guard in [`count`] lets
/// that once through.
#[inline]
pub(crate) fn keeping<T>(f: impl FnOnce() -> T) -> T {
    filling(f)
}

/// Costly work drawing does outside the memo and the state, each named
/// with why it is not kept: the allowlist of the guard in [`count`]. The
/// owner decides each entry (`docs/SPEED.md` §3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Allowed {
    /// A secret the person opened to see or to copy by hand, drawn from
    /// its source on each frame: a seed's words parsed for its SeedQR
    /// grid on the backup's copy page, a vault key record's words and
    /// a BIP-85 child shown. Nothing secret is kept, on the state or in
    /// the memo for a frame (§2 rule 1), so there is nothing to keep it
    /// in; it is drawn only while that page or row is open.
    SecretShown,
}

/// Runs `f`, costly work drawing does by an [`Allowed`] entry.
#[inline]
pub(crate) fn allowed<T>(why: Allowed, f: impl FnOnce() -> T) -> T {
    let _ = why;
    filling(f)
}

/// Notes one piece of work of kind `w` on this thread. Costs an add. In
/// debug and test builds, panics when called while a frame is drawn
/// outside [`filling`], [`keeping`] and [`allowed`]: the work belongs on
/// the state or in the memo (`docs/SPEED.md` §3.3).
#[inline]
#[cfg_attr(debug_assertions, track_caller)]
pub fn count(w: Work) {
    #[cfg(debug_assertions)]
    if DRAWING.with(Cell::get)
        && FILLING.with(Cell::get) == 0
        && !PIXELS.contains(&w)
        && !std::thread::panicking()
    {
        let at = std::panic::Location::caller();
        panic!(
            "costly work while drawing: {w:?} in {}:{}; compute it on the state or through the drawing memo (docs/SPEED.md §3.3)\ncalled from: {}",
            at.file(),
            at.line(),
            callers()
        );
    }
    COUNTS.with(|c| {
        let cell = &c[w as usize];
        cell.set(cell.get().wrapping_add(1));
    });
}

/// The app's own functions on the stack, innermost first, for the
/// guard's message.
#[cfg(debug_assertions)]
fn callers() -> String {
    let trace = std::backtrace::Backtrace::force_capture().to_string();
    trace
        .lines()
        .filter_map(|l| l.trim().split_once(": ").map(|(_, f)| f))
        .map(|f| f.trim_start_matches('<'))
        .filter(|f| f.starts_with("faraday_core::") && !f.starts_with("faraday_core::work::"))
        .take(8)
        .map(|f| f.trim_start_matches("faraday_core::").replace(">::", "::"))
        .collect::<Vec<_>>()
        .join(" <- ")
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
    install();
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
