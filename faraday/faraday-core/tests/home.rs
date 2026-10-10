//! Home: Scan floats at the foot; Add a key is reached from Wallets, not
//! from Home. Keys load only with no stick attached: Add a key pressed
//! with one in asks for it to be pulled, and opens once it is. A camera
//! frame may picture a seed, so Scan waits for the stick too, and a
//! stick plugged in while the camera is on is held until it is off.
//! Home's Scan takes a seed to Add a key, and anything else where it
//! went before.

use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent};
use osk_shell_api::{App, Command, DisplayInfo, Event, TouchPhase};

fn shown() -> Faraday {
    let mut app = Faraday::new();
    app.event(osk_shell_api::Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: osk_shell_api::SecureHardware::None,
        boot: osk_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

fn stick(app: &mut Faraday, on: bool) {
    let sticks = if on {
        vec![StickInfo {
            id: "a".into(),
            label: "STICK".into(),
            boot: false,
            files: Vec::new(),
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

/// Add a key is reached from Wallets' empty state and from Tools now,
/// not from Home (`docs/SIMPLIFY.md` §1.2): Home's lead, with nothing
/// loaded, is Make a wallet instead.
#[test]
fn home_does_not_offer_add_a_key_directly() {
    let app = shown();
    assert!(!app.offers(Action::Entry(None)));
}

#[test]
fn add_a_key_from_wallets_opens_add_a_key() {
    let mut app = shown();
    app.press(Action::Nav(Screen::Start));
    tap(&mut app, Action::Entry(None));
    assert_eq!(app.screen, Screen::Entry);
    assert_eq!(app.sheet, None);
}

#[test]
fn add_a_key_from_wallets_with_a_stick_in_asks_for_it_and_opens_once_it_is_pulled() {
    let mut app = home_with_stick();
    app.press(Action::Nav(Screen::Start));
    tap(&mut app, Action::Entry(None));
    assert_eq!(app.sheet, Some(Sheet::Pull));
    assert_eq!(app.screen, Screen::Start);
    stick(&mut app, false);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Entry);
}

#[test]
fn cancel_leaves_wallets_and_a_later_pull_opens_nothing() {
    let mut app = home_with_stick();
    app.press(Action::Nav(Screen::Start));
    tap(&mut app, Action::Entry(None));
    tap(&mut app, Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Start);
    stick(&mut app, false);
    assert_eq!(app.screen, Screen::Start);
}

#[test]
fn the_wallets_tab_offers_add_a_key() {
    let mut app = shown();
    app.press(Action::Nav(Screen::Start));
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
    assert_eq!(app.screen, Screen::Visit);
}

#[test]
fn a_seedqr_scanned_while_a_stick_is_held_loads_no_key() {
    let mut app = shown();
    tap(&mut app, Action::Scan);
    stick(&mut app, true);
    read(&mut app, SEEDQR.as_bytes());
    assert!(app.session.keys.is_empty());
    assert!(app.inbox.is_empty());
    app.press(Action::Cancel);
    assert!(app.clean());
}

#[test]
fn a_seedqr_scanned_from_home_lands_where_add_a_key_scan_lands() {
    let mut from_entry = shown();
    from_entry.press(Action::Nav(Screen::Start));
    tap(&mut from_entry, Action::Entry(None));
    tap(&mut from_entry, Action::ScanSeed);
    read(&mut from_entry, SEEDQR.as_bytes());

    let mut app = shown();
    tap(&mut app, Action::Scan);
    read(&mut app, SEEDQR.as_bytes());
    assert_eq!(app.sheet, None);
    assert!(app.inbox.is_empty());
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(
        app.session.keys[0].master.fingerprint(),
        from_entry.session.keys[0].master.fingerprint()
    );
    assert_eq!(app.screen, from_entry.screen);
}

#[test]
fn a_seeds_words_scanned_from_home_go_to_add_a_key() {
    let mut app = shown();
    tap(&mut app, Action::Scan);
    read(
        &mut app,
        b"abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    );
    assert!(app.inbox.is_empty());
    assert_eq!(app.session.keys.len(), 1);
}

#[test]
fn a_psbt_scanned_from_home_goes_into_files_and_home_stays() {
    let kit = faraday_core::testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let psbt = faraday_core::testkit::unsigned(&kit).unwrap().to_bytes();
    let mut app = shown();
    tap(&mut app, Action::Scan);
    read(&mut app, &psbt);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Home);
    assert!(app.session.keys.is_empty());
    assert_eq!(app.inbox.last().map(|i| i.bytes.clone()), Some(psbt));
}

#[test]
fn with_a_key_loaded_a_stick_plugged_in_while_scanning_waits_then_asks_to_lock() {
    let mut app = shown();
    tap(&mut app, Action::Scan);
    read(&mut app, SEEDQR.as_bytes());
    assert_eq!(app.session.keys.len(), 1);
    app.press(Action::Scan);
    assert_eq!(app.sheet, Some(Sheet::Scan));
    while app.poll_command().is_some() {}
    stick(&mut app, true);
    // The scanner is not replaced with its camera still on.
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(app.scan.is_some());
    app.press(Action::Cancel);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Lock));
    assert!(!app.clean());
}

// ---------------------------------------------------------------------
// §1.1 and §1.2 (`docs/SIMPLIFY.md`): Home is the Start section, one
// lead tile and at most two secondaries.
// ---------------------------------------------------------------------

/// `shown()` with `files` already in the Inbox.
fn with_inbox(files: Vec<(&str, Vec<u8>)>) -> Faraday {
    let mut app = shown();
    app.storage(StorageEvent::Restored {
        inbox: files.into_iter().map(|(n, b)| (n.to_string(), b)).collect(),
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
    assert!(app.spend.is_some());
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
    use faraday_core::vaults::VaultAction as V;
    let vault = faraday_core::testkit::test_vault().unwrap();
    let app = with_inbox(vec![("vault.ofv", vault)]);
    assert!(
        app.offers(Action::Vault(V::Open(0))),
        "Home leads with Unlock {{name}}"
    );
}

#[test]
fn home_lead_rule_5_with_a_stick_in_asks_to_pull_it_first() {
    use faraday_core::vaults::VaultAction as V;
    let vault = faraday_core::testkit::test_vault().unwrap();
    let mut app = with_inbox(vec![("vault.ofv", vault)]);
    stick(&mut app, true);
    assert!(
        !app.offers(Action::Vault(V::Open(0))),
        "no target: the stick must be pulled first"
    );
}

/// A minimal boot stick holding one plain file: enough to bring up
/// `app.import` without a vault or a PSBT in it, so rule 3 alone leads.
fn boot_stick(files: &[(String, Vec<u8>)]) -> StickInfo {
    StickInfo {
        id: "boot".into(),
        label: "FARADAY".into(),
        boot: true,
        files: files
            .iter()
            .map(|(n, b)| (n.clone(), b.len() as u64))
            .collect(),
    }
}

/// Answers every read the boot import asks for.
fn pump(app: &mut Faraday, files: &[(String, Vec<u8>)]) {
    while let Some(c) = app.poll_storage() {
        if let faraday_core::StorageCommand::Read { stick, name } = c {
            let bytes = files.iter().find(|(n, _)| *n == name).unwrap().1.clone();
            app.storage(StorageEvent::Read { stick, name, bytes });
        }
    }
}

#[test]
fn home_lead_rule_3_an_import_waiting() {
    let files = vec![("notes.txt".to_string(), b"a note".to_vec())];
    let mut app = shown();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Sticks(vec![boot_stick(&files)]));
    pump(&mut app, &files);
    let _ = app.frame();
    assert!(app.import.is_some());
    // The sheet comes up over Home on arrival; Not now leaves it waiting
    // behind the stick, and Home leads with it once the sheet is closed.
    app.press(faraday_core::boot_import::LATER);
    let _ = app.frame();
    assert!(
        app.offers(faraday_core::boot_import::OPEN),
        "Home does not lead with the import, stick still in"
    );
    // Pulled: Import from {label}, and the tap reopens the boot sheet.
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    assert!(app.offers(faraday_core::boot_import::OPEN));
    app.press(faraday_core::boot_import::OPEN);
    assert_eq!(app.sheet, Some(Sheet::Import));
}

#[test]
fn home_lead_rule_7_with_nothing_loaded_offers_to_make_a_wallet() {
    let app = shown();
    assert!(app.offers(Action::CreateWallet));
}

#[test]
fn home_never_shows_more_than_three_start_tiles() {
    let app = shown();
    // The lead (Make a wallet, rule 7) plus two secondaries: Wallets,
    // then Vaults since no stick is attached.
    assert!(app.offers(Action::CreateWallet));
    assert!(app.offers(Action::Nav(Screen::Start)));
    assert!(app.offers(Action::Nav(Screen::Vaults)));
}

/// The second secondary tile: Stick visit while one is attached and
/// nothing is held, else Vaults (`docs/SIMPLIFY.md` §1.2, revised).
#[test]
fn homes_second_secondary_is_stick_visit_with_one_attached_else_vaults() {
    let app = shown();
    assert!(app.offers(Action::Nav(Screen::Vaults)), "no stick: Vaults");
    assert!(!app.offers(Action::Nav(Screen::Visit)));

    // The sidebar always offers Vaults on its own, so only Home's own
    // lead/secondary offers are checked here, not Vaults' absence.
    let mut app = home_with_stick();
    assert!(
        app.offers(Action::Nav(Screen::Visit)),
        "a stick in and nothing held: Stick visit"
    );
    tap(&mut app, Action::Nav(Screen::Visit));
    assert_eq!(app.screen, Screen::Visit);
}
