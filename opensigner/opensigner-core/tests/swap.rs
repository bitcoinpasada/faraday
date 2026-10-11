//! A coordinator handing back the wallet that was set up, with one
//! cosigner's key replaced by someone else's (UX.md E2).
//!
//! Every fingerprint a person recognises is still on the screen when the
//! origins are kept and only the extended public key is swapped, so what
//! the review has to say is which key changed and against which wallet.

mod common;

use common::{ABANDON, Harness, PANEL};
use opensigner_core::{ScreenKind, ids, strings};
use osk_bip::policy::WalletPolicy;
use osk_shell_api::{Event, FileKind};

/// The 2-of-3 wallet of the test key and two fixed cosigners
/// (`just psbt-fixtures`).
const POLICY: &str = include_str!("../../../tools/vectors/psbt/wallet-2of3.policy");

/// The third cosigner of that wallet: its master, and the account key
/// the policy carries for it.
const THIRD: &str = "b8688df1";
const THIRD_KEY: &str = "tpubDEfobrrtptRTbKf4gysDhoabneABDTAcdj3Vbn4XwPsLE2pmqpizSPRG6zHsbAMuiSgWmWPsYCLHTKTPpyrGJ5rAoTpKoQNZcxodiPf2tSJ";
/// The second cosigner's key, which the two-key test moves.
const SECOND_KEY: &str = "tpubDFPtPArj4GzBEFHohegg1Xatrc1Fi9oSox5LzuSRX91miwQxuUrEpBxpvDRsmZYJKYFhgdK3UStsjC8JKXfUbMinjFqiEM4uNwzVaCaHpys";
/// A key of no cosigner of that wallet, which stands in for the
/// attacker's.
const FOREIGN_KEY: &str = "tpubD6NzVbkrYhZ4XHndKkuB8FifXm8r5FQHwrN6oZuWCz13qb93rtgKvD4PQsqC4HP4yhV3tA2fqr2RbY5mNXfM7RxXUoeABoDtsFUq2zJq6YK";
/// The attacker's own master, where the swap does not keep the origin.
const ATTACKER: &str = "9c8d7e6f";

/// A mainnet account key of no master this device holds.
const FOREIGN_XPUB: &str = "xpub6ERApfZwUNrhLCkDtcHTcxd75RbzS1ed54G1LkBUHQVHQKqhMkhgbmJbZRkrgZw4koxb5JaHWkY4ALHY2grBGRjaDMzQLcgJvLJuZZvRcEL";

/// Wallets › "Load a wallet", and `text` read from a file there: the
/// review, before anything is added.
fn review(h: &mut Harness, text: &str) {
    h.open_add_wallet(ids::WALLETS_LOAD);
    h.send(Event::CameraUnavailable);
    h.tap(ids::SCAN_FILE);
    h.send(Event::File {
        kind: FileKind::Any,
        bytes: text.as_bytes().to_vec(),
    });
    assert_eq!(h.app.screen(), ScreenKind::Inspect);
}

/// The same, taken as far as the wallet being in use.
fn load(h: &mut Harness, text: &str) {
    review(h, text);
    h.tap(ids::INSPECT_USE_WALLET);
}

/// Whether the screen states `text` anywhere.
fn says(h: &Harness, text: &str) -> bool {
    h.app.texts().iter().any(|t| t == text)
}

/// The card's title, which is on the screen whenever a swap is found.
fn title() -> &'static str {
    strings::EN.inspect_swap
}

/// How many policy wallets Home lists.
fn wallets(h: &mut Harness) -> usize {
    h.open_wallets();
    (0..8)
        .take_while(|i| {
            h.app
                .rect_of(ids::at(ids::WALLETS_POLICY_ROW_BASE, *i))
                .is_some()
        })
        .count()
}

/// One cosigner's key replaced under the origin that was there: every
/// fingerprint is the one a person knows, so the card says the
/// fingerprint is the same and the key is not.
#[test]
fn a_key_swapped_under_the_same_origin_is_named_on_the_review() {
    let mut h = Harness::new(PANEL);
    load(&mut h, POLICY);
    review(&mut h, &POLICY.replace(THIRD_KEY, FOREIGN_KEY));

    assert!(says(&h, title()), "{:?}", h.app.texts());
    let quorum = strings::fill(strings::EN.wallet_quorum, &["2", "3"]);
    let wallet = WalletPolicy::parse(POLICY).expect("the fixture parses");
    let card = format!(
        "{quorum} \u{00b7} {} \u{00b7} {} \u{00b7} {}",
        strings::EN.script_segwit,
        wallet.checksum(),
        strings::fill1(strings::EN.inspect_swap_same, THIRD),
    );
    assert!(
        says(&h, &card),
        "the wallet in use and the key that changed, want {card:?}, got {:?}",
        h.app.texts()
    );

    assert_eq!(
        h.app.hold_buttons(),
        vec![ids::INSPECT_USE_WALLET],
        "a danger card makes adding the wallet a hold"
    );
    h.hold(ids::INSPECT_USE_WALLET);
    assert_eq!(wallets(&mut h), 2, "the person may have changed a cosigner");
}

/// A key replaced by one whose origin is the attacker's own master: the
/// card names the fingerprint that went and the one that came.
#[test]
fn a_key_swapped_with_a_new_origin_names_both_masters() {
    let mut h = Harness::new(PANEL);
    load(&mut h, POLICY);
    let swapped = POLICY
        .replace(THIRD_KEY, FOREIGN_KEY)
        .replace(&format!("[{THIRD}/"), &format!("[{ATTACKER}/"));
    review(&mut h, &swapped);

    assert!(says(&h, title()), "{:?}", h.app.texts());
    assert!(
        h.app.texts().iter().any(|t| t.contains(&strings::fill(
            strings::EN.inspect_swap_replaced,
            &[THIRD, ATTACKER]
        ))),
        "both masters: {:?}",
        h.app.texts()
    );
}

/// Two keys apart is a different wallet, not a swap of this one.
#[test]
fn two_keys_apart_is_a_different_wallet() {
    let mut h = Harness::new(PANEL);
    load(&mut h, POLICY);
    let other = POLICY
        .replace(SECOND_KEY, "@second@")
        .replace(THIRD_KEY, SECOND_KEY)
        .replace("@second@", THIRD_KEY);
    review(&mut h, &other);

    assert!(!says(&h, title()), "{:?}", h.app.texts());
    assert!(h.app.hold_buttons().is_empty(), "adding it is a tap");
}

/// The wallet in use itself: nothing differs, so nothing is said, and
/// the review opens the row it already has.
#[test]
fn the_wallet_in_use_carries_no_card() {
    let mut h = Harness::new(PANEL);
    load(&mut h, POLICY);
    review(&mut h, POLICY);

    assert!(!says(&h, title()), "{:?}", h.app.texts());
    assert!(h.app.hold_buttons().is_empty());
    h.tap(ids::INSPECT_FORGET_WALLET);
    assert_eq!(wallets(&mut h), 0, "the row the review opened is the one");
}

/// With no wallet in use there is nothing to differ from.
#[test]
fn a_review_with_no_wallet_in_use_carries_no_card() {
    let mut h = Harness::new(PANEL);
    review(&mut h, POLICY);

    assert!(!says(&h, title()), "{:?}", h.app.texts());
    assert!(h.app.hold_buttons().is_empty());
}

/// A single key that names a master this device holds and is not the key
/// that master derives at that path.
#[test]
fn a_key_claiming_this_device_and_not_being_it_is_refused_by_name() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    review(
        &mut h,
        &format!("wpkh([73c5da0a/84'/0'/0']{FOREIGN_XPUB}/<0;1>/*)"),
    );

    assert!(
        says(
            &h,
            &strings::fill1(strings::EN.inspect_swap_claims, "73c5da0a")
        ),
        "{:?}",
        h.app.texts()
    );
    assert_eq!(h.app.hold_buttons(), vec![ids::INSPECT_USE_WALLET]);
}

/// The device's own key, read back in: the origin names it and the key
/// is the one it derives, so the review says nothing.
#[test]
fn the_devices_own_account_key_claims_nothing() {
    let mut h = Harness::new(PANEL);
    h.start_load(&ABANDON);
    h.finish_load(None);
    let zpub = "zpub6rFR7y4Q2AijBEqTUquhVz398htDFrtymD9xYYfG1m4wAcvPhXNfE3EfH1r1ADqtfSdVCToUG868RvUUkgDKf31mGDtKsAYz2oz2AGutZYs";
    let xpub = osk_bip::slip132::decode_xpub(zpub)
        .expect("a SLIP-132 key")
        .0;
    review(&mut h, &format!("wpkh([73c5da0a/84'/0'/0']{xpub}/<0;1>/*)"));

    assert!(
        !says(
            &h,
            &strings::fill1(strings::EN.inspect_swap_claims, "73c5da0a")
        ),
        "{:?}",
        h.app.texts()
    );
    assert!(h.app.hold_buttons().is_empty());
}
