//! Home: Scan floats at the foot; Add a key is reached from Wallets, not
//! from Home, and opens New key on its Start card. Keys load only with no stick attached: Add a key pressed
//! with one in asks for it to be pulled, and opens once it is. A camera
//! frame may picture a seed, so Scan waits for the stick too, and a
//! stick plugged in while the camera is on is held until it is off.
//! Home's Scan takes a seed to Add a key, and anything else where it
//! went before.

use faraday_core::nav::{MenuPress, Way};
use faraday_core::testkit::{KindText, Plain};
use faraday_core::{Action, Faraday, OskPress, Screen, Sheet, StickInfo, StorageEvent};
use faraday_shell_api::{App, Command, DisplayInfo, Event, TouchPhase};

fn shown() -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
    app.event(faraday_shell_api::Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

fn stick(app: &mut Faraday, on: bool) {
    let sticks = if on {
        vec![StickInfo {
            make: Default::default(),
            serial: None,
            id: faraday_text::Text::of("a"),
            label: faraday_text::Text::of("STICK"),
            boot: false,
            files: Vec::new(),
            vaults: Vec::new(),
        }]
    } else {
        Vec::new()
    };
    app.storage(StorageEvent::Sticks(sticks));
    let _ = app.frame();
}

/// Presses `action` where the last frame drew it.
fn tap(app: &mut Faraday, action: Action) {
    let _ = app.frame();
    let (x, y) = app
        .where_offered(action)
        .unwrap_or_else(|| panic!("{action:?} is not on screen"));
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    // The frame that shows the press, and the tick after it, on which a
    // press that stretches a key does its work (`docs/SPEED.md` §9).
    let _ = app.frame();
    let now = app.now_ms() + 16;
    app.event(Event::Tick { now_ms: now });
    let _ = app.frame();
}

/// Home with a stick attached: the stick visit opens on its arrival, and
/// Home is one press away.
fn home_with_stick() -> Faraday {
    let mut app = shown();
    stick(&mut app, true);
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app
}

#[test]
fn home_offers_scan_with_no_stick_attached() {
    let app = shown();
    assert!(app.offers(Action::Scan));
}

#[test]
fn home_offers_scan_with_a_stick_attached() {
    let app = home_with_stick();
    assert!(app.offers(Action::Scan));
}

#[test]
fn scan_on_home_starts_the_camera() {
    let mut app = shown();
    while app.poll_command().is_some() {}
    tap(&mut app, Action::Scan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(std::iter::from_fn(|| app.poll_command()).any(|c| c == Command::CameraOn));
}

fn sized(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
    app.event(faraday_shell_api::Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

/// Whether the last frame floats Scan at the window's bottom-right (a
/// page's own Scan sits in its content, and the button is drawn last).
fn floats_scan(app: &Faraday, w: u16, h: u16) -> bool {
    app.hit_box(Action::Scan)
        .is_some_and(|(x, y, bw, bh)| x + bw + 32 >= w && x + bw <= w && y + bh + 120 >= h)
}

const FIVE: [Screen; 5] = [
    Screen::Check,
    Screen::WalletsStart,
    Screen::Files,
    Screen::Vaults,
    Screen::Catalog,
];

/// Whether the last frame has Scan in the navigation: in the sidebar or
/// the rail, at the window's left and at its foot, and not floating.
fn scan_in_navigation(app: &Faraday, w: u16, h: u16, nav_w: u16) -> bool {
    app.hit_box(Action::Scan)
        .is_some_and(|(x, y, bw, bh)| x + bw <= nav_w && y + bh > h * 3 / 4)
        && !floats_scan(app, w, h)
}

/// Scan is at the foot of the sidebar on 1280x800 and of the rail on a rail
/// window, on every page, and opens the scanner; Back returns to the page.
#[test]
fn scan_is_in_the_navigation_on_every_page_of_the_wide_windows() {
    // The sidebar is 240 wide; the rail 80.
    for (w, h, nav_w) in [(1280, 800, 240), (700, 800, 80), (1024, 600, 80)] {
        for screen in [Screen::Home, Screen::Settings].into_iter().chain(FIVE) {
            let mut app = sized(w, h, 160);
            app.press(Action::Nav(screen));
            let _ = app.frame();
            assert!(
                scan_in_navigation(&app, w, h, nav_w),
                "{screen:?} at {w}x{h}"
            );
            while app.poll_command().is_some() {}
            tap(&mut app, Action::Scan);
            assert_eq!(app.sheet, Some(Sheet::Scan));
            assert!(std::iter::from_fn(|| app.poll_command()).any(|c| c == Command::CameraOn));
        }
    }
}

/// Scan floats on the top page of Check, Wallets, Files, Vaults and Tools
/// as it does on Home, on a phone-size window, and starts
/// the camera; Back returns to the page it was pressed on.
#[test]
fn scan_floats_on_the_five_destinations() {
    for (w, h) in [(400, 800)] {
        for screen in FIVE {
            let mut app = sized(w, h, 160);
            // A vault file in Files: Vaults shows its page, not New vault.
            app.storage(faraday_core::StorageEvent::Restored {
                inbox: faraday_core::files_from(vec![(
                    "vault.kdbx".to_string(),
                    faraday_core::testkit::test_vault().unwrap(),
                )]),
                outbox: Vec::new(),
                kept: Vec::new(),
            });
            app.press(Action::Nav(screen));
            let _ = app.frame();
            assert!(floats_scan(&app, w, h), "{screen:?} at {w}x{h}");
            while app.poll_command().is_some() {}
            tap(&mut app, Action::Scan);
            assert_eq!(app.sheet, Some(Sheet::Scan));
            assert!(std::iter::from_fn(|| app.poll_command()).any(|c| c == Command::CameraOn));
            app.press(Action::Cancel);
            let _ = app.frame();
            assert_eq!(app.screen, screen, "Back from the scanner at {w}x{h}");
            assert_eq!(app.sheet, None);
        }
    }
}

#[test]
fn scan_does_not_float_on_settings_or_inside_a_flow() {
    for (w, h) in [(1280, 800), (400, 800)] {
        let mut app = sized(w, h, 160);
        app.press(Action::Nav(Screen::Settings));
        let _ = app.frame();
        assert!(!floats_scan(&app, w, h), "Settings at {w}x{h}");
        let mut app = sized(w, h, 160);
        app.press(Action::Nav(Screen::WalletsStart));
        tap(&mut app, Action::Entry(None));
        assert_eq!(app.screen, Screen::Entry, "Add a key at {w}x{h}");
        // The compact Add a key page keeps New key under its fold.
        app.press(Action::KeyGen(None));
        let _ = app.frame();
        assert_eq!(app.screen, Screen::KeyGen);
        assert!(!floats_scan(&app, w, h), "a flow at {w}x{h}");
    }
}

/// The small panel's menu layout keeps Scan on Home alone.
#[test]
fn scan_on_the_small_panel_stays_on_home() {
    let mut app = sized(480, 640, 286);
    assert!(floats_scan(&app, 480, 640));
    for screen in FIVE {
        app.press(Action::Nav(screen));
        let _ = app.frame();
        assert!(
            !floats_scan(&app, 480, 640),
            "{screen:?} on the small panel"
        );
    }
}

/// Scan's hit box, and whether its touch target is whole, in `app`'s
/// last frame.
fn scan_target(app: &Faraday) -> ((u16, u16, u16, u16), bool) {
    let hit = app.hit_box(Action::Scan).expect("Scan is drawn");
    let whole = app
        .drawn_targets()
        .into_iter()
        .filter(|(a, ..)| *a == Action::Scan)
        .all(|(_, target, _, _, grown)| target == grown);
    (hit, whole)
}

/// On Files, with a long list, and on Wallets' start page, Scan at 1024x600
/// (at the top of the rail) is as large as on Home and no control takes a
/// share of its target.
#[test]
fn scan_keeps_its_whole_target_over_a_list_at_1024x600() {
    let home = sized(1024, 600, 160);
    let ((_, _, bw, bh), home_whole) = scan_target(&home);
    assert!(home_whole);
    let mut app = sized(1024, 600, 160);
    stick(&mut app, true);
    let names: Vec<(String, Vec<u8>)> = (0..12)
        .map(|i| (format!("file-{i}.txt"), b"x".to_vec()))
        .collect();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(names),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::Nav(Screen::Files));
    let _ = app.frame();
    let ((_, _, w, h), whole) = scan_target(&app);
    assert_eq!(((w, h), whole), ((bw, bh), true), "Files");
    app.press(Action::Nav(Screen::WalletsStart));
    let _ = app.frame();
    let ((_, _, w, h), whole) = scan_target(&app);
    assert_eq!(((w, h), whole), ((bw, bh), true), "Wallets");
}

/// Add a key is reached from Wallets' empty state and from Tools now,
/// not from Home: Home's lead, with nothing
/// loaded, is Make a new wallet instead.
#[test]
fn home_does_not_offer_add_a_key_directly() {
    let app = shown();
    assert!(!app.offers(Action::Entry(None)));
    assert!(!app.offers(Action::KeyGen(None)));
}

#[test]
fn add_a_key_from_wallets_opens_new_key_on_start() {
    let mut app = shown();
    app.press(Action::Nav(Screen::WalletsStart));
    tap(&mut app, Action::Entry(None));
    tap(&mut app, Action::KeyGen(None));
    assert_eq!(app.screen, Screen::KeyGen);
    assert_eq!(app.sheet, None);
    assert_eq!(
        app.keygen().unwrap().open,
        Some(faraday_core::keygen::kstep::START),
        "Start opens with nothing ticked"
    );
}

#[test]
fn add_a_key_from_wallets_with_a_stick_in_asks_for_it_and_opens_once_it_is_pulled() {
    let mut app = home_with_stick();
    app.press(Action::Nav(Screen::WalletsStart));
    tap(&mut app, Action::Entry(None));
    assert_eq!(app.sheet, Some(Sheet::Pull));
    assert_eq!(app.screen, Screen::WalletsStart);
    stick(&mut app, false);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Entry);
}

#[test]
fn cancel_leaves_wallets_and_a_later_pull_opens_nothing() {
    let mut app = home_with_stick();
    app.press(Action::Nav(Screen::WalletsStart));
    tap(&mut app, Action::Entry(None));
    tap(&mut app, Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::WalletsStart);
    stick(&mut app, false);
    assert_eq!(app.screen, Screen::WalletsStart);
}

#[test]
fn the_wallets_tab_offers_add_a_key() {
    let mut app = shown();
    app.press(Action::Nav(Screen::WalletsStart));
    tap(&mut app, Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry);
}

#[test]
fn make_a_key_with_a_stick_in_waits_for_the_pull() {
    let mut app = home_with_stick();
    app.press(Action::KeyGen(None));
    assert_eq!(app.sheet, Some(Sheet::Pull));
    stick(&mut app, false);
    assert_eq!(app.screen, Screen::KeyGen);
}

/// The camera reads `bytes`.
fn read(app: &mut Faraday, bytes: &[u8]) {
    app.event(Event::Scanned {
        bytes: bytes.to_vec(),
    });
    let _ = app.frame();
}

/// A seed read: its sheet asks how it loads, and it loads without a
/// passphrase.
fn without_passphrase(app: &mut Faraday) {
    assert_eq!(
        app.sheet,
        Some(Sheet::SeedRead),
        "the seed asks how it loads"
    );
    app.press(Action::SeedRead(faraday_core::seeds::read::Choice::Plain));
    faraday_core::testkit::keep_published(app);
    let _ = app.frame();
}

/// The SeedQR of "abandon" eleven times and "about".
const SEEDQR: &str = "000000000000000000000000000000000000000000000003";

#[test]
fn scan_with_a_stick_in_asks_for_it_and_opens_the_camera_once_it_is_pulled() {
    let mut app = home_with_stick();
    while app.poll_command().is_some() {}
    tap(&mut app, Action::Scan);
    assert_eq!(app.sheet, Some(Sheet::Pull));
    assert!(!std::iter::from_fn(|| app.poll_command()).any(|c| c == Command::CameraOn));
    stick(&mut app, false);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(std::iter::from_fn(|| app.poll_command()).any(|c| c == Command::CameraOn));
}

#[test]
fn a_stick_plugged_in_while_scanning_is_held_until_the_camera_is_off() {
    let mut app = shown();
    assert!(app.clean());
    tap(&mut app, Action::Scan);
    stick(&mut app, true);
    assert!(!app.clean(), "a stick may be handed out with the camera on");
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert_eq!(app.screen, Screen::Home);
    app.press(Action::Cancel);
    let _ = app.frame();
    assert!(app.clean());
    assert_eq!(app.screen, Screen::Files);
}

#[test]
fn a_stick_plugged_in_under_the_idle_warning_over_a_scan_is_still_held() {
    const MIN: u64 = 60_000;
    // The warning comes only where a lock would wipe something.
    let mut app = shown();
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("spending-wallet.txt"),
            kit.descriptor.into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    faraday_core::testkit::load_wallet(&mut app, 0);
    tap(&mut app, Action::Scan);
    app.event(Event::Tick { now_ms: 0 });
    app.event(Event::Tick { now_ms: 26 * MIN });
    let _ = app.frame();
    assert_eq!(
        app.sheet,
        Some(Sheet::IdleWarn),
        "the warning is over the scan"
    );
    assert!(app.scan().is_some(), "the scan runs under the warning");
    assert!(!app.clean(), "the camera is on under the warning");
    let before = app.screen;
    stick(&mut app, true);
    assert!(!app.clean(), "a stick may be handed out with the camera on");
    assert_eq!(app.screen, before, "the stick was not visited");
}

/// The kernel shows storage before the disk process lists it: with a scan
/// held it is held back too, never put a Lock sheet over the scan.
fn attached_during_a_scan() -> Faraday {
    let mut app = shown();
    tap(&mut app, Action::Scan);
    while app.poll_command().is_some() {}
    app.storage(StorageEvent::Attached(true));
    assert_eq!(app.sheet, Some(Sheet::Scan), "the scanner stays");
    assert!(app.scan().is_some());
    assert!(!app.clean());
    app
}

#[test]
fn storage_attached_during_a_scan_is_held_until_the_scan_ends() {
    let mut app = attached_during_a_scan();
    // The stick listed meanwhile is held back too: the scanner stays and
    // the stick is visited once the scan ends.
    stick(&mut app, true);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert_ne!(app.screen, Screen::Files);
    app.press(Action::Cancel);
    let cmds: Vec<_> = std::iter::from_fn(|| app.poll_command()).collect();
    assert!(cmds.contains(&Command::CameraOff));
    assert!(app.scan().is_none());
    assert_eq!(app.screen, Screen::Files);
}

/// A process that holds a key, so not clean, with Home's scanner up.
fn tainted_scanning() -> Faraday {
    let mut app = shown();
    tap(&mut app, Action::Scan);
    read(&mut app, SEEDQR.as_bytes());
    without_passphrase(&mut app);
    assert_eq!(app.session.keys.len(), 1);
    app.press(Action::Scan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    while app.poll_command().is_some() {}
    app
}

#[test]
fn storage_attached_during_a_scan_in_a_process_not_clean_asks_for_lock_at_its_end() {
    let mut app = tainted_scanning();
    app.storage(StorageEvent::Attached(true));
    assert_eq!(app.sheet, Some(Sheet::Scan), "the scanner stays");
    app.press(Action::Cancel);
    assert!(app.scan().is_none());
    assert_eq!(app.sheet, Some(Sheet::Lock));
}

#[test]
fn a_stick_listed_during_a_scan_in_a_process_not_clean_asks_for_lock_at_its_end() {
    let mut app = tainted_scanning();
    app.storage(StorageEvent::Attached(true));
    stick(&mut app, true);
    assert_eq!(app.sheet, Some(Sheet::Scan), "the scanner stays");
    app.press(Action::Cancel);
    assert!(app.scan().is_none());
    assert_eq!(app.sheet, Some(Sheet::Lock));
}

#[test]
fn a_listing_emptied_with_storage_still_attached_keeps_the_hold_for_the_scans_end() {
    let mut app = tainted_scanning();
    app.storage(StorageEvent::Attached(true));
    stick(&mut app, true);
    stick(&mut app, false);
    assert_eq!(app.sheet, Some(Sheet::Scan), "the scanner stays");
    app.press(Action::Cancel);
    assert_eq!(app.sheet, Some(Sheet::Lock));
}

#[test]
fn storage_pulled_during_a_scan_leaves_the_scan_to_cancel() {
    let mut app = attached_during_a_scan();
    app.storage(StorageEvent::Attached(false));
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(app.scan().is_some());
    app.press(Action::Cancel);
    assert!(app.scan().is_none());
    assert!(app.clean());
}

#[test]
fn storage_attached_under_the_idle_warning_over_a_scan_waits_for_the_scan() {
    const MIN: u64 = 60_000;
    let mut app = shown();
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("spending-wallet.txt"),
            kit.descriptor.into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    faraday_core::testkit::load_wallet(&mut app, 0);
    tap(&mut app, Action::Scan);
    app.event(Event::Tick { now_ms: 0 });
    app.event(Event::Tick { now_ms: 26 * MIN });
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    app.storage(StorageEvent::Attached(true));
    stick(&mut app, true);
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    assert!(app.scan().is_some());
    assert_ne!(app.screen, Screen::Files);
    // A touch puts the warning away and the scanner is back, the stick
    // still waiting for the scan to end.
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x: 2, y: 2, phase });
    }
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert_ne!(app.screen, Screen::Files);
    app.press(Action::Cancel);
    let _ = app.frame();
    assert!(app.scan().is_none(), "no scan is left held with no sheet");
    assert_eq!(app.screen, Screen::Files, "the stick waited for the scan");
}

#[test]
fn a_seedqr_scanned_while_a_stick_is_held_loads_no_key() {
    let mut app = shown();
    tap(&mut app, Action::Scan);
    stick(&mut app, true);
    read(&mut app, SEEDQR.as_bytes());
    assert!(app.session.keys.is_empty());
    assert!(app.inbox().is_empty());
    app.press(Action::Cancel);
    assert!(app.clean());
}

#[test]
fn a_seedqr_scanned_from_home_loads_the_key_add_a_keys_scan_loads() {
    let mut from_add = shown();
    from_add.press(Action::Nav(Screen::WalletsStart));
    tap(&mut from_add, Action::Entry(None));
    tap(&mut from_add, Action::ScanSeed);
    assert_eq!(from_add.sheet, Some(Sheet::Scan), "Scan a SeedQR moves on");
    read(&mut from_add, SEEDQR.as_bytes());
    without_passphrase(&mut from_add);
    faraday_core::testkit::keep_published(&mut from_add);

    let mut app = shown();
    tap(&mut app, Action::Scan);
    read(&mut app, SEEDQR.as_bytes());
    without_passphrase(&mut app);
    assert_eq!(app.sheet, None);
    assert!(app.inbox().is_empty());
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(
        app.session.keys[0].fingerprint,
        from_add.session.keys[0].fingerprint
    );
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn a_seeds_words_scanned_from_home_go_to_add_a_key() {
    let mut app = shown();
    tap(&mut app, Action::Scan);
    read(
        &mut app,
        b"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    without_passphrase(&mut app);
    assert!(app.inbox().is_empty());
    assert_eq!(app.session.keys.len(), 1);
}

#[test]
fn a_psbt_scanned_from_home_opens_its_review() {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
    let mut app = shown();
    tap(&mut app, Action::Scan);
    read(&mut app, &psbt);
    faraday_core::testkit::settle(&mut app);
    // No card in between: the PSBT is in Files and its spend is open,
    // where nothing is signed before the review's Continue.
    assert_eq!(app.sheet, None);
    assert!(app.scan().is_none());
    assert_eq!(app.screen, Screen::Spend);
    assert!(app.spend().is_some());
    assert!(app.session.keys.is_empty());
    assert_eq!(app.inbox().last().map(|i| i.bytes.to_vec()), Some(psbt));
}

#[test]
fn with_a_key_loaded_a_stick_plugged_in_while_scanning_waits_then_asks_to_lock() {
    let mut app = shown();
    tap(&mut app, Action::Scan);
    read(&mut app, SEEDQR.as_bytes());
    without_passphrase(&mut app);
    assert_eq!(app.session.keys.len(), 1);
    app.press(Action::Scan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    while app.poll_command().is_some() {}
    stick(&mut app, true);
    // The scanner is not replaced with its camera still on.
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(app.scan().is_some());
    app.press(Action::Cancel);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Lock));
    assert!(!app.clean());
}

// ---------------------------------------------------------------------
// §1.1 and §1.2: Home is the Start section, a
// lead tile when something is under way, then the four ways in.
// ---------------------------------------------------------------------

/// `shown()` with `files` already in the Inbox.
fn with_inbox(files: Vec<(&str, Vec<u8>)>) -> Faraday {
    let mut app = shown();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(
            files.into_iter().map(|(n, b)| (n.to_string(), b)).collect(),
        ),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let _ = app.frame();
    app
}

#[test]
fn home_lead_rule_1_a_spend_under_way_continues_it() {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
    let mut app = with_inbox(vec![("spend.psbt", psbt)]);
    app.press(Action::StartSpend(0));
    assert!(app.spend().is_some());
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    assert!(
        app.offers(Action::Nav(Screen::Spend)),
        "Home leads with Continue signing"
    );
}

#[test]
fn home_lead_rule_2_a_psbt_in_files_offers_to_sign_it() {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
    let app = with_inbox(vec![("spend.psbt", psbt)]);
    assert!(
        app.offers(Action::StartSpend(0)),
        "Home leads with Sign {{name}}"
    );
}

#[test]
fn home_lead_rule_5_a_locked_vault_offers_to_unlock_it() {
    use faraday_core::vault::VaultAction as V;
    let vault = faraday_core::testkit::test_vault().unwrap();
    let app = with_inbox(vec![("vault.kdbx", vault)]);
    assert!(
        app.offers(Action::Vault(V::Open(0))),
        "Home leads with Unlock {{name}}"
    );
}

#[test]
fn home_lead_rule_5_with_a_stick_in_asks_to_pull_it_first() {
    use faraday_core::vault::VaultAction as V;
    let vault = faraday_core::testkit::test_vault().unwrap();
    let mut app = with_inbox(vec![("vault.kdbx", vault)]);
    stick(&mut app, true);
    assert!(
        !app.offers(Action::Vault(V::Open(0))),
        "no target: the stick must be pulled first"
    );
}

/// A minimal boot stick: a plain file and the test vault, enough to bring
/// up `app.import` without a PSBT in it, so rule 3 alone leads.
fn boot_stick(files: &[(String, Vec<u8>)]) -> StickInfo {
    StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("boot"),
        label: faraday_text::Text::of("FARADAY"),
        boot: true,
        files: files
            .iter()
            .map(|(n, b)| (faraday_text::Text::of(n), b.len() as u64))
            .collect(),
        vaults: Vec::new(),
    }
}

/// Answers every read the boot import asks for.
fn pump(app: &mut Faraday, files: &[(String, Vec<u8>)]) {
    while let Some(c) = app.poll_storage() {
        if let faraday_core::StorageCommand::Read { stick, name } = c {
            let bytes = files.iter().find(|(n, _)| *n == *name).unwrap().1.clone();
            app.storage(StorageEvent::Read {
                stick,
                name,
                bytes: bytes.into(),
            });
        }
    }
}

#[test]
fn home_lead_rule_3_an_import_waiting() {
    let files = vec![
        ("notes.txt".to_string(), b"a note".to_vec()),
        (
            "vault.kdbx".to_string(),
            faraday_core::testkit::test_vault().unwrap(),
        ),
    ];
    let mut app = shown();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    pump(&mut app, &files);
    let _ = app.frame();
    assert!(app.import().is_some());
    // The sheet comes up over Home on arrival; Later leaves it waiting,
    // and Home leads with it once the sheet is closed.
    app.press(faraday_core::boot_import::LATER);
    let _ = app.frame();
    assert!(
        app.offers(faraday_core::boot_import::OPEN),
        "Home does not lead with the import, stick still in"
    );
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Import from Boot stick"),
        "{texts:?}"
    );
    // Imported and put off again, pulled: Unlock {name}, the stick's
    // vault, and the tap reopens the sheet with its passphrase field.
    app.press(faraday_core::boot_import::OPEN);
    app.press(Action::Import(faraday_core::boot_import::ImportAction::Go));
    pump(&mut app, &files);
    app.press(faraday_core::boot_import::LATER);
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "vault.kdbx"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Unlock"), "{texts:?}");
    assert!(app.offers(faraday_core::boot_import::OPEN));
    app.press(faraday_core::boot_import::OPEN);
    assert_eq!(app.sheet, Some(Sheet::Import));
    let _ = app.frame();
    assert!(app.offers(Action::Import(
        faraday_core::boot_import::ImportAction::Submit
    )));
}

#[test]
fn home_lead_rule_7_with_nothing_loaded_offers_to_make_a_wallet() {
    let app = shown();
    assert!(app.offers(Action::CreateWallet));
}

/// Home's buttons for making and restoring a wallet read "New wallet" and
/// "Restore", and the empty Wallets tab "Make a new wallet" and "Restore an
/// existing wallet".
#[test]
fn home_and_empty_wallets_name_the_two_flows() {
    let mut app = shown();
    let joined = app.drawn_texts().join(" ");
    assert!(joined.contains("New wallet"), "{joined}");
    assert!(joined.contains("Restore"), "{joined}");
    assert!(joined.contains("Spend from backup"), "{joined}");
    assert!(
        !joined.contains("Create a wallet"),
        "the old name is gone: {joined}"
    );
    assert!(
        !joined.contains("Load or restore a wallet"),
        "the old name is gone: {joined}"
    );
    tap(&mut app, Action::Nav(Screen::WalletsStart));
    let joined = app.drawn_texts().join(" ");
    assert!(
        joined.contains("Make a new wallet"),
        "the empty Wallets tab: {joined}"
    );
    assert!(
        joined.contains("Restore an existing wallet"),
        "the empty Wallets tab: {joined}"
    );
}

/// Home's Start with nothing under way is its four ways in, each opening
/// its flow, whose way back is Home: Make a new wallet, Restore an
/// existing wallet, Spend from a backup, and Check.
#[test]
fn homes_four_ways_in_open_their_flows_and_lead_back_home() {
    for (action, screen) in [
        (Action::CreateWallet, Screen::Create),
        (Action::RestoreWallet, Screen::Restore),
        (Action::Nav(Screen::SpendBackup), Screen::SpendBackup),
        (Action::Nav(Screen::Check), Screen::Check),
    ] {
        let mut app = shown();
        assert!(app.offers(action), "{action:?} is on Home");
        tap(&mut app, action);
        assert_eq!(app.screen, screen);
        // Back steps back to the flow's first step, then leaves it
        // (F104).
        for _ in 0..4 {
            if app.screen != screen {
                break;
            }
            app.event(Event::Back);
            let _ = app.frame();
        }
        assert_eq!(app.screen, Screen::Home, "back from {screen:?}");
    }
}

/// Opened from Wallets, the same flows lead back to Wallets.
#[test]
fn the_flows_opened_from_wallets_lead_back_to_wallets() {
    for (action, screen) in [
        (Action::CreateWallet, Screen::Create),
        (Action::RestoreWallet, Screen::Restore),
        (Action::Nav(Screen::SpendBackup), Screen::SpendBackup),
    ] {
        let mut app = shown();
        app.press(Action::Nav(Screen::WalletsStart));
        tap(&mut app, action);
        assert_eq!(app.screen, screen);
        app.event(Event::Back);
        let _ = app.frame();
        assert_eq!(app.screen, Screen::WalletsStart, "back from {screen:?}");
    }
}

/// Home's fourth tile is Check whether or not a vault is in Files.
#[test]
fn homes_fourth_tile_is_check_with_or_without_a_vault() {
    use faraday_core::vault::VaultAction as V;
    let vault = faraday_core::testkit::test_vault().unwrap();
    let app = with_inbox(vec![("vault.kdbx", vault)]);
    assert!(app.offers(Action::Nav(Screen::Check)));
    assert!(!app.offers(Action::Vault(V::Create)));
}

/// The rail lists Check, so Home on a rail-size window draws no tile for
/// it; the phone-size window's bar lacks Check, so its Home keeps the tile.
#[test]
fn home_draws_no_check_tile_where_the_rail_lists_check() {
    let count = |app: &Faraday| {
        app.offered_actions()
            .into_iter()
            .filter(|a| *a == Action::Nav(Screen::Check))
            .count()
    };
    let rail = sized(1024, 768, 160);
    assert_eq!(count(&rail), 1, "only the rail's own destination");
    let phone = sized(400, 800, 160);
    assert_eq!(count(&phone), 1, "the tile, the bar having no Check");
}

/// A stick for the tile tests.
fn tile_stick(id: &str, boot: bool, files: &[&str]) -> StickInfo {
    StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of(id),
        label: faraday_text::Text::of(&id.to_uppercase()),
        boot,
        files: files
            .iter()
            .map(|n| (faraday_text::Text::of(n), 10))
            .collect(),
        vaults: Vec::new(),
    }
}

/// Whether Home offers "Files on" a stick.
fn offers_files_on(app: &mut Faraday) -> bool {
    app.drawn_texts().iter().any(|t| t.starts_with("Files on "))
}

/// Stick visit joins Home's tiles while a stick that has something to
/// offer is attached and nothing is held: not for a boot stick that is
/// empty or holds only the settings file.
#[test]
fn home_offers_files_on_the_stick_with_one_attached() {
    let mut app = shown();
    app.storage(StorageEvent::Sticks(vec![tile_stick("a", true, &[])]));
    assert_eq!(app.screen, Screen::Home);
    assert!(!offers_files_on(&mut app), "an empty boot stick: no tile");

    let mut app = shown();
    app.storage(StorageEvent::Sticks(vec![tile_stick(
        "a",
        true,
        &[faraday_core::stick::settings::FILE],
    )]));
    assert_eq!(app.screen, Screen::Home);
    assert!(
        !offers_files_on(&mut app),
        "only the settings file: no tile"
    );

    // Another stick put in opens its visit.
    let mut app = shown();
    app.storage(StorageEvent::Sticks(vec![tile_stick("a", true, &[])]));
    app.storage(StorageEvent::Sticks(vec![
        tile_stick("a", true, &[]),
        tile_stick("b", false, &[]),
    ]));
    assert_eq!(app.screen, Screen::Files, "a second stick: its visit opens");

    // The boot stick pulled and put in again is visited like any stick.
    let mut app = shown();
    app.storage(StorageEvent::Sticks(vec![tile_stick("a", true, &[])]));
    app.storage(StorageEvent::Sticks(Vec::new()));
    app.storage(StorageEvent::Sticks(vec![tile_stick("a", true, &[])]));
    assert_eq!(app.screen, Screen::Files, "put in again: its visit opens");
}

/// Rule 5 with what the vault held when last seen open (§3.4): after a
/// lock, the Unlock lead's line names it.
#[test]
fn home_lead_rule_5_reads_what_the_vault_held_when_last_seen_open() {
    use faraday_core::StorageCommand;
    use faraday_core::vault::VaultAction as V;
    let vault = faraday_core::testkit::test_vault().unwrap();
    let mut app = with_inbox(vec![("vault.kdbx", vault)]);
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.press(Action::Vault(V::Open(0)));
    for c in faraday_core::testkit::VAULT_PASSPHRASE.chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the test vault did not unlock");
    app.press(Action::Lock);
    let mut boxes = (Vec::new(), Vec::new(), Vec::new());
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes {
            inbox,
            outbox,
            kept,
        } = c
        {
            boxes = (inbox, outbox, kept);
        }
    }

    // The next process.
    let mut app = shown();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(boxes.0),
        outbox: faraday_core::files_from(boxes.1),
        kept: boxes.2,
    });
    let _ = app.frame();
    assert!(
        app.offers(Action::Vault(V::Open(0))),
        "Home leads with Unlock {{name}}"
    );
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Test vault"),
        "the lead names the vault it remembers: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains("wallets · key ")),
        "the lead's line is what the vault held: {texts:?}"
    );
}

/// Answers every write a stick visit sends as written and matched.
fn answer_writes(app: &mut Faraday) {
    while let Some(c) = app.poll_storage() {
        if let faraday_core::StorageCommand::Write { stick, name, .. } = c {
            app.storage(StorageEvent::Written {
                stick,
                wrote_as: name.clone(),
                name,
            });
        }
    }
}

/// Rule 4 (§4.3): after a stick visit wrote something, with nothing
/// loaded, Home leads with what was written, and its tap opens Files.
#[test]
fn home_lead_rule_4_a_receipt_with_nothing_loaded_shows_what_was_written() {
    let wallet = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "spending-wallet.txt")
        .unwrap();
    let mut app = shown();
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: faraday_core::files_from(vec![wallet]),
        kept: Vec::new(),
    });
    stick(&mut app, true);
    app.press(Action::Nav(Screen::Files));
    app.press(Action::VisitWrite);
    answer_writes(&mut app);
    stick(&mut app, false);
    app.press(Action::Nav(Screen::Home));
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Written to STICK"),
        "Home leads with the receipt: {texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains("spending-wallet.txt")),
        "the lead's line names the files: {texts:?}"
    );
    tap(&mut app, Action::Nav(Screen::Files));
    assert_eq!(app.screen, Screen::Files);
}

/// Rule 6 (§3.5): a vault changed since it was written leads with
/// writing it, and the tap opens Files, where the visit is.
#[test]
fn home_lead_rule_6_a_vault_changed_since_written_offers_to_write_it() {
    let c = faraday_core::testkit::compute();
    use faraday_core::vault::VaultAction as V;
    let vault = faraday_core::testkit::test_vault().unwrap();
    let mut app = with_inbox(vec![("vault.kdbx", vault)]);
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    // A key the test vault does not hold, to save into it.
    app.session
        .keys
        .add_words(
            &c,
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon \
             abandon about",
            "k",
            None,
        )
        .unwrap();
    app.press(Action::Vault(V::Open(0)));
    for c in faraday_core::testkit::VAULT_PASSPHRASE.chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the test vault did not unlock");
    app.press(Action::Vault(V::LoadTicked));
    faraday_core::testkit::keep_published(&mut app);
    app.press(Action::Vault(V::AddKind(0)));
    app.press(Action::Vault(V::SaveKey(0)));
    app.press(Action::Nav(Screen::Home));
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Write"),
        "Home leads with writing the vault: {texts:?}"
    );
    tap(&mut app, Action::Nav(Screen::Files));
    assert_eq!(app.screen, Screen::Files);
}

/// `shown()` with the Spending test wallet loaded from Files.
fn with_a_wallet() -> Faraday {
    let wallet = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "spending-wallet.txt")
        .unwrap();
    let mut app = with_inbox(vec![("spending-wallet.txt", wallet.1)]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    assert_eq!(app.session.wallets.len(), 1, "the wallet did not load");
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app
}

/// Backups joins Home's tiles once a wallet is known, and opens the
/// Backups screen; a stick attached with nothing held adds Stick visit
/// beside it.
#[test]
fn home_offers_backups_once_a_wallet_is_known() {
    let mut app = with_a_wallet();
    assert!(
        app.offers(Action::Nav(Screen::Backups)),
        "a wallet known: Backups"
    );
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "1 not backed up"),
        "the tile counts the wallets with no plan: {texts:?}"
    );
    tap(&mut app, Action::Nav(Screen::Backups));
    assert_eq!(app.screen, Screen::Backups);

    app.press(Action::Nav(Screen::Home));
    stick(&mut app, true);
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    assert!(app.offers(Action::Nav(Screen::Backups)));
}

/// The small Home's grid has Backups once a wallet is known, and not
/// before.
#[test]
fn the_small_home_offers_backups_once_a_wallet_is_known() {
    let small = |app: &mut Faraday| {
        app.event(Event::Display(DisplayInfo {
            width: 480,
            height: 640,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: faraday_shell_api::SecureHardware::None,
            boot: faraday_shell_api::BootState::Unknown,
            memory_mib: None,
        }));
        let _ = app.frame();
    };
    let mut app = shown();
    small(&mut app);
    assert!(!app.offers(Action::Nav(Screen::Backups)), "no wallet known");
    let mut app = with_a_wallet();
    small(&mut app);
    assert!(app.offers(Action::Nav(Screen::Backups)));
}

/// The Pi's Home names a wallet that needs something now in the box of
/// what is loaded, and folds the wallets that need nothing into one line.
#[test]
fn the_pi_s_home_names_a_wallet_that_needs_something_and_folds_the_rest() {
    let pi = |app: &mut Faraday| {
        app.event(Event::Display(DisplayInfo {
            width: 480,
            height: 640,
            dpi: 160,
            inset_bottom: 0,
            inset_top: 0,
            buttons: 0,
            camera_fixed: false,
            secure: faraday_shell_api::SecureHardware::None,
            boot: faraday_shell_api::BootState::Unknown,
            memory_mib: None,
        }));
    };
    let mut app = with_a_wallet();
    pi(&mut app);
    let name = app.session.wallets[0].name.to_string();
    let shown = crate::common::texts(&mut app);
    assert!(
        !shown.iter().any(|t| t.contains(&name)),
        "a wallet that needs nothing is named: {shown:?}"
    );
    assert!(shown.iter().any(|t| t == "All wallets"), "{shown:?}");
    assert!(shown.iter().any(|t| t == "1"), "{shown:?}");

    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
    let wallet = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "spending-wallet.txt")
        .unwrap();
    let mut app = with_inbox(vec![
        ("spending-wallet.txt", wallet.1),
        ("spend.psbt", psbt),
    ]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    pi(&mut app);
    let shown = crate::common::texts(&mut app);
    assert!(
        shown.iter().any(|t| t.contains(&name)),
        "Home does not name {name}: {shown:?}"
    );
}

/// Files waiting to be written to the stick show on the Pi's Home.
#[test]
fn the_pi_s_home_shows_files_waiting_to_be_written() {
    let mut app = with_a_wallet();
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    app.outbox.push(faraday_core::Item {
        name: faraday_text::Text::of("a.psbt"),
        bytes: (Vec::new()).into(),
        kind: faraday_core::wallet::FileKind::Psbt,
        secret: false,
        picture: None,
        sha: None,
        facts: None,
        wallet: None,
        psbt: None,
        outpoints: Vec::new(),
        unsigned_txid: None,
    });
    let shown = crate::common::texts(&mut app);
    assert!(
        shown
            .iter()
            .any(|t| t.starts_with("1 file to write to the stick")),
        "{shown:?}"
    );
}

/// With nothing loaded Home leads with Make a new wallet and shows
/// Start's tiles.
#[test]
fn with_nothing_loaded_the_three_first_actions_stand_as_headings_only() {
    let mut app = shown();
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t == "Start"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "Make"), "no pill: {texts:?}");
    assert!(
        !texts.iter().any(|t| t == "From its backups"),
        "no small line: {texts:?}"
    );
    for a in FIRST {
        assert!(app.offers(a), "{a:?}");
    }
}

/// A key no wallet uses is a row under Keys that opens the key, with no
/// Make a wallet button and no lead, and the three first actions stand where
/// they do.
#[test]
fn a_key_with_no_wallet_is_a_row_that_opens_the_key() {
    let mut app = shown();
    tap(&mut app, Action::Scan);
    read(&mut app, SEEDQR.as_bytes());
    without_passphrase(&mut app);
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::Nav(Screen::Home));
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.contains("No wallet uses it")),
        "{texts:?}"
    );
    assert!(has_key_row(&app, fp));
    assert!(!app.offers(Action::KeyWallet(fp, 1)));
    for a in FIRST {
        assert!(app.offers(a), "{a:?} stands with the key");
    }
}

/// With a wallet loaded the three first actions still stand, there is no
/// Sign button, the wallet has its row though it needs nothing, and there is
/// no All wallets row.
#[test]
fn a_wallet_with_nothing_to_do_has_a_row_and_there_is_no_all_wallets_row() {
    let mut app = with_a_wallet();
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t == "Sign"), "{texts:?}");
    for a in FIRST {
        assert!(app.offers(a), "{a:?} stands with a wallet loaded");
    }
    assert!(has_wallet_row(&app, 0), "every wallet has a row");
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t == "All wallets"), "{texts:?}");
}

/// A wallet with a PSBT in Files to sign has a Waiting row for the PSBT with
/// Sign, and its own row, which has no Sign; Sign starts the spend.
#[test]
fn a_wallet_with_a_psbt_to_sign_is_home_s_lead_and_sign_opens_its_spend() {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
    let wallet = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "spending-wallet.txt")
        .unwrap();
    let mut app = with_inbox(vec![
        ("spending-wallet.txt", wallet.1),
        ("spend.psbt", psbt),
    ]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert_eq!(
        texts.iter().filter(|t| *t == "Sign").count(),
        1,
        "{texts:?}"
    );
    assert!(texts.iter().any(|t| t == "A PSBT to sign"), "{texts:?}");
    assert!(has_wallet_row(&app, 0));
    let sign = app
        .offered_actions()
        .into_iter()
        .find(|a| matches!(a, Action::StartSpend(_)))
        .expect("Sign");
    tap(&mut app, sign);
    assert!(app.spend().is_some());
}

/// With a wallet loaded each first action still opens its flow: as a pill
/// on the wide layout and at 1024 wide, through the Start menu on the
/// rail's narrower windows.
#[test]
fn with_a_wallet_loaded_each_first_action_opens_its_flow() {
    let flows = [Screen::Create, Screen::Restore, Screen::SpendBackup];
    for (action, screen) in FIRST.into_iter().zip(flows) {
        let mut app = short_with_a_wallet(1280, 800);
        tap(&mut app, action);
        assert_eq!(app.screen, screen, "1280x800 {action:?}");
    }
    for (width, height) in [(900, 700), (720, 900), (400, 500)] {
        for (way, screen) in WAYS.into_iter().zip(flows) {
            let mut app = short_with_a_wallet(width, height);
            tap(&mut app, Action::NavMenu(MenuPress::Start));
            tap(&mut app, Action::NavMenu(MenuPress::Way(way)));
            assert_eq!(app.screen, screen, "{width}x{height} {way:?}");
        }
    }
}

const WAYS: [Way; 3] = [Way::New, Way::Restore, Way::Spend];

/// On a rail window under 1020 wide the Start menu closes by Escape, by a
/// press outside it and by Back, and is drawn again by the button.
#[test]
fn the_start_menu_closes_by_escape_back_and_a_press_outside() {
    let row = Action::NavMenu(MenuPress::Way(Way::New));
    for (width, height) in [(900, 700), (400, 500)] {
        let mut app = short_with_a_wallet(width, height);
        tap(&mut app, Action::NavMenu(MenuPress::Start));
        assert!(app.offers(row), "{width}x{height} opens");
        app.event(Event::Key(faraday_shell_api::Key::Escape));
        let _ = app.frame();
        assert!(!app.offers(row), "{width}x{height} Escape");
        tap(&mut app, Action::NavMenu(MenuPress::Start));
        app.press(Action::Back);
        let _ = app.frame();
        assert!(!app.offers(row), "{width}x{height} Back");
        tap(&mut app, Action::NavMenu(MenuPress::Start));
        for phase in [TouchPhase::Down, TouchPhase::Up] {
            app.event(Event::Touch {
                x: 2,
                y: height - 2,
                phase,
            });
        }
        let _ = app.frame();
        assert!(!app.offers(row), "{width}x{height} outside");
        assert_eq!(app.screen, Screen::Home, "{width}x{height}");
    }
}

/// The Start menu is a menu over the page: at 400x500 and 900x700 each of
/// its rows is in the window whole and 48 tall.
#[test]
fn the_start_menu_rows_are_whole_in_the_window_and_48_tall() {
    for (width, height) in [(900u16, 700u16), (400, 500)] {
        let mut app = short_with_a_wallet(width, height);
        tap(&mut app, Action::NavMenu(MenuPress::Start));
        for way in WAYS {
            let row = Action::NavMenu(MenuPress::Way(way));
            let (x, y, w, h) = app.hit_box(row).expect("row");
            assert_eq!(h, 48, "{width}x{height} {way:?} height");
            assert!(
                x + w <= width && y + h <= height,
                "{width}x{height} {way:?} inside: {x},{y} {w}x{h}"
            );
        }
    }
}

/// At 400 by 500 the search and the Start button are reachable.
#[test]
fn at_400_by_500_the_search_and_the_start_button_are_reachable() {
    let mut app = short_with_a_wallet(400, 500);
    for a in [
        Action::TypeHere(faraday_core::edit::Here::HomeFind),
        Action::NavMenu(MenuPress::Start),
    ] {
        assert!(scrolled_to(&mut app, 400, 500, a), "{a:?}");
    }
    let (_, _, w, h) = app.hit_box(Action::NavMenu(MenuPress::Start)).unwrap();
    assert!(w >= 48 && h >= 48, "{w}x{h}");
}

/// Between 600 and 1020 wide the search and the one Start button stand in
/// the bar, side by side and under it.
#[test]
fn between_600_and_1020_wide_the_search_and_start_stand_in_the_bar_apart() {
    for width in [600, 720, 900, 1000] {
        let app = loaded(2, 0, width, 800);
        let a = app
            .hit_box(Action::TypeHere(faraday_core::edit::Here::HomeFind))
            .unwrap();
        let b = app.hit_box(Action::NavMenu(MenuPress::Start)).unwrap();
        assert!(a.1 + a.3 <= 64 && b.1 + b.3 <= 64, "{width}: {a:?} {b:?}");
        assert!(a.0 + a.2 <= b.0, "{width}: {a:?} overlaps {b:?}");
        assert!(a.2 >= 200 || width < 700, "{width}: field {}", a.2);
    }
}

/// A key on Home says its fingerprint and opens itself; there is no Make a
/// wallet button and no All keys row.
#[test]
fn a_key_on_home_opens_itself_and_has_no_make_a_wallet_or_all_keys() {
    {
        let (width, height) = (1280, 800);
        let mut app = on_test_net(width, height);
        tap(&mut app, Action::Scan);
        read(&mut app, SEEDQR.as_bytes());
        without_passphrase(&mut app);
        let fp = app.session.keys[0].fingerprint.0;
        app.press(Action::Nav(Screen::Home));
        let texts = app.drawn_texts();
        assert!(
            !texts.iter().any(|t| t == "Make a wallet"),
            "{width}: {texts:?}"
        );
        assert!(
            !texts.iter().any(|t| t.contains("passphrase")),
            "{width}: no passphrase mark: {texts:?}"
        );
        assert!(has_key_row(&app, fp));
        let texts = app.drawn_texts();
        assert!(!texts.iter().any(|t| t == "All keys"), "{texts:?}");
        // A press on the row opens the key's details where the card has
        // them, Explore where it has none.
        let row = key_row(&app, fp);
        tap(&mut app, row);
        let opened = app.offers(Action::ExploreKey(fp)) && app.screen == Screen::Home;
        assert!(opened || app.screen == Screen::Explore, "{width}");
    }
}

#[test]
fn a_key_loaded_with_a_passphrase_is_marked_on_home() {
    let mut app = on_test_net(1280, 800);
    tap(&mut app, Action::Scan);
    read(&mut app, SEEDQR.as_bytes());
    app.press(Action::SeedRead(faraday_core::seeds::read::Choice::AddPass));
    for field in [0, 1] {
        app.press(Action::SeedRead(faraday_core::seeds::read::Choice::Field(
            field,
        )));
        for c in "extra".chars() {
            app.event(Event::Key(faraday_shell_api::Key::Char(c)));
        }
    }
    faraday_core::testkit::pause(&mut app);
    app.press(Action::SeedRead(
        faraday_core::seeds::read::Choice::WithPass,
    ));
    faraday_core::testkit::keep_published(&mut app);
    let _ = app.frame();
    app.press(Action::Nav(Screen::Home));
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.contains("passphrase")),
        "the key says so: {texts:?}"
    );
}

/// A search typed takes the page: the first actions, wallets, keys and
/// Backups are not drawn, and clearing it brings them back.
#[test]
fn a_search_typed_on_the_wide_home_takes_the_page_and_clearing_it_brings_the_rest_back() {
    for (width, height) in [(1280, 800), (720, 900)] {
        let mut app = short_with_a_wallet(width, height);
        tap(&mut app, Action::Scan);
        read(&mut app, SEEDQR.as_bytes());
        without_passphrase(&mut app);
        // The phone-size window keeps its box and All keys.
        let key = if width > 1000 {
            key_row(&app, app.session.keys[0].fingerprint.0)
        } else {
            Action::Explore
        };
        app.press(Action::Nav(Screen::Home));
        // The wide layout's sidebar still lists Backups.
        let rest: Vec<Action> = if width > 1000 {
            vec![Action::CreateWallet, key]
        } else {
            vec![
                Action::NavMenu(MenuPress::Start),
                Action::Nav(Screen::Backups),
                key,
            ]
        };
        for &a in &rest {
            assert!(app.offers(a), "{width}: {a:?}");
        }
        type_on_home(&mut app, "send");
        // The top row stays while the results take the page.
        assert!(app.offers(rest[0]), "{width}: the row stays");
        for &a in &rest[1..] {
            // The wide layout keeps This session's card, the results under
            // it; the phone-size window's results take the page.
            assert_eq!(app.offers(a), width > 1000, "{width}: {a:?} while typed");
        }
        assert!(app.where_offered(Action::Catalog(sign_tile())).is_some());
        app.event(Event::Key(faraday_shell_api::Key::Escape));
        let _ = app.frame();
        for &a in &rest {
            assert!(app.offers(a), "{width}: {a:?} is back");
        }
    }
}

fn window(width: u16, height: u16, dpi: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

fn type_on_home(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    // The frame, and the one the page follows the results with.
    let _ = app.frame();
    let _ = app.frame();
}

fn sign_tile() -> u8 {
    faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Sign a transaction")
        .expect("the Sign a transaction tile") as u8
}

fn wallet_tile() -> u8 {
    faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Make a new wallet")
        .expect("the Make a new wallet tile") as u8
}

/// Leaving the flow a Home search's result opened, by Cancel or by Back
/// from its first step, lands on Home with the words still typed and
/// the same rows listed.
#[test]
fn leaving_a_flow_begun_from_a_search_result_returns_to_the_results() {
    for leave in [Action::CancelFlow, Action::Back] {
        for mut app in [shown(), window(400, 800, 160)] {
            type_on_home(&mut app, "wallet");
            let rows = app.offers(Action::Catalog(wallet_tile()));
            assert!(rows, "the result is listed");
            tap(&mut app, Action::Catalog(wallet_tile()));
            assert_eq!(app.screen, Screen::Create);
            assert!(faraday_core::testkit::home_find(&app).is_empty());
            let _ = app.frame();
            app.press(leave);
            if app.sheet == Some(Sheet::CancelFlow(faraday_core::in_progress::Going::Create)) {
                app.press(Action::CancelFlowNow);
            }
            assert_eq!(app.screen, Screen::Home);
            assert_eq!(faraday_core::testkit::home_find(&app), "wallet");
            let _ = app.frame();
            assert!(app.offers(Action::Catalog(wallet_tile())));
        }
    }
}

fn tile_named(name: &str) -> u8 {
    faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == name)
        .expect("the tile") as u8
}

/// A page a search result opens: Back lands on the results; once
/// another screen was opened after it, Back lands on Home without them.
#[test]
fn back_from_a_page_a_search_result_opened_returns_to_the_results() {
    let mut app = shown();
    type_on_home(&mut app, "check");
    tap(&mut app, Action::Catalog(tile_named("Check")));
    assert_eq!(app.screen, Screen::Check);
    let _ = app.frame();
    app.press(Action::Back);
    assert_eq!(app.screen, Screen::Home);
    assert_eq!(faraday_core::testkit::home_find(&app), "check");

    let mut app = shown();
    type_on_home(&mut app, "check");
    tap(&mut app, Action::Catalog(tile_named("Check")));
    app.press(Action::Nav(Screen::WalletsStart));
    let _ = app.frame();
    app.press(Action::Back);
    app.press(Action::Back);
    assert_eq!(app.screen, Screen::Home);
    assert!(faraday_core::testkit::home_find(&app).is_empty());
}

/// A second search between: Cancel lands on the words of the last one.
#[test]
fn a_second_search_between_returns_to_the_last_results() {
    let mut app = shown();
    type_on_home(&mut app, "wallet");
    tap(&mut app, Action::Catalog(wallet_tile()));
    let _ = app.frame();
    app.press(Action::Back);
    if app.sheet == Some(Sheet::CancelFlow(faraday_core::in_progress::Going::Create)) {
        app.press(Action::CancelFlowNow);
    }
    assert_eq!(faraday_core::testkit::home_find(&app), "wallet");
    app.event(Event::Key(faraday_shell_api::Key::Escape));
    type_on_home(&mut app, "check");
    tap(&mut app, Action::Catalog(tile_named("Check")));
    let _ = app.frame();
    app.press(Action::Back);
    assert_eq!(faraday_core::testkit::home_find(&app), "check");
}

/// A lock ends the place: no words come back.
#[test]
fn a_lock_drops_the_search_a_flow_was_begun_from() {
    let mut app = shown();
    type_on_home(&mut app, "wallet");
    tap(&mut app, Action::Catalog(wallet_tile()));
    let _ = app.frame();
    app.press(Action::Lock);
    app.press(Action::Nav(Screen::Home));
    assert!(faraday_core::testkit::home_find(&app).is_empty());
}

/// A flow begun from a wallet's own entry stays as it was: no words.
#[test]
fn a_search_does_not_return_for_a_flow_begun_another_way() {
    let mut app = shown();
    type_on_home(&mut app, "wallet");
    tap(&mut app, Action::Nav(Screen::WalletsStart));
    tap(&mut app, Action::CreateWallet);
    let _ = app.frame();
    app.press(Action::Back);
    if app.sheet == Some(Sheet::CancelFlow(faraday_core::in_progress::Going::Create)) {
        app.press(Action::CancelFlowNow);
    }
    assert_eq!(app.screen, Screen::WalletsStart);
}

/// The same flow begun from Home's own button lands on Home with no
/// search, as before.
#[test]
fn leaving_a_flow_begun_from_a_home_button_returns_to_home_without_a_search() {
    let mut app = shown();
    tap(&mut app, Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    let _ = app.frame();
    app.press(Action::Back);
    if app.sheet == Some(Sheet::CancelFlow(faraday_core::in_progress::Going::Create)) {
        app.press(Action::CancelFlowNow);
    }
    assert_eq!(app.screen, Screen::Home);
    assert!(faraday_core::testkit::home_find(&app).is_empty());
}

#[test]
fn typing_on_home_searches_on_a_wide_and_a_phone_window() {
    for mut app in [shown(), window(400, 800, 160)] {
        type_on_home(&mut app, "send");
        assert_eq!(faraday_core::testkit::home_find(&app), "send");
        assert!(app.where_offered(Action::Catalog(sign_tile())).is_some());
        // The result opens its flow (the scanner, with no PSBT in Files),
        // and Home is Home again.
        tap(&mut app, Action::Catalog(sign_tile()));
        assert_eq!(app.sheet, Some(faraday_core::Sheet::Scan));
        assert!(faraday_core::testkit::home_find(&app).is_empty());
    }
}

#[test]
fn pressing_homes_search_bar_in_a_narrow_window_brings_the_keyboard_up_to_type_into_it() {
    let bar = Action::TypeHere(faraday_core::edit::Here::HomeFind);
    let mut app = window(1081, 2400, 420);
    let _ = app.frame();
    // Typing with no press leaves the keyboard down.
    type_on_home(&mut app, "x");
    assert!(app.keyboard_top().is_none());
    tap(&mut app, bar);
    assert!(app.keyboard_top().is_some());
    assert!(
        app.where_offered(Action::Osk(OskPress::Char('s')))
            .is_some()
    );
    for c in "send".chars() {
        tap(&mut app, Action::Osk(OskPress::Char(c)));
    }
    assert_eq!(faraday_core::testkit::home_find(&app), "xsend");
    // Leaving the page puts it away.
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    assert!(app.keyboard_top().is_none());
}

#[test]
fn pressing_homes_search_bar_in_the_wide_layout_leaves_the_keyboard_down() {
    let mut app = shown();
    tap(
        &mut app,
        Action::TypeHere(faraday_core::edit::Here::HomeFind),
    );
    assert!(app.keyboard_top().is_none());
}

#[test]
fn escape_clears_homes_search() {
    let mut app = shown();
    type_on_home(&mut app, "send");
    app.event(Event::Key(faraday_shell_api::Key::Escape));
    assert!(faraday_core::testkit::home_find(&app).is_empty());
}

#[test]
fn the_cross_in_homes_search_bar_clears_it() {
    for mut app in [shown(), window(400, 800, 160)] {
        type_on_home(&mut app, "send");
        assert!(app.hit_box(Action::HomeSearchClear).is_some());
        tap(&mut app, Action::HomeSearchClear);
        assert!(faraday_core::testkit::home_find(&app).is_empty());
        assert!(app.hit_box(Action::HomeSearchClear).is_none());
    }
}

#[test]
fn the_pis_home_has_no_search() {
    let mut app = window(480, 640, 286);
    type_on_home(&mut app, "send");
    assert!(faraday_core::testkit::home_find(&app).is_empty());
    assert!(app.where_offered(Action::Catalog(sign_tile())).is_none());
}

fn shows_card(app: &mut Faraday, title: &str) -> bool {
    app.drawn_texts().iter().any(|t| t == title)
}

#[test]
fn home_shows_a_settings_card_for_any_word_on_it() {
    for (word, card) in [
        ("appearance", "Appearance"),
        ("contrast", "Appearance"),
        ("orange", "Appearance"),
        ("time", "Time"),
        ("about", "About"),
    ] {
        for mut app in [shown(), window(400, 800, 160)] {
            type_on_home(&mut app, word);
            assert!(shows_card(&mut app, card), "{word}");
        }
    }
}

#[test]
fn a_change_made_on_a_card_in_homes_results_shows_on_settings() {
    for mut app in [shown(), window(400, 800, 160)] {
        type_on_home(&mut app, "contrast");
        let mut high = app.theme();
        high.contrast = faraday_core::ui::Contrast::High;
        tap(&mut app, Action::Theme(high));
        assert_eq!(app.theme().contrast, faraday_core::ui::Contrast::High);
        app.press(Action::Nav(Screen::Settings));
        crate::common::settings_section(&mut app, "Appearance");
        let mut standard = high;
        standard.contrast = faraday_core::ui::Contrast::Standard;
        assert!(app.offers(Action::Theme(standard)));
        assert_eq!(app.theme().contrast, faraday_core::ui::Contrast::High);
    }
}

#[test]
fn a_press_in_homes_empty_search_bar_shows_a_blinking_caret_and_typing_goes_in() {
    let mut app = shown();
    let blinks = |app: &mut Faraday| {
        let mut drawn = false;
        let start = app.now_ms();
        for step in 1..=80u64 {
            app.event(Event::Tick {
                now_ms: start + step * 16,
            });
            drawn |= std::iter::from_fn(|| app.poll_command()).any(|c| c == Command::Draw);
            let _ = app.frame();
        }
        drawn
    };
    // Whatever Home was still settling on arrival is done first.
    let _ = blinks(&mut app);
    assert!(
        !blinks(&mut app),
        "nothing blinks before the bar is pressed"
    );
    tap(
        &mut app,
        Action::TypeHere(faraday_core::edit::Here::HomeFind),
    );
    assert!(blinks(&mut app), "the caret blinks once it is pressed");
    type_on_home(&mut app, "send");
    assert_eq!(faraday_core::testkit::home_find(&app), "send");
}

const FIRST: [Action; 3] = [
    Action::CreateWallet,
    Action::RestoreWallet,
    Action::Nav(Screen::SpendBackup),
];

fn wide() -> Faraday {
    shown()
}

fn phone() -> Faraday {
    window(400, 800, 160)
}

#[test]
fn with_nothing_loaded_each_first_action_is_pressed_as_a_whole() {
    let flows = [Screen::Create, Screen::Restore, Screen::SpendBackup];
    for make in [wide as fn() -> Faraday, phone] {
        for (action, screen) in FIRST.into_iter().zip(flows) {
            let mut app = make();
            tap(&mut app, action);
            assert_eq!(app.screen, screen, "{action:?}");
        }
    }
}

/// On the Pi's menu the top pair is Wallets then Check, one under the other
/// in the one column of rows; a
/// phone-size window keeps Check first (Wallets there is on the bar).
#[test]
fn the_pi_s_menu_puts_wallets_left_of_check_and_a_phone_keeps_check_first() {
    let boxes = |app: &Faraday| {
        (
            app.hit_box(Action::Nav(Screen::WalletsStart)).unwrap(),
            app.hit_box(Action::Nav(Screen::Check)).unwrap(),
        )
    };
    let (w, c) = boxes(&window(480, 640, 286));
    assert_eq!(w.0, c.0, "one column");
    assert!(w.1 < c.1, "Wallets is above Check");
    let (w, c) = boxes(&window(400, 800, 160));
    assert!(c.1 < w.1, "Check is in the page, Wallets on the bar below");
}

/// The bar stands first, level with the three buttons, and stays where it is
/// while a search is typed and its results scroll; the buttons stay with it.
#[test]
fn the_wide_homes_bar_stays_where_it_was_while_a_search_is_typed() {
    let bar = Action::TypeHere(faraday_core::edit::Here::HomeFind);
    let mut app = wide();
    let before = app.hit_box(bar);
    assert!(before.is_some());
    let first = app.hit_box(FIRST[0]).unwrap();
    assert!(before.unwrap().0 < first.0, "the bar is before the buttons");
    assert_eq!(before.unwrap().1, first.1, "level");
    assert_eq!(before.unwrap().3, first.3, "as tall");
    type_on_home(&mut app, "send");
    assert_eq!(before, app.hit_box(bar));
    assert!(app.offers(Action::CreateWallet));
    // Scrolling the results moves the bar no more.
    app.event(Event::Scroll {
        x: 300,
        y: 700,
        dy: 3000,
    });
    let _ = app.frame();
    assert_eq!(before, app.hit_box(bar));
}

/// Home with the boot stick's import waiting and nothing loaded.
fn with_an_import_waiting(make: fn() -> Faraday) -> Faraday {
    let files = vec![("notes.txt".to_string(), b"a note".to_vec())];
    let mut app = make();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    pump(&mut app, &files);
    let _ = app.frame();
    app.press(faraday_core::boot_import::LATER);
    let _ = app.frame();
    app
}

#[test]
fn the_import_lead_is_a_card_with_no_pill_and_typing_a_search_leaves_the_actions_out_and_the_card_keeps_it()
 {
    let open = faraday_core::boot_import::OPEN;
    let mut app = with_an_import_waiting(wide);
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t == "Import"), "no pill: {texts:?}");
    assert!(app.offers(open));
    let bar = Action::TypeHere(faraday_core::edit::Here::HomeFind);
    let before = app.hit_box(bar);
    type_on_home(&mut app, "send");
    assert!(app.offers(open), "the lead stays in This session's card");
    assert_eq!(before, app.hit_box(bar));
    app.event(Event::Key(faraday_shell_api::Key::Escape));
    let _ = app.frame();
    tap(&mut app, open);
    assert!(app.import_on_show());
}

#[test]
fn with_something_typed_homes_buttons_stay_and_can_be_pressed() {
    let mut app = shown();
    assert!(app.offers(Action::Nav(Screen::Check)), "on Home at first");
    type_on_home(&mut app, "send");
    assert_eq!(faraday_core::testkit::home_find(&app), "send");
    assert!(app.where_offered(Action::Nav(Screen::Check)).is_some());
    tap(&mut app, Action::Nav(Screen::Check));
    assert_eq!(app.screen, Screen::Check);
}

/// Whether a tick at `now_ms` asks for a frame.
fn tick_draws(app: &mut Faraday, now_ms: u64) -> bool {
    app.event(Event::Tick { now_ms });
    std::iter::from_fn(|| app.poll_command()).any(|c| c == Command::Draw)
}

#[test]
fn a_press_in_homes_search_bar_shows_a_blinking_caret_and_at_rest_none_shows() {
    let bar = Action::TypeHere(faraday_core::edit::Here::HomeFind);
    for mut app in [shown(), window(400, 800, 160)] {
        let _ = app.frame();
        while app.poll_command().is_some() {}
        let at_rest = app.now_ms();
        for k in 1..=100 {
            assert!(
                !tick_draws(&mut app, at_rest + 16 * k),
                "no caret at rest at {k}"
            );
        }
        tap(&mut app, bar);
        let t = app.now_ms();
        while app.poll_command().is_some() {}
        let on = app.frame().rgba.to_vec();
        assert!(tick_draws(&mut app, t + 600), "the caret goes off");
        let off = app.frame().rgba.to_vec();
        assert!(on != off);
        assert!(tick_draws(&mut app, t + 1_100), "and comes back");
    }
}

/// A window of `width` by `height` at 160 dpi with the test wallet loaded.
/// A Testnet app in a `width` by `height` window at 160 dpi.
fn on_test_net(width: u16, height: u16) -> Faraday {
    let mut app = faraday_core::testkit::new_on_test_net();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

fn short_with_a_wallet(width: u16, height: u16) -> Faraday {
    let wallet = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "spending-wallet.txt")
        .unwrap();
    let mut app = on_test_net(width, height);
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![("spending-wallet.txt".to_string(), wallet.1)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let _ = app.frame();
    faraday_core::testkit::load_wallet(&mut app, 0);
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app
}

/// Whether the last frame offers `action` inside a window `height` tall.
fn on_screen(app: &Faraday, action: Action, height: u16) -> bool {
    app.where_offered(action).is_some_and(|(_, y)| y < height)
}

fn scroll_home(app: &mut Faraday, width: u16, dy: i16) {
    app.event(Event::Scroll {
        x: width / 2,
        y: 70,
        dy,
    });
    app.event(Event::ScrollEnd {
        x: width / 2,
        y: 70,
    });
    let _ = app.frame();
}

/// Scrolls Home from its top, a step at a time, until `action` is inside a
/// window `height` tall; whether it got there.
fn scrolled_to(app: &mut Faraday, width: u16, height: u16, action: Action) -> bool {
    for _ in 0..80 {
        if on_screen(app, action, height) {
            return true;
        }
        scroll_home(app, width, 60);
    }
    false
}

/// On a short wide window the bar stays in view and what is under it
/// scrolls, a search typed or not; on a short phone window the whole page
/// scrolls as one, the bar and its results reachable.
#[test]
fn a_home_taller_than_its_window_keeps_the_bar_still_or_scrolls_to_it() {
    let bar = Action::TypeHere(faraday_core::edit::Here::HomeFind);
    let mut app = short_with_a_wallet(1366, 320);
    assert!(on_screen(&app, bar, 320), "the bar is in view");
    let before = app.hit_box(bar);
    assert!(
        scrolled_to(&mut app, 1366, 320, Action::Nav(Screen::Backups)),
        "Backups is out of reach"
    );
    assert_eq!(before, app.hit_box(bar), "the bar stays");
    type_on_home(&mut app, "send");
    assert!(
        scrolled_to(&mut app, 1366, 320, Action::Catalog(sign_tile())),
        "the result is out of reach"
    );
    assert_eq!(before, app.hit_box(bar), "the bar stays");

    let mut app = short_with_a_wallet(400, 500);
    assert!(
        scrolled_to(&mut app, 400, 500, bar),
        "the bar is out of reach"
    );
    type_on_home(&mut app, "send");
    assert_eq!(faraday_core::testkit::home_find(&app), "send");
    assert!(
        scrolled_to(&mut app, 400, 500, Action::Catalog(sign_tile())),
        "the result is out of reach"
    );
}

/// On the phone-size window the places under the bar stay reachable by
/// scrolling, with no search typed and with one.
#[test]
fn the_phone_homes_places_under_the_bar_are_reachable_by_scrolling() {
    let mut app = short_with_a_wallet(400, 500);
    assert!(scrolled_to(
        &mut app,
        400,
        500,
        Action::Nav(Screen::Backups)
    ));
    type_on_home(&mut app, "send");
    assert!(scrolled_to(
        &mut app,
        400,
        500,
        Action::Catalog(sign_tile())
    ));
}

/// Whether Home draws the box of what is loaded (its files line).
fn shows_loaded_box(app: &mut Faraday) -> bool {
    let _ = app.frame();
    app.drawn_texts().iter().any(|t| t.contains("file"))
}

#[test]
fn the_phone_homes_search_is_offered_above_the_three_actions() {
    let mut app = phone();
    let bar = app.hit_box(Action::TypeHere(faraday_core::edit::Here::HomeFind));
    let bar = bar.expect("the search bar is offered");
    for action in FIRST {
        let b = app.hit_box(action).expect("a first action is offered");
        assert!(bar.1 < b.1, "{action:?} is under the bar");
    }
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Search"), "{texts:?}");
}

#[test]
fn a_search_typed_on_the_phone_takes_the_page_and_clearing_it_brings_the_rest_back() {
    let mut app = phone();
    assert!(app.offers(Action::Nav(Screen::Check)));
    type_on_home(&mut app, "send");
    for action in FIRST {
        assert!(!app.offers(action), "{action:?} while typing");
    }
    assert!(!app.offers(Action::Nav(Screen::Check)));
    assert!(app.offers(Action::Catalog(sign_tile())));
    tap(&mut app, Action::HomeSearchClear);
    for action in FIRST {
        assert!(app.offers(action), "{action:?} once cleared");
    }
    assert!(app.offers(Action::Nav(Screen::Check)));
}

#[test]
fn the_box_of_what_is_loaded_is_left_out_with_nothing_loaded_on_the_phone_and_the_pi() {
    for make in [phone as fn() -> Faraday, || window(480, 640, 286)] {
        let mut app = make();
        assert!(!shows_loaded_box(&mut app), "with nothing loaded");
        stick(&mut app, true);
        app.press(Action::Nav(Screen::Home));
        assert!(shows_loaded_box(&mut app), "with a stick in");
    }
}

#[test]
fn on_a_1280_wide_window_the_three_first_actions_stand_side_by_side_and_none_is_cut() {
    let mut app = window(1280, 800, 160);
    let boxes: Vec<_> = FIRST
        .iter()
        .map(|a| app.hit_box(*a).expect("offered"))
        .collect();
    assert!(boxes.iter().all(|b| b.1 == boxes[0].1 && b.3 == boxes[0].3));
    assert!(boxes[0].0 < boxes[1].0 && boxes[1].0 < boxes[2].0);
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t.ends_with('…')), "{texts:?}");
}

/// At 1366 by 768 the search bar and the three buttons stand in the top
/// bar, as tall as the bar, level, and under them the card starts; each
/// button opens its flow.
#[test]
fn at_1366_by_768_the_search_and_buttons_stand_in_the_top_bar_and_the_buttons_open_their_flows() {
    let app = loaded(2, 0, 1366, 768);
    let bar = app
        .hit_box(Action::TypeHere(faraday_core::edit::Here::HomeFind))
        .expect("the bar");
    let buttons: Vec<_> = FIRST
        .iter()
        .map(|a| app.hit_box(*a).expect("button"))
        .collect();
    let row = app
        .hit_box(key_row(&app, key_fp(&app, 0)))
        .expect("a row of the card");
    assert!(bar.1 + bar.3 <= 64 && row.1 >= 64, "{bar:?} {row:?}");
    for b in std::iter::once(&bar).chain(&buttons) {
        assert_eq!((b.1, b.3), (bar.1, 48));
    }
    for (action, screen) in [
        (FIRST[0], Screen::Create),
        (FIRST[1], Screen::Restore),
        (FIRST[2], Screen::SpendBackup),
    ] {
        let mut app = loaded(2, 0, 1366, 768);
        tap(&mut app, action);
        assert_eq!(app.screen, screen, "{action:?}");
    }
}

// ---- what is loaded is on Home, not in the sidebar ----

/// `seeds` seeds and `wallets` single-key wallets over the first of them,
/// on Home, in a `width` by `height` window.
fn loaded(seeds: u8, wallets: usize, width: u16, height: u16) -> Faraday {
    use faraday_bip::bip39::Language;
    let c = faraday_core::testkit::compute();
    let mut app = on_test_net(width, height);
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(Vec::<(String, Vec<u8>)>::new()),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let _ = app.frame();
    for i in 0..seeds {
        let m = faraday_keys::words::from_entropy(Language::English, &[i + 1; 16]).unwrap();
        let words: Vec<&str> = m
            .indices()
            .iter()
            .map(|&w| Language::English.words()[usize::from(w)])
            .collect();
        app.session
            .keys
            .add_words(&c, &words.join(" "), &format!("Seed {i}"), None)
            .unwrap();
    }
    for a in 0..wallets {
        add_wallet_over(&mut app, 0, a as u32, &format!("Wallet {a}"));
    }
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app
}

/// A single-key wallet named `name` over account `a` of key `k`.
fn add_wallet_over(app: &mut Faraday, k: usize, a: u32, name: &str) {
    let c = faraday_core::testkit::compute();
    let text = faraday_core::create::NewKind::NativeSegwit
        .key_text_at(&c, app.session.keys.master_for_tests(k), a)
        .unwrap();
    let d = format!("wpkh({}/<0;1>/*)", &*text);
    app.session
        .add_wallet_confirmed(&c, name, &d, "test", faraday_core::wallet::Origin::Named)
        .unwrap_or_else(|e| panic!("{d}: {}", &*e.text()));
}

/// Home's row for key `fp`: the one that opens its details where the card
/// has them (the wide layout), else the one that opens Explore.
fn key_row(app: &Faraday, fp: [u8; 4]) -> Action {
    let open = Action::RowOpen(faraday_core::Row::Key(fp));
    if app.offers(open) {
        open
    } else {
        Action::ExploreKey(fp)
    }
}

fn has_key_row(app: &Faraday, fp: [u8; 4]) -> bool {
    app.offers(key_row(app, fp))
}

/// Home's row for wallet `w`, as [`key_row`].
fn wallet_row(app: &Faraday, w: usize) -> Action {
    let open = Action::RowOpen(faraday_core::Row::Wallet(w));
    if app.offers(open) {
        open
    } else {
        Action::OpenWallet(w)
    }
}

fn has_wallet_row(app: &Faraday, w: usize) -> bool {
    app.offers(wallet_row(app, w))
}

fn key_fp(app: &Faraday, k: usize) -> [u8; 4] {
    app.session.keys[k].fingerprint.0
}

/// Every key and wallet loaded is a row on the wide Home that opens it, one
/// that needs nothing included; there is no All keys or All wallets row.
#[test]
fn a_few_loaded_keys_and_wallets_all_have_a_row() {
    let mut app = loaded(3, 2, 1366, 1200);
    for k in 0..3 {
        assert!(has_key_row(&app, key_fp(&app, k)), "key {k}");
        assert!(
            !app.offers(Action::KeyWallet(key_fp(&app, k), 1)),
            "key {k}"
        );
    }
    for w in 0..2 {
        assert!(has_wallet_row(&app, w), "wallet {w}");
    }
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t == "All keys"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "All wallets"), "{texts:?}");
    assert!(!app.offers(Action::Explore));
}

/// A key a wallet uses names the wallet in its line; a key no wallet uses
/// says so.
#[test]
fn a_key_names_the_wallet_that_uses_it() {
    let mut app = loaded(2, 1, 1366, 1200);
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t.contains("Used by Wallet 0")),
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|t| t.contains("No wallet uses it")),
        "{texts:?}"
    );
}

/// With many keys no row is dropped: the card grows with its rows, and where
/// the window is short it scrolls within itself instead.
#[test]
fn with_many_keys_every_key_has_a_row() {
    let app = loaded(30, 20, 1366, 4000);
    for k in 0..30 {
        assert!(has_key_row(&app, key_fp(&app, k)), "key {k}");
    }
    let mut short = loaded(30, 20, 1366, 600);
    assert!(has_key_row(&short, key_fp(&short, 0)));
    let _ = short.frame();
    assert!(
        !short.offers(Action::Explore),
        "no All keys row to fall back on"
    );
}

/// A file written to the stick.
fn a_file() -> faraday_core::Item {
    faraday_core::Item {
        name: faraday_text::Text::of("a.psbt"),
        bytes: (Vec::new()).into(),
        kind: faraday_core::wallet::FileKind::Psbt,
        secret: false,
        picture: None,
        sha: None,
        facts: None,
        wallet: None,
        psbt: None,
        outpoints: Vec::new(),
        unsigned_txid: None,
    }
}

/// Home with a PSBT waiting for each of the test wallets, a wallet for each
/// that needs it signed, and a file to write: more rows than a short wide
/// window holds.
fn crowded(height: u16) -> Faraday {
    crowded_at(1366, height)
}

fn crowded_at(width: u16, height: u16) -> Faraday {
    let kits = faraday_core::testkit::kits();
    let mut files: Vec<(String, Vec<u8>)> = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .filter(|(n, _)| n.ends_with("-wallet.txt"))
        .collect();
    let wallets = files.len();
    for kit in &kits {
        if let Ok(p) = faraday_core::testkit::unsigned(kit) {
            files.push((format!("{}.psbt", kit.id), p.to_bytes()));
        }
    }
    let mut app = sized(width, height, 160);
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(files),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let _ = app.frame();
    for _ in 0..wallets {
        faraday_core::testkit::load_wallet(&mut app, 0);
    }
    app.outbox.push(a_file());
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app
}

/// On a short window the first actions keep their place and the card keeps
/// its rows, whether or not it scrolls within itself: the page stays still
/// under a scroll over the card. The key rows drop before the card scrolls.
#[test]
fn the_box_scrolls_within_itself_and_the_first_actions_keep_their_place() {
    for h in [600, 640, 700, 768, 900] {
        let mut app = crowded(h);
        let top = app.where_offered(Action::CreateWallet);
        let all = app
            .where_offered(Action::Nav(Screen::Backups))
            .unwrap_or_else(|| panic!("Backups row at {h}"));
        scroll_over_box(&mut app, 1366, all.1);
        assert_eq!(app.where_offered(Action::CreateWallet), top, "{h}");
        assert!(
            app.where_offered(Action::Nav(Screen::Backups)).is_some(),
            "{h}"
        );
    }
}

/// On the rail's windows the first actions are reachable however many rows
/// there are, and no place tile is drawn: the rail lists them.
#[test]
fn on_the_rail_the_first_actions_are_reachable_and_home_draws_no_place_tiles() {
    for h in [600, 768, 1100] {
        let app = crowded_at(1024, h);
        let top = app.where_offered(Action::Nav(Screen::SpendBackup));
        assert!(top.is_some(), "the first actions are reachable at {h}");
        for dest in [Screen::Settings, Screen::Tools] {
            let n = app
                .offered_actions()
                .into_iter()
                .filter(|a| *a == Action::Nav(dest))
                .count();
            assert!(n <= 1, "{dest:?} is drawn once at most: the rail's own");
        }
    }
}

/// On the rail's and the sidebar's windows a wheel scroll over the box or
/// over the page moves no first action: the page stays still.
#[test]
fn a_crowded_home_on_the_rail_and_sidebar_keeps_the_page_still_under_the_wheel() {
    for (w, h) in [(1024, 600), (1024, 768), (1366, 768)] {
        for over_box in [true, false] {
            let mut app = crowded_at(w, h);
            let before = app.where_offered(Action::Nav(Screen::SpendBackup));
            let all = app.where_offered(Action::Nav(Screen::Files));
            let y = if over_box {
                all.map_or(h / 2, |a| a.1)
            } else {
                200
            };
            scroll_over_box(&mut app, w, y);
            assert_eq!(
                app.where_offered(Action::Nav(Screen::SpendBackup)),
                before,
                "{w}x{h}, over the {}",
                if over_box { "box" } else { "page" }
            );
        }
    }
}

fn scroll_over_box(app: &mut Faraday, width: u16, y: u16) {
    app.event(Event::Scroll {
        x: width / 2,
        y: y + 5,
        dy: 60,
    });
    app.event(Event::ScrollEnd {
        x: width / 2,
        y: y + 5,
    });
    let _ = app.frame();
}

/// Among many wallets that are watch-only, and one the seeds here sign
/// for, every one has a row on Home.
#[test]
fn many_wallets_that_need_nothing_each_have_a_row_on_home() {
    let mut app = loaded(2, 0, 1366, 2400);
    for a in 0..12u32 {
        add_wallet_over(&mut app, 1, a, &format!("Watched {a}"));
    }
    add_wallet_over(&mut app, 0, 0, "Ready");
    // The second seed leaves: its twelve wallets are watch-only now.
    app.session.keys.remove(1);
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    for w in 0..13 {
        assert!(has_wallet_row(&app, w), "wallet {w}");
    }
    assert!(on_screen(&app, Action::Nav(Screen::WalletsStart), 2400));
    tap(&mut app, Action::Nav(Screen::WalletsStart));
    assert!(matches!(app.screen, Screen::Wallets | Screen::WalletsStart));
}

/// A vault open is the Vaults entry's badge, a count, and nothing on Home
/// or in the sidebar says "open".
#[test]
fn a_vault_open_is_the_vaults_badge_not_a_chip_on_home() {
    use faraday_core::vault::VaultAction as V;
    let vault = faraday_core::testkit::test_vault().unwrap();
    let mut app = wide();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![("vault.kdbx".to_string(), vault)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let _ = app.frame();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.press(Action::Vault(V::Open(0)));
    for c in faraday_core::testkit::VAULT_PASSPHRASE.chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults().open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults().open.len(), 1, "the test vault did not unlock");
    app.press(Action::Vault(V::LoadTicked));
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    let badges = drawn_boxes(&mut app, "1");
    let vaults = app.hit_box(Action::Nav(Screen::Vaults)).unwrap();
    assert!(
        badges.iter().any(|b| inside(*b, vaults)),
        "no count on Vaults: {badges:?}"
    );
    let texts = app.drawn_texts().plain();
    assert!(!texts.iter().any(|t| t.contains("open")), "{texts:?}");
    let mut none = wide();
    none.press(Action::Nav(Screen::Home));
    let _ = none.frame();
    let vaults = none.hit_box(Action::Nav(Screen::Vaults)).unwrap();
    let badges = drawn_boxes(&mut none, "1");
    assert!(!badges.iter().any(|b| inside(*b, vaults)), "{badges:?}");
}

/// The boxes the frame draws the text `want` in.
fn drawn_boxes(app: &mut Faraday, want: &str) -> Vec<(i32, i32, i32, i32)> {
    let _ = app.frame();
    app.drawn_text_boxes()
        .into_iter()
        .filter(|(t, _)| t == want)
        .map(|(_, b)| b)
        .collect()
}

/// Whether `inner`'s middle is in `outer`.
fn inside(inner: (i32, i32, i32, i32), outer: (u16, u16, u16, u16)) -> bool {
    let (cx, cy) = (inner.0 + inner.2 / 2, inner.1 + inner.3 / 2);
    let (x, y, w, h) = (
        i32::from(outer.0),
        i32::from(outer.1),
        i32::from(outer.2),
        i32::from(outer.3),
    );
    cx >= x && cx < x + w && cy >= y && cy < y + h
}

/// Wallets' badge counts the wallets that need something now and is none
/// while no wallet does.
#[test]
fn the_wallets_badge_counts_the_wallets_that_need_something() {
    let mut idle = with_a_wallet();
    let at = idle.hit_box(Action::Nav(Screen::WalletsStart)).unwrap();
    let badges = drawn_boxes(&mut idle, "1");
    assert!(!badges.iter().any(|b| inside(*b, at)), "{badges:?}");

    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
    let wallet = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "spending-wallet.txt")
        .unwrap();
    let mut app = with_inbox(vec![
        ("spending-wallet.txt", wallet.1),
        ("spend.psbt", psbt),
    ]);
    faraday_core::testkit::load_wallet(&mut app, 0);
    app.press(Action::Nav(Screen::Home));
    let at = app.hit_box(Action::Nav(Screen::WalletsStart)).unwrap();
    let badges = drawn_boxes(&mut app, "1");
    assert!(
        badges.iter().any(|b| inside(*b, at)),
        "no count on Wallets: {badges:?}"
    );
}

/// Home at `width` x `height` with a wallet, a key no wallet uses and, when
/// `psbt`, an unsigned PSBT for the wallet in Files.
fn wallet_and_a_spare_key(width: u16, height: u16, psbt: bool) -> Faraday {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let wallet = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "spending-wallet.txt")
        .unwrap();
    let mut files = vec![("spending-wallet.txt".to_string(), wallet.1)];
    if psbt {
        let bytes = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
        files.push(("spend.psbt".to_string(), bytes));
    }
    let mut app = on_test_net(width, height);
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(files),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let _ = app.frame();
    faraday_core::testkit::load_wallet(&mut app, 0);
    let c = faraday_core::testkit::compute();
    app.session
        .keys
        .add_words(
            &c,
            "legal winner thank year wave sausage worth useful legal winner thank yellow",
            "Spare",
            None,
        )
        .unwrap();
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app
}

fn spare_key_at(app: &Faraday) -> Option<(u16, u16)> {
    (0..app.session.keys.len()).find_map(|k| {
        app.where_offered(key_row(app, key_fp(app, k)))
            .or_else(|| app.where_offered(Action::KeyWallet(key_fp(app, k), 1)))
    })
}

/// On a wide window Home's card holds what needs the person (Backups) and
/// the keys here side by side, both reachable; on a window under 840 dp of
/// card they stand in one list.
#[test]
fn home_card_splits_what_needs_you_from_the_keys_on_a_wide_window_only() {
    let wide = wallet_and_a_spare_key(1920, 1080, false);
    let backups = wide
        .where_offered(Action::Nav(Screen::Backups))
        .expect("Backups row");
    let key = spare_key_at(&wide).expect("key row");
    assert!(key.0 > backups.0, "the keys stand right of what needs you");

    let narrow = wallet_and_a_spare_key(1024, 600, false);
    let backups = narrow
        .where_offered(Action::Nav(Screen::Backups))
        .expect("Backups row");
    let key = spare_key_at(&narrow).expect("key row");
    assert!(
        (key.0 as i32 - backups.0 as i32).abs() < 40,
        "one list: {backups:?} {key:?}"
    );
    assert!(key.1 > backups.1, "the key row is under Backups");
}

/// A PSBT in Files is a Waiting row inside the card: its Sign is drawn once,
/// and the wallet's own row, which has no Sign, stands too.
#[test]
fn home_lead_stands_inside_the_card_once() {
    for (w, h) in [(1920, 1080), (1024, 600)] {
        let app = wallet_and_a_spare_key(w, h, true);
        let signs = app
            .offered_actions()
            .into_iter()
            .filter(|a| matches!(a, Action::StartSpend(_)))
            .count();
        assert_eq!(signs, 1, "{w}x{h}");
        // The phone-size window keeps its box, which has no row for a
        // wallet that needs nothing.
        assert_eq!(
            has_wallet_row(&app, 0),
            w >= 1920,
            "{w}x{h}: the wallet's own row"
        );
        assert!(
            spare_key_at(&app).is_some(),
            "{w}x{h}: the keys are still there"
        );
    }
}

/// Every key and every wallet loaded has a row on the wide Home's card: a
/// key opens itself, a wallet its card, and none is cut or has a Make a
/// wallet button or an All keys row.
#[test]
fn every_key_and_every_wallet_has_a_row_on_home() {
    let mut app = loaded(8, 2, 1366, 1400);
    for k in 0..8 {
        assert!(has_key_row(&app, key_fp(&app, k)), "key {k}");
    }
    for w in 0..2 {
        assert!(has_wallet_row(&app, w), "wallet {w}");
    }
    let texts = app.drawn_texts();
    for gone in ["All keys", "Make a wallet", "All wallets"] {
        assert!(!texts.iter().any(|t| t == gone), "{gone}: {texts:?}");
    }
    for group in ["Wallets", "Keys"] {
        assert!(texts.iter().any(|t| t == group), "{group}: {texts:?}");
    }
}

/// Two PSBTs waiting in Files are two rows, each with its own Sign.
#[test]
fn two_waiting_psbts_have_two_sign_buttons() {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
    let mut app = with_inbox(vec![("a.psbt", psbt.clone()), ("b.psbt", psbt)]);
    assert!(app.offers(Action::StartSpend(0)), "first Sign");
    assert!(app.offers(Action::StartSpend(1)), "second Sign");
    let texts = app.drawn_texts();
    assert_eq!(
        texts.iter().filter(|t| *t == "Sign").count(),
        2,
        "{texts:?}"
    );
}

/// Two locked vault files are two rows, each with its own Unlock.
#[test]
fn two_locked_vault_files_have_two_unlock_buttons() {
    use faraday_core::vault::VaultAction as V;
    let first = faraday_core::testkit::test_vault().unwrap();
    let second = faraday_core::testkit::kdbx_vault("Second", "pw", 2, Vec::new()).unwrap();
    let app = with_inbox(vec![("a.kdbx", first), ("b.kdbx", second)]);
    assert!(app.offers(Action::Vault(V::Open(0))), "first Unlock");
    assert!(app.offers(Action::Vault(V::Open(1))), "second Unlock");
}

/// With nothing loaded and nothing waiting the card holds one line, and has
/// no Make or Restore row.
#[test]
fn the_empty_card_says_nothing_loaded_and_has_no_make_or_restore_row() {
    let mut app = on_test_net(1366, 900);
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Nothing loaded"), "{texts:?}");
    for gone in [
        "Make a new wallet",
        "Restore an existing wallet",
        "Make a wallet",
    ] {
        assert!(!texts.iter().any(|t| t == gone), "{gone}: {texts:?}");
    }
}

/// The boxes of Home's search and its three first actions on the last frame.
fn bar_boxes(app: &Faraday) -> Vec<(u16, u16, u16, u16)> {
    [
        Action::TypeHere(faraday_core::edit::Here::HomeFind),
        Action::CreateWallet,
        Action::RestoreWallet,
        Action::Nav(Screen::SpendBackup),
    ]
    .into_iter()
    .map(|a| {
        app.hit_box(a)
            .unwrap_or_else(|| panic!("{a:?} is not on screen"))
    })
    .collect()
}

/// The three first actions stand in the top bar on Home, with the search.
#[test]
fn home_s_search_and_three_first_actions_stand_in_the_top_bar() {
    let mut app = on_test_net(1366, 768);
    let _ = app.frame();
    for (x, y, w, h) in bar_boxes(&app) {
        assert!(x > 240 && w > 0 && y + h <= 64, "{x},{y} {w}x{h}");
    }
    // Text only: no icon word beside the label.
    let texts = app.drawn_texts();
    for label in ["New wallet", "Restore", "Spend from backup"] {
        assert!(texts.iter().any(|t| t == label), "{label}: {texts:?}");
    }
}

/// At the narrowest and the widest windows with the rail, nothing in the
/// bar overlaps: the search, the three pills and the bar's own actions.
#[test]
fn home_s_top_bar_parts_do_not_overlap_at_any_wide_width() {
    for (w, h) in [(1024, 768), (1280, 800), (1366, 768), (1920, 1080)] {
        let mut app = on_test_net(w, h);
        let _ = app.frame();
        let mut boxes = bar_boxes(&app);
        boxes.sort_by_key(|b| b.0);
        for pair in boxes.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(a.0 + a.2 <= b.0, "{w}x{h}: {a:?} overlaps {b:?}");
        }
        let mid = |b: (u16, u16, u16, u16)| u32::from(b.0) + u32::from(b.2) / 2;
        println!("{w}x{h}: search {}", boxes[0].2);
        let last = boxes[3];
        assert!(mid(last) < u32::from(w), "{w}x{h}: {last:?}");
    }
}

/// At 1024 by 768 (a window with the rail) the search and the three
/// buttons stand in the top bar and the page has no row of its own: the
/// first row of the page starts under the bar. A press on each still opens
/// what it did.
#[test]
fn at_1024_by_768_the_search_and_buttons_stand_in_the_top_bar_and_open_what_they_did() {
    let app = loaded(2, 0, 1024, 768);
    assert_eq!(app.nav(), faraday_core::nav::Nav::Rail);
    let mut boxes = bar_boxes(&app);
    for (x, y, w, h) in &boxes {
        assert!(*w > 0 && y + h <= 64, "{x},{y} {w}x{h}");
    }
    boxes.sort_by_key(|b| b.0);
    for pair in boxes.windows(2) {
        assert!(pair[0].0 + pair[0].2 <= pair[1].0, "{pair:?} overlap");
    }
    for (action, screen) in [
        (FIRST[0], Screen::Create),
        (FIRST[1], Screen::Restore),
        (FIRST[2], Screen::SpendBackup),
    ] {
        let mut app = loaded(2, 0, 1024, 768);
        tap(&mut app, action);
        assert_eq!(app.screen, screen, "{action:?}");
    }
    let mut app = loaded(2, 0, 1024, 768);
    type_on_home(&mut app, "send");
    let (_, y) = app
        .where_offered(Action::Catalog(sign_tile()))
        .expect("a result of the page");
    assert!(y >= 64, "{y}: the page starts under the bar");
    tap(&mut app, Action::Catalog(sign_tile()));
    assert_eq!(app.sheet, Some(faraday_core::Sheet::Scan));
}

// ----- Add randomness's camera counts as the camera on, as the scan's -----

/// Add randomness open from Settings with its camera taking pictures, in a
/// clean process.
fn adding_randomness_with_the_camera() -> Faraday {
    use faraday_core::fresh::OwnSource;
    use faraday_core::fresh::page::Act;
    let mut app = shown();
    app.storage(StorageEvent::Cameras(vec![(
        faraday_text::Text::of("cam0"),
        faraday_text::Text::of("Camera"),
    )]));
    app.press(Action::Nav(Screen::Settings));
    crate::common::settings_section(&mut app, "Randomness");
    app.press(Action::Own(Act::Open));
    app.press(Action::Own(Act::Source(OwnSource::Camera)));
    assert_eq!(app.screen, Screen::Randomness);
    while app.poll_command().is_some() {}
    app
}

/// Camera off by the page's own way: leaving it.
fn leave_the_page(app: &mut Faraday) {
    app.press(Action::Back);
    let _ = app.frame();
    let cmds: Vec<_> = std::iter::from_fn(|| app.poll_command()).collect();
    assert!(cmds.contains(&Command::CameraOff));
}

#[test]
fn a_stick_shown_while_adding_randomness_with_the_camera_is_held_until_it_is_off() {
    let mut app = adding_randomness_with_the_camera();
    assert!(!app.clean(), "a stick may be handed out with the camera on");
    app.storage(StorageEvent::Attached(true));
    stick(&mut app, true);
    assert_ne!(app.sheet, Some(Sheet::Lock));
    assert_eq!(app.screen, Screen::Randomness, "the page stays");
    assert!(!app.clean());
    leave_the_page(&mut app);
    assert!(app.clean());
    assert_eq!(
        app.screen,
        Screen::Files,
        "the stick arrives once it is off"
    );
}

#[test]
fn a_stick_listed_while_adding_randomness_with_the_camera_is_held_until_it_is_off() {
    let mut app = adding_randomness_with_the_camera();
    stick(&mut app, true);
    assert_eq!(app.screen, Screen::Randomness, "the page stays");
    assert_ne!(app.sheet, Some(Sheet::Lock));
    leave_the_page(&mut app);
    assert_eq!(app.screen, Screen::Files);
}

#[test]
fn a_stick_in_before_the_camera_of_add_randomness_comes_on_is_not_handed_out() {
    let mut app = shown();
    stick(&mut app, true);
    assert!(app.clean());
    let mut app = {
        use faraday_core::fresh::OwnSource;
        use faraday_core::fresh::page::Act;
        app.storage(StorageEvent::Cameras(vec![(
            faraday_text::Text::of("cam0"),
            faraday_text::Text::of("Camera"),
        )]));
        app.press(Action::Nav(Screen::Settings));
        crate::common::settings_section(&mut app, "Randomness");
        app.press(Action::Own(Act::Open));
        app.press(Action::Own(Act::Source(OwnSource::Camera)));
        app
    };
    assert!(!app.clean(), "the camera is on, no stick is handed out");
    leave_the_page(&mut app);
    assert!(app.clean());
}

#[test]
fn storage_pulled_while_adding_randomness_with_the_camera_offers_nothing_later() {
    let mut app = adding_randomness_with_the_camera();
    app.storage(StorageEvent::Attached(true));
    app.storage(StorageEvent::Attached(false));
    leave_the_page(&mut app);
    assert!(app.clean());
    assert_ne!(app.sheet, Some(Sheet::Lock));
    assert_ne!(app.screen, Screen::Files);
}

#[test]
fn a_stick_under_the_idle_warning_over_add_randomness_with_the_camera_is_still_held() {
    const MIN: u64 = 60_000;
    use faraday_core::fresh::OwnSource;
    use faraday_core::fresh::page::Act;
    let mut app = shown();
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("spending-wallet.txt"),
            kit.descriptor.into_bytes(),
        )]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    faraday_core::testkit::load_wallet(&mut app, 0);
    app.storage(StorageEvent::Cameras(vec![(
        faraday_text::Text::of("cam0"),
        faraday_text::Text::of("Camera"),
    )]));
    app.press(Action::Nav(Screen::Settings));
    crate::common::settings_section(&mut app, "Randomness");
    app.press(Action::Own(Act::Open));
    app.press(Action::Own(Act::Source(OwnSource::Camera)));
    app.event(Event::Tick { now_ms: 0 });
    app.event(Event::Tick { now_ms: 26 * MIN });
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
    assert!(!app.clean(), "the camera is on under the warning");
    let before = app.screen;
    app.storage(StorageEvent::Attached(true));
    stick(&mut app, true);
    assert_eq!(app.screen, before, "the stick was not visited");
    assert_eq!(app.sheet, Some(Sheet::IdleWarn));
}
