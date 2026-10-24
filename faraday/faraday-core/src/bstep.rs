//! The backup checklist's items, numbered: the number an open item is
//! kept by, and the item a number stands for.

use crate::plan::Item;

/// Print the blank templates.
pub const BLANK: u8 = 0;
/// The public files for the software chosen.
pub const PUBLIC: u8 = 2;
/// The wallet sheet or the shares.
pub const SHEETS: u8 = 3;
/// One envelope per place.
pub const ENVELOPE: u8 = 4;
/// The seeds as files.
pub const FILES: u8 = 6;
/// The descriptor shown to the watch-only software.
pub const SHOW: u8 = 8;
/// Write to sticks: one lock, then each stick by place.
pub const STICKS: u8 = 10;
/// Copy seed i by hand and check it: `COPY + i`, the seed by its
/// place among the wallet's, below [`CODEX32`].
pub const COPY: u8 = 16;
/// Seed i's codex32 strings on paper: `CODEX32 + i`, below
/// [`PASSPHRASE`] (`docs/CODEX32.md` §9.2).
pub const CODEX32: u8 = 44;
/// Seed i's passphrase on paper, checked: `PASSPHRASE + i`, below
/// [`VAULT`].
pub const PASSPHRASE: u8 = 72;
/// Vault v made and filled: `VAULT + v`, the first vault `VAULT`.
pub const VAULT: u8 = 128;

/// Whether `n` is a seed's copy by hand.
pub fn is_copy(n: u8) -> bool {
    (COPY..CODEX32).contains(&n)
}

/// Whether `n` is a seed's codex32 strings.
pub fn is_codex32(n: u8) -> bool {
    (CODEX32..PASSPHRASE).contains(&n)
}

/// Whether `n` is a seed's passphrase on paper.
pub fn is_passphrase(n: u8) -> bool {
    (PASSPHRASE..VAULT).contains(&n)
}

/// An item's number.
pub fn of(item: Item) -> u8 {
    match item {
        Item::Templates => BLANK,
        Item::Copy(i) => COPY + i.min(usize::from(CODEX32 - COPY - 1)) as u8,
        Item::Codex32(i) => CODEX32 + i.min(usize::from(PASSPHRASE - CODEX32 - 1)) as u8,
        Item::Passphrase(i) => PASSPHRASE + i.min(usize::from(VAULT - PASSPHRASE - 1)) as u8,
        Item::Vault(v) => VAULT.saturating_add(v.min(127) as u8),
        Item::SeedFiles => FILES,
        Item::Sheets => SHEETS,
        Item::PublicFiles => PUBLIC,
        Item::ShowDescriptor => SHOW,
        Item::Envelopes => ENVELOPE,
        Item::Sticks => STICKS,
    }
}

/// The item a number stands for.
pub fn item(n: u8) -> Option<Item> {
    Some(match n {
        BLANK => Item::Templates,
        FILES => Item::SeedFiles,
        SHEETS => Item::Sheets,
        PUBLIC => Item::PublicFiles,
        SHOW => Item::ShowDescriptor,
        ENVELOPE => Item::Envelopes,
        STICKS => Item::Sticks,
        n if n >= VAULT => Item::Vault(usize::from(n - VAULT)),
        n if n >= PASSPHRASE => Item::Passphrase(usize::from(n - PASSPHRASE)),
        n if n >= CODEX32 => Item::Codex32(usize::from(n - CODEX32)),
        n if n >= COPY => Item::Copy(usize::from(n - COPY)),
        _ => return None,
    })
}
