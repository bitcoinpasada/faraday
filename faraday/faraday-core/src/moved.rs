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

    use crate::work::budget::{DESKTOP, PI, Panel, walk_on};
    use crate::{Faraday, testkit};

    /// Where the screen's page region is, in pixels, and how far it goes.
    fn region(app: &Faraday) -> Option<(faraday_draw::Rect, f32)> {
        let e = app.region_now()?;
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

    /// Every step size, to each end and past it, and back, on the screen
    /// the app is on. Returns the moved frames taken.
    fn scrolls(app: &mut Faraday, name: &str) -> u32 {
        app.settle();
        app.draw_whole_next();
        let _ = app.frame();
        let Some((view, max)) = region(app) else {
            return 0;
        };
        let at = (
            (view.x + view.w / 2).clamp(0, i32::from(u16::MAX)) as u16,
            (view.y + view.h / 2).clamp(0, i32::from(u16::MAX)) as u16,
        );
        let start = app.out.kept.taken;
        // A screen that takes no moved frame is drawn whole: nothing to
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

    fn every_screen_on(panel: Panel, settings: bool) {
        let c = testkit::compute();
        let mut moving: Vec<(String, u32)> = Vec::new();
        let _ = walk_on(&c, panel, &mut |app, name| {
            let n = scrolls(app, name);
            if n > 0 {
                moving.push((name.to_string(), n));
            }
        });
        assert!(
            !settings || moving.iter().any(|(name, _)| name == "Settings"),
            "Settings takes moved frames: {moving:?}"
        );
    }

    #[test]
    fn every_scroll_step_shows_what_a_whole_frame_shows() {
        every_screen_on(PI, true);
        // Faraday Demo's Settings is not opted in: a corner of a card
        // differs by one anti-aliased pixel between a moved row and a
        // drawn one.
        every_screen_on(DESKTOP, false);
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
                let mid = region(app).map(|(v, _)| (v.x + v.w / 2, v.y + v.h / 2));
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
}
