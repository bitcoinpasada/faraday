//! A key file on Create a vault (`docs/VAULT.md` §3.2, §4): **Add a key
//! file** opens a dialog with two rows, make one or choose a file in Files.
//! Nothing is drawn or read before **Create vault**: a made key file's
//! random bytes are drawn then, a chosen file is read then, and a refused
//! file is named with nothing made. The vault is kept only once the key
//! file's `.keyx` is in the Outbox; a chosen file leaves Files with it.

use super::KeyChoice;
use super::unlock_keys::{self, Bad};
use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::vault::VaultAction as V;
use crate::{Action, Faraday, Sheet};
use faraday_draw::tokens::{
    BODY_M, BODY_S, BUTTON_H, DIALOG_MAX, GAP, GAP_SMALL, LIST_1, PAD, RADIUS_SMALL,
};
use faraday_vault::kdbx::Components;

fn va(a: V) -> Action {
    Action::Vault(a)
}

/// **Add a key file**, or the chip: the dialog.
pub(crate) fn ask(app: &mut Faraday) {
    if app.vaults.create.is_some() {
        app.vaults.state.key_menu = true;
        app.vaults.state.key_for_create = true;
        app.sheet = Some(Sheet::KeyFile);
    }
}

/// **Make a key file**: drawn at Create, nothing now.
pub(crate) fn make(app: &mut Faraday) {
    if let Some(c) = app.vaults.state.create.as_mut() {
        c.key = KeyChoice::Make;
        c.error = None;
    }
    cancel(app);
}

/// **Choose a file**: the list of the files in Files that are not vaults.
pub(crate) fn choose(app: &mut Faraday) {
    app.vaults.state.key_pick = 0;
    app.vaults.state.key_menu = false;
}

/// **Cancel**.
pub(crate) fn cancel(app: &mut Faraday) {
    unlock_keys::cancel(app);
}

/// The chip's ×.
pub(crate) fn drop_key(app: &mut Faraday) {
    if let Some(c) = app.vaults.state.create.as_mut() {
        c.key = KeyChoice::None;
        c.error = None;
    }
}

/// The key file Create has been given, read now: the chosen file's name
/// and key, `None` for none or a made one; why a chosen file is refused
/// otherwise, naming it.
pub(crate) fn chosen(app: &Faraday) -> Result<Option<(String, Components)>, String> {
    let Some(KeyChoice::File(name)) = app.vaults.create.as_ref().map(|c| &c.key) else {
        return Ok(None);
    };
    match unlock_keys::read_named(app, name) {
        Ok(key) => Ok(Some((name.clone(), Components::with_key_file(key)))),
        Err(Bad::KeyFile(e) | Bad::Secret(e)) => Err(e),
    }
}

/// The chip's text for a key file: the file's name, or the one Create
/// makes.
pub(crate) fn chip(app: &View) -> Option<String> {
    let c = app.vaults.create.as_ref()?;
    match &c.key {
        KeyChoice::None => None,
        KeyChoice::Make => Some(fill(
            EN.vault_keyx_made_at_create,
            &[&crate::vault::vault_stem(&c.name.text).to_string()],
        )),
        KeyChoice::File(name) => Some(name.clone()),
    }
}

/// The button or chip under the passphrases at (x, y), `w` wide. Returns
/// the height it takes.
pub(crate) fn draw(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let chip = chip(app);
    let mut place = |_: f32| (x, y);
    unlock_keys::key_slot(
        ui,
        chip.as_deref(),
        w,
        &mut place,
        (V::CKeyAsk, V::CKeyDrop),
    );
    BUTTON_H
}

/// **Add a key file**: the warning, the two ways to have one, **Cancel**.
pub(crate) fn dialog(_app: &View, ui: &mut Ui, w: f32, h: f32) {
    let head = (
        faraday_draw::widgets::Icon::File.into(),
        Role::Primary.paint(),
        EN.vault_add_key_file,
    );
    let actions = [(EN.vault_cancel, Style::Secondary, va(V::CKeyCancel))];
    let rows = [
        (EN.vault_make_key_file, V::CKeyMake),
        (EN.vault_choose_a_file, V::CKeyChoose),
    ];
    crate::compact::dialog(
        ui,
        (w, h, DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += ui.wrap_as(
                (x, cy, iw),
                BODY_S,
                Role::OnSurfaceVariant,
                EN.vault_key_file_warning,
            ) + GAP;
            for (label, act) in rows {
                ui.fill(
                    x,
                    cy,
                    iw,
                    LIST_1,
                    RADIUS_SMALL,
                    Role::SurfaceContainerHighest,
                );
                ui.text_mid_as((x + PAD, cy, LIST_1), BODY_M, Role::OnSurface, label);
                ui.press_box(x, cy, iw, LIST_1, va(act));
                cy += LIST_1 + GAP_SMALL;
            }
            cy + GAP - GAP_SMALL - y
        },
        &actions,
    );
}
