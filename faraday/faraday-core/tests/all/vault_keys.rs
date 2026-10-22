//! Keys wherever they are: a wallet's missing key loads from an open vault
//! that holds it, a locked vault is offered for unlocking and the person
//! comes back to where they were, a key the quorum does not need is not
//! asked for, and a vault holding wallets with their seeds loads each
//! with its seeds in one press.

use faraday_core::vault::{SlotOf, VaultAction as V};
use faraday_core::{Action, Faraday, Screen, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

fn kit_file(name: &str) -> (String, Vec<u8>) {
    testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap()
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
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.vaults.ms_per_unit = Some(180);
    app
}

fn find(app: &Faraday, name: &str) -> usize {
    app.inbox.iter().position(|i| i.name == name).unwrap()
}

/// Types the test vault's passphrase on the Unlock screen and waits.
fn type_and_unlock(app: &mut Faraday) {
    for c in testkit::VAULT_PASSPHRASES[0].chars() {
        app.event(Event::Key(Key::Char(c)));
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
}

fn add_key(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

#[test]
fn a_missing_key_in_an_open_vault_loads_from_the_wallet_card() {
    let mut app = device(vec![kit_file("spending-wallet.txt"), kit_file("vault.ofv")]);
    // Opened from a flow (the vault way), not the Vaults list: it shows
    // what it holds without loading it until asked, as today
    // (`docs/history/NEW-WALLET.md` §11.1a).
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
    type_and_unlock(&mut app);
    assert!(app.session.keys.is_empty(), "unlocking loads nothing");
    let w = find(&app, "spending-wallet.txt");
    app.press(Action::LoadWallet(w));
    assert_eq!(app.screen, Screen::Wallets);
    let _ = app.frame();
    let fp = app.session.slots(&app.session.wallets[0])[0]
        .fingerprint
        .unwrap();
    assert!(app.offers(Action::Vault(V::LoadKeyOf(fp.0, SlotOf::Loaded(0, 0)))));
    app.press(Action::Vault(V::LoadKeyOf(fp.0, SlotOf::Loaded(0, 0))));
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.keys[0].fingerprint, fp);
}

#[test]
fn a_locked_vault_is_offered_from_the_wallet_card_and_unlocking_comes_back() {
    let mut app = device(vec![kit_file("spending-wallet.txt"), kit_file("vault.ofv")]);
    let w = find(&app, "spending-wallet.txt");
    app.press(Action::LoadWallet(w));
    let _ = app.frame();
    let unlock = Action::Vault(V::OpenFrom(0, Screen::Wallets));
    assert!(app.offers(unlock), "the locked vault is not offered");
    app.press(unlock);
    assert_eq!(app.screen, Screen::Unlock);
    type_and_unlock(&mut app);
    assert_eq!(app.screen, Screen::Wallets, "unlocking does not come back");
    let _ = app.frame();
    let fp = app.session.slots(&app.session.wallets[0])[0]
        .fingerprint
        .unwrap();
    assert!(app.offers(Action::Vault(V::LoadKeyOf(fp.0, SlotOf::Loaded(0, 0)))));
}

#[test]
fn a_key_the_quorum_does_not_need_is_not_asked_for() {
    let mut app = device(vec![kit_file("savings-wallet.txt")]);
    add_key(&mut app, 0);
    add_key(&mut app, 1);
    let w = find(&app, "savings-wallet.txt");
    app.press(Action::LoadWallet(w));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    let slots = app.session.slots(&app.session.wallets[0]);
    let missing: Vec<_> = slots.iter().filter(|s| s.held_by.is_none()).collect();
    assert_eq!(missing.len(), 1, "Savings is a 2 of 3 with two keys here");
    assert!(!app.offers(Action::Entry(missing[0].fingerprint.map(|f| f.0))));
}

#[test]
fn a_key_the_quorum_needs_is_asked_for() {
    let mut app = device(vec![kit_file("savings-wallet.txt")]);
    add_key(&mut app, 0);
    let w = find(&app, "savings-wallet.txt");
    app.press(Action::LoadWallet(w));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    // One Scan a key takes whichever missing key comes
    // (`docs/SCAN-TO-SIGN.md` §2), not a way in per key.
    assert!(app.offers(Action::ScanKey(0)));
    let slots = app.session.slots(&app.session.wallets[0]);
    let per_key = slots
        .iter()
        .filter(|s| s.held_by.is_none())
        .filter(|s| app.offers(Action::Entry(s.fingerprint.map(|f| f.0))))
        .count();
    assert_eq!(per_key, 0);
}

#[test]
fn a_vaults_wallets_load_with_their_seeds_in_one_press() {
    let c = testkit::compute();
    let mut app = device(vec![kit_file("vault.ofv")]);
    // Opened from a flow (the vault way), not the Vaults list: nothing
    // loads until `LoadWithKeys` is pressed, as today
    // (`docs/history/NEW-WALLET.md` §11.1a).
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
    type_and_unlock(&mut app);
    let together = app.vault_wallets_with_keys(&c, 0);
    assert!(
        together.iter().any(|w| w.name == "Spending"),
        "Spending and test key 1 are in the test vault together"
    );
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::LoadWithKeys(0))));
    app.press(Action::Vault(V::LoadWithKeys(0)));
    assert_eq!(app.session.keys.len(), 1, "test key 1 loads with them");
    assert_eq!(app.session.wallets.len(), together.len());
    assert!(app.vault_wallets_with_keys(&c, 0).iter().all(|w| w.done));
}

#[test]
fn only_the_chosen_wallets_load_with_their_seeds() {
    let c = testkit::compute();
    let mut app = device(vec![kit_file("vault.ofv")]);
    // Opened from a flow (the vault way), not the Vaults list: nothing
    // loads until chosen (`docs/history/NEW-WALLET.md` §11.1a).
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
    type_and_unlock(&mut app);
    let together = app.vault_wallets_with_keys(&c, 0);
    assert!(together.len() > 1);
    // Keep only the first.
    for w in &together[1..] {
        app.press(Action::Vault(V::Choose(0, w.record)));
    }
    app.press(Action::Vault(V::LoadWithKeys(0)));
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.session.wallets[0].name, together[0].name);
}

// ---------------------------------------------------------------------
// §3.3 (`docs/history/SIMPLIFY.md`): Vault contents lists the kinds it holds,
// then Add….
// ---------------------------------------------------------------------

/// A vault made here and opened, with nothing in it.
fn new_open_vault(app: &mut Faraday) {
    for i in 0..8u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x50 + i; 32],
        )));
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Create));
    for second in [false, true] {
        app.press(Action::Vault(V::CFocus(0, second)));
        for c in "test phrase".chars() {
            app.event(Event::Key(Key::Char(c)));
        }
    }
    app.press(Action::Vault(V::CGo));
    for t in 1..60u64 {
        if app.screen == Screen::Unlock {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.screen, Screen::Unlock, "the vault was made");
    for c in "test phrase".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 60..120u64 {
        if !app.vaults.open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
    assert_eq!(app.vaults.open.len(), 1, "the new vault did not open");
}

#[test]
fn a_vault_with_one_key_lists_keys_every_add_and_no_other_kind() {
    let mut app = device(Vec::new());
    app.apk_signing = true;
    add_key(&mut app, 0);
    new_open_vault(&mut app);
    app.press(Action::Nav(Screen::VaultContents));
    let _ = app.frame();
    for k in 0..faraday_core::vault::CATEGORIES.len() {
        assert!(
            app.offers(Action::Vault(V::AddKind(k))),
            "an empty vault offers kind {k} to add"
        );
    }
    app.press(Action::Vault(V::AddKind(0)));
    app.press(Action::Vault(V::SaveKey(0)));
    app.press(Action::Vault(V::ItemBack));
    let _ = app.frame();
    assert!(app.offers(Action::Vault(V::Category(0))), "Bitcoin keys");
    for k in 0..faraday_core::vault::CATEGORIES.len() {
        assert!(
            app.offers(Action::Vault(V::AddKind(k))),
            "a vault with a key still offers kind {k} to add"
        );
    }
    for k in 1..faraday_core::vault::CATEGORIES.len() {
        assert!(
            !app.offers(Action::Vault(V::Category(k))),
            "kind {k} holds nothing and is not listed"
        );
    }
}

/// A key made here from coin flips and saved into the test vault, which
/// is locked: the vault as it is written to the stick, and the key's
/// words.
fn vault_holding_a_made_key() -> (Vec<(String, Vec<u8>)>, faraday_bip::bip39::Mnemonic) {
    let c = testkit::compute();
    use faraday_core::create::NewKind;
    use faraday_core::keygen::Way;
    use faraday_core::{StorageCommand, cstep};

    let mut app = device(vec![kit_file("vault.ofv")]);
    app.press(Action::CreateWallet);
    let at = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::NativeSegwit)
        .unwrap() as u8;
    app.press(Action::CKind(at));
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::KeyGen(Some(0)));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip((i * 7 + i / 5) % 2 == 0));
    }
    app.press(Action::KNext);
    app.press(Action::KLock);
    app.press(Action::KNext);
    let words = app
        .session
        .keys
        .reveal(0, faraday_keys::Reveal::Words)
        .to_string();
    let made =
        faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, &words)
            .unwrap();
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_and_unlock(&mut app);
    app.press(Action::Vault(V::AddKind(0)));
    app.press(Action::Vault(V::SaveKey(0)));
    // The system's randomness, which sealing the vault draws on.
    for i in 0..8u8 {
        app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
            [0x40 + i; 32],
        )));
    }
    app.press(Action::Lock);
    let mut out = None;
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { outbox, .. } = c {
            out = Some(outbox);
        }
    }
    let vault = out
        .expect("nothing queued on locking")
        .into_iter()
        .filter(|(n, _)| n == "vault.ofv")
        .collect();
    (vault, made)
}

fn type_words(
    app: &mut Faraday,
    language: faraday_bip::bip39::Language,
    m: &faraday_bip::bip39::Mnemonic,
) {
    app.press(Action::Entry(None));
    let at = faraday_core::forms::LANGUAGES
        .iter()
        .position(|l| *l == language)
        .unwrap();
    app.press(Action::EntryLanguage(at as u8));
    for (n, &i) in m.indices().iter().enumerate() {
        if n > 0 {
            app.event(Event::Key(Key::Char(' ')));
        }
        for c in language.typed(i).unwrap().as_chars() {
            app.event(Event::Key(Key::Char(*c)));
        }
    }
    app.press(Action::EntryAdd);
}

#[test]
fn a_key_whose_words_are_in_the_vault_reads_as_kept_there_and_another_does_not() {
    let c = testkit::compute();
    use faraday_bip::bip39::{Language, Mnemonic};
    let (vault, made) = vault_holding_a_made_key();
    let mut app = device(vault);
    app.vaults.ms_per_unit = Some(180);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_and_unlock(&mut app);
    // The vault's copy of the key says its backup is pending; a key of
    // other words is not the one it means.
    add_key(&mut app, 0);
    let other = app.session.keys[0].fingerprint;
    assert!(!app.key_held(&c, other), "a key of other words is held");
    // The same words typed are the key kept there.
    type_words(&mut app, Language::English, &made);
    assert_eq!(app.session.keys.len(), 2);
    let same = app.session.keys[1].fingerprint;
    assert!(
        app.key_held(&c, same),
        "the words in the vault read as kept"
    );
    assert!(!app.key_held(&c, other));
    // The same seed typed from another language's list reads as kept too.
    let es = Mnemonic::from_indices(Language::Spanish, made.indices()).unwrap();
    type_words(&mut app, Language::Spanish, &es);
    assert_eq!(app.session.keys.len(), 3, "{:?}", app.entry.error);
    assert!(app.key_held(&c, app.session.keys[2].fingerprint));
}

// ---- Create's Keys card offers an open vault's keys (NEW-WALLET §16.1) ----

/// The master fingerprint of `words` with no passphrase, worked out here.
fn fp_of_words(words: &str) -> [u8; 4] {
    let c = testkit::compute();
    let m = faraday_bip::bip39::Mnemonic::parse(&c, faraday_bip::bip39::Language::English, words)
        .unwrap();
    let seed = m.to_seed(&c, b"").unwrap();
    faraday_bip::keys::MasterKey::from_seed(&c, &seed, testkit::NET)
        .fingerprint()
        .0
}

/// Where open vault 0 keeps the key whose label is `label`.
fn key_record(app: &Faraday, label: &str) -> usize {
    use faraday_vault::records::{field, kind};
    app.vaults.open[0]
        .contents
        .records
        .iter()
        .position(|r| r.kind == kind::KEY && r.text(field::KEY_LABEL) == Some(label))
        .expect("the vault keeps the key")
}

/// Create opened as `kind` with its default quorum, on Keys.
fn create_on_keys(app: &mut Faraday, kind: faraday_core::create::NewKind) {
    use faraday_core::cstep;
    let i = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == kind)
        .unwrap() as u8;
    app.press(Action::CreateWallet);
    app.press(Action::CKind(i));
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CNext(cstep::QUORUM));
    assert_eq!(app.create.as_ref().and_then(|c| c.open), Some(cstep::KEYS));
}

#[test]
fn an_open_vaults_key_fills_a_slot_and_the_vault_stays_open() {
    use faraday_core::create::{NewKind, Source};
    let mut app = device(vec![kit_file("vault.ofv")]);
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
    type_and_unlock(&mut app);
    add_key(&mut app, 1);
    add_key(&mut app, 2);
    create_on_keys(&mut app, NewKind::Multi);
    let r = key_record(&app, testkit::TEST_SEEDS[0].1);
    let fp = fp_of_words(&testkit::test_words(testkit::TEST_SEEDS[0].0));
    let press = Action::CSlotVault(0, 0, r);
    let texts = app.drawn_texts();
    assert!(app.offers(press), "Key 1 offers the vault's key");
    assert!(
        texts.iter().any(|t| t == "vault.ofv"),
        "under the vault's name: {texts:?}"
    );
    let line = faraday_core::wallet::key_line(
        faraday_bip::keys::Fingerprint(fp),
        testkit::TEST_SEEDS[0].1,
    );
    assert!(texts.contains(&line), "the key reads {line}");
    app.press(press);
    assert_eq!(app.session.keys.len(), 3, "the key is loaded");
    assert!(app.session.keys.all().any(|k| k.fingerprint.0 == fp));
    assert_eq!(
        app.create.as_ref().map(|c| c.slots[0].clone()),
        Some(Source::Here(fp)),
        "and fills Key 1"
    );
    assert_eq!(app.vaults.open.len(), 1, "the vault is still open");
    let _ = app.frame();
    assert!(
        !app.offers(Action::CSlotVault(1, 0, r)),
        "a loaded key is offered from the session, not the vault"
    );
}

#[test]
fn a_threshold_kind_offers_only_a_vaults_shares() {
    let c = testkit::compute();
    use faraday_bip::bip39::{Language, Mnemonic};
    use faraday_core::create::NewKind;
    use faraday_vault::records::{self, field, kind};
    use faraday_vault::{Contents, Cost, Record};
    let twelve = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                  abandon about";
    let mut contents = Contents {
        records: vec![Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Mixed")],
    };
    for (words, label) in [
        (twelve.to_string(), "Twelve words"),
        (
            testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Twenty-four words",
        ),
    ] {
        let m = Mnemonic::parse(&c, Language::English, &words).unwrap();
        contents.records.push(
            Record::new(kind::KEY)
                .with(field::KEY, &records::words_payload(&m))
                .with(field::KEY_LABEL, label.as_bytes()),
        );
    }
    let cost = Cost {
        memory_kib: 64 * 1024,
        passes: 3,
        lanes: 1,
    };
    let file = faraday_vault::create(
        &c,
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[testkit::VAULT_PASSPHRASES[0].as_bytes()],
        &[contents],
        &[0x42; 32],
    )
    .unwrap();
    let mut app = device(vec![("mixed.ofv".to_string(), file)]);
    app.press(Action::Vault(V::OpenFrom(0, Screen::Files)));
    type_and_unlock(&mut app);
    create_on_keys(&mut app, NewKind::Threshold);
    let share = key_record(&app, "Twenty-four words");
    let not = key_record(&app, "Twelve words");
    let _ = app.frame();
    assert!(
        app.offers(Action::CSlotVault(0, 0, share)),
        "a share is offered"
    );
    assert!(
        !app.offers(Action::CSlotVault(0, 0, not)),
        "twelve words are no share"
    );
}

#[test]
fn a_locked_vault_reads_unlock_and_unlocking_returns_to_keys_with_its_keys() {
    use faraday_core::create::NewKind;
    use faraday_core::cstep;
    let mut app = device(vec![kit_file("vault.ofv")]);
    add_key(&mut app, 1);
    add_key(&mut app, 2);
    create_on_keys(&mut app, NewKind::Multi);
    let unlock = Action::Vault(V::OpenFrom(0, Screen::Create));
    let texts = app.drawn_texts();
    assert!(app.offers(unlock), "the locked vault is offered");
    assert!(
        texts.iter().any(|t| t == "Unlock vault.ofv"),
        "as Unlock and its name: {texts:?}"
    );
    app.press(unlock);
    assert_eq!(app.screen, Screen::Unlock);
    type_and_unlock(&mut app);
    assert_eq!(app.screen, Screen::Create, "unlocking comes back");
    assert_eq!(
        app.create.as_ref().and_then(|c| c.open),
        Some(cstep::KEYS),
        "to the Keys card"
    );
    assert_eq!(app.session.keys.len(), 2, "nothing is loaded");
    let _ = app.frame();
    let r = key_record(&app, testkit::TEST_SEEDS[0].1);
    assert!(
        app.offers(Action::CSlotVault(0, 0, r)),
        "the vault's keys are offered now"
    );
    assert!(!app.offers(unlock), "and it is not offered for unlocking");
}
