//! The Inbox put together: a wallet that comes in several files is one
//! wallet; a seed that comes in several backups is one seed; a seed a
//! wallet names loads with it; a seed no wallet names is a potential
//! wallet, made one with a passphrase or without; split seeds (SLIP-39,
//! codex32) count once they add up; a sealed OpenSigner backup opens with
//! its passphrase.

use faraday_core::inbox::SeedSource;
use faraday_core::testkit;
use faraday_core::wallet::FileKind;
use faraday_core::{Action, Faraday, Sheet, StorageEvent};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

fn with_inbox(files: Vec<(String, Vec<u8>)>) -> Faraday {
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
        inbox: files,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn kit_files(pick: impl Fn(&str) -> bool) -> Vec<(String, Vec<u8>)> {
    testkit::files()
        .unwrap()
        .into_iter()
        .filter(|(n, _)| pick(n))
        .collect()
}

fn words(seed: &str) -> Vec<u8> {
    format!("{}\n", testkit::test_words(seed)).into_bytes()
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

#[test]
fn a_wallet_in_several_files_is_one_wallet() {
    let files = kit_files(|n| n.starts_with("savings-") && !n.contains("psbt"));
    let mut app = with_inbox(files);
    let wallet_files = app
        .inbox
        .iter()
        .filter(|i| i.kind == FileKind::Wallet)
        .count();
    assert!(
        wallet_files > 1,
        "the test kit has Savings in several forms"
    );
    let found = app.inbox_found();
    let savings: Vec<_> = found.wallets.iter().filter(|w| w.keys.len() == 3).collect();
    assert_eq!(savings.len(), 1, "one Savings, however many files");
    assert_eq!(savings[0].files.len(), wallet_files);
    app.press(Action::InboxLoad);
    assert_eq!(app.session.wallets.len(), 1);
}

#[test]
fn a_seed_in_several_backups_is_one_seed() {
    let app = with_inbox(vec![
        ("bacon-words.txt".to_string(), words("bacon")),
        ("bacon-seedqr-words.txt".to_string(), words("bacon")),
    ]);
    let found = app.inbox_found();
    assert_eq!(found.seeds.len(), 1);
    assert_eq!(found.seeds[0].sources.len(), 2);
}

#[test]
fn a_seed_its_wallet_names_loads_with_it_and_is_no_potential_wallet() {
    let mut files = kit_files(|n| n == "spending-wallet.txt");
    files.push(("bacon-words.txt".to_string(), words("bacon")));
    let mut app = with_inbox(files);
    let found = app.inbox_found();
    assert!(found.potential().is_empty());
    assert_eq!(found.to_load().len(), 1);
    assert_eq!(found.to_load()[0].seeds_here, 1);
    app.press(Action::InboxLoad);
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.session.keys.len(), 1);
}

#[test]
fn loading_a_seed_from_the_inbox_loads_the_wallet_that_names_it() {
    let mut files = kit_files(|n| n == "spending-wallet.txt");
    files.push(("bacon-words.txt".to_string(), words("bacon")));
    let mut app = with_inbox(files);
    let k = app
        .inbox
        .iter()
        .position(|i| i.name == "bacon-words.txt")
        .unwrap();
    app.press(Action::LoadKey(k));
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(
        app.session.wallets.len(),
        1,
        "its wallet did not come with it"
    );
}

#[test]
fn a_seed_no_wallet_names_waits_as_a_potential_wallet() {
    let mut app = with_inbox(vec![("summer-words.txt".to_string(), words("summer"))]);
    let found = app.inbox_found();
    assert_eq!(found.potential().len(), 1);
    assert!(app.session.wallets.is_empty(), "nothing is a wallet yet");
    let fp = found.potential()[0].fingerprint;
    app.press(Action::PotentialOpen(fp.0));
    assert_eq!(app.sheet, Some(Sheet::Potential));
    // No passphrase: Native SegWit, the first kind.
    app.press(Action::PotentialMake);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.keys[0].master.fingerprint(), fp);
    assert_eq!(app.session.wallets.len(), 1);
}

#[test]
fn a_passphrase_makes_the_potential_wallet_another_key() {
    let mut app = with_inbox(vec![("summer-words.txt".to_string(), words("summer"))]);
    let fp = app.inbox_found().potential()[0].fingerprint;
    app.press(Action::PotentialOpen(fp.0));
    app.press(Action::PotentialType);
    type_text(&mut app, "TREZOR");
    let with = app.potential.as_ref().unwrap().with.unwrap();
    assert_ne!(with, fp, "a passphrase gives another fingerprint");
    app.press(Action::PotentialKind(1));
    app.press(Action::PotentialMake);
    assert_eq!(app.session.keys.len(), 1);
    assert_eq!(app.session.keys[0].master.fingerprint(), with);
    assert_eq!(app.session.wallets.len(), 1);
    assert!(
        app.session.wallets[0]
            .policy
            .to_descriptor()
            .starts_with("tr(")
    );
}

#[test]
fn codex32_strings_count_once_they_add_up() {
    // BIP 93 vector 2: two shares of a 2-of-n.
    let one = with_inbox(vec![(
        "share-a.txt".to_string(),
        b"MS12NAMEA320ZYXWVUTSRQPNMLKJHGFEDCAXRPP870HKKQRM\n".to_vec(),
    )]);
    assert_eq!(one.inbox[0].kind, FileKind::SeedPart);
    let found = one.inbox_found();
    assert!(found.seeds.is_empty());
    assert_eq!(found.waiting.len(), 1, "one share of two waits");

    let two = with_inbox(vec![
        (
            "share-a.txt".to_string(),
            b"MS12NAMEA320ZYXWVUTSRQPNMLKJHGFEDCAXRPP870HKKQRM\n".to_vec(),
        ),
        (
            "share-c.txt".to_string(),
            b"MS12NAMECACDEFGHJKLMNPQRSTUVWXYZ023FTR2GDZMPY6PN\n".to_vec(),
        ),
    ]);
    let found = two.inbox_found();
    assert_eq!(found.seeds.len(), 1);
    assert!(matches!(found.seeds[0].sources[0], SeedSource::Codex32(_)));
}

#[test]
fn slip39_shares_count_once_they_add_up() {
    // SLIP-39 vector 4, 2 of 3.
    let app = with_inbox(vec![
        (
            "share-1.txt".to_string(),
            b"shadow pistol academic always adequate wildlife fancy gross oasis cylinder mustang wrist rescue view short owner flip making coding armed\n".to_vec(),
        ),
        (
            "share-2.txt".to_string(),
            b"shadow pistol academic acid actress prayer class unknown daughter sweater depict flip twice unkind craft early superior advocate guest smoking\n".to_vec(),
        ),
    ]);
    let found = app.inbox_found();
    assert_eq!(found.seeds.len(), 1);
    assert!(matches!(found.seeds[0].sources[0], SeedSource::Slip39(_)));
}

#[test]
fn a_sealed_backup_opens_with_its_passphrase() {
    let files = kit_files(|n| n.ends_with(".oskb"));
    assert!(!files.is_empty());
    let mut app = with_inbox(files);
    let k = app
        .inbox
        .iter()
        .position(|i| i.kind == FileKind::Backup)
        .unwrap();
    assert!(
        app.inbox_found()
            .waiting
            .iter()
            .any(|q| q.backup == Some(k))
    );
    app.press(Action::BackupOpen(k));
    type_text(&mut app, "wrong");
    app.press(Action::PotentialMake);
    assert!(app.session.keys.is_empty());
    assert!(app.potential.as_ref().unwrap().error.is_some());
    type_text(&mut app, "backup");
    app.press(Action::PotentialMake);
    assert_eq!(app.session.keys.len(), 1, "the backup's seed did not load");
}

#[test]
fn an_account_xpub_alone_becomes_a_watch_only_wallet_of_its_paths_kind() {
    let files = kit_files(|n| n.starts_with("xpub-3-") && n.ends_with("-single.txt"));
    assert_eq!(files.len(), 1);
    let mut app = with_inbox(files);
    let found = app.inbox_found();
    assert_eq!(found.xpubs.len(), 1);
    assert_eq!(
        found.xpubs[0].kind,
        Some(faraday_core::create::NewKind::NativeSegwit),
        "m/84' is a native SegWit account"
    );
    app.press(Action::XpubOpen(0));
    assert_eq!(app.sheet, Some(Sheet::Potential));
    app.press(Action::PotentialMake);
    assert_eq!(app.session.wallets.len(), 1);
    assert!(app.session.keys.is_empty(), "watch-only: no key");
    assert!(
        app.session.wallets[0]
            .policy
            .to_descriptor()
            .starts_with("wpkh(")
    );
}

#[test]
fn a_cosigners_multisig_xpub_is_not_a_wallet_on_its_own() {
    let files = kit_files(|n| n.starts_with("xpub-3-") && n.ends_with("-multisig.txt"));
    let app = with_inbox(files);
    assert!(app.inbox_found().xpubs.is_empty());
}

#[test]
fn an_xpub_whose_seed_is_here_gives_way_to_the_seed() {
    let mut files = kit_files(|n| n.starts_with("xpub-3-") && n.ends_with("-single.txt"));
    files.push(("summer-words.txt".to_string(), words("summer")));
    let app = with_inbox(files);
    let found = app.inbox_found();
    assert!(found.xpubs.is_empty());
    assert_eq!(found.potential().len(), 1);
}
