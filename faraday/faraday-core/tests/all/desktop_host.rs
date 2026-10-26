//! Faraday Demo's host (`docs/DESKTOP-HOST.md`): the app laid out as
//! the device in the window less a strip at its foot, or framed at a
//! preview size; the strip, outside the app, holding the stick pills,
//! Samples, Size and only the demo aids of the screen on show; nothing
//! demo-only or Desktop-only drawn inside the app.

use crate::common::*;
use faraday_core::codex32::{C32, Card, SPLITS, Split, WAYS, WalkAct, Way};
use faraday_core::keygen::{Start, kstep};
use faraday_core::{Action, BOOT_FOLDER, Faraday, Host, HostMenu, HostPress, Screen, testkit};
use faraday_shell_api::{
    App, BootState, Command, DisplayInfo, EntropyBytes, Event, SecureHardware, TouchPhase,
};
use faraday_ui::keyboard::OskPress;

/// The Pi panel's size in dp, as the Size menu lists it.
const PI: (u16, u16) = (268, 358);

fn display(w: u16, h: u16) -> Event {
    Event::Display(DisplayInfo {
        width: w,
        height: h,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    })
}

/// Faraday Demo as it starts, in Practice with its samples or on Testnet,
/// with `sticks` in (the boot stick, Stick 2 and on), held by the host
/// in a window `w` × `h`, on Home, a frame drawn.
fn desktop(shell: &mut Shell, sticks: usize, demo: bool, (w, h): (u16, u16)) -> Host {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    app.shell_sticks = true;
    app.online = true;
    app.set_roller_offered(true);
    if demo {
        testkit::into_demo(&mut app);
    } else {
        testkit::move_to(&mut app.session, testkit::NET).expect("testnet");
    }
    app.samples = Some(testkit::demo_samples());
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeKeep);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x31 + i * 7; 32])));
    }
    app.press(Action::ReduceMotion(true));
    for n in 1..=sticks {
        let folder = if n == 1 {
            BOOT_FOLDER.to_string()
        } else {
            format!("STICK-{n}")
        };
        let mut d = Disk::empty(&folder, &folder);
        d.boot = n == 1;
        shell.add(d);
        shell.plug(&mut app, &folder);
    }
    let mut host = Host::new(app);
    host.event(display(w, h));
    host.app.press(Action::Nav(Screen::Home));
    host.app.settle();
    let _ = host.frame();
    host
}

/// A press on the host's control `p` where the last frame drew it, as a
/// pointer would, and a frame after.
fn click(host: &mut Host, p: HostPress) {
    let _ = host.frame();
    let (x, y) = host
        .where_offered(p)
        .unwrap_or_else(|| panic!("{p:?} is not in the strip"));
    tap_at(host, x, y);
}

fn tap_at(host: &mut Host, x: u16, y: u16) {
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        host.event(Event::Touch { x, y, phase });
    }
    let _ = host.frame();
}

/// The shell of the desktop app, in little: a folder pressed is plugged
/// in when it is out and pulled when it is in.
fn shell_follows(shell: &mut Shell, app: &mut Faraday) {
    for folder in app.take_stick_toggles() {
        if shell.present.contains(&folder) {
            shell.pull(app, &folder);
        } else {
            if !shell.disks.iter().any(|d| d.id == folder) {
                shell.add(Disk::empty(&folder, &folder));
            }
            shell.plug(app, &folder);
        }
    }
}

/// What the strip says now.
fn strip(host: &mut Host) -> Vec<String> {
    let _ = host.frame();
    host.strip_texts().to_vec()
}

/// What the app draws inside its area now.
fn app_texts(host: &mut Host) -> Vec<String> {
    let _ = host.frame();
    host.app.drawn_texts()
}

/// Nothing only Faraday Demo or Practice has is drawn inside the app.
fn app_holds_no_host_item(host: &mut Host) {
    let texts = app_texts(host);
    for t in [
        "Demo only",
        "Load samples",
        "Samples",
        "Use a sample",
        "Use faraday",
        "Boot stick · in",
        "Boot stick · out",
        "Stick 2 · in",
        "Stick 2 · out",
        "Preview size",
        "Play options",
    ] {
        assert!(
            !texts.iter().any(|x| x == t),
            "the app area draws {t:?}: {texts:?}"
        );
    }
}

#[test]
fn in_demo_the_strip_holds_the_sticks_samples_and_size_and_the_app_none_of_them() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 1, true, (1280, 800));
    let s = strip(&mut host);
    for t in [
        "Boot stick · in",
        "Stick 2 · out",
        "Samples",
        "Size · This window",
    ] {
        assert!(s.iter().any(|x| x == t), "the strip offers {t:?}: {s:?}");
    }
    // Home offers no demo aid.
    assert!(!s.iter().any(|x| x == "Practice"), "no aid on Home: {s:?}");
    // The app is the window less the strip, at its top left; the mode
    // pill stays in it, as the device has it.
    assert_eq!(host.app_area(), Some((0, 0, 1280, 800 - 56)));
    assert!(
        app_texts(&mut host)
            .iter()
            .any(|t| t == "Practice · Testnet")
    );
    app_holds_no_host_item(&mut host);
    // Samples opens the samples sheet, the host's.
    click(&mut host, HostPress::Menu(HostMenu::Samples));
    assert_eq!(host.menu_open(), Some(HostMenu::Samples));
    assert_eq!(host.app.sheet, None);
}

#[test]
fn on_testnet_the_strip_holds_the_sticks_and_size_alone() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 1, false, (1280, 800));
    let s = strip(&mut host);
    assert!(s.iter().any(|x| x == "Boot stick · in"), "{s:?}");
    assert!(s.iter().any(|x| x == "Size · This window"), "{s:?}");
    assert!(
        !s.iter().any(|x| x == "Samples" || x == "Practice"),
        "{s:?}"
    );
    // Not on Add a key either, which offers samples in Practice.
    host.app.press(Action::Entry(None));
    let s = strip(&mut host);
    assert!(
        !s.iter().any(|x| x == "Practice" || x == "Use a sample"),
        "no practice aid off Practice: {s:?}"
    );
    app_holds_no_host_item(&mut host);
}

#[test]
fn a_stick_is_pulled_and_put_back_from_its_pill_and_from_the_sticks_menu() {
    let mut shell = Shell::default();
    // Five sticks in a narrow window: the rest fold into Sticks · N.
    let mut host = desktop(&mut shell, 5, true, (640, 600));
    let s = strip(&mut host);
    assert!(s.iter().any(|x| x == "Boot stick · in"), "{s:?}");
    assert!(s.iter().any(|x| x.starts_with("Sticks · ")), "{s:?}");
    // The boot stick's pill pulls it.
    click(&mut host, HostPress::App(Action::StickPill(1)));
    shell_follows(&mut shell, &mut host.app);
    assert!(!shell.present.iter().any(|f| f == BOOT_FOLDER));
    assert!(strip(&mut host).iter().any(|x| x == "Boot stick · out"));
    // Stick 4, from the menu: pulled, the menu closed; then put back.
    click(&mut host, HostPress::Menu(HostMenu::Sticks));
    assert_eq!(host.menu_open(), Some(HostMenu::Sticks));
    assert!(strip(&mut host).iter().any(|x| x == "Stick 4 · in"));
    click(&mut host, HostPress::App(Action::StickPill(4)));
    shell_follows(&mut shell, &mut host.app);
    assert_eq!(host.menu_open(), None);
    assert!(!shell.present.iter().any(|f| f == "STICK-4"));
    click(&mut host, HostPress::Menu(HostMenu::Sticks));
    assert!(strip(&mut host).iter().any(|x| x == "Stick 4 · out"));
    click(&mut host, HostPress::App(Action::StickPill(4)));
    shell_follows(&mut shell, &mut host.app);
    assert!(shell.present.iter().any(|f| f == "STICK-4"));
}

#[test]
fn size_shows_the_pi_panel_framed_above_the_strip_and_a_press_inside_reaches_it() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 1, true, (1280, 800));
    click(&mut host, HostPress::Menu(HostMenu::Size));
    assert!(strip(&mut host).iter().any(|x| x == "Pi panel"));
    click(&mut host, HostPress::Size(Some(PI)));
    // The shell keeps the size across a lock.
    let mut asked = None;
    while let Some(c) = host.poll_command() {
        if let Command::PreviewSize { size } = c {
            asked = Some(size);
        }
    }
    assert_eq!(asked, Some(Some(PI)));
    assert!(strip(&mut host).iter().any(|x| x == "Size · Pi panel"));
    // Laid out as the Pi's panel: compact, short, Home's menu.
    host.app.press(Action::Nav(Screen::Home));
    let _ = host.frame();
    assert!(host.app.is_compact() && host.app.window().short());
    assert_eq!(host.app.nav(), faraday_core::nav::Nav::Menu);
    // The frame centred in the space above the strip, never in it.
    let (x, y, w, h) = host.app_area().expect("the frame");
    assert_eq!((w, h), (268, 358));
    assert_eq!(x, (1280 - 268) / 2);
    assert!(y + h <= 800 - 56, "above the strip: {y} + {h}");
    app_holds_no_host_item(&mut host);
    // Vaults' tile, pressed where the window shows it.
    let (fx, fy) = host
        .app
        .where_offered(Action::Nav(Screen::Vaults))
        .expect("Home's menu offers Vaults");
    tap_at(&mut host, x as u16 + fx, y as u16 + fy);
    assert_eq!(host.app.screen, Screen::Vaults);
    // A press beside the frame reaches nothing.
    tap_at(&mut host, 10, 10);
    assert_eq!(host.app.screen, Screen::Vaults);
    // Back to the window's own size.
    click(&mut host, HostPress::Menu(HostMenu::Size));
    click(&mut host, HostPress::Size(None));
    assert_eq!(host.app_area(), Some((0, 0, 1280, 800 - 56)));
}

#[test]
fn a_menu_closes_on_a_press_beside_it_and_does_nothing_else() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 1, true, (1280, 800));
    click(&mut host, HostPress::Menu(HostMenu::Size));
    assert_eq!(host.menu_open(), Some(HostMenu::Size));
    // On Home's first tile, under no menu row.
    let (fx, fy) = host
        .app
        .where_offered(Action::Nav(Screen::Vaults))
        .or_else(|| host.app.where_offered(Action::CreateWallet))
        .expect("a tile");
    tap_at(&mut host, fx, fy.min(40));
    assert_eq!(host.menu_open(), None);
    assert_eq!(host.app.screen, Screen::Home, "the press went no further");
}

#[test]
fn add_a_key_offers_use_a_sample_and_use_faraday_in_the_strip_alone() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 0, true, (1280, 800));
    host.app.press(Action::Entry(None));
    let s = strip(&mut host);
    for t in ["Practice", "Use a sample", "Use faraday"] {
        assert!(s.iter().any(|x| x == t), "the strip offers {t:?}: {s:?}");
    }
    app_holds_no_host_item(&mut host);
    // Use a sample, pressed in the strip, fills the words.
    assert!(app_texts(&mut host).iter().any(|t| t == "0 words"));
    click(&mut host, HostPress::App(Action::UseSample));
    let texts = app_texts(&mut host);
    assert!(
        !texts.iter().any(|t| t == "0 words"),
        "the words filled: {texts:?}"
    );
    // Back on Home the aids are gone.
    host.app.press(Action::Nav(Screen::Home));
    let s = strip(&mut host);
    assert!(
        !s.iter().any(|x| x == "Use a sample" || x == "Practice"),
        "{s:?}"
    );
}

/// Make codex32 shares, 2 of 3 named `name`, on the entry by dice.
fn on_codex32_dice(host: &mut Host) {
    let c = |host: &mut Host, p: C32| host.app.press(Action::Codex32(p));
    host.app.press(Action::Nav(Screen::WalletsStart));
    host.app.press(Action::KeyGen(None));
    host.app.press(Action::KStep(kstep::START));
    host.app.press(Action::KStart(Start::MakeCodex32.index()));
    let split = SPLITS.iter().position(|s| *s == Split::TwoOfThree).unwrap();
    c(host, C32::Split(split as u8));
    c(host, C32::Next);
    for ch in "name".chars() {
        c(host, C32::Key(OskPress::Char(ch)));
    }
    c(host, C32::Next);
    let w = WAYS.iter().position(|w| *w == Way::Dice).unwrap();
    c(host, C32::Way(w as u8));
    c(host, C32::Next);
    let m = host.app.keygen().unwrap().codex32.as_ref().unwrap();
    assert_eq!(m.open(), Some(Card::Enter));
    let _ = host.frame();
}

#[test]
fn the_codex32_walk_stands_in_the_strip_and_next_steps_it_from_one_place() {
    let next = HostPress::App(Action::Codex32(C32::Walk(WalkAct::Next)));
    for (w, h) in [(1280u16, 800u16), (1920, 1080), (960, 580)] {
        let mut shell = Shell::default();
        let mut host = desktop(&mut shell, 0, true, (w, h));
        on_codex32_dice(&mut host);
        let s = strip(&mut host);
        for t in ["Practice", "Use a sample", "Previous", "Next", "Play"] {
            assert!(
                s.iter().any(|x| x == t),
                "{w}x{h}: the strip offers {t:?}: {s:?}"
            );
        }
        app_holds_no_host_item(&mut host);
        let at = host.where_offered(next).expect("Next in the strip");
        let (_, ay, _, ah) = host.app_area().unwrap();
        assert!(i32::from(at.1) > ay + ah, "{w}x{h}: Next under the app");
        // Next steps the walk; the step's count and working are the
        // Working area's, in the app; Next keeps its place.
        for i in 0..20 {
            click(&mut host, next);
            if i == 0 {
                let texts = app_texts(&mut host);
                assert!(
                    texts.iter().any(|t| t.starts_with("Step 1 of ")),
                    "{w}x{h}: the Working area's count: {texts:?}"
                );
            }
            assert_eq!(
                host.where_offered(next),
                Some(at),
                "{w}x{h}: Next moved at {i}"
            );
        }
        let m = host.app.keygen().unwrap().codex32.as_ref().unwrap();
        assert!(m.walk().shown().is_some(), "{w}x{h}: the walk stepped");
    }
}

#[test]
fn plays_options_open_from_the_strip_and_take_a_choice() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 0, true, (1280, 800));
    on_codex32_dice(&mut host);
    click(&mut host, HostPress::Menu(HostMenu::Play));
    let s = strip(&mut host);
    for t in ["Play runs", "Speed", "Fastest"] {
        assert!(s.iter().any(|x| x == t), "the menu lists {t:?}: {s:?}");
    }
    let fastest = HostPress::App(Action::Codex32(C32::Walk(WalkAct::Speed(3))));
    click(&mut host, fastest);
    assert_eq!(host.menu_open(), None);
    let m = host.app.keygen().unwrap().codex32.as_ref().unwrap();
    assert_eq!(
        m.walk().speed(),
        faraday_core::codex32::SPEEDS[3],
        "the speed chosen"
    );
}

#[test]
fn the_dice_roller_stands_in_the_strip_on_new_keys_dice_and_rolls_from_there() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 0, true, (1280, 800));
    host.app.press(Action::KeyGen(None));
    host.app.press(Action::KWords(12));
    host.app
        .press(Action::KWay(faraday_core::keygen::Way::DiceHashed.index()));
    host.app.press(Action::KNext);
    let roll = HostPress::App(Action::KRoller(faraday_core::vault::RollerAct::Roll));
    let _ = host.frame();
    let at = host.where_offered(roll).expect("Roll in the strip");
    let (_, ay, _, ah) = host.app_area().unwrap();
    assert!(i32::from(at.1) > ay + ah, "under the app: {at:?}");
    // The strip grows to the roller's row; the app is laid out above it.
    assert!(ah < 800 - 56, "the strip taller than 56 dp: {ah}");
    app_holds_no_host_item(&mut host);
    let texts = app_texts(&mut host);
    assert!(
        !texts.iter().any(|t| t == "Roll"),
        "no roller in the app: {texts:?}"
    );
    let before = host.app.keygen().unwrap().dice.len();
    click(&mut host, roll);
    for t in 1..200u64 {
        host.event(Event::Tick { now_ms: t * 50 });
    }
    host.app.settle();
    let after = host.app.keygen().unwrap().dice.len();
    assert!(after > before, "Roll rolled: {before} then {after}");
}

#[test]
fn generate_with_dice_has_its_roller_in_the_strip() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 0, true, (1280, 800));
    host.app.press(Action::Nav(Screen::Vaults));
    host.app
        .press(Action::Vault(faraday_core::vault::VaultAction::Create));
    host.app
        .press(Action::Vault(faraday_core::vault::VaultAction::Dice(0)));
    let _ = host.frame();
    let roll = HostPress::App(Action::Vault(faraday_core::vault::VaultAction::Roller(
        faraday_core::vault::RollerAct::Roll,
    )));
    assert!(host.where_offered(roll).is_some(), "Roll in the strip");
    let s = strip(&mut host);
    assert!(s.iter().any(|x| x == "Use faraday"), "{s:?}");
    app_holds_no_host_item(&mut host);
}

#[test]
fn held_bare_the_app_takes_the_whole_window_with_no_strip() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    app.shell_sticks = true;
    let mut host = Host::bare(app);
    host.event(display(800, 600));
    let _ = host.frame();
    assert_eq!(host.app_area(), Some((0, 0, 800, 600)));
    assert!(host.strip_texts().is_empty());
}

#[test]
fn at_the_narrowest_every_control_stands_inside_the_window_and_samples_is_in_the_demo_menu() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 2, true, (268, 600));
    host.app.press(Action::Entry(None));
    let s = strip(&mut host);
    for t in ["Sticks", "Size", "Practice"] {
        assert!(s.iter().any(|x| x == t), "the strip offers {t:?}: {s:?}");
    }
    for p in [
        HostPress::Menu(HostMenu::Sticks),
        HostPress::Menu(HostMenu::Size),
        HostPress::Menu(HostMenu::More),
    ] {
        let (x, _) = host.where_offered(p).expect("in the strip");
        assert!(x < 268, "{p:?} inside the window at {x}");
    }
    click(&mut host, HostPress::Menu(HostMenu::More));
    click(&mut host, HostPress::Menu(HostMenu::Samples));
    assert_eq!(host.menu_open(), Some(HostMenu::Samples));
    // At 268 dp the sheet keeps inside the window, over the app's foot.
    let (x, _, w, _) = host.sheet_area().expect("the sheet is drawn");
    assert!(x >= 0 && x + w <= 268, "the sheet at {x}, {w} wide");
}

#[test]
fn the_dies_rolls_per_click_menu_chooses_how_many_a_roll_makes() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 0, true, (1280, 800));
    host.app.press(Action::KeyGen(None));
    host.app.press(Action::KWords(12));
    host.app
        .press(Action::KWay(faraday_core::keygen::Way::DiceHashed.index()));
    host.app.press(Action::KNext);
    let _ = host.frame();
    assert!(strip(&mut host).iter().any(|x| x == "Rolls per click · 1"));
    click(&mut host, HostPress::Menu(HostMenu::Rolls));
    let five = HostPress::App(Action::KRoller(faraday_core::vault::RollerAct::Per(
        faraday_core::vault::RollerPerClick::Five,
    )));
    click(&mut host, five);
    assert_eq!(host.menu_open(), None);
    assert!(strip(&mut host).iter().any(|x| x == "Rolls per click · 5"));
    click(
        &mut host,
        HostPress::App(Action::KRoller(faraday_core::vault::RollerAct::Roll)),
    );
    host.app.settle();
    assert_eq!(
        host.app.keygen().unwrap().dice.len(),
        5,
        "one click, five rolls"
    );
}

/// Whether the rectangles `a` and `b`, each left, top, width and height,
/// overlap.
fn overlap(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    a.0 < b.0 + b.2 && b.0 < a.0 + a.2 && a.1 < b.1 + b.3 && b.1 < a.1 + a.3
}

/// A tick on the sample key at `i` on Samples' sheet.
fn tick_key(i: u8) -> HostPress {
    HostPress::App(Action::SampleTick(faraday_core::samples::Pick::Key(i)))
}

/// The first sample key Samples' sheet offers to tick: one not loaded.
fn first_offered(host: &mut Host) -> u8 {
    let _ = host.frame();
    (0..32)
        .find(|i| host.where_offered(tick_key(*i)).is_some())
        .unwrap_or_else(|| panic!("no sample key to tick: {:?}", host.strip_texts()))
}

#[test]
fn samples_opens_the_hosts_sheet_above_the_strip_and_what_is_ticked_loads() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 0, true, (1280, 800));
    click(&mut host, HostPress::Menu(HostMenu::Samples));
    // The sheet is the host's: the app opens none and draws none of it.
    assert_eq!(host.menu_open(), Some(HostMenu::Samples));
    assert_eq!(host.app.sheet, None);
    let s = strip(&mut host);
    for t in [
        "Load samples",
        "Wallets",
        "Keys",
        "Select all",
        "Not now",
        "Load ticked",
    ] {
        assert!(s.iter().any(|x| x == t), "the sheet offers {t:?}: {s:?}");
    }
    let texts = app_texts(&mut host);
    assert!(
        !texts.iter().any(|t| t == "Load ticked" || t == "Not now"),
        "the app draws the sheet: {texts:?}"
    );
    // It stands above the strip, in This window over the app's foot.
    let (_, y, _, h) = host.sheet_area().expect("the sheet is drawn");
    assert!(y >= 0 && y + h <= 800 - 56, "the sheet at {y}, {h} tall");
    // A tick keeps it open; Load ticked loads the key and closes it.
    let keys = host.app.session.keys.len();
    let i = first_offered(&mut host);
    click(&mut host, tick_key(i));
    assert_eq!(host.menu_open(), Some(HostMenu::Samples));
    assert!(strip(&mut host).iter().any(|x| x == "Unselect all"));
    click(&mut host, HostPress::App(Action::SamplesLoad));
    assert_eq!(host.menu_open(), None);
    assert_eq!(host.app.session.keys.len(), keys + 1);
    // Opened again, the loaded key stands ticked and takes no press.
    click(&mut host, HostPress::Menu(HostMenu::Samples));
    assert_eq!(host.where_offered(tick_key(i)), None);
}

#[test]
fn samples_with_a_stick_in_asks_for_it_out_in_the_app_as_before() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 1, true, (1280, 800));
    click(&mut host, HostPress::Menu(HostMenu::Samples));
    let s = strip(&mut host);
    assert!(
        s.iter().any(|x| x.starts_with("Pull the ")),
        "the sheet says the stick must be out: {s:?}"
    );
    let i = first_offered(&mut host);
    click(&mut host, tick_key(i));
    click(&mut host, HostPress::App(Action::SamplesLoad));
    assert_eq!(host.menu_open(), None);
    assert_eq!(host.app.sheet, Some(faraday_core::Sheet::Pull));
}

#[test]
fn in_the_pi_preview_samples_sheet_stands_beside_the_frame_and_a_press_beside_it_closes_it() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 0, true, (1280, 800));
    click(&mut host, HostPress::Menu(HostMenu::Size));
    click(&mut host, HostPress::Size(Some(PI)));
    click(&mut host, HostPress::Menu(HostMenu::Samples));
    let frame = host.app_area().expect("the frame is shown");
    let sheet = host.sheet_area().expect("the sheet is drawn");
    assert!(
        !overlap(frame, sheet),
        "the sheet {sheet:?} stands over the frame {frame:?}"
    );
    assert!(sheet.1 + sheet.3 <= 800 - 56, "above the strip: {sheet:?}");
    // A press on the sheet itself keeps it; one beside it, on the
    // frame, closes it and reaches nothing.
    let (x, y) = (sheet.0 + sheet.2 / 2, sheet.1 + 4);
    tap_at(&mut host, x as u16, y as u16);
    assert_eq!(host.menu_open(), Some(HostMenu::Samples));
    let (fx, fy) = (frame.0 + frame.2 / 2, frame.1 + frame.3 / 2);
    tap_at(&mut host, fx as u16, fy as u16);
    assert_eq!(host.menu_open(), None);
    assert_eq!(host.app.screen, Screen::Home);
    assert_eq!(host.app.sheet, None);
}

/// The window's pixels as a shell shows `parts`: the base, the inner
/// frame copied over it where it stands.
fn shown(parts: &faraday_shell_api::Parts<'_>) -> Vec<u8> {
    let mut out = parts.base.rgba.to_vec();
    if let Some((f, x, y)) = parts.inner {
        let (bw, fw) = (usize::from(parts.base.width), usize::from(f.width));
        for r in 0..usize::from(f.height) {
            let at = ((usize::from(y) + r) * bw + usize::from(x)) * 4;
            out[at..at + fw * 4].copy_from_slice(&f.rgba[r * fw * 4..(r + 1) * fw * 4]);
        }
    }
    out
}

/// The window as the shell shows the host's parts, whether the app's
/// frame was handed over unplaced, and the window as the host's whole
/// frame shows it.
fn both(host: &mut Host) -> (Vec<u8>, bool, Vec<u8>) {
    let (parts, handed) = {
        let p = host.parts();
        (shown(&p), p.inner.is_some())
    };
    (parts, handed, host.frame().rgba.to_vec())
}

#[test]
fn the_window_shows_the_same_with_the_apps_frame_copied_into_it_once() {
    for size in [(1366, 768), (1280, 800)] {
        let mut shell = Shell::default();
        let mut host = desktop(&mut shell, 2, true, size);
        host.app.press(Action::Entry(None));
        // This window: the app's frame handed over, and the window the
        // same as the host's whole frame.
        let (parts, handed, whole) = both(&mut host);
        assert!(handed, "at {size:?} the app's frame is copied once");
        assert!(parts == whole, "at {size:?} the window differs");
        // A menu stands over the app: the frame is whole.
        click(&mut host, HostPress::Menu(HostMenu::Size));
        let (parts, handed, whole) = both(&mut host);
        assert!(!handed && parts == whole, "with the Size menu at {size:?}");
        // The Pi panel, framed in the dark space: its small frame placed,
        // the window the same.
        click(&mut host, HostPress::Size(Some(PI)));
        let (parts, _, whole) = both(&mut host);
        assert!(parts == whole, "the Pi preview differs at {size:?}");
        // This window again: handed over.
        click(&mut host, HostPress::Menu(HostMenu::Size));
        click(&mut host, HostPress::Size(None));
        let (parts, handed, whole) = both(&mut host);
        assert!(handed && parts == whole, "This window again at {size:?}");
    }
}

#[test]
fn the_strips_more_rows_are_offered_at_the_clocks_very_start() {
    // Scripted alone, motion on, the clock at its start: More, opened at
    // rest to read its rows, is whole however early the clock is.
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    app.shell_sticks = true;
    testkit::into_demo(&mut app);
    app.samples = Some(testkit::demo_samples());
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeKeep);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x31 + i * 7; 32])));
    }
    app.event(display(268, 600));
    app.press(Action::Entry(None));
    let _ = app.frame();
    assert!(app.now_ms() < 300, "the clock at {}", app.now_ms());
    let offered = Host::strip_actions_for(&mut app);
    for a in [Action::UseSample, Action::Samples] {
        assert!(offered.contains(&a), "More offers {a:?}: {offered:?}");
    }
}
