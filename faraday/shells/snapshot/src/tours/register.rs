//! The register tour: a spend from Savings with no wallet registered,
//! its PSBT carrying the cosigners' extended keys. The Transaction card
//! with the blocked line and **Register this wallet**, the sheet, the
//! sheet with a refusal (a PSBT whose third key is another account's),
//! and the spend once registered; then the same spend without the keys,
//! which says to load the descriptor.

use crate::tour::{Tour, scroll, scroll_to};
use faraday_bip::policy::WalletPolicy;
use faraday_core::testkit;
use faraday_core::wallet::step;
use faraday_core::{Action, Screen, StorageEvent};

fn open_transaction(t: &mut Tour) {
    if t.app
        .spend()
        .is_some_and(|s| s.open != Some(step::TRANSACTION))
    {
        t.press(Action::Step(step::TRANSACTION));
    }
}

pub(crate) fn register_tour(t: &mut Tour) -> Result<(), String> {
    let c = testkit::compute();
    let kit = testkit::kits().remove(0);
    let with = testkit::unsigned_with_xpubs(&kit)?;
    let without = testkit::unsigned(&kit)?;
    // The third key's entry carries another account's key under the
    // origin the input states.
    let mut wrong = with.clone();
    let policy = WalletPolicy::parse_any(&c, &kit.descriptor).map_err(|e| format!("{e:?}"))?;
    let other = WalletPolicy::parse_any(
        &c,
        &format!("wpkh({}/<0;1>/*)", testkit::key(2, "m/48'/1'/1'/2'")),
    )
    .map_err(|e| format!("{e:?}"))?;
    let xpubs = &mut wrong.inner_mut().xpub;
    let origin = xpubs
        .remove(policy.keys()[2].xpub())
        .ok_or("the third key is in the global map")?;
    xpubs.insert(*other.keys()[0].xpub(), origin);
    t.app.storage(StorageEvent::Restored {
        inbox: vec![
            ("savings-unsigned.psbt".to_string(), with.to_bytes()),
            ("savings-bare.psbt".to_string(), without.to_bytes()),
            ("savings-wrong.psbt".to_string(), wrong.to_bytes()),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.press(Action::Entry(None));
    t.type_key(0);
    t.press(Action::Nav(Screen::Files));

    t.press(Action::StartSpend(2));
    t.press(Action::RegisterOpen);
    t.press(Action::RegisterHold);
    t.shot("register-refused")?;
    t.press(Action::RegisterCancel);

    t.press(Action::StartSpend(0));
    open_transaction(t);
    scroll_to(t, Action::RegisterOpen);
    t.shot("register-blocked")?;
    t.press(Action::RegisterOpen);
    t.shot("register-sheet")?;
    t.press(Action::RegisterField);
    t.shot("register-sheet-typing")?;
    t.press(Action::RegisterHold);
    open_transaction(t);
    t.shot("register-registered")?;

    t.press(Action::RemoveWallet(0));
    t.press(Action::StartSpend(1));
    open_transaction(t);
    for _ in 0..3 {
        scroll(t, 120);
    }
    t.shot("register-no-xpubs")?;
    Ok(())
}
