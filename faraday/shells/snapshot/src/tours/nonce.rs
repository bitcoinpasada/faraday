//! The nonce tour: Faraday as a check on another signer (`docs/FLOWS.md`,
//! Nonce check). Files with a PSBT signed elsewhere and a finished
//! transaction, each offering Nonce check; the check of the PSBT with no
//! key here, then with the cosigner's key loaded (recomputed); the
//! finished transaction beside the PSBT that states its amounts, and its
//! hex; and the PSBT with a signature damaged, refused.

use crate::tour::{Tour, scroll_to};
use faraday_core::testkit;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_shell_api::{App, Event, Key};

/// Costs a spend signed and finished. Savings' spend signed with test
/// key 1 here and its cosigner's copy collected, as the finished
/// transaction's hex.
fn finished_hex(
    kit: &testkit::Kit,
    unsigned: Vec<u8>,
    cosigner: Vec<u8>,
) -> Result<String, String> {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: vec![
            ("spend.psbt".to_string(), unsigned),
            (
                "wallet.txt".to_string(),
                kit.descriptor.clone().into_bytes(),
            ),
            ("cosigner.psbt".to_string(), cosigner),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let at = |app: &Faraday, n: &str| app.inbox().iter().position(|i| i.name == n);
    app.press(Action::LoadWallet(
        at(&app, "wallet.txt").ok_or("no wallet")?,
    ));
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    app.press(Action::StartSpend(
        at(&app, "spend.psbt").ok_or("no spend")?,
    ));
    app.press(Action::SignHere);
    app.press(Action::Collect(at(&app, "cosigner.psbt").ok_or("no copy")?));
    app.spend()
        .and_then(|s| s.spend.finished_hex())
        .ok_or_else(|| "the spend did not finish".to_string())
}

pub(crate) fn nonce_tour(t: &mut Tour) -> Result<(), String> {
    let kit = testkit::kits().remove(0);
    let (unsigned, cosigner) = testkit::spend()?;
    let hex = finished_hex(&kit, unsigned.to_bytes(), cosigner.to_bytes())?;
    // The cosigner's copy with its first signature changed by one bit.
    let mut wrong = cosigner.clone();
    {
        use faraday_bip::bitcoin::secp256k1::ecdsa::Signature;
        let sig = wrong
            .inner_mut()
            .inputs
            .first_mut()
            .and_then(|i| i.partial_sigs.values_mut().next())
            .ok_or("the copy carries no signature")?;
        let mut compact = sig.signature.serialize_compact();
        compact[63] ^= 1;
        sig.signature = Signature::from_compact(&compact).map_err(|e| e.to_string())?;
    }
    t.app.storage(StorageEvent::Restored {
        inbox: vec![
            ("savings-cosigner.psbt".to_string(), cosigner.to_bytes()),
            ("savings-final.txn".to_string(), hex.into_bytes()),
            ("savings-unsigned.psbt".to_string(), unsigned.to_bytes()),
            ("savings-damaged.psbt".to_string(), wrong.to_bytes()),
        ],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let at = |t: &Tour, n: &str| {
        t.app
            .inbox()
            .iter()
            .position(|i| i.name == n)
            .ok_or_else(|| format!("{n} is not in Files"))
    };
    t.press(Action::Nav(Screen::Files));
    t.shot("nonce-files")?;
    t.press(Action::NonceCheck(faraday_core::decode::CheckPress::Inbox(
        at(t, "savings-cosigner.psbt")?,
    )));
    if t.app.screen != Screen::NonceCheck {
        return Err("Nonce check opened no check".into());
    }
    t.shot("nonce-elsewhere")?;
    // The cosigner's seed loaded here: its signatures recomputed.
    t.press(Action::Entry(None));
    t.type_key(1);
    t.press(Action::Nav(Screen::Files));
    t.press(Action::NonceCheck(faraday_core::decode::CheckPress::Inbox(
        at(t, "savings-cosigner.psbt")?,
    )));
    t.shot("nonce-recomputed")?;
    t.press(Action::NonceCheck(faraday_core::decode::CheckPress::Inbox(
        at(t, "savings-final.txn")?,
    )));
    // Its actions whole on a small panel.
    scroll_to(t, Action::NonceCheck(faraday_core::decode::CheckPress::Hex));
    t.shot("nonce-raw")?;
    t.press(Action::NonceCheck(faraday_core::decode::CheckPress::Hex));
    t.shot("nonce-raw-hex")?;
    t.press(Action::NonceCheck(faraday_core::decode::CheckPress::Inbox(
        at(t, "savings-damaged.psbt")?,
    )));
    t.shot("nonce-refused")
}
