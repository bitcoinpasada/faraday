//! The pictures of `docs/guide/walk/messages.md`.

use super::{key_session, kind_at};
use crate::tour::{Tour, type_text};
use faraday_core::create::NewKind;
use faraday_core::{Action, Screen, cstep};

/// Costs what making a single-key wallet costs. A fresh stick session
/// with the guide's key typed in and New wallet 1 made from it: Sign a
/// message; its Message card with the guide's text typed, and its
/// Signature card once signed; then Check a signed message on it,
/// brought back into Files.
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
    t.press(Action::MNext(0));
    type_text(t, "Faraday guide example, 27 October 2026");
    t.shot("messages-text")?;
    t.press(Action::MNext(1));
    t.press(Action::MNext(2));
    t.press(Action::MSign);
    t.shot("messages-signed")?;
    // The signed message, waiting for the stick, brought back into Files
    // as a stick visit brings it, and checked.
    let signed = t
        .app
        .outbox
        .iter()
        .find(|i| i.name.starts_with("message-") && i.name.ends_with(".txt"))
        .ok_or("no signed message for the stick")?;
    let item = faraday_core::Item {
        name: signed.name.clone(),
        kind: faraday_core::wallet::FileKind::Message,
        bytes: signed.bytes.clone(),
        secret: false,
        picture: None,
        sha: None,
        wallet: None,
        psbt: None,
    };
    faraday_core::testkit::inbox_push(&mut t.app, item);
    let at = t.app.inbox().iter().count() - 1;
    t.press(Action::Nav(Screen::Files));
    t.press(Action::CheckMessage(at));
    t.shot("messages-check")
}
