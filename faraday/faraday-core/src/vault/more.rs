//! The open vault's ⋮ (`docs/VAULT.md` §3.2, §8): **Key file** and
//! **Remove YubiKey secret**. The ⋮ is a Material menu
//! ([`crate::menu::draw`]); **Key file** is a basic dialog of three rows
//! that each act at once, and **Remove YubiKey secret** the dialog of
//! [`super::trim`]. Both are changes the next seal writes, counted for the
//! lock as a new passphrase is.
//!
//! A key file made is drawn when **Make a key file** is pressed, and its
//! `.keyx` goes to the Outbox then, under the vault's name, never over a
//! file there; if it cannot be kept, no key file is pending. A key file
//! chosen is read at the press, a refused one named with nothing changed,
//! and its copy leaves Files once the seal that writes it succeeds.

use super::unlock_keys::{self, Bad};
use crate::drawing::View;
use crate::menu::{self, Line};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::vault::VaultAction as V;
use crate::{Action, Faraday, Sheet};
use faraday_draw::tokens::{BODY_M, BODY_S, DIALOG_MAX, GAP, GAP_SMALL, LIST_1, PAD, RADIUS_SMALL};
use faraday_draw::widgets::Icon;
use faraday_keys::vault::VaultId;
use zeroize::Zeroizing;

fn va(a: V) -> Action {
    Action::Vault(a)
}

/// Which of the open vault's ⋮ screens is up on the key file sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum More {
    /// The menu, with where the ⋮ was drawn, in pixels, if it was.
    Menu(Option<(i32, i32, i32, i32)>),
    /// **Key file**: the dialog.
    Key,
    /// **Choose a file**: the list of files in Files.
    Pick,
}

/// What the ⋮ holds for the vault on the page: **Key file** and whether
/// **Remove YubiKey secret** (or **Keep**, where a removal waits) is
/// offered. A lookup.
pub(crate) fn offered(app: &Faraday) -> (bool, bool) {
    let Some(id) = app.vaults.current_id() else {
        return (false, false);
    };
    let store = app.vaults.store();
    if store.read_only(id).is_some() {
        return (false, false);
    }
    let yubikey = store.yubikey_removal_pending(id) || store.needs(id).is_some_and(|n| n.1);
    (!super::screen::writing(app), yubikey)
}

/// Whether the open vault has a ⋮ at all.
pub(crate) fn shown(app: &Faraday) -> bool {
    let (key, yubikey) = offered(app);
    key || yubikey
}

/// The ⋮: its menu.
pub(crate) fn ask(app: &mut Faraday) {
    if !shown(app) {
        return;
    }
    let anchor = app
        .pointer
        .rect_of(va(V::MoreAsk))
        .map(|r| (r.x, r.y, r.w, r.h));
    app.vaults.state.key_for_create = false;
    app.vaults.state.key_menu = false;
    app.vaults.state.more = Some(More::Menu(anchor));
    app.sheet = Some(Sheet::KeyFile);
}

/// **Key file**: the dialog.
pub(crate) fn key_ask(app: &mut Faraday) {
    if !offered(app).0 {
        return;
    }
    app.vaults.state.more = Some(More::Key);
    app.sheet = Some(Sheet::KeyFile);
}

/// The dialog closed, whichever way: its state goes with it.
pub(crate) fn settle(app: &mut Faraday) -> bool {
    let mut moved = false;
    if app.vaults.more.is_some() && app.sheet != Some(Sheet::KeyFile) {
        app.vaults.state.more = None;
        moved = true;
    }
    drop_gone(app);
    // A chosen file whose vault was let go unsealed stays in Files.
    if let Some((id, _)) = app.vaults.key_chosen.as_ref()
        && !app.vaults.open.iter().any(|o| o.id == *id)
    {
        app.vaults.state.key_chosen = None;
    }
    moved
}

/// The `.keyx` of a vault no longer open (let go unsealed) leaves the Outbox.
pub(crate) fn drop_gone(app: &mut Faraday) {
    let gone: Vec<VaultId> = app
        .vaults
        .key_outbox
        .iter()
        .map(|k| k.0)
        .filter(|id| !app.vaults.open.iter().any(|o| o.id == *id))
        .collect();
    for id in gone {
        unkeep(app, id);
    }
}

/// The Outbox's own `.keyx` named `name` was written to the stick (not a
/// copy from the Inbox of the same name): it is no longer a pending item
/// of the Outbox to take back.
pub(crate) fn written(app: &mut Faraday, name: &str) {
    app.vaults.state.key_outbox.retain(|k| k.1 != name);
}

/// The open vault's id and name where it may be changed.
fn target(app: &Faraday) -> Option<(VaultId, String)> {
    let id = app.vaults.current_id()?;
    if app.vaults.store().read_only(id).is_some() || super::screen::writing(app) {
        return None;
    }
    let name = app.vaults.open.iter().find(|o| o.id == id)?.name.clone();
    Some((id, name))
}

/// The `.keyx` made for the pending key file of vault `id`, if any, leaves
/// the Outbox (its item drops and wipes): its key will not be used.
fn unkeep(app: &mut Faraday, id: VaultId) {
    let Some(at) = app.vaults.key_outbox.iter().position(|k| k.0 == id) else {
        return;
    };
    let (_, name) = app.vaults.state.key_outbox.remove(at);
    app.outbox.retain(|i| i.name != name);
    app.save_boxes();
}

/// The person took `name` out of the Outbox: if it is the `.keyx` of a
/// vault's pending made key, that key is taken back, so the next seal
/// writes the vault's own components and not a key with no document.
pub(crate) fn outbox_removed(app: &mut Faraday, name: &str) {
    let Some(at) = app
        .vaults
        .key_outbox
        .iter()
        .position(|k| k.1 == name && app.vaults.open.iter().any(|o| o.id == k.0))
    else {
        return;
    };
    let id = app.vaults.key_outbox[at].0;
    let was = app.vaults.store().key_file_changing(id);
    if app.vaults.edit(id, |s| s.unmake_key_file(id)) {
        count_key_change(app, id, was);
    } else if app.vaults.store().key_file_made(id) {
        // Refused (a sealing vault): the key is still pending, so its
        // `.keyx` stays listed.
        return;
    }
    app.vaults.state.key_outbox.remove(at);
}

/// A change of vault `id`'s key file counts once, however many times it
/// is made, chosen or removed before the seal: `was` is whether one was
/// waiting before the edit. A key file change taken back leaves none.
fn count_key_change(app: &mut Faraday, id: VaultId, was: bool) {
    let now = app.vaults.store().key_file_changing(id);
    if now && !was {
        app.vaults.changed(id);
    } else if was && !now {
        app.vaults.unchanged(id);
    }
}

/// The dialog and the sheet gone.
fn done(app: &mut Faraday) {
    app.vaults.state.more = None;
    if app.sheet == Some(Sheet::KeyFile) {
        app.sheet = None;
    }
}

/// The name `{stem}.keyx` in the Outbox: a number after the stem where
/// another file of Files or the Outbox has it, so no file is replaced.
fn free_keyx(app: &Faraday, vault_name: &str) -> String {
    let stem = vault_name.strip_suffix(".kdbx").unwrap_or(vault_name);
    let held = |n: &str| {
        app.outbox
            .iter()
            .chain(app.inbox.iter())
            .any(|i| i.name.eq_ignore_ascii_case(n))
    };
    let first = format!("{stem}.keyx");
    if !held(&first) {
        return first;
    }
    (2..100)
        .map(|n| format!("{stem}-{n}.keyx"))
        .find(|n| !held(n))
        .unwrap_or_else(|| format!("{stem}-100.keyx"))
}

/// **Make a key file**: drawn now, its `.keyx` in the Outbox at once; the
/// next seal writes the vault under it.
pub(crate) fn make(app: &mut Faraday) {
    let Some((id, name)) = target(app) else {
        return;
    };
    let Some(bytes) = app.fresh_bytes(b"vault key file", 32) else {
        done(app);
        app.toast(EN.no_randomness);
        return;
    };
    let mut random = Zeroizing::new([0u8; 32]);
    random.copy_from_slice(&bytes);
    drop(bytes);
    let was = app.vaults.store().key_file_changing(id);
    if !app.vaults.edit(id, |s| s.make_key_file(id, &random)) {
        done(app);
        return;
    }
    let reveal = faraday_keys::Reveal::OutboxFile(faraday_keys::Part::KeyFile);
    let held = app.vaults.edit(id, |s| s.key_file_document(id, reveal));
    let Some(document) = held.map(|h| h.clone()) else {
        app.vaults.edit(id, |s| s.unmake_key_file(id));
        count_key_change(app, id, was);
        unkeep(app, id);
        app.vaults.state.key_chosen = None;
        done(app);
        app.toast(EN.vault_key_file_not_kept_open);
        return;
    };
    unkeep(app, id);
    let keyx = free_keyx(app, &name);
    let item = crate::Item::new(&app.compute(), &keyx, document.to_vec());
    app.outbox_secret(crate::secrets::OutReason::KeyFile, item);
    app.vaults.state.key_outbox.push((id, keyx));
    app.vaults.state.key_chosen = None;
    count_key_change(app, id, was);
    done(app);
    app.toast(EN.vault_key_file_next_seal);
}

/// **Choose a file**: the list of the files in Files that are not vaults.
pub(crate) fn choose(app: &mut Faraday) {
    if target(app).is_none() {
        return;
    }
    app.vaults.state.key_pick = 0;
    app.vaults.state.more = Some(More::Pick);
}

/// **Choose** in that list: the marked file is read now and is the next
/// seal's key file; a refused one is named and nothing changes.
pub(crate) fn chosen(app: &mut Faraday) {
    let Some((id, _)) = target(app) else {
        return;
    };
    let files = unlock_keys::candidates(app);
    let Some((file, _)) = files.get(app.vaults.key_pick).cloned() else {
        return;
    };
    match unlock_keys::read_named(app, &file) {
        Ok(key) => {
            let was = app.vaults.store().key_file_changing(id);
            if app.vaults.edit(id, |s| s.set_key_file(id, Some(key))) {
                unkeep(app, id);
                app.vaults.state.key_chosen = Some((id, file));
                count_key_change(app, id, was);
                app.toast(EN.vault_key_file_next_seal);
            }
        }
        Err(Bad::KeyFile(e) | Bad::Secret(e)) => app.toast(&e),
    }
    done(app);
}

/// **Remove key file**: the next seal writes none.
pub(crate) fn remove(app: &mut Faraday) {
    let Some((id, _)) = target(app) else {
        return;
    };
    let was = app.vaults.store().key_file_changing(id);
    if app.vaults.edit(id, |s| s.set_key_file(id, None)) {
        unkeep(app, id);
        app.vaults.state.key_chosen = None;
        count_key_change(app, id, was);
        app.toast(EN.vault_key_file_next_seal);
    }
    done(app);
}

/// The vault `id` sealed: a key file chosen for it leaves Files, wiped as
/// it drops.
pub(crate) fn sealed(app: &mut Faraday, id: VaultId) {
    // The seal wrote the made key: its `.keyx` stays.
    app.vaults.state.key_outbox.retain(|k| k.0 != id);
    if app.vaults.key_chosen.as_ref().is_some_and(|c| c.0 == id)
        && let Some((_, name)) = app.vaults.state.key_chosen.take()
    {
        app.inbox.retain(|i| i.name != name);
        app.save_boxes();
    }
}

/// The screen on the key file sheet: the menu, the dialog or the list.
pub(crate) fn draw(app: &View, ui: &mut Ui, w: f32, h: f32) {
    match app.vaults.more {
        Some(More::Menu(anchor)) => menu_sheet(app, ui, w, h, anchor),
        Some(More::Key) => dialog(app, ui, w, h),
        Some(More::Pick) => unlock_keys::dialog(app, ui, w, h),
        None => {}
    }
}

/// The ⋮ menu: the vault's name over its choices.
fn menu_sheet(app: &View, ui: &mut Ui, w: f32, h: f32, anchor: Option<(i32, i32, i32, i32)>) {
    let Some(v) = app.vaults.open.get(app.vaults.current) else {
        return;
    };
    let (key, yubikey) = offered(app);
    let mut lines = Vec::new();
    if key {
        lines.push(Line::Item(EN.vault_key_file.to_string(), va(V::VKeyAsk)));
    }
    if yubikey {
        let label = if app.vaults.store().yubikey_removal_pending(v.id) {
            EN.vault_keep_yubikey
        } else {
            EN.vault_remove_yubikey
        };
        lines.push(Line::Item(label.to_string(), va(V::YubiKeyRemoveAsk)));
    }
    let title = v.label(app.vaults.store());
    menu::draw(
        ui,
        w,
        h,
        &menu::View {
            title: &title,
            icon: Icon::Lock,
            lines: &lines,
            at: 0,
            keys: false,
            anchor,
            cancel: (EN.vault_cancel, va(V::KeyFileCancel)),
            shown: 1.0,
        },
    );
}

/// **Key file**: what the next seal writes, the warning, the ways to
/// change it and **Cancel**.
fn dialog(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(id) = app.vaults.current_id() else {
        return;
    };
    let has = app.vaults.store().needs(id).is_some_and(|n| n.0);
    let next = fill(
        EN.vault_next_seal_key_file,
        &[if has {
            EN.vault_next_seal_a_key_file
        } else {
            EN.vault_next_seal_none
        }],
    );
    let head = (Icon::File.into(), Role::Primary.paint(), EN.vault_key_file);
    let actions = [(EN.vault_cancel, Style::Secondary, va(V::KeyFileCancel))];
    let mut rows = vec![
        (EN.vault_make_key_file, V::VKeyMake),
        (EN.vault_choose_a_file, V::VKeyChoose),
    ];
    if has {
        rows.push((EN.vault_remove_key_file, V::VKeyRemove));
    }
    crate::compact::dialog(
        ui,
        (w, h, DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += ui.wrap_as((x, cy, iw), BODY_M, Role::OnSurface, &next) + GAP_SMALL;
            cy += ui.wrap_as(
                (x, cy, iw),
                BODY_S,
                Role::OnSurfaceVariant,
                EN.vault_key_file_warning,
            ) + GAP;
            for (label, act) in &rows {
                ui.fill(
                    x,
                    cy,
                    iw,
                    LIST_1,
                    RADIUS_SMALL,
                    Role::SurfaceContainerHighest,
                );
                ui.text_mid_as((x + PAD, cy, LIST_1), BODY_M, Role::OnSurface, label);
                ui.press_box(x, cy, iw, LIST_1, va(*act));
                cy += LIST_1 + GAP_SMALL;
            }
            cy + GAP - GAP_SMALL - y
        },
        &actions,
    );
}
