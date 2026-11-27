//! The time zone molecule: the time zone row, which opens the list, and the pair of steppers
//! that move it by a quarter of an hour (09 §3.2).

use faraday_atoms::button::{ButtonModel, Style};
use faraday_atoms::{Offered, Pressed, Shown, Ui};
use faraday_tokens::dims::{BODY_S, BUTTON_H, TITLE_S};
use faraday_tokens::palette::Role;

/// Which of the two ways the zone is shown and changed.
pub enum TimeZoneKind<'s, A> {
    /// The row: its label and a button that reads the zone and opens the list.
    Row { label: Shown<'s>, open: Offered<A> },
    /// The zone between a minus and a plus button.
    Steps { minus: Offered<A>, plus: Offered<A> },
}

/// A time zone as a row or as steppers.
pub struct TimeZoneModel<'s, A> {
    /// The zone as it reads ("UTC+01:00", or that it is not set), made by `present`.
    pub value: Shown<'s>,
    pub kind: TimeZoneKind<'s, A>,
}

/// A secondary button of the row or steppers.
fn button<'s, A: Clone>(label: Shown<'s>, offered: &Offered<A>) -> ButtonModel<'s, A> {
    ButtonModel {
        label,
        style: Style::Outlined,
        offered: offered.clone(),
        chosen: false,
        icon: None,
    }
}

/// Draws the model from (x, y), `w` wide. Returns its height.
pub fn draw<A: Pressed>(
    m: &TimeZoneModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, _w): (f32, f32, f32),
) -> f32 {
    match &m.kind {
        TimeZoneKind::Row { label, open } => row(m, ui, (x, y), (label, open)),
        TimeZoneKind::Steps { minus, plus } => steppers(m, ui, (x, y), (minus, plus)),
    }
}

fn row<A: Pressed>(
    m: &TimeZoneModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y): (f32, f32),
    (label, open): (&Shown<'_>, &Offered<A>),
) -> f32 {
    let bh = ui.button_height(BUTTON_H);
    let (ly, bx, h) = if ui.compact {
        ui.showing(label, |ui| {
            ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, label.as_str());
        });
        let ly = y + ui.advance_as(BODY_S, 24.0);
        (ly, x, ly - y + bh)
    } else {
        ui.showing(label, |ui| {
            ui.text_mid_as((x, y, 40.0), BODY_S, Role::OnSurfaceVariant, label.as_str());
        });
        (y, x + 180.0, bh)
    };
    faraday_atoms::button::draw(&button(m.value.clone(), open), ui, (bx, ly, None, BUTTON_H));
    h
}

fn steppers<A: Pressed>(
    m: &TimeZoneModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y): (f32, f32),
    (minus, plus): (&Offered<A>, &Offered<A>),
) -> f32 {
    let step = |ui: &mut Ui<'_, A>, at: f32, sign: &'static str, offered: &Offered<A>| {
        let b = button(Shown::plain(sign), offered);
        faraday_atoms::button::draw(&b, ui, (at, y, Some(40.0), BUTTON_H));
    };
    step(ui, x, "-", minus);
    // A one-line slot between the buttons: larger text fits down there.
    let slot = 84.0;
    let value = m.value.as_str();
    let vw = ui.text_fit_width(TITLE_S, value, slot);
    let bh = ui.button_height(BUTTON_H);
    ui.showing(&m.value, |ui| {
        let at = (x + 48.0 + (slot - vw) / 2.0, y, bh, slot);
        ui.text_fit_as(at, TITLE_S, Role::OnSurface, value);
    });
    step(ui, x + 132.0, "+", plus);
    bh
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::*;

    fn presses(kind: TimeZoneKind<'static, P>) -> Vec<u8> {
        let mut canvas = canvas(160);
        let m = TimeZoneModel {
            value: Shown::plain("UTC+01:00"),
            kind,
        };
        let out = frame(&mut canvas, &at_rest(None), |ui| {
            ui.clear(page());
            draw(&m, ui, (8.0, 8.0, 300.0));
        });
        out.hits.iter().map(|h| h.press.0).collect()
    }

    /// Invariant: The row opens the list with one press; the steppers send their own, the minus before the plus.
    /// Regression: The two steppers send each other's press, or the row's button is not pressable.
    /// Owner: kit
    #[test]
    fn the_row_opens_the_list_and_the_steppers_step_either_way() {
        let row = TimeZoneKind::Row {
            label: Shown::plain("Time zone"),
            open: Offered::Yes(P(1)),
        };
        assert_eq!(presses(row), [1]);
        let steps = TimeZoneKind::Steps {
            minus: Offered::Yes(P(2)),
            plus: Offered::Yes(P(3)),
        };
        assert_eq!(presses(steps), [2, 3]);
    }
}
