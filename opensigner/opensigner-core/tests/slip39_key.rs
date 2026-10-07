//! Loading a key from SLIP-39 shares (`docs/PLANNING.md` §16.107), on
//! the vectors Trezor publishes with `python-shamir-mnemonic`.
//!
//! Every share here is typed word by word through Load a key ›
//! SLIP-39 shares, as a person types it, and the key that comes out is
//! checked against the BIP-32 root key the vector states. The vectors'
//! root keys are under the passphrase `TREZOR`.

mod common;

use common::{Element, Harness, PANEL, SECURE_PHONE};
use opensigner_core::strings::EN;
use opensigner_core::{ScreenKind, ids};
use osk_bip::keys::{Fingerprint, MasterKey};
use serde_json::Value;

const VECTORS: &str = include_str!("../../../tools/vectors/slip39/vectors.json");
const PASSPHRASE: &str = "TREZOR";

/// A case of the vector file: its shares and the root key they give.
struct Vector {
    shares: Vec<String>,
    xprv: String,
}

/// The case whose description starts with `number.`, which is how the
/// file numbers them.
fn vector(number: &str) -> Vector {
    let parsed: Value = serde_json::from_str(VECTORS).unwrap();
    let case = parsed
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c[0].as_str().unwrap().starts_with(&alloc_format(number)))
        .unwrap_or_else(|| panic!("no vector {number}"));
    Vector {
        shares: case[1]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m.as_str().unwrap().to_string())
            .collect(),
        xprv: case[3].as_str().unwrap().to_string(),
    }
}

fn alloc_format(number: &str) -> String {
    format!("{number}.")
}

/// The master fingerprint of a vector's root key.
fn root_fingerprint(xprv: &str) -> Fingerprint {
    MasterKey::decode(xprv)
        .expect("a master xprv")
        .fingerprint()
}

/// The result line and rows now on screen.
fn on_screen(h: &Harness) -> Vec<String> {
    h.app.texts()
}

fn shows(h: &Harness, text: &str) -> bool {
    on_screen(h).iter().any(|t| t == text)
}

/// Two shares of a 2-of-3 backup, typed one after another, give the key
/// the vector states once its passphrase is typed. The result after the
/// first share says the group still wants one more.
#[test]
fn two_shares_and_a_passphrase_give_the_key_the_vector_states() {
    let v = vector("4");
    let mut h = Harness::new(PANEL);
    h.open_slip39(0);

    h.type_share(&v.shares[0]);
    assert_eq!(h.app.screen(), ScreenKind::Load);
    assert!(
        shows(&h, EN.load_share_accepted_title),
        "the share was refused: {:?}",
        on_screen(&h)
    );
    assert!(
        shows(&h, "1 of 2 shares"),
        "the group's count is not on the result: {:?}",
        on_screen(&h)
    );
    assert!(shows(&h, EN.load_share_more), "{:?}", on_screen(&h));

    // Not enough yet, so Continue asks for the next share.
    h.tap(ids::LOAD_CONTINUE);
    assert_eq!(
        h.app.load_step(),
        Some(opensigner_core::load::Step::Words),
        "Continue did not ask for the second share"
    );
    assert!(
        shows(&h, "Share 2 \u{00b7} Word 1 of 20"),
        "{:?}",
        on_screen(&h)
    );

    h.type_share(&v.shares[1]);
    assert!(shows(&h, "2 of 2 shares"), "{:?}", on_screen(&h));
    assert!(shows(&h, EN.load_share_enough), "{:?}", on_screen(&h));

    // The vectors' root key is the one under TREZOR, so the two
    // fingerprints "Which key?" compares are different keys and the
    // vector's is the one with the passphrase.
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_ADD_PASSPHRASE, ids::LOAD_PASS_CONTINUE);
    h.type_text(PASSPHRASE);
    h.key(osk_shell_api::Key::Enter);
    assert_eq!(
        h.app.load_step(),
        Some(opensigner_core::load::Step::PassphraseConfirm)
    );
    let expected = root_fingerprint(&v.xprv);
    let hex: String = expected.to_hex().iter().map(|b| *b as char).collect();
    assert!(
        shows(&h, &hex),
        "the passphrase key's fingerprint is not on the comparison: {:?}",
        on_screen(&h)
    );
    h.tap(ids::LOAD_WHICH_CONTINUE);
    h.add_key();
    if h.app.screen() == ScreenKind::Keep {
        h.tap(ids::BACK);
    }
    assert_eq!(h.app.fingerprints(), vec![expected]);
}

/// A backup of four groups, two of which must be present: the result
/// counts each group up as its shares arrive, and the key is the
/// vector's.
#[test]
fn a_backup_in_groups_counts_each_group_up() {
    let v = vector("17");
    let mut h = Harness::new(PANEL);
    h.open_slip39(0);
    for (i, share) in v.shares.iter().enumerate() {
        h.type_share(share);
        assert!(
            shows(&h, EN.load_share_accepted_title),
            "share {} was refused: {:?}",
            i + 1,
            on_screen(&h)
        );
        h.tap(ids::LOAD_CONTINUE);
    }
    // The last Continue left the words step only if more were wanted.
    assert_eq!(
        h.app.load_step(),
        Some(opensigner_core::load::Step::PassphraseOffer),
        "the set was not enough"
    );
    h.finish_passphrase(Some(PASSPHRASE));
    assert_eq!(h.app.fingerprints(), vec![root_fingerprint(&v.xprv)]);
}

/// The same backup one group short: every share is taken, and Continue
/// keeps asking for another.
#[test]
fn a_group_short_keeps_asking() {
    let v = vector("16");
    let mut h = Harness::new(PANEL);
    h.open_slip39(0);
    for share in &v.shares {
        h.type_share(share);
        assert!(
            shows(&h, EN.load_share_accepted_title),
            "{:?}",
            on_screen(&h)
        );
        h.tap(ids::LOAD_CONTINUE);
    }
    assert_eq!(
        h.app.load_step(),
        Some(opensigner_core::load::Step::Words),
        "an incomplete set went on to the passphrase"
    );
}

/// A share from another backup is refused by name, the chevron comes
/// back to that share's own words, and the share already in is kept.
#[test]
fn a_share_from_another_backup_is_refused_and_the_first_is_kept() {
    let v = vector("6");
    let mut h = Harness::new(PANEL);
    h.open_slip39(0);
    h.type_share(&v.shares[0]);
    assert!(shows(&h, EN.load_share_accepted_title));
    h.tap(ids::LOAD_CONTINUE);

    h.type_share(&v.shares[1]);
    assert!(
        shows(&h, EN.load_share_refused_title),
        "{:?}",
        on_screen(&h)
    );
    assert!(
        shows(&h, "the shares are from different backups"),
        "the refusal does not say why: {:?}",
        on_screen(&h)
    );

    h.tap(ids::BACK);
    assert_eq!(h.app.load_step(), Some(opensigner_core::load::Step::Words));
    assert!(
        shows(&h, "Share 2 \u{00b7} Word 1 of 20"),
        "the first share was given back too: {:?}",
        on_screen(&h)
    );
}

/// A mistyped share fails its checksum and the result says so.
#[test]
fn a_share_with_a_bad_checksum_is_refused() {
    let v = vector("2");
    let mut h = Harness::new(PANEL);
    h.open_slip39(0);
    h.type_share(&v.shares[0]);
    assert!(
        shows(&h, EN.load_share_refused_title),
        "{:?}",
        on_screen(&h)
    );
    assert!(shows(&h, "checksum does not match"), "{:?}", on_screen(&h));
}

/// The same share twice is one share, and the second is refused.
#[test]
fn the_same_share_twice_is_refused() {
    let v = vector("4");
    let mut h = Harness::new(PANEL);
    h.open_slip39(0);
    h.type_share(&v.shares[0]);
    h.tap(ids::LOAD_CONTINUE);
    h.type_share(&v.shares[0]);
    assert!(
        shows(&h, EN.load_share_refused_title),
        "{:?}",
        on_screen(&h)
    );
    assert!(
        shows(&h, "two shares of one group are the same share"),
        "{:?}",
        on_screen(&h)
    );
}

/// A 256-bit backup is 33-word shares, taken through the 33 row of the
/// count step.
#[test]
fn a_thirty_three_word_share_gives_a_key() {
    let v = vector("23");
    let mut h = Harness::new(PANEL);
    h.open_slip39(1);
    h.type_share(&v.shares[0]);
    h.tap(ids::LOAD_CONTINUE);
    h.type_share(&v.shares[1]);
    h.tap(ids::LOAD_CONTINUE);
    h.finish_passphrase(Some(PASSPHRASE));
    assert_eq!(h.app.fingerprints(), vec![root_fingerprint(&v.xprv)]);
}

/// A loaded SLIP-39 key on its own page: what it is made of, the one
/// backup it has, and the BIP-85 child it opens.
#[test]
fn the_key_page_states_the_shares_and_offers_only_their_backup() {
    let v = vector("4");
    let mut h = Harness::new(PANEL);
    h.open_slip39(0);
    h.type_share(&v.shares[0]);
    h.tap(ids::LOAD_CONTINUE);
    h.type_share(&v.shares[1]);
    h.tap(ids::LOAD_CONTINUE);
    h.finish_passphrase(None);

    h.open_key(0);
    assert!(
        shows(&h, EN.key_made_slip39),
        "the key page does not say what it is made of: {:?}",
        on_screen(&h)
    );

    h.tap(ids::DETAIL_BACKUP);
    assert_eq!(h.app.screen(), ScreenKind::BackupMenu);
    let labels = h.app.labels();
    assert!(
        labels.iter().any(|l| l == EN.backup_slip39),
        "the shares are not on the menu: {labels:?}"
    );
    // §16.112 rule 1: the encrypted container takes a master seed, so a
    // key with no words has an encrypted backup too — of its seed.
    assert!(
        labels.iter().any(|l| l == EN.backup_encrypted),
        "a master secret can be sealed under a passphrase: {labels:?}"
    );
    for row in [
        EN.backup_words,
        EN.backup_seedqr,
        EN.backup_compact,
        EN.backup_grid,
        EN.backup_steel,
        EN.xor_split,
        EN.backup_verify,
    ] {
        assert!(
            !labels.iter().any(|l| l == row),
            "{row} needs words this key does not have: {labels:?}"
        );
    }
    assert!(
        h.app.rect_of(ids::BACKUP_WORDS).is_none(),
        "a key with no words offers to show them"
    );

    // BIP-85 derives from the master, which a SLIP-39 key has like any
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
    let keys = h.app.fingerprints();
    assert_eq!(keys.len(), 2, "the child was not added");
    assert_eq!(h.app.has_mnemonic(1), Some(true), "with words of its own");
}

/// The same key read under a passphrase says so on its page.
#[test]
fn a_passphrase_is_named_on_the_key_page() {
    let v = vector("4");
    let mut h = Harness::new(PANEL);
    h.open_slip39(0);
    h.type_share(&v.shares[0]);
    h.tap(ids::LOAD_CONTINUE);
    h.type_share(&v.shares[1]);
    h.tap(ids::LOAD_CONTINUE);
    h.finish_passphrase(Some(PASSPHRASE));
    h.open_key(0);
    assert!(
        shows(&h, EN.key_made_slip39_passphrase),
        "{:?}",
        on_screen(&h)
    );
}

/// A device that keeps keys keeps a SLIP-39 key as its master secret,
/// and a restart brings back the same key.
#[test]
fn a_slip39_key_is_kept_and_comes_back() {
    let v = vector("4");
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.open_slip39(0);
    h.type_share(&v.shares[0]);
    h.tap(ids::LOAD_CONTINUE);
    h.type_share(&v.shares[1]);
    h.tap(ids::LOAD_CONTINUE);
    h.finish_passphrase(None);
    let fingerprint = h.app.fingerprints()[0];

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
}

/// A SLIP-39 key read under a passphrase is never in the blob, so the
/// device has nothing to keep and the hold is dead.
#[test]
fn a_slip39_key_over_a_passphrase_is_not_kept() {
    let v = vector("4");
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.open_slip39(0);
    h.type_share(&v.shares[0]);
    h.tap(ids::LOAD_CONTINUE);
    h.type_share(&v.shares[1]);
    h.tap(ids::LOAD_CONTINUE);
    h.finish_passphrase(Some(PASSPHRASE));
    h.open_key(0);
    assert!(
        h.app.rect_of(ids::KEEP_ROW).is_none(),
        "a passphrase key is offered to the device"
    );
}

/// A key with no words has an encrypted backup of its master seed
/// (`docs/PLANNING.md` §16.112 rule 1), and that file read back on a
/// bare device gives the same key.
#[test]
fn a_slip39_keys_encrypted_backup_reads_back_to_the_same_key() {
    const BACKUP_PASSPHRASE: &str = "a long enough one";
    let v = vector("4");
    let mut h = Harness::new(PANEL);
    h.open_slip39(0);
    h.type_share(&v.shares[0]);
    h.tap(ids::LOAD_CONTINUE);
    h.type_share(&v.shares[1]);
    h.tap(ids::LOAD_CONTINUE);
    h.finish_passphrase(None);
    let fingerprint = h.app.fingerprints()[0];

    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_ENCRYPTED);
    h.tap(ids::FORM_CONTINUE);
    h.type_text(BACKUP_PASSPHRASE);
    h.key(osk_shell_api::Key::Enter);
    h.type_text(BACKUP_PASSPHRASE);
    h.key(osk_shell_api::Key::Enter);
    assert!(
        shows(&h, EN.backup_inside_seed),
        "the result says the file holds the seed: {:?}",
        on_screen(&h)
    );
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

    // A file that holds a seed is the same length as one that holds
    // words, so neither says which it is.
    assert_eq!(bytes.len(), osk_backup::oskb::LEN);

    let mut h = Harness::new(PANEL);
    h.go_home();
    h.tap(ids::HOME_SCAN);
    h.scan_bytes(&bytes);
    h.type_text(BACKUP_PASSPHRASE);
    h.key(osk_shell_api::Key::Enter);
    h.add_key();
    assert_eq!(h.app.fingerprints(), vec![fingerprint], "the same key");
    assert_eq!(
        h.app.has_mnemonic(0),
        Some(false),
        "a key read from a seed has no words"
    );
}
