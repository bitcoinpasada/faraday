//! The fee bump on Sign a transaction's review (`docs/COMPARE.md` §4):
//! with Bitcoin Core's replacement open and its original in Files, the
//! Transaction step says whether only the fee is raised, from that file;
//! a file of the same transaction does not count; a file brought in
//! while the review is open does.
//!
//! The pairs are Bitcoin Core's own (`tools/vectors/psbt/`, the
//! `bump-core-*` files).

use crate::common::texts;
use faraday_core::compare::ComparePress;
use faraday_core::strings::{EN, fill};
use faraday_core::testkit;
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_psbt::Psbt;
use faraday_shell_api::App;

const ABANDON: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon \
                       abandon abandon about";

fn vector(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/../../tools/vectors/psbt/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The Core replacement with its payment's script another address's: the
/// bump-pay-other case.
fn pay_other() -> Vec<u8> {
    use faraday_bip::bitcoin::hashes::Hash;
    use faraday_bip::bitcoin::{PubkeyHash, ScriptBuf};
    let c = testkit::compute();
    let text = String::from_utf8(vector("bump-core-replacement.psbt")).unwrap();
    let mut psbt = Psbt::parse_base64(&c, text.trim()).unwrap();
    let other = ScriptBuf::new_p2pkh(&PubkeyHash::from_byte_array([7; 20]));
    // The payment is the output without a key origin of ours.
    let at = psbt
        .inner()
        .outputs
        .iter()
        .position(|o| o.bip32_derivation.is_empty() && o.tap_key_origins.is_empty())
        .expect("the payment");
    psbt.inner_mut().unsigned_tx.output[at].script_pubkey = other;
    psbt.to_base64().into_bytes()
}

fn files(list: &[(&str, Vec<u8>)]) -> Vec<(String, Vec<u8>)> {
    list.iter()
        .map(|(n, b)| (n.to_string(), b.clone()))
        .collect()
}

fn restore(app: &mut Faraday, list: &[(&str, Vec<u8>)]) {
    app.storage(StorageEvent::Restored {
        inbox: files(list),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
}

/// `name` open on Sign a transaction's Transaction step, with the
/// "abandon" key loaded, from the files in Files.
fn reviewing(list: &[(&str, Vec<u8>)], name: &str) -> Faraday {
    let mut app = testkit::started();
    restore(&mut app, list);
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    let c = testkit::compute();
    app.session.keys.add_words(&c, ABANDON, "", None).unwrap();
    open(&mut app, name);
    app
}

fn open(app: &mut Faraday, name: &str) {
    let at = app.inbox().iter().position(|i| i.name == name).unwrap();
    app.press(Action::StartSpend(at));
    assert!(app.spend().is_some(), "{:?}", texts(app));
    if app.spend().unwrap().open != Some(step::TRANSACTION) {
        app.press(Action::Step(step::TRANSACTION));
    }
    assert_eq!(app.spend().unwrap().open, Some(step::TRANSACTION));
}

fn shows(app: &mut Faraday, line: &str) -> bool {
    texts(app).iter().any(|t| t == line)
}

fn has_part(app: &mut Faraday, part: &str) -> bool {
    texts(app).iter().any(|t| t.contains(part))
}

fn pair() -> [(&'static str, Vec<u8>); 2] {
    [
        (
            "bump-core-replacement.psbt",
            vector("bump-core-replacement.psbt"),
        ),
        ("bump-core-original.psbt", vector("bump-core-original.psbt")),
    ]
}

#[test]
fn the_core_replacement_says_only_the_fee_is_raised_from_the_original() {
    let mut app = reviewing(&pair(), "bump-core-replacement.psbt");
    let result = fill(EN.bump_only_fee_from, &["bump-core-original.psbt"]);
    assert!(
        shows(&mut app, EN.compare_fee_bump_label),
        "{:?}",
        texts(&mut app)
    );
    assert!(shows(&mut app, &result), "{:?}", texts(&mut app));
    assert!(!has_part(&mut app, "More than the fee changed"));
}

#[test]
fn compare_on_the_review_opens_the_comparison_with_the_original_first_and_done_returns() {
    let mut app = reviewing(&pair(), "bump-core-replacement.psbt");
    // The button the review draws on the original.
    let _ = app.frame();
    let press = app
        .drawn_actions()
        .into_iter()
        .find(|a| matches!(a, Action::Compare(ComparePress::FromReview(..))))
        .expect("the review draws a Compare button");
    app.press(press);
    assert_eq!(app.screen, Screen::Compare);
    assert!(shows(&mut app, EN.compare_result), "{:?}", texts(&mut app));
    // The original is the one the replacement is judged against.
    assert!(shows(&mut app, EN.bump_only_fee), "{:?}", texts(&mut app));
    assert!(has_part(&mut app, "bump-core-original.psbt"));
    assert!(has_part(&mut app, "bump-core-replacement.psbt"));
    app.press(Action::Compare(ComparePress::Back));
    assert_eq!(app.screen, Screen::Spend);
    assert!(shows(&mut app, EN.compare_fee_bump_label));
}

#[test]
fn the_review_offers_compare_on_each_other_transaction() {
    let mut app = reviewing(&pair(), "bump-core-replacement.psbt");
    let n = texts(&mut app)
        .iter()
        .filter(|t| *t == EN.compare_from_review)
        .count();
    assert_eq!(n, 1);
}

#[test]
fn the_finished_original_counts_as_the_other_transaction() {
    let list = [
        (
            "bump-core-replacement.psbt",
            vector("bump-core-replacement.psbt"),
        ),
        ("bump-core-original.tx", vector("bump-core-original.tx")),
    ];
    let mut app = reviewing(&list, "bump-core-replacement.psbt");
    let result = fill(EN.bump_only_fee_from, &["bump-core-original.tx"]);
    assert!(shows(&mut app, &result), "{:?}", texts(&mut app));
}

#[test]
fn with_no_other_transaction_there_is_no_fee_bump() {
    let list = [(
        "bump-core-replacement.psbt",
        vector("bump-core-replacement.psbt"),
    )];
    let mut app = reviewing(&list, "bump-core-replacement.psbt");
    assert!(
        !shows(&mut app, EN.compare_fee_bump_label),
        "{:?}",
        texts(&mut app)
    );
}

#[test]
fn a_file_of_the_same_unsigned_transaction_does_not_count() {
    let list = [
        (
            "bump-core-replacement.psbt",
            vector("bump-core-replacement.psbt"),
        ),
        ("copy.psbt", vector("bump-core-replacement.psbt")),
    ];
    let mut app = reviewing(&list, "bump-core-replacement.psbt");
    assert!(
        !shows(&mut app, EN.compare_fee_bump_label),
        "{:?}",
        texts(&mut app)
    );
}

#[test]
fn a_file_that_spends_none_of_the_inputs_does_not_count() {
    let list = [
        (
            "bump-core-replacement.psbt",
            vector("bump-core-replacement.psbt"),
        ),
        ("demo.psbt", vector("demo-regtest.psbt")),
    ];
    let mut app = reviewing(&list, "bump-core-replacement.psbt");
    assert!(
        !shows(&mut app, EN.compare_fee_bump_label),
        "{:?}",
        texts(&mut app)
    );
}

#[test]
fn the_original_brought_in_while_the_review_is_open_adds_the_result() {
    let only = [(
        "bump-core-replacement.psbt",
        vector("bump-core-replacement.psbt"),
    )];
    let mut app = reviewing(&only, "bump-core-replacement.psbt");
    assert!(!shows(&mut app, EN.compare_fee_bump_label));
    restore(&mut app, &pair());
    let result = fill(EN.bump_only_fee_from, &["bump-core-original.psbt"]);
    assert!(shows(&mut app, &result), "{:?}", texts(&mut app));
    // And taken out again, it goes.
    restore(&mut app, &only);
    assert!(
        !shows(&mut app, EN.compare_fee_bump_label),
        "{:?}",
        texts(&mut app)
    );
}

#[test]
fn a_replacement_that_pays_another_address_shows_its_lines_and_signing_goes_on() {
    let list = [
        ("bump-pay-other.psbt", pay_other()),
        ("bump-core-original.psbt", vector("bump-core-original.psbt")),
    ];
    let mut app = reviewing(&list, "bump-pay-other.psbt");
    let result = fill(EN.bump_more_changed_from, &["bump-core-original.psbt"]);
    assert!(shows(&mut app, &result), "{:?}", texts(&mut app));
    assert!(
        has_part(&mut app, "No longer pays"),
        "{:?}",
        texts(&mut app)
    );
    assert!(has_part(&mut app, "which the original does not"));
    // The fee bump changes no other warning's level: nothing is refused.
    let s = app.spend().unwrap();
    assert!(s.refused.is_none(), "{:?}", s.refused);
    assert!(s.steps.contains(&step::SIGN));
}
