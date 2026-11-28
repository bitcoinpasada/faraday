//! The caution molecule: a line in the warning tone (04 §12.1, 09 §3.2).

use faraday_atoms::{Pressed, Shown, Ui};
use faraday_tokens::dims::{BODY_M, LABEL_L};
use faraday_tokens::palette::Role;

/// How large a caution is set: the label large of a caution over a list, or the body medium of
/// one in a sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    /// Material's label large: the caution over a wallet's receive addresses.
    Label,
    /// Material's body medium: the caution a sheet or a dialog carries.
    Body,
}

/// A caution: a statement of fact in the warning tone, wrapped to the room it has.
pub struct CautionModel<'s> {
    /// What it says.
    pub text: Shown<'s>,
    /// How large it is set.
    pub size: Size,
}

/// Draws the caution at (x, y), `w` wide, wrapped. Returns its height, no gap under it.
pub fn draw<A: Pressed>(m: &CautionModel<'_>, ui: &mut Ui<'_, A>, at: (f32, f32, f32)) -> f32 {
    draw_in(m, ui, at, Role::Warning)
}

/// The same line in the error tone: what is lost (`main`'s lock sheet lists what is kept nowhere
/// else in it).
pub fn draw_error<A: Pressed>(
    m: &CautionModel<'_>,
    ui: &mut Ui<'_, A>,
    at: (f32, f32, f32),
) -> f32 {
    draw_in(m, ui, at, Role::Error)
}

fn draw_in<A: Pressed>(
    m: &CautionModel<'_>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    tone: Role,
) -> f32 {
    let role = match m.size {
        Size::Label => LABEL_L,
        Size::Body => BODY_M,
    };
    ui.showing(&m.text, |ui| {
        ui.wrap_as((x, y, w), role, tone, m.text.as_str())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas};
    use faraday_atoms::{MotionStore, Theme};
    use faraday_draw::canvas::InkKind;

    /// Invariant: A caution wider than its room wraps onto more lines and its height is the lines it took, so what follows starts under it.
    /// Regression: A long caution is cut to one line, or its height is one line while it draws several and what follows is drawn over it.
    /// Owner: kit
    #[test]
    fn a_long_caution_wraps_and_returns_the_height_it_took() {
        let mut c = canvas(160);
        let mut store = MotionStore::new();
        let pointer = at_rest(None);
        let mut ui: Ui<'_, P> = Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, true);
        let m = CautionModel {
            text: Shown::plain(
                "Some keys of this wallet are not checked, so its addresses may not be yours",
            ),
            size: Size::Label,
        };
        let h = draw(&m, &mut ui, (16.0, 16.0, 200.0));
        let lines = ui
            .finish()
            .ink
            .expect("the record is on")
            .0
            .into_iter()
            .filter(|i| matches!(i.kind, InkKind::Text { .. }))
            .count();
        assert!(lines >= 2, "{lines} lines");
        assert!(h >= 2.0 * 14.0, "{h}");
    }
}
