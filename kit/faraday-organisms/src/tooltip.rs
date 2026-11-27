//! The tooltip organism (09 §3.3): Material 3's tooltips (`docs/DESIGN.md` §4, Tooltip), a plain
//! one, a short label on the inverse surface, and a rich one, a line or more of text on a raised
//! surface.
//!
//! A control marks what a tooltip is about with [`Ui::tip_plain`] and the frame hands the anchors
//! out (`Ui::take_tips`); the app keeps a [`Tips`] that the pointer, the keyboard and the finger
//! move, and, once the screen is drawn, draws [`Tips::model`] with [`draw`], over everything
//! else. Drawing reads what [`Tips`] decided; it decides nothing.
//!
//! When it shows: the pointer resting on the anchor for [`TIP_DELAY_MS`], or the keyboard on its
//! owner for as long; a finger held on it for [`TIP_LONG_PRESS_MS`], whose lift then presses
//! nothing, for a rich one only: a plain tooltip is for the pointer and the keyboard, never for
//! a finger (Material). When it goes: the pointer leaving, the keyboard moving on, Escape, any
//! press.

pub use faraday_atoms::TipKind;
use faraday_atoms::{Pressed, Shown, TipAnchor, Ui};
use faraday_text::Text;
use faraday_tokens::dims::{
    PAD, SHAPE_M, SHAPE_XS, TIP_DELAY_MS, TIP_GAP, TIP_LONG_PRESS_MS, TIP_PLAIN_MAX_W,
    TIP_PLAIN_MIN_H, TIP_PLAIN_PAD_X, TIP_PLAIN_PAD_Y, TIP_PLAIN_TEXT, TIP_RICH_LEVEL,
    TIP_RICH_MAX_W, TIP_RICH_PAD_X, TIP_RICH_PAD_Y, TIP_RICH_TEXT,
};
use faraday_tokens::palette::{Paint, Role};

/// A plain tooltip's container: Material's inverse surface.
const TIP_PLAIN_FILL: Paint = Paint::role(Role::InverseSurface);
/// A plain tooltip's text: Material's inverse on-surface.
const TIP_PLAIN_INK: Paint = Paint::role(Role::InverseOnSurface);
/// A rich tooltip's container: Material's surface container.
const TIP_RICH_FILL: Paint = Paint::role(Role::SurfaceContainer);
/// A rich tooltip's supporting text: Material's on-surface variant.
const TIP_RICH_INK: Paint = Paint::role(Role::OnSurfaceVariant);
/// A rich tooltip's edge where High contrast draws no shadow.
const TIP_RICH_EDGE: Paint = Paint::role(Role::Outline);

/// What brought a tooltip up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipBy {
    /// The pointer resting on the anchor.
    Hover,
    /// The keyboard on the anchor's owner.
    Focus,
    /// A finger held on the anchor.
    Touch,
    /// A press of the anchor's own button.
    Press,
}

/// The tooltip to draw: its words, which kind it is and the box of what it is about, in pixels.
pub struct TipModel<'s> {
    /// What it says.
    pub text: Shown<'s>,
    /// Plain or rich.
    pub kind: TipKind,
    /// The anchor's box in pixels: x, y, width, height.
    pub at: (i32, i32, i32, i32),
}

/// Which tooltip shows, which is coming, and why: kept by the app, moved by its events, read by
/// drawing.
#[derive(Debug, Clone)]
pub struct Tips<A> {
    /// The anchors the last frame drew.
    anchors: Vec<TipAnchor<A>>,
    /// A tooltip on its way: whose, why, and since when (ms).
    coming: Option<(A, TipBy, u64)>,
    /// The tooltip showing, and why.
    shown: Option<(A, TipBy)>,
}

impl<A> Default for Tips<A> {
    fn default() -> Self {
        Tips {
            anchors: Vec::new(),
            coming: None,
            shown: None,
        }
    }
}

impl<A: Pressed> Tips<A> {
    /// The owner whose tooltip shows.
    pub fn shown(&self) -> Option<A> {
        self.shown.as_ref().map(|(a, _)| a.clone())
    }

    /// What brought the showing tooltip up.
    pub fn shown_by(&self) -> Option<TipBy> {
        self.shown.as_ref().map(|(_, by)| *by)
    }

    /// Whether a tooltip is on its way: ticks move it.
    pub fn waiting(&self) -> bool {
        self.coming.is_some()
    }

    /// The anchor under (x, y), the topmost drawn.
    fn under(&self, x: i32, y: i32) -> Option<&TipAnchor<A>> {
        self.anchors.iter().rev().find(|t| t.rect.contains(x, y))
    }

    /// Whether the last frame drew an anchor of `owner`.
    fn drawn(&self, owner: &A) -> bool {
        self.anchors.iter().any(|t| &t.press == owner)
    }

    /// Hides the tooltip. Returns whether one showed.
    fn hide(&mut self) -> bool {
        self.shown.take().is_some()
    }

    /// Takes the anchors a frame drew; a tooltip, or one coming, whose anchor the frame no
    /// longer drew goes. Returns whether one that showed went.
    pub fn set_anchors(&mut self, anchors: Vec<TipAnchor<A>>) -> bool {
        self.anchors = anchors;
        if self.coming.as_ref().is_some_and(|(a, _, _)| !self.drawn(a)) {
            self.coming = None;
        }
        match &self.shown {
            Some((a, _)) if !self.drawn(a) => self.hide(),
            _ => false,
        }
    }

    /// Starts waiting for `a`'s tooltip for `by` unless it already waits for it.
    fn expect(&mut self, a: A, by: TipBy, now: u64) {
        if self.shown().as_ref() != Some(&a)
            && !self
                .coming
                .as_ref()
                .is_some_and(|(c, b, _)| *c == a && *b == by)
        {
            self.coming = Some((a, by, now));
        }
    }

    /// Stops waiting for a tooltip `by` brought.
    fn stop(&mut self, by: TipBy) {
        if self.coming.as_ref().is_some_and(|(_, b, _)| *b == by) {
            self.coming = None;
        }
    }

    /// The pointer is at `at` with no button down, or gone (`None`), at `now` ms. Returns
    /// whether what shows changed.
    pub fn hover(&mut self, at: Option<(i32, i32)>, now: u64) -> bool {
        let over = at
            .and_then(|(x, y)| self.under(x, y))
            .map(|t| t.press.clone());
        let mut changed = false;
        if self
            .shown
            .as_ref()
            .is_some_and(|(a, by)| *by == TipBy::Hover && over.as_ref() != Some(a))
        {
            changed = self.hide();
        }
        match over {
            Some(a) => self.expect(a, TipBy::Hover, now),
            None => self.stop(TipBy::Hover),
        }
        changed
    }

    /// The keyboard is on `focused`, at `now` ms. Returns whether what shows changed.
    pub fn focus(&mut self, focused: Option<A>, now: u64) -> bool {
        let on = focused.filter(|a| self.drawn(a));
        let mut changed = false;
        if self
            .shown
            .as_ref()
            .is_some_and(|(a, by)| *by == TipBy::Focus && on.as_ref() != Some(a))
        {
            changed = self.hide();
        }
        match on {
            Some(a) => self.expect(a, TipBy::Focus, now),
            None => self.stop(TipBy::Focus),
        }
        changed
    }

    /// A finger or a button went down at (x, y), at `now` ms: any tooltip goes, and a rich one on
    /// the anchor there starts its long press (a plain one never shows to a finger). Returns
    /// whether what shows changed.
    pub fn press_down(&mut self, (x, y): (i32, i32), now: u64) -> bool {
        let changed = self.hide();
        self.coming = self
            .under(x, y)
            .filter(|t| t.kind == TipKind::Rich)
            .map(|t| (t.press.clone(), TipBy::Touch, now));
        changed
    }

    /// The finger moved to (x, y): off the anchor, or into a scroll, the long press is not one.
    pub fn press_moved(&mut self, (x, y): (i32, i32), scrolling: bool) {
        let off = match &self.coming {
            Some((a, TipBy::Touch, _)) => {
                scrolling || self.under(x, y).is_none_or(|t| &t.press != a)
            }
            _ => false,
        };
        if off {
            self.coming = None;
        }
    }

    /// The finger lifted: a press too short to be a long press is a press. A rich tooltip the
    /// long press showed stays until the next press.
    pub fn press_up(&mut self) {
        self.stop(TipBy::Touch);
    }

    /// A press of `owner`'s own button (Material: a rich tooltip may open on a click of its
    /// icon): its tooltip shows at once, and stays until the next press or Escape.
    pub fn show(&mut self, owner: A) {
        if self.drawn(&owner) {
            self.coming = None;
            self.shown = Some((owner, TipBy::Press));
        }
    }

    /// Escape: the tooltip goes. Returns whether one showed.
    pub fn dismiss(&mut self) -> bool {
        self.coming = None;
        self.hide()
    }

    /// Time moved on to `now` ms: a tooltip whose wait is over shows. Returns whether what shows
    /// changed.
    pub fn tick(&mut self, now: u64) -> bool {
        let Some((a, by, since)) = self.coming.take() else {
            return false;
        };
        let wait = if by == TipBy::Touch {
            TIP_LONG_PRESS_MS
        } else {
            TIP_DELAY_MS
        };
        if now < since + wait {
            self.coming = Some((a, by, since));
            return false;
        }
        self.shown = Some((a, by));
        true
    }

    /// The tooltip that shows, if the last frame drew its anchor: what [`draw`] takes. Its words
    /// are the anchor's own.
    pub fn model(&self) -> Option<TipModel<'static>> {
        let (owner, _) = self.shown.as_ref()?;
        let t = self.anchors.iter().find(|t| &t.press == owner)?;
        let r = t.rect;
        Some(TipModel {
            text: Shown::made(Text::from_utf8_lossy(t.text.as_bytes())),
            kind: t.kind,
            at: (r.x, r.y, r.w, r.h),
        })
    }
}

/// Draws the tooltip over its anchor's box `m.at`: above it, or below where there is no room
/// above, kept inside the canvas. Called once the screen is drawn, so it is over everything.
pub fn draw<A: Pressed>(m: &TipModel<'_>, ui: &mut Ui<'_, A>) {
    let f = ui.f;
    let (ax, ay) = (m.at.0 as f32 / f - ui.ox, m.at.1 as f32 / f - ui.oy);
    let (aw, ah) = (m.at.2 as f32 / f, m.at.3 as f32 / f);
    let (role, pad_x, pad_y, max_w) = match m.kind {
        TipKind::Plain => (
            TIP_PLAIN_TEXT,
            TIP_PLAIN_PAD_X,
            TIP_PLAIN_PAD_Y,
            TIP_PLAIN_MAX_W,
        ),
        TipKind::Rich => (
            TIP_RICH_TEXT,
            TIP_RICH_PAD_X,
            TIP_RICH_PAD_Y,
            TIP_RICH_MAX_W,
        ),
    };
    let canvas_w = ui.c.width() as f32 / f;
    let canvas_h = ui.c.height() as f32 / f;
    let margin = PAD;
    let max_w = max_w.min(canvas_w - 2.0 * margin);
    let text = m.text.as_str();
    let text_w = ui.measure_as(role, text);
    let inner = (max_w - 2.0 * pad_x).max(1.0);
    let lines = ui.wrap_lines_as(role, text, inner);
    let w = if lines == 1 {
        text_w + 2.0 * pad_x
    } else {
        max_w
    };
    let lh = ui.line_as(role);
    let h = lines as f32 * lh + 2.0 * pad_y;
    let h = if m.kind == TipKind::Plain {
        h.max(TIP_PLAIN_MIN_H)
    } else {
        h
    };
    // A plain tooltip is centred on what it is about, a rich one starts where it starts.
    let x = match m.kind {
        TipKind::Plain => ax + aw / 2.0 - w / 2.0,
        TipKind::Rich => ax,
    };
    let x = x.clamp(
        margin - ui.ox,
        (canvas_w - margin - w - ui.ox).max(margin - ui.ox),
    );
    let above = ay - TIP_GAP - h;
    let y = if above >= -ui.oy || ay + ah + TIP_GAP + h > canvas_h - ui.oy {
        above.max(-ui.oy)
    } else {
        ay + ah + TIP_GAP
    };
    ui.showing(&m.text, |ui| match m.kind {
        TipKind::Plain => {
            ui.fill(x, y, w, h, SHAPE_XS, TIP_PLAIN_FILL);
            let ty = y + (h - lines as f32 * lh) / 2.0;
            let ty = ty + (lh - ui.line(role.size, role.face.into())) / 2.0;
            ui.wrap_as((x + pad_x, ty, inner), role, TIP_PLAIN_INK, text);
        }
        TipKind::Rich => {
            ui.shadow(x, y, w, h, SHAPE_M, TIP_RICH_LEVEL);
            ui.fill(x, y, w, h, SHAPE_M, TIP_RICH_FILL);
            if ui.theme.is_high_contrast() {
                ui.stroke(x, y, w, h, SHAPE_M, TIP_RICH_EDGE);
            }
            let ty = y + pad_y + (lh - ui.line(role.size, role.face.into())) / 2.0;
            ui.wrap_as((x + pad_x, ty, inner), role, TIP_RICH_INK, text);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::P;
    use faraday_draw::Rect;

    fn spot(press: P, kind: TipKind) -> TipAnchor<P> {
        TipAnchor {
            rect: Rect::new(10, 10, 40, 40),
            press,
            text: "Help".to_string(),
            kind,
        }
    }

    /// Invariant: A plain tooltip shows once the pointer has rested on its anchor for the delay and goes when the pointer leaves, and a finger never brings it up.
    /// Regression: A plain tooltip pops up the moment the pointer arrives or on a touch, or stays after the pointer has left.
    /// Owner: kit
    #[test]
    fn a_plain_tooltip_is_for_the_pointer_and_waits_for_its_delay() {
        let mut tips = Tips::default();
        tips.set_anchors(vec![spot(P(1), TipKind::Plain)]);
        tips.hover(Some((20, 20)), 0);
        assert!(!tips.tick(1));
        assert!(tips.tick(10_000));
        assert_eq!(tips.shown(), Some(P(1)));
        assert!(tips.hover(None, 10_001));
        assert_eq!(tips.shown(), None);
        tips.press_down((20, 20), 0);
        assert!(!tips.waiting());
    }

    /// Invariant: A rich tooltip shows to a finger held on its anchor for the long press, and a press that lifts sooner shows nothing.
    /// Regression: The long press never fires, or fires after the finger lifted.
    /// Owner: kit
    #[test]
    fn a_rich_tooltip_shows_to_a_held_finger_only() {
        let mut tips = Tips::default();
        tips.set_anchors(vec![spot(P(2), TipKind::Rich)]);
        tips.press_down((20, 20), 0);
        tips.press_up();
        assert!(!tips.tick(10_000));
        tips.press_down((20, 20), 0);
        assert!(tips.tick(10_000));
        assert_eq!(tips.shown_by(), Some(TipBy::Touch));
    }
}
