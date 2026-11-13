//! **Remove codex32 set** and **Remove BIP-85 origin** (`docs/VAULT.md`
//! §7): a seed in an open vault drops that custom data from its entry, and
//! nothing else, at the next seal. A dialog says copies sealed before still
//! hold it. The press waits at the dialog for the vault's version
//! ([`faraday_keys::vault::OpenVaults::version`]) to be the one it was
//! asked at: any change meanwhile, or a lock, leaves everything alone.

use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::vault::VaultAction as V;
use crate::{Action, Faraday, Sheet};
use faraday_draw::tokens::{BODY_M, DIALOG_MAX, GAP};
use faraday_draw::widgets::Icon;
use faraday_keys::vault::VaultId;
use faraday_vault::records::field;

/// The dialog while it is up.
#[derive(Debug, Clone)]
pub struct Trim {
    /// The vault.
    pub id: VaultId,
    /// The record's place in it.
    pub record: usize,
    /// The field to drop: `KEY_CODEX32` or `KEY_CHILD`.
    pub field: u8,
    /// The vault's version when asked.
    pub version: u64,
    /// The vault's name, for the dialog.
    pub name: String,
}

/// The dialog's `field` for **Remove YubiKey secret** of the open vault
/// (or **Keep** where a removal waits): no record, no field.
pub(crate) const YUBIKEY: u8 = u8::MAX;

/// The open vault's **Remove YubiKey secret**, or **Keep YubiKey secret**
/// where a removal waits: the dialog.
pub(crate) fn ask_yubikey(app: &mut Faraday) {
    let Some(id) = app.vaults.current_id() else {
        return;
    };
    let store = app.vaults.store();
    let (pending, held) = (store.yubikey_removal_pending(id), store.needs(id));
    if store.read_only(id).is_some() || !(pending || held.is_some_and(|n| n.1)) {
        return;
    }
    let name = app
        .vaults
        .open
        .iter()
        .find(|o| o.id == id)
        .map_or_else(Default::default, |o| o.label(store).as_str().into());
    let version = store.version(id);
    app.vaults.state.more = None;
    app.vaults.state.trim = Some(Trim {
        id,
        record: 0,
        field: YUBIKEY,
        version,
        name,
    });
    app.sheet = Some(Sheet::Trim);
}

/// Puts up the dialog for the selected seed's field `number`.
pub(crate) fn ask(app: &mut Faraday, number: u8) {
    if number != field::KEY_CODEX32 && number != field::KEY_CHILD {
        return;
    }
    let (Some(record), Some(id)) = (app.vault_selected_index(), app.vaults.current_id()) else {
        return;
    };
    let store = app.vaults.store();
    if store.read_only(id).is_some()
        || !store.records(id).get(record).is_some_and(|r| r.has(number))
    {
        return;
    }
    let name = app
        .vaults
        .open
        .iter()
        .find(|o| o.id == id)
        .map_or_else(Default::default, |o| o.label(store).as_str().into());
    let version = store.version(id);
    app.vaults.state.trim = Some(Trim {
        id,
        record,
        field: number,
        version,
        name,
    });
    app.sheet = Some(Sheet::Trim);
}

/// **Remove**: the field leaves the entry, if nothing changed meanwhile.
pub(crate) fn go(app: &mut Faraday) {
    let Some(t) = app.vaults.state.trim.take() else {
        return;
    };
    if app.sheet == Some(Sheet::Trim) {
        app.sheet = None;
    }
    if t.field == YUBIKEY {
        return yubikey_go(app, &t);
    }
    if app
        .vaults
        .edit(t.id, |s| s.drop_custom(t.id, t.record, t.field, t.version))
    {
        app.vaults.changed(t.id);
    }
}

/// **Remove** or **Keep** on the YubiKey secret: the next seal leaves it
/// out, or the removal is taken back; the same as a new passphrase
/// for the lock and the seal.
fn yubikey_go(app: &mut Faraday, t: &Trim) {
    let pending = app.vaults.store().yubikey_removal_pending(t.id);
    let done = app.vaults.edit(t.id, |s| {
        if pending {
            s.keep_yubikey(t.id)
        } else {
            s.remove_yubikey(t.id)
        }
    });
    if done {
        if pending {
            // A removal taken back is no change at all.
            app.vaults.unchanged(t.id);
        } else {
            app.vaults.changed(t.id);
        }
        app.toast(if pending {
            EN.vault_yubikey_kept
        } else {
            EN.vault_yubikey_removed
        });
    }
}

/// **Cancel**.
pub(crate) fn close(app: &mut Faraday) {
    app.vaults.state.trim = None;
    if app.sheet == Some(Sheet::Trim) {
        app.sheet = None;
    }
}

/// The dialog gone another way, or its vault changed or locked, is
/// **Cancel**. True when it dropped it.
pub(crate) fn settle(app: &mut Faraday) -> bool {
    let Some(t) = app.vaults.state.trim.as_ref() else {
        return false;
    };
    let open = app.vaults.open.iter().any(|o| o.id == t.id);
    if app.sheet == Some(Sheet::Trim) && open && app.vaults.store().version(t.id) == t.version {
        return false;
    }
    close(app);
    true
}

/// The dialog.
pub(crate) fn draw(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(t) = app.vaults.state.trim.as_ref() else {
        return;
    };
    let keep = t.field == YUBIKEY && app.vaults.store().yubikey_removal_pending(t.id);
    let (title, line) = if t.field == YUBIKEY && keep {
        (EN.vault_keep_yubikey, EN.vault_yubikey_keep_line)
    } else if t.field == YUBIKEY {
        (EN.vault_remove_yubikey, EN.vault_yubikey_remove_line)
    } else if t.field == field::KEY_CODEX32 {
        (EN.vault_trim_codex32, EN.vault_trim_codex32_line)
    } else {
        (EN.vault_trim_child, EN.vault_trim_child_line)
    };
    let line = fill(line, &[&t.name, &t.name]);
    let go = if keep {
        EN.vault_keep
    } else {
        EN.vault_trim_go
    };
    let actions = [
        (
            EN.vault_copy_cancel,
            Style::Secondary,
            Action::Vault(V::TrimCancel),
        ),
        (go, Style::Primary, Action::Vault(V::TrimGo)),
    ];
    crate::compact::dialog(
        ui,
        (w, h, DIALOG_MAX),
        (Icon::Warning.into(), Role::Warning.paint(), title),
        &mut |ui, x, y, iw| ui.wrap_as((x, y, iw), BODY_M, Role::OnSurface, &line) + GAP,
        &actions,
    );
}
