//! The pictures of `docs/guide/walk/messages.md`.

use super::{key_session, kind_at};
use crate::tour::Tour;
use faraday_core::create::NewKind;
use faraday_core::{Action, Screen, cstep};

/// Costs what making a single-key wallet costs. A fresh stick session
/// with the guide's key typed in and New wallet 1 made from it: Sign a
/// message, then Check a signed message.
pub(super) fn run(t: &mut Tour) -> Result<(), String> {
    key_session(t);
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::CreateWallet);
    t.press(Action::CKind(kind_at(NewKind::NativeSegwit)?));
    t.press(Action::CVaults(false));
    let fp = t.app.session.keys[0].fingerprint.0;
    t.press(Action::CSlotHere(0, fp));
    t.press(Action::CNext(cstep::KEYS));
    t.press(Action::CNext(cstep::CHECK));
    t.press(Action::SignMessage);
    t.shot("messages-start")?;
    t.press(Action::Nav(Screen::CheckMessage));
    t.shot("messages-check")
}
