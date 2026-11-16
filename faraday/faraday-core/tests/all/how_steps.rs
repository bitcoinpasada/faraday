//! How Faraday does it on a flow of steps (`docs/CRYPTO-LEARN.md` §8.1):
//! Spend's Cryptography tab holds the actions offered on the open step
//! and the steps before it, those of the open step first, and an action
//! of a later step shows only once that step is reached.

use crate::common::add_key;
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, StorageEvent};

fn kit(id: &str) -> Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

/// The spending wallet and its key loaded, its transaction open on Spend.
fn spending() -> Faraday {
    let k = kit("spending");
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
    add_key(&mut app, "bacon");
    let at = app.inbox().iter().position(|i| i.name == "spend.psbt");
    app.press(Action::StartSpend(at.unwrap()));
    app
}

fn ids(app: &Faraday) -> Vec<&'static str> {
    app.how_blocks().iter().map(|b| b.id).collect()
}

#[test]
fn signing_actions_show_from_the_sign_step_and_the_open_steps_come_first() {
    let mut app = spending();
    if app.spend().unwrap().open != Some(step::TRANSACTION) {
        app.press(Action::Step(step::TRANSACTION));
    }
    assert_eq!(app.spend().unwrap().open, Some(step::TRANSACTION));
    let at_review = ids(&app);
    assert!(at_review.contains(&"psbt-review"), "{at_review:?}");
    assert!(!at_review.contains(&"sign-ecdsa"), "{at_review:?}");
    assert!(app.how_offered());

    testkit::steps_before_sign_done(&mut app);
    if app.spend().unwrap().open != Some(step::SIGN) {
        app.press(Action::Step(step::SIGN));
    }
    assert_eq!(app.spend().unwrap().open, Some(step::SIGN));
    let at_sign = ids(&app);
    assert!(at_sign.contains(&"psbt-review"), "{at_sign:?}");
    let sign = at_sign.iter().position(|&i| i == "sign-ecdsa");
    let review = at_sign.iter().position(|&i| i == "psbt-review");
    assert!(sign.is_some() && sign < review, "{at_sign:?}");
}
