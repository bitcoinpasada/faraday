//! The invoice tour: Decode a Lightning invoice.

use crate::tour::{Tour, type_text};
use faraday_core::invoice::InvoicePress;
use faraday_core::{Action, Screen};

/// BOLT #11's example "Please send $3 for a cup of coffee".
const COFFEE: &str = "lnbc2500u1pvjluezsp5zyg3zyg3zyg3zyg3zyg3zyg3zyg3zyg3zyg3zyg3zyg3zyg3zygspp5qqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqypqdq5xysxxatsyp3k7enxv4jsxqzpu9qrsgquk0rl77nj30yxdy8j9vdx85fkpmdla2087ne0xh8nhedh8w27kyke0lp53ut353s06fv3qfegext0eh0ymjpf39tuven09sam30g4vgpfna3rh";
/// The same, its last character changed: the checksum fails.
const SPOILED: &str = "lnbc2500u1pvjluezsp5zyg3zyg3zyg3zyg3zyg3zyg3zyg3zyg3zyg3zyg3zyg3zyg3zygspp5qqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqqqsyqcyq5rqwzqfqypqdq5xysxxatsyp3k7enxv4jsxqzpu9qrsgquk0rl77nj30yxdy8j9vdx85fkpmdla2087ne0xh8nhedh8w27kyke0lp53ut353s06fv3qfegext0eh0ymjpf39tuven09sam30g4vgpfna3rg";

/// The small panel's keyboard put away, so the page under it is seen;
/// nothing where the window has none up.
fn hide_keyboard(t: &mut Tour) {
    t.press(Action::Osk(faraday_core::OskPress::Hide));
}

/// Decode a Lightning invoice: empty, a decoded invoice and a refused one.
pub(crate) fn invoice_tour(t: &mut Tour) -> Result<(), String> {
    t.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|tile| tile.name == "Decode a Lightning invoice")
        .ok_or("no Decode a Lightning invoice tile")?;
    t.press(Action::Catalog(tile as u8));
    t.shot("invoice-empty")?;
    type_text(t, COFFEE);
    t.press(Action::Invoice(InvoicePress::Decode));
    hide_keyboard(t);
    t.shot("invoice-decoded")?;
    t.press(Action::Invoice(InvoicePress::Clear));
    type_text(t, SPOILED);
    t.press(Action::Invoice(InvoicePress::Decode));
    hide_keyboard(t);
    t.shot("invoice-refused")?;
    Ok(())
}
