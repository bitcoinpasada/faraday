//! Transfer, in the online app: Send (Downloads, newest first, each file
//! pressed shown as codes) and Receive (the camera, and what it saved).

use crate::Action;
use crate::drawing::View;
use crate::screens::{section_label, title};
use crate::strings::EN;
use crate::strings::fill;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
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
        (x0 + 48.0, (cw - 96.0).min(1100.0))
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
    y += 24.0;
    if !t.dir.is_empty() {
        let dir = ui.fit(12.0, W::M, &t.dir, w);
        ui.text(x, y, 12.0, W::M, Role::OnSurfaceVariant, &dir);
        y += 22.0;
    }
    if let Some(path) = t.reading.as_deref() {
        let name = path.rsplit('/').next().unwrap_or(path);
        let line = ui.fit(13.0, W::R, &fill(EN.transfer_reading, &[name]), w);
        ui.text(x, y, 13.0, W::R, Role::OnSurfaceVariant, &line);
        y += 24.0;
    } else if let Some(why) = t.refused.as_deref() {
        y += ui.wrap(x, y, w, 13.0, W::S, Role::Error, why) + 10.0;
    }
    if t.files.is_empty() {
        ui.text(
            x,
            y + 4.0,
            13.0,
            W::R,
            Role::OnSurfaceVariant,
            EN.transfer_no_files,
        );
        return y + 28.0 - y0;
    }
    ui.rule(x, y, w, Role::OutlineVariant);
    for (i, (name, size)) in t.files.iter().enumerate() {
        let action = Action::TransferSend(i);
        if ui.is_pressed(action) {
            ui.wash(
                x - 8.0,
                y,
                w + 16.0,
                ROW,
                8.0,
                Role::OnSurface.at(faraday_draw::tokens::STATE_PRESS),
            );
        }
        let over = *size > crate::transfer::MAX_SEND;
        let tail = if over {
            EN.transfer_over_256_kib.to_string()
        } else {
            size_text(*size)
        };
        let tw = ui.measure(12.0, W::R, &tail);
        ui.text_right(
            x + w,
            y,
            ROW,
            12.0,
            W::R,
            if over {
                Role::Warning
            } else {
                Role::OnSurfaceVariant
            },
            &tail,
        );
        let label = ui.fit(13.0, W::S, name, w - tw - 16.0);
        ui.text_mid(
            x,
            y,
            ROW,
            13.0,
            W::S,
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
    y += 28.0;
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
        y += ui.wrap(x, y, w, 12.0, W::R, tone, line) + 8.0;
    }
    y - y0
}
