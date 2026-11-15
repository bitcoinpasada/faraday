//! The multisig-check tour (`docs/FLOWS.md`, Check a multisig setup):
//! the screen with nothing read; a setup scanned and its Result; a
//! descriptor whose checksum does not hold, refused on the scanner.

use crate::tour::Tour;
use faraday_core::multisig_check::MsAction as M;
use faraday_core::testkit;
use faraday_core::{Action, Screen};
use faraday_shell_api::{App, Event};

/// A descriptor of the test wallet, scanned.
fn scan(t: &mut Tour, text: &str) {
    t.press(Action::MultisigCheck(M::Scan));
    t.app.event(Event::Scanned {
        bytes: text.as_bytes().to_vec(),
    });
    t.tick();
}

pub(crate) fn multisig_check_tour(t: &mut Tour) -> Result<(), String> {
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "savings")
        .ok_or("no savings wallet")?;
    t.press(Action::Nav(Screen::Tools));
    t.press(Action::MultisigCheck(M::Open));
    t.shot("multisig-check-empty")?;
    scan(t, &kit.descriptor);
    t.shot("multisig-check-result")?;
    t.press(Action::MultisigCheck(M::Another));
    let (body, sum) = kit.descriptor.split_once('#').ok_or("no checksum")?;
    let last = if sum.ends_with('q') { 'p' } else { 'q' };
    let bad = format!("{body}#{}{last}", &sum[..sum.len() - 1]);
    scan(t, &bad);
    t.shot("multisig-check-refused")?;
    Ok(())
}
