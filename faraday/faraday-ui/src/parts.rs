//! Pieces of a page that the desktop app's screens and sheets and
//! Faraday's other apps share: a title, a section's label, buttons in
//! rows, a sheet's box, a QR code, and the height of what is not drawn.

use crate::pal::*;
use crate::{Style, Ui, W};
use faraday_draw::tokens::{BUTTON_H, BUTTON_PAD, GAP, HEADLINE_M, LABEL_L, LABEL_M, TITLE_S};

/// A page's title, in Material's headline medium. Returns its height:
/// the role's line.
pub fn title<A: Copy + PartialEq>(ui: &mut Ui<A>, x: f32, y: f32, s: &str) -> f32 {
    ui.text_as(x, y, HEADLINE_M, Role::OnSurface, s);
    ui.line_as(HEADLINE_M)
}

/// A section's label, in Material's title small.
pub fn section_label<A: Copy + PartialEq>(ui: &mut Ui<A>, x: f32, y: f32, s: &str) {
    ui.text_as(x, y, TITLE_S, Role::OnSurfaceVariant, s);
}

/// Buttons [`BUTTON_H`] tall laid left to right, wrapping to a new row
/// at the width. Returns the height they took, a row being a button and
/// the gap under it.
pub fn button_rows<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    w: f32,
    items: &[(String, Style, A)],
) -> f32 {
    // As tall as larger text needs (`docs/TEXT-SIZE.md` §5).
    let bh = ui.button_height(BUTTON_H);
    let (mut bx, mut by) = (x, y);
    for (label, style, action) in items {
        // No wider than the row; a long label is cut.
        let bw = (ui.measure_as(LABEL_L, label) + 2.0 * BUTTON_PAD).min(w);
        if bx > x && bx + bw > x + w {
            bx = x;
            by += bh + GAP;
        }
        let label = ui.fit_as(LABEL_L, label, bw - BUTTON_PAD);
        bx += ui.button(bx, by, Some(bw), bh, &label, *style, *action) + GAP;
    }
    by - y + bh + GAP
}

/// Buttons `h` high, or as tall as larger text needs
/// ([`Ui::button_height`]), laid left to right, wrapping to a new row at
/// the width, as a narrow column needs. Returns the height they took and
/// the gap under them.
pub fn wrap_buttons<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    items: &[(&str, Style, A)],
) -> f32 {
    let h = ui.button_height(h);
    let (mut bx, mut by) = (x, y);
    for &(label, style, action) in items {
        let bw = (ui.measure_as(LABEL_L, label) + 2.0 * BUTTON_PAD).min(w);
        if bx > x && bx + bw > x + w {
            bx = x;
            by += h + GAP;
        }
        let label = ui.fit_as(LABEL_L, label, bw - BUTTON_PAD);
        bx += ui.button(bx, by, Some(bw), h, &label, style, action) + GAP;
    }
    by - y + h + GAP
}

/// The height [`wrap_buttons`] takes for these labels, `h` high in `w`,
/// measured and not drawn.
pub fn wrap_buttons_height<A: Copy + PartialEq>(
    ui: &Ui<A>,
    w: f32,
    h: f32,
    labels: &[&str],
) -> f32 {
    let h = ui.button_height(h);
    let (mut bx, mut by) = (0.0, 0.0);
    for label in labels {
        let bw = (ui.measure_as(LABEL_L, label) + 2.0 * BUTTON_PAD).min(w);
        if bx > 0.0 && bx + bw > w {
            bx = 0.0;
            by += h + GAP;
        }
        bx += bw + GAP;
    }
    by + h + GAP
}

/// The height `draw` takes, drawing nothing and keeping no hits: what a
/// card is measured with before its surface goes down.
pub fn measured<A: Copy + PartialEq>(ui: &mut Ui<A>, draw: impl FnOnce(&mut Ui<A>) -> f32) -> f32 {
    let mark = (ui.hits.len(), ui.reach.len());
    ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
    let h = draw(ui);
    ui.c.pop_clip();
    ui.hits.truncate(mark.0);
    ui.reach.truncate(mark.1);
    h
}

/// The height `s` takes wrapped to `width`, drawn nowhere.
pub fn measure_wrap<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    width: f32,
    size: f32,
    w: W,
    s: &str,
) -> f32 {
    ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
    let h = ui.wrap(0.0, 0.0, width, size, w, Role::OnSurface, s);
    ui.c.pop_clip();
    h
}

/// A sheet's box, `sw` by `sh`, centred in a frame `w` by `h` over the
/// page, with its shadow; a press beside it takes [`Ui::outside`].
/// Returns its top-left corner.
pub fn sheet_box<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    w: f32,
    h: f32,
    sw: f32,
    sh: f32,
) -> (f32, f32) {
    sheet_box_in(ui, (0.0, w, h), sw, sh)
}

/// [`sheet_box`] centred in a frame `w` by `h` whose left edge is at
/// `x0`: a sheet over a page that starts right of a sidebar.
pub fn sheet_box_in<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x0, w, h): (f32, f32, f32),
    sw: f32,
    sh: f32,
) -> (f32, f32) {
    let x = x0 + (w - sw) / 2.0;
    let y = (h - sh) / 2.0;
    if let Some(a) = ui.outside {
        ui.hit_around(x, y, sw, sh, a);
    }
    // Material's dialog: extra-large corners.
    use faraday_draw::tokens::SHAPE_XL;
    ui.shadow(
        x,
        y,
        sw,
        sh,
        SHAPE_XL,
        faraday_draw::tokens::ELEVATION_DIALOG,
    );
    ui.fill(x, y, sw, sh, SHAPE_XL, Role::SurfaceContainerHigh);
    // At High contrast, where it casts no shadow, its outline sets it
    // apart (`docs/THEMES.md` §9).
    if ui.theme.is_high_contrast() {
        ui.stroke(x, y, sw, sh, SHAPE_XL, Role::Outline);
    }
    (x, y)
}

/// A QR code on a white card, square modules, the quiet zone included.
pub fn qr_code<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    m: &faraday_codec::qr::QrMatrix,
    x: f32,
    y: f32,
    side: f32,
) {
    ui.fill(x, y, side, side, 8.0, faraday_draw::Color::WHITE);
    let n = m.size();
    let quiet = 4.0;
    // Whole pixels per module, so every module is the same size.
    let px_side = ui.px(side);
    let module_px = (px_side as f32 / (n as f32 + 2.0 * quiet)).floor().max(1.0) as i32;
    let used = module_px * n as i32;
    let r = ui.rect(x, y, side, side);
    let ox = r.x + (r.w - used) / 2;
    let oy = r.y + (r.h - used) / 2;
    for row in 0..n {
        let mut col = 0;
        while col < n {
            if m.module(col, row) {
                let start = col;
                while col < n && m.module(col, row) {
                    col += 1;
                }
                let rect = faraday_draw::Rect::new(
                    ox + start as i32 * module_px,
                    oy + row as i32 * module_px,
                    (col - start) as i32 * module_px,
                    module_px,
                );
                ui.c.fill_rect(rect, faraday_draw::Color::BLACK);
            } else {
                col += 1;
            }
        }
    }
}

pub use faraday_draw::tokens::{STEP_INDENT, STEP_SIDE, STEP_TOUCH};

/// A guide step's number in a circle, its top-left at (x, y): outlined
/// while it is to do; once `ticked`, filled in ok with the number in
/// on-ok (`docs/THEMES.md` §6 OK), a mark 3:1 or more on every surface.
/// With an `action`, a tap anywhere in the touch square around it takes
/// the action, its focus ring round the circle. Returns [`STEP_SIDE`].
pub fn step_circle<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    n: u32,
    ticked: bool,
    action: Option<A>,
) -> f32 {
    let d = STEP_SIDE;
    let r = d / 2.0;
    let label = n.to_string();
    let role = if label.len() > 1 { LABEL_M } else { TITLE_S };
    let pressed = action.is_some_and(|a| ui.is_pressed(a));
    let fg = if ticked {
        ui.dot(x + r, y + r, r, Role::Ok);
        Role::OnOk.paint()
    } else {
        if pressed {
            ui.dot(x + r, y + r, r, Role::SurfaceContainerHighest);
        }
        ui.stroke(x, y, d, d, r, Role::OnSurfaceVariant);
        ui.stroke(
            x + 1.0,
            y + 1.0,
            d - 2.0,
            d - 2.0,
            r - 1.0,
            Role::OnSurfaceVariant,
        );
        Role::OnSurface.paint()
    };
    let tw = ui.measure_as(role, &label);
    ui.text_mid_as((x + r - tw / 2.0, y, d), role, fg, &label);
    if let Some(a) = action {
        let pad = (STEP_TOUCH - d) / 2.0;
        let target = (x - pad, y - pad, STEP_TOUCH, STEP_TOUCH);
        ui.hit_ringed((x, y, d, d, r), target, a);
    }
    d
}
