//! The convert tour.

use crate::tour::Tour;
use faraday_core::convert::{ConvertAction as C, ROWS};
use faraday_core::testkit;
use faraday_core::{Action, Screen};

/// Convert a seed (`docs/SEED-CONVERSION.md` §6): From with no key, then
/// To for a key of words, and the xprv's Result masked and shown.
pub(crate) fn convert_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|tile| tile.name == "Convert a seed")
        .ok_or("no Convert a seed tile")?;
    t.press(Action::Catalog(tile as u8));
    t.shot("convert-from-empty")?;
    let c = testkit::compute();
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    t.app
        .session
        .keys
        .add_words(&c, &words, testkit::TEST_SEEDS[0].1, None)
        .map_err(|e| format!("{e:?}"))?;
    t.shot("convert-from")?;
    let key = t.app.session.keys[0].fingerprint.0;
    t.press(Action::Convert(C::Key(key)));
    t.shot("convert-to")?;
    let xprv = ROWS
        .iter()
        .position(|r| r.name == "xprv")
        .ok_or("no xprv row")?;
    t.press(Action::Convert(C::To(xprv as u8)));
    t.shot("convert-result")?;
    t.press(Action::Convert(C::Show));
    t.shot("convert-result-shown")?;
    Ok(())
}
