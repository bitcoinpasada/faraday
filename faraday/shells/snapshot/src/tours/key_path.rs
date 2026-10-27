//! The key path tour (F132): a taproot multisig wallet whose key path a
//! key not of the wallet spends, from Files to its sheet, its card and
//! its spend's check; then the same wallet over the NUMS point.

use crate::tour::Tour;
use faraday_core::testkit::{self, Kit};
use faraday_core::wallet::step;
use faraday_core::{Action, Screen, StorageEvent};

fn kit(internal: &str) -> Kit {
    let p = "m/48'/1'/0'/3'";
    Kit {
        id: "shared",
        name: "Shared",
        descriptor: format!(
            "tr({internal},multi_a(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
            testkit::key(0, p),
            testkit::key(1, p),
            testkit::key(2, p)
        ),
        cosigner: None,
        fee: 1_820,
    }
}

pub(crate) fn key_path_tour(t: &mut Tour) -> Result<(), String> {
    let other = kit("79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798");
    let nums = kit("50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0");
    let unsigned = testkit::unsigned(&other)?;
    t.app.storage(StorageEvent::Restored {
        inbox: vec![
            (
                "shared-wallet.txt".to_string(),
                other.descriptor.clone().into_bytes(),
            ),
            ("shared-unsigned.psbt".to_string(), unsigned.to_bytes()),
            (
                "nums-wallet.txt".to_string(),
                nums.descriptor.clone().into_bytes(),
            ),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let at = |t: &Tour, name: &str| {
        t.app
            .inbox()
            .iter()
            .position(|i| i.name == name)
            .ok_or(format!("{name} is not in Files"))
    };
    t.press(Action::Nav(Screen::Files));
    t.press(Action::LoadWallet(at(t, "shared-wallet.txt")?));
    t.shot("keypath-sheet")?;
    t.press(Action::KeyPathHold);
    for n in 0..2 {
        t.press(Action::Entry(None));
        t.type_key(n);
    }
    t.press(Action::Nav(Screen::Wallets));
    t.shot("keypath-card")?;
    t.press(Action::StartSpend(at(t, "shared-unsigned.psbt")?));
    t.press(Action::Step(step::TRANSACTION));
    t.shot("keypath-spend-check")?;
    t.press(Action::LoadWallet(at(t, "nums-wallet.txt")?));
    t.press(Action::Nav(Screen::Wallets));
    t.shot("keypath-nums-card")?;
    Ok(())
}
