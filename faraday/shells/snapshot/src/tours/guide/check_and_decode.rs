//! The pictures of `docs/guide/walk/check-and-decode.md`. Its Decode a
//! transaction is drawn where the guide's spend finishes
//! (`spend::spend_from_stick`); the Nonce check page, of a PSBT another
//! signer made, here.

use super::key_session;
use crate::tour::{Tour, type_text};
use faraday_core::decode::CheckPress;
use faraday_core::{Action, Screen};

/// Costs what the tool screens cost, and a nonce check. A fresh stick
/// session with the guide's key typed in: Tools, and the calculators on
/// Hashes; then the nonce check of a PSBT signed elsewhere.
pub(super) fn run(t: &mut Tour) -> Result<(), String> {
    key_session(t);
    t.press(Action::Nav(Screen::Catalog));
    t.shot("check-and-decode-tools")?;
    hashes(t, "check-and-decode-calculators")?;
    nonce_check(t)
}

/// Costs a PSBT read and a nonce check. A fresh stick session: Savings'
/// spend as its cosigner (test key 2) signed it, brought in on a stick,
/// with that key added here and the check opened from Files, so its
/// signature is signed again here and the nonce rule that made it named.
fn nonce_check(t: &mut Tour) -> Result<(), String> {
    let (_, cosigner) = faraday_core::testkit::spend()?;
    crate::tours::screens::stick_session(t);
    super::files_in(t, &[("savings-cosigner.psbt", cosigner.to_bytes())], &[]);
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::Entry(None));
    t.type_key(1);
    t.press(Action::Nav(Screen::Files));
    let k = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == "savings-cosigner.psbt")
        .ok_or("savings-cosigner.psbt is not in Files")?;
    t.press(Action::NonceCheck(CheckPress::Inbox(k)));
    if t.app.screen != Screen::NonceCheck {
        return Err("Nonce check opened no check".into());
    }
    t.shot("nonce-check")
}

/// Costs three hashes. Tools' Hashes calculator opened from Tools, `abc`
/// typed, the shot named `name`.
pub(crate) fn hashes(t: &mut Tour, name: &str) -> Result<(), String> {
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Tools);
    t.press(Action::TTool(0));
    type_text(t, "abc");
    t.shot(name)
}
