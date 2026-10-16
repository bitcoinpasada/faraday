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
    /// The held list chip, as the list draws it.
    held_view: Option<ChipView>,
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

/// What a chip's line needs beside its name and its marks: its glyph,
/// its link, warning and × marks, each a mark and a small gap.
fn marks_w(cv: &ChipView) -> f32 {
    let one = tokens::BOARD_MARK + tokens::BOARD_CHIP_GAP;
    let t = cv.chip.thing();
    let n = usize::from(glyph(t).is_some())
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
    if cv.fixed {
        ui.stroke(x, y, w, h, r, INNER);
    } else {
        ui.fill(x, y, w, h, r, if hot { BORDER } else { INNER });
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
    let face = if mono(t) { W::M } else { W::R };
    let size = fit.size;
    // A fingerprint is never cut; anything else is fitted.
    let name = if mono(t) {
        cv.name.clone()
    } else {
        ui.fit(size, face, &cv.name, (right - cx).max(0.0))
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
    ui.fill(x, y, w, h, tokens::GAP, SURFACE);
    ui.stroke(x, y, w, h, tokens::GAP, INNER);
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
        ui.fill(x, y, w, h, tokens::RADIUS, BG);
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

/// Stick `s` of place `p`: its label, the vault on it (its chip, then
/// what the vault holds), then its files. Returns its height.
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
    let pad = tokens::BOARD_PAD;
    // The files part: the stick as a whole, under the vault part.
    let files: Vec<ChipView> = st.files.chips.clone();
    let iw = w - 2.0 * pad;
    // Laid out first, so the stick's box is drawn under what is on it.
    let vault_h = st.vault.as_ref().map_or(0.0, |(vc, vz)| {
        let (_, head_h) = flow(ui, std::slice::from_ref(vc), 0.0, 0.0, iw - 2.0 * pad);
        let (_, inner_h) = flow(ui, &vz.chips, 0.0, 0.0, iw - 2.0 * pad);
        pad + head_h + tokens::BOARD_CHIP_GAP + inner_h.max(tokens::BOARD_CHIP) + pad
    });
    let (_, files_h) = flow(ui, &files, 0.0, 0.0, iw);
    let files_h = if files.is_empty() && st.vault.is_some() {
        0.0
    } else {
        files_h.max(tokens::BOARD_CHIP)
    };
    let h = tokens::BOARD_ZONE_LABEL
        + vault_h
        + if vault_h > 0.0 && files_h > 0.0 {
            tokens::BOARD_CHIP_GAP
        } else {
            0.0
        }
        + files_h
        + pad;
    let fz = Zone::Files(p, s);
    ui.fill(x, y, w, h, tokens::GAP, SURFACE);
    ui.stroke(x, y, w, h, tokens::GAP, INNER);
    ui.hit_quiet(x, y, w, h, Action::Board(BoardPress::Zone(fz)));
    ui.text_mid(
        x + pad,
        y,
        tokens::BOARD_ZONE_LABEL,
        tokens::CAPTION,
        W::S,
        MUTED,
        &st.label,
    );
    let mut cy = y + tokens::BOARD_ZONE_LABEL;
    if let Some((vc, vz)) = &st.vault {
        let (vx, vw) = (x + pad, iw);
        ui.fill(vx, cy, vw, vault_h, tokens::GAP, BG);
        ui.stroke(vx, cy, vw, vault_h, tokens::GAP, ACCENT.with_alpha(90));
        ui.hit_quiet(
            vx,
            cy,
            vw,
            vault_h,
            Action::Board(BoardPress::Zone(vz.zone)),
        );
        let (hp, head_h) = flow(
            ui,
            std::slice::from_ref(vc),
            vx + pad,
            cy + pad,
            vw - 2.0 * pad,
        );
        if let Some(r) = hp.first() {
            chip(ui, fr, vc, r.0, r.1, r.2, true);
        }
        let iy = cy + pad + head_h + tokens::BOARD_CHIP_GAP;
        let (places, _) = flow(ui, &vz.chips, vx + pad, iy, vw - 2.0 * pad);
        for (c, r) in vz.chips.iter().zip(places) {
            chip(ui, fr, c, r.0, r.1, r.2, true);
        }
        if fr.edged(vz.zone, (vx, cy, vw, vault_h)) {
            edge(ui, (vx, cy, vw, vault_h), tokens::GAP, ACCENT);
        }
        cy += vault_h + tokens::BOARD_CHIP_GAP;
    }
    let (places, _) = flow(ui, &files, x + pad, cy, iw);
    for (c, r) in files.iter().zip(places) {
        chip(ui, fr, c, r.0, r.1, r.2, true);
    }
    let files_box = (x, y, w, h);
    if fr.edged(fz, files_box) {
        edge(ui, files_box, tokens::GAP, ACCENT);
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
        ui.fill(x, y, w, h, tokens::RADIUS, BG);
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
    ui.fill(x, y, w, h, tokens::RADIUS, if hot { INNER } else { BG });
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
    if fr.held == Some(e.chip) {
        fr.held_view = Some(cv);
    }
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
        let minus = Action::BOmit(omit.saturating_sub(1));
        let plus = Action::BOmit((omit + 1).min(most));
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

/// The board in Back up's plan (`docs/BACKUP-FLOW.md` §19.1): the place
/// cards in rows with the **+** card after them, the software place, the
/// list at the right (under the cards where they would not fit beside
/// it), the check lines under it all; over them a placed chip's forms
/// and the chip being dragged. Returns its height.
pub(crate) fn board(app: &Faraday, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    let Some(v) = app.board_view() else {
        return 0.0;
    };
    let Some(b) = app.backup.as_ref() else {
        return 0.0;
    };
    let (f, ox, oy) = (ui.f.max(0.1), ui.ox, ui.oy);
    let mut fr = Frame {
        shape: app.plan_shape(b.wallet),
        held: b.board.held,
        forms: b.board.forms,
        drag: b
            .board
            .drag
            .map(|(px, py)| (px as f32 / f - ox, py as f32 / f - oy)),
        locked: v.locked,
        held_at: None,
        held_view: None,
    };
    let gap = tokens::BOARD_GAP;
    let list_w = tokens::BOARD_LIST_WIDTH;
    let beside = w - list_w - gap >= tokens::BOARD_CARD_MIN;
    let cards_w = if beside { w - list_w - gap } else { w };
    let n = (((cards_w + gap) / (tokens::BOARD_CARD_MIN + gap)).floor() as usize).max(1);
    let card_w = ((cards_w - gap * (n - 1) as f32) / n as f32).min(tokens::BOARD_CARD_MAX);
    // The list's surface first, so a chip dropped beside its chips is
    // dropped on the list.
    let (lx, ly) = if beside {
        (x + cards_w + gap, y)
    } else {
        (x, y)
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
    let mut cy = y;
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
    let cards_h = cy - y - gap;
    // The list: its zone under its chips.
    let (lx, ly) = if beside {
        (lx, ly)
    } else {
        (x, y + cards_h + gap)
    };
    let lw = if beside {
        list_w
    } else {
        w.min(tokens::BOARD_CARD_MAX)
    };
    let pad = tokens::BOARD_PAD;
    let (ix, iy, iw) = (lx + pad, ly + pad, lw - 2.0 * pad);
    let list_h = measure(ui, &mut |ui| list(ui, &mut fr, &v, ix, iy, iw)) + 2.0 * pad;
    ui.hit_quiet(
        lx,
        ly,
        lw,
        list_h,
        Action::Board(BoardPress::Zone(Zone::List)),
    );
    let taken = fr.held.is_some_and(|c| matches!(c, Chip::Placed(..)));
    if taken && fr.drag.is_none_or(|d| inside(d, (lx, ly, lw, list_h))) {
        edge(ui, (lx, ly, lw, list_h), tokens::GAP, ACCENT);
    }
    list(ui, &mut fr, &v, ix, iy, iw);
    let mut bottom = if beside {
        y + cards_h.max(list_h)
    } else {
        ly + list_h
    };
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
        let pw = tokens::BOARD_POPUP_WIDTH;
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
    // The chip being dragged, under the finger.
    if let (Some((dx, dy)), Some(c)) = (fr.drag, fr.held) {
        let cv = match c {
            Chip::Placed(..) => fr.held_at.clone().map(|(_, cv)| cv),
            Chip::List(_) => fr.held_view.clone(),
        };
        if let Some(cv) = cv {
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
    bottom - y + tokens::GAP
}
