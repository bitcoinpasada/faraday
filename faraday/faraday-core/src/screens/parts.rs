//! Parts two or more screens draw: rows, buttons, fields and word cells.

use crate::drawing::View;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{CoreUi, Style, Ui, W};
use crate::wallet::{FileKind, Session};
use crate::{Action, Faraday};
use faraday_draw::tokens;
use faraday_draw::widgets::Icon;
use faraday_ui::parts::{section_label, wrap_buttons};

/// A screen's walk-through in Guided mode, under its title. Returns the
/// height it took: nothing in Steps only.
pub(crate) fn guide_text(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32, text: &str) -> f32 {
    if !app.guided {
        return 0.0;
    }
    let h = ui.wrap(x + 14.0, y, w - 14.0, 14.0, W::R, TEXT, text);
    ui.fill(x, y, 3.0, h, 1.5, ACCENT.with_alpha(140));
    h + 18.0
}

/// The caution under a field where a seed or a secret is typed or
/// pasted, on an online shell only: a statement of fact in the caution
/// tone, in every mode. Returns the height it took, with the gap under
/// it; nothing on the stick or the Pi.
pub(crate) fn never_real_seed(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    never_real_seed_if(true, app, ui, x, y, w)
}

/// The line one gap under a one-line text field whose top is `top`, on
/// an online shell. Returns the gap and the line's height; 0 elsewhere.
pub(crate) fn never_under(app: &View, ui: &mut Ui, x: f32, top: f32, w: f32) -> f32 {
    never_below(app, ui, x, top + tokens::TEXT_FIELD, w)
}

/// The line one gap under a field whose bottom edge is at `bottom`, on an
/// online shell. Returns the gap and the line's height; 0 elsewhere.
pub(crate) fn never_below(app: &View, ui: &mut Ui, x: f32, bottom: f32, w: f32) -> f32 {
    if app.online {
        grid_never(ui, x, bottom, w)
    } else {
        0.0
    }
}

/// [`never_real_seed`], only where `typed` says the field can be typed
/// into (not while a stick is attached).
pub(crate) fn never_real_seed_if(
    typed: bool,
    app: &View,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    if typed {
        never_under(app, ui, x, y, w)
    } else {
        0.0
    }
}

/// Whether the seeds loaded here sign for `w` on their own (2), some of
/// its seeds are here (1), or none (0).
pub(crate) fn sign_readiness(app: &Faraday, w: &crate::wallet::Wallet) -> u8 {
    let here = app
        .session
        .slots(w)
        .iter()
        .filter(|s| s.held_by.is_some())
        .count();
    if here >= crate::wallet::needed(w) {
        2
    } else if here > 0 {
        1
    } else {
        0
    }
}

/// Hex in groups of sixteen, so a long run wraps.
pub(crate) fn spaced(hex: &str) -> String {
    hex.as_bytes()
        .chunks(16)
        .map(|c| String::from_utf8_lossy(c).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// What the Wallets tab offers for a key in no wallet.
pub(crate) const FROM_KEY: &str = EN.app_make_a_wallet_from;

/// The side panel beside a step-by-step column (Spend, Back up, Create a
/// vault): 320 wide in a full-size window, down to 240 as the window
/// narrows, so that the column keeps its room.
pub(crate) fn side_panel_w(cw: f32) -> f32 {
    (cw * 0.31).clamp(240.0, 320.0)
}

/// The labels of the rows ticked, joined; "None" when none is.
pub(crate) fn ticked(labels: &[&str], on: &[bool]) -> String {
    let v: Vec<&str> = labels
        .iter()
        .zip(on)
        .filter(|(_, o)| **o)
        .map(|(l, _)| *l)
        .collect();
    if v.is_empty() {
        EN.ticked_none.to_string()
    } else {
        v.join(" · ")
    }
}

/// The page's forward action, `h` high, `w` wide or as wide as its
/// label: on a small panel pinned at its foot instead. Returns whether it
/// was drawn here, so the caller makes room for it only then.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub(crate) fn pin_button(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: Option<f32>,
    h: f32,
    label: &str,
    style: Style,
    action: Action,
) -> bool {
    if ui.pinning {
        ui.pin = Some((label.to_string(), style, action));
        return false;
    }
    ui.button(x, y, w, h, label, style, action);
    true
}

/// A step's next, at the column's right end; on a small panel pinned at
/// its foot instead. Returns whether it was drawn here, so the caller
/// makes room for it only then.
#[must_use]
pub(crate) fn next_button(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    action: Action,
) -> bool {
    if ui.pinning {
        ui.pin = Some((label.to_string(), Style::Primary, action));
        return false;
    }
    let bw = ui.measure(14.0, W::S, label) + 36.0;
    ui.button(x + w - bw, y, Some(bw), 40.0, label, Style::Primary, action);
    true
}

/// Buttons on the left and the step's next on the right, on one row; on
/// a small panel too narrow for both, the next goes on a row of its own
/// under them. Returns the height used.
pub(crate) fn buttons_and_next(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    left: &[(&str, Style, Action)],
    next: Option<(&str, Action)>,
) -> f32 {
    // Pinned at the foot: only the other buttons are here.
    let next = match next {
        Some((label, action)) if ui.pinning => {
            ui.pin = Some((label.to_string(), Style::Primary, action));
            None
        }
        n => n,
    };
    if left.is_empty() && next.is_none() {
        return 0.0;
    }
    let nw = next.map_or(0.0, |(l, _)| ui.measure(14.0, W::S, l) + 36.0);
    if ui.compact {
        // The buttons wrap to the panel's width, each cut to fit; the
        // next goes on their last row when there is room, else under it.
        let (mut bx, mut by) = (x, y);
        for &(label, style, action) in left {
            let bw = (ui.measure(14.0, W::S, label) + 32.0).min(w);
            if bx > x && bx + bw > x + w {
                bx = x;
                by += 48.0;
            }
            let label = ui.fit(14.0, W::S, label, bw - 24.0);
            bx += ui.button(bx, by, Some(bw), 40.0, &label, style, action) + 6.0;
        }
        if next.is_some() && !left.is_empty() && bx + nw + 2.0 > x + w {
            by += 48.0;
        }
        if let Some((label, action)) = next {
            let _ = next_button(ui, x, by, w, label, action);
        }
        return by - y + 48.0;
    }
    let lw: f32 = left
        .iter()
        .map(|(l, ..)| ui.measure(14.0, W::S, l) + 32.0 + 6.0)
        .sum();
    let mut bx = x;
    for &(label, style, action) in left {
        bx += ui.button(bx, y, None, 40.0, label, style, action) + 6.0;
    }
    let below = !left.is_empty() && lw + nw + 8.0 > w;
    let ny = if below { y + 48.0 } else { y };
    if let Some((label, action)) = next {
        let _ = next_button(ui, x, ny, w, label, action);
    }
    ny - y + 48.0
}

/// A labelled count with − and + beside it. On a small panel the buttons
/// sit at the row's right end.
#[allow(clippy::too_many_arguments)]
pub(crate) fn stepper(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: usize,
    minus: Action,
    plus: Action,
    enabled: bool,
) {
    let style = if enabled {
        Style::Secondary
    } else {
        Style::Disabled
    };
    let bx = if ui.compact { x + w - 124.0 } else { x + 180.0 };
    ui.text_mid(x, y, 40.0, 13.0, W::R, MUTED, label);
    ui.button(bx, y, Some(40.0), 40.0, "-", style, minus);
    let v = value.to_string();
    let vw = ui.measure(18.0, W::S, &v);
    ui.text_mid(bx + 62.0 - vw / 2.0, y, 40.0, 18.0, W::S, TEXT, &v);
    ui.button(bx + 84.0, y, Some(40.0), 40.0, "+", style, plus);
}

/// The closed line of a card that asks about vaults: the answer, as the
/// person gave it.
pub(super) fn vaults_answer_line(ask: crate::VaultAsk) -> String {
    match ask.answer {
        Some(true) => EN.plan_yes.to_string(),
        Some(false) => EN.plan_no.to_string(),
        None => String::new(),
    }
}

/// A Kind card's rows: Single key and Multisig, both native SegWit, the
/// two everyday kinds, and the other eight behind **More kinds** until
/// `more`. Create's Kind and Restore's draw it. Returns the height used.
#[allow(clippy::too_many_arguments)]
pub(crate) fn kind_rows(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    chosen: crate::create::NewKind,
    more: bool,
    enabled: bool,
    pick: fn(u8) -> Action,
    more_action: Action,
) -> f32 {
    use crate::create::NewKind;
    let mut cy = y;
    let shown: Vec<usize> = if more {
        (0..NewKind::ALL.len()).collect()
    } else {
        vec![0, 4]
    };
    for &i in &shown {
        let k = &NewKind::ALL[i];
        let on = chosen == *k;
        ui.fill(
            x,
            cy,
            w,
            48.0,
            8.0,
            if on { ACCENT.with_alpha(26) } else { BG },
        );
        ui.stroke(
            x,
            cy,
            w,
            48.0,
            8.0,
            if on { ACCENT.with_alpha(110) } else { INNER },
        );
        if ui.compact {
            let name = ui.fit(14.0, W::S, k.name(), w - 28.0);
            ui.text(x + 14.0, cy + 6.0, 14.0, W::S, TEXT, &name);
            let line = ui.fit(12.0, W::R, k.line(), w - 28.0);
            ui.text(x + 14.0, cy + 26.0, 12.0, W::R, MUTED, &line);
        } else {
            ui.text_mid(x + 14.0, cy, 48.0, 14.0, W::S, TEXT, k.name());
            let line = ui.fit(12.0, W::R, k.line(), w - 260.0);
            ui.text_right(x + w - 14.0, cy, 48.0, 12.0, W::R, MUTED, &line);
        }
        if enabled {
            ui.hit(x, cy, w, 48.0, pick(i as u8));
        }
        cy += 54.0;
    }
    if !more && enabled {
        ui.button(
            x,
            cy,
            None,
            34.0,
            EN.kind_rows_more_kinds,
            Style::Ghost,
            more_action,
        );
        cy += 44.0;
    }
    cy - y
}

/// A wallet's first two receive addresses and first change address,
/// under a label, as Create's Check and Restore's show them, with the
/// warning while a key made here waits for its backup. Returns the
/// height used.
pub(crate) fn first_addresses(app: &View, ui: &mut Ui, i: usize, x: f32, y: f32, w: f32) -> f32 {
    let Some(wl) = app.session.wallets.get(i) else {
        return 0.0;
    };
    let mut cy = y;
    // So that a multisig's three rows do not read as one address per
    // key (`docs/history/NEW-WALLET.md` §2.4).
    section_label(ui, x, cy, EN.first_addresses_the);
    cy += 24.0;
    if app.receive_held(wl) {
        cy += ui.wrap(x, cy, w, 13.0, W::S, WARN, crate::glance::RECEIVE_WARNING) + 8.0;
        return cy - y;
    }
    for (label, change, idx) in [
        (EN.first_addresses_receive_0, false, 0),
        (EN.first_addresses_receive, false, 1),
        (EN.first_addresses_change, true, 0),
    ] {
        ui.text(x, cy + 2.0, 12.0, W::R, MUTED, label);
        let a = app.receive_line(wl, change, idx).0;
        let ax = if ui.compact {
            cy += 20.0;
            0.0
        } else {
            96.0
        };
        cy += ui.wrap(x + ax, cy, w - ax, 13.0, W::M, TEXT, &a).max(20.0) + 10.0;
    }
    cy - y
}

/// Where a file a flow made stands: For the stick, with its place in
/// the list for Remove; or written, by the receipt. `None` when neither.
pub(super) fn made_place(app: &View, file: &str) -> Option<(String, Option<usize>)> {
    if let Some(k) = app.outbox.iter().position(|i| i.name == file) {
        return Some((app.medium.for_box().to_string(), Some(k)));
    }
    app.receipt
        .as_ref()
        .filter(|r| r.wrote(file))
        .map(|r| (fill(EN.made_place_written, &[&r.label.to_string()]), None))
}

/// A made file's row state and its button: For the stick with Remove;
/// written, by the receipt, with `make` to make it again; or not made,
/// with `make`. `remove` takes it off For the stick.
pub(super) fn made_state<'a>(
    app: &View,
    file: &str,
    remove: Option<Action>,
    make: (&'a str, Action),
) -> (Option<String>, (&'a str, Style, Action)) {
    match made_place(app, file) {
        Some((t, Some(k))) => (
            Some(t),
            (
                EN.made_state_remove,
                Style::Ghost,
                remove.unwrap_or(Action::OutboxRemove(k)),
            ),
        ),
        Some((t, None)) => (
            Some(t),
            (EN.made_state_make_again, Style::Secondary, make.1),
        ),
        None => (None, (make.0, Style::Secondary, make.1)),
    }
}

/// One row of a file a flow makes: its name and what it is, and at the
/// right where it is, when it is made, and its button (Remove while it
/// waits For the stick, else the one that makes it), with `extra`
/// buttons before them (Show as QR, PNG). On a small panel the name and what it is are two
/// lines, and the buttons go on a row under the name. Returns the row's
/// height.
#[allow(clippy::too_many_arguments)]
pub(super) fn file_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    name: &str,
    detail: &str,
    done: Option<&str>,
    button: (&str, Style, Action),
    extra: &[(&str, Action)],
) -> f32 {
    if !extra.is_empty() {
        return file_row_extra(ui, x, y, w, name, detail, done, button, extra);
    }
    let bw = ui.measure(13.0, W::S, button.0) + 32.0;
    let done_w = done.map_or(0.0, |t| ui.measure(13.0, W::S, t) + 10.0);
    let right = bw + done_w;
    let rh = if ui.compact { 52.0 } else { 36.0 };
    let by = y + (rh - 34.0) / 2.0;
    ui.button(x + w - bw, by, Some(bw), 34.0, button.0, button.1, button.2);
    if ui.compact {
        // Where it is takes the second line, in place of what it is, so
        // the name keeps the room beside the button.
        let room = w - bw - 10.0;
        let name = ui.fit(14.0, W::S, name, room);
        ui.text(x, y + 6.0, 14.0, W::S, TEXT, &name);
        let (line, tone) = match done {
            Some(t) => (t, OK),
            None => (detail, MUTED),
        };
        let line = ui.fit(12.0, W::R, line, room);
        ui.text(x, y + 28.0, 12.0, W::R, tone, &line);
    } else {
        if let Some(t) = done {
            ui.text_right(x + w - bw - 10.0, y, rh, 13.0, W::S, OK, t);
        }
        let name_w = 200.0_f32.min(w - right - 10.0);
        let n = ui.fit(14.0, W::S, name, name_w);
        ui.text_mid(x, y, 36.0, 14.0, W::S, TEXT, &n);
        let room = w - right - 220.0 - 10.0;
        if room > 60.0 {
            let d = ui.fit(12.0, W::R, detail, room);
            ui.text_mid(x + 220.0, y, 36.0, 12.0, W::R, MUTED, &d);
        }
    }
    ui.rule(x, y + rh + 4.0, w, INNER);
    rh + 8.0
}

#[allow(clippy::too_many_arguments)]
fn file_row_extra(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    name: &str,
    detail: &str,
    done: Option<&str>,
    button: (&str, Style, Action),
    extra: &[(&str, Action)],
) -> f32 {
    let done_w = done.map_or(0.0, |t| ui.measure(13.0, W::S, t));
    if ui.compact {
        let room = w - if done_w > 0.0 { done_w + 10.0 } else { 0.0 };
        let n = ui.fit(14.0, W::S, name, room);
        ui.text(x, y + 6.0, 14.0, W::S, TEXT, &n);
        if let Some(t) = done {
            ui.text_right(x + w, y, 28.0, 13.0, W::S, OK, t);
        }
        let d = ui.fit(12.0, W::R, detail, w);
        ui.text(x, y + 28.0, 12.0, W::R, MUTED, &d);
        let mut items: Vec<(&str, Style, Action)> = extra
            .iter()
            .map(|&(l, a)| (l, Style::Secondary, a))
            .collect();
        items.push(button);
        // One row across the panel when the labels fit it, the space
        // left shared out; else wrapped.
        let gap = 6.0;
        let widths: Vec<f32> = items
            .iter()
            .map(|(l, ..)| ui.measure(13.0, W::S, l) + 20.0)
            .collect();
        let used = widths.iter().sum::<f32>() + gap * (items.len() as f32 - 1.0);
        let rh = if used <= w {
            let extra = (w - used) / items.len() as f32;
            let mut bx = x;
            for (&(label, style, action), bw) in items.iter().zip(&widths) {
                bx += ui.button(bx, y + 50.0, Some(bw + extra), 34.0, label, style, action) + gap;
            }
            50.0 + 34.0
        } else {
            50.0 + wrap_buttons(ui, x, y + 50.0, w, 34.0, &items) - 8.0
        };
        ui.rule(x, y + rh + 4.0, w, INNER);
        return rh + 8.0;
    }
    let rh = 36.0;
    let by = y + (rh - 34.0) / 2.0;
    let mut right = x + w;
    let bw = ui.measure(13.0, W::S, button.0) + 32.0;
    right -= bw;
    ui.button(right, by, Some(bw), 34.0, button.0, button.1, button.2);
    right -= 6.0;
    if let Some(t) = done {
        ui.text_right(right - 4.0, y, rh, 13.0, W::S, OK, t);
        right -= done_w + 16.0;
    }
    for &(label, action) in extra.iter().rev() {
        let bw = ui.measure(13.0, W::S, label) + 32.0;
        right -= bw;
        ui.button(right, by, Some(bw), 34.0, label, Style::Secondary, action);
        right -= 6.0;
    }
    let name_w = 200.0_f32.min(right - x - 10.0);
    let n = ui.fit(14.0, W::S, name, name_w);
    ui.text_mid(x, y, rh, 14.0, W::S, TEXT, &n);
    let room = right - (x + 220.0) - 10.0;
    if room > 60.0 {
        let d = ui.fit(12.0, W::R, detail, room);
        ui.text_mid(x + 220.0, y, rh, 12.0, W::R, MUTED, &d);
    }
    ui.rule(x, y + rh + 4.0, w, INNER);
    rh + 8.0
}

/// A wallet's descriptor as a summary row (`docs/DESIGN.md` §4.5): label
/// above; the shape, the keys' fingerprints and the checksum below; a
/// chevron; the whole row opens `action`. Returns its height.
/// A masked passphrase field with its label at the left (above it on a
/// small panel): dots for what is typed, the **No passphrase** pill
/// while it is empty, the caret while it is focused. In Demo `chip` is the press of the
/// `faraday` chip the empty field offers, and `faraday` is drawn in its
/// code box. Returns the height used.
#[allow(clippy::too_many_arguments)]
pub(crate) fn pass_field(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    text: &crate::secret_text::SecretText,
    (on, chip): (bool, Option<(Action, bool)>),
    action: Action,
) -> f32 {
    let (px, py, pw) = if ui.compact {
        ui.text(x, y, 13.0, W::R, MUTED, label);
        (x, y + 22.0, w)
    } else {
        ui.text_mid(x, y, 40.0, 13.0, W::R, MUTED, label);
        (x + 150.0, y, 380.0f32.min(w - 150.0))
    };
    ui.fill(px, py, pw, 40.0, 8.0, BG);
    ui.stroke(px, py, pw, 40.0, 8.0, if on { ACCENT } else { BORDER });
    dots_field(ui, px, py, pw, text, (on, chip), action);
    py - y + 48.0
}

/// What a BIP-39 passphrase field 40 tall and `w` wide at (x, y) shows:
/// a dot for each character typed, with the selection and the caret
/// while `on`, or the **No passphrase** pill while it is empty
/// (`docs/history/NEW-WALLET.md` §13.4). In Demo (`chip`) the empty field
/// offers the `faraday` chip, and `faraday` is drawn in its code box.
pub(crate) fn dots_field(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    text: &str,
    (on, chip): (bool, Option<(Action, bool)>),
    action: Action,
) {
    // The chip, drawn after, is pressed before the field.
    ui.hit(x, y, w, tokens::TEXT_FIELD, action);
    if text.is_empty() {
        ui.no_passphrase(x, y, 40.0, on);
        if let Some((a, true)) = chip {
            crate::samples::put::pass_chip(ui, (x, y, w, tokens::TEXT_FIELD), a, true);
        }
    } else if chip.is_some() && text == crate::demo::PASSPHRASE {
        code_field(ui, x, y, w, tokens::TEXT_FIELD, text);
    } else {
        let dots = "•".repeat(text.chars().count());
        ui.typed(
            x + 12.0,
            y,
            40.0,
            14.0,
            W::M,
            TEXT,
            &dots,
            w - 24.0,
            on,
            action,
        );
    }
}

/// A passphrase field in Demo that holds `faraday`: it in its code box at
/// the field's text inset, unmasked (`docs/MODES.md` §6.2). No eye.
pub(crate) fn code_field(ui: &mut Ui, x: f32, y: f32, w: f32, h: f32, text: &str) {
    ui.fill(x, y, w, h, tokens::FIELD_RADIUS, BG);
    ui.stroke(x, y, w, h, tokens::FIELD_RADIUS, BORDER);
    let size = tokens::PASS_CODE_TEXT;
    let top = y + (h - ui.code_height(size)) / 2.0;
    ui.code_box(x + tokens::PASS_CODE_INSET, top, size, text);
}

/// The **Locked in** tag: a lock and the words, in `OK`. Returns its
/// width.
pub(crate) fn locked_tag(ui: &mut Ui, x: f32, y: f32) -> f32 {
    let label = EN.locked_tag_locked_in;
    let width = ui.measure(12.0, W::S, label) + 40.0;
    ui.fill(x, y, width, 26.0, 13.0, OK.with_alpha(30));
    ui.icon(x + 6.0, y + 3.0, 20.0, Icon::Lock, 10.0, OK);
    ui.text_mid(x + 28.0, y, 26.0, 12.0, W::S, OK, label);
    width
}

pub(crate) fn descriptor_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    wl: &crate::wallet::Wallet,
    action: Action,
) -> f32 {
    let fps: Vec<String> = wl
        .policy()
        .keys()
        .iter()
        .map(|k| crate::wallet_backup::fp_of(k.fingerprint()))
        .collect();
    let mut value = format!("{} · {}", Session::shape(wl), fps.join(", "));
    if wl.policy().record().is_none() {
        value.push_str(&format!(" · #{}", wl.checksum()));
    }
    ui.text(x, y, 12.0, W::R, MUTED, EN.descriptor_row_descriptor);
    let vh = ui
        .wrap(x, y + 20.0, w - 28.0, 13.0, W::M, TEXT, &value)
        .max(18.0);
    let h = 20.0 + vh + 8.0;
    ui.icon(
        x + w - 20.0,
        y + (h - 20.0) / 2.0,
        20.0,
        Icon::ChevronRight,
        9.0,
        DIM,
    );
    ui.hit(x, y - 4.0, w, h, action);
    ui.rule(x, y + h, w, INNER);
    h + 12.0
}

pub(crate) fn kind_line(kind: FileKind, bytes: usize) -> String {
    format!("{} · {}", kind_name(kind), size_text(bytes))
}

/// A file's size as [`kind_line`] gives it.
pub(crate) fn size_text(bytes: usize) -> String {
    if bytes >= 1_000_000 {
        fill(EN.kind_line_mb, &[&format!("{:.1}", bytes as f32 / 1e6)])
    } else if bytes >= 1000 {
        fill(EN.kind_line_kb, &[&format!("{:.1}", bytes as f32 / 1e3)])
    } else {
        format!("{bytes} B")
    }
}

/// What a file of this kind is called.
pub(crate) fn kind_name(kind: FileKind) -> &'static str {
    match kind {
        FileKind::Psbt => "PSBT",
        FileKind::Wallet => EN.kind_name_wallet_descriptor,
        FileKind::SecretWallet => EN.kind_name_secret_wallet,
        FileKind::Transaction => EN.kind_name_finished,
        FileKind::Sheet => EN.kind_name_sheet_for,
        FileKind::Message => EN.kind_name_signed_message,
        FileKind::Key => EN.kind_name_account_xpub,
        FileKind::Words => EN.kind_name_seed_words,
        FileKind::Share => EN.kind_name_share_of,
        FileKind::Carry => EN.kind_name_threshold,
        FileKind::Vault => EN.kind_name_vault_locked,
        FileKind::Entries => EN.kind_name_entries_for,
        FileKind::Backup => EN.kind_name_oskb,
        FileKind::Kdbx => EN.kind_name_keepass_database,
        FileKind::Pdf => EN.kind_name_pdf_to_print,
        FileKind::EfiImage => EN.kind_name_efi_image,
        FileKind::SeedPart => EN.kind_name_part_of_a,
        FileKind::Text => EN.kind_name_text,
        FileKind::Gpg => EN.kind_name_gpg,
        FileKind::Apk => EN.apk_kind_apk,
        FileKind::Certificate => EN.apk_kind_certificate,
        FileKind::Other => EN.kind_name_file,
    }
}

/// What pulling the stick does next: the keys an Import and load copied
/// in load, or the vault it brought asks for its passphrase.
pub(super) fn pull_line(app: &View) -> String {
    // Only words copied in for Import and load are keys waiting to load.
    let n = app
        .inbox
        .iter()
        .filter(|i| i.kind == FileKind::Words && app.visit.load_after.contains(&i.name))
        .count();
    let vault = app
        .vault_files()
        .into_iter()
        .find(|f| f.open.is_none() && !f.in_outbox)
        .map(|f| f.name);
    let noun = app.medium.noun();
    match (n, vault) {
        (0, Some(v)) if !app.holds_secret() => {
            fill(EN.pull_line_remove_the_to, &[noun, &v.to_string()])
        }
        (0, _) => fill(EN.pull_line_remove_the, &[noun]),
        (1, _) => fill(EN.pull_line_1_key_copied, &[noun]),
        (n, _) => fill(EN.pull_line_keys_copied, &[&n.to_string(), noun]),
    }
}

/// How a grid of word cells is drawn: as many as `boxes`, a word taken
/// as bullets when `masked`, the caret at the word being typed when
/// `caret`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cells {
    pub boxes: usize,
    pub masked: bool,
    pub caret: bool,
    /// Draw the never-a-real-seed line one gap under the grid (online).
    pub never: bool,
}

/// The never-a-real-seed line one gap under a grid whose last row ends
/// at `bottom`. Returns the gap and the line's height.
pub(crate) fn grid_never(ui: &mut Ui, x: f32, bottom: f32, w: f32) -> f32 {
    tokens::GAP
        + ui.wrap(
            x,
            bottom + tokens::GAP,
            w,
            13.0,
            W::R,
            WARN,
            EN.never_real_seed,
        )
}

/// A Latin list's words as typed, in a grid of numbered cells with the
/// words the word being typed can still become under it: Add a key's
/// word entry and Check my copy's. `masked` draws a word once taken as
/// bullets, and the one being typed in full. Returns where the next row
/// starts.
pub(crate) fn typed_cells(
    ui: &mut Ui,
    (x, y, width): (f32, f32, f32),
    lang: faraday_bip::bip39::Language,
    typed: &str,
    cells: Cells,
) -> f32 {
    let Cells {
        boxes,
        masked,
        caret,
        never,
    } = cells;
    let mut y = y;
    let words: Vec<&str> = typed.split_whitespace().collect();
    let typing_word = !typed.is_empty() && !typed.ends_with(' ');
    // A grid of numbered word cells, as many as the count chosen.
    let cols = 4;
    let cellw = (width - 3.0 * 10.0) / cols as f32;
    let shown = boxes;
    for k in 0..shown {
        let cx = x + (k % cols) as f32 * (cellw + 10.0);
        let cy = y + (k / cols) as f32 * 46.0;
        let current = k == words.len() - usize::from(typing_word && !words.is_empty()) && k < shown;
        let edge = if current {
            ACCENT.with_alpha(110)
        } else {
            INNER
        };
        ui.fill(cx, cy, cellw, 38.0, 8.0, BG);
        ui.stroke(cx, cy, cellw, 38.0, 8.0, edge);
        ui.text_mid(cx + 10.0, cy, 38.0, 11.0, W::R, DIM, &(k + 1).to_string());
        if let Some(wd) = words.get(k) {
            let last_partial = typing_word && k + 1 == words.len();
            if masked && !last_partial {
                ui.text_mid(cx + 36.0, cy, 38.0, 14.0, W::M, TEXT, "••••••");
                continue;
            }
            let found = crate::forms::typed_index(lang, wd);
            let c = if found.is_some() || last_partial {
                TEXT
            } else {
                ERR
            };
            // A finished word in the list's own spelling.
            let shown = match found {
                Some(i) if !last_partial => lang.word(i),
                _ => wd,
            };
            let tw = ui.text_mid(cx + 36.0, cy, 38.0, 14.0, W::M, c, shown);
            if last_partial && caret {
                ui.caret(cx + 37.0 + tw, cy + 10.0, 18.0);
            }
        } else if current && caret {
            ui.caret(cx + 37.0, cy + 10.0, 18.0);
        }
    }
    let rows_h = shown.div_ceil(cols) as f32 * 46.0;
    y += rows_h + 12.0;
    if never {
        y += grid_never(ui, x, y - 12.0 - 8.0, width) - tokens::GAP;
    }

    // Candidates for the word being typed.
    if typing_word && let Some(prefix) = words.last() {
        let cands: Vec<&str> = crate::forms::typed_candidates(lang, prefix)
            .into_iter()
            .take(8)
            .map(|i| lang.word(i))
            .collect();
        let line = if cands.is_empty() {
            EN.entry_no_word_starts.to_string()
        } else {
            format!("{}   {}", cands.join("  "), EN.entry_tab_completes)
        };
        ui.text(
            x,
            y,
            13.0,
            W::M,
            if cands.is_empty() { ERR } else { MUTED },
            &line,
        );
    }
    y + 30.0
}

/// The words taken on a list's own keyboard, in a grid, the next one's
/// keys in its cell; `masked` draws a word once taken as bullets. Returns
/// its height.
pub(crate) fn keyed_cells(
    ui: &mut Ui,
    (x, y, width): (f32, f32, f32),
    lang: faraday_bip::bip39::Language,
    taken: &[u16],
    prefix: &str,
    cells: Cells,
) -> f32 {
    let Cells {
        boxes: shown,
        masked,
        caret,
        never,
    } = cells;
    let cols = 6;
    let cellw = (width - (cols - 1) as f32 * 8.0) / cols as f32;
    for k in 0..shown {
        let cx = x + (k % cols) as f32 * (cellw + 8.0);
        let cy = y + (k / cols) as f32 * 42.0;
        let current = k == taken.len();
        ui.fill(cx, cy, cellw, 36.0, 8.0, BG);
        ui.stroke(
            cx,
            cy,
            cellw,
            36.0,
            8.0,
            if current {
                ACCENT.with_alpha(110)
            } else {
                INNER
            },
        );
        ui.text_mid(cx + 8.0, cy, 36.0, 11.0, W::R, DIM, &(k + 1).to_string());
        let word = match taken.get(k) {
            Some(_) if masked => Some(("••••••".to_string(), TEXT)),
            Some(&i) => Some((lang.word_display(i).to_string(), TEXT)),
            None if current && !prefix.is_empty() => Some((prefix.to_string(), ACCENT)),
            None => None,
        };
        let tw = match word {
            Some((t, c)) => {
                let t = ui.fit(14.0, W::M, &t, cellw - 36.0);
                ui.text_mid(cx + 30.0, cy, 36.0, 14.0, W::M, c, &t)
            }
            None => 0.0,
        };
        if current && caret {
            ui.caret(cx + 31.0 + tw, cy + 9.0, 18.0);
        }
    }
    let rows_h = shown.div_ceil(cols) as f32 * 42.0;
    if never {
        rows_h + 8.0 + grid_never(ui, x, y + rows_h - 6.0, width)
    } else {
        rows_h + 8.0
    }
}

/// What the keys typed on a list's own keyboard can be: a pill each,
/// pressed to take it (`hint`: with a line saying so). Returns its height.
pub(crate) fn keyed_pills(
    ui: &mut Ui,
    x: f32,
    y: f32,
    width: f32,
    w: &crate::word_entry::WordEntry,
    lang: faraday_bip::bip39::Language,
    hint: bool,
) -> f32 {
    let mut y = y;
    let top = y;
    let prefix: String = w.prefix().iter().collect();
    let cands: Vec<u16> = w.candidates().collect();
    if cands.is_empty() {
        let line = if prefix.is_empty() {
            EN.keyed_words_type_the
        } else if w.awaiting_tone() {
            EN.keyed_words_now_its
        } else {
            EN.keyed_words_no_word
        };
        ui.text_mid(x, y, 30.0, 13.0, W::R, MUTED, line);
        y += 34.0;
    } else {
        if hint {
            ui.text(x, y, 12.0, W::R, MUTED, EN.keyed_words_press_a);
            y += 22.0;
        }
        let mut bx = x;
        for (n, &i) in cands.iter().enumerate() {
            let word = lang.word_display(i);
            let bw = ui.measure(14.0, W::M, word) + 24.0;
            if bx > x && bx + bw > x + width {
                bx = x;
                y += 32.0;
            }
            bx += ui.word_pill(bx, y, word, Action::EntryCandidate(n as u8)) + 6.0;
        }
        y += 36.0;
    }
    y - top
}

/// The word keyboard, for the list being typed, every key
/// that leads to no word off. Returns where the next row starts.
pub(crate) fn word_keys(
    shift: bool,
    w: &crate::word_entry::WordEntry,
    ui: &mut Ui,
    x: f32,
    y: f32,
    width: f32,
    kh: f32,
) -> f32 {
    use faraday_draw::widgets::keyboard::{self, KeyInput, Modifiers};
    let kind = w.keyboard();
    let rows = keyboard::rows(kind) as f32;
    let area = faraday_draw::geom::Rect::new(x as i32, y as i32, width as i32, (rows * kh) as i32);
    let ctx = faraday_draw::layout::LayoutCtx::new(
        faraday_draw::geom::Scale::new(160),
        faraday_draw::geom::SizeClass::Wide,
    );
    let mods = Modifiers {
        shift,
        symbols: false,
    };
    for cap in keyboard::keys(kind, area, &ctx, w.enabled_keys(), None, mods) {
        let r = cap.rect;
        let (kx, ky, kw, kh) = (r.x as f32, r.y as f32, r.w as f32 - 4.0, r.h as f32 - 4.0);
        match cap.input {
            KeyInput::Char(c) => {
                let style = if cap.enabled {
                    Style::Secondary
                } else {
                    Style::Disabled
                };
                ui.button(
                    kx,
                    ky,
                    Some(kw),
                    kh,
                    &c.to_string(),
                    style,
                    Action::EntryKey(c),
                );
            }
            KeyInput::Backspace => {
                ui.button(
                    kx,
                    ky,
                    Some(kw),
                    kh,
                    "",
                    Style::Secondary,
                    Action::EntryKeyBack,
                );
                ui.icon(
                    kx + kw / 2.0 - 9.0,
                    ky + kh / 2.0 - 9.0,
                    18.0,
                    Icon::Delete,
                    12.0,
                    TEXT,
                );
            }
            KeyInput::Shift => {
                let style = if shift {
                    Style::Primary
                } else {
                    Style::Secondary
                };
                ui.button(
                    kx,
                    ky,
                    Some(kw),
                    kh,
                    EN.word_keys_shift,
                    style,
                    Action::EntryShift,
                );
            }
            _ => {}
        }
    }
    y + rows * kh + 12.0
}

/// A section of a small panel's page: a card with its title, measured
/// first so its surface goes down under what `draw` puts in it at the
/// inner (x, y, w). Returns its height with the gap under it.
pub(super) fn section(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    title: &str,
    draw: &dyn Fn(&mut Ui, f32, f32, f32) -> f32,
) -> f32 {
    let (ix, iw) = (x + 14.0, w - 28.0);
    let inner = crate::compact::screen::measured(ui, |ui| draw(ui, ix, y + 44.0, iw));
    let h = 44.0 + inner + 12.0;
    ui.card(x, y, w, h, LINE);
    ui.text(ix, y + 16.0, 15.0, W::S, TEXT, title);
    draw(ui, ix, y + 44.0, iw);
    h + 14.0
}

/// Time zone: a label, the offset stepped ±15 minutes, stopping at
/// −12:00 and +14:00 (`docs/TOTP.md` §2.1). Settings and Set the time
/// press the same action. On a small panel the label goes on its own
/// line, as the shares card's does. Returns the row's height.
pub(super) fn tz_row(ui: &mut Ui, x: f32, y: f32, _w: f32, tz_min: i16) -> f32 {
    let (ly, bx, h) = if ui.compact {
        ui.text(x, y, 13.0, W::R, MUTED, EN.tz_row_time_zone);
        (y + 24.0, x, 64.0)
    } else {
        ui.text_mid(x, y, 40.0, 13.0, W::R, MUTED, EN.tz_row_time_zone);
        (y, x + 180.0, 40.0)
    };
    ui.button(
        bx,
        ly,
        Some(40.0),
        40.0,
        "-",
        Style::Secondary,
        Action::TzStep(-15),
    );
    let v = format!("UTC{}", crate::vault::tz_offset_text(tz_min));
    let vw = ui.measure(14.0, W::S, &v);
    ui.text_mid(
        bx + 48.0 + (84.0 - vw) / 2.0,
        ly,
        40.0,
        14.0,
        W::S,
        TEXT,
        &v,
    );
    ui.button(
        bx + 132.0,
        ly,
        Some(40.0),
        40.0,
        "+",
        Style::Secondary,
        Action::TzStep(15),
    );
    h
}
