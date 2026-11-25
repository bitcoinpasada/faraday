//! The row-details tour (`docs/WORKSPACE.md` §7 point 1): a row of the
//! session pane opened in the page's second pane, on a window that is not
//! compact. Tools with a key open, a wallet open and a held file open, and
//! Wallets with a key open, and Home with a key, a wallet and a file open,
//! and Files with a file's details open, with no stick and with one in.

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
    testkit::inbox_file(&mut t.app, "notes.txt", "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq\nH9L5yLFjti0QTHhPyFrZCT1V/MMnBtXKmoiKDZ78NDBjERki6ZTQZdSMCtkgoNmp17By9ItJr8o7ChX0XxY91nk=\nHello\n"
            .as_bytes()
            .to_vec());
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
    t.shot("rowdetails-home-file")?;
    // Files: a file's details in the stick's place, with no stick and with
    // one in.
    if t.app.offers(Action::RowClose) {
        t.press(Action::RowClose);
    }
    t.press(Action::Nav(Screen::Files));
    t.press(Action::RowOpen(Row::File(0)));
    t.shot("rowdetails-files-no-stick")?;
    t.press(Action::RowClose);
    t.sticks(true);
    if t.app.sheet.is_some() {
        t.press(Action::Cancel);
    }
    t.press(Action::Nav(Screen::Files));
    t.shot("rowdetails-files-stick")?;
    t.press(Action::RowOpen(Row::File(0)));
    t.shot("rowdetails-files-stick-details")?;
    // A public file's tick box beside the chevron and the menu button:
    // not ticked, ticked, and ticked with its details open.
    // The keys and the wallet held in the session keep the stick from use
    // (no tick column); a session without them lets the stick be written.
    t.press(Action::RowClose);
    t.app.session = testkit::session();
    t.press(Action::Nav(Screen::Files));
    let note = t
        .app
        .inbox()
        .iter()
        .position(|i| i.name == "notes.txt")
        .ok_or("no notes.txt")?;
    t.shot("rowdetails-files-tick-off")?;
    t.press(Action::VisitInbox(note));
    t.shot("rowdetails-files-tick-on")?;
    t.press(Action::RowOpen(Row::File(note)));
    t.shot("rowdetails-files-tick-on-details")
}
