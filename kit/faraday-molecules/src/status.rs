//! The status molecules: the status chip (04 §12.1) and the busy line (09 §3.2).

use faraday_atoms::hits::Shape;
use faraday_atoms::{Glyph, Offered, Pressed, Shown, Ui, W};
use faraday_text::Text;
use faraday_tokens::dims::{
    GAP, GAP_SMALL, GRID, ICON_SMALL, LABEL_L, NAV_ICON, PROGRESS_H, STATUS_CHIP_H,
    STATUS_CHIP_PAD, TITLE_M,
};
use faraday_tokens::palette::Role;

/// A status chip: one of the sidebar's rows where there is no expanded rail, at the top of Home,
/// and a flow in progress at the top of every destination. Material's assist chip. What a press
/// on it does is the `Offered` given to [`draw`] (the model has no press: 09 §3.4).
pub struct StatusModel<'s> {
    /// Its icon, when it has one.
    pub icon: Option<Glyph>,
    /// Its head.
    pub head: Shown<'s>,
    /// The head in the medium face rather than the regular one.
    pub bold: bool,
    /// What follows the head, muted; empty for none.
    pub tail: Shown<'s>,
}

/// The line that says what the work that runs is, with the progress indicator under it.
pub struct BusyModel<'s> {
    pub line: Shown<'s>,
    /// How much is done, 0 to 1, or none for work with no count.
    pub done: Option<f32>,
    /// How far the moving bar is through its pass; none with reduce motion.
    pub phase: Option<f32>,
}

/// Draws the chip at (x, y), at most `max` wide: its icon, its head and, muted after it, its
/// tail. A press anywhere in its touch target takes `offered`'s press. Its text is a one-line
/// slot: larger text fits down to Standard (`docs/TEXT-SIZE.md` §3). Returns its width.
pub fn draw<A: Pressed>(
    m: &StatusModel<'_>,
    ui: &mut Ui<'_, A>,
    (x, y, max): (f32, f32, f32),
    offered: &Offered<A>,
) -> f32 {
    let face = if m.bold { W::S } else { W::R };
    let (head, tail) = (m.head.as_str(), m.tail.as_str());
    let lead = if m.icon.is_some() {
        ICON_SMALL + GAP_SMALL
    } else {
        0.0
    };
    let inner = max - 2.0 * STATUS_CHIP_PAD - lead;
    // Material's chip label: label large, in the face the head asks for.
    let size = LABEL_L.size;
    let mut wide = ui.measure(size, face, head);
    if !tail.is_empty() {
        wide += GRID + ui.measure(size, W::R, tail);
    }
    let k = ui.fit_factor_of(wide, inner);
    ui.showing(&m.head, |ui| {
        ui.showing(&m.tail, |ui| {
            ui.fixed_text(|ui| {
                let at = ((x, y, max), (size * k, lead, inner), face);
                chip_drawn(m, ui, at, offered)
            })
        })
    })
}

/// [`draw`] with its text at `size`, the lead before it and the room inside it worked out.
fn chip_drawn<A: Pressed>(
    m: &StatusModel<'_>,
    ui: &mut Ui<'_, A>,
    ((x, y, max), (size, lead, inner), face): ((f32, f32, f32), (f32, f32, f32), W),
    offered: &Offered<A>,
) -> f32 {
    let (head, tail) = (m.head.as_str(), m.tail.as_str());
    let hw_full = ui.measure(size, face, head);
    let head_s = ui.fit(size, face, head, inner.min(hw_full));
    let hw = ui.measure(size, face, &head_s);
    let tail_s = if tail.is_empty() {
        Text::new()
    } else {
        ui.fit(size, W::R, tail, (inner - hw - GRID).max(0.0))
    };
    let tw = if tail_s.is_empty() {
        0.0
    } else {
        ui.measure(size, W::R, &tail_s) + GRID
    };
    let w = (2.0 * STATUS_CHIP_PAD + lead + hw + tw).min(max);
    let r = GRID;
    ui.offered(offered, |ui, press, _| {
        if let Some(a) = press {
            // Ringed round the chip as drawn, its press taken a touch target tall.
            let touch = ui.touch;
            let ty = y - (touch - STATUS_CHIP_H) / 2.0;
            let shape = Shape::new(x, y, w, STATUS_CHIP_H, r);
            ui.press_to(shape, (x, ty, w, touch), Role::OnSurface.paint(), a);
        }
    });
    ui.stroke(x, y, w, STATUS_CHIP_H, r, Role::Outline);
    let mut tx = x + STATUS_CHIP_PAD;
    if let Some(i) = m.icon {
        let iy = y + (STATUS_CHIP_H - ICON_SMALL) / 2.0;
        ui.icon(
            tx,
            iy,
            ICON_SMALL,
            i,
            NAV_ICON - GAP_SMALL,
            Role::OnSurfaceVariant,
        );
        tx += lead;
    }
    ui.text_mid(tx, y, STATUS_CHIP_H, size, face, Role::OnSurface, &head_s);
    if !tail_s.is_empty() {
        let at = tx + hw + GRID;
        ui.text_mid(
            at,
            y,
            STATUS_CHIP_H,
            size,
            W::R,
            Role::OnSurfaceVariant,
            &tail_s,
        );
    }
    w
}

/// The height of the busy line: the line the work is, a gap and the progress indicator.
pub fn busy_height<A: Pressed>(ui: &Ui<'_, A>) -> f32 {
    ui.line_as(TITLE_M) + GAP + PROGRESS_H
}

/// Draws the busy line from (x, y), `w` wide: the line in the accent, and under it the progress
/// indicator, filled by the share done where the work has a count, a bar crossing its track
/// where it has none, and still under reduce motion. Returns its height.
pub fn draw_busy<A: Pressed>(
    m: &BusyModel<'_>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let h = busy_height(ui);
    let line = ui.fit_as(TITLE_M, m.line.as_str(), w);
    ui.showing(&m.line, |ui| {
        ui.text_mid_as((x, y, h - PROGRESS_H), TITLE_M, Role::Primary, &line);
    });
    ui.progress((x, y + h - PROGRESS_H, w), m.done, m.phase);
    h
}
