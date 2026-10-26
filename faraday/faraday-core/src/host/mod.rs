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

use std::collections::VecDeque;

use faraday_draw::Canvas;
use faraday_draw::geom::Rect;
use faraday_draw::tokens as t;
use faraday_shell_api::{App, Command, DisplayInfo, Event, Frame, Parts, TouchPhase};

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
    canvas: Option<Canvas>,
    hits: Vec<(Rect, HostPress)>,
    pressed: Option<HostPress>,
    hovered: Option<HostPress>,
    /// The menu open, and when it opened; `None`, opened at rest, whole
    /// whatever the clock says.
    menu: Option<(HostMenu, Option<u64>)>,
    /// The menu closing, and when it began to.
    leaving: Option<(HostMenu, u64)>,
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
    /// How far Samples' sheet's lists are scrolled, dp.
    sheet_scroll: f32,
    /// Samples' sheet as last drawn: where it stands in the window's
    /// pixels and the furthest its lists scroll, dp.
    sheet_view: Option<(Rect, f32)>,
    /// How far the strip's row is scrolled sideways, dp, where it is wider
    /// than the window; the furthest it scrolls in the frame last drawn.
    strip_scroll: f32,
    strip_max: f32,
    /// A press begun on the strip: where, in the window's pixels, the
    /// strip's scroll then, and whether it has moved past the slop, so
    /// that it drags the row and presses nothing.
    drag: Option<(i32, f32, bool)>,
    /// Recording where the host's text and icons leave their ink
    /// ([`Host::record_ink`]).
    ink_on: bool,
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
            hits: Vec::new(),
            pressed: None,
            hovered: None,
            menu: None,
            leaving: None,
            held: Held::None,
            now: 0,
            f: 1.0,
            out: VecDeque::new(),
            labels: Vec::new(),
            bare: false,
            sheet_scroll: 0.0,
            sheet_view: None,
            strip_scroll: 0.0,
            strip_max: 0.0,
            drag: None,
            ink_on: false,
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
        self.relayout();
    }

    /// Opens `menu` at once, as a script does: for drivers that take
    /// pictures of the screens.
    pub fn open_menu_at_rest(&mut self, menu: HostMenu) {
        if menu == HostMenu::Samples {
            self.sheet_scroll = 0.0;
            self.app.press(Action::Samples);
        }
        self.menu = Some((menu, None));
        self.leaving = None;
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
            && !self.app.reduce_motion()
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
                if m == HostMenu::Samples {
                    // The app lists the samples as its own sheet would,
                    // and opens none while held.
                    self.sheet_scroll = 0.0;
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
        // A press on the strip may drag its row sideways where it scrolls.
        if phase == TouchPhase::Down {
            self.drag = (self.menu.is_none() && self.on_strip(py) && self.strip_max > 0.0)
                .then_some((px, self.strip_scroll, false));
        }
        if let Some((x0, s0, moved)) = self.drag {
            let dx = px - x0;
            let past = moved || (dx.abs() as f32) > crate::motion::SLOP * self.f();
            if past && phase != TouchPhase::Down {
                self.drag = Some((x0, s0, true));
                self.pressed = None;
                self.held = Held::Nowhere;
                self.strip_scroll = (s0 - dx as f32 / self.f()).clamp(0.0, self.strip_max);
                self.out.push_back(Command::Draw);
                if phase == TouchPhase::Up {
                    self.drag = None;
                    self.held = Held::None;
                }
                return;
            }
            if phase == TouchPhase::Up {
                self.drag = None;
            }
        }
        match phase {
            TouchPhase::Down => {
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
    /// it. Returns where the app's frame stands where it was left out.
    fn paint(&mut self, whole: bool) -> Option<Area> {
        // A press made on the app itself may show or hide the roller,
        // which the strip grows for.
        self.relayout();
        let mut canvas = self.canvas.take()?;
        let area = self.area();
        // Only at the window's own size: a preview's frame is small, and
        // placing it costs less than copying the window round it in parts.
        let unplaced = area.filter(|a| {
            !whole
                && self.preview.is_none()
                && a.w == a.fw
                && a.h == a.fh
                && a.x >= 0
                && a.y >= 0
                && self.menu.is_none()
                && self.leaving.is_none()
        });
        // At the window's own size the app's frame covers all above the
        // strip, which the strip's drawing fills: only a preview needs
        // the host's space round its frame.
        if self.preview.is_some() || area.is_none() {
            canvas.clear(screen::Look::now(self).surround);
        } else if self.ink_on {
            // What the host draws this frame alone.
            canvas.record_ink(true);
        }
        if let Some(a) = area {
            let frame = self.app.frame();
            if unplaced.is_none() {
                canvas.place_scaled((a.x, a.y, a.w, a.h), &frame);
            }
        }
        let mut hits = std::mem::take(&mut self.hits);
        hits.clear();
        {
            let mut ui = faraday_ui::Ui::new(&mut canvas, self.f(), &mut hits, self.pressed);
            ui.hovered = self.hovered;
            if !self.bare {
                let drawn = screen::draw(self, &mut ui, area, self.strip_scroll);
                self.labels = drawn.labels;
                self.sheet_view = drawn.sheet;
                self.strip_max = drawn.strip_max;
                self.strip_scroll = self.strip_scroll.min(drawn.strip_max);
            }
        }
        self.hits = hits;
        if self.moving() {
            self.out.push_back(Command::Draw);
        }
        self.canvas = Some(canvas);
        unplaced
    }

    /// Whether a menu is part of the way in or out.
    fn moving(&self) -> bool {
        if self.app.reduce_motion() {
            return false;
        }
        let opening = self
            .menu
            .and_then(|(_, at)| at)
            .is_some_and(|at| self.now.saturating_sub(at) < crate::motion::MENU_IN_MS);
        let closing = self
            .leaving
            .is_some_and(|(_, at)| self.now.saturating_sub(at) < crate::motion::MENU_OUT_MS);
        opening || closing
    }
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
                let mut canvas = Canvas::new(&d);
                canvas.record_ink(self.ink_on);
                // What no part covers is the host's own space.
                canvas.clear(screen::Look::now(self).surround);
                self.f = canvas.scale().factor();
                canvas.set_counter(crate::work::core_count);
                self.canvas = Some(canvas);
                self.told = None;
                self.relayout();
            }
            // The shell's size, kept across a lock.
            Event::PreviewOffered { chosen } => self.set_preview(chosen),
            Event::Tick { now_ms } => {
                self.now = now_ms;
                if self
                    .leaving
                    .is_some_and(|(_, at)| now_ms.saturating_sub(at) >= crate::motion::MENU_OUT_MS)
                {
                    self.leaving = None;
                }
                self.app.event(event);
            }
            Event::Touch { x, y, phase } => self.touch(x, y, phase),
            Event::Hover { x, y } => {
                let (px, py) = (i32::from(x), i32::from(y));
                self.hovered = self.hit_at(px, py);
                match self.area().filter(|a| a.inside(px, py)) {
                    Some(a) if self.hovered.is_none() => {
                        let (x, y) = a.to_app(px, py);
                        self.app.event(Event::Hover { x, y });
                    }
                    _ => self.app.event(Event::HoverEnd),
                }
            }
            Event::HoverEnd => {
                self.hovered = None;
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
                    // Samples' sheet scrolls its lists under the pointer.
                    if self.menu_open() == Some(HostMenu::Samples)
                        && let Some((r, max)) = self.sheet_view
                        && r.contains(px, py)
                    {
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
        self.out.push_back(Command::Draw);
    }

    fn poll_command(&mut self) -> Option<Command> {
        self.out.pop_front().or_else(|| self.app.poll_command())
    }

    /// Costs the app's frame, a copy of its pixels into the window's and
    /// the strip's drawing.
    fn frame(&mut self) -> Frame<'_> {
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

    /// Costs the app's frame and the strip's drawing. The window's frame
    /// with the app's handed over unplaced at the window's own size with
    /// nothing of the host's over it, so the shell copies each pixel once
    /// (`docs/DESKTOP-HOST.md` §9); placed, as [`App::frame`] gives it,
    /// where a menu or Samples' sheet stands over it and for a preview,
    /// whose small frame costs less to place than the window round it.
    fn parts(&mut self) -> Parts<'_> {
        let unplaced = self.paint(false);
        let base = match self.canvas.as_ref() {
            Some(c) => c.frame(),
            None => Frame {
                width: 0,
                height: 0,
                rgba: &[],
            },
        };
        let inner = unplaced.map(|a| {
            let at = |v: i32| v.clamp(0, i32::from(u16::MAX)) as u16;
            (painted(&self.app), at(a.x), at(a.y))
        });
        Parts { base, inner }
    }
}
