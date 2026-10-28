//! Transfer, in the online app: Send (Downloads, newest first, each file
//! pressed shown as codes) and Receive (the camera, and what it saved).

use crate::Action;
use crate::drawing::View;
use crate::screens::{section_label, title};
use crate::strings::EN;
use crate::strings::fill;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{BODY_M, BODY_S, LABEL_L};
use faraday_draw::tokens::{BUTTON_H, GAP};
use faraday_ui::parts::wrap_buttons;

/// A row of the Downloads list.
const ROW: f32 = 44.0;

/// A size as the Downloads list gives it.
fn size_text(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        fill(EN.transfer_mb, &[&format!("{:.1}", bytes as f32 / 1e6)])
    } else if bytes >= 1000 {
        fill(EN.transfer_kb, &[&format!("{:.1}", bytes as f32 / 1e3)])
    } else {
        format!("{bytes} B")
    }
}

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let compact = ui.compact;
    let (x, w) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        use faraday_draw::tokens::{PAGE_INSET, TRANSFER_W};
        (x0 + PAGE_INSET, (cw - 2.0 * PAGE_INSET).min(TRANSFER_W))
    };
    let clip = ui.rect(x0, 0.0, cw, h);
    ui.c.push_clip(clip);
    let top = if compact { 14.0 } else { 36.0 } - app.list_offset;
    let mut y = top;
    if !compact {
        title(ui, x, y, EN.transfer_transfer);
        y += 52.0;
    }
    let side = !compact && w >= 600.0;
    let end = if side {
        let colw = (w - 32.0) / 2.0;
        let left = send(app, ui, x, y, colw);
        let right = receive(app, ui, x + colw + 32.0, y, colw);
        y + left.max(right)
    } else {
        let r = receive(app, ui, x, y, w);
        y += r + 20.0;
        y + send(app, ui, x, y, w)
    };
    ui.c.pop_clip();
    app.scroll.content_h().set(end - top + 24.0);
    ui.report_scroll(clip, app.scroll.content_h().get() - h);
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
            EN.transfer_over_256_kib.to_string()
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
        ui.hit(x - 8.0, y, w + 16.0, ROW, action);
        y += ROW;
        ui.rule(x, y, w, Role::OutlineVariant);
    }
    y + 8.0 - y0
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
        let tone = if *warn {
            Role::Warning
        } else {
            Role::OnSurface
        };
        y += ui.wrap_as((x, y, w), BODY_S, tone, line) + 8.0;
    }
    y - y0
}
