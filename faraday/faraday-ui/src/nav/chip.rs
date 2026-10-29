//! A status chip (`docs/LAYOUT.md` §3, §4): one of the sidebar's rows
//! where there is no expanded rail, at the top of Home, and a flow in
//! progress at the top of every destination. Material's assist chip.

use crate::pal::*;
use crate::{Glyph, Shape, Ui, W};
use faraday_draw::tokens::{
    GAP_SMALL, GRID, ICON_SMALL, LABEL_L, NAV_ICON, STATUS_CHIP_H, STATUS_CHIP_PAD, TOUCH,
};

/// What a status chip shows and does.
#[derive(Debug, Clone, Copy)]
pub struct Chip<'s, A> {
    /// Its icon, when it has one.
    pub icon: Option<Glyph>,
    /// Its head and the face it is set in.
    pub head: (&'s str, W),
    /// What follows the head, muted; empty for none.
    pub tail: &'s str,
    /// What a press on it does; none for a chip that only shows.
    pub action: Option<A>,
}

/// Chip `c` at (x, y), at most `max` wide: its icon, its head and, muted
/// after it, its tail. A press anywhere in its touch target takes its
/// action. Its text is a one-line slot: larger text fits down to
/// Standard (`docs/TEXT-SIZE.md` §3). Returns its width.
pub fn status_chip<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, max): (f32, f32, f32),
    c: &Chip<A>,
) -> f32 {
    let lead = if c.icon.is_some() {
        ICON_SMALL + GAP_SMALL
    } else {
        0.0
    };
    let inner = max - 2.0 * STATUS_CHIP_PAD - lead;
    // Material's chip label: label large, in the face the head asks for.
    let size = LABEL_L.size;
    let mut wide = ui.measure(size, c.head.1, c.head.0);
    if !c.tail.is_empty() {
        wide += GRID + ui.measure(size, W::R, c.tail);
    }
    let k = ui.fit_factor_of(wide, inner);
    ui.fixed_text(|ui| chip_drawn(ui, (x, y, max), c, (size * k, lead, inner)))
}

/// [`status_chip`] with its text at `size`, the lead before it and the
/// room inside it worked out.
fn chip_drawn<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, max): (f32, f32, f32),
    c: &Chip<A>,
    (size, lead, inner): (f32, f32, f32),
) -> f32 {
    let Chip {
        icon,
        head,
        tail,
        action,
    } = *c;
    let hw_full = ui.measure(size, head.1, head.0);
    let head_s = ui.fit(size, head.1, head.0, inner.min(hw_full));
    let hw = ui.measure(size, head.1, &head_s);
    let tail_s = if tail.is_empty() {
        String::new()
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
    if let Some(a) = action {
        // Ringed round the chip as drawn, its press taken a touch target
        // tall.
        let ty = y - (TOUCH - STATUS_CHIP_H) / 2.0;
        let shape = Shape::new(x, y, w, STATUS_CHIP_H, r);
        ui.press_to(shape, (x, ty, w, TOUCH), Role::OnSurface.paint(), a);
    }
    ui.stroke(x, y, w, STATUS_CHIP_H, r, Role::Outline);
    let mut tx = x + STATUS_CHIP_PAD;
    if let Some(i) = icon {
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
    ui.text_mid(tx, y, STATUS_CHIP_H, size, head.1, Role::OnSurface, &head_s);
    if !tail_s.is_empty() {
        ui.text_mid(
            tx + hw + GRID,
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
