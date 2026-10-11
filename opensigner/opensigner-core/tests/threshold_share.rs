//! A 24-word key that is a member of a FROST group (`docs/PLANNING.md`
//! §16.103, §16.104 rule 3): what loading one shows, where it lands,
//! what the wallet says about it, and what a lock, a wipe and an exit
//! leave.
//!
//! The group is the committed 2-of-3 regtest record and its three
//! 24-word keys, `tools/vectors/psbt/wallet-threshold-regtest*`.

mod common;

use common::{ABANDON, Harness, PANEL, PIN};
use opensigner_core::load::Step as LoadStep;
use osk_shell_api::Key;

/// The passphrase the encrypted backup is made under.
const PASSPHRASE: &str = "correct horse battery staple";

use opensigner_core::backup::BackupStep;
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::bip39::Language;
use osk_bip::threshold::ThresholdRecord;
use osk_shell_api::{Event, FileKind};
use osk_ui::widgets::Icon;

const RECORD: &str = include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest.record");
const SHARE_0: &str =
    include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest-share-0.txt");
const SHARE_1: &str =
    include_str!("../../../tools/vectors/psbt/wallet-threshold-regtest-share-1.txt");

fn record() -> ThresholdRecord {
    ThresholdRecord::parse(RECORD).expect("the committed record")
}

fn words(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

/// A device on regtest with one key loaded, which is what makes Add a
/// menu rather than Home's start rows.
fn device() -> Harness {
    let mut h = Harness::new(PANEL);
    h.set_network(3);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h
}

/// Reads the group record at the wallet scanner and puts it in use.
fn use_record(h: &mut Harness) {
    h.open_add_wallet(ids::WALLETS_LOAD);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: RECORD.as_bytes().to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Inspect);
    h.tap(ids::INSPECT_USE_WALLET);
    h.go_home();
}

/// The fingerprint of member `i`'s public share, which is what the
/// wallet's Keys review shows and what the paper carries.
fn member_print(i: usize) -> String {
    record()
        .share_fingerprint(i)
        .expect("a present member")
        .to_string()
}

/// The glyph of the one FROST wallet's row on Wallets.
fn threshold_row_glyph(h: &Harness) -> Icon {
    h.app
        .row_glyphs()
        .into_iter()
        .find(|(label, _)| label.starts_with(strings::EN.wallet_kind_threshold))
        .expect("the FROST row")
        .1
}

fn says(h: &Harness, text: &str) -> bool {
    h.app.texts().iter().any(|t| t == text)
}

// ---------------------------------------------------------------------

/// A member's 24 words are loaded through Load a key and are a key like
/// any other: Keys lists it by its master fingerprint and says nothing
/// about a group. With the record registered the wallet's row carries
/// the key glyph and its Keys review marks that one member.
#[test]
fn a_member_is_a_key_and_the_wallet_finds_it_by_public_share() {
    let mut h = device();
    use_record(&mut h);
    h.start_load_24(&words(SHARE_1));
    h.finish_load(None);

    h.open_keys();
    let prints: Vec<String> = h.app.fingerprints().iter().map(|f| f.to_string()).collect();
    assert_eq!(prints.len(), 2, "two keys, no list of its own: {prints:?}");
    assert!(
        !prints.contains(&member_print(1)),
        "Keys names a key by its master fingerprint: {prints:?}"
    );

    h.go_home();
    h.open_wallets();
    assert_eq!(
        threshold_row_glyph(&h),
        Icon::Wallet,
        "this device holds one of its keys"
    );

    // The Keys review names the record's three members by the
    // fingerprint of each public share, and marks the one held here.
    h.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0));
    h.tap(ids::WALLET_KEYS);
    for i in 0..3 {
        assert!(
            says(&h, &member_print(i)),
            "member {i}: {:?}",
            h.app.texts()
        );
    }
    let held: Vec<Icon> = h.app.row_glyphs().iter().map(|(_, icon)| *icon).collect();
    assert_eq!(
        held.iter().filter(|i| **i == Icon::Wallet).count(),
        1,
        "one member is held: {held:?}"
    );
    assert_eq!(
        held.iter().filter(|i| **i == Icon::Eye).count(),
        2,
        "the other two are watched: {held:?}"
    );
}

/// With no record registered the same words are a key and nothing more:
/// no wallet is made up for it, and nothing on its page says "share".
/// Reading the record afterwards gives the wallet its member.
#[test]
fn without_a_record_it_is_a_key_and_nothing_more() {
    let mut h = device();
    h.start_load_24(&words(SHARE_0));
    h.finish_load(None);
    h.open_wallets();
    assert!(
        h.app
            .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0))
            .is_none(),
        "no wallet was made up for it: {:?}",
        h.app.texts()
    );

    h.go_home();
    h.open_key(1);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);
    let texts = h.app.texts();
    assert!(
        !texts.iter().any(|t| t.to_lowercase().contains("share")),
        "nothing about shares: {texts:?}"
    );

    // The record arrives: the wallet is registered, and its row carries
    // the wallet glyph because a loaded key computes one of its members.
    h.go_home();
    use_record(&mut h);
    h.open_wallets();
    assert_eq!(threshold_row_glyph(&h), Icon::Wallet);
}

/// The same words are one key however they arrive: loading them twice
/// leaves one row on Keys.
#[test]
fn the_same_words_are_the_same_key() {
    let mut h = device();
    h.start_load_24(&words(SHARE_0));
    h.finish_load(None);
    let before = h.app.fingerprints();
    h.go_home();
    h.start_load_24(&words(SHARE_0));
    h.finish_load(None);
    h.open_keys();
    assert_eq!(h.app.fingerprints(), before, "one key, not two");
}

/// A member's encrypted backup is the same bytes a key's is, and it
/// reads back through Load a key to the same key.
#[test]
fn a_member_makes_an_encrypted_backup_that_reads_back() {
    let mut h = device();
    h.start_load_24(&words(SHARE_0));
    h.finish_load(None);
    h.go_home();
    h.open_keys();
    let before = h.app.fingerprints();

    h.open_key(1);
    h.tap(ids::DETAIL_BACKUP);
    assert_eq!(h.app.screen(), ScreenKind::BackupMenu);
    h.tap(ids::BACKUP_ENCRYPTED);
    // The form comes before the passphrase (§16.112); the file is the
    // row already checked.
    h.tap(ids::FORM_CONTINUE);
    assert_eq!(h.app.backup_step(), Some(BackupStep::Passphrase));
    h.type_text(PASSPHRASE);
    h.key(Key::Enter);
    h.type_text(PASSPHRASE);
    h.key(Key::Enter);
    assert_eq!(h.app.backup_step(), Some(BackupStep::Encrypted));

    h.seen.clear();
    h.tap(ids::BACKUP_SAVE);
    let bytes = h
        .seen
        .iter()
        .find_map(|c| match c {
            osk_shell_api::Command::WriteFile { bytes, .. } => Some(bytes.clone()),
            _ => None,
        })
        .expect("the backup was written");
    // The backup is the format `tools/backup/decrypt.py` reads: 24
    // words, their language, and 32 bytes.
    let opened =
        osk_backup::oskb::open(&bytes, PASSPHRASE.as_bytes()).expect("the passphrase opens it");
    assert_eq!(opened.word_count(), 24);
    assert_eq!(opened.language(), Language::English);
    assert_eq!(opened.entropy().expose().as_bytes().len(), 32);

    // It reads back through Load a key, to the same key.
    h.go_home();
    h.open_key(1);
    h.tap(ids::KEY_FORGET);
    h.hold(ids::DETAIL_FORGET);
    h.open_keys();
    assert_eq!(h.app.fingerprints().len(), 1);

    h.open_add(ids::KEYS_LOAD);
    h.choose(ids::LOAD_SOURCE_BACKUP, ids::LOAD_SOURCE_CONTINUE);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes,
    });
    h.type_text(PASSPHRASE);
    h.key(Key::Enter);
    assert_eq!(h.app.load_step(), Some(LoadStep::Checksum));
    h.finish_load(None);
    h.open_keys();
    assert_eq!(h.app.fingerprints(), before);
}

/// A lock keeps the key and an unlock brings it back; a wipe takes it.
#[test]
fn a_member_is_locked_wiped_and_left_the_way_a_key_is() {
    let mut h = device();
    h.start_load_24(&words(SHARE_0));
    h.finish_load(None);
    h.go_home();
    assert!(
        h.app.rect_of(ids::STATUS_LOCK).is_some(),
        "there is something to lock"
    );
    h.tap(ids::STATUS_LOCK);
    assert_eq!(h.app.screen(), ScreenKind::Lock);
    h.unlock(PIN);
    h.open_keys();
    assert_eq!(h.app.fingerprints().len(), 2, "the keys came back");

    h.go_home();
    h.tap(ids::at(ids::HOME_TILE_BASE, 5));
    h.tap(ids::SETTINGS_WIPE_ROW);
    h.hold(ids::SETTINGS_WIPE);
    h.open_keys();
    assert!(h.app.fingerprints().is_empty(), "a wipe takes the keys");

    // An exit takes them too, and leaves the terminal screen.
    let mut h = device();
    h.start_load_24(&words(SHARE_0));
    h.finish_load(None);
    h.go_home();
    h.tap(ids::at(ids::HOME_TILE_BASE, 5));
    h.tap(ids::SETTINGS_EXIT_ROW);
    h.hold(ids::SETTINGS_EXIT);
    assert_eq!(h.app.screen(), ScreenKind::Ended);
    assert!(h.app.fingerprints().is_empty(), "nothing is left in memory");
}

/// A fresh device has nothing loaded; Add a key and Add a wallet are
/// still reachable, because neither needs a key first.
#[test]
fn add_a_key_and_add_a_wallet_are_reachable_with_nothing_loaded() {
    let mut h = Harness::new(PANEL);
    h.go_home();
    assert_eq!(h.app.screen(), ScreenKind::Home);
    h.open_keys();
    h.tap(ids::KEYS_ADD);
    assert_eq!(h.app.screen(), ScreenKind::Add);
    h.tap(ids::KEYS_LOAD);
    assert_eq!(h.app.screen(), ScreenKind::Load);

    h.go_home();
    h.open_wallets();
    h.tap(ids::WALLETS_ADD);
    assert_eq!(h.app.screen(), ScreenKind::AddWallet);
    assert!(
        h.app.texts().iter().any(|t| t == strings::EN.build_new),
        "{:?}",
        h.app.texts()
    );
    h.tap(ids::BUILD_NEW);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.wallet_threshold),
        "the kinds include FROST: {:?}",
        h.app.texts()
    );
}

/// A passphrase key is made from the same words as its parent, so it
/// is not a second copy of the parent's membership in a FROST group:
/// the wallet's Keys review marks the plain key and leaves the other
/// two members not loaded, whatever a passphrase key would say.
#[test]
fn a_passphrase_key_is_not_a_member_of_its_parents_group() {
    let mut h = device();
    use_record(&mut h);
    h.start_load_24(&words(SHARE_1));
    h.finish_load(None);
    h.go_home();
    let before = h.app.fingerprints().len();
    h.open_passphrase(before - 1);
    h.type_text("decoy");
    h.key(Key::Enter);
    h.drain();
    assert_eq!(
        h.app.fingerprints().len(),
        before + 1,
        "the member's passphrase key is one more key"
    );

    h.go_home();
    h.open_wallets();
    h.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0));
    h.tap(ids::WALLET_KEYS);
    let texts = h.app.texts();
    assert_eq!(
        texts
            .iter()
            .filter(|t| *t == strings::EN.key_not_loaded)
            .count(),
        2,
        "two members are not loaded: {texts:?}"
    );
}
