//! When the window ticks and draws (`docs/MOTION.md` §3.2).
//!
//! The idle tick comes every [`TICK`]. While something moves, the ticks
//! come a frame apart: on Wayland with the compositor's own refreshes
//! (the window's redraws are delivered with the screen's next refresh
//! once a frame is presented), on X11 from the timer, on a grid of
//! refreshes from the first one. A machine that cannot draw a frame in
//! a refresh gets every other refresh, in even steps, rather than one
//! and then two.

use std::time::{Duration, Instant};

use osk_shell_api::Event;

/// The idle tick.
pub const TICK: Duration = Duration::from_millis(50);

/// The tick while something moves and the compositor does not pace the
/// frames (X11): a refresh of a screen that does not say its rate, on a
/// grid from the first one (`docs/MOTION.md` §3.2). On Wayland the
/// ticks come with the screen's own refreshes instead.
pub const FRAME: Duration = Duration::from_micros(16_667);
/// How often `--timings` prints what the frames cost.
const TIMINGS_EVERY: Duration = Duration::from_secs(1);
/// A pause between two frames longer than this is a rest, not a dropped
/// frame, and is left out of the frame-to-frame figures.
const FRAME_REST: Duration = Duration::from_millis(100);
/// The coarsest grid step, in refreshes. A machine slower than that
/// draws as fast as it can.
const MOST_STEPS: u32 = 4;
/// A frame must fit the step with this much room for the step to be
/// kept; a step that no longer holds it is lengthened at once.
const KEEP: f64 = 1.25;
/// The step is shortened only when a frame fits the shorter one with
/// this much room, so the step does not flip at the boundary.
const SHORTEN: f64 = 1.6;

/// The window's clock and pace.
pub struct Pacer {
    /// When the window opened: ticks carry the milliseconds since.
    pub epoch: Instant,
    /// When the next tick is due.
    pub next_tick: Instant,
    /// The last tick asked for a frame: something is moving, so the next
    /// tick comes a frame later rather than at the idle rate.
    pub animating: bool,
    /// The compositor paces the frames (Wayland): a redraw asked for is
    /// delivered with the screen's next refresh, so the tick that moves
    /// what moves is sent on each redraw and every frame is drawn for
    /// the moment it will be shown. Without it (X11) the ticks come
    /// from the timer, on the 60 Hz grid.
    pub paced: bool,
    /// What the frames cost, printed once a second with `--timings`.
    timings: Option<FrameTimings>,
    /// What recent frames cost (render and copy), weighted towards the
    /// newest, and the whole refreshes the timer's grid steps by to hold
    /// one. Only the timer's grid uses it; a compositor that paces the
    /// frames settles the rate itself.
    cost: Option<Duration>,
    steps: u32,
}

impl Pacer {
    /// A clock starting now; `timings` prints what the frames cost once
    /// a second.
    pub fn new(timings: bool) -> Pacer {
        let now = Instant::now();
        Pacer {
            epoch: now,
            next_tick: now,
            animating: false,
            paced: false,
            timings: timings.then(FrameTimings::new),
            cost: None,
            steps: 1,
        }
    }

    /// The milliseconds since the window opened, for a tick.
    pub fn now_ms(&self) -> u64 {
        self.epoch.elapsed().as_millis() as u64
    }

    /// The tick event for now.
    pub fn tick(&self) -> Event {
        Event::Tick {
            now_ms: self.now_ms(),
        }
    }

    /// An event is about to be delivered. What may start something
    /// moving is followed by a tick within a frame, so the motion starts
    /// at once. With the compositor pacing the frames, the redraw it
    /// asks for brings that tick itself.
    pub fn before(&mut self, event: &Event) {
        if !self.paced
            && matches!(
                event,
                Event::Wheel { .. }
                    | Event::Scroll { .. }
                    | Event::ScrollEnd { .. }
                    | Event::Touch { .. }
            )
        {
            self.next_tick = self.next_tick.min(Instant::now() + FRAME);
        }
    }

    /// Whether `--timings` is on.
    pub fn timed(&self) -> bool {
        self.timings.is_some()
    }

    /// A frame took `took` to render and copy, on a screen that
    /// refreshes every `refresh`: the timer's grid step is the fewest
    /// refreshes that hold one with room. One slow frame (the frame a
    /// sheet opens on) moves the smoothed cost a fifth of the way and
    /// changes nothing; a machine slow every frame does.
    pub fn drew(&mut self, took: Duration, refresh: Duration) {
        let cost = match self.cost {
            None => took,
            Some(c) => (c * 4 + took) / 5,
        };
        self.cost = Some(cost);
        let holds = |steps: u32, room: f64| refresh * steps >= cost.mul_f64(room);
        if !holds(self.steps, KEEP) {
            self.steps = (self.steps + 1..=MOST_STEPS)
                .find(|&n| holds(n, KEEP))
                .unwrap_or(MOST_STEPS);
        } else if self.steps > 1 && holds(self.steps - 1, SHORTEN) {
            self.steps -= 1;
        }
    }

    /// Records one frame presented at `now` for `--timings`.
    pub fn presented(&mut self, render: Duration, blit: Duration, now: Instant) {
        if let Some(t) = self.timings.as_mut() {
            t.frame(render, blit, now);
        }
    }

    /// The tick due at `now` has been sent: when the next one is.
    /// Without the compositor's pacing, a motion runs on a grid of
    /// frames from its first tick, not a frame after each pass, so the
    /// steps stay even; a pass that ran past its deadline starts the
    /// grid again from now. With the pacing, the ticks ride the
    /// redraws, and this one is the idle tick, or the one that keeps
    /// time while the window is hidden and the compositor sends no
    /// refreshes.
    pub fn ticked_at(&mut self, now: Instant, refresh: Duration) {
        self.next_tick = if self.animating && !self.paced {
            let step = refresh * self.steps;
            let on_grid = self.next_tick + step;
            if on_grid > now { on_grid } else { now + step }
        } else {
            now + TICK
        };
    }
}

/// What a second's frames cost: the render, the copy into the window,
/// and the time from one frame to the next while something moved.
struct FrameTimings {
    since: Instant,
    /// When the last frame was presented.
    last: Option<Instant>,
    frames: u32,
    /// Mean and worst, as a total and a maximum.
    render: (Duration, Duration),
    blit: (Duration, Duration),
    /// Between consecutive frames of a motion, and how many of them.
    gap: (Duration, Duration),
    gaps: u32,
}

impl FrameTimings {
    fn new() -> FrameTimings {
        FrameTimings {
            since: Instant::now(),
            last: None,
            frames: 0,
            render: (Duration::ZERO, Duration::ZERO),
            blit: (Duration::ZERO, Duration::ZERO),
            gap: (Duration::ZERO, Duration::ZERO),
            gaps: 0,
        }
    }

    /// Records one frame presented at `now`, and prints a line once a
    /// second.
    fn frame(&mut self, render: Duration, blit: Duration, now: Instant) {
        self.frames += 1;
        self.render = (self.render.0 + render, self.render.1.max(render));
        self.blit = (self.blit.0 + blit, self.blit.1.max(blit));
        if let Some(last) = self.last {
            let gap = now - last;
            if gap < FRAME_REST {
                self.gap = (self.gap.0 + gap, self.gap.1.max(gap));
                self.gaps += 1;
            }
        }
        self.last = Some(now);
        if now - self.since < TIMINGS_EVERY {
            return;
        }
        let ms = |d: Duration| d.as_secs_f64() * 1000.0;
        let mean = |total: Duration, n: u32| ms(total) / f64::from(n.max(1));
        eprintln!(
            "faraday: {} frames · render {:.1} ms (worst {:.1}) · blit {:.1} ms (worst {:.1}) · frame to frame {:.1} ms (worst {:.1}) over {}",
            self.frames,
            mean(self.render.0, self.frames),
            ms(self.render.1),
            mean(self.blit.0, self.frames),
            ms(self.blit.1),
            mean(self.gap.0, self.gaps),
            ms(self.gap.1),
            self.gaps,
        );
        *self = FrameTimings {
            since: now,
            last: self.last,
            ..FrameTimings::new()
        };
    }
}
