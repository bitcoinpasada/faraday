//! The silent payments tour.

use crate::tour::Tour;
use faraday_bip::silent_wallet::SilentWallet;
use faraday_core::{Action, Screen};
use faraday_crypto::work::Compute;
use faraday_ui::keyboard::OskPress;

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
    t.shot("silent-dns-empty")?;
    for (here, text) in [
        (faraday_core::edit::Here::SilentName, "matt"),
        (faraday_core::edit::Here::SilentDomain, "example.com"),
    ] {
        t.press(Action::TypeHere(here));
        crate::tour::type_text(t, text);
    }
    t.shot("silent-dns")?;
    t.press(Action::Osk(OskPress::Hide));
    t.shot("silent-dns-record")?;
    t.press(Action::ShowCode(faraday_core::Code::SilentDns));
    t.shot("silent-dns-qr")?;
    t.press(Action::Cancel);
    t.press(Action::TypeHere(faraday_core::edit::Here::SilentDomain));
    crate::tour::type_text(t, " x");
    t.shot("silent-dns-bad")?;
    t.press(Action::Osk(OskPress::Hide));
    t.shot("silent-dns-bad-closed")?;
    let mut other = SilentWallet::parse(&honest).ok_or("the record does not read")?;
    other.scan = other.spend;
    for (name, text, key) in [
        ("silent-unchecked", honest.as_str().into(), false),
        ("silent-differs", other.to_text(), true),
    ] {
        t.restart();
        faraday_core::testkit::inbox_push(
            &mut t.app,
            faraday_core::Item {
                name: faraday_text::Text::of(&format!("{name}.txt")),
                bytes: text.into_bytes().into(),
                kind: faraday_core::wallet::FileKind::Wallet,
                secret: false,
                picture: None,
                sha: None,
                facts: None,
                wallet: None,
                psbt: None,
                outpoints: Vec::new(),
                unsigned_txid: None,
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
