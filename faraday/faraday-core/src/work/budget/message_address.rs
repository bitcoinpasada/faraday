//! Sign a message works its address out once, at the press that chooses the
//! wallet or the number (`docs/SPEED.md` §3.3); drawing it works out
//! nothing.

use crate::work::{Work, since, snapshot};
use crate::{Action, Faraday, mstep, testkit};
use faraday_shell_api::App;

fn app() -> Faraday {
    let c = testkit::compute();
    let mut app = testkit::started();
    app.session = testkit::session();
    app.session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .expect("the key");
    for id in ["spending", "taproot"] {
        let kit = testkit::kits().into_iter().find(|k| k.id == id).unwrap();
        app.session
            .add_wallet_confirmed(
                &c,
                kit.name,
                &kit.descriptor,
                "test",
                crate::wallet::Origin::Named,
            )
            .expect("the wallet");
    }
    app.press(Action::SignMessage);
    let _ = app.frame();
    app
}

#[test]
fn choosing_a_wallet_or_a_number_derives_one_address_and_drawing_none() {
    let mut app = app();
    for a in [Action::MWallet(1), Action::MIndex(1), Action::MIndex(1)] {
        let before = snapshot();
        app.press(a);
        assert_eq!(since(&before).of(Work::Address), 1, "{a:?}, the press");
        // The address card is drawn only on its own step (a wallet chosen
        // moves on), and a derivation made while drawing counts only while
        // the wallet has not kept the address.
        app.press(Action::MStep(mstep::ADDRESS));
        for w in &app.session.wallets {
            w.forget_addresses();
        }
        let before = snapshot();
        let _ = app.frame();
        app.out.redraw();
        let _ = app.frame();
        assert_eq!(since(&before).of(Work::Address), 0, "{a:?}, drawing");
        assert!(app.offers(Action::MIndex(1)), "{a:?}, the card is drawn");
    }
}
