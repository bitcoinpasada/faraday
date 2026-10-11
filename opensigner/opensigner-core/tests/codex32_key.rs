//! Loading a key from a codex32 string (`docs/PLANNING.md` §16.109), on
//! the vectors BIP 93 publishes.
//!
//! Every string here is typed character by character through Load a key
//! › Codex32, on the keyboard the screen draws, as a person types it;
//! the key that comes out is checked against the BIP-32 root key the
//! vector states.

mod common;

use common::{Element, Harness, PANEL, SECURE_PHONE};
use opensigner_core::load::Step;
use opensigner_core::strings::EN;
use opensigner_core::{ScreenKind, ids};
use osk_bip::keys::{Fingerprint, MasterKey};
use osk_shell_api::Key;
use osk_ui::widgets::keyboard::KeyInput;
use serde_json::Value;

const VECTORS: &str = include_str!("../../../tools/vectors/bip93/vectors.json");

fn vectors() -> Value {
    serde_json::from_str(VECTORS).unwrap()
}

fn text(v: &Value) -> String {
    v.as_str().unwrap().to_string()
}

/// The unshared secret of BIP 93 vector `number`, and its root key.
fn secret(number: &str) -> (String, String) {
    let parsed = vectors();
    let case = parsed["secrets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["vector"] == number)
        .unwrap_or_else(|| panic!("no secret vector {number}"));
    (text(&case["string"]), text(&case["xprv"]))
}

/// One share set of BIP 93: the shares it publishes, the shares it
/// derives, and the root key the secret gives.
struct Set {
    shares: Vec<String>,
    derived: Vec<String>,
    xprv: String,
}

fn set(number: &str) -> Set {
    let parsed = vectors();
    let case = parsed["sets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["vector"] == number)
        .unwrap_or_else(|| panic!("no share set {number}"))
        .clone();
    Set {
        shares: case["shares"]
            .as_array()
            .unwrap()
            .iter()
            .map(text)
            .collect(),
        derived: case["derived"]
            .as_object()
            .unwrap()
            .values()
            .map(text)
            .collect(),
        xprv: text(&case["xprv"]),
    }
}

/// The other encodings BIP 93 lists of one master seed.
fn alternates(number: &str) -> Vec<String> {
    let parsed = vectors();
    parsed["alternates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["vector"] == number)
        .unwrap_or_else(|| panic!("no alternates for {number}"))["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(text)
        .collect()
}

/// The master fingerprint of a vector's root key.
fn root_fingerprint(xprv: &str) -> Fingerprint {
    MasterKey::decode(xprv)
        .expect("a master xprv")
        .fingerprint()
}

fn hex(fp: Fingerprint) -> String {
    fp.to_hex().iter().map(|b| *b as char).collect()
}

fn on_screen(h: &Harness) -> Vec<String> {
    h.app.texts()
}

fn shows(h: &Harness, t: &str) -> bool {
    on_screen(h).iter().any(|x| x == t)
}

/// Types a string without pressing ✓, which is how a test looks at the
/// entry while it is still being typed.
fn type_only(h: &mut Harness, string: &str) {
    let lower = string.to_ascii_lowercase();
    for c in lower.strip_prefix("ms1").expect("a codex32 string").chars() {
        h.pad(ids::LOAD_KEYBOARD, KeyInput::Char(c));
    }
}

/// The confirmation onwards: the key is added and the session takes its
/// PIN, as every other wizard ends.
fn add(h: &mut Harness) {
    assert_eq!(h.app.load_step(), Some(Step::Confirm), "{:?}", on_screen(h));
    h.add_key();
    if h.app.screen() == ScreenKind::Keep {
        h.tap(ids::BACK);
    }
}

/// A codex32 secret typed whole is the key the vector states, with no
/// passphrase asked for: BIP 93 has none.
#[test]
fn a_typed_secret_is_the_key_the_vector_states() {
    for number in ["1", "5"] {
        let (string, xprv) = secret(number);
        let mut h = Harness::new(PANEL);
        h.open_codex32();
        h.type_codex32(&string);
        add(&mut h);
        assert_eq!(
            h.app.fingerprints(),
            vec![root_fingerprint(&xprv)],
            "vector {number}"
        );
    }
}

/// Two shares of a 2-of-n set: the first is accepted and the result says
/// one of two is in, the second completes the set, and the key is the
/// one the vector's secret states.
#[test]
fn two_shares_give_the_key_the_set_states() {
    let s = set("2");
    let mut h = Harness::new(PANEL);
    h.open_codex32();

    h.type_codex32(&s.shares[0]);
    assert!(
        shows(&h, EN.load_share_accepted_title),
        "the share was refused: {:?}",
        on_screen(&h)
    );
    assert!(shows(&h, "1 of 2 needed"), "{:?}", on_screen(&h));
    assert!(shows(&h, EN.load_share_more), "{:?}", on_screen(&h));

    h.tap(ids::LOAD_CONTINUE);
    assert_eq!(h.app.load_step(), Some(Step::Words));
    assert!(shows(&h, "Share 2"), "{:?}", on_screen(&h));

    h.type_codex32(&s.shares[1]);
    assert!(shows(&h, "2 of 2 needed"), "{:?}", on_screen(&h));
    assert!(shows(&h, EN.load_share_enough), "{:?}", on_screen(&h));
    h.tap(ids::LOAD_CONTINUE);
    add(&mut h);
    assert_eq!(h.app.fingerprints(), vec![root_fingerprint(&s.xprv)]);
}

/// A 3-of-n set: two shares are not enough and Continue keeps asking,
/// and the third opens the key.
#[test]
fn three_of_a_set_are_needed_and_two_keep_asking() {
    let s = set("3");
    let mut h = Harness::new(PANEL);
    h.open_codex32();

    h.type_codex32(&s.shares[0]);
    h.tap(ids::LOAD_CONTINUE);
    h.type_codex32(&s.shares[1]);
    assert!(shows(&h, "2 of 3 needed"), "{:?}", on_screen(&h));
    h.tap(ids::LOAD_CONTINUE);
    assert_eq!(
        h.app.load_step(),
        Some(Step::Words),
        "two shares of three went on to the key"
    );

    h.type_codex32(&s.derived[0]);
    assert!(shows(&h, "3 of 3 needed"), "{:?}", on_screen(&h));
    h.tap(ids::LOAD_CONTINUE);
    add(&mut h);
    assert_eq!(h.app.fingerprints(), vec![root_fingerprint(&s.xprv)]);
}

/// A share of another set is refused by name, and so is the same share
/// twice.
#[test]
fn a_share_of_another_set_and_the_same_share_twice_are_refused() {
    let first = set("2");
    let other = set("3");
    let mut h = Harness::new(PANEL);
    h.open_codex32();
    h.type_codex32(&first.shares[0]);
    assert!(shows(&h, EN.load_share_accepted_title));
    h.tap(ids::LOAD_CONTINUE);

    h.type_codex32(&other.shares[0]);
    assert!(
        shows(&h, EN.load_share_refused_title),
        "{:?}",
        on_screen(&h)
    );
    assert!(
        shows(&h, "identifiers differ across the set"),
        "the refusal does not say why: {:?}",
        on_screen(&h)
    );

    // The chevron comes back to the string it was about, and the share
    // already in is kept.
    h.tap(ids::BACK);
    assert_eq!(h.app.load_step(), Some(Step::Words));
    h.type_codex32(&first.shares[0]);
    assert!(
        shows(&h, "two strings carry the same share index"),
        "{:?}",
        on_screen(&h)
    );
}

/// One character wrong: the string is a length BIP 93 allows, the line
/// under the field states the codec's reason, and ✓ does not answer.
#[test]
fn a_mistyped_string_says_so_and_cannot_be_taken() {
    let (string, _) = secret("1");
    let mut wrong = string.clone();
    let last = wrong.pop().expect("a string");
    wrong.push(if last == 'q' { 'p' } else { 'q' });

    let mut h = Harness::new(PANEL);
    h.open_codex32();
    type_only(&mut h, &wrong);
    assert!(shows(&h, "checksum does not match"), "{:?}", on_screen(&h));
    assert!(
        h.app.key_rect(ids::LOAD_KEYBOARD, KeyInput::Done).is_none(),
        "the string can be taken with a character wrong"
    );
    // Enter does nothing either, and the string stays where it is.
    h.key(Key::Enter);
    assert_eq!(h.app.load_step(), Some(Step::Words));
}

/// The other encodings BIP 93 lists of one master seed differ only in
/// their padding bits, and every one of them is the same key.
#[test]
fn the_alternates_of_a_vector_are_one_key() {
    let s = set("3");
    let expected = root_fingerprint(&s.xprv);
    for string in alternates("3") {
        let mut h = Harness::new(PANEL);
        h.open_codex32();
        h.type_codex32(&string);
        add(&mut h);
        assert_eq!(h.app.fingerprints(), vec![expected], "{string}");
    }
}

/// A scanned secret lands on the confirmation with the vector's key; a
/// scanned share starts the gathering the typed one starts.
#[test]
fn a_scanned_string_reaches_the_same_entry() {
    let (string, xprv) = secret("1");
    let mut h = Harness::new(PANEL);
    h.tap(ids::HOME_SCAN);
    h.scan_bytes(string.as_bytes());
    assert_eq!(h.app.screen(), ScreenKind::Load);
    assert!(
        shows(&h, &hex(root_fingerprint(&xprv))),
        "{:?}",
        on_screen(&h)
    );
    add(&mut h);
    assert_eq!(h.app.fingerprints(), vec![root_fingerprint(&xprv)]);

    let s = set("2");
    let mut h = Harness::new(PANEL);
    h.tap(ids::HOME_SCAN);
    h.scan_bytes(s.shares[0].as_bytes());
    assert_eq!(h.app.screen(), ScreenKind::Load);
    assert!(
        shows(&h, EN.load_share_accepted_title),
        "{:?}",
        on_screen(&h)
    );
    h.tap(ids::LOAD_CONTINUE);
    h.type_codex32(&s.shares[1]);
    h.tap(ids::LOAD_CONTINUE);
    add(&mut h);
    assert_eq!(h.app.fingerprints(), vec![root_fingerprint(&s.xprv)]);
}

/// The key's own page: what it is made of, the backups a seed of its
/// length can be written as, and the BIP-85 child it opens.
#[test]
fn the_key_page_states_codex32_and_offers_the_backups_its_seed_allows() {
    let (string, _) = secret("1");
    let mut h = Harness::new(PANEL);
    h.open_codex32();
    h.type_codex32(&string);
    add(&mut h);

    h.open_key(0);
    assert!(shows(&h, EN.key_made_codex32), "{:?}", on_screen(&h));

    h.tap(ids::DETAIL_BACKUP);
    assert_eq!(h.app.screen(), ScreenKind::BackupMenu);
    let labels = h.app.labels();
    assert!(
        labels.iter().any(|l| l == EN.backup_codex32),
        "codex32 is not on the menu: {labels:?}"
    );
    assert!(
        labels.iter().any(|l| l == EN.backup_slip39),
        "a 16-byte seed can be split as SLIP-39: {labels:?}"
    );
    // §16.112 rule 1: the encrypted container takes a master seed, so a
    // key with no words has an encrypted backup too — of its seed.
    assert!(
        labels.iter().any(|l| l == EN.backup_encrypted),
        "a seed can be sealed under a passphrase: {labels:?}"
    );
    for row in [EN.backup_words, EN.backup_seedqr, EN.backup_verify] {
        assert!(
            !labels.iter().any(|l| l == row),
            "{row} needs words this key does not have: {labels:?}"
        );
    }

    // BIP-85 derives from the master, which a codex32 key has like any
    // other, and the child is twelve words of its own.
    h.go_home();
    h.open_child(0);
    h.choose(
        ids::at(ids::OPEN_CHILD_WORDS_BASE, 0),
        ids::OPEN_CHILD_CONTINUE,
    );
    h.type_pin(ids::OPEN_CHILD_KEYBOARD, "0");
    h.drain();
    assert_eq!(h.app.screen(), ScreenKind::Opened);
    assert_eq!(h.app.fingerprints().len(), 2, "the child was not added");
    assert_eq!(h.app.has_mnemonic(1), Some(true), "with words of its own");
}

/// A wallet from a codex32 key's page opens on the kind: the key has no
/// words, so there is no passphrase to add and "Passphrase?" is not
/// asked (§16.129).
#[test]
fn a_wallet_from_a_codex32_key_opens_on_the_kind() {
    let (string, _) = secret("1");
    let mut h = Harness::new(PANEL);
    h.open_codex32();
    h.type_codex32(&string);
    add(&mut h);

    h.open_key(0);
    h.tap(ids::DETAIL_ADD_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Build);
    assert_eq!(h.app.build_step(), Some(opensigner_core::build::Step::Kind));
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);
}

/// A 512-bit codex32 seed is no SLIP-39 master secret, so the Backup
/// menu offers codex32 alone.
#[test]
fn a_512_bit_seed_has_no_slip39_backup() {
    let (string, _) = secret("5");
    let mut h = Harness::new(PANEL);
    h.open_codex32();
    h.type_codex32(&string);
    add(&mut h);
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    let labels = h.app.labels();
    assert!(labels.iter().any(|l| l == EN.backup_codex32), "{labels:?}");
    assert!(
        !labels.iter().any(|l| l == EN.backup_slip39),
        "a 64-byte seed is offered as SLIP-39 shares: {labels:?}"
    );
}

/// A device that keeps keys keeps a codex32 key as its seed, and a
/// restart brings back the same key, still a codex32 key.
#[test]
fn a_codex32_key_is_kept_and_comes_back() {
    let (string, xprv) = secret("4");
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.open_codex32();
    h.type_codex32(&string);
    add(&mut h);
    let fingerprint = root_fingerprint(&xprv);
    assert_eq!(h.app.fingerprints(), vec![fingerprint]);

    h.open_key(0);
    h.tap(ids::KEEP_ROW);
    assert_eq!(h.app.screen(), ScreenKind::Keep);
    h.hold(ids::KEEP_HOLD);
    assert!(h.app.secret_kept(), "the shell is holding no blob");
    let element = h.element.clone().expect("an element");

    let mut h = Harness::kept(SECURE_PHONE, element);
    assert_eq!(h.app.screen(), ScreenKind::StoredKey);
    h.type_pin(ids::KEEP_PIN_KEYBOARD, common::PIN);
    assert_eq!(h.app.fingerprints(), vec![fingerprint]);
    h.open_key(0);
    assert!(
        shows(&h, EN.key_made_codex32),
        "the key came back as another kind: {:?}",
        on_screen(&h)
    );
}

/// A 512-bit codex32 seed does not fit the room a slot has, so the
/// device does not offer to keep it.
#[test]
fn a_512_bit_key_is_not_kept() {
    let (string, _) = secret("5");
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.open_codex32();
    h.type_codex32(&string);
    add(&mut h);
    h.open_key(0);
    assert!(
        h.app.rect_of(ids::KEEP_ROW).is_none(),
        "a seed the blob cannot hold is offered to the device"
    );
}
