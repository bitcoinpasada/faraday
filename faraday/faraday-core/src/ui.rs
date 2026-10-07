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
    /// Text on the accent.
    pub const ON_ACCENT: Color = Color::rgb(0x0d, 0x12, 0x18);
    /// Done, verified, can sign.
    pub const OK: Color = Color::rgb(0x8b, 0xd4, 0xb2);
    /// Waiting, unsaved, needs attention.
    pub const WARN: Color = Color::rgb(0xf0, 0xc0, 0x77);
    /// Refused.
    pub const ERR: Color = Color::rgb(0xef, 0x9b, 0x9b);
}

use pal::*;

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
        self.c.fill_circle(
            (cx + self.ox) * self.f,
            (cy + self.oy) * self.f,
            r * self.f,
            color,
        );
    }

    /// A filled rectangle with rounded corners.
    pub fn fill(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
        let rect = self.rect(x, y, w, h);
        if r <= 0.0 {
            self.c.fill_rect(rect, color);
        } else {
            self.c.fill_rounded_rect(rect, r * self.f, color);
        }
    }

    /// An outlined rectangle with rounded corners.
    pub fn stroke(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
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
        let rect = self.rect(x, y, boxw, boxw);
        self.c.icon(rect, icon, size, color);
    }

    /// Makes a region pressable: the part of it inside the clip, so what
    /// is scrolled out of sight cannot be pressed.
    pub fn hit(&mut self, x: f32, y: f32, w: f32, h: f32, action: Action) {
        let rect = self.rect(x, y, w, h).intersect(&self.c.clip());
        if rect.w > 0 && rect.h > 0 {
            self.hits.push((rect, action));
        }
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
        match style {
            Style::Primary => {
                let bg = if pressed {
                    ACCENT.mix(BG, 0.25)
                } else {
                    ACCENT
                };
                self.fill(x, y, width, h, 10.0, bg);
            }
            Style::Secondary => {
                if pressed {
                    self.fill(x, y, width, h, 10.0, INNER);
                }
                self.stroke(x, y, width, h, 10.0, BORDER);
            }
            Style::Ghost => {
                if pressed {
                    self.fill(x, y, width, h, 10.0, INNER);
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
            self.hit(x, y, width, h, action);
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
