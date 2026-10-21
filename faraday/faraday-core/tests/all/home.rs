//! Home: Scan floats at the foot; Add a key is reached from Wallets, not
//! from Home, and opens New key on its Start card. Keys load only with no stick attached: Add a key pressed
//! with one in asks for it to be pulled, and opens once it is. A camera
//! frame may picture a seed, so Scan waits for the stick too, and a
//! stick plugged in while the camera is on is held until it is off.
//! Home's Scan takes a seed to Add a key, and anything else where it
//! went before.

use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageEvent};
use faraday_shell_api::{App, Command, DisplayInfo, Event, TouchPhase};

fn shown() -> Faraday {
    let mut app = Faraday::new();
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
            serial: None,
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

/// Add a key is reached from Wallets' empty state and from Tools now,
/// not from Home (`docs/history/SIMPLIFY.md` §1.2): Home's lead, with nothing
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
    app.press(Action::Nav(Screen::Start));
    tap(&mut app, Action::KeyGen(None));
    assert_eq!(app.screen, Screen::KeyGen);
    assert_eq!(app.sheet, None);
    assert_eq!(
        app.keygen.as_ref().unwrap().open,
        Some(faraday_core::keygen::kstep::START)
    );
}

#[test]
fn add_a_key_from_wallets_with_a_stick_in_asks_for_it_and_opens_once_it_is_pulled() {
    let mut app = home_with_stick();
    app.press(Action::Nav(Screen::Start));
    tap(&mut app, Action::KeyGen(None));
    assert_eq!(app.sheet, Some(Sheet::Pull));
    assert_eq!(app.screen, Screen::Start);
    stick(&mut app, false);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::KeyGen);
}

#[test]
fn cancel_leaves_wallets_and_a_later_pull_opens_nothing() {
    let mut app = home_with_stick();
    app.press(Action::Nav(Screen::Start));
    tap(&mut app, Action::KeyGen(None));
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
fn a_seedqr_scanned_from_home_loads_the_key_add_a_keys_scan_loads() {
    use faraday_core::keygen::Start;
    let mut from_add = shown();
    from_add.press(Action::Nav(Screen::Start));
    tap(&mut from_add, Action::KeyGen(None));
    tap(&mut from_add, Action::KStart(Start::Scan.index()));
    tap(&mut from_add, Action::KNext);
    read(&mut from_add, SEEDQR.as_bytes());
    tap(&mut from_add, Action::KLock);
    tap(&mut from_add, Action::KNext);

    let mut app = shown();
    tap(&mut app, Action::Scan);
    read(&mut app, SEEDQR.as_bytes());
    without_passphrase(&mut app);
    assert_eq!(app.sheet, None);
    assert!(app.inbox.is_empty());
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(
        app.session.keys[0].fingerprint(),
        from_add.session.keys[0].fingerprint()
    );
    assert_eq!(app.screen, Screen::Wallets);
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
    without_passphrase(&mut app);
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
// §1.1 and §1.2 (`docs/history/SIMPLIFY.md`): Home is the Start section, a
// lead tile when something is under way, then the four ways in.
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
    use faraday_core::vault::VaultAction as V;
    let vault = faraday_core::testkit::test_vault().unwrap();
    let app = with_inbox(vec![("vault.ofv", vault)]);
    assert!(
        app.offers(Action::Vault(V::Open(0))),
        "Home leads with Unlock {{name}}"
    );
}

#[test]
fn home_lead_rule_5_with_a_stick_in_asks_to_pull_it_first() {
    use faraday_core::vault::VaultAction as V;
    let vault = faraday_core::testkit::test_vault().unwrap();
    let mut app = with_inbox(vec![("vault.ofv", vault)]);
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
        serial: None,
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
    let files = vec![
        ("notes.txt".to_string(), b"a note".to_vec()),
        (
            "vault.ofv".to_string(),
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
    assert!(app.import.is_some());
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
    app.press(faraday_core::boot_import::LATER);
    app.storage(StorageEvent::Sticks(Vec::new()));
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Unlock vault.ofv"), "{texts:?}");
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

/// SCAN-TO-SIGN §1: Home's two tiles for making and restoring a wallet
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
    tap(&mut app, Action::Nav(Screen::Start));
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
/// existing wallet, Spend from a backup, and Vaults, which with no vault
/// at all opens Create a vault.
#[test]
fn homes_four_ways_in_open_their_flows_and_lead_back_home() {
    use faraday_core::vault::VaultAction as V;
    for (action, screen) in [
        (Action::CreateWallet, Screen::Create),
        (Action::RestoreWallet, Screen::Restore),
        (Action::Nav(Screen::Family), Screen::Family),
        (Action::Vault(V::Create), Screen::CreateVault),
    ] {
        let mut app = shown();
        assert!(app.offers(action), "{action:?} is on Home");
        tap(&mut app, action);
        assert_eq!(app.screen, screen);
        app.event(Event::Back);
        let _ = app.frame();
        assert_eq!(app.screen, Screen::Home, "back from {screen:?}");
    }
}

/// Opened from Wallets, the same flows lead back to Wallets.
#[test]
fn the_flows_opened_from_wallets_lead_back_to_wallets() {
    for (action, screen) in [
        (Action::CreateWallet, Screen::Create),
        (Action::RestoreWallet, Screen::Restore),
        (Action::Nav(Screen::Family), Screen::Family),
    ] {
        let mut app = shown();
        app.press(Action::Nav(Screen::Start));
        tap(&mut app, action);
        assert_eq!(app.screen, screen);
        app.event(Event::Back);
        let _ = app.frame();
        assert_eq!(app.screen, Screen::Start, "back from {screen:?}");
    }
}

/// With a vault in Files, Home's Vaults opens the list, not Create a
/// vault.
#[test]
fn homes_vaults_opens_the_list_once_there_is_a_vault() {
    use faraday_core::vault::VaultAction as V;
    let vault = faraday_core::testkit::test_vault().unwrap();
    let app = with_inbox(vec![("vault.ofv", vault)]);
    assert!(!app.offers(Action::Vault(V::Create)));
}

/// Stick visit joins Home's tiles while a stick is attached and nothing
/// is held.
#[test]
fn home_offers_stick_visit_with_one_attached() {
    let mut app = home_with_stick();
    assert!(
        app.offers(Action::Nav(Screen::Visit)),
        "a stick in and nothing held: Stick visit"
    );
    tap(&mut app, Action::Nav(Screen::Visit));
    assert_eq!(app.screen, Screen::Visit);
}

/// Rule 5 with what the vault held when last seen open (§3.4): after a
/// lock, the Unlock lead's line names it.
#[test]
fn home_lead_rule_5_reads_what_the_vault_held_when_last_seen_open() {
    use faraday_core::StorageCommand;
    use faraday_core::vault::VaultAction as V;
    let vault = faraday_core::testkit::test_vault().unwrap();
    let mut app = with_inbox(vec![("vault.ofv", vault)]);
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.press(Action::Vault(V::Open(0)));
    for c in faraday_core::testkit::VAULT_PASSPHRASES[0].chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults.open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "the test vault did not unlock");
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
        inbox: boxes.0,
        outbox: boxes.1,
        kept: boxes.2,
    });
    let _ = app.frame();
    assert!(
        app.offers(Action::Vault(V::Open(0))),
        "Home leads with Unlock {{name}}"
    );
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Unlock Test vault"),
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
        outbox: vec![wallet],
        kept: Vec::new(),
    });
    stick(&mut app, true);
    app.press(Action::Nav(Screen::Visit));
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
    let mut app = with_inbox(vec![("vault.ofv", vault)]);
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    // A key the test vault does not hold, to save into it.
    app.session
        .add_words(
            &c,
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon \
             abandon about",
            "k",
            None,
        )
        .unwrap();
    app.press(Action::Vault(V::Open(0)));
    for c in faraday_core::testkit::VAULT_PASSPHRASES[0].chars() {
        app.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults.open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "the test vault did not unlock");
    app.press(Action::Vault(V::LoadTicked));
    app.press(Action::Vault(V::AddKind(0)));
    app.press(Action::Vault(V::SaveKey(0)));
    app.press(Action::Nav(Screen::Home));
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("Write ") && t.ends_with(" to a stick")),
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
    app.press(Action::LoadWallet(0));
    assert_eq!(app.session.wallets.len(), 1, "the wallet did not load");
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app
}

/// Backups joins Home's tiles once a wallet is known, and opens the
/// Backups screen; a stick attached with nothing held adds Stick visit
/// beside it (`docs/history/SIMPLIFY.md` §1.2, §5).
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
    assert!(
        app.offers(Action::Nav(Screen::Visit)),
        "a stick in: Stick visit"
    );
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
