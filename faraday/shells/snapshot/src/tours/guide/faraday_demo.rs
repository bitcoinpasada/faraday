//! The pictures of `docs/guide/walk/faraday-demo.md`.

use super::{c32, c32_type};
use crate::tour::{Tour, display};
use faraday_core::codex32::{C32, SPLITS, Split, WAYS, Way as C32Way};
use faraday_core::keygen::Start;
use faraday_core::{Action, Faraday, Host, HostPress, Screen};
use faraday_shell_api::{App, Event};

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
    t.app.clipboard = true;
    t.press(Action::Entry(None));
    host_shot_marking(t, "starter-tour-aids", Action::UseSample)?;
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::KeyGen(None));
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

/// The window as Faraday Demo's host shows it, its strip with the
/// screen's aids at the foot, saved as picture `name`, with the strip's
/// `next` marked beside it as [`Tour::shot_marking`] marks the app's.
fn host_shot_marking(t: &mut Tour, name: &str, next: Action) -> Result<(), String> {
    let win = t.size();
    t.pump();
    let mut app = std::mem::replace(&mut t.app, Faraday::new());
    // Faraday Demo's sticks are its shell's folders: their pills stand
    // in the strip, the boot stick's first.
    let bare = !app.shell_sticks;
    app.shell_sticks = true;
    let mut host = Host::new(app);
    host.event(display(win, t.dpi));
    t.now += 4000;
    host.event(Event::Tick { now_ms: t.now });
    let _ = host.frame();
    host.event(Event::Tick { now_ms: t.now });
    host.app.settle();
    let frame = host.frame();
    let path = t.out.join(format!("{name}.png"));
    crate::tour::write_png(&path, frame.width, frame.height, frame.rgba)?;
    let mark = host.offered_area(HostPress::App(next));
    t.app = host.app;
    if bare {
        t.app.shell_sticks = false;
    }
    t.app.event(display(win, t.dpi));
    t.pump();
    let Some((x, y, w, h)) = mark else {
        use std::io::Write;
        let notes = t.out.join("unmarked.txt");
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&notes)
            .map_err(|e| format!("{}: {e}", notes.display()))?;
        return writeln!(f, "{name}: {next:?} is not on screen").map_err(|e| e.to_string());
    };
    let json = t.out.join(format!("{name}.mark.json"));
    let text = format!("{{\"x\": {x}, \"y\": {y}, \"w\": {w}, \"h\": {h}}}\n");
    std::fs::write(&json, text).map_err(|e| format!("{}: {e}", json.display()))
}
