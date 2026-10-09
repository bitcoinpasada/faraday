//! Any loaded wallet can be backed up again, with every public file
//! Create offers. A wallet loaded from a descriptor file, with no seed
//! here, opens its backup on the presets with no question about seeds,
//! and a plan that sends it to every software as files puts each public
//! file in the Outbox, the BSMS record and Bitcoin Core's import among
//! them. A restored one-key wallet saves itself into a vault from its
//! checklist, unlocking it on the way. With two wallets loaded, Back up from Tools
//! backs up the one picked on Wallets, and its chip switches to the
//! other. The key rows list the wallet's keys whose seeds are here.

use faraday_core::backup::Tone;
use faraday_core::catalog::{Go, TILES};
use faraday_core::seeds::SeedsAction as S;
use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Code, Faraday, Screen, bstep, plan, qrow, qstep};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

fn kit_file(name: &str) -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no {name} in the kit"))
}

fn device(inbox: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(faraday_core::StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(faraday_core::StorageEvent::Memory {
        available_mib: 15_000,
    });
    let _ = app.frame();
    app
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Loads the wallet in Inbox file `name`; returns its index.
fn load(app: &mut Faraday, name: &str) -> usize {
    let i = app
        .inbox
        .iter()
        .position(|it| it.name == name)
        .unwrap_or_else(|| panic!("{name} is not in Files"));
    let before = app.session.wallets.len();
    app.press(Action::LoadWallet(i));
    assert_eq!(app.session.wallets.len(), before + 1, "{name} did not load");
    before
}

/// Test key n typed into Add a key.
fn add_key(app: &mut Faraday, n: usize) -> osk_bip::keys::Fingerprint {
    app.press(Action::Entry(None));
    type_text(app, &testkit::test_words(testkit::TEST_SEEDS[n].0));
    app.press(Action::EntryAdd);
    app.session
        .keys
        .last()
        .expect("the key was not added")
        .master
        .fingerprint()
}

fn in_outbox(app: &Faraday, name: &str) -> bool {
    app.outbox.iter().any(|i| i.name == name)
}

/// Presses `action`, which the screen on show must offer somewhere down
/// its column.
fn press_offered(app: &mut Faraday, action: Action) {
    for _ in 0..20 {
        app.settle();
        let _ = app.frame();
        if app.offers(action) {
            app.press(action);
            return;
        }
        app.event(Event::Scroll {
            x: 400,
            y: 384,
            dy: 300,
        });
    }
    panic!("{action:?} is not offered");
}

/// The plan with the description to every software, as text and QR
/// pictures, then the checklist made from it.
fn every_software(app: &mut Faraday) {
    press_offered(app, Action::BPreset(2));
    for row in 0..5u8 {
        if row != plan::software::NOT_SURE as u8 {
            app.press(Action::BAnswer(qrow::SOFTWARE, row));
        }
    }
    app.press(Action::BAnswer(qrow::FORM, plan::form::TEXT as u8));
    app.press(Action::BChecklist);
}

#[test]
fn a_watch_only_wallet_plans_no_seeds_and_puts_each_public_file_in_the_outbox() {
    let mut app = device(vec![kit_file("savings-wallet.txt")]);
    let w = load(&mut app, "savings-wallet.txt");
    assert!(app.session.keys.is_empty());
    app.press(Action::Backup(w));
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.backup.as_ref().unwrap().q, Some(qstep::PRESET));
    assert!(!app.backup_questions().contains(&qstep::SEEDS));
    every_software(&mut app);
    let items = app.backup_items();
    assert!(!items.contains(&bstep::BLANK) && !items.contains(&bstep::COPY));
    for n in &items {
        let it = bstep::item(*n).unwrap();
        assert!(!app.backup_item_done(it), "{it:?} is done already");
    }
    press_offered(&mut app, Action::BStep(bstep::PUBLIC));
    let want = testkit::public_files("Savings", &testkit::savings()).unwrap();
    for (what, name) in [
        (1, "savings-descriptor.txt"),
        (5, "savings-wallet.json"),
        (2, "savings-multisig-config.txt"),
        (6, "savings-bsms.txt"),
        (7, "savings-bitcoin-core.json"),
        (8, "savings-descriptor.png"),
    ] {
        press_offered(&mut app, Action::PublicOut(w, what));
        assert!(in_outbox(&app, name), "{name} is not in the Outbox");
        if !name.ends_with(".pdf") {
            let made = &app.outbox.iter().find(|i| i.name == name).unwrap().bytes;
            let kit = &want.iter().find(|(n, _)| n == name).unwrap().1;
            assert_eq!(made, kit, "{name}");
        }
    }
    for code in [Code::MultisigConfig(w), Code::Bsms(w)] {
        press_offered(&mut app, Action::CodePng(code));
    }
    assert!(in_outbox(&app, "savings-multisig-config.png"));
    assert!(in_outbox(&app, "savings-bsms.png"));
    // No seed here: no key of it to give the cosigners.
    let _ = app.frame();
    for slot in 0..3 {
        assert!(!app.offers(Action::WalletKeyOut(w, slot)));
    }
    assert!(app.backup_item_done(plan::Item::PublicFiles));
    // The shares are the next item, a press away.
    press_offered(&mut app, Action::BStep(bstep::SHEETS));
    assert_eq!(app.backup.as_ref().unwrap().open, Some(bstep::SHEETS));
    press_offered(&mut app, Action::BOut(4));
    assert!(in_outbox(&app, "savings-share-1-of-3.pdf"));
    assert!(app.backup_item_done(plan::Item::Sheets));
}

#[test]
fn a_restored_single_key_wallet_saves_itself_into_a_vault_from_its_backup() {
    let mut app = device(vec![kit_file("vault.ofv")]);
    let fp = add_key(&mut app, 1);
    app.press(Action::KeyWallet(fp.0, 1));
    app.press(Action::Seeds(S::Make));
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    press_offered(&mut app, Action::BPreset(1));
    press_offered(&mut app, Action::BChecklist);
    // Seeds here: it opens on the blank templates.
    assert_eq!(app.backup.as_ref().unwrap().open, Some(bstep::BLANK));
    press_offered(&mut app, Action::BStep(bstep::WALLET));
    // The vault is locked: Unlock it, and back to this step.
    app.vaults.ms_per_unit = Some(180);
    press_offered(&mut app, Action::Vault(V::OpenFrom(0, Screen::Backup)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults.open.is_empty() && app.screen == Screen::Backup {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "the test vault did not unlock");
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.backup.as_ref().unwrap().open, Some(bstep::WALLET));
    assert_eq!(app.backup_kept().unwrap().wallet.1, Tone::Warn);
    press_offered(&mut app, Action::Vault(V::SaveWallet(w)));
    let kept = app.backup_kept().unwrap();
    assert_eq!(kept.wallet.1, Tone::Ok, "{:?}", kept.wallet);
    assert!(kept.wallet.0.starts_with("Wallet in "), "{:?}", kept.wallet);
    // Saved once: the item says so rather than offering it again.
    let _ = app.frame();
    assert!(!app.offers(Action::Vault(V::SaveWallet(w))));
    assert!(app.backup_item_done(plan::Item::WalletVault));
}

#[test]
fn back_up_from_tools_takes_the_wallet_picked_and_its_chip_switches_to_the_other() {
    let mut app = device(vec![
        kit_file("savings-wallet.txt"),
        kit_file("spending-wallet.txt"),
    ]);
    let savings = load(&mut app, "savings-wallet.txt");
    let spending = load(&mut app, "spending-wallet.txt");
    app.press(Action::OpenWallet(spending));
    let tile = TILES
        .iter()
        .position(|t| t.go == Go::Backup)
        .expect("a Back up tile") as u8;
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile));
    assert_eq!(app.screen, Screen::Backup);
    assert_eq!(app.backup.as_ref().unwrap().wallet, spending);
    // The chip names it, and lists the loaded wallets.
    press_offered(&mut app, Action::BWallets);
    press_offered(&mut app, Action::Backup(savings));
    let b = app.backup.as_ref().unwrap();
    assert_eq!(b.wallet, savings);
    assert!(!b.pick, "the list closes");
    // A press beside the list closes it, the backup unchanged.
    press_offered(&mut app, Action::BWallets);
    assert!(app.backup.as_ref().unwrap().pick);
    app.press(Action::BWallets);
    assert!(!app.backup.as_ref().unwrap().pick);
    assert_eq!(app.backup.as_ref().unwrap().wallet, savings);
}

#[test]
fn one_wallet_has_no_chip() {
    let mut app = device(vec![kit_file("spending-wallet.txt")]);
    let w = load(&mut app, "spending-wallet.txt");
    app.press(Action::Backup(w));
    let _ = app.frame();
    assert!(!app.offers(Action::BWallets));
}

#[test]
fn the_key_rows_list_the_keys_whose_seeds_are_here() {
    let mut app = device(vec![kit_file("savings-wallet.txt")]);
    let w = load(&mut app, "savings-wallet.txt");
    let here = [add_key(&mut app, 0), add_key(&mut app, 2)];
    let slot_of = |app: &Faraday, fp| {
        app.session.wallets[w]
            .policy
            .keys()
            .iter()
            .position(|k| k.fingerprint() == Some(fp))
            .expect("a slot") as u8
    };
    app.press(Action::Backup(w));
    every_software(&mut app);
    app.press(Action::BStep(bstep::PUBLIC));
    let _ = app.frame();
    // Test key 2's seed is not here: its slot has no row.
    let away = (0..3u8)
        .find(|&k| {
            let fp = app.session.wallets[w].policy.keys()[usize::from(k)].fingerprint();
            !here.iter().any(|h| Some(*h) == fp)
        })
        .expect("a slot not held here");
    assert!(!app.offers(Action::WalletKeyOut(w, away)));
    for fp in here {
        let slot = slot_of(&app, fp);
        let name = faraday_core::wallet::fp_text(fp);
        press_offered(&mut app, Action::WalletKeyOut(w, slot));
        let file = app
            .outbox
            .iter()
            .find(|i| i.name == format!("xpub-{name}.txt"))
            .expect("the key's file");
        let text = String::from_utf8(file.bytes.clone()).unwrap();
        let key = app.session.wallets[w].policy.keys()[usize::from(slot)].key_text();
        assert!(text.contains(&key), "{text}");
        press_offered(&mut app, Action::WalletKeyBsms(w, slot));
        let record = app
            .outbox
            .iter()
            .find(|i| i.name == format!("xpub-{name}-bsms.txt"))
            .expect("the key's BSMS record");
        assert!(String::from_utf8_lossy(&record.bytes).starts_with("BSMS 1.0"));
        press_offered(&mut app, Action::CodePng(Code::WalletKey(w, slot)));
        assert!(in_outbox(&app, &format!("xpub-{name}.png")));
    }
    let sent = &app.backup.as_ref().unwrap().sent;
    assert!(sent.iter().any(|n| n.ends_with("-bsms.txt")), "{sent:?}");
}
