//! Make a new wallet opens on Start (`docs/BACKUP-FLOW.md` §20.14 item 1):
//! Plan the backup first ticked, I know what I want, The standard way. I
//! know what I want opens Kind as Create always has (`docs/NEW-WALLET.md`
//! §2.1, reversing `SIMPLIFY.md` §2.1 for this flow): Kind defaults to
//! single key, Multisig is the other everyday choice, and the other eight
//! kinds are behind More kinds. It ends in the backup plan (`SIMPLIFY.md`
//! §2.3): Keys' Continue makes the wallet, Check follows, and Back up
//! offers the plan's presets. Plan the backup first says the layers on the
//! Access layers card, suggests the wallet's shape and leaves every key
//! to the person.

use faraday_core::create::NewKind;
use faraday_core::create_plan::{CreateWay, WalletShape};
use faraday_core::plan::{Answers, Preset};
use faraday_core::{Action, Faraday, Screen, cstep, testkit};
use osk_shell_api::{App, Event, Key};

fn frost_index() -> u8 {
    NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Threshold)
        .expect("FROST is one of the ten kinds") as u8
}

fn multi_index() -> u8 {
    NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .expect("Multisig is one of the ten kinds") as u8
}

/// Loads a test seed as a key in the session, through Add a key.
fn add_key(app: &mut Faraday, seed: &str) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(seed).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

fn opened() -> Faraday {
    let mut app = testkit::started();
    faraday_core::testkit::create_knowing(&mut app);
    assert_eq!(app.screen, Screen::Create);
    app
}

#[test]
fn a_fresh_create_opens_on_start_with_plan_the_backup_first_ticked() {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    let c = app.create.as_ref().expect("create");
    assert_eq!(c.open, Some(cstep::START), "Create opens on Start");
    assert_eq!(c.way, CreateWay::Plan, "Plan the backup first is ticked");
    let texts = app.drawn_texts();
    for way in [
        "Plan the backup first",
        "I know what I want",
        "The standard way",
    ] {
        assert!(
            texts.iter().any(|t| t == way),
            "{way} is offered: {texts:?}"
        );
    }
}

#[test]
fn i_know_what_i_want_opens_kind_with_the_single_key_kind_and_no_layers() {
    let app = opened();
    let c = app.create.as_ref().expect("create");
    assert_eq!(c.kind, NewKind::NativeSegwit);
    assert_eq!(c.open, Some(cstep::KIND), "I know what I want opens Kind");
    assert!(!c.done[cstep::KIND as usize], "not answered yet");
    let cards = app.create_cards();
    assert!(
        !cards.contains(&cstep::LAYERS) && !cards.contains(&cstep::WALLET),
        "nothing about layers is asked: {cards:?}"
    );
}

#[test]
fn continue_closes_kind_and_change_reopens_it_keeping_the_choice() {
    let mut app = opened();
    app.press(Action::CKind(1)); // Taproot, ticked while Kind is open
    app.press(Action::CNext(cstep::KIND)); // Kind's own Continue
    assert_eq!(app.create.as_ref().map(|c| c.kind), Some(NewKind::Taproot));
    assert_eq!(
        app.create.as_ref().and_then(|c| c.open),
        Some(cstep::WHERE),
        "Continue closes Kind as done and opens Where"
    );
    app.press(Action::CStep(cstep::KIND));
    assert_eq!(
        app.create.as_ref().and_then(|c| c.open),
        Some(cstep::KIND),
        "Change opens Kind"
    );
    assert_eq!(
        app.create.as_ref().map(|c| c.kind),
        Some(NewKind::Taproot),
        "the kind chosen earlier is kept"
    );
}

#[test]
fn more_kinds_offers_frost() {
    let mut app = opened();
    // Kind is already open: a fresh Create opens on it.
    let _ = app.frame();
    assert!(
        !app.offers(Action::CKind(frost_index())),
        "FROST is behind More kinds"
    );
    app.press(Action::CMoreKinds);
    let _ = app.frame();
    assert!(
        app.offers(Action::CKind(frost_index())),
        "More kinds offers FROST"
    );
}

#[test]
fn a_tools_tile_for_musig2_opens_create_on_kind_with_musig2_ticked() {
    let mut app = testkit::started();
    let i = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "MuSig2")
        .expect("a MuSig2 tile");
    app.press(Action::Catalog(i as u8));
    assert_eq!(app.screen, Screen::Create);
    let c = app.create.as_ref().expect("create");
    assert_eq!(c.kind, NewKind::MuSig);
    assert_eq!(
        c.open,
        Some(cstep::KIND),
        "Create opens on Kind, with MuSig2 already ticked"
    );
    assert!(!c.done[cstep::KIND as usize]);
}

#[test]
fn a_tools_tile_for_multisig_ticks_its_quorum_default_too() {
    let mut app = testkit::started();
    let i = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Multisig")
        .expect("a Multisig tile");
    app.press(Action::Catalog(i as u8));
    let c = app.create.as_ref().expect("create");
    assert_eq!(c.kind, NewKind::Multi);
    assert_eq!(c.m, 2, "2 of 3 default");
    assert_eq!(c.n, 3);
    assert!(!c.done[cstep::QUORUM as usize]);
    assert_eq!(c.open, Some(cstep::KIND), "the flow opens on Kind");
}

/// A single-key wallet made from Create over a key typed in, stopped on
/// Check; returns the wallet's index.
fn made() -> (Faraday, usize) {
    made_on(testkit::started())
}

/// [`made`] on the test network.
fn made_on_testnet() -> (Faraday, usize) {
    let mut app = testkit::started();
    app.press(Action::Network(osk_bip::keys::Network::Testnet));
    assert_eq!(app.session.network(), osk_bip::keys::Network::Testnet);
    made_on(app)
}

fn made_on(mut app: Faraday) -> (Faraday, usize) {
    app.press(Action::Entry(None));
    for c in testkit::test_words("bacon").chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    faraday_core::testkit::create_knowing(&mut app);
    app.press(Action::CNext(cstep::KIND)); // single key, the default: opens Keys
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    let fp = app.session.keys[0].fingerprint().0;
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    let c = app.create.as_ref().expect("create");
    let built = c.built.expect("Keys' Continue makes the wallet");
    assert_eq!(c.open, Some(cstep::CHECK), "Check follows Keys");
    (app, built)
}

#[test]
fn after_check_the_next_card_is_back_up_with_three_presets() {
    let (mut app, _) = made();
    app.press(Action::CNext(cstep::CHECK));
    assert_eq!(
        app.create.as_ref().and_then(|c| c.open),
        Some(cstep::BACKUP),
        "Back up follows Check"
    );
    let _ = app.frame();
    for k in 0..3u8 {
        assert!(app.offers(Action::CBackup(k)), "preset {k} is offered");
    }
    assert!(!app.offers(Action::CBackup(3)), "three presets, no more");
}

#[test]
fn paper_and_vault_opens_the_backup_on_the_new_wallet_with_that_preset() {
    let (mut app, built) = made();
    app.press(Action::CNext(cstep::CHECK));
    let k = Preset::ALL
        .iter()
        .position(|p| *p == Preset::PaperVault)
        .expect("Paper and vault") as u8;
    app.press(Action::CBackup(k));
    assert_eq!(app.screen, Screen::Backup);
    let b = app.backup.as_ref().expect("backup");
    assert_eq!(b.wallet, built, "the backup is of the wallet just made");
    assert_eq!(
        b.answers,
        Answers::preset(&app.plan_shape(built), Preset::PaperVault),
        "the preset is applied"
    );
}

#[test]
fn back_from_the_backup_returns_to_the_wallet_card() {
    let (mut app, built) = made();
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup(0));
    let _ = app.frame();
    assert!(
        app.offers(Action::OpenWallet(built)),
        "the way back is offered"
    );
    app.press(Action::OpenWallet(built));
    assert_eq!(app.screen, Screen::Wallets);
    assert_eq!(app.wallet, built, "on the wallet just made");
}

#[test]
fn checks_address_rows_are_named_the_wallets_first_addresses() {
    let (mut app, _) = made();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "The wallet's first addresses"),
        "Check names whose addresses the rows are: {texts:?}"
    );
}

/// A 2-of-3 multisig, Create opened on Kind and advanced to Keys.
fn multisig_on_keys() -> Faraday {
    let mut app = testkit::started();
    // Start stands above Kind now: the foot button needs the height.
    tall(&mut app);
    add_key(&mut app, "bacon");
    add_key(&mut app, "zebra");
    faraday_core::testkit::create_knowing(&mut app);
    app.press(Action::CKind(multi_index()));
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CNext(cstep::QUORUM)); // 2 of 3, the default
    app
}

#[test]
fn a_key_in_one_slot_is_offered_in_no_other_and_clearing_offers_it_again() {
    let mut app = multisig_on_keys();
    let fp = app.session.keys[0].fingerprint().0;
    app.press(Action::CSlotHere(0, fp));
    let _ = app.frame();
    assert!(
        !app.offers(Action::CSlotHere(1, fp)),
        "Key 1's key is not offered for Key 2"
    );
    app.press(Action::CSlotClear(0));
    let _ = app.frame();
    assert!(
        app.offers(Action::CSlotHere(1, fp)),
        "Clear on Key 1 offers it again"
    );
}

#[test]
fn the_keys_foot_button_opens_new_key_for_the_first_empty_slot() {
    let mut app = multisig_on_keys();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "New key for Key 1"),
        "the foot button names the first empty slot: {texts:?}"
    );
    let fp = app.session.keys[0].fingerprint().0;
    app.press(Action::CSlotHere(0, fp));
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "New key for Key 2"),
        "the foot button moves to the next empty slot: {texts:?}"
    );
}

/// New key on its own, from this device's generator: locked in with no
/// passphrase, the quiz skipped, and Done. Returns the key's fingerprint.
fn key_made_on_its_own(app: &mut Faraday) -> [u8; 4] {
    use faraday_core::keygen::Way;
    app.press(Action::KeyGen(None));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Device.index()));
    app.press(Action::KNext);
    app.event(Event::Entropy(osk_shell_api::EntropyBytes::new([0x6c; 32])));
    app.press(Action::KNext);
    app.press(Action::KLock);
    app.press(Action::KNext);
    app.press(Action::KNext);
    app.press(Action::KSkip);
    app.press(Action::KSkip);
    app.press(Action::KAdd);
    assert!(app.keygen.is_none(), "New key is done");
    app.session.keys[0].fingerprint().0
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

#[test]
fn a_key_made_on_its_own_takes_a_passphrase_in_a_single_key_slot() {
    let mut app = testkit::started();
    let first = key_made_on_its_own(&mut app);
    let words = app.session.keys[0].words.clone().expect("words");
    faraday_core::testkit::create_knowing(&mut app);
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CSlotHere(0, first));
    let _ = app.frame();
    assert!(app.offers(Action::CPassOpen(0)));
    app.press(Action::CPassOpen(0));
    type_text(&mut app, "Ride the 7 bus");
    app.press(Action::CPassField(1));
    type_text(&mut app, "Ride the 7 bus");
    app.press(Action::CPassLock);
    let with = faraday_core::wallet::Session::default()
        .add_words_with(&words, "Ride the 7 bus", "", None)
        .unwrap()
        .0;
    assert_ne!(with, first);
    let c = app.create.as_ref().unwrap();
    assert_eq!(c.slots[0], faraday_core::create::Source::Here(with));
    assert!(
        app.session.keys.iter().any(|k| k.fingerprint().0 == first),
        "the key without the passphrase stays loaded"
    );
    assert!(app.drawn_texts().iter().any(|t| t == "Locked in"));
    app.press(Action::CNext(cstep::KEYS));
    let w = app
        .create
        .as_ref()
        .unwrap()
        .built
        .expect("the wallet is made");
    let fps: Vec<[u8; 4]> = app
        .session
        .slots(&app.session.wallets[w])
        .iter()
        .filter_map(|s| s.fingerprint.map(|f| f.0))
        .collect();
    assert_eq!(fps, vec![with], "the wallet is the passphrase key's");
}

#[test]
fn a_slot_whose_key_has_a_passphrase_offers_no_add_a_passphrase() {
    let mut app = testkit::started();
    add_key(&mut app, "bacon");
    let plain = app.session.keys[0].fingerprint().0;
    let words = testkit::test_words("bacon");
    let with = app
        .session
        .add_words_with(&words, "Ride the 7 bus", "with", None)
        .unwrap()
        .0;
    faraday_core::testkit::create_knowing(&mut app);
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CSlotHere(0, with));
    let _ = app.frame();
    assert!(!app.offers(Action::CPassOpen(0)));
    app.press(Action::CSlotHere(0, plain));
    let _ = app.frame();
    assert!(app.offers(Action::CPassOpen(0)));
}

#[test]
fn unequal_passphrases_in_a_slot_lock_nothing_in() {
    let mut app = testkit::started();
    add_key(&mut app, "bacon");
    let plain = app.session.keys[0].fingerprint().0;
    faraday_core::testkit::create_knowing(&mut app);
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CSlotHere(0, plain));
    app.press(Action::CPassOpen(0));
    type_text(&mut app, "one");
    app.press(Action::CPassField(1));
    type_text(&mut app, "two");
    app.press(Action::CPassLock);
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(
        app.create.as_ref().unwrap().slots[0],
        faraday_core::create::Source::Here(plain)
    );
    assert!(
        app.drawn_texts()
            .iter()
            .any(|t| t == "The two passphrases differ")
    );
}

// ---- New wallet from this one (NEW-WALLET §16.2) ----

/// Presses the open card's Continue, answering No to vaults, until the
/// wallet is made; returns it.
fn continue_throughout(app: &mut Faraday) -> usize {
    for _ in 0..12 {
        let c = app.create.as_ref().expect("create");
        if let Some(b) = c.built {
            return b;
        }
        let k = c.open.expect("a card is open");
        if k == cstep::WHERE {
            app.press(Action::CVaults(false));
        }
        app.press(Action::CNext(k));
    }
    panic!(
        "the wallet was not made: {:?}",
        app.create.as_ref().and_then(|c| c.error.clone())
    );
}

/// The account step of a wallet key's path: its third.
fn account_of_key(k: &osk_bip::policy::PolicyKey) -> u32 {
    use osk_bip::bitcoin::bip32::ChildNumber;
    match k.path().and_then(|p| p.into_iter().nth(2).copied()) {
        Some(ChildNumber::Hardened { index }) => index,
        other => panic!("no hardened account step: {other:?}"),
    }
}

/// The master fingerprint of a test seed, worked out here.
fn fp_of(word: &str) -> [u8; 4] {
    use osk_bip::bip39::{Language, Mnemonic};
    let m = Mnemonic::parse(Language::English, &testkit::test_words(word)).unwrap();
    let seed = m.to_seed(b"").unwrap();
    osk_bip::keys::MasterKey::from_seed(&seed, testkit::NET)
        .fingerprint()
        .0
}

/// The P2WPKH address a test seed gives at `path`, derived here with
/// osk-bip and rust-bitcoin, not through Faraday's wallet.
fn p2wpkh_at(word: &str, path: &str) -> String {
    use osk_bip::bip39::{Language, Mnemonic};
    use osk_bip::bitcoin::bip32::DerivationPath;
    use osk_bip::bitcoin::{Address, CompressedPublicKey, KnownHrp};
    let m = Mnemonic::parse(Language::English, &testkit::test_words(word)).unwrap();
    let seed = m.to_seed(b"").unwrap();
    let master = osk_bip::keys::MasterKey::from_seed(&seed, testkit::NET);
    let path: DerivationPath = path.parse().unwrap();
    let public = master.derive(&path).to_xpub().public_key;
    Address::p2wpkh(&CompressedPublicKey(public), KnownHrp::Testnets).to_string()
}

#[test]
fn new_wallet_from_a_single_key_wallet_is_its_words_at_account_1() {
    let (mut app, original) = made_on_testnet();
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::OpenWallet(original));
    let _ = app.frame();
    assert!(
        app.offers(Action::NewWalletFrom(original)),
        "the wallet's card offers New wallet from this one"
    );
    let name = app.session.wallets[original].name.clone();
    let sum = app.session.wallets[original].checksum().to_string();
    let first = app
        .session
        .address(&app.session.wallets[original], false, 0);
    app.press(Action::NewWalletFrom(original));
    assert_eq!(app.screen, Screen::Create);
    let c = app.create.as_ref().unwrap();
    assert_eq!(c.kind, NewKind::NativeSegwit, "the wallet's kind");
    assert_eq!(c.account, 1, "the next account its keys do not use");
    assert!(
        c.done[cstep::KIND as usize],
        "Kind is closed with its value"
    );
    let made = continue_throughout(&mut app);
    assert_ne!(made, original);
    let w = &app.session.wallets[made];
    let address = app.session.address(w, false, 0);
    assert_eq!(
        address,
        p2wpkh_at("bacon", "m/84'/1'/1'/0/0"),
        "the same words at account 1"
    );
    assert_ne!(address, first, "a new, separate wallet");
    assert_eq!(w.name, format!("{name} · account 1"));
    // The original is not touched.
    let o = &app.session.wallets[original];
    assert_eq!(o.name, name);
    assert_eq!(o.checksum(), sum);
    assert_eq!(app.session.address(o, false, 0), first);
}

#[test]
fn new_wallet_from_a_two_of_three_held_here_takes_every_key_at_account_1() {
    let mut app = testkit::started();
    for seed in ["bacon", "zebra", "summer"] {
        add_key(&mut app, seed);
    }
    faraday_core::testkit::create_knowing(&mut app);
    app.press(Action::CKind(multi_index()));
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CNext(cstep::QUORUM));
    for k in 0..3u8 {
        let fp = app.session.keys[usize::from(k)].fingerprint().0;
        app.press(Action::CSlotHere(k, fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    let original = app.create.as_ref().and_then(|c| c.built).expect("made");
    let sum = app.session.wallets[original].checksum().to_string();
    app.press(Action::NewWalletFrom(original));
    let made = continue_throughout(&mut app);
    let w = &app.session.wallets[made];
    assert_eq!(NewKind::of(w.policy()), Some(NewKind::Multi));
    let keys = w.policy().keys();
    assert_eq!(keys.len(), 3);
    for k in keys {
        assert_eq!(account_of_key(k), 1, "{} is at account 1", k.key_text());
    }
    let mut fps: Vec<[u8; 4]> = keys.iter().map(|k| k.fingerprint().unwrap().0).collect();
    fps.sort();
    let mut want: Vec<[u8; 4]> = ["bacon", "zebra", "summer"].map(fp_of).to_vec();
    want.sort();
    assert_eq!(fps, want, "the same three keys");
    assert_eq!(app.session.wallets[original].checksum(), sum);
}

#[test]
fn a_cosigners_xpub_at_another_account_leaves_its_slot_to_someone_else() {
    use faraday_core::create::Source;
    let mut app = testkit::started();
    add_key(&mut app, "bacon");
    add_key(&mut app, "zebra");
    let keys: Vec<String> = (0..3).map(|n| testkit::key(n, "m/48'/1'/0'/2'")).collect();
    let i = app
        .session
        .add_wallet(
            "Shared",
            &NewKind::Multi.descriptor(2, &keys),
            "Test",
            faraday_core::wallet::Origin::Named,
        )
        .unwrap();
    let summer = fp_of("summer");
    let pos = app.session.wallets[i]
        .policy()
        .keys()
        .iter()
        .position(|k| k.fingerprint().map(|f| f.0) == Some(summer))
        .unwrap();
    let xpub = app.session.wallets[i].policy().keys()[pos].key_text();
    app.press(Action::NewWalletFrom(i));
    let c = app.create.as_ref().unwrap();
    assert_eq!(c.account, 1);
    assert_eq!(c.slots[pos], Source::Later, "Someone else");
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    assert_eq!(app.create.as_ref().and_then(|c| c.open), Some(cstep::KEYS));
    let line = format!("Needs Key {}'s xpub at account 1", pos + 1);
    let texts = app.drawn_texts();
    assert!(texts.contains(&line), "{line}: {texts:?}");
    app.press(Action::CNext(cstep::KEYS));
    assert!(
        app.create.as_ref().unwrap().built.is_none(),
        "no wallet until the cosigner's xpub is in"
    );
    // At the wallet's own account the xpub it has is that key again.
    app.press(Action::CAccount(-1));
    assert_eq!(
        app.create.as_ref().unwrap().slots[pos],
        Source::Cosigner(xpub)
    );
}

#[test]
fn changing_kind_to_taproot_keeps_the_keys_and_makes_a_taproot_wallet() {
    let (mut app, original) = made();
    app.press(Action::NewWalletFrom(original));
    app.press(Action::CStep(cstep::KIND));
    let taproot = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Taproot)
        .unwrap() as u8;
    app.press(Action::CKind(taproot));
    app.press(Action::CNext(cstep::KIND));
    let made = continue_throughout(&mut app);
    let w = &app.session.wallets[made];
    assert_eq!(NewKind::of(w.policy()), Some(NewKind::Taproot));
    let keys = w.policy().keys();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].fingerprint().map(|f| f.0), Some(fp_of("bacon")));
    assert_eq!(account_of_key(&keys[0]), 1);
    assert_eq!(w.name, format!("{} · account 1", NewKind::Taproot.name()));
}

// ---- Plan the backup first (`docs/BACKUP-FLOW.md` §20.14) ----

/// A display tall enough for a card and its foot button together.
fn tall(app: &mut Faraday) {
    use osk_shell_api::{BootState, DisplayInfo, SecureHardware};
    app.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 2400,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

/// Create opened on Start, with the test seeds `words` loaded first by
/// Add a key, and Continue pressed with Plan the backup first ticked: the
/// Access layers card, with the fresh plan.
fn planning_with(words: &[&str]) -> Faraday {
    let mut app = testkit::started();
    tall(&mut app);
    for w in words {
        add_key(&mut app, w);
    }
    app.press(Action::CreateWallet);
    app.press(Action::CNext(cstep::START));
    app
}

fn planning() -> Faraday {
    planning_with(&[])
}

#[test]
fn plan_the_backup_first_opens_the_access_layers_card_with_the_fresh_plan() {
    let mut app = planning();
    let c = app.create.as_ref().expect("create");
    assert_eq!(c.open, Some(cstep::LAYERS));
    let a = &c.plan.as_ref().expect("a plan").answers;
    // §20.14 item 2: three places, one layer, its paper route any 2 of
    // them, one seed in words and a part of the description at each.
    assert_eq!(a.places, 3);
    assert_eq!(a.layers.len(), 1);
    let r = &a.layers[0].routes[0];
    assert!(r.paper);
    assert_eq!((r.places.clone(), r.any), (vec![0, 1, 2], 2));
    let k = r.kinds.expect("kinds set");
    assert_eq!(k.seeds, 1);
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Suggest a wallet"),
        "the card's foot button: {texts:?}"
    );
}

#[test]
fn kind_and_keys_read_after_wallet_and_stay_closed_until_a_shape_is_chosen() {
    let mut app = planning();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().filter(|t| *t == "After Wallet").count() >= 2,
        "Kind and Keys wait for the Wallet card: {texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t == "Single key · native SegWit"),
        "no kind shows as chosen: {texts:?}"
    );
    app.press(Action::CStep(cstep::KIND));
    assert_eq!(
        app.create.as_ref().and_then(|c| c.open),
        Some(cstep::LAYERS),
        "Kind does not open"
    );
    app.press(Action::CSuggest);
    app.press(Action::CShape(WalletShape::TwoOfThree.index()));
    let texts = app.drawn_texts();
    assert!(!texts.iter().any(|t| t == "After Wallet"), "{texts:?}");
    assert!(
        texts.iter().any(|t| t == "Multisig · native SegWit"),
        "{texts:?}"
    );
}

#[test]
fn the_fresh_plan_suggests_a_two_of_three_first_and_dims_single_key_with_its_conflict() {
    let mut app = planning();
    app.press(Action::CSuggest);
    let c = app.create.as_ref().expect("create");
    assert_eq!(c.open, Some(cstep::WALLET), "the Wallet card opens");
    let fits = &c.plan.as_ref().expect("plan").fits;
    // Any 2 of 3 places, one seed and one part each. A 2 of 3: two places
    // give two seeds and two of the three parts (each leaving one key
    // off), so the route is met; one place gives one seed, short of 2,
    // so a stranger needs 2 places. Single key: one place's seed alone
    // spends while the route asks for two places, a conflict. Single key
    // with a passphrase: nothing remembers it, no route can sign. 3 of 5:
    // 2 places × 1 seed is fewer than 3.
    assert_eq!(fits[0].shape, WalletShape::TwoOfThree);
    assert!(fits[0].fits);
    assert_eq!(
        fits[0].line,
        "2 of 3: a stranger needs 2 places; 3 keys to make"
    );
    assert_eq!(fits.iter().filter(|f| f.fits).count(), 1, "{fits:?}");
    let single = fits
        .iter()
        .find(|f| f.shape == WalletShape::Single)
        .expect("single key tried");
    assert!(!single.fits);
    assert_eq!(
        single.line,
        "Single key: Access layer 1, route 1: a stranger who finds one of its places can spend"
    );
    // Nothing remembers a passphrase on the fresh plan: that is the cause.
    let pass = fits
        .iter()
        .find(|f| f.shape == WalletShape::SinglePassphrase)
        .expect("single key with a passphrase tried");
    assert!(!pass.fits);
    assert_eq!(
        pass.line,
        "Single key with a passphrase: no route remembers the passphrase"
    );
    let _ = app.frame();
    assert!(app.offers(Action::CShape(WalletShape::TwoOfThree.index())));
    assert!(
        !app.offers(Action::CShape(WalletShape::Single.index())),
        "a shape that does not fit is dimmed"
    );
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t.starts_with("Single key: Access layer 1")),
        "{texts:?}"
    );
}

#[test]
fn choosing_two_of_three_fills_kind_and_quorum_and_opens_keys_for_three_keys_none_made() {
    let mut app = planning_with(&["bacon"]);
    let keys = app.session.keys.len();
    app.press(Action::CSuggest);
    app.press(Action::CShape(WalletShape::TwoOfThree.index()));
    let c = app.create.as_ref().expect("create");
    assert_eq!((c.kind, c.m, c.n), (NewKind::Multi, 2, 3));
    assert!(c.done[cstep::KIND as usize] && c.done[cstep::QUORUM as usize]);
    // The layers keep nothing in a vault: no vault is set up before the
    // keys, and the next card is Keys.
    assert_eq!(c.open, Some(cstep::KEYS));
    assert_eq!(c.slots, vec![faraday_core::create::Source::Empty; 3]);
    assert_eq!(app.session.keys.len(), keys, "Faraday made no key");
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "New key for Key 1"), "{texts:?}");
    // Change still opens Kind and Quorum.
    app.press(Action::CStep(cstep::QUORUM));
    assert_eq!(
        app.create.as_ref().and_then(|c| c.open),
        Some(cstep::QUORUM)
    );
}

#[test]
fn a_quorum_the_layers_cannot_give_is_a_warning_and_keys_still_open() {
    let mut app = planning();
    app.press(Action::CSuggest);
    app.press(Action::CShape(WalletShape::TwoOfThree.index()));
    app.press(Action::CStep(cstep::QUORUM));
    // 3 of 3: any 2 places of 1 seed each give 2 seeds, short of 3.
    app.press(Action::CM(1));
    let c = app.create.as_ref().expect("create");
    assert_eq!(c.m, 3);
    let warning = c
        .plan
        .as_ref()
        .and_then(|p| p.warning.clone())
        .expect("a warning");
    assert!(warning.starts_with("3 of 3: "), "{warning}");
    let texts = app.drawn_texts();
    assert!(texts.contains(&warning), "{texts:?}");
    app.press(Action::CNext(cstep::QUORUM));
    assert_eq!(
        app.create.as_ref().and_then(|c| c.open),
        Some(cstep::KEYS),
        "a warning, never a refusal"
    );
}

#[test]
fn after_the_keys_are_made_back_up_opens_with_the_board_placed_for_the_layers() {
    let mut app = planning_with(&["bacon", "zebra", "summer"]);
    app.press(Action::CSuggest);
    app.press(Action::CShape(WalletShape::TwoOfThree.index()));
    // The person fills each slot with a key loaded here.
    let fps: Vec<[u8; 4]> = app.session.keys.iter().map(|k| k.fingerprint().0).collect();
    for (slot, fp) in fps.iter().enumerate() {
        app.press(Action::CSlotHere(slot as u8, *fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    let built = app.create.as_ref().and_then(|c| c.built).expect("made");
    app.press(Action::CNext(cstep::CHECK));
    let _ = app.frame();
    assert!(
        app.offers(Action::CBackupPlanned),
        "Create's Back up: Work it out"
    );
    app.press(Action::CBackupPlanned);
    assert_eq!(app.screen, Screen::Backup);
    let b = app.backup.as_ref().expect("backup");
    assert_eq!(b.wallet, built);
    assert_eq!(
        b.q,
        Some(faraday_core::qstep::BOARD),
        "the board opens filled"
    );
    let shape = app.plan_shape(built);
    let layers = b.answers.access_layers(&shape);
    assert_eq!(layers.len(), 1, "the layers set in Create");
    assert!(faraday_core::access::layer_met(
        &shape, &b.answers, &layers[0]
    ));
    // One seed and one part at each of the three places: any two spend,
    // none alone does.
    assert_eq!(faraday_core::access::stranger_margin(&shape, &b.answers), 1);
    assert!(app.board_shown());
}

#[test]
fn one_place_one_seed_and_a_passphrase_remembered_suggests_single_key_with_a_passphrase() {
    use faraday_core::layers::LayersPress as P;
    let mut app = planning();
    // The route holds Place 1 alone and remembers a passphrase.
    app.press(Action::Layers(P::Place(0, 0, 1)));
    app.press(Action::Layers(P::Place(0, 0, 2)));
    app.press(Action::Layers(P::Passphrase(0, 0, 0)));
    let r = app
        .create
        .as_ref()
        .unwrap()
        .plan
        .as_ref()
        .unwrap()
        .answers
        .layers[0]
        .routes[0]
        .clone();
    assert_eq!((r.places.clone(), r.any), (vec![0], 1));
    app.press(Action::CSuggest);
    let fits = &app.create.as_ref().unwrap().plan.as_ref().unwrap().fits;
    // With the passphrase the stranger at Place 1 has the seed and not
    // the passphrase: no place spends. Without one Place 1 alone spends,
    // a conflict; 2 of 3 and 3 of 5 need more seeds than one place holds.
    assert_eq!(fits[0].shape, WalletShape::SinglePassphrase);
    assert!(fits[0].fits);
    assert_eq!(fits.iter().filter(|f| f.fits).count(), 1, "{fits:?}");
    assert!(
        fits[0].line.starts_with(
            "Single key with a passphrase: a stranger who finds every place cannot spend"
        ),
        "{}",
        fits[0].line
    );
}

#[test]
fn no_shape_that_fits_leaves_the_conflicts_on_the_access_layers_card() {
    use faraday_core::layers::LayersPress as P;
    let mut app = planning();
    // The paper route off paper: no layer can spend from paper (§20.12
    // item 3), whatever the wallet.
    app.press(Action::Layers(P::Paper(0, 0)));
    app.press(Action::CSuggest);
    let c = app.create.as_ref().unwrap();
    assert_eq!(c.open, Some(cstep::LAYERS), "the card stays");
    assert!(c.plan.as_ref().unwrap().fits.iter().all(|f| !f.fits));
    let texts = app.drawn_texts();
    assert!(
        texts
            .iter()
            .any(|t| t == "Access layer 1 has no paper route"),
        "{texts:?}"
    );
    let _ = app.frame();
    assert!(app.offers(Action::Layers(P::AddPaperRoute(0))), "its fix");
}

#[test]
fn the_standard_way_shows_the_checklist_and_no_board() {
    let mut app = testkit::started();
    add_key(&mut app, "bacon");
    app.press(Action::CreateWallet);
    app.press(Action::CWay(CreateWay::Standard.index()));
    app.press(Action::CNext(cstep::START));
    assert_eq!(app.create.as_ref().and_then(|c| c.open), Some(cstep::KIND));
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    let fp = app.session.keys[0].fingerprint().0;
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    app.press(Action::CNext(cstep::CHECK));
    let _ = app.frame();
    assert!(app.offers(Action::CBackupStandard));
    app.press(Action::CBackupStandard);
    assert_eq!(app.screen, Screen::Backup);
    let b = app.backup.as_ref().expect("backup");
    assert!(b.standard);
    assert_eq!(b.stage, faraday_core::BStage::Checklist);
    use faraday_core::{bstep, plan::Item};
    assert_eq!(
        app.backup_items(),
        vec![
            bstep::of(Item::Copy(0)),
            bstep::of(Item::ShowDescriptor),
            bstep::of(Item::Sheets)
        ],
        "the seed's words and SeedQR, the description shown and for the stick"
    );
    assert!(!app.board_shown(), "no board");
    let texts = app.drawn_texts();
    assert!(
        !texts.iter().any(|t| t == "Access layers"),
        "no layers: {texts:?}"
    );
}

#[test]
fn back_ups_start_from_offers_the_standard_way() {
    let (mut app, built) = made();
    app.press(Action::Backup(built));
    let _ = app.frame();
    assert!(app.offers(Action::BStandard));
    app.press(Action::BStandard);
    assert!(app.backup.as_ref().is_some_and(|b| b.standard));
}
