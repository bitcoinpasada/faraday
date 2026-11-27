//! Hits and the Tab order (04 §7.1), and the way to make a region pressable
//! (`docs/M3-CHECKS.md` §4.2). Each function registers the press (its box, its touch target,
//! the Tab order) and draws its state layer and focus ring in the same call: neither can be
//! left out. The layer is drawn where the call is made, so a component calls it after its
//! container and before its content. Hover is Faraday's (F108, F113): every function draws
//! it where the pointer rests.

use faraday_draw::geom::Rect;
use faraday_tokens::dims::{STATE_FOCUS, STATE_HOVER, alpha_of};
use faraday_tokens::palette::{Paint, Role};

use crate::Pressed;
use crate::ui::Ui;

/// A press recorded where it was drawn.
pub struct Hit<R> {
    /// The control as drawn.
    pub rect: Rect,
    /// Where a press is taken: the control grown to the touch floor (`main`'s `Reach`). Two
    /// targets that would overlap share the gap between their controls, each up to the middle,
    /// so a target never covers another's control nor reaches past the middle of the gap.
    pub target: Rect,
    /// What a press sends, lifted to the root's press.
    pub press: R,
    /// 0 for the page, then 1 for the lowest sheet over it and up; only the top layer drawn
    /// takes presses.
    pub layer: u8,
}

/// The hit a press at pixel (x, y) takes: of the top layer drawn, the last drawn first, the one
/// whose target it falls in.
pub fn hit_at<R>(hits: &[Hit<R>], x: i32, y: i32) -> Option<&Hit<R>> {
    let top = hits.iter().map(|h| h.layer).max()?;
    hits.iter()
        .rev()
        .find(|h| h.layer == top && h.target.contains(x, y))
}

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

impl<A: Pressed> Ui<'_, A> {
    /// Layer in `ink` over `shape`, ring round it, target `shape` grown
    /// to TOUCH.
    pub fn press(&mut self, shape: Shape, ink: Paint, action: A) {
        self.press_to(shape, shape.rect(), ink, action);
    }

    /// As [`Ui::press`], the target the larger box `target` (an icon
    /// button, a navigation pill).
    pub fn press_to(&mut self, shape: Shape, target: (f32, f32, f32, f32), ink: Paint, action: A) {
        self.press_custom(target, action, |ui, action| {
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
        self.press_custom(target, action, |ui, action| {
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
    pub(crate) fn press_segment(&mut self, shape: Shape, ring: Ring, ink: Paint, action: A) {
        let Ring::Segment { first, last } = ring else {
            return self.press(shape, ink, action);
        };
        self.press_custom(shape.rect(), action, |ui, action| {
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
        self.press_custom(shape.rect(), action, |ui, action| {
            let opacity = if ui.is_focused(action) {
                Some(STATE_FOCUS)
            } else if ui.over == Some(true) {
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
}

/// Targets `a` and `b`, of controls drawn at `da` and `db`, each cut
/// back to the middle of the gap between the controls, across the axis
/// they are furthest apart on; as they were when the controls overlap.
fn split(a: Rect, b: Rect, da: Rect, db: Rect) -> (Rect, Rect) {
    let (mut a, mut b) = (a, b);
    let gap_x = (db.x - da.right()).max(da.x - db.right());
    let gap_y = (db.y - da.bottom()).max(da.y - db.bottom());
    if gap_x < 0 && gap_y < 0 {
        return (a, b);
    }
    if gap_x >= gap_y {
        if da.right() <= db.x {
            let mid = (da.right() + db.x) / 2;
            a.w = a.w.min(mid - a.x);
            let r = b.right();
            b.x = b.x.max(mid);
            b.w = r - b.x;
        } else {
            let mid = (db.right() + da.x) / 2;
            b.w = b.w.min(mid - b.x);
            let r = a.right();
            a.x = a.x.max(mid);
            a.w = r - a.x;
        }
    } else if da.bottom() <= db.y {
        let mid = (da.bottom() + db.y) / 2;
        a.h = a.h.min(mid - a.y);
        let bt = b.bottom();
        b.y = b.y.max(mid);
        b.h = bt - b.y;
    } else {
        let mid = (db.bottom() + da.y) / 2;
        b.h = b.h.min(mid - b.y);
        let at = a.bottom();
        a.y = a.y.max(mid);
        a.h = at - a.y;
    }
    (a, b)
}

impl<A: Pressed> Ui<'_, A> {
    /// A press whose layer and ring `draw` lays itself, for the few controls whose layer is not
    /// their box (a switch's handle, a menu item's inset ring); the press is taken on `target`.
    /// A control drawn dimmed takes nothing, unless the app answers it.
    pub fn press_custom(
        &mut self,
        target: (f32, f32, f32, f32),
        action: A,
        draw: impl FnOnce(&mut Self, &A),
    ) {
        let (x, y, w, h) = target;
        if self.answering {
            self.answer_hit(target, action);
            return;
        }
        self.over = Some(self.pointer_over(x, y, w, h));
        draw(self, &action);
        self.over = None;
        self.hit_drawn(x, y, w, h, action);
    }

    /// Whether the pointer is over the box (x, y, w, h), design units, inside the clip.
    pub fn pointer_over(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        let r = self.rect(x, y, w, h).intersect(&self.seen_clip());
        self.at.is_some_and(|p| r.contains(p.x, p.y))
    }

    /// Whether the pointer is over the box and a button or a finger is down on it.
    pub fn pointer_down_on(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.down && self.pointer_over(x, y, w, h)
    }

    /// The presses taken so far this frame, to measure what is drawn next
    /// ([`Ui::presses_rewind`], [`Ui::clip_presses_since`]).
    pub fn presses_mark(&self) -> PressMark {
        PressMark {
            hits: self.hits.len(),
            reach: self.tab.len(),
        }
    }

    /// Forgets the presses taken since `mark`: what was drawn only to be measured, or drawn and
    /// then left out.
    pub fn presses_rewind(&mut self, mark: PressMark) {
        self.hits.truncate(mark.hits);
        self.tab.truncate(mark.reach);
    }

    /// Cuts the presses taken since `mark` to `clip`, in pixels: one with nothing left in it is
    /// dropped, one part-way out keeps its box and its target inside `clip`, so what is scrolled
    /// out of a pane cannot be pressed.
    pub fn clip_presses_since(&mut self, mark: PressMark, clip: Rect) {
        let mut k = mark.hits;
        let mut dropped: Vec<usize> = Vec::new();
        while k < self.hits.len() {
            let r = self.hits[k].rect.intersect(&clip);
            if r.is_empty() {
                dropped.push(k);
            } else {
                self.hits[k].rect = r;
                self.hits[k].target = self.hits[k].target.intersect(&clip);
            }
            k += 1;
        }
        self.remove_hits(&dropped);
    }

    /// Takes away the hits at the indices `gone` (ascending), and from the Tab order.
    fn remove_hits(&mut self, gone: &[usize]) {
        for &i in gone.iter().rev() {
            self.hits.remove(i);
        }
        self.tab.retain(|t| !gone.contains(t));
        for t in self.tab.iter_mut() {
            *t -= gone.iter().filter(|&&g| g < *t).count();
        }
    }

    /// Forgets the presses drawn so far that lie wholly inside `area`, in pixels: a pane drawn
    /// over a page takes the page's presses under it.
    pub fn drop_presses_inside(&mut self, area: Rect) {
        let gone: Vec<usize> = (0..self.hits.len())
            .filter(|&i| {
                let r = self.hits[i].rect;
                r.x >= area.x
                    && r.y >= area.y
                    && r.right() <= area.right()
                    && r.bottom() <= area.bottom()
            })
            .collect();
        self.remove_hits(&gone);
    }

    /// Cuts the presses drawn so far at the top of `area`, in pixels: one across `area` that
    /// begins in or below it is dropped, one that runs into it from above is cut to end there. A
    /// surface drawn over a page at its foot takes the presses under it.
    pub fn cover_presses(&mut self, area: Rect) {
        let mut gone = Vec::new();
        for (k, hit) in self.hits.iter_mut().enumerate() {
            let r = hit.rect;
            let across = r.x < area.x + area.w && r.x + r.w > area.x;
            if across && r.y >= area.y {
                gone.push(k);
            } else if across && r.y + r.h > area.y {
                hit.rect.h = area.y - r.y;
                hit.target = hit.target.intersect(&hit.rect);
            }
        }
        self.remove_hits(&gone);
    }

    /// Forgets every press drawn so far this frame: what is under a sheet is out of reach.
    pub fn clear_hits(&mut self) {
        self.hits.clear();
        self.tab.clear();
        self.tips.clear();
    }

    /// Keeps where the hit at `at`, just offered (pixels, clipped), is taken: grown to the touch
    /// floor inside `clip`, sharing any overlap with the targets before it.
    fn reach_for(&mut self, at: usize, clip: Rect) {
        let drawn = self.hits[at].rect;
        let floor = self.px(self.touch);
        let gw = (floor - drawn.w).max(0);
        let gh = (floor - drawn.h).max(0);
        let mut target = self.grown(drawn, clip);
        if gw > 0 || gh > 0 {
            for &i in &self.tab {
                let before = &mut self.hits[i];
                if before.target.intersect(&target).is_empty() {
                    continue;
                }
                let (a, b) = split(before.target, target, before.rect, drawn);
                before.target = a;
                target = b;
            }
        }
        self.hits[at].target = target;
        self.tab.push(at);
    }

    /// `drawn` grown to the touch floor, centred, inside `clip`: what the clip alone leaves of
    /// its target.
    pub fn grown(&self, drawn: Rect, clip: Rect) -> Rect {
        let floor = self.px(self.touch);
        let gw = (floor - drawn.w).max(0);
        let gh = (floor - drawn.h).max(0);
        Rect::new(
            drawn.x - gw / 2,
            drawn.y - gh / 2,
            drawn.w + gw,
            drawn.h + gh,
        )
        .intersect(&clip)
    }

    /// Records the press region of a dimmed control the app answers: no layer, no ring and no
    /// place in the keyboard's order, only a target for the press.
    pub fn answer_hit(&mut self, (x, y, w, h): (f32, f32, f32, f32), action: A) {
        if self.around || !self.answering {
            return;
        }
        let clip = self.hit_clip.unwrap_or_else(|| self.seen_clip());
        let rect = self.rect(x, y, w, h).intersect(&clip);
        if rect.w > 0 && rect.h > 0 {
            let layer = self.layer;
            self.hits.push(Hit {
                rect,
                target: rect,
                press: action,
                layer,
            });
        }
    }

    /// Makes a region pressable: the part of it inside the clip, so what is scrolled out of
    /// sight cannot be pressed.
    ///
    /// It shows its state as Material's state layer over what was drawn there, hovered, with the
    /// keyboard on it (and the focus ring) and pressed, in small corners ([`Ui::press`] takes
    /// the control's own), unless it draws its own (a button) or covers so much of the screen
    /// that a layer would light the page. It takes a press at least `TOUCH` square.
    pub fn hit(&mut self, x: f32, y: f32, w: f32, h: f32, action: A) {
        // A control drawn dimmed takes no press, wherever it is drawn.
        if self.answering {
            self.answer_hit((x, y, w, h), action);
            return;
        }
        let clip = self.hit_clip.unwrap_or_else(|| self.seen_clip());
        let whole = self.rect(x, y, w, h);
        let rect = whole.intersect(&clip);
        // A clip with no room is a pass that measures and draws nothing.
        let level = !clip.is_empty() && whole.y < clip.bottom() && whole.bottom() > clip.y;
        if !self.around && level && (whole.x < clip.x || whole.right() > clip.right()) {
            self.cut.push(whole);
        }
        if rect.w > 0 && rect.h > 0 {
            let screen = i64::from(self.c.width()) * i64::from(self.c.height());
            let small = i64::from(rect.w) * i64::from(rect.h) * 4 < screen;
            use faraday_tokens::dims::SHAPE_S;
            // Pressed, a held secret shows what it holds as drawn, its code readable under the
            // finger ([`Ui::hit_held`]).
            let pressed = self.pointer_down_on(x, y, w, h);
            if small && !self.quiet_hover && !(self.held && pressed) {
                self.state_layer((x, y, w, h, SHAPE_S), Role::OnSurface.paint(), &action);
            }
            // A region with its own hover look is still ringed while the keyboard is on it,
            // unless it rings itself.
            if small && !self.own_ring {
                self.focus_ring((x, y, w, h, SHAPE_S), &action);
            }
            let layer = self.layer;
            let at = self.hits.len();
            self.hits.push(Hit {
                rect,
                target: rect,
                press: action,
                layer,
            });
            if !self.around {
                self.reach_for(at, clip);
            }
        }
    }

    /// [`Ui::hit`] for a held secret (a code shown while a finger is on it): no pressed layer
    /// over it, so what it shows reads as drawn; hovered and focused as [`Ui::hit`].
    #[cfg(test)]
    pub(crate) fn hit_held(&mut self, x: f32, y: f32, w: f32, h: f32, action: A) {
        self.held = true;
        self.hit(x, y, w, h, action);
        self.held = false;
    }

    /// [`Ui::hit`] for a region that draws its own hover look: no state layer; the focus ring is
    /// still drawn round it.
    pub fn hit_quiet(&mut self, x: f32, y: f32, w: f32, h: f32, action: A) {
        self.quiet_hover = true;
        self.hit(x, y, w, h, action);
        self.quiet_hover = false;
    }

    /// [`Ui::hit`] for a control that draws its own state layer and focus ring in its own shape
    /// (a button, a segment, the floating action).
    pub fn hit_drawn(&mut self, x: f32, y: f32, w: f32, h: f32, action: A) {
        self.own_ring = true;
        self.hit_quiet(x, y, w, h, action);
        self.own_ring = false;
    }

    /// Presses everywhere outside the box at (x, y), w by h, take `action`: a sheet that a press
    /// beside it closes. No hover look.
    pub fn hit_around(&mut self, x: f32, y: f32, w: f32, h: f32, action: A) {
        const FAR: f32 = 10_000.0;
        self.quiet_hover = true;
        self.around = true;
        self.hit(x - FAR, y - FAR, w + 2.0 * FAR, FAR, action.clone());
        self.hit(x - FAR, y + h, w + 2.0 * FAR, FAR, action.clone());
        self.hit(x - FAR, y, FAR, h, action.clone());
        self.hit(x + w, y, FAR, h, action);
        self.around = false;
        self.quiet_hover = false;
    }

    /// Draws only inside `rect` until [`Ui::unreveal`], while what is drawn stays pressable as
    /// if it all showed.
    pub fn reveal(&mut self, rect: Rect) {
        self.hit_clip = Some(self.seen_clip());
        self.c.push_clip(rect);
    }

    /// Ends [`Ui::reveal`].
    pub fn unreveal(&mut self) {
        self.c.pop_clip();
        self.hit_clip = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::testing::*;
    use crate::{Look, Pointer};

    /// Invariant: A press just beside a small button still presses it, and one far from it does not.
    /// Regression: A small button takes presses only on its drawn box, so a finger a little off it misses.
    /// Owner: kit
    #[test]
    fn a_press_just_beside_a_small_button_still_presses_it() {
        let mut c = canvas(160);
        let out = frame(&mut c, &at_rest(None), |ui| {
            ui.button(100.0, 100.0, Some(80.0), 30.0, "Go", Look::Secondary, P(1));
        });
        // 30 tall: 9 above and 9 below take the press too.
        assert_eq!(hit_at(&out.hits, 140, 92).map(|h| h.press), Some(P(1)));
        assert_eq!(hit_at(&out.hits, 140, 138).map(|h| h.press), Some(P(1)));
        assert_eq!(hit_at(&out.hits, 140, 80).map(|h| h.press), None);
    }

    /// Invariant: Two buttons close together share the gap between them, and neither's reach covers the other's button.
    /// Regression: A grown touch target of one button swallows the press meant for the button next to it.
    /// Owner: kit
    #[test]
    fn two_buttons_close_together_share_the_gap_between_them() {
        let mut c = canvas(160);
        let out = frame(&mut c, &at_rest(None), |ui| {
            ui.button(100.0, 100.0, Some(80.0), 30.0, "Up", Look::Secondary, P(1));
            ui.button(
                100.0,
                138.0,
                Some(80.0),
                30.0,
                "Down",
                Look::Secondary,
                P(2),
            );
        });
        // The gap is 130 to 138: each takes it up to 134.
        assert_eq!(hit_at(&out.hits, 140, 133).map(|h| h.press), Some(P(1)));
        assert_eq!(hit_at(&out.hits, 140, 134).map(|h| h.press), Some(P(2)));
        // Neither reaches over the other's button.
        let up = out.hits.iter().find(|h| h.press == P(1)).map(|h| h.target);
        assert!(up.is_some_and(|t| t.bottom() <= 138));
    }

    /// Invariant: A finger on a region shows it is pressed, and a finger on a held secret leaves what it shows as drawn.
    /// Regression: A held secret is covered by its own pressed layer, so the code under the finger reads dimmer.
    /// Owner: kit
    #[test]
    fn a_pressed_region_shows_it_and_a_held_secret_reads_as_drawn() {
        let pointer = Pointer {
            at: Some(faraday_port::values::Point { x: 160, y: 120 }),
            down: true,
            focused: None,
        };
        let rest = {
            let mut c = canvas(160);
            frame(&mut c, &at_rest(None), |ui| ui.clear(page()));
            pixel(&c, 160, 120)
        };
        for (held, changed) in [(false, true), (true, false)] {
            let mut c = canvas(160);
            frame(&mut c, &pointer, |ui| {
                ui.clear(page());
                if held {
                    ui.hit_held(100.0, 100.0, 120.0, 40.0, P(1));
                } else {
                    ui.hit(100.0, 100.0, 120.0, 40.0, P(1));
                }
            });
            assert_eq!(pixel(&c, 160, 120) != rest, changed, "held {held}");
        }
    }
}
