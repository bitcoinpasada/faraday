//! The code box molecule: the exact characters of a value, in the mono face on a tinted,
//! rounded box (`docs/DESIGN.md` §4.3; 09 §3.2).

use faraday_atoms::{Pressed, Shown, Ui, W};
use faraday_tokens::dims::{GAP, GAP_SMALL, RADIUS_SMALL};
use faraday_tokens::palette::Role;
use faraday_tokens::type_roles::TypeRole;

/// A value in its code box, with the text before it if any.
pub struct CodeBoxModel<'s> {
    /// The text before the box, regular; empty for the box alone.
    pub lead: Shown<'s>,
    /// The value, as it is written.
    pub code: Shown<'s>,
    /// The type role whose size both are set at.
    pub role: TypeRole,
}

/// The height of a code box at `size`.
pub fn height_at<A: Pressed>(ui: &Ui<'_, A>, size: f32) -> f32 {
    ui.line(size, W::M) + 2.0 * GAP_SMALL
}

/// The box alone, its top left at (x, y), `size` high text, padded by tokens and with no quotes.
/// Returns its width.
pub fn code_box<A: Pressed>(ui: &mut Ui<'_, A>, (x, y): (f32, f32), size: f32, text: &str) -> f32 {
    let width = ui.measure(size, W::M, text) + 2.0 * GAP;
    let h = height_at(ui, size);
    ui.fill(x, y, width, h, RADIUS_SMALL, Role::SurfaceContainerHighest);
    ui.text_mid(x + GAP, y, h, size, W::M, Role::OnSurface, text);
    width
}

/// Draws the lead and the code in its box, the box on the next line where it does not fit
/// beside the lead in `w`. Returns the height.
pub fn draw<A: Pressed>(
    m: &CodeBoxModel<'_>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let size = m.role.size;
    ui.showing(&m.lead, |ui| {
        ui.showing(&m.code, |ui| {
            let (lead, code) = (m.lead.as_str(), m.code.as_str());
            let h = height_at(ui, size);
            if lead.is_empty() {
                code_box(ui, (x, y), size, code);
                return h;
            }
            let lead_w = ui.measure(size, W::R, lead);
            let box_w = ui.measure(size, W::M, code) + 2.0 * GAP;
            if lead_w + GAP + box_w <= w {
                ui.text_mid(x, y, h, size, W::R, Role::OnSurfaceVariant, lead);
                code_box(ui, (x + lead_w + GAP, y), size, code);
                return h;
            }
            let lead_h = ui.wrap(x, y, w, size, W::R, Role::OnSurfaceVariant, lead);
            code_box(ui, (x, y + lead_h + GAP_SMALL), size, code);
            lead_h + GAP_SMALL + h
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas};
    use faraday_atoms::{MotionStore, Theme};
    use faraday_tokens::dims::BODY_M;

    /// Invariant: A code box stands beside its lead where both fit the room, and under it where they do not, the height returned holding both.
    /// Regression: A long value's box runs past the room beside its lead, or the second line's height is lost and what follows is drawn over the box.
    /// Owner: kit
    #[test]
    fn a_code_box_goes_under_its_lead_where_it_does_not_fit_beside_it() {
        let mut c = canvas(160);
        let mut store = MotionStore::new();
        let pointer = at_rest(None);
        let mut ui: Ui<'_, P> = Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, true);
        let m = CodeBoxModel {
            lead: Shown::plain("The word is"),
            code: Shown::plain("abandon-ability"),
            role: BODY_M,
        };
        let one = draw(&m, &mut ui, (0.0, 0.0, 400.0));
        let two = draw(&m, &mut ui, (0.0, 0.0, 120.0));
        assert!(two > one, "{two} not taller than {one}");
    }
}
