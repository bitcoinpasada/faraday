//! The vault screens (`docs/history/PLAN.md` §6.1, the prototype's Vaults, Unlock,
//! Create a vault and Vault contents): drawing only. What a press does is
//! `vault/mod.rs`.

use crate::strings::EN;
use crate::strings::fill;
use faraday_crypto::work::Compute;
use faraday_draw::widgets::Icon;

use crate::demo::Show;
use crate::drawing::{Scrolled, View};
use crate::screens::{guide_text, never_real_seed_if, never_under, section_label, title};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::vault::{
    self, CATEGORIES, CUSTOM, CUSTOM_MEMORY, CUSTOM_PASSES, Focus, HOLD_MS, MACHINES, PRESETS,
    SIZES, TextBox, VaultAction as V, cost_text, file_text, mib_text, size_text, vstep,
};
use crate::wallet::{Session, Wallet, fp_text};
use crate::{Action, Code, Faraday, Screen, flow};
use faraday_draw::tokens;
use faraday_keys::vault::RecordView;
use faraday_keys::{Part, Reveal};
use faraday_vault::records::{field, kind};
use faraday_vault::{SLOT_SIZES, file_len};

fn va(a: V) -> Action {
    Action::Vault(a)
}

/// A waiting action: **Back up first**, which opens the flow of the key
/// shown on its step.
fn back_up_first(kind_: crate::backup_flow::Kind) -> Action {
    Action::Flow(crate::backup_flow::steps::FlowAction::BackUpFirst(kind_))
}

/// A file's size, for Sign a file's rows.
fn gpg_file_size(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        fill(EN.vault_mb, &[&format!("{:.1}", bytes as f32 / 1e6)])
    } else if bytes >= 1000 {
        fill(EN.vault_kb, &[&format!("{:.1}", bytes as f32 / 1e3)])
    } else {
        format!("{bytes} B")
    }
}

/// A line of typing: masked for a passphrase or a secret field, with a
/// caret while typing goes to it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn text_box(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    b: &TextBox,
    masked: bool,
    focused: bool,
    action: Action,
) {
    ui.fill(x, y, w, 40.0, 8.0, BG);
    ui.stroke(x, y, w, 40.0, 8.0, if focused { ACCENT } else { BORDER });
    // The dots stand for the characters one to one, so a press or a
    // selection in them is by position.
    let dots;
    let shown: &str = if masked {
        dots = "•".repeat(b.text.chars().count());
        &dots
    } else {
        &b.text
    };
    ui.typed(
        x + 12.0,
        y,
        40.0,
        14.0,
        W::M,
        TEXT,
        shown,
        w - 30.0,
        focused,
        action,
    );
    ui.hit(x, y, w, 40.0, action);
}

/// The warning that stands above a passphrase while a stick is attached:
/// nothing secret is typed until it is pulled. Returns its height.
pub(crate) fn stick_banner(ui: &mut Ui, x: f32, y: f32, w: f32, text: &str) -> f32 {
    if ui.compact {
        // Wrapped beside its icon on a small panel.
        ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
        let th = ui.wrap(x + 40.0, y + 12.0, w - 52.0, 13.0, W::S, WARN, text);
        ui.c.pop_clip();
        let bh = th + 24.0;
        ui.fill(x, y, w, bh, 10.0, WARN.with_alpha(30));
        ui.stroke(x, y, w, bh, 10.0, WARN.with_alpha(110));
        ui.icon(
            x + 10.0,
            y + bh / 2.0 - 10.0,
            20.0,
            Icon::Warning,
            11.0,
            WARN,
        );
        ui.wrap(x + 40.0, y + 12.0, w - 52.0, 13.0, W::S, WARN, text);
        return bh + 12.0;
    }
    ui.fill(x, y, w, 48.0, 10.0, WARN.with_alpha(30));
    ui.stroke(x, y, w, 48.0, 10.0, WARN.with_alpha(110));
    ui.icon(x + 14.0, y + 14.0, 20.0, Icon::Warning, 11.0, WARN);
    ui.text_mid(x + 44.0, y, 48.0, 14.0, W::S, WARN, text);
    60.0
}

/// A passphrase field that takes no typing: a stick is attached.
pub(crate) fn stick_field(medium: crate::Medium, ui: &mut Ui, x: f32, y: f32, w: f32) {
    ui.fill(x, y, w, 40.0, 8.0, SURFACE);
    ui.stroke(x, y, w, 40.0, 8.0, INNER);
    let text = fill(EN.vault_remove_the_first, &[medium.noun()]);
    ui.text_mid(x + 12.0, y, 40.0, 13.0, W::R, DIM, &text);
}

/// A passphrase field with an eye at its right end: pressing the eye
/// shows what is typed in the clear, pressing it again masks it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn secret_box(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    b: &TextBox,
    show: Show,
    focused: bool,
    action: Action,
    eye: Action,
) {
    // Demo's `faraday`: in its code box, no eye; a press in it focuses the
    // field.
    if show == Show::Code {
        crate::screens::code_field(ui, x, y, w, tokens::TEXT_FIELD, &b.text);
        ui.hit(x, y, w, tokens::TEXT_FIELD, action);
        return;
    }
    let shown = show == Show::Shown;
    text_box(ui, x, y, w, b, !shown, focused, action);
    // The eye sits inside the field's right end, over any long text.
    let ex = x + w - 40.0;
    let pressed = ui.is_pressed(eye);
    ui.fill(
        ex + 4.0,
        y + 4.0,
        32.0,
        32.0,
        6.0,
        if pressed { INNER } else { BG },
    );
    ui.icon(
        ex,
        y,
        40.0,
        if shown { Icon::EyeOff } else { Icon::Eye },
        15.0,
        if shown { ACCENT } else { MUTED },
    );
    ui.hit(ex, y, 40.0, 40.0, eye);
}

/// Several lines of typing. Returns its height.
fn text_area(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    b: &TextBox,
    focused: bool,
    action: Action,
) -> f32 {
    let mut shown = b.text.clone();
    if focused {
        shown.push_str(ui.caret_char());
    }
    let th = ui.wrap(x + 12.0, y + 10.0, w - 24.0, 14.0, W::R, TEXT, &shown);
    let h = (th + 20.0).max(96.0);
    ui.stroke(x, y, w, h, 8.0, if focused { ACCENT } else { BORDER });
    ui.hit(x, y, w, h, action);
    h
}

/// A secret value shown only while held.
fn secret(ui: &mut Ui, x: f32, y: f32, w: f32, value: &str, mono: bool, id: usize) -> f32 {
    secret_with(ui, x, y, w, || value.to_string().into(), mono, id)
}

/// A secret value shown only while held, made only then.
fn secret_with(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    value: impl FnOnce() -> zeroize::Zeroizing<String>,
    mono: bool,
    id: usize,
) -> f32 {
    let action = va(V::Reveal(id));
    let held = ui.is_pressed(action);
    let face = if mono { W::M } else { W::R };
    let value = if held {
        value()
    } else {
        zeroize::Zeroizing::new("•".repeat(24))
    };
    let shown = if held {
        crate::secret_text::SecretText::of(&value)
    } else {
        crate::secret_text::SecretText::of(&"•".repeat(value.chars().count().clamp(8, 24)))
    };
    let tw = w - 28.0;
    // Measured in a pass that draws nothing, so the field's surface goes
    // down before the words and does not cover them.
    let th = if held {
        ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
        let th = ui.wrap(x + 14.0, y + 10.0, tw, 14.0, face, TEXT, &shown);
        ui.c.pop_clip();
        th
    } else {
        20.0
    };
    let bh = th.max(20.0) + 38.0;
    ui.fill(x, y, w, bh, 8.0, BG);
    ui.stroke(
        x,
        y,
        w,
        bh,
        8.0,
        if held { WARN } else { WARN.with_alpha(110) },
    );
    if held {
        ui.wrap(x + 14.0, y + 10.0, tw, 14.0, face, TEXT, &shown);
    } else {
        ui.text(x + 14.0, y + 10.0, 14.0, W::M, MUTED, &shown);
    }
    let hint = if held {
        EN.vault_release_to_hide
    } else {
        EN.vault_hold_to_show
    };
    ui.text(x + 14.0, y + bh - 24.0, 11.0, W::R, WARN, hint);
    ui.hit(x, y, w, bh, action);
    bh
}

/// Show SeedQR: the selected key's SeedQR shown only while held, behind
/// the same hold as its words. The code is the one the press made
/// ([`Faraday::vault_seedqr_press`]); drawing makes none. Returns the
/// height.
fn seedqr_box(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let action = va(V::ShowSeedQr);
    let held = ui.is_pressed(action);
    let code = app
        .vaults
        .seedqr
        .as_ref()
        .filter(|(at, _)| held && Some(*at) == app.vault_selected_index())
        .map(|(_, m)| m);
    // The code's side, as a code is shown anywhere in the app, or the
    // field's width where that is less.
    let class = if ui.compact {
        faraday_draw::SizeClass::Small
    } else {
        faraday_draw::SizeClass::Wide
    };
    let side = faraday_draw::tokens::qr_side(class).min(w - 28.0);
    let bh = if code.is_some() { side + 58.0 } else { 58.0 };
    ui.fill(x, y, w, bh, 8.0, BG);
    ui.stroke(
        x,
        y,
        w,
        bh,
        8.0,
        if held { WARN } else { WARN.with_alpha(110) },
    );
    match code {
        Some(m) => crate::screens::qr_code(ui, m, x + 14.0, y + 10.0, side),
        None => {
            ui.text(x + 14.0, y + 10.0, 14.0, W::S, MUTED, EN.vault_show_seedqr);
        }
    }
    let hint = if held {
        EN.vault_release_to_hide
    } else {
        EN.vault_hold_to_show
    };
    ui.text(x + 14.0, y + bh - 24.0, 11.0, W::R, WARN, hint);
    ui.hit(x, y, w, bh, action);
    bh
}

/// "Hold to delete from vault", filling while it is held.
fn hold_delete(app: &View, ui: &mut Ui, x: f32, y: f32) -> f32 {
    hold_button(
        app,
        ui,
        x,
        y,
        EN.vault_hold_to_delete,
        va(V::HoldDelete),
        ERR,
    )
}

/// A button that acts when held, filling while it is held.
pub(crate) fn hold_button(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    label: &str,
    action: Action,
    color: faraday_draw::Color,
) -> f32 {
    hold_button_in(app, ui, x, y, f32::MAX, label, action, color)
}

/// [`hold_button`] no wider than `room`: a long label is cut to fit.
#[allow(clippy::too_many_arguments)]
pub(crate) fn hold_button_in(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    room: f32,
    label: &str,
    action: Action,
    color: faraday_draw::Color,
) -> f32 {
    let w = (ui.measure(13.0, W::S, label) + 32.0).min(room);
    let label = ui.fit(13.0, W::S, label, w - 32.0);
    let label = label.as_str();
    if ui.is_pressed(action) {
        let held = app
            .now_ms
            .saturating_sub(app.vaults.pressed_at)
            .min(HOLD_MS) as f32
            / HOLD_MS as f32;
        ui.fill(x, y, w * held, 36.0, 10.0, color.with_alpha(60));
    }
    ui.stroke(x, y, w, 36.0, 10.0, color.with_alpha(115));
    ui.text_mid(x + 16.0, y, 36.0, 13.0, W::S, color, label);
    ui.hit(x, y, w, 36.0, action);
    w
}

// ---------------------------------------------------------------------
// The list
// ---------------------------------------------------------------------

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn list(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return list_compact(app, ui, x0, cw, h);
    }
    let x = x0 + 48.0;
    let w = cw - 96.0;
    let mut y = 36.0;
    // Made from another flow: the way back to it, and no second vault
    // started in its place (that would drop the way back).
    if let Some(s) = app.vaults.back_to {
        back_link(app, ui, x, 28.0, (app.back_link_name(s), va(V::Back)));
        y = 54.0;
    }
    title(ui, x, y, EN.vault_vaults);
    if app.vaults.back_to.is_none() {
        let bw = ui.measure(15.0, W::S, EN.vault_create_a_vault) + 32.0;
        ui.button(
            x + w - bw,
            y - 4.0,
            Some(bw),
            44.0,
            EN.vault_create_a_vault,
            Style::Primary,
            va(V::Create),
        );
    }
    y += 60.0;
    y += guide_text(
        app,
        ui,
        x,
        y,
        w,
        &fill(
            EN.vault_a_vault_holds,
            &[app.medium.a(), app.medium.for_the()],
        ),
    );
    let files = app.vault_files();
    if files.is_empty() {
        ui.text(x, y + 8.0, 15.0, W::R, MUTED, EN.vault_no_vault_in_files);
        crate::samples::put::button(app, ui, x, y + tokens::TEXT_FIELD);
        return;
    }
    let in_inbox = |salt: &[u8; 32]| {
        app.inbox
            .iter()
            .filter(|i| i.kind == crate::FileKind::Vault)
            .any(|i| faraday_vault::read_header(&i.bytes).is_ok_and(|h| &h.salt == salt))
    };
    for (i, f) in files.iter().enumerate() {
        let open = f.open.and_then(|o| app.vaults.open.get(o));
        let fresh = f.in_outbox && !in_inbox(&f.header.salt);
        // A locked vault seen open since power-on goes by its name.
        let remembered = app.vault_summary(f).map(|s| s.name.as_str());
        let (title_text, state, state_color, icon_color, edge) = match open {
            Some(v) if v.changes > 0 => (
                v.label(app.vaults.store()),
                fill(
                    EN.vault_open_unsaved,
                    &[
                        &v.changes.to_string(),
                        (if v.changes == 1 {
                            EN.vault_change
                        } else {
                            EN.vault_changes
                        }),
                    ],
                ),
                WARN,
                ACCENT,
                LINE,
            ),
            Some(v) => (
                v.label(app.vaults.store()),
                EN.vault_open_button.to_string(),
                OK,
                ACCENT,
                LINE,
            ),
            None if !app.may_load_keys() => (
                remembered
                    .unwrap_or(if fresh {
                        EN.vault_new_vault
                    } else {
                        EN.vault_locked_vault
                    })
                    .to_string(),
                fill(EN.vault_remove_to_unlock, &[app.medium.noun()]),
                WARN,
                MUTED,
                LINE,
            ),
            None if fresh => (
                remembered.unwrap_or(EN.vault_new_vault).to_string(),
                EN.vault_locked_not_written.to_string(),
                OK,
                OK,
                OK.with_alpha(90),
            ),
            None => (
                remembered.unwrap_or(EN.vault_locked_vault).to_string(),
                EN.vault_locked.to_string(),
                MUTED,
                MUTED,
                LINE,
            ),
        };
        let ch = 92.0;
        ui.fill(x, y, w, ch, 12.0, SURFACE);
        ui.stroke(x, y, w, ch, 12.0, edge);
        ui.fill(
            x + 18.0,
            y + 22.0,
            48.0,
            48.0,
            10.0,
            icon_color.with_alpha(30),
        );
        ui.icon(x + 18.0, y + 22.0, 48.0, Icon::Lock, 20.0, icon_color);
        ui.text(x + 84.0, y + 16.0, 16.0, W::S, TEXT, &title_text);
        ui.text(x + 84.0, y + 42.0, 12.0, W::M, MUTED, &f.name);
        let where_ = if f.in_outbox {
            app.medium.for_box()
        } else {
            app.medium.from_box()
        };
        let line = list_line(app, f, where_);
        let line = ui.fit(12.0, W::R, &line, w - 84.0 - 300.0);
        ui.text(x + 84.0, y + 62.0, 12.0, W::R, DIM, &line);
        // An open vault: what is in it, and Lock, which seals it (and
        // every other) into the Outbox. A locked one: Unlock.
        let buttons: &[(&str, Style, Action)] = if open.is_some() {
            &[
                (EN.vault_view_contents, Style::Secondary, va(V::Open(i))),
                (EN.vault_lock, Style::Primary, Action::LockAsk),
            ]
        } else {
            &[(EN.vault_unlock, Style::Primary, va(V::Open(i)))]
        };
        let mut bx = x + w - 20.0;
        for &(label, style, action) in buttons.iter().rev() {
            let bw = ui.measure(14.0, W::S, label) + 36.0;
            bx -= bw;
            ui.button(bx, y + 26.0, Some(bw), 40.0, label, style, action);
            bx -= 8.0;
        }
        let sw = ui.measure(13.0, W::S, &state);
        ui.text_mid(
            bx - 12.0 - sw,
            y + 26.0,
            40.0,
            13.0,
            W::S,
            state_color,
            &state,
        );
        y += ch + 12.0;
    }
}

// ---------------------------------------------------------------------
// Unlock
// ---------------------------------------------------------------------

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn unlock(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return unlock_compact(app, ui, x0, cw, h);
    }
    let x = x0 + 48.0;
    let w = (cw - 96.0).min(760.0);
    let mut y = 28.0;
    back_link(app, ui, x, y, unlock_back(app));
    y += faraday_draw::tokens::BACK_LINK_ROW;
    title(ui, x, y, EN.vault_unlock);
    y += 56.0;
    if just_made_picked(app) {
        ui.text(x, y - 14.0, 14.0, W::R, MUTED, ONCE_MORE);
        y += 20.0;
    }
    y += guide_text(app, ui, x, y, w, EN.vault_choose_the_vault);
    if !app.may_load_keys() {
        let pull = fill(EN.vault_remove_the_then_type, &[app.medium.noun()]);
        y += stick_banner(ui, x, y, w, &pull);
    }
    let files = app.vault_files();
    let locked: Vec<(usize, &vault::VaultFile)> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.open.is_none())
        .collect();
    if locked.is_empty() {
        ui.text(x, y, 15.0, W::R, MUTED, EN.vault_no_locked_vault);
        ui.button(
            x,
            y + 36.0,
            None,
            40.0,
            EN.vault_create_a_vault,
            Style::Secondary,
            va(V::Create),
        );
        return;
    }
    for (i, f) in &locked {
        let on = *i == app.vaults.pick;
        let ch = 72.0;
        ui.fill(
            x,
            y,
            w,
            ch,
            12.0,
            if on { ACCENT.with_alpha(18) } else { SURFACE },
        );
        ui.stroke(x, y, w, ch, 12.0, if on { ACCENT } else { BORDER });
        let fg = if on { ACCENT } else { MUTED };
        ui.fill(x + 16.0, y + 14.0, 44.0, 44.0, 10.0, fg.with_alpha(30));
        ui.icon(x + 16.0, y + 14.0, 44.0, Icon::Lock, 18.0, fg);
        ui.text(x + 76.0, y + 14.0, 14.0, W::M, TEXT, &f.name);
        let where_ = if f.in_outbox {
            app.medium.for_box()
        } else {
            app.medium.from_box()
        };
        ui.text(
            x + 76.0,
            y + 40.0,
            12.0,
            W::R,
            MUTED,
            &format!("{where_} · {}", file_text(f.len)),
        );
        let mem = f.header.cost.memory_kib / 1024;
        let cost = cost_text(&f.header.cost);
        let time = fill(
            EN.vault_unlock_time_here,
            &[&app.vaults.time_text(mem, f.header.cost.passes).to_string()],
        );
        ui.text_right(x + w - 18.0, y + 10.0, 24.0, 13.0, W::S, TEXT, &cost);
        ui.text_right(x + w - 18.0, y + 36.0, 24.0, 12.0, W::R, MUTED, &time);
        ui.hit(x, y, w, ch, va(V::Pick(*i)));
        y += ch + 10.0;
    }
    y += 14.0;
    section_label(ui, x, y, EN.vault_passphrase);
    y += 26.0;
    if app.may_load_keys() {
        let focused = app.vaults.focus == Some(Focus::Passphrase);
        crate::demo::pass_box(
            app,
            ui,
            (x, y, w),
            &app.vaults.passphrase,
            (app.vaults.typed_shown, focused),
            (va(V::FocusPassphrase), va(V::ShowTyped)),
            crate::demo::PassField::Unlock,
        );
    } else {
        stick_field(app.medium, ui, x, y, w);
    }
    y += 56.0 + never_under(app, ui, x, y, w);
    let working = app.vaults.working == Some(vault::Work::Unlock);
    if working {
        let f = files.get(app.vaults.pick);
        let t = f
            .map(|f| {
                app.vaults
                    .time_text(f.header.cost.memory_kib / 1024, f.header.cost.passes)
            })
            .unwrap_or_default();
        ui.text_mid(
            x,
            y,
            46.0,
            15.0,
            W::S,
            ACCENT,
            &fill(EN.vault_unlocking, &[&t.to_string()]),
        );
    } else {
        let style = if app.may_load_keys() {
            Style::Primary
        } else {
            Style::Disabled
        };
        ui.button(
            x,
            y,
            Some(160.0),
            46.0,
            EN.vault_unlock,
            style,
            va(V::Unlock),
        );
        let free = match app.vaults.memory_free_mib {
            Some(m) => fill(EN.vault_this_computer_free, &[&mib_text(m).to_string()]),
            None => EN.vault_this_computer_memory.to_string(),
        };
        ui.text_mid(x + 180.0, y, 46.0, 13.0, W::R, MUTED, &free);
    }
    y += 60.0;
    if let Some(e) = &app.vaults.unlock_error {
        ui.wrap(x, y, w, 14.0, W::R, ERR, e);
    }
}

/// The third line of a vault's row: where it is, its size and cost while
/// open; once locked, what it held when last seen open, or that unlocking
/// shows it.
fn list_line(app: &View, f: &vault::VaultFile, where_: &str) -> String {
    let line = if f.open.is_some() {
        format!(
            "{where_} · {} · {}",
            file_text(f.len),
            cost_text(&f.header.cost)
        )
    } else {
        match app.vault_summary(f) {
            Some(s) => s.line(app.tz_min()),
            None => EN.vault_unlock_to_see.to_string(),
        }
    };
    // Its currency first (§3.5): never written, current on a stick, or
    // changed since written.
    match app.currency(f) {
        Some(c) => format!("{} · {line}", c.line()),
        None => line,
    }
}

/// `s`, parts split by " · ", over two lines of `w` at 12 regular: as
/// many parts as fit on the first, the rest fitted to the second.
fn two_lines(ui: &mut Ui, s: &str, w: f32) -> (String, String) {
    let parts: Vec<&str> = s.split(" · ").collect();
    let mut n = 1;
    while n < parts.len() && ui.measure(12.0, W::R, &parts[..=n].join(" · ")) <= w {
        n += 1;
    }
    let first = ui.fit(12.0, W::R, &parts[..n].join(" · "), w);
    let second = ui.fit(12.0, W::R, &parts[n..].join(" · "), w);
    (first, second)
}

/// Unlock's caption for the vault just made.
const ONCE_MORE: &str = EN.vault_type_the_passphrase;

/// Whether Unlock has the vault just made picked.
fn just_made_picked(app: &View) -> bool {
    app.vaults.just_made.as_ref().is_some_and(|n| {
        app.vault_files()
            .get(app.vaults.pick)
            .is_some_and(|f| &f.name == n)
    })
}

/// Where Unlock's way back goes: the boot import's sheet when it was
/// opened from there, else the vault list.
pub(crate) fn unlock_back(app: &View) -> (&'static str, Action) {
    match app.vaults.back_to {
        Some(Screen::Home) if app.import.open() => (EN.vault_import, crate::boot_import::OPEN),
        // Unlocking for another flow: back to it.
        Some(s) => (app.back_link_name(s), va(V::Back)),
        None => (EN.vault_vaults, Action::Nav(Screen::Vaults)),
    }
}

/// [`list`] on a small panel: Create a vault, then each vault as a card
/// with its buttons under what it says. The way back to a flow that made
/// one is the bar's.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn list_compact(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    use crate::compact::screen::{about, finish};
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    if app.vaults.back_to.is_none() {
        ui.button(
            x,
            y,
            Some(w),
            44.0,
            EN.vault_create_a_vault,
            Style::Primary,
            va(V::Create),
        );
        y += 58.0;
    }
    let files = app.vault_files();
    if files.is_empty() {
        y += ui.wrap(x, y, w, 14.0, W::R, MUTED, EN.vault_no_vault_in_files) + 12.0;
        y += crate::samples::put::button(app, ui, x, y);
    }
    let in_inbox = |salt: &[u8; 32]| {
        app.inbox
            .iter()
            .filter(|i| i.kind == crate::FileKind::Vault)
            .any(|i| faraday_vault::read_header(&i.bytes).is_ok_and(|h| &h.salt == salt))
    };
    for (i, f) in files.iter().enumerate() {
        let open = f.open.and_then(|o| app.vaults.open.get(o));
        let fresh = f.in_outbox && !in_inbox(&f.header.salt);
        // A locked vault seen open since power-on goes by its name.
        let remembered = app.vault_summary(f).map(|s| s.name.as_str());
        let (title_text, state, state_color, icon_color, edge) = match open {
            Some(v) if v.changes > 0 => (
                v.label(app.vaults.store()),
                fill(
                    EN.vault_open_unsaved,
                    &[
                        &v.changes.to_string(),
                        (if v.changes == 1 {
                            EN.vault_change
                        } else {
                            EN.vault_changes
                        }),
                    ],
                ),
                WARN,
                ACCENT,
                LINE,
            ),
            Some(v) => (
                v.label(app.vaults.store()),
                EN.vault_open_button.to_string(),
                OK,
                ACCENT,
                LINE,
            ),
            None if !app.may_load_keys() => (
                remembered
                    .unwrap_or(if fresh {
                        EN.vault_new_vault
                    } else {
                        EN.vault_locked_vault
                    })
                    .to_string(),
                fill(EN.vault_remove_to_unlock, &[app.medium.noun()]),
                WARN,
                MUTED,
                LINE,
            ),
            None if fresh => (
                remembered.unwrap_or(EN.vault_new_vault).to_string(),
                EN.vault_locked_not_written.to_string(),
                OK,
                OK,
                OK.with_alpha(90),
            ),
            None => (
                remembered.unwrap_or(EN.vault_locked_vault).to_string(),
                EN.vault_locked.to_string(),
                MUTED,
                MUTED,
                LINE,
            ),
        };
        let buttons: &[(&str, Style, Action)] = if open.is_some() {
            &[
                (EN.vault_view_contents, Style::Secondary, va(V::Open(i))),
                (EN.vault_lock, Style::Primary, Action::LockAsk),
            ]
        } else {
            &[(EN.vault_unlock, Style::Primary, va(V::Open(i)))]
        };
        let ch = 112.0 + 40.0;
        ui.fill(x, y, w, ch, 12.0, SURFACE);
        ui.stroke(x, y, w, ch, 12.0, edge);
        ui.fill(
            x + 12.0,
            y + 14.0,
            36.0,
            36.0,
            8.0,
            icon_color.with_alpha(30),
        );
        ui.icon(x + 12.0, y + 14.0, 36.0, Icon::Lock, 15.0, icon_color);
        let room = w - 60.0 - 12.0;
        let t = ui.fit(15.0, W::S, &title_text, room);
        ui.text(x + 60.0, y + 10.0, 15.0, W::S, TEXT, &t);
        let name = ui.fit(12.0, W::M, &f.name, room);
        ui.text(x + 60.0, y + 34.0, 12.0, W::M, MUTED, &name);
        let where_ = if f.in_outbox {
            app.medium.for_box()
        } else {
            app.medium.from_box()
        };
        let line = list_line(app, f, where_);
        // What a locked vault held, when it is longer than a line, takes
        // the state's line too: the Unlock button says it is locked.
        if open.is_none()
            && app.vault_summary(f).is_some()
            && ui.measure(12.0, W::R, &line) > w - 24.0
        {
            let (first, second) = two_lines(ui, &line, w - 24.0);
            ui.text(x + 12.0, y + 58.0, 12.0, W::R, DIM, &first);
            ui.text(x + 12.0, y + 78.0, 12.0, W::R, DIM, &second);
        } else {
            let line = ui.fit(12.0, W::R, &line, w - 24.0);
            ui.text(x + 12.0, y + 58.0, 12.0, W::R, DIM, &line);
            let state = ui.fit(13.0, W::S, &state, w - 24.0);
            ui.text(x + 12.0, y + 78.0, 13.0, W::S, state_color, &state);
        }
        // The buttons share the card's last row.
        let n = buttons.len() as f32;
        let bw = (w - 24.0 - 8.0 * (n - 1.0)) / n;
        for (k, &(label, style, action)) in buttons.iter().enumerate() {
            let bx = x + 12.0 + k as f32 * (bw + 8.0);
            ui.button(bx, y + 102.0, Some(bw), 40.0, label, style, action);
        }
        y += ch + 12.0;
    }
    y += about(
        ui,
        x,
        y,
        w,
        &fill(
            EN.vault_a_vault_holds,
            &[app.medium.a(), app.medium.for_the()],
        ),
    );
    finish(app, ui, x0, cw, h, y - top + 16.0);
}

/// [`unlock`] on a small panel: the vaults, the passphrase, Unlock, each
/// the panel's width.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn unlock_compact(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    use crate::compact::screen::{about, finish};
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    if just_made_picked(app) {
        y += ui.wrap(x, y, w, 14.0, W::R, MUTED, ONCE_MORE) + 12.0;
    }
    if !app.may_load_keys() {
        let pull = fill(EN.vault_remove_the_then_type, &[app.medium.noun()]);
        y += stick_banner(ui, x, y, w, &pull);
    }
    let files = app.vault_files();
    let locked: Vec<(usize, &vault::VaultFile)> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.open.is_none())
        .collect();
    if locked.is_empty() {
        y += ui.wrap(x, y, w, 14.0, W::R, MUTED, EN.vault_no_locked_vault) + 12.0;
        ui.button(
            x,
            y,
            Some(w),
            40.0,
            EN.vault_create_a_vault,
            Style::Secondary,
            va(V::Create),
        );
        finish(app, ui, x0, cw, h, y - top + 56.0);
        return;
    }
    for (i, f) in &locked {
        let on = *i == app.vaults.pick;
        let ch = 82.0;
        ui.fill(
            x,
            y,
            w,
            ch,
            12.0,
            if on { ACCENT.with_alpha(18) } else { SURFACE },
        );
        ui.stroke(x, y, w, ch, 12.0, if on { ACCENT } else { BORDER });
        let fg = if on { ACCENT } else { MUTED };
        ui.fill(x + 12.0, y + 12.0, 36.0, 36.0, 8.0, fg.with_alpha(30));
        ui.icon(x + 12.0, y + 12.0, 36.0, Icon::Lock, 15.0, fg);
        let room = w - 60.0 - 12.0;
        let name = ui.fit(14.0, W::M, &f.name, room);
        ui.text(x + 60.0, y + 10.0, 14.0, W::M, TEXT, &name);
        let where_ = if f.in_outbox {
            app.medium.for_box()
        } else {
            app.medium.from_box()
        };
        let line = ui.fit(
            12.0,
            W::R,
            &format!("{where_} · {}", file_text(f.len)),
            room,
        );
        ui.text(x + 60.0, y + 32.0, 12.0, W::R, MUTED, &line);
        let mem = f.header.cost.memory_kib / 1024;
        let cost = fill(
            EN.vault_here,
            &[
                &cost_text(&f.header.cost).to_string(),
                &app.vaults.time_text(mem, f.header.cost.passes).to_string(),
            ],
        );
        let cost = ui.fit(12.0, W::S, &cost, w - 24.0);
        ui.text(x + 12.0, y + 56.0, 12.0, W::S, TEXT, &cost);
        ui.hit(x, y, w, ch, va(V::Pick(*i)));
        y += ch + 10.0;
    }
    y += 6.0;
    section_label(ui, x, y, EN.vault_passphrase);
    y += 24.0;
    if app.may_load_keys() {
        let focused = app.vaults.focus == Some(Focus::Passphrase);
        crate::demo::pass_box(
            app,
            ui,
            (x, y, w),
            &app.vaults.passphrase,
            (app.vaults.typed_shown, focused),
            (va(V::FocusPassphrase), va(V::ShowTyped)),
            crate::demo::PassField::Unlock,
        );
    } else {
        stick_field(app.medium, ui, x, y, w);
    }
    y += 52.0 + never_under(app, ui, x, y, w);
    if app.vaults.working == Some(vault::Work::Unlock) {
        let f = files.get(app.vaults.pick);
        let t = f
            .map(|f| {
                app.vaults
                    .time_text(f.header.cost.memory_kib / 1024, f.header.cost.passes)
            })
            .unwrap_or_default();
        ui.text_mid(
            x,
            y,
            44.0,
            15.0,
            W::S,
            ACCENT,
            &fill(EN.vault_unlocking, &[&t.to_string()]),
        );
        y += 52.0;
    } else {
        let style = if app.may_load_keys() {
            Style::Primary
        } else {
            Style::Disabled
        };
        if crate::screens::pin_button(
            ui,
            x,
            y,
            Some(w),
            44.0,
            EN.vault_unlock,
            style,
            va(V::Unlock),
        ) {
            y += 52.0;
        }
        let free = match app.vaults.memory_free_mib {
            Some(m) => fill(EN.vault_this_computer_free, &[&mib_text(m).to_string()]),
            None => EN.vault_this_computer_memory.to_string(),
        };
        y += ui.wrap(x, y, w, 13.0, W::R, MUTED, &free) + 12.0;
    }
    if let Some(e) = &app.vaults.unlock_error {
        y += ui.wrap(x, y, w, 14.0, W::R, ERR, e) + 12.0;
    }
    y += about(ui, x, y, w, EN.vault_choose_the_vault);
    finish(app, ui, x0, cw, h, y - top + 16.0);
}

pub(crate) fn back_link(app: &View, ui: &mut Ui, x: f32, y: f32, back: (&str, Action)) {
    let (label, action) = app.named_back(back);
    ui.icon(x - 4.0, y, 18.0, Icon::ChevronLeft, 12.0, MUTED);
    ui.text_mid(x + 14.0, y, 18.0, 13.0, W::R, MUTED, label);
    let lw = ui.measure(13.0, W::R, label) + 20.0;
    ui.hit(x - 6.0, y - 4.0, lw, 26.0, action);
    ui.back(action);
}

// ---------------------------------------------------------------------
// Create a vault
// ---------------------------------------------------------------------

/// Each step's title, by `vstep`.
pub(crate) const VSTEPS: [&str; vstep::COUNT] = [
    EN.vault_where_will_you,
    EN.vault_unlock_cost,
    EN.vault_space_per_passphrase,
    EN.vault_name_and_passphrases,
    EN.vault_size,
];

pub(crate) fn create(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let panel_w = crate::screens::side_panel_w(cw);
    let Some(c) = app.vaults.create.as_ref() else {
        return;
    };
    let summaries = create_summaries(app);
    let then = app
        .vaults
        .back_to
        .map(|s| fill(EN.vault_then, &[app.back_name(s)]));
    let steps = c.steps();
    let cards: Vec<flow::Card> = steps
        .iter()
        .map(|&k| flow::Card {
            title: VSTEPS[k as usize].to_string(),
            summary: if c.done[k as usize] {
                summaries[k as usize].clone()
            } else {
                EN.vault_not_chosen_yet.to_string()
            },
            mono: false,
            done: c.done[k as usize],
            open: c.open == Some(k),
            // The size has a default: closed at entry, with Change.
            default: k == vstep::PRESET,
            toggle: va(V::CStep(k)),
            guide: Some(create_guide(k)),
        })
        .collect();
    // A small panel has no side panel: its summary and Create vault
    // follow the last step.
    let compact = ui.compact;
    let col = flow::Column {
        area_x: x0,
        area_w: if compact { cw } else { cw - panel_w },
        x: x0 + 40.0,
        w: (cw - panel_w - 72.0).min(820.0),
        h,
        // Made for another flow: back to it, and where it goes next.
        back: Some(app.named_back(match app.vaults.back_to {
            Some(s) => (app.back_link_name(s), va(V::Back)),
            None if app.vaults.from_home => (EN.vault_home, Action::Nav(Screen::Home)),
            None => (EN.vault_vaults, Action::Nav(Screen::Vaults)),
        })),
        heading: EN.vault_create_a_vault,
        guided: app.guided(),
        switch: true,
        note: None,
        chip: then.as_deref(),
        chip_tap: None,
        cancel: app.cancel_offered().is_some(),
    };
    let scroll = c.scroll;
    let (next, again) = {
        let app_ref: &View = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            create_body(app_ref, ui, steps[i], x, y, w)
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    app.scroll(Scrolled::CreateVault, next, again);
    if !compact {
        create_panel(app, ui, x0 + cw - panel_w, panel_w, h, &summaries);
    }
}

fn create_guide(k: u8) -> String {
    match k {
        vstep::WHERE => EN.vault_a_vault_opens_only.to_string(),
        vstep::COST => EN.vault_guide_guess_cost.to_string(),
        vstep::SIZE => EN.vault_every_passphrase.to_string(),
        vstep::PRESET => EN.vault_a_vault_opens.to_string(),
        _ => EN.vault_each_passphrase.to_string(),
    }
}

/// The summary line of each step, by `vstep`, and of the panel's
/// choices.
fn create_summaries(app: &View) -> [String; vstep::COUNT] {
    let Some(c) = app.vaults.create.as_ref() else {
        return Default::default();
    };
    let mut places = Vec::new();
    if !c.not_here {
        places.push(EN.vault_this_computer.to_string());
    }
    for (k, m) in MACHINES.iter().enumerate() {
        if c.on[k] {
            places.push(m.1.to_string());
        }
    }
    let (k, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
    let name = if k == CUSTOM {
        EN.vault_custom
    } else {
        PRESETS[k].0
    };
    let n = c.phrases.len();
    let size = match app.vaults.size_row() {
        Some(r) => fill(
            EN.vault_cost_row,
            &[
                (SIZES[r].0),
                &mib_text(mem).to_string(),
                &passes.to_string(),
            ],
        ),
        None => fill(
            EN.vault_cost_row,
            &[name, &mib_text(mem).to_string(), &passes.to_string()],
        ),
    };
    let ok = (0..n).all(|i| vault::Vaults::phrase_ok(c, i));
    if places.is_empty() {
        places.push(EN.vault_none_chosen.to_string());
    }
    [
        places.join(", "),
        fill(
            EN.vault_cost_row,
            &[name, &mib_text(mem).to_string(), &passes.to_string()],
        ),
        fill(
            EN.vault_file_n,
            &[
                &size_text(c.slot).to_string(),
                &file_text(file_len(c.slot)).to_string(),
            ],
        ),
        fill(
            EN.vault_ofv,
            &[
                &vault::vault_stem(&c.name.text).to_string(),
                &n.to_string(),
                (if n == 1 {
                    EN.vault_passphrase_inline
                } else {
                    EN.vault_passphrases
                }),
                (if ok {
                    EN.vault_match
                } else {
                    EN.vault_not_finished
                }),
            ],
        ),
        size,
    ]
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn create_body(app: &View, ui: &mut Ui, n: u8, x: f32, y: f32, w: f32) -> f32 {
    let Some(c) = app.vaults.create.as_ref() else {
        return 0.0;
    };
    let mut cy = y;
    match n {
        vstep::WHERE => {
            // This computer: the most unlock memory it gives a vault, and
            // what it has free.
            let here = match app.vaults.memory_free_mib {
                Some(free) => fill(
                    EN.vault_free,
                    &[
                        &mib_text(app.vaults.here_limit()).to_string(),
                        &mib_text(free).to_string(),
                    ],
                ),
                None => EN.vault_memory_not_reported.to_string(),
            };
            ui.checkbox(x, cy + 12.0, !c.not_here, true);
            ui.text_mid(x + 32.0, cy, 40.0, 14.0, W::S, TEXT, EN.vault_this_computer);
            ui.text_right(x + w, cy, 40.0, 13.0, W::R, MUTED, &here);
            ui.hit(x, cy, w, 40.0, va(V::CHere));
            ui.rule(x, cy + 42.0, w, INNER);
            cy += 46.0;
            for (k, m) in MACHINES.iter().enumerate() {
                ui.checkbox(x, cy + 12.0, c.on[k], true);
                ui.text_mid(x + 32.0, cy, 40.0, 14.0, W::S, TEXT, m.0);
                ui.text_right(x + w, cy, 40.0, 13.0, W::R, MUTED, m.2);
                ui.hit(x, cy, w, 40.0, va(V::CMachine(k)));
                ui.rule(x, cy + 42.0, w, INNER);
                cy += 46.0;
            }
            cy += 10.0;
            if crate::screens::next_button(ui, x, cy, w, EN.vault_continue, va(V::CNext(n))) {
                cy += 50.0;
            }
        }
        vstep::COST => {
            let (weakest, limit) = app.vaults.weakest();
            let suggested = app.vaults.suggested();
            let (current, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
            let choices = PRESETS.iter().copied().chain(std::iter::once((
                EN.vault_custom,
                c.memory,
                c.passes,
            )));
            for (k, (label, m, p)) in choices.enumerate() {
                let on = k == current;
                let fits = m <= limit;
                let (note, color) = if k == CUSTOM {
                    (EN.vault_choose_memory.to_string(), DIM)
                } else if k == suggested {
                    (EN.vault_suggested.to_string(), ACCENT)
                } else if fits {
                    (EN.vault_fits.to_string(), DIM)
                } else {
                    (fill(EN.vault_too_large, &[&weakest.to_string()]), WARN)
                };
                ui.fill(
                    x,
                    cy,
                    w,
                    50.0,
                    8.0,
                    if on { ACCENT.with_alpha(22) } else { BG },
                );
                ui.stroke(x, cy, w, 50.0, 8.0, if on { ACCENT } else { INNER });
                // The name and what the row says of it, then the cost and
                // the bits it adds.
                ui.text(x + 16.0, cy + 8.0, 14.0, W::S, TEXT, label);
                let room = w - 44.0 - ui.measure(14.0, W::S, label);
                let note = ui.fit(12.0, W::R, &note, room);
                ui.text_right(x + w - 16.0, cy + 5.0, 24.0, 12.0, W::R, color, &note);
                let detail = fill(
                    EN.vault_passes_bits,
                    &[
                        &mib_text(m).to_string(),
                        &p.to_string(),
                        &format!("{:.1}", vault::cost_bits(m, p)),
                    ],
                );
                let detail = ui.fit(12.0, W::M, &detail, w - 32.0);
                ui.text(x + 16.0, cy + 29.0, 12.0, W::M, MUTED, &detail);
                ui.hit(x, cy, w, 50.0, va(V::CPreset(k)));
                cy += 56.0;
            }
            if current == CUSTOM {
                cy += 4.0;
                ui.text(x, cy, 12.0, W::R, MUTED, EN.vault_memory);
                cy += 22.0;
                let mut bx = x;
                for (i, m) in CUSTOM_MEMORY.iter().enumerate() {
                    let style = if *m == c.memory {
                        Style::Primary
                    } else {
                        Style::Secondary
                    };
                    bx += ui.button(bx, cy, None, 32.0, &mib_text(*m), style, va(V::CMemory(i)))
                        + 6.0;
                }
                cy += 44.0;
                ui.text(x, cy, 12.0, W::R, MUTED, EN.vault_passes_label);
                cy += 22.0;
                let mut bx = x;
                for (i, p) in CUSTOM_PASSES.iter().enumerate() {
                    let style = if *p == c.passes {
                        Style::Primary
                    } else {
                        Style::Secondary
                    };
                    bx += ui.button(
                        bx,
                        cy,
                        Some(44.0),
                        32.0,
                        &p.to_string(),
                        style,
                        va(V::CPasses(i)),
                    ) + 6.0;
                }
                cy += 46.0;
            }
            // The unlock memory against the most the weakest machine
            // gives a vault.
            let needs = fill(EN.vault_unlock_memory_n, &[&mib_text(mem).to_string()]);
            ui.text(x, cy, 13.0, W::S, TEXT, &needs);
            let room = w - ui.measure(13.0, W::S, &needs) - 12.0;
            let has = ui.fit(
                12.0,
                W::R,
                &fill(
                    EN.vault_memory_limit,
                    &[&weakest.to_string(), &mib_text(limit).to_string()],
                ),
                room,
            );
            ui.text_right(x + w, cy - 4.0, 24.0, 12.0, W::R, MUTED, &has);
            cy += 24.0;
            ui.fill(x, cy, w, 8.0, 4.0, INNER);
            let share = (mem as f32 / limit.max(1) as f32).min(1.0);
            ui.fill(
                x,
                cy,
                w * share,
                8.0,
                4.0,
                if mem <= limit { ACCENT } else { WARN },
            );
            cy += 20.0;
            ui.text(
                x,
                cy,
                12.0,
                W::R,
                MUTED,
                &fill(
                    EN.vault_unlocks_in_here,
                    &[&app.vaults.time_text(mem, passes).to_string()],
                ),
            );
            cy += 22.0;
            ui.text(
                x,
                cy,
                12.0,
                W::R,
                MUTED,
                &fill(
                    EN.vault_cost_bits,
                    &[&format!("{:.1}", vault::cost_bits(mem, passes))],
                ),
            );
            cy += 30.0;
            if crate::screens::next_button(ui, x, cy, w, EN.vault_continue, va(V::CNext(n))) {
                cy += 50.0;
            }
        }
        vstep::PRESET => {
            // Two sizes, each with its unlock time here and the memory it
            // needs; Customise opens the three cards they stand for.
            let chosen = app.vaults.size_row();
            let free = app.vaults.memory_free_mib;
            let slot = app
                .vaults
                .create
                .as_ref()
                .map_or(faraday_vault::DEFAULT_SLOT, |c| c.slot);
            let row_h = if ui.compact { 70.0 } else { 56.0 };
            for (r, (label, preset, _)) in SIZES.iter().enumerate() {
                let (_, mem, passes) = PRESETS[*preset];
                let on = chosen == Some(r);
                ui.fill(
                    x,
                    cy,
                    w,
                    row_h,
                    8.0,
                    if on { ACCENT.with_alpha(26) } else { BG },
                );
                ui.stroke(
                    x,
                    cy,
                    w,
                    row_h,
                    8.0,
                    if on { ACCENT.with_alpha(110) } else { INNER },
                );
                let cost = fill(
                    EN.vault_passes,
                    &[&mib_text(mem).to_string(), &passes.to_string()],
                );
                let time = match app.vaults.seconds(mem, passes) {
                    Some(_) => fill(
                        EN.vault_unlocks_in_here,
                        &[&app.vaults.time_text(mem, passes).to_string()],
                    ),
                    None => EN.vault_unlock_time_unmeasured.to_string(),
                };
                // Short of memory here: what this computer has free, in
                // place of the time.
                let short = free.filter(|f| vault::vault_need_mib(mem, slot) > *f);
                if ui.compact {
                    // Three lines: the name, the cost and its time here,
                    // the memory it needs.
                    let name = ui.fit(14.0, W::S, label, w - 28.0);
                    ui.text(x + 14.0, cy + 6.0, 14.0, W::S, TEXT, &name);
                    let here = match app.vaults.seconds(mem, passes) {
                        Some(_) => fill(
                            EN.vault_here,
                            &[
                                &cost.to_string(),
                                &app.vaults.time_text(mem, passes).to_string(),
                            ],
                        ),
                        None => cost.clone(),
                    };
                    let here = ui.fit(12.0, W::R, &here, w - 28.0);
                    ui.text(x + 14.0, cy + 26.0, 12.0, W::R, MUTED, &here);
                    if let Some(f) = short {
                        let has = fill(EN.vault_this_computer_has, &[&mib_text(f).to_string()]);
                        let has = ui.fit(12.0, W::R, &has, w - 28.0);
                        ui.text(x + 14.0, cy + 46.0, 12.0, W::R, WARN, &has);
                    }
                } else {
                    ui.text(x + 14.0, cy + 8.0, 14.0, W::S, TEXT, label);
                    ui.text_right(x + w - 14.0, cy + 5.0, 24.0, 13.0, W::S, TEXT, &cost);
                    let line = match short {
                        Some(f) => fill(EN.vault_this_computer_has, &[&mib_text(f).to_string()]),
                        None => time,
                    };
                    let line = ui.fit(12.0, W::R, &line, w - 28.0);
                    ui.text(
                        x + 14.0,
                        cy + 31.0,
                        12.0,
                        W::R,
                        if short.is_some() { WARN } else { MUTED },
                        &line,
                    );
                }
                ui.hit(x, cy, w, row_h, va(V::CSizeRow(r)));
                cy += row_h + 6.0;
            }
            ui.button(
                x,
                cy,
                None,
                34.0,
                EN.vault_customise,
                Style::Ghost,
                va(V::CCustomise),
            );
            cy += 50.0;
            if crate::screens::next_button(ui, x, cy, w, EN.vault_continue, va(V::CNext(n))) {
                cy += 50.0;
            }
        }
        vstep::SIZE => {
            let bw = (w - 18.0) / 4.0;
            for (k, s) in SLOT_SIZES.iter().enumerate() {
                let on = *s == c.slot;
                let bx = x + k as f32 * (bw + 6.0);
                ui.fill(
                    bx,
                    cy,
                    bw,
                    62.0,
                    8.0,
                    if on { ACCENT.with_alpha(22) } else { BG },
                );
                ui.stroke(bx, cy, bw, 62.0, 8.0, if on { ACCENT } else { INNER });
                let t = size_text(*s);
                let tw = ui.measure(15.0, W::S, &t);
                ui.text(bx + (bw - tw) / 2.0, cy + 12.0, 15.0, W::S, TEXT, &t);
                let sub = fill(
                    EN.vault_file_n_inline,
                    &[&file_text(file_len(*s)).to_string()],
                );
                let sw = ui.measure(12.0, W::R, &sub);
                ui.text(bx + (bw - sw) / 2.0, cy + 36.0, 12.0, W::R, MUTED, &sub);
                ui.hit(bx, cy, bw, 62.0, va(V::CSize(k)));
            }
            cy += 80.0;
            if crate::screens::next_button(ui, x, cy, w, EN.vault_continue, va(V::CNext(n))) {
                cy += 50.0;
            }
        }
        _ => {
            let half = (w - 12.0) / 2.0;
            let may = app.may_load_keys();
            // Its name, which names the file: what it holds, since the
            // file is all anyone who sees the stick sees (`docs/VAULT.md`
            // §6).
            ui.text_mid(x, cy, 36.0, 13.0, W::S, TEXT, EN.vault_name);
            cy += 34.0;
            cy += ui.wrap(
                x,
                cy,
                w,
                12.0,
                W::R,
                MUTED,
                &fill(EN.vault_what_it_holds, &[app.medium.noun()]),
            ) + 10.0;
            let nw = w.min(360.0);
            text_box(
                ui,
                x,
                cy,
                nw,
                &c.name,
                false,
                app.vaults.focus == Some(Focus::Name),
                va(V::CName),
            );
            let file = fill(
                EN.vault_file_ofv,
                &[&vault::vault_stem(&c.name.text).to_string()],
            );
            if nw + 16.0 + ui.measure(13.0, W::R, &file) <= w {
                ui.text_mid(x + nw + 16.0, cy, 40.0, 13.0, W::R, MUTED, &file);
                cy += 56.0;
            } else {
                ui.text(x, cy + 48.0, 13.0, W::R, MUTED, &file);
                cy += 78.0;
            }
            if !may {
                let pull = fill(EN.vault_remove_the_then, &[app.medium.noun()]);
                cy += stick_banner(ui, x, cy, w, &pull);
            }
            ui.button(
                x,
                cy,
                None,
                32.0,
                if c.shown {
                    EN.vault_hide_passphrases
                } else {
                    EN.vault_show_passphrases
                },
                Style::Ghost,
                va(V::CShow),
            );
            cy += 42.0;
            for (i, (a, b)) in c.phrases.iter().enumerate() {
                let rolling = app.vaults.dice.as_ref().is_some_and(|d| d.0 == i);
                ui.text_mid(
                    x,
                    cy,
                    36.0,
                    13.0,
                    W::S,
                    TEXT,
                    &fill(EN.vault_passphrase_n, &[&(i + 1).to_string()]),
                );
                if i > 0 {
                    ui.button(
                        x + w - (ui.measure(13.0, W::S, EN.vault_remove) + 28.0),
                        cy,
                        None,
                        36.0,
                        EN.vault_remove,
                        Style::Secondary,
                        va(V::CRemovePhrase(i)),
                    );
                }
                cy += 44.0;
                // Two ways to make it, side by side: dice, or typed.
                let dice_style = if !may {
                    Style::Disabled
                } else if rolling {
                    Style::Secondary
                } else if a.text.is_empty() {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                let dw = ui.button(
                    x,
                    cy,
                    None,
                    40.0,
                    EN.vault_generate_dice,
                    dice_style,
                    va(V::Dice(i)),
                );
                // Beside the button; on a small panel under it.
                if ui.compact {
                    cy += 46.0;
                    ui.text(x, cy, 13.0, W::R, MUTED, EN.vault_or_type_your);
                    cy += 26.0;
                } else {
                    ui.text_mid(
                        x + dw + 14.0,
                        cy,
                        40.0,
                        13.0,
                        W::R,
                        MUTED,
                        EN.vault_or_type_your,
                    );
                    cy += 52.0;
                }
                if may {
                    let fa = app.vaults.focus == Some(Focus::Phrase(i, false));
                    let fb = app.vaults.focus == Some(Focus::Phrase(i, true));
                    crate::demo::pass_box(
                        app,
                        ui,
                        (x, cy, half),
                        a,
                        (c.shown, fa),
                        (va(V::CFocus(i, false)), va(V::CShow)),
                        crate::demo::PassField::Phrase(i as u8, false),
                    );
                    crate::demo::pass_box(
                        app,
                        ui,
                        (x + half + 12.0, cy, half),
                        b,
                        (c.shown, fb),
                        (va(V::CFocus(i, true)), va(V::CShow)),
                        crate::demo::PassField::Phrase(i as u8, true),
                    );
                } else {
                    stick_field(app.medium, ui, x, cy, half);
                    stick_field(app.medium, ui, x + half + 12.0, cy, half);
                }
                cy += 46.0 + never_real_seed_if(may, app, ui, x, cy, w);
                let (status, color) = if !b.text.is_empty() && *a.text == *b.text {
                    (EN.vault_matches, OK)
                } else if !b.text.is_empty() {
                    (EN.vault_does_not_match, WARN)
                } else {
                    (EN.vault_type_it_then, DIM)
                };
                ui.text(x, cy, 12.0, W::R, color, status);
                cy += 22.0;
                // Its strength: the dice's bits and the cost's, when the
                // dice made it.
                let (_, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
                let cost = vault::cost_bits(mem, passes);
                let strength = match vault::Vaults::phrase_bits(c, i) {
                    Some(own) => Some((
                        fill(
                            EN.vault_bits_from_dice,
                            &[
                                &format!("{own:.1}"),
                                &format!("{cost:.1}"),
                                &format!("{:.1}", own + cost),
                            ],
                        ),
                        if own >= vault::STRONG_BITS as f32 {
                            OK
                        } else {
                            MUTED
                        },
                    )),
                    None if !a.text.is_empty() && !app.pass_is_public(&a.text) => {
                        Some((fill(EN.vault_typed_its_own, &[&format!("{cost:.1}")]), DIM))
                    }
                    None => None,
                };
                if let Some((line, tone)) = strength {
                    cy += ui.wrap(x, cy, w, 12.0, W::R, tone, &line);
                }
                // The demo's public passphrase stands in for the rolled
                // words, as New key's public seed does (`docs/DEMO.md` §5.2).
                if app.dice_made_public(c, i) {
                    cy += 4.0;
                    cy += ui.wrap(x, cy, w, 13.0, W::S, WARN, EN.vault_demo_uses_public);
                }
                cy += 12.0;
                if let Some((di, rolls, list)) = app.vaults.dice.as_ref()
                    && *di == i
                {
                    cy += dice_panel(app, ui, x, cy, w, rolls, *list);
                }
            }
            if c.phrases.len() < faraday_vault::SLOTS {
                ui.button(
                    x,
                    cy,
                    None,
                    36.0,
                    EN.vault_add_another_passphrase,
                    Style::Secondary,
                    va(V::CAddPhrase),
                );
                cy += 46.0;
            }
            // A small panel has no side panel: what will be made, and
            // Create vault, come after the last step's controls.
            if ui.compact {
                cy += 10.0;
                ui.rule(x, cy, w, LINE);
                cy += 16.0;
                cy += create_summary(app, ui, x, cy, w) + 8.0;
                cy += create_button(app, ui, x, cy, w) + 8.0;
            }
        }
    }
    cy - y
}

/// Rolling dice for a passphrase: the rolls typed, the words they make,
/// and the strength so far. Returns its height.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn dice_panel(
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    rolls: &TextBox,
    list: faraday_bip::diceware::List,
) -> f32 {
    use faraday_bip::diceware::List;
    let mut cy = y;
    ui.fill(x, cy, w, 4.0, 2.0, ACCENT.with_alpha(60));
    cy += 14.0;
    ui.text(x, cy, 13.0, W::S, TEXT, EN.dice_list_title);
    cy += 22.0;
    let lists = [
        (List::Large, EN.dice_list_large, EN.dice_list_large_detail),
        (
            List::Short1,
            EN.dice_list_short1,
            EN.dice_list_short1_detail,
        ),
        (
            List::Short2,
            EN.dice_list_short2,
            EN.dice_list_short2_detail,
        ),
    ];
    // Side by side; on a small panel one under another.
    let compact = ui.compact;
    let lw = if compact { w } else { (w - 2.0 * 10.0) / 3.0 };
    for (k, (l, name, detail)) in lists.iter().enumerate() {
        let on = *l == list;
        let (bx, cy) = if compact {
            (x, cy + k as f32 * 52.0)
        } else {
            (x + k as f32 * (lw + 10.0), cy)
        };
        let detail = ui.fit(11.0, W::R, detail, lw - 20.0);
        ui.fill(
            bx,
            cy,
            lw,
            44.0,
            8.0,
            if on { ACCENT.with_alpha(22) } else { BG },
        );
        ui.stroke(bx, cy, lw, 44.0, 8.0, if on { ACCENT } else { INNER });
        ui.text(bx + 10.0, cy + 6.0, 13.0, W::S, TEXT, name);
        ui.text(bx + 10.0, cy + 24.0, 11.0, W::R, MUTED, &detail);
        ui.hit(bx, cy, lw, 44.0, Action::Vault(V::DiceList(k as u8)));
    }
    cy += if compact { 3.0 * 52.0 + 4.0 } else { 56.0 };
    cy += ui.wrap(
        x,
        cy,
        w,
        13.0,
        W::S,
        TEXT,
        &fill(EN.vault_dice_per_word, &[&list.dice_per_word().to_string()]),
    ) + 8.0;
    let focused = app.vaults.focus == Some(Focus::Dice);
    text_box(
        ui,
        x,
        cy,
        w,
        rolls,
        false,
        focused,
        Action::Vault(V::Dice(app.vaults.dice.as_ref().map_or(0, |d| d.0))),
    );
    cy += 50.0 + never_under(app, ui, x, cy, w);
    let words = vault::dice_words(&rolls.text, list);
    let per = list.dice_per_word();
    let left = rolls.text.len() % per;
    // The words as the rolls make them, each a pill that opens it in its
    // list, as New key shows its words.
    if words.is_empty() {
        ui.text(x, cy, 13.0, W::R, DIM, EN.vault_no_word_yet);
        cy += 26.0;
    } else {
        ui.text(x, cy, 12.0, W::R, MUTED, crate::wordlist::PRESS_A_WORD);
        cy += 22.0;
        let place = crate::wordlist::WordList::Eff(list).place();
        let mut bx = x;
        for (word, i) in words
            .iter()
            .zip(vault::dice_word_indices(&rolls.text, list))
        {
            let bw = ui.measure(14.0, W::M, word) + 24.0;
            if bx > x && bx + bw > x + w {
                bx = x;
                cy += 32.0;
            }
            bx += ui.word_pill(
                bx,
                cy,
                word,
                Action::WordList(crate::wordlist::WordListAction::Open(place, Some(i))),
            ) + 6.0;
        }
        cy += 38.0;
    }
    let bits = list.bits(words.len());
    let (_, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
    let own = words.len() as f32 * list.bits_per_word();
    let status = fill(
        EN.vault_dice_strength,
        &[
            &words.len().to_string(),
            (if words.len() == 1 {
                EN.vault_word
            } else {
                EN.vault_words_inline
            }),
            &format!("{own:.1}"),
            &format!("{:.1}", own + vault::cost_bits(mem, passes)),
            &(if left > 0 {
                format!(
                    " {}",
                    fill(
                        EN.vault_dice_for_the_next_word,
                        &[&left.to_string(), &per.to_string()]
                    )
                )
            } else {
                String::new()
            })
            .to_string(),
        ],
    );
    cy += ui.wrap(
        x,
        cy,
        w,
        12.0,
        W::R,
        if bits >= vault::STRONG_BITS {
            OK
        } else {
            MUTED
        },
        &status,
    ) + 8.0;
    // How long is long enough, and where the lists come from.
    cy += ui.wrap(x, cy, w, 12.0, W::R, MUTED, &vault::dice_aim(list)) + 12.0;
    let style = if words.is_empty() {
        Style::Disabled
    } else {
        Style::Primary
    };
    let uw = ui.button(
        x,
        cy,
        None,
        36.0,
        EN.vault_use_these_words,
        style,
        va(V::DiceUse),
    );
    ui.button(
        x + uw + 8.0,
        cy,
        None,
        36.0,
        EN.vault_cancel,
        Style::Ghost,
        va(V::DiceClose),
    );
    cy += 50.0;
    cy - y
}

fn create_panel(
    app: &View,
    ui: &mut Ui,
    px: f32,
    pw: f32,
    h: f32,
    summaries: &[String; vstep::COUNT],
) {
    let Some(c) = app.vaults.create.as_ref() else {
        return;
    };
    ui.fill(px, 0.0, pw, h, 0.0, SURFACE);
    ui.fill(px, 0.0, 1.0, h, 0.0, LINE);
    let x = px + 26.0;
    let w = pw - 52.0;
    let mut y = tokens::SIDE_PANEL_TOP;
    ui.text(x, y, 13.0, W::S, MUTED, EN.vault_your_choices);
    y += 28.0;
    for (k, label) in [
        EN.vault_opens_on_label,
        EN.vault_unlock_cost,
        EN.vault_space_per_passphrase,
        EN.vault_name_and_passphrases,
    ]
    .iter()
    .enumerate()
    {
        // Until Customise, the size stands for the first three: they
        // carry its values, and open its card.
        let by_size = !c.customise && k as u8 != vstep::PHRASES;
        let live = by_size || c.done[k] || c.open == Some(k as u8);
        ui.text(x, y, 12.0, W::R, MUTED, label);
        let v = if live {
            summaries[k].clone()
        } else {
            EN.vault_not_chosen_yet.to_string()
        };
        let v = ui.fit(13.0, W::R, &v, w);
        ui.text(x, y + 18.0, 13.0, W::R, if live { TEXT } else { DIM }, &v);
        let step = if by_size { vstep::PRESET } else { k as u8 };
        ui.hit(x - 8.0, y - 4.0, w + 16.0, 42.0, va(V::CStep(step)));
        y += 46.0;
    }
    y += create_summary(app, ui, x, y, w);
    let by = (h - 26.0 - 46.0).max(y + 12.0);
    create_button(app, ui, x, by, w);
}

/// What Create vault will make: warnings about its cost, the file, its
/// size, memory and unlock time, and the passphrases' strength. Returns
/// its height.
fn create_summary(app: &View, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let Some(c) = app.vaults.create.as_ref() else {
        return 0.0;
    };
    let mut y = y0;
    let (k, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
    let (weakest, limit) = app.vaults.weakest();
    let name = if k == CUSTOM {
        EN.vault_custom
    } else {
        PRESETS[k].0
    };
    let mut warnings: Vec<(String, faraday_draw::Color)> = Vec::new();
    let slot = app
        .vaults
        .create
        .as_ref()
        .map_or(faraday_vault::DEFAULT_SLOT, |c| c.slot);
    if let Some(free) = app.vaults.memory_free_mib
        && vault::vault_need_mib(mem, slot) > free
    {
        warnings.push((
            fill(EN.vault_this_computer_n, &[&mib_text(mem).to_string()]),
            ERR,
        ));
    }
    if mem > limit {
        warnings.push((
            fill(
                EN.vault_memory_mismatch,
                &[
                    name,
                    &mib_text(mem).to_string(),
                    &weakest.to_string(),
                    &mib_text(limit).to_string(),
                ],
            ),
            WARN,
        ));
    }
    if app.vaults.seconds(mem, passes).is_some_and(|s| s > 10.0) {
        warnings.push((
            fill(
                EN.vault_unlocking_takes,
                &[&app.vaults.time_text(mem, passes).to_string()],
            ),
            WARN,
        ));
    }
    if c.on[1..].iter().any(|on| *on) {
        warnings.push((EN.vault_raspberry_pi.to_string(), MUTED));
    }
    y += 4.0;
    for (t, color) in &warnings {
        let th = ui.wrap(x + 12.0, y + 10.0, w - 24.0, 12.0, W::R, *color, t);
        ui.fill(x, y, w, th + 20.0, 8.0, color.with_alpha(26));
        ui.wrap(x + 12.0, y + 10.0, w - 24.0, 12.0, W::R, *color, t);
        y += th + 28.0;
    }
    y += 8.0;
    ui.text(x, y, 13.0, W::S, MUTED, EN.vault_will_create);
    y += 26.0;
    let cost = vault::cost_bits(mem, passes);
    let strength = c.weakest_text(cost, app.demo_passphrase());
    let facts = [
        (
            EN.vault_file,
            format!("{}.ofv", vault::vault_stem(&c.name.text)),
        ),
        (EN.vault_size, file_text(file_len(c.slot))),
        (EN.vault_unlock_memory, mib_text(mem)),
        (EN.vault_unlock_here, app.vaults.time_text(mem, passes)),
        (
            EN.vault_unlock_cost_adds,
            fill(EN.vault_bits, &[&format!("{cost:.1}")]),
        ),
        (
            if c.phrases.len() > 1 {
                EN.vault_weakest_passphrase
            } else {
                EN.vault_strength
            },
            strength,
        ),
        (EN.vault_goes_to_label, app.medium.for_box().to_string()),
    ];
    for (k, v) in facts.iter().filter(|(_, v)| !v.is_empty()) {
        ui.text_mid(x, y, 30.0, 12.0, W::R, MUTED, k);
        ui.text_right(x + w, y, 30.0, 13.0, W::R, TEXT, v);
        y += 30.0;
    }
    if let Some(e) = &c.error {
        y += 6.0;
        y += ui.wrap(x, y, w, 13.0, W::R, ERR, e) + 6.0;
    }
    y - y0
}

/// Create vault, or how long creating takes while it works. Returns its
/// height.
fn create_button(app: &View, ui: &mut Ui, x: f32, by: f32, w: f32) -> f32 {
    let (_, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
    if app.vaults.working == Some(vault::Work::Create) {
        ui.text_mid(
            x,
            by,
            46.0,
            15.0,
            W::S,
            ACCENT,
            &fill(
                EN.vault_creating,
                &[&app.vaults.time_text(mem, passes).to_string()],
            ),
        );
    } else if !crate::screens::pin_button(
        ui,
        x,
        by,
        Some(w),
        46.0,
        EN.vault_create_vault,
        if app.may_load_keys() {
            Style::Primary
        } else {
            Style::Disabled
        },
        va(V::CGo),
    ) {
        // Pinned at the panel's foot.
        return 0.0;
    }
    46.0
}

// ---------------------------------------------------------------------
// Vault contents
// ---------------------------------------------------------------------

/// An item's title, its line, and whether the title is data.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn item_heading(app: &View, r: RecordView<'_>) -> (String, String, bool) {
    match r.kind() {
        kind::KEY => {
            let fp = app
                .vault_key_fingerprint(r)
                .unwrap_or_else(|| EN.vault_key.to_string());
            let words = app
                .record_word_count(r)
                .map(|n| fill(EN.vault_words_n, &[&n.to_string()]))
                .unwrap_or_else(|| EN.vault_master_seed.into());
            let loads = r.loads_at_unlock();
            let label = r.public_text(field::KEY_LABEL).unwrap_or("");
            let mut line = vec![words];
            if !label.is_empty() {
                line.push(label.to_string());
            }
            if !loads {
                line.push(EN.vault_not_loaded_at_unlock_inline.to_string());
            }
            (fp, line.join(" · "), true)
        }
        kind::WALLET => {
            let name = r
                .public_text(field::WALLET_NAME)
                .unwrap_or(EN.vault_wallet)
                .to_string();
            let mut line = app
                .record_described(r)
                .map_or_else(|| EN.vault_wallet.into(), |d| d.shape.clone());
            if !r.loads_at_unlock() {
                line = format!("{line} · {}", EN.vault_not_loaded_at_unlock_inline);
            }
            (name, line, false)
        }
        kind::ENTRY => {
            let mut parts = Vec::new();
            if r.has(field::PASSWORD) {
                parts.push(EN.vault_password);
            }
            if r.has(field::TOTP) {
                parts.push(EN.vault_totp);
            }
            if parts.is_empty() {
                parts.push(EN.vault_entry);
            }
            (
                r.public_text(field::TITLE)
                    .unwrap_or(EN.vault_entry)
                    .to_string(),
                parts.join(" · "),
                false,
            )
        }
        kind::NOTE => {
            // The first line of a note is its title: a secret put on screen.
            let note = app.reveal_record(r, field::NOTE, Reveal::VaultEntry(Part::Note));
            let t: &str = &note;
            let first: String = t.lines().next().unwrap_or("").chars().take(40).collect();
            let first = if first.is_empty() {
                EN.vault_note.to_string()
            } else {
                first
            };
            (
                first,
                fill(EN.vault_note_characters, &[&t.chars().count().to_string()]),
                false,
            )
        }
        kind::SHEET => {
            let (_, name, _) = faraday_keys::vault::sheet_parts(r);
            (
                if name.is_empty() {
                    EN.vault_recovery_sheet.into()
                } else {
                    name.clone()
                },
                fill(EN.vault_recovery_sheet_n, &[&name.to_string()]),
                false,
            )
        }
        kind::GPG => {
            let line = match crate::gpg::shown(r, app.tz_min()) {
                Some(g) if g.expires == EN.gpg_never => EN.vault_ed25519_does.to_string(),
                Some(g) => fill(EN.vault_ed25519_expires, &[&g.expires.to_string()]),
                None => "Ed25519".to_string(),
            };
            (
                r.public_text(faraday_keys::vault::gpg_field::USER_ID)
                    .unwrap_or(EN.vault_gpg_key)
                    .to_string(),
                line,
                false,
            )
        }
        kind::SECURE_BOOT => (
            EN.vault_owner_keys.to_string(),
            EN.vault_pk_kek_db.to_string(),
            false,
        ),
        kind::WALLET_SHARE => {
            let n = |f: u8| {
                r.public(f)
                    .and_then(|b| b.first())
                    .map_or(String::new(), |b| b.to_string())
            };
            (
                fill(
                    EN.backup_flow_vault_share,
                    &[
                        r.public_text(field::SHARE_WALLET_NAME)
                            .unwrap_or(EN.vault_wallet),
                        &n(field::SHARE_NUMBER),
                        &n(field::SHARE_OF),
                    ],
                ),
                String::new(),
                false,
            )
        }
        // A key's passphrase kept without its words: named by the key.
        kind::PASSPHRASE => {
            let fp = r
                .public(field::PASSPHRASE_KEY)
                .and_then(|b| <[u8; 4]>::try_from(b).ok())
                .map(|b| fp_text(faraday_bip::keys::Fingerprint(b)))
                .unwrap_or_default();
            (fill(EN.plan_passphrase_label, &[&fp]), String::new(), true)
        }
        kind::APK_KEY => match crate::apk::shown(r, app.tz_min()) {
            Some(s) => (s.name, s.package, false),
            None => (EN.vault_record.to_string(), String::new(), false),
        },
        _ => (EN.vault_record.to_string(), String::new(), false),
    }
}

/// The shape of a wallet read already, as the session names one. Costs
/// the descriptor's checksum.
pub(crate) fn policy_shape(policy: faraday_bip::policy::WalletPolicy) -> String {
    Session::shape(&Wallet::new(
        String::new(),
        policy,
        String::new(),
        crate::wallet::Origin::Named,
    ))
}

/// Costs what `key_identity` costs. A key record's fingerprint, worked
/// out once per key and kept. Four bytes: it finds a candidate; whether
/// the record is a given key is [`record_is_key`]'s to say.
pub(crate) fn key_fingerprint(c: &Compute, app: &Faraday, r: RecordView<'_>) -> Option<String> {
    key_identity(c, app, r).map(|(fp, _)| fp)
}

/// Costs what `key_identity` costs. Whether key record `r` holds
/// session key `key`: the same master key, its passphrase applied, told
/// by all 32 bytes of [`crate::wallet::master_id`] (F65), the
/// fingerprint finding the candidate. Kept once worked out, as the
/// fingerprint is.
pub(crate) fn record_is_key(
    c: &Compute,
    app: &Faraday,
    r: RecordView<'_>,
    key: &faraday_keys::KeyInfo,
) -> bool {
    key_identity(c, app, r).is_some_and(|(fp, id)| fp == fp_text(key.fingerprint) && id == key.id.0)
}

/// Costs a PBKDF2 and two elliptic-curve multiplications the first
/// time. A key record's fingerprint and [`crate::wallet::master_id`],
/// worked out once per key and kept.
pub(crate) fn key_identity(
    c: &Compute,
    app: &Faraday,
    r: RecordView<'_>,
) -> Option<(String, [u8; 32])> {
    app.vaults.key_identity(c, app.session.network(), r)
}

/// Costs a parse of the words, and a PBKDF2 and two elliptic-curve
/// multiplications the first time. [`key_identity`] of a vault key record
/// of session key `k`'s words alone, without its passphrase, kept with the
/// records' by the same id.
pub(crate) fn words_identity(c: &Compute, app: &Faraday, k: usize) -> Option<(String, [u8; 32])> {
    let id = app.session.keys.words_record_id(c, k)?;
    app.vaults
        .kept_identity(id, || app.session.keys.words_identity(c, k))
}

/// Costs a PBKDF2 and a key derivation the first time for a record
/// whose fingerprint matches. Whether key record `r` is the key a
/// wallet's descriptor names as `key`: its master fingerprint the
/// descriptor's, and its account key at the descriptor's path the
/// descriptor's own, public key and chain code both, where the
/// fingerprint alone only finds a candidate (F65). A key with no origin
/// is never confirmed. The account key is worked out once per record
/// and path and kept.
pub(crate) fn record_is_policy_key(
    c: &Compute,
    app: &Faraday,
    r: RecordView<'_>,
    key: &faraday_bip::policy::PolicyKey,
) -> bool {
    let (Some(fp), Some(path)) = (key.fingerprint(), key.path()) else {
        return false;
    };
    if key_fingerprint(c, app, r).as_deref() != Some(fp_text(fp).as_str()) {
        return false;
    }
    let Some(id) = faraday_keys::vault::key_record_id(r) else {
        return false;
    };
    let want = faraday_keys::vault::account_bytes(key.xpub());
    let at = path.to_string();
    if let Some((.., kept)) = app
        .vaults
        .accounts
        .borrow()
        .iter()
        .find(|(k, p, _)| *k == id && *p == at)
    {
        return kept.is_some_and(|b| b == want);
    }
    // Kept by the record's id and the path once worked out.
    let got = faraday_keys::vault::key_account(c, app.session.network(), r, path);
    let mut kept = app.vaults.accounts.borrow_mut();
    if kept.len() >= vault::ACCOUNTS_KEPT {
        kept.remove(0);
    }
    kept.push((id, at, got));
    got.is_some_and(|b| b == want)
}

fn words_text(m: &faraday_bip::bip39::Mnemonic) -> zeroize::Zeroizing<String> {
    let lang = m.language();
    // Sized first, so that no copy is left behind by a reallocation.
    let size: usize = m.indices().iter().map(|i| lang.word(*i).len() + 1).sum();
    let mut text = zeroize::Zeroizing::new(String::with_capacity(size));
    for (k, i) in m.indices().iter().enumerate() {
        if k > 0 {
            text.push(' ');
        }
        text.push_str(lang.word(*i));
    }
    text
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn contents(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return contents_compact(app, ui, x0, cw, h);
    }
    let x = x0 + 40.0;
    let w = cw - 80.0;
    let mut y = 28.0;
    let back = (EN.vault_vaults, Action::Nav(Screen::Vaults));
    back_link(app, ui, x, y, back);
    y += faraday_draw::tokens::BACK_LINK_ROW;
    let Some(v) = app.vaults.open.get(app.vaults.current) else {
        title(ui, x, y, EN.vault_vault_contents);
        ui.text(x, y + 56.0, 15.0, W::R, MUTED, EN.vault_no_vault_is_open);
        if let Some((label, a)) = app.vault_way(Screen::VaultContents) {
            ui.button(x, y + 90.0, None, 44.0, &label, Style::Primary, a);
        }
        return;
    };
    title(ui, x, y, &v.label(app.vaults.store()));
    let lw = ui.measure(28.0, W::S, &v.label(app.vaults.store()));
    let state = if v.changes == 0 {
        EN.vault_open_button.to_string()
    } else {
        fill(
            EN.vault_open_unsaved,
            &[
                &v.changes.to_string(),
                (if v.changes == 1 {
                    EN.vault_change
                } else {
                    EN.vault_changes
                }),
            ],
        )
    };
    ui.text_mid(
        x + lw + 16.0,
        y + 2.0,
        32.0,
        13.0,
        W::S,
        if v.changes == 0 { OK } else { WARN },
        &state,
    );
    let lockw = ui.measure(14.0, W::S, EN.vault_lock) + 36.0;
    ui.button(
        x + w - lockw,
        y - 2.0,
        Some(lockw),
        40.0,
        EN.vault_lock,
        Style::Primary,
        Action::LockAsk,
    );
    let rw = ui.measure(13.0, W::S, EN.vault_rename) + 28.0;
    ui.button(
        x + w - lockw - 8.0 - rw,
        y + 2.0,
        Some(rw),
        34.0,
        EN.vault_rename,
        Style::Ghost,
        va(V::Rename),
    );
    y += 40.0;
    let sub = format!("{} · {}", v.name, cost_text(&v.header().cost));
    ui.text(x, y, 12.0, W::M, MUTED, &sub);
    let mut vx = x + ui.measure(12.0, W::M, &sub) + 24.0;
    if app.vaults.open.len() > 1 {
        for (i, o) in app.vaults.open.iter().enumerate() {
            let style = if i == app.vaults.current {
                Style::Primary
            } else {
                Style::Secondary
            };
            vx += ui.button(
                vx,
                y - 8.0,
                None,
                28.0,
                &o.label(app.vaults.store()),
                style,
                va(V::Show(i)),
            ) + 6.0;
        }
    }
    y += 30.0;
    y += guide_text(
        app,
        ui,
        x,
        y,
        w,
        &fill(EN.vault_secret_values, &[app.medium.noun()]),
    );

    // Three columns: kinds, items, the item. A wallet's chart takes the
    // room the other two give up (`docs/history/NEW-WALLET.md` §7.2).
    let chart_shown = app
        .vault_selected_index()
        .and_then(|i| app.vaults.records(v.id).get(i))
        .is_some_and(|r| r.kind() == kind::WALLET);
    let (kinds_w, items_w) = if chart_shown {
        (
            faraday_draw::tokens::CHART_PANE_KINDS,
            faraday_draw::tokens::CHART_PANE_ITEMS,
        )
    } else {
        let kinds_w = 190.0;
        (kinds_w, ((w - kinds_w) * 0.38).clamp(220.0, 340.0))
    };
    let detail_x = x + kinds_w + items_w + 48.0;
    let detail_w = w - kinds_w - items_w - 48.0;
    let top = y;
    let mut ky = top;
    // The kinds it holds, then a row for each kind to add.
    let empty = shows_empty(app, v);
    for (k, label, count) in listed_kinds(app, v) {
        let on = k == app.vaults.category;
        if on {
            ui.fill(x, ky, kinds_w, 38.0, 8.0, ACCENT.with_alpha(30));
        }
        ui.text_mid(
            x + 12.0,
            ky,
            38.0,
            14.0,
            W::S,
            if on { TEXT } else { MUTED },
            label,
        );
        ui.text_right(
            x + kinds_w - 12.0,
            ky,
            38.0,
            13.0,
            W::R,
            MUTED,
            &count.to_string(),
        );
        ui.hit(x, ky, kinds_w, 38.0, va(V::Category(k)));
        ky += 42.0;
    }
    if !empty {
        if ky > top {
            ky += 8.0;
        }
        add_rows(app, ui, x, ky, kinds_w, 38.0);
    }

    let ix = x + kinds_w + 16.0;
    if empty {
        ui.text(ix, top + 6.0, 15.0, W::S, TEXT, NOTHING_YET);
        let mut by = top + 40.0;
        for (label, style, action) in empty_buttons(app) {
            let bw = ui.measure(14.0, W::S, &label) + 36.0;
            ui.button(ix, by, Some(bw), 40.0, &label, style, action);
            by += 50.0;
        }
        return;
    }
    let (cat_label, kinds) = CATEGORIES[app.vaults.category];
    let items: Vec<RecordView<'_>> = app
        .vaults
        .records(v.id)
        .iter()
        .filter(|(_, r)| kinds.contains(&r.kind()))
        .map(|(_, r)| r)
        .collect();
    let add = add_label(app.vaults.category);
    ui.text_mid(ix, top, 34.0, 14.0, W::S, TEXT, cat_label);
    if let Some(a) = add {
        let aw = ui.measure(13.0, W::S, a) + 26.0;
        ui.button(
            ix + items_w - aw,
            top,
            Some(aw),
            34.0,
            a,
            Style::Secondary,
            va(V::Add),
        );
    }
    let mut iy = top + 44.0;
    // A second row: what else can fill this kind.
    let entries_files = app
        .inbox
        .iter()
        .filter(|i| i.kind == crate::FileKind::Entries)
        .count();
    let backups: Vec<usize> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| i.kind == crate::FileKind::Backup)
        .map(|(k, _)| k)
        .collect();
    let mut extra: Vec<(String, V)> = Vec::new();
    match app.vaults.category {
        2 => {
            extra.push((EN.vault_scan_a_code.to_string(), V::ScanEntry));
            if entries_files > 0 {
                extra.push((
                    fill(EN.vault_import_from_files, &[&entries_files.to_string()]),
                    V::ImportAllEntries,
                ));
            }
        }
        0 | 3 => {
            for k in &backups {
                extra.push((
                    fill(EN.vault_import_n, &[&app.inbox[*k].name.to_string()]),
                    V::ImportBackup(*k),
                ));
            }
        }
        _ => {}
    }
    if !extra.is_empty() {
        let mut bx = ix;
        for (label, a) in &extra {
            let label = ui.fit(12.0, W::S, label, items_w - 30.0);
            let bw = ui.measure(12.0, W::S, &label) + 24.0;
            if bx + bw > ix + items_w {
                bx = ix;
                iy += 38.0;
            }
            ui.button(bx, iy, Some(bw), 30.0, &label, Style::Secondary, va(*a));
            bx += bw + 6.0;
        }
        iy += 42.0;
    }
    if app.vaults.category == 2 {
        let label = EN.vault_totp_codes;
        let bw = ui.measure(12.0, W::S, label) + 24.0;
        ui.button(
            ix,
            iy,
            Some(bw),
            30.0,
            label,
            Style::Secondary,
            Action::OpenCodes,
        );
        iy += 42.0;
    }
    let sel = app.vaults.item[app.vaults.category].min(items.len().saturating_sub(1));
    let list_bottom = h - 24.0;
    let row = 58.0;
    let fit = ((list_bottom - iy) / row).floor().max(1.0) as usize;
    let first = sel.saturating_sub(fit.saturating_sub(1));
    for (k, r) in items.iter().enumerate().skip(first).take(fit) {
        let (t, line, mono) = item_heading(app, *r);
        let on = k == sel && app.vaults.form.is_none() && !app.vaults.saving;
        ui.fill(
            ix,
            iy,
            items_w,
            row - 6.0,
            8.0,
            if on { SURFACE } else { BG },
        );
        ui.stroke(
            ix,
            iy,
            items_w,
            row - 6.0,
            8.0,
            if on { ACCENT.with_alpha(120) } else { LINE },
        );
        let t = ui.fit(14.0, if mono { W::M } else { W::S }, &t, items_w - 24.0);
        ui.text(
            ix + 12.0,
            iy + 8.0,
            14.0,
            if mono { W::M } else { W::S },
            TEXT,
            &t,
        );
        let line = ui.fit(12.0, W::R, &line, items_w - 24.0);
        ui.text(ix + 12.0, iy + 30.0, 12.0, W::R, MUTED, &line);
        ui.hit(ix, iy, items_w, row - 6.0, va(V::Item(k)));
        iy += row;
    }
    if items.is_empty() {
        ui.text(ix, iy, 13.0, W::R, DIM, EN.vault_none_here);
    }

    // The detail: a form, the session's keys or wallets to save, or the item.
    ui.fill(detail_x - 16.0, top - 8.0, 1.0, h - top - 16.0, 0.0, LINE);
    // The item scrolls when it is taller than the room under the head: a
    // wallet's chart may be.
    let gap = faraday_draw::tokens::GAP;
    let view_top = top - gap;
    let clip = ui.rect(detail_x - gap, view_top, detail_w + 2.0 * gap, h - view_top);
    ui.c.push_clip(clip);
    let off = app.vaults.item_offset();
    let foot = detail(app, ui, v, &items, sel, detail_x, top - off, detail_w);
    ui.c.pop_clip();
    let content = foot + off - top;
    let max = content - (h - top);
    ui.report_scroll_in(crate::ui::Slot::VaultItem, clip, max, off);
}

/// What an open vault with nothing in it says.
const NOTHING_YET: &str = EN.vault_empty_line;

/// What adds an item of kind `k`, as its category's Add button reads.
fn add_label(k: usize) -> Option<&'static str> {
    match k {
        0 => Some(EN.vault_save_a_key),
        1 => Some(EN.vault_save_a_wallet),
        2 => Some(EN.vault_add_an_entry),
        3 => Some(EN.vault_add_a_note),
        4 => Some(EN.vault_make_a_key),
        5 => Some(EN.vault_make_keys),
        6 => Some(EN.apk_make_a_key),
        _ => None,
    }
}

/// The kinds the contents list: those the slot holds something of, and
/// the one shown when it was chosen; each with its place in `CATEGORIES`,
/// label and count.
fn listed_kinds(app: &View, v: &vault::OpenVault) -> Vec<(usize, &'static str, usize)> {
    CATEGORIES
        .iter()
        .enumerate()
        .map(|(k, (label, kinds))| {
            let n = app
                .vaults
                .records(v.id)
                .iter()
                .filter(|(_, r)| kinds.contains(&r.kind()))
                .count();
            (k, *label, n)
        })
        .filter(|&(k, _, n)| n > 0 || (k == app.vaults.category && app.vaults.category_chosen))
        .collect()
}

/// Whether the contents show an empty vault's next steps: nothing in it,
/// and nothing chosen or under way.
fn shows_empty(app: &View, v: &vault::OpenVault) -> bool {
    let held = CATEGORIES.iter().any(|(_, kinds)| {
        app.vaults
            .records(v.id)
            .iter()
            .any(|(_, r)| kinds.contains(&r.kind()))
    });
    !held && !app.vaults.category_chosen && !contents_detail(app)
}

/// An empty vault's next steps: every way to add, then writing it out.
fn empty_buttons(app: &View) -> Vec<(String, Style, Action)> {
    let mut out: Vec<(String, Style, Action)> = (0..CATEGORIES.len())
        .filter(|&k| app.category_listed(k))
        .filter_map(|k| add_full(k).map(|a| (a.to_string(), Style::Secondary, va(V::AddKind(k)))))
        .collect();
    out.push((
        fill(EN.vault_write_to_medium, &[app.medium.a()]),
        Style::Secondary,
        va(V::WriteOut),
    ));
    out
}

/// What adds an item of kind `k` where no category says which: an empty
/// vault's buttons.
fn add_full(k: usize) -> Option<&'static str> {
    match k {
        0 => Some(EN.vault_save_a_bitcoin),
        1 => Some(EN.vault_save_a_wallet),
        2 => Some(EN.vault_add_an_entry),
        3 => Some(EN.vault_add_a_note),
        4 => Some(EN.vault_make_a_gpg_key),
        5 => Some(EN.vault_make_secure_boot),
        6 => Some(EN.apk_make_a_key),
        _ => None,
    }
}

/// The same, short, for the add rows under the kinds.
fn add_short(k: usize) -> Option<&'static str> {
    match k {
        0 => Some(EN.vault_bitcoin_key),
        1 => Some(EN.vault_wallet),
        2 => Some(EN.vault_entry),
        3 => Some(EN.vault_note),
        4 => Some(EN.vault_gpg_key),
        5 => Some(EN.vault_secure_boot),
        6 => Some(EN.apk_key_short),
        _ => None,
    }
}

/// A row for each kind to add at (x, y), w wide and `row` tall, the Add
/// icon in the accent colour before its label. Returns their height.
fn add_rows(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, row: f32) -> f32 {
    let mut cy = y;
    for k in (0..CATEGORIES.len()).filter(|&k| app.category_listed(k)) {
        let Some(add) = add_short(k) else {
            continue;
        };
        ui.icon(
            x + 4.0,
            cy + (row - 20.0) / 2.0,
            20.0,
            Icon::Add,
            12.0,
            ACCENT,
        );
        let t = ui.fit(13.0, W::S, add, w - 34.0);
        ui.text_mid(x + 30.0, cy, row, 13.0, W::S, TEXT, &t);
        ui.hit(x, cy, w, row, va(V::AddKind(k)));
        cy += row + 4.0;
    }
    cy - y
}

/// Whether the contents show an item, or something being added or made,
/// rather than the vault's list: on a small panel, a page of its own.
pub(crate) fn contents_detail(app: &Faraday) -> bool {
    let v = &app.vaults;
    v.item_open
        || v.form.is_some()
        || v.prompt.is_some()
        || v.saving
        || v.signing
        || v.sb_images
        || v.apk.signing
}

/// [`contents`] on a small panel: the vault, its kinds and the items of
/// the kind chosen, as a list; a chosen item, or what is being added or
/// made, as a page of its own that the bar goes back from.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn contents_compact(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    use crate::compact::screen::{about, finish, row};
    use crate::screens::wrap_buttons;
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    let Some(v) = app.vaults.open.get(app.vaults.current) else {
        ui.text(x, y, 15.0, W::R, MUTED, EN.vault_no_vault_is_open);
        if let Some((label, a)) = app.vault_way(Screen::VaultContents) {
            let label = ui.fit(15.0, W::S, &label, w - 24.0);
            ui.button(x, y + 32.0, Some(w), 44.0, &label, Style::Primary, a);
        }
        return;
    };
    let (cat_label, kinds) = CATEGORIES[app.vaults.category];
    let items: Vec<RecordView<'_>> = app
        .vaults
        .records(v.id)
        .iter()
        .filter(|(_, r)| kinds.contains(&r.kind()))
        .map(|(_, r)| r)
        .collect();
    let sel = app.vaults.item[app.vaults.category].min(items.len().saturating_sub(1));
    if contents_detail(app) {
        y = detail(app, ui, v, &items, sel, x, y, w);
        finish(app, ui, x0, cw, h, y - top + 16.0);
        return;
    }
    let label = ui.fit(18.0, W::S, &v.label(app.vaults.store()), w);
    ui.text(x, y, 18.0, W::S, TEXT, &label);
    y += 28.0;
    let state = if v.changes == 0 {
        EN.vault_open_button.to_string()
    } else {
        fill(
            EN.vault_open_unsaved,
            &[
                &v.changes.to_string(),
                (if v.changes == 1 {
                    EN.vault_change
                } else {
                    EN.vault_changes
                }),
            ],
        )
    };
    ui.text(
        x,
        y,
        13.0,
        W::S,
        if v.changes == 0 { OK } else { WARN },
        &state,
    );
    y += 22.0;
    let sub = format!("{} · {}", v.name, cost_text(&v.header().cost));
    let sub = ui.fit(12.0, W::M, &sub, w);
    ui.text(x, y, 12.0, W::M, MUTED, &sub);
    y += 26.0;
    let half = (w - 8.0) / 2.0;
    ui.button(
        x,
        y,
        Some(half),
        40.0,
        EN.vault_rename,
        Style::Secondary,
        va(V::Rename),
    );
    ui.button(
        x + half + 8.0,
        y,
        Some(half),
        40.0,
        EN.vault_lock,
        Style::Primary,
        Action::LockAsk,
    );
    y += 52.0;
    if app.vaults.open.len() > 1 {
        let labels: Vec<String> = app
            .vaults
            .open
            .iter()
            .map(|o| o.label(app.vaults.store()))
            .collect();
        let items: Vec<(&str, Style, Action)> = labels
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let style = if i == app.vaults.current {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                (l.as_str(), style, va(V::Show(i)))
            })
            .collect();
        y += wrap_buttons(ui, x, y, w, 32.0, &items) + 4.0;
    }
    // The kinds it holds, each with how many, then a row for each kind
    // to add.
    let listed = listed_kinds(app, v);
    let kind_labels: Vec<(usize, String)> = listed
        .iter()
        .map(|(k, label, n)| (*k, format!("{label} {n}")))
        .collect();
    let kind_items: Vec<(&str, Style, Action)> = kind_labels
        .iter()
        .map(|(k, l)| {
            let style = if *k == app.vaults.category {
                Style::Primary
            } else {
                Style::Secondary
            };
            (l.as_str(), style, va(V::Category(*k)))
        })
        .collect();
    let empty = shows_empty(app, v);
    if !kind_items.is_empty() {
        y += wrap_buttons(ui, x, y, w, 34.0, &kind_items) + 4.0;
    }
    if !empty {
        y += add_rows(app, ui, x, y, w, 36.0) + 4.0;
    }
    ui.rule(x, y, w, LINE);
    y += 14.0;
    if empty {
        ui.text(x, y, 15.0, W::S, TEXT, NOTHING_YET);
        y += 34.0;
        for (label, style, action) in empty_buttons(app) {
            ui.button(x, y, Some(w), 44.0, &label, style, action);
            y += 54.0;
        }
        finish(app, ui, x0, cw, h, y - top + 16.0);
        return;
    }
    section_label(ui, x, y, cat_label);
    y += 28.0;
    // What can fill this kind.
    let add = add_label(app.vaults.category);
    let entries_files = app
        .inbox
        .iter()
        .filter(|i| i.kind == crate::FileKind::Entries)
        .count();
    let mut extra: Vec<(String, V)> = Vec::new();
    if let Some(a) = add {
        extra.push((a.to_string(), V::Add));
    }
    match app.vaults.category {
        2 => {
            extra.push((EN.vault_scan_a_code.to_string(), V::ScanEntry));
            if entries_files > 0 {
                extra.push((
                    fill(EN.vault_import_from_files, &[&entries_files.to_string()]),
                    V::ImportAllEntries,
                ));
            }
        }
        0 | 3 => {
            for (k, i) in app.inbox.iter().enumerate() {
                if i.kind == crate::FileKind::Backup {
                    extra.push((
                        fill(EN.vault_import_n, &[&i.name.to_string()]),
                        V::ImportBackup(k),
                    ));
                }
            }
        }
        _ => {}
    }
    let extra_items: Vec<(&str, Style, Action)> = extra
        .iter()
        .map(|(l, a)| (l.as_str(), Style::Secondary, va(*a)))
        .collect();
    y += wrap_buttons(ui, x, y, w, 36.0, &extra_items) + 4.0;
    // TOTP codes for the entries in open vaults (`docs/TOTP.md` §5.1),
    // beside Add an entry and Scan a code.
    if app.vaults.category == 2 {
        let label = EN.vault_totp_codes;
        let bw = ui.measure(13.0, W::S, label) + 28.0;
        ui.button(
            x,
            y,
            Some(bw),
            36.0,
            label,
            Style::Secondary,
            Action::OpenCodes,
        );
        y += 44.0;
    }
    for (k, r) in items.iter().enumerate() {
        let (t, line, _) = item_heading(app, *r);
        y += row(ui, x, y, w, None, &t, &line, MUTED, Some(va(V::Item(k))));
    }
    if items.is_empty() {
        ui.text(x, y, 13.0, W::R, DIM, EN.vault_none_here);
        y += 30.0;
    }
    y += about(
        ui,
        x,
        y,
        w,
        &fill(EN.vault_secret_values, &[app.medium.noun()]),
    );
    finish(app, ui, x0, cw, h, y - top + 16.0);
}

/// A small text pill (`docs/DESIGN.md` §4.8): the entry detail's HOTP
/// and Unreadable badges in place of a code.
fn code_badge(ui: &mut Ui, x: f32, y: f32, label: &str) {
    let tw = ui.measure(12.0, W::S, label);
    let pw = tw + 24.0;
    ui.fill(x, y, pw, 26.0, 13.0, MUTED.with_alpha(30));
    ui.stroke(x, y, pw, 26.0, 13.0, MUTED.with_alpha(110));
    ui.text_mid(x + (pw - tw) / 2.0, y, 26.0, 12.0, W::S, MUTED, label);
}

/// The entry form on the codes screen (`docs/TOTP.md` §5.3): Add an
/// entry, or the form a scanned setup code filled in. Always a new
/// entry — editing one happens on Vault contents, where its row leads.
/// Returns the height it took.
pub(crate) fn codes_form(app: &View, ui: &mut Ui, dx: f32, dy: f32, dw: f32) -> f32 {
    let top = dy;
    let mut dy = dy;
    let Some(form) = &app.vaults.form else {
        return 0.0;
    };
    ui.text(dx, dy, 18.0, W::S, TEXT, EN.vault_add_an_entry);
    dy += 40.0;
    for (k, (_, label, b, masked)) in form.fields.iter().enumerate() {
        ui.text(dx, dy, 12.0, W::R, MUTED, label);
        dy += 20.0;
        let focused = app.vaults.focus == Some(Focus::Field(k));
        if vault::multiline(form.kind, form.fields[k].0) {
            dy += text_area(ui, dx, dy, dw, b, focused, va(V::FocusField(k))) + 12.0;
        } else if *masked {
            secret_box(
                ui,
                dx,
                dy,
                dw,
                b,
                app.field_shows(form, k),
                app.field_caret(form, k, (b, focused)),
                va(V::FocusField(k)),
                va(V::ShowTyped),
            );
            app.password_chip(ui, form, k, (dx, dy, dw), (b, focused));
            dy += 52.0;
        } else {
            text_box(ui, dx, dy, dw, b, false, focused, va(V::FocusField(k)));
            dy += 52.0;
        }
    }
    dy += 8.0;
    ui.button(
        dx,
        dy,
        Some(120.0),
        40.0,
        EN.vault_save,
        Style::Primary,
        va(V::FormSave),
    );
    ui.button(
        dx + 130.0,
        dy,
        Some(120.0),
        40.0,
        EN.vault_cancel,
        Style::Ghost,
        va(V::FormCancel),
    );
    dy += 56.0;
    dy - top
}

/// The detail of the contents at (dx, dy), dw wide: a form, a prompt,
/// the session's keys or wallets to save, a GPG key's files to sign,
/// Secure Boot's images, or the item. Returns where it ends.
#[allow(clippy::too_many_arguments)]
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn detail(
    app: &View,
    ui: &mut Ui,
    v: &vault::OpenVault,
    items: &[RecordView<'_>],
    sel: usize,
    dx: f32,
    mut dy: f32,
    dw: f32,
) -> f32 {
    if let Some(form) = &app.vaults.form {
        let revoking = form.kind == kind::GPG && form.fields.first().is_some_and(|f| f.0 == 3);
        let heading = match (form.kind, form.record) {
            (kind::GPG, _) if revoking => EN.vault_make_a_revocation,
            (kind::SLOT_LABEL, _) => EN.vault_rename_the_vault,
            (kind::NOTE, None) => EN.vault_add_a_note,
            (kind::NOTE, Some(_)) => EN.vault_edit_the_note,
            (kind::GPG, _) => EN.vault_make_a_key,
            (kind::SECURE_BOOT, _) => EN.vault_make_secure_boot,
            (kind::APK_KEY, _) => EN.apk_make_a_key,

            (_, None) => EN.vault_add_an_entry,
            _ => EN.vault_edit_the_entry,
        };
        ui.text(dx, dy, 18.0, W::S, TEXT, heading);
        dy += 40.0;
        for (k, (_, label, b, masked)) in form.fields.iter().enumerate() {
            ui.text(dx, dy, 12.0, W::R, MUTED, label);
            dy += 20.0;
            let focused = app.vaults.focus == Some(Focus::Field(k));
            if vault::multiline(form.kind, form.fields[k].0) {
                dy += text_area(ui, dx, dy, dw, b, focused, va(V::FocusField(k))) + 12.0;
            } else {
                if *masked {
                    secret_box(
                        ui,
                        dx,
                        dy,
                        dw,
                        b,
                        app.field_shows(form, k),
                        app.field_caret(form, k, (b, focused)),
                        va(V::FocusField(k)),
                        va(V::ShowTyped),
                    );
                    app.password_chip(ui, form, k, (dx, dy, dw), (b, focused));
                } else {
                    text_box(ui, dx, dy, dw, b, false, focused, va(V::FocusField(k)));
                }
                dy += 52.0;
            }
        }
        if revoking {
            ui.text(dx, dy, 12.0, W::R, MUTED, EN.vault_reason);
            dy += 22.0;
            dy += reasons_row(app, ui, dx, dy, dw) + 16.0;
        } else if form.kind == kind::GPG {
            ui.text(dx, dy, 12.0, W::R, MUTED, EN.vault_expires);
            dy += 22.0;
            dy += years_row(ui, dx, dy, dw, Some(app.vaults.gpg_years), V::GpgYears) + 16.0;
        }
        let save = match form.kind {
            kind::GPG if revoking => EN.vault_make_the_revocation,
            kind::GPG => EN.vault_make_the_key,
            kind::APK_KEY => EN.apk_make_the_key,
            kind::SECURE_BOOT => EN.vault_make_the_keys,
            _ => EN.vault_save,
        };
        // On a small panel Save is pinned at the foot and Cancel stays.
        let sw = if crate::screens::pin_button(
            ui,
            dx,
            dy,
            None,
            40.0,
            save,
            Style::Primary,
            va(V::FormSave),
        ) {
            ui.measure(14.0, W::S, save) + 42.0
        } else {
            0.0
        };
        ui.button(
            dx + sw,
            dy,
            None,
            40.0,
            EN.vault_cancel,
            Style::Ghost,
            va(V::FormCancel),
        );
        return dy;
    }
    if let Some(p) = &app.vaults.prompt {
        let heading = match p.purpose {
            vault::Purpose::Backup(k) => fill(
                EN.vault_open_item,
                &[(app
                    .inbox
                    .get(k)
                    .map_or(EN.vault_the_backup, |i| i.name.as_str()))],
            ),
            vault::Purpose::KeyPassphrase | vault::Purpose::ForWallet(..) => {
                EN.vault_load_with_a_bip.to_string()
            }
            vault::Purpose::Kdbx => EN.vault_export_for_keepass.to_string(),
        };
        let heading = ui.fit(18.0, W::S, &heading, dw);
        ui.text(dx, dy, 18.0, W::S, TEXT, &heading);
        dy += 40.0;
        ui.text(dx, dy, 12.0, W::R, MUTED, EN.vault_passphrase);
        dy += 20.0;
        let focused = app.vaults.focus == Some(Focus::Prompt);
        crate::demo::pass_box(
            app,
            ui,
            (dx, dy, dw),
            &p.text,
            (app.vaults.typed_shown, focused),
            (va(V::FocusPrompt), va(V::ShowTyped)),
            crate::demo::PassField::Prompt,
        );
        dy += 56.0 + never_under(app, ui, dx, dy, dw);
        if app.vaults.working == Some(vault::Work::Backup) {
            ui.text_mid(dx, dy, 40.0, 15.0, W::S, ACCENT, EN.vault_opening);
        } else if app.vaults.working == Some(vault::Work::Kdbx) {
            ui.text_mid(dx, dy, 40.0, 15.0, W::S, ACCENT, EN.vault_sealing);
        } else {
            let label = match p.purpose {
                vault::Purpose::Backup(_) => EN.vault_open_button,
                vault::Purpose::Kdbx => EN.vault_seal_and_export,
                vault::Purpose::KeyPassphrase | vault::Purpose::ForWallet(..) => EN.vault_load,
            };
            let sw = if crate::screens::pin_button(
                ui,
                dx,
                dy,
                None,
                40.0,
                label,
                Style::Primary,
                va(V::PromptGo),
            ) {
                ui.measure(14.0, W::S, label) + 42.0
            } else {
                0.0
            };
            ui.button(
                dx + sw,
                dy,
                None,
                40.0,
                EN.vault_cancel,
                Style::Ghost,
                va(V::PromptCancel),
            );
        }
        dy += 54.0;
        if let Some(e) = &p.error {
            ui.wrap(dx, dy, dw, 13.0, W::R, ERR, e);
        }
        return dy;
    }
    if app.vaults.saving {
        return save_panel(app, ui, v, dx, dy, dw);
    }
    if app.vaults.sb_images {
        return images_panel(app, ui, dx, dy, dw);
    }
    if app.vaults.apk.signing {
        return apk_sign_panel(app, ui, items, sel, dx, dy, dw);
    }
    if app.vaults.signing {
        ui.text(dx, dy, 18.0, W::S, TEXT, EN.vault_sign_a_file);
        dy += 40.0;
        // The key that signs, named as GnuPG's pinentry names it: user
        // ID, key IDs and creation date, and which subkey signs.
        if let Some(g) = items
            .get(sel)
            .and_then(|r| crate::gpg::shown(*r, app.tz_min()))
        {
            if let Some(uid) = g.user_ids.first() {
                dy += ui.wrap(dx, dy, dw, 14.0, W::S, TEXT, &format!("\"{uid}\"")) + 4.0;
            }
            dy += ui.wrap(dx, dy, dw, 12.0, W::M, MUTED, &g.subkey) + 4.0;
            dy += ui.wrap(
                dx,
                dy,
                dw,
                12.0,
                W::R,
                MUTED,
                &fill(
                    EN.vault_signing_key_ids,
                    &[
                        &g.subkey_id.to_string(),
                        &g.key_id.to_string(),
                        &g.created.to_string(),
                    ],
                ),
            ) + 16.0;
        }
        let files = app.gpg_files();
        if files.is_empty() {
            ui.text(
                dx,
                dy,
                13.0,
                W::R,
                DIM,
                &fill(EN.vault_no_file, &[app.medium.from_the()]),
            );
            dy += 30.0;
        } else {
            let all_ticked = files.iter().all(|f| app.gpg_ticked(f));
            ui.checkbox(dx, dy + 11.0, all_ticked, true);
            ui.text_mid(dx + 30.0, dy, 40.0, 14.0, W::M, TEXT, EN.vault_select_all);
            ui.hit(dx, dy, dw, 40.0, va(V::GpgTickAll));
            ui.rule(dx, dy + 40.0, dw, INNER);
            dy += 42.0;
        }
        for (fi, f) in files.iter().enumerate() {
            let ticked = app.gpg_ticked(f);
            let signed = app
                .outbox
                .iter()
                .any(|o| o.name == format!("{}.asc", f.name));
            // Show as QR and PNG hold one signature over this file each;
            // ticking, Sign and its sheet's hold is the way to make it. They follow
            // the file by its hash, not its place in the Inbox.
            let prefix: [u8; 8] = f.hash[..8].try_into().unwrap_or([0; 8]);
            let buttons = [
                (
                    EN.vault_show_as_qr,
                    Action::ShowCode(Code::GpgSignature(prefix)),
                ),
                ("PNG", Action::CodePng(Code::GpgSignature(prefix))),
            ];
            let signed_w = if signed {
                ui.measure(12.0, W::S, EN.vault_signed) + 18.0
            } else {
                0.0
            };
            let widths: Vec<f32> = buttons
                .iter()
                .map(|(l, _)| ui.measure(13.0, W::S, l) + 30.0)
                .collect();
            let all = widths.iter().sum::<f32>() + 6.0;
            let (name_w, by) = if ui.compact {
                (dw - 40.0 - signed_w - 8.0, dy + 40.0)
            } else {
                (dw - 40.0 - all - signed_w - 24.0, dy + 2.0)
            };
            ui.checkbox(dx, dy + 11.0, ticked, true);
            let line = format!("{} · {}", f.name, gpg_file_size(f.size as u64));
            let name = ui.fit(14.0, W::M, &line, name_w);
            ui.text_mid(dx + 30.0, dy, 40.0, 14.0, W::M, TEXT, &name);
            if signed {
                ui.chip(
                    dx + 30.0 + name_w + 8.0,
                    dy + 10.0,
                    EN.vault_signed,
                    OK,
                    OK.with_alpha(30),
                );
            }
            // On a small panel the buttons are on the line below: the
            // whole first line ticks.
            let hit_w = if ui.compact { dw } else { dw - all - 16.0 };
            ui.hit(dx, dy, hit_w, 40.0, va(V::GpgTick(fi)));
            let mut bx = if ui.compact { dx + 30.0 } else { dx + dw - all };
            for ((label, action), bw) in buttons.iter().zip(&widths) {
                ui.button(bx, by, Some(*bw), 36.0, label, Style::Secondary, *action);
                bx += bw + 6.0;
            }
            // What is signed: the file's SHA-256, and a small text file's
            // text, so that the bytes can be checked before the hold.
            let mut ey = dy + if ui.compact { 84.0 } else { 46.0 };
            let hex: String = f.hash.iter().map(|b| format!("{b:02x}")).collect();
            ey += ui.wrap(dx + 30.0, ey, dw - 30.0, 12.0, W::M, MUTED, &hex) + 6.0;
            if let Some(t) = &f.text {
                ey += ui.wrap(dx + 30.0, ey, dw - 30.0, 12.0, W::M, TEXT, t) + 6.0;
            }
            ui.rule(dx, ey, dw, INNER);
            dy = ey + 8.0;
        }
        dy += 8.0;
        // Sign opens Sign with {key} over the page; only its hold signs
        // (F63).
        let any = files.iter().any(|f| app.gpg_ticked(f));
        let w = ui.button(
            dx,
            dy,
            None,
            36.0,
            EN.vault_sign,
            if any { Style::Primary } else { Style::Disabled },
            va(V::GpgSign),
        );
        ui.button(
            dx + w + 10.0,
            dy,
            None,
            36.0,
            EN.vault_back_to_the_key,
            Style::Ghost,
            va(V::GpgSignPick),
        );
        return dy + 48.0;
    }
    let Some(&r) = items.get(sel) else {
        return dy;
    };
    let (t, line, mono) = item_heading(app, r);
    ui.text(dx, dy, 18.0, if mono { W::M } else { W::S }, TEXT, &t);
    dy += 30.0;
    ui.text(dx, dy, 13.0, W::R, MUTED, &line);
    dy += 34.0;
    let mut secret_id = 0usize;
    let field_row = |ui: &mut Ui, dy: &mut f32, label: &str, value: &str, mono: bool| {
        ui.text(dx, *dy, 12.0, W::R, MUTED, label);
        *dy += 20.0;
        *dy += ui.wrap(
            dx,
            *dy,
            dw,
            14.0,
            if mono { W::M } else { W::R },
            TEXT,
            value,
        ) + 14.0;
    };
    let mut hold_row = |ui: &mut Ui, dy: &mut f32, label: &str, value: &str, mono: bool| {
        ui.text(dx, *dy, 12.0, W::R, MUTED, label);
        *dy += 20.0;
        *dy += secret(ui, dx, *dy, dw, value, mono, secret_id) + 14.0;
        secret_id += 1;
    };
    let mut actions: Vec<(&str, Action)> = Vec::new();
    // Where a new row of them starts, whether or not the last is full.
    let mut breaks: Vec<usize> = Vec::new();
    match r.kind() {
        kind::KEY => {
            let fp = app.vault_key_fingerprint(r).unwrap_or_default();
            field_row(ui, &mut dy, EN.vault_fingerprint, &fp, true);
            // Read from the record once for the frame: a secret shown.
            let m = app.record_words_shown(r);
            if let Some(m) = &m {
                hold_row(ui, &mut dy, EN.vault_words, &words_text(m), true);
                if m.language() == faraday_bip::bip39::Language::English {
                    ui.text(dx, dy, 12.0, W::R, MUTED, EN.board_seedqr);
                    dy += 20.0;
                    dy += seedqr_box(app, ui, dx, dy, dw) + 14.0;
                }
            }
            let loads = r.loads_at_unlock();
            // A seed of a wallet in this vault loads with it (F62).
            let with = app
                .vault_selected_index()
                .and_then(|i| app.vault_key_wallet(app.vaults.current, i));
            field_row(
                ui,
                &mut dy,
                EN.vault_bip_39_passphrase,
                if r.has(field::KEY_PASSPHRASE) {
                    EN.vault_stored
                } else {
                    EN.vault_not_stored
                },
                false,
            );
            match &with {
                Some(w) => field_row(
                    ui,
                    &mut dy,
                    EN.vault_load_at_unlock,
                    &fill(EN.vault_loads_with_wallet, &[w]),
                    false,
                ),
                None => field_row(
                    ui,
                    &mut dy,
                    EN.vault_load_at_unlock,
                    if loads { EN.vault_yes } else { EN.vault_no },
                    false,
                ),
            }
            let here = app.session.keys.all().any(|k| fp_text(k.fingerprint) == fp);
            field_row(
                ui,
                &mut dy,
                EN.vault_in_this_session,
                if here {
                    EN.vault_loaded
                } else {
                    EN.vault_not_loaded
                },
                false,
            );
            if !here {
                actions.push((EN.vault_load_into_session, va(V::Load)));
                if m.is_some() && !r.has(field::KEY_PASSPHRASE) {
                    actions.push((EN.vault_load_with_a_passphrase, va(V::LoadWithPassphrase)));
                }
            }
            if with.is_none() {
                actions.push((
                    if loads {
                        EN.vault_dont_load_at_unlock
                    } else {
                        EN.vault_load_at_unlock
                    },
                    va(V::ToggleLoad),
                ));
            }
            if m.is_some() {
                actions.push((EN.vault_save_to_a_stick, va(V::SaveToStick)));
            }
        }
        kind::WALLET => {
            // The wallet's chart the other way up (`docs/history/NEW-WALLET.md`
            // §7.2): where each seed is kept, its keys, the wallet.
            let up = crate::glance::Direction::BackupFirst;
            if let Some(g) = app
                .vault_selected_index()
                .and_then(|i| app.glance_of_vault(app.vaults.current, i))
            {
                dy += if ui.compact {
                    crate::glance::draw_column(ui, &g, up, dx, dy, dw)
                } else {
                    crate::glance::draw(ui, &g, up, dx, dy, dw)
                } + faraday_draw::tokens::PAD;
            }
            let text = r.public_text(field::WALLET).unwrap_or("");
            field_row(ui, &mut dy, EN.vault_descriptor, text, true);
            let loads = r.loads_at_unlock();
            field_row(
                ui,
                &mut dy,
                EN.vault_load_at_unlock,
                if loads { EN.vault_yes } else { EN.vault_no },
                false,
            );
            actions.push((EN.vault_open_in_wallets, va(V::Load)));
            actions.push((
                if loads {
                    EN.vault_dont_load_at_unlock
                } else {
                    EN.vault_load_at_unlock
                },
                va(V::ToggleLoad),
            ));
            actions.push((EN.vault_show_as_qr, va(V::WalletQr)));
            actions.push((EN.vault_save_to_a_stick, va(V::SaveToStick)));
        }
        kind::ENTRY => {
            if let Some(u) = r.public_text(field::USERNAME) {
                field_row(ui, &mut dy, EN.vault_username, u, false);
            }
            if r.has(field::PASSWORD) {
                let p = app.reveal_record(r, field::PASSWORD, Reveal::VaultEntry(Part::Password));
                hold_row(ui, &mut dy, EN.vault_password, &p, true);
            }
            if let Some(u) = r.public_text(field::URL) {
                field_row(ui, &mut dy, EN.vault_url, u, true);
            }
            if r.has(field::TOTP) {
                // A row before the secret: the code, live, or Set the
                // time while the clock is not checked (`docs/TOTP.md`
                // §6).
                ui.text(dx, dy, 12.0, W::R, MUTED, EN.vault_code);
                dy += 20.0;
                let now = app.clock_checked().then(|| app.clock_now().unwrap_or(0));
                match app.entry_code(r, now) {
                    Ok(Some(code)) => {
                        let (grouped, left) = (code.grouped, code.left);
                        let gw = ui.measure(18.0, W::M, &grouped);
                        ui.text(dx, dy, 18.0, W::M, TEXT, &grouped);
                        ui.text(
                            dx + gw + 12.0,
                            dy + 4.0,
                            12.0,
                            W::R,
                            MUTED,
                            &format!("{left} s"),
                        );
                        dy += 34.0;
                    }
                    Ok(None) => {
                        ui.button(
                            dx,
                            dy,
                            None,
                            36.0,
                            EN.vault_set_the_time,
                            Style::Secondary,
                            Action::OpenSetTime,
                        );
                        dy += 44.0;
                    }
                    Err(faraday_keys::totp::NotTotp::Hotp) => {
                        code_badge(ui, dx, dy, "HOTP");
                        dy += 36.0;
                    }
                    Err(faraday_keys::totp::NotTotp::Unreadable) => {
                        code_badge(ui, dx, dy, EN.vault_unreadable);
                        dy += 36.0;
                    }
                }
                dy += 10.0;
                let t = app.reveal_record(r, field::TOTP, Reveal::VaultEntry(Part::Totp));
                hold_row(ui, &mut dy, EN.vault_totp_secret, &t, true);
            }
            if r.has(field::NOTES) {
                let n = app.reveal_record(r, field::NOTES, Reveal::VaultEntry(Part::EntryNotes));
                field_row(ui, &mut dy, EN.vault_notes, &n, false);
            }
            actions.push((EN.vault_edit, va(V::Edit)));
            actions.push((EN.vault_export_for_keepass, va(V::ExportKdbx)));
        }
        kind::NOTE => {
            hold_row(
                ui,
                &mut dy,
                EN.vault_text,
                &app.reveal_record(r, field::NOTE, Reveal::VaultEntry(Part::Note)),
                false,
            );
            actions.push((EN.vault_edit, va(V::Edit)));
        }
        kind::SHEET => {
            let (d, _, note) = faraday_keys::vault::sheet_parts(r);
            field_row(ui, &mut dy, EN.vault_descriptor, &d, true);
            if !note.is_empty() {
                field_row(ui, &mut dy, EN.vault_note, &note, false);
            }
        }
        kind::GPG => {
            if let Some(g) = crate::gpg::shown(r, app.tz_min()) {
                field_row(ui, &mut dy, EN.vault_fingerprint, &g.fingerprint, true);
                field_row(ui, &mut dy, EN.vault_signing_subkey, &g.subkey, true);
                for uid in &g.user_ids {
                    field_row(ui, &mut dy, EN.vault_user_id, uid, false);
                }
                let valid = if g.expires == EN.vault_never {
                    fill(EN.vault_gpg_no_expiry, &[&g.created.to_string()])
                } else {
                    fill(
                        EN.vault_gpg_validity,
                        &[&g.created.to_string(), &g.expires.to_string()],
                    )
                };
                field_row(ui, &mut dy, EN.vault_valid, &valid, false);
                if let Some(t) = &g.revoked {
                    field_row(
                        ui,
                        &mut dy,
                        EN.vault_status,
                        &fill(EN.vault_revoked_date, &[&t.to_string()]),
                        false,
                    );
                }
                // Where the revocation certificate is kept, as GnuPG says
                // where it stored it.
                let kept = match &g.revocation_kept {
                    Some(d) => fill(EN.vault_kept_with_the_key, &[&d.to_string()]),
                    None => EN.vault_none_kept.to_string(),
                };
                field_row(ui, &mut dy, EN.vault_revocation_certificate, &kept, false);
                // The paperkey is made only while it is held.
                ui.text(dx, dy, 12.0, W::R, MUTED, EN.vault_paperkey);
                dy += 20.0;
                dy += secret_with(
                    ui,
                    dx,
                    dy,
                    dw,
                    || {
                        app.paperkey_shown(r)
                            .map(faraday_keys::Held::release)
                            .unwrap_or_default()
                    },
                    true,
                    0,
                ) + 14.0;
                // The term now, then the new term to choose: nothing is
                // renewed until one is chosen.
                let now_line = if g.expires == EN.vault_never {
                    EN.vault_term_never.to_string()
                } else {
                    fill(EN.vault_term_until, &[&g.expires.to_string()])
                };
                ui.text(dx, dy, 12.0, W::R, MUTED, &now_line);
                dy += 22.0;
                ui.text(dx, dy, 12.0, W::R, MUTED, EN.vault_renew_label);
                dy += 22.0;
                // Renew at the row's end; on a small panel under it.
                let rw = ui.measure(13.0, W::S, EN.vault_renew) + 30.0;
                let yh = years_row(
                    ui,
                    dx,
                    dy,
                    if ui.compact { dw } else { dw - rw - 8.0 },
                    app.vaults.renew_years,
                    V::GpgRenewTerm,
                );
                let ry = if ui.compact { dy + yh + 8.0 } else { dy };
                if app.selected_waits().is_some() {
                    let bw = ui.measure(13.0, W::S, EN.backup_flow_first) + 30.0;
                    ui.button(
                        dx + dw - bw,
                        ry,
                        Some(bw),
                        32.0,
                        EN.backup_flow_first,
                        Style::Secondary,
                        back_up_first(crate::backup_flow::Kind::Gpg),
                    );
                } else {
                    ui.button(
                        dx + dw - rw,
                        ry,
                        Some(rw),
                        32.0,
                        EN.vault_renew,
                        if app.vaults.renew_years.is_some() {
                            Style::Secondary
                        } else {
                            Style::Disabled
                        },
                        va(V::GpgRenew),
                    );
                }
                dy = ry + 46.0;
            }
            dy += backup_row(app, ui, crate::backup_flow::Kind::Gpg, r, dx, dy, dw);
            // Each file on a row of its own: to the Outbox, as a code,
            // as a picture. The revocation certificate is a secret: it
            // goes through the secret sheet, and has no code or picture.
            // On a small panel the code and the picture go under the
            // file's own button.
            breaks.push(actions.len());
            actions.push((EN.vault_export_public, va(V::GpgExport)));
            if ui.compact {
                breaks.push(actions.len());
            }
            actions.push((EN.vault_show_as_qr, Action::ShowCode(Code::GpgKey)));
            actions.push(("PNG", Action::CodePng(Code::GpgKey)));
            breaks.push(actions.len());
            actions.push((EN.vault_export_revocation, va(V::GpgExportRevocation)));
            actions.push((EN.vault_make_a_revocation, va(V::GpgRevocationPage)));
            breaks.push(actions.len());
            if app.selected_waits().is_some() {
                actions.push((
                    EN.backup_flow_first,
                    back_up_first(crate::backup_flow::Kind::Gpg),
                ));
            } else {
                actions.push((EN.vault_sign_a_file, va(V::GpgSignPick)));
            }
        }
        kind::SECURE_BOOT => {
            if let Some(sb) = crate::secure_boot::shown(r) {
                field_row(ui, &mut dy, EN.vault_owner_guid, &sb.owner, true);
                for (label, name) in ["PK", "KEK", "db"].iter().zip(&sb.names) {
                    field_row(ui, &mut dy, label, name, false);
                }
            }
            ui.text(dx, dy, 12.0, W::R, MUTED, EN.vault_enrolment_adds);
            dy += 22.0;
            let own_items: Vec<(&str, Style, Action)> = [
                (EN.vault_windows_compatible, false),
                (EN.vault_own_keys_only, true),
            ]
            .into_iter()
            .map(|(label, own)| {
                let style = if app.vaults.sb_own_only == own {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                (label, style, va(V::SbOwnOnly(own)))
            })
            .collect();
            dy += crate::screens::wrap_buttons(ui, dx, dy, dw, 32.0, &own_items);
            let line = if app.vaults.sb_own_only {
                EN.vault_own_keys_only_warning
            } else {
                EN.vault_yours_with_microsoft
            };
            dy += ui.wrap(dx, dy, dw, 12.0, W::R, MUTED, line) + 14.0;
            dy += backup_row(app, ui, crate::backup_flow::Kind::SecureBoot, r, dx, dy, dw);
            if app.selected_waits().is_some() {
                actions.push((
                    EN.backup_flow_first,
                    back_up_first(crate::backup_flow::Kind::SecureBoot),
                ));
            } else {
                actions.push((EN.vault_enrolment_files, va(V::SbEnrol)));
            }
            actions.push((EN.vault_sign_or_check, va(V::SbImages)));
        }
        kind::APK_KEY => {
            let fp = crate::apk::fingerprint_of(r);
            let waiting = fp.and_then(|fp| app.backup_waits(crate::backup_flow::Kind::Apk, &fp));
            // The page leads with the backup while the key waits for it.
            dy += backup_row(app, ui, crate::backup_flow::Kind::Apk, r, dx, dy, dw);
            if let Some(s) = crate::apk::shown(r, app.tz_min()) {
                field_row(ui, &mut dy, EN.apk_name, &s.name, false);
                field_row(ui, &mut dy, EN.apk_package, &s.package, false);
                field_row(
                    ui,
                    &mut dy,
                    EN.apk_certificate_sha_256,
                    &s.fingerprint,
                    true,
                );
                field_row(ui, &mut dy, EN.apk_key, EN.apk_rsa_3072, false);
                field_row(ui, &mut dy, EN.apk_made, &s.made, false);
            }
            breaks.push(actions.len());
            actions.push((EN.apk_certificate, va(V::ApkExport)));
            if ui.compact {
                breaks.push(actions.len());
            }
            actions.push((EN.vault_show_as_qr, Action::ShowCode(Code::ApkCertificate)));
            actions.push(("PNG", Action::CodePng(Code::ApkCertificate)));
            breaks.push(actions.len());
            if waiting.is_none() {
                actions.push((EN.apk_sign_an_apk, va(V::ApkSignOpen)));
            } else {
                actions.push((
                    EN.backup_flow_first,
                    back_up_first(crate::backup_flow::Kind::Apk),
                ));
            }
        }
        _ => {}
    }
    dy += 6.0;
    let mut bx = dx;
    for (i, (label, a)) in actions.into_iter().enumerate() {
        let bw = ui.measure(13.0, W::S, label) + 30.0;
        if bx > dx && (bx + bw > dx + dw || breaks.contains(&i)) {
            bx = dx;
            dy += 44.0;
        }
        ui.button(bx, dy, Some(bw), 36.0, label, Style::Secondary, a);
        bx += bw + 8.0;
    }
    dy += 48.0;
    let dw_hold = hold_delete(app, ui, dx, dy);
    if r.kind() == kind::GPG {
        hold_button(
            app,
            ui,
            dx + dw_hold + 10.0,
            dy,
            EN.vault_hold_to_mark_revoked,
            va(V::GpgHoldRevoke),
            WARN,
        );
    }
    dy + 48.0
}

/// The Backup row of a key made in the backup flow (`docs/BACKUP-FLOW.md`
/// §8): how many places are written, and **Continue the backup** until
/// every one is. A key unlocked from a stick has no row. Returns the
/// height.
fn backup_row(
    app: &View,
    ui: &mut Ui,
    kind_: crate::backup_flow::Kind,
    r: RecordView<'_>,
    dx: f32,
    dy: f32,
    dw: f32,
) -> f32 {
    let Some(subject) = crate::backup_flow::subject_of(kind_, r) else {
        return 0.0;
    };
    let bytes: Vec<u8> = (0..subject.len() / 2)
        .filter_map(|i| u8::from_str_radix(subject.get(i * 2..i * 2 + 2)?, 16).ok())
        .collect();
    let Some(row) = app.backup_row(kind_, &bytes) else {
        return 0.0;
    };
    let mut h = 0.0;
    ui.text(dx, dy, 12.0, W::R, MUTED, EN.backup_flow_row);
    h += 20.0;
    h += ui.wrap(
        dx,
        dy + h,
        dw,
        14.0,
        W::R,
        if row.waiting { WARN } else { TEXT },
        &row.line,
    ) + 8.0;
    if row.waiting {
        ui.button(
            dx,
            dy + h,
            None,
            36.0,
            EN.backup_flow_continue,
            Style::Secondary,
            Action::Flow(crate::backup_flow::steps::FlowAction::Resume),
        );
        h += 48.0;
    } else {
        h += 6.0;
    }
    h
}

/// **Sign with {key}**, opened by Sign a file's **Sign** (F63): the files
/// ticked, each with its size and the start of its SHA-256; the time the
/// signatures will carry, with **Set the time**; then the hold that signs
/// and Cancel. With the clock not known the time says so and the hold is
/// disabled. Hashes a file the page has not hashed yet.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn gpg_sign_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let chosen = app.gpg_chosen();
    let who = app.gpg_user_id().unwrap_or_default();
    let title = fill(EN.vault_sign_sheet_title, &[&who]);
    let now = app.clock_now();
    let when = match now {
        Some(_) => {
            let zone = format!("UTC{}", vault::tz_offset_text(app.tz_min()));
            format!("{} {zone}", app.now_local_text())
        }
        None => EN.set_time_not_known.to_string(),
    };
    let label = match chosen.len() {
        0 => EN.vault_hold_to_sign.to_string(),
        1 => EN.vault_hold_to_sign_1_file.to_string(),
        n => fill(EN.vault_hold_to_sign_files, &[&n.to_string()]),
    };
    let ready = now.is_some() && !chosen.is_empty();
    let place = if ui.compact {
        (8.0, w - 16.0)
    } else {
        let sw = (w - 48.0).min(640.0);
        ((w - sw) / 2.0, sw)
    };
    let margin = if ui.compact { 8.0 } else { 32.0 };
    let pad = if ui.compact { 16.0 } else { 32.0 };
    crate::compact::scroll_sheet(ui, place, h, margin, pad, &mut |ui, x, y, iw| {
        let mut cy = y;
        if ui.compact {
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Sign, ACCENT, &title);
        } else {
            ui.icon(x, cy, 30.0, Icon::Sign, 18.0, ACCENT);
            let t = ui.fit(20.0, W::S, &title, iw - 42.0);
            ui.text_mid(x + 42.0, cy, 30.0, 20.0, W::S, TEXT, &t);
            cy += 46.0;
        }
        for f in &chosen {
            let line = format!("{} · {}", f.name, gpg_file_size(f.size as u64));
            let line = ui.fit(14.0, W::M, &line, iw);
            ui.text(x, cy, 14.0, W::M, TEXT, &line);
            cy += 22.0;
            let hex: String = f.hash[..8].iter().map(|b| format!("{b:02x}")).collect();
            ui.text(x, cy, 12.0, W::M, MUTED, &format!("{hex}…"));
            cy += 22.0;
            ui.rule(x, cy, iw, INNER);
            cy += 10.0;
        }
        // The time the signatures will carry, and the way to change it.
        ui.text(x, cy, 12.0, W::R, MUTED, EN.vault_signature_time);
        cy += 20.0;
        let set_w = ui.measure(13.0, W::S, EN.vault_set_the_time) + 30.0;
        let tone = if now.is_some() { TEXT } else { WARN };
        if ui.compact {
            cy += ui.wrap(x, cy, iw, 14.0, W::M, tone, &when) + 8.0;
            ui.button(
                x,
                cy,
                Some(set_w),
                36.0,
                EN.vault_set_the_time,
                Style::Secondary,
                Action::OpenSetTime,
            );
            cy += 46.0;
        } else {
            let t = ui.fit(14.0, W::M, &when, iw - set_w - 12.0);
            ui.text_mid(x, cy, 36.0, 14.0, W::M, tone, &t);
            ui.button(
                x + iw - set_w,
                cy,
                Some(set_w),
                36.0,
                EN.vault_set_the_time,
                Style::Secondary,
                Action::OpenSetTime,
            );
            cy += 46.0;
        }
        ui.rule(x, cy, iw, INNER);
        cy += 14.0;
        let hw = if ready {
            hold_button_in(app, ui, x, cy, iw, &label, va(V::GpgHoldSign), ACCENT)
        } else {
            ui.button(
                x,
                cy,
                None,
                36.0,
                &label,
                Style::Disabled,
                va(V::GpgHoldSign),
            )
        };
        let cancel_w = ui.measure(13.0, W::S, EN.vault_cancel) + 30.0;
        if hw + 10.0 + cancel_w <= iw {
            ui.button(
                x + hw + 10.0,
                cy,
                Some(cancel_w),
                36.0,
                EN.vault_cancel,
                Style::Ghost,
                Action::Cancel,
            );
        } else {
            cy += 44.0;
            ui.button(
                x,
                cy,
                Some(cancel_w),
                36.0,
                EN.vault_cancel,
                Style::Ghost,
                Action::Cancel,
            );
        }
        cy += 36.0;
        cy - y
    });
}

/// Save to a stick's popup (`docs/BACKUP-FLOW.md` §17.7): for a key,
/// the line that says what the stick gives whoever finds it, first; then
/// each form with its files' names, ticked or not; Save and Cancel.
pub(crate) fn stick_save_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(s) = app.vaults.stick_save.as_ref() else {
        return;
    };
    let any = s.files.iter().any(|f| f.ticked);
    let items: Vec<(&str, Style, Action)> = vec![
        (
            EN.vault_save,
            if any { Style::Primary } else { Style::Disabled },
            va(V::SaveToStickGo),
        ),
        (EN.vault_cancel, Style::Ghost, Action::Cancel),
    ];
    let body = |ui: &mut Ui, x: f32, y: f32, iw: f32| -> f32 {
        let mut cy = y;
        if s.seed {
            cy += ui.wrap(x, cy, iw, 13.0, W::S, WARN, EN.vault_anyone_who_finds) + 14.0;
        }
        for (k, f) in s.files.iter().enumerate() {
            let top = cy;
            ui.checkbox(x, cy + 11.0, f.ticked, true);
            ui.text_mid(x + 30.0, cy, 40.0, 14.0, W::S, TEXT, &f.label);
            cy += 34.0;
            for n in &f.names {
                let n = ui.fit(12.0, W::M, n, iw - 30.0);
                ui.text(x + 30.0, cy, 12.0, W::M, MUTED, &n);
                cy += 18.0;
            }
            cy += 6.0;
            ui.hit(x, top, iw, cy - top, va(V::SaveTick(k as u8)));
            ui.rule(x, cy, iw, INNER);
            cy += 6.0;
        }
        cy - y
    };
    let title = EN.vault_save_to_a_stick;
    if ui.compact {
        crate::compact::sheet(ui, w, h, &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += crate::compact::sheet_head(ui, x, cy, iw, app.medium.icon(), ACCENT, title);
            cy += body(ui, x, cy, iw) + 8.0;
            cy += crate::compact::buttons(ui, x, cy, iw, &items);
            cy - y
        });
        return;
    }
    let sw = 560.0;
    let (ix, iw) = (32.0, sw - 64.0);
    // Measured first, out of sight, for the sheet's height.
    ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
    let body_h = body(ui, 0.0, 0.0, iw);
    let buttons_h = crate::screens::wrap_buttons(ui, 0.0, 0.0, iw, 44.0, &items);
    ui.c.pop_clip();
    let sh = 30.0 + 30.0 + 22.0 + body_h + 12.0 + buttons_h + 24.0;
    let (x, y) = crate::screens::sheet_box(ui, w, h, sw, sh);
    let x = x + ix;
    ui.icon(x, y + 30.0, 30.0, app.medium.icon(), 18.0, ACCENT);
    ui.text_mid(x + 42.0, y + 30.0, 30.0, 20.0, W::S, TEXT, title);
    let mut cy = y + 82.0;
    cy += body(ui, x, cy, iw) + 12.0;
    crate::screens::wrap_buttons(ui, x, cy, iw, 44.0, &items);
}

/// The session's keys or wallets that are not yet in this vault, each
/// with Save.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn save_panel(app: &View, ui: &mut Ui, v: &vault::OpenVault, dx: f32, mut dy: f32, dw: f32) -> f32 {
    let keys = app.vaults.category == 0;
    ui.text(
        dx,
        dy,
        18.0,
        W::S,
        TEXT,
        if keys {
            EN.vault_save_a_key
        } else {
            EN.vault_save_a_wallet
        },
    );
    dy += 40.0;
    let mut any = false;
    if keys {
        let held: Vec<String> = app
            .vaults
            .records(v.id)
            .of(kind::KEY)
            .filter_map(|(_, r)| app.vault_key_fingerprint(r))
            .collect();
        for (k, key) in app.session.keys.all().enumerate() {
            let fp = fp_text(key.fingerprint);
            if held.contains(&fp) || !key.has_words() {
                continue;
            }
            any = true;
            ui.text_mid(dx, dy, 40.0, 14.0, W::M, TEXT, &fp);
            let label = if key.has_passphrase {
                fill(EN.vault_bip_39_passphrase_n, &[&key.label.to_string()])
            } else {
                key.label.clone()
            };
            if ui.compact {
                // The key and its name, then its buttons under them.
                let label = ui.fit(13.0, W::R, &label, dw - 100.0);
                ui.text_mid(dx + 100.0, dy, 40.0, 13.0, W::R, MUTED, &label);
                dy += 42.0;
                let mut items = vec![(EN.vault_save, Style::Secondary, va(V::SaveKey(k)))];
                if key.has_passphrase {
                    items.push((
                        EN.vault_save_passphrase_button,
                        Style::Secondary,
                        va(V::SaveKeyWithPassphrase(k)),
                    ));
                }
                dy += crate::screens::wrap_buttons(ui, dx, dy, dw, 36.0, &items);
                ui.rule(dx, dy - 2.0, dw, INNER);
                dy += 6.0;
                continue;
            }
            let two = key.has_passphrase;
            let room = if two { dw - 110.0 } else { dw - 200.0 };
            let label = ui.fit(13.0, W::R, &label, room);
            ui.text_mid(dx + 100.0, dy, 40.0, 13.0, W::R, MUTED, &label);
            if two {
                dy += 40.0;
            }
            ui.button(
                dx + dw - 80.0,
                dy + 2.0,
                Some(80.0),
                36.0,
                EN.vault_save,
                Style::Secondary,
                va(V::SaveKey(k)),
            );
            if key.has_passphrase {
                let pw = ui.measure(13.0, W::S, EN.vault_save_passphrase_button) + 26.0;
                ui.button(
                    dx + dw - 88.0 - pw,
                    dy + 2.0,
                    Some(pw),
                    36.0,
                    EN.vault_save_passphrase_button,
                    Style::Secondary,
                    va(V::SaveKeyWithPassphrase(k)),
                );
            }
            ui.rule(dx, dy + 44.0, dw, INNER);
            dy += 48.0;
        }
        if !any {
            ui.text(dx, dy, 13.0, W::R, DIM, EN.vault_every_key_saved);
            dy += 30.0;
        }
    } else {
        let held: Vec<String> = app
            .described_of(v.id, kind::WALLET)
            .iter()
            .map(|(_, _, d)| d.policy.to_descriptor())
            .collect();
        for (k, wl) in app.session.wallets.iter().enumerate() {
            if held.contains(&wl.policy().to_descriptor()) {
                continue;
            }
            any = true;
            if ui.compact {
                // The name over its shape, Save beside them.
                let name = ui.fit(14.0, W::S, &wl.name, dw - 96.0);
                ui.text(dx, dy + 2.0, 14.0, W::S, TEXT, &name);
                let shape = ui.fit(12.0, W::R, &Session::shape(wl), dw - 96.0);
                ui.text(dx, dy + 24.0, 12.0, W::R, MUTED, &shape);
                ui.button(
                    dx + dw - 80.0,
                    dy + 4.0,
                    Some(80.0),
                    36.0,
                    EN.vault_save,
                    Style::Secondary,
                    va(V::SaveWallet(k)),
                );
                ui.rule(dx, dy + 48.0, dw, INNER);
                dy += 52.0;
                continue;
            }
            ui.text_mid(dx, dy, 40.0, 14.0, W::S, TEXT, &wl.name);
            let shape = ui.fit(12.0, W::R, &Session::shape(wl), dw - 260.0);
            ui.text_mid(dx + 150.0, dy, 40.0, 12.0, W::R, MUTED, &shape);
            ui.button(
                dx + dw - 80.0,
                dy + 2.0,
                Some(80.0),
                36.0,
                EN.vault_save,
                Style::Secondary,
                va(V::SaveWallet(k)),
            );
            ui.rule(dx, dy + 44.0, dw, INNER);
            dy += 48.0;
        }
        if !any {
            ui.text(dx, dy, 13.0, W::R, DIM, EN.vault_every_wallet);
            dy += 30.0;
        }
    }
    dy
}

/// Lock, asked for while a vault is open: what is sealed into the Outbox
/// and what is in the session but in no open vault. With no seed to seal
/// with, what is not sealed, and a lock that loses it.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn lock_ask(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let waits = app.seal_waits();
    let (icon, tone, sub, sealed_label, lock) = if waits {
        (
            Icon::Error,
            ERR,
            EN.seed_missing,
            EN.vault_not_sealed,
            (EN.vault_lock_without_sealing, Action::LockUnsealed),
        )
    } else {
        (
            Icon::Lock,
            ACCENT,
            EN.vault_lock_wipe_warning,
            EN.vault_sealed,
            (EN.vault_lock, Action::Lock),
        )
    };
    let (keys, wallets) = app.unsaved();
    let sealed: Vec<String> = app
        .vaults
        .open
        .iter()
        .filter(|v| v.changes > 0)
        .map(|v| v.label(app.vaults.store()))
        .collect();
    let rows: Vec<(&str, String)> = [
        (sealed_label, sealed.join(", ")),
        (
            EN.vault_keys_not_saved,
            if keys.is_empty() {
                String::new()
            } else {
                format!("{} · {}", keys.len(), keys.join(", "))
            },
        ),
        (
            EN.vault_wallets_not_saved,
            if wallets.is_empty() {
                String::new()
            } else {
                format!("{} · {}", wallets.len(), wallets.join(", "))
            },
        ),
        // Make a new wallet ends with this lock (`docs/history/NEW-WALLET.md`
        // §18).
        (
            EN.lock_held_label,
            if app.lock_held() && !app.idle.lock_for_flow() {
                EN.lock_held_create.to_string()
            } else {
                String::new()
            },
        ),
    ]
    .into_iter()
    .filter(|(_, v)| !v.is_empty())
    .collect();
    if ui.compact {
        let rows: Vec<(&str, String, faraday_draw::Color)> = rows
            .into_iter()
            .map(|(k, v)| {
                let tone = if waits && k == sealed_label {
                    ERR
                } else {
                    TEXT
                };
                (k, v, tone)
            })
            .collect();
        crate::compact::kv_sheet(
            ui,
            w,
            h,
            (icon, tone, EN.vault_lock),
            sub,
            &rows,
            &[
                (EN.vault_back_to_the_vault, Style::Secondary, Action::Cancel),
                (lock.0, Style::Primary, lock.1),
            ],
        );
        return;
    }
    let sh = 210.0 + rows.len() as f32 * 44.0;
    let (x, y) = crate::screens::sheet_box(ui, w, h, 560.0, sh);
    let ix = x + 32.0;
    let iw = 560.0 - 64.0;
    ui.icon(ix, y + 30.0, 30.0, icon, 18.0, tone);
    ui.text_mid(ix + 42.0, y + 30.0, 30.0, 20.0, W::S, TEXT, EN.vault_lock);
    let sub_tone = if waits { ERR } else { MUTED };
    ui.text(ix, y + 76.0, 13.0, W::R, sub_tone, sub);
    let mut ry = y + 106.0;
    for (k, v) in &rows {
        ui.text_mid(ix, ry, 40.0, 13.0, W::R, MUTED, k);
        let v = ui.fit(14.0, W::R, v, iw - 150.0);
        let fg = if waits && *k == sealed_label {
            ERR
        } else {
            TEXT
        };
        ui.text_mid(ix + 150.0, ry, 40.0, 14.0, W::R, fg, &v);
        ui.rule(ix, ry + 40.0, iw, INNER);
        ry += 44.0;
    }
    let by = y + sh - 32.0 - 46.0;
    let bw = (iw - 12.0) / 2.0;
    ui.button(
        ix,
        by,
        Some(bw),
        46.0,
        EN.vault_back_to_the_vault,
        Style::Secondary,
        Action::Cancel,
    );
    ui.button(
        ix + bw + 12.0,
        by,
        Some(bw),
        46.0,
        lock.0,
        Style::Primary,
        lock.1,
    );
}

/// The expiry choices for a GPG key, `chosen` highlighted. Returns the
/// row's height.
pub(crate) fn years_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    chosen: Option<u32>,
    act: fn(u32) -> V,
) -> f32 {
    let items: Vec<(&str, Style, Action)> = [
        (EN.vault_1_year, 1u32),
        (EN.vault_2_years, 2),
        (EN.vault_3_years, 3),
        (EN.vault_5_years, 5),
        (EN.vault_never, 0),
    ]
    .into_iter()
    .map(|(label, years)| {
        let style = if chosen == Some(years) {
            Style::Primary
        } else {
            Style::Secondary
        };
        (label, style, va(act(years)))
    })
    .collect();
    crate::screens::wrap_buttons(ui, x, y, w, 32.0, &items) - 8.0
}

/// GnuPG's reasons for a revocation, in its menu's order. Returns the
/// row's height.
fn reasons_row(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let items: Vec<(&str, Style, Action)> = [
        EN.vault_reason_0,
        EN.vault_reason_1,
        EN.vault_reason_2,
        EN.vault_reason_3,
    ]
    .into_iter()
    .enumerate()
    .map(|(n, label)| {
        let style = if usize::from(app.vaults.revoke_reason) == n {
            Style::Primary
        } else {
            Style::Secondary
        };
        (label, style, va(V::GpgReason(n as u8)))
    })
    .collect();
    crate::screens::wrap_buttons(ui, x, y, w, 32.0, &items) - 8.0
}

/// The Inbox's EFI images, each to sign or check with the selected db key;
/// or, once one is chosen to sign, its SHA-256 to compare with the build
/// record before the hold that signs it (`docs/history/PLAN.md` §8).
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn images_panel(app: &View, ui: &mut Ui, dx: f32, mut dy: f32, dw: f32) -> f32 {
    if let Some(k) = app.vaults.sb_sign
        && let Some(item) = app.inbox.get(k)
    {
        ui.text(
            dx,
            dy,
            18.0,
            W::S,
            TEXT,
            &fill(EN.vault_sign_n, &[&item.name.to_string()]),
        );
        dy += 40.0;
        ui.text(dx, dy, 12.0, W::R, MUTED, "SHA-256");
        dy += 20.0;
        // Taken when the image came in (`Item::sha`): a lookup.
        let hash = item
            .sha
            .map(|s| crate::secure_boot::image_sha256(&s))
            .unwrap_or_default();
        dy += ui.wrap(dx, dy, dw, 15.0, W::M, TEXT, &hash) + 14.0;
        dy += ui.wrap(dx, dy, dw, 13.0, W::R, MUTED, EN.vault_compare_build_record) + 16.0;
        let hw = hold_button(
            app,
            ui,
            dx,
            dy,
            EN.vault_hold_to_sign,
            va(V::SbHoldSign),
            ACCENT,
        );
        ui.button(
            dx + hw + 10.0,
            dy,
            None,
            36.0,
            EN.vault_back,
            Style::Ghost,
            va(V::SbImages),
        );
        return dy + 48.0;
    }
    ui.text(dx, dy, 18.0, W::S, TEXT, EN.vault_sign_or_check);
    dy += 40.0;
    let images: Vec<(usize, &crate::Item)> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| crate::secure_boot::is_image(&i.name))
        .collect();
    if images.is_empty() {
        ui.text(
            dx,
            dy,
            13.0,
            W::R,
            DIM,
            &fill(EN.vault_no_efi_image, &[app.medium.from_the()]),
        );
        dy += 30.0;
    }
    for (k, it) in images {
        // On a small panel the name has a line of its own.
        let room = if ui.compact { dw } else { dw - 200.0 };
        let name = ui.fit(14.0, W::M, &it.name, room);
        ui.text_mid(dx, dy, 40.0, 14.0, W::M, TEXT, &name);
        if ui.compact {
            dy += 36.0;
        }
        if app.selected_waits().is_some() {
            let bw = ui.measure(13.0, W::S, EN.backup_flow_first) + 30.0;
            ui.button(
                dx + dw - 168.0 - bw - 8.0,
                dy + 2.0,
                Some(bw),
                36.0,
                EN.backup_flow_first,
                Style::Secondary,
                back_up_first(crate::backup_flow::Kind::SecureBoot),
            );
        } else {
            ui.button(
                dx + dw - 80.0,
                dy + 2.0,
                Some(80.0),
                36.0,
                EN.vault_sign,
                Style::Secondary,
                va(V::SbPick(k)),
            );
        }
        ui.button(
            dx + dw - 168.0,
            dy + 2.0,
            Some(80.0),
            36.0,
            EN.vault_check,
            Style::Secondary,
            va(V::SbCheck(k)),
        );
        ui.rule(dx, dy + 44.0, dw, INNER);
        dy += 48.0;
    }
    if let Some(said) = &app.vaults.sb_checked {
        dy += 6.0;
        dy += ui.wrap(dx, dy, dw, 13.0, W::S, TEXT, said) + 10.0;
    }
    ui.button(
        dx,
        dy + 8.0,
        None,
        36.0,
        EN.vault_back_to_the_keys,
        Style::Ghost,
        va(V::SbImages),
    );
    dy + 56.0
}

/// Costs what `described_of` costs. Whether open vault `v` holds a
/// wallet with the same descriptor, as [`crate::wallet::same_wallet`]
/// compares them: from its wallet records as the vault keeps them read
/// ([`crate::vault::Vaults::described`]).
pub(crate) fn vault_has_wallet(c: &Compute, app: &Faraday, v: usize, w: &Wallet) -> bool {
    let want = crate::wallet::same_wallet(w.policy());
    app.vaults.open.get(v).is_some_and(|o| {
        app.vaults
            .described_of(c, o.id, kind::WALLET)
            .any(|(_, _, d)| d.same == want)
    })
}

/// Costs what `vault_key` costs. Whether open vault `v` holds session
/// key `key` ([`record_is_key`]).
pub(crate) fn vault_has_key(
    c: &Compute,
    app: &Faraday,
    v: usize,
    key: &faraday_keys::KeyInfo,
) -> bool {
    vault_key(c, app, v, key).is_some()
}

/// Costs what `record_is_key` costs. Whether open vault `v` holds
/// session key `key`, and if so whether its record keeps the BIP-39
/// passphrase: a record of the same master key, told by all 32 bytes of
/// its id, not by its fingerprint alone (F65).
pub(crate) fn vault_key(
    c: &Compute,
    app: &Faraday,
    v: usize,
    key: &faraday_keys::KeyInfo,
) -> Option<bool> {
    let o = app.vaults.open.get(v)?;
    app.vaults
        .records(o.id)
        .of(kind::KEY)
        .find(|(_, r)| record_is_key(c, app, *r, key))
        .map(|(_, r)| r.has(field::KEY_PASSPHRASE))
}

// ---------------------------------------------------------------------
// APK signing keys (`docs/APK-SIGNING.md` §4)
// ---------------------------------------------------------------------

/// Under the Make an APK key form's fields: what the name is, and, where
/// several vaults are open, which one the key goes to, none chosen in
/// advance. Returns its height.
/// A label above a value, wrapped, at (dx, dy). Returns the height.
fn apk_row(ui: &mut Ui, dx: f32, dy: f32, dw: f32, label: &str, value: &str, mono: bool) -> f32 {
    ui.text(dx, dy, 12.0, W::R, MUTED, label);
    let h = ui.wrap(
        dx,
        dy + 20.0,
        dw,
        14.0,
        if mono { W::M } else { W::R },
        TEXT,
        value,
    );
    20.0 + h + 14.0
}

/// Sign an APK: which APK, then the review; or which key, where Files
/// offered an APK several keys fit.
fn apk_sign_panel(
    app: &View,
    ui: &mut Ui,
    items: &[RecordView<'_>],
    sel: usize,
    dx: f32,
    mut dy: f32,
    dw: f32,
) -> f32 {
    let Some(&r) = items.get(sel) else {
        return dy;
    };
    if let Some(review) = &app.vaults.apk.review {
        return apk_review(app, ui, review, dx, dy, dw);
    }
    let package = r.public_text(field::APK_PACKAGE).unwrap_or("").to_string();
    if let Some(name) = &app.vaults.apk.which {
        ui.text(dx, dy, 18.0, W::S, TEXT, EN.apk_which_key);
        dy += 40.0;
        if let Some(info) = app.apk_info(name) {
            for (v, at, key_name, fp) in app.apk_keys_for(&info.package) {
                let label = format!("{key_name} · {}", &faraday_apk::hex(&fp)[..8]);
                let w = ui.button(
                    dx,
                    dy,
                    None,
                    40.0,
                    &label,
                    Style::Secondary,
                    va(V::ApkKeyPick(v, at)),
                );
                let _ = w;
                dy += 48.0;
            }
        }
        return dy;
    }
    ui.text(dx, dy, 18.0, W::S, TEXT, EN.apk_sign_an_apk);
    dy += 40.0;
    let listed = app.apk_listed(&package);
    if listed.is_empty() {
        let line = fill(EN.apk_copy_in_first, &[app.medium.a()]);
        dy += ui.wrap(dx, dy, dw, 13.0, W::R, DIM, &line) + 16.0;
    }
    for l in &listed {
        let room = if ui.compact { dw } else { dw - 110.0 };
        let line = format!("{} · {}", l.name, crate::screens::size_text(l.size));
        let line = ui.fit(14.0, W::M, &line, room);
        ui.text_mid(dx, dy, 40.0, 14.0, W::M, TEXT, &line);
        if ui.compact {
            dy += 36.0;
        }
        match &l.why {
            Some(why) => {
                let text = why.text();
                let text = ui.fit(12.0, W::S, &text, dw);
                if ui.compact {
                    ui.text(dx, dy, 12.0, W::S, WARN, &text);
                    dy += 24.0;
                } else {
                    let tw = ui.measure(12.0, W::S, &text);
                    ui.text_mid(dx + dw - tw, dy, 40.0, 12.0, W::S, WARN, &text);
                }
            }
            None => {
                ui.button(
                    dx + dw - 90.0,
                    dy + 2.0,
                    Some(90.0),
                    36.0,
                    EN.apk_sign,
                    Style::Secondary,
                    va(V::ApkPick(l.inbox)),
                );
            }
        }
        ui.rule(dx, dy + 44.0, dw, INNER);
        dy += 48.0;
    }
    ui.button(
        dx,
        dy + 8.0,
        None,
        36.0,
        EN.apk_back,
        Style::Ghost,
        va(V::ApkBack),
    );
    dy + 56.0
}

/// "Sign this APK?": what the file is, the second build typed, and the
/// hold, which does nothing until the second build matches.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn apk_review(
    app: &View,
    ui: &mut Ui,
    review: &crate::apk::Review,
    dx: f32,
    mut dy: f32,
    dw: f32,
) -> f32 {
    use crate::apk::Second;
    ui.text(dx, dy, 18.0, W::S, TEXT, EN.apk_review_title);
    dy += 40.0;
    let Some(item) = app.inbox.iter().find(|i| i.name == review.name) else {
        return dy;
    };
    let info = app.apk_info(&review.name);
    dy += apk_row(ui, dx, dy, dw, EN.apk_file, &item.name, false);
    dy += apk_row(
        ui,
        dx,
        dy,
        dw,
        EN.apk_size,
        &crate::screens::size_text(item.bytes.len()),
        false,
    );
    dy += apk_row(
        ui,
        dx,
        dy,
        dw,
        EN.apk_sha_256,
        &faraday_apk::key::grouped(&review.sha),
        true,
    );
    ui.text(dx, dy, 12.0, W::R, MUTED, EN.apk_second_build);
    dy += 20.0;
    let focused = app.vaults.focus == Some(Focus::Second);
    text_box(
        ui,
        dx,
        dy,
        dw,
        &app.vaults.apk.second,
        false,
        focused,
        va(V::ApkSecond),
    );
    dy += 48.0;
    let second = app.apk_second();
    let (line, color) = match second {
        Second::Empty => (EN.apk_enter_second.to_string(), MUTED),
        Second::TooShort => (EN.apk_hex_too_short.to_string(), WARN),
        Second::Differs => (EN.apk_does_not_match.to_string(), WARN),
        Second::Matches(n) => (
            format!(
                "{} · {}",
                EN.apk_matches,
                faraday_apk::key::grouped_prefix(&review.sha, n)
            ),
            OK,
        ),
    };
    dy += ui.wrap(dx, dy, dw, 13.0, W::S, color, &line) + 14.0;
    if let Some(info) = &info {
        dy += apk_row(ui, dx, dy, dw, EN.apk_package, &info.package, false);
        dy += apk_row(
            ui,
            dx,
            dy,
            dw,
            EN.apk_version,
            &crate::apk::version_text(info),
            false,
        );
        let android = fill(
            EN.apk_and_later,
            &[&crate::apk::android_release(info.min_sdk)],
        );
        dy += apk_row(ui, dx, dy, dw, EN.apk_android, &android, false);
    }
    if let Some((v, at)) = app.apk_find(&review.key)
        && let Some(rec) = app.vaults.records(app.vaults.open[v].id).get(at)
        && let Some(s) = crate::apk::shown(rec, app.tz_min())
    {
        let key = format!("{} · {}", s.name, &s.hex[..16]);
        dy += apk_row(ui, dx, dy, dw, EN.apk_key_row, &key, true);
    }
    dy += apk_row(
        ui,
        dx,
        dy,
        dw,
        EN.apk_will_write,
        &crate::apk::output_name(&item.name),
        false,
    );
    let hw = if matches!(second, Second::Matches(_)) {
        hold_button(
            app,
            ui,
            dx,
            dy,
            EN.apk_hold_to_sign,
            va(V::ApkHoldSign),
            ACCENT,
        )
    } else {
        ui.button(
            dx,
            dy,
            None,
            36.0,
            EN.apk_hold_to_sign,
            Style::Disabled,
            va(V::ApkSecond),
        )
    };
    ui.button(
        dx + hw + 10.0,
        dy,
        None,
        36.0,
        EN.apk_back,
        Style::Ghost,
        va(V::ApkBack),
    );
    dy + 56.0
}
