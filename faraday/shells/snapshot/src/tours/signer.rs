//! The signer tour: Check a signer (`docs/SIGNER-CHECK.md`). Files with
//! the PSBT sent to a device, the one Bitcoin Core returned and one that
//! came back with a proprietary field added; the flow's cards, a clean
//! Result, and a Result with a finding.

use crate::tour::{Tour, scroll, scroll_to};
use faraday_core::catalog::{Go, TILES};
use faraday_core::decode::check::{SignerPress, Slot};
use faraday_core::{Action, Screen, StorageEvent};

/// A PSBT of `tools/vectors/psbt/`.
fn vector(name: &str) -> Result<Vec<u8>, String> {
    let path = format!(
        "{}/../../../tools/vectors/psbt/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).map_err(|e| format!("{path}: {e}"))
}

pub(crate) fn signer_tour(t: &mut Tour) -> Result<(), String> {
    // Nothing in Files: the empty card, with its ways to get a PSBT.
    let tile = TILES
        .iter()
        .position(|g| g.go == Go::SignerCheck)
        .ok_or("no Check a signer tile")? as u8;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(tile));
    t.shot("signer-empty")?;
    t.press(Action::SignerCheck(SignerPress::Back));
    t.app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            ("sent.psbt".to_string(), vector("signer-core-sent.psbt")?),
            (
                "signed.psbt".to_string(),
                vector("signer-core-signed.psbt")?,
            ),
            (
                "device-added.psbt".to_string(),
                vector("signer-proprietary-added.psbt")?,
            ),
            (
                "regtest-sent.psbt".to_string(),
                vector("demo-regtest.psbt")?,
            ),
            (
                "random-nonce.psbt".to_string(),
                vector("signer-random-nonce.psbt")?,
            ),
            (
                "bump-original.psbt".to_string(),
                vector("bump-core-original.psbt")?,
            ),
            (
                "bump-replacement.psbt".to_string(),
                vector("bump-core-replacement.psbt")?,
            ),
            (
                "bump-original.tx".to_string(),
                vector("bump-core-original.tx")?,
            ),
        ]),
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
    let tile = TILES
        .iter()
        .position(|g| g.go == Go::SignerCheck)
        .ok_or("no Check a signer tile")? as u8;
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::Catalog(tile));
    t.shot("signer-sent")?;
    t.press(Action::SignerCheck(SignerPress::Pick(
        Slot::Sent,
        at(t, "sent.psbt")?,
    )));
    t.shot("signer-returned")?;
    t.press(Action::SignerCheck(SignerPress::Pick(
        Slot::Returned,
        at(t, "signed.psbt")?,
    )));
    t.shot("signer-clean")?;
    scroll_to(t, Action::SignerCheck(SignerPress::Again));
    t.shot("signer-clean-end")?;
    t.press(Action::SignerCheck(SignerPress::Card(1)));
    t.press(Action::SignerCheck(SignerPress::Pick(
        Slot::Returned,
        at(t, "device-added.psbt")?,
    )));
    t.shot("signer-findings")?;
    scroll_to(t, Action::SignerCheck(SignerPress::Again));
    t.shot("signer-findings-end")?;
    // Swapped, the device-added file is Sent: the signature and the
    // field are gone from what came back.
    t.press(Action::SignerCheck(SignerPress::Swap));
    t.shot("signer-swapped")?;
    // A key that signed and is not loaded: the nonce fact names it, and
    // Load is offered.
    t.press(Action::SignerCheck(SignerPress::Card(0)));
    t.press(Action::SignerCheck(SignerPress::Pick(
        Slot::Sent,
        at(t, "regtest-sent.psbt")?,
    )));
    t.press(Action::SignerCheck(SignerPress::Pick(
        Slot::Returned,
        at(t, "random-nonce.psbt")?,
    )));
    t.shot("signer-not-loaded")?;
    // The facts block of that report, scrolled up to.
    scroll(t, 240);
    t.shot("signer-not-loaded-facts")?;
    scroll_to(t, Action::SignerCheck(SignerPress::Load(0)));
    t.shot("signer-not-loaded-end")?;
    // A fee bump: the replacement spends the original's inputs, and the
    // fee-bump verdict stands with the signer check.
    t.press(Action::SignerCheck(SignerPress::Card(0)));
    t.press(Action::SignerCheck(SignerPress::Pick(
        Slot::Sent,
        at(t, "bump-original.psbt")?,
    )));
    t.press(Action::SignerCheck(SignerPress::Pick(
        Slot::Returned,
        at(t, "bump-replacement.psbt")?,
    )));
    t.shot("signer-fee-bump")?;
    scroll_to(t, Action::SignerCheck(SignerPress::Again));
    t.shot("signer-fee-bump-end")?;
    // A finished transaction as Returned: the facts and the differences,
    // and no verdict.
    t.press(Action::SignerCheck(SignerPress::Card(1)));
    t.press(Action::SignerCheck(SignerPress::Pick(
        Slot::Returned,
        at(t, "bump-original.tx")?,
    )));
    t.shot("signer-finished")?;
    scroll_to(t, Action::SignerCheck(SignerPress::Again));
    t.shot("signer-finished-end")?;
    // The Sent file replaced in Files by another under the same name:
    // its closed card says so, on its one line.
    t.app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (
                "regtest-sent.psbt".to_string(),
                vector("signer-core-sent.psbt")?,
            ),
            (
                "random-nonce.psbt".to_string(),
                vector("signer-random-nonce.psbt")?,
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.tick();
    t.press(Action::SignerCheck(SignerPress::Card(1)));
    t.shot("signer-differs-closed")?;
    t.press(Action::SignerCheck(SignerPress::Card(2)));
    t.shot("signer-differs")
}
