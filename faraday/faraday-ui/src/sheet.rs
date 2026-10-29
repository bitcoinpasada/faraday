//! A sheet on a small panel: as tall as what it holds, its buttons kept
//! at its foot, and what is above them scrolled when it is taller than
//! the panel.

use faraday_draw::widgets::Icon;

use crate::pal::*;
use crate::{Plain, Style, Ui};
use faraday_draw::tokens::{BODY_M, BODY_S, BUTTON_H, BUTTON_PAD, GAP, GAP_SMALL, LABEL_L, PAD};

/// What a sheet holds: drawn at (x, y) in a width, it returns the height
/// it took.
pub type Body<'b, A> = dyn for<'r, 'u> FnMut(&'r mut Ui<'u, A>, f32, f32, f32) -> f32 + 'b;

/// A sheet on a small panel: the panel's width less a margin, as tall as
/// what `body` draws (measured first, drawing nothing), centred. `body`
/// draws at (x, y) in a width and returns the height it took.
pub fn sheet<A: Copy + PartialEq>(ui: &mut Ui<A>, w: f32, h: f32, body: &mut Body<'_, A>) {
    scroll_sheet(ui, (GAP, w - 2.0 * GAP), h, GAP, PAD, body);
}

/// A sheet `sw` wide at `sx`, as tall as what `body` draws and at most
/// `h` less `margin` above and below, centred, with `pad` inside: what is
/// above its buttons scrolls when it is taller. [`sheet`] on a small
/// panel; a long sheet on the desktop too.
pub fn scroll_sheet<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    place: (f32, f32),
    h: f32,
    margin: f32,
    pad: f32,
    body: &mut Body<'_, A>,
) {
    framed(
        ui,
        place,
        (h, margin, pad),
        (false, Role::SurfaceContainerLow),
        body,
    );
}

/// [`scroll_sheet`], its buttons at its end as wide as their labels
/// when `end`, else sharing its width ([`buttons`]); filled in the
/// container role `fill` (a sheet's is surface-container-low, a
/// dialog's surface-container-high: `docs/THEMES.md` §6).
fn framed<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (sx, sw): (f32, f32),
    (h, margin, pad): (f32, f32, f32),
    (end, fill): (bool, Role),
    body: &mut Body<'_, A>,
) {
    use faraday_draw::tokens::SHAPE_XL;
    let (ix, iw) = (sx + pad, sw - 2.0 * pad);
    let foot_of = |ui: &mut Ui<A>, y: f32, items: &[(&str, Style, A)]| {
        if end {
            end_buttons(ui, ix, y, iw, items)
        } else {
            buttons(ui, ix, y, iw, items)
        }
    };
    // The sheet's buttons ([`buttons`]) are kept at its foot, and what is
    // above them scrolls when the sheet is taller than the panel.
    let mark = (ui.hits.len(), ui.reach.len());
    ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
    ui.sheet_pin = None;
    ui.sheet_pinning = true;
    let ch = body(ui, ix, 0.0, iw);
    let pins = ui.sheet_pin.take();
    let items: Vec<(&str, Style, A)> = pins
        .iter()
        .flatten()
        .map(|(l, s, a)| (l.as_str(), *s, *a))
        .collect();
    ui.sheet_pinning = false;
    let bh = foot_of(ui, 0.0, &items);
    ui.c.pop_clip();
    ui.hits.truncate(mark.0);
    ui.reach.truncate(mark.1);
    let foot = if items.is_empty() { 0.0 } else { bh + pad };
    let sh = (ch + 2.0 * pad + foot).min(h - 2.0 * margin);
    let sy = ((h - sh) / 2.0).max(margin);
    if let Some(a) = ui.outside {
        ui.press_plain((sx, sy, sw, sh), Plain::Beside, a);
    }
    // Material's dialog and sheet: extra-large corners.
    ui.shadow(
        sx,
        sy,
        sw,
        sh,
        SHAPE_XL,
        faraday_draw::tokens::ELEVATION_SHEET,
    );
    ui.fill(sx, sy, sw, sh, SHAPE_XL, fill);
    // At High contrast, where it casts no shadow, its outline sets it
    // apart (`docs/THEMES.md` §9); elsewhere its shadow and scrim do.
    if ui.theme.is_high_contrast() {
        ui.stroke(sx, sy, sw, sh, SHAPE_XL, Role::Outline);
    }
    // Taller than the panel: what is above the buttons scrolls.
    let view = sh - foot;
    let max = (ch + 2.0 * pad - view).max(0.0);
    let off = ui.offset.clamp(0.0, max);
    let clip = ui.rect(sx, sy, sw, view);
    ui.c.push_clip(clip);
    ui.sheet_pinning = true;
    body(ui, ix, sy + pad - off, iw);
    ui.sheet_pinning = false;
    ui.sheet_pin = None;
    ui.c.pop_clip();
    // Its edges fade into its own container (`docs/THEMES.md` §7).
    ui.report_scroll_on(clip, max, fill);
    if !items.is_empty() {
        if max > 0.0 {
            ui.fill(
                sx + 1.0,
                sy + view,
                sw - 2.0,
                1.0,
                0.0,
                Role::OutlineVariant,
            );
        }
        foot_of(ui, sy + view, &items);
    }
}

/// Material's standard side sheet (Material 3, Side sheets;
/// `docs/DESIGN.md` §4.16): docked against `right`, from `top` down
/// `h`, `w` wide, drawn `shown` of the way in from the edge (0 to 1).
/// Its container is the surface, with a divider along its leading
/// edge and no scrim, so what lies beside it stays as it is and takes
/// presses. Its header holds `head`'s icon and title and, at its end,
/// `close` as a text button; what `body` draws is under it, scrolling
/// where it is taller than the sheet, and the buttons `body` hands over
/// ([`buttons`]) are pinned at its foot.
pub fn side_sheet<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (right, top, w, h): (f32, f32, f32, f32),
    shown: f32,
    head: (crate::Glyph, Paint, &str),
    close: (&str, A),
    body: &mut Body<'_, A>,
) {
    use faraday_draw::tokens::{SIDE_SHEET_PAD as PAD, TOUCH};
    let sx = right - w * shown.clamp(0.0, 1.0);
    let (ix, iw) = (sx + PAD, w - 2.0 * PAD);
    ui.fill(sx, top, w, h, 0.0, Role::Surface);
    ui.fill(sx, top, 1.0, h, 0.0, Role::OutlineVariant);
    // The header: the title, and Close at its end in a touch target.
    let close_w = ui.measure_as(LABEL_L, close.0) + 2.0 * PAD;
    let th = sheet_head(
        ui,
        ix,
        top + PAD,
        iw - close_w - GAP,
        head.0,
        head.1,
        head.2,
    );
    ui.button(
        sx + w - PAD / 2.0 - close_w,
        top + PAD / 2.0 + (TOUCH - BUTTON_H) / 2.0,
        Some(close_w),
        BUTTON_H,
        close.0,
        Style::Ghost,
        close.1,
    );
    let body_top = top + PAD + th.max(TOUCH - PAD / 2.0);
    side_body(ui, (sx, body_top, w, top + h - body_top), body);
}

/// A side sheet's body ([`side_sheet`]) in the box (x, y, w, h): what
/// `body` draws, scrolled where it is taller than the box, and its
/// pinned buttons at the foot under a divider while it scrolls.
fn side_body<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (sx, sy, sw, sh): (f32, f32, f32, f32),
    body: &mut Body<'_, A>,
) {
    use faraday_draw::tokens::SIDE_SHEET_PAD as PAD;
    let (ix, iw) = (sx + PAD, sw - 2.0 * PAD);
    // Measured first, drawing nothing, for its height and its buttons.
    let mark = (ui.hits.len(), ui.reach.len());
    ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
    ui.sheet_pin = None;
    ui.sheet_pinning = true;
    let ch = body(ui, ix, 0.0, iw);
    let pins = ui.sheet_pin.take();
    let items: Vec<(&str, Style, A)> = pins
        .iter()
        .flatten()
        .map(|(l, s, a)| (l.as_str(), *s, *a))
        .collect();
    ui.sheet_pinning = false;
    let bh = buttons(ui, ix, 0.0, iw, &items);
    ui.c.pop_clip();
    ui.hits.truncate(mark.0);
    ui.reach.truncate(mark.1);
    let foot = if items.is_empty() {
        0.0
    } else {
        bh + 2.0 * PAD
    };
    let view = (sh - foot).max(0.0);
    let max = (ch + PAD - view).max(0.0);
    let off = ui.offset.clamp(0.0, max);
    let clip = ui.rect(sx + 1.0, sy, sw - 1.0, view);
    ui.c.push_clip(clip);
    ui.sheet_pinning = true;
    body(ui, ix, sy - off, iw);
    ui.sheet_pinning = false;
    ui.sheet_pin = None;
    ui.c.pop_clip();
    ui.report_scroll(clip, max);
    if !items.is_empty() {
        if max > 0.0 {
            ui.fill(
                sx + 1.0,
                sy + view,
                sw - 1.0,
                1.0,
                0.0,
                Role::OutlineVariant,
            );
        }
        buttons(ui, ix, sy + view + PAD, iw, &items);
    }
}

/// A dialog (Material's basic dialog, `docs/DESIGN.md` §4.11): its head,
/// what `body` draws under it, and `actions`, in one box centred in a
/// frame `w` by `h`; as tall as what it holds, and what is above its
/// actions scrolls when that is taller than the frame. On a small panel
/// (`Ui::compact`) it is the panel's width less a margin and its actions
/// share its width; elsewhere it is [`DIALOG_MIN`] to [`DIALOG_MAX`]
/// wide, `want` where that is between them, with [`DIALOG_PAD`] inside
/// and its actions at its end as wide as their labels.
pub fn dialog<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (w, h, want): (f32, f32, f32),
    head: (crate::Glyph, crate::pal::Paint, &str),
    body: &mut Body<'_, A>,
    actions: &[(&str, Style, A)],
) {
    dialog_in(ui, (0.0, w, h, want), head, body, actions);
}

/// [`dialog`] centred in a frame `w` by `h` whose left edge is at `x0`:
/// a dialog over a page that starts right of a sidebar.
pub fn dialog_in<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x0, w, h, want): (f32, f32, f32, f32),
    head: (crate::Glyph, crate::pal::Paint, &str),
    body: &mut Body<'_, A>,
    actions: &[(&str, Style, A)],
) {
    use faraday_draw::tokens::{DIALOG_MAX, DIALOG_MIN, DIALOG_PAD};
    let (place, frame, end) = if ui.compact {
        ((x0 + GAP, w - 2.0 * GAP), (h, GAP, PAD), false)
    } else {
        let dw = want.clamp(DIALOG_MIN, DIALOG_MAX).min(w - 2.0 * PAD);
        ((x0 + (w - dw) / 2.0, dw), (h, PAD, DIALOG_PAD), true)
    };
    framed(
        ui,
        place,
        frame,
        (end, Role::SurfaceContainerHigh),
        &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += sheet_head(ui, x, cy, iw, head.0, head.1, head.2);
            cy += body(ui, x, cy, iw);
            cy += buttons(ui, x, cy, iw, actions);
            cy - y
        },
    );
}

/// A dialog's actions at its end, each as wide as its label, the last
/// at the right edge of `w` from x: Material's dialog. One under another
/// at the width where they do not fit side by side. Returns their
/// height.
fn end_buttons<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    w: f32,
    items: &[(&str, Style, A)],
) -> f32 {
    use faraday_draw::tokens::DIALOG_ACTION_GAP as G;
    if items.is_empty() {
        return 0.0;
    }
    // As tall as larger text needs (`docs/TEXT-SIZE.md` §5).
    let h = ui.button_height(BUTTON_H);
    let widths: Vec<f32> = items
        .iter()
        .map(|(l, ..)| ui.measure_as(LABEL_L, l) + 2.0 * BUTTON_PAD)
        .collect();
    let total = widths.iter().sum::<f32>() + G * (items.len() - 1) as f32;
    if total > w {
        return buttons(ui, x, y, w, items);
    }
    let mut bx = x + w - total;
    for (&(label, style, action), bw) in items.iter().zip(widths) {
        ui.button(bx, y, Some(bw), h, label, style, action);
        bx += bw + G;
    }
    h
}

/// A sheet's title: its icon, and the words wrapped beside it in
/// Material's headline small, title large on the small panel
/// ([`faraday_draw::tokens::sheet_head`]). Returns its height.
pub fn sheet_head<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    w: f32,
    icon: impl Into<crate::Glyph>,
    tone: impl Into<Paint>,
    title: &str,
) -> f32 {
    use faraday_draw::tokens::{ICON, ICON_GLYPH_SMALL};
    let tone: Paint = tone.into();
    let role = faraday_draw::tokens::sheet_head(ui.compact);
    let line = ui.line_as(role);
    let iy = y + (ui.line(role.size, role.face.into()) - ICON) / 2.0;
    ui.icon(x - GAP_SMALL, iy, ICON, icon, ICON_GLYPH_SMALL, tone);
    let at = (x + ICON, y, w - ICON);
    let th = ui.wrap_as(at, role, Role::OnSurface, title);
    th.max(line) + GAP
}

/// A label over its value, each wrapped. Returns its height.
pub fn kv<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    w: f32,
    key: &str,
    value: &str,
    tone: impl Into<Paint>,
) -> f32 {
    let tone: Paint = tone.into();
    let vy = ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, key) + GAP_SMALL;
    let vh = ui.wrap_as((x, y + vy, w), BODY_M, tone, value);
    ui.rule(x, y + vy + vh + GAP_SMALL, w, Role::OutlineVariant);
    vy + vh + GAP_SMALL + GAP
}

/// A row of buttons: side by side when every label fits its share, one
/// under another, full width, when not. Returns their height.
pub fn buttons<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    x: f32,
    y: f32,
    w: f32,
    items: &[(&str, Style, A)],
) -> f32 {
    if items.is_empty() {
        return 0.0;
    }
    // Inside a sheet, they go to its foot ([`sheet`]).
    if ui.sheet_pinning {
        ui.sheet_pin = Some(
            items
                .iter()
                .map(|&(l, s, a)| (l.to_string(), s, a))
                .collect(),
        );
        return 0.0;
    }
    let n = items.len() as f32;
    let share = (w - (n - 1.0) * GAP) / n;
    // As tall as larger text needs (`docs/TEXT-SIZE.md` §5).
    let bh = ui.button_height(BUTTON_H);
    let fits = items
        .iter()
        .all(|(l, ..)| ui.measure_as(LABEL_L, l) + BUTTON_PAD <= share);
    if fits {
        for (i, &(label, style, action)) in items.iter().enumerate() {
            let bx = x + i as f32 * (share + GAP);
            ui.button(bx, y, Some(share), bh, label, style, action);
        }
        bh
    } else {
        let step = bh + GAP;
        for (i, &(label, style, action)) in items.iter().enumerate() {
            ui.button(x, y + i as f32 * step, Some(w), bh, label, style, action);
        }
        n * step - GAP
    }
}

/// The dialogs that say what a lock does ([`dialog`]): a title, a line,
/// rows of what happens to what, and their actions.
pub fn kv_sheet<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    w: f32,
    h: f32,
    head: (Icon, crate::pal::Paint, &str),
    sub: &str,
    rows: &[(&str, String, crate::pal::Paint)],
    actions: &[(&str, Style, A)],
) {
    use faraday_draw::tokens::DIALOG_MAX;
    let title = (head.0.into(), head.1, head.2);
    dialog(
        ui,
        (w, h, DIALOG_MAX),
        title,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            if !sub.is_empty() {
                cy += ui.wrap_as((x, cy, iw), BODY_M, Role::OnSurfaceVariant, sub) + PAD;
            }
            for (k, v, tone) in rows {
                cy += kv(ui, x, cy, iw, k, v, *tone);
            }
            cy - y
        },
        actions,
    );
}
