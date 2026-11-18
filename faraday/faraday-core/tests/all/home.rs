//! Home: Scan floats at the foot; Add a key is reached from Wallets, not
//! from Home, and opens New key on its Start card. Keys load only with no stick attached: Add a key pressed
//! with one in asks for it to be pulled, and opens once it is. A camera
//! frame may picture a seed, so Scan waits for the stick too, and a
//! stick plugged in while the camera is on is held until it is off.
//! Home's Scan takes a seed to Add a key, and anything else where it
//! went before.

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

/// Scan floats on the top page of Check, Wallets, Files, Vaults and Tools
/// as it does on Home, on a wide window and a phone-size one, and starts
/// the camera; Back returns to the page it was pressed on.
#[test]
fn scan_floats_on_the_five_destinations() {
    for (w, h) in [(1280, 800), (400, 800)] {
        for screen in FIVE {
            let mut app = sized(w, h, 160);
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
fn scan_is_not_on_settings_or_inside_a_flow() {
    for (w, h) in [(1280, 800), (400, 800)] {
        let mut app = sized(w, h, 160);
        app.press(Action::Nav(Screen::Settings));
        let _ = app.frame();
        assert!(!floats_scan(&app, w, h), "Settings at {w}x{h}");
        let mut app = sized(w, h, 160);
        app.press(Action::Nav(Screen::WalletsStart));
        tap(&mut app, Action::KeyGen(None));
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

/// On Files, with a long list scrolled under it, and on Wallets' start
/// page, Scan at 1024x600 is as large as on Home and no control scrolled
/// under it takes a share of its target.
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
    tap(&mut app, Action::KeyGen(None));
    assert_eq!(app.sheet, Some(Sheet::Pull));
    assert_eq!(app.screen, Screen::WalletsStart);
    stick(&mut app, false);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::KeyGen);
}

#[test]
fn cancel_leaves_wallets_and_a_later_pull_opens_nothing() {
    let mut app = home_with_stick();
    app.press(Action::Nav(Screen::WalletsStart));
    tap(&mut app, Action::KeyGen(None));
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
    tap(&mut app, Action::KeyGen(None));
    assert_eq!(app.screen, Screen::KeyGen);
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
    use faraday_core::keygen::Start;
    let mut from_add = shown();
    from_add.press(Action::Nav(Screen::WalletsStart));
    tap(&mut from_add, Action::KeyGen(None));
    // The card is opening: until it has, its lower rows are still under
    // the next card's header, which a press there reaches. Let it finish
    // before pressing a row on it.
    for _ in 0..4 {
        let now = from_add.now_ms() + 300;
        from_add.event(Event::Tick { now_ms: now });
        let _ = from_add.frame();
    }
    tap(&mut from_add, Action::KStart(Start::Scan.index()));
    assert_eq!(from_add.sheet, Some(Sheet::Scan), "Scan a SeedQR moves on");
    read(&mut from_add, SEEDQR.as_bytes());
    assert_eq!(
        from_add.keygen().map(|k| k.open),
        Some(Some(faraday_core::keygen::kstep::KEY))
    );
    tap(&mut from_add, Action::KLock);
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

/// Home's two tiles for making and restoring a wallet
/// read "Make a new wallet" and "Restore an existing wallet", and so
/// does the empty Wallets tab.
#[test]
fn home_and_empty_wallets_name_the_two_flows() {
    let mut app = shown();
    let joined = app.drawn_texts().join(" ");
    assert!(joined.contains("Make a new wallet"), "{joined}");
    assert!(joined.contains("Restore an existing wallet"), "{joined}");
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

/// Stick visit joins Home's tiles while a stick is attached and nothing
/// is held.
#[test]
fn home_offers_files_on_the_stick_with_one_attached() {
    // The boot stick at its first look leaves Home on screen: it waits.
    let mut app = shown();
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of("a"),
        label: faraday_text::Text::of("FARADAY"),
        boot: true,
        files: Vec::new(),
        vaults: Vec::new(),
    }]));
    assert_eq!(app.screen, Screen::Home);
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Files on Boot stick"),
        "a stick waiting: the lead reads Files on the stick: {texts:?}"
    );
    tap(&mut app, Action::Nav(Screen::Files));
    assert_eq!(app.screen, Screen::Files);
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

/// The Pi's Home names the wallet that is loaded, in the status strip
/// under its name.
#[test]
fn the_pi_s_home_names_a_loaded_wallet() {
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
    let name = app.session.wallets[0].name.to_string();
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

/// A key no wallet uses leads with Make a wallet from it, and Start's
/// tiles are gone.
#[test]
fn a_key_with_no_wallet_leads_with_make_a_wallet() {
    let mut app = shown();
    tap(&mut app, Action::Scan);
    read(&mut app, SEEDQR.as_bytes());
    without_passphrase(&mut app);
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::Nav(Screen::Home));
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "No wallet uses it"), "{texts:?}");
    assert!(app.offers(Action::KeyWallet(fp, 1)));
    assert!(!app.offers(Action::RestoreWallet), "Start is gone");
    assert!(!app.offers(Action::CreateWallet), "Make is not the lead");
}

/// A loaded wallet shows its card with Sign, which opens what the wallet
/// card's first button opens, and Start's tiles are gone.
#[test]
fn a_loaded_wallet_shows_sign_and_start_is_gone() {
    use faraday_core::spend_backup::SpendBackupAction as F;
    let mut app = with_a_wallet();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Sign"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "Receive"), "{texts:?}");
    assert!(!texts.iter().any(|t| t == "Start"), "{texts:?}");
    assert!(!app.offers(Action::RestoreWallet));
    assert!(!app.offers(Action::CreateWallet));
    assert!(app.offers(Action::OpenWallet(0)));
    tap(&mut app, Action::SpendBackup(F::SpendFrom(0)));
    assert_eq!(app.screen, Screen::SpendBackup);
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

#[test]
fn theme_typed_on_home_shows_the_theme_setting() {
    for mut app in [shown(), window(400, 800, 160)] {
        type_on_home(&mut app, "theme");
        let button = (0..64u8)
            .map(Action::SettingAt)
            .find(|&a| app.hit_box(a).is_some())
            .expect("a setting button for theme");
        tap(&mut app, button);
        assert_eq!(app.screen, Screen::Settings);
        assert!(faraday_core::testkit::home_find(&app).is_empty());
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

/// The hit boxes of the three first actions and of `more`.
fn boxes(app: &Faraday, more: Action) -> Vec<Option<(u16, u16, u16, u16)>> {
    FIRST
        .iter()
        .chain([&more])
        .map(|a| app.hit_box(*a))
        .collect()
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

/// On the Pi's menu the top pair is Wallets then Check on one row; a
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
    assert_eq!(w.1, c.1, "one row");
    assert!(w.0 < c.0, "Wallets is left of Check");
    let (w, c) = boxes(&window(400, 800, 160));
    assert!(c.1 < w.1, "Check is in the page, Wallets on the bar below");
}

#[test]
fn the_wide_homes_first_actions_and_bar_stay_where_they_were_while_a_search_is_typed() {
    let bar = Action::TypeHere(faraday_core::edit::Here::HomeFind);
    {
        let make = wide;
        let mut app = make();
        let before = boxes(&app, bar);
        assert!(before.iter().all(Option::is_some));
        type_on_home(&mut app, "send");
        assert_eq!(before, boxes(&app, bar));
        // Scrolling the results moves none of them either.
        app.event(Event::Scroll {
            x: 300,
            y: 700,
            dy: 3000,
        });
        let _ = app.frame();
        assert_eq!(before, boxes(&app, bar));
        tap(&mut app, Action::CreateWallet);
        assert_eq!(app.screen, Screen::Create);
    }
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
fn the_import_lead_is_a_card_with_no_pill_and_typing_a_search_leaves_it_and_the_row() {
    let open = faraday_core::boot_import::OPEN;
    {
        let make = wide;
        let mut app = with_an_import_waiting(make);
        let texts = app.drawn_texts();
        assert!(!texts.iter().any(|t| t == "Import"), "no pill: {texts:?}");
        let before = boxes(&app, open);
        assert!(before.iter().all(Option::is_some), "{before:?}");
        type_on_home(&mut app, "send");
        assert_eq!(before, boxes(&app, open));
        tap(&mut app, open);
        assert!(app.sheet.is_some());
    }
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
fn short_with_a_wallet(width: u16, height: u16) -> Faraday {
    let wallet = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "spending-wallet.txt")
        .unwrap();
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

/// A Home taller than its window scrolls as one page, the bar and what is
/// under it reachable, with a search typed or not.
#[test]
fn a_home_taller_than_its_window_scrolls_to_the_search_bar_and_its_results() {
    let bar = Action::TypeHere(faraday_core::edit::Here::HomeFind);
    for (width, height) in [(1366, 140), (400, 500)] {
        let mut app = short_with_a_wallet(width, height);
        // On the short wide window the bar starts below it; on the phone
        // it starts in view and the results are what is out of it.
        if width > 1000 {
            assert!(
                !on_screen(&app, bar, height),
                "{width}x{height}: the bar is below the window: {:?}",
                app.where_offered(bar)
            );
        }
        assert!(
            scrolled_to(&mut app, width, height, bar),
            "{width}x{height}: the bar is out of reach"
        );
        // Typed on a keyboard of the person's own: the on-screen one,
        // taller than a window this short, would cover all of it.
        type_on_home(&mut app, "send");
        assert_eq!(faraday_core::testkit::home_find(&app), "send");
        assert!(
            scrolled_to(&mut app, width, height, Action::Catalog(sign_tile())),
            "{width}x{height}: the result is out of reach"
        );
    }
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
    // Equal columns, to a pixel of rounding.
    assert!(boxes.iter().all(|b| b.2.abs_diff(boxes[0].2) <= 1));
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t.ends_with('…')), "{texts:?}");
}
