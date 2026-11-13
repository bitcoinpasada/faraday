//! The Sign card's own button signs only once the steps before it are
//! done, whichever way the card was reached.

use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::step;
use faraday_core::{Action, StorageEvent};
use faraday_shell_api::{App, Event, Key};

fn kit() -> Kit {
    testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .unwrap()
}

#[test]
fn the_sign_card_opened_from_its_header_signs_nothing_until_the_steps_before_it_are_done() {
    let psbt = testkit::unsigned(&kit()).unwrap();
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("spend.psbt"), psbt.to_bytes()),
            (
                faraday_text::Text::of("spending-wallet.txt"),
                kit().descriptor.into_bytes(),
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.press(Action::LoadWallet(1));
    app.press(Action::Entry(None));
    for c in testkit::test_words("bacon").chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.press(Action::StartSpend(0));
    assert!(!app.spend().unwrap().done[step::TRANSACTION as usize]);
    app.press(Action::Step(step::SIGN));
    assert!(app.spend().unwrap().spend.signed_here.is_empty());
    app.press(Action::SignHere);
    let s = app.spend().unwrap();
    assert!(s.spend.signed_here.is_empty());
    let first = s
        .steps
        .iter()
        .copied()
        .find(|&n| !s.done[n as usize])
        .unwrap();
    assert_eq!(s.open, Some(first));
    app.press(Action::Primary);
    assert!(app.spend().unwrap().spend.signed_here.is_empty());
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::Step(step::SIGN));
    app.press(Action::SignHere);
    assert!(!app.spend().unwrap().spend.signed_here.is_empty());
}
