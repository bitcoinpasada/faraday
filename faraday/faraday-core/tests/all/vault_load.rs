//! Unlocking a vault asks what to load (`docs/DECISIONS.md` F62). After
//! an unlock from the boot sheet or the Vaults tab, **Load from {vault}**
//! shows a card per wallet with the seeds of it the vault keeps and a
//! Keys card for the rest, ticked from each record's Load at unlock.
//! **Load ticked** loads exactly what is ticked, each thing once; **Not
//! now** loads nothing and leaves the vault open. A tick changed on the
//! sheet never changes the saved setting. A wallet the backup flow seals
//! into a vault loads at unlock unless its vault item turns it off.

use crate::common::{device, type_text};
use std::rc::Rc;

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_core::boot_import::ImportAction as I;
use faraday_core::vault::VaultAction as V;
use faraday_core::vault::load::{LoadCard, LoadView};
use faraday_core::{Action, Faraday, Screen, Sheet, StickInfo, StorageCommand, StorageEvent};
use faraday_core::{bstep, testkit};
use faraday_shell_api::{App, Event};
use faraday_vault::records::{self, field, kind};
use faraday_vault::{Contents, Cost, Record};

type Files = Vec<(String, Vec<u8>)>;

const PHRASE: &str = "load phrase";

/// Unlocks vault file `name` from the Vaults tab with `phrase`.
fn unlock_on_vaults(app: &mut Faraday, name: &str, phrase: &str) {
    app.press(Action::Nav(Screen::Vaults));
    let n = app
        .vault_files()
        .iter()
        .position(|f| f.name == name)
        .expect("the vault is listed");
    let before = app.vaults().open.len();
    app.press(Action::Vault(V::Open(n)));
    type_text(app, phrase);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if app.vaults().open.len() > before {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(
        app.vaults().open.len(),
        before + 1,
        "{:?}",
        app.vaults().unlock_error
    );
}

fn words(phrase: &str) -> Mnemonic {
    let c = testkit::compute();
    Mnemonic::parse(&c, Language::English, phrase).unwrap()
}

fn test_key(n: usize) -> Record {
    Record::new(kind::KEY)
        .with(
            field::KEY,
            &records::words_payload(&words(&testkit::test_words(testkit::TEST_SEEDS[n].0))),
        )
        .with(field::KEY_LABEL, testkit::TEST_SEEDS[n].1.as_bytes())
}

fn loose_key(phrase: &str, label: &str) -> Record {
    Record::new(kind::KEY)
        .with(field::KEY, &records::words_payload(&words(phrase)))
        .with(field::KEY_LABEL, label.as_bytes())
}

fn wallet(name: &str, text: &str) -> Record {
    Record::new(kind::WALLET)
        .with(field::WALLET, text.as_bytes())
        .with(field::WALLET_NAME, name.as_bytes())
}

fn not_at_unlock(mut r: Record) -> Record {
    records::set_loads_at_unlock(&mut r, false);
    r
}

/// A vault of test keys 1 and 2; Savings (the 2-of-3 over the three
/// test keys, so both are its seeds); Spending (test key 1 alone) set not
/// to load at unlock; a single-key wallet over test key 3, which the
/// vault does not hold; and two keys in no wallet, the second set not to
/// load at unlock.
fn load_vault() -> Vec<u8> {
    let c = testkit::compute();
    let slot = Contents {
        records: vec![
            Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Cellar"),
            test_key(0),
            test_key(1),
            loose_key(
                "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
                "Loose A",
            ),
            not_at_unlock(loose_key(
                "legal winner thank year wave sausage worth useful legal winner thank yellow",
                "Loose B",
            )),
            wallet("Savings", &testkit::savings()),
            not_at_unlock(wallet(
                "Spending",
                &format!("wpkh({}/<0;1>/*)", testkit::key(0, "m/84'/1'/0'")),
            )),
            wallet(
                "Summer",
                &format!("wpkh({}/<0;1>/*)", testkit::key(2, "m/84'/1'/0'")),
            ),
        ],
    };
    let cost = Cost {
        memory_kib: 64 * 1024,
        passes: 1,
        lanes: 1,
    };
    faraday_vault::create(
        &c,
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[PHRASE],
        &[slot],
        &[0x5d; 32],
    )
    .unwrap()
}

fn view(app: &Faraday) -> Rc<LoadView> {
    let c = testkit::compute();
    app.vault_load_view_now(&c).expect("the Load sheet is up")
}

fn card<'a>(v: &'a LoadView, name: &str) -> &'a LoadCard {
    v.cards.iter().find(|c| c.name == name).expect(name)
}

fn loose<'a>(v: &'a LoadView, label: &str) -> &'a faraday_core::vault::load::LoadSeed {
    v.keys
        .iter()
        .find(|k| k.line.ends_with(label))
        .expect(label)
}

fn wallet_names(app: &Faraday) -> Vec<String> {
    let mut v: Vec<String> = app.session.wallets.iter().map(|w| w.name.clone()).collect();
    v.sort();
    v
}

fn key_labels(app: &Faraday) -> Vec<String> {
    let mut v: Vec<String> = app.session.keys.all().map(|k| k.label.clone()).collect();
    v.sort();
    v
}

/// The sheet after an unlock from the Vaults tab: a card per wallet in
/// the vault's order with its kind, its seeds here and what is kept
/// elsewhere, the Keys card last; each ticked from its setting. Load
/// ticked loads exactly that and lands on Wallets.
#[test]
fn from_the_vaults_tab_the_sheet_ticks_by_setting_and_load_ticked_loads_that() {
    let mut app = device(vec![("cellar.fvlt".into(), load_vault())]);
    unlock_on_vaults(&mut app, "cellar.fvlt", PHRASE);
    assert_eq!(app.sheet, Some(Sheet::UnlockLoad));
    assert!(app.session.keys.is_empty() && app.session.wallets.is_empty());
    let v = view(&app);
    assert_eq!(v.title, "Load from Cellar");
    assert_eq!(v.count, "3 wallets · 4 keys");
    let names: Vec<&str> = v.cards.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["Savings", "Spending", "Summer"]);
    let savings = card(&v, "Savings");
    assert!(savings.ticked);
    assert_eq!(savings.kind, "2 of 3 · native SegWit multisig");
    assert_eq!(savings.seeds.len(), 2);
    assert_eq!(
        savings.elsewhere.as_deref(),
        Some("1 of 3 seeds kept elsewhere")
    );
    // Test key 1 is Spending's seed too.
    let key1 = &savings.seeds[0];
    assert!(key1.line.ends_with("Test key 1"), "{}", key1.line);
    assert_eq!(key1.also, ["Spending"]);
    assert_eq!(key1.also_line.as_deref(), Some("also in Spending"));
    assert!(!card(&v, "Spending").ticked, "set not to load at unlock");
    let summer = card(&v, "Summer");
    assert!(summer.ticked && summer.seeds.is_empty());
    assert_eq!(summer.elsewhere.as_deref(), Some("Watch-only here"));
    assert!(loose(&v, "Loose A").ticked);
    assert!(!loose(&v, "Loose B").ticked, "set not to load at unlock");
    // The texts a person reads.
    let _ = app.frame();
    let texts = app.drawn_texts();
    for want in [
        "Load from Cellar",
        "Unselect all",
        "Not now",
        "Load ticked",
        "Watch-only here",
        "also in Spending",
    ] {
        assert!(texts.iter().any(|t| t == want), "{want} not in {texts:?}");
    }

    app.press(Action::Vault(V::LoadTicked));
    assert_eq!((app.screen, app.sheet), (Screen::Wallets, None));
    assert_eq!(wallet_names(&app), ["Savings", "Summer"]);
    assert_eq!(key_labels(&app), ["Loose A", "Test key 1", "Test key 2"]);
}

/// Not now loads nothing, the vault stays open on Files, and the Vaults
/// tab still loads from it.
#[test]
fn not_now_loads_nothing_and_the_vault_can_still_load_later() {
    let mut app = device(vec![("cellar.fvlt".into(), load_vault())]);
    unlock_on_vaults(&mut app, "cellar.fvlt", PHRASE);
    app.press(Action::Vault(V::LoadNotNow));
    assert_eq!((app.screen, app.sheet), (Screen::Files, None));
    assert_eq!(app.vaults().open.len(), 1);
    assert!(app.session.keys.is_empty() && app.session.wallets.is_empty());
    app.press(Action::Vault(V::LoadAll(0)));
    assert_eq!(app.session.wallets.len(), 3);
    assert_eq!(app.session.keys.len(), 4);
}

/// A wallet set not to load starts unticked; ticked on the sheet it
/// loads with its seed, and its setting stays No.
#[test]
fn a_wallet_set_not_to_load_ticked_on_the_sheet_loads_and_keeps_its_setting() {
    let mut app = device(vec![("cellar.fvlt".into(), load_vault())]);
    unlock_on_vaults(&mut app, "cellar.fvlt", PHRASE);
    let spending = card(&view(&app), "Spending").record;
    let savings = card(&view(&app), "Savings").record;
    // Only Spending.
    app.press(Action::Vault(V::LoadTickAll));
    assert!(!view(&app).any_ticked(), "Unselect all");
    app.press(Action::Vault(V::LoadTick(spending)));
    app.press(Action::Vault(V::LoadTicked));
    assert_eq!(wallet_names(&app), ["Spending"]);
    assert_eq!(key_labels(&app), ["Test key 1"]);
    let r = &app
        .vaults()
        .store()
        .contents_for_tests(app.vaults().open[0].id)
        .records[spending];
    assert!(!records::loads_at_unlock(r), "the tick changed the setting");
    let r = &app
        .vaults()
        .store()
        .contents_for_tests(app.vaults().open[0].id)
        .records[savings];
    assert!(records::loads_at_unlock(r));
    assert_eq!(
        app.vaults().open[0].changes,
        0,
        "the sheet changed the vault"
    );
}

/// A seed in two wallets loads with the one ticked, and the other wallet
/// does not load.
#[test]
fn a_seed_in_two_wallets_loads_with_the_ticked_one_alone() {
    let mut app = device(vec![("cellar.fvlt".into(), load_vault())]);
    unlock_on_vaults(&mut app, "cellar.fvlt", PHRASE);
    let v = view(&app);
    let (savings, summer, a) = (
        card(&v, "Savings").record,
        card(&v, "Summer").record,
        loose(&v, "Loose A").record,
    );
    for r in [summer, a] {
        app.press(Action::Vault(V::LoadTick(r)));
    }
    assert!(
        view(&app)
            .cards
            .iter()
            .all(|c| c.ticked == (c.record == savings))
    );
    app.press(Action::Vault(V::LoadTicked));
    assert_eq!(wallet_names(&app), ["Savings"]);
    assert_eq!(key_labels(&app), ["Test key 1", "Test key 2"]);
}

/// A key in no wallet of the vault loads by its own setting: on its own,
/// Loose A ticked and Loose B not; ticked by hand, Loose B loads.
#[test]
fn a_key_in_no_wallet_follows_its_own_setting() {
    let mut app = device(vec![("cellar.fvlt".into(), load_vault())]);
    unlock_on_vaults(&mut app, "cellar.fvlt", PHRASE);
    let v = view(&app);
    for c in &v.cards {
        if c.ticked {
            app.press(Action::Vault(V::LoadTick(c.record)));
        }
    }
    let b = loose(&v, "Loose B").record;
    app.press(Action::Vault(V::LoadTick(b)));
    app.press(Action::Vault(V::LoadTicked));
    assert!(app.session.wallets.is_empty());
    assert_eq!(key_labels(&app), ["Loose A", "Loose B"]);
    // Load ticked with only keys stays on Files.
    assert_eq!((app.screen, app.sheet), (Screen::Files, None));
}

/// What the session holds already is Loaded on the sheet: a card with
/// everything loaded has no checkbox, one with something missing loads
/// the rest, and nothing loads twice.
#[test]
fn what_is_loaded_already_shows_loaded_and_is_not_loaded_twice() {
    let c = faraday_core::testkit::compute();
    let mut app = device(vec![("cellar.fvlt".into(), load_vault())]);
    // Test key 1 and Summer, here before the unlock.
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[0].0));
    app.press(Action::EntryAdd);
    app.session
        .add_wallet(
            &c,
            "Summer",
            &format!("wpkh({}/<0;1>/*)", testkit::key(2, "m/84'/1'/0'")),
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    unlock_on_vaults(&mut app, "cellar.fvlt", PHRASE);
    let v = view(&app);
    let summer = card(&v, "Summer");
    assert!(summer.loaded && summer.done && !summer.ticked);
    let savings = card(&v, "Savings");
    assert!(!savings.done && savings.ticked);
    assert!(savings.seeds[0].loaded && !savings.seeds[1].loaded);
    let _ = app.frame();
    assert!(app.drawn_texts().iter().any(|t| t == "Loaded"));
    // A card with nothing left to load takes no tick.
    app.press(Action::Vault(V::LoadTick(summer.record)));
    assert!(!card(&view(&app), "Summer").ticked);
    app.press(Action::Vault(V::LoadTicked));
    assert_eq!(wallet_names(&app), ["Savings", "Summer"]);
    assert_eq!(key_labels(&app).len(), 3, "{:?}", key_labels(&app));
    let ones = app
        .session
        .keys
        .all()
        .filter(|k| k.label == testkit::TEST_SEEDS[1].1)
        .count();
    assert_eq!(ones, 1);
}

/// Answers the app's reads from the boot stick's files.
fn pump(app: &mut Faraday, files: &[(String, Vec<u8>)]) {
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c
            && let Some((_, bytes)) = files.iter().find(|(n, _)| *n == name)
        {
            app.storage(StorageEvent::Read {
                stick,
                name,
                bytes: bytes.clone(),
            });
        }
    }
}

/// From the boot sheet: the Load sheet after the unlock, ticked by
/// setting; Load ticked loads that and ends the import on Wallets. Not
/// now on the first vault goes on to the next one's Unlock.
#[test]
fn from_the_boot_sheet_each_unlock_asks_what_to_load_before_the_next() {
    let files: Files = vec![
        ("cellar.fvlt".into(), load_vault()),
        ("vault.fvlt".into(), testkit::test_vault().unwrap()),
    ];
    let mut app = testkit::started();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    faraday_core::testkit::vault_speed(&mut app, Some(100));
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        serial: None,
        id: "boot".into(),
        label: "FARADAY".into(),
        boot: true,
        files: files
            .iter()
            .map(|(n, b)| (n.clone(), b.len() as u64))
            .collect(),
    }]));
    app.press(Action::Import(I::Go));
    pump(&mut app, &files);
    app.storage(StorageEvent::Sticks(Vec::new()));
    let unlock = |app: &mut Faraday, phrase: &str| {
        type_text(app, phrase);
        app.press(Action::Import(I::Submit));
        let _ = app.frame();
        app.event(Event::Tick { now_ms: 1000 });
        let _ = app.frame();
    };
    assert_eq!(
        app.import_field().map(|v| v.name),
        Some("cellar.fvlt".into())
    );
    unlock(&mut app, PHRASE);
    assert_eq!(
        (app.screen, app.sheet),
        (Screen::Home, Some(Sheet::UnlockLoad))
    );
    assert!(!card(&view(&app), "Spending").ticked);
    app.press(Action::Vault(V::LoadNotNow));
    assert!(app.session.wallets.is_empty());
    // The next vault's Unlock, then its Load sheet.
    assert_eq!(app.sheet, Some(Sheet::Import));
    assert_eq!(
        app.import_field().map(|v| v.name),
        Some("vault.fvlt".into())
    );
    unlock(&mut app, testkit::VAULT_PASSPHRASES[0]);
    assert_eq!(app.sheet, Some(Sheet::UnlockLoad));
    assert!(view(&app).cards.iter().all(|c| c.ticked));
    // Test key 1 is in every test wallet: counted, not named.
    let n = card(&view(&app), "Savings").seeds[0].also.len();
    assert!(n >= 3);
    assert_eq!(
        card(&view(&app), "Savings").seeds[0].also_line,
        Some(format!("also in {n} other wallets"))
    );
    app.press(Action::Vault(V::LoadTicked));
    assert!(app.import().is_none());
    assert_eq!((app.screen, app.sheet), (Screen::Wallets, None));
    assert!(app.session.wallets.iter().any(|w| w.name == "Savings"));
    assert_eq!(app.vaults().open.len(), 2);
}

/// A wallet the backup's checklist saves into a vault loads at unlock;
/// with the vault item's Load at unlock turned off it starts unticked at
/// the next unlock, on another power-on.
#[test]
fn a_wallet_backed_up_into_a_vault_loads_at_unlock_unless_turned_off() {
    let c = faraday_core::testkit::compute();
    for off in [false, true] {
        let vault = testkit::files()
            .unwrap()
            .into_iter()
            .find(|(n, _)| n == "vault.fvlt")
            .unwrap();
        let mut app = device(vec![vault]);
        // Test key 2 typed in and a one-key wallet over it.
        app.press(Action::Entry(None));
        type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
        app.press(Action::EntryAdd);
        let fp = app.session.keys.last().unwrap().fingerprint;
        app.press(Action::KeyWallet(fp.0, 1));
        app.press(Action::Seeds(faraday_core::seeds::SeedsAction::Make));
        let w = app.session.wallets.len() - 1;
        let name = app.session.wallets[w].name.clone();
        unlock_on_vaults(&mut app, "vault.fvlt", testkit::VAULT_PASSPHRASES[0]);
        app.press(Action::Vault(V::LoadNotNow));
        app.press(Action::Backup(w));
        testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
        app.press(Action::BChecklist);
        if app.backup_items_now(&c).contains(&bstep::BLANK) {
            app.press(Action::BNext(bstep::BLANK));
        }
        // The copy by hand first, typed back: the vault item comes after.
        app.press(Action::BReveal);
        app.storage(StorageEvent::Cameras(Vec::new()));
        app.press(Action::BCheck);
        testkit::type_words(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
        // Matched, the copy's item moved on by itself.
        if !crate::common::moved_on(&app, Action::BStep(bstep::VAULT)) {
            app.press(Action::BStep(bstep::VAULT));
        }
        // The vault item's Load at unlock, somewhere down its column.
        let mut offered = false;
        for _ in 0..20 {
            app.settle();
            let _ = app.frame();
            if app.offers(Action::BVaultLoad(0)) {
                offered = true;
                break;
            }
            app.event(Event::Scroll {
                x: 400,
                y: 384,
                dy: 300,
            });
        }
        let texts = app.drawn_texts();
        assert!(
            offered,
            "no Load at unlock: {:?} {:?} {:?} {texts:?}",
            app.screen,
            app.sheet,
            app.backup().map(|b| b.open),
        );
        if off {
            app.press(Action::BVaultLoad(0));
        }
        app.press(Action::BVaultSave(0));
        let ours = |app: &Faraday| -> bool {
            let (_, r) = app
                .vaults()
                .store()
                .contents_for_tests(app.vaults().open[0].id)
                .of(kind::WALLET)
                .find(|(_, r)| r.text(field::WALLET_NAME) == Some(name.as_str()))
                .expect("the wallet is in the vault");
            records::loads_at_unlock(r)
        };
        assert_eq!(ours(&app), !off);

        // Another power-on with the vault sealed at the lock, with the
        // randomness a shell gives.
        for i in 0..8u8 {
            app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
                [0x60 + i; 32],
            )));
        }
        app.press(Action::Lock);
        if app.sheet == Some(Sheet::LockAsk) {
            app.press(Action::LockUnsealed);
        }
        let mut outbox: Files = Vec::new();
        while let Some(c) = app.poll_storage() {
            if let StorageCommand::SaveBoxes { outbox: o, .. } = c {
                outbox = o;
            }
        }
        let sealed: Files = outbox
            .into_iter()
            .filter(|(n, _)| n.ends_with(".fvlt"))
            .collect();
        assert_eq!(sealed.len(), 1, "the vault was not sealed");
        let file = sealed[0].0.clone();
        let mut app = device(sealed);
        unlock_on_vaults(&mut app, &file, testkit::VAULT_PASSPHRASES[0]);
        assert_eq!(card(&view(&app), &name).ticked, !off, "off: {off}");
    }
}

/// The 2-of-3 over the three test keys, its third used with a BIP-39
/// passphrase the vault does not keep.
fn trio_with_passphrase() -> String {
    let c = testkit::compute();
    use faraday_bip::bitcoin::bip32::DerivationPath;
    use faraday_bip::keys::MasterKey;
    let p = "m/48'/1'/0'/2'";
    let m = words(&testkit::test_words(testkit::TEST_SEEDS[2].0));
    let master = MasterKey::from_seed(&c, &m.to_seed(&c, b"kept apart").unwrap(), testkit::NET);
    let path: DerivationPath = p.parse().unwrap();
    let third = format!(
        "[{}/48h/1h/0h/2h]{}",
        master.fingerprint(),
        master.derive(&c, &path).to_xpub(&c)
    );
    format!(
        "wsh(sortedmulti(2,{}/<0;1>/*,{}/<0;1>/*,{third}/<0;1>/*))",
        testkit::key(0, p),
        testkit::key(1, p)
    )
}

/// A vault of the three test keys, the third kept without the
/// passphrase Trio uses it with, Trio, and `extra`.
fn trio_vault(extra: Vec<Record>) -> Vec<u8> {
    let c = testkit::compute();
    let mut records = vec![
        Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Trio"),
        test_key(0),
        test_key(1),
        test_key(2),
        wallet("Trio", &trio_with_passphrase()),
    ];
    records.extend(extra);
    let cost = Cost {
        memory_kib: 64 * 1024,
        passes: 1,
        lanes: 1,
    };
    faraday_vault::create(
        &c,
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[PHRASE],
        &[Contents { records }],
        &[0x5e; 32],
    )
    .unwrap()
}

/// Trio's vault unlocked and Trio's card alone loaded: two of its three
/// keys here.
fn trio_loaded(extra: Vec<Record>) -> Faraday {
    let mut app = device(vec![("trio.fvlt".into(), trio_vault(extra))]);
    unlock_on_vaults(&mut app, "trio.fvlt", PHRASE);
    let v = view(&app);
    for k in v.keys.iter().filter(|k| k.ticked) {
        app.press(Action::Vault(V::LoadTick(k.record)));
    }
    app.press(Action::Vault(V::LoadTicked));
    assert_eq!(wallet_names(&app), ["Trio"]);
    assert_eq!(held(&app), 2);
    app
}

fn held(app: &Faraday) -> usize {
    app.session
        .slots(&app.session.wallets[0])
        .iter()
        .filter(|s| s.held_by.is_some())
        .count()
}

/// The chart's Load with a passphrase, on Trio's card.
fn load_with_a_passphrase(app: &mut Faraday) {
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let way = Action::Vault(V::PassphraseKey(0));
    assert!(app.offers(way), "{:?}", app.drawn_texts());
    app.press(way);
    assert_eq!(app.sheet, Some(Sheet::PassphraseSeeds));
}

fn try_passphrase(app: &mut Faraday, passphrase: &str) {
    type_text(app, passphrase);
    app.press(Action::Vault(V::PromptGo));
}

/// A seed kept without the passphrase a wallet of the vault uses it with
/// is another key: the sheet keeps it off the wallet's card, on the Keys
/// card. The wallet loaded short of it, its chart offers the seed with a
/// passphrase typed now; the one candidate opens its field at once, and
/// the right passphrase gives the wallet every key.
#[test]
fn a_seed_kept_without_its_passphrase_is_on_the_keys_card_and_loads_with_one_later() {
    let mut app = device(vec![("trio.fvlt".into(), trio_vault(Vec::new()))]);
    unlock_on_vaults(&mut app, "trio.fvlt", PHRASE);
    let v = view(&app);
    let trio = card(&v, "Trio");
    assert_eq!(trio.seeds.len(), 2);
    assert!(
        trio.seeds.iter().all(|s| !s.line.ends_with("Test key 3")),
        "the passphrase-less seed is on the wallet's card"
    );
    assert_eq!(
        trio.elsewhere.as_deref(),
        Some("1 of 3 seeds kept elsewhere")
    );
    assert_eq!(v.keys.len(), 1);
    assert!(v.keys[0].line.ends_with("Test key 3"));
    let mut app = trio_loaded(Vec::new());
    load_with_a_passphrase(&mut app);
    // One seed: its field, not the list.
    assert!(app.vaults().pass_seeds.as_ref().unwrap().picked.is_some());
    try_passphrase(&mut app, "kept apart");
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Wallets);
    assert_eq!(held(&app), 3, "the wallet is still short of its key");
}

/// A wrong passphrase on the right seed loads nothing and says which key
/// it gave; the right one then loads.
#[test]
fn a_wrong_passphrase_loads_nothing_and_says_what_it_gave() {
    let mut app = trio_loaded(Vec::new());
    load_with_a_passphrase(&mut app);
    try_passphrase(&mut app, "kept apart, wrongly");
    assert_eq!(app.session.keys.len(), 2);
    assert_eq!(app.sheet, Some(Sheet::PassphraseSeeds));
    let error = app.vaults().prompt.as_ref().unwrap().error.clone().unwrap();
    assert!(
        error.starts_with("This passphrase gives ") && error.ends_with(", not a key of Trio"),
        "{error}"
    );
    let _ = app.frame();
    assert!(app.drawn_texts().contains(&error));
    try_passphrase(&mut app, "kept apart");
    assert_eq!(held(&app), 3);
    // Never stored in the vault.
    let stored = app
        .vaults()
        .store()
        .contents_for_tests(app.vaults().open[0].id)
        .of(kind::KEY)
        .any(|(_, r)| r.field(field::KEY_PASSPHRASE).is_some());
    assert!(!stored);
}

/// With two seeds kept with no passphrase the sheet lists both; the one
/// not of the wallet loads nothing whatever passphrase is typed.
#[test]
fn two_seeds_with_no_passphrase_are_listed_and_the_other_one_loads_nothing() {
    let c = testkit::compute();
    let mut app = trio_loaded(vec![loose_key(
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        "Loose A",
    )]);
    load_with_a_passphrase(&mut app);
    assert!(app.vaults().pass_seeds.as_ref().unwrap().picked.is_none());
    let seeds = app.vault_passphrase_seeds_now(&c);
    assert_eq!(seeds.len(), 2);
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Seeds with no passphrase stored"));
    for s in seeds.iter() {
        assert!(texts.contains(&s.line), "{} not in {texts:?}", s.line);
    }
    let loose = seeds.iter().find(|s| s.line.ends_with("Loose A")).unwrap();
    app.press(Action::Vault(V::PassphrasePick(loose.vault, loose.record)));
    try_passphrase(&mut app, "kept apart");
    assert_eq!(app.session.keys.len(), 2, "an unrelated key loaded");
    assert!(app.vaults().prompt.as_ref().unwrap().error.is_some());
    // Another seed: the list again, then the right one.
    app.press(Action::Vault(V::PassphraseList));
    let third = seeds
        .iter()
        .find(|s| s.line.ends_with("Test key 3"))
        .unwrap();
    app.press(Action::Vault(V::PassphrasePick(third.vault, third.record)));
    try_passphrase(&mut app, "kept apart");
    assert_eq!(held(&app), 3);
}
