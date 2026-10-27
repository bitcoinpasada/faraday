//! Material 3's tooltips (`docs/DESIGN.md` §4, Tooltip): a plain one, a
//! short label on the inverse surface, and a rich one, a line or more of
//! text on a raised surface.
//!
//! A screen marks what a tooltip is about with [`Ui::tip_anchor`]; the
//! app keeps a [`Tips`] that the pointer, the keyboard and the finger
//! move, and sets [`TipFrame::shown`] before a frame so that
//! [`Ui::draw_tip`], called once the screen is drawn, draws the tooltip
//! above everything else. Drawing reads what [`Tips`] decided; it
//! decides nothing.
//!
//! When it shows: the pointer resting on the anchor for
//! [`TIP_DELAY_MS`], or the keyboard on its owner for as long; a finger
//! held on it for [`TIP_LONG_PRESS_MS`], whose lift then presses nothing,
//! for a rich one only: a plain tooltip is for the pointer and the
//! keyboard, never for a finger (Material). When it goes: the pointer
//! leaving, the keyboard moving on, Escape, any press.

use faraday_draw::Rect;
use faraday_draw::tokens::{
    SHAPE_M, SHAPE_XS, TIP_DELAY_MS, TIP_GAP, TIP_LONG_PRESS_MS, TIP_PLAIN_MAX_W, TIP_PLAIN_MIN_H,
    TIP_PLAIN_PAD_X, TIP_PLAIN_PAD_Y, TIP_PLAIN_TEXT, TIP_RICH_LEVEL, TIP_RICH_MAX_W,
    TIP_RICH_PAD_X, TIP_RICH_PAD_Y, TIP_RICH_TEXT,
};

use crate::Ui;
use crate::pal::{Paint, Role};

/// A plain tooltip's container: Material's inverse surface.
pub const TIP_PLAIN_FILL: Paint = Paint::role(Role::InverseSurface);
/// A plain tooltip's text: Material's inverse on-surface.
pub const TIP_PLAIN_INK: Paint = Paint::role(Role::InverseOnSurface);
/// A rich tooltip's container: Material's surface container.
pub const TIP_RICH_FILL: Paint = Paint::role(Role::SurfaceContainer);
/// A rich tooltip's supporting text: Material's on-surface variant.
pub const TIP_RICH_INK: Paint = Paint::role(Role::OnSurfaceVariant);
/// A rich tooltip's edge where High contrast draws no shadow.
pub const TIP_RICH_EDGE: Paint = Paint::role(Role::Outline);

/// Which of Material's two tooltips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipKind {
    /// A few words naming a control, on the inverse surface.
    Plain,
    /// A sentence or more, on a raised surface.
    Rich,
}

/// What a tooltip is about, as a frame drew it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TipAnchor<A> {
    /// Where the pointer or a finger finds it, in pixels.
    pub at: Rect,
    /// The control it belongs to: the keyboard on it shows the tooltip.
    pub owner: A,
    /// What the tooltip says.
    pub text: &'static str,
    /// Plain or rich.
    pub kind: TipKind,
}

/// One frame's tooltips: the anchors drawn and the one shown.
#[derive(Debug, Clone)]
pub struct TipFrame<A> {
    /// The anchors drawn this frame, in drawing order.
    pub anchors: Vec<TipAnchor<A>>,
    /// The owner whose tooltip shows, as [`Tips::shown`] says.
    pub shown: Option<A>,
}

impl<A> Default for TipFrame<A> {
    fn default() -> Self {
        TipFrame {
            anchors: Vec::new(),
            shown: None,
        }
    }
}

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

/// Which tooltip shows, which is coming, and why: kept by the app, moved
/// by its events, read by drawing.
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

impl<A: Copy + PartialEq> Tips<A> {
    /// The owner whose tooltip shows.
    pub fn shown(&self) -> Option<A> {
        self.shown.map(|(a, _)| a)
    }

    /// What brought the showing tooltip up.
    pub fn shown_by(&self) -> Option<TipBy> {
        self.shown.map(|(_, by)| by)
    }

    /// Whether a tooltip is on its way: ticks move it.
    pub fn waiting(&self) -> bool {
        self.coming.is_some()
    }

    /// The anchor under (x, y), the topmost drawn.
    fn under(&self, x: i32, y: i32) -> Option<&TipAnchor<A>> {
        self.anchors.iter().rev().find(|t| t.at.contains(x, y))
    }

    /// Whether the last frame drew an anchor of `owner`.
    fn drawn(&self, owner: A) -> bool {
        self.anchors.iter().any(|t| t.owner == owner)
    }

    /// Hides the tooltip. Returns whether one showed.
    fn hide(&mut self) -> bool {
        self.shown.take().is_some()
    }

    /// Takes the anchors a frame drew; a tooltip, or one coming, whose
    /// anchor the frame no longer drew goes. Returns whether one that
    /// showed went.
    pub fn set_anchors(&mut self, anchors: Vec<TipAnchor<A>>) -> bool {
        self.anchors = anchors;
        if self.coming.is_some_and(|(a, _, _)| !self.drawn(a)) {
            self.coming = None;
        }
        match self.shown {
            Some((a, _)) if !self.drawn(a) => self.hide(),
            _ => false,
        }
    }

    /// The pointer is at `at` with no button down, or gone (`None`), at
    /// `now` ms. Returns whether what shows changed.
    pub fn hover(&mut self, at: Option<(i32, i32)>, now: u64) -> bool {
        let over = at.and_then(|(x, y)| self.under(x, y)).map(|t| t.owner);
        let mut changed = false;
        if self
            .shown
            .is_some_and(|(a, by)| by == TipBy::Hover && over != Some(a))
        {
            changed = self.hide();
        }
        match over {
            Some(a) if self.shown() != Some(a) => {
                if !self
                    .coming
                    .is_some_and(|(c, by, _)| c == a && by == TipBy::Hover)
                {
                    self.coming = Some((a, TipBy::Hover, now));
                }
            }
            Some(_) => {}
            None => {
                if self.coming.is_some_and(|(_, by, _)| by == TipBy::Hover) {
                    self.coming = None;
                }
            }
        }
        changed
    }

    /// The keyboard is on `focused`, at `now` ms. Returns whether what
    /// shows changed.
    pub fn focus(&mut self, focused: Option<A>, now: u64) -> bool {
        let on = focused.filter(|a| self.drawn(*a));
        let mut changed = false;
        if self
            .shown
            .is_some_and(|(a, by)| by == TipBy::Focus && on != Some(a))
        {
            changed = self.hide();
        }
        match on {
            Some(a) if self.shown() != Some(a) => {
                if !self
                    .coming
                    .is_some_and(|(c, by, _)| c == a && by == TipBy::Focus)
                {
                    self.coming = Some((a, TipBy::Focus, now));
                }
            }
            Some(_) => {}
            None => {
                if self.coming.is_some_and(|(_, by, _)| by == TipBy::Focus) {
                    self.coming = None;
                }
            }
        }
        changed
    }

    /// A finger or a button went down at (x, y), at `now` ms: any tooltip
    /// goes, and a rich one on the anchor there starts its long press (a
    /// plain one never shows to a finger). Returns whether what shows
    /// changed.
    pub fn press_down(&mut self, (x, y): (i32, i32), now: u64) -> bool {
        let changed = self.hide();
        self.coming = self
            .under(x, y)
            .filter(|t| t.kind == TipKind::Rich)
            .map(|t| (t.owner, TipBy::Touch, now));
        changed
    }

    /// The finger moved to (x, y): off the anchor, or into a scroll, the
    /// long press is not one.
    pub fn press_moved(&mut self, (x, y): (i32, i32), scrolling: bool) {
        if let Some((a, TipBy::Touch, _)) = self.coming
            && (scrolling || self.under(x, y).is_none_or(|t| t.owner != a))
        {
            self.coming = None;
        }
    }

    /// The finger lifted: a press too short to be a long press is a
    /// press. A rich tooltip the long press showed stays until the next
    /// press.
    pub fn press_up(&mut self) {
        if self.coming.is_some_and(|(_, by, _)| by == TipBy::Touch) {
            self.coming = None;
        }
    }

    /// A press of `owner`'s own button (Material: a rich tooltip may
    /// open on a click of its icon): its tooltip shows at once, and stays
    /// until the next press or Escape.
    pub fn show(&mut self, owner: A) {
        if self.drawn(owner) {
            self.coming = None;
            self.shown = Some((owner, TipBy::Press));
        }
    }

    /// Escape: the tooltip goes. Returns whether one showed.
    pub fn dismiss(&mut self) -> bool {
        self.coming = None;
        self.hide()
    }

    /// Time moved on to `now` ms: a tooltip whose wait is over shows. Returns whether what shows changed.
    pub fn tick(&mut self, now: u64) -> bool {
        let Some((a, by, since)) = self.coming else {
            return false;
        };
        let wait = if by == TipBy::Touch {
            TIP_LONG_PRESS_MS
        } else {
            TIP_DELAY_MS
        };
        if now < since + wait {
            return false;
        }
        self.coming = None;
        self.shown = Some((a, by));
        true
    }
}

impl<A: Copy + PartialEq> Ui<'_, A> {
    /// Marks the box at (x, y), w by h, as what a tooltip saying `text`
    /// is about, belonging to the control `owner`.
    pub fn tip_anchor(
        &mut self,
        (x, y, w, h): (f32, f32, f32, f32),
        owner: A,
        text: &'static str,
        kind: TipKind,
    ) {
        let at = self.rect(x, y, w, h).intersect(&self.c.clip());
        if at.w > 0 && at.h > 0 {
            self.tips.anchors.push(TipAnchor {
                at,
                owner,
                text,
                kind,
            });
        }
    }

    /// Marks the box at (x, y), w by h, as an icon-only control `owner`
    /// that a plain tooltip names: `label` is the action, a word or two,
    /// never an explanation.
    pub fn tip_plain(&mut self, r: (f32, f32, f32, f32), owner: A, label: &'static str) {
        self.tip_anchor(r, owner, label, TipKind::Plain);
    }

    /// [`Ui::hit`] at (x, y), w by h, for an icon-only control, with the
    /// plain tooltip `label` naming it over the same box.
    pub fn hit_tip(&mut self, (x, y, w, h): (f32, f32, f32, f32), action: A, label: &'static str) {
        self.hit(x, y, w, h, action);
        self.tip_plain((x, y, w, h), action, label);
    }

    /// Draws the tooltip [`TipFrame::shown`] names, if this frame drew
    /// its anchor: above it, or below where there is no room above, kept
    /// inside the canvas. Called once the screen is drawn, so it is over
    /// everything.
    pub fn draw_tip(&mut self) {
        let Some(owner) = self.tips.shown else {
            return;
        };
        let Some(t) = self.tips.anchors.iter().find(|t| t.owner == owner).copied() else {
            return;
        };
        let f = self.f;
        let (ax, ay) = (t.at.x as f32 / f - self.ox, t.at.y as f32 / f - self.oy);
        let (aw, ah) = (t.at.w as f32 / f, t.at.h as f32 / f);
        let (role, pad_x, pad_y, max_w) = match t.kind {
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
        let canvas_w = self.c.width() as f32 / f;
        let canvas_h = self.c.height() as f32 / f;
        let margin = faraday_draw::tokens::PAD;
        let max_w = max_w.min(canvas_w - 2.0 * margin);
        let text_w = self.measure_as(role, t.text);
        let inner = (max_w - 2.0 * pad_x).max(1.0);
        let lines = self.wrap_lines_as(role, t.text, inner);
        let w = if lines == 1 {
            text_w + 2.0 * pad_x
        } else {
            max_w
        };
        let lh = self.line_as(role);
        let h = lines as f32 * lh + 2.0 * pad_y;
        let h = if t.kind == TipKind::Plain {
            h.max(TIP_PLAIN_MIN_H)
        } else {
            h
        };
        // A plain tooltip is centred on what it is about, a rich one
        // starts where it starts.
        let x = match t.kind {
            TipKind::Plain => ax + aw / 2.0 - w / 2.0,
            TipKind::Rich => ax,
        };
        let x = x.clamp(
            margin - self.ox,
            (canvas_w - margin - w - self.ox).max(margin - self.ox),
        );
        let above = ay - TIP_GAP - h;
        let y = if above >= -self.oy || ay + ah + TIP_GAP + h > canvas_h - self.oy {
            above.max(-self.oy)
        } else {
            ay + ah + TIP_GAP
        };
        match t.kind {
            TipKind::Plain => {
                self.fill(x, y, w, h, SHAPE_XS, TIP_PLAIN_FILL);
                let ty = y + (h - lines as f32 * lh) / 2.0;
                let ty = ty + (lh - self.line(role.size, role.face.into())) / 2.0;
                self.wrap_as((x + pad_x, ty, inner), role, TIP_PLAIN_INK, t.text);
            }
            TipKind::Rich => {
                self.shadow(x, y, w, h, SHAPE_M, TIP_RICH_LEVEL);
                self.fill(x, y, w, h, SHAPE_M, TIP_RICH_FILL);
                if self.theme.is_high_contrast() {
                    self.stroke(x, y, w, h, SHAPE_M, TIP_RICH_EDGE);
                }
                let ty = y + pad_y + (lh - self.line(role.size, role.face.into())) / 2.0;
                self.wrap_as((x + pad_x, ty, inner), role, TIP_RICH_INK, t.text);
            }
        }
    }
}
