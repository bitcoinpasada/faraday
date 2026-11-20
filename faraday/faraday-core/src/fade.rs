//! A change of screen or sheet cross-fading in (`docs/MOTION.md` §3.5):
//! when a frame shows a change, the old frame kept to fade from and the
//! new one kept aside; the steps after, which blend the two without
//! drawing either again; and how far apart this machine shows a fade's
//! steps, which is timed and reported, never acted on.

mod bands;

use faraday_draw::{Canvas, Rect};
use faraday_shell_api::Command;

use crate::moved::{Last, Look};
use crate::ui::{self, Role, Theme};
use crate::{Faraday, ScreenKey, Sheet, motion};

/// What a frame drew that a change is known by: the screen and sheet, the
/// guide's pane and tab, the theme, and the step open in a flow on the
/// container.
type Drawn = (ScreenKey, (bool, usize), Theme, Option<u8>);

/// A change of screen, sheet or theme fading in, with the frosted page under an
/// open sheet and the screen the last frame drew. Its fields are private.
#[derive(Default)]
pub(crate) struct Fade {
    /// The frame before a change of screen or sheet, kept while the new
    /// one fades in over it, and when the fade's clock started: `None`
    /// for the frame that showed the change, whose cost (frosting the
    /// page under a sheet is the slowest frame there is) is not taken
    /// out of the fade.
    from: Option<(Vec<u8>, Option<u64>)>,
    /// The new frame of the fade under way, as last drawn in full, kept
    /// aside so that a step of the fade nothing else asked a frame for
    /// blends the two kept frames instead of drawing the screen again
    /// (`Faraday::fade_step`). Drawn again, and kept again, by every
    /// frame drawn in full while the fade runs; empty with no fade.
    after: Vec<u8>,
    /// Where `from`'s frame and `after` differ, but for the ring
    /// `edge_drawn_over` wide round the canvas: the rectangles a step
    /// blends (`Canvas::blend_noting`). Kept in step with the two: noted
    /// again by every frame drawn in full while the fade runs, which
    /// keeps `after` again, and emptied with them when the fade ends.
    /// Outside it the canvas already shows both frames as they agree.
    bands: Vec<Rect>,
    /// How wide a ring round the canvas whoever holds the app draws over
    /// every frame (Faraday Demo's focus edge), left out of `bands`.
    edge_drawn_over: i32,
    /// Where the canvas may differ from the frame the shell last took
    /// (`Faraday::take_changed`): a fade's steps add their bands; `None`,
    /// anywhere, after a frame drawn in full or a new display.
    changed: Option<Vec<Rect>>,
    /// The fade asked for its next step and nothing has asked for a
    /// frame drawn in full since (`dirty` is not set).
    due: bool,
    /// The tick of the last fade step, while the fade's steps follow one
    /// another with no frame drawn in full between: the gap to the next
    /// is how fast this machine shows a fade.
    step_at: Option<u64>,
    /// The last few gaps between the steps of fades, in milliseconds,
    /// each with the tick it was taken at, oldest first: how fast this
    /// machine shows a fade, for the timings. Only steps that blend the
    /// two kept frames are timed, so what a screen costs to draw, and the
    /// frame a change is drawn on, never count, and gaps older than
    /// `motion::FADE_GAPS_FRESH_MS` no longer count. Emptied with a new
    /// display.
    gaps: Vec<(u64, u16)>,
    /// What fades have done since a shell's timings last took it
    /// (`Faraday::take_fade_counts`).
    counts: FadeCounts,
    /// The frosted page under the open sheet, and the screen, sheet and
    /// theme it was made for.
    frost: Option<((ScreenKey, Theme), Vec<u8>)>,
    /// The screen and sheet the last frame drew, whether the guide's pane
    /// stood beside the page and the tab on show in the sheet or the
    /// pane, and the theme, so that a change is known at the next.
    last_drawn: Option<Drawn>,
    /// Whether the fade under way is a change of theme, which takes
    /// longer and is eased; set when a fade starts.
    theme_fade: bool,
    /// Whether the fade under way is a change of a flow's step on the
    /// container: the card's place fades through the page's surface, the
    /// rest cross-fades (`docs/FLOW-CONTAINER.md` §3); set when a fade
    /// starts.
    step_fade: bool,
    /// Where the last frame drew the flow's card, in pixels: what a step's
    /// fade goes through the surface in. A rectangle, no image.
    card: Option<Rect>,
    /// An image of the page's surface, as large as the frames, that a
    /// step's card goes out to and comes in from: made when the new frame
    /// of a step's fade is first kept, empty with no step fade. One more
    /// frame of memory while one runs; it holds nothing drawn.
    surface: Vec<u8>,
    /// How a whole frame is painted across the cores (`bands.rs`).
    cores: bands::Bands,
    /// The last frame drawn in full was drawn while a secret was being
    /// typed or shown (`Faraday::secret_typed`, `Faraday::secret_shown`): a change from it is drawn at once,
    /// so no image of it is kept to fade from (`docs/SECRET-LEAKS.md` §5).
    secret_drawn: bool,
    /// What drew each image kept here, for the tours' check of a secret
    /// left in one (`crate::leaks`).
    #[cfg(feature = "testkit")]
    marks: crate::leaks::Marks,
}

/// Holds nothing a lock wipes: the fade's timing.
impl crate::at_lock::Public for Fade {}

impl Fade {
    /// How far through the fade under way the blend is, 0 to 1, and
    /// whether the fade is over: a change of theme is eased over
    /// `motion::THEME_FADE_MS`, a change of screen linear over
    /// `motion::FADE_MS`.
    fn blend(&self, at: Option<u64>, now_ms: u64) -> (f32, bool) {
        let ms = if self.theme_fade {
            motion::THEME_FADE_MS
        } else if self.step_fade {
            motion::STEP_FADE_MS
        } else {
            motion::FADE_MS
        };
        let t = motion::progress(at, now_ms, ms);
        let shown = if self.theme_fade {
            motion::ease_move(t)
        } else {
            t
        };
        (shown, t >= 1.0)
    }

    /// The fade under way ends where it is: the next frame is the
    /// screen at rest.
    pub(crate) fn end(&mut self) {
        self.from = None;
        self.after = Vec::new();
        self.bands.clear();
        self.surface = Vec::new();
        self.step_fade = false;
    }

    /// The card's place inside `canvas`, when a step's fade goes through
    /// the surface there.
    fn card_in(&self, canvas: &Canvas) -> Option<Rect> {
        let whole = Rect::new(0, 0, canvas.width(), canvas.height());
        self.card
            .map(|c| c.intersect(&whole))
            .filter(|c| self.step_fade && !c.is_empty())
    }

    /// Costs a pass over the card's place. Step `t` of a step's fade, the
    /// card's place only: the old card out to the surface over the first
    /// part, the new one in from it over the rest, every pixel written
    /// (`motion::step_fade`). Returns how many bands it was split in.
    fn step_card(&self, canvas: &mut Canvas, t: f32) -> usize {
        let (Some((before, _)), Some(card)) = (self.from.as_ref(), self.card_in(canvas)) else {
            return 0;
        };
        if self.surface.is_empty() {
            return 0;
        }
        let (going, e) = motion::step_fade(t);
        let pair = if going {
            (before.as_slice(), self.surface.as_slice())
        } else {
            (self.surface.as_slice(), self.after.as_slice())
        };
        self.cores.blend_all_within(canvas, pair, e, &[card])
    }

    /// Costs a pass over the bands. A later step of a step's fade: the
    /// bands outside the card's place cross-fade linearly, the card's
    /// place goes through the surface. Returns how many bands it was
    /// split in, the larger of the two passes.
    fn step_blend(&self, canvas: &mut Canvas, t: f32) -> usize {
        let Some((before, _)) = self.from.as_ref() else {
            return 0;
        };
        let mut outside = Vec::new();
        match self.card_in(canvas) {
            Some(card) => bands::outside(&self.bands, card, &mut outside),
            None => outside.extend_from_slice(&self.bands),
        }
        let n = self
            .cores
            .blend_within(canvas, before, &self.after, t, &outside);
        n.max(self.step_card(canvas, t))
    }

    /// Where the frame being drawn put the flow's card, in pixels, or
    /// that it put none.
    pub(crate) fn set_card(&mut self, card: Option<Rect>) {
        self.card = card;
    }

    /// The next frame is the screen at rest, not part of the way from
    /// the one before.
    pub(crate) fn restart(&mut self) {
        self.end();
        self.last_drawn = None;
    }

    /// The display changed: nothing kept is of this size. A fade's step
    /// costs a blend of the whole canvas, and what it cost at another
    /// size says nothing of this one.
    pub(crate) fn display_changed(&mut self) {
        self.frost = None;
        self.restart();
        self.changed = None;
        self.gaps.clear();
    }

    /// The frosted page kept for `key`, taken out to be used by the
    /// frame; `None` when it was made for another screen, sheet or
    /// theme.
    pub(crate) fn take_frost(&mut self, key: &(ScreenKey, Theme)) -> Option<Vec<u8>> {
        let page = self
            .frost
            .take()
            .filter(|(k, _)| k == key)
            .map(|(_, page)| page);
        #[cfg(feature = "testkit")]
        {
            self.marks.frost_reused = page.is_some();
        }
        page
    }

    /// Keeps the frosted page the frame made for `key`.
    pub(crate) fn keep_frost(&mut self, key: (ScreenKey, Theme), page: Option<Vec<u8>>) {
        self.frost = page.map(|page| (key, page));
    }

    /// A tick: a fade whose clock has not started starts it, a frame
    /// back, so this frame is the first step of it rather than the old
    /// screen again.
    pub(crate) fn tick_clock(&mut self, now_ms: u64) {
        if let Some((_, at @ None)) = self.from.as_mut() {
            *at = Some(now_ms.saturating_sub(motion::FADE_FIRST_STEP_MS));
        }
    }

    /// A tick: a fade under way asks for its next step; returns whether
    /// it does.
    pub(crate) fn tick_due(&mut self) -> bool {
        if self.from.is_some() {
            self.due = true;
        }
        self.from.is_some()
    }

    /// Whether a fade is under way: the canvas shows a blend, not the
    /// frame last drawn.
    pub(crate) fn under_way(&self) -> bool {
        self.from.is_some() || self.due
    }

    /// Whether the next frame is a fade's step.
    pub(crate) fn is_due(&self) -> bool {
        self.due
    }

    /// The gaps between fade steps recent enough to count, oldest first.
    fn fresh_gaps(&self, now_ms: u64) -> Vec<u16> {
        self.gaps
            .iter()
            .filter(|(at, _)| now_ms.saturating_sub(*at) <= motion::FADE_GAPS_FRESH_MS)
            .map(|(_, gap)| *gap)
            .collect()
    }

    /// A step of the current fade came `gap` ms after the one before it.
    /// A change of theme redraws every pixel and its first frames are
    /// slow on a small machine; that says nothing about fades of
    /// screens, so its steps are not timed.
    fn step_of_fade(&mut self, now_ms: u64, gap: u16) {
        if !self.theme_fade {
            self.step_timed(now_ms, gap);
        }
    }

    /// A step of a fade came `gap` ms after the one before it, at
    /// `now_ms`: kept with the last few.
    fn step_timed(&mut self, now_ms: u64, gap: u16) {
        if self.gaps.len() == motion::FADE_GAPS_KEPT {
            self.gaps.remove(0);
        }
        self.gaps.push((now_ms, gap));
    }
}

/// What fades have done since a shell last took them, for its timings
/// (`Faraday::take_fade_counts`): how often a change faded, and how
/// fast this machine has lately shown a fade's steps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FadeCounts {
    /// Changes of screen or sheet that faded in.
    pub started: u32,
    /// Changes drawn at once because reduce motion is on.
    pub reduced: u32,
    /// Steps of a fade drawn by blending the two kept frames.
    pub steps: u32,
    /// How many timed gaps between steps are recent enough to count.
    pub gaps_known: u8,
    /// The median of those gaps, milliseconds, when there are any.
    pub median_gap_ms: Option<u16>,
    /// The longest of those gaps, milliseconds, when there are any.
    pub worst_gap_ms: Option<u16>,
    /// The most bands a blend of a fade's step was split in, 0 when none
    /// ran (`bands.rs`).
    pub bands: u8,
}

impl Faraday {
    /// Before a frame is drawn in full on `canvas`, which still holds the
    /// frame before it: a change of screen or sheet, of the guide
    /// pane's presence, or of the guide tab, starts a fade from that frame. The first frame ever,
    /// and the first after a resize, have nothing to fade from, nor has a
    /// frame drawn while a secret was typed or shown. Costs a
    /// copy of the whole canvas when a fade starts.
    pub(crate) fn fade_begin(&mut self, canvas: &mut Canvas) {
        // The pane opening, closing or changing its tab is a change as a
        // sheet's is (`docs/MOTION.md` §3.5, Material's side sheets).
        let pane = self.guide_pane_shown();
        let page = if pane || self.sheet == Some(Sheet::Guide) {
            self.guide.state().tab()
        } else {
            0
        };
        let page = (pane, page);
        let theme = self.settings.theme();
        // The step open in a flow on the container: a change of it fades
        // through (`docs/FLOW-CONTAINER.md` §3).
        let step = self
            .steps_on_show()
            .and_then(|(flow, steps)| flow.contained().and(steps.open));
        let drawn_key = ((self.screen, self.sheet), page, theme, step);
        // Drawn in full: what asked for this frame is drawn as it is now,
        // the gap across it is not a fade's step, and what it changed is
        // not known.
        self.fade.due = false;
        self.fade.step_at = None;
        self.fade.changed = None;
        if self.fade.last_drawn == Some(drawn_key) {
            return;
        }
        // The frame before showed a secret, typed or shown: it is
        // not kept, and any fade under way ends where it is.
        if self.fade.secret_drawn {
            self.fade.end();
            self.fade.last_drawn = Some(drawn_key);
            return;
        }
        // Only the step differs, both are steps of a flow on the container.
        let step_change = self.fade.last_drawn.is_some_and(|(k, p, t, s)| {
            (k, p, t) == (drawn_key.0, drawn_key.1, drawn_key.2)
                && s.is_some()
                && step.is_some()
                && s != step
        });
        // The step now shows or takes a secret: no frame is kept to fade
        // from or to, as for a frame before that did
        // (`docs/MOTION.md` §3.5).
        if step_change && self.step_secret() {
            self.fade.end();
            self.fade.last_drawn = Some(drawn_key);
            return;
        }
        let own_motion = false;
        let fades = self.fade.last_drawn.is_some() && self.fades() && !own_motion;
        self.fade.theme_fade = self.fade.last_drawn.is_some_and(|(_, _, t, _)| t != theme);
        self.fade.step_fade = step_change && fades;
        // A fade begun over one under way goes through a new surface.
        self.fade.surface = Vec::new();
        if self.fade.last_drawn.is_some() && !own_motion {
            let n = &mut self.fade.counts;
            match (fades, self.settings.reduce_motion()) {
                (true, _) => n.started += 1,
                (false, _) => n.reduced += 1,
            }
        }
        // This frame is all the old one; the fade's clock starts at the
        // next tick (`Event::Tick`), however long this frame takes.
        self.fade.from = match (fades, self.fade.from.take()) {
            (false, _) => None,
            (true, Some((mut before, _))) => {
                canvas.snapshot_into(&mut before);
                Some((before, None))
            }
            (true, None) => Some((canvas.snapshot(), None)),
        };
        self.fade.last_drawn = Some(drawn_key);
        // The frame kept is the one just before this, as it was drawn.
        #[cfg(feature = "testkit")]
        {
            self.fade.marks.from = self.fade.marks.frame.clone();
        }
    }

    /// After a frame is drawn: notes what it showed, `forms` the planted
    /// forms found in its text, and what each image kept was made of
    /// (`crate::leaks`). Nothing is drawn or kept.
    #[cfg(feature = "testkit")]
    pub(crate) fn fade_marks(&mut self, only_moved: bool, forms: crate::leaks::Forms) {
        use crate::leaks::{Seen, take_page};
        let before = self.fade.marks.frame.take();
        // A frame that moved rows drew only the rows that came in: the
        // rest is what the frame before showed.
        let forms = if forms.is_empty() && only_moved {
            before.map(|b| b.forms).unwrap_or_default()
        } else {
            forms
        };
        let frame = Seen {
            // not shown: the tours' check names the screen in its failure
            screen: format!("{:?}", (self.screen, self.sheet)),
            forms,
        };
        match self.fade.frost.as_ref() {
            None => self.fade.marks.frost = None,
            Some(((key, _), _)) if !self.fade.marks.frost_reused => {
                let page = take_page().unwrap_or_else(|| frame.forms.clone());
                self.fade.marks.frost = Some(Seen {
                    // not shown: the tours' check names the screen in its failure
                    screen: format!("{key:?}"),
                    forms: page,
                });
            }
            Some(_) => {}
        }
        self.fade.marks.after = Some(frame.clone());
        self.fade.marks.frame = Some(frame);
    }

    /// The planted forms the last frame drawn showed in its text.
    #[cfg(feature = "testkit")]
    pub fn shown_forms(&self) -> crate::leaks::Forms {
        self.fade
            .marks
            .frame
            .as_ref()
            .map(|f| f.forms.clone())
            .unwrap_or_default()
    }

    /// The images the core keeps across frames now, each with the screen
    /// it was made from and the planted forms that screen showed, for the
    /// tours' check (`docs/SECRET-LEAKS.md` §5). The frosted copy of the
    /// page under a sheet, and the two frames of a fade under way.
    #[cfg(feature = "testkit")]
    pub fn kept_images(&self) -> Vec<crate::leaks::KeptImage> {
        use crate::leaks::KeptImage;
        let mut out = Vec::new();
        let mut add = |what, seen: &Option<crate::leaks::Seen>| {
            if let Some(s) = seen {
                out.push(KeptImage {
                    what,
                    screen: s.screen.clone(),
                    forms: s.forms.clone(),
                });
            }
        };
        if self.fade.frost.is_some() {
            add("frost", &self.fade.marks.frost);
        }
        if self.fade.from.is_some() {
            // not shown: the tours' check names the kept image in its failure
            add("fade from", &self.fade.marks.from);
        }
        if !self.fade.after.is_empty() {
            // not shown: the tours' check names the kept image in its failure
            add("fade after", &self.fade.marks.after);
        }
        out
    }

    /// Whether the state a step change opens shows or takes a secret: what
    /// the next frame draws as a secret frame does (`Self::secret_typed`,
    /// `Self::secret_shown`, and dice rolls on Create a vault).
    fn step_secret(&self) -> bool {
        self.secret_typed()
            || self.secret_shown()
            || self
                .vaults
                .dice
                .open()
                .is_some_and(|d| !d.1.text.is_empty())
    }

    /// Whether the frame shows a file's text a secret may be in: the vault
    /// page's sign list with a file's text shown behind its eye. Reads
    /// state only.
    fn secret_shown(&self) -> bool {
        self.screen == crate::Screen::VaultContents
            && self.vaults.signing
            && !self.vaults.sign_shown.is_empty()
    }

    /// After a frame is drawn in full on `canvas`: while a fade is under
    /// way, the new frame is kept aside and the canvas shows it part of
    /// the way from the old, and the frame asks for the next step, so the
    /// shell keeps drawing from the first one; whether a secret was typed
    /// or shown is noted for the next change. Costs a blend of the whole
    /// canvas while a fade is under way.
    pub(crate) fn fade_end(&mut self, canvas: &mut Canvas) {
        // The frame is painted before it is kept aside or shown.
        self.fade.cores.paint(canvas);
        self.fade.secret_drawn = self.secret_typed() || self.secret_shown();
        // Whatever draws on the canvas after the frame paints as it calls.
        canvas.record_calls(false);
        let surface = self.theme().role(Role::Surface);
        let Some((before, at)) = &self.fade.from else {
            return;
        };
        let (t, over) = at.map_or((0.0, false), |at| self.fade.blend(Some(at), self.now_ms));
        if over {
            self.fade.end();
        } else {
            canvas.swap_pixels(&mut self.fade.after);
            if self.fade.step_fade && self.fade.surface.is_empty() {
                self.fade.surface = surface_image(self.fade.after.len(), surface);
            }
            let inset = self.fade.edge_drawn_over;
            let n = self.fade.cores.blend_noting(
                canvas,
                before,
                &self.fade.after,
                (t, inset),
                &mut self.fade.bands,
            );
            // The card's place of a step's fade, over what the linear
            // blend just wrote there.
            let n = n.max(self.fade.step_card(canvas, t));
            self.fade.counts.bands = self.fade.counts.bands.max(n as u8);
            self.fade.due = true;
            self.out.commands.push_back(Command::Draw);
        }
    }

    /// Draws the next step of the fade under way when nothing but the
    /// fade asked for a frame: the new frame kept aside when it was last
    /// drawn in full, blended with the old one where the two differ, so a
    /// step costs a blend of those bands, not a draw of the screen, and
    /// they are what changed for the shell (`Self::take_changed`). Times
    /// the step from the one before it for [`Self::fades`].
    pub(crate) fn fade_step(&mut self) {
        self.fade.due = false;
        if self.canvas.is_none() || self.fade.from.is_none() {
            return;
        }
        let surface = self.theme().role(Role::Surface);
        if self.fade.step_fade && self.fade.surface.is_empty() {
            self.fade.surface = surface_image(self.fade.after.len(), surface);
        }
        if let Some(last) = self.fade.step_at
            && self.now_ms > last
        {
            let gap = (self.now_ms - last).min(1000) as u16;
            self.fade.step_of_fade(self.now_ms, gap);
        }
        let (Some(canvas), Some((before, at))) = (self.canvas.as_mut(), self.fade.from.as_ref())
        else {
            return;
        };
        self.fade.step_at = Some(self.now_ms);
        self.fade.counts.steps += 1;
        let (t, over) = at.map_or((0.0, false), |at| self.fade.blend(Some(at), self.now_ms));
        let card = self.fade.card_in(canvas);
        if let Some(changed) = self.fade.changed.as_mut() {
            changed.extend_from_slice(&self.fade.bands);
            changed.extend(card);
        }
        if over {
            // The new frame at rest, as it was drawn.
            canvas.swap_pixels(&mut self.fade.after);
            self.fade.end();
            self.fade.step_at = None;
        } else {
            let n = if self.fade.step_fade {
                self.fade.step_blend(canvas, t)
            } else {
                self.fade
                    .cores
                    .blend_within(canvas, before, &self.fade.after, t, &self.fade.bands)
            };
            self.fade.counts.bands = self.fade.counts.bands.max(n as u8);
            self.fade.due = true;
            self.out.commands.push_back(Command::Draw);
        }
    }

    /// Whoever holds the app draws over a ring `px` wide round its canvas
    /// every frame (Faraday Demo's focus edge): a fade leaves it out of
    /// what it blends and of what it says changed.
    ///
    /// Used by faraday-host.
    #[doc(hidden)]
    pub fn set_edge_drawn_over(&mut self, px: i32) {
        self.fade.edge_drawn_over = px.max(0);
    }

    /// Where the canvas may differ from the frame this was last called
    /// after, as rectangles of its pixels, or `None` where that is not
    /// known (a frame drawn in full): a fade's steps give their bands, so
    /// a shell copies and presents those alone. Counted from now on.
    ///
    /// Used by faraday-host.
    #[doc(hidden)]
    pub fn take_changed(&mut self) -> Option<Vec<Rect>> {
        self.fade.changed.replace(Vec::new())
    }

    /// Whether the next frame the shell asks for is a fade's step, a
    /// blend of the two kept frames, rather than a frame drawn in full.
    pub fn next_frame_blends(&self) -> bool {
        !self.out.wanted() && self.fade.is_due()
    }

    /// Whether the next frame asked for changes the app's pixels: drawn in
    /// full, or a fade's step. Otherwise the canvas is shown as it is.
    ///
    /// Used by faraday-host.
    #[doc(hidden)]
    pub fn paints_next(&self) -> bool {
        self.out.wanted() || self.fade.is_due()
    }

    /// What fades have done since this was last called, and the counts
    /// set back to nothing. Costs a sort of the few gaps kept.
    pub fn take_fade_counts(&mut self) -> FadeCounts {
        let mut gaps = self.fade.fresh_gaps(self.now_ms);
        gaps.sort_unstable();
        FadeCounts {
            gaps_known: gaps.len() as u8,
            median_gap_ms: gaps.get(gaps.len() / 2).copied(),
            worst_gap_ms: gaps.last().copied(),
            ..std::mem::take(&mut self.fade.counts)
        }
    }

    /// Whether a change of screen or sheet fades in: not with reduce
    /// motion, and not on a machine whose fades' steps lately came too
    /// far apart for a fade to read as one rather than as steps, until it
    /// has shown a run of frames fast enough again (`Fade::shown`).
    /// Until a few steps have been timed, it does.
    ///
    /// Used by faraday-host.
    #[doc(hidden)]
    pub fn fades(&self) -> bool {
        !self.settings.reduce_motion()
    }
}

// The moved frame's decision (`moved.rs`, `docs/FRAMES.md` §3.3).
impl Faraday {
    /// The next frame is drawn whole: a test of the moved frame draws the
    /// same state both ways.
    #[cfg(test)]
    pub(crate) fn draw_whole_next(&mut self) {
        self.out.kept.draw_whole_next();
        self.out.redraw();
    }

    /// The frames after this are drawn recorded, and painted in `n`
    /// bands on real threads when `n` is over 1.
    #[cfg(test)]
    pub(crate) fn draw_banded(&mut self, n: usize) {
        self.fade.cores.set_for_test(n);
        self.out.redraw();
    }

    /// What the last frame painted from the list: its calls, and its
    /// barriers for the bands it was painted in.
    #[cfg(test)]
    pub(crate) fn last_painted(&self) -> (usize, usize) {
        self.fade.cores.seen()
    }

    /// The frames after this are drawn recorded into the canvas's list and
    /// painted from it when `on`, or painted as each call is made.
    #[cfg(test)]
    pub(crate) fn draw_recorded(&mut self, on: bool) {
        self.fade.cores.set_for_test(usize::from(on));
        self.out.redraw();
    }

    /// Whether the frame about to be drawn may be a moved frame
    /// (`docs/FRAMES.md` §3.3), and if so the region whose rows it moves.
    /// `only_moved`: nothing asked for the frame but the region's motion.
    /// `stretch`: the stretch this frame is drawn with, pixels.
    /// A whole frame is then recorded and painted in bands by
    /// [`Self::fade_end`]; a moved frame paints its strip as the calls
    /// are made (`docs/FRAMES.md` §4.2).
    pub(crate) fn moved_plan(
        &mut self,
        c: &mut faraday_draw::Canvas,
        only_moved: bool,
        stretch: i32,
    ) -> Option<ui::Plan> {
        let plan = self.plan_moved(c, only_moved, stretch);
        c.record_calls(plan.is_none() && self.fade.cores.records());
        plan
    }

    fn plan_moved(
        &mut self,
        c: &faraday_draw::Canvas,
        only_moved: bool,
        stretch: i32,
    ) -> Option<ui::Plan> {
        let whole_next = self.out.kept.take_whole_next();
        let last = self.out.kept.last()?;
        let hot = self.pointer.hovered().is_some()
            || self.pointer.pressed().is_some()
            || self.pointer.tip_shown().is_some();
        let calm = !whole_next
            // 2. Nothing asked for the frame but the region's motion.
            && only_moved
            // 1. The last frame drew the same region, as drawn now.
            && last.drew(self.region_key(), self.slot(), self.look(c))
            && self.sheet.is_none()
            // 3. The stretch before and after is within the view.
            && last.stretches_within(stretch)
            // 4. Nothing in the region moves of itself.
            && last.calm()
            && !hot
            && !self.fade.under_way()
            && self.busy().is_none()
            && self.out.toast.is_none()
            && self.disclosed.as_ref().is_none_or(|d| !d.moving)
            && (!self.shows_clock() || last.clock_is(self.clock_now()))
            // 5. The ink audit reads every string's ink, which a strip
            // does not draw.
            && !c.recording_ink();
        calm.then(|| last.plan())
    }

    /// What a frame on `c` is drawn for, besides its region.
    fn look(&self, c: &faraday_draw::Canvas) -> Look {
        Look {
            theme: self.theme(),
            f: self.f,
            text: self.text_size(),
            size: (c.width(), c.height()),
        }
    }

    /// After a frame is drawn: what the next may move, and how this one
    /// moved the rows for the shell.
    pub(crate) fn moved_drew(&mut self, layer: &ui::Layer, stretch: i32, caret: bool) {
        // No region kept its pixels: what an earlier page left goes.
        if layer.of.is_none()
            && let Some(c) = self.canvas.as_mut()
        {
            c.drop_layer();
        }
        let Some(c) = self.canvas.as_ref() else {
            self.out.kept.keep(None);
            return;
        };
        let look = self.look(c);
        let hot = self.pointer.hovered().is_some() || self.pointer.pressed().is_some();
        let last = layer.of.map(|(slot, view)| {
            Last::new(
                (self.region_key(), slot),
                (view, layer.px, stretch),
                look,
                (!caret && !hot, self.clock_now()),
            )
        });
        self.out.kept.keep(last);
        match (layer.moved, layer.of) {
            (Some(d), Some((_, view))) => self.out.kept.moved_frame(view, d),
            _ => self.out.kept.whole_frame(),
        }
    }

    /// The current frame as parts, with how it moved the rows since the
    /// shell last took them (`App::parts`).
    pub(crate) fn parts_moved(&mut self) -> faraday_shell_api::Parts<'_> {
        let _ = faraday_shell_api::App::frame(self);
        let moved = self.out.kept.take_since();
        faraday_shell_api::Parts {
            base: faraday_shell_api::App::frame(self),
            base_y: 0,
            inner: None,
            changed: None,
            moved,
        }
    }
}

/// An image of `len` bytes of the opaque `color`, the page's surface, for
/// a step's card to fade through.
fn surface_image(len: usize, color: faraday_draw::Color) -> Vec<u8> {
    [color.r, color.g, color.b, 255]
        .iter()
        .copied()
        .cycle()
        .take(len)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::Fade;
    use crate::motion;

    /// A fade's steps `gap` ms apart after `now`; returns the time of the
    /// last.
    fn fade_steps(f: &mut Fade, mut now: u64, gap: u64, steps: usize) -> u64 {
        for _ in 0..steps {
            now += gap;
            f.step_of_fade(now, gap as u16);
        }
        now
    }

    #[test]
    fn slow_steps_are_timed_and_fades_stay_on() {
        let mut f = Fade::default();
        let end = fade_steps(&mut f, 1_000, 70, 20);
        assert_eq!(f.fresh_gaps(end), vec![70; motion::FADE_GAPS_KEPT]);
    }

    #[test]
    fn gaps_older_than_the_fresh_time_are_not_counted() {
        let mut f = Fade::default();
        let end = fade_steps(&mut f, 1_000, 30, 3);
        assert!(
            f.fresh_gaps(end + motion::FADE_GAPS_FRESH_MS + 1)
                .is_empty()
        );
    }

    #[test]
    fn slow_theme_change_steps_are_not_timed() {
        let mut f = Fade {
            theme_fade: true,
            ..Fade::default()
        };
        let end = fade_steps(&mut f, 1_000, 70, 20);
        assert!(f.fresh_gaps(end).is_empty());
    }

    #[test]
    fn a_step_after_a_frame_drawn_in_full_mid_fade_is_part_of_the_way_everywhere() {
        use faraday_shell_api::{App, Event};

        use crate::{Action, Screen};
        let mut app = crate::testkit::started();
        let mut now = 1_000;
        let tick = |app: &mut crate::Faraday, at: u64| {
            app.event(Event::Tick { now_ms: at });
            while app.poll_command().is_some() {}
            let _ = app.frame();
        };
        for _ in 0..10 {
            now += 16;
            tick(&mut app, now);
        }
        app.press(Action::Nav(Screen::Settings));
        for _ in 0..2 {
            now += 16;
            tick(&mut app, now);
        }
        // The pointer moved over the sidebar mid-fade, where the two
        // screens agree: drawn in full, the new frame kept aside again,
        // and what differs from the old one noted again, outside the
        // bands noted before.
        let w = app.canvas.as_ref().map_or(0, |c| c.width() as usize);
        let outside = |app: &crate::Faraday, kept: &[u8], old: &[faraday_draw::Rect]| {
            (0..kept.len()).any(|p| {
                let (x, y) = ((p / 4 % w) as i32, (p / 4 / w) as i32);
                kept[p] != app.fade.after[p] && !old.iter().any(|b| b.contains(x, y))
            })
        };
        let moved = (40..760).step_by(24).any(|y| {
            let (kept, old) = (app.fade.after.clone(), app.fade.bands.clone());
            app.event(Event::Hover { x: 40, y });
            while app.poll_command().is_some() {}
            let _ = app.frame();
            outside(&app, &kept, &old)
        });
        assert!(
            moved,
            "a hover changed the new frame where the screens agree"
        );
        now += 16;
        tick(&mut app, now);
        assert!(
            app.next_frame_blends(),
            "a step of the fade, still under way"
        );
        let (before, at) = app.fade.from.as_ref().expect("a fade under way");
        let (t, over) = app.fade.blend(*at, app.now_ms);
        assert!(!over && t > 0.0);
        // What a whole blend of the two kept frames shows at this step.
        let t = (t * 256.0) as u16;
        let after = &app.fade.after;
        let shown = app.canvas.as_ref().expect("a canvas").frame().rgba;
        let wrong = (0..shown.len())
            .filter(|&p| {
                let want = (u16::from(before[p]) * (256 - t) + u16::from(after[p]) * t) >> 8;
                u16::from(shown[p]) != want
            })
            .count();
        assert_eq!(wrong, 0, "each byte as far from the old frame to the new");
    }
}
