//! The small panel's step flow (`docs/SMALL-PANEL.md` on `main`): one page for the open step,
//! scrolled in the room above the pin bar, and the pin bar: the card's last block, where that is
//! a row of buttons, stands at the panel's foot instead of at the card's end, so the forward
//! action never scrolls away. The bar's height is known from the model, so the page is laid out
//! for it in the same frame (04 §12.4).

use faraday_atoms::button::ButtonModel;
use faraday_atoms::region::Region;
use faraday_atoms::{Pressed, Ui};
use faraday_molecules::buttons::draw_items;
use faraday_organisms::content::{Block, Content};
use faraday_tokens::dims::{COLUMN_MAX, FLOW_END_PAD, GAP, MARGIN_COMPACT};
use faraday_tokens::palette::Role;

use super::{FlowModel, card_scroll, column};

/// The pin bar's height: the buttons' row, a margin over and under it.
const PIN_H: f32 = 64.0;

/// The buttons the card keeps for the pin bar: its last block, where that is a row of buttons.
pub(crate) fn pinned<'a, 's, A>(card: &'a Content<'s, A>) -> Option<&'a [ButtonModel<'s, A>]> {
    match card.0.last() {
        Some(Block::Buttons(items)) if !items.is_empty() => Some(items),
        _ => None,
    }
}

/// The pin bar at the foot of a page `cw` by `h`: a hairline over a container, the buttons
/// sharing the width.
fn pin_bar<A: Pressed>(items: &[ButtonModel<'_, A>], ui: &mut Ui<'_, A>, (cw, h): (f32, f32)) {
    let y = h - PIN_H;
    ui.fill(0.0, y, cw, PIN_H, 0.0, Role::SurfaceContainerLow);
    ui.fill(0.0, y, cw, 1.0, 0.0, Role::OutlineVariant);
    draw_items(
        items,
        &[],
        false,
        ui,
        (MARGIN_COMPACT, y + 10.0, cw - 2.0 * MARGIN_COMPACT),
    );
}

/// The page of the open step in the room `cw` by `h`.
pub(crate) fn page<A: Pressed>(m: &FlowModel<'_, A>, ui: &mut Ui<'_, A>, (cw, h): (f32, f32)) {
    let pin = pinned(&m.card);
    let view_h = h - pin.map_or(0.0, |_| PIN_H);
    let (x, w) = (MARGIN_COMPACT, (cw - 2.0 * MARGIN_COMPACT).min(COLUMN_MAX));
    let clip = ui.rect(0.0, 0.0, cw, view_h);
    ui.anchor(m.step);
    let open = ui.region_open(Region::new(card_scroll(m.step), clip));
    let used = column(m, ui, (x, GAP, w), (pin.is_some(), true));
    ui.region_close(open, GAP + used + FLOW_END_PAD - view_h);
    if let Some(items) = pin {
        pin_bar(items, ui, (cw, h));
    }
}
