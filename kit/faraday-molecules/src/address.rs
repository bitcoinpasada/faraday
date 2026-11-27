//! The address molecule: an address whole and wrapped, in the mono face, under or beside its
//! label, and the row that opens its code (`docs/QR.md` §7.1; 09 §3.2).

use faraday_atoms::{Offered, Pressed, Shown, Ui};
use faraday_tokens::dims::{BODY_M, BODY_S, GAP_SMALL, ROW_CHEVRON_ROOM, TOUCH};
use faraday_tokens::palette::Role;

/// An address and, before it, what it is.
pub struct AddressModel<'s> {
    /// What the address is, in the muted tone; empty for the address alone.
    pub label: Shown<'s>,
    /// The address, whole.
    pub address: Shown<'s>,
}

/// Where an address starts beside its label.
const BESIDE: f32 = 96.0;

/// The least height of a labelled address's line.
const LINE_LEAST: f32 = 20.0;

/// Draws the address from (x, y), `w` wide. Where `offered` takes a press the row is a touch
/// target tall at least, the address in its middle, a chevron at its end; otherwise the text
/// alone. A label stands before the address where it leaves it room, else over it. Returns the
/// height, no gap under it.
pub fn draw<A: Pressed>(
    m: &AddressModel<'_>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    offered: &Offered<A>,
) -> f32 {
    ui.showing(&m.label, |ui| {
        ui.showing(&m.address, |ui| {
            ui.offered(offered, |ui, press, _| {
                lines(m, ui, (x, y, w), press)
            })
        })
    })
}

/// [`draw`] with the press the row takes, if any.
fn lines<A: Pressed>(
    m: &AddressModel<'_>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    press: Option<A>,
) -> f32 {
    let (label, address) = (m.label.as_str(), m.address.as_str());
    // The address beside its label where the label leaves it room, else under it.
    let (ax, ay, least) = if label.is_empty() {
        (0.0, 0.0, 0.0)
    } else if ui.compact || ui.measure_as(BODY_S, label) + GAP_SMALL > BESIDE {
        (0.0, ui.advance_as(BODY_S, 20.0), LINE_LEAST)
    } else {
        (BESIDE, 0.0, LINE_LEAST)
    };
    let room = if press.is_some() { ROW_CHEVRON_ROOM } else { 0.0 };
    let laid = ui.wrapped_as(BODY_M.mono(), address, w - ax - room);
    let natural = ay + laid.h.max(least);
    // A row that opens a code is a touch target tall, its text in its middle.
    let pad = if press.is_some() {
        ((TOUCH - natural) / 2.0).max(0.0)
    } else {
        0.0
    };
    let band = natural + 2.0 * pad;
    if let Some(a) = press {
        ui.press_box(x, y, w, band, a);
        ui.row_chevron(x + w, y, band, Role::OnSurfaceVariant);
    }
    if !label.is_empty() {
        ui.text_as(x, y + pad + 2.0, BODY_S, Role::OnSurfaceVariant, label);
    }
    ui.draw_wrapped((x + ax, y + pad + ay), Role::OnSurface, &laid);
    band
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas};
    use faraday_atoms::{MotionStore, Theme};

    const ADDRESS: &str = "bc1q 8c6f d6u2 0r6m 5yq9 zz4k w2pv f0xt aw8c 6fd6 u20r 6m5y";

    /// Invariant: An address that opens its code takes a press over its whole row, a touch target tall at least, and an address that opens nothing takes none.
    /// Regression: A short address's row is only as tall as its text, a target too small to press, or a change address takes a press it does not act on.
    /// Owner: kit
    #[test]
    fn an_address_that_opens_its_code_is_a_touch_target_tall() {
        let drawn = |offered: Offered<P>| {
            let mut c = canvas(160);
            let mut store = MotionStore::new();
            let pointer = at_rest(None);
            let mut ui: Ui<'_, P> =
                Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, true);
            let m = AddressModel {
                label: Shown::plain("Receive 0"),
                address: Shown::plain(ADDRESS),
            };
            let h = draw(&m, &mut ui, (16.0, 16.0, 320.0), &offered);
            (h, ui.touch, ui.finish().hits)
        };
        let (h, touch, hits) = drawn(Offered::Yes(P(1)));
        assert_eq!(hits.len(), 1);
        assert!(h >= touch, "{h} under {touch}");
        let (_, _, none) = drawn(Offered::No);
        assert!(none.is_empty());
    }
}
