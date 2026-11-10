//! Read a Bitcoin Core wallet in Tools (`docs/decisions/F229.md`): the
//! File card with a wallet to choose, and the Result of an encrypted
//! wallet read with its passphrase, and scrolled to its pairs.

use crate::tour::{Tour, type_text};
use faraday_core::core_wallet::tool::CoreWalletAction as W;
use faraday_core::{Action, Screen, StorageEvent};

const V: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../tools/vectors/core-wallet"
);

pub(crate) fn core_wallet_tour(t: &mut Tour) -> Result<(), String> {
    let bytes =
        std::fs::read(format!("{V}/encrypted.dat")).map_err(|e| format!("encrypted.dat: {e}"))?;
    t.app.storage(StorageEvent::Restored {
        inbox: vec![("wallet.dat".to_string(), bytes)],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|tile| tile.name == "Read a Bitcoin Core wallet")
        .ok_or("no Read a Bitcoin Core wallet tile")?;
    t.press(Action::Catalog(tile as u8));
    t.press(Action::CoreWallet(W::File(0)));
    t.shot("core-wallet-file")?;
    t.press(Action::CoreWallet(W::Focus));
    type_text(t, "faraday");
    t.press(Action::CoreWallet(W::Read));
    t.shot("core-wallet-result")?;
    crate::tour::scroll_to(t, Action::CoreWallet(W::Take(0)));
    t.shot("core-wallet-pairs")?;
    Ok(())
}
