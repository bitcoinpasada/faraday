//! A wallet removed while a transaction is open: the spend keeps the
//! wallet it is from, and closes when that wallet is the one removed.

use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, StorageEvent};

fn kit(id: &str) -> Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

const WALLETS: [&str; 3] = ["savings", "spending", "nested"];

fn at(app: &Faraday, name: &str) -> usize {
    app.inbox().iter().position(|i| i.name == name).unwrap()
}

/// Savings, Spending and Nested loaded, in that order, with a spend of
/// each in the Inbox and the one of Spending open on its Check card.
fn open_spend_of_the_middle_wallet() -> Faraday {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    let mut inbox: Vec<(String, Vec<u8>)> = WALLETS
        .iter()
        .map(|id| testkit::wallet_file(&kit(id)))
        .collect();
    for id in ["spending", "nested"] {
        let psbt = testkit::unsigned(&kit(id)).unwrap();
        inbox.push((format!("{id}.psbt"), psbt.to_bytes()));
    }
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    for id in WALLETS {
        let file = testkit::wallet_file(&kit(id)).0;
        app.press(Action::LoadWallet(at(&app, &file)));
    }
    assert_eq!(app.session.wallets.len(), 3);
    app.press(Action::StartSpend(at(&app, "spending.psbt")));
    app
}

fn names(app: &Faraday) -> Vec<String> {
    app.session
        .wallets
        .iter()
        .map(|w| w.checksum().to_string())
        .collect()
}

#[test]
fn removing_the_first_wallet_leaves_the_spend_on_its_own_and_continue_checks_that_one() {
    let mut app = open_spend_of_the_middle_wallet();
    let before = names(&app);
    let own = {
        let s = app.spend().expect("a spend is open");
        before[s.wallet.expect("the spend names its wallet")].clone()
    };
    assert_eq!(own, before[1], "the spend is from the middle wallet");
    app.press(Action::RemoveWallet(0));
    let after = names(&app);
    assert_eq!(after, before[1..], "the first wallet is gone");
    let s = app.spend().expect("the spend stays open");
    assert_eq!(after[s.wallet.expect("still names a wallet")], own);
    app.press(Action::StepNext(step::CHECK));
    // Continue marked the middle wallet's Check, and no other.
    app.press(Action::StartSpend(at(&app, "spending.psbt")));
    assert!(app.spend().unwrap().done[step::CHECK as usize]);
    app.press(Action::StartSpend(at(&app, "nested.psbt")));
    assert!(!app.spend().unwrap().done[step::CHECK as usize]);
}

#[test]
fn removing_the_spends_own_wallet_closes_the_spend() {
    let mut app = open_spend_of_the_middle_wallet();
    let own = app.spend().unwrap().wallet.expect("a wallet");
    app.press(Action::RemoveWallet(own));
    assert!(app.spend().is_none(), "the spend is closed");
}
