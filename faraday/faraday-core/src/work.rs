//! Counts of the costly work the app does, by kind, so a test can say
//! that drawing a frame twice, or moving the pointer, did none it should
//! not (`docs/SPEED.md` §3.3 and §7).
//!
//! Each costly function counts itself on its first line. The core's
//! count through the token they take (`faraday_crypto::work::Compute`, which
//! this crate mints with [`core_count`]); a canvas notes its pixel work
//! with the counter this crate sets on it (`faraday_draw::Canvas::set_counter`);
//! this crate's own kinds call [`count`], in functions that take the
//! token as their gate. A
//! count is a thread-local add, about a nanosecond, always compiled: not
//! a cargo feature and not `cfg(test)`, so faraday-core is built once. The counts
//! are per thread so that tests running side by side in one process do
//! not see each other's work; drawing and events run on the thread that
//! calls the app. Nothing reads them outside a test but the dev card's
//! shell, which times the events that did each kind
//! (`docs/DEVELOPING.md` §6.2), and takes the vault stretches noted with
//! [`note_vault`] for it.
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
    /// (`faraday-bip`).
    MasterKey,
    /// An account or child key derived from a master key, or an extended
    /// public key made from a private one (`faraday-bip`).
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
    /// A PSBT parsed from its bytes or its base64 (`faraday-psbt`).
    PsbtParse,
    /// A PSBT inspected against the session's keys and wallets
    /// (`faraday_psbt::inspect`, also inside every signing).
    PsbtInspect,
    /// A PSBT or a message signed (`faraday-psbt`).
    Sign,
    /// A wallet's script derived to find which wallet a PSBT spends from
    /// ([`crate::wallet::Wallet::script_at`]).
    WalletMatch,
    /// A PNG's pixels decoded to look for QR codes (`faraday-files`).
    PngDecode,
    /// QR codes looked for in a picture's pixels (`faraday-codec`).
    QrDecode,
    /// A key stretched: PBKDF2, scrypt or Argon2 (`faraday-crypto`,
    /// `faraday-backup`, `faraday-vault`).
    Kdf,
    /// A canvas reduced and blurred: a sheet's frosted backdrop, made
    /// once when the sheet opens (`faraday-draw`).
    Blur,
    /// A whole canvas copied: a snapshot taken, put back or blended
    /// (`faraday-draw`): a sheet's kept backdrop each frame, a fade's.
    FullCanvas,
    /// A codex32 checksum, worksheet column or interpolation of strings
    /// (`faraday-bip`'s `codex32`).
    Codex32,
    /// A key's signing context randomized from the session's randomness
    /// (`docs/KEYS.md` §2): an elliptic-curve multiplication.
    Blind,
    /// The first bytes of a file on a stick read, for a listing to tell
    /// a vault by its contents.
    FileRead,
}

/// Every kind, in the order the counts keep them.
pub const KINDS: [Work; 21] = [
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
    Work::Codex32,
    Work::Blind,
    Work::FileRead,
];

/// The kinds that are drawing's own pixel work: a frame does them (a
/// sheet's backdrop put back, a fade blended), so the budget test counts
/// them per frame instead of expecting none.
pub const PIXELS: [Work; 2] = [Work::Blur, Work::FullCanvas];

/// The core's kind as this crate counts it.
fn of_core(w: faraday_crypto::work::Work) -> Work {
    use faraday_crypto::work::Work as C;
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
        C::Codex32 => Work::Codex32,
        C::Blind => Work::Blind,
        C::FileRead => Work::FileRead,
    }
}

/// The counter the core's costly functions call through the token, and
/// a canvas with its pixel work (`faraday_crypto::work`): the Starter's
/// canvas counts with it too. Costs an add.
pub fn core_count(w: faraday_crypto::work::Work) {
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

/// What a vault's key stretch was for, for the dev card's timings
/// (`docs/DEVELOPING.md` §6.2): the act, the cost, which is the vault's
/// header and not secret, and how it ended. Nothing in it comes from a
/// passphrase, a name or the vault's contents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VaultWork {
    /// What the stretch was for.
    pub act: VaultAct,
    /// The Argon2id memory, MiB.
    pub memory_mib: u32,
    /// The Argon2id passes.
    pub passes: u32,
    /// How it ended.
    pub ended: VaultEnded,
}

/// What a vault's key stretch was for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultAct {
    /// A vault made: one stretch for each passphrase.
    Made,
    /// A vault unlocked.
    Unlocked,
    /// The one 64 MiB pass at boot that times this computer.
    Measured,
}

/// How a vault's key stretch ended. A wrong passphrase is not told
/// apart from a right one: both stretched in full.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultEnded {
    /// The stretch ran.
    Ran,
    /// Its memory could not be allocated (`faraday_vault::Error::Memory`).
    NoMemory,
    /// Not started: the memory free here is less than it needs
    /// (`docs/VAULT.md` §3.1).
    NotFree,
    /// The file's header was refused before anything ran.
    Refused,
}

impl VaultEnded {
    /// How a vault's open or make ended, by its error's kind. Cheap.
    pub fn of<T>(result: &Result<T, faraday_vault::Error>) -> VaultEnded {
        use faraday_vault::Error as E;
        match result {
            Err(E::Memory(_)) => VaultEnded::NoMemory,
            Err(E::Magic | E::Version | E::SlotSize | E::Length | E::Cost) => VaultEnded::Refused,
            _ => VaultEnded::Ran,
        }
    }
}

impl VaultWork {
    /// The preset this cost is (`docs/VAULT.md` §3.1), or Custom. Cheap.
    pub fn strength(&self) -> &'static str {
        crate::vault::PRESETS
            .iter()
            .find(|p| p.1 == self.memory_mib && p.2 == self.passes)
            .map_or(faraday_strings::EN.vault_custom, |p| p.0)
    }
}

/// The most vault notes kept for the shell to take: a shell that takes
/// none (every shell but the dev card's) keeps this many and no more.
const VAULT_NOTES: usize = 8;

thread_local! {
    static VAULTS: std::cell::RefCell<Vec<VaultWork>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Notes a vault's key stretch, `act` at `cost`, ended `ended`, for the
/// dev card's timings ([`take_vault_work`]). Costs a push.
pub fn note_vault(act: VaultAct, cost: faraday_vault::Cost, ended: VaultEnded) {
    VAULTS.with(|v| {
        let mut v = v.borrow_mut();
        if v.len() < VAULT_NOTES {
            v.push(VaultWork {
                act,
                memory_mib: cost.memory_kib / 1024,
                passes: cost.passes,
                ended,
            });
        }
    });
}

/// `refusal`, noted as a stretch of `memory_mib` and `passes` not tried
/// for want of memory free ([`VaultEnded::NotFree`]) when there is one.
/// Costs a push.
pub fn refused<T>(act: VaultAct, memory_mib: u32, passes: u32, refusal: Option<T>) -> Option<T> {
    if refusal.is_some() {
        let cost = faraday_vault::Cost {
            memory_kib: memory_mib * 1024,
            passes,
            lanes: 1,
        };
        note_vault(act, cost, VaultEnded::NotFree);
    }
    refusal
}

/// The vault stretches noted on this thread since the last call, oldest
/// first. The dev card's shell takes them after each event it times.
pub fn take_vault_work() -> Vec<VaultWork> {
    VAULTS.with(|v| std::mem::take(&mut *v.borrow_mut()))
}

#[cfg(test)]
mod budget;
#[cfg(test)]
mod kept;
