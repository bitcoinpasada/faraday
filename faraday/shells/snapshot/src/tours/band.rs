//! The Waiting band tour (`docs/WORKSPACE.md` §1, §2): Home with only a PSBT
//! waiting, then Home, Tools, Check and Wallets with a PSBT waiting, a flow
//! in progress and a locked vault file.

use crate::tour::Tour;
use faraday_core::testkit;
use faraday_core::{Action, Screen, rstep};

pub(crate) fn band_tour(t: &mut Tour) -> Result<(), String> {
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
    let kit = testkit::kits()
        .into_iter()
        .find(|k| k.id == "spending")
        .ok_or("no spending wallet")?;
    let psbt = testkit::unsigned(&kit).map_err(|e| format!("{e:?}"))?;
    testkit::inbox_file(&mut t.app, "spend.psbt", psbt.to_bytes());
    t.press(Action::Nav(Screen::Home));
    t.shot("band-home-psbt-only")?;
    let vault = testkit::kdbx_vault("Work", "work passphrase", 0x77, Vec::new())
        .map_err(|e| String::from(&*e))?;
    testkit::inbox_file_at(&mut t.app, 1, "work.kdbx", vault);
    t.press(Action::RestoreWallet);
    t.press(Action::RNext(rstep::KIND));
    for (page, name) in [
        (Screen::Home, "band-home"),
        (Screen::Catalog, "band-tools"),
        (Screen::Check, "band-check"),
        (Screen::Wallets, "band-wallets"),
    ] {
        t.press(Action::Nav(page));
        t.shot(name)?;
    }
    Ok(())
}
