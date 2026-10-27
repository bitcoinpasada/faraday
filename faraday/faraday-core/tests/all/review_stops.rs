//! Review stops where a check keeps the person's money safe and has no
//! way past it, and offers a hold only where a decision names a case for
//! one (`docs/decisions/F136.md`): a fee this high is paid only after
//! "Hold to pay this fee"; a key origin that does not give the input's
//! key is refused at Review, as signing refuses it; an output that claims
//! to be change and cannot be verified is drawn as a payment.

use crate::common::{add_key, hold, offered, texts};
use faraday_core::strings::EN;
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, StorageEvent};
use faraday_psbt::Psbt;

fn kit(id: &str) -> Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

/// The spending wallet and its key loaded, and `psbt` open on Review.
fn reviewing(psbt: &Psbt) -> Faraday {
    reviewing_with(psbt, true)
}

/// The spending wallet loaded, its key too when `key`, and `psbt` open
/// on Review.
fn reviewing_with(psbt: &Psbt, key: bool) -> Faraday {
    let k = kit("spending");
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: vec![
            ("spending-wallet.txt".to_string(), k.descriptor.into_bytes()),
            ("spend.psbt".to_string(), psbt.to_bytes()),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    if key {
        add_key(&mut app, "bacon");
    }
    let at = app.inbox().iter().position(|i| i.name == "spend.psbt");
    app.press(Action::StartSpend(at.unwrap()));
    if app.spend().unwrap().open != Some(step::TRANSACTION) {
        app.press(Action::Step(step::TRANSACTION));
    }
    assert_eq!(app.spend().unwrap().open, Some(step::TRANSACTION));
    app
}

/// Everything the frame draws, as one line: a wrapped line reads whole.
fn screen(app: &mut Faraday) -> String {
    texts(app).join(" ")
}

fn count(app: &mut Faraday, want: &str) -> usize {
    texts(app).iter().filter(|t| *t == want).count()
}

#[test]
fn a_high_fee_is_paid_only_after_its_hold() {
    let mut psbt = testkit::unsigned(&kit("spending")).unwrap();
    // Three million sat less change: the fee is twice what it sends.
    psbt.inner_mut().unsigned_tx.output[1].value -=
        faraday_psbt::bitcoin::Amount::from_sat(3_000_000);
    let mut app = reviewing(&psbt);
    let shown = screen(&mut app);
    assert!(shown.contains(EN.step_fee_hold), "{shown}");
    assert!(shown.contains(EN.step_fee_hold_cases), "{shown}");
    assert!(!offered(&mut app, Action::StepNext(step::TRANSACTION)));
    // Continue and Sign before the hold: nothing goes on, nothing signs.
    app.press(Action::StepNext(step::TRANSACTION));
    let s = app.spend().unwrap();
    assert!(!s.done[step::TRANSACTION as usize]);
    assert_eq!(s.error.as_deref(), Some(EN.step_review_hold_first));
    app.press(Action::SignHere);
    assert!(app.spend().unwrap().spend.signed_here.is_empty());
    // Held: Review is done and the key signs.
    if app.spend().unwrap().open != Some(step::TRANSACTION) {
        app.press(Action::Step(step::TRANSACTION));
    }
    hold(&mut app, Action::ReviewHold);
    let s = app.spend().unwrap();
    assert!(s.done[step::TRANSACTION as usize]);
    assert!(!s.held.is_empty());
    app.press(Action::SignHere);
    assert!(!app.spend().unwrap().spend.signed_here.is_empty());
}

#[test]
fn a_key_origin_that_does_not_give_the_inputs_key_is_refused_at_review() {
    let mut psbt = testkit::unsigned(&kit("spending")).unwrap();
    // The same key named at another index of the account.
    for (_, path) in psbt.inner_mut().inputs[0].bip32_derivation.values_mut() {
        let at: Vec<_> = path.into_iter().copied().collect();
        let (last, rest) = at.split_last().unwrap();
        let other =
            faraday_psbt::bitcoin::bip32::ChildNumber::from_normal_idx(u32::from(*last) + 5)
                .unwrap();
        *path = rest.iter().copied().chain([other]).collect();
    }
    let mut app = reviewing(&psbt);
    assert!(app.spend().unwrap().inspection.refuses());
    let shown = screen(&mut app);
    assert!(shown.contains(EN.step_review_refused), "{shown}");
    assert!(!offered(&mut app, Action::StepNext(step::TRANSACTION)));
    assert!(!offered(&mut app, Action::ReviewHold));
    app.press(Action::StepNext(step::TRANSACTION));
    let s = app.spend().unwrap();
    assert!(!s.done[step::TRANSACTION as usize]);
    assert_eq!(s.error.as_deref(), Some(EN.step_review_refused));
    app.press(Action::SignHere);
    assert!(app.spend().unwrap().spend.signed_here.is_empty());
}

#[test]
fn change_that_cannot_be_verified_is_drawn_as_a_payment() {
    let psbt = testkit::unsigned(&kit("spending")).unwrap();
    let mut app = reviewing(&psbt);
    let (sends, change) = (
        count(&mut app, EN.step_sends),
        count(&mut app, EN.step_change),
    );
    // The change output's origin names a key that is not loaded.
    let mut claimed = psbt.clone();
    for (fp, _) in claimed.inner_mut().outputs[1].bip32_derivation.values_mut() {
        *fp = faraday_psbt::bitcoin::bip32::Fingerprint::from([9, 9, 9, 9]);
    }
    let mut app = reviewing(&claimed);
    assert!(matches!(
        app.spend().unwrap().inspection.outputs[1].kind,
        faraday_psbt::OutputKind::UnverifiedChange { .. }
    ));
    assert_eq!(count(&mut app, EN.step_sends), sends + 1);
    assert_eq!(count(&mut app, EN.step_change), change - 1);
    assert!(!texts(&mut app).iter().any(|t| t.contains("Unverified")));
}

/// With no key of the wallet here, Review goes on to collect the
/// signatures made elsewhere: no key here taking part is no refusal.
#[test]
fn with_no_key_here_review_goes_on_to_collect() {
    let psbt = testkit::unsigned(&kit("spending")).unwrap();
    let mut app = reviewing_with(&psbt, false);
    assert!(
        app.spend()
            .unwrap()
            .inspection
            .dangers()
            .contains(&faraday_psbt::WarningKind::NoParticipatingKey)
    );
    let shown = screen(&mut app);
    assert!(!shown.contains(EN.step_review_refused), "{shown}");
    assert!(offered(&mut app, Action::StepNext(step::TRANSACTION)));
    app.press(Action::StepNext(step::TRANSACTION));
    let s = app.spend().unwrap();
    assert!(s.done[step::TRANSACTION as usize]);
    assert_eq!(s.error, None);
}

/// The hold accepts a fee only on Review as it stands: on a transaction
/// nothing signs, or one with no fee to hold for, it does nothing.
#[test]
fn the_fee_hold_accepts_nothing_where_review_has_no_fee_to_hold() {
    let plain = testkit::unsigned(&kit("spending")).unwrap();
    let mut refused = plain.clone();
    for (_, path) in refused.inner_mut().inputs[0].bip32_derivation.values_mut() {
        let at: Vec<_> = path.into_iter().copied().collect();
        let (last, rest) = at.split_last().unwrap();
        let other =
            faraday_psbt::bitcoin::bip32::ChildNumber::from_normal_idx(u32::from(*last) + 5)
                .unwrap();
        *path = rest.iter().copied().chain([other]).collect();
    }
    for psbt in [plain, refused] {
        let mut app = reviewing(&psbt);
        app.press(Action::ReviewHold);
        let s = app.spend().unwrap();
        assert!(s.held.is_empty());
        assert!(!s.done[step::TRANSACTION as usize]);
    }
}
