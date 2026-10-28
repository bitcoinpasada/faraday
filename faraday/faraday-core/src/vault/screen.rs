//! The vault screens (`docs/history/PLAN.md` §6.1, the prototype's Vaults, Unlock,
//! Create a vault and Vault contents): drawing only. What a press does is
//! `vault/mod.rs`.

use crate::strings::EN;
use crate::strings::fill;
use faraday_crypto::work::Compute;
use faraday_draw::tokens::{
    BODY_L, BODY_M, BODY_S, BUTTON_H, FIELD_H, FIELD_PAD, HEADLINE_M, LABEL_L, LABEL_M, TITLE_M,
};
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

mod save;
mod search;
mod secret;
pub(crate) use save::{gpg_sign_sheet, stick_save_sheet, years_row};
use save::{images_panel, reasons_row, save_panel};
pub(crate) use search::bar as search_bar;
use secret::{secret, secret_with, seedqr_box};

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
    ui.field_box((x, y, w), focused, None);
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
        x + FIELD_PAD,
        y,
        FIELD_H,
        BODY_L.mono().size,
        W::from(BODY_L.mono().face),
        Role::OnSurface,
        shown,
        w - 2.0 * FIELD_PAD,
        focused,
        action,
    );
    ui.hit(x, y, w, FIELD_H, action);
}

/// The warning that stands above a passphrase while a stick is attached:
/// nothing secret is typed until it is pulled. Returns its height.
pub(crate) fn stick_banner(ui: &mut Ui, x: f32, y: f32, w: f32, text: &str) -> f32 {
    let warn = Role::Warning;
    if ui.compact {
        // Wrapped beside its icon on a small panel.
        ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
        let th = ui.wrap_as((x + 40.0, y + 12.0, w - 52.0), LABEL_L, warn, text);
        ui.c.pop_clip();
        let bh = th + 24.0;
        ui.fill(x, y, w, bh, 10.0, Role::WarningContainer);
        ui.stroke(x, y, w, bh, 10.0, Role::Warning.with_alpha(110));
        ui.icon(
            x + 10.0,
            y + bh / 2.0 - 10.0,
            20.0,
            Icon::Warning,
            11.0,
            Role::OnWarningContainer,
        );
        ui.wrap_as(
            (x + 40.0, y + 12.0, w - 52.0),
            LABEL_L,
            Role::OnWarningContainer,
            text,
        );
        return bh + 12.0;
    }
    ui.fill(x, y, w, 48.0, 10.0, Role::WarningContainer);
    ui.stroke(x, y, w, 48.0, 10.0, Role::Warning.with_alpha(110));
    ui.icon(
        x + 14.0,
        y + 14.0,
        20.0,
        Icon::Warning,
        11.0,
        Role::OnWarningContainer,
    );
    ui.text_mid_as((x + 44.0, y, 48.0), LABEL_L, Role::OnWarningContainer, text);
    60.0
}

/// A passphrase field that takes no typing: a stick is attached.
pub(crate) fn stick_field(medium: crate::Medium, ui: &mut Ui, x: f32, y: f32, w: f32) {
    ui.field_box((x, y, w), false, None);
    let text = fill(EN.vault_remove_the_first, &[medium.noun()]);
    ui.text_mid_as(
        (x + FIELD_PAD, y, FIELD_H),
        BODY_L,
        Role::OnSurfaceVariant,
        &text,
    );
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
    let (muted, primary, inset) = (
        Role::OnSurfaceVariant,
        Role::Primary,
        Role::SurfaceContainerHighest,
    );
    // Practice's `faraday`: in its code box, no eye; a press in it focuses the
    // field.
    if show == Show::Code {
        crate::screens::code_field(ui, x, y, w, tokens::TEXT_FIELD, &b.text);
        ui.hit(x, y, w, tokens::TEXT_FIELD, action);
        return;
    }
    let shown = show == Show::Shown;
    text_box(ui, x, y, w, b, !shown, focused, action);
    // The eye sits inside the field's right end, over any long text.
    let ex = x + w - FIELD_H;
    let pressed = ui.is_pressed(eye);
    crate::ui::fill_pressed(
        ui,
        (ex + 8.0, y + 8.0, FIELD_H - 16.0, FIELD_H - 16.0, 6.0),
        inset,
        pressed,
    );
    ui.icon(
        ex,
        y,
        FIELD_H,
        if shown { Icon::EyeOff } else { Icon::Eye },
        15.0,
        if shown { primary } else { muted },
    );
    crate::compact::eye_hit(ui, (ex, y, FIELD_H, FIELD_H), eye, shown);
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
    let (ink, primary, rule) = (Role::OnSurface, Role::Primary, Role::OutlineVariant);
    let mut shown = b.text.clone();
    if focused {
        shown.push_str(ui.caret_char());
    }
    let th = ui.wrap_as((x + 12.0, y + 10.0, w - 24.0), BODY_M, ink, &shown);
    let h = (th + 20.0).max(96.0);
    ui.stroke(x, y, w, h, 8.0, if focused { primary } else { rule });
    ui.hit(x, y, w, h, action);
    h
}

/// "Hold to delete from vault", filling while it is held.
fn hold_delete(app: &View, ui: &mut Ui, x: f32, y: f32) -> f32 {
    let bad = Role::Error;
    hold_button(
        app,
        ui,
        x,
        y,
        EN.vault_hold_to_delete,
        va(V::HoldDelete),
        bad.paint(),
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
    color: crate::ui::Paint,
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
    color: crate::ui::Paint,
) -> f32 {
    let w = (ui.measure_as(LABEL_L, label) + 32.0).min(room);
    let label = ui.fit_as(LABEL_L, label, w - 32.0);
    let label = label.as_str();
    if ui.is_pressed(action) {
        let held = app
            .now_ms
            .saturating_sub(app.vaults.pressed_at)
            .min(HOLD_MS) as f32
            / HOLD_MS as f32;
        ui.fill(x, y, w * held, BUTTON_H, 10.0, color.with_alpha(60));
    }
    ui.stroke(x, y, w, BUTTON_H, 10.0, color.with_alpha(115));
    ui.text_mid_as((x + 16.0, y, BUTTON_H), LABEL_L, color, label);
    ui.hit(x, y, w, BUTTON_H, action);
    w
}

// ---------------------------------------------------------------------
// The list
// ---------------------------------------------------------------------

pub(crate) fn list(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let ink = Role::OnSurface;
    let muted = Role::OnSurfaceVariant;
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
        let bw = ui.measure_as(LABEL_L, EN.vault_create_a_vault) + 32.0;
        ui.button(
            x + w - bw,
            y - 4.0,
            Some(bw),
            BUTTON_H,
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
        ui.text_as(x, y + 8.0, BODY_M, muted, EN.vault_no_vault_in_files);
        if app.put_on().is_some() {
            crate::samples::put::offer(ui, Action::UseSample);
        }
        return;
    }
    for (i, f) in files.iter().enumerate() {
        let open = f.open.and_then(|o| app.vaults.open.get(o));
        let l = super::panes_screen::look(app, f);
        let (title_text, state) = (l.title, l.state);
        let (state_color, icon_color, edge) = (l.tone, l.icon, l.edge);
        let ch = 92.0;
        ui.fill(x, y, w, ch, 12.0, Role::SurfaceContainerLow);
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
        ui.text_as(x + 84.0, y + 16.0, TITLE_M, ink, &title_text);
        ui.text_as(x + 84.0, y + 42.0, BODY_S.mono(), muted, &f.name);
        let where_ = if f.in_outbox {
            app.medium.for_box()
        } else {
            app.medium.from_box()
        };
        let line = list_line(app, f, where_);
        let line = ui.fit_as(BODY_S, &line, w - 84.0 - 300.0);
        ui.text_as(x + 84.0, y + 62.0, BODY_S, muted, &line);
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
            let bw = ui.measure_as(LABEL_L, label) + 36.0;
            bx -= bw;
            ui.button(bx, y + 26.0, Some(bw), BUTTON_H, label, style, action);
            bx -= 8.0;
        }
        let sw = ui.measure_as(LABEL_L, &state);
        ui.text_mid_as(
            (bx - 12.0 - sw, y + 26.0, 40.0),
            LABEL_L,
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
    let (rule, bad) = (Role::OutlineVariant, Role::Error);
    let ink = Role::OnSurface;
    let muted = Role::OnSurfaceVariant;
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
        ui.text_as(x, y - 14.0, BODY_M, muted, ONCE_MORE);
        y += ui.line_as(BODY_M);
    }
    y += guide_text(app, ui, x, y, w, EN.vault_choose_the_vault);
    if !app.may_load_keys() {
        let pull = fill(EN.vault_remove_the_then_type, &[app.medium.noun()]);
        y += stick_banner(ui, x, y, w, &pull);
    }
    let files = app.vault_files();
    let beside = crate::panes::screen::shown(app, Screen::Unlock);
    let locked: Vec<(usize, &vault::VaultFile)> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.open.is_none())
        .collect();
    if locked.is_empty() {
        ui.text_as(x, y, BODY_M, muted, EN.vault_no_locked_vault);
        ui.button(
            x,
            y + 36.0,
            None,
            BUTTON_H,
            EN.vault_create_a_vault,
            Style::Secondary,
            va(V::Create),
        );
        return;
    }
    for (i, f) in locked.iter().filter(|_| !beside) {
        let on = *i == app.vaults.pick;
        let (ink, muted) = (on_selected(on, ink), on_selected(on, muted));
        let ch = 72.0;
        let fill_ = selected_or(on, Role::SurfaceContainerLow);
        ui.fill(x, y, w, ch, 12.0, fill_);
        ui.stroke(x, y, w, ch, 12.0, selected_or(on, rule));
        let fg = muted;
        ui.fill(x + 16.0, y + 14.0, 44.0, 44.0, 10.0, fg.with_alpha(30));
        ui.icon(x + 16.0, y + 14.0, 44.0, Icon::Lock, 18.0, fg);
        ui.text_as(x + 76.0, y + 14.0, BODY_M.mono(), ink, &f.name);
        let where_ = if f.in_outbox {
            app.medium.for_box()
        } else {
            app.medium.from_box()
        };
        ui.text_as(
            x + 76.0,
            y + 40.0,
            BODY_S,
            muted,
            &format!("{where_} · {}", file_text(f.len)),
        );
        let mem = f.header.cost.memory_kib / 1024;
        let cost = cost_text(&f.header.cost);
        let time = fill(
            EN.vault_unlock_time_here,
            &[&app.vaults.time_text(mem, f.header.cost.passes).to_string()],
        );
        ui.text_right_as((x + w - 18.0, y + 10.0, 24.0), LABEL_L, ink, &cost);
        ui.text_right_as((x + w - 18.0, y + 36.0, 24.0), BODY_S, muted, &time);
        ui.hit(x, y, w, ch, va(V::Pick(*i)));
        y += ch + 10.0;
    }
    y += 14.0;
    section_label(ui, x, y, EN.vault_passphrase);
    y += ui.advance_as(faraday_draw::tokens::TITLE_S, 26.0);
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
    y += FIELD_H + 16.0 + never_under(app, ui, x, y, w);
    let working = app.vaults.working == Some(vault::Work::Unlock);
    if working {
        let f = files.get(app.vaults.pick);
        let t = f
            .map(|f| {
                app.vaults
                    .time_text(f.header.cost.memory_kib / 1024, f.header.cost.passes)
            })
            .unwrap_or_default();
        let line = fill(EN.vault_unlocking, &[&t.to_string()]);
        crate::screens::working_line(ui, (x, y, w, 46.0), &line);
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
            BUTTON_H,
            EN.vault_unlock,
            style,
            va(V::Unlock),
        );
        let free = match app.vaults.memory_free_mib {
            Some(m) => fill(EN.vault_this_computer_free, &[&mib_text(m).to_string()]),
            None => EN.vault_this_computer_memory.to_string(),
        };
        ui.text_mid_as((x + 180.0, y, 46.0), BODY_S, muted, &free);
    }
    y += 60.0;
    if let Some(e) = &app.vaults.unlock_error {
        ui.wrap_as((x, y, w), BODY_M, bad, e);
    }
}

/// The third line of a vault's row: where it is, its size and cost while
/// open; once locked, what it held when last seen open, or that unlocking
/// shows it.
pub(super) fn list_line(app: &View, f: &vault::VaultFile, where_: &str) -> String {
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
    while n < parts.len() && ui.measure_as(BODY_S, &parts[..=n].join(" · ")) <= w {
        n += 1;
    }
    let first = ui.fit_as(BODY_S, &parts[..n].join(" · "), w);
    let second = ui.fit_as(BODY_S, &parts[n..].join(" · "), w);
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
fn list_compact(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let ink = Role::OnSurface;
    let muted = Role::OnSurfaceVariant;
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
            BUTTON_H,
            EN.vault_create_a_vault,
            Style::Primary,
            va(V::Create),
        );
        y += 58.0;
    }
    let files = app.vault_files();
    if files.is_empty() {
        y += ui.wrap_as((x, y, w), BODY_M, muted, EN.vault_no_vault_in_files) + 12.0;
        if app.put_on().is_some() {
            crate::samples::put::offer(ui, Action::UseSample);
        }
    }
    for (i, f) in files.iter().enumerate() {
        let open = f.open.and_then(|o| app.vaults.open.get(o));
        let l = super::panes_screen::look(app, f);
        let (title_text, state) = (l.title, l.state);
        let (state_color, icon_color, edge) = (l.tone, l.icon, l.edge);
        let buttons: &[(&str, Style, Action)] = if open.is_some() {
            &[
                (EN.vault_view_contents, Style::Secondary, va(V::Open(i))),
                (EN.vault_lock, Style::Primary, Action::LockAsk),
            ]
        } else {
            &[(EN.vault_unlock, Style::Primary, va(V::Open(i)))]
        };
        let ch = 112.0 + 40.0;
        ui.fill(x, y, w, ch, 12.0, Role::SurfaceContainerLow);
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
        let t = ui.fit_as(TITLE_M, &title_text, room);
        ui.text_as(x + 60.0, y + 10.0, TITLE_M, ink, &t);
        let name = ui.fit_as(BODY_S.mono(), &f.name, room);
        ui.text_as(x + 60.0, y + 34.0, BODY_S.mono(), muted, &name);
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
            && ui.measure_as(BODY_S, &line) > w - 24.0
        {
            let (first, second) = two_lines(ui, &line, w - 24.0);
            ui.text_as(x + 12.0, y + 58.0, BODY_S, muted, &first);
            ui.text_as(x + 12.0, y + 78.0, BODY_S, muted, &second);
        } else {
            let line = ui.fit_as(BODY_S, &line, w - 24.0);
            ui.text_as(x + 12.0, y + 58.0, BODY_S, muted, &line);
            let state = ui.fit_as(LABEL_L, &state, w - 24.0);
            ui.text_as(x + 12.0, y + 78.0, LABEL_L, state_color, &state);
        }
        // The buttons share the card's last row.
        let n = buttons.len() as f32;
        let bw = (w - 24.0 - 8.0 * (n - 1.0)) / n;
        for (k, &(label, style, action)) in buttons.iter().enumerate() {
            let bx = x + 12.0 + k as f32 * (bw + 8.0);
            ui.button(bx, y + 102.0, Some(bw), BUTTON_H, label, style, action);
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
    let (rule, bad) = (Role::OutlineVariant, Role::Error);
    let ink = Role::OnSurface;
    let muted = Role::OnSurfaceVariant;
    use crate::compact::M;
    use crate::compact::screen::{about, finish};
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    if just_made_picked(app) {
        y += ui.wrap_as((x, y, w), BODY_M, muted, ONCE_MORE) + 12.0;
    }
    if !app.may_load_keys() {
        let pull = fill(EN.vault_remove_the_then_type, &[app.medium.noun()]);
        y += stick_banner(ui, x, y, w, &pull);
    }
    let files = app.vault_files();
    let beside = crate::panes::screen::shown(app, Screen::Unlock);
    let locked: Vec<(usize, &vault::VaultFile)> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.open.is_none())
        .collect();
    if locked.is_empty() {
        y += ui.wrap_as((x, y, w), BODY_M, muted, EN.vault_no_locked_vault) + 12.0;
        ui.button(
            x,
            y,
            Some(w),
            BUTTON_H,
            EN.vault_create_a_vault,
            Style::Secondary,
            va(V::Create),
        );
        finish(app, ui, x0, cw, h, y - top + 56.0);
        return;
    }
    for (i, f) in locked.iter().filter(|_| !beside) {
        let on = *i == app.vaults.pick;
        let (ink, muted) = (on_selected(on, ink), on_selected(on, muted));
        let ch = 82.0;
        let fill_ = selected_or(on, Role::SurfaceContainerLow);
        ui.fill(x, y, w, ch, 12.0, fill_);
        ui.stroke(x, y, w, ch, 12.0, selected_or(on, rule));
        let fg = muted;
        ui.fill(x + 12.0, y + 12.0, 36.0, 36.0, 8.0, fg.with_alpha(30));
        ui.icon(x + 12.0, y + 12.0, 36.0, Icon::Lock, 15.0, fg);
        let room = w - 60.0 - 12.0;
        let name = ui.fit_as(BODY_M.mono(), &f.name, room);
        ui.text_as(x + 60.0, y + 10.0, BODY_M.mono(), ink, &name);
        let where_ = if f.in_outbox {
            app.medium.for_box()
        } else {
            app.medium.from_box()
        };
        let line = ui.fit_as(BODY_S, &format!("{where_} · {}", file_text(f.len)), room);
        ui.text_as(x + 60.0, y + 32.0, BODY_S, muted, &line);
        let mem = f.header.cost.memory_kib / 1024;
        let cost = fill(
            EN.vault_here,
            &[
                &cost_text(&f.header.cost).to_string(),
                &app.vaults.time_text(mem, f.header.cost.passes).to_string(),
            ],
        );
        let cost = ui.fit_as(LABEL_M, &cost, w - 24.0);
        ui.text_as(x + 12.0, y + 56.0, LABEL_M, ink, &cost);
        ui.hit(x, y, w, ch, va(V::Pick(*i)));
        y += ch + 10.0;
    }
    y += 6.0;
    section_label(ui, x, y, EN.vault_passphrase);
    y += ui.line_as(faraday_draw::tokens::TITLE_S) + faraday_draw::tokens::GAP_SMALL;
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
    y += FIELD_H + 12.0 + never_under(app, ui, x, y, w);
    if app.vaults.working == Some(vault::Work::Unlock) {
        let f = files.get(app.vaults.pick);
        let t = f
            .map(|f| {
                app.vaults
                    .time_text(f.header.cost.memory_kib / 1024, f.header.cost.passes)
            })
            .unwrap_or_default();
        let line = fill(EN.vault_unlocking, &[&t.to_string()]);
        crate::screens::working_line(ui, (x, y, w, 44.0), &line);
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
        y += ui.wrap_as((x, y, w), BODY_S, muted, &free) + 12.0;
    }
    if let Some(e) = &app.vaults.unlock_error {
        y += ui.wrap_as((x, y, w), BODY_M, bad, e) + 12.0;
    }
    y += about(ui, x, y, w, EN.vault_choose_the_vault);
    finish(app, ui, x0, cw, h, y - top + 16.0);
}

pub(crate) fn back_link(app: &View, ui: &mut Ui, x: f32, y: f32, back: (&str, Action)) {
    let muted = Role::OnSurfaceVariant;
    let (label, action) = app.named_back(back);
    ui.icon(x - 4.0, y, 18.0, Icon::ChevronLeft, 12.0, muted);
    ui.text_mid_as((x + 14.0, y, 18.0), BODY_S, muted, label);
    let lw = ui.measure_as(BODY_S, label) + 20.0;
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
            default: k == vstep::PRESET, // the size: closed at entry, with Change
            toggle: va(V::CStep(k)),
            guide: Some(create_guide(k)),
            learn: Some(crate::learn::Topic::Vaults),
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
    let good = Role::Ok;
    let (muted, ink, primary, warn, inset) = (
        Role::OnSurfaceVariant,
        Role::OnSurface,
        Role::Primary,
        Role::Warning,
        Role::SurfaceContainerHighest,
    );
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
            ui.text_mid_as((x + 32.0, cy, 40.0), LABEL_L, ink, EN.vault_this_computer);
            ui.text_right_as((x + w, cy, 40.0), BODY_S, muted, &here);
            ui.hit(x, cy, w, 40.0, va(V::CHere));
            ui.rule(x, cy + 42.0, w, Role::OutlineVariant);
            cy += 46.0;
            for (k, m) in MACHINES.iter().enumerate() {
                ui.checkbox(x, cy + 12.0, c.on[k], true);
                ui.text_mid_as((x + 32.0, cy, 40.0), LABEL_L, ink, m.0);
                ui.text_right_as((x + w, cy, 40.0), BODY_S, muted, m.2);
                ui.hit(x, cy, w, 40.0, va(V::CMachine(k)));
                ui.rule(x, cy + 42.0, w, Role::OutlineVariant);
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
                    (EN.vault_choose_memory.to_string(), muted)
                } else if k == suggested {
                    (EN.vault_suggested.to_string(), primary)
                } else if fits {
                    (EN.vault_fits.to_string(), muted)
                } else {
                    (fill(EN.vault_too_large, &[&weakest.to_string()]), warn)
                };
                let (ink, muted) = (on_selected(on, ink), on_selected(on, muted));
                let color = on_selected(on, color);
                ui.fill(x, cy, w, 50.0, 8.0, selected_or(on, inset));
                let edge = selected_or(on, Role::OutlineVariant);
                ui.stroke(x, cy, w, 50.0, 8.0, edge);
                // The name and what the row says of it, then the cost and
                // the bits it adds.
                ui.text_as(x + 16.0, cy + 8.0, LABEL_L, ink, label);
                let room = w - 44.0 - ui.measure_as(LABEL_L, label);
                let note = ui.fit_as(BODY_S, &note, room);
                ui.text_right_as((x + w - 16.0, cy + 5.0, 24.0), BODY_S, color, &note);
                let detail = fill(
                    EN.vault_passes_bits,
                    &[
                        &mib_text(m).to_string(),
                        &p.to_string(),
                        &format!("{:.1}", vault::cost_bits(m, p)),
                    ],
                );
                let detail = ui.fit_as(BODY_S.mono(), &detail, w - 32.0);
                ui.text_as(x + 16.0, cy + 29.0, BODY_S.mono(), muted, &detail);
                ui.hit(x, cy, w, 50.0, va(V::CPreset(k)));
                cy += 56.0;
            }
            if current == CUSTOM {
                cy += 4.0;
                ui.text_as(x, cy, BODY_S, muted, EN.vault_memory);
                cy += ui.advance_as(BODY_S, 22.0);
                let mut bx = x;
                for (i, m) in CUSTOM_MEMORY.iter().enumerate() {
                    let style = if *m == c.memory {
                        Style::Primary
                    } else {
                        Style::Secondary
                    };
                    bx += ui.button(
                        bx,
                        cy,
                        None,
                        BUTTON_H,
                        &mib_text(*m),
                        style,
                        va(V::CMemory(i)),
                    ) + 6.0;
                }
                cy += 44.0;
                ui.text_as(x, cy, BODY_S, muted, EN.vault_passes_label);
                cy += ui.advance_as(BODY_S, 22.0);
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
                        BUTTON_H,
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
            ui.text_as(x, cy, LABEL_L, ink, &needs);
            let room = w - ui.measure_as(LABEL_L, &needs) - 12.0;
            let has = ui.fit_as(
                BODY_S,
                &fill(
                    EN.vault_memory_limit,
                    &[&weakest.to_string(), &mib_text(limit).to_string()],
                ),
                room,
            );
            ui.text_right_as((x + w, cy - 4.0, 24.0), BODY_S, muted, &has);
            cy += 24.0;
            ui.fill(x, cy, w, 8.0, 4.0, inset);
            let share = (mem as f32 / limit.max(1) as f32).min(1.0);
            ui.fill(
                x,
                cy,
                w * share,
                8.0,
                4.0,
                if mem <= limit { primary } else { warn },
            );
            cy += 20.0;
            ui.text_as(
                x,
                cy,
                BODY_S,
                muted,
                &fill(
                    EN.vault_unlocks_in_here,
                    &[&app.vaults.time_text(mem, passes).to_string()],
                ),
            );
            cy += ui.advance_as(BODY_S, 22.0);
            ui.text_as(
                x,
                cy,
                BODY_S,
                muted,
                &fill(
                    EN.vault_cost_bits,
                    &[&format!("{:.1}", vault::cost_bits(mem, passes))],
                ),
            );
            cy += ui.advance_as(BODY_S, 30.0);
            // A preset pressed moves on (F103); Custom's two rows keep it.
            if current == CUSTOM
                && crate::screens::next_button(ui, x, cy, w, EN.vault_continue, va(V::CNext(n)))
            {
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
                let (ink, muted) = (on_selected(on, ink), on_selected(on, muted));
                let warn = on_selected(on, warn);
                ui.fill(x, cy, w, row_h, 8.0, selected_or(on, inset));
                let edge = selected_or(on, Role::OutlineVariant);
                ui.stroke(x, cy, w, row_h, 8.0, edge);
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
                    let name = ui.fit_as(LABEL_L, label, w - 28.0);
                    ui.text_as(x + 14.0, cy + 6.0, LABEL_L, ink, &name);
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
                    let here = ui.fit_as(BODY_S, &here, w - 28.0);
                    ui.text_as(x + 14.0, cy + 26.0, BODY_S, muted, &here);
                    if let Some(f) = short {
                        let has = fill(EN.vault_this_computer_has, &[&mib_text(f).to_string()]);
                        let has = ui.fit_as(BODY_S, &has, w - 28.0);
                        ui.text_as(x + 14.0, cy + 46.0, BODY_S, warn, &has);
                    }
                } else {
                    ui.text_as(x + 14.0, cy + 8.0, LABEL_L, ink, label);
                    ui.text_right_as((x + w - 14.0, cy + 5.0, 24.0), LABEL_L, ink, &cost);
                    let line = match short {
                        Some(f) => fill(EN.vault_this_computer_has, &[&mib_text(f).to_string()]),
                        None => time,
                    };
                    let line = ui.fit_as(BODY_S, &line, w - 28.0);
                    ui.text_as(
                        x + 14.0,
                        cy + 31.0,
                        BODY_S,
                        if short.is_some() { warn } else { muted },
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
                BUTTON_H,
                EN.vault_customise,
                Style::Ghost,
                va(V::CCustomise),
            );
            cy += 50.0;
        }
        vstep::SIZE => {
            let bw = (w - 18.0) / 4.0;
            for (k, s) in SLOT_SIZES.iter().enumerate() {
                let on = *s == c.slot;
                let (ink, muted) = (on_selected(on, ink), on_selected(on, muted));
                let bx = x + k as f32 * (bw + 6.0);
                ui.fill(bx, cy, bw, 62.0, 8.0, selected_or(on, inset));
                let edge = selected_or(on, Role::OutlineVariant);
                ui.stroke(bx, cy, bw, 62.0, 8.0, edge);
                let t = size_text(*s);
                let tw = ui.measure_as(TITLE_M, &t);
                ui.text_as(bx + (bw - tw) / 2.0, cy + 12.0, TITLE_M, ink, &t);
                let sub = fill(
                    EN.vault_file_n_inline,
                    &[&file_text(file_len(*s)).to_string()],
                );
                let sw = ui.measure_as(BODY_S, &sub);
                ui.text_as(bx + (bw - sw) / 2.0, cy + 36.0, BODY_S, muted, &sub);
                ui.hit(bx, cy, bw, 62.0, va(V::CSize(k)));
            }
            cy += 80.0;
        }
        _ => {
            let half = (w - 12.0) / 2.0;
            let may = app.may_load_keys();
            // Its name, which names the file: what it holds, since the
            // file is all anyone who sees the stick sees (`docs/VAULT.md`
            // §6).
            ui.text_mid_as((x, cy, 36.0), LABEL_L, ink, EN.vault_name);
            cy += 34.0;
            cy += ui.wrap_as(
                (x, cy, w),
                BODY_S,
                muted,
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
            if nw + 16.0 + ui.measure_as(BODY_S, &file) <= w {
                ui.text_mid_as((x + nw + 16.0, cy, FIELD_H), BODY_S, muted, &file);
                cy += FIELD_H + 16.0;
            } else {
                ui.text_as(x, cy + FIELD_H + 8.0, BODY_S, muted, &file);
                cy += FIELD_H + 38.0;
            }
            if !may {
                let pull = fill(EN.vault_remove_the_then, &[app.medium.noun()]);
                cy += stick_banner(ui, x, cy, w, &pull);
            }
            ui.button(
                x,
                cy,
                None,
                BUTTON_H,
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
                ui.text_mid_as(
                    (x, cy, 36.0),
                    LABEL_L,
                    ink,
                    &fill(EN.vault_passphrase_n, &[&(i + 1).to_string()]),
                );
                if i > 0 {
                    ui.button(
                        x + w - (ui.measure_as(LABEL_L, EN.vault_remove) + 28.0),
                        cy,
                        None,
                        BUTTON_H,
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
                    BUTTON_H,
                    EN.vault_generate_dice,
                    dice_style,
                    va(V::Dice(i)),
                );
                // Beside the button; on a small panel under it.
                if ui.compact {
                    cy += 46.0;
                    ui.text_as(x, cy, BODY_S, muted, EN.vault_or_type_your);
                    cy += ui.advance_as(BODY_S, 26.0);
                } else {
                    ui.text_mid_as(
                        (x + dw + 14.0, cy, 40.0),
                        BODY_S,
                        muted,
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
                cy += FIELD_H + 6.0 + never_real_seed_if(may, app, ui, x, cy, w);
                let (status, color) = if !b.text.is_empty() && *a.text == *b.text {
                    (EN.vault_matches, good)
                } else if !b.text.is_empty() {
                    (EN.vault_does_not_match, warn)
                } else {
                    (EN.vault_type_it_then, muted)
                };
                let on = matches!(app.vaults.focus, Some(Focus::Phrase(j, _)) if j == i);
                let line = ui.field_supporting((x, cy, w), color, status, on);
                cy += line.max(22.0);
                // Its strength: the dice's bits and the cost's, if dice made it.
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
                            good
                        } else {
                            muted
                        },
                    )),
                    None if !a.text.is_empty() && !app.pass_is_public(&a.text) => Some((
                        fill(EN.vault_typed_its_own, &[&format!("{cost:.1}")]),
                        muted,
                    )),
                    None => None,
                };
                if let Some((line, tone)) = strength {
                    cy += ui.wrap_as((x, cy, w), BODY_S, tone, &line);
                }
                // The demo's public passphrase stands in for the rolled
                // words: the "Demo only" card says so (`docs/DEMO.md` §5.2).
                cy += 12.0;
                if let Some((di, rolls, list)) = app.vaults.dice.as_ref()
                    && *di == i
                {
                    cy += super::dice_screen::dice_panel(app, ui, x, cy, w, rolls, *list);
                }
            }
            if c.phrases.len() < faraday_vault::SLOTS {
                ui.button(
                    x,
                    cy,
                    None,
                    BUTTON_H,
                    EN.vault_add_another_passphrase,
                    Style::Tonal,
                    va(V::CAddPhrase),
                );
                cy += 46.0;
            }
            // A small panel has no side panel: what will be made, and
            // Create vault, come after the last step's controls.
            if ui.compact {
                cy += 10.0;
                ui.rule(x, cy, w, Role::OutlineVariant);
                cy += 16.0;
                cy += create_summary(app, ui, x, cy, w) + 8.0;
                cy += create_button(app, ui, x, cy, w) + 8.0;
            }
        }
    }
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
    let (ink, rule) = (Role::OnSurface, Role::OutlineVariant);
    let muted = Role::OnSurfaceVariant;
    let Some(c) = app.vaults.create.as_ref() else {
        return;
    };
    ui.fill(px, 0.0, pw, h, 0.0, Role::Surface);
    ui.fill(px, 0.0, 1.0, h, 0.0, rule);
    let x = px + 26.0;
    let w = pw - 52.0;
    let mut y = tokens::SIDE_PANEL_TOP;
    ui.text_as(x, y, LABEL_L, muted, EN.vault_your_choices);
    y += ui.line_as(LABEL_L) + faraday_draw::tokens::GRID;
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
        ui.text_as(x, y, BODY_S, muted, label);
        let v = if live {
            summaries[k].clone()
        } else {
            EN.vault_not_chosen_yet.to_string()
        };
        let v = ui.fit_as(BODY_S, &v, w);
        ui.text_as(
            x,
            y + 18.0,
            BODY_S,
            if live { ink.paint() } else { crate::ui::off() },
            &v,
        );
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
    let (bad, warn) = (Role::Error, Role::Warning);
    let (muted, ink) = (Role::OnSurfaceVariant, Role::OnSurface);
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
    let mut warnings: Vec<(String, crate::ui::Paint)> = Vec::new();
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
            bad.paint(),
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
            warn.paint(),
        ));
    }
    if app.vaults.seconds(mem, passes).is_some_and(|s| s > 10.0) {
        warnings.push((
            fill(
                EN.vault_unlocking_takes,
                &[&app.vaults.time_text(mem, passes).to_string()],
            ),
            warn.paint(),
        ));
    }
    if c.on[1..].iter().any(|on| *on) {
        warnings.push((EN.vault_raspberry_pi.to_string(), muted.paint()));
    }
    y += 4.0;
    for (t, color) in &warnings {
        let th = ui.wrap_as((x + 12.0, y + 10.0, w - 24.0), BODY_S, *color, t);
        ui.fill(x, y, w, th + 20.0, 8.0, color.with_alpha(26));
        ui.wrap_as((x + 12.0, y + 10.0, w - 24.0), BODY_S, *color, t);
        y += th + 28.0;
    }
    y += 8.0;
    ui.text_as(x, y, LABEL_L, muted, EN.vault_will_create);
    y += ui.advance_as(LABEL_L, 26.0);
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
        ui.text_mid_as((x, y, 30.0), BODY_S, muted, k);
        ui.text_right_as((x + w, y, 30.0), BODY_S, ink, v);
        y += 30.0;
    }
    if let Some(e) = &c.error {
        y += 6.0;
        y += ui.wrap_as((x, y, w), BODY_S, bad, e) + 6.0;
    }
    y - y0
}

/// Create vault, or how long creating takes while it works. Returns its
/// height.
fn create_button(app: &View, ui: &mut Ui, x: f32, by: f32, w: f32) -> f32 {
    let (_, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
    if app.vaults.working == Some(vault::Work::Create) {
        let t = app.vaults.time_text(mem, passes).to_string();
        let line = fill(EN.vault_creating, &[&t]);
        crate::screens::working_line(ui, (x, by, w, 46.0), &line);
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
            let name = super::wallet_name_in(r, field::WALLET_NAME, EN.vault_wallet).into_owned();
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
                        &super::wallet_name_in(r, field::SHARE_WALLET_NAME, EN.vault_wallet),
                        &n(field::SHARE_NUMBER),
                        &n(field::SHARE_OF),
                    ],
                ),
                String::new(),
                false,
            )
        }
        kind::CODEX32_SHARE => crate::codex32_backup::screen::vault_row(r),
        // A key's passphrase kept without its words: named by the key.
        kind::PASSPHRASE => {
            let fp = crate::codex32_backup::screen::record_fp(r, field::PASSPHRASE_KEY);
            (fill(EN.plan_passphrase_label, &[&fp]), String::new(), true)
        }
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
    let (good, warn) = (Role::Ok, Role::Warning);
    let rule = Role::OutlineVariant;
    let (muted, ink) = (Role::OnSurfaceVariant, Role::OnSurface);
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
        ui.text_as(x, y + 56.0, BODY_M, muted, EN.vault_no_vault_is_open);
        if let Some((label, a)) = app.vault_way(Screen::VaultContents) {
            ui.button(x, y + 90.0, None, BUTTON_H, &label, Style::Primary, a);
        }
        return;
    };
    title(ui, x, y, &v.label(app.vaults.store()));
    let lw = ui.measure_as(HEADLINE_M.medium(), &v.label(app.vaults.store()));
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
    ui.text_mid_as(
        (x + lw + 16.0, y + 2.0, 32.0),
        LABEL_L,
        if v.changes == 0 { good } else { warn },
        &state,
    );
    // Lock and Rename end left of the top row's pills and ?, which the
    // page scrolls under (`docs/LAYOUT.md` §15).
    let end = (x + w).min(ui.top_row_left - tokens::GAP);
    let lockw = ui.measure_as(LABEL_L, EN.vault_lock) + 36.0;
    ui.button(
        end - lockw,
        y - 2.0,
        Some(lockw),
        BUTTON_H,
        EN.vault_lock,
        Style::Primary,
        Action::LockAsk,
    );
    let rw = ui.measure_as(LABEL_L, EN.vault_rename) + 28.0;
    ui.button(
        end - lockw - 8.0 - rw,
        y + 2.0,
        Some(rw),
        BUTTON_H,
        EN.vault_rename,
        Style::Ghost,
        va(V::Rename),
    );
    y += 40.0;
    let sub = format!("{} · {}", v.name, cost_text(&v.header().cost));
    ui.text_as(x, y, BODY_S.mono(), muted, &sub);
    let mut vx = x + ui.measure_as(BODY_S.mono(), &sub) + 24.0;
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
                BUTTON_H,
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
    // The item's reading parts keep the column; a wallet's chart takes
    // the room to the window's edge (`docs/LAYOUT.md` §14).
    let room = w - kinds_w - items_w - 48.0;
    let detail_w = if chart_shown {
        room
    } else {
        room.min(tokens::COLUMN_MAX)
    };
    let top = y;
    let mut ky = top;
    // The kinds it holds, then a row for each kind to add.
    let empty = shows_empty(app, v);
    for (k, label, count) in listed_kinds(app, v) {
        let on = k == app.vaults.category;
        if on {
            ui.fill(x, ky, kinds_w, 38.0, 8.0, Role::SecondaryContainer);
        }
        let tone = on_selected(on, muted);
        ui.text_mid_as((x + 12.0, ky, 38.0), LABEL_L, tone, label);
        let n = count.to_string();
        ui.text_right_as((x + kinds_w - 12.0, ky, 38.0), BODY_S, tone, &n);
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
        ui.text_as(ix, top + 6.0, TITLE_M, ink, NOTHING_YET);
        let mut by = top + 40.0;
        for (label, style, action) in empty_buttons(app) {
            let bw = ui.measure_as(LABEL_L, &label) + 36.0;
            ui.button(ix, by, Some(bw), BUTTON_H, &label, style, action);
            by += 50.0;
        }
        return;
    }
    let cat_label = CATEGORIES[app.vaults.category].0;
    let items = search::shown(app, v);
    let add = add_label(app.vaults.category);
    ui.text_mid_as((ix, top, 34.0), LABEL_L, ink, cat_label);
    if let Some(a) = add {
        let aw = ui.measure_as(LABEL_L, a) + 26.0;
        ui.button(
            ix + items_w - aw,
            top,
            Some(aw),
            BUTTON_H,
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
    let mut extra: Vec<(String, V)> = Vec::new();
    if app.vaults.category == 2 {
        extra.push((EN.vault_scan_a_code.to_string(), V::ScanEntry));
        if entries_files > 0 {
            extra.push((
                fill(EN.vault_import_from_files, &[&entries_files.to_string()]),
                V::ImportAllEntries,
            ));
        }
    }
    if !extra.is_empty() {
        let mut bx = ix;
        for (label, a) in &extra {
            let label = ui.fit_as(LABEL_L, label, items_w - 30.0);
            let bw = ui.measure_as(LABEL_L, &label) + 24.0;
            if bx + bw > ix + items_w {
                bx = ix;
                iy += 38.0;
            }
            ui.button(bx, iy, Some(bw), BUTTON_H, &label, Style::Secondary, va(*a));
            bx += bw + 6.0;
        }
        iy += 42.0;
    }
    if app.vaults.category == 2 {
        let label = EN.vault_totp_codes;
        let bw = ui.measure_as(LABEL_L, label) + 24.0;
        ui.button(
            ix,
            iy,
            Some(bw),
            BUTTON_H,
            label,
            Style::Secondary,
            Action::OpenCodes,
        );
        iy += 42.0;
    }
    if vault::search::searchable(app.vaults.category) {
        iy += search::bar(app, ui, ix, iy, items_w, vault::SearchIn::Items);
    }
    let sel = app.vaults.item[app.vaults.category].min(items.len().saturating_sub(1));
    search::item_rows(app, ui, &items, sel, (ix, iy, items_w), h - 24.0);

    // The detail: a form, the session's keys or wallets to save, or the item.
    ui.fill(detail_x - 16.0, top - 8.0, 1.0, h - top - 16.0, 0.0, rule);
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
        _ => None,
    }
}

/// A row for each kind to add at (x, y), w wide and `row` tall, the Add
/// icon in the accent colour before its label. Returns their height.
fn add_rows(ui: &mut Ui, x: f32, y: f32, w: f32, row: f32) -> f32 {
    let (ink, primary) = (Role::OnSurface, Role::Primary);
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
            primary,
        );
        let t = ui.fit_as(LABEL_L, add, w - 34.0);
        ui.text_mid_as((x + 30.0, cy, row), LABEL_L, ink, &t);
        ui.hit(x, cy, w, row, va(V::AddKind(k)));
        cy += row + 4.0;
    }
    cy - y
}

/// Whether the contents show an item, or something being added or made,
/// rather than the vault's list: on a small panel, a page of its own.
pub(crate) fn contents_detail(app: &Faraday) -> bool {
    let v = &app.vaults;
    v.item_open || v.form.is_some() || v.prompt.is_some() || v.saving || v.signing || v.sb_images
}

/// [`contents`] on a small panel: the vault, its kinds and the items of
/// the kind chosen, as a list; a chosen item, or what is being added or
/// made, as a page of its own that the bar goes back from.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn contents_compact(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let (good, warn) = (Role::Ok, Role::Warning);
    let ink = Role::OnSurface;
    let muted = Role::OnSurfaceVariant;
    use crate::compact::M;
    use crate::compact::screen::{about, finish};
    use crate::screens::wrap_buttons;
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    let Some(v) = app.vaults.open.get(app.vaults.current) else {
        ui.text_as(x, y, BODY_M, muted, EN.vault_no_vault_is_open);
        if let Some((label, a)) = app.vault_way(Screen::VaultContents) {
            let label = ui.fit_as(TITLE_M, &label, w - 24.0);
            ui.button(x, y + 32.0, Some(w), BUTTON_H, &label, Style::Primary, a);
        }
        return;
    };
    let cat_label = CATEGORIES[app.vaults.category].0;
    let items = search::shown(app, v);
    let sel = app.vaults.item[app.vaults.category].min(items.len().saturating_sub(1));
    if app.vaults.focus == Some(Focus::Search(vault::SearchIn::Items)) && !contents_detail(app) {
        return search::compact_view(app, ui, &items, (x0, cw, h), (x, top, w));
    }
    if contents_detail(app) {
        y = detail(app, ui, v, &items, sel, x, y, w);
        finish(app, ui, x0, cw, h, y - top + 16.0);
        return;
    }
    let label = ui.fit_as(TITLE_M, &v.label(app.vaults.store()), w);
    ui.text_as(x, y, TITLE_M, ink, &label);
    y += ui.line_as(TITLE_M) + faraday_draw::tokens::GAP_SMALL;
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
    ui.text_as(
        x,
        y,
        LABEL_L,
        if v.changes == 0 { good } else { warn },
        &state,
    );
    y += ui.advance_as(LABEL_L, 22.0);
    let sub = format!("{} · {}", v.name, cost_text(&v.header().cost));
    let sub = ui.fit_as(BODY_S.mono(), &sub, w);
    ui.text_as(x, y, BODY_S.mono(), muted, &sub);
    y += ui.advance_as(BODY_S.mono(), 26.0);
    let half = (w - 8.0) / 2.0;
    ui.button(
        x,
        y,
        Some(half),
        BUTTON_H,
        EN.vault_rename,
        Style::Secondary,
        va(V::Rename),
    );
    ui.button(
        x + half + 8.0,
        y,
        Some(half),
        BUTTON_H,
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
        y += add_rows(ui, x, y, w, 36.0) + 4.0;
    }
    ui.rule(x, y, w, Role::OutlineVariant);
    y += 14.0;
    if empty {
        ui.text_as(x, y, TITLE_M, ink, NOTHING_YET);
        y += ui.advance_as(TITLE_M, 34.0);
        for (label, style, action) in empty_buttons(app) {
            ui.button(x, y, Some(w), BUTTON_H, &label, style, action);
            y += 54.0;
        }
        finish(app, ui, x0, cw, h, y - top + 16.0);
        return;
    }
    section_label(ui, x, y, cat_label);
    y += ui.line_as(faraday_draw::tokens::TITLE_S) + faraday_draw::tokens::GRID;
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
    if app.vaults.category == 2 {
        extra.push((EN.vault_scan_a_code.to_string(), V::ScanEntry));
        if entries_files > 0 {
            extra.push((
                fill(EN.vault_import_from_files, &[&entries_files.to_string()]),
                V::ImportAllEntries,
            ));
        }
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
        let bw = ui.measure_as(LABEL_L, label) + 28.0;
        ui.button(
            x,
            y,
            Some(bw),
            BUTTON_H,
            label,
            Style::Secondary,
            Action::OpenCodes,
        );
        y += 44.0;
    }
    if vault::search::searchable(app.vaults.category) {
        y += search::bar(app, ui, x, y, w, vault::SearchIn::Items);
    }
    y += search::compact_rows(app, ui, &items, x, y, w);
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
    let muted = Role::OnSurfaceVariant;
    let tw = ui.measure_as(LABEL_M, label);
    let pw = tw + 24.0;
    ui.fill(x, y, pw, 26.0, 13.0, Role::OnSurfaceVariant.with_alpha(30));
    ui.stroke(x, y, pw, 26.0, 13.0, Role::OnSurfaceVariant.with_alpha(110));
    ui.text_mid_as((x + (pw - tw) / 2.0, y, 26.0), LABEL_M, muted, label);
}

/// The entry form on the codes screen (`docs/TOTP.md` §5.3): Add an
/// entry, or the form a scanned setup code filled in. Always a new
/// entry — editing one happens on Vault contents, where its row leads.
/// Returns the height it took.
pub(crate) fn codes_form(app: &View, ui: &mut Ui, dx: f32, dy: f32, dw: f32) -> f32 {
    let (muted, ink) = (Role::OnSurfaceVariant, Role::OnSurface);
    let top = dy;
    let mut dy = dy;
    let Some(form) = &app.vaults.form else {
        return 0.0;
    };
    ui.text_as(dx, dy, TITLE_M, ink, EN.vault_add_an_entry);
    dy += ui.line_as(TITLE_M) + faraday_draw::tokens::PAD;
    for (k, (_, label, b, masked)) in form.fields.iter().enumerate() {
        ui.text_as(dx, dy, BODY_S, muted, label);
        dy += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
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
                focused,
                va(V::FocusField(k)),
                va(V::ShowTyped),
            );
            app.password_chip(ui, form, k, b);
            dy += FIELD_H + 12.0;
        } else {
            text_box(ui, dx, dy, dw, b, false, focused, va(V::FocusField(k)));
            dy += FIELD_H + 12.0;
        }
    }
    dy += 8.0;
    ui.button(
        dx,
        dy,
        Some(120.0),
        BUTTON_H,
        EN.vault_save,
        Style::Primary,
        va(V::FormSave),
    );
    ui.button(
        dx + 130.0,
        dy,
        Some(120.0),
        BUTTON_H,
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
    let (bad, warn) = (Role::Error, Role::Warning);
    let (muted, ink) = (Role::OnSurfaceVariant, Role::OnSurface);
    if let Some(form) = &app.vaults.form {
        let revoking = form.kind == kind::GPG && form.fields.first().is_some_and(|f| f.0 == 3);
        let heading = match (form.kind, form.record) {
            (kind::GPG, _) if revoking => EN.vault_make_a_revocation,
            (kind::SLOT_LABEL, _) => EN.vault_rename_the_vault,
            (kind::NOTE, None) => EN.vault_add_a_note,
            (kind::NOTE, Some(_)) => EN.vault_edit_the_note,
            (kind::GPG, _) => EN.vault_make_a_key,
            (kind::SECURE_BOOT, _) => EN.vault_make_secure_boot,
            (_, None) => EN.vault_add_an_entry,
            _ => EN.vault_edit_the_entry,
        };
        ui.text_as(dx, dy, TITLE_M, ink, heading);
        dy += ui.line_as(TITLE_M) + faraday_draw::tokens::PAD;
        for (k, (_, label, b, masked)) in form.fields.iter().enumerate() {
            ui.text_as(dx, dy, BODY_S, muted, label);
            dy += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
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
                        focused,
                        va(V::FocusField(k)),
                        va(V::ShowTyped),
                    );
                    app.password_chip(ui, form, k, b);
                } else {
                    text_box(ui, dx, dy, dw, b, false, focused, va(V::FocusField(k)));
                }
                dy += FIELD_H + 12.0;
            }
        }
        if revoking {
            ui.text_as(dx, dy, BODY_S, muted, EN.vault_reason);
            dy += ui.advance_as(BODY_S, 22.0);
            dy += reasons_row(app, ui, dx, dy, dw) + 16.0;
        } else if form.kind == kind::GPG {
            ui.text_as(dx, dy, BODY_S, muted, EN.vault_expires);
            dy += ui.advance_as(BODY_S, 22.0);
            dy += years_row(ui, dx, dy, dw, Some(app.vaults.gpg_years), V::GpgYears) + 16.0;
        }
        let save = match form.kind {
            kind::GPG if revoking => EN.vault_make_the_revocation,
            kind::GPG => EN.vault_make_the_key,
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
            ui.measure_as(LABEL_L, save) + 42.0
        } else {
            0.0
        };
        ui.button(
            dx + sw,
            dy,
            None,
            BUTTON_H,
            EN.vault_cancel,
            Style::Ghost,
            va(V::FormCancel),
        );
        return dy;
    }
    if let Some(p) = &app.vaults.prompt {
        let heading = match p.purpose {
            vault::Purpose::KeyPassphrase | vault::Purpose::ForWallet(..) => {
                EN.vault_load_with_a_bip.to_string()
            }
            vault::Purpose::Kdbx => EN.vault_export_for_keepass.to_string(),
        };
        let heading = ui.fit_as(TITLE_M, &heading, dw);
        ui.text_as(dx, dy, TITLE_M, ink, &heading);
        dy += ui.line_as(TITLE_M) + faraday_draw::tokens::PAD;
        ui.text_as(dx, dy, BODY_S, muted, EN.vault_passphrase);
        dy += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
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
        dy += FIELD_H + 16.0 + never_under(app, ui, dx, dy, dw);
        if app.vaults.working == Some(vault::Work::Kdbx) {
            crate::screens::working_line(ui, (dx, dy, dw, 40.0), EN.vault_sealing);
        } else {
            let label = match p.purpose {
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
                ui.measure_as(LABEL_L, label) + 42.0
            } else {
                0.0
            };
            ui.button(
                dx + sw,
                dy,
                None,
                BUTTON_H,
                EN.vault_cancel,
                Style::Ghost,
                va(V::PromptCancel),
            );
        }
        dy += 54.0;
        if let Some(e) = &p.error {
            ui.wrap_as((dx, dy, dw), BODY_S, bad, e);
        }
        return dy;
    }
    if app.vaults.saving {
        return save_panel(app, ui, v, dx, dy, dw);
    }
    if app.vaults.sb_images {
        return images_panel(app, ui, dx, dy, dw);
    }
    if app.vaults.signing {
        ui.text_as(dx, dy, TITLE_M, ink, EN.vault_sign_a_file);
        dy += ui.line_as(TITLE_M) + faraday_draw::tokens::PAD;
        // The key that signs, named as GnuPG's pinentry names it: user
        // ID, key IDs and creation date, and which subkey signs.
        if let Some(g) = items
            .get(sel)
            .and_then(|r| crate::gpg::shown(*r, app.tz_min()))
        {
            if let Some(uid) = g.user_ids.first() {
                dy += ui.wrap_as((dx, dy, dw), LABEL_L, ink, &format!("\"{uid}\"")) + 4.0;
            }
            dy += ui.wrap_as((dx, dy, dw), BODY_S.mono(), muted, &g.subkey) + 4.0;
            dy += ui.wrap_as(
                (dx, dy, dw),
                BODY_S,
                muted,
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
            ui.text_as(
                dx,
                dy,
                BODY_S,
                muted,
                &fill(EN.vault_no_file, &[app.medium.from_the()]),
            );
            dy += ui.advance_as(BODY_S, 30.0);
        } else {
            let all_ticked = files.iter().all(|f| app.gpg_ticked(f));
            ui.checkbox(dx, dy + 11.0, all_ticked, true);
            ui.text_mid_as(
                (dx + 30.0, dy, 40.0),
                BODY_M.mono(),
                ink,
                EN.vault_select_all,
            );
            ui.hit(dx, dy, dw, 40.0, va(V::GpgTickAll));
            ui.rule(dx, dy + 40.0, dw, Role::OutlineVariant);
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
                ui.measure_as(LABEL_M, EN.vault_signed) + 18.0
            } else {
                0.0
            };
            let widths: Vec<f32> = buttons
                .iter()
                .map(|(l, _)| ui.measure_as(LABEL_L, l) + 30.0)
                .collect();
            let all = widths.iter().sum::<f32>() + 6.0;
            let (name_w, by) = if ui.compact {
                (dw - 40.0 - signed_w - 8.0, dy + 40.0)
            } else {
                (dw - 40.0 - all - signed_w - 24.0, dy + 2.0)
            };
            ui.checkbox(dx, dy + 11.0, ticked, true);
            let line = format!("{} · {}", f.name, gpg_file_size(f.size as u64));
            let name = ui.fit_as(BODY_M.mono(), &line, name_w);
            ui.text_mid_as((dx + 30.0, dy, 40.0), BODY_M.mono(), ink, &name);
            if signed {
                ui.chip(
                    dx + 30.0 + name_w + 8.0,
                    dy + 10.0,
                    EN.vault_signed,
                    Role::OnOkContainer,
                    Role::OkContainer,
                );
            }
            // On a small panel the buttons are on the line below: the
            // whole first line ticks.
            let hit_w = if ui.compact { dw } else { dw - all - 16.0 };
            ui.hit(dx, dy, hit_w, 40.0, va(V::GpgTick(fi)));
            let mut bx = if ui.compact { dx + 30.0 } else { dx + dw - all };
            for ((label, action), bw) in buttons.iter().zip(&widths) {
                ui.button(
                    bx,
                    by,
                    Some(*bw),
                    BUTTON_H,
                    label,
                    Style::Secondary,
                    *action,
                );
                bx += bw + 6.0;
            }
            // What is signed: the file's SHA-256, and a small text file's
            // text, so that the bytes can be checked before the hold.
            let mut ey = dy + if ui.compact { 84.0 } else { 46.0 };
            let hex: String = f.hash.iter().map(|b| format!("{b:02x}")).collect();
            ey += ui.wrap_as((dx + 30.0, ey, dw - 30.0), BODY_S.mono(), muted, &hex) + 6.0;
            if let Some(t) = &f.text {
                ey += ui.wrap_as((dx + 30.0, ey, dw - 30.0), BODY_S.mono(), ink, t) + 6.0;
            }
            ui.rule(dx, ey, dw, Role::OutlineVariant);
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
            BUTTON_H,
            EN.vault_sign,
            if any { Style::Primary } else { Style::Disabled },
            va(V::GpgSign),
        );
        ui.button(
            dx + w + 10.0,
            dy,
            None,
            BUTTON_H,
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
    ui.text_as(dx, dy, if mono { BODY_L.mono() } else { TITLE_M }, ink, &t);
    dy += ui.advance_as(if mono { BODY_L.mono() } else { TITLE_M }, 30.0);
    ui.text_as(dx, dy, BODY_S, muted, &line);
    dy += ui.advance_as(BODY_S, 34.0);
    let mut secret_id = 0usize;
    // The secrets shown by their eyes here, read once for the page.
    let shown_fields = app.vaults.fields_shown(app.vault_on_page());
    let field_row = |ui: &mut Ui, dy: &mut f32, label: &str, value: &str, mono: bool| {
        ui.text_as(dx, *dy, BODY_S, muted, label);
        *dy += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
        *dy += ui.wrap_as(
            (dx, *dy, dw),
            if mono { BODY_M.mono() } else { BODY_M },
            ink,
            value,
        ) + 14.0;
    };
    let mut secret_row = |ui: &mut Ui, dy: &mut f32, label: &str, value: &str, mono: bool| {
        ui.text_as(dx, *dy, BODY_S, muted, label);
        *dy += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
        let shown = shown_fields & (1u64 << secret_id.min(63)) != 0;
        let eye = (va(V::Reveal(secret_id)), shown);
        *dy += secret(ui, (dx, *dy, dw), value, mono, eye) + 14.0;
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
                secret_row(ui, &mut dy, EN.vault_words, &words_text(m), true);
                if m.language() == faraday_bip::bip39::Language::English {
                    ui.text_as(dx, dy, BODY_S, muted, EN.board_seedqr);
                    dy += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
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
                secret_row(ui, &mut dy, EN.vault_password, &p, true);
            }
            if let Some(u) = r.public_text(field::URL) {
                field_row(ui, &mut dy, EN.vault_url, u, true);
            }
            if r.has(field::TOTP) {
                // A row before the secret: the code, live, or Set the
                // time while the clock is not checked (`docs/TOTP.md`
                // §6).
                ui.text_as(dx, dy, BODY_S, muted, EN.vault_code);
                dy += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
                let now = app.clock_now().filter(|_| app.clock_checked());
                match app.entry_code(r, now) {
                    Ok(Some(code)) => {
                        let (grouped, left) = (code.grouped, code.left);
                        let gw = ui.measure_as(BODY_L.mono(), &grouped);
                        ui.text_as(dx, dy, BODY_L.mono(), ink, &grouped);
                        ui.text_as(
                            dx + gw + 12.0,
                            dy + 4.0,
                            BODY_S,
                            muted,
                            &format!("{left} s"),
                        );
                        dy += ui.advance_as(BODY_S, 34.0);
                    }
                    Ok(None) => {
                        ui.button(
                            dx,
                            dy,
                            None,
                            BUTTON_H,
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
                secret_row(ui, &mut dy, EN.vault_totp_secret, &t, true);
            }
            if r.has(field::NOTES) {
                let n = app.reveal_record(r, field::NOTES, Reveal::VaultEntry(Part::EntryNotes));
                secret_row(ui, &mut dy, EN.vault_notes, &n, false);
            }
            actions.push((EN.vault_edit, va(V::Edit)));
            actions.push((EN.vault_export_for_keepass, va(V::ExportKdbx)));
        }
        kind::NOTE => {
            secret_row(
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
                // The paperkey is made only while its eye shows it.
                ui.text_as(dx, dy, BODY_S, muted, EN.vault_paperkey);
                dy += ui.line_as(BODY_S) + faraday_draw::tokens::GAP_SMALL;
                dy += secret_with(
                    ui,
                    (dx, dy, dw),
                    || {
                        app.paperkey_shown(r)
                            .map(faraday_keys::Held::release)
                            .unwrap_or_default()
                    },
                    true,
                    (va(V::Reveal(0)), shown_fields & 1 != 0),
                ) + 14.0;
                // The term now, then the new term to choose: nothing is
                // renewed until one is chosen.
                let now_line = if g.expires == EN.vault_never {
                    EN.vault_term_never.to_string()
                } else {
                    fill(EN.vault_term_until, &[&g.expires.to_string()])
                };
                ui.text_as(dx, dy, BODY_S, muted, &now_line);
                dy += ui.advance_as(BODY_S, 22.0);
                ui.text_as(dx, dy, BODY_S, muted, EN.vault_renew_label);
                dy += ui.advance_as(BODY_S, 22.0);
                // Renew at the row's end; on a small panel under it.
                let rw = ui.measure_as(LABEL_L, EN.vault_renew) + 30.0;
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
                    let bw = ui.measure_as(LABEL_L, EN.backup_flow_first) + 30.0;
                    ui.button(
                        dx + dw - bw,
                        ry,
                        Some(bw),
                        BUTTON_H,
                        EN.backup_flow_first,
                        Style::Secondary,
                        back_up_first(crate::backup_flow::Kind::Gpg),
                    );
                } else {
                    ui.button(
                        dx + dw - rw,
                        ry,
                        Some(rw),
                        BUTTON_H,
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
            ui.text_as(dx, dy, BODY_S, muted, EN.vault_enrolment_adds);
            dy += ui.advance_as(BODY_S, 22.0);
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
            dy += ui.wrap_as((dx, dy, dw), BODY_S, muted, line) + 14.0;
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
        _ => {}
    }
    dy += 6.0;
    let mut bx = dx;
    for (i, (label, a)) in actions.into_iter().enumerate() {
        let bw = ui.measure_as(LABEL_L, label) + 30.0;
        if bx > dx && (bx + bw > dx + dw || breaks.contains(&i)) {
            bx = dx;
            dy += 44.0;
        }
        ui.button(bx, dy, Some(bw), BUTTON_H, label, Style::Secondary, a);
        bx += bw + 8.0;
    }
    dy += 48.0;
    let dw_hold = hold_delete(app, ui, dx, dy);
    if r.kind() == kind::GPG {
        let label = EN.vault_hold_to_mark_revoked;
        // Beside Delete where it fits, else under it.
        let (mut hx, room) = (dx + dw_hold + 10.0, dx + dw);
        if hx + ui.measure_as(LABEL_L, label) + 2.0 * faraday_draw::tokens::PAD > room {
            (hx, dy) = (dx, dy + faraday_draw::tokens::TOUCH);
        }
        let hold = va(V::GpgHoldRevoke);
        hold_button_in(app, ui, hx, dy, room - hx, label, hold, warn.paint());
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
    let (muted, ink, warn) = (Role::OnSurfaceVariant, Role::OnSurface, Role::Warning);
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
    ui.text_as(dx, dy, BODY_S, muted, EN.backup_flow_row);
    h += 20.0;
    h += ui.wrap_as(
        (dx, dy + h, dw),
        BODY_M,
        if row.waiting { warn } else { ink },
        &row.line,
    ) + 8.0;
    if row.waiting {
        ui.button(
            dx,
            dy + h,
            None,
            BUTTON_H,
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
