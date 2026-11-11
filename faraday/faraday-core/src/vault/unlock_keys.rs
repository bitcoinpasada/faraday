//! What Unlock takes besides the passphrase (`docs/VAULT.md` §3.2, §4): a
//! key file chosen from the files in Files that are not vaults, and a
//! YubiKey's secret typed. Neither is read until **Unlock**; nothing is
//! remembered once the vault opens, the person goes back or the device
//! locks. A key file is wiped from Files and removed once a vault has
//! opened with it.

use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use crate::vault::VaultAction as V;
use crate::vault::screen::secret_box;
use crate::vault::{Focus, TextBox};
use crate::wallet::FileKind;
use crate::{Action, Faraday, Sheet};
use faraday_draw::tokens::{
    BODY_M, BODY_S, BUTTON_H, DIALOG_MAX, FIELD_H, FILTER_CHIP_H, GAP, GAP_SMALL, GRID, ICON_SMALL,
    LABEL_L, LIST_1, PAD, RADIUS_SMALL, TITLE_S, shape_full,
};
use faraday_draw::widgets::Icon;
use faraday_keys::{PartsRefused, UnlockParts};
use faraday_vault::kdbx::keyfile::KeyFileError;

/// The mark that takes a chip's or a field's content away.
const TAKE_AWAY: &str = "\u{d7}";

fn va(a: V) -> Action {
    Action::Vault(a)
}

/// Why Unlock's key file or secret was refused before any run.
pub(crate) enum Bad {
    /// The key file, said under the buttons.
    KeyFile(String),
    /// The secret, said under its field.
    Secret(String),
}

/// The files in Files that a key file can be chosen from: those that are
/// not vaults, by name and size.
pub(crate) fn candidates(app: &Faraday) -> Vec<(String, usize)> {
    app.inbox
        .iter()
        .filter(|i| i.kind != FileKind::Vault)
        .map(|i| (i.name.clone(), i.bytes.len()))
        .collect()
}

/// The bytes of the file `name` of Files, or why there are none.
fn bytes_named<'a>(app: &'a Faraday, name: &str) -> Result<&'a [u8], Bad> {
    match app.inbox.iter().find(|i| i.name == name) {
        Some(item) => Ok(&item.bytes),
        None => Err(Bad::KeyFile(fill(EN.vault_key_file_gone, &[name]))),
    }
}

/// What a refused key file says, naming the file.
fn key_file_line(e: KeyFileError, name: &str) -> String {
    let line = match e {
        KeyFileError::Empty => EN.vault_key_file_empty,
        KeyFileError::Database => EN.vault_key_file_vault,
        KeyFileError::Damaged => EN.vault_key_file_damaged,
    };
    fill(line, &[name])
}

/// Takes the key file `name` of Files, read now (in `faraday-keys`), as
/// the next seal's key file of open vault `id`: whether the vault took it,
/// or why the file is refused, naming it.
pub(crate) fn set_named(
    app: &mut Faraday,
    id: faraday_keys::vault::VaultId,
    name: &str,
) -> Result<bool, Bad> {
    let Some(item) = app.inbox.iter().find(|i| i.name == name) else {
        return Err(Bad::KeyFile(fill(EN.vault_key_file_gone, &[name])));
    };
    let bytes = &item.bytes;
    app.vaults
        .edit_or(id, |s| s.set_key_file_from(id, bytes))
        .map_err(|e| Bad::KeyFile(key_file_line(e, name)))
}

/// The key file `name` of Files alone as the make takes it (read in
/// `faraday-keys`); why it is refused otherwise, naming the file.
pub(crate) fn file_parts(app: &Faraday, name: &str) -> Result<UnlockParts, Bad> {
    UnlockParts::read(Some(bytes_named(app, name)?), None).map_err(|e| match e {
        PartsRefused::KeyFile(e) => Bad::KeyFile(key_file_line(e, name)),
        PartsRefused::Secret(_) => Bad::Secret(String::new()),
    })
}

/// The key file and the secret Unlock has been given, as the open takes
/// them (read in `faraday-keys`); why one is refused otherwise, naming the
/// file.
pub(crate) fn parts(app: &Faraday) -> Result<UnlockParts, Bad> {
    let named = match app.vaults.unlock_key.as_deref() {
        None => None,
        Some(name) => Some((name, bytes_named(app, name)?)),
    };
    let typed = app.vaults.yubikey.as_ref().map(|b| b.text.as_str());
    UnlockParts::read(named.map(|(_, b)| b), typed).map_err(|e| match e {
        PartsRefused::KeyFile(e) => Bad::KeyFile(key_file_line(e, named.map_or("", |n| n.0))),
        PartsRefused::Secret(e) if e.stray => Bad::Secret(EN.vault_secret_stray.to_string()),
        PartsRefused::Secret(e) => {
            Bad::Secret(fill(EN.vault_secret_digits, &[&e.digits.to_string()]))
        }
    })
}

/// A vault opened with the key file: it leaves Files, wiped as it drops.
pub(crate) fn used(app: &mut Faraday) {
    if let Some(name) = app.vaults.state.unlock_key.take() {
        app.inbox.retain(|i| i.name != name);
        app.save_boxes();
    }
    app.vaults.clear_unlock_parts();
}

/// **Add a key file**, or the chip: the dialog, the chosen file marked.
pub(crate) fn ask(app: &mut Faraday) {
    let files = candidates(app);
    let chosen = app
        .vaults
        .unlock_key
        .as_ref()
        .and_then(|n| files.iter().position(|f| &f.0 == n));
    app.vaults.state.key_pick = chosen.unwrap_or(0);
    app.vaults.state.more = None;
    app.vaults.state.key_for_create = false;
    app.vaults.state.key_menu = false;
    app.sheet = Some(Sheet::KeyFile);
}

/// A file of the dialog marked.
pub(crate) fn pick(app: &mut Faraday, n: usize) {
    if n < candidates(app).len() {
        app.vaults.state.key_pick = n;
    }
}

/// **Choose**: the marked file is the key file. Nothing is read.
pub(crate) fn choose(app: &mut Faraday) {
    if app.vaults.more.is_some() {
        return super::more::chosen(app);
    }
    let files = candidates(app);
    if let Some((name, _)) = files.get(app.vaults.key_pick) {
        if app.vaults.key_for_create {
            if let Some(c) = app.vaults.state.create.as_mut() {
                c.key = super::KeyChoice::File(name.clone());
                c.error = None;
            }
        } else {
            app.vaults.state.unlock_key = Some(name.clone());
            app.vaults.state.unlock_error = None;
        }
    }
    cancel(app);
}

/// **Cancel**.
pub(crate) fn cancel(app: &mut Faraday) {
    app.vaults.state.more = None;
    if app.sheet == Some(Sheet::KeyFile) {
        app.sheet = None;
    }
    app.vaults.state.key_for_create = false;
    app.vaults.state.key_menu = false;
}

/// The chip's ×.
pub(crate) fn drop_key(app: &mut Faraday) {
    app.vaults.state.unlock_key = None;
    app.vaults.state.unlock_error = None;
}

/// **Add a YubiKey secret**: its field, typed in at once.
pub(crate) fn add_secret(app: &mut Faraday) {
    app.vaults.state.yubikey = Some(TextBox::default());
    app.vaults.state.focus = app.may_load_keys().then_some(Focus::YubiKey);
}

/// The field's ×: the secret is wiped.
pub(crate) fn drop_secret(app: &mut Faraday) {
    app.vaults.state.yubikey = None;
    app.vaults.state.secret_error = None;
    app.vaults.state.unlock_error = None;
    if app.vaults.focus == Some(Focus::YubiKey) {
        app.vaults.state.focus = Some(Focus::Passphrase);
    }
}

/// **Add a key file**, or the input chip for the file chosen (its name and
/// its ×), placed by `place`, which gives the corner a control `width` wide
/// takes. `ask` is the button's and the name's action, `drop` the ×'s.
pub(crate) fn key_slot(
    ui: &mut Ui,
    chosen: Option<&str>,
    w: f32,
    place: &mut dyn FnMut(f32) -> (f32, f32),
    (ask, drop): (V, V),
) {
    let ink = Role::OnSurface;
    let muted = Role::OnSurfaceVariant;
    match chosen {
        None => {
            let width = ui.button_width(EN.vault_add_key_file, Style::Ghost);
            let (bx, by) = place(width);
            ui.button(
                bx,
                by,
                None,
                BUTTON_H,
                EN.vault_add_key_file,
                Style::Ghost,
                va(ask),
            );
        }
        Some(name) => {
            // An input chip: the file's name and its ×.
            let mark = FILTER_CHIP_H;
            let room = w - mark - 2.0 * GAP;
            let name = ui.fit_as(LABEL_L, name, room);
            let width = ui.measure_as(LABEL_L, &name) + mark + GAP;
            let (bx, by) = place(width);
            let top = by + (BUTTON_H - FILTER_CHIP_H) / 2.0;
            let r = shape_full(FILTER_CHIP_H);
            ui.stroke(bx, top, width, FILTER_CHIP_H, r, Role::Outline);
            ui.text_mid_as((bx + GAP, top, FILTER_CHIP_H), LABEL_L, ink, &name);
            let touch = ui.touch.max(BUTTON_H);
            let ty = by + (BUTTON_H - touch) / 2.0;
            ui.press_box(bx, ty, width - mark, touch, va(ask));
            let mx = bx + width - mark;
            let glyph = ui.measure_as(LABEL_L, TAKE_AWAY);
            ui.text_mid_as(
                (mx + (mark - glyph) / 2.0, top, FILTER_CHIP_H),
                LABEL_L,
                muted,
                TAKE_AWAY,
            );
            // The × is pressed over its full target, reaching into the name's.
            let reach = touch.max(mark);
            ui.press_box(mx + (mark - reach) / 2.0, ty, reach, touch, va(drop));
        }
    }
}

/// The two text buttons or their chip and field, under the passphrase at
/// (x, y), `w` wide. Returns the height they take.
pub(crate) fn draw(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    let muted = Role::OnSurfaceVariant;
    let mut cx = x;
    let mut cy = y;
    let mut place = |width: f32| -> (f32, f32) {
        if cx > x && cx + width > x + w {
            cx = x;
            cy += BUTTON_H + GAP_SMALL;
        }
        let at = (cx, cy);
        cx += width + GAP_SMALL;
        at
    };
    key_slot(
        ui,
        app.vaults.unlock_key.as_deref(),
        w,
        &mut place,
        (V::KeyFileAsk, V::KeyFileDrop),
    );
    if app.vaults.yubikey.is_none() {
        let width = ui.button_width(EN.vault_add_yubikey, Style::Ghost);
        let (bx, by) = place(width);
        ui.button(
            bx,
            by,
            None,
            BUTTON_H,
            EN.vault_add_yubikey,
            Style::Ghost,
            va(V::YubiKeyAdd),
        );
    }
    let mut end = cy + BUTTON_H;
    if let Some(b) = app.vaults.yubikey.as_ref() {
        end += GAP_SMALL;
        crate::screens::section_label(ui, x, end, EN.vault_yubikey_secret);
        end += ui.line_as(TITLE_S) + GAP_SMALL;
        let focused = app.vaults.focus == Some(Focus::YubiKey);
        secret_box(
            ui,
            x,
            end,
            w - FIELD_H,
            b,
            app.pass_shows(app.vaults.typed_shown, &b.text),
            focused,
            va(V::FocusYubiKey),
            va(V::ShowTyped),
        );
        let glyph = ui.measure_as(LABEL_L, TAKE_AWAY);
        let gx = x + w - FIELD_H;
        ui.text_mid_as(
            (gx + (FIELD_H - glyph) / 2.0, end, FIELD_H),
            LABEL_L,
            muted,
            TAKE_AWAY,
        );
        ui.press_box(gx, end, FIELD_H, FIELD_H, va(V::YubiKeyDrop));
        end += FIELD_H + GAP_SMALL;
        if let Some(e) = &app.vaults.secret_error {
            end += ui.wrap_as((x, end, w), BODY_M, Role::Error, e) + GAP_SMALL;
        }
    }
    end - y
}

/// **Choose a key file**: the files in Files that are not vaults, one
/// line each with a radio mark, **Cancel** and **Choose**.
pub(crate) fn dialog(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let files = candidates_view(app);
    let marked = app.vaults.key_pick;
    let head = (
        Icon::File.into(),
        Role::Primary.paint(),
        EN.vault_choose_key_file,
    );
    let actions = [
        (EN.vault_cancel, Style::Secondary, va(V::KeyFileCancel)),
        (
            EN.vault_choose,
            Style::Primary.when(!files.is_empty()),
            va(V::KeyFileChoose),
        ),
    ];
    crate::compact::dialog(
        ui,
        (w, h, DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            if files.is_empty() {
                cy += ui.wrap_as(
                    (x, cy, iw),
                    BODY_S,
                    Role::OnSurfaceVariant,
                    EN.vault_no_key_files,
                ) + GAP;
            }
            for (k, name) in files.iter().enumerate() {
                let on = k == marked;
                let fill_ = selected_or(on, Role::SurfaceContainerHighest);
                ui.fill(x, cy, iw, LIST_1, RADIUS_SMALL, fill_);
                ui.stroke(
                    x,
                    cy,
                    iw,
                    LIST_1,
                    RADIUS_SMALL,
                    selected_or(on, Role::OutlineVariant),
                );
                let mark = on_selected(on, Role::OutlineVariant);
                let ry = cy + (LIST_1 - ICON_SMALL) / 2.0;
                ui.stroke(
                    x + PAD,
                    ry,
                    ICON_SMALL,
                    ICON_SMALL,
                    shape_full(ICON_SMALL),
                    mark,
                );
                if on {
                    let dot = ICON_SMALL / 2.0;
                    ui.fill(
                        x + PAD + dot / 2.0,
                        ry + dot / 2.0,
                        dot,
                        dot,
                        shape_full(dot),
                        mark,
                    );
                }
                let room = iw - 2.0 * PAD - ICON_SMALL - GRID;
                let line = ui.fit_as(BODY_M.mono(), name, room);
                ui.text_mid_as(
                    (x + PAD + ICON_SMALL + GRID, cy, LIST_1),
                    BODY_M.mono(),
                    on_selected(on, Role::OnSurface),
                    &line,
                );
                ui.press_box(x, cy, iw, LIST_1, va(V::KeyFilePick(k)));
                cy += LIST_1 + GAP_SMALL;
            }
            cy + GAP - GAP_SMALL - y
        },
        &actions,
    );
}

/// [`candidates`] for drawing: the names.
fn candidates_view(app: &View) -> Vec<String> {
    app.inbox
        .iter()
        .filter(|i| i.kind != FileKind::Vault)
        .map(|i| i.name.clone())
        .collect()
}
