//! A wallet file that carries secret text beside its descriptor, in a
//! member or a comment its reader skips, is a secret file and asks before
//! it is copied to a stick; the files other wallets write stay public.

use faraday_core::wallet::{FileKind, classify};

const CORE: &str = include_str!("../vectors/interop/core/listdescriptors.json");
const ELECTRUM: &str = include_str!("../vectors/interop/sparrow/electrum-singlesig-wallet.json");
const DESCRIPTOR: &str = include_str!("../vectors/interop/bluewallet/coldcardmk4-descriptor.txt");
const WORDS: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
const XPRV: &str = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";

fn kind(text: &str) -> FileKind {
    let c = faraday_core::testkit::compute();
    classify(&c, "wallet.txt", text.as_bytes())
}

#[test]
fn other_wallets_public_files_stay_wallet_files() {
    assert_eq!(kind(CORE), FileKind::Wallet);
    assert_eq!(kind(ELECTRUM), FileKind::Wallet);
    assert_eq!(kind(DESCRIPTOR), FileKind::Wallet);
}

#[test]
fn a_wallet_json_with_a_seed_or_a_private_key_in_another_member_is_a_secret_file() {
    for secret in [WORDS, XPRV] {
        let with = CORE.replacen('{', &format!("{{\n  \"note\": \"{secret}\","), 1);
        assert_eq!(kind(&with), FileKind::SecretWallet, "{secret}");
    }
}

#[test]
fn a_descriptor_with_a_seed_in_a_comment_line_is_a_secret_file() {
    let with = format!("# seed: {WORDS}\n{DESCRIPTOR}");
    assert_eq!(kind(&with), FileKind::SecretWallet);
    let with = format!("# {XPRV}\n{DESCRIPTOR}");
    assert_eq!(kind(&with), FileKind::SecretWallet);
    let plain = format!("# my savings\n{DESCRIPTOR}");
    assert_eq!(kind(&plain), FileKind::Wallet);
}

const MULTISIG: &str = include_str!("../vectors/interop/bluewallet/fromsparrow-coldcard.txt");

#[test]
fn a_multisig_config_with_a_seed_after_a_hash_or_in_its_name_is_a_secret_file() {
    assert_eq!(kind(MULTISIG), FileKind::Wallet);
    let policy = MULTISIG.lines().find(|l| l.starts_with("Policy:")).unwrap();
    let with = MULTISIG.replacen(policy, &format!("{policy}  # {WORDS}"), 1);
    assert_eq!(kind(&with), FileKind::SecretWallet);
    let name = MULTISIG.lines().find(|l| l.starts_with("Name:")).unwrap();
    let with = MULTISIG.replacen(name, &format!("Name: {WORDS}"), 1);
    assert_eq!(kind(&with), FileKind::SecretWallet);
}

#[test]
fn a_labelled_seed_in_a_json_string_nested_or_in_an_array_is_a_secret_file() {
    let with = CORE.replacen('{', &format!("{{\n  \"note\": \"seed: {WORDS}\","), 1);
    assert_eq!(kind(&with), FileKind::SecretWallet);
    let with = CORE.replacen(
        '{',
        &format!("{{\n  \"extra\": {{\"list\": [1, [\"backup: {WORDS}\"]]}},"),
        1,
    );
    assert_eq!(kind(&with), FileKind::SecretWallet);
}

#[test]
fn a_notes_file_or_an_export_with_a_seed_is_not_called_a_wallet_file() {
    let notes = format!("# Notes\n\n## Recovery: {WORDS}\n");
    let export = format!("{{\"title\": \"bank\", \"notes\": \"{WORDS}\"}}");
    for text in [notes, export] {
        let k = kind(&text);
        assert!(
            !matches!(k, FileKind::Wallet | FileKind::SecretWallet),
            "{k:?}"
        );
    }
}

const KEY: &str = "[086ee178/84h/0h/0h]xpub6CqWTnie1ut9ZDD9xDeCn1VXk83VdAPSm8ZPfPNbb8w5z1e7jyy8zuX721uKj8u4GNxXqAevgEZjciUansnyz6ZhnSKyQWZwx2dpAxuCuDe";

#[test]
fn a_key_file_with_a_seed_beside_the_key_is_a_secret_file() {
    assert_eq!(kind(KEY), FileKind::Key);
    for with in [
        format!("{KEY}\n{WORDS}\n"),
        format!("{KEY}\n# {WORDS}\n"),
        format!("{KEY}\n## seed: {WORDS}\n"),
    ] {
        assert_eq!(kind(&with), FileKind::KeyWithSecret, "{with}");
    }
}

#[test]
fn a_coldcard_key_file_with_a_seed_in_a_member_is_a_secret_file() {
    let json = format!(
        "{{\"xfp\": \"73C5DA0A\", \"note\": \"{WORDS}\", \"p2wsh_deriv\": \"m/48'/0'/0'/2'\", \"p2wsh\": \"{KEY_PLAIN}\"}}"
    );
    assert_eq!(kind(&json), FileKind::KeyWithSecret);
}

const KEY_PLAIN: &str = "xpub6CqWTnie1ut9ZDD9xDeCn1VXk83VdAPSm8ZPfPNbb8w5z1e7jyy8zuX721uKj8u4GNxXqAevgEZjciUansnyz6ZhnSKyQWZwx2dpAxuCuDe";

#[test]
fn a_descriptor_with_a_seed_after_a_run_of_hashes_is_a_secret_file() {
    let with = format!("## {WORDS}\n{DESCRIPTOR}");
    assert_eq!(kind(&with), FileKind::SecretWallet);
}

#[test]
fn a_specter_diy_command_with_a_seed_as_its_name_is_a_secret_file() {
    let d = DESCRIPTOR
        .lines()
        .find(|l| l.contains("wpkh("))
        .unwrap()
        .trim();
    assert_eq!(kind(&format!("addwallet Savings&{d}")), FileKind::Wallet);
    assert_eq!(
        kind(&format!("addwallet {WORDS}&{d}")),
        FileKind::SecretWallet
    );
}

fn numbered() -> String {
    WORDS
        .split(' ')
        .enumerate()
        .map(|(i, w)| format!("{}. {w}", i + 1))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_key_file_with_a_seed_over_several_lines_is_a_secret_file() {
    let one_per_line = WORDS.replace(' ', "\n");
    let rows: Vec<String> = WORDS
        .split(' ')
        .collect::<Vec<_>>()
        .chunks(4)
        .map(|c| c.join(" "))
        .collect();
    for seed in [one_per_line, numbered(), rows.join("\n")] {
        assert_eq!(
            kind(&format!("{KEY}\n{seed}\n")),
            FileKind::KeyWithSecret,
            "{seed}"
        );
    }
}

#[test]
fn a_key_json_with_a_seed_as_an_array_of_words_is_a_secret_file() {
    let array = WORDS
        .split(' ')
        .map(|w| format!("    \"{w}\""))
        .collect::<Vec<_>>()
        .join(",\n");
    let json = format!(
        "{{\n  \"xfp\": \"73C5DA0A\",\n  \"p2wsh_deriv\": \"m/48'/0'/0'/2'\",\n  \"words\": [\n{array}\n  ],\n  \"p2wsh\": \"{KEY_PLAIN}\"\n}}"
    );
    assert_eq!(kind(&json), FileKind::KeyWithSecret);
}

#[test]
fn a_wallet_file_with_a_seed_over_several_comment_lines_is_a_secret_file() {
    let seed: Vec<String> = WORDS.split(' ').map(|w| format!("# {w}")).collect();
    let with = format!("{}\n{DESCRIPTOR}", seed.join("\n"));
    assert_eq!(kind(&with), FileKind::SecretWallet);
    let array = WORDS
        .split(' ')
        .map(|w| format!("\"{w}\""))
        .collect::<Vec<_>>()
        .join(",\n");
    let with = CORE.replacen('{', &format!("{{\n  \"note\": [\n{array}\n],"), 1);
    assert_eq!(kind(&with), FileKind::SecretWallet);
}

#[test]
fn a_key_file_with_ordinary_labels_stays_a_public_key_file() {
    assert_eq!(
        kind(&format!("# my cosigner key, on the ledger\n{KEY}\n")),
        FileKind::Key
    );
    let json = format!(
        "{{\n  \"xfp\": \"73C5DA0A\",\n  \"name\": \"legal winner thank year\",\n  \"p2wsh_deriv\": \"m/48'/0'/0'/2'\",\n  \"p2wsh\": \"{KEY_PLAIN}\"\n}}"
    );
    assert_eq!(kind(&json), FileKind::Key);
    assert_eq!(
        kind(&format!("# my savings, the first of two\n{DESCRIPTOR}")),
        FileKind::Wallet
    );
}

/// The Inbox position of `file` once Make a new wallet's Keys card for a
/// multisig wallet has it, and the app.
fn keys_card_with(name: &str, text: &str) -> (faraday_core::Faraday, usize) {
    use faraday_shell_api::{App, Event, Key};
    let mut app = faraday_core::testkit::started();
    // The files' keys are Mainnet keys: the session moves there, and the
    // key added is Trezor's "all all ..." seed, which is no sample (the
    // abandon seed in the files' notes is never loaded).
    faraday_core::testkit::move_to(&mut app.session, faraday_bip::keys::Network::Mainnet).unwrap();
    app.press(faraday_core::Action::Entry(None));
    for c in "all all all all all all all all all all all all".chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(faraday_core::Action::EntryAdd);
    faraday_core::testkit::keep_published(&mut app);
    let multi = faraday_core::create::NewKind::ALL
        .iter()
        .position(|k| *k == faraday_core::create::NewKind::Multi)
        .unwrap() as u8;
    app.press(faraday_core::Action::CreateWallet);
    app.press(faraday_core::Action::CKind(multi));
    app.press(faraday_core::Action::CNext(faraday_core::cstep::KIND));
    app.press(faraday_core::Action::CNext(faraday_core::cstep::QUORUM));
    faraday_core::testkit::inbox_file(&mut app, name, text.as_bytes().to_vec());
    let _ = app.frame();
    let at = app.inbox().iter().position(|i| i.name == name).unwrap();
    (app, at)
}

const COLDCARD: &str = include_str!("../vectors/interop/bluewallet/coldcardQ.json");

#[test]
fn a_key_file_with_a_seed_beside_it_still_fills_a_cosigners_slot_and_is_called_secret() {
    let text = format!(
        "{{\"xfp\": \"73C5DA0A\", \"note\": \"{WORDS}\", \"p2wsh_deriv\": \"m/48'/0'/0'/2'\", \"p2wsh\": \"{KEY_PLAIN}\"}}"
    );
    let (mut app, at) = keys_card_with("cosigner.json", &text);
    assert_eq!(app.inbox()[at].kind, FileKind::KeyWithSecret);
    assert!(app.offers(faraday_core::Action::CSlotFile(1, at)));
    app.press(faraday_core::Action::CSlotFile(1, at));
    match &app.create().unwrap().slots[1] {
        faraday_core::create::Source::Cosigner(k) => {
            assert!(k.starts_with("[73c5da0a/48"), "{}", k.as_str())
        }
        other => panic!("slot 2 is {other:?}"),
    }
}

#[test]
fn files_calls_a_key_file_with_a_seed_a_key_file_with_a_secret() {
    let text = format!("{KEY}\n# {WORDS}\n");
    let mut app = faraday_core::testkit::started();
    faraday_core::testkit::inbox_file(&mut app, "key.txt", text.into_bytes());
    app.press(faraday_core::Action::Nav(faraday_core::Screen::Files));
    let shown = crate::common::texts(&mut app);
    assert!(
        shown.iter().any(|t| t.contains("Key file with a secret")),
        "{shown:?}"
    );
}

#[test]
fn a_wallet_file_with_a_seed_in_a_note_is_still_listed_in_the_keys_card_and_fills_a_slot() {
    let plain = keys_card_with("plain.json", COLDCARD);
    assert!(plain.0.offers(faraday_core::Action::CSlotFile(1, plain.1)));
    let text = COLDCARD.replacen('{', &format!("{{\"note\": \"{WORDS}\","), 1);
    let (mut app, at) = keys_card_with("noted.json", &text);
    assert_eq!(app.inbox()[at].kind, FileKind::SecretWallet);
    assert!(app.offers(faraday_core::Action::CSlotFile(1, at)));
    app.press(faraday_core::Action::CSlotFile(1, at));
    assert!(
        matches!(
            app.create().unwrap().slots[1],
            faraday_core::create::Source::Cosigner(_)
        ),
        "the slot is empty"
    );
}

#[test]
fn a_seed_over_several_lines_is_found_among_other_words_of_the_list() {
    let lines = WORDS.replace(' ', "\n");
    // A heading that is a list word, as the file's own words.
    assert_eq!(
        kind(&format!("{KEY}\nRecovery phrase\n{lines}\n")),
        FileKind::KeyWithSecret
    );
    // A comment numbered with a colon, in a key file and in a wallet file.
    let numbered: Vec<String> = WORDS
        .split(' ')
        .enumerate()
        .map(|(i, w)| format!("# {}: {w}", i + 1))
        .collect();
    assert_eq!(
        kind(&format!("{KEY}\n{}\n", numbered.join("\n"))),
        FileKind::KeyWithSecret
    );
    assert_eq!(
        kind(&format!("{}\n{DESCRIPTOR}", numbered.join("\n"))),
        FileKind::SecretWallet
    );
    // A multisig config of Sparrow's, whose comments and name are list
    // words too, with the seed as comment lines at its end.
    let comments: Vec<String> = WORDS.split(' ').map(|w| format!("# {w}")).collect();
    assert_eq!(
        kind(&format!("{MULTISIG}\n{}\n", comments.join("\n"))),
        FileKind::SecretWallet
    );
}

#[test]
fn a_24_word_seed_over_several_lines_is_found_after_a_word_of_the_list() {
    let words24 = format!("{}art", "abandon ".repeat(23));
    let lines = words24.replace(' ', "\n");
    assert_eq!(
        kind(&format!("{KEY}\nRecovery phrase\n{lines}\n")),
        FileKind::KeyWithSecret
    );
    let comments: Vec<String> = words24.split(' ').map(|w| format!("# {w}")).collect();
    assert_eq!(
        kind(&format!("{MULTISIG}\n{}\n", comments.join("\n"))),
        FileKind::SecretWallet
    );
}

#[test]
fn a_key_file_with_a_big_file_of_words_is_read_without_delay() {
    for token in ["a", "abcd", "abandon"] {
        let filler = format!("{token} ").repeat(100_000);
        let start = std::time::Instant::now();
        let k = kind(&format!("{KEY}\n{filler}\n"));
        assert!(
            start.elapsed().as_secs() < 10,
            "{token}: {:?}",
            start.elapsed()
        );
        assert!(
            k == FileKind::Key || k == FileKind::KeyWithSecret,
            "{token}: {k:?}"
        );
    }
}

#[test]
fn a_key_file_with_a_private_key_copied_in_groups_of_four_is_named_as_holding_a_secret() {
    let groups: Vec<&str> = XPRV
        .as_bytes()
        .chunks(4)
        .map(|g| std::str::from_utf8(g).unwrap())
        .collect();
    let file = format!("{KEY}\n{}\n", groups.join("\n"));
    assert_eq!(kind(&file), FileKind::KeyWithSecret);
}

#[test]
fn a_big_file_of_short_tokens_is_read_without_delay() {
    for token in ["a", "ab", "1", "xy"] {
        let lines = format!("{token}\n").repeat(200_000);
        let one_line = format!("{token} ").repeat(1_000_000);
        for (what, file) in [
            ("lines", format!("{KEY}\n{lines}")),
            ("one line", format!("{KEY}\n{one_line}\n")),
            ("lines, no key", lines.clone()),
            ("one line, no key", one_line.clone()),
        ] {
            let start = std::time::Instant::now();
            let k = kind(&file);
            assert!(
                start.elapsed().as_secs() < 10,
                "{token} {what}: {:?}",
                start.elapsed()
            );
            if what.ends_with("no key") {
                assert_ne!(k, FileKind::Key, "{token} {what}");
            } else {
                assert_eq!(k, FileKind::Key, "{token} {what}");
            }
        }
    }
}

fn secret_wallet_files() -> Vec<(String, Vec<u8>)> {
    let (name, bytes) = faraday_core::testkit::files()
        .unwrap()
        .into_iter()
        .find(|(n, _)| n == "savings-wallet.txt")
        .unwrap();
    let text = format!("# {WORDS}\n{}", String::from_utf8(bytes).unwrap());
    vec![(name, text.into_bytes())]
}

#[test]
fn restore_takes_a_wallet_file_with_a_seed_comment_as_it_takes_a_wallet_file() {
    let mut app = crate::common::device(secret_wallet_files());
    assert_eq!(app.inbox()[0].kind, FileKind::SecretWallet);
    app.press(faraday_core::Action::RestoreWallet);
    crate::common::texts(&mut app);
    assert!(app.offers(faraday_core::Action::RFromFiles));
    app.press(faraday_core::Action::RFromFiles);
    faraday_core::testkit::confirm_wallets(&mut app);
    let r = app.restore().expect("restoring");
    assert!(r.wallet.is_some(), "{:?}", r.error);
}

#[test]
fn spend_offers_a_wallet_file_with_a_seed_comment_and_opens_its_wallet() {
    use faraday_core::spend_backup::{SpendBackupAction as F, page};
    let mut app = crate::common::device(secret_wallet_files());
    app.press(faraday_core::Action::Nav(faraday_core::Screen::SpendBackup));
    app.press(faraday_core::Action::SpendBackup(F::Next(page::MAP)));
    let shown = crate::common::texts(&mut app);
    assert!(
        shown.iter().any(|t| t == "Open savings-wallet.txt"),
        "{shown:?}"
    );
    assert!(app.offers(faraday_core::Action::SpendBackup(F::UseWallet(0))));
    app.press(faraday_core::Action::SpendBackup(F::UseWallet(0)));
    faraday_core::testkit::confirm_wallets(&mut app);
    assert!(app.spend_backup_wallet().is_some());
}

fn named_by_words() -> Vec<(String, Vec<u8>)> {
    let name = MULTISIG.lines().find(|l| l.starts_with("Name:")).unwrap();
    let with = MULTISIG.replacen(name, &format!("Name: {WORDS}"), 1);
    vec![(
        format!("{}.txt", WORDS.replace(' ', "-")),
        with.into_bytes(),
    )]
}

fn only_wallet_name(app: &faraday_core::Faraday) -> String {
    assert_eq!(app.session.wallets.len(), 1);
    app.session.wallets[0].name.as_str().to_string()
}

#[test]
fn a_multisig_config_named_by_seed_words_is_not_named_so_when_loaded_from_files() {
    let mut app = crate::common::device(named_by_words());
    app.press(faraday_core::Action::LoadWallet(0));
    faraday_core::testkit::confirm_wallets(&mut app);
    let name = only_wallet_name(&app);
    assert!(!name.contains("abandon"), "{name}");
}

#[test]
fn a_multisig_config_named_by_seed_words_is_not_named_so_when_restored() {
    let mut app = crate::common::device(named_by_words());
    app.press(faraday_core::Action::RestoreWallet);
    app.press(faraday_core::Action::RUse(0));
    faraday_core::testkit::confirm_wallets(&mut app);
    let name = only_wallet_name(&app);
    assert!(!name.contains("abandon"), "{name}");
}

#[test]
fn a_multisig_config_named_by_seed_words_is_not_named_so_when_spend_opens_it() {
    use faraday_core::spend_backup::{SpendBackupAction as F, page};
    let mut app = crate::common::device(named_by_words());
    app.press(faraday_core::Action::Nav(faraday_core::Screen::SpendBackup));
    app.press(faraday_core::Action::SpendBackup(F::Next(page::MAP)));
    app.press(faraday_core::Action::SpendBackup(F::UseWallet(0)));
    faraday_core::testkit::confirm_wallets(&mut app);
    let name = only_wallet_name(&app);
    assert!(!name.contains("abandon"), "{name}");
}
