//! The way to make a region pressable (`docs/M3-CHECKS.md` §4.2). Each
//! function registers the press (its box, its touch target, Tab order,
//! as [`Ui::hit`] does) and draws its state layer and focus ring in the
//! same call: neither can be left out. The layer is drawn where the call
//! is made, so a component calls it after its container and before its
//! content (Material: the layer sits between container and content).
//! Hover is Faraday's (F108, F113): every function draws it where the
//! pointer rests.

use crate::pal::*;
use crate::{Reach, Ui};
use faraday_draw::Rect;
use faraday_draw::tokens::{STATE_FOCUS, STATE_HOVER, alpha_of};

/// A control's shape: where its state layer and focus ring go, design
/// units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    /// Left.
    pub x: f32,
    /// Top.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
    /// Corner radius.
    pub r: f32,
}

impl Shape {
    /// The shape of the box (x, y, w, h) with corners of `r`.
    pub const fn new(x: f32, y: f32, w: f32, h: f32, r: f32) -> Shape {
        Shape { x, y, w, h, r }
    }

    const fn at(self) -> (f32, f32, f32, f32, f32) {
        (self.x, self.y, self.w, self.h, self.r)
    }

    const fn rect(self) -> (f32, f32, f32, f32) {
        (self.x, self.y, self.w, self.h)
    }
}

/// How the focus ring follows the shape: whole, or one segment of a
/// segmented button, square where it meets another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ring {
    /// All round the shape.
    Whole,
    /// One segment of a segmented button.
    Segment {
        /// The button's first segment: round at its start.
        first: bool,
        /// The button's last segment: round at its end.
        last: bool,
    },
}

/// A press with no state layer, and the case that allows it. Each
/// variant names its case; a new one is the owner's decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plain {
    /// Beside an open sheet: its way out ([`Ui::hit_around`]).
    Beside,
    /// A menu's or sheet's own body: a press there closes nothing.
    Body,
    /// The page itself, a quarter of the window or more.
    Page,
}

/// A press with no pressed layer (hover and focus kept), and the case
/// that allows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unpressed {
    /// A held secret: what it shows reads as drawn under the finger.
    HeldSecret,
    /// An on-screen keyboard's key: a watcher reads nothing from a
    /// press (F152, Faraday's difference from Material).
    Keyboard,
}

/// How many presses a frame holds at one point, to come back to
/// ([`Ui::presses_mark`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PressMark {
    hits: usize,
    reach: usize,
}

impl<A: Copy + PartialEq> Ui<'_, A> {
    /// Layer in `ink` over `shape`, ring round it, target `shape` grown
    /// to TOUCH.
    pub fn press(&mut self, shape: Shape, ink: Paint, action: A) {
        self.press_to(shape, shape.rect(), ink, action);
    }

    /// As [`Ui::press`], the target the larger box `target` (an icon
    /// button, a navigation pill).
    pub fn press_to(&mut self, shape: Shape, target: (f32, f32, f32, f32), ink: Paint, action: A) {
        self.press_custom(target, action, |ui| {
            ui.state_layer(shape.at(), ink, action);
            ui.focus_ring(shape.at(), action);
        });
    }

    /// As [`Ui::press_to`], the layer a header band's: `shape`'s top
    /// corners rounded, its foot square, where it meets the box under it.
    pub fn press_head(
        &mut self,
        shape: Shape,
        target: (f32, f32, f32, f32),
        ink: Paint,
        action: A,
    ) {
        self.press_custom(target, action, |ui| {
            // A shape a corner taller, its foot cut off at the band's.
            let band = ui.rect(shape.x, shape.y, shape.w, shape.h);
            ui.c.push_clip(band);
            let tall = Shape::new(shape.x, shape.y, shape.w, shape.h + shape.r, shape.r);
            ui.state_layer(tall.at(), ink, action);
            ui.c.pop_clip();
            ui.focus_ring(shape.at(), action);
        });
    }

    /// As [`Ui::press`], the ring one segment's. The layer is the
    /// segment's, square where it meets another.
    pub fn press_segment(&mut self, shape: Shape, ring: Ring, ink: Paint, action: A) {
        let Ring::Segment { first, last } = ring else {
            return self.press(shape, ink, action);
        };
        self.press_custom(shape.rect(), action, |ui| {
            // The whole button's shape cut to the segment: the segment
            // widened past each inner end by its corner.
            let out = |end: bool| if end { 0.0 } else { shape.r };
            let (left, right) = (out(first), out(last));
            let seg = ui.rect(shape.x, shape.y, shape.w, shape.h);
            ui.c.push_clip(seg);
            let wide = Shape::new(
                shape.x - left,
                shape.y,
                shape.w + left + right,
                shape.h,
                shape.r,
            );
            ui.state_layer(wide.at(), ink, action);
            ui.c.pop_clip();
            ui.segment_ring(shape.at(), (first, last), action);
        });
    }

    /// Hover and focus layers and the ring, no pressed layer, for one of
    /// [`Unpressed`]'s cases.
    pub fn press_unpressed(&mut self, shape: Shape, ink: Paint, why: Unpressed, action: A) {
        // Both cases are the same drawing; the variant names the case.
        let _ = why;
        self.press_custom(shape.rect(), action, |ui| {
            let opacity = if ui.focused == Some(action) {
                Some(STATE_FOCUS)
            } else if ui.is_hovered(action) {
                Some(STATE_HOVER)
            } else {
                None
            };
            if let Some(o) = opacity {
                let (x, y, w, h, r) = shape.at();
                ui.wash(x, y, w, h, r, ink.with_alpha(alpha_of(o)));
            }
            ui.focus_ring(shape.at(), action);
        });
    }

    /// No layer, for one of [`Plain`]'s cases.
    pub fn press_plain(&mut self, at: (f32, f32, f32, f32), why: Plain, action: A) {
        let (x, y, w, h) = at;
        match why {
            Plain::Beside => self.hit_around(x, y, w, h, action),
            Plain::Body | Plain::Page => self.hit_drawn(x, y, w, h, action),
        }
    }

    /// Today's default, unchanged: on-surface over the box, `SHAPE_S`
    /// corners.
    pub fn press_box(&mut self, x: f32, y: f32, w: f32, h: f32, action: A) {
        self.hit(x, y, w, h, action);
    }

    /// A press whose layer and ring `draw` lays itself, for the few
    /// controls whose layer is not their box (a switch's handle, a menu
    /// item's inset ring); the press is taken on `target`. A dimmed
    /// action draws and takes nothing.
    pub(crate) fn press_custom(
        &mut self,
        target: (f32, f32, f32, f32),
        action: A,
        draw: impl FnOnce(&mut Self),
    ) {
        if self.dimmed.contains(&action) {
            return;
        }
        draw(self);
        let (x, y, w, h) = target;
        self.hit_drawn(x, y, w, h, action);
    }

    /// The presses taken so far this frame, to measure what is drawn
    /// next ([`Ui::presses_rewind`], [`Ui::clip_presses_since`]).
    pub fn presses_mark(&self) -> PressMark {
        PressMark {
            hits: self.hits.len(),
            reach: self.reach.len(),
        }
    }

    /// Forgets the presses taken since `mark`: what was drawn only to be
    /// measured, or drawn and then left out.
    pub fn presses_rewind(&mut self, mark: PressMark) {
        self.hits.truncate(mark.hits);
        self.reach.truncate(mark.reach);
    }

    /// Cuts the presses taken since `mark` to `clip`, in pixels: one with
    /// nothing left in it is dropped, one part-way out keeps its box and
    /// its target inside `clip` (marked cut), so what is scrolled out of
    /// a pane cannot be pressed.
    pub fn clip_presses_since(&mut self, mark: PressMark, clip: Rect) {
        let mut k = mark.hits;
        while k < self.hits.len() {
            let r = self.hits[k].0.intersect(&clip);
            if r.is_empty() {
                self.hits.remove(k);
            } else {
                self.hits[k].0 = r;
                k += 1;
            }
        }
        let mut k = mark.reach;
        while k < self.reach.len() {
            let t = &mut self.reach[k];
            let drawn = t.drawn.intersect(&clip);
            if drawn.is_empty() {
                self.reach.remove(k);
                continue;
            }
            t.cut |= drawn != t.drawn;
            t.drawn = drawn;
            t.target = t.target.intersect(&clip);
            k += 1;
        }
    }

    /// Takes the frame's touch targets, in the order drawn, to hand to the
    /// pointer (the boxes were given to [`Ui::new`]'s list).
    pub fn take_presses(&mut self) -> Vec<Reach<A>> {
        std::mem::take(&mut self.reach)
    }

    /// Forgets the presses drawn so far that lie wholly inside `area`, in
    /// pixels: a pane drawn over a page takes the page's presses under it.
    pub fn drop_presses_inside(&mut self, area: Rect) {
        self.hits.retain(|(r, _)| {
            !(r.x >= area.x
                && r.y >= area.y
                && r.right() <= area.right()
                && r.bottom() <= area.bottom())
        });
    }

    /// Cuts the presses drawn so far at the top of `area`, in pixels: one
    /// across `area` that begins in or below it is dropped, one that runs
    /// into it from above is cut to end there. A surface drawn over a page
    /// at its foot takes the presses under it.
    pub fn cover_presses(&mut self, area: Rect) {
        let mut k = 0;
        while k < self.hits.len() {
            let r = self.hits[k].0;
            let across = r.x < area.x + area.w && r.x + r.w > area.x;
            if across && r.y >= area.y {
                self.hits.remove(k);
                continue;
            }
            if across && r.y + r.h > area.y {
                self.hits[k].0.h = area.y - r.y;
            }
            k += 1;
        }
    }
}
