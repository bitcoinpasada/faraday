//! Faraday Demo's host (`docs/DESKTOP-HOST.md`): the app held by the
//! host, its strip at the window's foot. Home in Practice at the window's
//! own size, the Size menu, the Pi panel and the 7-inch panel previewed;
//! Make codex32 shares in Practice with the computer's walk in the strip and
//! Play's options open, and in the dark theme, whose host is light; then
//! Home on Testnet, whose strip holds only the sticks and Size.

use crate::tour::{Tour, display};
use faraday_core::{Action, Faraday, Host, HostMenu, Screen, StickInfo, StorageEvent, testkit};
use faraday_shell_api::{App as _, Event};

/// The Pi panel's size and the 7-inch panel's, in dp, as the Size menu
/// lists them (`PREVIEW_SIZES`).
const PI: (u16, u16) = (268, 358);
const SEVEN_INCH: (u16, u16) = (964, 565);

/// The app's size in a window `win` at 160 dpi: the window less the
/// host's strip.
fn app_size(win: (u16, u16)) -> (u16, u16) {
    (win.0, win.1.saturating_sub(Host::STRIP_DP as u16))
}

/// The app as Faraday Demo starts it, in Practice or on Testnet, with the
/// boot stick in and Stick 2 out.
fn desktop(t: &mut Tour, win: (u16, u16), demo: bool) {
    t.app = Faraday::new();
    t.app.event(display(app_size(win), t.dpi));
    t.app.shell_sticks = true;
    t.app.online = true;
    t.app.seed_line = true;
    t.app.clipboard = true;
    t.app.set_roller_offered(true);
    t.app.session = faraday_core::wallet::Session::on(testkit::net());
    if demo {
        t.app.session.set_demo(&faraday_samples::SAMPLES);
    }
    t.app.samples = Some(&faraday_samples::SAMPLES);
    t.app.set_modes(faraday_core::DESKTOP_MODES);
    // The scheme `FARADAY_SNAP_THEME` names, as the other tours take it:
    // the app made again here would otherwise start in the first start's.
    if let Ok(words) = std::env::var("FARADAY_SNAP_THEME")
        && let Some(theme) = faraday_core::ui::Theme::from_words(&words)
    {
        t.app.press(Action::Theme(theme));
    }
    if let Ok(words) = std::env::var("FARADAY_SNAP_TEXT")
        && let Some(size) = faraday_core::ui::TextSize::from_id(&words)
    {
        t.app.press(Action::TextSize(size));
    }
    let stick = |name: &str| StickInfo {
        id: name.into(),
        label: name.into(),
        boot: name == "BOOTSTICK",
        serial: None,
        files: Vec::new(),
        vaults: Vec::new(),
    };
    t.app
        .storage(StorageEvent::Sticks(vec![stick("BOOTSTICK")]));
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    t.tick();
    t.tick();
}

/// The window as the host shows it, at `preview` (`None`: the window's
/// own size), with `menu` open at rest, saved as the next picture.
fn shot(
    t: &mut Tour,
    win: (u16, u16),
    name: &str,
    preview: Option<(u16, u16)>,
    menu: Option<HostMenu>,
) -> Result<(), String> {
    t.pump();
    let app = std::mem::replace(&mut t.app, Faraday::new());
    let mut host = Host::new(app);
    host.event(display(win, t.dpi));
    host.set_preview(preview);
    t.now += 4000;
    host.event(Event::Tick { now_ms: t.now });
    let _ = host.frame();
    host.event(Event::Tick { now_ms: t.now });
    host.app.settle();
    let _ = host.frame();
    if let Some(m) = menu {
        host.open_menu_at_rest(m);
    }
    t.n += 1;
    let path = t.out.join(format!("{:02}-{name}.png", t.n));
    let frame = host.frame();
    let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(
        std::io::BufWriter::new(file),
        u32::from(frame.width),
        u32::from(frame.height),
    );
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().map_err(|e| e.to_string())?;
    w.write_image_data(frame.rgba).map_err(|e| e.to_string())?;
    println!("{}", path.display());
    t.app = host.app;
    // At the host's size for the presses that follow.
    t.app.event(display(app_size(win), t.dpi));
    Ok(())
}

pub(crate) fn host_tour(t: &mut Tour) -> Result<(), String> {
    let win = t.size();
    desktop(t, win, true);
    t.press(Action::Nav(Screen::Home));
    shot(t, win, "host-demo-home", None, None)?;
    shot(t, win, "host-size-menu", None, Some(HostMenu::Size))?;
    shot(t, win, "host-samples-sheet", None, Some(HostMenu::Samples))?;
    shot(t, win, "host-pi-panel", Some(PI), None)?;
    shot(
        t,
        win,
        "host-pi-samples-sheet",
        Some(PI),
        Some(HostMenu::Samples),
    )?;
    shot(t, win, "host-7in-panel", Some(SEVEN_INCH), None)?;
    // A key is made with no stick in: the boot stick pulled, as from its
    // pill in the strip.
    t.press(Action::StickPill(1));
    t.app.storage(StorageEvent::Sticks(Vec::new()));
    rollers(t, win)?;
    codex32(t, win)?;
    desktop(t, win, false);
    t.press(Action::Nav(Screen::Home));
    shot(t, win, "host-testnet-home", None, None)?;
    shot(t, win, "host-testnet-pi-panel", Some(PI), None)
}

/// Make codex32 shares in Practice on twenty-faced dice, the computer's walk
/// a few steps in: its Previous, Next, Play and Play's options in the
/// strip, then the menu open; the Pi panel's size too.
fn codex32(t: &mut Tour, win: (u16, u16)) -> Result<(), String> {
    use faraday_core::codex32::{C32, DIE_SIZES, OskPress, Step, WAYS, WalkAct, Way};
    use faraday_core::keygen::{Start, kstep};
    let c = |t: &mut Tour, p: C32| t.press(Action::Codex32(p));
    t.press(Action::Nav(Screen::WalletsStart));
    t.press(Action::KeyGen(None));
    t.press(Action::KStep(kstep::START));
    t.press(Action::KStart(Start::MakeCodex32.index()));
    c(t, C32::Next);
    for ch in "name".chars() {
        c(t, C32::Key(OskPress::Char(ch)));
    }
    c(t, C32::Next);
    let dice = WAYS.iter().position(|w| *w == Way::Dice).unwrap_or(0);
    c(t, C32::Way(dice as u8));
    let d20 = DIE_SIZES.iter().position(|&n| n == 20).unwrap_or(0);
    c(t, C32::Faces(d20 as u8));
    c(t, C32::Next);
    crate::tour::scroll(t, -3000);
    shot(t, win, "host-codex32-ready", None, None)?;
    let step = |t: &Tour| {
        t.app
            .keygen()
            .and_then(|k| k.codex32.as_ref())
            .and_then(|m| m.walk().shown().map(|(_, _, s)| s))
    };
    for _ in 0..80 {
        c(t, C32::Walk(WalkAct::Next));
        t.tick();
        if step(t).is_some_and(|s| matches!(s, Step::Second { .. })) {
            break;
        }
    }
    shot(t, win, "host-codex32-walk", None, None)?;
    // The host in the app's theme turned inside out: a dark app, a light
    // host.
    let was = t.app.theme();
    let dark = faraday_core::ui::Mode::Dark;
    t.press(Action::Theme(faraday_core::ui::Theme { mode: dark, ..was }));
    shot(t, win, "host-codex32-walk-dark", None, None)?;
    shot(t, win, "host-codex32-pi-panel-dark", Some(PI), None)?;
    t.press(Action::Theme(was));
    shot(
        t,
        win,
        "host-codex32-play-options",
        None,
        Some(HostMenu::Play),
    )?;
    shot(t, win, "host-codex32-pi-panel", Some(PI), None)
}

/// The demo's other aids: Add a key's Use a sample and Use faraday, New
/// key's dice entry and Create a vault's Generate with dice with the
/// computer's die in the strip.
fn rollers(t: &mut Tour, win: (u16, u16)) -> Result<(), String> {
    use faraday_core::keygen::Way;
    use faraday_core::vault::VaultAction as V;
    t.press(Action::Entry(None));
    shot(t, win, "host-add-key", None, None)?;
    t.press(Action::KeyGen(None));
    t.press(Action::KWords(12));
    t.press(Action::KWay(Way::DiceHashed.index()));
    t.press(Action::KNext);
    shot(t, win, "host-newkey-dice", None, None)?;
    shot(t, win, "host-newkey-dice-pi-panel", Some(PI), None)?;
    t.press(Action::Nav(Screen::Vaults));
    t.press(Action::Vault(V::Create));
    t.press(Action::Vault(V::Dice(0)));
    shot(t, win, "host-vault-dice", None, None)
}
