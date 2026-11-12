//! The moved frame: when a scroll step moves the rows already drawn and
//! paints only the rows the move brings into view (`docs/FRAMES.md` §3).
//! This decides it; [`crate::ui::Ui::region`] draws it.

use faraday_draw::Rect;
use faraday_shell_api::Moved;

use crate::ScreenKey;
use crate::ui;

/// What the frame before drew of the active region, for the next frame to
/// move: the region and its pixels as its content left them
/// (`Canvas::layer`), and what the frame was drawn for.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Last {
    key: ScreenKey,
    slot: ui::Slot,
    view: Rect,
    px: i32,
    stretch: i32,
    theme: ui::Theme,
    f: f32,
    text: ui::TextSize,
    size: (i32, i32),
    /// No caret drawn and nothing hovered or pressed: nothing in the layer
    /// changes of itself.
    calm: bool,
    /// The clock as the frame drew it, for a screen that shows it.
    clock: Option<u64>,
}

/// What a frame was drawn for, besides its region: compared with what the
/// next frame is drawn for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Look {
    pub(crate) theme: ui::Theme,
    pub(crate) f: f32,
    pub(crate) text: ui::TextSize,
    pub(crate) size: (i32, i32),
}

impl Last {
    /// The region `slot` of screen `key` as drawn into `view`, its content
    /// `px` pixels up and stretched by `stretch`, for `look`; `calm`: no
    /// caret and nothing hovered or pressed; `clock`: the clock it showed.
    pub(crate) fn new(
        (key, slot): (ScreenKey, ui::Slot),
        (view, px, stretch): (Rect, i32, i32),
        look: Look,
        (calm, clock): (bool, Option<u64>),
    ) -> Self {
        Self {
            key,
            slot,
            view,
            px,
            stretch,
            theme: look.theme,
            f: look.f,
            text: look.text,
            size: look.size,
            calm,
            clock,
        }
    }

    /// Whether the frame drew region `slot` of screen `key` for `look`.
    pub(crate) fn drew(&self, key: ScreenKey, slot: ui::Slot, look: Look) -> bool {
        let mine = Look {
            theme: self.theme,
            f: self.f,
            text: self.text,
            size: self.size,
        };
        self.key == key && self.slot == slot && mine == look
    }

    /// Whether the stretch of that frame and `stretch` are both within the
    /// view.
    pub(crate) fn stretches_within(&self, stretch: i32) -> bool {
        self.stretch.abs() < self.view.h && stretch.abs() < self.view.h
    }

    /// Whether nothing in the layer changes of itself.
    pub(crate) fn calm(&self) -> bool {
        self.calm
    }

    /// Whether the clock the frame drew is `now`.
    pub(crate) fn clock_is(&self, now: Option<u64>) -> bool {
        self.clock == now
    }

    /// What the region moved from: its slot, view and content offset.
    pub(crate) fn plan(&self) -> ui::Plan {
        ui::Plan {
            slot: self.slot,
            view: self.view,
            px: self.px,
        }
    }
}

/// What the frames the shell has not taken did to the region's rows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Since {
    /// No frame since the shell took the last.
    #[default]
    Nothing,
    /// Moved frames of one region since.
    Moved(Moved),
    /// A frame drawn whole, or moves that do not compose.
    Whole,
}

/// What the moved frame keeps between frames.
#[derive(Debug, Default)]
pub(crate) struct State {
    last: Option<Last>,
    since: Since,
    /// The next frame is drawn whole, whatever else holds.
    whole_next: bool,
    /// Frames drawn as moved frames.
    #[cfg(test)]
    pub(crate) taken: u32,
}

impl State {
    /// What the last frame drew, if it drew a region.
    pub(crate) fn last(&self) -> Option<Last> {
        self.last
    }

    /// The next frame is drawn whole, whatever else holds.
    #[cfg(test)]
    pub(crate) fn draw_whole_next(&mut self) {
        self.whole_next = true;
    }

    /// Whether the next frame was to be drawn whole; it is asked once.
    pub(crate) fn take_whole_next(&mut self) -> bool {
        std::mem::take(&mut self.whole_next)
    }

    /// What the frame just drawn left of its region.
    pub(crate) fn keep(&mut self, last: Option<Last>) {
        self.last = last;
    }

    /// A frame drawn whole, or a fade's step: nothing can be said of how
    /// the rows moved.
    pub(crate) fn whole_frame(&mut self) {
        self.since = Since::Whole;
    }

    /// A moved frame of the region `view`, by `d` rows up.
    pub(crate) fn moved_frame(&mut self, view: Rect, d: i32) {
        let to = |v: i32| u16::try_from(v.max(0)).unwrap_or(u16::MAX);
        let m = Moved {
            x: to(view.x),
            y: to(view.y),
            width: to(view.w),
            height: to(view.h),
            dy: i16::try_from(-d).unwrap_or(i16::MAX),
        };
        self.since = match self.since {
            Since::Nothing => Since::Moved(m),
            Since::Moved(was) => was.then(m).map_or(Since::Whole, Since::Moved),
            Since::Whole => Since::Whole,
        };
        #[cfg(test)]
        {
            self.taken += 1;
        }
    }

    /// How the frames since the last call moved the rows, if they did
    /// ([`faraday_shell_api::Parts::moved`]).
    pub(crate) fn take_since(&mut self) -> Option<Moved> {
        match std::mem::take(&mut self.since) {
            Since::Moved(m) => Some(m),
            Since::Nothing | Since::Whole => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use faraday_shell_api::{App, Event, Key, TouchPhase};

    use crate::work::budget::{DESKTOP, PI, Panel, heavy_on, walk_on};
    use crate::{Faraday, testkit};

    /// Where the region in `slot` is, in pixels, and how far it goes.
    fn region(app: &Faraday, slot: crate::ui::Slot) -> Option<(faraday_draw::Rect, f32)> {
        let e = app.regions().find(|e| e.slot == slot)?;
        (e.max > 0.0).then_some((e.view, e.max * app.f))
    }

    /// One scroll of `dy` pixels (positive moves the content up) over
    /// the region, then the frame as drawn, and the same state drawn
    /// whole. Returns whether the first was a moved frame.
    fn step(app: &mut Faraday, at: (u16, u16), dy: i16, what: &str) -> bool {
        let before = app.out.kept.taken;
        app.event(Event::Scroll {
            x: at.0,
            y: at.1,
            dy,
        });
        while app.poll_command().is_some() {}
        let moved = app.frame().rgba.to_vec();
        let took = app.out.kept.taken > before;
        app.draw_whole_next();
        let whole = app.frame().rgba.to_vec();
        if moved != whole {
            let i = moved
                .iter()
                .zip(&whole)
                .position(|(a, b)| a != b)
                .unwrap_or(0);
            let w = app.canvas.as_ref().map_or(1, |c| c.width() as usize).max(1);
            panic!(
                "{what}: scrolling {dy} px, a moved frame (taken: {took}) differs from the same \
                 state drawn whole, first at row {} column {} ({} bytes differ)",
                i / 4 / w,
                i / 4 % w,
                moved.iter().zip(&whole).filter(|(a, b)| a != b).count()
            );
        }
        took
    }

    /// Every step size, to each end and past it, and back, on each
    /// region of the screen the app is on, the wheel pointed at it as a
    /// person would. Returns the moved frames taken, by region.
    fn scrolls(app: &mut Faraday, name: &str) -> Vec<(String, u32)> {
        app.settle();
        // A toast left from the walk's setup is a timer of its own: a
        // screen is scrolled once it is gone.
        app.out.toast = None;
        // A page left past its end by the last screen is put back by the
        // frame, which asks for another: both are drawn before a step.
        for _ in 0..2 {
            app.draw_whole_next();
            let _ = app.frame();
        }
        // The walk's clock does not run a step card's opening to its end, and the
        // frames just drawn may have begun it.
        if let Some(d) = app.disclosed.as_mut() {
            d.moving = false;
            app.draw_whole_next();
            let _ = app.frame();
        }
        let slots: Vec<crate::ui::Slot> = app.regions().map(|e| e.slot).collect();
        let mut taken = Vec::new();
        for slot in slots {
            let label = format!("{name} {slot:?}");
            let n = scroll_region(app, slot, &label);
            if n > 0 {
                taken.push((label, n));
            }
            app.settle();
            app.draw_whole_next();
            let _ = app.frame();
        }
        taken
    }

    fn scroll_region(app: &mut Faraday, slot: crate::ui::Slot, name: &str) -> u32 {
        let Some((view, max)) = region(app, slot) else {
            return 0;
        };
        let at = (
            (view.x + view.w / 2).clamp(0, i32::from(u16::MAX)) as u16,
            (view.y + view.h / 2).clamp(0, i32::from(u16::MAX)) as u16,
        );
        let start = app.out.kept.taken;
        // The first wheel step into a region makes it the active one and is
        // drawn whole; the moved frames come after it.
        if app.slot() != slot {
            step(app, at, 1, name);
        }
        // A region that takes no moved frame is drawn whole: nothing to
        // compare.
        if !step(app, at, 7, name) {
            return 0;
        }
        let third = (view.h / 3).clamp(2, 4000) as i16;
        let reach = (max as i32 / i32::from(third) + 3).min(40);
        for size in [1, 7, 40, third] {
            for k in 0..6 {
                step(app, at, size, &format!("{name}, step {size} (down {k})"));
            }
            for k in 0..reach {
                step(app, at, third, &format!("{name}, to the end ({k})"));
            }
            for k in 0..2 {
                step(app, at, third, &format!("{name}, past the end ({k})"));
                step(app, at, size, &format!("{name}, past the end by {size}"));
            }
            for k in 0..reach + 2 {
                step(app, at, -third, &format!("{name}, back to the start ({k})"));
            }
            step(app, at, -size, &format!("{name}, past the start by {size}"));
            step(app, at, size, &format!("{name}, off the start by {size}"));
        }
        app.out.kept.taken - start
    }

    /// Walks every screen of `panel`, scrolling each region, and fails
    /// when a region in `expected` took no moved frame.
    fn every_screen_on(panel: Panel, expected: &[&str]) {
        let c = testkit::compute();
        let mut moving: Vec<(String, u32)> = Vec::new();
        let _ = walk_on(&c, panel, &mut |app, name| {
            moving.extend(scrolls(app, name));
        });
        for name in expected {
            assert!(
                moving.iter().any(|(n, _)| n == name),
                "{name} takes moved frames: {moving:?}"
            );
        }
    }

    #[test]
    fn every_scroll_step_shows_what_a_whole_frame_shows() {
        every_screen_on(
            PI,
            &[
                "Create Page",
                "Spend Page",
                "GPG key flow Page",
                "Settings Page",
                "Wallets Page",
                "Wallet chart Page",
                "Files Page",
                "Vault contents Page",
                "Backup checklist Page",
                "Backups Page",
                "Backup board Page",
            ],
        );
        every_screen_on(
            DESKTOP,
            &[
                "Backup checklist Page",
                "Wallets Card",
                "Wallets ListPane",
                "Vault contents VaultItem",
                "Settings Page",
                "Backup board Page",
                "GPG key flow Page",
                "Spend Page",
                "Backup reopened Page",
            ],
        );
    }

    #[test]
    fn every_other_event_draws_the_frame_whole() {
        let c = testkit::compute();
        for panel in [PI, DESKTOP] {
            let mut checked = panel != PI;
            let _ = walk_on(&c, panel, &mut |app, name| {
                if name != "Settings" || panel != PI {
                    return;
                }
                app.settle();
                app.draw_whole_next();
                let _ = app.frame();
                let mid =
                    region(app, crate::ui::Slot::Page).map(|(v, _)| (v.x + v.w / 2, v.y + v.h / 2));
                let Some(mid) = mid else { return };
                let at = (mid.0 as u16, mid.1 as u16);
                let others = [
                    Event::Key(Key::Down),
                    Event::Key(Key::Char('a')),
                    Event::Hover { x: at.0, y: at.1 },
                    Event::HoverEnd,
                    Event::Touch {
                        x: at.0,
                        y: at.1,
                        phase: TouchPhase::Down,
                    },
                    Event::Touch {
                        x: at.0,
                        y: at.1,
                        phase: TouchPhase::Up,
                    },
                    Event::ScrollEnd { x: at.0, y: at.1 },
                    Event::Focus(true),
                    Event::Back,
                ];
                // The same step, moved, is a moved frame at this point.
                app.draw_whole_next();
                let _ = app.frame();
                assert!(
                    step(app, at, 7, "Settings, control"),
                    "Settings takes a moved frame on {panel:?}"
                );
                for event in others {
                    app.draw_whole_next();
                    let _ = app.frame();
                    let before = app.out.kept.taken;
                    app.event(event.clone());
                    let _ = app.frame();
                    assert_eq!(app.out.kept.taken, before, "{event:?} drew a moved frame");
                }
                checked = true;
            });
            assert!(checked, "Settings has a page that scrolls on {panel:?}");
        }
    }

    /// A screen drawn into the canvas's list and painted from it is the
    /// screen drawn call by call, on every screen of the walk.
    #[test]
    fn a_frame_recorded_then_replayed_is_the_frame_drawn() {
        let c = testkit::compute();
        for panel in [PI, DESKTOP] {
            let mut screens = 0;
            let _ = walk_on(&c, panel, &mut |app, name| {
                app.settle();
                app.out.toast = None;
                for _ in 0..2 {
                    app.draw_whole_next();
                    let _ = app.frame();
                }
                let direct = app.frame().rgba.to_vec();
                app.draw_whole_next();
                let direct_again = app.frame().rgba.to_vec();
                app.draw_recorded(true);
                app.draw_whole_next();
                let recorded = app.frame().rgba.to_vec();
                let (calls, _) = app.last_painted();
                app.draw_recorded(false);
                assert!(
                    calls > 0,
                    "{name}: the frame recorded no calls on {panel:?}"
                );
                assert!(
                    direct == direct_again,
                    "{name}: a frame drawn twice differs on {panel:?}"
                );
                assert!(
                    direct == recorded,
                    "{name}: a recorded frame differs from the frame drawn on {panel:?}, {} bytes",
                    direct.iter().zip(&recorded).filter(|(a, b)| a != b).count()
                );
                screens += 1;
            });
            assert!(screens > 10, "the walk reaches its screens on {panel:?}");
        }
    }

    /// A screen drawn into the canvas's list and painted in bands, each
    /// on a thread of its own, is the screen drawn call by call, on
    /// every screen of the walk.
    #[test]
    fn a_frame_painted_in_bands_on_threads_is_the_frame_drawn() {
        let c = testkit::compute();
        for panel in [PI, DESKTOP] {
            let mut screens = 0;
            let _ = walk_on(&c, panel, &mut |app, name| {
                app.settle();
                app.out.toast = None;
                for _ in 0..2 {
                    app.draw_whole_next();
                    let _ = app.frame();
                }
                let direct = app.frame().rgba.to_vec();
                for n in [2, 3, 4] {
                    app.draw_banded(n);
                    app.draw_whole_next();
                    let banded = app.frame().rgba.to_vec();
                    let (calls, _) = app.last_painted();
                    app.draw_banded(0);
                    assert!(calls > 0, "{name}: no calls recorded on {panel:?}");
                    assert!(
                        direct == banded,
                        "{name}: {n} bands differ from the frame drawn on {panel:?}, {} bytes",
                        direct.iter().zip(&banded).filter(|(a, b)| a != b).count()
                    );
                }
                screens += 1;
            });
            assert!(screens > 10, "the walk reaches its screens on {panel:?}");
        }
    }

    /// Home scrolled a few pixels at a time over a session with wallets
    /// and files, on the Pi's panel: what each step draws is the same
    /// state drawn whole in one pass, and in two, three and four bands
    /// on threads, so no row of the frame is left from another frame or
    /// painted twice.
    #[test]
    fn home_scrolled_by_steps_is_the_same_frame_drawn_whole_or_in_bands() {
        let (mut app, _) = heavy_on(PI);
        app.press(crate::Action::Nav(crate::Screen::Home));
        app.settle();
        app.out.toast = None;
        let rows = |a: &[u8], b: &[u8]| -> Vec<usize> {
            let w = 480 * 4;
            (0..a.len() / w)
                .filter(|&r| a[r * w..(r + 1) * w] != b[r * w..(r + 1) * w])
                .collect()
        };
        let steps: Vec<i16> = [(9i16, 120), (-23, 60), (5, 80)]
            .iter()
            .flat_map(|&(d, n)| std::iter::repeat_n(d, n))
            .collect();
        let mut bad: Vec<String> = Vec::new();
        let (mut scrolled, mut before) = (0, Vec::new());
        for (i, dy) in steps.into_iter().enumerate() {
            app.event(Event::Scroll { x: 240, y: 320, dy });
            while app.poll_command().is_some() {}
            let drawn = app.frame().rgba.to_vec();
            scrolled += usize::from(drawn != before);
            before.clone_from(&drawn);
            app.draw_banded(0);
            app.draw_whole_next();
            let whole = app.frame().rgba.to_vec();
            if drawn != whole {
                bad.push(format!(
                    "step {i} dy {dy}: drawn vs whole, rows {:?}",
                    rows(&drawn, &whole)
                ));
            }
            for n in [2, 3, 4] {
                app.draw_banded(n);
                app.draw_whole_next();
                let banded = app.frame().rgba.to_vec();
                app.draw_banded(0);
                if banded != whole {
                    bad.push(format!(
                        "step {i} dy {dy}: {n} bands vs whole, rows {:?}",
                        rows(&banded, &whole)
                    ));
                }
            }
            if bad.len() > 12 {
                break;
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
        assert!(scrolled > 50, "Home scrolled in {scrolled} steps");
    }
}
