//! The silent payments tour.

use crate::tour::Tour;
use faraday_bip::silent_wallet::SilentWallet;
use faraday_core::{Action, Screen};
use faraday_crypto::work::Compute;

/// Costs what the record and the pages drawn cost. A silent payments
/// wallet's record from Files: without its key, its
/// address not checked, on its card and its page; then a record whose
/// scan key is not its key's, once the key is loaded: no address, and
/// why.
pub(crate) fn silent_tour(c: &Compute, t: &mut Tour) -> Result<(), String> {
    t.press(Action::Entry(None));
    t.type_key(0);
    t.press(Action::Silent);
    let honest = t.app.silent_record(c).ok_or("no silent payments record")?;
    let mut other = SilentWallet::parse(&honest).ok_or("the record does not read")?;
    other.scan = other.spend;
    for (name, text, key) in [
        ("silent-unchecked", honest, false),
        ("silent-differs", other.to_text(), true),
    ] {
        t.restart();
        faraday_core::testkit::inbox_push(
            &mut t.app,
            faraday_core::Item {
                name: format!("{name}.txt"),
                bytes: text.into_bytes(),
                kind: faraday_core::wallet::FileKind::Wallet,
                secret: false,
                picture: None,
                sha: None,
                wallet: None,
                psbt: None,
            },
        );
        t.press(Action::LoadWallet(0));
        if key {
            t.press(Action::Entry(None));
            t.type_key(0);
        }
        t.press(Action::PickWallet(0));
        t.press(Action::Nav(Screen::Wallets));
        t.shot(&format!("wallet-{name}"))?;
        t.press(Action::SWallet(0));
        t.shot(name)?;
    }
    Ok(())
}
