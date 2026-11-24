//! A wallet's share record in a vault is its own: backing a wallet up
//! into a vault leaves the share record of another wallet of the same
//! 8-character checksum (`docs/WALLETS.md` §4).

use crate::chart_edit::{desktop, fresh_vault};
use crate::common::*;
use faraday_core::board::Thing;
use faraday_core::testkit::{self, WALLET_ID_FIRST as FIRST, WALLET_ID_TWIN as TWIN};
use faraday_core::wallet::Origin;
use faraday_core::{Action, Faraday};
use faraday_vault::Record;
use faraday_vault::records::{field, kind};

fn shares(app: &Faraday) -> Vec<(Vec<u8>, Vec<u8>)> {
    contents(app, 0)
        .of(kind::WALLET_SHARE)
        .map(|(_, r)| {
            (
                r.field(field::SHARE_WALLET).unwrap_or(&[]).to_vec(),
                r.field(field::SHARE_NUMBER).unwrap_or(&[]).to_vec(),
            )
        })
        .collect()
}

/// The vault holds the twin's Share 2; the first wallet's Share 2 is
/// saved there, and the twin's record is still in it.
#[test]
fn backing_a_wallet_up_leaves_the_share_record_of_its_checksum_twin() {
    let c = testkit::compute();
    let mut app = desktop();
    for (name, d) in [("First", FIRST), ("Twin", TWIN)] {
        app.session
            .add_wallet_confirmed(&c, name, d, "kit", Origin::Named)
            .expect("the wallet reads");
    }
    fresh_vault(&mut app);
    let id = app.vaults().open[0].id;
    let twins = Record::new(kind::WALLET_SHARE)
        .with(field::SHARE, b"twin share text")
        .with(field::SHARE_WALLET_NAME, b"Twin")
        .with(field::SHARE_NUMBER, &[2])
        .with(field::SHARE_OF, &[3])
        .with(field::SHARE_WALLET, TWIN.as_bytes());
    testkit::vault_edit(&mut app, id, |s| s.insert(id, twins));
    assert_eq!(shares(&app).len(), 1);

    app.press(Action::Backup(0));
    testkit::backup_as(&mut app, faraday_core::plan::Preset::PaperVault);
    no_words(&mut app);
    description_out_of(&mut app, 0);
    no_paper_description(&mut app);
    no_description_files(&mut app);
    into_vault(&mut app, 0, Thing::Share(1));
    app.press(Action::BChecklist);
    app.press(Action::BVaultSave(0));

    let got = shares(&app);
    assert_eq!(got.len(), 2, "the first's Share 2 added, the twin's kept");
    assert!(got.iter().any(|(w, _)| w == TWIN.as_bytes()));
    assert!(got.iter().any(|(w, _)| w != TWIN.as_bytes()));
}
