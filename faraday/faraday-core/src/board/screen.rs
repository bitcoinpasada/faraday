//! The backup board drawn (`docs/DESIGN.md` §4.17,
//! `docs/BACKUP-FLOW.md` §17): the place cards with their zones, the
//! **+** card, the software place, the list beside them, the check lines
//! under them, a placed chip's form toggles and the chip being dragged,
//! and the sheet over a drop that lets a place see the balance. It reads
//! the board as the frame's memo has it ([`Faraday::board_view_now`]) and the
//! chip held; it works nothing out but where things go.

use faraday_draw::tokens;
use faraday_draw::widgets::Icon;

use crate::Action;
use crate::board::{
    BoardPress, BoardView, CardView, Chip, ChipView, Entry, MenuRow, Thing, Zone, takes,
};
use crate::drawing::View;
use crate::menu::{measure, outline};
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Glyph, Style, Ui, W};

/// What one frame of the board knows while it draws.
struct Frame {
    /// The wallet's shape, for which zones take the chip held.
    shape: crate::plan::Shape,
    /// The chip held, by a tap or a drag.
    held: Option<Chip>,
    /// Where the dragged chip is, in units, while it is dragged.
    drag: Option<(f32, f32)>,
    /// After the lock.
    locked: bool,
    /// Where the held chip was drawn, and what it reads.
    held_at: Option<((f32, f32, f32, f32), ChipView)>,
    /// A stick's glyph: the USB stick, or the SD card on the Pi.
    stick_glyph: Glyph,
    /// The chip whose ⋮ the keyboard is on.
    focus: Option<Chip>,
}

impl Frame {
    /// Whether zone `z` (drawn at `r`) is edged: it takes the chip held,
    /// and while a chip is dragged, it is under it.
    fn edged(&self, z: Zone, r: (f32, f32, f32, f32)) -> bool {
        let Some(c) = self.held else {
            return false;
        };
        if matches!(c, Chip::Placed(from, _) if from == z) {
            return false;
        }
        let t = c.thing();
        if t == Thing::Software || crate::board::target(z, t) != z {
            return false;
        }
        if !takes(&self.shape, z, t, self.locked) {
            return false;
        }
        self.drag.is_none_or(|p| inside(p, r))
    }
}

/// Whether point `p` is in the box `r` (x, y, w, h).
fn inside(p: (f32, f32), r: (f32, f32, f32, f32)) -> bool {
    p.0 >= r.0 && p.0 < r.0 + r.2 && p.1 >= r.1 && p.1 < r.1 + r.3
}

/// The edge round what takes the chip held, or round the chip itself.
fn edge(ui: &mut Ui, r: (f32, f32, f32, f32), radius: f32, color: faraday_draw::Color) {
    let rect = ui.rect(r.0, r.1, r.2, r.3);
    let c = ui.col(color);
    ui.c.stroke_rounded_rect(rect, radius * ui.f, tokens::BOARD_EDGE * ui.f, c);
}

/// The × mark, a cross centred on (cx, cy).
fn cross(ui: &mut Ui, cx: f32, cy: f32, color: faraday_draw::Color) {
    let h = tokens::BOARD_MARK / 2.0;
    let (f, ox, oy) = (ui.f, ui.ox, ui.oy);
    let c = ui.col(color);
    let s = tokens::BOARD_MARK_STROKE * f;
    let px = |v: f32, o: f32| (v + o) * f;
    ui.c.line(
        px(cx - h, ox),
        px(cy - h, oy),
        px(cx + h, ox),
        px(cy + h, oy),
        s,
        c,
    );
    ui.c.line(
        px(cx - h, ox),
        px(cy + h, oy),
        px(cx + h, ox),
        px(cy - h, oy),
        s,
        c,
    );
}

/// The link mark at (x, cy): two links of a chain, for a vault drawn at
/// every place that keeps its stick.
fn link(ui: &mut Ui, x: f32, cy: f32, color: faraday_draw::Color) {
    let q = tokens::BOARD_MARK / 4.0;
    let rect_a = ui.rect(x, cy - q, 3.0 * q, 2.0 * q);
    let rect_b = ui.rect(x + q, cy - q, 3.0 * q, 2.0 * q);
    let c = ui.col(color);
    let (r, s) = (q * ui.f, tokens::BOARD_MARK_STROKE * ui.f);
    ui.c.stroke_rounded_rect(rect_a, r, s, c);
    ui.c.stroke_rounded_rect(rect_b, r, s, c);
}

/// A chip's glyph: the fingerprint before a seed's, the passphrase's
/// before its own, the closed lock before a vault's (§19.17 item 5).
fn glyph(t: Thing) -> Option<Icon> {
    match t {
        Thing::Seed(_) => Some(Icon::Fingerprint),
        Thing::Pass(_) => Some(Icon::Passphrase),
        Thing::Vault(_) => Some(Icon::Lock),
        _ => None,
    }
}

/// A chip's name is read character by character: a fingerprint.
fn mono(t: Thing) -> bool {
    matches!(t, Thing::Seed(_) | Thing::Pass(_))
}

/// A placed chip inside a vault: it carries the lock mark.
fn sealed(cv: &ChipView) -> bool {
    matches!(cv.chip, Chip::Placed(Zone::Vault(..), _))
}

/// What a chip's line needs beside its name and its marks: its glyph,
/// its link, warning, lock and × marks, each a mark and a small gap, and
/// its ⋮ part.
fn marks_w(cv: &ChipView) -> f32 {
    let one = tokens::BOARD_MARK + tokens::BOARD_CHIP_GAP;
    let t = cv.chip.thing();
    let n = usize::from(glyph(t).is_some())
        + usize::from(sealed(cv))
        + usize::from(cv.link)
        + usize::from(cv.warn)
        + usize::from(cv.off && matches!(cv.chip, Chip::Placed(..)) && !cv.fixed);
    n as f32 * one + if cv.menu { menu_w() } else { 0.0 }
}

/// The width of a chip's ⋮ part at its right end, from its rule: the ⋮
/// and its padding, a press target as the × is (`docs/BACKUP-FLOW.md`
/// §17.20 item 1).
fn menu_w() -> f32 {
    tokens::BOARD_MARK + 2.0 * tokens::BOARD_PAD
}

/// A chip's ⋮ part at (x, y), `h` tall (`docs/BACKUP-FLOW.md` §17.20):
/// a thin rule, then the three dots in `color`; a press opens the chip's
/// menu, and never drags or selects it. Edged in the accent while the
/// keyboard is on it (on a vault's band, in `VAULT_ON`). `hits`: it
/// takes presses.
#[allow(clippy::too_many_arguments)]
fn dots(
    ui: &mut Ui,
    fr: &Frame,
    c: Chip,
    x: f32,
    y: f32,
    h: f32,
    color: faraday_draw::Color,
    hits: bool,
) {
    let mw = menu_w();
    let a = Action::Board(BoardPress::Menu(c));
    if hits && (ui.is_pressed(a) || ui.is_hovered(a)) {
        ui.fill(x + 1.0, y + 1.0, mw - 2.0, h - 2.0, (h - 2.0) / 2.0, INNER);
    }
    let inset = tokens::GAP_SMALL;
    ui.fill(
        x,
        y + inset,
        1.0,
        h - 2.0 * inset,
        0.0,
        color.with_alpha(tokens::BOARD_MENU_RULE),
    );
    crate::menu::dots(ui, x + mw / 2.0, y + h / 2.0, color);
    if fr.focus == Some(c) {
        outline(
            ui,
            (x + 1.0, y + 1.0, mw - 2.0, h - 2.0),
            (h - 2.0) / 2.0,
            tokens::FOCUS_STROKE,
            // On a vault's band, whose fill is the accent's kin, the
            // band's own text colour.
            if color == VAULT_ON { VAULT_ON } else { ACCENT },
        );
    }
    if hits {
        ui.hit(x, y, mw, h, a);
    }
}

/// How a chip is laid out: its size, and the size its name is drawn at.
#[derive(Clone, Copy)]
struct Fit {
    /// Its width.
    w: f32,
    /// Its height: a chip's, or with its forms on a line under its name.
    h: f32,
    /// Its name's type size.
    size: f32,
}

/// How a chip is laid out at most `avail` wide: its name at the label
/// size with its forms beside it, or under it; else at the caption size,
/// the same two ways. A fingerprint is never cut (`docs/DESIGN.md` §4.4):
/// its chip is at least as wide as it.
fn chip_size(ui: &Ui, cv: &ChipView, avail: f32) -> Fit {
    let t = cv.chip.thing();
    let face = if mono(t) { W::M } else { W::R };
    let base = 2.0 * tokens::BOARD_PAD + marks_w(cv);
    let marks = if !cv.forms.is_empty() {
        tokens::BOARD_CHIP_GAP + toggles_w(ui, cv)
    } else if cv.marks.is_empty() {
        0.0
    } else {
        tokens::BOARD_CHIP_GAP + ui.measure(tokens::CAPTION, W::R, &format!("· {}", cv.marks))
    };
    let one = tokens::BOARD_CHIP;
    let two = if cv.forms.is_empty() {
        tokens::BOARD_CHIP + ui.line(tokens::CAPTION, W::R)
    } else {
        tokens::BOARD_CHIP + tokens::BOARD_TAG
    };
    let has_marks = !cv.marks.is_empty() || !cv.forms.is_empty();
    for size in [tokens::LABEL, tokens::CAPTION] {
        let need = base + ui.measure(size, face, &cv.name);
        if need + marks <= avail {
            return Fit {
                w: need + marks,
                h: one,
                size,
            };
        }
        if need <= avail && has_marks {
            return Fit {
                w: avail,
                h: two,
                size,
            };
        }
    }
    let need = base + ui.measure(tokens::CAPTION, face, &cv.name);
    let least = if mono(t) { need } else { 0.0 };
    Fit {
        w: avail.max(least),
        h: if has_marks { two } else { one },
        size: tokens::CAPTION,
    }
}

/// One form toggle's width: its check mark's room and its name.
fn toggle_w(ui: &Ui, label: &str) -> f32 {
    2.0 * tokens::BOARD_PAD
        + tokens::BOARD_MARK
        + tokens::BOARD_CHIP_GAP
        + ui.measure(tokens::CAPTION, W::R, label)
}

/// The width a chip's form toggles take in a row.
fn toggles_w(ui: &Ui, cv: &ChipView) -> f32 {
    let n = cv.forms.len();
    cv.forms.iter().map(|(l, _)| toggle_w(ui, l)).sum::<f32>()
        + n.saturating_sub(1) as f32 * tokens::BOARD_CHIP_GAP
}

/// A chip's form toggles drawn from (x, y), a line `line_h` tall
/// (`docs/BACKUP-FLOW.md` §17.3): each its name in a pill, ticked where
/// the form is kept; a press turns it on or off. `hits`: they take
/// presses.
fn toggles(ui: &mut Ui, cv: &ChipView, x: f32, y: f32, line_h: f32, hits: bool) {
    let Chip::Placed(z, t) = cv.chip else {
        return;
    };
    let th = tokens::BOARD_TAG;
    let ty = y + (line_h - th) / 2.0;
    let mut tx = x;
    for (k, (label, on)) in cv.forms.iter().enumerate() {
        let tw = toggle_w(ui, label);
        let action = Action::Board(BoardPress::Form(z, t, u8::try_from(k).unwrap_or(u8::MAX)));
        let hot = hits && (ui.is_pressed(action) || ui.is_hovered(action));
        ui.fill(tx, ty, tw, th, th / 2.0, if hot { INNER } else { SURFACE });
        ui.stroke(tx, ty, tw, th, th / 2.0, if *on { ACCENT } else { LINE });
        let mut lx = tx + tokens::BOARD_PAD;
        if *on {
            ui.icon(
                lx,
                ty + (th - tokens::BOARD_MARK) / 2.0,
                tokens::BOARD_MARK,
                Icon::Check,
                tokens::BOARD_MARK,
                ACCENT,
            );
        }
        lx += tokens::BOARD_MARK + tokens::BOARD_CHIP_GAP;
        ui.text_mid(
            lx,
            ty,
            th,
            tokens::CAPTION,
            W::R,
            if *on { TEXT } else { MUTED },
            label,
        );
        if hits {
            ui.hit_quiet(tx, ty, tw, th, action);
        }
        tx += tw + tokens::BOARD_CHIP_GAP;
    }
}

/// Draws a chip at (x, y), laid out as [`chip_size`] gave it. `hits`:
/// it takes presses (not the copy that follows a drag).
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn chip(ui: &mut Ui, fr: &mut Frame, cv: &ChipView, x: f32, y: f32, fit: Fit, hits: bool) {
    let (w, h) = (fit.w, fit.h);
    let t = cv.chip.thing();
    let r = tokens::BOARD_CHIP / 2.0;
    let held = fr.held == Some(cv.chip);
    let lifted = held && fr.drag.is_some() && hits;
    let action = Action::Board(BoardPress::Chip(cv.chip));
    let hot = hits && !cv.fixed && (ui.is_pressed(action) || ui.is_hovered(action));
    // A chip is the surface with a stronger neutral edge and a faint
    // shadow, so it stands on every box's fill: paper, a stick box, a
    // vault box, the list (§19.15).
    if cv.fixed {
        ui.stroke(x, y, w, h, r, CHIP_EDGE);
    } else {
        ui.shadow(x, y, w, h, r, tokens::ELEVATION_SHEET);
        ui.fill(x, y, w, h, r, if hot { INNER } else { SURFACE });
        ui.stroke(x, y, w, h, r, CHIP_EDGE);
    }
    let fg = if cv.fixed || cv.faint || lifted {
        DIM
    } else {
        TEXT
    };
    let mut cx = x + tokens::BOARD_PAD;
    let line_h = tokens::BOARD_CHIP;
    if let Some(g) = glyph(t) {
        ui.icon(
            cx,
            y + (line_h - tokens::BOARD_MARK) / 2.0,
            tokens::BOARD_MARK,
            g,
            tokens::BOARD_MARK,
            if cv.fixed || cv.faint { DIM } else { MUTED },
        );
        cx += tokens::BOARD_MARK + tokens::BOARD_CHIP_GAP;
    }
    // The marks at the right, from the edge in: the ⋮ after its rule, ×,
    // warning, link.
    let mw = if cv.menu { menu_w() } else { 0.0 };
    let mut right = x + w - mw - tokens::BOARD_PAD;
    let off = cv.off && !cv.fixed && matches!(cv.chip, Chip::Placed(..));
    if off {
        cross(
            ui,
            right - tokens::BOARD_MARK / 2.0,
            y + line_h / 2.0,
            MUTED,
        );
        if hits && let Chip::Placed(z, t) = cv.chip {
            let side = tokens::BOARD_MARK + 2.0 * tokens::BOARD_PAD;
            ui.hit(
                x + w - mw - side,
                y,
                side,
                line_h,
                Action::Board(BoardPress::Off(z, t)),
            );
        }
        right -= tokens::BOARD_MARK + tokens::BOARD_CHIP_GAP;
    }
    if cv.warn {
        ui.icon(
            right - tokens::BOARD_MARK,
            y + (line_h - tokens::BOARD_MARK) / 2.0,
            tokens::BOARD_MARK,
            Icon::Warning,
            tokens::BOARD_MARK,
            WARN,
        );
        right -= tokens::BOARD_MARK + tokens::BOARD_CHIP_GAP;
    }
    if cv.link {
        link(ui, right - tokens::BOARD_MARK, y + line_h / 2.0, ACCENT);
        right -= tokens::BOARD_MARK + tokens::BOARD_CHIP_GAP;
    }
    if sealed(cv) {
        ui.icon(
            right - tokens::BOARD_MARK,
            y + (line_h - tokens::BOARD_MARK) / 2.0,
            tokens::BOARD_MARK,
            Icon::Lock,
            tokens::BOARD_MARK,
            VAULT_EDGE,
        );
        right -= tokens::BOARD_MARK + tokens::BOARD_CHIP_GAP;
    }
    let face = if mono(t) { W::M } else { W::R };
    let size = fit.size;
    // A fingerprint is never cut; anything else is fitted.
    let name = if mono(t) {
        cv.name.clone()
    } else {
        // Laid out to fit (`chip_size`): the half unit is the rounding of
        // a box placed at a fraction of a pixel.
        ui.fit(size, face, &cv.name, (right - cx).max(0.0) + 0.5)
    };
    let nw = ui.text_mid(cx, y, line_h, size, face, fg, &name);
    // The form toggles, beside the name or on a line under it; drawn
    // after the chip's own press so a press on one is the toggle's.
    let toggles_at = (!cv.forms.is_empty()).then_some({
        if h > line_h {
            (x + tokens::BOARD_PAD, y + line_h - tokens::BOARD_CHIP_GAP)
        } else {
            (cx + nw + tokens::BOARD_CHIP_GAP, y)
        }
    });
    if !cv.marks.is_empty() {
        let marks = format!("· {}", cv.marks);
        if h > line_h {
            let shown = ui.fit(
                tokens::CAPTION,
                W::R,
                &cv.marks,
                w - 2.0 * tokens::BOARD_PAD,
            );
            ui.text(
                x + tokens::BOARD_PAD,
                y + line_h - tokens::BOARD_CHIP_GAP,
                tokens::CAPTION,
                W::R,
                MUTED,
                &shown,
            );
        } else {
            // Laid out to fit beside the name (`chip_size`): drawn whole.
            let mx = cx + nw + tokens::BOARD_CHIP_GAP;
            ui.text_mid(mx, y, line_h, tokens::CAPTION, W::R, MUTED, &marks);
        }
    }
    if cv.warn && !held {
        ui.stroke(x, y, w, h, r, WARN);
    }
    if held && !lifted {
        edge(ui, (x, y, w, h), r, ACCENT);
    }
    if hits && !cv.fixed {
        // The chip's body; its × over it at the right. With toggles, the
        // body is its name's part: a press on a toggle is the toggle's.
        let body = if off {
            w - mw - tokens::BOARD_MARK - 2.0 * tokens::BOARD_PAD
        } else {
            w - mw
        };
        let (body, body_h) = match toggles_at {
            Some((tx, _)) if h <= line_h => (tx - x, h),
            Some(_) => (body, line_h),
            None => (body, h),
        };
        ui.hit_quiet(x, y, body, body_h, action);
        if held {
            fr.held_at = Some(((x, y, w, h), cv.clone()));
        }
    }
    // The ⋮, its own press target over the chip's right end, after the
    // chip's own press (§19.20 item 1).
    if cv.menu {
        let color = if lifted || cv.faint { DIM } else { MUTED };
        dots(
            ui,
            fr,
            cv.chip,
            x + w - mw,
            y,
            line_h,
            color,
            hits && !cv.fixed,
        );
    }
    if let Some((tx, ty)) = toggles_at {
        let th = if h > line_h {
            tokens::BOARD_TAG
        } else {
            line_h
        };
        toggles(ui, cv, tx, ty, th, hits && !cv.fixed);
    }
}

/// A zone's chips laid out from (x, y) in `w`: each chip's place and
/// size, and the height they take.
fn flow(ui: &Ui, chips: &[ChipView], x: f32, y: f32, w: f32) -> (Vec<(f32, f32, Fit)>, f32) {
    let mut out = Vec::with_capacity(chips.len());
    let (mut cx, mut cy, mut row_h) = (x, y, 0.0f32);
    for cv in chips {
        let fit = chip_size(ui, cv, w);
        if cx > x && cx + fit.w > x + w {
            cx = x;
            cy += row_h + tokens::BOARD_CHIP_GAP;
            row_h = 0.0;
        }
        out.push((cx, cy, fit));
        cx += fit.w + tokens::BOARD_CHIP_GAP;
        row_h = row_h.max(fit.h);
    }
    let h = if chips.is_empty() {
        0.0
    } else {
        cy + row_h - y
    };
    (out, h)
}

/// A box (x, y, w, h).
type Box4 = (f32, f32, f32, f32);

/// A box's edge drawn in dashes along its sides, `color`, at
/// `BOARD_BOX_EDGE`: **+ stick** and **+ place**. The corners, `radius`
/// round, are left open.
fn dashed(ui: &mut Ui, r: Box4, radius: f32, color: faraday_draw::Color) {
    let (f, ox, oy) = (ui.f, ui.ox, ui.oy);
    let c = ui.col(color);
    let s = tokens::BOARD_BOX_EDGE * f;
    let d = tokens::BOARD_DASH;
    let half = tokens::BOARD_BOX_EDGE / 2.0;
    let (x0, y0) = (r.0 + half, r.1 + half);
    let (x1, y1) = (r.0 + r.2 - half, r.1 + r.3 - half);
    // Each side from one corner's end to the next's start, a dash and a
    // gap at a time.
    let sides = [
        (x0 + radius, y0, x1 - radius, y0),
        (x0 + radius, y1, x1 - radius, y1),
        (x0, y0 + radius, x0, y1 - radius),
        (x1, y0 + radius, x1, y1 - radius),
    ];
    for (ax, ay, bx, by) in sides {
        let len = (bx - ax).abs().max((by - ay).abs());
        if len <= 0.0 {
            continue;
        }
        let (ux, uy) = ((bx - ax) / len, (by - ay) / len);
        let mut t = 0.0;
        while t < len {
            let e = (t + d).min(len);
            ui.c.line(
                (ax + ux * t + ox) * f,
                (ay + uy * t + oy) * f,
                (ax + ux * e + ox) * f,
                (ay + uy * e + oy) * f,
                s,
                c,
            );
            t += 2.0 * d;
        }
    }
}

/// A box with its top corners rounded and its bottom square: a header
/// band.
fn band(ui: &mut Ui, x: f32, y: f32, w: f32, h: f32, color: faraday_draw::Color) {
    let r = tokens::GAP;
    ui.fill(x, y, w, h, r, color);
    ui.fill(x, y + h - r, w, r, 0.0, color);
}

/// What a zone's chips take `w` wide, under its label row: Paper's and
/// Scanned in's height, a chip's at least.
fn paper_h(ui: &Ui, chips: &[ChipView], w: f32) -> f32 {
    let pad = tokens::BOARD_PAD;
    let (_, ch) = flow(ui, chips, 0.0, 0.0, w - 2.0 * pad);
    tokens::BOARD_ZONE_LABEL + ch.max(tokens::BOARD_CHIP) + pad
}

/// A zone box's colours: its fill, its edge, and its label's.
type Tones = (
    faraday_draw::Color,
    faraday_draw::Color,
    faraday_draw::Color,
);

/// Paper and Scanned in, in the paper colours.
const PAPER: Tones = (PAPER_FILL, PAPER_EDGE, PAPER_TEXT);

/// A stick's files box, in the files colours (§19.17).
const FILES: Tones = (FILES_FILL, FILES_EDGE, FILES_TEXT);

/// Paper, or Scanned in at the software place, or a stick's files box
/// (`docs/DESIGN.md` §4.17): a box `r` in its colours, its glyph and
/// label in its header row, then its chips; it takes presses (and drops)
/// as zone `z`.
#[allow(clippy::too_many_arguments)]
fn paper(
    ui: &mut Ui,
    fr: &mut Frame,
    z: Zone,
    label: &str,
    chips: &[ChipView],
    r: Box4,
    tones: Tones,
    hot: Box4,
) {
    let (x, y, w, h) = r;
    let (fill, edge_c, text) = tones;
    let pad = tokens::BOARD_PAD;
    let lh = tokens::BOARD_ZONE_LABEL;
    let (places, _) = flow(ui, chips, x + pad, y + lh, w - 2.0 * pad);
    ui.fill(x, y, w, h, tokens::GAP, fill);
    outline(ui, r, tokens::GAP, tokens::BOARD_BOX_EDGE, edge_c);
    ui.hit_quiet(x, y, w, h, Action::Board(BoardPress::Zone(z)));
    let mark = tokens::BOARD_MARK;
    ui.icon(x + pad, y + (lh - mark) / 2.0, mark, Icon::File, mark, text);
    ui.text_mid(
        x + pad + mark + tokens::BOARD_CHIP_GAP,
        y,
        lh,
        tokens::CAPTION,
        W::S,
        text,
        label,
    );
    for (cv, at) in chips.iter().zip(places) {
        chip(ui, fr, cv, at.0, at.1, at.2, true);
    }
    // Edged where it takes the chip held; while it is dragged, with the
    // pointer in `hot` (the box itself, or for a files box the stick box
    // round it outside its vault, §19.17 item 2).
    if fr.edged(z, hot) {
        edge(ui, r, tokens::GAP, ACCENT);
    }
}

/// A stick box `r` with its head: the stick glyph and "Stick n" in slate.
/// Returns the head's height.
fn stick_box(ui: &mut Ui, glyph: Glyph, label: &str, r: Box4) -> f32 {
    let (x, y, w, h) = r;
    let sp = tokens::BOARD_STICK_PAD;
    let hh = tokens::BOARD_STICK_HEAD;
    let mark = tokens::BOARD_MARK;
    ui.fill(x, y, w, h, tokens::GAP, STICK_FILL);
    outline(ui, r, tokens::GAP, tokens::BOARD_STICK_EDGE, STICK_EDGE);
    let gx = x + sp + tokens::GAP_SMALL;
    ui.icon(gx, y + (hh - mark) / 2.0, mark, glyph, mark, STICK_TEXT);
    ui.text_mid(
        gx + mark + tokens::BOARD_CHIP_GAP,
        y,
        hh,
        tokens::CAPTION,
        W::S,
        STICK_TEXT,
        label,
    );
    hh
}

/// The **+ stick** box `r`: a new stick for what is dropped on it, in the
/// stick's fill with its edge dashed.
fn add_stick(ui: &mut Ui, fr: &Frame, p: usize, r: Box4) {
    let (x, y, w, h) = r;
    let z = Zone::NewStick(p);
    let a = Action::Board(BoardPress::Zone(z));
    // While a chip it takes is held, its edge is drawn solid, at every
    // place (§19.16 item 2).
    let takes_held = fr.held.is_some_and(|c| {
        takes(&fr.shape, z, c.thing(), fr.locked) && !matches!(c, Chip::List(Thing::Software))
    });
    ui.fill(x, y, w, h, tokens::GAP, STICK_FILL);
    if takes_held {
        outline(ui, r, tokens::GAP, tokens::BOARD_BOX_EDGE, STICK_EDGE);
    } else {
        dashed(ui, r, tokens::GAP, STICK_EDGE);
    }
    let lw = ui.measure(tokens::CAPTION, W::S, EN.board_add_stick);
    ui.text_mid(
        x + ((w - lw) / 2.0).max(tokens::GAP_SMALL),
        y,
        h,
        tokens::CAPTION,
        W::S,
        STICK_TEXT,
        EN.board_add_stick,
    );
    ui.hit_quiet(x, y, w, h, a);
    let on = takes_held && fr.drag.is_none_or(|d| inside(d, r));
    if on {
        edge(ui, r, tokens::GAP, ACCENT);
    }
}

/// Where a place's parts go (`docs/BACKUP-FLOW.md` §17.15 item 1): each
/// stick box, **+ stick** where the place has it, and Paper (or Scanned
/// in), and the height they take.
struct Row {
    /// Each stick box.
    sticks: Vec<Box4>,
    /// **+ stick**.
    add: Option<Box4>,
    /// Paper, or Scanned in.
    paper: Box4,
    /// The height of it all.
    h: f32,
}

/// A place's parts laid out from (x, y) in `w`: on `wide` left to right,
/// the sticks sharing the width with Paper and wrapping onto further
/// lines inside the place when they do not fit at `BOARD_BOX_MIN` each,
/// every box as tall as the row (a line of sticks as tall as its
/// tallest); in the narrow column one under another. `stick_h` and
/// `paper_h` give a box's own height at a width.
#[allow(clippy::too_many_arguments)]
fn row(
    stick_h: &dyn Fn(usize, f32) -> f32,
    paper_h: &dyn Fn(f32) -> f32,
    n: usize,
    add: bool,
    narrow: bool,
    x: f32,
    y: f32,
    w: f32,
) -> Row {
    let g = tokens::BOARD_PAD;
    let mut sticks = Vec::with_capacity(n);
    if narrow {
        let mut cy = y;
        for i in 0..n {
            let h = stick_h(i, w);
            sticks.push((x, cy, w, h));
            cy += h + g;
        }
        let add = add.then(|| {
            let r = (x, cy, w, tokens::BOARD_CHIP);
            cy += tokens::BOARD_CHIP + g;
            r
        });
        let ph = paper_h(w);
        return Row {
            sticks,
            add,
            paper: (x, cy, w, ph),
            h: cy + ph - y,
        };
    }
    let a = if add {
        tokens::BOARD_ADD_STICK + g
    } else {
        0.0
    };
    let parts = (n + 1) as f32;
    let per = (w - a - g * (parts - 1.0)) / parts;
    let (area, pw, k) = if n == 0 {
        (0.0, w - a, 0)
    } else if per >= tokens::BOARD_BOX_MIN {
        (w - a - per - g, per, n)
    } else {
        let pw = tokens::BOARD_BOX_MIN.min(w / 2.0);
        let area = w - a - pw - g;
        let k = (((area + g) / (tokens::BOARD_BOX_MIN + g)).floor() as usize).clamp(1, n);
        (area, pw, k)
    };
    let sw = if k > 0 {
        (area - g * (k - 1) as f32) / k as f32
    } else {
        0.0
    };
    let mut ly = y;
    let mut lines: Vec<(usize, usize, f32)> = Vec::new();
    for start in (0..n).step_by(k.max(1)) {
        let end = (start + k).min(n);
        let lh = (start..end).map(|i| stick_h(i, sw)).fold(0.0f32, f32::max);
        lines.push((start, end, lh));
        ly += lh + g;
    }
    let sticks_h = if n == 0 { 0.0 } else { ly - g - y };
    let ph = paper_h(pw);
    let h = sticks_h.max(ph).max(tokens::BOARD_CHIP);
    let mut ly = y;
    let last = lines.len();
    for (j, (start, end, lh)) in lines.into_iter().enumerate() {
        // The last line takes what the row has over the sticks' height.
        let lh = if j + 1 == last { lh + h - sticks_h } else { lh };
        for i in start..end {
            sticks.push((x + (i - start) as f32 * (sw + g), ly, sw, lh));
        }
        ly += lh + g;
    }
    let ax = x + if n == 0 { 0.0 } else { area + g };
    Row {
        sticks,
        add: add.then_some((ax, y, tokens::BOARD_ADD_STICK, h)),
        paper: (ax + a, y, pw, h),
        h,
    }
}

/// A place's header band at (x, y), `w` wide (`docs/DESIGN.md` §4.17):
/// the place glyph, its name (a field while names are typed), its lines
/// of fact as outlined tags in their tones, one after another as far as
/// they fit, and its × when it can go.
#[allow(clippy::too_many_arguments)]
fn place_head(
    ui: &mut Ui,
    name: &str,
    facts: &[(&str, faraday_draw::Color)],
    x: f32,
    y: f32,
    w: f32,
    name_press: Option<(Action, bool)>,
    off: Option<Action>,
) {
    let pad = tokens::BOARD_PAD;
    let hh = tokens::BOARD_HEAD;
    let mark = tokens::BOARD_MARK;
    band(ui, x, y, w, hh, PLACE_BAND);
    ui.icon(
        x + pad,
        y + (hh - mark) / 2.0,
        mark,
        Icon::House,
        mark,
        PLACE_TEXT,
    );
    let nx = x + pad + mark + tokens::BOARD_CHIP_GAP;
    let right = x + w - pad - mark - pad;
    let tag_ws: Vec<f32> = facts
        .iter()
        .map(|(t, _)| ui.measure(tokens::CAPTION, W::R, t) + 2.0 * pad + tokens::GAP)
        .collect();
    let tag_w: f32 = tag_ws.iter().sum();
    let room = (right - nx - tag_w).max((right - nx) / 2.0).max(0.0);
    // While it is typed, the name is a line of typing: just pressed, the
    // whole of it selected and no caret (§19.18).
    let nw = match name_press {
        Some((a, true)) => {
            let nw = ui.typed(
                nx,
                y,
                hh,
                tokens::LABEL,
                W::S,
                PLACE_TEXT,
                name,
                room,
                true,
                a,
            );
            ui.rule(nx, y + hh - tokens::GAP_SMALL, room, ACCENT);
            nw
        }
        _ => {
            let shown = ui.fit(tokens::LABEL, W::S, name, room);
            ui.text_mid(nx, y, hh, tokens::LABEL, W::S, PLACE_TEXT, &shown)
        }
    };
    if let Some((a, _)) = name_press {
        ui.hit(nx, y, room, hh, a);
    }
    let mut tx = nx + nw + tokens::GAP;
    for (&(text, tone), &want) in facts.iter().zip(&tag_ws) {
        let tw = (want - tokens::GAP).min(right - tx);
        // A tag that cannot show its first letters is left off, and so is
        // every one after it: one with less room for its text than the
        // tag is tall would show no more than "…".
        if tw <= 2.0 * pad + tokens::BOARD_TAG {
            break;
        }
        let th = tokens::BOARD_TAG;
        let ty = y + (hh - th) / 2.0;
        ui.fill(tx, ty, tw, th, th / 2.0, SURFACE);
        ui.stroke(tx, ty, tw, th, th / 2.0, tone);
        // Laid out to fit (`tag_ws`): the half unit is the rounding of a
        // box placed at a fraction of a pixel.
        let t = ui.fit(tokens::CAPTION, W::R, text, tw - 2.0 * pad + 0.5);
        ui.text_mid(tx + pad, ty, th, tokens::CAPTION, W::R, tone, &t);
        tx += want;
    }
    if let Some(a) = off {
        cross(ui, x + w - pad - mark / 2.0, y + hh / 2.0, PLACE_TEXT);
        let side = mark + 2.0 * pad;
        ui.hit(x + w - side, y, side, hh, a);
    }
}

/// A place's box `h` tall at (x, y), `w` wide: a pale green fill, a green
/// edge with a thick left stripe. Returns the inside's left edge and
/// width, under the header band.
fn place_box(ui: &mut Ui, x: f32, y: f32, w: f32, h: f32) -> (f32, f32) {
    let e = tokens::BOARD_BOX_EDGE;
    let stripe = tokens::BOARD_PLACE_STRIPE;
    ui.fill(x, y, w, h, tokens::RADIUS, PLACE_EDGE);
    ui.fill(
        x + stripe,
        y + e,
        w - stripe - e,
        h - 2.0 * e,
        tokens::GAP,
        PLACE_FILL,
    );
    (x + stripe, w - stripe - e)
}

/// Place `cv` at (x, y), `w` wide (`docs/BACKUP-FLOW.md` §17.15): its
/// header band, then its sticks, **+ stick** and Paper. Returns its
/// height.
fn place(ui: &mut Ui, fr: &mut Frame, v: &BoardView, cv: &CardView, x: f32, y: f32, w: f32) -> f32 {
    let pad = tokens::BOARD_PAD;
    let e = tokens::BOARD_BOX_EDGE;
    let stripe = tokens::BOARD_PLACE_STRIPE;
    let (ix, iw) = (x + stripe + pad, w - stripe - e - 2.0 * pad);
    let top = y + e + tokens::BOARD_HEAD + pad;
    let lay = {
        let ui_ref: &Ui = ui;
        row(
            &|i, sw| stick_h(ui_ref, &cv.sticks[i], sw),
            &|pw| paper_h(ui_ref, &cv.paper.chips, pw),
            cv.sticks.len(),
            v.custom,
            ui_ref.compact,
            ix,
            top,
            iw,
        )
    };
    let h = top + lay.h + pad + e - y;
    let (bx, bw) = place_box(ui, x, y, w, h);
    ui.hit_quiet(
        x,
        y,
        w,
        h,
        Action::Board(BoardPress::Zone(Zone::Card(cv.p))),
    );
    // A vault's chip held goes to the place as a whole.
    if let Some(c) = fr.held
        && let Thing::Vault(_) = c.thing()
        && takes(&fr.shape, Zone::Card(cv.p), c.thing(), fr.locked)
        && !matches!(c, Chip::Placed(Zone::Files(p, _), _) if p == cv.p)
        && fr.drag.is_none_or(|d| inside(d, (x, y, w, h)))
    {
        edge(ui, (x, y, w, h), tokens::RADIUS, ACCENT);
    }
    let p8 = u8::try_from(cv.p).unwrap_or(u8::MAX);
    let removable = v.custom && v.places.len() > 1 && (!v.locked || cv.p + 1 == v.places.len());
    let name_press = v
        .names_typed
        .then_some((Action::BName(p8), v.naming == Some(cv.p)));
    place_head(
        ui,
        &cv.name,
        &cv.facts
            .iter()
            .map(|(t, k)| (t.as_str(), crate::glance::alone_color(*k)))
            .collect::<Vec<_>>(),
        bx,
        y + e,
        bw,
        name_press,
        removable.then_some(Action::Board(BoardPress::RemovePlace(cv.p))),
    );
    for (st, r) in cv.sticks.iter().zip(&lay.sticks) {
        stick(ui, fr, st, *r, v.custom);
    }
    if let Some(r) = lay.add {
        add_stick(ui, fr, cv.p, r);
    }
    paper(
        ui,
        fr,
        cv.paper.zone,
        EN.board_paper,
        &cv.paper.chips,
        lay.paper,
        PAPER,
        lay.paper,
    );
    h
}

/// A vault box's header band at (x, y), `w` wide: the vault's glyph and
/// "Vault n · sealed" (`label`, or `short` where it does not fit) on the
/// solid band, its link and warning marks, and the × that takes its stick
/// off this place. The band is the vault's chip: a press selects it, a
/// drag moves its stick.
fn vault_head(
    ui: &mut Ui,
    fr: &mut Frame,
    vc: &ChipView,
    (label, short): (&str, &str),
    x: f32,
    y: f32,
    w: f32,
) {
    let hh = tokens::BOARD_VAULT_HEAD;
    let pad = tokens::BOARD_VAULT_PAD + tokens::GAP_SMALL;
    let mark = tokens::BOARD_MARK;
    let action = Action::Board(BoardPress::Chip(vc.chip));
    let held = fr.held == Some(vc.chip);
    band(ui, x, y, w, hh, VAULT_EDGE);
    if ui.is_pressed(action) || ui.is_hovered(action) {
        band(
            ui,
            x,
            y,
            w,
            hh,
            VAULT_ON.with_alpha(tokens::BOARD_VAULT_HOT),
        );
    }
    ui.icon(
        x + pad,
        y + (hh - mark) / 2.0,
        mark,
        Icon::Lock,
        mark,
        VAULT_ON,
    );
    let mw = if vc.menu { menu_w() } else { 0.0 };
    let mut right = x + w - mw - pad;
    let mut off_w = 0.0;
    if vc.off
        && let Chip::Placed(z, t) = vc.chip
    {
        cross(ui, right - mark / 2.0, y + hh / 2.0, VAULT_ON);
        off_w = mark + 2.0 * pad;
        ui.hit(
            x + w - mw - off_w,
            y,
            off_w,
            hh,
            Action::Board(BoardPress::Off(z, t)),
        );
        right -= mark + tokens::BOARD_CHIP_GAP;
    }
    if vc.warn {
        ui.icon(
            right - mark,
            y + (hh - mark) / 2.0,
            mark,
            Icon::Warning,
            mark,
            VAULT_ON,
        );
        right -= mark + tokens::BOARD_CHIP_GAP;
    }
    if vc.link {
        link(ui, right - mark, y + hh / 2.0, VAULT_ON);
        right -= mark + tokens::BOARD_CHIP_GAP;
    }
    let nx = x + pad + mark + tokens::BOARD_CHIP_GAP;
    let room = (right - nx).max(0.0);
    let label = if ui.measure(tokens::LABEL, W::S, label) > room {
        short
    } else {
        label
    };
    let name = ui.fit(tokens::LABEL, W::S, label, room);
    ui.text_mid(nx, y, hh, tokens::LABEL, W::S, VAULT_ON, &name);
    if held && fr.drag.is_none() {
        let i = tokens::BOARD_EDGE;
        edge(
            ui,
            (x + i, y + i, w - 2.0 * i, hh - 2.0 * i),
            tokens::GAP_SMALL,
            VAULT_ON,
        );
    }
    ui.hit_quiet(x, y, w - mw - off_w, hh, action);
    if held {
        fr.held_at = Some(((x, y, w, hh), vc.clone()));
    }
    if vc.menu {
        dots(ui, fr, vc.chip, x + w - mw, y, hh, VAULT_ON, true);
    }
}

/// A stick box's own height `w` wide: its head, its files box, its vault
/// box.
fn stick_h(ui: &Ui, st: &crate::board::StickView, w: f32) -> f32 {
    let (files_h, vault_h) = stick_parts(ui, st, w);
    let vault = if vault_h > 0.0 {
        tokens::BOARD_STICK_PAD + vault_h
    } else {
        0.0
    };
    tokens::BOARD_STICK_HEAD + files_h + vault + tokens::BOARD_STICK_PAD
}

/// A stick box's parts `w` wide: its files box's height (a chip row at
/// least, empty), and its vault box's (none without a vault).
fn stick_parts(ui: &Ui, st: &crate::board::StickView, w: f32) -> (f32, f32) {
    let sp = tokens::BOARD_STICK_PAD;
    let vp = tokens::BOARD_VAULT_PAD;
    let iw = w - 2.0 * sp;
    let vault_h = st.vault.as_ref().map_or(0.0, |(_, vz)| {
        let (_, inner_h) = flow(ui, &vz.chips, 0.0, 0.0, iw - 2.0 * vp);
        tokens::BOARD_VAULT_HEAD + vp + inner_h.max(tokens::BOARD_CHIP) + vp
    });
    (paper_h(ui, &st.files.chips, iw), vault_h)
}

/// Stick `s` of place `p` in box `r` (`docs/DESIGN.md` §4.17): headed
/// "Stick n" with the × that takes it off; inside it the files box, the
/// files kept on the stick outside any vault, on every stick (§19.17),
/// and under it the vault box when the stick carries a vault (its band,
/// what it holds inside).
fn stick(ui: &mut Ui, fr: &mut Frame, st: &crate::board::StickView, r: Box4, custom: bool) {
    let (x, y, w, h) = r;
    let sp = tokens::BOARD_STICK_PAD;
    let vp = tokens::BOARD_VAULT_PAD;
    let files: &[ChipView] = &st.files.chips;
    let iw = w - 2.0 * sp;
    let (files_h, vault_h) = stick_parts(ui, st, w);
    let fz = st.files.zone;
    let hh = stick_box(ui, fr.stick_glyph, &st.label, r);
    // The stick box outside its files box and vault box still keeps a
    // drop as a file (§19.17 item 2).
    ui.hit_quiet(x, y, w, h, Action::Board(BoardPress::Zone(fz)));
    // Its × (after the lock, only for a stick that carries a vault alone:
    // its files stay as made, §17.8); none while the board is read-only.
    if custom && !(fr.locked && !files.is_empty()) {
        let mark = tokens::BOARD_MARK;
        cross(
            ui,
            x + w - sp - mark / 2.0 - tokens::GAP_SMALL,
            y + hh / 2.0,
            STICK_TEXT,
        );
        let side = mark + 2.0 * tokens::BOARD_PAD;
        ui.hit(x + w - side, y, side, hh, Action::Board(st.remove));
    }
    let fr_box = (x + sp, y + hh, iw, files_h);
    let vr = (x + sp, y + hh + files_h + sp, iw, vault_h);
    // The files box is the edged target while the pointer is on the stick
    // outside its vault.
    let in_vault = st.vault.is_some() && fr.drag.is_some_and(|d| inside(d, vr));
    let hot = if in_vault { (0.0, 0.0, 0.0, 0.0) } else { r };
    paper(
        ui,
        fr,
        fz,
        EN.board_files_not_sealed,
        files,
        fr_box,
        FILES,
        hot,
    );
    if let Some((vc, vz)) = &st.vault {
        let (vx, vy, vw, _) = vr;
        ui.fill(vx, vy, vw, vault_h, tokens::GAP, VAULT_FILL);
        ui.hit_quiet(
            vx,
            vy,
            vw,
            vault_h,
            Action::Board(BoardPress::Zone(vz.zone)),
        );
        vault_head(ui, fr, vc, (&st.sealed, &st.sealed_short), vx, vy, vw);
        let iy = vy + tokens::BOARD_VAULT_HEAD + vp;
        let (places, _) = flow(ui, &vz.chips, vx + vp, iy, vw - 2.0 * vp);
        for (c, at) in vz.chips.iter().zip(places) {
            chip(ui, fr, c, at.0, at.1, at.2, true);
        }
        outline(ui, vr, tokens::GAP, tokens::BOARD_VAULT_STROKE, VAULT_EDGE);
        if fr.edged(vz.zone, vr) {
            edge(ui, vr, tokens::GAP, ACCENT);
        }
    }
}

/// The software place at (x, y), `w` wide: its header band with its title
/// and ×, its stick of public files and **Scanned in**, laid as a place's
/// row is. Returns its height.
fn software(ui: &mut Ui, fr: &mut Frame, v: &BoardView, x: f32, y: f32, w: f32) -> f32 {
    let Some(sv) = &v.software else {
        return 0.0;
    };
    let pad = tokens::BOARD_PAD;
    let e = tokens::BOARD_BOX_EDGE;
    let stripe = tokens::BOARD_PLACE_STRIPE;
    let (ix, iw) = (x + stripe + pad, w - stripe - e - 2.0 * pad);
    let top = y + e + tokens::BOARD_HEAD + pad;
    let chips = sv.stick.as_ref().map_or(&[][..], |z| &z.chips[..]);
    let stick_files_h = |ui: &Ui, sw: f32| -> f32 {
        let sp = tokens::BOARD_STICK_PAD;
        tokens::BOARD_STICK_HEAD + paper_h(ui, chips, sw - 2.0 * sp) + sp
    };
    let lay = {
        let ui_ref: &Ui = ui;
        row(
            &|_, sw| stick_files_h(ui_ref, sw),
            &|pw| paper_h(ui_ref, &sv.scanned.chips, pw),
            1,
            false,
            ui_ref.compact,
            ix,
            top,
            iw,
        )
    };
    let h = top + lay.h + pad + e - y;
    let (bx, bw) = place_box(ui, x, y, w, h);
    let off = (v.custom && !v.locked).then_some(Action::Board(BoardPress::RemoveSoftware));
    place_head(ui, &sv.title, &[], bx, y + e, bw, None, off);
    // Its stick of public files: its files in a files box, no vault box
    // (§19.17 item 3).
    if let Some(&r) = lay.sticks.first() {
        let hh = stick_box(ui, fr.stick_glyph, &sv.stick_label, r);
        let z = Zone::SoftStick;
        ui.hit_quiet(r.0, r.1, r.2, r.3, Action::Board(BoardPress::Zone(z)));
        let sp = tokens::BOARD_STICK_PAD;
        let fh = paper_h(ui, chips, r.2 - 2.0 * sp);
        let files_box = (r.0 + sp, r.1 + hh, r.2 - 2.0 * sp, fh);
        paper(
            ui,
            fr,
            z,
            EN.board_files_not_sealed,
            chips,
            files_box,
            FILES,
            r,
        );
    }
    paper(
        ui,
        fr,
        sv.scanned.zone,
        EN.board_scanned_zone,
        &sv.scanned.chips,
        lay.paper,
        PAPER,
        lay.paper,
    );
    h
}

/// The **+ place** row at (x, y), `w` wide: a place more.
fn add_place(ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let a = Action::Board(BoardPress::AddPlace);
    let h = tokens::BOARD_HEAD;
    if ui.is_pressed(a) || ui.is_hovered(a) {
        ui.fill(x, y, w, h, tokens::GAP, INNER);
    }
    dashed(ui, (x, y, w, h), tokens::GAP, MUTED);
    let lw = ui.measure(tokens::CAPTION, W::S, EN.board_add_place);
    ui.text_mid(
        x + (w - lw) / 2.0,
        y,
        h,
        tokens::CAPTION,
        W::S,
        MUTED,
        EN.board_add_place,
    );
    ui.hit_quiet(x, y, w, h, a);
    h
}

/// One entry of the list at (x, y), `w` wide: its chip and its count at
/// the right. Returns its height.
fn entry(
    ui: &mut Ui,
    fr: &mut Frame,
    e: &Entry,
    x: f32,
    y: f32,
    w: f32,
    off: Option<Action>,
) -> f32 {
    let count_w = if e.count.is_empty() {
        0.0
    } else {
        ui.measure(tokens::CAPTION, W::R, &e.count) + tokens::BOARD_CHIP_GAP
    };
    let off_w = off.map_or(0.0, |_| tokens::BOARD_MARK + 2.0 * tokens::BOARD_PAD);
    let cv = ChipView {
        chip: e.chip,
        name: e.name.clone(),
        marks: String::new(),
        forms: Vec::new(),
        warn: false,
        link: false,
        fixed: e.dim,
        faint: e.faint,
        off: false,
        menu: e.menu,
    };
    // The count beside the chip, or under it where the chip's name would
    // drop a size beside it.
    let beside = count_w == 0.0 || chip_size(ui, &cv, w - count_w - off_w).size == tokens::LABEL;
    let avail = if beside {
        w - count_w - off_w
    } else {
        w - off_w
    };
    let fit = chip_size(ui, &cv, avail);
    let ch = fit.h;
    chip(ui, fr, &cv, x, y, fit, !e.dim);
    let count_color = if e.dim { DIM } else { MUTED };
    let mut under = 0.0;
    if !e.count.is_empty() && beside {
        ui.text_right(
            x + w - off_w,
            y,
            ch,
            tokens::CAPTION,
            W::R,
            count_color,
            &e.count,
        );
    } else if !e.count.is_empty() {
        ui.text(
            x + tokens::BOARD_PAD,
            y + ch + tokens::BOARD_CHIP_GAP,
            tokens::CAPTION,
            W::R,
            count_color,
            &e.count,
        );
        under = tokens::BOARD_CHIP_GAP + ui.line(tokens::CAPTION, W::R);
    }
    if let Some(a) = off {
        cross(ui, x + w - off_w / 2.0, y + tokens::BOARD_CHIP / 2.0, MUTED);
        ui.hit(x + w - off_w, y, off_w, ch, a);
    }
    let mut cy = y + ch + under;
    if let Some(line) = &e.holds {
        cy += tokens::BOARD_CHIP_GAP;
        cy += ui.wrap(x, cy, w, tokens::CAPTION, W::R, WARN, line) + tokens::BOARD_CHIP_GAP;
        if let Thing::Vault(v) = e.chip.thing() {
            cy += faraday_ui::parts::wrap_buttons(
                ui,
                x,
                cy,
                w,
                tokens::BOARD_CHIP,
                &[(
                    EN.board_take_off,
                    Style::Secondary,
                    Action::Board(BoardPress::RemoveVaultSure(v)),
                )],
            );
        }
    }
    cy - y
}

/// A group's label in the list. Returns its height.
fn group(ui: &mut Ui, label: &str, x: f32, y: f32) -> f32 {
    ui.text_mid(
        x,
        y,
        tokens::BOARD_ZONE_LABEL,
        tokens::CAPTION,
        W::S,
        MUTED,
        label,
    );
    tokens::BOARD_ZONE_LABEL
}

/// The list: the seeds, the passphrases, the description with **Add
/// partial descriptions to this backup** under it, the vaults with
/// **+ vault**, and the watch-only software. Returns its height.
fn list(ui: &mut Ui, fr: &mut Frame, v: &BoardView, x: f32, y: f32, w: f32) -> f32 {
    let gap = tokens::BOARD_CHIP_GAP;
    let mut cy = y;
    let entries = |ui: &mut Ui, fr: &mut Frame, label: &str, es: &[Entry], cy: &mut f32| {
        if es.is_empty() {
            return;
        }
        *cy += group(ui, label, x, *cy);
        for e in es {
            *cy += entry(ui, fr, e, x, *cy, w, None) + gap;
        }
        *cy += gap;
    };
    entries(ui, fr, EN.board_seeds, &v.seeds, &mut cy);
    // A seed's codex32 strings, under the seeds where it has them.
    cy += codex32_groups(ui, fr, v, (x, cy, w), true);
    entries(ui, fr, EN.board_passphrases, &v.passes, &mut cy);
    cy += group(ui, EN.board_wallet_description, x, cy);
    if let Some(whole) = v.description.first() {
        cy += entry(ui, fr, whole, x, cy, w, None) + gap;
    }
    if v.splits {
        // One button hands a share to every place, or takes them all off
        // (`docs/BACKUP-FLOW.md` §17.19 item 1).
        let label = if v.partials {
            EN.board_partials_remove
        } else {
            EN.board_partials_add
        };
        cy += gap;
        cy += faraday_ui::parts::wrap_buttons(
            ui,
            x,
            cy,
            w,
            tokens::BOARD_CHIP,
            &[(label, Style::Secondary, Action::Board(BoardPress::Partials))],
        ) + gap;
        // The Learn page on shares, as the description card had it.
        let label = EN.plan_what_is_a_share;
        let bw = (ui.measure(tokens::CAPTION, W::S, label) + 2.0 * tokens::BOARD_PAD).min(w);
        ui.button(
            x,
            cy,
            Some(bw),
            tokens::BOARD_CHIP,
            label,
            Style::Ghost,
            Action::LearnShares,
        );
        cy += tokens::BOARD_CHIP + gap;
    }
    cy += gap;
    cy += group(ui, EN.board_vaults, x, cy);
    for e in &v.vaults {
        let off = match e.chip.thing() {
            Thing::Vault(n) => Some(Action::Board(BoardPress::RemoveVault(n))),
            _ => None,
        };
        cy += entry(ui, fr, e, x, cy, w, off) + gap;
    }
    cy += faraday_ui::parts::wrap_buttons(
        ui,
        x,
        cy,
        w,
        tokens::BOARD_CHIP,
        &[(
            EN.board_add_vault,
            Style::Secondary,
            Action::Board(BoardPress::AddVault),
        )],
    ) + gap;
    if let Some(e) = &v.software_entry {
        cy += gap;
        cy += entry(ui, fr, e, x, cy, w, None) + gap;
    }
    // A seed without strings: turned to codex32 strings, at the foot.
    cy += codex32_groups(ui, fr, v, (x, cy, w), false);
    cy - y
}

/// Each seed's codex32 group under the seeds (`docs/CODEX32.md` §9.5):
/// its head, a chip per string, and while the plan makes the set its
/// threshold and strings, **+ share**, and **Codex32** or **Take off
/// codex32**. Returns the height used.
fn codex32_groups(
    ui: &mut Ui,
    fr: &mut Frame,
    v: &BoardView,
    (x, y, w): (f32, f32, f32),
    on: bool,
) -> f32 {
    use crate::board::codex32::C32Board;
    let gap = tokens::BOARD_CHIP_GAP;
    let mut cy = y;
    for g in v.c32.iter().filter(|g| g.on == on) {
        let press = |p: C32Board| Action::Board(BoardPress::C32(g.seed, p));
        if !g.on {
            let label = fill(EN.c32_board_toggle, &[&g.head]);
            cy += faraday_ui::parts::wrap_buttons(
                ui,
                x,
                cy,
                w,
                tokens::BOARD_CHIP,
                &[(label.as_str(), Style::Secondary, press(C32Board::On))],
            ) + gap;
            continue;
        }
        cy += group(ui, &g.head, x, cy);
        for e in &g.strings {
            cy += entry(ui, fr, e, x, cy, w, None) + gap;
        }
        if let Some(line) = &g.line {
            cy += ui.wrap(x, cy, w, tokens::CAPTION, W::R, MUTED, line) + gap;
        }
        if let Some((k, n)) = g.kn {
            crate::screens::stepper(
                ui,
                x,
                cy,
                w,
                EN.codex32_threshold,
                usize::from(k),
                press(C32Board::K(-1)),
                press(C32Board::K(1)),
                true,
            );
            cy += tokens::STEPPER_ROW;
            crate::screens::stepper(
                ui,
                x,
                cy,
                w,
                EN.codex32_strings,
                usize::from(n),
                press(C32Board::N(-1)),
                press(C32Board::N(1)),
                true,
            );
            cy += tokens::STEPPER_ROW;
            cy += faraday_ui::parts::wrap_buttons(
                ui,
                x,
                cy,
                w,
                tokens::BOARD_CHIP,
                &[
                    (EN.c32_board_plus, Style::Secondary, press(C32Board::N(1))),
                    (EN.c32_board_off, Style::Ghost, press(C32Board::Off)),
                ],
            ) + gap;
        }
        cy += gap;
    }
    cy - y
}

/// What one frame of the board knows while it draws: the chip held, as
/// the board on screen holds it.
fn frame(app: &View, ui: &Ui, v: &BoardView) -> Option<Frame> {
    let st = app.board_state()?;
    let (f, ox, oy) = (ui.f.max(0.1), ui.ox, ui.oy);
    Some(Frame {
        shape: v.shape.clone(),
        held: st.held,
        drag: st
            .drag
            .map(|(px, py)| (px as f32 / f - ox, py as f32 / f - oy)),
        locked: v.locked,
        held_at: None,
        stick_glyph: app.medium.icon(),
        focus: st.focus,
    })
}

/// The list as a sheet, opened by **Add** in the narrow column
/// (`docs/BACKUP-FLOW.md` §17.12 item 3): a press on a chip closes it
/// with the chip selected.
pub(crate) fn list_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(v) = app.board_view() else {
        return;
    };
    let Some(mut fr) = frame(app, ui, &v) else {
        return;
    };
    let place = if ui.compact {
        (tokens::GAP, w - 2.0 * tokens::GAP)
    } else {
        let sw = (w - 2.0 * tokens::BOARD_GAP).min(tokens::BOARD_POPUP_WIDTH * 2.0);
        ((w - sw) / 2.0, sw)
    };
    let pad = tokens::GRID * 2.0;
    crate::compact::scroll_sheet(ui, place, h, tokens::GAP, pad, &mut |ui, x, y, iw| {
        let mut cy = y;
        cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Add, TEXT, EN.board_add);
        cy += list(ui, &mut fr, &v, x, cy, iw);
        cy += crate::compact::buttons(
            ui,
            x,
            cy,
            iw,
            &[(EN.board_list_close, Style::Secondary, Action::Cancel)],
        );
        cy - y
    });
}

/// The sheet over a drop that brought a place nearer a spend (§19.22),
/// with the warning mark, or let a place see the balance it could not
/// before (`docs/BACKUP-FLOW.md` §17.5): its title, the line naming what
/// there makes the description, **Keep it** and **Take it off**. It
/// reads what the drop worked out.
pub(crate) fn sees_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(sees) = app.board_state().and_then(|st| st.sees.clone()) else {
        return;
    };
    let place = if ui.compact {
        (tokens::GAP, w - 2.0 * tokens::GAP)
    } else {
        let sw = (w - 2.0 * tokens::BOARD_GAP).min(tokens::BOARD_POPUP_WIDTH * 2.0);
        ((w - sw) / 2.0, sw)
    };
    let pad = tokens::GRID * 2.0;
    crate::compact::scroll_sheet(ui, place, h, tokens::GAP, pad, &mut |ui, x, y, iw| {
        let mut cy = y;
        let (icon, tone) = if sees.spend {
            (Icon::Warning, WARN)
        } else {
            (Icon::Eye, TEXT)
        };
        cy += crate::compact::sheet_head(ui, x, cy, iw, icon, tone, &sees.title);
        cy += ui.wrap(x, cy, iw, tokens::LABEL, W::R, TEXT, &sees.line) + tokens::GAP;
        cy += crate::compact::buttons(
            ui,
            x,
            cy,
            iw,
            &[
                (
                    EN.board_take_it_off,
                    Style::Secondary,
                    Action::Board(BoardPress::SeesUndo),
                ),
                (
                    EN.board_keep_button,
                    Style::Primary,
                    Action::Board(BoardPress::SeesKeep),
                ),
            ],
        );
        cy - y
    });
}

/// The sheet after partial descriptions are added
/// (`docs/BACKUP-FLOW.md` §17.19 item 4): each place holds a part; where
/// the whole is still kept, **Take it off every place**; **Keep it**. It
/// reads what the press worked out.
pub(crate) fn partials_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some((line, off)) = app.board_state().and_then(|st| st.partials.clone()) else {
        return;
    };
    let place = if ui.compact {
        (tokens::GAP, w - 2.0 * tokens::GAP)
    } else {
        let sw = (w - 2.0 * tokens::BOARD_GAP).min(tokens::BOARD_POPUP_WIDTH * 2.0);
        ((w - sw) / 2.0, sw)
    };
    let pad = tokens::GRID * 2.0;
    crate::compact::scroll_sheet(ui, place, h, tokens::GAP, pad, &mut |ui, x, y, iw| {
        let mut cy = y;
        cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::File, TEXT, EN.board_partials_each);
        let rest = line
            .strip_prefix(EN.board_partials_each)
            .map_or(line.as_str(), str::trim);
        if !rest.is_empty() {
            cy += ui.wrap(x, cy, iw, tokens::LABEL, W::R, TEXT, rest) + tokens::GAP;
        }
        let keep = (
            EN.board_keep_button,
            Style::Primary,
            Action::Board(BoardPress::PartialsKeep),
        );
        let buttons: Vec<(&str, Style, Action)> = if off {
            vec![
                (
                    EN.board_take_it_off_every,
                    Style::Secondary,
                    Action::Board(BoardPress::PartialsWholeOff),
                ),
                keep,
            ]
        } else {
            vec![keep]
        };
        cy += crate::compact::buttons(ui, x, cy, iw, &buttons);
        cy - y
    });
}

/// The read-only board's two buttons, in the list's place (the narrow
/// column's Add): **Add watch-only software**, or **Take off watch-only
/// software** while the software place is there, then **Customize
/// backup** (`docs/BACKUP-FLOW.md` §17.21 item 3).
fn read_only_buttons(v: &crate::board::BoardView) -> Vec<(&'static str, Style, Action)> {
    let software = if v.software.is_some() {
        EN.board_take_off_watch
    } else {
        EN.board_add_watch_only
    };
    vec![
        (
            software,
            Style::Secondary,
            Action::Board(BoardPress::Software),
        ),
        (
            EN.board_customize,
            Style::Secondary,
            Action::Board(BoardPress::Customize),
        ),
    ]
}

/// The board on a plan card (`docs/BACKUP-FLOW.md` §17.15): the places
/// one under another, each a row the column's width, **+ place** after
/// them, the software place last, the plan's three checks and the check
/// lines under them; over them a placed chip's forms. On `wide` the list
/// is the right-hand column ([`list_panel`]); in the narrow column a
/// place's parts stand one under another, under **Add**, which opens the
/// list as a sheet (§19.12 item 3). Returns its height.
pub(crate) fn board(app: &View, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(v) = app.board_view() else {
        return 0.0;
    };
    let Some(mut fr) = frame(app, ui, &v) else {
        return 0.0;
    };
    let gap = tokens::BOARD_PLACE_GAP;
    let mut cy = y;
    if ui.compact {
        // Read-only until Customize backup, its two buttons in Add's place
        // (§19.6, §19.21 item 3).
        let buttons = if v.custom {
            vec![(
                EN.board_add,
                Style::Secondary,
                Action::Board(BoardPress::OpenList),
            )]
        } else {
            read_only_buttons(&v)
        };
        cy += faraday_ui::parts::wrap_buttons(ui, x, cy, w, tokens::BOARD_CHIP, &buttons)
            + tokens::BOARD_PAD;
    }
    for cv in &v.places {
        cy += place(ui, &mut fr, &v, cv, x, cy, w) + gap;
    }
    // A place may be added after the lock too (§17.8).
    // Not at one place per key (§19.19).
    if v.custom && v.room {
        cy += add_place(ui, x, cy, w) + gap;
    }
    if v.software.is_some() {
        cy += software(ui, &mut fr, &v, x, cy, w) + gap;
    }
    let mut bottom = cy - gap;
    // The plan's three checks, as the Map drew them, under the board
    // (§19.14; the owner's choice, 2026-10-16: the board's top is kept
    // free).
    bottom += tokens::GAP;
    let name = |p: usize| v.places.get(p).map_or_else(String::new, |c| c.name.clone());
    bottom += crate::glance::check_lines(ui, x, bottom, w, v.check, false, &name);
    // The checks that stand, a line each (§19.5).
    for line in &v.warnings {
        bottom += tokens::GAP;
        let side = tokens::BOARD_MARK;
        ui.icon(
            x,
            bottom + tokens::GAP_SMALL,
            side,
            Icon::Warning,
            side,
            WARN,
        );
        bottom += ui.wrap(
            x + side + tokens::GAP,
            bottom,
            w - side - tokens::GAP,
            tokens::LABEL,
            W::R,
            WARN,
            line,
        );
    }
    bottom - y + tokens::GAP
}

/// The chip `c` as the board draws it now: a placed chip in its zone, or
/// a list chip as the list draws it.
fn view_of(v: &BoardView, c: Chip) -> Option<ChipView> {
    let mut zones: Vec<&crate::board::ZoneView> = Vec::new();
    let mut heads: Vec<&ChipView> = Vec::new();
    for card in &v.places {
        zones.push(&card.paper);
        for st in &card.sticks {
            zones.push(&st.files);
            if let Some((head, z)) = &st.vault {
                heads.push(head);
                zones.push(z);
            }
        }
    }
    if let Some(sv) = &v.software {
        zones.push(&sv.scanned);
        if let Some(z) = &sv.stick {
            zones.push(z);
        }
    }
    let placed = zones
        .into_iter()
        .flat_map(|z| z.chips.iter())
        .chain(heads)
        .find(|cv| cv.chip == c)
        .cloned();
    placed.or_else(|| {
        let e = v
            .seeds
            .iter()
            .chain(&v.passes)
            .chain(&v.description)
            .chain(&v.vaults)
            .chain(v.software_entry.iter())
            .find(|e| e.chip == c)?;
        Some(ChipView {
            chip: e.chip,
            name: e.name.clone(),
            marks: String::new(),
            forms: Vec::new(),
            warn: false,
            link: false,
            fixed: false,
            faint: e.faint,
            off: false,
            menu: e.menu,
        })
    })
}

/// The board's right-hand column on `wide`, where Back up's map panel is
/// when the board is not shown (`docs/BACKUP-FLOW.md` §17.14, the
/// owner's layout): the list, scrolling on its own; a placed chip dragged onto it is taken off. Over the whole
/// screen, last, the chip being dragged.
pub(crate) fn list_panel(app: &View, ui: &mut Ui, px: f32, pw: f32, h: f32) {
    let Some(v) = app.board_view() else {
        return;
    };
    let Some(mut fr) = frame(app, ui, &v) else {
        return;
    };
    ui.fill(px, 0.0, pw, h, 0.0, SURFACE);
    ui.fill(px, 0.0, 1.0, h, 0.0, LINE);
    let pad = tokens::BOARD_PAD;
    let (x, w) = (px + tokens::GRID * 2.0, pw - tokens::GRID * 4.0);
    // Clear of the network pill and the ? at the window's top right.
    let top = tokens::SIDE_PANEL_TOP;
    let view = (h - top).max(0.0);
    let draw = |ui: &mut Ui, fr: &mut Frame, y: f32| -> f32 {
        let mut cy = y + tokens::GRID;
        // Read-only until Customize backup: the button in the list's
        // place (§19.6).
        if v.custom {
            cy += list(ui, fr, &v, x, cy, w);
        } else {
            cy += faraday_ui::parts::wrap_buttons(
                ui,
                x,
                cy,
                w,
                tokens::BOARD_CHIP,
                &read_only_buttons(&v),
            );
        }
        cy + pad - y
    };
    let content = measure(ui, &mut |ui| draw(ui, &mut fr, 0.0));
    let shift = app.flows.board_list_offset().min((content - view).max(0.0));
    let clip = ui.rect(px, top, pw, view);
    ui.hit_quiet(
        px,
        top,
        pw,
        view,
        Action::Board(BoardPress::Zone(Zone::List)),
    );
    let taken = fr.held.is_some_and(|c| matches!(c, Chip::Placed(..)));
    ui.c.push_clip(clip);
    let mark = ui.hits.len();
    draw(ui, &mut fr, top - shift);
    // What is scrolled out of the column takes no press.
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
    if taken && fr.drag.is_none_or(|d| inside(d, (px, top, pw, view))) {
        edge(
            ui,
            (px + pad, top, pw - 2.0 * pad, view - pad),
            tokens::GAP,
            ACCENT,
        );
    }
    ui.report_scroll_beside(
        crate::ui::Slot::BoardList,
        clip,
        content - view,
        app.flows.board_list_offset(),
    );
    // The chip being dragged, under the finger.
    if let (Some((dx, dy)), Some(c)) = (fr.drag, fr.held)
        && let Some(cv) = view_of(&v, c)
    {
        let fit = chip_size(ui, &cv, tokens::BOARD_CARD_MAX);
        let (gx, gy) = (dx - fit.w / 2.0, dy - fit.h / 2.0);
        ui.shadow(
            gx,
            gy,
            fit.w,
            fit.h,
            tokens::BOARD_CHIP / 2.0,
            tokens::ELEVATION_RAISED,
        );
        let mut ghost = Frame { held: None, ..fr };
        chip(ui, &mut ghost, &cv, gx, gy, fit, false);
        edge(ui, (gx, gy, fit.w, fit.h), tokens::BOARD_CHIP / 2.0, ACCENT);
    }
}

/// A chip's ⋮ menu (`docs/BACKUP-FLOW.md` §17.20 item 5), as every ⋮
/// menu is drawn ([`crate::menu::draw`]): Add to… under the add glyph, a
/// placed chip's menu under the chip's own glyph. It reads what the press
/// that opened it worked out.
pub(crate) fn menu_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(menu) = app.board_state().and_then(|st| st.menu.clone()) else {
        return;
    };
    let lines: Vec<crate::menu::Line> = menu
        .rows
        .iter()
        .map(|r| match r {
            MenuRow::Head(name) => crate::menu::Line::Head(name.clone()),
            MenuRow::Item(label, p) => crate::menu::Line::Item(label.clone(), Action::Board(*p)),
        })
        .collect();
    let icon = match menu.chip {
        Chip::List(_) => Icon::Add,
        Chip::Placed(_, t) => glyph(t).unwrap_or(Icon::File),
    };
    crate::menu::draw(
        ui,
        w,
        h,
        &crate::menu::View {
            title: &menu.title,
            icon,
            lines: &lines,
            at: menu.at,
            keys: menu.keys,
            anchor: menu.anchor,
            cancel: (EN.board_menu_cancel, Action::Board(BoardPress::MenuClose)),
        },
    );
}
