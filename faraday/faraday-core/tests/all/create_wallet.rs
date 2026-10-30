//! Make a new wallet opens with Kind closed on its default, single key,
//! and the card after it open; a kind pressed moves on by itself, the
//! ticked one too. Multisig and Single key, backup in shares are the
//! other everyday choices, and the other eight kinds are behind More
//! kinds. It ends in the backup plan (`docs/history/SIMPLIFY.md` §2.3):
//! Keys' Make the wallet makes it, Check's It matches follows, and Back
//! up offers the plan's presets.

use crate::common::{add_key, type_text};
use faraday_core::create::NewKind;
use faraday_core::plan::{Answers, Preset};
use faraday_core::{Action, Faraday, Screen, cstep, testkit};
use faraday_shell_api::{App, Event, Key};

fn multi_index() -> u8 {
    NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .expect("Multisig is one of the ten kinds") as u8
}

fn opened() -> Faraday {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::CKind(0));
    assert_eq!(app.screen, Screen::Create);
    app
}

#[test]
fn a_fresh_create_opens_with_kind_closed_on_single_key() {
    let app = opened();
    let c = app.create().expect("create");
    assert_eq!(c.kind, NewKind::NativeSegwit);
    assert_eq!(c.open, Some(cstep::WHERE), "the card after Kind is open");
    let mut app = app;
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Single key · native SegWit"),
        "Kind closed on its default, shown: {texts:?}"
    );
    app.press(Action::CVaults(false));
    assert!(
        app.create().unwrap().done[cstep::KIND as usize],
        "the default is taken once a later card is answered"
    );
}

#[test]
fn a_kind_pressed_moves_on_and_its_head_reopens_it_keeping_the_choice() {
    let mut app = opened();
    app.press(Action::CStep(cstep::KIND));
    let _ = app.frame();
    assert!(
        !app.drawn_texts().iter().any(|t| t == "Continue"),
        "Kind has no Continue"
    );
    app.press(Action::CMoreKinds);
    app.press(Action::CKind(kind_index(NewKind::Taproot)));
    assert_eq!(app.create().map(|c| c.kind), Some(NewKind::Taproot));
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(cstep::WHERE),
        "the kind pressed closes Kind and opens Use vaults"
    );
    app.press(Action::CStep(cstep::KIND));
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::KIND));
    assert_eq!(
        app.create().map(|c| c.kind),
        Some(NewKind::Taproot),
        "the kind chosen earlier is kept"
    );
    // The kind already ticked, pressed again, moves on too.
    app.press(Action::CKind(kind_index(NewKind::Taproot)));
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::WHERE));
}

#[test]
fn yes_or_no_to_vaults_moves_on_and_so_does_the_answer_given_pressed_again() {
    let mut app = opened();
    app.press(Action::CVaults(true));
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(cstep::HOW_MANY),
        "Yes opens How many"
    );
    app.press(Action::CStep(cstep::WHERE));
    app.press(Action::CVaults(false));
    // No opens Keys, which with nothing loaded is New key for the slot.
    assert_eq!(app.screen, Screen::KeyGen, "No opens the keys");
    app.press(Action::CStep(cstep::WHERE));
    app.press(Action::CVaults(false));
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(cstep::KEYS),
        "No pressed again moves on as well"
    );
}

#[test]
fn use_vaults_closed_before_it_is_answered_says_so() {
    let mut app = opened();
    app.press(Action::CStep(cstep::WHERE));
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Not answered"),
        "the closed card names its state: {texts:?}"
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
    let c = app.create().expect("create");
    assert_eq!(c.kind, NewKind::MuSig);
    assert_eq!(
        c.open,
        Some(cstep::QUORUM),
        "Create opens past Kind, closed on MuSig2"
    );
    assert!(c.done[cstep::KIND as usize]);
}

#[test]
fn a_tools_tile_for_multisig_ticks_its_quorum_default_too() {
    let mut app = testkit::started();
    let i = faraday_core::catalog::TILES
        .iter()
        .position(|t| t.name == "Multisig")
        .expect("a Multisig tile");
    app.press(Action::Catalog(i as u8));
    let c = app.create().expect("create");
    assert_eq!(c.kind, NewKind::Multi);
    assert_eq!(c.m, 2, "2 of 3 default");
    assert_eq!(c.n, 3);
    assert!(!c.done[cstep::QUORUM as usize]);
    assert_eq!(c.open, Some(cstep::QUORUM), "the flow opens on Quorum");
}

/// A single-key wallet made from Create over a key typed in, stopped on
/// Check; returns the wallet's index.
fn made() -> (Faraday, usize) {
    made_on(testkit::started())
}

/// [`made`] on the test network.
fn made_on_testnet() -> (Faraday, usize) {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, faraday_bip::keys::Network::Testnet).unwrap();
    assert_eq!(app.session.network(), faraday_bip::keys::Network::Testnet);
    made_on(app)
}

fn made_on(mut app: Faraday) -> (Faraday, usize) {
    app.press(Action::Entry(None));
    for c in testkit::test_words("bacon").chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.press(Action::CreateWallet);
    app.press(Action::CKind(0));
    app.press(Action::CNext(cstep::KIND)); // single key, the default: opens Keys
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    let fp = app.session.keys[0].fingerprint.0;
    app.press(Action::CSlotHere(0, fp));
    app.press(Action::CNext(cstep::KEYS));
    let c = app.create().expect("create");
    let built = c.built.expect("Keys' Continue makes the wallet");
    assert_eq!(c.open, Some(cstep::CHECK), "Check follows Keys");
    (app, built)
}

#[test]
fn it_matches_leaves_check_open_on_back_up_wallet_with_no_back_up_card() {
    let (mut app, _) = made();
    let _ = app.frame();
    assert!(!app.offers(Action::CBackup), "It matches comes first");
    app.press(Action::CNext(cstep::CHECK));
    assert_eq!(
        app.create().and_then(|c| c.open),
        Some(cstep::CHECK),
        "Check stays open"
    );
    assert!(!app.create_cards().contains(&cstep::BACKUP));
    let _ = app.frame();
    assert!(app.offers(Action::CBackup), "Back up wallet is offered");
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Back up wallet"));
    assert!(!texts.iter().any(|t| t == "Back up"), "no Back up card");
    for gone in ["Paper only", "Paper and vault", "The standard way"] {
        assert!(!texts.iter().any(|t| t == gone), "{gone} is not offered");
    }
}

#[test]
fn back_up_this_wallet_opens_the_board_read_only_on_the_standard_way() {
    let (mut app, built) = made();
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    assert_eq!(app.screen, Screen::Backup);
    let b = app.backup().expect("backup");
    assert_eq!(b.wallet, built, "the backup is of the wallet just made");
    assert_eq!(b.q, Some(faraday_core::qstep::BOARD), "on the board");
    assert_eq!(
        b.answers,
        Answers::preset(&app.plan_shape_now(built), Preset::Standard),
        "The standard way placed"
    );
    assert!(!b.answers.custom, "read-only until Customize backup");
    let _ = app.frame();
    use faraday_core::board::BoardPress;
    assert!(app.offers(Action::Board(BoardPress::Customize)));
    assert!(app.offers(Action::Board(BoardPress::Software)));
    assert!(
        !app.offers(Action::Board(BoardPress::AddVault)),
        "no list yet"
    );
}

#[test]
fn back_from_the_backup_returns_to_the_wallet_card() {
    let (mut app, built) = made();
    app.press(Action::CNext(cstep::CHECK));
    app.press(Action::CBackup);
    faraday_core::testkit::backup_as(&mut app, faraday_core::plan::Preset::Paper);
    let _ = app.frame();
    assert!(
        app.offers(Action::OpenWallet(built)),
        "the way back is offered"
    );
    app.press(Action::OpenWallet(built));
    assert_eq!(app.screen, Screen::Wallets);
    assert_eq!(app.wallet_index(), built, "on the wallet just made");
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
    add_key(&mut app, "bacon");
    add_key(&mut app, "zebra");
    app.press(Action::CreateWallet);
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
    let fp = app.session.keys[0].fingerprint.0;
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
fn the_keys_foot_shows_make_the_wallet_once_every_slot_is_filled() {
    let mut app = multisig_on_keys();
    let texts = app.drawn_texts();
    assert!(
        !texts.iter().any(|t| t.starts_with("New key for")),
        "no foot button repeats the slot's own New key: {texts:?}"
    );
    assert!(!texts.iter().any(|t| t == "Make the wallet"));
    app.press(Action::CSlotHere(0, app.session.keys[0].fingerprint.0));
    app.press(Action::CSlotHere(1, app.session.keys[1].fingerprint.0));
    app.press(Action::CSlotLater(2));
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Make the wallet"),
        "every slot filled: {texts:?}"
    );
}

/// New key on its own, from this device's generator: locked in with no
/// passphrase, the quiz skipped, and Done. Returns the key's fingerprint.
fn key_made_on_its_own(app: &mut Faraday) -> [u8; 4] {
    use faraday_core::keygen::Way;
    app.press(Action::KeyGen(None));
    app.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Device.index()));
    app.press(Action::KNext);
    app.event(Event::Entropy(faraday_shell_api::EntropyBytes::new(
        [0x6c; 32],
    )));
    app.press(Action::KNext);
    app.press(Action::KLock);
    app.press(Action::KNext);
    app.press(Action::KNext);
    app.press(Action::KSkip);
    app.press(Action::KSkip);
    app.press(Action::KAdd);
    assert!(app.keygen().is_none(), "New key is done");
    app.session.keys[0].fingerprint.0
}

#[test]
fn a_key_made_on_its_own_takes_a_passphrase_in_a_single_key_slot() {
    let c = testkit::compute();
    let mut app = testkit::started();
    let first = key_made_on_its_own(&mut app);
    let words = app
        .session
        .keys
        .reveal(0, faraday_keys::Reveal::Words)
        .to_string();
    app.press(Action::CreateWallet);
    app.press(Action::CKind(0));
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
        .keys
        .add_words_with(&c, &words, "Ride the 7 bus", "", None)
        .unwrap()
        .0;
    assert_ne!(with, first);
    let c = app.create().unwrap();
    assert_eq!(c.slots[0], faraday_core::create::Source::Here(with));
    assert!(
        app.session.keys.all().any(|k| k.fingerprint.0 == first),
        "the key without the passphrase stays loaded"
    );
    assert!(app.drawn_texts().iter().any(|t| t == "Locked in"));
    app.press(Action::CNext(cstep::KEYS));
    let w = app.create().unwrap().built.expect("the wallet is made");
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
    let c = testkit::compute();
    let mut app = testkit::started();
    add_key(&mut app, "bacon");
    let plain = app.session.keys[0].fingerprint.0;
    let words = testkit::test_words("bacon");
    let with = app
        .session
        .keys
        .add_words_with(&c, &words, "Ride the 7 bus", "with", None)
        .unwrap()
        .0;
    app.press(Action::CreateWallet);
    app.press(Action::CKind(0));
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
    let plain = app.session.keys[0].fingerprint.0;
    app.press(Action::CreateWallet);
    app.press(Action::CKind(0));
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
        app.create().unwrap().slots[0],
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
        let c = app.create().expect("create");
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
        app.create().and_then(|c| c.error.clone())
    );
}

/// The account step of a wallet key's path: its third.
fn account_of_key(k: &faraday_bip::policy::PolicyKey) -> u32 {
    use faraday_bip::bitcoin::bip32::ChildNumber;
    match k.path().and_then(|p| p.into_iter().nth(2).copied()) {
        Some(ChildNumber::Hardened { index }) => index,
        other => panic!("no hardened account step: {other:?}"),
    }
}

/// The master fingerprint of a test seed, worked out here.
fn fp_of(word: &str) -> [u8; 4] {
    let c = testkit::compute();
    use faraday_bip::bip39::{Language, Mnemonic};
    let m = Mnemonic::parse(&c, Language::English, &testkit::test_words(word)).unwrap();
    let seed = m.to_seed(&c, b"").unwrap();
    faraday_bip::keys::MasterKey::from_seed(&c, &seed, testkit::NET)
        .fingerprint()
        .0
}

/// The P2WPKH address a test seed gives at `path`, derived here with
/// faraday-bip and rust-bitcoin, not through Faraday's wallet.
fn p2wpkh_at(word: &str, path: &str) -> String {
    let c = testkit::compute();
    use faraday_bip::bip39::{Language, Mnemonic};
    use faraday_bip::bitcoin::bip32::DerivationPath;
    use faraday_bip::bitcoin::{Address, CompressedPublicKey, KnownHrp};
    let m = Mnemonic::parse(&c, Language::English, &testkit::test_words(word)).unwrap();
    let seed = m.to_seed(&c, b"").unwrap();
    let master = faraday_bip::keys::MasterKey::from_seed(&c, &seed, testkit::NET);
    let path: DerivationPath = path.parse().unwrap();
    let public = master.derive(&c, &path).to_xpub(&c).public_key;
    Address::p2wpkh(&CompressedPublicKey(public), KnownHrp::Testnets).to_string()
}

#[test]
fn new_wallet_from_a_single_key_wallet_is_its_words_at_account_1() {
    let c = faraday_core::testkit::compute();
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
        .address(&c, &app.session.wallets[original], false, 0);
    app.press(Action::NewWalletFrom(original));
    assert_eq!(app.screen, Screen::Create);
    let c = app.create().unwrap();
    assert_eq!(c.kind, NewKind::NativeSegwit, "the wallet's kind");
    assert_eq!(c.account, 1, "the next account its keys do not use");
    assert!(
        c.done[cstep::KIND as usize],
        "Kind is closed with its value"
    );
    let made = continue_throughout(&mut app);
    assert_ne!(made, original);
    let w = &app.session.wallets[made];
    let address = app
        .session
        .address(&faraday_core::testkit::compute(), w, false, 0);
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
    assert_eq!(
        app.session
            .address(&faraday_core::testkit::compute(), o, false, 0),
        first
    );
}

#[test]
fn new_wallet_from_a_two_of_three_held_here_takes_every_key_at_account_1() {
    let mut app = testkit::started();
    for seed in ["bacon", "zebra", "summer"] {
        add_key(&mut app, seed);
    }
    app.press(Action::CreateWallet);
    app.press(Action::CKind(multi_index()));
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CNext(cstep::QUORUM));
    for k in 0..3u8 {
        let fp = app.session.keys[usize::from(k)].fingerprint.0;
        app.press(Action::CSlotHere(k, fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    let original = app.create().and_then(|c| c.built).expect("made");
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
    let c = faraday_core::testkit::compute();
    use faraday_core::create::Source;
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    add_key(&mut app, "bacon");
    add_key(&mut app, "zebra");
    let keys: Vec<String> = (0..3).map(|n| testkit::key(n, "m/48'/1'/0'/2'")).collect();
    let i = app
        .session
        .add_wallet(
            &c,
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
    let c = app.create().unwrap();
    assert_eq!(c.account, 1);
    assert_eq!(c.slots[pos], Source::Later, "Someone else");
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::KEYS));
    let line = format!("Needs Key {}'s xpub at account 1", pos + 1);
    let texts = app.drawn_texts();
    assert!(texts.contains(&line), "{line}: {texts:?}");
    app.press(Action::CNext(cstep::KEYS));
    assert!(
        app.create().unwrap().built.is_none(),
        "no wallet until the cosigner's xpub is in"
    );
    // At the wallet's own account the xpub it has is that key again.
    app.press(Action::CAccount(-1));
    assert_eq!(app.create().unwrap().slots[pos], Source::Cosigner(xpub));
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

// NEW-WALLET §17 (F55, F98): the derivation path on Check, its account
// changed on the Account row, at the foot of Quorum or the head of Keys.

fn kind_index(kind: NewKind) -> u8 {
    NewKind::ALL
        .iter()
        .position(|k| *k == kind)
        .expect("one of the ten kinds") as u8
}

fn drawn_joined(app: &mut Faraday) -> String {
    app.drawn_texts().join("\n")
}

#[test]
fn check_shows_the_path_and_change_makes_the_wallet_again_at_another_account() {
    let (mut app, _) = made_on_testnet();
    let text = drawn_joined(&mut app);
    assert!(text.contains("m/84'/1'/0'"), "Check shows the path: {text}");
    assert!(
        app.offers(Action::CPathChange),
        "the Path row offers Change"
    );
    app.press(Action::CPathChange);
    let c = app.create().expect("create");
    assert_eq!(c.built, None, "the wallet is taken back");
    assert_eq!(
        c.open,
        Some(cstep::KEYS),
        "a single key's Keys opens, its Account row at its head"
    );
    assert!(app.session.wallets.is_empty(), "no wallet left behind");
    let _ = app.frame();
    assert!(app.offers(Action::CAccount(1)), "the stepper is open");
    app.press(Action::CAccount(1));
    app.press(Action::CAccount(1));
    let made = continue_throughout(&mut app);
    assert_eq!(app.session.wallets.len(), 1, "one wallet, not two");
    let w = &app.session.wallets[made];
    assert_eq!(
        app.session
            .address(&faraday_core::testkit::compute(), w, false, 0),
        p2wpkh_at("bacon", "m/84'/1'/2'/0/0"),
        "the same words at account 2"
    );
    let text = drawn_joined(&mut app);
    assert!(
        text.contains("m/84'/1'/2'"),
        "Check shows account 2: {text}"
    );
}

#[test]
fn a_multisig_at_account_1_set_on_quorum_has_it_in_its_path_and_check_returns_there() {
    let mut app = testkit::started();
    for seed in ["bacon", "zebra", "summer"] {
        add_key(&mut app, seed);
    }
    app.press(Action::CreateWallet);
    app.press(Action::CStep(cstep::KIND));
    let _ = app.frame();
    assert!(!app.offers(Action::CAccount(0)), "Kind asks no account");
    app.press(Action::CKind(multi_index()));
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::QUORUM));
    let _ = app.frame();
    assert!(
        app.offers(Action::CAccount(0)) && !app.offers(Action::CAccount(1)),
        "Quorum shows the account with Change, the stepper closed"
    );
    app.press(Action::CAccount(0));
    let _ = app.frame();
    assert!(app.offers(Action::CAccount(1)), "Change opens the stepper");
    app.press(Action::CAccount(1));
    app.press(Action::CNext(cstep::QUORUM));
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    for k in 0..3u8 {
        let fp = app.session.keys[usize::from(k)].fingerprint.0;
        app.press(Action::CSlotHere(k, fp));
    }
    app.press(Action::CNext(cstep::KEYS));
    let made = app.create().and_then(|c| c.built).expect("made");
    for k in app.session.wallets[made].policy().keys() {
        assert_eq!(account_of_key(k), 1, "{} is at account 1", k.key_text());
    }
    let want = NewKind::Multi.path_at(app.session.network(), 1);
    let text = drawn_joined(&mut app);
    assert!(text.contains(&want), "Check shows {want}: {text}");
    app.press(Action::CPathChange);
    let c = app.create().expect("create");
    assert_eq!(c.built, None, "the wallet is taken back");
    assert_eq!(c.open, Some(cstep::QUORUM), "Change returns to Quorum");
    assert_eq!(c.account, 1, "at the account it was made at");
    let _ = app.frame();
    assert!(
        app.offers(Action::CAccount(1)),
        "Quorum's Account row is open on its stepper"
    );
}

#[test]
fn a_kind_without_an_account_offers_no_account_row() {
    let mut app = opened();
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CVaults(false));
    // New key opens in place of Keys; back from it, the Keys card is
    // still there to open.
    assert_eq!(app.screen, Screen::KeyGen);
    app.press(Action::CancelFlow);
    app.press(Action::CStep(cstep::KEYS));
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::KEYS));
    let _ = app.frame();
    assert!(
        app.offers(Action::CAccount(0)),
        "a single key's Keys offers its account"
    );
    app.press(Action::CStep(cstep::KIND));
    app.press(Action::CKind(kind_index(NewKind::MultiLegacy)));
    app.press(Action::CNext(cstep::KIND));
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::QUORUM));
    let _ = app.frame();
    assert!(
        !app.offers(Action::CAccount(0)),
        "m/45' has no account to choose"
    );
}

#[test]
fn checks_foot_reads_it_matches() {
    let (mut app, _) = made();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "It matches"),
        "Check's press says the addresses match: {texts:?}"
    );
}

/// Make a new wallet at Keys: single key, No to vaults.
fn single_key_to_keys(app: &mut Faraday) {
    app.press(Action::CNext(cstep::KIND));
    app.press(Action::CVaults(false));
}

/// New key at its entry card, a few dice rolled.
fn roll_some(app: &mut Faraday) {
    app.press(Action::KWords(12));
    app.press(Action::KWay(faraday_core::keygen::Way::DiceHashed.index()));
    app.press(Action::KNext);
    for i in 0..5u8 {
        app.press(Action::KRoll(i % 6 + 1));
    }
    assert!(!app.keygen().unwrap().dice.is_empty(), "the rolls are in");
}

#[test]
fn a_single_key_with_nothing_loaded_goes_from_the_step_before_keys_to_new_key() {
    let mut app = opened();
    single_key_to_keys(&mut app);
    assert_eq!(app.screen, Screen::KeyGen, "New key in place of Keys");
    let k = app.keygen().unwrap();
    assert_eq!(k.slot, Some(0), "for slot 1, as its button would");
    assert_eq!(k.back, Screen::Create);
    let c = app.create().unwrap();
    assert!(!c.done[cstep::KEYS as usize], "Keys is not done yet");
    assert!(
        app.create_cards().contains(&cstep::KEYS),
        "Keys stays listed"
    );
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "New key"), "{texts:?}");
}

#[test]
fn with_a_key_in_the_session_the_keys_card_shows() {
    let mut app = testkit::started();
    add_key(&mut app, "bacon");
    app.press(Action::CreateWallet);
    app.press(Action::CKind(0));
    single_key_to_keys(&mut app);
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::KEYS));
    let fp = app.session.keys[0].fingerprint.0;
    let _ = app.frame();
    assert!(app.offers(Action::CSlotHere(0, fp)), "the key is offered");
    assert!(app.offers(Action::KeyGen(Some(0))), "and New key beside it");
}

#[test]
fn a_multisig_shows_the_keys_card() {
    let mut app = opened();
    app.press(Action::CStep(cstep::KIND));
    app.press(Action::CKind(multi_index()));
    app.press(Action::CNext(cstep::QUORUM));
    app.press(Action::CVaults(false));
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::KEYS));
}

#[test]
fn cancel_in_new_key_from_create_with_rolls_asks_then_returns_to_keys_with_the_slot_not_chosen() {
    let mut app = testkit::started();
    add_key(&mut app, "bacon");
    app.press(Action::CreateWallet);
    app.press(Action::CKind(0));
    single_key_to_keys(&mut app);
    app.press(Action::KeyGen(Some(0)));
    let _ = app.frame();
    assert!(
        app.offers(Action::CancelFlow),
        "New key from Create has Cancel"
    );
    roll_some(&mut app);
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, Some(faraday_core::Sheet::CancelKey), "it asks");
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t == "Discard this key?"), "{texts:?}");
    // Keep going keeps the rolls.
    app.press(Action::Cancel);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::KeyGen);
    assert!(!app.keygen().unwrap().dice.is_empty(), "the rolls are kept");
    // Discard returns to Make a new wallet at Keys, the slot not chosen.
    app.press(Action::CancelFlow);
    app.press(Action::CancelFlowNow);
    assert_eq!(app.sheet, None);
    assert_eq!(app.screen, Screen::Create);
    assert!(app.keygen().is_none(), "the key so far is gone");
    let c = app.create().unwrap();
    assert_eq!(c.open, Some(cstep::KEYS));
    assert_eq!(c.slots[0], faraday_core::create::Source::Empty);
    assert_eq!(app.session.keys.len(), 1, "no key was added");
}

#[test]
fn cancel_with_nothing_entered_returns_without_asking_to_the_step_before_a_passed_over_keys() {
    let mut app = opened();
    single_key_to_keys(&mut app);
    assert_eq!(app.screen, Screen::KeyGen);
    app.press(Action::CancelFlow);
    assert_eq!(app.sheet, None, "nothing entered: no question");
    assert_eq!(app.screen, Screen::Create);
    let c = app.create().unwrap();
    assert_eq!(c.open, Some(cstep::WHERE), "the step before Keys");
    assert_eq!(c.slots[0], faraday_core::create::Source::Empty);
    // Its Continue opens New key again.
    app.press(Action::CNext(cstep::WHERE));
    assert_eq!(app.screen, Screen::KeyGen);
    // Rolls entered, it asks first, and lands there too.
    roll_some(&mut app);
    app.press(Action::CancelFlow);
    app.press(Action::CancelFlowNow);
    assert_eq!(app.screen, Screen::Create);
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::WHERE));
}

#[test]
fn new_key_opened_from_home_keeps_its_back_and_has_no_cancel() {
    let mut app = testkit::started();
    app.press(Action::Nav(Screen::Home));
    app.press(Action::KeyGen(None));
    assert_eq!(app.screen, Screen::KeyGen);
    roll_some(&mut app);
    let _ = app.frame();
    assert!(!app.offers(Action::CancelFlow), "no Cancel on its own");
    app.press(Action::CancelFlow);
    assert_eq!(app.screen, Screen::KeyGen, "nothing happens");
    assert_eq!(app.sheet, None);
    assert!(app.offers(Action::Nav(Screen::Home)), "its back goes Home");
    app.press(Action::Nav(Screen::Home));
    assert_eq!(app.screen, Screen::Home);
}

/// A letter outside printable ASCII typed into a slot's passphrase is
/// not taken, and the slot says it is refused.
#[test]
fn a_slots_passphrase_refuses_a_letter_outside_ascii_and_says_so() {
    let mut app = testkit::started();
    let first = key_made_on_its_own(&mut app);
    app.press(Action::CreateWallet);
    app.press(Action::CKind(0));
    app.press(Action::CVaults(false));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CSlotHere(0, first));
    app.press(Action::CPassOpen(0));
    type_text(&mut app, "café");
    assert_eq!(&*app.create().unwrap().pass, "caf");
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == crate::ascii_passphrase::REFUSED),
        "{texts:?}"
    );
}

/// Check's two receive rows each open their address's code, the address
/// as the row shows it, with no picture (`docs/QR.md` §7).
#[test]
fn checks_receive_rows_open_their_codes() {
    let c = testkit::compute();
    let (mut app, w) = made();
    for i in 0..2 {
        let want = app
            .session
            .address_shown(&c, &app.session.wallets[w], false, i);
        crate::common::opens_its_code(&mut app, faraday_core::Code::Receive(w, i), &want);
    }
}
