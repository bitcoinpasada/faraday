//! The settings owner's state and changes (03 §7.2, §7.3).

use super::change;
use crate::cx::Edits;
use crate::message::Public;

/// The settings owner's pages (06 §4.2). The kernel's, as `vaults::Page` is.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    Settings,
    SetTime,
}

/// The person's choices: idle lock and off minutes, QR speed, time offset, appearance, scale,
/// rotation.
// Spec: 03 §7.2 sketches the fields as a comment; the settings owner's crate keeps them.
pub struct Settings(());

impl Settings {
    pub fn edits(&self) -> Edits {
        todo!()
    }
}

/// One choice set, or the time set.
// Spec: 03 §7.3 names `settings::Change` without variants.
pub struct Set(());

pub enum Change {
    Set(Set),
}

pub enum Done {
    Set(()),
}

// Spec: 03 §2.4 names a settings change among `Changed`; it is not shaped.
pub struct Changed(());

change!(Set => ());
