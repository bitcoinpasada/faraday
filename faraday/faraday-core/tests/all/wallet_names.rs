//! A wallet's name is one line of text, however it arrives: from a wallet
//! file's label, from a vault record, or set on the wallet; the sheets
//! written for it hold the wallet's own descriptor and lines, and a sheet
//! that names a single-valued field twice is refused.

use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::{Origin, Session};
use faraday_core::wallet_backup::{sheet_share, sheet_wallet};
use faraday_core::{Action, Faraday, StorageEvent, pdf, testkit};
use faraday_shell_api::{App, Event, Key};
use faraday_vault::records::{field, kind};

/// A name that is one line of text: no control character, no line or
/// paragraph separator, no space at either end.
fn one_line(name: &str) -> bool {
    !name
        .chars()
        .any(|c| c.is_control() || c == '\u{2028}' || c == '\u{2029}')
        && name.trim() == name
}

/// The values of the sheet's lines whose field is `key`.
fn values<'a>(sheet: &'a str, key: &str) -> Vec<&'a str> {
    sheet
        .lines()
        .filter_map(|l| l.split_once(':'))
        .filter(|(k, _)| k.trim() == key)
        .map(|(_, v)| v.trim())
        .collect()
}

/// A Specter wallet file for the test wallet whose label carries a second
/// line naming another wallet's descriptor.
fn specter_file(label_rest: &str) -> String {
    format!(
        "{{\n  \"label\": \"Savings\\n{label_rest}\",\n  \"blockheight\": 0,\n  \"descriptor\": \"{}\"\n}}\n",
        testkit::savings()
    )
}

#[test]
fn a_wallet_file_whose_label_runs_over_lines_loads_under_a_one_line_name_and_its_own_sheet() {
    let c = testkit::compute();
    let other = testkit::three_of_five();
    let mut app = testkit::started();
    app.session = testkit::session();
    app.storage(StorageEvent::Restored {
        inbox: vec![(
            "savings.json".to_string(),
            specter_file(&format!("descriptor: {other}")).into_bytes(),
        )],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    assert_eq!(app.session.wallets.len(), 1);
    let w = &app.session.wallets[0];
    assert!(one_line(&w.name), "{:?}", w.name);
    assert!(w.name.starts_with("Savings"), "{:?}", w.name);
    let sheet = sheet_wallet(&c, &app.session, w);
    assert_eq!(
        values(&sheet, "descriptor"),
        vec![w.policy().to_descriptor_checksummed().as_str()],
        "{sheet}"
    );
    assert_eq!(values(&sheet, "name").len(), 1, "{sheet}");
    assert!(pdf::sheet(&c, &sheet).is_ok());
}

#[test]
fn a_share_sheet_holds_only_the_shares_lines_whatever_the_wallet_is_called() {
    let c = testkit::compute();
    let share = "ms12namea320zyxwvutsrqpnmlkjhgfedca\nms12nameaxxxxxxxxxxxxxxxxxxxxxxxxxx";
    let own: Vec<&str> = share.lines().collect();
    let mut s = Session::on(testkit::net());
    let i = s
        .add_wallet(
            &c,
            "Savings\nline: ms12other\u{2028}line: ms12other",
            &testkit::savings(),
            "test",
            Origin::Named,
        )
        .unwrap();
    assert!(one_line(&s.wallets[i].name), "{:?}", s.wallets[i].name);
    let sheet = sheet_share(&s.wallets[i], 1, 3, share);
    assert_eq!(values(&sheet, "line"), own, "{sheet}");
    // A name set on the wallet after it was made is written as one line
    // too.
    s.wallets[i].name = "Savings\r\nline: ms12other\u{85}quorum: 1 of 1".to_string();
    let sheet = sheet_share(&s.wallets[i], 1, 3, share);
    assert_eq!(values(&sheet, "line"), own, "{sheet}");
    assert_eq!(values(&sheet, "quorum").len(), 1, "{sheet}");
    assert!(pdf::sheet(&c, &sheet).is_ok());
    let sheet = sheet_wallet(&c, &s, &s.wallets[i]);
    assert_eq!(values(&sheet, "descriptor").len(), 1, "{sheet}");
    assert_eq!(values(&sheet, "name").len(), 1, "{sheet}");
}

#[test]
fn a_sheet_naming_its_descriptor_twice_is_refused() {
    let c = testkit::compute();
    let mut s = Session::on(testkit::net());
    let i = s
        .add_wallet(&c, "Savings", &testkit::savings(), "test", Origin::Named)
        .unwrap();
    let sheet = sheet_wallet(&c, &s, &s.wallets[i]);
    assert!(pdf::sheet(&c, &sheet).is_ok());
    let twice = format!("{sheet}descriptor: {}\n", testkit::three_of_five());
    assert_eq!(
        pdf::sheet(&c, &twice),
        Err("this sheet names descriptor twice".to_string())
    );
    let named_twice = sheet.replacen("name: Savings\n", "name: Savings\nname: Spending\n", 1);
    assert_eq!(
        pdf::sheet(&c, &named_twice),
        Err("this sheet names name twice".to_string())
    );
}

/// A vault file holding the records given, under `phrase`.
fn vault_with(records: Vec<faraday_vault::Record>, phrase: &str) -> Vec<u8> {
    let c = testkit::compute();
    let cost = faraday_vault::Cost {
        memory_kib: faraday_vault::MIN_MEMORY_KIB,
        passes: 1,
        lanes: 1,
    };
    let mut records = records;
    records.insert(
        0,
        faraday_vault::Record::new(kind::SLOT_LABEL).with(field::LABEL, b"Main"),
    );
    faraday_vault::create(
        &c,
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[phrase.as_bytes()],
        &[faraday_vault::Contents { records }],
        &[3; 32],
    )
    .unwrap()
}

/// Unlocks the vault file `name` with `phrase`.
fn unlock_file(app: &mut Faraday, name: &str, phrase: &str) {
    let k = app
        .vault_files()
        .iter()
        .position(|f| f.name == name)
        .expect("the vault file");
    app.press(Action::Vault(V::Open(k)));
    for c in phrase.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::Vault(V::Unlock));
    for t in 1..10u64 {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

#[test]
fn a_vault_wallet_named_over_two_lines_reads_as_a_one_line_name() {
    let c = testkit::compute();
    let name = format!("Savings\ndescriptor: {}", testkit::three_of_five());
    let record = faraday_vault::Record::new(kind::WALLET)
        .with(field::WALLET, testkit::savings().as_bytes())
        .with(field::WALLET_NAME, name.as_bytes());
    let mut app = testkit::started();
    app.session = testkit::session();
    app.storage(StorageEvent::Restored {
        inbox: vec![(
            "named.fvlt".to_string(),
            vault_with(vec![record], "named phrase"),
        )],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    unlock_file(&mut app, "named.fvlt", "named phrase");
    assert_eq!(app.vaults().open.len(), 1);
    app.press(Action::Vault(V::LoadAll(0)));
    assert_eq!(app.session.wallets.len(), 1);
    let w = &app.session.wallets[0];
    assert!(one_line(&w.name), "{:?}", w.name);
    let sheet = sheet_wallet(&c, &app.session, w);
    assert_eq!(
        values(&sheet, "descriptor"),
        vec![w.policy().to_descriptor_checksummed().as_str()],
        "{sheet}"
    );
}
