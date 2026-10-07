//! A key kept on the device (`docs/PLANNING.md` §15 item 32): what a
//! person and a thief each get.
//!
//! The shell here is a Tier B one with a secure element: a fixed HMAC
//! key it never hands over, one blob it keeps, and the answers the
//! kept-secret channel asks for. A restart is a new [`Harness`] over the
//! same [`Element`], which is what a phone does when the app is opened
//! again.

mod common;

use common::{
    ABANDON, Element, Harness, PHONE, PIN, SECURE_PHONE, UNVERIFIED_PHONE, VERIFIED_PHONE,
    expected_fingerprint,
};
use opensigner_core::session::DEFAULT_WIPE_MS;
use opensigner_core::{AssuranceTier, ScreenKind, ids, strings};
use osk_bip::bip39::{Language, Mnemonic};
use osk_bip::keys::{Fingerprint, Network};
use osk_keep::KEEP_ATTEMPTS;
use osk_shell_api::{Command, Event, FileKind, Key};
use osk_ui::widgets::Icon;

/// The duress PIN a test sets; never the session PIN.
const DURESS: &str = "1379";

/// A second key's words, for a device that keeps more than one.
const ZOO: [&str; 12] = [
    "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "zoo", "wrong",
];

/// Adds the key over `words` with no passphrase to a session that has a
/// PIN, and declines the offer to keep it where one is made.
fn add_key(h: &mut Harness, words: &[&str]) {
    h.start_load(words);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    if h.app.screen() == ScreenKind::Keep {
        h.tap(ids::BACK);
    }
    assert_eq!(h.app.screen(), ScreenKind::Home);
}

/// A Tier B device with a key loaded, its words kept on the device, and
/// the element it kept them in.
fn keep_a_key(passphrase: Option<&str>) -> (Fingerprint, Element) {
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.start_load(&ABANDON);
    h.finish_load(passphrase);
    let fingerprint = h.app.fingerprints()[0];
    h.open_key(0);
    h.tap(ids::KEEP_ROW);
    assert_eq!(h.app.screen(), ScreenKind::Keep);
    h.hold(ids::KEEP_HOLD);
    assert_eq!(
        h.app.screen(),
        ScreenKind::KeyDetail,
        "keeping gives the key menu back"
    );
    assert!(h.app.secret_kept(), "the shell is holding the blob");
    (fingerprint, h.element.clone().expect("an element"))
}

/// Back until Home, where the status line's gear is.
fn go_home(h: &mut Harness) {
    while h.app.screen() != ScreenKind::Home {
        h.tap(ids::BACK);
    }
}

/// `digits` on the pad a device that keeps a key opens on, and returns
/// to whenever no key is loaded.
fn open_stored(h: &mut Harness, digits: &str) {
    assert_eq!(
        h.app.screen(),
        ScreenKind::StoredKey,
        "the pad is the screen"
    );
    h.type_pin(ids::KEEP_PIN_KEYBOARD, digits);
}

/// "Wrong PIN · 3 left before removal", as the pad's caption says it.
fn left_before_removal(n: u8) -> String {
    strings::fill1(strings::EN.keep_wrong, &format!("{n}"))
}

/// The whole point: the key is on the device after the app is closed,
/// and the right PIN brings it back as it was.
#[test]
fn a_kept_key_comes_back_after_a_restart() {
    let (fingerprint, element) = keep_a_key(None);

    let mut h = Harness::kept(SECURE_PHONE, element);
    assert!(h.app.fingerprints().is_empty(), "nothing is loaded yet");
    open_stored(&mut h, PIN);

    assert_eq!(h.app.fingerprints(), vec![fingerprint], "the same key");
    assert_eq!(h.app.has_mnemonic(0), Some(true), "with its words");
    assert_eq!(
        h.app.backup_verified(0),
        Some(false),
        "and its backup state"
    );
    assert!(h.app.has_pin(), "the PIN that opened it is the session PIN");
    h.open_key(0);
    assert!(
        h.app.rect_of(ids::DETAIL_OPEN_PASSPHRASE).is_some(),
        "the passphrase is opened over the key that came back"
    );
}

/// Eight wrong PINs and the key is gone, with the pad's caption counting
/// down on the way there.
#[test]
fn eight_wrong_pins_remove_the_stored_key() {
    let (_, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);

    for used in 1..KEEP_ATTEMPTS {
        open_stored(&mut h, "9999");
        assert!(h.app.fingerprints().is_empty(), "nothing opened");
        assert_eq!(h.app.screen(), ScreenKind::StoredKey, "the pad stays");
        let left = left_before_removal(KEEP_ATTEMPTS - used);
        assert!(
            h.app.texts().contains(&left),
            "the pad should say {left:?}, and says {:?}",
            h.app.texts()
        );
    }

    open_stored(&mut h, "9999");
    assert_eq!(h.app.screen(), ScreenKind::KeptRemoved);
    assert!(!h.app.secret_kept());
    assert!(
        h.element.as_ref().is_some_and(|e| e.blob.is_none()),
        "the shell was told to forget it"
    );
    h.tap(ids::KEEP_REMOVED_DONE);
    assert_eq!(
        h.app.screen(),
        ScreenKind::Home,
        "and the pad does not come back"
    );
}

/// A device that keeps a key opens on its PIN pad, with nowhere to go
/// back to; the PIN is the front door to everything (§16.64). Whenever
/// no key is loaded again — here, after the automatic wipe — the pad is
/// the screen again.
#[test]
fn a_device_that_keeps_a_key_opens_on_its_pad() {
    let (fingerprint, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);
    assert_eq!(h.app.screen(), ScreenKind::StoredKey);
    assert!(
        h.app.rect_of(ids::BACK).is_none(),
        "the front door has no back"
    );
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.lock_title)),
        "the one PIN pad's title: {:?}",
        h.app.texts()
    );

    h.type_pin(ids::KEEP_PIN_KEYBOARD, PIN);
    assert_eq!(h.app.fingerprints(), vec![fingerprint]);
    assert_ne!(h.app.screen(), ScreenKind::StoredKey);

    h.tick(DEFAULT_WIPE_MS);
    assert!(
        h.app.fingerprints().is_empty(),
        "the automatic wipe cleared memory"
    );
    assert!(h.app.secret_kept(), "and left the stored key");
    assert_eq!(h.app.screen(), ScreenKind::StoredKey, "so the pad is back");
    h.type_pin(ids::KEEP_PIN_KEYBOARD, PIN);
    assert_eq!(h.app.fingerprints(), vec![fingerprint]);
}

/// A blob the shell says it has and then cannot produce — its keys were
/// invalidated and it deleted the file, say — leaves no pad behind: the
/// app is the empty one, with nothing to type a PIN into.
#[test]
fn a_blob_the_shell_cannot_produce_leaves_no_pad_behind() {
    let (_, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);
    assert_eq!(h.app.screen(), ScreenKind::StoredKey);
    h.element.as_mut().expect("an element").blob = None;

    h.type_pin(ids::KEEP_PIN_KEYBOARD, PIN);
    assert!(!h.app.secret_kept());
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert!(h.app.fingerprints().is_empty());
}

/// On a device that can keep a key and keeps none, adding a key ends on
/// the offer to keep it (§16.65): the hold keeps it and "Not now"
/// declines, and the key is added either way. "Not now" is on the offer
/// and not on the same screen reached from the key menu. A device that
/// cannot keep anything goes straight Home, and one that already keeps a
/// key is not asked.
#[test]
fn adding_a_key_offers_to_keep_it_at_once() {
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    assert_eq!(
        h.app.screen(),
        ScreenKind::Keep,
        "the offer follows the PIN"
    );
    assert_eq!(h.app.fingerprints().len(), 1, "the key is already added");
    assert!(
        h.app.rect_of(ids::KEEP_NOT_NOW).is_some(),
        "the offer says how to decline it"
    );
    h.hold(ids::KEEP_HOLD);
    assert!(h.app.secret_kept(), "held: kept");
    assert_eq!(h.app.screen(), ScreenKind::Home);
    let element = h.element.clone().expect("an element");

    // Declining leaves the key added and nothing kept.
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    assert_eq!(h.app.screen(), ScreenKind::Keep);
    h.tap(ids::KEEP_NOT_NOW);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert_eq!(h.app.fingerprints().len(), 1);
    assert!(!h.app.secret_kept());

    // The same screen asked for from the key menu carries the hold
    // alone: the chevron is the way back from a row that was tapped.
    h.open_key(0);
    h.tap(ids::KEEP_ROW);
    assert_eq!(h.app.screen(), ScreenKind::Keep);
    assert!(h.app.rect_of(ids::KEEP_NOT_NOW).is_none());
    h.tap(ids::BACK);
    assert_eq!(h.app.screen(), ScreenKind::KeyDetail);
    assert!(!h.app.secret_kept());

    // No secure element: no offer.
    let mut h = Harness::with_tier(PHONE, AssuranceTier::B, true);
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_SKIP, ids::LOAD_PASS_CONTINUE);
    h.add_key();
    assert_eq!(h.app.screen(), ScreenKind::Home);

    // A key already kept: a second key is added without the offer.
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_ADD_PASSPHRASE, ids::LOAD_PASS_CONTINUE);
    h.type_text("other");
    h.key(osk_shell_api::Key::Enter);
    h.tap(ids::LOAD_WHICH_CONTINUE);
    h.add_key();
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert_eq!(h.app.fingerprints().len(), 2);
}

/// Forgetting the key this device keeps forgets the copy on the device
/// too (§16.65): the Forget screen says so, nothing is kept afterwards,
/// and the next start is the empty app rather than a pad that brings
/// the key back.
#[test]
fn forgetting_the_kept_key_removes_it_from_the_device() {
    let s = &strings::EN;
    let (_, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    h.open_key(0);
    h.tap(ids::KEY_FORGET);
    assert_eq!(h.app.screen(), ScreenKind::Forget);
    let texts = h.app.texts();
    for want in [s.keep_stored_row, s.settings_wipe_stored] {
        assert!(texts.contains(&String::from(want)), "{want:?} in {texts:?}");
    }

    h.seen.clear();
    h.hold(ids::DETAIL_FORGET);
    assert!(h.app.fingerprints().is_empty(), "gone from memory");
    assert!(!h.app.secret_kept(), "and from the device");
    assert!(h.asked_to_forget(), "the shell was told");
    assert!(h.element.as_ref().is_some_and(|e| e.blob.is_none()));
    assert_ne!(
        h.app.screen(),
        ScreenKind::StoredKey,
        "no pad to bring it back"
    );

    let h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    assert_eq!(
        h.app.screen(),
        ScreenKind::Home,
        "the next start is the empty app"
    );

    // A passphrase key over the same words is one set of words on the
    // device: it is kept — its Forget says so — and forgetting it
    // leaves the words kept for the key that still has them.
    let (_, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    h.start_load(&ABANDON);
    h.tap(ids::LOAD_CONTINUE);
    h.choose(ids::LOAD_ADD_PASSPHRASE, ids::LOAD_PASS_CONTINUE);
    h.type_text("other");
    h.key(osk_shell_api::Key::Enter);
    h.tap(ids::LOAD_WHICH_CONTINUE);
    h.add_key();
    h.open_key(1);
    h.tap(ids::KEY_FORGET);
    assert!(
        h.app.texts().contains(&String::from(s.keep_stored_row)),
        "every key with words is a kept key"
    );
    h.seen.clear();
    h.hold(ids::DETAIL_FORGET);
    assert_eq!(h.app.fingerprints().len(), 1);
    assert!(
        h.app.secret_kept(),
        "the words are still kept for the other key"
    );
    assert!(!h.asked_to_forget());
}

/// One PIN, every key (§16.66): a device that keeps keys keeps all of
/// them, a key added while keys are kept goes to the device with no
/// word to the element, every one comes back on the next start, and
/// the stored bytes are the same size however many there are.
#[test]
fn every_key_is_kept_under_the_one_pin() {
    let (first, element) = keep_a_key(None);
    let one_key = element.blob.as_ref().map(Vec::len);
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    h.seen.clear();

    add_key(&mut h, &ZOO);
    assert_eq!(h.app.fingerprints().len(), 2);
    let second = h.app.fingerprints()[1];
    assert!(h.mac_salts().is_empty(), "the element was asked");
    assert!(
        h.seen
            .iter()
            .any(|c| matches!(c, Command::StoreSecret { .. })),
        "the device was not given the new key"
    );
    let element = h.element.clone().expect("an element");
    assert_eq!(
        element.blob.as_ref().map(Vec::len),
        one_key,
        "two keys and one are the same number of bytes"
    );

    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    assert_eq!(h.app.fingerprints(), vec![first, second], "both come back");
    assert!(h.app.has_pin(), "under the one PIN");
}

/// Forgetting one of the kept keys leaves the others on the device;
/// forgetting the last one forgets the blob (§16.66).
#[test]
fn forgetting_one_kept_key_leaves_the_others_kept() {
    let (first, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    add_key(&mut h, &ZOO);
    let second = h.app.fingerprints()[1];

    h.seen.clear();
    h.open_key(0);
    h.tap(ids::KEY_FORGET);
    h.hold(ids::DETAIL_FORGET);
    assert_eq!(h.app.fingerprints(), vec![second]);
    assert!(h.app.secret_kept(), "the other key is still kept");
    assert!(!h.asked_to_forget(), "the blob stays");
    assert!(h.mac_salts().is_empty(), "the element was asked");

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    open_stored(&mut h, PIN);
    assert_eq!(
        h.app.fingerprints(),
        vec![second],
        "only the kept one comes back"
    );
    assert_ne!(first, second);

    h.open_key(0);
    h.tap(ids::KEY_FORGET);
    h.hold(ids::DETAIL_FORGET);
    assert!(h.app.fingerprints().is_empty());
    assert!(!h.app.secret_kept(), "the last key took the blob with it");
    assert!(h.asked_to_forget());
    let h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    assert_eq!(h.app.screen(), ScreenKind::Home, "nothing is kept");
}

/// A wallet in use, as a single-key descriptor over the test key's own
/// account: public data, and what a device that keeps keys keeps with
/// them.
const WALLET: &str = "wpkh([73c5da0a/84'/0'/0']xpub6CatWdiZiodmUeTDp8LT5or8nmbKNcuyvz7WyksVFkKB4RHwCD3XyuvPEbvqAQY3rAPshWcMLoP2fMFMKHPJ4ZeZXYVUhLv1VMrjPC7PW6V/<0;1>/*)";

/// Add › "Load a wallet", and `WALLET` read in there and used.
fn use_the_wallet(h: &mut Harness) {
    go_home(h);
    h.open_add_wallet(ids::WALLETS_LOAD);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: WALLET.as_bytes().to_vec(),
    });
    h.tap(ids::INSPECT_USE_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
}

/// Whether Home lists a policy.
fn lists_a_wallet(h: &mut Harness) -> bool {
    go_home(h);
    h.open_wallets();
    h.app
        .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0))
        .is_some()
}

/// A device that keeps keys keeps the wallets in use beside them, so
/// Wallets is not empty on a restart where Keys is not (§16.72). What is
/// forgotten stays forgotten, and the last key takes the wallets with
/// it.
#[test]
fn a_wallet_in_use_comes_back_with_the_kept_keys() {
    let (_, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    use_the_wallet(&mut h);

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    open_stored(&mut h, PIN);
    assert!(lists_a_wallet(&mut h), "the wallet came back with the key");

    h.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0));
    h.tap(ids::WALLET_FORGET);
    assert!(!lists_a_wallet(&mut h), "forgotten here");

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    open_stored(&mut h, PIN);
    assert!(!lists_a_wallet(&mut h), "and forgotten on the device");

    // The wallets are kept under the keys' own protection, so the last
    // key takes the blob and the wallets with it.
    use_the_wallet(&mut h);
    go_home(&mut h);
    h.open_key(0);
    h.tap(ids::KEY_FORGET);
    h.hold(ids::DETAIL_FORGET);
    assert!(!h.app.secret_kept());
    assert!(h.element.as_ref().is_some_and(|e| e.blob.is_none()));
    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    assert!(!lists_a_wallet(&mut h), "nothing is kept");
}

/// A device that keeps nothing keeps no wallets either: the session's
/// wallets are the session's.
#[test]
fn a_stateless_device_forgets_the_wallets_it_was_shown() {
    let mut h = Harness::new(SECURE_PHONE);
    h.start_load(&ABANDON);
    h.finish_load(None);
    use_the_wallet(&mut h);
    assert!(lists_a_wallet(&mut h));

    let mut h = Harness::new(SECURE_PHONE);
    assert!(!lists_a_wallet(&mut h), "a restart starts with nothing");
}

/// The duress PIN opens a device that erases itself and says nothing.
#[test]
fn the_duress_pin_removes_everything_and_shows_no_error() {
    let (_, element) = keep_a_key(None);

    // Set the duress PIN on the device that is keeping the key.
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    go_home(&mut h);
    h.open_settings();
    h.tap(ids::KEEP_DURESS_ROW);
    assert_eq!(h.app.screen(), ScreenKind::DuressPin);
    h.type_pin(ids::KEEP_DURESS_KEYBOARD, DURESS);
    h.type_pin(ids::KEEP_DURESS_KEYBOARD, DURESS);
    assert_eq!(h.app.screen(), ScreenKind::Settings);
    let element = h.element.clone().expect("an element");
    assert!(element.blob.is_some(), "the key is still kept");

    // On the next start, the duress PIN opens an empty app.
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, DURESS);
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert!(h.app.fingerprints().is_empty());
    assert!(!h.app.secret_kept(), "no stored key is offered");
    assert!(
        h.element.as_ref().is_some_and(|e| e.blob.is_none()),
        "and the bytes are gone"
    );
    assert!(
        !h.app.texts().iter().any(|t| t.contains("Wrong")),
        "nothing on screen says a wrong PIN was entered"
    );

    // The storage PIN still opens the key on a device that was not
    // handed over, so the decoy is a second door and not a replacement.
    let (_, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    assert_eq!(h.app.fingerprints().len(), 1);
}

/// The bytes are worth nothing off the device they were written on, and
/// nothing after they are edited.
#[test]
fn a_moved_or_edited_blob_opens_nothing() {
    let (_, element) = keep_a_key(None);

    // The same blob on a device whose element holds another key.
    let mut moved = element.clone();
    moved.key = [0x11; 32];
    let mut h = Harness::kept(SECURE_PHONE, moved);
    open_stored(&mut h, PIN);
    assert!(h.app.fingerprints().is_empty(), "another device opens it");
    assert_eq!(h.app.screen(), ScreenKind::StoredKey);

    // The same element, with one byte of the header changed.
    let mut edited = element;
    edited.blob.as_mut().expect("a blob")[20] ^= 1;
    let mut h = Harness::kept(SECURE_PHONE, edited);
    open_stored(&mut h, PIN);
    assert!(h.app.fingerprints().is_empty(), "an edited header opens it");
}

/// What the shell is holding is ciphertext: none of the secret is in it.
#[test]
fn the_stored_bytes_hold_no_seed_no_words_and_no_pin() {
    let (_, element) = keep_a_key(None);
    let blob = element.blob.expect("a blob");

    let mnemonic = Mnemonic::parse(Language::English, &ABANDON.join(" ")).expect("the words");
    let seed = mnemonic.to_seed(b"").expect("a seed");
    let indices: Vec<u8> = mnemonic
        .indices()
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .collect();

    for run in seed.expose().windows(8) {
        assert!(!contains(&blob, run), "a run of the seed is in the blob");
    }
    for run in indices.windows(8) {
        assert!(!contains(&blob, run), "the word indices are in the blob");
    }
    assert!(!contains(&blob, PIN.as_bytes()), "the PIN is in the blob");
}

/// Nothing is offered and nothing is stored where a key cannot be kept.
#[test]
fn a_device_without_secure_hardware_or_tier_b_keeps_nothing() {
    // Tier B, but the shell has no secure element.
    let mut h = Harness::with_tier(PHONE, AssuranceTier::B, true);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    assert!(h.app.rect_of(ids::KEEP_ROW).is_none(), "no row on Tier B");

    // A secure element, but not the tier that keeps anything.
    let mut h = Harness::with_tier(SECURE_PHONE, AssuranceTier::C, true);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    assert!(h.app.rect_of(ids::KEEP_ROW).is_none(), "no row on Tier C");
    go_home(&mut h);
    h.open_settings();
    assert!(h.app.rect_of(ids::KEEP_DURESS_ROW).is_none());

    let commands = h.drain();
    assert!(
        !commands
            .iter()
            .any(|c| matches!(c, Command::StoreSecret { .. })),
        "and the shell is never asked to store anything"
    );
}

/// A passphrase is what the person types each session, so it is not in
/// the blob and it does not come back with the words.
#[test]
fn a_passphrase_is_not_kept_with_the_words() {
    let (with_passphrase, element) = keep_a_key(Some("swordfish"));
    assert_eq!(
        with_passphrase,
        expected_fingerprint(b"swordfish", Network::Mainnet)
    );
    assert!(
        !contains(&element.blob.clone().expect("a blob"), b"swordfish"),
        "the passphrase is in the blob"
    );

    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    assert_eq!(
        h.app.fingerprints(),
        vec![expected_fingerprint(b"", Network::Mainnet)],
        "the words came back, and the passphrase did not"
    );

    // The way to the passphrase key is to open it from the words that
    // came back, behind what the person types.
    h.open_key(0);
    assert!(
        h.app.rect_of(ids::DETAIL_OPEN_PASSPHRASE).is_some(),
        "the passphrase is asked for again"
    );
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

/// A system nobody verified is not one the app can promise anything
/// about, so it does not run: the refusal is the whole screen and Exit
/// is the only control. A device that booted the system it should have
/// runs as any other does.
#[test]
fn an_unverified_boot_refuses_to_run() {
    let s = &strings::EN;
    let mut h = Harness::kept(UNVERIFIED_PHONE, Element::default());
    assert_eq!(h.app.screen(), ScreenKind::BootRefused);
    let texts = h.app.texts();
    for want in [
        s.boot_refused_title,
        s.boot_refused_system,
        s.boot_refused_system_value,
        s.boot_refused_fix,
        s.boot_refused_fix_value,
    ] {
        assert!(texts.iter().any(|t| t == want), "{want:?}: {texts:?}");
    }
    assert!(
        h.app.rect_of(ids::at(ids::HOME_TILE_BASE, 4)).is_none(),
        "there is no Home behind it"
    );
    assert!(h.app.rect_of(ids::BACK).is_none(), "and no way past it");

    // Exit is the only control, and the refusal stays until the shell
    // acts on it.
    h.seen.clear();
    h.tap(ids::BOOT_EXIT);
    assert!(
        h.seen.contains(&Command::Exit),
        "the shell was asked to close: {:?}",
        h.seen
    );
    assert_eq!(h.app.screen(), ScreenKind::BootRefused);

    let mut h = Harness::kept(VERIFIED_PHONE, Element::default());
    assert_eq!(h.app.screen(), ScreenKind::Home);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::KEEP_ROW);
    assert_eq!(h.app.screen(), ScreenKind::Keep);
    assert!(
        h.app.rect_of(ids::KEEP_HOLD).is_some(),
        "a key can be kept on it"
    );
}

/// A timeout is not a decision to give up the key on the device: the
/// auto-wipe takes what is in memory, and the stored key opens again on
/// the next start.
#[test]
fn an_automatic_wipe_leaves_the_key_kept_on_the_device() {
    let (fingerprint, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    assert_eq!(h.app.fingerprints(), vec![fingerprint]);

    h.seen.clear();
    h.tick(DEFAULT_WIPE_MS + 1);
    assert!(h.app.fingerprints().is_empty(), "memory is wiped");
    assert_eq!(h.app.screen(), ScreenKind::StoredKey, "the pad is back");
    assert!(h.app.secret_kept(), "the stored key stays");
    assert!(!h.asked_to_forget(), "the shell is not asked to forget it");

    let element = h.element.clone().expect("an element");
    assert!(element.blob.is_some(), "and the bytes are still there");
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    assert_eq!(
        h.app.fingerprints(),
        vec![fingerprint],
        "the same key comes back"
    );
}

/// A right PIN ends the run of wrong ones: the count is of PINs typed
/// since the key last opened, not since it was kept.
#[test]
fn opening_the_stored_key_resets_the_count_of_wrong_pins() {
    let (_, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);
    for _ in 0..3 {
        open_stored(&mut h, "9999");
    }
    open_stored(&mut h, PIN);
    assert_eq!(h.app.fingerprints().len(), 1, "the key opened");
    let element = h.element.clone().expect("an element");

    // On the next start every one of the eight is available again.
    let mut h = Harness::kept(SECURE_PHONE, element);
    for used in 1..KEEP_ATTEMPTS {
        open_stored(&mut h, "9999");
        assert_eq!(h.app.screen(), ScreenKind::StoredKey, "wrong PIN {used}");
    }
    assert!(h.app.secret_kept(), "seven wrong PINs left the key kept");
    open_stored(&mut h, "9999");
    assert_eq!(h.app.screen(), ScreenKind::KeptRemoved);
}

/// One pad, wherever a PIN is asked for (§16.63): on a device that keeps
/// a key, the lock screen tries the stored key too, so wrong PINs there
/// count toward its removal — with the same caption as its own pad —
/// and the eighth removes it from the device and from memory. The
/// session's own count of five never runs on such a device.
#[test]
fn wrong_pins_on_the_lock_screen_count_against_the_stored_key() {
    let s = &strings::EN;
    let (fingerprint, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    go_home(&mut h);
    h.open_settings();
    h.tap(ids::SETTINGS_LOCK);
    assert!(h.app.is_locked());

    h.seen.clear();
    for used in 1..KEEP_ATTEMPTS {
        h.type_pin(ids::LOCK_KEYBOARD, "9999");
        assert!(h.app.is_locked(), "wrong PIN {used} unlocked or wiped");
        let left = strings::fill1(s.keep_wrong, &format!("{}", KEEP_ATTEMPTS - used));
        assert!(h.app.texts().contains(&left), "{:?}", h.app.texts());
        assert!(
            !h.app.texts().iter().any(|t| t.contains("before wipe")),
            "the session's count is not what is shown"
        );
    }
    assert!(h.app.secret_kept(), "seven wrong PINs left the key kept");
    assert!(!h.asked_to_forget());

    // The right PIN opens the session and ends the run.
    h.type_pin(ids::LOCK_KEYBOARD, PIN);
    assert!(!h.app.is_locked());
    assert_eq!(h.app.fingerprints(), vec![fingerprint]);
    h.open_settings();
    h.tap(ids::SETTINGS_LOCK);
    for used in 1..KEEP_ATTEMPTS {
        h.type_pin(ids::LOCK_KEYBOARD, "9999");
        assert!(h.app.is_locked(), "wrong PIN {used} after the reset");
    }
    assert!(h.app.secret_kept(), "the count started over");

    // The eighth removes the stored key, and the key in memory with it.
    h.type_pin(ids::LOCK_KEYBOARD, "9999");
    assert_eq!(h.app.screen(), ScreenKind::KeptRemoved);
    assert!(!h.app.is_locked());
    assert!(h.app.fingerprints().is_empty(), "the key in memory stayed");
    assert!(!h.app.secret_kept());
    assert!(h.asked_to_forget());
    assert!(h.element.as_ref().is_some_and(|e| e.blob.is_none()));
}

/// The duress PIN works on the lock screen as it does on the stored
/// key's own pad: everything goes, and nothing says so.
#[test]
fn the_duress_pin_on_the_lock_screen_removes_everything() {
    let (_, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    go_home(&mut h);
    h.open_settings();
    h.tap(ids::KEEP_DURESS_ROW);
    h.type_pin(ids::KEEP_DURESS_KEYBOARD, DURESS);
    h.type_pin(ids::KEEP_DURESS_KEYBOARD, DURESS);
    assert_eq!(h.app.screen(), ScreenKind::Settings);
    h.tap(ids::SETTINGS_LOCK);
    assert!(h.app.is_locked());

    h.type_pin(ids::LOCK_KEYBOARD, DURESS);
    assert!(!h.app.is_locked());
    assert_eq!(h.app.screen(), ScreenKind::Home);
    assert!(h.app.fingerprints().is_empty());
    assert!(!h.app.secret_kept(), "no stored key is offered");
    assert!(h.asked_to_forget());
    assert!(h.element.as_ref().is_some_and(|e| e.blob.is_none()));
    assert!(
        !h.app.texts().iter().any(|t| t.contains("Wrong")),
        "nothing on screen says a wrong PIN was entered"
    );
}

/// Dismissing the system prompt on the lock screen costs nothing: no
/// count, no caption, and the pad is where it was.
#[test]
fn a_dismissed_prompt_on_the_lock_screen_counts_nothing() {
    let (fingerprint, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    go_home(&mut h);
    h.open_settings();
    h.tap(ids::SETTINGS_LOCK);

    h.element.as_mut().expect("an element").available = false;
    h.type_pin(ids::LOCK_KEYBOARD, "9999");
    assert!(h.app.is_locked());
    assert!(
        !h.app.texts().iter().any(|t| t.contains("Wrong")),
        "{:?}",
        h.app.texts()
    );

    h.element.as_mut().expect("an element").available = true;
    h.type_pin(ids::LOCK_KEYBOARD, "9999");
    let left = strings::fill1(strings::EN.keep_wrong, &format!("{}", KEEP_ATTEMPTS - 1));
    assert!(h.app.texts().contains(&left), "the first counted wrong PIN");
    h.type_pin(ids::LOCK_KEYBOARD, PIN);
    assert_eq!(h.app.fingerprints(), vec![fingerprint]);
}

/// The wipe a person asks for is the one that takes everything: the
/// screen says the stored key goes, and it does.
#[test]
fn wiping_every_key_removes_the_key_kept_on_the_device() {
    let s = &strings::EN;
    let (_, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    go_home(&mut h);
    h.open_settings();
    h.tap(ids::SETTINGS_WIPE_ROW);
    assert_eq!(h.app.screen(), ScreenKind::WipeAll);
    let texts = h.app.texts();
    for want in [s.keep_stored_row, s.settings_wipe_stored] {
        assert!(
            texts.iter().any(|t| t == want),
            "the screen should say {want:?}: {texts:?}"
        );
    }

    h.seen.clear();
    h.hold(ids::SETTINGS_WIPE);
    assert_eq!(h.app.screen(), ScreenKind::Wiped);
    assert!(h.asked_to_forget(), "the shell was told to forget it");
    assert!(!h.app.secret_kept());
    assert!(
        h.element.as_ref().is_some_and(|e| e.blob.is_none()),
        "and the bytes are gone"
    );
}

/// Every attempt asks the element a different question, so a tag read
/// out of one attempt is worth nothing for the next PIN tried.
#[test]
fn each_pin_asks_the_secure_element_a_different_question() {
    let (_, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);
    h.seen.clear();
    open_stored(&mut h, "1111");
    open_stored(&mut h, "2222");
    let salts = h.mac_salts();
    assert_eq!(salts.len(), 2, "one question per attempt: {salts:?}");
    assert_ne!(salts[0], salts[1], "two PINs asked the same question");
}

/// The other half of that: the element's answer to one PIN's question
/// opens nothing under another PIN.
#[test]
fn a_tag_for_one_pin_opens_nothing_under_another() {
    let (_, element) = keep_a_key(None);
    let blob = element.blob.clone().expect("a blob");
    let header = osk_keep::header(&blob).expect("a header");
    let right = osk_keep::challenge(&header, PIN.as_bytes()).expect("the PIN stretched");
    let wrong = osk_keep::challenge(&header, b"1111").expect("the PIN stretched");

    let tag = element.mac(right.challenge());
    assert!(
        matches!(
            osk_keep::open(&blob, &tag, &right),
            osk_keep::Opened::Keys(..)
        ),
        "the PIN the tag was made for opens the keys"
    );
    assert!(
        matches!(osk_keep::open(&blob, &tag, &wrong), osk_keep::Opened::Wrong),
        "a captured tag opens the blob under another PIN"
    );
    assert!(
        matches!(
            osk_keep::open(&blob, &element.mac(wrong.challenge()), &wrong),
            osk_keep::Opened::Wrong
        ),
        "a wrong PIN with its own tag opens something"
    );
}

/// The session PIN is also the storage PIN, and a lock rotates the
/// session key under it. A key kept after a lock and an unlock is kept
/// under the same digits, so the next restart opens it with them.
#[test]
fn a_key_kept_after_a_lock_opens_under_the_same_pin() {
    let mut h = Harness::kept(SECURE_PHONE, Element::default());
    h.start_load(&ABANDON);
    h.finish_load(None);
    let fingerprint = h.app.fingerprints()[0];

    h.open_settings();
    h.tap(ids::SETTINGS_LOCK);
    assert!(h.app.is_locked());
    h.unlock(PIN);
    go_home(&mut h);

    h.open_key(0);
    h.tap(ids::KEEP_ROW);
    h.hold(ids::KEEP_HOLD);
    assert!(h.app.secret_kept(), "kept after the lock");
    let element = h.element.clone().expect("an element");

    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    assert_eq!(h.app.fingerprints(), vec![fingerprint], "the same key");
    assert!(h.app.has_pin(), "the same digits opened it");
}

// ----- Whether a passphrase wallet is kept at all (§16.104 rule 7) -----

/// The passphrase the wallet's key is opened with.
const PASSPHRASE: &str = "swordfish";

/// Opens the passphrase key of the kept words and builds a SegWit
/// single-sig wallet over it, answering the Choice that follows "Add
/// this wallet" with "Yes" when `keep`, with "No" otherwise.
fn add_a_passphrase_wallet(h: &mut Harness, keep: bool) {
    h.open_passphrase(0);
    h.type_text(PASSPHRASE);
    h.key(Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Opened);
    assert_eq!(h.app.fingerprints().len(), 2, "the words and the key");

    h.add_single_sig(1, 2);
    assert!(
        h.app.rect_of(ids::at(ids::PICK_BASE, 1)).is_some(),
        "the wallet's page asks whether to keep it"
    );
    let texts = h.app.texts();
    assert!(
        texts.iter().any(|t| t == strings::EN.wallet_keep_title),
        "titled with the question: {texts:?}"
    );
    if keep {
        h.tap(ids::at(ids::PICK_BASE, 1));
    }
    h.tap(ids::PICK_CONTINUE);
}

/// A device that keeps keys, with the test words kept and a session
/// open on them.
fn a_device_holding_the_words() -> Harness {
    let (_, element) = keep_a_key(None);
    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    h
}

/// The kept descriptor is evidence the wallet exists, so a wallet over a
/// passphrase key is written only if the person says to. "No" leaves it
/// the session's.
#[test]
fn a_passphrase_wallet_answered_no_is_gone_after_a_restart() {
    let mut h = a_device_holding_the_words();
    add_a_passphrase_wallet(&mut h, false);
    assert!(lists_a_wallet(&mut h), "the wallet is in use this session");

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    open_stored(&mut h, PIN);
    assert_eq!(h.app.fingerprints().len(), 1, "the plain key is back");
    assert!(!lists_a_wallet(&mut h), "and the wallet was never written");
}

/// "Yes" keeps it as any policy is kept: the addresses come back, the
/// row carries the eye because no key of it is loaded, and its member
/// says so, which is where rule 6's path starts.
#[test]
fn a_passphrase_wallet_answered_yes_comes_back_without_its_key() {
    let mut h = a_device_holding_the_words();
    add_a_passphrase_wallet(&mut h, true);

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    open_stored(&mut h, PIN);
    assert!(lists_a_wallet(&mut h), "the wallet came back");
    assert!(
        h.app
            .row_glyphs()
            .iter()
            .any(|(_, icon)| *icon == Icon::Eye),
        "with the eye: no key of it is loaded"
    );

    h.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0));
    h.tap(ids::WALLET_ADDRESSES);
    assert_eq!(h.app.screen(), ScreenKind::Addresses, "its addresses");
    h.tap(ids::BACK);
    h.tap(ids::WALLET_KEYS);
    assert!(
        h.app
            .texts()
            .iter()
            .any(|t| t == strings::EN.key_not_loaded),
        "and its member is not loaded: {:?}",
        h.app.texts()
    );
}

/// The row on the wallet's page is the same choice, and it changes it
/// both ways.
#[test]
fn the_kept_on_this_device_row_takes_a_wallet_out_of_the_blob_and_back() {
    let mut h = a_device_holding_the_words();
    add_a_passphrase_wallet(&mut h, true);
    h.tap(ids::WALLET_KEEP);

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    open_stored(&mut h, PIN);
    assert!(!lists_a_wallet(&mut h), "turned off, it is not written");

    // On again, from the page of the wallet added a second time.
    add_a_passphrase_wallet(&mut h, false);
    h.tap(ids::WALLET_KEEP);
    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    open_stored(&mut h, PIN);
    assert!(lists_a_wallet(&mut h), "turned on, it is");
}

/// The question is about a wallet this device made from a passphrase key
/// it holds. A wallet over plain words is kept as any policy is, and its
/// page still carries the row.
#[test]
fn a_wallet_with_no_passphrase_key_is_kept_without_being_asked() {
    let mut h = a_device_holding_the_words();
    h.add_single_sig(0, 2);
    assert!(
        h.app.rect_of(ids::at(ids::PICK_BASE, 1)).is_none(),
        "nothing to ask: {:?}",
        h.app.texts()
    );
    assert!(
        h.app.rect_of(ids::WALLET_KEEP).is_some(),
        "the row is there"
    );

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    open_stored(&mut h, PIN);
    assert!(lists_a_wallet(&mut h), "kept with the keys");
}

/// A device that keeps nothing has no blob for a wallet to be in, so it
/// neither asks nor offers the row.
#[test]
fn a_device_that_keeps_no_keys_never_asks_about_a_wallet() {
    let mut h = Harness::new(SECURE_PHONE);
    h.start_load(&ABANDON);
    h.finish_load(None);
    h.open_passphrase(0);
    h.type_text(PASSPHRASE);
    h.key(Key::Enter);
    h.add_single_sig(1, 2);
    assert!(
        h.app.rect_of(ids::at(ids::PICK_BASE, 1)).is_none(),
        "no Choice"
    );
    assert!(h.app.rect_of(ids::WALLET_KEEP).is_none(), "and no row");
}

/// The seed of a passphrase key is never in the blob under any answer:
/// what comes back after a restart is the words and the plain key they
/// state.
#[test]
fn a_passphrase_key_is_not_in_the_blob_whatever_the_wallet_answer_is() {
    let mut h = a_device_holding_the_words();
    add_a_passphrase_wallet(&mut h, true);
    let plain = h.app.fingerprints()[0];
    let with_passphrase = h.app.fingerprints()[1];

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    open_stored(&mut h, PIN);
    assert_eq!(
        h.app.fingerprints(),
        vec![plain],
        "the plain key is back and the passphrase key is not"
    );
    assert!(
        !contains(
            &h.element.clone().expect("an element").blob.expect("a blob"),
            PASSPHRASE.as_bytes()
        ),
        "the passphrase is in the blob"
    );
    assert_ne!(plain, with_passphrase);
}

// ----- Notes and recovery sheets kept on the device (§16.112 pass E3) -----

/// Tools › Notes, from wherever the app is.
fn open_notes(h: &mut Harness) {
    go_home(h);
    h.tap(ids::at(ids::HOME_TILE_BASE, 3));
    h.tap(ids::TOOLS_NOTES);
    assert_eq!(h.app.screen(), ScreenKind::Notes);
}

/// Types `text` as a new note and leaves the Document it lands on.
fn write_a_note(h: &mut Harness, text: &str) {
    open_notes(h);
    h.tap(ids::NOTES_NEW);
    assert_eq!(h.app.screen(), ScreenKind::NoteText);
    h.type_text(text);
    h.key(Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Note);
}

/// Whether any text on screen holds `text`.
fn shows(h: &Harness, text: &str) -> bool {
    h.app.texts().iter().any(|t| t.contains(text))
}

/// The note a person says to keep is on the device after a restart, and
/// reads as it was written.
#[test]
fn a_kept_note_comes_back_after_a_restart() {
    let note = "Words with the notary. Keys in the safe.";
    let mut h = a_device_holding_the_words();
    write_a_note(&mut h, note);
    h.tap(ids::NOTE_KEEP);

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    open_stored(&mut h, PIN);
    open_notes(&mut h);
    h.tap(ids::at(ids::NOTES_KEPT_BASE, 0));
    assert_eq!(h.app.screen(), ScreenKind::Note);
    assert!(shows(&h, note), "the note came back: {:?}", h.app.texts());

    // Forgetting it forgets it on the device too.
    h.tap(ids::NOTE_FORGET);
    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    open_stored(&mut h, PIN);
    open_notes(&mut h);
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.notes_none)),
        "nothing is kept: {:?}",
        h.app.texts()
    );
}

/// A wallet's recovery sheet is kept on the same toggle, and comes back
/// on the wallet's own page.
#[test]
fn a_kept_recovery_sheet_comes_back_on_its_wallet() {
    let note = "The heir opens this with the passphrase in the will.";
    let mut h = a_device_holding_the_words();
    use_the_wallet(&mut h);
    h.tap(ids::WALLET_SHEET);
    assert_eq!(h.app.screen(), ScreenKind::Sheet);
    h.tap(ids::SHEET_NOTE);
    h.type_text(note);
    h.key(Key::Enter);
    assert_eq!(h.app.screen(), ScreenKind::Sheet);
    h.tap(ids::SHEET_KEEP);

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    open_stored(&mut h, PIN);
    assert!(lists_a_wallet(&mut h), "the wallet came back");
    h.tap(ids::at(ids::WALLETS_POLICY_ROW_BASE, 0));
    h.tap(ids::WALLET_SHEET);
    assert!(
        shows(&h, note),
        "the sheet came back with its note: {:?}",
        h.app.texts()
    );

    // The wallet forgotten, the sheet is still a document, and is
    // listed under Tools › Notes until the wallet is added again.
    h.tap(ids::BACK);
    h.tap(ids::WALLET_FORGET);
    open_notes(&mut h);
    h.tap(ids::at(ids::NOTES_SHEET_BASE, 0));
    assert!(
        shows(&h, note),
        "the orphaned sheet reads whole: {:?}",
        h.app.texts()
    );
    h.tap(ids::SHEET_ADD_WALLET);
    assert_eq!(h.app.screen(), ScreenKind::Wallet);
    h.tap(ids::WALLET_SHEET);
    assert!(
        shows(&h, note),
        "and moved back to the wallet: {:?}",
        h.app.texts()
    );
    open_notes(&mut h);
    assert!(
        h.app.rect_of(ids::at(ids::NOTES_SHEET_BASE, 0)).is_none(),
        "so Notes no longer lists it: {:?}",
        h.app.texts()
    );
}

/// The record holds eight. The ninth is refused on a dead row that says
/// how many are kept.
#[test]
fn a_ninth_item_is_refused_and_the_row_says_how_many_are_kept() {
    let mut h = a_device_holding_the_words();
    for i in 0..8 {
        write_a_note(&mut h, &format!("Note {i}"));
        h.tap(ids::NOTE_KEEP);
    }
    write_a_note(&mut h, "Note 8");
    assert!(
        shows(&h, strings::EN.notes_keep_full),
        "the row says why it is dead: {:?}",
        h.app.texts()
    );
    assert!(
        h.app.rect_of(ids::NOTE_KEEP).is_none(),
        "and there is nothing to tap"
    );

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    open_stored(&mut h, PIN);
    open_notes(&mut h);
    assert!(
        h.app.rect_of(ids::at(ids::NOTES_KEPT_BASE, 7)).is_some(),
        "the eight are on the device: {:?}",
        h.app.texts()
    );
    assert!(!shows(&h, "Note 8"), "and the ninth never was");
}

/// A blob written by the build before this one — same header, same
/// keys, wallets and duress records, no notes record — opens under the
/// same PIN and the same element answer. The next write makes it a
/// version-5 blob, which opens again.
#[test]
fn a_version_4_blob_opens_and_the_next_write_makes_it_version_5() {
    let mut h = a_device_holding_the_words();
    use_the_wallet(&mut h);
    let fingerprint = h.app.fingerprints()[0];
    let mut element = h.element.clone().expect("an element");

    // The version-4 layout is this one without its last record: the
    // bytes before it are what that build wrote.
    let blob = element.blob.as_mut().expect("a blob");
    assert_eq!(blob.len(), osk_keep::BLOB_LEN);
    blob.truncate(osk_keep::BLOB_LEN_V4);
    blob[0] = osk_keep::VERSION_V4;

    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    assert_eq!(h.app.fingerprints(), vec![fingerprint], "the key is back");
    assert!(lists_a_wallet(&mut h), "and the wallet with it");
    open_notes(&mut h);
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.notes_none)),
        "a version-4 blob keeps no notes: {:?}",
        h.app.texts()
    );

    // Keeping a note is a write, and the write is version 5.
    write_a_note(&mut h, "Kept after the change");
    h.tap(ids::NOTE_KEEP);
    let element = h.element.clone().expect("an element");
    let blob = element.blob.clone().expect("a blob");
    assert_eq!(blob.len(), osk_keep::BLOB_LEN);
    assert_eq!(blob[0], osk_keep::VERSION);

    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    assert_eq!(h.app.fingerprints(), vec![fingerprint], "the key still");
    open_notes(&mut h);
    h.tap(ids::at(ids::NOTES_KEPT_BASE, 0));
    assert!(
        shows(&h, "Kept after the change"),
        "and the note is kept: {:?}",
        h.app.texts()
    );
}

/// A duress PIN set on a version-4 blob still wipes the device after
/// that blob has become a version-5 one: the upgrade rewrites three
/// records and leaves the duress record, which it could not write.
#[test]
fn the_duress_pin_survives_the_upgrade_to_version_5() {
    let mut h = a_device_holding_the_words();
    go_home(&mut h);
    h.open_settings();
    h.tap(ids::KEEP_DURESS_ROW);
    h.type_pin(ids::KEEP_DURESS_KEYBOARD, DURESS);
    h.type_pin(ids::KEEP_DURESS_KEYBOARD, DURESS);
    let mut element = h.element.clone().expect("an element");

    let blob = element.blob.as_mut().expect("a blob");
    blob.truncate(osk_keep::BLOB_LEN_V4);
    blob[0] = osk_keep::VERSION_V4;

    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, PIN);
    write_a_note(&mut h, "Written after the upgrade");
    h.tap(ids::NOTE_KEEP);
    let element = h.element.clone().expect("an element");
    assert_eq!(
        element.blob.as_ref().expect("a blob").len(),
        osk_keep::BLOB_LEN,
        "the blob is version 5 now"
    );

    let mut h = Harness::kept(SECURE_PHONE, element);
    open_stored(&mut h, DURESS);
    assert!(h.app.fingerprints().is_empty(), "nothing loaded");
    assert!(h.asked_to_forget(), "the device erased itself");
}

/// Wipe takes the notes with the keys: the blob is gone and a restart
/// has neither.
#[test]
fn a_wipe_takes_the_kept_notes_with_the_keys() {
    let mut h = a_device_holding_the_words();
    write_a_note(&mut h, "Gone with the keys");
    h.tap(ids::NOTE_KEEP);
    go_home(&mut h);
    h.open_settings();
    h.tap(ids::SETTINGS_WIPE_ROW);
    h.hold(ids::SETTINGS_WIPE);
    assert!(h.element.as_ref().is_some_and(|e| e.blob.is_none()));

    let mut h = Harness::kept(SECURE_PHONE, h.element.clone().expect("an element"));
    assert_eq!(h.app.screen(), ScreenKind::Home, "nothing is kept");
    open_notes(&mut h);
    assert!(
        h.app
            .texts()
            .contains(&String::from(strings::EN.notes_none)),
        "and no note: {:?}",
        h.app.texts()
    );
}
