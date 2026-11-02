//! What the board draws for a stick and the vaults on it.

use super::{BoardPress, ChipView, ZoneView};

/// One stick at a place.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StickView {
    /// "Stick 1".
    pub label: String,
    /// What its × does: the stick taken off, or an empty stick.
    pub remove: BoardPress,
    /// The vaults on it, in vault order, each in a box of its own
    /// (`docs/STICK-SETUP.md` §6.3).
    pub vaults: Vec<VaultView>,
    /// Its files.
    pub files: ZoneView,
}

/// One vault on a stick.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VaultView {
    /// Its chip, the stick's head for it.
    pub head: ChipView,
    /// The vault's own zone.
    pub inside: ZoneView,
    /// The vault's header band, "Vault 1 · sealed", or "Vault 1 · sealed ·
    /// made in the checklist" while it is not made yet.
    pub sealed: String,
    /// The band where `sealed` does not fit: the same, or "Vault 1 · new".
    pub sealed_short: String,
}
