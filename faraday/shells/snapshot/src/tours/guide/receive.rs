//! The pictures of `docs/guide/walk/receive.md`.

use super::last_wallet;
use crate::tour::Tour;
use faraday_core::{Action, Screen};
use faraday_crypto::work::Compute;
use faraday_shell_api::{App, Event};

/// Costs an address derivation. §6.1: Scan; the wallet's receive
/// address 0 as Sparrow shows it, found; then an address of no loaded
/// wallet (BIP 84's first test address).
pub(super) fn receive(c: &Compute, t: &mut Tour) -> Result<(), String> {
    let w = last_wallet(t)?;
    let net = t.app.session.network();
    let ours = t.app.session.wallets[w].address_on(c, net, false, 0);
    t.press(Action::Nav(Screen::Home));
    t.press(Action::Scan);
    crate::tours::site_tour::camera(t);
    t.shot("receive-scan")?;
    t.app.event(Event::Scanned {
        bytes: ours.into_bytes(),
    });
    t.pump();
    t.shot_after("6-1-found", 0)?;
    t.app.event(Event::Scanned {
        bytes: b"bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu".to_vec(),
    });
    t.pump();
    t.shot_after("receive-not-ours", 0)?;
    t.press(Action::Cancel);
    Ok(())
}
