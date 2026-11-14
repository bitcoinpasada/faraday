//! Renders Faraday's screens to PNG along one scripted tour.
//!
//! ```text
//! faraday-snapshot [--sd-card] [--network mainnet] WxH[@DPI] TESTKIT_DIR OUT_DIR [TOUR]
//! ```
//!
//! `--sd-card` runs the tour as the Pi's stick shell starts the app:
//! with the removable medium an SD card (`Medium::SdCard`), so every
//! screen names it as the Pi does.
//!
//! `--network mainnet` runs the tour as a Mainnet stick shows it (no
//! network pill, `bc1` addresses, `xpub`s): the session stays on
//! Mainnet and the test wallets the tour makes in process are made on
//! it (`faraday_core::testkit::use_network`); `TESTKIT_DIR` must then be
//! what `faraday-testkit --network mainnet` wrote. Without it, both are
//! on testnet. The `modes` tour's Testnet screens stay on testnet.
//!
//! `TESTKIT_DIR` is what `faraday-testkit` wrote; its files stand in
//! for a stick called TESTSTICK. Each step of the tour is one numbered
//! PNG in `OUT_DIR`. `TOUR` is one of the names in `tours::TOURS` (the
//! usage line prints them; each tour's file says what it shows); without
//! one, the default tour runs.
//!
//! `@DPI` defaults to 160 (a desktop monitor); a real panel must give
//! its own, since the core picks `small`/`medium`/`wide` from physical
//! width, not pixels. The Pi's Waveshare panel is `480x640@286`; the
//! smallest panel the design supports is `240x320@143`.

mod leaks;
mod targets;
mod tour;
mod tours;

use crate::tour::{Small, Tour, display};
use faraday_core::{Action, Faraday, Medium};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Orientation};
use std::path::Path;
use std::process::ExitCode;

/// The panel's own orientation where `size` at `dpi` is one of the
/// stick's 7-inch or 10-inch panels, either way round: the 7-inch
/// 1024 × 600 at 170 dpi, landscape natively, and the 10-inch 800 × 1280
/// at 150 dpi, portrait natively.
fn panel_native(size: (u16, u16), dpi: u16) -> Option<Orientation> {
    match (size, dpi) {
        ((1024, 600) | (600, 1024), 170) => Some(Orientation::Landscape),
        ((1280, 800) | (800, 1280), 150) => Some(Orientation::Portrait),
        _ => None,
    }
}

/// Offers Rotation as the stick does, on a 7-inch or 10-inch panel's
/// render.
fn offer_rotation(app: &mut Faraday, size: (u16, u16), dpi: u16) {
    // A 7-inch or 10-inch panel's render (`docs/ORIENTATION.md` §10):
    // the stick offers Rotation there, chosen the way this render
    // stands. The render is already that size: the turn the app asks
    // for is left unanswered.
    if let Some(native) = panel_native(size, dpi) {
        app.event(faraday_shell_api::Event::RotationOffered { native });
        if size.1 > size.0 {
            app.press(Action::Rotation(Orientation::Portrait));
        }
    }
}

/// Costs what the tour it runs costs.
fn run(
    c: &Compute,
    size: (u16, u16),
    dpi: u16,
    (kit, out): (&Path, &Path),
    only: Option<&str>,
    medium: Medium,
    pictures: bool,
) -> Result<Vec<Small>, String> {
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut app = Faraday::new();
    app.medium = medium;
    app.event(display(size, dpi));
    offer_rotation(&mut app, size, dpi);
    // The session goes to the test wallets' network (testnet unless
    // `--network` chose another), since a session on one network takes
    // none of another's wallets once a key is loaded.
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::net())
        .map_err(|e| String::from(&*e.text()))?;
    // `FARADAY_SNAP_THEME=<theme>[,<mode>[,<contrast>]]` takes the tour
    // in that scheme ("frost,dark,high"; a stored id alone, "nord",
    // opens as a settings file of it would) instead of the first start's.
    if let Ok(words) = std::env::var("FARADAY_SNAP_THEME")
        && let Some(theme) = faraday_core::ui::Theme::from_words(&words)
    {
        app.press(Action::Theme(theme));
    }
    // `FARADAY_SNAP_TEXT=large|larger` takes the tour at that text size
    // (`docs/TEXT-SIZE.md` §7), and each frame with text cut off at a
    // side or drawn over other text is listed in `cut.txt` beside the
    // frames.
    let text = std::env::var("FARADAY_SNAP_TEXT").ok();
    if let Some(words) = &text {
        let size = faraday_core::ui::TextSize::from_id(words)
            .ok_or_else(|| format!("FARADAY_SNAP_TEXT={words}: standard, large or larger"))?;
        app.press(Action::TextSize(size));
        let cut = out.join("cut.txt");
        std::fs::write(&cut, "").map_err(|e| format!("{}: {e}", cut.display()))?;
    }
    let mut t = Tour {
        app,
        out: out.to_path_buf(),
        kit: kit.to_path_buf(),
        n: 0,
        now: 0,
        sticks: Vec::new(),
        answers: 0,
        dpi,
        host: None,
        boxes: None,
        go_on: true,
        pictures,
        targets: Some(Vec::new()),
        name: only.unwrap_or("default").to_string(),
        restarts: 0,
        leaks: Vec::new(),
        opened: leaks::Opened::default(),
    };
    leaks::plant();
    match only.and_then(|name| tours::run_named(c, &mut t, name)) {
        Some(done) => done,
        None => tours::default::default_tour(c, &mut t, medium),
    }?;
    if !t.leaks.is_empty() {
        return Err(format!(
            "a secret is left after Lock, a hide or leaving a page (docs/SECRET-LEAKS.md §5):\n  {}",
            t.leaks.join("\n  ")
        ));
    }
    Ok(t.targets.take().unwrap_or_default())
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().collect();
    let medium = match args.iter().position(|a| a == "--sd-card") {
        Some(i) => {
            args.remove(i);
            Medium::SdCard
        }
        None => Medium::Stick,
    };
    if let Some(i) = args.iter().position(|a| a == "--network") {
        let Some(network) = args
            .get(i + 1)
            .and_then(|n| faraday_bip::keys::Network::from_name(n))
        else {
            eprintln!("--network takes mainnet, testnet, signet or regtest");
            return ExitCode::from(2);
        };
        args.drain(i..=i + 1);
        if faraday_core::testkit::use_network(network).is_err() {
            return ExitCode::FAILURE;
        }
    }
    if !(4..=5).contains(&args.len()) {
        eprintln!(
            "usage: faraday-snapshot [--sd-card] [--network NAME] WxH[@DPI] TESTKIT_DIR OUT_DIR [{}]",
            tours::names()
        );
        return ExitCode::from(2);
    }
    let (size_arg, dpi_arg) = match args[1].split_once('@') {
        Some((s, d)) => (s, Some(d)),
        None => (args[1].as_str(), None),
    };
    let Some((w, h)) = size_arg
        .split_once('x')
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
    else {
        eprintln!("size must be WxH[@DPI]");
        return ExitCode::from(2);
    };
    // 160 is a desktop monitor's rough density; a real panel (the Pi's
    // 480x640 at 286 dpi, or the 240x320 minimum at 143) must say its
    // own, because the core picks its size class from physical width,
    // not from pixels.
    let dpi: u16 = match dpi_arg.map(str::parse) {
        Some(Ok(d)) => d,
        Some(Err(_)) => {
            eprintln!("dpi must be a number");
            return ExitCode::from(2);
        }
        None => 160,
    };
    // The tool's token, for the costly work it does outside the app's
    // events once that work takes one (docs/COMPUTE.md §3). A tool makes
    // no drawing claim, so it counts nothing.
    #[expect(
        clippy::disallowed_methods,
        reason = "a mint: the snapshot tool (docs/COMPUTE.md §3)"
    )]
    let compute = faraday_crypto::work::Compute::new(|_| {});
    let only = args.get(4).map(String::as_str);
    if only.is_some_and(|m| !tours::TOURS.iter().any(|(n, _)| *n == m)) {
        eprintln!("the tour is {}", tours::names().replace('|', ", "));
        return ExitCode::from(2);
    }
    match run(
        &compute,
        (w, h),
        dpi,
        (Path::new(&args[2]), Path::new(&args[3])),
        only,
        medium,
        true,
    ) {
        Ok(small) => match targets::write(Path::new(&args[3]), only, &small) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("faraday-snapshot: {e}");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            eprintln!("faraday-snapshot: {e}");
            ExitCode::FAILURE
        }
    }
}
