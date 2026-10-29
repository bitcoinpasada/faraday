//! Faraday Demo's host (`docs/DESKTOP-HOST.md`): the desktop app as a
//! simulator of the device. The host holds the app and lays it out as the
//! device would be at the size shown: the window less a strip at its foot
//! ("This window"), or a device's screen (the Pi panel, the 7-inch panel)
//! framed and centred in the host's space above the strip, scaled down only
//! where the window is smaller. The strip, in the host's own look and
//! outside the app, holds what only Faraday Demo has: the stick pills,
//! Samples, the preview size and the screen's demo aids. The app draws
//! none of them while it is held ([`Faraday::hosted`]); what it offers for
//! the demo it notes as it draws, and the strip reads that note.
//!
//! The pointer is carried into the app's frame: a press, a hover, a scroll
//! or the wheel inside it reaches the app at its own pixel; a press begun
//! there follows the pointer to the frame's edge; a press on the strip or
//! a host menu is the host's; anywhere else, nothing. Keys and every other
//! event are the app's.

pub(crate) mod screen;
mod scroll;

use std::collections::VecDeque;

use faraday_draw::Canvas;
use faraday_draw::geom::Rect;
use faraday_draw::tokens as t;
use faraday_shell_api::{App, Changed, Command, DisplayInfo, Event, Frame, Parts, TouchPhase};

use crate::motion::{Motion, Region, SLOP};
use crate::{Action, Faraday};

/// A press on the host's own controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HostPress {
    /// One of the app's actions: a stick put in or pulled, Load samples,
    /// a demo aid.
    App(Action),
    /// A host menu opened, or closed where it is open.
    Menu(HostMenu),
    /// A preview size chosen; `None` is the window itself.
    Size(Option<(u16, u16)>),
    /// Samples' sheet's own surface: a press there does nothing, and
    /// closes nothing.
    Sheet,
}

/// Where a press that has not ended began.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Held {
    /// No press.
    #[default]
    None,
    /// In the app's frame: it follows the pointer there.
    App,
    /// On a host control.
    Host,
    /// Elsewhere: it does nothing.
    Nowhere,
}

/// Where the app's frame stands in the window, in pixels, and its own
/// size: the same where it fits, smaller where it is scaled down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Area {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width shown.
    pub w: i32,
    /// Height shown.
    pub h: i32,
    /// The frame's own width.
    pub fw: i32,
    /// The frame's own height.
    pub fh: i32,
}

impl Area {
    fn inside(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }

    /// The frame's pixel nearest the window's pixel (`x`, `y`). A lookup.
    fn to_app(self, x: i32, y: i32) -> (u16, u16) {
        let dx = (x - self.x).clamp(0, self.w.max(1) - 1);
        let dy = (y - self.y).clamp(0, self.h.max(1) - 1);
        let fx = i64::from(dx) * i64::from(self.fw) / i64::from(self.w.max(1));
        let fy = i64::from(dy) * i64::from(self.fh) / i64::from(self.h.max(1));
        (
            fx.clamp(0, i64::from(u16::MAX)) as u16,
            fy.clamp(0, i64::from(u16::MAX)) as u16,
        )
    }
}

/// Faraday Demo's host, holding the app it shows.
pub struct Host {
    /// The app.
    pub app: Faraday,
    window: Option<DisplayInfo>,
    /// The preview size, in dp; `None` is the window less the strip.
    preview: Option<(u16, u16)>,
    /// The app's display as last told, in pixels.
    told: Option<(u16, u16)>,
    /// The window's whole frame, made only while it is drawn whole: for a
    /// preview, under a menu or Samples' sheet, and for [`App::frame`].
    /// At the window's own size with nothing of the host's over the app
    /// it is dropped, and the window is the app's frame and the strip's
    /// ([`Host::strip`]), so the host holds no canvas the window's size.
    canvas: Option<Canvas>,
    /// The strip alone, the window's width and the strip's height, at
    /// the window's scale: what the host draws at the window's own size
    /// with nothing over the app (`docs/DESKTOP-HOST.md` §9).
    strip: Option<Canvas>,
    hits: Vec<(Rect, HostPress)>,
    pressed: Option<HostPress>,
    hovered: Option<HostPress>,
    /// The menu open, and when it opened; `None`, opened at rest, whole
    /// whatever the clock says.
    menu: Option<(HostMenu, Option<u64>)>,
    /// The menu closing, and when it began to.
    leaving: Option<(HostMenu, u64)>,
    /// The frame last drawn showed a menu part of the way in or out: the
    /// next tick asks for the frame that shows it at rest.
    drawn_moving: bool,
    /// What the strip's controls said in the frame last drawn, left to
    /// right, and the open menu's rows after them.
    labels: Vec<String>,
    held: Held,
    now: u64,
    /// The window's pixels per dp, without the person's Display scale:
    /// the host is not the app.
    f: f32,
    out: VecDeque<Command>,
    /// Holding the app with no strip and no preview: a window that
    /// stands for a device's screen (`--panel`).
    bare: bool,
    /// How far the open menu's rows or Samples' sheet's lists are
    /// scrolled, dp.
    sheet_scroll: f32,
    /// The open menu or Samples' sheet as last drawn: where it stands in
    /// the window's pixels and the furthest its rows scroll, dp.
    sheet_view: Option<(Rect, f32)>,
    /// How the menu's or sheet's rows move under a finger and after it
    /// (`docs/MOTION.md` §3.3), as the app's own pages do.
    list_motion: Motion,
    /// A press begun on the open menu or sheet where its rows scroll:
    /// where it went down and was last, in the window's pixels, and
    /// whether it has moved past the slop, so that it scrolls the rows
    /// and presses nothing.
    list_drag: Option<(i32, i32, bool)>,
    /// How far the strip's row is scrolled sideways, dp, where it is wider
    /// than the window; the furthest it scrolls in the frame last drawn.
    strip_scroll: f32,
    strip_max: f32,
    /// How the strip's row moves under a finger and after it.
    strip_motion: Motion,
    /// A press begun on the strip: where, in the window's pixels, it went
    /// down and was last, and whether it has moved past the slop, so
    /// that it drags the row and presses nothing.
    drag: Option<(i32, i32, bool)>,
    /// Recording where the host's text and icons leave their ink
    /// ([`Host::record_ink`]).
    ink_on: bool,
    /// What says where the pointer and the keys go
    /// (`docs/DESKTOP-HOST.md` §4): the pointer is over the app's frame;
    /// a key reached the app since the pointer last went to the strip or
    /// left the window (so at the start); the window has the keyboard.
    pointer_in: bool,
    keys_in: bool,
    window_focus: bool,
    /// The focus border lit (the pointer or the keys reach the app), and
    /// the change to it under way: how lit it was, 0 to 1, and when it
    /// began; `None` at rest.
    edge_on: bool,
    edge_move: Option<(f32, u64)>,
    /// The sticks the app asks for, as last seen ([`Host::sync_asked`]),
    /// and when they were first asked for: their pills pulse from then.
    asked: Vec<usize>,
    pulse_at: u64,
    /// An event other than a tick came since the frame last drawn whole:
    /// a frame that host motion on the strip alone asks for draws the
    /// strip and nothing else while this is false.
    stale: bool,
    /// The frame last drawn whole showed a menu or the focus border part
    /// of the way.
    drawn_whole_moving: bool,
    /// The focus border as last drawn on the app's frame: how wide, and
    /// its colour.
    edge_drawn: Option<(i32, faraday_draw::Color)>,
    /// Where the window may differ from the parts last given, for
    /// [`Parts::changed`]: the app's own changes where it stands, the
    /// strip, and the focus border where its colour changed.
    changed: Vec<Changed>,
    /// How the parts last given were laid out, the app's frame handed
    /// over unplaced: the window's size, where the app stands and the
    /// strip's top. `None` after the window was given whole. What changed
    /// is known only from parts laid out the same way.
    given: Option<(u16, u16, i32, i32, u16)>,
}

impl Host {
    /// A host holding `app`, which from now on draws none of what the
    /// host's strip holds.
    pub fn new(mut app: Faraday) -> Host {
        app.set_hosted(true);
        Host {
            app,
            window: None,
            preview: None,
            told: None,
            canvas: None,
            strip: None,
            hits: Vec::new(),
            pressed: None,
            hovered: None,
            menu: None,
            leaving: None,
            drawn_moving: false,
            held: Held::None,
            now: 0,
            f: 1.0,
            out: VecDeque::new(),
            labels: Vec::new(),
            bare: false,
            sheet_scroll: 0.0,
            sheet_view: None,
            list_motion: Motion::default(),
            list_drag: None,
            strip_scroll: 0.0,
            strip_max: 0.0,
            strip_motion: Motion::default(),
            drag: None,
            ink_on: false,
            pointer_in: false,
            keys_in: true,
            window_focus: true,
            edge_on: true,
            edge_move: None,
            asked: Vec::new(),
            pulse_at: 0,
            stale: true,
            drawn_whole_moving: false,
            edge_drawn: None,
            changed: Vec::new(),
            given: None,
        }
    }

    /// A host that only passes `app` its events and shows its frame, for
    /// a window that stands for a device's screen: no strip, no preview,
    /// and the app draws its own pills as the device's would.
    pub fn bare(app: Faraday) -> Host {
        let mut host = Host::new(app);
        host.app.set_hosted(false);
        host.bare = true;
        host
    }

    /// The strip's height at rest, dp: for drivers that lay the app out
    /// before handing it to a host.
    pub const STRIP_DP: f32 = t::HOST_STRIP_H;

    /// The preview size shown, in dp; `None` for the window itself.
    pub fn preview(&self) -> Option<(u16, u16)> {
        self.preview
    }

    /// Shows the app at `size` in dp, or at the window less the strip for
    /// `None`.
    pub fn set_preview(&mut self, size: Option<(u16, u16)>) {
        self.preview = size.filter(|_| !self.bare);
        self.stale = true;
        self.relayout();
    }

    /// Opens `menu` at once, as a script does: for drivers that take
    /// pictures of the screens.
    pub fn open_menu_at_rest(&mut self, menu: HostMenu) {
        self.list_top();
        if menu == HostMenu::Samples {
            self.app.press(Action::Samples);
        }
        self.menu = Some((menu, None));
        self.leaving = None;
        self.stale = true;
    }

    /// The menu open now, if any.
    pub fn menu_open(&self) -> Option<HostMenu> {
        self.menu.map(|m| m.0)
    }

    /// Where the host's control for `press` stood in the frame last
    /// drawn: its centre in the window's pixels. A lookup.
    pub fn where_offered(&self, press: HostPress) -> Option<(u16, u16)> {
        self.hits
            .iter()
            .rev()
            .find(|(_, p)| *p == press)
            .map(|(r, _)| {
                (
                    (r.x + r.w / 2).clamp(0, i32::from(u16::MAX)) as u16,
                    (r.y + r.h / 2).clamp(0, i32::from(u16::MAX)) as u16,
                )
            })
    }

    /// What the strip's controls said in the frame last drawn, left to
    /// right, then the open menu's rows. A lookup.
    pub fn strip_texts(&self) -> &[String] {
        &self.labels
    }

    /// Where Samples' sheet stood in the frame last drawn, in the window's
    /// pixels: its left and top edges, its width and height. A lookup.
    pub fn sheet_area(&self) -> Option<(i32, i32, i32, i32)> {
        self.sheet_view
            .filter(|_| self.menu_open() == Some(HostMenu::Samples))
            .map(|(r, _)| (r.x, r.y, r.w, r.h))
    }

    /// Where the app's frame stands in the window, in pixels: its left
    /// and top edges, its width and height as shown. A lookup.
    pub fn app_area(&self) -> Option<(i32, i32, i32, i32)> {
        self.area().map(|a| (a.x, a.y, a.w, a.h))
    }

    /// Where the strip stands in the window, in pixels: its left and top
    /// edges, its width and height. A lookup.
    pub fn strip_area(&self) -> Option<(i32, i32, i32, i32)> {
        let d = self.window.filter(|_| !self.bare)?;
        let h = self.strip_px();
        Some((0, i32::from(d.height) - h, i32::from(d.width), h))
    }

    /// Where the host's control for `press` stood in the frame last
    /// drawn, in the window's pixels: its left and top edges, its width
    /// and height, as drawn and as pressed. A lookup.
    pub fn offered_area(&self, press: HostPress) -> Option<(i32, i32, i32, i32)> {
        self.hits
            .iter()
            .rev()
            .find(|(_, p)| *p == press)
            .map(|(r, _)| (r.x, r.y, r.w, r.h))
    }

    /// Starts or stops recording where the host's own text and icons
    /// leave their ink each frame ([`Host::ink`]): for a test of what
    /// the host writes reading on what it is written on.
    pub fn record_ink(&mut self, on: bool) {
        self.ink_on = on;
        if let Some(c) = self.canvas.as_mut() {
            c.record_ink(on);
        }
    }

    /// The bytes the host's own canvases hold now: the strip's, and the
    /// window's whole frame while it is held. A lookup.
    pub fn canvas_bytes(&self) -> usize {
        [&self.canvas, &self.strip]
            .into_iter()
            .flatten()
            .map(|c| c.frame().rgba.len())
            .sum()
    }

    /// What the host drew in the frame last drawn and where, while
    /// [`Host::record_ink`] has it on. A lookup.
    pub fn ink(&self) -> &[faraday_draw::canvas::Ink] {
        self.canvas.as_ref().map_or(&[], |c| c.ink())
    }

    /// Whether the window's pixel (`x`, `y`) is on the strip. Arithmetic.
    fn on_strip(&self, y: i32) -> bool {
        self.strip_area().is_some_and(|(_, top, _, _)| y >= top)
    }

    /// Scrolls the strip's row `dy` dp sideways, where it is wider than
    /// the window.
    fn scroll_strip(&mut self, dy: f32) {
        self.strip_motion.stop();
        self.strip_scroll = (self.strip_scroll + dy).clamp(0.0, self.strip_max);
        self.out.push_back(Command::Draw);
    }

    /// Costs a frame of the app and the strip. What the strip offers for
    /// `app` as it stands ([`Host::strip_texts`]): the app held for one
    /// frame in a window of its own display, then given back unhosted at
    /// that display. For drivers and tests that script the app alone.
    pub fn strip_for(app: &mut Faraday) -> Vec<String> {
        Host::peek(app).0
    }

    /// Costs a frame of the app and the strip. The app's presses the
    /// strip offers for `app` as it stands, its menus' rows among them,
    /// as [`Host::strip_for`] holds it.
    pub fn strip_actions_for(app: &mut Faraday) -> Vec<Action> {
        Host::peek(app).1
    }

    /// Costs a frame. The strip's texts and the app's presses in it for
    /// `app`, held for one frame and given back.
    fn peek(app: &mut Faraday) -> (Vec<String>, Vec<Action>) {
        let Some(d) = app.last_display_info() else {
            return (Vec::new(), Vec::new());
        };
        let taken = std::mem::take(app);
        let mut host = Host::new(taken);
        host.event(Event::Display(d));
        host.event(Event::Tick {
            now_ms: host.app.now_ms(),
        });
        let _ = host.frame();
        let texts = host.labels.clone();
        // The aids in More are presses too, as its menu would list them.
        host.open_menu_at_rest(HostMenu::More);
        let _ = host.frame();
        let actions = host
            .hits
            .iter()
            .filter_map(|(_, p)| match p {
                HostPress::App(a) => Some(*a),
                // Samples opens the host's sheet of what the app's own
                // Samples lists.
                HostPress::Menu(HostMenu::Samples) => Some(Action::Samples),
                _ => None,
            })
            .collect();
        host.app.set_hosted(false);
        host.app.event(Event::Display(d));
        *app = host.app;
        while app.poll_command().is_some() {}
        (texts, actions)
    }

    /// The window's pixels per dp.
    fn f(&self) -> f32 {
        self.f
    }

    /// The strip's height, pixels.
    fn strip_px(&self) -> i32 {
        if self.bare {
            return 0;
        }
        (self.strip_dp() * self.f()).round() as i32
    }

    /// The strip's height, dp: `HOST_STRIP_H`, or the computer's dice
    /// roller's row with a gap above and below it, inside the aids'
    /// section, while the page shows one (New key's dice, Generate with
    /// dice). The window's alone: the preview shown changes nothing. A
    /// lookup.
    pub(crate) fn strip_dp(&self) -> f32 {
        if screen::roller_shown(&self.app).is_some() {
            t::HOST_STRIP_H.max(crate::fresh::roller::screen::ROW_H + 2.0 * t::GAP)
        } else {
            t::HOST_STRIP_H
        }
    }

    /// The app's display for the window `d`, in pixels.
    fn app_px(&self, d: &DisplayInfo) -> (u16, u16) {
        let f = self.f();
        match self.preview {
            None => (
                d.width,
                (i32::from(d.height) - self.strip_px()).max(1) as u16,
            ),
            Some((w, h)) => (
                (f32::from(w) * f).round().max(1.0) as u16,
                (f32::from(h) * f).round().max(1.0) as u16,
            ),
        }
    }

    /// Tells the app its display when it changed.
    fn relayout(&mut self) {
        let Some(d) = self.window else {
            return;
        };
        let (w, h) = self.app_px(&d);
        if self.told == Some((w, h)) {
            return;
        }
        self.told = Some((w, h));
        self.app.event(Event::Display(DisplayInfo {
            width: w,
            height: h,
            ..d
        }));
    }

    /// Where the app's frame stands now: at the window's top left for
    /// This window; centred above the strip for a preview, at its own
    /// pixels where it fits inside `MARGIN_WIDE`, else scaled down to
    /// fit. Arithmetic.
    pub(crate) fn area(&self) -> Option<Area> {
        let d = self.window?;
        let (fw, fh) = self.told?;
        let (fw, fh) = (i32::from(fw), i32::from(fh));
        if self.preview.is_none() {
            return Some(Area {
                x: 0,
                y: 0,
                w: fw,
                h: fh,
                fw,
                fh,
            });
        }
        let (ww, wh) = (i32::from(d.width), i32::from(d.height) - self.strip_px());
        let m = t::MARGIN_WIDE * self.f();
        let s = ((ww as f32 - 2.0 * m) / fw as f32)
            .min((wh as f32 - 2.0 * m) / fh as f32)
            .clamp(t::PREVIEW_SCALE_MIN, 1.0);
        let (w, h) = (
            (fw as f32 * s).round().max(1.0) as i32,
            (fh as f32 * s).round().max(1.0) as i32,
        );
        Some(Area {
            x: (ww - w) / 2,
            y: ((wh - h) / 2).max(0),
            w,
            h,
            fw,
            fh,
        })
    }

    fn hit_at(&self, x: i32, y: i32) -> Option<HostPress> {
        self.hits
            .iter()
            .rev()
            .find(|(r, _)| r.contains(x, y))
            .map(|(_, p)| *p)
    }

    fn close_menu(&mut self) {
        if let Some((m, _)) = self.menu.take()
            && self.app.fades()
        {
            self.leaving = Some((m, self.now));
        }
    }

    /// What a press on the host's control `p` does.
    fn activate(&mut self, p: HostPress) {
        match p {
            HostPress::App(a) => {
                // A tick on Samples' sheet keeps it open; Load closes it.
                let ticks = matches!(a, Action::SampleTick(_) | Action::SamplesAll);
                if !(ticks && self.menu_open() == Some(HostMenu::Samples)) {
                    self.close_menu();
                }
                self.app.press(a);
            }
            HostPress::Menu(m) if self.menu.map(|o| o.0) == Some(m) => self.close_menu(),
            HostPress::Menu(m) => {
                self.list_top();
                if m == HostMenu::Samples {
                    // The app lists the samples as its own sheet would,
                    // and opens none while held.
                    self.app.press(Action::Samples);
                }
                self.leaving = None;
                self.menu = Some((m, Some(self.now)));
            }
            HostPress::Sheet => {}
            HostPress::Size(size) => {
                self.close_menu();
                self.set_preview(size);
                // The shell keeps the size across a lock.
                self.out.push_back(Command::PreviewSize { size });
            }
        }
    }

    fn touch(&mut self, x: u16, y: u16, phase: TouchPhase) {
        let (px, py) = (i32::from(x), i32::from(y));
        let area = self.area();
        // A press on the open menu or sheet may scroll its rows, one on
        // the strip drag its row sideways, where they scroll.
        if self.list_touch(px, py, phase) || self.strip_touch(px, py, phase) {
            return;
        }
        match phase {
            TouchPhase::Down => {
                // A press goes where the pointer is, and the keys with it.
                self.pointer_in = self.hit_at(px, py).is_none()
                    && self.menu.is_none()
                    && area.is_some_and(|a| a.inside(px, py));
                self.keys_in = self.pointer_in;
                if let Some(h) = self.hit_at(px, py) {
                    self.held = Held::Host;
                    self.pressed = Some(h);
                } else if self.menu.is_some() {
                    // A press outside an open menu closes it, and does
                    // nothing else (Material's menu).
                    self.close_menu();
                    self.held = Held::Nowhere;
                } else if let Some(a) = area.filter(|a| a.inside(px, py)) {
                    self.held = Held::App;
                    let (x, y) = a.to_app(px, py);
                    self.app.event(Event::Touch { x, y, phase });
                } else {
                    self.held = Held::Nowhere;
                }
            }
            TouchPhase::Move => {
                if self.held == Held::App
                    && let Some(a) = area
                {
                    let (x, y) = a.to_app(px, py);
                    self.app.event(Event::Touch { x, y, phase });
                }
            }
            TouchPhase::Up => {
                match self.held {
                    Held::App => {
                        if let Some(a) = area {
                            let (x, y) = a.to_app(px, py);
                            self.app.event(Event::Touch { x, y, phase });
                        }
                    }
                    Held::Host => {
                        let hit = self.hit_at(px, py);
                        if let Some(p) = hit.filter(|h| Some(*h) == self.pressed) {
                            self.activate(p);
                        }
                    }
                    Held::None | Held::Nowhere => {}
                }
                self.pressed = None;
                self.held = Held::None;
            }
        }
    }

    /// Costs the app's frame and the strip's drawing. Paints the app and,
    /// over it, the strip, a preview's edge and the menu or sheet open;
    /// the app's frame is copied into the window's where `whole`, else
    /// only for a preview or where something of the host's stands over
    /// it. Returns where the app's frame stands where it was left out:
    /// then only the strip is drawn, on its own canvas.
    fn paint(&mut self, whole: bool) -> Option<Area> {
        // A press made on the app itself may show or hide the roller,
        // which the strip grows for.
        self.relayout();
        let d = self.window?;
        let area = self.area();
        // Only at the window's own size: a preview's frame is small, and
        // placing it costs less than copying the window round it in parts.
        let unplaced = area.filter(|a| {
            !whole
                && self.preview.is_none()
                && a.w == a.fw
                && a.h == a.fh
                && a.x == 0
                && a.y == 0
                && self.menu.is_none()
                && self.leaving.is_none()
        });
        let top = if unplaced.is_some() {
            i32::from(d.height) - self.strip_px()
        } else {
            0
        };
        // Only the strip moves (a pill pulsing), no menu stands over the
        // app, and nothing else changed since the window's whole frame was
        // last drawn: the strip alone is drawn again over it, so the pulse
        // costs a strip, not the window (`docs/DESKTOP-HOST.md` §9).
        let strip_only = unplaced.is_none()
            && self.menu.is_none()
            && self.leaving.is_none()
            && !self.stale
            && !self.ink_on
            && !self.drawn_whole_moving
            && !self.whole_moving()
            && !self.app.paints_next()
            && self.canvas.as_ref().is_some_and(|c| {
                c.width() == i32::from(d.width) && c.height() == i32::from(d.height)
            });
        let mut canvas = self.canvas_for(&d, unplaced.is_some());
        // At the window's own size the app's frame covers all above the
        // strip, which the strip's drawing fills: only a preview needs
        // the host's space round its frame.
        if let Some(c) = canvas.as_mut()
            && !strip_only
        {
            if self.preview.is_some() || area.is_none() {
                c.clear(screen::Look::now(self).surround);
            } else if self.ink_on && unplaced.is_none() {
                // What the host draws this frame alone.
                c.record_ink(true);
            }
        }
        if let Some(a) = area
            && !strip_only
        {
            let _ = self.app.frame();
            self.edge_on_app();
            let frame = painted(&self.app);
            if unplaced.is_none()
                && let Some(c) = canvas.as_mut()
            {
                c.place_scaled((a.x, a.y, a.w, a.h), &frame);
            }
        }
        let mut hits = std::mem::take(&mut self.hits);
        hits.clear();
        if let Some(c) = canvas.as_mut()
            && !self.bare
        {
            if strip_only {
                let h = self.strip_px();
                c.push_clip(Rect::new(0, i32::from(d.height) - h, i32::from(d.width), h));
            }
            let mut ui = faraday_ui::Ui::new(c, self.f(), &mut hits, self.pressed);
            ui.hovered = self.hovered;
            let drawn = screen::draw(self, &mut ui, area, self.strip_scroll);
            self.labels = drawn.labels;
            self.sheet_view = drawn.sheet;
            self.strip_max = drawn.strip_max;
            self.strip_scroll = self.strip_scroll.min(drawn.strip_max);
            if strip_only {
                c.pop_clip();
            }
        }
        // Drawn on the strip's own canvas, its presses stand where the
        // strip stands in the window.
        for (r, _) in &mut hits {
            r.y += top;
        }
        self.hits = hits;
        self.drawn_moving = self.moving();
        if unplaced.is_none() && !strip_only {
            self.stale = false;
            self.drawn_whole_moving = self.whole_moving();
        }
        if unplaced.is_some() {
            self.stale = true;
        }
        if self.drawn_moving {
            self.out.push_back(Command::Draw);
        }
        if unplaced.is_some() {
            self.strip = canvas;
        } else {
            self.canvas = canvas;
        }
        unplaced
    }

    /// The canvas this frame is drawn on, for the window `d`: the strip's
    /// own where `strip_only` (none held bare, which has no strip), the
    /// whole window's canvas dropped; else the whole window's. Costs a
    /// canvas's allocation where it is not held at its size.
    fn canvas_for(&mut self, d: &DisplayInfo, strip_only: bool) -> Option<Canvas> {
        let scale = faraday_draw::Scale::for_class(d.dpi, faraday_draw::SizeClass::of(d));
        if strip_only {
            self.canvas = None;
            let h = u16::try_from(self.strip_px()).ok().filter(|&h| h > 0)?;
            let held = self
                .strip
                .take()
                .filter(|c| c.width() == i32::from(d.width) && c.height() == i32::from(h));
            return Some(held.unwrap_or_else(|| {
                let mut c = Canvas::part(d.width, h, scale);
                c.set_counter(crate::work::core_count);
                c
            }));
        }
        let held = self
            .canvas
            .take()
            .filter(|c| c.width() == i32::from(d.width) && c.height() == i32::from(d.height));
        Some(held.unwrap_or_else(|| {
            let mut c = Canvas::part(d.width, d.height, scale);
            c.record_ink(self.ink_on);
            c.set_counter(crate::work::core_count);
            // What no part covers is the host's own space.
            c.clear(screen::Look::now(self).surround);
            c
        }))
    }

    /// Whether the host's own motion runs: a menu fading in or out, the
    /// focus border changing colour, a stick pill pulsing, none of them
    /// under Reduce motion or with fades off (`docs/MOTION.md` §3.5); a
    /// list coasting or springing back, which Reduce motion stops at the
    /// finger's lift.
    fn moving(&self) -> bool {
        self.whole_moving() || self.pulsing() || self.lists_moving()
    }

    /// Whether a menu fades in or out or the focus border changes colour:
    /// motion over or round the app's frame, which a frame draws whole.
    fn whole_moving(&self) -> bool {
        if !self.app.fades() {
            return false;
        }
        let opening = self
            .menu
            .and_then(|(_, at)| at)
            .is_some_and(|at| self.now.saturating_sub(at) < crate::motion::MENU_IN_MS);
        let closing = self
            .leaving
            .is_some_and(|(_, at)| self.now.saturating_sub(at) < crate::motion::MENU_OUT_MS);
        let edge = self
            .edge_move
            .is_some_and(|(_, at)| self.now.saturating_sub(at) < t::HOST_FOCUS_FADE_MS);
        opening || closing || edge
    }

    /// Whether a stick pill the app asks for is pulsing: its pulses not
    /// yet run, fades on. Arithmetic.
    fn pulsing(&self) -> bool {
        !self.asked.is_empty()
            && self.app.fades()
            && self.now.saturating_sub(self.pulse_at) < t::HOST_PULSE_MS * t::HOST_PULSES
    }

    /// The primary layer on a stick pill the app asks for now: rising and
    /// falling from the steady highlight while it pulses, steady after,
    /// under Reduce motion and with fades off. Arithmetic.
    pub(crate) fn mark_layer(&self) -> f32 {
        if !self.pulsing() {
            return t::HOST_MARK_LAYER;
        }
        let into = self.now.saturating_sub(self.pulse_at) % t::HOST_PULSE_MS;
        let rise = (std::f32::consts::PI * into as f32 / t::HOST_PULSE_MS as f32).sin();
        t::HOST_MARK_LAYER + (t::HOST_PULSE_LAYER - t::HOST_MARK_LAYER) * rise
    }

    /// Whether the pointer or the keys reach the app now: the focus
    /// border's target. A lookup.
    fn focus_now(&self) -> bool {
        self.window_focus && (self.pointer_in || self.keys_in)
    }

    /// How lit the focus border is now, 0 (outline-variant) to 1
    /// (primary): on Material's standard curve over
    /// `HOST_FOCUS_FADE_MS` while it changes. Arithmetic.
    pub(crate) fn edge_shown(&self) -> f32 {
        let to = if self.edge_on { 1.0 } else { 0.0 };
        match self.edge_move {
            Some((from, at)) if self.app.fades() => {
                let p = crate::motion::progress(Some(at), self.now, t::HOST_FOCUS_FADE_MS);
                from + (to - from) * crate::motion::ease_move(p)
            }
            _ => to,
        }
    }

    /// Starts the focus border's change where the pointer or the keys
    /// moved into or out of the app, from how lit it is now.
    fn sync_focus(&mut self) {
        let on = self.focus_now();
        if on == self.edge_on {
            return;
        }
        let from = self.edge_shown();
        self.edge_on = on;
        self.edge_move = self.app.fades().then_some((from, self.now));
        self.out.push_back(Command::Draw);
    }

    /// Notes the sticks the app asks for now (`Pill::asked`); where one
    /// is newly asked for, the pulse starts again. A lookup of the pills.
    fn sync_asked(&mut self) {
        if self.bare || !self.app.shell_sticks {
            return;
        }
        let asked: Vec<usize> = self
            .app
            .stick_pills()
            .into_iter()
            .filter(|p| p.asked)
            .map(|p| p.n)
            .collect();
        if asked == self.asked {
            return;
        }
        if asked.iter().any(|n| !self.asked.contains(n)) {
            self.pulse_at = self.now;
        }
        self.asked = asked;
        self.out.push_back(Command::Draw);
    }

    /// The focus border round the app's frame at the window's own size,
    /// `HOST_FOCUS_EDGE` wide inside its edge, drawn on the app's frame
    /// as it stands: the app's primary while the pointer or the keys
    /// reach it, its outline-variant while they do not. Costs four thin
    /// fills.
    fn edge_on_app(&mut self) {
        if self.bare || self.preview.is_some() {
            return;
        }
        let theme = self.app.theme();
        let color = theme
            .role(faraday_ui::Role::OutlineVariant)
            .mix(theme.role(faraday_ui::Role::Primary), self.edge_shown());
        let e = edge_px(self.f());
        let Some(c) = self.app.canvas.as_mut() else {
            return;
        };
        self.edge_drawn = Some((e, color));
        let (w, h) = (c.width(), c.height());
        for r in [
            Rect::new(0, 0, w, e),
            Rect::new(0, h - e, w, e),
            Rect::new(0, e, e, h - 2 * e),
            Rect::new(w - e, e, e, h - 2 * e),
        ] {
            c.fill_rect(r, color);
        }
    }

    /// A tick: the host's motion moved on, the app ticked.
    fn tick(&mut self, now_ms: u64) {
        self.now = now_ms;
        if self
            .leaving
            .is_some_and(|(_, at)| now_ms.saturating_sub(at) >= crate::motion::MENU_OUT_MS)
        {
            self.leaving = None;
        }
        if self
            .edge_move
            .is_some_and(|(_, at)| now_ms.saturating_sub(at) >= t::HOST_FOCUS_FADE_MS)
        {
            self.edge_move = None;
        }
        self.app.event(Event::Tick { now_ms });
        self.relayout();
        self.sync_asked();
        // A list coasting moves on; the frame showing it is asked for
        // below, while it moves and once after.
        self.lists_tick(now_ms);
        // A tick asks for a frame only while the host's own motion
        // runs (a menu fading, the focus border, a pill pulsing, a list
        // coasting),
        // and for the one after that shows it at rest; the app
        // asks for its own through `poll_command`. An idle window
        // draws nothing (`docs/DESKTOP-HOST.md` §9).
        if self.drawn_moving || self.moving() {
            self.out.push_back(Command::Draw);
        }
    }

    /// The pointer moved over the window with no button down, at (`x`,
    /// `y`): a host control hovered, or the app's frame at its own pixel.
    fn hover(&mut self, x: u16, y: u16) {
        let (px, py) = (i32::from(x), i32::from(y));
        self.hovered = self.hit_at(px, py);
        match self.area().filter(|a| a.inside(px, py)) {
            Some(a) if self.hovered.is_none() => {
                self.pointer_in = true;
                let (x, y) = a.to_app(px, py);
                self.app.event(Event::Hover { x, y });
            }
            _ => {
                // On the strip, a menu or the host's space: the
                // keys go to the app only once a key is pressed.
                self.pointer_in = false;
                self.keys_in = false;
                self.app.event(Event::HoverEnd);
            }
        }
    }
}

/// The focus border's width in the window's pixels at `f` pixels a dp:
/// one at least. Arithmetic.
pub(crate) fn edge_px(f: f32) -> i32 {
    (t::HOST_FOCUS_EDGE * f).round().max(1.0) as i32
}

/// The app's frame as last painted, without painting: the host has had
/// [`App::frame`] paint it this frame. A lookup.
fn painted(app: &Faraday) -> Frame<'_> {
    match app.canvas.as_ref() {
        Some(c) => c.frame(),
        None => Frame {
            width: 0,
            height: 0,
            rgba: &[],
        },
    }
}

/// The host's menus, each opening up from its control on the strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostMenu {
    /// Every stick, in or out, a choice putting it in or pulling it.
    Sticks,
    /// The preview sizes.
    Size,
    /// Play's options on Make codex32 shares: how far it runs, how fast.
    Play,
    /// The demo aids the strip has no room for, and Samples where the
    /// strip is narrowest.
    More,
    /// The computer's die's Rolls per click: 1, 5, 10, To the end, Number.
    Rolls,
    /// Load samples' sheet: the sample wallets and keys to tick, and Load
    /// ticked (`docs/DESKTOP-HOST.md` §6).
    Samples,
}

impl App for Host {
    fn event(&mut self, event: Event) {
        match event {
            Event::Display(d) => {
                if d.width == 0 || d.height == 0 {
                    return;
                }
                self.window = Some(d);
                // The canvases are made at the new size when next drawn.
                self.canvas = None;
                self.strip = None;
                self.f =
                    faraday_draw::Scale::for_class(d.dpi, faraday_draw::SizeClass::of(&d)).factor();
                self.told = None;
                self.relayout();
            }
            // The shell's size, kept across a lock.
            Event::PreviewOffered { chosen } => self.set_preview(chosen),
            Event::Tick { now_ms } => {
                self.tick(now_ms);
                return;
            }
            Event::Touch { x, y, phase } => self.touch(x, y, phase),
            Event::Hover { x, y } => self.hover(x, y),
            Event::HoverEnd => {
                self.hovered = None;
                self.pointer_in = false;
                self.keys_in = false;
                self.app.event(event);
            }
            // The window's focus is the host's: the app has no use for it.
            Event::Focus(on) => self.window_focus = on,
            Event::Key(_) => {
                self.keys_in = true;
                self.app.event(event);
            }
            Event::Scroll { x, y, dy } | Event::Wheel { x, y, dy } => {
                let (px, py) = (i32::from(x), i32::from(y));
                // How far the host's own lists move, dp.
                let step = match event {
                    // The window's pixels per dp: never zero.
                    Event::Scroll { .. } => f32::from(dy) / self.f(),
                    _ => f32::from(dy) / crate::motion::SHELL_NOTCH_PX * crate::motion::WHEEL_NOTCH,
                };
                if self.menu.is_some() {
                    // The open menu or Samples' sheet scrolls its rows
                    // under the pointer.
                    if let Some((r, max)) = self.sheet_view
                        && r.contains(px, py)
                    {
                        self.list_motion.stop();
                        self.sheet_scroll = (self.sheet_scroll + step).clamp(0.0, max);
                        self.out.push_back(Command::Draw);
                    }
                    return;
                }
                // The strip's row scrolls sideways under the wheel where
                // it is wider than the window.
                if self.on_strip(py) {
                    if self.strip_max > 0.0 {
                        self.scroll_strip(step);
                    }
                    return;
                }
                if let Some(a) = self.area().filter(|a| a.inside(px, py)) {
                    let (x, y) = a.to_app(px, py);
                    self.app.event(match event {
                        Event::Scroll { .. } => Event::Scroll { x, y, dy },
                        _ => Event::Wheel { x, y, dy },
                    });
                }
            }
            Event::ScrollEnd { x, y } => {
                if let Some(a) = self.area() {
                    let (x, y) = a.to_app(i32::from(x), i32::from(y));
                    self.app.event(Event::ScrollEnd { x, y });
                }
            }
            Event::Back if self.menu.is_some() => self.close_menu(),
            other => self.app.event(other),
        }
        // The strip grows while a roller is on the page: the app is laid
        // out again above it.
        self.relayout();
        self.sync_focus();
        self.sync_asked();
        self.stale = true;
        self.out.push_back(Command::Draw);
    }

    fn poll_command(&mut self) -> Option<Command> {
        self.out.pop_front().or_else(|| self.app.poll_command())
    }

    /// Costs the app's frame, a copy of its pixels into the window's and
    /// the strip's drawing.
    fn frame(&mut self) -> Frame<'_> {
        self.given = None;
        self.paint(true);
        match self.canvas.as_ref() {
            Some(c) => c.frame(),
            None => Frame {
                width: 0,
                height: 0,
                rgba: &[],
            },
        }
    }

    /// Costs the app's frame and the strip's drawing. At the window's own
    /// size with nothing of the host's over the app, the app's frame
    /// handed over unplaced and the strip's own canvas standing at the
    /// window's foot, so the shell copies each pixel once and the host
    /// holds no canvas the window's size (`docs/DESKTOP-HOST.md` §9);
    /// else the window's whole frame, as [`App::frame`] gives it, where a
    /// menu or Samples' sheet stands over the app and for a preview,
    /// whose small frame costs less to place than the window round it.
    ///
    /// Given unplaced as the parts before them were, they say what may
    /// have changed since ([`Parts::changed`]): where the app says its
    /// frame changed (a fade's steps), the strip, and the focus border
    /// where its colour changed; else nothing is said.
    fn parts(&mut self) -> Parts<'_> {
        let edge = if self.bare || self.preview.is_some() {
            0
        } else {
            edge_px(self.f())
        };
        self.app.set_edge_drawn_over(edge);
        let edge_before = self.edge_drawn;
        let unplaced = self.paint(false);
        let app_changed = self.app.take_changed();
        let empty = Frame {
            width: 0,
            height: 0,
            rgba: &[],
        };
        let (Some(a), Some(d)) = (unplaced, self.window) else {
            self.given = None;
            let base = self.canvas.as_ref().map_or(empty, Canvas::frame);
            return Parts {
                base,
                base_y: 0,
                inner: None,
                changed: None,
            };
        };
        let at = |v: i32| v.clamp(0, i32::from(u16::MAX)) as u16;
        let base_y = self.strip_area().map_or(at(a.h), |(_, top, _, _)| at(top));
        let layout = Some((d.width, d.height, a.x, a.y, base_y));
        let known = app_changed.filter(|_| self.given == layout);
        self.given = layout;
        self.changed.clear();
        if let Some(rects) = &known {
            let mut put = |x: i32, y: i32, w: i32, h: i32| {
                if w > 0 && h > 0 {
                    self.changed.push(Changed {
                        x: at(x),
                        y: at(y),
                        width: at(w),
                        height: at(h),
                    });
                }
            };
            for r in rects {
                put(a.x + r.x, a.y + r.y, r.w, r.h);
            }
            let strip_h = self.strip.as_ref().map_or(0, Canvas::height);
            put(0, i32::from(base_y), i32::from(d.width), strip_h);
            if self.edge_drawn != edge_before {
                let (w, h) = (a.w, a.h);
                put(a.x, a.y, w, edge);
                put(a.x, a.y + h - edge, w, edge);
                put(a.x, a.y, edge, h);
                put(a.x + w - edge, a.y, edge, h);
            }
        }
        Parts {
            base: self.strip.as_ref().map_or(empty, Canvas::frame),
            base_y,
            inner: Some((painted(&self.app), at(a.x), at(a.y))),
            changed: known.is_some().then_some(self.changed.as_slice()),
        }
    }
}
