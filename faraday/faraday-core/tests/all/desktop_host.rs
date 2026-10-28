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

/// The strip's height at rest in the tests' windows, 160 dpi: a pixel
/// a dp.
const STRIP: i32 = faraday_draw::tokens::HOST_STRIP_H as i32;

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
    assert_eq!(host.app_area(), Some((0, 0, 1280, 800 - STRIP)));
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
    assert!(y + h <= 800 - STRIP, "above the strip: {y} + {h}");
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
    assert_eq!(host.app_area(), Some((0, 0, 1280, 800 - STRIP)));
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
    assert!(
        ah < 800 - STRIP,
        "the strip grown past its height at rest: {ah}"
    );
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
    assert!(y >= 0 && y + h <= 800 - STRIP, "the sheet at {y}, {h} tall");
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
    assert!(
        sheet.1 + sheet.3 <= 800 - STRIP,
        "above the strip: {sheet:?}"
    );
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
    let (bw, by) = (usize::from(parts.base.width), usize::from(parts.base_y));
    let bh = usize::from(parts.base.height);
    let mut out = vec![0; bw * (by + bh) * 4];
    out[by * bw * 4..].copy_from_slice(parts.base.rgba);
    if let Some((f, x, y)) = parts.inner {
        let fw = usize::from(f.width);
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

/// The host's text, in the colour it was drawn in, against the pixels
/// round it, for each line the frame now drawn writes at full strength.
fn host_reads(host: &mut Host) -> Vec<(String, f64)> {
    use super::fills_read::{over, ratio, under};
    use faraday_draw::canvas::InkKind;
    let (width, height, rgba) = {
        let f = host.frame();
        (i32::from(f.width), i32::from(f.height), f.rgba.to_vec())
    };
    host.ink()
        .iter()
        .filter_map(|i| match &i.kind {
            InkKind::Text { text, color, .. } if color.a == 255 => {
                let r = (i.rect.x, i.rect.y, i.rect.w, i.rect.h);
                let bg = under(&rgba, width, height, r);
                Some((text.clone(), ratio(over(*color, bg), bg)))
            }
            _ => None,
        })
        .collect()
}

/// The colour of the window's pixel (`x`, `y`) in the frame now drawn.
fn pixel(host: &mut Host, x: i32, y: i32) -> faraday_draw::Color {
    let f = host.frame();
    let i = ((y * i32::from(f.width) + x) * 4) as usize;
    faraday_draw::Color::rgb(f.rgba[i], f.rgba[i + 1], f.rgba[i + 2])
}

#[test]
fn the_hosts_colours_follow_the_apps_theme_in_stark_contrast_to_it() {
    use super::fills_read::{luminance, ratio};
    use faraday_ui::pal::Role;
    use faraday_ui::{Mode, Theme};
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 1, true, (1280, 800));
    let mut seen = Vec::new();
    for theme in Theme::all() {
        host.app.press(Action::Theme(theme));
        // The strip's own surface at its left end, by the window's foot.
        let strip = pixel(&mut host, 1, 800 - 2);
        let app = theme.role(Role::Surface);
        assert_eq!(strip, theme.role(Role::InverseSurface), "{theme:?}");
        // A light app has a dark host, a dark app a light one: apart at
        // a glance.
        let dark_host = luminance(strip) < luminance(app);
        assert_eq!(dark_host, theme.mode == Mode::Light, "{theme:?}");
        let got = ratio(strip, app);
        assert!(got >= 3.0, "{theme:?}: the strip on the app {got:.2}:1");
        seen.push(strip);
    }
    seen.sort_by_key(|c| (c.r, c.g, c.b));
    seen.dedup();
    assert!(seen.len() >= 6, "the strip follows the theme: {seen:?}");
    // The space round a preview's frame too.
    click(&mut host, HostPress::Menu(HostMenu::Size));
    click(&mut host, HostPress::Size(Some(PI)));
    for theme in Theme::all() {
        host.app.press(Action::Theme(theme));
        let round = pixel(&mut host, 2, 2);
        let got = ratio(round, theme.role(Role::Surface));
        assert!(got >= 3.0, "{theme:?}: round the frame {got:.2}:1");
    }
}

#[test]
fn a_host_menu_is_outlined_in_the_hosts_own_scheme_at_high_contrast_alone() {
    use faraday_ui::pal::Role;
    use faraday_ui::{Contrast, Mode, Theme};
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 1, true, (1280, 800));
    for contrast in [Contrast::Standard, Contrast::High] {
        let app = Theme {
            contrast,
            ..host.app.theme()
        };
        host.app.press(Action::Theme(app));
        host.open_menu_at_rest(HostMenu::Size);
        let _ = host.frame();
        let (x, y, _, h) = host.offered_area(HostPress::Sheet).expect("the menu");
        // The host's scheme is the app's turned inside out.
        let mode = match app.mode {
            Mode::Light => Mode::Dark,
            Mode::Dark => Mode::Light,
        };
        let outline = Theme::new(app.name, mode, contrast).role(Role::Outline);
        let edge = pixel(&mut host, x, y + h / 2);
        assert_eq!(
            edge == outline,
            contrast == Contrast::High,
            "{contrast:?}: {edge:?}"
        );
    }
}

#[test]
fn what_the_host_writes_reads_on_its_fill_in_every_scheme() {
    use faraday_ui::Theme;
    let mut short = Vec::new();
    let mut scenes: Vec<(&str, Host)> = Vec::new();
    // Home: the stick pills, Samples, Size.
    let mut shell = Shell::default();
    scenes.push(("Home", desktop(&mut shell, 2, true, (1280, 800))));
    // The Size menu open, its sizes and the size shown checked.
    let mut shell = Shell::default();
    let mut menu = desktop(&mut shell, 1, true, (1280, 800));
    menu.open_menu_at_rest(HostMenu::Size);
    scenes.push(("the Size menu", menu));
    // Samples' sheet, a stick in, asking for it out.
    let mut shell = Shell::default();
    let mut sheet = desktop(&mut shell, 1, true, (1280, 800));
    sheet.open_menu_at_rest(HostMenu::Samples);
    scenes.push(("Samples' sheet", sheet));
    // Add a key: the aids' section with its buttons and lines.
    let mut shell = Shell::default();
    let mut add = desktop(&mut shell, 0, true, (1280, 800));
    add.app.press(Action::Entry(None));
    scenes.push(("Add a key", add));
    // New key's dice: the die, and Roll the section's main press.
    let mut shell = Shell::default();
    let mut dice = desktop(&mut shell, 0, true, (1280, 800));
    dice.app.press(Action::KeyGen(None));
    dice.app.press(Action::KWords(12));
    dice.app
        .press(Action::KWay(faraday_core::keygen::Way::DiceHashed.index()));
    dice.app.press(Action::KNext);
    scenes.push(("New key's dice", dice));
    for (name, host) in &mut scenes {
        host.record_ink(true);
        for theme in Theme::all() {
            host.app.press(Action::Theme(theme));
            let need = if theme.is_high_contrast() { 7.0 } else { 4.5 };
            let read = host_reads(host);
            assert!(!read.is_empty(), "{name}, {theme:?}: nothing written");
            for (text, got) in read {
                if got < need {
                    short.push(format!("{name}, {theme:?}: {text:?} is {got:.2}:1"));
                }
            }
        }
    }
    assert!(
        short.is_empty(),
        "{} short:\n{}",
        short.len(),
        short.join("\n")
    );
}

/// A box in the window's pixels: its left and top edges, width, height.
type Box4 = (i32, i32, i32, i32);

/// Where the strip's always-there controls stand: the first stick's
/// pill, or Sticks, and Size.
fn ends(host: &Host) -> (Box4, Box4) {
    let first = host
        .offered_area(HostPress::App(Action::StickPill(1)))
        .or_else(|| host.offered_area(HostPress::Menu(HostMenu::Sticks)))
        .expect("a stick's pill");
    let size = host
        .offered_area(HostPress::Menu(HostMenu::Size))
        .expect("Size");
    (first, size)
}

#[test]
fn the_strips_controls_are_centred_in_the_window() {
    for (w, h) in [(1280u16, 800u16), (1920, 1080)] {
        let mut shell = Shell::default();
        let mut host = desktop(&mut shell, 2, true, (w, h));
        let _ = host.frame();
        let ((x, ..), (sx, _, sw, _)) = ends(&host);
        let (left, right) = (x, i32::from(w) - (sx + sw));
        assert!(left > 100, "{w}: the row starts at {x}");
        assert!((left - right).abs() <= 1, "{w}: {left} left, {right} right");
    }
}

#[test]
fn the_strips_controls_are_48_dp_tall_whatever_the_window_and_the_preview() {
    let windows = [
        (1280u16, 800u16),
        (1920, 1080),
        (2560, 1440),
        (960, 1080),
        (390, 844),
    ];
    for (w, h) in windows {
        let mut at_window = None;
        for preview in [None, Some(PI)] {
            let mut shell = Shell::default();
            let mut host = desktop(&mut shell, 2, true, (w, h));
            host.set_preview(preview);
            let _ = host.frame();
            let (first, size) = ends(&host);
            for (x, _, cw, ch) in [first, size] {
                assert!(ch >= 48, "{w}x{h} {preview:?}: a control {ch} tall");
                assert!(x >= 0 && x + cw <= i32::from(w), "{w}x{h}: at {x}");
            }
            // The Pi previewed keeps the strip the window gives.
            let strip = (host.strip_area(), first.3, size.3);
            match at_window {
                None => at_window = Some(strip),
                Some(s) => assert_eq!(s, strip, "{w}x{h}: the Pi preview's strip"),
            }
        }
    }
}

/// Add a key in Practice in a window `w` × `h`, two sticks known: the
/// strip's end is Practice's More.
fn add_a_key_at(shell: &mut Shell, (w, h): (u16, u16)) -> Host {
    let mut host = desktop(shell, 2, true, (w, h));
    host.app.press(Action::Entry(None));
    let _ = host.frame();
    host
}

#[test]
fn at_a_phone_width_the_strip_scrolls_sideways_and_its_end_is_reached() {
    let more = HostPress::Menu(HostMenu::More);
    let sticks = HostPress::Menu(HostMenu::Sticks);
    // Phone portrait: the strip's row, folded, fits inside the margins.
    let mut shell = Shell::default();
    let host = add_a_key_at(&mut shell, (390, 844));
    let (sx, ..) = host.offered_area(sticks).expect("Sticks");
    let (mx, _, mw, _) = host.offered_area(more).expect("More");
    assert!(sx > 16 && mx + mw < 390 - 16, "inside: {sx}, {mx} {mw}");
    // Narrower still, the row is wider than the window less its margins:
    // it starts at the margin and the wheel over it scrolls it sideways,
    // its controls 48 dp tall all the while.
    let mut shell = Shell::default();
    let mut host = add_a_key_at(&mut shell, (268, 600));
    let (x0, y, _, ch) = host.offered_area(sticks).expect("Sticks");
    assert_eq!((x0, ch), (16, 48), "from the margin, 48 dp tall");
    let (m0, ..) = host.offered_area(more).expect("More");
    for _ in 0..10 {
        host.event(Event::Wheel {
            x: 10,
            y: y as u16 + 4,
            dy: 120,
        });
    }
    let _ = host.frame();
    let (m1, _, mw, _) = host.offered_area(more).expect("More");
    assert!(m1 < m0, "the row scrolled: More from {m0} to {m1}");
    assert!(m1 >= 16 && m1 + mw <= 268 - 16, "More whole at {m1}");
    click(&mut host, more);
    assert_eq!(host.menu_open(), Some(HostMenu::More));
    // A finger drags it back; the press it began on presses nothing.
    let mut shell = Shell::default();
    let mut host = add_a_key_at(&mut shell, (268, 600));
    let (m0, my, ..) = host.offered_area(more).expect("More");
    let (fx, fy) = ((m0 + 4) as u16, (my + 4) as u16);
    host.event(Event::Touch {
        x: fx,
        y: fy,
        phase: TouchPhase::Down,
    });
    for (x, phase) in [(fx - 30, TouchPhase::Move), (fx - 30, TouchPhase::Up)] {
        host.event(Event::Touch { x, y: fy, phase });
    }
    let _ = host.frame();
    let (m1, ..) = host.offered_area(more).expect("More");
    assert!(m1 < m0, "dragged: More from {m0} to {m1}");
    assert_eq!(host.menu_open(), None, "the drag opened nothing");
}

/// Ticks `n` times `step` ms apart after `from` as the shell does, a
/// frame drawn for each tick that asks for one; returns how many asked
/// and the time of the last tick.
fn ticks(host: &mut Host, from: u64, step: u64, n: u64) -> (u64, u64) {
    let mut asked = 0;
    let mut now = from;
    for _ in 0..n {
        now += step;
        host.event(Event::Tick { now_ms: now });
        let mut draw = false;
        while let Some(c) = host.poll_command() {
            draw |= matches!(c, Command::Draw);
        }
        if draw {
            asked += 1;
            let _ = host.parts();
            while host.poll_command().is_some() {}
        }
    }
    (asked, now)
}

#[test]
fn an_idle_demo_asks_for_no_frame_at_any_size() {
    for size in [
        (1280, 800),
        (1366, 768),
        (1920, 1080),
        (390, 844),
        (268, 600),
    ] {
        for motion in [false, true] {
            let mut shell = Shell::default();
            let mut host = desktop(&mut shell, 2, true, size);
            host.app.press(Action::ReduceMotion(!motion));
            // Settled: what the press asked for drawn.
            let start = host.app.now_ms();
            let (_, now) = ticks(&mut host, start, 16, 60);
            let (asked, now) = ticks(&mut host, now, 16, 300);
            assert_eq!(
                asked, 0,
                "at {size:?}, motion {motion}: frames asked at rest"
            );
            // The same in the Pi preview, framed in the window.
            click(&mut host, HostPress::Menu(HostMenu::Size));
            let (_, now) = ticks(&mut host, now, 16, 60);
            click(&mut host, HostPress::Size(Some(PI)));
            let (_, now) = ticks(&mut host, now, 16, 60);
            let (asked, mut now) = ticks(&mut host, now, 16, 300);
            assert_eq!(asked, 0, "at {size:?} in the Pi preview, motion {motion}");
            // The pointer over the strip, then over the app: the frame's
            // border changes, and then the window rests again.
            for (x, y) in [(2, size.1 - 2), (size.0 / 2, size.1 / 3)] {
                host.event(Event::Hover { x, y });
                while host.poll_command().is_some() {}
                let _ = host.parts();
                let (_, then) = ticks(&mut host, now, 16, 60);
                let (asked, then) = ticks(&mut host, then, 16, 300);
                now = then;
                assert_eq!(asked, 0, "at {size:?} after a hover, motion {motion}");
            }
        }
    }
}

#[test]
fn a_menu_closing_moves_to_its_end_and_then_the_window_rests() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 2, true, (1280, 800));
    host.app.press(Action::ReduceMotion(false));
    let start = host.app.now_ms();
    let (_, now) = ticks(&mut host, start, 16, 60);
    click(&mut host, HostPress::Menu(HostMenu::Size));
    // It opens over its time, a frame each tick, then rests.
    let (opening, now) = ticks(&mut host, now, 16, 60);
    assert!(opening >= 10, "the menu opened over {opening} frames");
    assert!(opening < 30, "and then rested");
    let (asked, now) = ticks(&mut host, now, 16, 100);
    assert_eq!(asked, 0, "open, at rest");
    let rows = host.strip_texts().len();
    host.event(Event::Back);
    while host.poll_command().is_some() {}
    let _ = host.parts();
    let (closing, now) = ticks(&mut host, now, 16, 60);
    assert!(closing >= 5, "it closed over {closing} frames");
    assert!(closing < 30, "and then rested");
    assert_eq!(host.menu_open(), None);
    assert!(host.strip_texts().len() < rows, "its rows are gone");
    let (asked, _) = ticks(&mut host, now, 16, 200);
    assert_eq!(asked, 0, "closed, at rest");
}

#[test]
fn at_the_windows_own_size_with_nothing_open_the_host_holds_only_the_strip() {
    for (w, h) in [(1280u16, 800u16), (1920, 1080)] {
        let mut shell = Shell::default();
        let mut host = desktop(&mut shell, 2, true, (w, h));
        let window = usize::from(w) * usize::from(h) * 4;
        let strip = usize::from(w) * STRIP as usize * 4;
        let _ = host.parts();
        assert_eq!(host.canvas_bytes(), strip, "at {w} × {h}: the strip alone");
        // A menu open: the window's whole frame, given back once it is
        // closed.
        click(&mut host, HostPress::Menu(HostMenu::Size));
        let _ = host.parts();
        assert_eq!(host.canvas_bytes(), window + strip, "under the menu");
        host.event(Event::Back);
        let _ = host.parts();
        assert_eq!(host.canvas_bytes(), strip, "the menu closed");
    }
}

/// The app's Pull sheet waiting for the sticks in to be pulled: a sample
/// key ticked on Samples' sheet and loaded with a stick in.
fn pull_sheet(host: &mut Host) {
    click(host, HostPress::Menu(HostMenu::Samples));
    let i = first_offered(host);
    click(host, tick_key(i));
    click(host, HostPress::App(Action::SamplesLoad));
    assert_eq!(host.app.sheet, Some(faraday_core::Sheet::Pull));
}

/// A pixel inside the control for `p` near its right end, clear of its
/// label, its dot and its chevron: the control's own fill.
fn inside_end(host: &mut Host, p: HostPress) -> faraday_draw::Color {
    let _ = host.frame();
    let (x, y, w, h) = host
        .offered_area(p)
        .unwrap_or_else(|| panic!("{p:?} offered"));
    pixel(host, x + w - 6, y + h / 2)
}

const BOOT_PILL: HostPress = HostPress::App(Action::StickPill(1));

#[test]
fn the_stick_the_app_asks_for_is_marked_while_its_sheet_waits_and_not_after() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 1, true, (1280, 800));
    let plain = inside_end(&mut host, BOOT_PILL);
    pull_sheet(&mut host);
    let marked = inside_end(&mut host, BOOT_PILL);
    assert_ne!(marked, plain, "the boot stick's pill is marked");
    // Under Reduce motion the mark is steady: at rest, no frame asked.
    let start = host.app.now_ms();
    let (_, now) = ticks(&mut host, start, 16, 10);
    let (asked, _) = ticks(&mut host, now, 16, 300);
    assert_eq!(asked, 0, "a steady mark asks for no frame");
    assert_eq!(inside_end(&mut host, BOOT_PILL), marked);
    // The stick pulled from its pill: the sheet goes, and the mark.
    click(&mut host, BOOT_PILL);
    shell_follows(&mut shell, &mut host.app);
    assert_eq!(host.app.sheet, None);
    assert_eq!(inside_end(&mut host, BOOT_PILL), plain, "the mark is gone");
}

#[test]
fn a_stick_pill_pulses_for_a_while_then_holds_its_mark_and_the_window_rests() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 1, true, (1280, 800));
    host.app.press(Action::ReduceMotion(false));
    let start = host.app.now_ms();
    let (_, now) = ticks(&mut host, start, 16, 60);
    let plain = inside_end(&mut host, BOOT_PILL);
    pull_sheet(&mut host);
    let mut seen = Vec::new();
    let mut now = now;
    for _ in 0..40 {
        let (_, n) = ticks(&mut host, now, 16, 1);
        now = n;
        seen.push(inside_end(&mut host, BOOT_PILL));
    }
    seen.dedup();
    assert!(seen.len() > 5, "the pill pulses: {seen:?}");
    assert!(seen.iter().all(|c| *c != plain), "and stays marked");
    // The pulses run their time, then the mark holds and the window rests.
    let (pulsing, now) = ticks(&mut host, now, 16, 300);
    assert!(pulsing > 0, "the pulse asked for its frames");
    let (asked, _) = ticks(&mut host, now, 16, 300);
    assert_eq!(asked, 0, "the mark at rest asks for no frame");
    assert_ne!(inside_end(&mut host, BOOT_PILL), plain);
}

#[test]
fn a_pulse_in_the_pi_preview_draws_the_strip_alone() {
    use faraday_core::work::{Work, since, snapshot};
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 1, true, (1280, 800));
    host.app.press(Action::ReduceMotion(false));
    click(&mut host, HostPress::Menu(HostMenu::Size));
    click(&mut host, HostPress::Size(Some(PI)));
    let start = host.app.now_ms();
    let (_, now) = ticks(&mut host, start, 16, 60);
    pull_sheet(&mut host);
    // The app's own fade to its sheet run.
    let (_, now) = ticks(&mut host, now, 16, 30);
    let before = snapshot();
    let (asked, _) = ticks(&mut host, now, 16, 60);
    assert!(asked > 30, "the pulse drew {asked} frames");
    let work = since(&before);
    assert_eq!(
        work.of(Work::FullCanvas),
        0,
        "a frame of the pulse placed the app's frame again"
    );
}

#[test]
fn sticks_n_is_marked_where_the_stick_asked_for_is_folded_into_it() {
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 5, true, (480, 640));
    let more = HostPress::Menu(HostMenu::Sticks);
    let plain = inside_end(&mut host, more);
    pull_sheet(&mut host);
    assert_ne!(inside_end(&mut host, more), plain, "Sticks · N is marked");
}

/// The colour of the app's focus border at the window's own size: its
/// left edge, half way down.
fn border(host: &mut Host) -> faraday_draw::Color {
    let (_, y, _, h) = host.app_area().expect("the app");
    pixel(host, 0, y + h / 2)
}

#[test]
fn the_apps_frame_is_bordered_in_primary_while_the_pointer_or_the_keys_reach_it() {
    use faraday_ui::pal::Role;
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 1, true, (1280, 800));
    let theme = host.app.theme();
    let (lit, dim) = (theme.role(Role::Primary), theme.role(Role::OutlineVariant));
    assert_eq!(border(&mut host), lit, "the app has the keys at the start");
    let hover = |host: &mut Host, x: u16, y: u16| host.event(Event::Hover { x, y });
    // The pointer over the strip: dim; back over the app: lit.
    hover(&mut host, 640, 800 - 4);
    assert_eq!(border(&mut host), dim, "the pointer on the strip");
    hover(&mut host, 640, 400);
    assert_eq!(border(&mut host), lit, "the pointer over the app");
    // Out of the window: dim; a key, which goes to the app: lit.
    host.event(Event::HoverEnd);
    assert_eq!(border(&mut host), dim, "the pointer out of the window");
    host.event(Event::Key(faraday_shell_api::Key::Char('a')));
    assert_eq!(border(&mut host), lit, "a key reached the app");
    // The window losing the keyboard to another: dim, whatever the
    // pointer; back, with the pointer over the app: lit.
    hover(&mut host, 640, 400);
    host.event(Event::Focus(false));
    assert_eq!(border(&mut host), dim, "the window lost the keys");
    host.event(Event::Focus(true));
    assert_eq!(border(&mut host), lit, "the window has them again");

    // In the Pi preview, the frame's border, just outside it, in the
    // host's scheme.
    click(&mut host, HostPress::Menu(HostMenu::Size));
    click(&mut host, HostPress::Size(Some(PI)));
    let (x, y, _, h) = host.app_area().expect("the frame");
    let around = faraday_ui::Theme::new(
        theme.name,
        match theme.mode {
            faraday_ui::Mode::Light => faraday_ui::Mode::Dark,
            faraday_ui::Mode::Dark => faraday_ui::Mode::Light,
        },
        theme.contrast,
    );
    hover(&mut host, (x + 10) as u16, (y + h / 2) as u16);
    assert_eq!(
        pixel(&mut host, x - 1, y + h / 2),
        around.role(Role::Primary)
    );
    hover(&mut host, 4, 4);
    assert_eq!(
        pixel(&mut host, x - 1, y + h / 2),
        around.role(Role::OutlineVariant),
        "the pointer in the host's space round the frame"
    );
}

#[test]
fn the_border_fades_between_its_colours_and_then_the_window_rests() {
    use faraday_ui::pal::Role;
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 1, true, (1280, 800));
    host.app.press(Action::ReduceMotion(false));
    let theme = host.app.theme();
    let (lit, dim) = (theme.role(Role::Primary), theme.role(Role::OutlineVariant));
    let start = host.app.now_ms();
    let (_, now) = ticks(&mut host, start, 16, 60);
    host.event(Event::Hover { x: 640, y: 800 - 4 });
    let (_, now) = ticks(&mut host, now, 16, 4);
    let part = border(&mut host);
    assert!(part != lit && part != dim, "part of the way: {part:?}");
    let (fading, now) = ticks(&mut host, now, 16, 60);
    assert!(
        fading > 0 && fading < 30,
        "it faded over {fading} more frames"
    );
    assert_eq!(border(&mut host), dim);
    let (asked, _) = ticks(&mut host, now, 16, 200);
    assert_eq!(asked, 0, "and then rested");
}

#[test]
fn a_menu_fades_in_over_the_app_and_is_there_at_once_under_reduce_motion() {
    let this_window = HostPress::Size(None);
    for motion in [true, false] {
        let mut shell = Shell::default();
        let mut host = desktop(&mut shell, 1, true, (1280, 800));
        host.app.press(Action::ReduceMotion(!motion));
        let start = host.app.now_ms();
        let (_, now) = ticks(&mut host, start, 16, 60);
        // What the menu's first row will stand on, then the row at rest.
        click(&mut host, HostPress::Menu(HostMenu::Size));
        let (_, now) = ticks(&mut host, now, 16, 60);
        let (x, y, _, _) = host.offered_area(this_window).expect("This window");
        let open = pixel(&mut host, x + 4, y + 4);
        host.event(Event::Back);
        let (_, now) = ticks(&mut host, now, 16, 60);
        let under = pixel(&mut host, x + 4, y + 4);
        assert_ne!(under, open, "the menu stands over the app");
        click(&mut host, HostPress::Menu(HostMenu::Size));
        let (_, _) = ticks(&mut host, now, 16, 3);
        let early = pixel(&mut host, x + 4, y + 4);
        if motion {
            assert!(
                early != open && early != under,
                "part of the way in: {early:?}"
            );
        } else {
            assert_eq!(early, open, "under Reduce motion, whole at once");
        }
    }
}

#[test]
fn a_roll_from_the_strip_is_in_the_entry_in_the_frame_that_shows_the_press() {
    use faraday_core::vault::{RollerAct, RollerPerClick};
    let roll = HostPress::App(Action::KRoller(RollerAct::Roll));
    let mut shell = Shell::default();
    let mut host = desktop(&mut shell, 0, true, (1280, 800));
    host.app.press(Action::ReduceMotion(false));
    host.app.press(Action::KeyGen(None));
    host.app.press(Action::KWords(12));
    host.app
        .press(Action::KWay(faraday_core::keygen::Way::DiceHashed.index()));
    host.app.press(Action::KNext);
    let _ = host.frame();
    // One value: in the entry with no tick between the press and the
    // frame.
    let before = host.app.keygen().unwrap().dice.len();
    click(&mut host, roll);
    assert_eq!(host.app.keygen().unwrap().dice.len(), before + 1);
    // Five a click: all five at once.
    click(&mut host, HostPress::Menu(HostMenu::Rolls));
    click(
        &mut host,
        HostPress::App(Action::KRoller(RollerAct::Per(RollerPerClick::Five))),
    );
    click(&mut host, roll);
    assert_eq!(host.app.keygen().unwrap().dice.len(), before + 6);
    // The die tumbles a moment in the strip, and its values are the five.
    assert!(
        host.app.keygen().unwrap().roller.active(),
        "the die tumbles"
    );
    let start = host.app.now_ms();
    let _ = ticks(&mut host, start, 16, 60);
    assert!(!host.app.keygen().unwrap().roller.active(), "and rests");
    assert_eq!(host.app.keygen().unwrap().dice.len(), before + 6);
}

/// A finger down at `at`, moved `dy` pixels in `steps` moves a tick of
/// `ms` apart, and lifted there; returns the time of the last tick.
fn finger_drag(host: &mut Host, at: (u16, u16), dy: i32, (steps, ms): (i32, u64), now: u64) -> u64 {
    let (x, y) = at;
    let mut now = now;
    host.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    let mut to = y;
    for i in 1..=steps {
        to = (i32::from(y) + dy * i / steps) as u16;
        now += ms;
        host.event(Event::Tick { now_ms: now });
        host.event(Event::Touch {
            x,
            y: to,
            phase: TouchPhase::Move,
        });
        let _ = host.frame();
    }
    host.event(Event::Touch {
        x,
        y: to,
        phase: TouchPhase::Up,
    });
    let _ = host.frame();
    now
}

/// Where the host's control for `p` stands in the last frame: its centre.
fn centre(host: &mut Host, p: HostPress) -> (u16, u16) {
    let _ = host.frame();
    host.where_offered(p)
        .unwrap_or_else(|| panic!("{p:?} is not offered: {:?}", host.strip_texts()))
}

/// Samples' sheet open in a window too short for its lists, motion on or
/// off, the window at rest; a sample key to tick and where it stands.
fn samples_open(shell: &mut Shell, motion: bool) -> (Host, HostPress, (u16, u16), u64) {
    let mut host = desktop(shell, 0, true, (1280, 600));
    host.app.press(Action::ReduceMotion(!motion));
    let start = host.app.now_ms();
    let (_, now) = ticks(&mut host, start, 16, 60);
    click(&mut host, HostPress::Menu(HostMenu::Samples));
    let (_, now) = ticks(&mut host, now, 16, 60);
    let key = tick_key(first_offered(&mut host));
    let at = centre(&mut host, key);
    (host, key, at, now)
}

/// Nothing on Samples' sheet is ticked.
fn none_ticked(host: &mut Host) -> bool {
    strip(host).iter().any(|t| t == "Select all")
}

#[test]
fn a_finger_drag_scrolls_samples_sheet_and_presses_nothing() {
    let mut shell = Shell::default();
    let (mut host, key, (x, y0), now) = samples_open(&mut shell, false);
    // Up by 120 pixels, the finger starting on a sample's tick box.
    finger_drag(&mut host, (x, y0), -120, (6, 16), now);
    assert!(host.list_scrolled() > 0.0, "the sheet's lists scrolled");
    let (_, y1) = centre(&mut host, key);
    assert!(y1 < y0, "the row moved up with the finger: {y0} to {y1}");
    // The drag ticked nothing and closed nothing; a tap still ticks.
    assert_eq!(host.menu_open(), Some(HostMenu::Samples));
    assert!(none_ticked(&mut host));
    click(&mut host, key);
    assert!(!none_ticked(&mut host), "the tap ticked it");
}

#[test]
fn a_finger_drag_scrolls_the_size_menu_to_its_last_row_which_a_tap_chooses() {
    let last = faraday_draw::tokens::PREVIEW_SIZES[8];
    let monitor = HostPress::Size(Some(last));
    let mut shell = Shell::default();
    // A window too short for the menu: it stands above the strip, its
    // rows scrolling inside it.
    let mut host = desktop(&mut shell, 0, true, (1280, 400));
    click(&mut host, HostPress::Menu(HostMenu::Size));
    let (_, my, _, mh) = host.menu_area().expect("the Size menu");
    assert!(
        my >= 0 && my + mh <= 400 - STRIP,
        "inside the window: {my}, {mh}"
    );
    let foot = host.offered_area(monitor).map(|(_, y, _, h)| y + h);
    assert!(foot.is_none_or(|b| b > my + mh), "the last row out of view");
    let at = centre(&mut host, HostPress::Size(None));
    let start = host.app.now_ms();
    finger_drag(&mut host, at, -200, (8, 16), start);
    assert_eq!(
        host.menu_open(),
        Some(HostMenu::Size),
        "the drag chose nothing"
    );
    assert_eq!(host.preview(), None);
    let (_, y) = centre(&mut host, monitor);
    assert!(
        (my..my + mh).contains(&i32::from(y)),
        "the last row in view"
    );
    click(&mut host, monitor);
    assert_eq!(host.preview(), Some(last));
    assert_eq!(host.menu_open(), None);
}

#[test]
fn a_fling_on_samples_sheet_coasts_then_settles_and_the_window_rests() {
    let mut shell = Shell::default();
    let (mut host, _, at, now) = samples_open(&mut shell, true);
    // A flick: 30 pixels a frame, lifted while moving.
    let now = finger_drag(&mut host, at, -150, (5, 16), now);
    let lifted = host.list_scrolled();
    let (coasting, now) = ticks(&mut host, now, 16, 10);
    assert_eq!(coasting, 10, "each tick of the coast draws");
    assert!(host.list_scrolled() > lifted, "it coasts on after the lift");
    // It slows to a stop, and then the window asks for nothing.
    let (_, now) = ticks(&mut host, now, 16, 200);
    let settled = host.list_scrolled();
    let (asked, _) = ticks(&mut host, now, 16, 200);
    assert_eq!(asked, 0, "at rest after the fling");
    assert_eq!(host.list_scrolled(), settled);
    assert!(none_ticked(&mut host));
}

#[test]
fn a_touch_on_samples_sheet_while_it_coasts_stops_it_and_presses_nothing() {
    let mut shell = Shell::default();
    let (mut host, _, at, now) = samples_open(&mut shell, true);
    let now = finger_drag(&mut host, at, -150, (5, 16), now);
    let (_, now) = ticks(&mut host, now, 16, 2);
    // A tap on whatever row is under the finger now.
    tap_at(&mut host, at.0, at.1);
    let stopped = host.list_scrolled();
    let (asked, _) = ticks(&mut host, now, 16, 60);
    assert!(asked <= 1, "it stopped: {asked} frames after");
    assert_eq!(host.list_scrolled(), stopped);
    assert!(none_ticked(&mut host), "the tap ticked nothing");
    assert_eq!(host.menu_open(), Some(HostMenu::Samples));
}

#[test]
fn under_reduce_motion_a_drag_on_samples_sheet_stops_at_the_lift() {
    let mut shell = Shell::default();
    let (mut host, _, at, now) = samples_open(&mut shell, false);
    let now = finger_drag(&mut host, at, -150, (5, 16), now);
    let lifted = host.list_scrolled();
    assert!(lifted > 0.0, "it followed the finger");
    let (asked, _) = ticks(&mut host, now, 16, 60);
    assert!(asked <= 1, "no coast: {asked} frames");
    assert_eq!(host.list_scrolled(), lifted);
}

#[test]
fn a_fling_along_the_narrow_strip_coasts_and_then_the_window_rests() {
    let more = HostPress::Menu(HostMenu::More);
    let mut shell = Shell::default();
    let mut host = add_a_key_at(&mut shell, (268, 600));
    host.app.press(Action::ReduceMotion(false));
    let start = host.app.now_ms();
    let (_, mut now) = ticks(&mut host, start, 16, 60);
    let (m0, my, ..) = host.offered_area(more).expect("More");
    let (mut x, y) = ((m0 + 4) as u16, (my + 4) as u16);
    let phase = TouchPhase::Down;
    host.event(Event::Touch { x, y, phase });
    for _ in 0..3 {
        now += 16;
        host.event(Event::Tick { now_ms: now });
        x -= 12;
        let phase = TouchPhase::Move;
        host.event(Event::Touch { x, y, phase });
        let _ = host.parts();
    }
    let phase = TouchPhase::Up;
    host.event(Event::Touch { x, y, phase });
    let _ = host.parts();
    let (lifted, ..) = host.offered_area(more).expect("More");
    let (coast, now) = ticks(&mut host, now, 16, 5);
    assert!(coast > 0, "the row coasts");
    let (_, now) = ticks(&mut host, now, 16, 200);
    let (moved, ..) = host.offered_area(more).expect("More");
    assert!(moved < lifted, "More went on from {lifted} to {moved}");
    let (asked, _) = ticks(&mut host, now, 16, 200);
    assert_eq!(asked, 0, "and then the window rests");
    assert_eq!(host.menu_open(), None, "the fling opened nothing");
}
