//! The pictures of `docs/guide/walk/vaults-and-passwords.md`.

use super::key_session;
use crate::tour::Tour;
use faraday_core::{Action, Screen};

/// A fresh stick session with the guide's key typed in: Create a vault,
/// and Set the time.
pub(super) fn run(t: &mut Tour) -> Result<(), String> {
    key_session(t);
    t.press(Action::Nav(Screen::CreateVault));
    t.shot("vaults-and-passwords-create")?;
    t.press(Action::Nav(Screen::SetTime));
    t.shot("vaults-and-passwords-set-time")
}
