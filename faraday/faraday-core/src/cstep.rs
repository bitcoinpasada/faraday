//! The create cards, in order: their numbers.

/// The kind.
pub const KIND: u8 = 0;
/// The quorum.
pub const QUORUM: u8 = 1;
/// The keys; their Continue makes the wallet.
pub const KEYS: u8 = 2;
/// The descriptor and the first addresses.
pub const CHECK: u8 = 3;
/// No card now: Check's **Back up wallet** opens Back up.
pub const BACKUP: u8 = 4;
/// Whether vaults are part of the backup, before Keys.
pub const WHERE: u8 = 5;
/// Bring in the sticks of the vaults, before Keys.
pub const STICKS: u8 = 6;
/// Unlock and make the vaults, before Keys.
pub const VAULTS: u8 = 7;
/// How many vaults, before the sticks.
pub const HOW_MANY: u8 = 8;
/// A single key's backup, words or codex32 shares, after Kind (F240).
pub const BACKUP_FORM: u8 = 9;
/// How many cards there are.
pub const COUNT: usize = 10;
