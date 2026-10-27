//! A ⋮ menu, wherever one is (`docs/BACKUP-FLOW.md` §17.20 item 5,
//! `docs/FLOWS.md` decision 61): on `wide` a popup under the ⋮ (above it
//! where it does not fit below), on the small panel a sheet with its way
//! out at its foot; its title, then its rows, each place's name over the
//! choices there. A press outside it closes it. The keyboard reaches it:
//! the arrows move, Enter or Space chooses, Escape closes. The backup
//! board's chips and the file rows share it; each works out its rows on
//! the press that opens it, and this draws what it was given.

use faraday_draw::tokens;
use faraday_draw::widgets::Icon;
use faraday_shell_api::Key as KeyIn;

use crate::Action;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{BODY_M, LABEL_M};

/// One row of a menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Line {
    /// A place's name, over the choices there.
    Head(String),
    /// A choice: what it reads, and the press it is.
    Item(String, Action),
}

/// A menu as it is drawn: what the press that opened it worked out.
pub(crate) struct View<'a> {
    /// Its title.
    pub title: &'a str,
    /// The glyph over its title on the small panel.
    pub icon: Icon,
    /// Its rows, in order.
    pub lines: &'a [Line],
    /// The choice the arrows are on, by its place among the choices.
    pub at: usize,
    /// Open or moved by a key: the arrows' choice is drawn so.
    pub keys: bool,
    /// Where the ⋮ was drawn, in pixels (x, y, w, h): the popup goes
    /// under it.
    pub anchor: Option<(i32, i32, i32, i32)>,
    /// The small panel's way out at the sheet's foot.
    pub cancel: (&'a str, Action),
    /// How much of the popup shows, 0 to 1, from its top edge down, while
    /// it opens or closes; 1 at rest.
    pub shown: f32,
}

/// What a key does in an open menu with `n` choices, the arrows on `at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    /// The arrows move to this choice.
    To(usize),
    /// The choice the arrows are on is chosen.
    Choose,
    /// The menu closes, back to its ⋮.
    Close,
}

/// What `key` does in an open menu (§19.20 item 5): Down and Up move,
/// round at the ends, Enter or Space chooses, Escape closes; `None` for
/// any other key, which the menu takes and does nothing with.
pub(crate) fn step(key: KeyIn, at: usize, n: usize) -> Option<Step> {
    match key {
        KeyIn::Down if n > 0 => Some(Step::To((at + 1) % n)),
        KeyIn::Up if n > 0 => Some(Step::To((at + n - 1) % n)),
        KeyIn::Enter | KeyIn::Char(' ') => Some(Step::Choose),
        KeyIn::Escape => Some(Step::Close),
        _ => None,
    }
}

/// Where a ⋮ moves among those on screen, `dots`, in the order drawn,
/// for Tab or Shift+Tab from `at`: the first or the last from none.
pub(crate) fn tab<T: PartialEq + Copy>(dots: &[T], at: Option<T>, back: bool) -> Option<T> {
    let n = dots.len();
    if n == 0 {
        return None;
    }
    let i = at.and_then(|c| dots.iter().position(|d| *d == c));
    let next = match (back, i) {
        (false, Some(i)) => (i + 1) % n,
        (false, None) => 0,
        (true, Some(i)) => (i + n - 1) % n,
        (true, None) => n - 1,
    };
    Some(dots[next])
}

/// The ⋮'s three dots, centred at (cx, cy) in `color`.
pub(crate) fn dots(ui: &mut Ui, cx: f32, cy: f32, color: crate::ui::Paint) {
    let d = tokens::BOARD_MENU_DOT;
    let step = (tokens::BOARD_MARK - d) / 2.0;
    for k in [-1.0, 0.0, 1.0] {
        ui.fill(cx - d / 2.0, cy + k * step - d / 2.0, d, d, d / 2.0, color);
    }
}

/// A ⋮ on its own, as a quiet square button `h` high at (x, y): a press
/// is `action`; edged in the accent while the keyboard is on it
/// (`focused`). Returns its width.
pub(crate) fn button(ui: &mut Ui, x: f32, y: f32, h: f32, action: Action, focused: bool) -> f32 {
    let w = h;
    let r = tokens::SHAPE_S;
    ui.state_layer((x, y, w, h, r), Role::OnSurface.paint(), action);
    ui.stroke(x, y, w, h, r, Role::Outline);
    dots(ui, x + w / 2.0, y + h / 2.0, Role::OnSurfaceVariant.paint());
    if focused {
        outline(
            ui,
            (x, y, w, h),
            r,
            tokens::FOCUS_STROKE,
            Role::Primary.paint(),
        );
    }
    ui.hit_quiet(x, y, w, h, action);
    w
}

/// A rounded rectangle's edge, `width` thick.
pub(crate) fn outline(
    ui: &mut Ui,
    r: (f32, f32, f32, f32),
    radius: f32,
    width: f32,
    color: crate::ui::Paint,
) {
    let rect = ui.rect(r.0, r.1, r.2, r.3);
    let c = ui.col(color);
    ui.c.stroke_rounded_rect(rect, radius * ui.f, width * ui.f, c);
}

/// What `draw` would take in height, drawn out of sight with no press
/// kept.
pub(crate) fn measure(ui: &mut Ui, draw: &mut dyn FnMut(&mut Ui) -> f32) -> f32 {
    let mark = (ui.hits.len(), ui.reach.len());
    ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
    let h = draw(ui);
    ui.c.pop_clip();
    ui.hits.truncate(mark.0);
    ui.reach.truncate(mark.1);
    h
}

/// A menu's rows from (x, y), `w` wide: each place's name over the
/// choices there, indented under it; the arrows' choice edged. Returns
/// their height.
fn rows(ui: &mut Ui, v: &View, x: f32, y: f32, w: f32) -> f32 {
    let rh = tokens::BOARD_MENU_ROW;
    let mut cy = y;
    let mut k = 0;
    let mut under = false;
    for line in v.lines {
        match line {
            Line::Head(name) => {
                if cy > y {
                    cy += tokens::GAP_SMALL;
                }
                let hx = x + tokens::GAP_SMALL;
                let shown = ui.fit_as(LABEL_M, name, w - tokens::GAP_SMALL);
                ui.text_mid_as(
                    (hx, cy, tokens::BOARD_ZONE_LABEL),
                    LABEL_M,
                    Role::OnSurfaceVariant,
                    &shown,
                );
                cy += tokens::BOARD_ZONE_LABEL;
                under = true;
            }
            Line::Item(label, a) => {
                let a = *a;
                let on = v.keys && v.at == k;
                if on {
                    ui.fill(
                        x,
                        cy,
                        w,
                        rh,
                        tokens::SHAPE_XS,
                        Role::SurfaceContainerHighest,
                    );
                    outline(
                        ui,
                        (x, cy, w, rh),
                        tokens::SHAPE_XS,
                        tokens::FOCUS_STROKE,
                        Role::Primary.paint(),
                    );
                }
                let ix = x + tokens::GAP_SMALL + if under { tokens::BOARD_PAD } else { 0.0 };
                let shown = ui.fit_as(BODY_M, label, x + w - ix - tokens::GAP_SMALL);
                ui.text_mid_as((ix, cy, rh), BODY_M, Role::OnSurface, &shown);
                ui.hit(x, cy, w, rh, a);
                cy += rh;
                k += 1;
            }
        }
    }
    cy - y
}

/// The menu drawn: a popup under its ⋮ on `wide`, a sheet on the small
/// panel (§19.20 item 5).
pub(crate) fn draw(ui: &mut Ui, w: f32, h: f32, v: &View) {
    if ui.compact {
        let place = (tokens::GAP, w - 2.0 * tokens::GAP);
        let pad = tokens::GRID * 2.0;
        crate::compact::scroll_sheet(ui, place, h, tokens::GAP, pad, &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += crate::compact::sheet_head(ui, x, cy, iw, v.icon, Role::OnSurface, v.title);
            cy += rows(ui, v, x, cy, iw) + tokens::GAP;
            cy += crate::compact::buttons(
                ui,
                x,
                cy,
                iw,
                &[(v.cancel.0, Style::Secondary, v.cancel.1)],
            );
            cy - y
        });
        return;
    }
    let pad = tokens::GAP_SMALL;
    let margin = tokens::GAP;
    let pw = tokens::BOARD_POPUP_WIDTH.min(w - 2.0 * margin);
    let iw = pw - 2.0 * pad;
    let body = |ui: &mut Ui, x: f32, y: f32| -> f32 {
        let shown = ui.fit_as(LABEL_M, v.title, iw - tokens::GAP_SMALL);
        ui.text_mid_as(
            (x + tokens::GAP_SMALL, y, tokens::BOARD_ZONE_LABEL),
            LABEL_M,
            Role::OnSurface,
            &shown,
        );
        tokens::BOARD_ZONE_LABEL + rows(ui, v, x, y + tokens::BOARD_ZONE_LABEL, iw)
    };
    let content = measure(ui, &mut |ui| body(ui, 0.0, 0.0)) + 2.0 * pad;
    let ph = content.min(h - 2.0 * margin);
    let f = ui.f.max(0.1);
    let (x, y) = match v.anchor {
        Some((ax, ay, aw, ah)) => {
            let (ax, ay) = (ax as f32 / f - ui.ox, ay as f32 / f - ui.oy);
            let (aw, ah) = (aw as f32 / f, ah as f32 / f);
            let x = (ax + aw - pw).clamp(margin, (w - pw - margin).max(margin));
            let y = if ay + ah + ph <= h - margin {
                ay + ah
            } else if ay - ph >= margin {
                ay - ph
            } else {
                (h - ph - margin).max(margin)
            };
            (x, y)
        }
        None => ((w - pw) / 2.0, (h - ph) / 2.0),
    };
    if let Some(a) = ui.outside {
        ui.hit_around(x, y, pw, ph, a);
    }
    let max = (content - ph).max(0.0);
    // Opening or closing, it unrolls from its top edge (Material's menu).
    let ph = (ph * v.shown.clamp(0.0, 1.0)).round();
    if ph <= 0.0 {
        return;
    }
    // Material's menu: extra-small corners.
    ui.shadow(x, y, pw, ph, tokens::SHAPE_XS, tokens::ELEVATION_MENU);
    ui.fill(x, y, pw, ph, tokens::SHAPE_XS, Role::SurfaceContainer);
    ui.stroke(x, y, pw, ph, tokens::SHAPE_XS, Role::OutlineVariant);
    let off = ui.offset.clamp(0.0, max);
    let clip = ui.rect(x, y, pw, ph);
    ui.c.push_clip(clip);
    let mark = ui.hits.len();
    body(ui, x + pad, y + pad - off);
    // What is scrolled out of the popup takes no press.
    let mut k = mark;
    while k < ui.hits.len() {
        let r = ui.hits[k].0.intersect(&clip);
        if r.w <= 0 || r.h <= 0 {
            ui.hits.remove(k);
        } else {
            ui.hits[k].0 = r;
            k += 1;
        }
    }
    ui.c.pop_clip();
    ui.report_scroll(clip, max);
}
