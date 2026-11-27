//! Segmented buttons (09 §3.2).

use faraday_tokens::palette::Role;

use crate::Pressed;
use crate::hits::{Ring, Shape};
use crate::shown::Shown;
use crate::ui::Ui;
use crate::ui::W;
use faraday_draw::icons::Icon;
use faraday_tokens::dims::{ICON_GAP, ICON_GLYPH_BUTTON, SEGMENT_PAD, shape_full};

/// A segmented button: its options, each with its press, and the one chosen.
pub struct SegmentedModel<'s, A> {
    pub options: &'s [(Shown<'s>, A)],
    /// Options by index that are disabled: no press (one past its end is enabled).
    pub off: &'s [bool],
    pub chosen: Option<usize>,
    /// While the choice moves, where the fill is, in segments from the first.
    pub slide: Option<f32>,
    /// The labels' size.
    pub size: f32,
}

/// Draws the button at (x, y), `h` tall, design units. Returns its width.
pub fn draw<A: Pressed>(m: &SegmentedModel<'_, A>, ui: &mut Ui<'_, A>, at: (f32, f32, f32)) -> f32 {
    let options: Vec<(&str, A)> = m
        .options
        .iter()
        .map(|(l, a)| (l.as_str(), a.clone()))
        .collect();
    let secret = m.options.iter().map(|o| &o.0).find(|s| s.is_secret());
    match secret {
        Some(s) => ui.showing(s, |ui| {
            ui.segmented_with(at, m.size, &options, m.chosen, m.slide, m.off)
        }),
        None => ui.segmented_with(at, m.size, &options, m.chosen, m.slide, m.off),
    }
}

impl<A: Pressed> Ui<'_, A> {
    /// The width of one segment of a segmented button over `labels` at
    /// `size`: the widest label, its padding and the tick's room, so no
    /// segment changes width as the choice moves.
    pub fn segment_width(&self, size: f32, labels: &[&str]) -> f32 {
        self.segment_width_of(size, labels.iter().copied())
    }

    /// [`Ui::segment_width`] over the labels `labels` yields.
    pub(crate) fn segment_width_of<'s>(
        &self,
        size: f32,
        labels: impl Iterator<Item = &'s str>,
    ) -> f32 {
        let widest = labels
            .map(|l| self.measure(size, W::S, l))
            .fold(0.0, f32::max);
        widest + 2.0 * SEGMENT_PAD + ICON_GLYPH_BUTTON + ICON_GAP
    }

    /// A segmented button (Material's): `options` side by side in one
    /// outlined container `h` tall at (x, y), round at its ends, each
    /// segment [`Ui::segment_width`] wide; the chosen one filled in
    /// secondary-container with a tick. `slide`, while the choice moves, is where
    /// the fill is, in segments from the first. Returns its width.
    pub fn segmented(
        &mut self,
        at: (f32, f32, f32),
        size: f32,
        options: &[(&str, A)],
        chosen: Option<usize>,
        slide: Option<f32>,
    ) -> f32 {
        self.segmented_with(at, size, options, chosen, slide, &[])
    }

    /// [`Ui::segmented`] with the options `off` marks (by index; one past
    /// its end is enabled) disabled: label and tick at the disabled
    /// opacity, no press (Material's disabled segment).
    pub fn segmented_with(
        &mut self,
        (x, y, h): (f32, f32, f32),
        size: f32,
        options: &[(&str, A)],
        chosen: Option<usize>,
        slide: Option<f32>,
        off: &[bool],
    ) -> f32 {
        let sw = self.segment_width_of(size, options.iter().map(|o| o.0));
        let total = sw * options.len() as f32;
        let r = shape_full(h);
        let shape = self.rect(x, y, total, h);
        let at = slide.or(chosen.map(|c| c as f32));
        if let Some(at) = at {
            let seg = self.rect(x + at * sw, y, sw, h).intersect(&shape);
            let fill = self.col(Role::SecondaryContainer);
            self.c.push_clip(seg);
            self.c.fill_rounded_rect(shape, r * self.f, fill);
            self.c.pop_clip();
        }
        for (i, (label, action)) in options.iter().enumerate() {
            let label = *label;
            let sx = x + i as f32 * sw;
            let on = chosen == Some(i);
            let off = off.get(i).copied().unwrap_or(false);
            let dim = Role::OnSurface.at(faraday_tokens::dims::DISABLED_CONTENT);
            let layer = if on {
                Role::OnSecondaryContainer
            } else {
                Role::OnSurface
            }
            .paint();
            let ends = (i == 0, i + 1 == options.len());
            let ring = Ring::Segment {
                first: ends.0,
                last: ends.1,
            };
            if !off {
                self.press_segment(Shape::new(sx, y, sw, h, r), ring, layer, action.clone());
            }
            if i > 0 {
                self.vrule(sx, y, h, Role::Outline);
            }
            let tw = self.measure(size, W::S, label);
            if on {
                let gw = ICON_GLYPH_BUTTON + ICON_GAP + tw;
                let gx = sx + (sw - gw) / 2.0;
                let iy = y + (h - ICON_GLYPH_BUTTON) / 2.0;
                self.icon(
                    gx,
                    iy,
                    ICON_GLYPH_BUTTON,
                    Icon::Done,
                    ICON_GLYPH_BUTTON,
                    if off {
                        dim
                    } else {
                        Role::OnSecondaryContainer.paint()
                    },
                );
                let lx = gx + ICON_GLYPH_BUTTON + ICON_GAP;
                let fg = if off {
                    dim
                } else {
                    Role::OnSecondaryContainer.paint()
                };
                self.text_mid(lx, y, h, size, W::S, fg, label);
            } else {
                self.text_mid(
                    sx + (sw - tw) / 2.0,
                    y,
                    h,
                    size,
                    W::S,
                    if off { dim } else { Role::OnSurface.paint() },
                    label,
                );
            }
        }
        self.stroke(x, y, total, h, r, Role::Outline);
        total
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::testing::*;

    /// Invariant: Every segment of a segmented button takes its own press.
    /// Regression: A segmented button takes one press for the whole control.
    /// Owner: kit
    #[test]
    fn every_segment_of_a_segmented_button_takes_its_own_press() {
        let mut c = canvas(160);
        let options = [("One", P(1)), ("Two", P(2)), ("Three", P(3))];
        let mut seg = 0.0;
        let out = frame(&mut c, &at_rest(None), |ui| {
            let w = ui.segmented((20.0, 20.0, 40.0), 14.0, &options, Some(1), None);
            seg = w / 3.0;
        });
        for (i, (_, p)) in options.iter().enumerate() {
            let x = (20.0 + seg * (i as f32 + 0.5)) as i32;
            let found = out.hits.iter().rev().find(|h| h.rect.contains(x, 40));
            assert_eq!(found.map(|h| h.press), Some(*p));
        }
    }
}
