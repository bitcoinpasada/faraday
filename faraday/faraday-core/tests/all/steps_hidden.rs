//! A step flow draws no step past the current one: Make a new wallet opens
//! with Kind alone, a kind pressed leaves Kind above, still pressable, and
//! the next step open with nothing after it offered; New key the same.

use faraday_core::keygen::Start;
use faraday_core::{Action, Faraday, Screen, cstep, testkit};
use faraday_shell_api::App;

/// Whether each of `cards` is offered, in order.
fn offered(app: &mut Faraday, cards: &[u8], press: fn(u8) -> Action) -> Vec<bool> {
    let _ = app.frame();
    cards.iter().map(|&k| app.offers(press(k))).collect()
}

/// Whether Kind's first row is offered, as the flow draws now.
fn kind_row(app: &mut Faraday) -> bool {
    let _ = app.frame();
    app.offers(Action::CKind(0))
}

#[test]
fn make_a_new_wallet_opens_with_kind_alone() {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    assert!(kind_row(&mut app), "Kind's choices are on the page");
    assert_eq!(app.create_cards(), vec![cstep::KIND]);
    // No step, closed or not, is listed on the page.
    for k in [cstep::QUORUM, cstep::KEYS, cstep::CHECK, cstep::BACKUP_FORM] {
        assert!(!crate::common::offered(&mut app, Action::CStep(k)));
    }
}

#[test]
fn make_a_new_wallet_after_a_kind_shows_the_open_step_only() {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::CKind(0));
    let open = app.create().and_then(|c| c.open).expect("a step is open");
    // Backup is the open step and is the last one there is for now.
    assert_eq!(open, cstep::BACKUP_FORM);
    assert_eq!(app.create_cards().last(), Some(&cstep::BACKUP_FORM));
    assert!(!kind_row(&mut app), "Kind is no longer on the page");
    let _ = app.frame();
    assert!(app.offers(Action::CBackupForm(
        faraday_core::create::BackupForm::Words.index()
    )));
    // Kind can be changed from Your choices, and stands as it was.
    app.press(Action::Choices(faraday_core::ChoicesPress::Row(
        cstep::KIND,
    )));
    assert_eq!(app.create().and_then(|c| c.open), Some(cstep::KIND));
    assert!(kind_row(&mut app));
    // With Backup answered, Keys is open and Kind and Backup are off the
    // page.
    app.press(Action::CKind(0));
    app.press(Action::CBackupForm(
        faraday_core::create::BackupForm::Words.index(),
    ));
    let open = app.create().and_then(|c| c.open).expect("a step is open");
    assert_eq!(open, cstep::KEYS);
    assert!(!kind_row(&mut app));
    assert_eq!(app.create_cards().last(), Some(&cstep::CHECK));
}

#[test]
fn new_key_shows_its_open_step_and_none_after() {
    let mut app = testkit::started();
    app.press(Action::Entry(None));
    app.press(Action::KeyGen(None));
    assert_eq!(app.screen, Screen::KeyGen);
    let all: Vec<u8> = app.keygen().unwrap().steps();
    let open = app.keygen().unwrap().open.expect("a step is open");
    let at = all.iter().position(|&k| k == open).unwrap();
    let shown = offered(&mut app, &all, Action::KStep);
    for (i, on) in shown.iter().enumerate() {
        assert_eq!(*on, i <= at, "step {i} with step {at} open");
    }
    app.press(Action::KStart(Start::Make.index()));
    let all: Vec<u8> = app.keygen().unwrap().steps();
    let open = app.keygen().unwrap().open.expect("a step is open");
    let at = all.iter().position(|&k| k == open).unwrap();
    let shown = offered(&mut app, &all, Action::KStep);
    assert!(shown[0], "Start stays above, to change");
    for (i, on) in shown.iter().enumerate() {
        assert_eq!(*on, i <= at, "step {i} with step {at} open");
    }
    assert!(at + 1 < all.len(), "a later step is left unshown");
}

/// Sign a transaction: a step after the one reached is neither offered nor
/// drawn, and a finished step above can be pressed.
#[test]
fn sign_a_transaction_shows_no_step_past_the_current_one() {
    use faraday_core::StorageEvent;
    use faraday_core::wallet::step;
    let k = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap();
    let (psbt, _) = testkit::spend().unwrap();
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (
                faraday_text::Text::of("spending-wallet.txt"),
                k.descriptor.into_bytes(),
            ),
            (faraday_text::Text::of("spend.psbt"), psbt.to_bytes()),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(0));
    crate::common::add_key(&mut app, "bacon");
    let at = app.inbox().iter().position(|i| i.name == "spend.psbt");
    app.press(Action::StartSpend(at.unwrap()));
    assert_eq!(app.screen, Screen::Spend);
    let steps = app.spend().unwrap().steps.clone();
    let shown = |app: &mut Faraday| -> Vec<bool> {
        let _ = app.frame();
        steps.iter().map(|&n| app.offers(Action::Step(n))).collect()
    };
    // The spend as it opens: nothing after the step reached.
    let first = shown(&mut app);
    let last = first.iter().rposition(|&on| on).expect("a step is shown");
    assert!(last + 1 < steps.len(), "a later step is left unshown");
    assert!(first[..=last].iter().all(|&on| on), "{first:?}");
    // With the steps before Sign done, Your choices offers each of them;
    // Sign is open, so it takes no press, and nothing after it is offered.
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::Step(step::SIGN));
    let at_sign = shown(&mut app);
    let sign = steps.iter().position(|&n| n == step::SIGN).unwrap();
    assert!(at_sign[..sign].iter().all(|&on| on), "{at_sign:?}");
    assert!(at_sign[sign..].iter().all(|&on| !on), "{at_sign:?}");
    app.press(Action::Step(step::TRANSACTION));
    assert_eq!(app.spend().unwrap().open, Some(step::TRANSACTION));
}
