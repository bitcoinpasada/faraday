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

#[test]
fn make_a_new_wallet_opens_with_kind_alone() {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    assert_eq!(app.screen, Screen::Create);
    assert!(crate::common::offered(&mut app, Action::CStep(cstep::KIND)));
    for k in [cstep::QUORUM, cstep::KEYS, cstep::CHECK, cstep::BACKUP_FORM] {
        assert!(!crate::common::offered(&mut app, Action::CStep(k)));
    }
}

#[test]
fn make_a_new_wallet_after_a_kind_shows_kind_and_the_open_step_only() {
    let mut app = testkit::started();
    app.press(Action::CreateWallet);
    app.press(Action::CKind(0));
    let cards = app.create_cards();
    let open = app.create().and_then(|c| c.open).expect("a step is open");
    let at = cards.iter().position(|&k| k == open).unwrap();
    assert!(at >= 1, "the open step follows Kind");
    let shown = offered(&mut app, &cards, Action::CStep);
    for (i, on) in shown.iter().enumerate() {
        assert_eq!(
            *on,
            i <= at,
            "card {i} of {} with step {at} open",
            cards.len()
        );
    }
    assert!(at + 1 < cards.len(), "a later step is left unshown");
    // Kind can be changed from where it stands.
    app.press(Action::CStep(cstep::KIND));
    let shown = offered(&mut app, &cards, Action::CStep);
    assert!(shown[0]);
    assert!(shown[at], "the step already reached stays");
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
