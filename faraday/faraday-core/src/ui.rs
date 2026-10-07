//! Drawing in design units.
//!
//! Every screen is laid out on a canvas of at least 1280 × 800 design
//! units (the prototype's frame), scaled to fill the display, with the
//! prototype's palette (`design/prototype/`). Text is `osk-ui`'s Noto
//! Sans and Noto Sans Mono: the Inter and JetBrains Mono of `PLAN.md` §9.2
//! wait for `faraday-ui`.

use osk_ui::fonts::Family;
use osk_ui::text::{self, TextAlign};
use osk_ui::widgets::Icon;
use osk_ui::{Canvas, Color, Font, Rect};

use crate::Action;

/// The prototype's palette.
pub mod pal {
    use osk_ui::Color;

    /// The page.
    pub const BG: Color = Color::rgb(0x11, 0x16, 0x1c);
    /// The sidebar.
    pub const SIDEBAR: Color = Color::rgb(0x0d, 0x12, 0x18);
    /// A card.
    pub const SURFACE: Color = Color::rgb(0x19, 0x21, 0x2a);
    /// A card's edge and the sidebar's.
    pub const LINE: Color = Color::rgb(0x1f, 0x29, 0x33);
    /// A rule inside a card, and an idle badge.
    pub const INNER: Color = Color::rgb(0x22, 0x2c, 0x37);
    /// A secondary button's edge.
    pub const BORDER: Color = Color::rgb(0x30, 0x3d, 0x49);
    /// Text.
    pub const TEXT: Color = Color::rgb(0xe0, 0xe9, 0xef);
    /// Labels.
    pub const MUTED: Color = Color::rgb(0x9a, 0xac, 0xb9);
    /// What is not there yet.
    pub const DIM: Color = Color::rgb(0x6f, 0x82, 0x91);
    /// The accent.
    pub const ACCENT: Color = Color::rgb(0x83, 0xd8, 0xef);
    /// Text on the accent. One step from `SIDEBAR`, so the light theme
    /// can tell them apart.
    pub const ON_ACCENT: Color = Color::rgb(0x0d, 0x12, 0x19);
    /// Done, verified, can sign.
    pub const OK: Color = Color::rgb(0x8b, 0xd4, 0xb2);
    /// Waiting, unsaved, needs attention.
    pub const WARN: Color = Color::rgb(0xf0, 0xc0, 0x77);
    /// Refused.
    pub const ERR: Color = Color::rgb(0xef, 0x9b, 0x9b);

    /// Each colour above and what it is in the light theme
    /// (`docs/MOTION.md` §3.6).
    pub const LIGHT: [(Color, Color); 14] = [
        (BG, Color::rgb(0xf4, 0xf6, 0xf8)),
        (SIDEBAR, Color::rgb(0xe9, 0xed, 0xf1)),
        (SURFACE, Color::rgb(0xff, 0xff, 0xff)),
        (LINE, Color::rgb(0xdc, 0xe2, 0xe8)),
        (INNER, Color::rgb(0xe8, 0xed, 0xf1)),
        (BORDER, Color::rgb(0xc6, 0xd0, 0xd8)),
        (TEXT, Color::rgb(0x15, 0x1c, 0x23)),
        (MUTED, Color::rgb(0x4f, 0x5d, 0x69)),
        (DIM, Color::rgb(0x7a, 0x88, 0x94)),
        (ACCENT, Color::rgb(0x0b, 0x7a, 0x9e)),
        (ON_ACCENT, Color::rgb(0xff, 0xff, 0xff)),
        (OK, Color::rgb(0x1d, 0x85, 0x56)),
        (WARN, Color::rgb(0x9a, 0x5d, 0x00)),
        (ERR, Color::rgb(0xbf, 0x34, 0x34)),
    ];
}

use pal::*;

/// Which palette the screens are drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    /// Light text on a dark page: the prototype's palette.
    #[default]
    Dark,
    /// Dark text on a light page.
    Light,
}

impl Theme {
    /// `c`, one of [`pal`]'s colours at any opacity, as this theme draws
    /// it. Any other colour, a QR code's black and white or a camera's
    /// picture, is drawn as it is.
    pub fn color(self, c: Color) -> Color {
        if self == Theme::Dark {
            return c;
        }
        pal::LIGHT
            .iter()
            .find(|(dark, _)| (dark.r, dark.g, dark.b) == (c.r, c.g, c.b))
            .map_or(c, |(_, light)| light.with_alpha(c.a))
    }
}

/// A face: text, emphasis, or data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W {
    /// Noto Sans.
    R,
    /// Noto Sans SemiBold.
    S,
    /// Noto Sans Mono.
    M,
}

/// How a button looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// The one action a screen leads with.
    Primary,
    /// Any other action.
    Secondary,
    /// A quiet action beside others.
    Ghost,
    /// Shown but not available now.
    Disabled,
}

/// One frame's drawing context.
pub struct Ui<'a> {
    /// The canvas.
    pub c: &'a mut Canvas,
    /// Pixels per design unit.
    pub f: f32,
    /// The canvas's type ramp over its length scale.
    tr: f32,
    /// Where each action can be pressed, in pixels.
    pub hits: &'a mut Vec<(Rect, Action)>,
    /// The action under a finger that is down.
    pub pressed: Option<Action>,
    /// When set, hits are only taken inside this layer (a sheet).
    pub layer: u8,
    /// Where design (0, 0) is, in design units: screens are drawn in a
    /// box that may sit inside a larger window.
    pub ox: f32,
    /// See [`Ui::ox`].
    pub oy: f32,
    /// All of the focused field is selected, and is drawn so.
    pub select_all: bool,
    /// The region this frame scrolls, as the screen or sheet that drew
    /// it last reported it.
    pub scrolled: Option<Scrolled>,
    /// How far the scrolling region is shown stretched past an end, in
    /// pixels: negative when its content is pulled down past its top.
    pub stretch: i32,
    /// The stretch belongs to a sheet's region rather than the screen's.
    pub stretch_in_sheet: bool,
    /// What is being drawn now is a sheet.
    pub in_sheet: bool,
    /// The overlay scrollbar for the scrolling region, when it shows:
    /// its opacity (0–255) and the region's offset in units.
    pub bar: Option<(u8, f32)>,
    /// The palette.
    pub theme: Theme,
    /// The action under the pointer, with no button down.
    pub hovered: Option<Action>,
    /// How far below its place a sheet is drawn while it rises in, units.
    pub sheet_rise: f32,
    /// The scrolling region's offset, units.
    pub offset: f32,
    /// The frosted copy of the page under the open sheet, kept from the
    /// frame the sheet opened on.
    pub frost: Option<Vec<u8>>,
    /// Where the Guided switch's pill is: 0 on Steps only, 1 on Guided.
    pub guided_shown: f32,
    /// A step card opening, and the one closing, while they move.
    pub disclosure: Option<Disclosure>,
    /// What the step column drew: which card is open and how tall its
    /// body is, units.
    pub column: Option<(Option<usize>, f32)>,
    /// Where the step column wants to glide to, to bring the open card
    /// into view, units.
    pub follow_to: Option<f32>,
    /// Hits are clipped to this rather than to the drawing's clip while
    /// a card's body is revealed: what is not yet uncovered can still be
    /// pressed where it will be.
    hit_clip: Option<Rect>,
    /// The hit being taken draws its own hover look.
    quiet_hover: bool,
}

/// A step card opening and the one closing, part of the way.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Disclosure {
    /// The card opening.
    pub open: Option<usize>,
    /// The card closing, and how tall its body was, units.
    pub closing: Option<(usize, f32)>,
    /// How far through, 0 to 1, eased.
    pub shown: f32,
}

/// A scrolled region as drawn: where it is and how far it goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scrolled {
    /// Where it is seen on the panel, in pixels.
    pub view: Rect,
    /// How far its content scrolls, in design units.
    pub max: f32,
    /// It draws its own scrollbar.
    pub own_bar: bool,
}

impl<'a> Ui<'a> {
    /// A context over `c`.
    pub fn new(
        c: &'a mut Canvas,
        f: f32,
        hits: &'a mut Vec<(Rect, Action)>,
        pressed: Option<Action>,
    ) -> Self {
        let s = c.scale();
        let tr = s.type_factor() / s.factor();
        Ui {
            c,
            f,
            tr,
            hits,
            pressed,
            layer: 0,
            ox: 0.0,
            oy: 0.0,
            select_all: false,
            scrolled: None,
            stretch: 0,
            stretch_in_sheet: false,
            in_sheet: false,
            bar: None,
            theme: Theme::Dark,
            hovered: None,
            sheet_rise: 0.0,
            offset: 0.0,
            frost: None,
            guided_shown: 1.0,
            disclosure: None,
            column: None,
            follow_to: None,
            hit_clip: None,
            quiet_hover: false,
        }
    }

    /// `c` in this frame's theme.
    pub fn col(&self, c: Color) -> Color {
        self.theme.color(c)
    }

    /// Fills the whole frame with `c`.
    pub fn clear(&mut self, c: Color) {
        let c = self.col(c);
        self.c.clear(c);
    }

    /// Says that this frame scrolls the region seen through `view`
    /// (pixels), whose content scrolls as far as `max` units. Called once
    /// the region's content is drawn: a stretch past an end
    /// (`docs/MOTION.md` §3.3) moves what was drawn in it, and what is
    /// pressable there, and the page shows in the gap.
    pub fn report_scroll(&mut self, view: Rect, max: f32) {
        self.report(view, max, false);
    }

    /// [`Ui::report_scroll`], for a region that draws its own scrollbar.
    pub fn report_scroll_own_bar(&mut self, view: Rect, max: f32) {
        self.report(view, max, true);
    }

    fn report(&mut self, view: Rect, max: f32, own_bar: bool) {
        let max = max.max(0.0);
        self.scrolled = Some(Scrolled { view, max, own_bar });
        if self.in_sheet != self.stretch_in_sheet {
            return;
        }
        self.stretch_view(view);
        self.edge_fades(view, max);
        if let Some((alpha, offset)) = self.bar
            && !own_bar
            && max > 0.0
            && alpha > 0
        {
            self.scroll_bar(view, max, offset, alpha);
        }
    }

    /// The overlay scrollbar: a thin thumb at the region's right edge,
    /// as long as the share of the content in view.
    fn scroll_bar(&mut self, view: Rect, max: f32, offset: f32, alpha: u8) {
        let f = self.f;
        let inset = (4.0 * f).round() as i32;
        let track = view.h - 2 * inset;
        let vh = view.h as f32;
        let thumb = ((track as f32 * vh / (vh + max * f)) as i32)
            .max((30.0 * f) as i32)
            .min(track);
        if track <= 0 || thumb <= 0 {
            return;
        }
        let at = ((track - thumb) as f32 * (offset / max).clamp(0.0, 1.0)).round() as i32;
        // Shortened by what the stretch pulls past the end, as a native
        // bar is.
        let squeeze = self.stretch.abs().min(thumb / 2);
        let (y, h) = if self.stretch < 0 {
            (view.y + inset, thumb - squeeze)
        } else {
            (
                view.y + inset + at + squeeze.min(track - thumb - at).max(0),
                thumb - squeeze,
            )
        };
        let w = (4.0 * f).round().max(2.0) as i32;
        let rect = Rect::new(view.right() - inset - w, y, w, h);
        let color = self.col(MUTED.with_alpha((u16::from(alpha) * 150 / 255) as u8));
        self.c.fill_rounded_rect(rect, w as f32 / 2.0, color);
    }

    /// The region's content fades into the page at an edge it continues
    /// past: under the top once scrolled, above the bottom while there is
    /// more. Each fade comes in over the first 24 units scrolled.
    fn edge_fades(&mut self, view: Rect, max: f32) {
        if max <= 0.0 || view.h <= 0 {
            return;
        }
        let band = ((24.0 * self.f).round() as i32).clamp(1, view.h / 4 + 1);
        let page = self.col(if self.in_sheet { SURFACE } else { BG });
        let reach = band as f32 / self.f;
        let top = (self.offset / reach).clamp(0.0, 1.0);
        let bottom = ((max - self.offset) / reach).clamp(0.0, 1.0);
        for i in 0..band {
            let k = 1.0 - (i as f32 + 0.5) / band as f32;
            let a = k * k * 230.0;
            if top > 0.0 {
                let row = Rect::new(view.x, view.y + i, view.w, 1);
                self.c.fill_rect(row, page.with_alpha((a * top) as u8));
            }
            if bottom > 0.0 {
                let row = Rect::new(view.x, view.bottom() - 1 - i, view.w, 1);
                self.c.fill_rect(row, page.with_alpha((a * bottom) as u8));
            }
        }
    }

    /// What a sheet stands on: the page, frosted (blurred once, the
    /// first frame, and kept) and dimmed.
    pub fn backdrop(&mut self) {
        match self.frost.as_deref() {
            Some(page) => self.c.restore(page),
            None => {
                // Reduced eight units to a pixel and blurred a little: as
                // soft as more blur at a fraction of the work.
                let scale = (8.0 * self.f).round().max(4.0) as u32;
                let dim = self.col(BG.with_alpha(150));
                self.c.frost(scale, 2, dim);
                // Kept dimmed: each frame after is one copy.
                self.frost = Some(self.c.snapshot());
            }
        }
    }

    /// A soft shadow under something that floats at (x, y), w by h with
    /// corners of `r`: drawn before it, in the theme's depth.
    pub fn shadow(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32) {
        let rect = self.rect(x, y, w, h);
        let color = match self.theme {
            Theme::Dark => Color::rgb(0, 0, 0).with_alpha(140),
            Theme::Light => Color::rgb(0x10, 0x18, 0x20).with_alpha(46),
        };
        let blur = 28.0 * self.f;
        let dy = (8.0 * self.f).round() as i32;
        self.c.shadow(rect, r * self.f, blur, dy, color);
    }

    /// Moves what was drawn in `view` by the stretch, and what is
    /// pressable there with it; the page shows in the gap.
    fn stretch_view(&mut self, view: Rect) {
        let n = self.stretch.abs().min(view.h);
        if n == 0 {
            return;
        }
        self.c.shift(view, -self.stretch);
        let gap = if self.stretch < 0 {
            Rect::new(view.x, view.y, view.w, n)
        } else {
            Rect::new(view.x, view.bottom() - n, view.w, n)
        };
        let page = self.col(if self.in_sheet { SURFACE } else { BG });
        self.c.fill_rect(gap, page);
        for (r, _) in self.hits.iter_mut() {
            if r.intersect(&view) == *r {
                *r = Rect::new(r.x, r.y - self.stretch, r.w, r.h).intersect(&view);
            }
        }
    }

    /// The selection behind text `shown` drawn at (x, y) in a field of
    /// height `h`, when the field is focused and all of it is selected.
    #[allow(clippy::too_many_arguments)]
    pub fn selection(&mut self, x: f32, y: f32, h: f32, size: f32, w: W, shown: &str, on: bool) {
        if on && self.select_all && !shown.is_empty() {
            let tw = self.measure(size, w, shown);
            self.fill(
                x - 2.0,
                y + h * 0.2,
                tw + 4.0,
                h * 0.6,
                3.0,
                ACCENT.with_alpha(90),
            );
        }
    }

    /// Design units to pixels.
    pub fn px(&self, v: f32) -> i32 {
        (v * self.f).round() as i32
    }

    /// A rectangle in design units, in pixels.
    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        let (x, y) = (x + self.ox, y + self.oy);
        let x0 = self.px(x);
        let y0 = self.px(y);
        Rect::new(x0, y0, self.px(x + w) - x0, self.px(y + h) - y0)
    }

    /// A filled circle.
    pub fn dot(&mut self, cx: f32, cy: f32, r: f32, color: Color) {
        let color = self.col(color);
        self.c.fill_circle(
            (cx + self.ox) * self.f,
            (cy + self.oy) * self.f,
            r * self.f,
            color,
        );
    }

    /// A filled rectangle with rounded corners.
    pub fn fill(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
        let color = self.col(color);
        let rect = self.rect(x, y, w, h);
        if r <= 0.0 {
            self.c.fill_rect(rect, color);
        } else {
            self.c.fill_rounded_rect(rect, r * self.f, color);
        }
    }

    /// An outlined rectangle with rounded corners.
    pub fn stroke(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
        let color = self.col(color);
        let rect = self.rect(x, y, w, h);
        let width = self.f.max(1.0);
        self.c.stroke_rounded_rect(rect, r * self.f, width, color);
    }

    /// A card: a surface with a one-unit edge.
    pub fn card(&mut self, x: f32, y: f32, w: f32, h: f32, edge: Color) {
        self.fill(x, y, w, h, 12.0, SURFACE);
        self.stroke(x, y, w, h, 12.0, edge);
    }

    /// A horizontal rule.
    pub fn rule(&mut self, x: f32, y: f32, w: f32, color: Color) {
        let color = self.col(color);
        let r = self.rect(x, y, w, 1.0);
        let rect = Rect::new(r.x, r.y, r.w, 1.max(self.px(1.0)));
        self.c.fill_rect(rect, color);
    }

    fn font(&self, size: f32, w: W) -> Font {
        let family = match w {
            W::R => Family::Regular,
            W::S => Family::SemiBold,
            W::M => Family::Mono,
        };
        let size = if w == W::M {
            size / self.tr * 0.94
        } else {
            size / self.tr
        };
        Font::new(family, size)
    }

    /// The width of `s` in design units.
    pub fn measure(&self, size: f32, w: W, s: &str) -> f32 {
        let face = self.font(size, w).sized(self.c.scale());
        text::width(&face, s) as f32 / self.f
    }

    /// The height of one line in design units.
    pub fn line(&self, size: f32, w: W) -> f32 {
        let face = self.font(size, w).sized(self.c.scale());
        face.line_height() as f32 / self.f
    }

    /// `s` with its top-left at (x, y). Returns its width.
    pub fn text(&mut self, x: f32, y: f32, size: f32, w: W, color: Color, s: &str) -> f32 {
        let color = self.col(color);
        let font = self.font(size, w);
        let px = self
            .c
            .text(self.px(x + self.ox), self.px(y + self.oy), s, font, color);
        px as f32 / self.f
    }

    /// `s` centred vertically in a band of height `h` starting at y.
    #[allow(clippy::too_many_arguments)]
    pub fn text_mid(
        &mut self,
        x: f32,
        y: f32,
        h: f32,
        size: f32,
        w: W,
        color: Color,
        s: &str,
    ) -> f32 {
        let lh = self.line(size, w);
        self.text(x, y + (h - lh) / 2.0, size, w, color, s)
    }

    /// `s` ending at `right`, centred in a band of height `h`.
    #[allow(clippy::too_many_arguments)]
    pub fn text_right(
        &mut self,
        right: f32,
        y: f32,
        h: f32,
        size: f32,
        w: W,
        color: Color,
        s: &str,
    ) {
        let width = self.measure(size, w, s);
        self.text_mid(right - width, y, h, size, w, color, s);
    }

    /// `s` cut with an ellipsis to fit `max` units.
    pub fn fit(&self, size: f32, w: W, s: &str, max: f32) -> String {
        if self.measure(size, w, s) <= max {
            return s.to_string();
        }
        let mut chars: Vec<char> = s.chars().collect();
        while !chars.is_empty() {
            chars.pop();
            let t: String = chars.iter().collect::<String>() + "…";
            if self.measure(size, w, &t) <= max {
                return t;
            }
        }
        String::new()
    }

    /// `s` wrapped to `width`. Returns the height used.
    #[allow(clippy::too_many_arguments)]
    pub fn wrap(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        size: f32,
        w: W,
        color: Color,
        s: &str,
    ) -> f32 {
        let color = self.col(color);
        let font = self.font(size, w);
        let rect = Rect::new(
            self.px(x + self.ox),
            self.px(y + self.oy),
            self.px(width),
            self.px(2000.0),
        );
        let px = self.c.text_wrapped(rect, s, font, color, TextAlign::Start);
        px as f32 / self.f
    }

    /// An icon centred in a square of side `boxw` at (x, y).
    pub fn icon(&mut self, x: f32, y: f32, boxw: f32, icon: Icon, size: f32, color: Color) {
        let color = self.col(color);
        let rect = self.rect(x, y, boxw, boxw);
        self.c.icon(rect, icon, size, color);
    }

    /// Makes a region pressable: the part of it inside the clip, so what
    /// is scrolled out of sight cannot be pressed.
    ///
    /// Under the pointer it shows so: a faint wash over what was drawn
    /// there, unless it is pressed, draws its own hover look (a button),
    /// or covers so much of the screen that a wash would light the page.
    pub fn hit(&mut self, x: f32, y: f32, w: f32, h: f32, action: Action) {
        let clip = self.hit_clip.unwrap_or_else(|| self.c.clip());
        let rect = self.rect(x, y, w, h).intersect(&clip);
        if rect.w > 0 && rect.h > 0 {
            let screen = i64::from(self.c.width()) * i64::from(self.c.height());
            if self.hovered == Some(action)
                && self.pressed != Some(action)
                && !self.quiet_hover
                && i64::from(rect.w) * i64::from(rect.h) * 4 < screen
            {
                let wash = self.col(TEXT.with_alpha(14));
                self.c.fill_rounded_rect(rect, 8.0 * self.f, wash);
            }
            self.hits.push((rect, action));
        }
    }

    /// Draws only inside `rect` until [`Ui::unreveal`], while what is
    /// drawn stays pressable as if it all showed.
    pub fn reveal(&mut self, rect: Rect) {
        self.hit_clip = Some(self.c.clip());
        self.c.push_clip(rect);
    }

    /// Ends [`Ui::reveal`].
    pub fn unreveal(&mut self) {
        self.c.pop_clip();
        self.hit_clip = None;
    }

    /// Whether `action` is under the pointer.
    pub fn is_hovered(&self, action: Action) -> bool {
        self.hovered == Some(action) && self.pressed != Some(action)
    }

    /// Whether `action` is under a finger.
    pub fn is_pressed(&self, action: Action) -> bool {
        self.pressed == Some(action)
    }

    /// A button. Returns its width.
    #[allow(clippy::too_many_arguments)]
    pub fn button(
        &mut self,
        x: f32,
        y: f32,
        w: Option<f32>,
        h: f32,
        label: &str,
        style: Style,
        action: Action,
    ) -> f32 {
        let size = if h >= 44.0 {
            15.0
        } else if h >= 38.0 {
            14.0
        } else {
            13.0
        };
        let weight = W::S;
        let width = w.unwrap_or_else(|| self.measure(size, weight, label) + 32.0);
        let pressed = self.is_pressed(action) && style != Style::Disabled;
        let hovered = self.is_hovered(action) && style != Style::Disabled;
        match style {
            Style::Primary => {
                // Mixed in the theme's own colours.
                let bg = if pressed {
                    self.col(ACCENT).mix(self.col(BG), 0.25)
                } else if hovered {
                    self.col(ACCENT).mix(self.col(TEXT), 0.12)
                } else {
                    ACCENT
                };
                self.fill(x, y, width, h, 10.0, bg);
            }
            Style::Secondary => {
                if pressed {
                    self.fill(x, y, width, h, 10.0, INNER);
                } else if hovered {
                    self.fill(x, y, width, h, 10.0, INNER.with_alpha(150));
                }
                let edge = if hovered { DIM } else { BORDER };
                self.stroke(x, y, width, h, 10.0, edge);
            }
            Style::Ghost => {
                if pressed {
                    self.fill(x, y, width, h, 10.0, INNER);
                } else if hovered {
                    self.fill(x, y, width, h, 10.0, INNER.with_alpha(150));
                }
            }
            Style::Disabled => {
                self.stroke(x, y, width, h, 10.0, INNER);
            }
        }
        let fg = match style {
            Style::Primary => ON_ACCENT,
            Style::Secondary => TEXT,
            Style::Ghost => MUTED,
            Style::Disabled => DIM,
        };
        let tw = self.measure(size, weight, label);
        self.text_mid(x + (width - tw) / 2.0, y, h, size, weight, fg, label);
        if style != Style::Disabled {
            self.quiet_hover = true;
            self.hit(x, y, width, h, action);
            self.quiet_hover = false;
        }
        width
    }

    /// A rounded label with a dot. Returns its width.
    pub fn chip(&mut self, x: f32, y: f32, label: &str, fg: Color, bg: Color) -> f32 {
        let h = 26.0;
        let width = self.measure(12.0, W::R, label) + 34.0;
        self.fill(x, y, width, h, 13.0, bg);
        self.dot(x + 13.0, y + h / 2.0, 3.0, fg);
        self.text_mid(x + 22.0, y, h, 12.0, W::R, fg, label);
        width
    }

    /// A pill with a plain label and an edge. Returns its width.
    pub fn pill(&mut self, x: f32, y: f32, h: f32, label: &str) -> f32 {
        let width = self.measure(12.0, W::R, label) + 24.0;
        self.fill(x, y, width, h, h / 2.0, SURFACE);
        self.stroke(x, y, width, h, h / 2.0, LINE);
        self.text_mid(x + 12.0, y, h, 12.0, W::R, MUTED, label);
        width
    }

    /// Threshold pips: `filled` of `n`.
    pub fn pips(&mut self, x: f32, y: f32, n: usize, filled: usize) -> f32 {
        let mut cx = x;
        for i in 0..n {
            if i < filled {
                self.fill(cx, y, 28.0, 10.0, 5.0, OK);
            } else {
                self.stroke(cx, y, 28.0, 10.0, 5.0, BORDER);
            }
            cx += 34.0;
        }
        cx - x - 6.0
    }

    /// A numbered or ticked step badge.
    pub fn badge(&mut self, x: f32, y: f32, label: &str, done: bool, open: bool) {
        let (bg, fg) = if done {
            (OK.with_alpha(40), OK)
        } else if open {
            (ACCENT, ON_ACCENT)
        } else {
            (INNER, MUTED)
        };
        self.dot(x + 13.0, y + 13.0, 13.0, bg);
        if done {
            self.icon(x, y, 26.0, Icon::Done, 12.0, fg);
        } else {
            let tw = self.measure(12.0, W::S, label);
            self.text_mid(x + 13.0 - tw / 2.0, y, 26.0, 12.0, W::S, fg, label);
        }
    }

    /// A checkbox.
    pub fn checkbox(&mut self, x: f32, y: f32, on: bool, enabled: bool) {
        if on && enabled {
            self.fill(x, y, 18.0, 18.0, 4.0, ACCENT);
            self.icon(x, y, 18.0, Icon::Done, 11.0, ON_ACCENT);
        } else {
            self.stroke(x, y, 18.0, 18.0, 4.0, if enabled { MUTED } else { INNER });
        }
    }
}

/// An address or a txid in groups of four.
pub fn grouped(s: &str) -> String {
    s.chars()
        .collect::<Vec<_>>()
        .chunks(4)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The first two groups of four, an ellipsis and the last four
/// characters.
pub fn short(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 16 {
        return grouped(s);
    }
    let head: String = chars[..8].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{} … {}", grouped(&head), tail)
}

/// An amount in BTC with eight decimals.
pub fn btc(sat: u64) -> String {
    format!("{}.{:08}", sat / 100_000_000, sat % 100_000_000)
}

/// A whole number with thousands separators.
pub fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}
