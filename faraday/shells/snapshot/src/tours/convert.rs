//! The convert tour.

use crate::tour::{Tour, type_text};
use faraday_core::convert::{self, ConvertAction as C, ROWS, cstep};
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Screen, StorageEvent};

/// Tools' tile for the flow.
fn open(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|tile| tile.name == "Convert a seed")
        .ok_or("no Convert a seed tile")?;
    t.press(Action::Catalog(tile as u8));
    Ok(())
}

/// Convert a seed (`docs/SEED-CONVERSION.md` §6): From with no key, From
/// with an open vault's key, and that key loaded; To for a key of words; the
/// xprv's Result masked and shown; Check my copy typing, wrong and right; and
/// the Secret out sheet.
pub(crate) fn convert_tour(t: &mut Tour) -> Result<(), String> {
    open(t)?;
    t.shot("convert-from-empty")?;
    let vault = testkit::files()?
        .into_iter()
        .find(|(n, _)| n == "vault.fvlt")
        .ok_or("no test vault")?;
    t.app.storage(StorageEvent::Restored {
        inbox: vec![vault],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    t.app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    testkit::vault_speed(&mut t.app, Some(180));
    t.press(Action::Vault(V::OpenFrom(0, Screen::Catalog)));
    type_text(t, testkit::VAULT_PASSPHRASES[0]);
    t.press(Action::Vault(V::Unlock));
    for _ in 0..10 {
        if !t.app.vaults().open.is_empty() {
            break;
        }
        t.tick();
    }
    open(t)?;
    t.shot("convert-from-vault")?;
    let record = t
        .app
        .vaults()
        .records(t.app.vaults().open[0].id)
        .iter()
        .position(|(_, r)| r.kind() == faraday_vault::records::kind::KEY)
        .ok_or("no key in the test vault")?;
    t.press(Action::Convert(C::VaultKey(0, record)));
    t.shot("convert-to")?;
    t.press(Action::Convert(C::Step(cstep::FROM)));
    t.shot("convert-from")?;
    t.press(Action::Convert(C::Step(cstep::TO)));
    let xprv = ROWS
        .iter()
        .position(|r| r.name == "xprv")
        .ok_or("no xprv row")?;
    t.press(Action::Convert(C::To(xprv as u8)));
    t.shot("convert-result")?;
    t.press(Action::Convert(C::Show));
    t.shot("convert-result-shown")?;
    t.press(Action::Convert(C::Check));
    type_text(t, "xprv9s21");
    t.shot("convert-check")?;
    t.press(Action::Convert(C::Compare));
    t.shot("convert-check-differs")?;
    let value = convert::shown(&t.app).ok_or("no value")?.to_string();
    type_text(t, &value);
    t.press(Action::Convert(C::Compare));
    t.shot("convert-check-matches")?;
    t.press(Action::Convert(C::Save));
    t.shot("convert-save")?;
    t.press(Action::Cancel);
    Ok(())
}
