//! **Save as a QR vault** on an item's page (`docs/VAULT.md` §13): a seed,
//! a codex32 share, a BIP-39 passphrase or a note that fits opens the QR
//! vault tool at its Passphrase step with the item as its contents; one
//! that does not fit puts up a dialog saying why. Whether it fits is
//! worked out on the press, never in a draw, and the item stays in the
//! vault, unchanged.

use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::vault::VaultAction as V;
use crate::{Action, Faraday, Sheet};
use faraday_draw::tokens::{BODY_M, DIALOG_MAX, GAP};
use faraday_draw::widgets::Icon;
use faraday_keys::QrFit;
use faraday_keys::vault::VaultId;

/// The dialog while it is up: why the item does not fit.
#[derive(Debug, Clone, Copy)]
pub struct QrNo {
    /// The vault.
    pub id: VaultId,
    /// Why.
    pub fit: QrFit,
}

/// **Save as a QR vault**: the tool, or the dialog.
pub(crate) fn press(app: &mut Faraday) {
    let (Some(at), Some(id)) = (app.vault_selected_index(), app.vaults.current_id()) else {
        return;
    };
    let c = app.compute();
    match app.vaults.store().qr_fit(&c, id, at) {
        QrFit::Fits => crate::qr_tool::open_item(app, id, at),
        QrFit::NotOffered => {}
        fit => {
            app.vaults.state.qr_no = Some(QrNo { id, fit });
            app.sheet = Some(Sheet::QrNoFit);
        }
    }
}

/// **Close**.
pub(crate) fn close(app: &mut Faraday) {
    app.vaults.state.qr_no = None;
    if app.sheet == Some(Sheet::QrNoFit) {
        app.sheet = None;
    }
}

/// The dialog gone another way, or its vault locked, is **Close**. True
/// when it dropped it.
pub(crate) fn settle(app: &mut Faraday) -> bool {
    let Some(n) = app.vaults.state.qr_no.as_ref() else {
        return false;
    };
    if app.sheet == Some(Sheet::QrNoFit) && app.vaults.open.iter().any(|o| o.id == n.id) {
        return false;
    }
    close(app);
    true
}

/// The line a fit that failed is told in.
fn line(fit: QrFit) -> String {
    match fit {
        QrFit::SeedNotQr => EN.vault_qr_seed_no.to_string(),
        QrFit::PassphraseTooLong => EN.vault_qr_pass_no.to_string(),
        QrFit::NoteTooLong(1) => EN.vault_qr_note_no_one.to_string(),
        QrFit::NoteTooLong(n) => fill(EN.vault_qr_note_no, &[&n.to_string(), &n.to_string()]),
        _ => EN.vault_qr_unwritable.to_string(),
    }
}

/// The dialog.
pub(crate) fn draw(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(n) = app.vaults.state.qr_no.as_ref() else {
        return;
    };
    let text = line(n.fit);
    let actions = [(EN.nav_close, Style::Primary, Action::Vault(V::QrNoClose))];
    crate::compact::dialog(
        ui,
        (w, h, DIALOG_MAX),
        (
            Icon::Warning.into(),
            Role::Warning.paint(),
            EN.vault_qr_save,
        ),
        &mut |ui, x, y, iw| ui.wrap_as((x, y, iw), BODY_M, Role::OnSurface, &text) + GAP,
        &actions,
    );
}
