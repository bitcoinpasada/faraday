//! The pictures of `docs/guide/walk/faraday-demo.md`.

use super::{c32, c32_type};
use crate::tour::{Tour, display};
use faraday_core::codex32::{C32, SPLITS, Split, WAYS, Way as C32Way};
use faraday_core::keygen::{Start, kstep};
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::App;

/// Faraday Demo as it opens: Practice on testnet, online, no stick in.
pub(crate) fn demo_session(t: &mut Tour) {
    let size = t.size();
    t.app = Faraday::new();
    t.app.event(display(size, t.dpi));
    t.app.online = true;
    t.app.seed_line = true;
    t.app.session = faraday_core::wallet::Session::on(faraday_bip::keys::Network::Testnet);
    t.app.session.set_demo(&faraday_samples::SAMPLES);
    t.app.samples = Some(&faraday_samples::SAMPLES);
    t.app.set_modes(faraday_core::DESKTOP_MODES);
    t.press(Action::Nav(Screen::Home));
}

/// Faraday Demo's Home as it opens, in Practice on testnet, with no
/// stick in; the Mode sheet; Load samples; Add a key with the Demo's
/// aids; then Make codex32 shares in Practice, its entry with Use a
/// sample in the Demo only card (no roller: Practice loads no key of its
/// own), and the sample in place of the strings once it is pressed.
pub(super) fn desktop_home(t: &mut Tour) -> Result<(), String> {
    demo_session(t);
    t.shot("1-11-desktop-home")?;
    t.shot_marking("starter-tour-demo-home", Action::ModeSheet)?;
    t.press(Action::ModeSheet);
    t.shot("starter-tour-mode")?;
    t.press(Action::Cancel);
    t.press(Action::Samples);
    t.shot("starter-tour-samples")?;
    t.press(Action::Cancel);
    t.press(Action::Entry(None));
    t.shot_marking("starter-tour-aids", Action::UseSample)?;
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::KeyGen(None));
    t.press(Action::KStep(kstep::START));
    t.press(Action::KStart(Start::MakeCodex32.index()));
    let two = SPLITS
        .iter()
        .position(|s| *s == Split::TwoOfThree)
        .ok_or("no 2 of 3")?;
    c32(t, C32::Split(two as u8));
    c32(t, C32::Next);
    c32_type(t, "name");
    c32(t, C32::Next);
    let dice = WAYS
        .iter()
        .position(|w| *w == C32Way::Dice)
        .ok_or("no dice")?;
    c32(t, C32::Way(dice as u8));
    c32(t, C32::Next);
    t.shot_marking("1-11-demo-codex32-entry", Action::Codex32(C32::Sample))?;
    c32(t, C32::Sample);
    c32(t, C32::Next);
    t.shot("1-11-demo-codex32-sample")
}
