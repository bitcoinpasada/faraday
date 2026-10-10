//! The vault screens (`PLAN.md` §6.1, the prototype's Vaults, Unlock,
//! Create a vault and Vault contents): drawing only. What a press does is
//! `vaults.rs`.

use opensigner_core::strings::EN;
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
    let shown: String = if masked {
        "•".repeat(b.text.chars().count().min(48))
    } else {
        b.text.to_string()
    };
    let shown = ui.fit(14.0, W::M, &shown, w - 30.0);
    ui.selection(x + 12.0, y, 40.0, 14.0, W::M, &shown, focused);
    ui.text_mid(x + 12.0, y, 40.0, 14.0, W::M, TEXT, &shown);
    if focused && !ui.select_all {
        let cx = x + 13.0 + ui.measure(14.0, W::M, &shown);
        ui.caret(cx, y + 11.0, 18.0);
    }
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
    let text = format!("Remove the {} first", medium.noun());
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
    let action = va(V::Reveal(id));
    let held = ui.is_pressed(action);
    let face = if mono { W::M } else { W::R };
    let shown = if held {
        crate::secret_text::SecretText::of(value)
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
        "Release to hide"
    } else {
        "Hold to show"
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
        "Hold to delete from vault",
        va(V::HoldDelete),
        ERR,
    )
}

/// A button that acts when held, filling while it is held.
fn hold_button(
    app: &Faraday,
    ui: &mut Ui,
    x: f32,
    y: f32,
    label: &str,
    action: Action,
    color: osk_ui::Color,
) -> f32 {
    let w = ui.measure(13.0, W::S, label) + 32.0;
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
    title(ui, x, y, "Vaults");
    if back.is_none() {
        let bw = ui.measure(15.0, W::S, "Create a vault") + 32.0;
        ui.button(
            x + w - bw,
            y - 4.0,
            Some(bw),
            44.0,
            "Create a vault",
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
        &format!(
            "A vault holds keys, wallets, entries and notes under one to four passphrases, each opening \
             its own contents. A locked vault shows only what its file states: its size and unlock cost. \
             A vault is copied in on {} visit, and is sealed {} again when the session locks with \
             changes in it.",
            app.medium.a(),
            app.medium.for_the()
        ),
    );
    let files = app.vault_files();
    if files.is_empty() {
        ui.text(x, y + 8.0, 15.0, W::R, MUTED, "No vault in Files");
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
                format!(
                    "Open · {} unsaved {}",
                    v.changes,
                    if v.changes == 1 { "change" } else { "changes" }
                ),
                WARN,
                ACCENT,
                LINE,
            ),
            Some(v) => (v.label(), "Open".to_string(), OK, ACCENT, LINE),
            None if !app.may_load_keys() => (
                remembered
                    .unwrap_or(if fresh { "New vault" } else { "Locked vault" })
                    .to_string(),
                format!("Remove the {} to unlock", app.medium.noun()),
                WARN,
                MUTED,
                LINE,
            ),
            None if fresh => (
                remembered.unwrap_or("New vault").to_string(),
                "Locked · not written yet".to_string(),
                OK,
                OK,
                OK.with_alpha(90),
            ),
            None => (
                remembered.unwrap_or("Locked vault").to_string(),
                "Locked".to_string(),
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
                ("View contents", Style::Secondary, va(V::Open(i))),
                ("Lock", Style::Primary, Action::LockAsk),
            ]
        } else {
            &[("Unlock", Style::Primary, va(V::Open(i)))]
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
    title(ui, x, y, "Unlock");
    y += 56.0;
    if just_made_picked(app) {
        ui.text(x, y - 14.0, 14.0, W::R, MUTED, ONCE_MORE);
        y += 20.0;
    }
    y += guide_text(
        app,
        ui,
        x,
        y,
        w,
        "Choose the vault and type a passphrase. Each passphrase opens its own contents, and a vault does \
         not say how many it has. The time is the Argon2id cost the vault was made with, measured on this \
         computer.",
    );
    if !app.may_load_keys() {
        let pull = format!("Remove the {}, then type the passphrase", app.medium.noun());
        y += stick_banner(ui, x, y, w, &pull);
    }
    let files = app.vault_files();
    let locked: Vec<(usize, &vaults::VaultFile)> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.open.is_none())
        .collect();
    if locked.is_empty() {
        ui.text(x, y, 15.0, W::R, MUTED, "No locked vault in Files");
        ui.button(
            x,
            y + 36.0,
            None,
            40.0,
            "Create a vault",
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
        ui.stroke(x, y, w, ch, 12.0, if on { ACCENT } else { LINE });
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
        let time = format!("{} here", app.vaults.time_text(mem, f.header.cost.passes));
        ui.text_right(x + w - 18.0, y + 10.0, 24.0, 13.0, W::S, TEXT, &cost);
        ui.text_right(x + w - 18.0, y + 36.0, 24.0, 12.0, W::R, MUTED, &time);
        ui.hit(x, y, w, ch, va(V::Pick(*i)));
        y += ch + 10.0;
    }
    y += 14.0;
    section_label(ui, x, y, "Passphrase");
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
        ui.text_mid(x, y, 46.0, 15.0, W::S, ACCENT, &format!("Unlocking · {t}"));
    } else {
        let style = if app.may_load_keys() {
            Style::Primary
        } else {
            Style::Disabled
        };
        ui.button(x, y, Some(160.0), 46.0, "Unlock", style, va(V::Unlock));
        let free = match app.vaults.memory_free_mib {
            Some(m) => format!("This computer: {} free", mib_text(m)),
            None => "This computer: memory not reported".to_string(),
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
            Some(s) => s.line(),
            None => "Unlock to see what it holds".to_string(),
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
const ONCE_MORE: &str = "Type the passphrase once more to open it";

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
        Some(Screen::Home) if app.import.is_some() => ("Import", crate::boot_import::OPEN),
        // Unlocking for another flow: back to it.
        Some(s) => (app.back_link_name(s), va(V::Back)),
        None => ("Vaults", Action::Nav(Screen::Vaults)),
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
            "Create a vault",
            Style::Primary,
            va(V::Create),
        );
        y += 58.0;
    }
    let files = app.vault_files();
    if files.is_empty() {
        y += ui.wrap(x, y, w, 14.0, W::R, MUTED, "No vault in Files") + 12.0;
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
                format!(
                    "Open · {} unsaved {}",
                    v.changes,
                    if v.changes == 1 { "change" } else { "changes" }
                ),
                WARN,
                ACCENT,
                LINE,
            ),
            Some(v) => (v.label(), "Open".to_string(), OK, ACCENT, LINE),
            None if !app.may_load_keys() => (
                remembered
                    .unwrap_or(if fresh { "New vault" } else { "Locked vault" })
                    .to_string(),
                format!("Remove the {} to unlock", app.medium.noun()),
                WARN,
                MUTED,
                LINE,
            ),
            None if fresh => (
                remembered.unwrap_or("New vault").to_string(),
                "Locked · not written yet".to_string(),
                OK,
                OK,
                OK.with_alpha(90),
            ),
            None => (
                remembered.unwrap_or("Locked vault").to_string(),
                "Locked".to_string(),
                MUTED,
                MUTED,
                LINE,
            ),
        };
        let buttons: &[(&str, Style, Action)] = if open.is_some() {
            &[
                ("View contents", Style::Secondary, va(V::Open(i))),
                ("Lock", Style::Primary, Action::LockAsk),
            ]
        } else {
            &[("Unlock", Style::Primary, va(V::Open(i)))]
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
        &format!(
            "A vault holds keys, wallets, entries and notes under one to four passphrases, each opening \
             its own contents. A locked vault shows only what its file states: its size and unlock cost. \
             A vault is copied in on {} visit, and is sealed {} again when the session locks with \
             changes in it.",
            app.medium.a(),
            app.medium.for_the()
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
        let pull = format!("Remove the {}, then type the passphrase", app.medium.noun());
        y += stick_banner(ui, x, y, w, &pull);
    }
    let files = app.vault_files();
    let locked: Vec<(usize, &vaults::VaultFile)> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| f.open.is_none())
        .collect();
    if locked.is_empty() {
        y += ui.wrap(x, y, w, 14.0, W::R, MUTED, "No locked vault in Files") + 12.0;
        ui.button(
            x,
            y,
            Some(w),
            40.0,
            "Create a vault",
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
        ui.stroke(x, y, w, ch, 12.0, if on { ACCENT } else { LINE });
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
        let cost = format!(
            "{} · {} here",
            cost_text(&f.header.cost),
            app.vaults.time_text(mem, f.header.cost.passes)
        );
        let cost = ui.fit(12.0, W::S, &cost, w - 24.0);
        ui.text(x + 12.0, y + 56.0, 12.0, W::S, TEXT, &cost);
        ui.hit(x, y, w, ch, va(V::Pick(*i)));
        y += ch + 10.0;
    }
    y += 6.0;
    section_label(ui, x, y, "Passphrase");
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
        ui.text_mid(x, y, 44.0, 15.0, W::S, ACCENT, &format!("Unlocking · {t}"));
        y += 52.0;
    } else {
        let style = if app.may_load_keys() {
            Style::Primary
        } else {
            Style::Disabled
        };
        if crate::screens::pin_button(ui, x, y, Some(w), 44.0, "Unlock", style, va(V::Unlock)) {
            y += 52.0;
        }
        let free = match app.vaults.memory_free_mib {
            Some(m) => format!("This computer: {} free", mib_text(m)),
            None => "This computer: memory not reported".to_string(),
        };
        y += ui.wrap(x, y, w, 13.0, W::R, MUTED, &free) + 12.0;
    }
    if let Some(e) = &app.vaults.unlock_error {
        y += ui.wrap(x, y, w, 14.0, W::R, ERR, e) + 12.0;
    }
    y += about(
        ui,
        x,
        y,
        w,
        "Choose the vault and type a passphrase. Each passphrase opens its own contents, and a vault does \
         not say how many it has. The time is the Argon2id cost the vault was made with, measured on this \
         computer.",
    );
    finish(app, ui, x0, cw, h, y - top + 16.0);
}

fn back_link(ui: &mut Ui, x: f32, y: f32, label: &str, action: Action) {
    ui.icon(x - 4.0, y, 18.0, Icon::ChevronLeft, 12.0, MUTED);
    ui.text_mid(x + 14.0, y, 18.0, 13.0, W::R, MUTED, label);
    let lw = ui.measure(13.0, W::R, label) + 20.0;
    ui.hit(x - 6.0, y - 4.0, lw, 26.0, action);
}

// ---------------------------------------------------------------------
// Create a vault
// ---------------------------------------------------------------------

/// Each step's title, by `vstep`.
const VSTEPS: [&str; vstep::COUNT] = [
    "Where will you open it?",
    "Unlock cost",
    "Space per passphrase",
    "Name and passphrases",
    "Size",
];

pub(crate) fn create(app: &mut Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let panel_w = 320.0;
    let Some(c) = app.vaults.create.as_ref() else {
        return;
    };
    let summaries = create_summaries(app);
    let then = app
        .vaults
        .back_to
        .map(|s| format!("Then: {}", app.back_name(s)));
    let steps = c.steps();
    let cards: Vec<flow::Card> = steps
        .iter()
        .map(|&k| flow::Card {
            title: VSTEPS[k as usize].to_string(),
            summary: if c.done[k as usize] {
                summaries[k as usize].clone()
            } else {
                "Not chosen yet".to_string()
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
            None => ("Vaults", Action::Nav(Screen::Vaults)),
        }),
        heading: "Create a vault",
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
        vstep::WHERE => "A vault opens only on a machine with the memory its cost asks for. Choose every kind of \
            machine it will ever be opened on; the cost suggested next fits the one with the least memory."
            .to_string(),
        vstep::COST => "Each guess at a passphrase costs this much memory and time. Argon2id works through \
            the memory 1 KiB at a time, once each pass, so a guess is memory in KiB times passes block \
            steps, and the cost adds the base-2 logarithm of that to the passphrase's own bits: 64 MiB \
            with 3 passes is 196,608 steps, ≈17.6 bits, and doubling the memory or the passes adds ≈1. \
            These are estimates, marked ≈: they count work, not how much harder memory is than time for \
            an attacker. A dice passphrase's own bits are exact. Memory is a hard limit: a machine \
            without it cannot open the vault at any speed. The passphrase matters more: one more dice \
            word adds 12.9 bits, and this whole range of cost adds ≈6. The cost cannot be changed later."
            .to_string(),
        vstep::SIZE => "Every passphrase gets the same space, fixed for the life of the vault. 256 KiB holds \
            keys, wallets, notes and about a hundred entries."
            .to_string(),
        vstep::PRESET => "A vault opens only on a machine with the memory its cost asks for, and the cost \
            cannot be changed later. 512 MiB opens on PCs; 64 MiB opens on a Raspberry Pi as well. \
            Customise chooses the machines, the cost and the space per passphrase one by one."
            .to_string(),
        _ => "Each passphrase opens its own contents, and the file does not say how many there are. They are \
            set now: adding or changing one later means making a new vault and moving the contents across. \
            The passphrase matters more than the cost: six words from the EFF long list are 77.5 bits, out \
            of reach at any cost here, while a short or reused passphrase is not protected at any cost."
            .to_string(),
    }
}

/// The summary line of each step, by `vstep`, and of the panel's
/// choices.
fn create_summaries(app: &Faraday) -> [String; vstep::COUNT] {
    let Some(c) = app.vaults.create.as_ref() else {
        return Default::default();
    };
    let mut places = vec!["This computer".to_string()];
    for (k, m) in MACHINES.iter().enumerate() {
        if c.on[k] {
            places.push(m.1.to_string());
        }
    }
    let (k, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
    let name = if k == CUSTOM { "Custom" } else { PRESETS[k].0 };
    let n = c.phrases.len();
    let size = match app.vaults.size_row() {
        Some(r) => format!("{} · {} · {passes} passes", SIZES[r].0, mib_text(mem)),
        None => format!("{name} · {} · {passes} passes", mib_text(mem)),
    };
    let ok = (0..n).all(|i| vaults::Vaults::phrase_ok(c, i));
    [
        places.join(", "),
        format!("{name} · {} · {passes} passes", mib_text(mem)),
        format!(
            "{} · file {}",
            size_text(c.slot),
            file_text(file_len(c.slot))
        ),
        format!(
            "{}.ofv · {n} {} · {}",
            vaults::vault_stem(&c.name.text),
            if n == 1 { "passphrase" } else { "passphrases" },
            if ok { "match" } else { "not finished" }
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
            let here = app
                .vaults
                .memory_free_mib
                .map(mib_text)
                .unwrap_or_else(|| "memory not reported".into());
            ui.checkbox(x, cy + 12.0, true, false);
            ui.text_mid(x + 32.0, cy, 40.0, 14.0, W::S, TEXT, "This computer");
            ui.text_right(x + w, cy, 40.0, 13.0, W::R, MUTED, &format!("{here} free"));
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
            if crate::screens::next_button(ui, x, cy, w, "Continue", va(V::CNext(n))) {
                cy += 50.0;
            }
        }
        vstep::COST => {
            let (weakest, ram) = app.vaults.weakest();
            let suggested = app.vaults.suggested();
            let (current, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
            let choices = PRESETS
                .iter()
                .copied()
                .chain(std::iter::once(("Custom", c.memory, c.passes)));
            for (k, (label, m, p)) in choices.enumerate() {
                let on = k == current;
                let fits = m <= ram / 2;
                let (note, color) = if k == CUSTOM {
                    ("Choose memory and passes".to_string(), DIM)
                } else if k == suggested {
                    ("Suggested".to_string(), ACCENT)
                } else if fits {
                    ("Fits".to_string(), DIM)
                } else {
                    (format!("Too large for {weakest}"), WARN)
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
                let detail = format!(
                    "{} · {p} passes · ≈{:.1} bits",
                    mib_text(m),
                    vaults::cost_bits(m, p)
                );
                let detail = ui.fit(12.0, W::M, &detail, w - 32.0);
                ui.text(x + 16.0, cy + 29.0, 12.0, W::M, MUTED, &detail);
                ui.hit(x, cy, w, 50.0, va(V::CPreset(k)));
                cy += 56.0;
            }
            if current == CUSTOM {
                cy += 4.0;
                ui.text(x, cy, 12.0, W::R, MUTED, "Memory");
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
                ui.text(x, cy, 12.0, W::R, MUTED, "Passes");
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
            // The memory it needs against the weakest machine's.
            let need = mem + 100;
            let needs = format!("Needs about {} free", mib_text(need));
            ui.text(x, cy, 13.0, W::S, TEXT, &needs);
            let room = w - ui.measure(13.0, W::S, &needs) - 12.0;
            let has = ui.fit(12.0, W::R, &format!("{weakest}: {}", mib_text(ram)), room);
            ui.text_right(x + w, cy - 4.0, 24.0, 12.0, W::R, MUTED, &has);
            cy += 24.0;
            ui.fill(x, cy, w, 8.0, 4.0, INNER);
            let share = (need as f32 / ram.max(1) as f32).min(1.0);
            ui.fill(
                x,
                cy,
                w * share,
                8.0,
                4.0,
                if mem <= ram / 2 { ACCENT } else { WARN },
            );
            cy += 20.0;
            ui.text(
                x,
                cy,
                12.0,
                W::R,
                MUTED,
                &format!("Unlocks in {} here", app.vaults.time_text(mem, passes)),
            );
            cy += 22.0;
            ui.text(
                x,
                cy,
                12.0,
                W::R,
                MUTED,
                &format!(
                    "Adds ≈{:.1} bits to every passphrase",
                    vaults::cost_bits(mem, passes)
                ),
            );
            cy += 30.0;
            if crate::screens::next_button(ui, x, cy, w, "Continue", va(V::CNext(n))) {
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
                let cost = format!("{} · {passes} passes", mib_text(mem));
                let time = match app.vaults.seconds(mem, passes) {
                    Some(_) => format!("Unlocks in {} here", app.vaults.time_text(mem, passes)),
                    None => "Unlock time not measured yet".to_string(),
                };
                let needs = format!("needs about {} free", mib_text(mem + 100));
                let short = free.is_some_and(|f| mem + 100 > f);
                if ui.compact {
                    // Three lines: the name, the cost and its time here,
                    // the memory it needs.
                    let name = ui.fit(14.0, W::S, label, w - 28.0);
                    ui.text(x + 14.0, cy + 6.0, 14.0, W::S, TEXT, &name);
                    let here = match app.vaults.seconds(mem, passes) {
                        Some(_) => format!("{cost} · {} here", app.vaults.time_text(mem, passes)),
                        None => cost.clone(),
                    };
                    let here = ui.fit(12.0, W::R, &here, w - 28.0);
                    ui.text(x + 14.0, cy + 26.0, 12.0, W::R, MUTED, &here);
                    let needs = format!("Needs about {} free", mib_text(mem + 100));
                    let needs = ui.fit(12.0, W::R, &needs, w - 28.0);
                    ui.text(
                        x + 14.0,
                        cy + 46.0,
                        12.0,
                        W::R,
                        if short { WARN } else { MUTED },
                        &needs,
                    );
                } else {
                    ui.text(x + 14.0, cy + 8.0, 14.0, W::S, TEXT, label);
                    ui.text_right(x + w - 14.0, cy + 5.0, 24.0, 13.0, W::S, TEXT, &cost);
                    let line = ui.fit(12.0, W::R, &format!("{time} · {needs}"), w - 28.0);
                    ui.text(
                        x + 14.0,
                        cy + 31.0,
                        12.0,
                        W::R,
                        if short { WARN } else { MUTED },
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
                "Customise",
                Style::Ghost,
                va(V::CCustomise),
            );
            cy += 50.0;
            if crate::screens::next_button(ui, x, cy, w, "Continue", va(V::CNext(n))) {
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
                let sub = format!("file {}", file_text(file_len(*s)));
                let sw = ui.measure(12.0, W::R, &sub);
                ui.text(bx + (bw - sw) / 2.0, cy + 36.0, 12.0, W::R, MUTED, &sub);
                ui.hit(bx, cy, bw, 62.0, va(V::CSize(k)));
            }
            cy += 80.0;
            if crate::screens::next_button(ui, x, cy, w, "Continue", va(V::CNext(n))) {
                cy += 50.0;
            }
        }
        _ => {
            let half = (w - 12.0) / 2.0;
            let may = app.may_load_keys();
            // Its name, which names the file: what it holds, since the
            // file is all anyone who sees the stick sees (`docs/VAULT.md`
            // §6).
            ui.text_mid(x, cy, 36.0, 13.0, W::S, TEXT, "Name");
            cy += 34.0;
            cy += ui.wrap(
                x,
                cy,
                w,
                12.0,
                W::R,
                MUTED,
                &format!(
                    "What it holds, for anyone who sees the {} · leave empty for vault.ofv",
                    app.medium.noun()
                ),
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
            let file = format!("File {}.ofv", vaults::vault_stem(&c.name.text));
            if nw + 16.0 + ui.measure(13.0, W::R, &file) <= w {
                ui.text_mid(x + nw + 16.0, cy, 40.0, 13.0, W::R, MUTED, &file);
                cy += 56.0;
            } else {
                ui.text(x, cy + 48.0, 13.0, W::R, MUTED, &file);
                cy += 78.0;
            }
            if !may {
                let pull = format!(
                    "Remove the {}, then type the passphrases",
                    app.medium.noun()
                );
                cy += stick_banner(ui, x, cy, w, &pull);
            }
            ui.button(
                x,
                cy,
                None,
                32.0,
                if c.shown {
                    "Hide passphrases"
                } else {
                    "Show passphrases"
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
                    &format!("Passphrase {}", i + 1),
                );
                if i > 0 {
                    ui.button(
                        x + w - (ui.measure(13.0, W::S, "Remove") + 28.0),
                        cy,
                        None,
                        36.0,
                        "Remove",
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
                    "Generate with dice",
                    dice_style,
                    va(V::Dice(i)),
                );
                // Beside the button; on a small panel under it.
                if ui.compact {
                    cy += 46.0;
                    ui.text(x, cy, 13.0, W::R, MUTED, "or type your own below");
                    cy += 26.0;
                } else {
                    ui.text_mid(
                        x + dw + 14.0,
                        cy,
                        40.0,
                        13.0,
                        W::R,
                        MUTED,
                        "or type your own below",
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
                    ("Matches", OK)
                } else if !b.text.is_empty() {
                    ("Does not match yet", WARN)
                } else {
                    ("Type it, then type it again", DIM)
                };
                ui.text(x, cy, 12.0, W::R, color, status);
                cy += 22.0;
                // Its strength: the dice's bits and the cost's, when the
                // dice made it.
                let (_, mem, passes) = app.vaults.form_cost().unwrap_or((0, 64, 3));
                let cost = vaults::cost_bits(mem, passes);
                let strength = match vaults::Vaults::phrase_bits(c, i) {
                    Some(own) => Some((
                        format!(
                            "{own:.1} bits from dice + ≈{cost:.1} from the unlock cost ≈ {:.1} bits",
                            own + cost
                        ),
                        if own >= vaults::STRONG_BITS as f32 {
                            OK
                        } else {
                            MUTED
                        },
                    )),
                    None if !a.text.is_empty() => Some((
                        format!(
                            "Typed: its own bits are not measured · the unlock cost adds ≈{cost:.1}"
                        ),
                        DIM,
                    )),
                    None => None,
                };
                if let Some((line, tone)) = strength {
                    cy += ui.wrap(x, cy, w, 12.0, W::R, tone, &line);
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
                    "Add another passphrase",
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
        &format!(
            "Roll {} dice for each word and type the numbers",
            list.dice_per_word()
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
        ui.text(x, cy, 13.0, W::R, DIM, "No word yet");
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
    let status = format!(
        "{} {} · {own:.1} bits · ≈{:.1} with the unlock cost{}",
        words.len(),
        if words.len() == 1 { "word" } else { "words" },
        own + vaults::cost_bits(mem, passes),
        if left > 0 {
            format!(" · {left} of {per} dice for the next word")
        } else {
            String::new()
        }
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
    let uw = ui.button(x, cy, None, 36.0, "Use these words", style, va(V::DiceUse));
    ui.button(
        x + uw + 8.0,
        cy,
        None,
        36.0,
        "Cancel",
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
    ui.text(x, y, 13.0, W::S, MUTED, "Your choices");
    y += 28.0;
    for (k, label) in [
        "Opens on",
        "Unlock cost",
        "Space per passphrase",
        "Name and passphrases",
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
            "Not chosen yet".to_string()
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
    let (weakest, ram) = app.vaults.weakest();
    let name = if k == CUSTOM { "Custom" } else { PRESETS[k].0 };
    let mut warnings: Vec<(String, osk_ui::Color)> = Vec::new();
    if let Some(free) = app.vaults.memory_free_mib
        && mem + 100 > free
    {
        warnings.push((
            format!(
                "This computer cannot allocate {}. A vault it could not open is never written.",
                mib_text(mem)
            ),
            ERR,
        ));
    }
    if mem > ram / 2 {
        warnings.push((
            format!(
                "{name} needs {}, more than half of {weakest}'s memory.",
                mib_text(mem)
            ),
            WARN,
        ));
    }
    if app.vaults.seconds(mem, passes).is_some_and(|s| s > 10.0) {
        warnings.push((
            format!(
                "Unlocking takes {} here.",
                app.vaults.time_text(mem, passes)
            ),
            WARN,
        ));
    }
    if c.on[1..].iter().any(|on| *on) {
        warnings.push((
            "Raspberry Pi unlock times are not measured yet.".to_string(),
            MUTED,
        ));
    }
    y += 4.0;
    for (t, color) in &warnings {
        let th = ui.wrap(x + 12.0, y + 10.0, w - 24.0, 12.0, W::R, *color, t);
        ui.fill(x, y, w, th + 20.0, 8.0, color.with_alpha(26));
        ui.wrap(x + 12.0, y + 10.0, w - 24.0, 12.0, W::R, *color, t);
        y += th + 28.0;
    }
    y += 8.0;
    ui.text(x, y, 13.0, W::S, MUTED, "Will create");
    y += 26.0;
    let cost = vaults::cost_bits(mem, passes);
    // The weakest passphrase's strength, known when the dice made every
    // one.
    let own: Vec<Option<f32>> = (0..c.phrases.len())
        .map(|i| vaults::Vaults::phrase_bits(c, i))
        .collect();
    let strength = if c.phrases.iter().all(|(a, _)| a.text.is_empty()) {
        "No passphrase yet".to_string()
    } else if let Some(min) = own
        .iter()
        .map(|b| b.map(|b| b + cost))
        .collect::<Option<Vec<f32>>>()
        .and_then(|v| v.into_iter().reduce(f32::min))
    {
        format!("≈{min:.1} bits")
    } else {
        "Not measured".to_string()
    };
    let facts = [
        ("File", format!("{}.ofv", vaults::vault_stem(&c.name.text))),
        ("Size", file_text(file_len(c.slot))),
        ("Memory to open", format!("about {}", mib_text(mem + 100))),
        ("Unlock here", app.vaults.time_text(mem, passes)),
        ("Unlock cost adds", format!("≈{cost:.1} bits")),
        (
            if c.phrases.len() > 1 {
                "Weakest passphrase"
            } else {
                "Strength"
            },
            strength,
        ),
        ("Goes to", app.medium.for_box().to_string()),
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
            &format!("Creating · {}", app.vaults.time_text(mem, passes)),
        );
    } else if !crate::screens::pin_button(
        ui,
        x,
        by,
        Some(w),
        46.0,
        "Create vault",
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
            let fp = key_fingerprint(app, r).unwrap_or_else(|| "Key".to_string());
            let words = records::words_of(r)
                .map(|m| format!("{} words", m.word_count()))
                .unwrap_or_else(|| "Master seed".into());
            let loads = r
                .field(field::KEY_FLAGS)
                .is_some_and(|f| f.first().is_some_and(|b| b & records::LOAD_AT_UNLOCK != 0));
            let label = r.text(field::KEY_LABEL).unwrap_or("");
            let mut line = vec![words];
            if !label.is_empty() {
                line.push(label.to_string());
            }
            if loads {
                line.push("chosen at unlock".to_string());
            }
            (fp, line.join(" · "), true)
        }
        kind::WALLET => {
            let name = r.text(field::WALLET_NAME).unwrap_or("Wallet").to_string();
            (
                name,
                wallet_shape(r).unwrap_or_else(|| "Wallet".into()),
                false,
            )
        }
        kind::ENTRY => {
            let mut parts = Vec::new();
            if r.field(field::PASSWORD).is_some() {
                parts.push("Password");
            }
            if r.field(field::TOTP).is_some() {
                parts.push("TOTP");
            }
            if parts.is_empty() {
                parts.push("Entry");
            }
            (
                r.text(field::TITLE).unwrap_or("Entry").to_string(),
                parts.join(" · "),
                false,
            )
        }
        kind::NOTE => {
            let t = r.text(field::NOTE).unwrap_or("");
            let first: String = t.lines().next().unwrap_or("").chars().take(40).collect();
            let first = if first.is_empty() {
                "Note".to_string()
            } else {
                first
            };
            (
                first,
                format!("Note · {} characters", t.chars().count()),
                false,
            )
        }
        kind::SHEET => {
            let (_, name, _) = sheet_parts(r);
            (
                if name.is_empty() {
                    "Recovery sheet".into()
                } else {
                    name.clone()
                },
                format!("Recovery sheet · {name}"),
                false,
            )
        }
        kind::GPG => {
            let line = match crate::gpg::shown(r) {
                Some(g) if g.expires == "Never" => "Ed25519 · does not expire".to_string(),
                Some(g) => format!("Ed25519 · expires {}", g.expires),
                None => "Ed25519".to_string(),
            };
            (r.text(5).unwrap_or("GPG key").to_string(), line, false)
        }
        kind::SECURE_BOOT => ("Owner keys".to_string(), "PK, KEK, db".to_string(), false),
        _ => ("Record".to_string(), String::new(), false),
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
    Some(Session::shape(&Wallet {
        name: String::new(),
        policy,
        source: String::new(),
    }))
}

/// A key record's fingerprint, worked out once per key and kept.
pub(crate) fn key_fingerprint(app: &Faraday, r: &Record) -> Option<String> {
    use osk_bip::bitcoin::hashes::{Hash, sha256};
    let payload = r.field(field::KEY)?;
    let id = sha256::Hash::hash(payload).to_byte_array();
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

fn words_text(r: &Record) -> Option<String> {
    let m = records::words_of(r)?;
    let lang = m.language();
    Some(
        m.indices()
            .iter()
            .map(|i| lang.word(*i))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

pub(crate) fn contents(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return contents_compact(app, ui, x0, cw, h);
    }
    let x = x0 + 40.0;
    let w = cw - 80.0;
    let mut y = 28.0;
    back_link(ui, x, y, "Vaults", Action::Nav(Screen::Vaults));
    y += 26.0;
    let Some(v) = app.vaults.open.get(app.vaults.current) else {
        title(ui, x, y, "Vault contents");
        ui.text(x, y + 56.0, 15.0, W::R, MUTED, "No vault is open");
        if let Some((label, a)) = app.vault_way(Screen::VaultContents) {
            ui.button(x, y + 90.0, None, 44.0, &label, Style::Primary, a);
        }
        return;
    };
    title(ui, x, y, &v.label());
    let lw = ui.measure(28.0, W::S, &v.label());
    let state = if v.changes == 0 {
        "Open".to_string()
    } else {
        format!(
            "Open · {} unsaved {}",
            v.changes,
            if v.changes == 1 { "change" } else { "changes" }
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
    let lockw = ui.measure(14.0, W::S, "Lock") + 36.0;
    ui.button(
        x + w - lockw,
        y - 2.0,
        Some(lockw),
        40.0,
        "Lock",
        Style::Primary,
        Action::LockAsk,
    );
    let rw = ui.measure(13.0, W::S, "Rename") + 28.0;
    ui.button(
        x + w - lockw - 8.0 - rw,
        y + 2.0,
        Some(rw),
        34.0,
        "Rename",
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
        &format!(
            "Secret values show only while held. Changes stay in this session until it locks; locking \
             seals them, and the next {} visit writes the vault back over its own file.",
            app.medium.noun()
        ),
    );

    // Three columns: kinds, items, the item. A wallet's chart takes the
    // room the other two give up (`docs/NEW-WALLET.md` §7.2).
    let chart_shown = !app.vaults.add_menu
        && app
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
    // The kinds it holds, then Add….
    for (k, label, count) in listed_kinds(app, v) {
        let on = k == app.vaults.category && !app.vaults.add_menu;
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
    if app.vaults.add_menu {
        ui.fill(x, ky, kinds_w, 38.0, 8.0, ACCENT.with_alpha(30));
    }
    ui.text_mid(
        x + 12.0,
        ky,
        38.0,
        14.0,
        W::S,
        if app.vaults.add_menu { TEXT } else { MUTED },
        ADD_ROW,
    );
    ui.hit(x, ky, kinds_w, 38.0, va(V::AddMenu));

    let ix = x + kinds_w + 16.0;
    if app.vaults.add_menu {
        add_list(ui, ix, top, items_w);
        return;
    }
    if shows_empty(app, v) {
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
            extra.push(("Scan a code".to_string(), V::ScanEntry));
            if entries_files > 0 {
                extra.push((
                    format!("Import from Files ({entries_files})"),
                    V::ImportAllEntries,
                ));
            }
        }
        0 | 3 => {
            for k in &backups {
                extra.push((
                    format!("Import {}", app.inbox[*k].name),
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
        ui.text(ix, iy, 13.0, W::R, DIM, "None in this vault");
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

/// The contents' row that lists every kind to add.
const ADD_ROW: &str = "Add…";
/// What an open vault with nothing in it says.
const NOTHING_YET: &str = "Nothing in it yet";

/// What adds an item of kind `k`, as its category's Add button reads.
fn add_label(k: usize) -> Option<&'static str> {
    match k {
        0 => Some("Save a key"),
        1 => Some("Save a wallet"),
        2 => Some("Add an entry"),
        3 => Some("Add a note"),
        4 => Some("Make a key"),
        5 => Some("Make keys"),
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
        .filter(|&(k, _, n)| {
            n > 0
                || (k == app.vaults.category && app.vaults.category_chosen && !app.vaults.add_menu)
        })
        .collect()
}

/// Whether the contents show an empty vault's next steps: nothing in it,
/// and nothing chosen or under way.
fn shows_empty(app: &Faraday, v: &vaults::OpenVault) -> bool {
    let held = CATEGORIES
        .iter()
        .any(|(_, kinds)| v.contents.records.iter().any(|r| kinds.contains(&r.kind)));
    !held && !app.vaults.category_chosen && !app.vaults.add_menu && !contents_detail(app)
}

/// An empty vault's next steps.
fn empty_buttons(app: &Faraday) -> [(String, Style, Action); 2] {
    [
        (
            "Put a wallet in it".to_string(),
            Style::Primary,
            va(V::PutWallet),
        ),
        (
            format!("Write it to {}", app.medium.a()),
            Style::Secondary,
            va(V::WriteOut),
        ),
    ]
}

/// The Add… list at (x, y), w wide: every kind, each with its Add action.
/// Returns its height.
fn add_list(ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let mut cy = y;
    for (k, (label, _)) in CATEGORIES.iter().enumerate() {
        let Some(add) = add_label(k) else {
            continue;
        };
        ui.fill(x, cy, w, 52.0, 8.0, BG);
        ui.stroke(x, cy, w, 52.0, 8.0, LINE);
        let t = ui.fit(14.0, W::S, add, w - 24.0);
        ui.text(x + 12.0, cy + 8.0, 14.0, W::S, TEXT, &t);
        let l = ui.fit(12.0, W::R, label, w - 24.0);
        ui.text(x + 12.0, cy + 30.0, 12.0, W::R, MUTED, &l);
        ui.hit(x, cy, w, 52.0, va(V::AddKind(k)));
        cy += 58.0;
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
fn contents_compact(app: &Faraday, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    use crate::compact::M;
    use crate::compact_screens::{about, finish, row};
    use crate::screens::wrap_buttons;
    let (x, w) = (x0 + M, cw - 2.0 * M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    let Some(v) = app.vaults.open.get(app.vaults.current) else {
        ui.text(x, y, 15.0, W::R, MUTED, "No vault is open");
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
        "Open".to_string()
    } else {
        format!(
            "Open · {} unsaved {}",
            v.changes,
            if v.changes == 1 { "change" } else { "changes" }
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
        "Rename",
        Style::Secondary,
        va(V::Rename),
    );
    ui.button(
        x + half + 8.0,
        y,
        Some(half),
        40.0,
        "Lock",
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
    // The kinds it holds, each with how many, then Add….
    let listed = listed_kinds(app, v);
    let kind_labels: Vec<(usize, String)> = listed
        .iter()
        .map(|(k, label, n)| (*k, format!("{label} {n}")))
        .collect();
    let mut kind_items: Vec<(&str, Style, Action)> = kind_labels
        .iter()
        .map(|(k, l)| {
            let style = if *k == app.vaults.category && !app.vaults.add_menu {
                Style::Primary
            } else {
                Style::Secondary
            };
            (l.as_str(), style, va(V::Category(*k)))
        })
        .collect();
    kind_items.push((
        ADD_ROW,
        if app.vaults.add_menu {
            Style::Primary
        } else {
            Style::Secondary
        },
        va(V::AddMenu),
    ));
    y += wrap_buttons(ui, x, y, w, 34.0, &kind_items) + 8.0;
    ui.rule(x, y, w, LINE);
    y += 14.0;
    if app.vaults.add_menu {
        y += add_list(ui, x, y, w) + 8.0;
        finish(app, ui, x0, cw, h, y - top + 16.0);
        return;
    }
    if shows_empty(app, v) {
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
            extra.push(("Scan a code".to_string(), V::ScanEntry));
            if entries_files > 0 {
                extra.push((
                    format!("Import from Files ({entries_files})"),
                    V::ImportAllEntries,
                ));
            }
        }
        0 | 3 => {
            for (k, i) in app.inbox.iter().enumerate() {
                if i.kind == crate::FileKind::Backup {
                    extra.push((format!("Import {}", i.name), V::ImportBackup(k)));
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
    for (k, r) in items.iter().enumerate() {
        let (t, line, _) = item_heading(app, r);
        y += row(ui, x, y, w, None, &t, &line, MUTED, Some(va(V::Item(k))));
    }
    if items.is_empty() {
        ui.text(x, y, 13.0, W::R, DIM, "None in this vault");
        y += 30.0;
    }
    y += about(
        ui,
        x,
        y,
        w,
        &format!(
            "Secret values show only while held. Changes stay in this session until it locks; locking \
             seals them, and the next {} visit writes the vault back over its own file.",
            app.medium.noun()
        ),
    );
    finish(app, ui, x0, cw, h, y - top + 16.0);
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
        let heading = match (form.kind, form.record) {
            (kind::SLOT_LABEL, _) => "Rename the vault",
            (kind::NOTE, None) => "Add a note",
            (kind::NOTE, Some(_)) => "Edit the note",
            (kind::GPG, _) => "Make a key",
            (kind::SECURE_BOOT, _) => "Make Secure Boot keys",

            (_, None) => "Add an entry",
            _ => "Edit the entry",
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
        if form.kind == kind::GPG {
            ui.text(dx, dy, 12.0, W::R, MUTED, "Expires");
            dy += 22.0;
            dy += years_row(app, ui, dx, dy, dw) + 16.0;
        }
        if app.vaults.working == Some(vaults::Work::SecureBoot) {
            ui.text_mid(dx, dy, 40.0, 15.0, W::S, ACCENT, "Making PK, KEK and db");
            return dy;
        }
        let save = match form.kind {
            kind::GPG => "Make the key",
            kind::SECURE_BOOT => "Make the keys",
            _ => "Save",
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
            "Cancel",
            Style::Ghost,
            va(V::FormCancel),
        );
        return dy;
    }
    if let Some(p) = &app.vaults.prompt {
        let heading = match p.purpose {
            vaults::Purpose::Backup(k) => format!(
                "Open {}",
                app.inbox.get(k).map_or("the backup", |i| i.name.as_str())
            ),
            vaults::Purpose::KeyPassphrase => "Load with a BIP-39 passphrase".to_string(),
            vaults::Purpose::Kdbx => "Export for KeePass".to_string(),
        };
        let heading = ui.fit(18.0, W::S, &heading, dw);
        ui.text(dx, dy, 18.0, W::S, TEXT, &heading);
        dy += 40.0;
        ui.text(dx, dy, 12.0, W::R, MUTED, "Passphrase");
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
            ui.text_mid(dx, dy, 40.0, 15.0, W::S, ACCENT, "Opening");
        } else if app.vaults.working == Some(vaults::Work::Kdbx) {
            ui.text_mid(dx, dy, 40.0, 15.0, W::S, ACCENT, "Sealing");
        } else {
            let label = match p.purpose {
                vaults::Purpose::Backup(_) => "Open",
                vaults::Purpose::Kdbx => "Seal and export",
                vaults::Purpose::KeyPassphrase => "Load",
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
                "Cancel",
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
    if app.vaults.signing {
        ui.text(dx, dy, 18.0, W::S, TEXT, "Sign a file");
        dy += 40.0;
        let files: Vec<(usize, &crate::Item)> = app.inbox.iter().enumerate().collect();
        if files.is_empty() {
            ui.text(
                dx,
                dy,
                13.0,
                W::R,
                DIM,
                &format!("No file {}", app.medium.from_the()),
            );
        }
        for (k, it) in files {
            // Sign puts the signature in the Outbox; beside it, the
            // signature as a code and as a picture of it.
            let buttons = [
                ("Show as QR", Action::ShowCode(Code::GpgSignature(k))),
                ("PNG", Action::CodePng(Code::GpgSignature(k))),
                ("Sign", va(V::GpgSign(k))),
            ];
            let widths: Vec<f32> = buttons
                .iter()
                .map(|(l, _)| ui.measure(13.0, W::S, l) + 30.0)
                .collect();
            let all = widths.iter().sum::<f32>() + 12.0;
            let (name_w, by) = if ui.compact {
                (dw, dy + 40.0)
            } else {
                (dw - all - 16.0, dy + 2.0)
            };
            let name = ui.fit(14.0, W::M, &it.name, name_w);
            ui.text_mid(dx, dy, 40.0, 14.0, W::M, TEXT, &name);
            let mut bx = if ui.compact { dx } else { dx + dw - all };
            for ((label, action), bw) in buttons.iter().zip(&widths) {
                ui.button(bx, by, Some(*bw), 36.0, label, Style::Secondary, *action);
                bx += bw + 6.0;
            }
            let rh = if ui.compact { 84.0 } else { 44.0 };
            ui.rule(dx, dy + rh, dw, INNER);
            dy += rh + 4.0;
        }
        dy += 8.0;
        ui.button(
            dx,
            dy,
            None,
            36.0,
            "Back to the key",
            Style::Ghost,
            va(V::GpgSignPick),
        );
        return dy;
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
            field_row(ui, &mut dy, "Fingerprint", &fp, true);
            if let Some(words) = words_text(r) {
                hold_row(ui, &mut dy, "Words", &words, true);
            }
            let loads = r
                .field(field::KEY_FLAGS)
                .is_some_and(|f| f.first().is_some_and(|b| b & records::LOAD_AT_UNLOCK != 0));
            field_row(
                ui,
                &mut dy,
                "BIP-39 passphrase",
                if r.field(field::KEY_PASSPHRASE).is_some() {
                    "Stored"
                } else {
                    "Not stored"
                },
                false,
            );
            field_row(
                ui,
                &mut dy,
                "Chosen at unlock",
                if loads { "Yes" } else { "No" },
                false,
            );
            let here = app
                .session
                .keys
                .iter()
                .any(|k| fp_text(k.master.fingerprint()) == fp);
            field_row(
                ui,
                &mut dy,
                "In this session",
                if here { "Loaded" } else { "Not loaded" },
                false,
            );
            if !here {
                actions.push(("Load into session", va(V::Load)));
                if records::words_of(r).is_some() && r.field(field::KEY_PASSPHRASE).is_none() {
                    actions.push(("Load with a passphrase", va(V::LoadWithPassphrase)));
                }
            }
            actions.push((
                if loads {
                    "Leave unchosen at unlock"
                } else {
                    "Choose at unlock"
                },
                va(V::ToggleLoad),
            ));
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
            field_row(ui, &mut dy, "Descriptor", text, true);
            actions.push(("Open in Wallets", va(V::Load)));
        }
        kind::ENTRY => {
            if let Some(u) = r.text(field::USERNAME) {
                field_row(ui, &mut dy, "Username", u, false);
            }
            if let Some(p) = r.text(field::PASSWORD) {
                hold_row(ui, &mut dy, "Password", p, true);
            }
            if let Some(u) = r.text(field::URL) {
                field_row(ui, &mut dy, "URL", u, true);
            }
            if let Some(t) = r.text(field::TOTP) {
                hold_row(ui, &mut dy, "TOTP secret", t, true);
            }
            if let Some(n) = r.text(field::NOTES) {
                field_row(ui, &mut dy, "Notes", n, false);
            }
            actions.push(("Edit", va(V::Edit)));
            actions.push(("Export for KeePass", va(V::ExportKdbx)));
        }
        kind::NOTE => {
            hold_row(
                ui,
                &mut dy,
                "Text",
                r.text(field::NOTE).unwrap_or(""),
                false,
            );
            actions.push(("Edit", va(V::Edit)));
        }
        kind::SHEET => {
            let (d, _, note) = sheet_parts(r);
            field_row(ui, &mut dy, "Descriptor", &d, true);
            if !note.is_empty() {
                field_row(ui, &mut dy, "Note", &note, false);
            }
        }
        kind::GPG => {
            if let Some(g) = crate::gpg::shown(r) {
                field_row(ui, &mut dy, "Fingerprint", &g.fingerprint, true);
                field_row(ui, &mut dy, "Signing subkey", &g.subkey, true);
                for uid in &g.user_ids {
                    field_row(ui, &mut dy, "User ID", uid, false);
                }
                let valid = if g.expires == "Never" {
                    format!("From {} · does not expire", g.created)
                } else {
                    format!("{} to {}", g.created, g.expires)
                };
                field_row(ui, &mut dy, "Valid", &valid, false);
                hold_row(ui, &mut dy, "Paperkey", &g.paperkey, true);
                ui.text(dx, dy, 12.0, W::R, MUTED, "Renew for");
                dy += 22.0;
                // Renew at the row's end; on a small panel under it.
                let rw = ui.measure(13.0, W::S, "Renew") + 30.0;
                let yh = years_row(app, ui, dx, dy, if ui.compact { dw } else { dw - rw - 8.0 });
                let ry = if ui.compact { dy + yh + 8.0 } else { dy };
                ui.button(
                    dx + dw - rw,
                    ry,
                    Some(rw),
                    32.0,
                    "Renew",
                    Style::Secondary,
                    va(V::GpgRenew),
                );
                dy = ry + 46.0;
            }
            // Each file on a row of its own: to the Outbox, as a code,
            // as a picture.
            // On a small panel the code and the picture go under the
            // file's own button.
            breaks.push(actions.len());
            actions.push(("Export public key", va(V::GpgExport)));
            if ui.compact {
                breaks.push(actions.len());
            }
            actions.push(("Show as QR", Action::ShowCode(Code::GpgKey)));
            actions.push(("PNG", Action::CodePng(Code::GpgKey)));
            breaks.push(actions.len());
            actions.push(("Revocation certificate", va(V::GpgRevoke)));
            if ui.compact {
                breaks.push(actions.len());
            }
            actions.push(("Show as QR", Action::ShowCode(Code::GpgRevocation)));
            actions.push(("PNG", Action::CodePng(Code::GpgRevocation)));
            breaks.push(actions.len());
            actions.push(("Sign a file", va(V::GpgSignPick)));
        }
        kind::SECURE_BOOT => {
            if let Some(sb) = crate::secureboot::shown(r) {
                field_row(ui, &mut dy, "Owner GUID", &sb.owner, true);
                for (label, name) in ["PK", "KEK", "db"].iter().zip(&sb.names) {
                    field_row(ui, &mut dy, label, name, false);
                }
            }
            ui.text(dx, dy, 12.0, W::R, MUTED, "Enrolment adds");
            dy += 22.0;
            let own_items: Vec<(&str, Style, Action)> =
                [("Windows-compatible", false), ("Own keys only", true)]
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
                "Your PK, KEK and db only. Windows will not start, nor any card or controller whose firmware Microsoft signed"
            } else {
                "Yours, with Microsoft's KEK CAs and Windows CAs. Not Microsoft's third-party CA: a card or controller whose firmware it signed may not start"
            };
            dy += ui.wrap(dx, dy, dw, 12.0, W::R, MUTED, line) + 14.0;
            actions.push(("Enrolment files", va(V::SbEnrol)));
            actions.push(("Sign or check an image", va(V::SbImages)));
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
    hold_delete(app, ui, dx, dy);
    dy + 48.0
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
        if keys { "Save a key" } else { "Save a wallet" },
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
            let fp = fp_text(key.master.fingerprint());
            if held.contains(&fp) || key.words.is_none() {
                continue;
            }
            any = true;
            ui.text_mid(dx, dy, 40.0, 14.0, W::M, TEXT, &fp);
            let label = if key.passphrase.is_some() {
                format!("{} · BIP-39 passphrase", key.label)
            } else {
                key.label.clone()
            };
            if ui.compact {
                // The key and its name, then its buttons under them.
                let label = ui.fit(13.0, W::R, &label, dw - 100.0);
                ui.text_mid(dx + 100.0, dy, 40.0, 13.0, W::R, MUTED, &label);
                dy += 42.0;
                let mut items = vec![("Save", Style::Secondary, va(V::SaveKey(k)))];
                if key.passphrase.is_some() {
                    items.push((
                        "Save with its passphrase",
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
                "Save",
                Style::Secondary,
                va(V::SaveKey(k)),
            );
            if key.passphrase.is_some() {
                let pw = ui.measure(13.0, W::S, "Save with its passphrase") + 26.0;
                ui.button(
                    dx + dw - 88.0 - pw,
                    dy + 2.0,
                    Some(pw),
                    36.0,
                    "Save with its passphrase",
                    Style::Secondary,
                    va(V::SaveKeyWithPassphrase(k)),
                );
            }
            ui.rule(dx, dy + 44.0, dw, INNER);
            dy += 48.0;
        }
        if !any {
            ui.text(
                dx,
                dy,
                13.0,
                W::R,
                DIM,
                "Every key in this session is in the vault",
            );
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
            if held.contains(&wl.policy.to_descriptor()) {
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
                    "Save",
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
                "Save",
                Style::Secondary,
                va(V::SaveWallet(k)),
            );
            ui.rule(dx, dy + 44.0, dw, INNER);
            dy += 48.0;
        }
        if !any {
            ui.text(
                dx,
                dy,
                13.0,
                W::R,
                DIM,
                "Every wallet in this session is in the vault",
            );
            dy += 30.0;
        }
    }
    dy
}

/// Lock, asked for while a vault is open: what is sealed into the Outbox
/// and what is in the session but in no open vault.
pub(crate) fn lock_ask(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
    let (keys, wallets) = app.unsaved();
    let sealed: Vec<String> = app
        .vaults
        .open
        .iter()
        .filter(|v| v.changes > 0)
        .map(|v| v.label())
        .collect();
    let rows: Vec<(&str, String)> = [
        ("Sealed", sealed.join(", ")),
        (
            "Keys not saved",
            if keys.is_empty() {
                String::new()
            } else {
                format!("{} · {}", keys.len(), keys.join(", "))
            },
        ),
        (
            "Wallets not saved",
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
        let rows: Vec<(&str, String, osk_ui::Color)> =
            rows.into_iter().map(|(k, v)| (k, v, TEXT)).collect();
        crate::compact::kv_sheet(
            ui,
            w,
            h,
            (Icon::Lock, ACCENT, "Lock"),
            "Everything not in a vault is wiped with the session",
            &rows,
            &[
                ("Back to the vault", Style::Secondary, Action::Cancel),
                ("Lock", Style::Primary, Action::Lock),
            ],
        );
        return;
    }
    let sh = 210.0 + rows.len() as f32 * 44.0;
    let (x, y) = crate::screens::sheet_box(ui, w, h, 560.0, sh);
    let ix = x + 32.0;
    let iw = 560.0 - 64.0;
    ui.icon(ix, y + 30.0, 30.0, Icon::Lock, 18.0, ACCENT);
    ui.text_mid(ix + 42.0, y + 30.0, 30.0, 20.0, W::S, TEXT, "Lock");
    ui.text(
        ix,
        y + 76.0,
        13.0,
        W::R,
        MUTED,
        "Everything not in a vault is wiped with the session",
    );
    let mut ry = y + 106.0;
    for (k, v) in &rows {
        ui.text_mid(ix, ry, 40.0, 13.0, W::R, MUTED, k);
        let v = ui.fit(14.0, W::R, v, iw - 150.0);
        ui.text_mid(ix + 150.0, ry, 40.0, 14.0, W::R, TEXT, &v);
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
        "Back to the vault",
        Style::Secondary,
        Action::Cancel,
    );
    ui.button(
        ix + bw + 12.0,
        by,
        Some(bw),
        46.0,
        "Lock",
        Style::Primary,
        Action::Lock,
    );
}

/// The expiry choices for a GPG key. Returns the row's height.
fn years_row(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let items: Vec<(&str, Style, Action)> = [
        ("1 year", 1u32),
        ("2 years", 2),
        ("5 years", 5),
        ("Never", 0),
    ]
    .into_iter()
    .map(|(label, years)| {
        let style = if app.vaults.gpg_years == years {
            Style::Primary
        } else {
            Style::Secondary
        };
        (label, style, va(V::GpgYears(years)))
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
        ui.text(dx, dy, 18.0, W::S, TEXT, &format!("Sign {}", item.name));
        dy += 40.0;
        ui.text(dx, dy, 12.0, W::R, MUTED, "SHA-256");
        dy += 20.0;
        let hash = crate::secureboot::image_sha256(&item.bytes);
        dy += ui.wrap(dx, dy, dw, 15.0, W::M, TEXT, &hash) + 14.0;
        dy += ui.wrap(
            dx,
            dy,
            dw,
            13.0,
            W::R,
            MUTED,
            "Compare it with the build record whose fingerprint you checked yourself",
        ) + 16.0;
        let hw = hold_button(app, ui, dx, dy, "Hold to sign", va(V::SbHoldSign), ACCENT);
        ui.button(
            dx + hw + 10.0,
            dy,
            None,
            36.0,
            "Back",
            Style::Ghost,
            va(V::SbImages),
        );
        return dy + 48.0;
    }
    ui.text(dx, dy, 18.0, W::S, TEXT, "Sign or check an image");
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
            &format!("No EFI image {}", app.medium.from_the()),
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
        ui.button(
            dx + dw - 80.0,
            dy + 2.0,
            Some(80.0),
            36.0,
            "Sign",
            Style::Secondary,
            va(V::SbPick(k)),
        );
        ui.button(
            dx + dw - 168.0,
            dy + 2.0,
            Some(80.0),
            36.0,
            "Check",
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
        "Back to the keys",
        Style::Ghost,
        va(V::SbImages),
    );
    dy + 56.0
}

/// Whether open vault `v` holds a wallet with the same descriptor.
pub(crate) fn vault_has_wallet(app: &Faraday, v: usize, w: &Wallet) -> bool {
    let want = crate::wallet::same_wallet(&w.policy);
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
