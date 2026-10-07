//! Opening a key from a key that is already loaded (`docs/PLANNING.md`
//! §16.67): the passphrase key of the same words, and a BIP-85 child
//! seed. Both live in memory for the session and neither reaches the
//! device.

mod common;

use common::{ABANDON, Element, Harness, PANEL, PIN, SECURE_PHONE, expected_fingerprint};
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::keys::{Fingerprint, Network};
use osk_shell_api::{Command, Key};
use osk_ui::widgets::keyboard::KeyInput;

/// The fingerprint as the app writes it on a screen: eight hex
/// characters.
fn hex(fp: Fingerprint) -> String {
    String::from_utf8(fp.to_hex().to_vec()).expect("ascii hex")
}

/// Whether the screen states `fp` as a value.
fn shows(h: &Harness, fp: Fingerprint) -> bool {
    h.app.texts().contains(&hex(fp))
}

/// Keeps the loaded key on the device and starts a fresh command window.
fn keep_and_forget_commands(h: &mut Harness) {
    h.open_key(0);
    h.tap(ids::KEEP_ROW);
    assert_eq!(h.app.screen(), ScreenKind::Keep);
    h.hold(ids::KEEP_HOLD);
    assert!(h.app.secret_kept(), "the shell is holding the blob");
    h.drain();
    h.seen.clear();
}

/// Whether the shell was asked to write a blob since the last clear.
fn wrote_a_blob(h: &Harness) -> bool {
    h.seen
        .iter()
        .any(|c| matches!(c, Command::StoreSecret { .. }))
}

/// The PIN pad a device that keeps a key opens on.
fn open_stored(h: &mut Harness, digits: &str) {
    assert_eq!(h.app.screen(), ScreenKind::StoredKey);
    h.type_pin(ids::KEEP_PIN_KEYBOARD, digits);
}

/// A key's page is where that key opens another key: the passphrase row
/// stands there on a key with words (`docs/PLANNING.md` §16.127 rule 3).
#[test]
fn a_key_page_offers_the_ways_to_open_a_key() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);

    let texts = h.app.texts();
    for want in [s.detail_open_passphrase, s.key_bip85_row] {
        assert!(texts.iter().any(|t| t == want), "{want:?} in {texts:?}");
    }
    assert!(h.app.rect_of(ids::DETAIL_OPEN_PASSPHRASE).is_some());
    assert!(h.app.rect_of(ids::DETAIL_BIP85).is_some());
}

/// The words are already here, so only the passphrase is typed.
#[test]
fn a_passphrase_key_is_opened_from_the_words_already_loaded() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    let parent = h.app.fingerprints()[0];

    h.open_passphrase(0);
    assert_eq!(h.app.screen(), ScreenKind::OpenPassphrase);

    // ✓ with nothing typed is the key that is already loaded, so it
    // does nothing.
    h.key(Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::OpenPassphrase);
    assert_eq!(h.app.fingerprints(), vec![parent]);

    h.type_text("passphrase");
    h.key(Key::Enter);

    let opened = expected_fingerprint(b"passphrase", Network::Mainnet);
    assert_eq!(h.app.screen(), ScreenKind::Opened, "which key was added");
    assert!(
        shows(&h, opened),
        "the key that was added: {:?}",
        h.app.texts()
    );
    assert_eq!(
        h.app.fingerprints(),
        vec![parent, opened],
        "the parent is still there, and the passphrase key is beside it"
    );
    assert_eq!(h.app.has_passphrase(1), Some(true));
    assert_eq!(h.app.has_mnemonic(1), Some(true), "it carries the words");

    // The action opens the wallet of the key that was added.
    h.tap(ids::OPENED_OPEN);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t.contains(&hex(opened))),
        "the opened key's own page, titled with it: {texts:?}"
    );

    // The same passphrase again is the key that is already here.
    h.open_passphrase(0);
    h.type_text("passphrase");
    h.key(Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Opened);
    assert_eq!(h.app.fingerprints().len(), 2, "nothing was added twice");

    // The chevron from the Result goes back to the key list: the page
    // the flow started on is replaced, and the key stays added either
    // way.
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::Keys);
}

/// A device that keeps keys keeps the words it was given, and not the
/// passphrase key opened over them.
#[test]
fn an_opened_passphrase_key_is_not_written_to_the_device() {
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.start_load(&ABANDON);
    h.finish_load(None);
    let parent = h.app.fingerprints()[0];
    keep_and_forget_commands(&mut h);

    h.open_passphrase(0);
    h.type_text("swordfish");
    h.key(Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Opened);
    h.drain();
    assert_eq!(
        h.app.fingerprints(),
        vec![parent, expected_fingerprint(b"swordfish", Network::Mainnet)]
    );
    assert!(!wrote_a_blob(&h), "opening a key wrote to the device");

    let element = h.element.clone().expect("an element");
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    assert_eq!(
        h.app.fingerprints(),
        vec![parent],
        "only the words came back"
    );
}

/// A BIP-85 child of the loaded key is a key of its own, with words of
/// its own, and the device keeps none of it.
#[test]
fn a_bip85_child_seed_is_opened_and_is_not_written_to_the_device() {
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.start_load(&ABANDON);
    h.finish_load(None);
    let parent = h.app.fingerprints()[0];
    keep_and_forget_commands(&mut h);

    h.open_child(0);
    h.choose(
        ids::at(ids::OPEN_CHILD_WORDS_BASE, 0),
        ids::OPEN_CHILD_CONTINUE,
    );
    assert_eq!(h.app.screen(), ScreenKind::OpenChild, "now the index");
    h.type_pin(ids::OPEN_CHILD_KEYBOARD, "0");
    h.drain();

    assert_eq!(h.app.screen(), ScreenKind::Opened);
    let keys = h.app.fingerprints();
    assert_eq!(keys.len(), 2, "the parent and the child");
    assert_ne!(keys[1], parent, "a key of its own");
    assert_eq!(h.app.has_mnemonic(1), Some(true), "with words of its own");
    assert_eq!(h.app.has_passphrase(1), Some(false));
    assert!(shows(&h, keys[1]), "the child that was added");
    assert!(!wrote_a_blob(&h), "opening a child wrote to the device");
    h.tap(ids::OPENED_OPEN);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);

    let element = h.element.clone().expect("an element");
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    assert_eq!(
        h.app.fingerprints(),
        vec![parent],
        "only the parent's words came back"
    );
}

/// An index BIP-85 has no child at is refused where every other entry
/// screen refuses what is typed: the caption line under the field.
#[test]
fn an_index_beyond_the_last_hardened_child_is_refused() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_child(0);
    h.choose(
        ids::at(ids::OPEN_CHILD_WORDS_BASE, 0),
        ids::OPEN_CHILD_CONTINUE,
    );

    for c in "2147483648".chars() {
        h.pad(ids::OPEN_CHILD_KEYBOARD, KeyInput::Char(c));
    }
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.open_child_index_range),
        "the caption states the range: {texts:?}"
    );
    assert!(
        h.app
            .key_rect(ids::OPEN_CHILD_KEYBOARD, KeyInput::Done)
            .is_none(),
        "the check is dead while the index is out of range"
    );

    // One digit fewer and it is a child the key has.
    h.pad(ids::OPEN_CHILD_KEYBOARD, KeyInput::Backspace);
    h.pad(ids::OPEN_CHILD_KEYBOARD, KeyInput::Done);
    assert_eq!(h.app.screen(), ScreenKind::Opened);
    assert_eq!(h.app.fingerprints().len(), 2);
}

/// ✓ on "Open passphrase" is dead until something is typed: an empty
/// passphrase is the key that is already loaded.
#[test]
fn the_check_is_dead_until_a_passphrase_is_typed() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_passphrase(0);
    assert!(
        h.app
            .key_rect(ids::OPEN_PASS_KEYBOARD, KeyInput::Done)
            .is_none(),
        "the check is live with nothing typed"
    );
    h.type_text("a");
    assert!(
        h.app
            .key_rect(ids::OPEN_PASS_KEYBOARD, KeyInput::Done)
            .is_some(),
        "the check is dead with a passphrase typed"
    );
}

/// The passphrase screen states the key the passphrase opens while it is
/// typed, so the person sees which key ✓ will add before adding it. With
/// nothing typed there is no key: an empty passphrase is the key that is
/// already loaded.
#[test]
fn the_passphrase_screen_states_the_key_as_it_is_typed() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    let parent = h.app.fingerprints()[0];

    h.open_passphrase(0);
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.value_none),
        "nothing typed names no key: {texts:?}"
    );
    assert!(
        !shows(&h, parent),
        "an empty passphrase is not a key to add"
    );

    h.type_text("passphrase");
    assert!(
        shows(&h, expected_fingerprint(b"passphrase", Network::Mainnet)),
        "the key this passphrase opens: {:?}",
        h.app.texts()
    );

    // Deleting back to nothing takes the key away again.
    for _ in 0.."passphrase".len() {
        h.pad(ids::OPEN_PASS_KEYBOARD, KeyInput::Backspace);
    }
    assert!(h.app.texts().iter().any(|t| t == s.value_none));
}

/// The child's index screen states the child the index derives as it is
/// typed, and states no key while the index is out of range.
#[test]
fn the_child_index_screen_states_the_child_as_it_is_typed() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);

    h.open_child(0);
    h.choose(
        ids::at(ids::OPEN_CHILD_WORDS_BASE, 0),
        ids::OPEN_CHILD_CONTINUE,
    );
    assert!(
        h.app.texts().iter().any(|t| t == s.value_none),
        "nothing typed names no child"
    );

    h.pad(ids::OPEN_CHILD_KEYBOARD, KeyInput::Char('0'));
    let preview: Vec<String> = h.app.texts();

    // An index the key has no child at names no child either, and the
    // caption still states the range.
    h.pad(ids::OPEN_CHILD_KEYBOARD, KeyInput::Backspace);
    for c in "2147483648".chars() {
        h.pad(ids::OPEN_CHILD_KEYBOARD, KeyInput::Char(c));
    }
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == s.value_none),
        "an index out of range names no child: {texts:?}"
    );
    assert!(texts.iter().any(|t| t == s.open_child_index_range));

    // Back to index 0, and the child that was previewed is the child
    // that is added.
    for _ in 0.."2147483648".len() {
        h.pad(ids::OPEN_CHILD_KEYBOARD, KeyInput::Backspace);
    }
    h.pad(ids::OPEN_CHILD_KEYBOARD, KeyInput::Char('0'));
    h.pad(ids::OPEN_CHILD_KEYBOARD, KeyInput::Done);
    assert_eq!(h.app.screen(), ScreenKind::Opened);
    let child = h.app.fingerprints()[1];
    assert!(
        preview.iter().any(|t| *t == hex(child)),
        "the index named the child it added: {preview:?}"
    );
}

// ----- A key made from a wallet's not-loaded row (§16.104 rule 6) -----

/// Other words, for the key a person loads instead of the one the
/// wallet asked for.
const ZOO: [&str; 12] = [
    "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "wrong",
];

/// The passphrase the wallet's key was opened with.
const PASSPHRASE: &str = "swordfish";

/// A single-sig wallet over a passphrase key, with that key forgotten
/// again: what a person has after a restart, and what puts them on the
/// wallet's Keys review with a member this device has no key for. Ends
/// on Add a key, opened from that row. Answers the fingerprint of the
/// key the wallet names.
fn a_wallet_whose_key_is_gone(h: &mut Harness) -> Fingerprint {
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_passphrase(0);
    h.type_text(PASSPHRASE);
    h.key(Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Opened);
    let wanted = h.app.fingerprints()[1];

    h.add_single_sig(1, 2);
    h.open_key(1);
    h.tap(ids::KEY_FORGET);
    h.hold(ids::DETAIL_FORGET);
    assert_eq!(h.app.fingerprints().len(), 1, "only the words are left");

    h.open_policy(0);
    h.tap(ids::WALLET_KEYS);
    assert_eq!(h.app.screen(), ScreenKind::WalletKeys);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.key_not_loaded),
        "the member this device has no key for"
    );
    h.tap(ids::at(ids::WALLET_KEY_ROW_BASE, 0));
    assert_eq!(h.app.screen(), ScreenKind::Add, "the row opens Add a key");
    wanted
}

/// The one way a wrong key arrives silently: a passphrase typed from a
/// wallet's row that opens some other key. It is named and not kept; the
/// right one loads and the review says so.
#[test]
fn a_passphrase_typed_from_a_wallets_row_must_give_that_wallets_key() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    let wanted = a_wallet_whose_key_is_gone(&mut h);

    // The wallet is asking for one key, so Add a key offers the two
    // ways one key opens another and asks which key to start from.
    h.tap(ids::ADD_OPEN_PASSPHRASE);
    h.tap(ids::at(ids::PICK_KEY_BASE, 0));
    assert_eq!(h.app.screen(), ScreenKind::OpenPassphrase);
    h.type_text("winter");
    h.key(Key::Enter);
    assert_eq!(
        h.app.screen(),
        ScreenKind::OpenPassphrase,
        "the wrong key is not added"
    );
    assert_eq!(h.app.fingerprints().len(), 1, "and Keys has no new key");
    let gave = expected_fingerprint(b"winter", Network::Mainnet);
    let statement = strings::fill(s.open_wrong_passphrase, &[&hex(gave), &hex(wanted)]);
    assert!(
        h.app.texts().contains(&statement),
        "both fingerprints, as a statement: {:?}",
        h.app.texts()
    );

    // Typing again clears it, and the right passphrase loads the key
    // the wallet named.
    for _ in 0.."winter".len() {
        h.pad(ids::OPEN_PASS_KEYBOARD, KeyInput::Backspace);
    }
    assert!(!h.app.texts().contains(&statement), "the statement is gone");
    h.type_text(PASSPHRASE);
    h.key(Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Opened);
    assert_eq!(h.app.fingerprints()[1], wanted);

    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::WalletKeys, "back at the review");
    assert!(
        !h.app.texts().iter().any(|t| t == s.key_not_loaded),
        "and the member is loaded: {:?}",
        h.app.texts()
    );
}

/// The same check on the other derived key: a child of the loaded words
/// is not the key the wallet named, so it is not kept.
#[test]
fn a_child_opened_from_a_wallets_row_must_be_that_wallets_key() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    let wanted = a_wallet_whose_key_is_gone(&mut h);

    h.tap(ids::ADD_OPEN_CHILD);
    h.tap(ids::at(ids::PICK_KEY_BASE, 0));
    assert_eq!(h.app.screen(), ScreenKind::OpenChild);
    h.choose(
        ids::at(ids::OPEN_CHILD_WORDS_BASE, 0),
        ids::OPEN_CHILD_CONTINUE,
    );
    h.type_pin(ids::OPEN_CHILD_KEYBOARD, "0");
    assert_eq!(
        h.app.screen(),
        ScreenKind::OpenChild,
        "the child is not this wallet's key"
    );
    assert_eq!(h.app.fingerprints().len(), 1, "and Keys has no new key");
    let texts = h.app.texts();
    let statement = texts
        .iter()
        .find(|t| t.starts_with(s.open_wrong_child.split('{').next().expect("a prefix")))
        .unwrap_or_else(|| panic!("the statement, in {texts:?}"));
    assert!(
        statement.contains(&hex(wanted)),
        "it names the fingerprint the wallet asked for: {statement}"
    );
}

/// Words are words: a key loaded from a wallet's row is loaded whatever
/// it is, because a person may be loading a different key on purpose.
/// The review simply says whether the member is loaded now.
#[test]
fn a_key_loaded_from_a_wallets_row_is_loaded_whatever_it_is() {
    let s = &strings::EN;
    let mut h = Harness::new(PANEL);
    a_wallet_whose_key_is_gone(&mut h);

    h.start_load(&ZOO);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    assert_eq!(
        h.app.screen(),
        ScreenKind::WalletKeys,
        "the review is where the flow started"
    );
    assert_eq!(h.app.fingerprints().len(), 2, "the key is loaded");
    assert!(
        h.app.texts().iter().any(|t| t == s.key_not_loaded),
        "and the member is still not loaded: {:?}",
        h.app.texts()
    );
}

/// From Keys, no wallet asked: a passphrase key is a key.
#[test]
fn a_passphrase_key_opened_from_keys_is_never_refused() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_passphrase(0);
    h.type_text("winter");
    h.key(Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Opened);
    assert_eq!(
        h.app.fingerprints()[1],
        expected_fingerprint(b"winter", Network::Mainnet)
    );
}

/// Leaving a wallet's row for Keys ends what the wallet asked: the next
/// key is made from Keys, where nothing is checked.
#[test]
fn a_key_made_from_keys_after_leaving_a_wallets_row_is_never_refused() {
    let mut h = Harness::new(PANEL);
    a_wallet_whose_key_is_gone(&mut h);
    h.go_home();

    h.open_passphrase(0);
    h.type_text("winter");
    h.key(Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Opened, "a key is a key");
    assert_eq!(
        h.app.fingerprints()[1],
        expected_fingerprint(b"winter", Network::Mainnet)
    );
}
