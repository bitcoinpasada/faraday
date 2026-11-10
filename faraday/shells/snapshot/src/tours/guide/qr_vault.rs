//! The pictures of `docs/guide/screens/qr-vault.md`.

use crate::tour::{Tour, scroll_to, type_text};
use faraday_core::qr_tool::{Field, Pick, QrToolAction as Q};
use faraday_core::{Action, Screen, testkit};

/// Costs a QR vault code sealed and opened once. Practice on testnet:
/// the QR vault tool's first card, a note typed and sealed under
/// "faraday" with the code shown, then Use a sample opened under the
/// same passphrase.
pub(super) fn run(t: &mut Tour) -> Result<(), String> {
    super::demo_session(t);
    t.press(Action::Nav(Screen::Catalog));
    let tile = faraday_core::catalog::TILES
        .iter()
        .position(|tile| tile.name == "QR vault")
        .ok_or("no QR vault tile")?;
    t.press(Action::Catalog(tile as u8));
    t.shot("qr-vault-contents")?;
    t.press(Action::QrTool(Q::Pick(Pick::Note)));
    type_text(t, "the safe is behind the books");
    t.press(Action::QrTool(Q::Next));
    type_text(t, "faraday");
    t.press(Action::QrTool(Q::Focus(Field::Again)));
    type_text(t, "faraday");
    t.press(Action::QrTool(Q::Make));
    testkit::settle(&mut t.app);
    t.tick();
    scroll_to(t, Action::QrTool(Q::Show));
    t.press(Action::QrTool(Q::Show));
    t.shot("qr-vault-make")?;
    t.press(Action::Cancel);
    scroll_to(t, Action::QrTool(Q::Another));
    t.press(Action::QrTool(Q::Another));
    t.press(Action::QrTool(Q::UseSample));
    type_text(t, "faraday");
    t.shot("qr-vault-open")?;
    t.press(Action::QrTool(Q::OpenCode));
    testkit::settle(&mut t.app);
    t.tick();
    t.shot("qr-vault-opened")
}
