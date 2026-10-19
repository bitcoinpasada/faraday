//! The vault screens (`PLAN.md` §6.1, the prototype's Vaults, Unlock,
//! Create a vault and Vault contents): drawing only. What a press does is
//! `vaults.rs`.

use crate::strings::EN as FEN;
use opensigner_core::strings::EN;
use opensigner_core::strings::fill;
use osk_ui::widgets::Icon;

use crate::screens::{guide_text, section_label, title};
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::vaults::{
    self, CATEGORIES, CUSTOM, CUSTOM_MEMORY, CUSTOM_PASSES, Focus, HOLD_MS, MACHINES, PRESETS,
    SIZES, TextBox, VaultAction as V, cost_text, file_text, mib_text, size_text, vstep,
};
use crate::wallet::{Session, Wallet, fp_text};
use crate::{Action, Code, Faraday, Screen, flow};
use faraday_vault::records::{self, field, kind};
use faraday_vault::{Record, SLOT_SIZES, file_len};

fn va(a: V) -> Action {
    Action::Vault(a)
}

/// A waiting action: **Back up first**, which opens the flow of the key
/// shown on its step.
fn back_up_first(kind_: crate::backup_flow::Kind) -> Action {
    Action::Flow(crate::backup_flow_steps::FlowAction::BackUpFirst(kind_))
}

/// A file's size, for Sign a file's rows.
fn gpg_file_size(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        fill(FEN.vault_mb, &[&format!("{:.1}", bytes as f32 / 1e6)])
    } else if bytes >= 1000 {
        fill(FEN.vault_kb, &[&format!("{:.1}", bytes as f32 / 1e3)])
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
        ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
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
    let text = fill(FEN.vault_remove_the_first, &[medium.noun()]);
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
    shown: bool,
    focused: bool,
    action: Action,
    eye: Action,
) {
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
        ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
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
        FEN.vault_release_to_hide
    } else {
        FEN.vault_hold_to_show
    };
    ui.text(x + 14.0, y + bh - 24.0, 11.0, W::R, WARN, hint);
    ui.hit(x, y, w, bh, action);
    bh
}

/// Show SeedQR: the selected key's SeedQR shown only while held, behind
/// the same hold as its words. The code is the one the press made
/// ([`Faraday::vault_seedqr_press`]); drawing makes none. Returns the
/// height.
fn seedqr_box(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
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
        osk_ui::SizeClass::Small
    } else {
        osk_ui::SizeClass::Wide
    };
    let side = osk_ui::tokens::qr_side(class).min(w - 28.0);
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
            ui.text(x + 14.0, y + 10.0, 14.0, W::S, MUTED, FEN.vault_show_seedqr);
        }
    }
    let hint = if held {
        FEN.vault_release_to_hide
    } else {
        FEN.vault_hold_to_show
    };
    ui.text(x + 14.0, y + bh - 24.0, 11.0, W::R, WARN, hint);
    ui.hit(x, y, w, bh, action);
    bh
}

/// "Hold to delete from vault", filling while it is held.
fn hold_delete(app: &Faraday, ui: &mut Ui, x: f32, y: f32) -> f32 {
    hold_button(
        app,
        ui,
        x,
        y,
        FEN.vault_hold_to_delete,
        va(V::HoldDelete),
        ERR,
    )
}

/// A button that acts when held, filling while it is held.
pub(crate) fn hold_button(
    app: &Faraday,
    ui: &mut Ui,
    x: f32,
    y: f32,
    label: &str,
    action: Action,
    color: osk_ui::Color,
) -> f32 {
    hold_button_in(app, ui, x, y, f32::MAX, label, action, color)
}

/// [`hold_button`] no wider than `room`: a long label is cut to fit.
#[allow(clippy::too_many_arguments)]
pub(crate) fn hold_button_in(
    app: &Faraday,
    ui: &mut Ui,
    x: f32,
    y: f32,
    room: f32,
    label: &str,
    action: Action,
    color: osk_ui::Color,
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

pub(crate) fn list(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return list_compact(app, ui, x0, cw, h);
    }
    let x = x0 + 48.0;
    let w = cw - 96.0;
    let mut y = 36.0;
    // Made from another flow: the way back to it, and no second vault
    // started in its place (that would drop the way back).
    let back = app.vaults.back_to.map(|s| app.back_link_name(s));
    if let Some(label) = back {
        back_link(ui, x, 28.0, label, va(V::Back));
        y = 54.0;
    }
    title(ui, x, y, FEN.vault_vaults);
    if back.is_none() {
        let bw = ui.measure(15.0, W::S, FEN.vault_create_a_vault) + 32.0;
        ui.button(
            x + w - bw,
            y - 4.0,
            Some(bw),
            44.0,
            FEN.vault_create_a_vault,
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
            FEN.vault_a_vault_holds,
            &[app.medium.a(), app.medium.for_the()],
        ),
    );
    let files = app.vault_files();
    if files.is_empty() {
        ui.text(x, y + 8.0, 15.0, W::R, MUTED, FEN.vault_no_vault_in_files);
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
                v.label(),
                fill(
                    FEN.vault_open_unsaved,
                    &[
                        &v.changes.to_string(),
                        (if v.changes == 1 {
                            FEN.vault_change
                        } else {
                            FEN.vault_changes
                        }),
                    ],
                ),
                WARN,
                ACCENT,
                LINE,
            ),
            Some(v) => (v.label(), FEN.vault_open_2.to_string(), OK, ACCENT, LINE),
            None if !app.may_load_keys() => (
                remembered
                    .unwrap_or(if fresh {
                        FEN.vault_new_vault
                    } else {
                        FEN.vault_locked_vault
                    })
                    .to_string(),
                fill(FEN.vault_remove_the_to, &[app.medium.noun()]),
                WARN,
                MUTED,
                LINE,
            ),
            None if fresh => (
                remembered.unwrap_or(FEN.vault_new_vault).to_string(),
                FEN.vault_locked_not_written.to_string(),
                OK,
                OK,
                OK.with_alpha(90),
            ),
            None => (
                remembered.unwrap_or(FEN.vault_locked_vault).to_string(),
                FEN.vault_locked.to_string(),
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
                (FEN.vault_view_contents, Style::Secondary, va(V::Open(i))),
                (FEN.vault_lock, Style::Primary, Action::LockAsk),
            ]
        } else {
            &[(FEN.vault_unlock, Style::Primary, va(V::Open(i)))]
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

pub(crate) fn unlock(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return unlock_compact(app, ui, x0, cw, h);
    }
    let x = x0 + 48.0;
    let w = (cw - 96.0).min(760.0);
    let mut y = 28.0;
    let (back, to) = unlock_back(app);
    back_link(ui, x, y, back, to);
    y += 26.0;
    title(ui, x, y, FEN.vault_unlock);
    y += 56.0;
    if just_made_picked(app) {
        ui.text(x, y - 14.0, 14.0, W::R, MUTED, ONCE_MORE);
        y += 20.0;
    }
    y += guide_text(app, ui, x, y, w, FEN.vault_choose_the_vault);
    if !app.may_load_keys() {
        let pull = fill(FEN.vault_remove_the_then_type, &[app.medium.noun()]);
        y += stick_banner(ui, x, y, w, &pull);
    }
    let files = app.vault_files();
    let locked: Vec<(usize, &vaults::VaultFile)> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.open.is_none())
        .collect();
    if locked.is_empty() {
        ui.text(x, y, 15.0, W::R, MUTED, FEN.vault_no_locked_vault);
        ui.button(
            x,
            y + 36.0,
            None,
            40.0,
            FEN.vault_create_a_vault,
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
            FEN.vault_here_2,
            &[&app.vaults.time_text(mem, f.header.cost.passes).to_string()],
        );
        ui.text_right(x + w - 18.0, y + 10.0, 24.0, 13.0, W::S, TEXT, &cost);
        ui.text_right(x + w - 18.0, y + 36.0, 24.0, 12.0, W::R, MUTED, &time);
        ui.hit(x, y, w, ch, va(V::Pick(*i)));
        y += ch + 10.0;
    }
    y += 14.0;
    section_label(ui, x, y, FEN.vault_passphrase);
    y += 26.0;
    if app.may_load_keys() {
        let focused = app.vaults.focus == Some(Focus::Passphrase);
        secret_box(
            ui,
            x,
            y,
            w,
            &app.vaults.passphrase,
            app.vaults.typed_shown,
            focused,
            va(V::FocusPassphrase),
            va(V::ShowTyped),
        );
    } else {
        stick_field(app.medium, ui, x, y, w);
    }
    y += 56.0;
    let working = app.vaults.working == Some(vaults::Work::Unlock);
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
            &fill(FEN.vault_unlocking, &[&t.to_string()]),
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
            FEN.vault_unlock,
            style,
            va(V::Unlock),
        );
        let free = match app.vaults.memory_free_mib {
            Some(m) => fill(FEN.vault_this_computer_free, &[&mib_text(m).to_string()]),
            None => FEN.vault_this_computer_memory.to_string(),
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
fn list_line(app: &Faraday, f: &vaults::VaultFile, where_: &str) -> String {
    let line = if f.open.is_some() {
        format!(
            "{where_} · {} · {}",
            file_text(f.len),
            cost_text(&f.header.cost)
        )
    } else {
        match app.vault_summary(f) {
            Some(s) => s.line(app.tz_min),
            None => FEN.vault_unlock_to_see.to_string(),
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
const ONCE_MORE: &str = FEN.vault_type_the_passphrase;

/// Whether Unlock has the vault just made picked.
fn just_made_picked(app: &Faraday) -> bool {
    app.vaults.just_made.as_ref().is_some_and(|n| {
        app.vault_files()
            .get(app.vaults.pick)
            .is_some_and(|f| &f.name == n)
    })
}

/// Where Unlock's way back goes: the boot import's sheet when it was
/// opened from there, else the vault list.
pub(crate) fn unlock_back(app: &Faraday) -> (&'static str, Action) {
    match app.vaults.back_to {
        Some(Screen::Home) if app.import.is_some() => (FEN.vault_import, crate::boot_import::OPEN),
        // Unlocking for another flow: back to it.
        Some(s) => (app.back_link_name(s), va(V::Back)),
        None => (FEN.vault_vaults, Action::Nav(Screen::Vaults)),
    }
}

/// [`list`] on a small panel: Create a vault, then each vault as a card
/// with its buttons under what it says. The way back to a flow that made
/// one is the bar's.
fn list_compact(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    use crate::compact_screens::{about, finish};
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    if app.vaults.back_to.is_none() {
        ui.button(
            x,
            y,
            Some(w),
            44.0,
            FEN.vault_create_a_vault,
            Style::Primary,
            va(V::Create),
        );
        y += 58.0;
    }
    let files = app.vault_files();
    if files.is_empty() {
        y += ui.wrap(x, y, w, 14.0, W::R, MUTED, FEN.vault_no_vault_in_files) + 12.0;
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
                v.label(),
                fill(
                    FEN.vault_open_unsaved,
                    &[
                        &v.changes.to_string(),
                        (if v.changes == 1 {
                            FEN.vault_change
                        } else {
                            FEN.vault_changes
                        }),
                    ],
                ),
                WARN,
                ACCENT,
                LINE,
            ),
            Some(v) => (v.label(), FEN.vault_open_2.to_string(), OK, ACCENT, LINE),
            None if !app.may_load_keys() => (
                remembered
                    .unwrap_or(if fresh {
                        FEN.vault_new_vault
                    } else {
                        FEN.vault_locked_vault
                    })
                    .to_string(),
                fill(FEN.vault_remove_the_to, &[app.medium.noun()]),
                WARN,
                MUTED,
                LINE,
            ),
            None if fresh => (
                remembered.unwrap_or(FEN.vault_new_vault).to_string(),
                FEN.vault_locked_not_written.to_string(),
                OK,
                OK,
                OK.with_alpha(90),
            ),
            None => (
                remembered.unwrap_or(FEN.vault_locked_vault).to_string(),
                FEN.vault_locked.to_string(),
                MUTED,
                MUTED,
                LINE,
            ),
        };
        let buttons: &[(&str, Style, Action)] = if open.is_some() {
            &[
                (FEN.vault_view_contents, Style::Secondary, va(V::Open(i))),
                (FEN.vault_lock, Style::Primary, Action::LockAsk),
            ]
        } else {
            &[(FEN.vault_unlock, Style::Primary, va(V::Open(i)))]
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
            FEN.vault_a_vault_holds,
            &[app.medium.a(), app.medium.for_the()],
        ),
    );
    finish(app, ui, x0, cw, h, y - top + 16.0);
}

/// [`unlock`] on a small panel: the vaults, the passphrase, Unlock, each
/// the panel's width.
fn unlock_compact(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    use crate::compact_screens::{about, finish};
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    if just_made_picked(app) {
        y += ui.wrap(x, y, w, 14.0, W::R, MUTED, ONCE_MORE) + 12.0;
    }
    if !app.may_load_keys() {
        let pull = fill(FEN.vault_remove_the_then_type, &[app.medium.noun()]);
        y += stick_banner(ui, x, y, w, &pull);
    }
    let files = app.vault_files();
    let locked: Vec<(usize, &vaults::VaultFile)> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.open.is_none())
        .collect();
    if locked.is_empty() {
        y += ui.wrap(x, y, w, 14.0, W::R, MUTED, FEN.vault_no_locked_vault) + 12.0;
        ui.button(
            x,
            y,
            Some(w),
            40.0,
            FEN.vault_create_a_vault,
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
            FEN.vault_here,
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
    section_label(ui, x, y, FEN.vault_passphrase);
    y += 24.0;
    if app.may_load_keys() {
        let focused = app.vaults.focus == Some(Focus::Passphrase);
        secret_box(
            ui,
            x,
            y,
            w,
            &app.vaults.passphrase,
            app.vaults.typed_shown,
            focused,
            va(V::FocusPassphrase),
            va(V::ShowTyped),
        );
    } else {
        stick_field(app.medium, ui, x, y, w);
    }
    y += 52.0;
    if app.vaults.working == Some(vaults::Work::Unlock) {
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
            &fill(FEN.vault_unlocking, &[&t.to_string()]),
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
            FEN.vault_unlock,
            style,
            va(V::Unlock),
        ) {
            y += 52.0;
        }
        let free = match app.vaults.memory_free_mib {
            Some(m) => fill(FEN.vault_this_computer_free, &[&mib_text(m).to_string()]),
            None => FEN.vault_this_computer_memory.to_string(),
        };
        y += ui.wrap(x, y, w, 13.0, W::R, MUTED, &free) + 12.0;
    }
    if let Some(e) = &app.vaults.unlock_error {
        y += ui.wrap(x, y, w, 14.0, W::R, ERR, e) + 12.0;
    }
    y += about(ui, x, y, w, FEN.vault_choose_the_vault);
    finish(app, ui, x0, cw, h, y - top + 16.0);
}

fn back_link(ui: &mut Ui, x: f32, y: f32, label: &str, action: Action) {
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
const VSTEPS: [&str; vstep::COUNT] = [
    FEN.vault_where_will_you,
    FEN.vault_unlock_cost,
    FEN.vault_space_per_passphrase,
    FEN.vault_name_and_passphrases,
    FEN.vault_size,
];

pub(crate) fn create(app: &mut Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let panel_w = crate::screens::side_panel_w(cw);
    let Some(c) = app.vaults.create.as_ref() else {
        return;
    };
    let summaries = create_summaries(app);
    let then = app
        .vaults
        .back_to
        .map(|s| fill(FEN.vault_then, &[app.back_name(s)]));
    let steps = c.steps();
    let cards: Vec<flow::Card> = steps
        .iter()
        .map(|&k| flow::Card {
            title: VSTEPS[k as usize].to_string(),
            summary: if c.done[k as usize] {
                summaries[k as usize].clone()
            } else {
                FEN.vault_not_chosen_yet.to_string()
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
        back: Some(match app.vaults.back_to {
            Some(s) => (app.back_link_name(s), va(V::Back)),
            None if app.vaults.from_home => (FEN.vault_home, Action::Nav(Screen::Home)),
            None => (FEN.vault_vaults, Action::Nav(Screen::Vaults)),
        }),
        heading: FEN.vault_create_a_vault,
        guided: app.guided,
        switch: true,
        note: None,
        chip: then.as_deref(),
        chip_tap: None,
    };
    let scroll = c.scroll;
    let (next, again) = {
        let app_ref: &Faraday = app;
        let mut body = |ui: &mut Ui, i: usize, x: f32, y: f32, w: f32| -> f32 {
            create_body(app_ref, ui, steps[i], x, y, w)
        };
        flow::column(ui, &col, &cards, scroll, &mut body)
    };
    if let Some(c) = app.vaults.create.as_mut() {
        c.scroll = next;
    }
    if again {
        app.dirty = true;
        app.commands.push_back(osk_shell_api::Command::Draw);
    }
    if !compact {
        create_panel(app, ui, x0 + cw - panel_w, panel_w, h, &summaries);
    }
}

fn create_guide(k: u8) -> String {
    match k {
        vstep::WHERE => FEN.vault_a_vault_opens_only.to_string(),
        vstep::COST => FEN.vault_each_guess_at.to_string(),
        vstep::SIZE => FEN.vault_every_passphrase.to_string(),
        vstep::PRESET => FEN.vault_a_vault_opens.to_string(),
        _ => FEN.vault_each_passphrase.to_string(),
    }
}

/// The summary line of each step, by `vstep`, and of the panel's
/// choices.
fn create_summaries(app: &Faraday) -> [String; vstep::COUNT] {
    let Some(c) = app.vaults.create.as_ref() else {
        return Default::default();
    };
    let mut places = Vec::new();
    if !c.not_here {
        places.push(FEN.vault_this_computer.to_string());
    }
    for (k, m) in MACHINES.iter().enumerate() {
        if c.on[k] {
            places.push(m.1.to_string());
        }
    }
    let (k, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
    let name = if k == CUSTOM {
        FEN.vault_custom
    } else {
        PRESETS[k].0
    };
    let n = c.phrases.len();
    let size = match app.vaults.size_row() {
        Some(r) => fill(
            FEN.vault_passes_3,
            &[
                (SIZES[r].0),
                &mib_text(mem).to_string(),
                &passes.to_string(),
            ],
        ),
        None => fill(
            FEN.vault_passes_3,
            &[name, &mib_text(mem).to_string(), &passes.to_string()],
        ),
    };
    let ok = (0..n).all(|i| vaults::Vaults::phrase_ok(c, i));
    if places.is_empty() {
        places.push(FEN.vault_none_chosen.to_string());
    }
    [
        places.join(", "),
        fill(
            FEN.vault_passes_3,
            &[name, &mib_text(mem).to_string(), &passes.to_string()],
        ),
        fill(
            FEN.vault_file_n,
            &[
                &size_text(c.slot).to_string(),
                &file_text(file_len(c.slot)).to_string(),
            ],
        ),
        fill(
            FEN.vault_ofv,
            &[
                &vaults::vault_stem(&c.name.text).to_string(),
                &n.to_string(),
                (if n == 1 {
                    FEN.vault_passphrase_inline
                } else {
                    FEN.vault_passphrases
                }),
                (if ok {
                    FEN.vault_match
                } else {
                    FEN.vault_not_finished
                }),
            ],
        ),
        size,
    ]
}

fn create_body(app: &Faraday, ui: &mut Ui, n: u8, x: f32, y: f32, w: f32) -> f32 {
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
                    FEN.vault_free,
                    &[
                        &mib_text(app.vaults.here_limit()).to_string(),
                        &mib_text(free).to_string(),
                    ],
                ),
                None => FEN.vault_memory_not_reported.to_string(),
            };
            ui.checkbox(x, cy + 12.0, !c.not_here, true);
            ui.text_mid(
                x + 32.0,
                cy,
                40.0,
                14.0,
                W::S,
                TEXT,
                FEN.vault_this_computer,
            );
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
            if crate::screens::next_button(ui, x, cy, w, FEN.vault_continue, va(V::CNext(n))) {
                cy += 50.0;
            }
        }
        vstep::COST => {
            let (weakest, limit) = app.vaults.weakest();
            let suggested = app.vaults.suggested();
            let (current, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
            let choices = PRESETS.iter().copied().chain(std::iter::once((
                FEN.vault_custom,
                c.memory,
                c.passes,
            )));
            for (k, (label, m, p)) in choices.enumerate() {
                let on = k == current;
                let fits = m <= limit;
                let (note, color) = if k == CUSTOM {
                    (FEN.vault_choose_memory.to_string(), DIM)
                } else if k == suggested {
                    (FEN.vault_suggested.to_string(), ACCENT)
                } else if fits {
                    (FEN.vault_fits.to_string(), DIM)
                } else {
                    (fill(FEN.vault_too_large_for, &[&weakest.to_string()]), WARN)
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
                    FEN.vault_passes_bits,
                    &[
                        &mib_text(m).to_string(),
                        &p.to_string(),
                        &format!("{:.1}", vaults::cost_bits(m, p)),
                    ],
                );
                let detail = ui.fit(12.0, W::M, &detail, w - 32.0);
                ui.text(x + 16.0, cy + 29.0, 12.0, W::M, MUTED, &detail);
                ui.hit(x, cy, w, 50.0, va(V::CPreset(k)));
                cy += 56.0;
            }
            if current == CUSTOM {
                cy += 4.0;
                ui.text(x, cy, 12.0, W::R, MUTED, FEN.vault_memory);
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
                ui.text(x, cy, 12.0, W::R, MUTED, FEN.vault_passes_2);
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
            let needs = fill(FEN.vault_unlock_memory_n, &[&mib_text(mem).to_string()]);
            ui.text(x, cy, 13.0, W::S, TEXT, &needs);
            let room = w - ui.measure(13.0, W::S, &needs) - 12.0;
            let has = ui.fit(
                12.0,
                W::R,
                &fill(
                    FEN.vault_up_to,
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
                    FEN.vault_unlocks_in_here,
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
                    FEN.vault_adds_bits_to,
                    &[&format!("{:.1}", vaults::cost_bits(mem, passes))],
                ),
            );
            cy += 30.0;
            if crate::screens::next_button(ui, x, cy, w, FEN.vault_continue, va(V::CNext(n))) {
                cy += 50.0;
            }
        }
        vstep::PRESET => {
            // Two sizes, each with its unlock time here and the memory it
            // needs; Customise opens the three cards they stand for.
            let chosen = app.vaults.size_row();
            let free = app.vaults.memory_free_mib;
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
                    FEN.vault_passes,
                    &[&mib_text(mem).to_string(), &passes.to_string()],
                );
                let time = match app.vaults.seconds(mem, passes) {
                    Some(_) => fill(
                        FEN.vault_unlocks_in_here,
                        &[&app.vaults.time_text(mem, passes).to_string()],
                    ),
                    None => FEN.vault_unlock_time_not.to_string(),
                };
                // Short of memory here: what this computer has free, in
                // place of the time.
                let short = free.filter(|f| mem + vaults::APP_MIB > *f);
                if ui.compact {
                    // Three lines: the name, the cost and its time here,
                    // the memory it needs.
                    let name = ui.fit(14.0, W::S, label, w - 28.0);
                    ui.text(x + 14.0, cy + 6.0, 14.0, W::S, TEXT, &name);
                    let here = match app.vaults.seconds(mem, passes) {
                        Some(_) => fill(
                            FEN.vault_here,
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
                        let has = fill(FEN.vault_this_computer_has, &[&mib_text(f).to_string()]);
                        let has = ui.fit(12.0, W::R, &has, w - 28.0);
                        ui.text(x + 14.0, cy + 46.0, 12.0, W::R, WARN, &has);
                    }
                } else {
                    ui.text(x + 14.0, cy + 8.0, 14.0, W::S, TEXT, label);
                    ui.text_right(x + w - 14.0, cy + 5.0, 24.0, 13.0, W::S, TEXT, &cost);
                    let line = match short {
                        Some(f) => fill(FEN.vault_this_computer_has, &[&mib_text(f).to_string()]),
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
                FEN.vault_customise,
                Style::Ghost,
                va(V::CCustomise),
            );
            cy += 50.0;
            if crate::screens::next_button(ui, x, cy, w, FEN.vault_continue, va(V::CNext(n))) {
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
                    FEN.vault_file_n_inline,
                    &[&file_text(file_len(*s)).to_string()],
                );
                let sw = ui.measure(12.0, W::R, &sub);
                ui.text(bx + (bw - sw) / 2.0, cy + 36.0, 12.0, W::R, MUTED, &sub);
                ui.hit(bx, cy, bw, 62.0, va(V::CSize(k)));
            }
            cy += 80.0;
            if crate::screens::next_button(ui, x, cy, w, FEN.vault_continue, va(V::CNext(n))) {
                cy += 50.0;
            }
        }
        _ => {
            let half = (w - 12.0) / 2.0;
            let may = app.may_load_keys();
            // Its name, which names the file: what it holds, since the
            // file is all anyone who sees the stick sees (`docs/VAULT.md`
            // §6).
            ui.text_mid(x, cy, 36.0, 13.0, W::S, TEXT, FEN.vault_name);
            cy += 34.0;
            cy += ui.wrap(
                x,
                cy,
                w,
                12.0,
                W::R,
                MUTED,
                &fill(FEN.vault_what_it_holds, &[app.medium.noun()]),
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
                FEN.vault_file_ofv,
                &[&vaults::vault_stem(&c.name.text).to_string()],
            );
            if nw + 16.0 + ui.measure(13.0, W::R, &file) <= w {
                ui.text_mid(x + nw + 16.0, cy, 40.0, 13.0, W::R, MUTED, &file);
                cy += 56.0;
            } else {
                ui.text(x, cy + 48.0, 13.0, W::R, MUTED, &file);
                cy += 78.0;
            }
            if !may {
                let pull = fill(FEN.vault_remove_the_then, &[app.medium.noun()]);
                cy += stick_banner(ui, x, cy, w, &pull);
            }
            ui.button(
                x,
                cy,
                None,
                32.0,
                if c.shown {
                    FEN.vault_hide_passphrases
                } else {
                    FEN.vault_show_passphrases
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
                    &fill(FEN.vault_passphrase_n, &[&(i + 1).to_string()]),
                );
                if i > 0 {
                    ui.button(
                        x + w - (ui.measure(13.0, W::S, FEN.vault_remove) + 28.0),
                        cy,
                        None,
                        36.0,
                        FEN.vault_remove,
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
                    FEN.vault_generate_with,
                    dice_style,
                    va(V::Dice(i)),
                );
                // Beside the button; on a small panel under it.
                if ui.compact {
                    cy += 46.0;
                    ui.text(x, cy, 13.0, W::R, MUTED, FEN.vault_or_type_your);
                    cy += 26.0;
                } else {
                    ui.text_mid(
                        x + dw + 14.0,
                        cy,
                        40.0,
                        13.0,
                        W::R,
                        MUTED,
                        FEN.vault_or_type_your,
                    );
                    cy += 52.0;
                }
                if may {
                    let fa = app.vaults.focus == Some(Focus::Phrase(i, false));
                    let fb = app.vaults.focus == Some(Focus::Phrase(i, true));
                    secret_box(
                        ui,
                        x,
                        cy,
                        half,
                        a,
                        c.shown,
                        fa,
                        va(V::CFocus(i, false)),
                        va(V::CShow),
                    );
                    secret_box(
                        ui,
                        x + half + 12.0,
                        cy,
                        half,
                        b,
                        c.shown,
                        fb,
                        va(V::CFocus(i, true)),
                        va(V::CShow),
                    );
                } else {
                    stick_field(app.medium, ui, x, cy, half);
                    stick_field(app.medium, ui, x + half + 12.0, cy, half);
                }
                cy += 46.0;
                let (status, color) = if !b.text.is_empty() && *a.text == *b.text {
                    (FEN.vault_matches, OK)
                } else if !b.text.is_empty() {
                    (FEN.vault_does_not_match, WARN)
                } else {
                    (FEN.vault_type_it_then, DIM)
                };
                ui.text(x, cy, 12.0, W::R, color, status);
                cy += 22.0;
                // Its strength: the dice's bits and the cost's, when the
                // dice made it.
                let (_, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
                let cost = vaults::cost_bits(mem, passes);
                let strength = match vaults::Vaults::phrase_bits(c, i) {
                    Some(own) => Some((
                        fill(
                            FEN.vault_bits_from_dice,
                            &[
                                &format!("{own:.1}"),
                                &format!("{cost:.1}"),
                                &format!("{:.1}", own + cost),
                            ],
                        ),
                        if own >= vaults::STRONG_BITS as f32 {
                            OK
                        } else {
                            MUTED
                        },
                    )),
                    None if !a.text.is_empty() => {
                        Some((fill(FEN.vault_typed_its_own, &[&format!("{cost:.1}")]), DIM))
                    }
                    None => None,
                };
                if let Some((line, tone)) = strength {
                    cy += ui.wrap(x, cy, w, 12.0, W::R, tone, &line);
                }
                // The demo's public passphrase stands in for the rolled
                // words, as New key's public seed does (`docs/DEMO.md` §5.2).
                if !app.makes_secrets()
                    && c.from_dice
                        .get(i)
                        .and_then(|d| d.as_ref())
                        .is_some_and(|(made, _)| {
                            made.as_str() == crate::demo::PASSPHRASE
                                && vaults::Vaults::phrase_bits(c, i).is_some()
                        })
                {
                    cy += 4.0;
                    cy += ui.wrap(x, cy, w, 13.0, W::S, WARN, FEN.vault_demo_uses_public);
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
                    FEN.vault_add_another_passphrase,
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
fn dice_panel(
    app: &Faraday,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    rolls: &TextBox,
    list: osk_bip::diceware::List,
) -> f32 {
    use osk_bip::diceware::List;
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
        &fill(
            FEN.vault_roll_dice_for,
            &[&list.dice_per_word().to_string()],
        ),
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
    cy += 50.0;
    let words = vaults::dice_words(&rolls.text, list);
    let per = list.dice_per_word();
    let left = rolls.text.len() % per;
    // The words as the rolls make them, each a pill that opens it in its
    // list, as New key shows its words.
    if words.is_empty() {
        ui.text(x, cy, 13.0, W::R, DIM, FEN.vault_no_word_yet);
        cy += 26.0;
    } else {
        ui.text(x, cy, 12.0, W::R, MUTED, crate::wordlist::PRESS_A_WORD);
        cy += 22.0;
        let place = crate::wordlist::WordList::Eff(list).place();
        let mut bx = x;
        for (word, i) in words
            .iter()
            .zip(vaults::dice_word_indices(&rolls.text, list))
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
        FEN.vault_bits_with_the,
        &[
            &words.len().to_string(),
            (if words.len() == 1 {
                FEN.vault_word
            } else {
                FEN.vault_words_inline
            }),
            &format!("{own:.1}"),
            &format!("{:.1}", own + vaults::cost_bits(mem, passes)),
            &(if left > 0 {
                format!(
                    " {}",
                    fill(
                        FEN.vault_dice_for_the_next_word,
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
        if bits >= vaults::STRONG_BITS {
            OK
        } else {
            MUTED
        },
        &status,
    ) + 8.0;
    // How long is long enough, and where the lists come from.
    cy += ui.wrap(x, cy, w, 12.0, W::R, MUTED, &vaults::dice_aim(list)) + 12.0;
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
        FEN.vault_use_these_words,
        style,
        va(V::DiceUse),
    );
    ui.button(
        x + uw + 8.0,
        cy,
        None,
        36.0,
        FEN.vault_cancel,
        Style::Ghost,
        va(V::DiceClose),
    );
    cy += 50.0;
    cy - y
}

fn create_panel(
    app: &Faraday,
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
    let mut y = 32.0;
    ui.text(x, y, 13.0, W::S, MUTED, FEN.vault_your_choices);
    y += 28.0;
    for (k, label) in [
        FEN.vault_opens_on,
        FEN.vault_unlock_cost,
        FEN.vault_space_per_passphrase,
        FEN.vault_name_and_passphrases,
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
            FEN.vault_not_chosen_yet.to_string()
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
fn create_summary(app: &Faraday, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let Some(c) = app.vaults.create.as_ref() else {
        return 0.0;
    };
    let mut y = y0;
    let (k, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
    let (weakest, limit) = app.vaults.weakest();
    let name = if k == CUSTOM {
        FEN.vault_custom
    } else {
        PRESETS[k].0
    };
    let mut warnings: Vec<(String, osk_ui::Color)> = Vec::new();
    if let Some(free) = app.vaults.memory_free_mib
        && mem + vaults::APP_MIB > free
    {
        warnings.push((
            fill(FEN.vault_this_computer_n, &[&mib_text(mem).to_string()]),
            ERR,
        ));
    }
    if mem > limit {
        warnings.push((
            fill(
                FEN.vault_needs_gives_a,
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
                FEN.vault_unlocking_takes,
                &[&app.vaults.time_text(mem, passes).to_string()],
            ),
            WARN,
        ));
    }
    if c.on[1..].iter().any(|on| *on) {
        warnings.push((FEN.vault_raspberry_pi.to_string(), MUTED));
    }
    y += 4.0;
    for (t, color) in &warnings {
        let th = ui.wrap(x + 12.0, y + 10.0, w - 24.0, 12.0, W::R, *color, t);
        ui.fill(x, y, w, th + 20.0, 8.0, color.with_alpha(26));
        ui.wrap(x + 12.0, y + 10.0, w - 24.0, 12.0, W::R, *color, t);
        y += th + 28.0;
    }
    y += 8.0;
    ui.text(x, y, 13.0, W::S, MUTED, FEN.vault_will_create);
    y += 26.0;
    let cost = vaults::cost_bits(mem, passes);
    // The weakest passphrase's strength, known when the dice made every
    // one.
    let own: Vec<Option<f32>> = (0..c.phrases.len())
        .map(|i| vaults::Vaults::phrase_bits(c, i))
        .collect();
    let strength = if c.phrases.iter().all(|(a, _)| a.text.is_empty()) {
        FEN.vault_no_passphrase.to_string()
    } else if let Some(min) = own
        .iter()
        .map(|b| b.map(|b| b + cost))
        .collect::<Option<Vec<f32>>>()
        .and_then(|v| v.into_iter().reduce(f32::min))
    {
        fill(FEN.vault_bits, &[&format!("{min:.1}")])
    } else {
        FEN.vault_not_measured.to_string()
    };
    let facts = [
        (
            FEN.vault_file,
            format!("{}.ofv", vaults::vault_stem(&c.name.text)),
        ),
        (FEN.vault_size, file_text(file_len(c.slot))),
        (FEN.vault_unlock_memory, mib_text(mem)),
        (FEN.vault_unlock_here, app.vaults.time_text(mem, passes)),
        (
            FEN.vault_unlock_cost_adds,
            fill(FEN.vault_bits, &[&format!("{cost:.1}")]),
        ),
        (
            if c.phrases.len() > 1 {
                FEN.vault_weakest_passphrase
            } else {
                FEN.vault_strength
            },
            strength,
        ),
        (FEN.vault_goes_to, app.medium.for_box().to_string()),
    ];
    for (k, v) in facts.iter() {
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
fn create_button(app: &Faraday, ui: &mut Ui, x: f32, by: f32, w: f32) -> f32 {
    let (_, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
    if app.vaults.working == Some(vaults::Work::Create) {
        ui.text_mid(
            x,
            by,
            46.0,
            15.0,
            W::S,
            ACCENT,
            &fill(
                FEN.vault_creating,
                &[&app.vaults.time_text(mem, passes).to_string()],
            ),
        );
    } else if !crate::screens::pin_button(
        ui,
        x,
        by,
        Some(w),
        46.0,
        FEN.vault_create_vault,
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
fn item_heading(app: &Faraday, r: &Record) -> (String, String, bool) {
    match r.kind {
        kind::KEY => {
            let fp = key_fingerprint(app, r).unwrap_or_else(|| FEN.vault_key.to_string());
            let words = records::words_of(r)
                .map(|m| fill(FEN.vault_words_n, &[&m.word_count().to_string()]))
                .unwrap_or_else(|| FEN.vault_master_seed.into());
            let loads = r
                .field(field::KEY_FLAGS)
                .is_some_and(|f| f.first().is_some_and(|b| b & records::LOAD_AT_UNLOCK != 0));
            let label = r.text(field::KEY_LABEL).unwrap_or("");
            let mut line = vec![words];
            if !label.is_empty() {
                line.push(label.to_string());
            }
            if loads {
                line.push(FEN.vault_chosen_at_unlock_inline.to_string());
            }
            (fp, line.join(" · "), true)
        }
        kind::WALLET => {
            let name = r
                .text(field::WALLET_NAME)
                .unwrap_or(FEN.vault_wallet)
                .to_string();
            (
                name,
                wallet_shape(r).unwrap_or_else(|| FEN.vault_wallet.into()),
                false,
            )
        }
        kind::ENTRY => {
            let mut parts = Vec::new();
            if r.field(field::PASSWORD).is_some() {
                parts.push(FEN.vault_password);
            }
            if r.field(field::TOTP).is_some() {
                parts.push(FEN.vault_totp);
            }
            if parts.is_empty() {
                parts.push(FEN.vault_entry);
            }
            (
                r.text(field::TITLE).unwrap_or(FEN.vault_entry).to_string(),
                parts.join(" · "),
                false,
            )
        }
        kind::NOTE => {
            let t = r.text(field::NOTE).unwrap_or("");
            let first: String = t.lines().next().unwrap_or("").chars().take(40).collect();
            let first = if first.is_empty() {
                FEN.vault_note.to_string()
            } else {
                first
            };
            (
                first,
                fill(FEN.vault_note_characters, &[&t.chars().count().to_string()]),
                false,
            )
        }
        kind::SHEET => {
            let (_, name, _) = sheet_parts(r);
            (
                if name.is_empty() {
                    FEN.vault_recovery_sheet.into()
                } else {
                    name.clone()
                },
                fill(FEN.vault_recovery_sheet_n, &[&name.to_string()]),
                false,
            )
        }
        kind::GPG => {
            let line = match crate::gpg::shown(r, app.tz_min) {
                Some(g) if g.expires == FEN.gpg_never => FEN.vault_ed25519_does.to_string(),
                Some(g) => fill(FEN.vault_ed25519_expires, &[&g.expires.to_string()]),
                None => "Ed25519".to_string(),
            };
            (
                r.text(5).unwrap_or(FEN.vault_gpg_key).to_string(),
                line,
                false,
            )
        }
        kind::SECURE_BOOT => (
            FEN.vault_owner_keys.to_string(),
            FEN.vault_pk_kek_db.to_string(),
            false,
        ),
        kind::WALLET_SHARE => {
            let n = |f: u8| {
                r.field(f)
                    .and_then(|b| b.first())
                    .map_or(String::new(), |b| b.to_string())
            };
            (
                fill(
                    FEN.backup_flow_vault_share,
                    &[
                        r.text(field::SHARE_WALLET_NAME).unwrap_or(FEN.vault_wallet),
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
                .field(field::PASSPHRASE_KEY)
                .and_then(|b| <[u8; 4]>::try_from(b).ok())
                .map(|b| fp_text(osk_bip::keys::Fingerprint(b)))
                .unwrap_or_default();
            (fill(FEN.plan_passphrase_of, &[&fp]), String::new(), true)
        }
        kind::APK_KEY => match crate::apk::shown(r, app.tz_min) {
            Some(s) => (s.name, s.package, false),
            None => (FEN.vault_record.to_string(), String::new(), false),
        },
        _ => (FEN.vault_record.to_string(), String::new(), false),
    }
}

fn sheet_parts(r: &Record) -> (String, String, String) {
    match r
        .field(field::SHEET)
        .and_then(|p| osk_backup::oskb::payload_of(osk_backup::oskb::KIND_SHEET, p))
    {
        Some(osk_backup::oskb::Opened::Sheet(s)) => (
            String::from_utf8_lossy(&s.descriptor).into_owned(),
            String::from_utf8_lossy(&s.name).into_owned(),
            String::from_utf8_lossy(&s.note).into_owned(),
        ),
        _ => Default::default(),
    }
}

pub(crate) fn wallet_shape(r: &Record) -> Option<String> {
    let text = r.text(field::WALLET)?;
    let policy = crate::wallet::read_wallet(text).ok()?;
    Some(Session::shape(&Wallet::new(
        String::new(),
        policy,
        String::new(),
        crate::wallet::Origin::Named,
    )))
}

/// A key record's fingerprint, worked out once per key and kept.
pub(crate) fn key_fingerprint(app: &Faraday, r: &Record) -> Option<String> {
    use osk_bip::bitcoin::hashes::{Hash, sha256};
    // By the payload and the passphrase stored with it: the same words
    // with a passphrase are another key.
    let payload = r.field(field::KEY)?;
    let mut engine = sha256::Hash::engine();
    osk_bip::bitcoin::hashes::HashEngine::input(&mut engine, payload);
    if let Some(p) = r.field(field::KEY_PASSPHRASE) {
        osk_bip::bitcoin::hashes::HashEngine::input(&mut engine, &[0]);
        osk_bip::bitcoin::hashes::HashEngine::input(&mut engine, p);
    }
    let id = sha256::Hash::from_engine(engine).to_byte_array();
    if let Some((_, fp)) = app
        .vaults
        .fingerprints
        .borrow()
        .iter()
        .find(|(k, _)| *k == id)
    {
        return Some(fp.clone());
    }
    let master = match records::words_of(r) {
        Some(m) => {
            let passphrase = r.text(field::KEY_PASSPHRASE).unwrap_or("");
            let seed = m.to_seed(passphrase.as_bytes()).ok()?;
            osk_bip::keys::MasterKey::from_seed(&seed, app.session.network())
        }
        None => {
            let seed = vaults::seed_of(r)?;
            let bytes = osk_crypto::SeedBytes::new(&seed)?;
            osk_bip::keys::MasterKey::from_seed_bytes(
                &osk_crypto::Secret::new(bytes),
                app.session.network(),
            )
        }
    };
    let fp = fp_text(master.fingerprint());
    app.vaults.fingerprints.borrow_mut().push((id, fp.clone()));
    Some(fp)
}

fn words_text(m: &osk_bip::bip39::Mnemonic) -> String {
    let lang = m.language();
    m.indices()
        .iter()
        .map(|i| lang.word(*i))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn contents(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return contents_compact(app, ui, x0, cw, h);
    }
    let x = x0 + 40.0;
    let w = cw - 80.0;
    let mut y = 28.0;
    back_link(ui, x, y, FEN.vault_vaults, Action::Nav(Screen::Vaults));
    y += 26.0;
    let Some(v) = app.vaults.open.get(app.vaults.current) else {
        title(ui, x, y, FEN.vault_vault_contents);
        ui.text(x, y + 56.0, 15.0, W::R, MUTED, FEN.vault_no_vault_is_open);
        if let Some((label, a)) = app.vault_way(Screen::VaultContents) {
            ui.button(x, y + 90.0, None, 44.0, &label, Style::Primary, a);
        }
        return;
    };
    title(ui, x, y, &v.label());
    let lw = ui.measure(28.0, W::S, &v.label());
    let state = if v.changes == 0 {
        FEN.vault_open_2.to_string()
    } else {
        fill(
            FEN.vault_open_unsaved,
            &[
                &v.changes.to_string(),
                (if v.changes == 1 {
                    FEN.vault_change
                } else {
                    FEN.vault_changes
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
    let lockw = ui.measure(14.0, W::S, FEN.vault_lock) + 36.0;
    ui.button(
        x + w - lockw,
        y - 2.0,
        Some(lockw),
        40.0,
        FEN.vault_lock,
        Style::Primary,
        Action::LockAsk,
    );
    let rw = ui.measure(13.0, W::S, FEN.vault_rename) + 28.0;
    ui.button(
        x + w - lockw - 8.0 - rw,
        y + 2.0,
        Some(rw),
        34.0,
        FEN.vault_rename,
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
            vx += ui.button(vx, y - 8.0, None, 28.0, &o.label(), style, va(V::Show(i))) + 6.0;
        }
    }
    y += 30.0;
    y += guide_text(
        app,
        ui,
        x,
        y,
        w,
        &fill(FEN.vault_secret_values, &[app.medium.noun()]),
    );

    // Three columns: kinds, items, the item. A wallet's chart takes the
    // room the other two give up (`docs/NEW-WALLET.md` §7.2).
    let chart_shown = app
        .vault_selected_index()
        .and_then(|i| v.contents.records.get(i))
        .is_some_and(|r| r.kind == kind::WALLET);
    let (kinds_w, items_w) = if chart_shown {
        (
            osk_ui::tokens::CHART_PANE_KINDS,
            osk_ui::tokens::CHART_PANE_ITEMS,
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
        add_rows(ui, x, ky, kinds_w, 38.0);
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
    let items: Vec<&Record> = v
        .contents
        .records
        .iter()
        .filter(|r| kinds.contains(&r.kind))
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
            extra.push((FEN.vault_scan_a_code.to_string(), V::ScanEntry));
            if entries_files > 0 {
                extra.push((
                    fill(FEN.vault_import_from_files, &[&entries_files.to_string()]),
                    V::ImportAllEntries,
                ));
            }
        }
        0 | 3 => {
            for k in &backups {
                extra.push((
                    fill(FEN.vault_import_n, &[&app.inbox[*k].name.to_string()]),
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
        let label = FEN.vault_totp_codes;
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
        let (t, line, mono) = item_heading(app, r);
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
        ui.text(ix, iy, 13.0, W::R, DIM, FEN.vault_none_in_this);
    }

    // The detail: a form, the session's keys or wallets to save, or the item.
    ui.fill(detail_x - 16.0, top - 8.0, 1.0, h - top - 16.0, 0.0, LINE);
    // The item scrolls when it is taller than the room under the head: a
    // wallet's chart may be.
    let gap = osk_ui::tokens::GAP;
    let view_top = top - gap;
    let clip = ui.rect(detail_x - gap, view_top, detail_w + 2.0 * gap, h - view_top);
    ui.c.push_clip(clip);
    let foot = detail(
        app,
        ui,
        v,
        &items,
        sel,
        detail_x,
        top - app.vault_item_offset,
        detail_w,
    );
    ui.c.pop_clip();
    let content = foot + app.vault_item_offset - top;
    let max = content - (h - top);
    ui.report_scroll_in(crate::ui::Slot::VaultItem, clip, max, app.vault_item_offset);
}

/// What an open vault with nothing in it says.
const NOTHING_YET: &str = FEN.vault_nothing_in_it;

/// What adds an item of kind `k`, as its category's Add button reads.
fn add_label(k: usize) -> Option<&'static str> {
    match k {
        0 => Some(FEN.vault_save_a_key),
        1 => Some(FEN.vault_save_a_wallet),
        2 => Some(FEN.vault_add_an_entry),
        3 => Some(FEN.vault_add_a_note),
        4 => Some(FEN.vault_make_a_key),
        5 => Some(FEN.vault_make_keys),
        6 => Some(FEN.apk_make_a_key),
        _ => None,
    }
}

/// The kinds the contents list: those the slot holds something of, and
/// the one shown when it was chosen; each with its place in `CATEGORIES`,
/// label and count.
fn listed_kinds(app: &Faraday, v: &vaults::OpenVault) -> Vec<(usize, &'static str, usize)> {
    CATEGORIES
        .iter()
        .enumerate()
        .map(|(k, (label, kinds))| {
            let n = v
                .contents
                .records
                .iter()
                .filter(|r| kinds.contains(&r.kind))
                .count();
            (k, *label, n)
        })
        .filter(|&(k, _, n)| n > 0 || (k == app.vaults.category && app.vaults.category_chosen))
        .collect()
}

/// Whether the contents show an empty vault's next steps: nothing in it,
/// and nothing chosen or under way.
fn shows_empty(app: &Faraday, v: &vaults::OpenVault) -> bool {
    let held = CATEGORIES
        .iter()
        .any(|(_, kinds)| v.contents.records.iter().any(|r| kinds.contains(&r.kind)));
    !held && !app.vaults.category_chosen && !contents_detail(app)
}

/// An empty vault's next steps: every way to add, then writing it out.
fn empty_buttons(app: &Faraday) -> Vec<(String, Style, Action)> {
    let mut out: Vec<(String, Style, Action)> = (0..CATEGORIES.len())
        .filter_map(|k| add_full(k).map(|a| (a.to_string(), Style::Secondary, va(V::AddKind(k)))))
        .collect();
    out.push((
        fill(FEN.vault_write_it_to, &[app.medium.a()]),
        Style::Secondary,
        va(V::WriteOut),
    ));
    out
}

/// What adds an item of kind `k` where no category says which: an empty
/// vault's buttons.
fn add_full(k: usize) -> Option<&'static str> {
    match k {
        0 => Some(FEN.vault_save_a_bitcoin),
        1 => Some(FEN.vault_save_a_wallet),
        2 => Some(FEN.vault_add_an_entry),
        3 => Some(FEN.vault_add_a_note),
        4 => Some(FEN.vault_make_a_gpg_key),
        5 => Some(FEN.vault_make_secure_boot),
        6 => Some(FEN.apk_make_a_key),
        _ => None,
    }
}

/// The same, short, for the add rows under the kinds.
fn add_short(k: usize) -> Option<&'static str> {
    match k {
        0 => Some(FEN.vault_bitcoin_key),
        1 => Some(FEN.vault_wallet),
        2 => Some(FEN.vault_entry),
        3 => Some(FEN.vault_note),
        4 => Some(FEN.vault_gpg_key),
        5 => Some(FEN.vault_secure_boot),
        6 => Some(FEN.apk_key_short),
        _ => None,
    }
}

/// A row for each kind to add at (x, y), w wide and `row` tall, the Add
/// icon in the accent colour before its label. Returns their height.
fn add_rows(ui: &mut Ui, x: f32, y: f32, w: f32, row: f32) -> f32 {
    let mut cy = y;
    for k in 0..CATEGORIES.len() {
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
fn contents_compact(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    use crate::compact_screens::{about, finish, row};
    use crate::screens::wrap_buttons;
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    let Some(v) = app.vaults.open.get(app.vaults.current) else {
        ui.text(x, y, 15.0, W::R, MUTED, FEN.vault_no_vault_is_open);
        if let Some((label, a)) = app.vault_way(Screen::VaultContents) {
            let label = ui.fit(15.0, W::S, &label, w - 24.0);
            ui.button(x, y + 32.0, Some(w), 44.0, &label, Style::Primary, a);
        }
        return;
    };
    let (cat_label, kinds) = CATEGORIES[app.vaults.category];
    let items: Vec<&Record> = v
        .contents
        .records
        .iter()
        .filter(|r| kinds.contains(&r.kind))
        .collect();
    let sel = app.vaults.item[app.vaults.category].min(items.len().saturating_sub(1));
    if contents_detail(app) {
        y = detail(app, ui, v, &items, sel, x, y, w);
        finish(app, ui, x0, cw, h, y - top + 16.0);
        return;
    }
    let label = ui.fit(18.0, W::S, &v.label(), w);
    ui.text(x, y, 18.0, W::S, TEXT, &label);
    y += 28.0;
    let state = if v.changes == 0 {
        FEN.vault_open_2.to_string()
    } else {
        fill(
            FEN.vault_open_unsaved,
            &[
                &v.changes.to_string(),
                (if v.changes == 1 {
                    FEN.vault_change
                } else {
                    FEN.vault_changes
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
        FEN.vault_rename,
        Style::Secondary,
        va(V::Rename),
    );
    ui.button(
        x + half + 8.0,
        y,
        Some(half),
        40.0,
        FEN.vault_lock,
        Style::Primary,
        Action::LockAsk,
    );
    y += 52.0;
    if app.vaults.open.len() > 1 {
        let labels: Vec<String> = app.vaults.open.iter().map(|o| o.label()).collect();
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
        y += add_rows(ui, x, y, w, 36.0) + 4.0;
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
            extra.push((FEN.vault_scan_a_code.to_string(), V::ScanEntry));
            if entries_files > 0 {
                extra.push((
                    fill(FEN.vault_import_from_files, &[&entries_files.to_string()]),
                    V::ImportAllEntries,
                ));
            }
        }
        0 | 3 => {
            for (k, i) in app.inbox.iter().enumerate() {
                if i.kind == crate::FileKind::Backup {
                    extra.push((
                        fill(FEN.vault_import_n, &[&i.name.to_string()]),
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
        let label = FEN.vault_totp_codes;
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
        let (t, line, _) = item_heading(app, r);
        y += row(ui, x, y, w, None, &t, &line, MUTED, Some(va(V::Item(k))));
    }
    if items.is_empty() {
        ui.text(x, y, 13.0, W::R, DIM, FEN.vault_none_in_this);
        y += 30.0;
    }
    y += about(
        ui,
        x,
        y,
        w,
        &fill(FEN.vault_secret_values, &[app.medium.noun()]),
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
pub(crate) fn codes_form(app: &Faraday, ui: &mut Ui, dx: f32, dy: f32, dw: f32) -> f32 {
    let top = dy;
    let mut dy = dy;
    let Some(form) = &app.vaults.form else {
        return 0.0;
    };
    ui.text(dx, dy, 18.0, W::S, TEXT, FEN.vault_add_an_entry);
    dy += 40.0;
    for (k, (_, label, b, masked)) in form.fields.iter().enumerate() {
        ui.text(dx, dy, 12.0, W::R, MUTED, label);
        dy += 20.0;
        let focused = app.vaults.focus == Some(Focus::Field(k));
        if vaults::multiline(form.kind, form.fields[k].0) {
            dy += text_area(ui, dx, dy, dw, b, focused, va(V::FocusField(k))) + 12.0;
        } else if *masked {
            secret_box(
                ui,
                dx,
                dy,
                dw,
                b,
                app.vaults.typed_shown,
                focused,
                va(V::FocusField(k)),
                va(V::ShowTyped),
            );
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
        FEN.vault_save,
        Style::Primary,
        va(V::FormSave),
    );
    ui.button(
        dx + 130.0,
        dy,
        Some(120.0),
        40.0,
        FEN.vault_cancel,
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
fn detail(
    app: &Faraday,
    ui: &mut Ui,
    v: &vaults::OpenVault,
    items: &[&Record],
    sel: usize,
    dx: f32,
    mut dy: f32,
    dw: f32,
) -> f32 {
    if let Some(form) = &app.vaults.form {
        let revoking = form.kind == kind::GPG && form.fields.first().is_some_and(|f| f.0 == 3);
        let heading = match (form.kind, form.record) {
            (kind::GPG, _) if revoking => FEN.vault_make_a_revocation,
            (kind::SLOT_LABEL, _) => FEN.vault_rename_the_vault,
            (kind::NOTE, None) => FEN.vault_add_a_note,
            (kind::NOTE, Some(_)) => FEN.vault_edit_the_note,
            (kind::GPG, _) => FEN.vault_make_a_key,
            (kind::SECURE_BOOT, _) => FEN.vault_make_secure_boot,
            (kind::APK_KEY, _) => FEN.apk_make_a_key,

            (_, None) => FEN.vault_add_an_entry,
            _ => FEN.vault_edit_the_entry,
        };
        ui.text(dx, dy, 18.0, W::S, TEXT, heading);
        dy += 40.0;
        for (k, (_, label, b, masked)) in form.fields.iter().enumerate() {
            ui.text(dx, dy, 12.0, W::R, MUTED, label);
            dy += 20.0;
            let focused = app.vaults.focus == Some(Focus::Field(k));
            if vaults::multiline(form.kind, form.fields[k].0) {
                dy += text_area(ui, dx, dy, dw, b, focused, va(V::FocusField(k))) + 12.0;
            } else {
                if *masked {
                    secret_box(
                        ui,
                        dx,
                        dy,
                        dw,
                        b,
                        app.vaults.typed_shown,
                        focused,
                        va(V::FocusField(k)),
                        va(V::ShowTyped),
                    );
                } else {
                    text_box(ui, dx, dy, dw, b, false, focused, va(V::FocusField(k)));
                }
                dy += 52.0;
            }
        }
        if revoking {
            ui.text(dx, dy, 12.0, W::R, MUTED, FEN.vault_reason);
            dy += 22.0;
            dy += reasons_row(app, ui, dx, dy, dw) + 16.0;
        } else if form.kind == kind::GPG {
            ui.text(dx, dy, 12.0, W::R, MUTED, FEN.vault_expires);
            dy += 22.0;
            dy += years_row(ui, dx, dy, dw, Some(app.vaults.gpg_years), V::GpgYears) + 16.0;
        }
        let save = match form.kind {
            kind::GPG if revoking => FEN.vault_make_the_revocation,
            kind::GPG => FEN.vault_make_the_key,
            kind::APK_KEY => FEN.apk_make_the_key,
            kind::SECURE_BOOT => FEN.vault_make_the_keys,
            _ => FEN.vault_save,
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
            FEN.vault_cancel,
            Style::Ghost,
            va(V::FormCancel),
        );
        return dy;
    }
    if let Some(p) = &app.vaults.prompt {
        let heading = match p.purpose {
            vaults::Purpose::Backup(k) => fill(
                FEN.vault_open_3,
                &[(app
                    .inbox
                    .get(k)
                    .map_or(FEN.vault_the_backup, |i| i.name.as_str()))],
            ),
            vaults::Purpose::KeyPassphrase => FEN.vault_load_with_a_bip.to_string(),
            vaults::Purpose::Kdbx => FEN.vault_export_for_keepass.to_string(),
        };
        let heading = ui.fit(18.0, W::S, &heading, dw);
        ui.text(dx, dy, 18.0, W::S, TEXT, &heading);
        dy += 40.0;
        ui.text(dx, dy, 12.0, W::R, MUTED, FEN.vault_passphrase);
        dy += 20.0;
        let focused = app.vaults.focus == Some(Focus::Prompt);
        secret_box(
            ui,
            dx,
            dy,
            dw,
            &p.text,
            app.vaults.typed_shown,
            focused,
            va(V::FocusPrompt),
            va(V::ShowTyped),
        );
        dy += 56.0;
        if app.vaults.working == Some(vaults::Work::Backup) {
            ui.text_mid(dx, dy, 40.0, 15.0, W::S, ACCENT, FEN.vault_opening);
        } else if app.vaults.working == Some(vaults::Work::Kdbx) {
            ui.text_mid(dx, dy, 40.0, 15.0, W::S, ACCENT, FEN.vault_sealing);
        } else {
            let label = match p.purpose {
                vaults::Purpose::Backup(_) => FEN.vault_open_2,
                vaults::Purpose::Kdbx => FEN.vault_seal_and_export,
                vaults::Purpose::KeyPassphrase => FEN.vault_load,
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
                FEN.vault_cancel,
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
        ui.text(dx, dy, 18.0, W::S, TEXT, FEN.vault_sign_a_file);
        dy += 40.0;
        // The key that signs, named as GnuPG's pinentry names it: user
        // ID, key IDs and creation date, and which subkey signs.
        if let Some(g) = items
            .get(sel)
            .and_then(|r| crate::gpg::shown(r, app.tz_min))
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
                    FEN.vault_signing_key_ids,
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
                &fill(FEN.vault_no_file, &[app.medium.from_the()]),
            );
            dy += 30.0;
        } else {
            let all_ticked = files.iter().all(|f| app.gpg_ticked(f));
            ui.checkbox(dx, dy + 11.0, all_ticked, true);
            ui.text_mid(dx + 30.0, dy, 40.0, 14.0, W::M, TEXT, FEN.vault_select_all);
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
            // ticking and holding Sign is the way to make it. They follow
            // the file by its hash, not its place in the Inbox.
            let prefix: [u8; 8] = f.hash[..8].try_into().unwrap_or([0; 8]);
            let buttons = [
                (
                    FEN.vault_show_as_qr,
                    Action::ShowCode(Code::GpgSignature(prefix)),
                ),
                ("PNG", Action::CodePng(Code::GpgSignature(prefix))),
            ];
            let signed_w = if signed {
                ui.measure(12.0, W::S, FEN.vault_signed) + 18.0
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
                    FEN.vault_signed,
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
        let n = files.iter().filter(|f| app.gpg_ticked(f)).count();
        let label = match n {
            0 => FEN.vault_hold_to_sign.to_string(),
            1 => FEN.vault_hold_to_sign_1_file.to_string(),
            n => fill(FEN.vault_hold_to_sign_files, &[&n.to_string()]),
        };
        let w = if n == 0 {
            ui.button(
                dx,
                dy,
                None,
                36.0,
                &label,
                Style::Disabled,
                va(V::GpgHoldSign),
            )
        } else {
            hold_button(app, ui, dx, dy, &label, va(V::GpgHoldSign), ACCENT)
        };
        ui.button(
            dx + w + 10.0,
            dy,
            None,
            36.0,
            FEN.vault_back_to_the_key,
            Style::Ghost,
            va(V::GpgSignPick),
        );
        return dy + 48.0;
    }
    let Some(r) = items.get(sel) else {
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
    match r.kind {
        kind::KEY => {
            let fp = key_fingerprint(app, r).unwrap_or_default();
            field_row(ui, &mut dy, FEN.vault_fingerprint, &fp, true);
            // Read from the record once for the frame.
            let m = records::words_of(r);
            if let Some(m) = &m {
                hold_row(ui, &mut dy, FEN.vault_words, &words_text(m), true);
                if m.language() == osk_bip::bip39::Language::English {
                    ui.text(dx, dy, 12.0, W::R, MUTED, FEN.board_seedqr);
                    dy += 20.0;
                    dy += seedqr_box(app, ui, dx, dy, dw) + 14.0;
                }
            }
            let loads = r
                .field(field::KEY_FLAGS)
                .is_some_and(|f| f.first().is_some_and(|b| b & records::LOAD_AT_UNLOCK != 0));
            field_row(
                ui,
                &mut dy,
                FEN.vault_bip_39_passphrase,
                if r.field(field::KEY_PASSPHRASE).is_some() {
                    FEN.vault_stored
                } else {
                    FEN.vault_not_stored
                },
                false,
            );
            field_row(
                ui,
                &mut dy,
                FEN.vault_chosen_at_unlock,
                if loads { FEN.vault_yes } else { FEN.vault_no },
                false,
            );
            let here = app
                .session
                .keys
                .iter()
                .any(|k| fp_text(k.fingerprint()) == fp);
            field_row(
                ui,
                &mut dy,
                FEN.vault_in_this_session,
                if here {
                    FEN.vault_loaded
                } else {
                    FEN.vault_not_loaded
                },
                false,
            );
            if !here {
                actions.push((FEN.vault_load_into_session, va(V::Load)));
                if m.is_some() && r.field(field::KEY_PASSPHRASE).is_none() {
                    actions.push((FEN.vault_load_with_a_passphrase, va(V::LoadWithPassphrase)));
                }
            }
            actions.push((
                if loads {
                    FEN.vault_leave_unchosen
                } else {
                    FEN.vault_choose_at_unlock
                },
                va(V::ToggleLoad),
            ));
            if m.is_some() {
                actions.push((FEN.vault_save_to_a_stick, va(V::SaveToStick)));
            }
        }
        kind::WALLET => {
            // The wallet's chart the other way up (`docs/NEW-WALLET.md`
            // §7.2): where each seed is kept, its keys, the wallet.
            let up = crate::glance::Direction::BackupFirst;
            if let Some(g) = app
                .vault_selected_index()
                .and_then(|i| crate::glance::of_vault(app, app.vaults.current, i))
            {
                dy += if ui.compact {
                    crate::glance::draw_column(ui, &g, up, dx, dy, dw)
                } else {
                    crate::glance::draw(ui, &g, up, dx, dy, dw)
                } + osk_ui::tokens::PAD;
            }
            let text = r.text(field::WALLET).unwrap_or("");
            field_row(ui, &mut dy, FEN.vault_descriptor, text, true);
            actions.push((FEN.vault_open_in_wallets, va(V::Load)));
            actions.push((FEN.vault_show_as_qr, va(V::WalletQr)));
            actions.push((FEN.vault_save_to_a_stick, va(V::SaveToStick)));
        }
        kind::ENTRY => {
            if let Some(u) = r.text(field::USERNAME) {
                field_row(ui, &mut dy, FEN.vault_username, u, false);
            }
            if let Some(p) = r.text(field::PASSWORD) {
                hold_row(ui, &mut dy, FEN.vault_password, p, true);
            }
            if let Some(u) = r.text(field::URL) {
                field_row(ui, &mut dy, FEN.vault_url, u, true);
            }
            if let Some(t) = r.text(field::TOTP) {
                // A row before the secret: the code, live, or Set the
                // time while the clock is not checked (`docs/TOTP.md`
                // §6).
                ui.text(dx, dy, 12.0, W::R, MUTED, FEN.vault_code);
                dy += 20.0;
                match faraday_vault::totp::parse(t) {
                    Ok(code) if app.clock_checked() => {
                        let now = app.clock_now().unwrap_or(0);
                        let grouped = faraday_vault::totp::grouped(&code.code(now));
                        let left = code.left(now);
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
                    Ok(_) => {
                        ui.button(
                            dx,
                            dy,
                            None,
                            36.0,
                            FEN.vault_set_the_time,
                            Style::Secondary,
                            Action::OpenSetTime,
                        );
                        dy += 44.0;
                    }
                    Err(faraday_vault::totp::NotTotp::Hotp) => {
                        code_badge(ui, dx, dy, "HOTP");
                        dy += 36.0;
                    }
                    Err(faraday_vault::totp::NotTotp::Unreadable) => {
                        code_badge(ui, dx, dy, FEN.vault_unreadable);
                        dy += 36.0;
                    }
                }
                dy += 10.0;
                hold_row(ui, &mut dy, FEN.vault_totp_secret, t, true);
            }
            if let Some(n) = r.text(field::NOTES) {
                field_row(ui, &mut dy, FEN.vault_notes, n, false);
            }
            actions.push((FEN.vault_edit, va(V::Edit)));
            actions.push((FEN.vault_export_for_keepass, va(V::ExportKdbx)));
        }
        kind::NOTE => {
            hold_row(
                ui,
                &mut dy,
                FEN.vault_text,
                r.text(field::NOTE).unwrap_or(""),
                false,
            );
            actions.push((FEN.vault_edit, va(V::Edit)));
        }
        kind::SHEET => {
            let (d, _, note) = sheet_parts(r);
            field_row(ui, &mut dy, FEN.vault_descriptor, &d, true);
            if !note.is_empty() {
                field_row(ui, &mut dy, FEN.vault_note, &note, false);
            }
        }
        kind::GPG => {
            if let Some(g) = crate::gpg::shown(r, app.tz_min) {
                field_row(ui, &mut dy, FEN.vault_fingerprint, &g.fingerprint, true);
                field_row(ui, &mut dy, FEN.vault_signing_subkey, &g.subkey, true);
                for uid in &g.user_ids {
                    field_row(ui, &mut dy, FEN.vault_user_id, uid, false);
                }
                let valid = if g.expires == FEN.vault_never {
                    fill(FEN.vault_from_does_not, &[&g.created.to_string()])
                } else {
                    fill(
                        FEN.vault_to,
                        &[&g.created.to_string(), &g.expires.to_string()],
                    )
                };
                field_row(ui, &mut dy, FEN.vault_valid, &valid, false);
                if let Some(t) = &g.revoked {
                    field_row(
                        ui,
                        &mut dy,
                        FEN.vault_status,
                        &fill(FEN.vault_revoked_on, &[&t.to_string()]),
                        false,
                    );
                }
                // Where the revocation certificate is kept, as GnuPG says
                // where it stored it.
                let kept = match &g.revocation_kept {
                    Some(d) => fill(FEN.vault_kept_with_the_key, &[&d.to_string()]),
                    None => FEN.vault_none_kept.to_string(),
                };
                field_row(ui, &mut dy, FEN.vault_revocation_certificate, &kept, false);
                // The paperkey is made only while it is held.
                ui.text(dx, dy, 12.0, W::R, MUTED, FEN.vault_paperkey);
                dy += 20.0;
                dy += secret_with(
                    ui,
                    dx,
                    dy,
                    dw,
                    || crate::gpg::paperkey(r).unwrap_or_default(),
                    true,
                    0,
                ) + 14.0;
                // The term now, then the new term to choose: nothing is
                // renewed until one is chosen.
                let now_line = if g.expires == FEN.vault_never {
                    FEN.vault_term_never.to_string()
                } else {
                    fill(FEN.vault_term_until, &[&g.expires.to_string()])
                };
                ui.text(dx, dy, 12.0, W::R, MUTED, &now_line);
                dy += 22.0;
                ui.text(dx, dy, 12.0, W::R, MUTED, FEN.vault_renew_for);
                dy += 22.0;
                // Renew at the row's end; on a small panel under it.
                let rw = ui.measure(13.0, W::S, FEN.vault_renew) + 30.0;
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
                    let bw = ui.measure(13.0, W::S, FEN.backup_flow_first) + 30.0;
                    ui.button(
                        dx + dw - bw,
                        ry,
                        Some(bw),
                        32.0,
                        FEN.backup_flow_first,
                        Style::Secondary,
                        back_up_first(crate::backup_flow::Kind::Gpg),
                    );
                } else {
                    ui.button(
                        dx + dw - rw,
                        ry,
                        Some(rw),
                        32.0,
                        FEN.vault_renew,
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
            actions.push((FEN.vault_export_public, va(V::GpgExport)));
            if ui.compact {
                breaks.push(actions.len());
            }
            actions.push((FEN.vault_show_as_qr, Action::ShowCode(Code::GpgKey)));
            actions.push(("PNG", Action::CodePng(Code::GpgKey)));
            breaks.push(actions.len());
            actions.push((FEN.vault_export_revocation, va(V::GpgExportRevocation)));
            actions.push((FEN.vault_make_a_revocation, va(V::GpgRevocationPage)));
            breaks.push(actions.len());
            if app.selected_waits().is_some() {
                actions.push((
                    FEN.backup_flow_first,
                    back_up_first(crate::backup_flow::Kind::Gpg),
                ));
            } else {
                actions.push((FEN.vault_sign_a_file, va(V::GpgSignPick)));
            }
        }
        kind::SECURE_BOOT => {
            if let Some(sb) = crate::secureboot::shown(r) {
                field_row(ui, &mut dy, FEN.vault_owner_guid, &sb.owner, true);
                for (label, name) in ["PK", "KEK", "db"].iter().zip(&sb.names) {
                    field_row(ui, &mut dy, label, name, false);
                }
            }
            ui.text(dx, dy, 12.0, W::R, MUTED, FEN.vault_enrolment_adds);
            dy += 22.0;
            let own_items: Vec<(&str, Style, Action)> = [
                (FEN.vault_windows_compatible, false),
                (FEN.vault_own_keys_only, true),
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
                FEN.vault_your_pk_kek_and
            } else {
                FEN.vault_yours_with_microsoft
            };
            dy += ui.wrap(dx, dy, dw, 12.0, W::R, MUTED, line) + 14.0;
            dy += backup_row(app, ui, crate::backup_flow::Kind::SecureBoot, r, dx, dy, dw);
            if app.selected_waits().is_some() {
                actions.push((
                    FEN.backup_flow_first,
                    back_up_first(crate::backup_flow::Kind::SecureBoot),
                ));
            } else {
                actions.push((FEN.vault_enrolment_files, va(V::SbEnrol)));
            }
            actions.push((FEN.vault_sign_or_check, va(V::SbImages)));
        }
        kind::APK_KEY => {
            let fp = crate::apk::fingerprint_of(r);
            let waiting = fp.and_then(|fp| app.backup_waits(crate::backup_flow::Kind::Apk, &fp));
            // The page leads with the backup while the key waits for it.
            dy += backup_row(app, ui, crate::backup_flow::Kind::Apk, r, dx, dy, dw);
            if let Some(s) = crate::apk::shown(r, app.tz_min) {
                field_row(ui, &mut dy, FEN.apk_name, &s.name, false);
                field_row(ui, &mut dy, FEN.apk_package, &s.package, false);
                field_row(
                    ui,
                    &mut dy,
                    FEN.apk_certificate_sha_256,
                    &s.fingerprint,
                    true,
                );
                field_row(ui, &mut dy, FEN.apk_key, FEN.apk_rsa_3072, false);
                field_row(ui, &mut dy, FEN.apk_made, &s.made, false);
            }
            breaks.push(actions.len());
            actions.push((FEN.apk_certificate, va(V::ApkExport)));
            if ui.compact {
                breaks.push(actions.len());
            }
            actions.push((FEN.vault_show_as_qr, Action::ShowCode(Code::ApkCertificate)));
            actions.push(("PNG", Action::CodePng(Code::ApkCertificate)));
            breaks.push(actions.len());
            if waiting.is_none() {
                actions.push((FEN.apk_sign_an_apk, va(V::ApkSignOpen)));
            } else {
                actions.push((
                    FEN.backup_flow_first,
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
    if r.kind == kind::GPG {
        hold_button(
            app,
            ui,
            dx + dw_hold + 10.0,
            dy,
            FEN.vault_hold_to_mark_revoked,
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
    app: &Faraday,
    ui: &mut Ui,
    kind_: crate::backup_flow::Kind,
    r: &Record,
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
    ui.text(dx, dy, 12.0, W::R, MUTED, FEN.backup_flow_row);
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
            FEN.backup_flow_continue,
            Style::Secondary,
            Action::Flow(crate::backup_flow_steps::FlowAction::Resume),
        );
        h += 48.0;
    } else {
        h += 6.0;
    }
    h
}

/// Save to a stick's popup (`docs/BACKUP-FLOW.md` §19.7): for a key,
/// the line that says what the stick gives whoever finds it, first; then
/// each form with its files' names, ticked or not; Save and Cancel.
pub(crate) fn stick_save_sheet(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let Some(s) = app.vaults.stick_save.as_ref() else {
        return;
    };
    let any = s.files.iter().any(|f| f.ticked);
    let items: Vec<(&str, Style, Action)> = vec![
        (
            FEN.vault_save,
            if any { Style::Primary } else { Style::Disabled },
            va(V::SaveToStickGo),
        ),
        (FEN.vault_cancel, Style::Ghost, Action::Cancel),
    ];
    let body = |ui: &mut Ui, x: f32, y: f32, iw: f32| -> f32 {
        let mut cy = y;
        if s.seed {
            cy += ui.wrap(x, cy, iw, 13.0, W::S, WARN, FEN.vault_anyone_who_finds) + 14.0;
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
    let title = FEN.vault_save_to_a_stick;
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
    ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
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
fn save_panel(
    app: &Faraday,
    ui: &mut Ui,
    v: &vaults::OpenVault,
    dx: f32,
    mut dy: f32,
    dw: f32,
) -> f32 {
    let keys = app.vaults.category == 0;
    ui.text(
        dx,
        dy,
        18.0,
        W::S,
        TEXT,
        if keys {
            FEN.vault_save_a_key
        } else {
            FEN.vault_save_a_wallet
        },
    );
    dy += 40.0;
    let mut any = false;
    if keys {
        let held: Vec<String> = v
            .contents
            .of(kind::KEY)
            .filter_map(|(_, r)| key_fingerprint(app, r))
            .collect();
        for (k, key) in app.session.keys.iter().enumerate() {
            let fp = fp_text(key.fingerprint());
            if held.contains(&fp) || key.words.is_none() {
                continue;
            }
            any = true;
            ui.text_mid(dx, dy, 40.0, 14.0, W::M, TEXT, &fp);
            let label = if key.passphrase.is_some() {
                fill(FEN.vault_bip_39_passphrase_n, &[&key.label.to_string()])
            } else {
                key.label.clone()
            };
            if ui.compact {
                // The key and its name, then its buttons under them.
                let label = ui.fit(13.0, W::R, &label, dw - 100.0);
                ui.text_mid(dx + 100.0, dy, 40.0, 13.0, W::R, MUTED, &label);
                dy += 42.0;
                let mut items = vec![(FEN.vault_save, Style::Secondary, va(V::SaveKey(k)))];
                if key.passphrase.is_some() {
                    items.push((
                        FEN.vault_save_with_its,
                        Style::Secondary,
                        va(V::SaveKeyWithPassphrase(k)),
                    ));
                }
                dy += crate::screens::wrap_buttons(ui, dx, dy, dw, 36.0, &items);
                ui.rule(dx, dy - 2.0, dw, INNER);
                dy += 6.0;
                continue;
            }
            let two = key.passphrase.is_some();
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
                FEN.vault_save,
                Style::Secondary,
                va(V::SaveKey(k)),
            );
            if key.passphrase.is_some() {
                let pw = ui.measure(13.0, W::S, FEN.vault_save_with_its) + 26.0;
                ui.button(
                    dx + dw - 88.0 - pw,
                    dy + 2.0,
                    Some(pw),
                    36.0,
                    FEN.vault_save_with_its,
                    Style::Secondary,
                    va(V::SaveKeyWithPassphrase(k)),
                );
            }
            ui.rule(dx, dy + 44.0, dw, INNER);
            dy += 48.0;
        }
        if !any {
            ui.text(dx, dy, 13.0, W::R, DIM, FEN.vault_every_key_in);
            dy += 30.0;
        }
    } else {
        let held: Vec<String> = v
            .contents
            .of(kind::WALLET)
            .filter_map(|(_, r)| {
                r.text(field::WALLET)
                    .and_then(|t| crate::wallet::read_wallet(t).ok())
            })
            .map(|p| p.to_descriptor())
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
                    FEN.vault_save,
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
                FEN.vault_save,
                Style::Secondary,
                va(V::SaveWallet(k)),
            );
            ui.rule(dx, dy + 44.0, dw, INNER);
            dy += 48.0;
        }
        if !any {
            ui.text(dx, dy, 13.0, W::R, DIM, FEN.vault_every_wallet);
            dy += 30.0;
        }
    }
    dy
}

/// Lock, asked for while a vault is open: what is sealed into the Outbox
/// and what is in the session but in no open vault. With no seed to seal
/// with, what is not sealed, and a lock that loses it.
pub(crate) fn lock_ask(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let waits = app.seal_waits();
    let (icon, tone, sub, sealed_label, lock) = if waits {
        (
            Icon::Error,
            ERR,
            FEN.seed_missing,
            FEN.vault_not_sealed,
            (FEN.vault_lock_without_sealing, Action::LockUnsealed),
        )
    } else {
        (
            Icon::Lock,
            ACCENT,
            FEN.vault_everything_not,
            FEN.vault_sealed,
            (FEN.vault_lock, Action::Lock),
        )
    };
    let (keys, wallets) = app.unsaved();
    let sealed: Vec<String> = app
        .vaults
        .open
        .iter()
        .filter(|v| v.changes > 0)
        .map(|v| v.label())
        .collect();
    let rows: Vec<(&str, String)> = [
        (sealed_label, sealed.join(", ")),
        (
            FEN.vault_keys_not_saved,
            if keys.is_empty() {
                String::new()
            } else {
                format!("{} · {}", keys.len(), keys.join(", "))
            },
        ),
        (
            FEN.vault_wallets_not_saved,
            if wallets.is_empty() {
                String::new()
            } else {
                format!("{} · {}", wallets.len(), wallets.join(", "))
            },
        ),
    ]
    .into_iter()
    .filter(|(_, v)| !v.is_empty())
    .collect();
    if ui.compact {
        let rows: Vec<(&str, String, osk_ui::Color)> = rows
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
            (icon, tone, FEN.vault_lock),
            sub,
            &rows,
            &[
                (
                    FEN.vault_back_to_the_vault,
                    Style::Secondary,
                    Action::Cancel,
                ),
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
    ui.text_mid(ix + 42.0, y + 30.0, 30.0, 20.0, W::S, TEXT, FEN.vault_lock);
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
        FEN.vault_back_to_the_vault,
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
        (FEN.vault_1_year, 1u32),
        (FEN.vault_2_years, 2),
        (FEN.vault_3_years, 3),
        (FEN.vault_5_years, 5),
        (FEN.vault_never, 0),
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
fn reasons_row(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let items: Vec<(&str, Style, Action)> = [
        FEN.vault_reason_0,
        FEN.vault_reason_1,
        FEN.vault_reason_2,
        FEN.vault_reason_3,
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
/// record before the hold that signs it (`PLAN.md` §8).
fn images_panel(app: &Faraday, ui: &mut Ui, dx: f32, mut dy: f32, dw: f32) -> f32 {
    if let Some(k) = app.vaults.sb_sign
        && let Some(item) = app.inbox.get(k)
    {
        ui.text(
            dx,
            dy,
            18.0,
            W::S,
            TEXT,
            &fill(FEN.vault_sign_n, &[&item.name.to_string()]),
        );
        dy += 40.0;
        ui.text(dx, dy, 12.0, W::R, MUTED, "SHA-256");
        dy += 20.0;
        let hash = crate::secureboot::image_sha256(&item.bytes);
        dy += ui.wrap(dx, dy, dw, 15.0, W::M, TEXT, &hash) + 14.0;
        dy += ui.wrap(dx, dy, dw, 13.0, W::R, MUTED, FEN.vault_compare_it_with) + 16.0;
        let hw = hold_button(
            app,
            ui,
            dx,
            dy,
            FEN.vault_hold_to_sign,
            va(V::SbHoldSign),
            ACCENT,
        );
        ui.button(
            dx + hw + 10.0,
            dy,
            None,
            36.0,
            FEN.vault_back,
            Style::Ghost,
            va(V::SbImages),
        );
        return dy + 48.0;
    }
    ui.text(dx, dy, 18.0, W::S, TEXT, FEN.vault_sign_or_check);
    dy += 40.0;
    let images: Vec<(usize, &crate::Item)> = app
        .inbox
        .iter()
        .enumerate()
        .filter(|(_, i)| crate::secureboot::is_image(&i.name))
        .collect();
    if images.is_empty() {
        ui.text(
            dx,
            dy,
            13.0,
            W::R,
            DIM,
            &fill(FEN.vault_no_efi_image, &[app.medium.from_the()]),
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
            let bw = ui.measure(13.0, W::S, FEN.backup_flow_first) + 30.0;
            ui.button(
                dx + dw - 168.0 - bw - 8.0,
                dy + 2.0,
                Some(bw),
                36.0,
                FEN.backup_flow_first,
                Style::Secondary,
                back_up_first(crate::backup_flow::Kind::SecureBoot),
            );
        } else {
            ui.button(
                dx + dw - 80.0,
                dy + 2.0,
                Some(80.0),
                36.0,
                FEN.vault_sign,
                Style::Secondary,
                va(V::SbPick(k)),
            );
        }
        ui.button(
            dx + dw - 168.0,
            dy + 2.0,
            Some(80.0),
            36.0,
            FEN.vault_check,
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
        FEN.vault_back_to_the_keys,
        Style::Ghost,
        va(V::SbImages),
    );
    dy + 56.0
}

/// Whether open vault `v` holds a wallet with the same descriptor.
pub(crate) fn vault_has_wallet(app: &Faraday, v: usize, w: &Wallet) -> bool {
    let want = crate::wallet::same_wallet(w.policy());
    app.vaults.open.get(v).is_some_and(|o| {
        o.contents.of(kind::WALLET).any(|(_, r)| {
            r.text(field::WALLET)
                .and_then(|t| crate::wallet::read_wallet(t).ok())
                .is_some_and(|p| crate::wallet::same_wallet(&p) == want)
        })
    })
}

/// Whether open vault `v` holds the key with fingerprint `fp`.
pub(crate) fn vault_has_key(app: &Faraday, v: usize, fp: osk_bip::keys::Fingerprint) -> bool {
    vault_key(app, v, fp).is_some()
}

/// Whether open vault `v` holds the key with fingerprint `fp`, and if so
/// whether its record keeps the BIP-39 passphrase.
pub(crate) fn vault_key(app: &Faraday, v: usize, fp: osk_bip::keys::Fingerprint) -> Option<bool> {
    let want = fp_text(fp);
    let o = app.vaults.open.get(v)?;
    o.contents
        .of(kind::KEY)
        .find(|(_, r)| key_fingerprint(app, r).as_deref() == Some(want.as_str()))
        .map(|(_, r)| r.field(field::KEY_PASSPHRASE).is_some())
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
    app: &Faraday,
    ui: &mut Ui,
    items: &[&Record],
    sel: usize,
    dx: f32,
    mut dy: f32,
    dw: f32,
) -> f32 {
    let Some(r) = items.get(sel) else {
        return dy;
    };
    if let Some(review) = &app.vaults.apk.review {
        return apk_review(app, ui, review, dx, dy, dw);
    }
    let package = r.text(field::APK_PACKAGE).unwrap_or("").to_string();
    if let Some(name) = &app.vaults.apk.which {
        ui.text(dx, dy, 18.0, W::S, TEXT, FEN.apk_which_key);
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
    ui.text(dx, dy, 18.0, W::S, TEXT, FEN.apk_sign_an_apk);
    dy += 40.0;
    let listed = app.apk_listed(&package);
    if listed.is_empty() {
        let line = fill(FEN.apk_copy_in_first, &[app.medium.a()]);
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
                    FEN.apk_sign,
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
        FEN.apk_back,
        Style::Ghost,
        va(V::ApkBack),
    );
    dy + 56.0
}

/// "Sign this APK?": what the file is, the second build typed, and the
/// hold, which does nothing until the second build matches.
fn apk_review(
    app: &Faraday,
    ui: &mut Ui,
    review: &crate::apk::Review,
    dx: f32,
    mut dy: f32,
    dw: f32,
) -> f32 {
    use crate::apk::Second;
    ui.text(dx, dy, 18.0, W::S, TEXT, FEN.apk_sign_this);
    dy += 40.0;
    let Some(item) = app.inbox.iter().find(|i| i.name == review.name) else {
        return dy;
    };
    let info = app.apk_info(&review.name);
    dy += apk_row(ui, dx, dy, dw, FEN.apk_file, &item.name, false);
    dy += apk_row(
        ui,
        dx,
        dy,
        dw,
        FEN.apk_size,
        &crate::screens::size_text(item.bytes.len()),
        false,
    );
    dy += apk_row(
        ui,
        dx,
        dy,
        dw,
        FEN.apk_sha_256,
        &faraday_apk::key::grouped(&review.sha),
        true,
    );
    ui.text(dx, dy, 12.0, W::R, MUTED, FEN.apk_second_build);
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
        Second::Empty => (FEN.apk_enter_second.to_string(), MUTED),
        Second::TooShort => (FEN.apk_at_least_32.to_string(), WARN),
        Second::Differs => (FEN.apk_does_not_match.to_string(), WARN),
        Second::Matches(n) => (
            format!(
                "{} · {}",
                FEN.apk_matches,
                faraday_apk::key::grouped_prefix(&review.sha, n)
            ),
            OK,
        ),
    };
    dy += ui.wrap(dx, dy, dw, 13.0, W::S, color, &line) + 14.0;
    if let Some(info) = &info {
        dy += apk_row(ui, dx, dy, dw, FEN.apk_package, &info.package, false);
        dy += apk_row(
            ui,
            dx,
            dy,
            dw,
            FEN.apk_version,
            &crate::apk::version_text(info),
            false,
        );
        let android = fill(
            FEN.apk_and_later,
            &[&crate::apk::android_release(info.min_sdk)],
        );
        dy += apk_row(ui, dx, dy, dw, FEN.apk_android, &android, false);
    }
    if let Some((v, at)) = app.apk_find(&review.key)
        && let Some(rec) = app.vaults.open[v].contents.records.get(at)
        && let Some(s) = crate::apk::shown(rec, app.tz_min)
    {
        let key = format!("{} · {}", s.name, &s.hex[..16]);
        dy += apk_row(ui, dx, dy, dw, FEN.apk_key_row, &key, true);
    }
    dy += apk_row(
        ui,
        dx,
        dy,
        dw,
        FEN.apk_will_write,
        &crate::apk::output_name(&item.name),
        false,
    );
    let hw = if matches!(second, Second::Matches(_)) {
        hold_button(
            app,
            ui,
            dx,
            dy,
            FEN.apk_hold_to_sign,
            va(V::ApkHoldSign),
            ACCENT,
        )
    } else {
        ui.button(
            dx,
            dy,
            None,
            36.0,
            FEN.apk_hold_to_sign,
            Style::Disabled,
            va(V::ApkSecond),
        )
    };
    ui.button(
        dx + hw + 10.0,
        dy,
        None,
        36.0,
        FEN.apk_back,
        Style::Ghost,
        va(V::ApkBack),
    );
    dy + 56.0
}
