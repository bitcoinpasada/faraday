//! The row-details tour (`docs/WORKSPACE.md` §7 point 1): a row of the
//! session pane opened in the page's second pane, on a window that is not
//! compact. Tools with a key open, a wallet open and a held file open, and
//! Wallets with a key open, and Home with a key, a wallet and a file open.

use crate::tour::Tour;
use faraday_core::testkit;
use faraday_core::{Action, Row, Screen};

pub(crate) fn row_details_tour(t: &mut Tour) -> Result<(), String> {
    let c = testkit::compute();
    t.app.session = testkit::session();
    t.app
        .session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .map_err(|e| format!("{e:?}"))?;
    let fp = t
        .app
        .session
        .keys
        .all()
        .next()
        .map(|k| k.fingerprint.0)
        .ok_or("no key")?;
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .ok_or("no spending wallet")?;
    t.app
        .session
        .add_wallet_confirmed(
            &c,
            kit.name,
            &kit.descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .map_err(|e| format!("{e:?}"))?;
    testkit::inbox_file(
        &mut t.app,
        "other-wallet.txt",
        kit.descriptor.clone().into_bytes(),
    );
    t.press(Action::Nav(Screen::Catalog));
    t.press(Action::RowOpen(Row::Key(fp)));
    t.shot("rowdetails-tools-key")?;
    t.press(Action::RowOpen(Row::Wallet(0)));
    t.shot("rowdetails-tools-wallet")?;
    t.press(Action::RowOpen(Row::File(0)));
    t.shot("rowdetails-tools-file")?;
    t.press(Action::RowClose);
    t.press(Action::Nav(Screen::Wallets));
    t.press(Action::RowOpen(Row::Key(fp)));
    t.shot("rowdetails-wallets-key")?;
    t.press(Action::Nav(Screen::Home));
    t.press(Action::RowOpen(Row::Key(fp)));
    t.shot("rowdetails-home-key")?;
    t.press(Action::RowOpen(Row::Wallet(0)));
    t.shot("rowdetails-home-wallet")?;
    t.press(Action::RowOpen(Row::File(0)));
    t.shot("rowdetails-home-file")
}
