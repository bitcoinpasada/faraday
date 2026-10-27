//! The host's lists under a finger (`docs/DESKTOP-HOST.md` §5): the open
//! menu's rows and Samples' sheet's lists, and the strip's row where it
//! is wider than the window, each dragged past the slop by a finger or
//! the mouse, coasting when it lifts and springing back from an end, as
//! the app's own pages do (`crate::motion`, `docs/MOTION.md` §3.3). A
//! press that moves past the slop presses nothing; a touch on a list
//! still coasting only stops it. Each motion asks for frames only while
//! it moves (`docs/SPEED.md` §12).

use faraday_shell_api::{Command, TouchPhase};

use super::{Held, Host, Motion, Region, SLOP};

impl Host {
    /// The open menu's or sheet's rows at their top, still.
    pub(super) fn list_top(&mut self) {
        self.sheet_scroll = 0.0;
        self.list_motion.stop();
        self.list_drag = None;
    }

    /// How far the open menu's rows or Samples' sheet's lists are shown
    /// scrolled, dp: where they are, stretched past an end. A lookup.
    pub(crate) fn list_shown(&self, max: f32) -> f32 {
        self.sheet_scroll.min(max) + self.list_motion.stretch()
    }

    /// How far the strip's row is shown stretched past an end, dp. A
    /// lookup.
    pub(crate) fn strip_stretch(&self) -> f32 {
        self.strip_motion.stretch()
    }

    /// How far the open menu's rows or Samples' sheet's lists are
    /// scrolled, dp: for a test of a finger on them. A lookup.
    pub fn list_scrolled(&self) -> f32 {
        self.sheet_scroll
    }

    /// Where the open menu or Samples' sheet stood in the frame last
    /// drawn, in the window's pixels: its left and top edges, its width
    /// and height. A lookup.
    pub fn menu_area(&self) -> Option<(i32, i32, i32, i32)> {
        self.sheet_view
            .filter(|_| self.menu.is_some())
            .map(|(r, _)| (r.x, r.y, r.w, r.h))
    }

    /// Whether the host's own lists move on their own: a coast or a
    /// spring. A lookup.
    pub(super) fn lists_moving(&self) -> bool {
        self.list_motion.moving() || self.strip_motion.moving()
    }

    /// Runs `go` on the open menu's or sheet's rows as last drawn; `None`
    /// where none is open.
    fn with_list<R>(&mut self, go: impl FnOnce(&mut Motion, &mut Region) -> R) -> Option<R> {
        let (r, max) = self.sheet_view.filter(|_| self.menu.is_some())?;
        let f = self.f();
        let mut region = Region {
            offset: &mut self.sheet_scroll,
            max: Some(max),
            view: r.h as f32 / f,
            f,
        };
        Some(go(&mut self.list_motion, &mut region))
    }

    /// Runs `go` on the strip's row.
    fn with_strip<R>(&mut self, go: impl FnOnce(&mut Motion, &mut Region) -> R) -> R {
        let f = self.f();
        let view = self.window.map_or(0.0, |d| f32::from(d.width) / f);
        let mut region = Region {
            offset: &mut self.strip_scroll,
            max: Some(self.strip_max),
            view,
            f,
        };
        go(&mut self.strip_motion, &mut region)
    }

    /// A tick: the lists coasting or springing back move on; the rows of
    /// a menu no longer open stop.
    pub(super) fn lists_tick(&mut self, now_ms: u64) {
        if self.with_list(|m, r| m.tick(r, now_ms)).is_none() {
            self.list_motion.stop();
        }
        self.with_strip(|m, r| m.tick(r, now_ms));
    }

    /// A finger or the mouse at (`px`, `py`) on the open menu or sheet
    /// where its rows scroll. Returns whether the host took it: a drag
    /// past the slop scrolls the rows and presses nothing, a touch on
    /// rows still coasting only stops them; a press that does not move
    /// goes on as a press.
    pub(super) fn list_touch(&mut self, px: i32, py: i32, phase: TouchPhase) -> bool {
        let f = self.f();
        match phase {
            TouchPhase::Down => {
                self.list_drag = None;
                let on = self
                    .sheet_view
                    .filter(|(r, max)| self.menu.is_some() && *max > 0.0 && r.contains(px, py));
                if on.is_none() {
                    return false;
                }
                self.list_motion.rigid = self.app.reduce_motion();
                if self.list_motion.flung(f) {
                    self.list_motion.stop();
                    self.pressed = None;
                    self.held = Held::Nowhere;
                    self.list_drag = Some((py, py, true));
                    return true;
                }
                self.list_drag = Some((py, py, false));
                false
            }
            TouchPhase::Move => {
                let Some((y0, last, moving)) = self.list_drag else {
                    return false;
                };
                if !moving && ((py - y0).abs() as f32) <= SLOP * f {
                    return false;
                }
                self.pressed = None;
                self.held = Held::Nowhere;
                self.list_drag = Some((y0, py, true));
                let dy = (last - py) as f32 / f;
                self.with_list(|m, r| m.pan(r, dy));
                self.out.push_back(Command::Draw);
                true
            }
            TouchPhase::Up => {
                let Some((_, _, moving)) = self.list_drag.take() else {
                    return false;
                };
                if !moving {
                    return false;
                }
                self.list_motion.release(f);
                self.pressed = None;
                self.held = Held::None;
                self.out.push_back(Command::Draw);
                true
            }
        }
    }

    /// A finger or the mouse at `px` on the strip, where its row is wider
    /// than the window. Returns whether the host took it: past the slop
    /// it drags the row sideways and presses nothing, coasting when it
    /// lifts; a touch on the row still coasting only stops it.
    pub(super) fn strip_touch(&mut self, px: i32, py: i32, phase: TouchPhase) -> bool {
        let f = self.f();
        if phase == TouchPhase::Down {
            let on = self.menu.is_none() && self.on_strip(py) && self.strip_max > 0.0;
            self.drag = on.then_some((px, px, false));
            if !on {
                return false;
            }
            self.strip_motion.rigid = self.app.reduce_motion();
            if self.strip_motion.flung(f) {
                self.strip_motion.stop();
                self.pressed = None;
                self.held = Held::Nowhere;
                self.drag = Some((px, px, true));
                return true;
            }
            return false;
        }
        let Some((x0, last, moved)) = self.drag else {
            return false;
        };
        let past = moved || ((px - x0).abs() as f32) > SLOP * f;
        if !past {
            if phase == TouchPhase::Up {
                self.drag = None;
            }
            return false;
        }
        self.drag = Some((x0, px, true));
        self.pressed = None;
        self.held = Held::Nowhere;
        let dx = (last - px) as f32 / f;
        self.with_strip(|m, r| m.pan(r, dx));
        if phase == TouchPhase::Up {
            self.drag = None;
            self.held = Held::None;
            self.strip_motion.release(f);
        }
        self.out.push_back(Command::Draw);
        true
    }
}
