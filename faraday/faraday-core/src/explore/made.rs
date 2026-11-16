//! **Wallet made**: the dialog over Explore's page after **Make a wallet**
//! (`docs/DESIGN.md` §5, Make a wallet): the wallet's name, its descriptor
//! row and its first addresses, **Done** and **Open the wallet**. Reads
//! the state.

use super::XAction;
use crate::Action;
use crate::drawing::View;
use crate::screens::{descriptor_row, first_addresses};
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::DIALOG_MAX;
use faraday_draw::widgets::Icon;

/// The dialog: the wallet's name for its head, the descriptor row
/// (opening the wallet's code) and the first addresses for its body.
pub(crate) fn dialog(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(i) = app.made_place() else {
        return;
    };
    let Some(wl) = app.session.wallets.get(i) else {
        return;
    };
    crate::compact::dialog(
        ui,
        (w, h, DIALOG_MAX),
        (Icon::Wallet.into(), Role::Primary.paint(), wl.name.as_str()),
        &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += descriptor_row(ui, x, cy, iw, wl, Action::QrWallet(i));
            cy += first_addresses(app, ui, i, x, cy, iw);
            cy - y + 6.0
        },
        &[
            (
                EN.made_wallet_done,
                Style::Ghost,
                Action::X(XAction::MadeDone),
            ),
            (
                EN.made_wallet_open,
                Style::Primary,
                Action::X(XAction::MadeOpen),
            ),
        ],
    );
}
