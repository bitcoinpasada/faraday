//! The stick pill: a stick's pill in its own tone (09 §3.2; `docs/THEMES.md` §6).
//!
//! While the stick is in, the pill is filled in its tone and what it writes is white or black on
//! it; while the flow asks for it, the pill is edged in its tone; otherwise it is quiet. Its
//! text is on-surface or on the tone at 4.5:1 or more in every scheme, 7:1 at High contrast,
//! and nothing in it is see-through.

use faraday_atoms::hits::Shape;
use faraday_atoms::{Offered, Pressed, Shown, Ui};
use faraday_tokens::Theme;
use faraday_tokens::dims::LABEL_M;
use faraday_tokens::palette::{Paint, Role, on_stick, stick};

/// Where a stick stands, as its pill shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickState {
    /// Neither in nor asked for.
    Quiet,
    /// The flow asks for it.
    Asked,
    /// In.
    Inserted,
}

/// A stick's pill: which stick's tone, where it stands and what it reads. What a press on it
/// does is the `Offered` given to [`draw`] (the model has no press: 09 §3.4).
pub struct StickPillModel<'s> {
    /// The stick's tone, counted from the boot stick's, which is 0.
    pub n: usize,
    pub state: StickState,
    pub label: Shown<'s>,
}

/// A pill's box and inside: its left edge, top, width and height, the room before its dot, the
/// dot's radius and how far in from the left padding its text starts, in units; its text is
/// Label M, as the mode pill's.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PillAt {
    /// Left edge.
    pub x: f32,
    /// Top.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
    /// Room from the left edge to the dot.
    pub pad: f32,
    /// The dot's radius.
    pub dot_r: f32,
    /// From the padding to the text, past the dot.
    pub dot_room: f32,
}

/// What a pill of tone `n` in `state` draws in, in `theme`: its text, its edge (none where it has
/// no edge of its own), its body and its dot.
fn paints(n: usize, state: StickState, theme: Theme) -> (Paint, Option<Paint>, Paint, Paint) {
    let tone = stick(n);
    match state {
        StickState::Inserted => {
            let ink = on_stick(theme);
            (ink, None, tone, ink)
        }
        StickState::Asked => (
            Role::OnSurface.paint(),
            Some(tone),
            Role::SurfaceContainerHighest.paint(),
            tone,
        ),
        StickState::Quiet => (
            Role::OnSurfaceVariant.paint(),
            Some(Role::OutlineVariant.paint()),
            Role::SurfaceContainerHighest.paint(),
            tone,
        ),
    }
}

/// Draws the pill in the box `at`: round at its ends, its dot before its label. With a press
/// offered it takes one, its state layer in its text's colour and its focus ring in its shape.
pub fn draw<A: Pressed>(
    m: &StickPillModel<'_>,
    ui: &mut Ui<'_, A>,
    at: PillAt,
    offered: &Offered<A>,
) {
    let PillAt { x, y, w, h, .. } = at;
    let r = h / 2.0;
    let (ink, edge, body, dot) = paints(m.n, m.state, ui.theme);
    ui.showing(&m.label, |ui| {
        ui.fill(x, y, w, h, r, body);
        if let Some(edge) = edge {
            ui.stroke(x, y, w, h, r, edge);
        }
        ui.offered(offered, |ui, press, _| {
            if let Some(a) = press {
                ui.press(Shape::new(x, y, w, h, r), ink, a);
            }
        });
        ui.dot(x + at.pad + at.dot_r, y + h / 2.0, at.dot_r, dot);
        ui.text_mid_as(
            (x + at.pad + at.dot_room, y, h),
            LABEL_M,
            ink,
            m.label.as_str(),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::*;

    fn at() -> PillAt {
        PillAt {
            x: 8.0,
            y: 8.0,
            w: 120.0,
            h: 28.0,
            pad: 10.0,
            dot_r: 4.0,
            dot_room: 14.0,
        }
    }

    fn pill(
        state: StickState,
        offered: Offered<P>,
    ) -> (faraday_atoms::FrameOut<P>, [(u8, u8, u8); 2]) {
        let mut canvas = canvas(160);
        let out = frame(&mut canvas, &at_rest(None), |ui| {
            ui.clear(page());
            let m = StickPillModel {
                n: 0,
                state,
                label: Shown::plain("Stick"),
            };
            draw(&m, ui, at(), &offered);
        });
        // Inside the pill left of its dot, and at its top edge's middle.
        (out, [pixel(&canvas, 118, 22), pixel(&canvas, 68, 8)])
    }

    /// Invariant: A stick that is in is filled in its tone; one that is asked for or not in is not, and a pill takes a press only where one is offered.
    /// Regression: Every pill is drawn filled whatever its state, or a quiet pill takes a press nobody offered.
    /// Owner: kit
    #[test]
    fn a_stick_that_is_in_is_filled_and_a_press_is_taken_only_where_offered() {
        let (inserted, in_px) = pill(StickState::Inserted, Offered::Yes(P(7)));
        let (quiet, quiet_px) = pill(StickState::Quiet, Offered::No);
        let (asked, asked_px) = pill(StickState::Asked, Offered::No);
        assert_ne!(in_px[0], quiet_px[0]);
        assert_eq!(quiet_px[0], asked_px[0]);
        assert_ne!(
            quiet_px[1], asked_px[1],
            "an asked pill is edged in its tone"
        );
        assert_eq!(inserted.hits.len(), 1);
        assert!(quiet.hits.is_empty() && asked.hits.is_empty());
    }
}
