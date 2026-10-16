//! The backup board drawn (`docs/DESIGN.md` §4.17,
//! `docs/BACKUP-FLOW.md` §19): the place cards with their zones, the
//! **+** card, the software place, the list beside them, the check lines
//! under them, a placed chip's forms and the chip being dragged. It reads
//! the board as the frame's memo has it ([`Faraday::board_view`]) and the
//! chip held; it works nothing out but where things go.

use osk_ui::tokens;
use osk_ui::widgets::Icon;

use crate::board::{BoardPress, BoardView, CardView, Chip, ChipView, Entry, Thing, Zone, takes};
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::{Action, Faraday};

/// What one frame of the board knows while it draws.
struct Frame {
    /// The wallet's shape, for which zones take the chip held.
    shape: crate::plan::Shape,
    /// The chip held, by a tap or a drag.
    held: Option<Chip>,
    /// Its forms are open.
    forms: bool,
    /// Where the dragged chip is, in units, while it is dragged.
    drag: Option<(f32, f32)>,
    /// After the lock.
    locked: bool,
    /// Where the held chip was drawn, and what it reads.
    held_at: Option<((f32, f32, f32, f32), ChipView)>,
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
fn edge(ui: &mut Ui, r: (f32, f32, f32, f32), radius: f32, color: osk_ui::Color) {
    let rect = ui.rect(r.0, r.1, r.2, r.3);
    let c = ui.col(color);
    ui.c.stroke_rounded_rect(rect, radius * ui.f, tokens::BOARD_EDGE * ui.f, c);
}

/// The × mark, a cross centred on (cx, cy).
fn cross(ui: &mut Ui, cx: f32, cy: f32, color: osk_ui::Color) {
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
fn link(ui: &mut Ui, x: f32, cy: f32, color: osk_ui::Color) {
    let q = tokens::BOARD_MARK / 4.0;
    let rect_a = ui.rect(x, cy - q, 3.0 * q, 2.0 * q);
    let rect_b = ui.rect(x + q, cy - q, 3.0 * q, 2.0 * q);
    let c = ui.col(color);
    let (r, s) = (q * ui.f, tokens::BOARD_MARK_STROKE * ui.f);
    ui.c.stroke_rounded_rect(rect_a, r, s, c);
    ui.c.stroke_rounded_rect(rect_b, r, s, c);
}

/// A chip's glyph: the fingerprint before a seed's, the passphrase's
/// before its own, the vault's before a vault's.
fn glyph(t: Thing) -> Option<Icon> {
    match t {
        Thing::Seed(_) => Some(Icon::Fingerprint),
        Thing::Pass(_) => Some(Icon::Passphrase),
        Thing::Vault(_) => Some(Icon::Vault),
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
/// its link, warning, lock and × marks, each a mark and a small gap.
fn marks_w(cv: &ChipView) -> f32 {
    let one = tokens::BOARD_MARK + tokens::BOARD_CHIP_GAP;
    let t = cv.chip.thing();
    let n = usize::from(glyph(t).is_some())
        + usize::from(sealed(cv))
        + usize::from(cv.link)
        + usize::from(cv.warn)
        + usize::from(cv.off && matches!(cv.chip, Chip::Placed(..)) && !cv.fixed);
    n as f32 * one
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
    let marks = if cv.marks.is_empty() {
        0.0
    } else {
        tokens::BOARD_CHIP_GAP + ui.measure(tokens::CAPTION, W::R, &format!("· {}", cv.marks))
    };
    let one = tokens::BOARD_CHIP;
    let two = tokens::BOARD_CHIP + ui.line(tokens::CAPTION, W::R);
    for size in [tokens::LABEL, tokens::CAPTION] {
        let need = base + ui.measure(size, face, &cv.name);
        if need + marks <= avail {
            return Fit {
                w: need + marks,
                h: one,
                size,
            };
        }
        if need <= avail && !cv.marks.is_empty() {
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
        h: if cv.marks.is_empty() { one } else { two },
        size: tokens::CAPTION,
    }
}

/// Draws a chip at (x, y), laid out as [`chip_size`] gave it. `hits`:
/// it takes presses (not the copy that follows a drag).
fn chip(ui: &mut Ui, fr: &mut Frame, cv: &ChipView, x: f32, y: f32, fit: Fit, hits: bool) {
    let (w, h) = (fit.w, fit.h);
    let t = cv.chip.thing();
    let r = tokens::BOARD_CHIP / 2.0;
    let held = fr.held == Some(cv.chip);
    let lifted = held && fr.drag.is_some() && hits;
    let action = Action::Board(BoardPress::Chip(cv.chip));
    let hot = hits && !cv.fixed && (ui.is_pressed(action) || ui.is_hovered(action));
    // A chip is the card's surface edged in the border, so it stands on
    // every level's fill: paper, a stick box, a vault box, the list.
    if cv.fixed {
        ui.stroke(x, y, w, h, r, BORDER);
    } else {
        ui.fill(x, y, w, h, r, if hot { INNER } else { SURFACE });
        ui.stroke(x, y, w, h, r, BORDER);
    }
    let fg = if cv.fixed || lifted { DIM } else { TEXT };
    let mut cx = x + tokens::BOARD_PAD;
    let line_h = tokens::BOARD_CHIP;
    if let Some(g) = glyph(t) {
        ui.icon(
            cx,
            y + (line_h - tokens::BOARD_MARK) / 2.0,
            tokens::BOARD_MARK,
            g,
            tokens::BOARD_MARK,
            if cv.fixed { DIM } else { MUTED },
        );
        cx += tokens::BOARD_MARK + tokens::BOARD_CHIP_GAP;
    }
    // The marks at the right, from the edge in: ×, warning, link.
    let mut right = x + w - tokens::BOARD_PAD;
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
                x + w - side,
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
            ACCENT,
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
        // The chip's body; its × over it at the right.
        let body = if off {
            w - tokens::BOARD_MARK - 2.0 * tokens::BOARD_PAD
        } else {
            w
        };
        ui.hit_quiet(x, y, body, h, action);
        if held {
            fr.held_at = Some(((x, y, w, h), cv.clone()));
        }
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

/// A zone: its label, then its chips, in a box that takes presses (and
/// drops) as zone `z`. An empty zone is a chip's height. Returns its
/// height.
#[allow(clippy::too_many_arguments)]
fn zone(
    ui: &mut Ui,
    fr: &mut Frame,
    z: Zone,
    label: Option<&str>,
    chips: &[ChipView],
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let pad = tokens::BOARD_PAD;
    let top = if label.is_some() {
        tokens::BOARD_ZONE_LABEL
    } else {
        pad
    };
    let (places, ch) = flow(ui, chips, x + pad, y + top, w - 2.0 * pad);
    let h = top + ch.max(tokens::BOARD_CHIP) + pad;
    // Paper and the software's zones stand on the place box's own
    // surface, edged: a stick box is the filled one (§19.14).
    ui.fill(x, y, w, h, tokens::GAP, SURFACE);
    ui.stroke(x, y, w, h, tokens::GAP, BORDER);
    ui.hit_quiet(x, y, w, h, Action::Board(BoardPress::Zone(z)));
    if let Some(l) = label {
        ui.text_mid(
            x + pad,
            y,
            tokens::BOARD_ZONE_LABEL,
            tokens::CAPTION,
            W::S,
            MUTED,
            l,
        );
    }
    for (cv, r) in chips.iter().zip(places) {
        chip(ui, fr, cv, r.0, r.1, r.2, true);
    }
    if fr.edged(z, (x, y, w, h)) {
        edge(ui, (x, y, w, h), tokens::GAP, ACCENT);
    }
    h
}

/// The head of a card: its name (a field while names are typed), and its
/// × when it can go. Returns its height.
fn head(
    ui: &mut Ui,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    name_press: Option<(Action, bool)>,
    off: Option<Action>,
) -> f32 {
    let pad = tokens::BOARD_PAD;
    let hh = tokens::BOARD_HEAD;
    let room = w - 2.0 * pad - tokens::BOARD_MARK - pad;
    let shown = ui.fit(tokens::LABEL, W::S, name, room);
    let nw = ui.text_mid(x + pad, y, hh, tokens::LABEL, W::S, TEXT, &shown);
    if let Some((a, typing)) = name_press {
        if typing {
            ui.caret(x + pad + nw + 1.0, y + tokens::GAP, hh - 2.0 * tokens::GAP);
            ui.rule(x + pad, y + hh - tokens::GAP_SMALL, room, ACCENT);
        }
        ui.hit(x + pad, y, room, hh, a);
    }
    if let Some(a) = off {
        let cx = x + w - pad - tokens::BOARD_MARK / 2.0;
        cross(ui, cx, y + hh / 2.0, MUTED);
        let side = tokens::BOARD_MARK + 2.0 * pad;
        ui.hit(x + w - side, y, side, hh, a);
    }
    hh
}

/// A place's card at (x, y), `w` wide; with `h` its surface is drawn that
/// tall, without it nothing but its height is wanted. Returns its height.
#[allow(clippy::too_many_arguments)]
fn card(
    ui: &mut Ui,
    fr: &mut Frame,
    v: &BoardView,
    cv: &CardView,
    x: f32,
    y: f32,
    w: f32,
    h: Option<f32>,
) -> f32 {
    let pad = tokens::BOARD_PAD;
    if let Some(h) = h {
        ui.fill(x, y, w, h, tokens::RADIUS, SURFACE);
        ui.stroke(x, y, w, h, tokens::RADIUS, BORDER);
        ui.hit_quiet(
            x,
            y,
            w,
            h,
            Action::Board(BoardPress::Zone(Zone::Card(cv.p))),
        );
        // A vault's chip held goes to the card as a whole.
        if let Some(c) = fr.held
            && let Thing::Vault(_) = c.thing()
            && takes(&fr.shape, Zone::Card(cv.p), c.thing(), fr.locked)
            && !matches!(c, Chip::Placed(Zone::Files(p, _), _) if p == cv.p)
            && fr.drag.is_none_or(|d| inside(d, (x, y, w, h)))
        {
            edge(ui, (x, y, w, h), tokens::RADIUS, ACCENT);
        }
    }
    let p8 = u8::try_from(cv.p).unwrap_or(u8::MAX);
    let removable = v.places.len() > 1 && (!v.locked || cv.p + 1 == v.places.len());
    let name_press = v
        .names_typed
        .then_some((Action::BName(p8), v.naming == Some(cv.p)));
    let mut cy = y + head(
        ui,
        &cv.name,
        x,
        y,
        w,
        name_press,
        removable.then_some(Action::Board(BoardPress::RemovePlace(cv.p))),
    );
    let (ix, iw) = (x + pad, w - 2.0 * pad);
    for (s, st) in cv.sticks.iter().enumerate() {
        cy += stick(ui, fr, cv.p, s, st, ix, cy, iw) + pad;
    }
    // + stick: a new stick for what is dropped on it.
    let nh = tokens::BOARD_CHIP;
    let z = Zone::NewStick(cv.p);
    ui.stroke(ix, cy, iw, nh, tokens::GAP, INNER);
    ui.text_mid(
        ix + pad,
        cy,
        nh,
        tokens::CAPTION,
        W::S,
        MUTED,
        EN.board_add_stick,
    );
    ui.hit_quiet(ix, cy, iw, nh, Action::Board(BoardPress::Zone(z)));
    let new_stick = fr.held.is_some_and(|c| {
        takes(&fr.shape, z, c.thing(), fr.locked) && !matches!(c, Chip::List(Thing::Software))
    }) && fr.drag.is_none_or(|d| inside(d, (ix, cy, iw, nh)));
    if new_stick {
        edge(ui, (ix, cy, iw, nh), tokens::GAP, ACCENT);
    }
    cy += nh + pad;
    cy += zone(
        ui,
        fr,
        cv.paper.zone,
        Some(EN.board_paper),
        &cv.paper.chips,
        ix,
        cy,
        iw,
    ) + pad;
    // The place's line of fact: what finding it alone gives.
    cy += ui.wrap(
        ix,
        cy,
        iw,
        tokens::CAPTION,
        W::R,
        crate::glance::alone_color(cv.alone.1),
        &cv.alone.0,
    ) + pad;
    cy - y
}

/// A box's edge at its own stroke: a stick box's and a vault box's are
/// heavier than a zone's.
fn outline(ui: &mut Ui, r: (f32, f32, f32, f32), radius: f32, width: f32, color: osk_ui::Color) {
    let rect = ui.rect(r.0, r.1, r.2, r.3);
    let c = ui.col(color);
    ui.c.stroke_rounded_rect(rect, radius * ui.f, width * ui.f, c);
}

/// A vault box's head at (x, y), `w` wide: the vault's glyph and name,
/// its link and warning marks, and the × that takes its stick off this
/// place. The head is the vault's chip: a press selects it, a drag moves
/// its stick.
fn vault_head(ui: &mut Ui, fr: &mut Frame, vc: &ChipView, x: f32, y: f32, w: f32) {
    let hh = tokens::BOARD_VAULT_HEAD;
    let pad = tokens::BOARD_VAULT_PAD;
    let mark = tokens::BOARD_MARK;
    let action = Action::Board(BoardPress::Chip(vc.chip));
    let held = fr.held == Some(vc.chip);
    let lifted = held && fr.drag.is_some();
    if ui.is_pressed(action) || ui.is_hovered(action) {
        ui.fill(
            x,
            y,
            w,
            hh,
            tokens::GAP,
            ACCENT.with_alpha(tokens::BOARD_VAULT_FILL),
        );
    }
    ui.icon(
        x + pad,
        y + (hh - mark) / 2.0,
        mark,
        Icon::Vault,
        mark,
        ACCENT,
    );
    let mut right = x + w - pad;
    let mut off_w = 0.0;
    if vc.off
        && let Chip::Placed(z, t) = vc.chip
    {
        cross(ui, right - mark / 2.0, y + hh / 2.0, MUTED);
        off_w = mark + 2.0 * pad;
        ui.hit(
            x + w - off_w,
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
            WARN,
        );
        right -= mark + tokens::BOARD_CHIP_GAP;
    }
    if vc.link {
        link(ui, right - mark, y + hh / 2.0, ACCENT);
        right -= mark + tokens::BOARD_CHIP_GAP;
    }
    let nx = x + pad + mark + tokens::BOARD_CHIP_GAP;
    let name = ui.fit(tokens::LABEL, W::S, &vc.name, (right - nx).max(0.0));
    ui.text_mid(
        nx,
        y,
        hh,
        tokens::LABEL,
        W::S,
        if lifted { DIM } else { TEXT },
        &name,
    );
    if held && !lifted {
        edge(ui, (x, y, w, hh), tokens::GAP, ACCENT);
    }
    ui.hit_quiet(x, y, w - off_w, hh, action);
    if held {
        fr.held_at = Some(((x, y, w, hh), vc.clone()));
    }
}

/// Stick `s` of place `p` (`docs/DESIGN.md` §4.17): a box of its own in
/// the place card, headed "Stick n" with the × that takes it off; inside
/// it the vault box when the stick carries a vault (tinted, headed by the
/// vault, what it holds inside), and beside it, outside the vault box,
/// the files kept on the stick. Returns its height.
#[allow(clippy::too_many_arguments)]
fn stick(
    ui: &mut Ui,
    fr: &mut Frame,
    p: usize,
    s: usize,
    st: &crate::board::StickView,
    x: f32,
    y: f32,
    w: f32,
) -> f32 {
    let sp = tokens::BOARD_STICK_PAD;
    let vp = tokens::BOARD_VAULT_PAD;
    let files: &[ChipView] = &st.files.chips;
    let iw = w - 2.0 * sp;
    // Laid out first, so each box is drawn under what is in it.
    let vault_h = st.vault.as_ref().map_or(0.0, |(_, vz)| {
        let (_, inner_h) = flow(ui, &vz.chips, 0.0, 0.0, iw - 2.0 * vp);
        tokens::BOARD_VAULT_HEAD + inner_h.max(tokens::BOARD_CHIP) + vp
    });
    let (_, files_h) = flow(ui, files, 0.0, 0.0, iw);
    let files_h = if files.is_empty() && st.vault.is_some() {
        0.0
    } else {
        files_h.max(tokens::BOARD_CHIP)
    };
    let between = if vault_h > 0.0 && files_h > 0.0 {
        sp
    } else {
        0.0
    };
    let h = tokens::BOARD_STICK_HEAD + vault_h + between + files_h + sp;
    let fz = Zone::Files(p, s);
    ui.fill(x, y, w, h, tokens::GAP, INNER);
    outline(
        ui,
        (x, y, w, h),
        tokens::GAP,
        tokens::BOARD_STICK_EDGE,
        BORDER,
    );
    ui.hit_quiet(x, y, w, h, Action::Board(BoardPress::Zone(fz)));
    // The head: "Stick n", and its × (after the lock, only for a stick
    // that carries a vault alone: its files stay as made, §17.8).
    let hh = tokens::BOARD_STICK_HEAD;
    ui.text_mid(
        x + sp + tokens::GAP_SMALL,
        y,
        hh,
        tokens::CAPTION,
        W::S,
        TEXT,
        &st.label,
    );
    if !(fr.locked && !files.is_empty()) {
        let mark = tokens::BOARD_MARK;
        cross(
            ui,
            x + w - sp - mark / 2.0 - tokens::GAP_SMALL,
            y + hh / 2.0,
            MUTED,
        );
        let side = mark + 2.0 * tokens::BOARD_PAD;
        ui.hit(
            x + w - side,
            y,
            side,
            hh,
            Action::Board(BoardPress::RemoveStick(p, s)),
        );
    }
    let mut cy = y + hh;
    let mut vault_box = None;
    if let Some((vc, vz)) = &st.vault {
        let (vx, vw) = (x + sp, iw);
        let tint = ACCENT.with_alpha(tokens::BOARD_VAULT_FILL);
        ui.fill(vx, cy, vw, vault_h, tokens::GAP, tint);
        outline(
            ui,
            (vx, cy, vw, vault_h),
            tokens::GAP,
            tokens::BOARD_VAULT_STROKE,
            ACCENT,
        );
        ui.hit_quiet(
            vx,
            cy,
            vw,
            vault_h,
            Action::Board(BoardPress::Zone(vz.zone)),
        );
        vault_head(ui, fr, vc, vx, cy, vw);
        let iy = cy + tokens::BOARD_VAULT_HEAD;
        let (places, _) = flow(ui, &vz.chips, vx + vp, iy, vw - 2.0 * vp);
        for (c, r) in vz.chips.iter().zip(places) {
            chip(ui, fr, c, r.0, r.1, r.2, true);
        }
        let r = (vx, cy, vw, vault_h);
        if fr.edged(vz.zone, r) {
            edge(ui, r, tokens::GAP, ACCENT);
        }
        vault_box = Some(r);
        cy += vault_h + between;
    }
    let (places, _) = flow(ui, files, x + sp, cy, iw);
    for (c, r) in files.iter().zip(places) {
        chip(ui, fr, c, r.0, r.1, r.2, true);
    }
    // The stick takes a file where the pointer is on it outside its vault.
    let in_vault = matches!((fr.drag, vault_box), (Some(d), Some(r)) if inside(d, r));
    if !in_vault && fr.edged(fz, (x, y, w, h)) {
        edge(ui, (x, y, w, h), tokens::GAP, ACCENT);
    }
    h
}

/// The software place's card: its title and ×, **Scanned in**, and its
/// stick of public files. Returns its height.
fn software_card(
    ui: &mut Ui,
    fr: &mut Frame,
    v: &BoardView,
    x: f32,
    y: f32,
    w: f32,
    h: Option<f32>,
) -> f32 {
    let Some(sv) = &v.software else {
        return 0.0;
    };
    let pad = tokens::BOARD_PAD;
    if let Some(h) = h {
        ui.fill(x, y, w, h, tokens::RADIUS, SURFACE);
        ui.stroke(x, y, w, h, tokens::RADIUS, BORDER);
    }
    let off = (!v.locked).then_some(Action::Board(BoardPress::RemoveSoftware));
    let mut cy = y + head(ui, &sv.title, x, y, w, None, off);
    let (ix, iw) = (x + pad, w - 2.0 * pad);
    cy += zone(
        ui,
        fr,
        sv.scanned.zone,
        Some(EN.board_scanned_in),
        &sv.scanned.chips,
        ix,
        cy,
        iw,
    ) + pad;
    let label = opensigner_core::strings::fill(EN.board_stick, &["1"]);
    let chips = sv.stick.as_ref().map_or(&[][..], |z| &z.chips[..]);
    cy += zone(ui, fr, Zone::SoftStick, Some(&label), chips, ix, cy, iw) + pad;
    cy - y
}

/// The **+** card: a place more.
fn plus_card(ui: &mut Ui, x: f32, y: f32, w: f32, h: f32) {
    let a = Action::Board(BoardPress::AddPlace);
    let hot = ui.is_pressed(a) || ui.is_hovered(a);
    ui.fill(
        x,
        y,
        w,
        h,
        tokens::RADIUS,
        if hot { INNER } else { SURFACE },
    );
    ui.stroke(x, y, w, h, tokens::RADIUS, BORDER);
    let side = tokens::BOARD_HEAD;
    ui.icon(
        x + (w - side) / 2.0,
        y + (h - side) / 2.0,
        side,
        Icon::Add,
        tokens::LABEL,
        MUTED,
    );
    ui.hit_quiet(x, y, w, h, a);
}

/// One entry of the list at (x, y), `w` wide: its chip, its count at the
/// right, and under it the keys a share carries. Returns its height.
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
        off: false,
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
    if !e.keys.is_empty() {
        // The keys a share carries, by fingerprint, laid left to right.
        let line = ui.line(tokens::CAPTION, W::M);
        let (left, right) = (x + tokens::BOARD_PAD, x + w);
        let (mut kx, mut ky) = (left, cy + tokens::BOARD_CHIP_GAP);
        for k in &e.keys {
            let kw = ui.measure(tokens::CAPTION, W::M, k);
            if kx > left && kx + kw > right {
                kx = left;
                ky += line;
            }
            ui.text(kx, ky, tokens::CAPTION, W::M, MUTED, k);
            kx += kw + tokens::GAP;
        }
        cy = ky + line + tokens::BOARD_CHIP_GAP;
    }
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

/// The list: the seeds, the passphrases, the description and its
/// shares with the keys left off each, the vaults with **+ vault**, and
/// the watch-only software. Returns its height.
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
    entries(ui, fr, EN.board_passphrases, &v.passes, &mut cy);
    cy += group(ui, EN.board_wallet_description, x, cy);
    if let Some(whole) = v.description.first() {
        cy += entry(ui, fr, whole, x, cy, w, None) + gap;
    }
    if let Some((omit, most)) = v.omit {
        cy += gap;
        cy += ui.wrap(
            x,
            cy,
            w,
            tokens::CAPTION,
            W::R,
            MUTED,
            EN.board_keys_left_off,
        ) + gap;
        let side = tokens::BOARD_CHIP;
        // Back up's keys left off, or the flow's own (§17.7).
        let omit_to = |n: usize| match v.flow {
            None => Action::BOmit(n),
            Some(_) => Action::Flow(crate::backup_flow_steps::FlowAction::Plan(
                crate::PlanPress::Omit(u8::try_from(n).unwrap_or(u8::MAX)),
            )),
        };
        let minus = omit_to(omit.saturating_sub(1));
        let plus = omit_to((omit + 1).min(most));
        let style = |on: bool| {
            if on {
                Style::Secondary
            } else {
                Style::Disabled
            }
        };
        ui.button(x, cy, Some(side), side, "-", style(omit > 0), minus);
        let n = omit.to_string();
        let nw = ui.measure(tokens::LABEL, W::S, &n);
        ui.text_mid(
            x + side + (side - nw) / 2.0,
            cy,
            side,
            tokens::LABEL,
            W::S,
            TEXT,
            &n,
        );
        ui.button(
            x + 2.0 * side,
            cy,
            Some(side),
            side,
            "+",
            style(omit < most),
            plus,
        );
        cy += side + gap;
        for e in v.description.iter().skip(1) {
            cy += entry(ui, fr, e, x, cy, w, None) + gap;
        }
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
    cy - y
}

/// What `draw` would take, drawn nowhere and pressing nothing.
fn measure(ui: &mut Ui, draw: &mut dyn FnMut(&mut Ui) -> f32) -> f32 {
    let mark = ui.hits.len();
    ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
    let h = draw(ui);
    ui.c.pop_clip();
    ui.hits.truncate(mark);
    h
}

/// What one frame of the board knows while it draws: the chip held, as
/// the board on screen holds it.
fn frame(app: &Faraday, ui: &Ui, v: &BoardView) -> Option<Frame> {
    let st = app.board_state()?;
    let (f, ox, oy) = (ui.f.max(0.1), ui.ox, ui.oy);
    Some(Frame {
        shape: v.shape.clone(),
        held: st.held,
        forms: st.forms,
        drag: st
            .drag
            .map(|(px, py)| (px as f32 / f - ox, py as f32 / f - oy)),
        locked: v.locked,
        held_at: None,
    })
}

/// The list as a sheet, opened by **Add** in the narrow column
/// (`docs/BACKUP-FLOW.md` §19.12 item 3): a press on a chip closes it
/// with the chip selected.
pub(crate) fn list_sheet(app: &Faraday, ui: &mut Ui, w: f32, h: f32) {
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
            &[(EN.plan_continue, Style::Secondary, Action::Cancel)],
        );
        cy - y
    });
}

/// The board on a plan card (`docs/BACKUP-FLOW.md` §19.1): the place
/// cards in rows across the column with the **+** card after them, the
/// software place, the plan's three checks and the check lines under
/// them; over them a placed chip's
/// forms. On `wide` the list is the right-hand column ([`list_panel`]);
/// in the narrow column the cards stand one under another under **Add**,
/// which opens the list as a sheet (§19.12 item 3). Returns its height.
pub(crate) fn board(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(v) = app.board_view() else {
        return 0.0;
    };
    let Some(mut fr) = frame(app, ui, &v) else {
        return 0.0;
    };
    let gap = tokens::BOARD_GAP;
    let narrow = ui.compact;
    let mut cy = y;
    if narrow {
        cy += faraday_ui::parts::wrap_buttons(
            ui,
            x,
            cy,
            w,
            tokens::BOARD_CHIP,
            &[(
                EN.board_add,
                Style::Secondary,
                Action::Board(BoardPress::OpenList),
            )],
        ) + gap;
    }
    let n = if narrow {
        1
    } else {
        (((w + gap) / (tokens::BOARD_CARD_MIN + gap)).floor() as usize).max(1)
    };
    let card_w = if narrow {
        w
    } else {
        ((w - gap * (n - 1) as f32) / n as f32).min(tokens::BOARD_CARD_MAX)
    };
    // Each card in turn: a place, the software place, then +.
    #[derive(Clone, Copy)]
    enum Item {
        Place(usize),
        Software,
        Plus,
    }
    let mut items: Vec<Item> = (0..v.places.len()).map(Item::Place).collect();
    if v.software.is_some() {
        items.push(Item::Software);
    }
    // A place may be added after the lock too (§17.8).
    items.push(Item::Plus);
    let draw_item =
        |ui: &mut Ui, fr: &mut Frame, it: Item, cx: f32, cy: f32, h: Option<f32>| -> f32 {
            match it {
                Item::Place(p) => card(ui, fr, &v, &v.places[p], cx, cy, card_w, h),
                Item::Software => software_card(ui, fr, &v, cx, cy, card_w, h),
                Item::Plus => {
                    let ph = h.unwrap_or(tokens::BOARD_HEAD + 2.0 * tokens::BOARD_CHIP);
                    if h.is_some() {
                        plus_card(ui, cx, cy, card_w, ph);
                    }
                    ph
                }
            }
        };
    for row in items.chunks(n) {
        let row_h = row
            .iter()
            .map(|&it| measure(ui, &mut |ui| draw_item(ui, &mut fr, it, x, cy, None)))
            .fold(0.0f32, f32::max);
        for (k, &it) in row.iter().enumerate() {
            let cx = x + k as f32 * (card_w + gap);
            draw_item(ui, &mut fr, it, cx, cy, Some(row_h));
        }
        cy += row_h + gap;
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
    // The held placed chip's forms, under it.
    if fr.forms
        && fr.drag.is_none()
        && let Some(((hx, hy, _, hh), cv)) = fr.held_at.clone()
        && let Chip::Placed(z, t) = cv.chip
        && !cv.forms.is_empty()
    {
        let pad = tokens::BOARD_PAD;
        let pw = tokens::BOARD_POPUP_WIDTH.min(w);
        let rows: Vec<(String, bool, bool, Action)> = cv
            .forms
            .iter()
            .enumerate()
            .map(|(k, (label, on))| {
                (
                    label.clone(),
                    *on,
                    true,
                    Action::Board(BoardPress::Form(z, t, u8::try_from(k).unwrap_or(u8::MAX))),
                )
            })
            .collect();
        let rows_h = measure(ui, &mut |ui| ui.multi_list(0.0, 0.0, pw - 2.0 * pad, &rows));
        let px = hx.min(x + w - pw).max(x);
        let py = hy + hh + tokens::GAP_SMALL;
        let ph = rows_h + 2.0 * pad;
        ui.shadow(px, py, pw, ph, tokens::GAP);
        ui.fill(px, py, pw, ph, tokens::GAP, SURFACE);
        ui.stroke(px, py, pw, ph, tokens::GAP, BORDER);
        ui.hit_quiet(px, py, pw, ph, Action::Board(BoardPress::Popup));
        ui.multi_list(px + pad, py + pad, pw - 2.0 * pad, &rows);
        bottom = bottom.max(py + ph);
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
            off: false,
        })
    })
}

/// The board's right-hand column on `wide`, where Back up's map panel is
/// when the board is not shown (`docs/BACKUP-FLOW.md` §19.14, the
/// owner's layout): the list, scrolling on its own; a placed chip dragged onto it is taken off. Over the whole
/// screen, last, the chip being dragged.
pub(crate) fn list_panel(app: &Faraday, ui: &mut Ui, px: f32, pw: f32, h: f32) {
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
    let top = tokens::GRID * 6.0;
    let view = (h - top).max(0.0);
    let draw = |ui: &mut Ui, fr: &mut Frame, y: f32| -> f32 {
        let mut cy = y + tokens::GRID;
        cy += list(ui, fr, &v, x, cy, w);
        cy + pad - y
    };
    let content = measure(ui, &mut |ui| draw(ui, &mut fr, 0.0));
    let shift = app.board_list_offset.min((content - view).max(0.0));
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
        app.board_list_offset,
    );
    // The chip being dragged, under the finger.
    if let (Some((dx, dy)), Some(c)) = (fr.drag, fr.held)
        && let Some(cv) = view_of(&v, c)
    {
        let fit = chip_size(ui, &cv, tokens::BOARD_CARD_MAX);
        let (gx, gy) = (dx - fit.w / 2.0, dy - fit.h / 2.0);
        ui.shadow(
            gx,
            gy + tokens::BOARD_LIFT,
            fit.w,
            fit.h,
            tokens::BOARD_CHIP / 2.0,
        );
        let mut ghost = Frame { held: None, ..fr };
        chip(ui, &mut ghost, &cv, gx, gy, fit, false);
        edge(ui, (gx, gy, fit.w, fit.h), tokens::BOARD_CHIP / 2.0, ACCENT);
    }
}
