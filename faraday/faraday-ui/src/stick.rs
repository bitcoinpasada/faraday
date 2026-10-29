//! A stick's pill (`docs/THEMES.md` §6, `docs/decisions/F142.md`): the
//! boot stick's and each backup stick's, in its own tone. While the
//! stick is in, the pill is filled in its tone and what it writes is
//! white or black on it; while the flow asks for it, the pill is edged in
//! its tone; otherwise it is quiet. Its text is on-surface or on the tone
//! at 4.5:1 or more in every scheme, 7:1 at High contrast, and nothing in
//! it is see-through.

use crate::pal::*;
use crate::{Shape, Ui, W};

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

/// A pill's box and inside: its left edge, top, width and height, the
/// room before its dot, the dot's radius, how far in from the left
/// padding its text starts, and the text's size, in units.
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
    /// The text's size, in the medium face.
    pub text: f32,
}

/// What a pill of tone `n` ([`stick`]) in `state` draws in, in `theme`:
/// its text, its edge (none where it has no edge of its own), its body
/// and its dot.
pub fn stick_paints(
    n: usize,
    state: StickState,
    theme: crate::Theme,
) -> (Paint, Option<Paint>, Paint, Paint) {
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

impl<A: Copy + PartialEq> Ui<'_, A> {
    /// Stick `n`'s pill ([`stick`]) at `at`, reading `label`, as `state`
    /// shows it ([`stick_paints`]): round at its ends, its dot before its
    /// label. With an `action` it takes a press, its state layer in its
    /// text's colour and its focus ring in its shape.
    pub fn stick_pill(
        &mut self,
        at: PillAt,
        n: usize,
        state: StickState,
        label: &str,
        action: Option<A>,
    ) {
        let PillAt { x, y, w, h, .. } = at;
        let r = h / 2.0;
        let (ink, edge, body, dot) = stick_paints(n, state, self.theme);
        self.fill(x, y, w, h, r, body);
        if let Some(edge) = edge {
            self.stroke(x, y, w, h, r, edge);
        }
        if let Some(a) = action {
            self.press(Shape::new(x, y, w, h, r), ink, a);
        }
        self.dot(x + at.pad + at.dot_r, y + h / 2.0, at.dot_r, dot);
        self.text_mid(x + at.pad + at.dot_room, y, h, at.text, W::S, ink, label);
    }
}
