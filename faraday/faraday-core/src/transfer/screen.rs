//! Transfer, in the online app: Send (Downloads, newest first, each file
//! pressed shown as codes) and Receive (the camera, and what it saved).

use crate::Action;
use crate::drawing::View;
use crate::screens::{section_label, size_text};
use crate::strings::EN;
use crate::strings::fill;
use crate::transfer::{MAX_TEXT, TextCheck, TextPress};
use crate::ui::Region;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{BODY_M, BODY_S, LABEL_L};
use faraday_draw::tokens::{
    BUTTON_H, FIELD_EDGE_ACTIVE, FIELD_GAP, FIELD_PAD, FIELD_PAD_V, FIELD_RADIUS, GAP,
};
use faraday_ui::parts::wrap_buttons;

/// A row of the Downloads list.
const ROW: f32 = 44.0;

/// Between Transfer's sections.
const SECTION_GAP: f32 = 20.0;

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let compact = ui.compact;
    let (x, w) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        use faraday_draw::tokens::{PAGE_INSET, TRANSFER_W};
        (x0 + PAGE_INSET, (cw - 2.0 * PAGE_INSET).min(TRANSFER_W))
    };
    let clip = ui.rect(x0, 0.0, cw, h);
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    // Beside the expanded rail the top app bar names the page; held, the
    // holder's top app bar above the page names it.
    let top = if compact {
        14.0
    } else if app.held {
        36.0
    } else {
        crate::screens::bar_head(ui, None, EN.transfer_transfer)
    };
    let mut y = top;
    let side = !compact && w >= 600.0;
    let end = if side {
        let colw = (w - 32.0) / 2.0;
        let left = send(app, ui, x, y, colw);
        let rx = x + colw + 32.0;
        let t = send_text(app, ui, rx, y, colw);
        let right = t + SECTION_GAP + receive(app, ui, rx, y + t + SECTION_GAP, colw);
        y + left.max(right)
    } else {
        let r = receive(app, ui, x, y, w);
        y += r + SECTION_GAP;
        let t = send_text(app, ui, x, y, w);
        y += t + SECTION_GAP;
        y + send(app, ui, x, y, w)
    };
    app.scroll.content_h().set(end - top + 24.0);
    ui.region_close(open, app.scroll.content_h().get() - h);
}

/// Send: the folder, why the last file was not sent, and its files.
/// Returns its height.
fn send(app: &View, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let t = app.transfer();
    let mut y = y0;
    section_label(ui, x, y, EN.transfer_send);
    y += ui.line_as(faraday_draw::tokens::TITLE_S) + faraday_draw::tokens::GAP_SMALL;
    if !t.dir.is_empty() {
        let dir = ui.fit_as(BODY_S.mono(), &t.dir, w);
        ui.text_as(x, y, BODY_S.mono(), Role::OnSurfaceVariant, &dir);
        y += ui.advance_as(BODY_S.mono(), 22.0);
    }
    if let Some(path) = t.reading.as_deref() {
        let name = path.rsplit('/').next().unwrap_or(path);
        let line = ui.fit_as(BODY_M, &fill(EN.transfer_reading, &[name]), w);
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, &line);
        y += ui.line_as(BODY_S) + faraday_draw::tokens::GRID;
    } else if let Some(why) = t.refused.as_deref() {
        y += ui.wrap_as((x, y, w), LABEL_L, Role::Error, why) + 10.0;
    }
    if t.files.is_empty() {
        ui.text_as(
            x,
            y + 4.0,
            BODY_S,
            Role::OnSurfaceVariant,
            EN.transfer_no_files,
        );
        return y + 28.0 - y0;
    }
    ui.rule(x, y, w, Role::OutlineVariant);
    for (i, (name, size)) in t.files.iter().enumerate() {
        let action = Action::TransferSend(i);
        let over = *size > crate::transfer::MAX_SEND;
        let tail = if over {
            fill(EN.transfer_over, &[&size_text(crate::transfer::MAX_SEND)])
        } else {
            size_text(*size)
        };
        let tw = ui.measure_as(BODY_S, &tail);
        ui.text_right_as(
            (x + w, y, ROW),
            BODY_S,
            if over {
                Role::Warning
            } else {
                Role::OnSurfaceVariant
            },
            &tail,
        );
        let label = ui.fit_as(LABEL_L, name, w - tw - 16.0);
        ui.text_mid_as(
            (x, y, ROW),
            LABEL_L,
            if over {
                crate::ui::off()
            } else {
                Role::OnSurface.paint()
            },
            &label,
        );
        ui.press_box(x - 8.0, y, w + 16.0, ROW, action);
        y += ROW;
        ui.rule(x, y, w, Role::OutlineVariant);
    }
    y + 8.0 - y0
}

/// The fewest and the most lines the text field shows; past the most it
/// shows the end of the text.
const TEXT_LINES: (usize, usize) = (3, 8);

/// Send text: an outlined field that grows with its text, the count or
/// what was refused under it, and its buttons. Returns its height.
fn send_text(app: &View, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let card = app.transfer_text();
    let mut y = y0;
    section_label(ui, x, y, EN.transfer_send_text);
    y += ui.line_as(faraday_draw::tokens::TITLE_S) + faraday_draw::tokens::GRID;
    let here = Action::TypeHere(crate::edit::Here::TransferText);
    let room = w - 2.0 * FIELD_PAD;
    let laid = ui.wrapped_as(BODY_M, card.text(), room);
    let line = ui.line_as(BODY_M);
    let label = ui.line_as(BODY_S);
    let lines = ((laid.h / line).round() as usize).clamp(TEXT_LINES.0, TEXT_LINES.1);
    let field_h = FIELD_PAD_V + label + lines as f32 * line + FIELD_PAD_V;
    crate::menu::outline(
        ui,
        (x, y, w, field_h),
        FIELD_RADIUS,
        FIELD_EDGE_ACTIVE,
        Role::Primary.paint(),
    );
    ui.press_box(x, y, w, field_h, here);
    ui.text_as(
        x + FIELD_PAD,
        y + FIELD_PAD_V / 2.0,
        BODY_S,
        Role::Primary,
        EN.transfer_text_label,
    );
    let top = y + FIELD_PAD_V / 2.0 + label;
    ui.c.push_clip(ui.rect(x, top, w, field_h - (top - y) - FIELD_PAD_V / 2.0));
    if card.is_empty() {
        ui.wrap_as(
            (x + FIELD_PAD, top, room),
            BODY_M,
            Role::OnSurfaceVariant,
            EN.transfer_text_hint,
        );
        let ch = BODY_M.size * ui.text_factor();
        ui.caret(x + FIELD_PAD, top + (line - ch) / 2.0, ch);
    } else {
        // Past the most lines the field shows the end of the text.
        let hidden = (laid.h / line).round() as usize;
        let up = hidden.saturating_sub(TEXT_LINES.1) as f32 * line;
        ui.draw_typed(
            (x + FIELD_PAD, top - up),
            Role::OnSurface,
            &laid,
            (true, here),
        );
    }
    ui.c.pop_clip();
    y += field_h + FIELD_GAP;
    // Under the field: what was refused, else what the text is.
    let said = match card.check() {
        TextCheck::Fine if card.is_empty() => None,
        TextCheck::Fine => {
            let n = card.text().trim().chars().count().to_string();
            let size = size_text(card.text().trim().len() as u64);
            Some((
                fill(EN.transfer_text_count, &[&n, &size]),
                Role::OnSurfaceVariant,
            ))
        }
        TextCheck::TooLong => Some((
            fill(EN.transfer_text_too_long, &[&size_text(MAX_TEXT as u64)]),
            Role::Error,
        )),
        TextCheck::Secret(k) => Some((
            fill(EN.transfer_text_secret, &[&lower_first(k.label())]),
            Role::Error,
        )),
        TextCheck::Partial => Some((EN.transfer_text_partial.to_string(), Role::Error)),
        TextCheck::Inside(k) => Some((
            fill(EN.transfer_text_maybe_secret, &[&lower_first(k.label())]),
            Role::Error,
        )),
    };
    if let Some((line, tone)) = said {
        y += ui.wrap_as((x + FIELD_PAD, y, room), BODY_S, tone, &line) + FIELD_GAP;
    }
    y += GAP / 2.0;
    let mut buttons = vec![(
        EN.transfer_text_paste,
        Style::Secondary.when(app.clipboard),
        Action::TransferText(TextPress::Paste),
    )];
    if !card.is_empty() {
        buttons.push((
            EN.transfer_text_clear,
            Style::Ghost,
            Action::TransferText(TextPress::Clear),
        ));
    }
    buttons.push((
        EN.transfer_text_show,
        Style::Primary.when(card.sendable().is_some()),
        Action::TransferText(TextPress::Show),
    ));
    y += wrap_buttons(ui, x, y, w, BUTTON_H, &buttons);
    y - y0
}

/// `s` with its first letter in lower case, to follow other words.
fn lower_first(s: &str) -> String {
    let mut it = s.chars();
    it.next()
        .map(|c| c.to_lowercase().chain(it).collect())
        .unwrap_or_default()
}

/// Receive: the camera, the folder, and what was saved this session,
/// newest first. Returns its height.
fn receive(app: &View, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let t = app.transfer();
    let mut y = y0;
    section_label(ui, x, y, EN.transfer_receive);
    y += ui.line_as(faraday_draw::tokens::TITLE_S) + faraday_draw::tokens::GRID;
    // The second button goes under the first where the two do not fit
    // side by side.
    let mut buttons = vec![(
        EN.transfer_scan_camera,
        Style::Primary,
        Action::TransferReceive,
    )];
    if t.opens {
        buttons.push((
            EN.transfer_open_folder,
            Style::Secondary,
            Action::TransferOpenFolder,
        ));
    }
    y += wrap_buttons(ui, x, y, w, BUTTON_H, &buttons) + GAP;
    for (line, warn) in t.saved.iter().rev() {
        let tone = if *warn { Role::Warning } else { Role::Ok };
        y += ui.wrap_as((x, y, w), BODY_S, tone, line) + 8.0;
    }
    y - y0
}
