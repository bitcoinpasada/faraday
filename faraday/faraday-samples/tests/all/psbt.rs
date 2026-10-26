//! The sample transaction (`docs/MODES.md` §6): every scanner that takes
//! a transaction offers it, the Spend tab's steps that ask for the
//! transaction have a Load sample PSBT button, and a person goes on from
//! there to a signed transaction.

use crate::common::demo_app;
use faraday_core::family::{FamilyAction as F, page};
use faraday_core::samples::Pick;
use faraday_core::{Action, Faraday, Screen, Sheet};
use faraday_samples::SAMPLES;
use faraday_shell_api::App;

fn psbt_files(app: &Faraday) -> usize {
    app.inbox()
        .iter()
        .filter(|i| i.name.ends_with(".psbt"))
        .count()
}

/// Practice with Savings loaded from the Load samples sheet.
pub(crate) fn with_savings() -> Faraday {
    let mut app = demo_app();
    app.press(Action::Nav(Screen::Home));
    let _ = app.frame();
    app.press(Action::Samples);
    let _ = app.frame();
    let at = SAMPLES
        .wallets
        .iter()
        .position(|w| w.name == "sample_savings")
        .expect("Savings");
    app.press(Action::SampleTick(Pick::Wallet(at as u8)));
    let _ = app.frame();
    app.press(Action::SamplesLoad);
    let _ = app.frame();
    assert_eq!(app.session.wallets.len(), 1);
    app
}

#[test]
fn home_scanner_offers_the_sample_psbt_and_one_press_closes_it() {
    let mut app = with_savings();
    app.press(Action::Nav(Screen::Home));
    app.press(Action::Scan);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(crate::common::strip_offers(&mut app, Action::ScanSample));
    assert!(
        crate::common::strip(&mut app)
            .iter()
            .any(|t| t == "Load sample PSBT")
    );
    app.press(Action::ScanSample);
    assert_eq!(app.sheet, None, "the scanner closed");
    assert_eq!(psbt_files(&app), 1);
    // Again: the same transaction is the file already in Files.
    app.press(Action::Scan);
    let _ = app.frame();
    app.press(Action::ScanSample);
    assert_eq!(app.sheet, None);
    assert_eq!(psbt_files(&app), 1);
}

#[test]
fn the_scanner_of_the_write_step_offers_it_too() {
    let mut app = with_savings();
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::SpendFrom(0)));
    let _ = app.frame();
    app.press(Action::Scan);
    let _ = app.frame();
    assert!(crate::common::strip_offers(&mut app, Action::ScanSample));
    app.press(Action::ScanSample);
    assert_eq!(app.sheet, None);
    assert!(app.spend().is_some(), "the review opened");
}

#[test]
fn the_spend_tab_loads_the_sample_psbt_reviews_and_signs_it() {
    let mut app = with_savings();
    app.press(Action::Nav(Screen::Family));
    app.press(Action::Family(F::SpendFrom(0)));
    let _ = app.frame();
    assert!(
        crate::common::strip_offers(&mut app, Action::SamplePsbt),
        "Load sample PSBT"
    );
    assert!(
        crate::common::strip(&mut app)
            .iter()
            .any(|t| t == "Load sample PSBT")
    );
    app.press(Action::SamplePsbt);
    let _ = app.frame();
    let s = app.spend().expect("the spend opened");
    assert_eq!(s.wallet, Some(0));
    app.press(Action::SignHere);
    let s = app.spend().unwrap();
    assert!(s.spend.finished.is_some(), "not signed: {:?}", s.error);
    // Pressed again, no second copy.
    assert_eq!(psbt_files(&app), 1);
}

/// Wallets, Savings, its spend: the first step's "Scan it", the sample
/// tile on the scanner, and the review and the signing after it.
fn signed_through(scan_button: impl Fn(&mut Faraday)) {
    let mut app = with_savings();
    app.press(Action::Nav(Screen::Wallets));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    app.press(Action::Family(F::SpendFrom(0)));
    let _ = app.frame();
    scan_button(&mut app);
    let _ = app.frame();
    assert_eq!(app.sheet, Some(Sheet::Scan));
    assert!(
        crate::common::strip_offers(&mut app, Action::ScanSample),
        "the sample tile"
    );
    assert!(
        crate::common::strip(&mut app)
            .iter()
            .any(|t| t == "Load sample PSBT")
    );
    app.press(Action::ScanSample);
    let _ = app.frame();
    assert_eq!(app.sheet, None, "the scanner closed");
    assert_eq!(app.screen, Screen::Family);
    let s = app.spend().expect("the spend opened");
    assert_eq!(s.wallet, Some(0));
    assert_eq!(s.open, Some(faraday_core::wallet::step::TRANSACTION));
    app.press(Action::SignHere);
    let s = app.spend().unwrap();
    assert!(s.spend.finished.is_some(), "not signed: {:?}", s.error);
    assert!(s.spend.signed_here.len() >= 2, "two of three signed");
    assert_eq!(psbt_files(&app), 1);
}

#[test]
fn from_the_first_steps_scan_it_the_sample_signs_on_the_wallets_path() {
    signed_through(|app| {
        let texts = app.drawn_texts();
        assert!(
            texts
                .iter()
                .any(|t| t == "I have the PSBT from Sparrow ready: Scan it"),
            "{texts:?}"
        );
        app.press(Action::Scan);
    });
}

#[test]
fn from_the_write_steps_scan_the_psbt_the_sample_signs_on_the_wallets_path() {
    signed_through(|app| {
        app.press(Action::Family(F::Next(page::CHECK)));
        let _ = app.frame();
        assert!(app.drawn_texts().iter().any(|t| t == "Scan the PSBT"));
        app.press(Action::Scan);
    });
}

#[test]
fn with_no_wallet_loaded_the_sample_psbt_still_comes_in_and_says_what_is_missing() {
    let mut app = demo_app();
    app.press(Action::Nav(Screen::Family));
    let _ = app.frame();
    app.press(Action::Family(F::Card(page::HOLDING)));
    let _ = app.frame();
    assert!(
        crate::common::strip_offers(&mut app, Action::SamplePsbt),
        "Load sample PSBT"
    );
    app.press(Action::SamplePsbt);
    let _ = app.frame();
    assert_eq!(psbt_files(&app), 1);
    assert!(app.session.keys.is_empty());
    assert!(app.session.wallets.is_empty());
    let texts = app.drawn_texts();
    assert!(texts.iter().any(|t| t.contains("transaction")), "{texts:?}");
}
