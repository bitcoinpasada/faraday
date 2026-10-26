//! Settings › Appearance (`docs/THEMES.md` §12), drawn the same in
//! Faraday and the Starter (§14): Theme as chips, each filled in that
//! theme's own colours, Mode and Contrast as segmented
//! buttons; and Material's switch, which Settings' Reduce motion row
//! uses.

use crate::pal::{Role, ThemeName};
use crate::{Contrast, Mode, Theme, Ui, W};
use faraday_draw::Color;
use faraday_draw::tokens::{
    APPEARANCE_LABEL_GAP, APPEARANCE_ROW_GAP, FILTER_CHIP_GAP, FILTER_CHIP_H, FILTER_CHIP_ICON,
    FILTER_CHIP_LEAD, FILTER_CHIP_TRAIL, GAP, LABEL, SEGMENTED_H, SHAPE_S, STATE_HOVER,
    STATE_PRESS, SWITCH_EDGE, SWITCH_H, SWITCH_HANDLE_OFF, SWITCH_HANDLE_ON, SWITCH_HANDLE_PRESSED,
    SWITCH_STATE, SWITCH_W, THEME_CHIP_DOT, THEME_CHIP_RING, THEME_CHIP_RING_GAP, THEME_CHIP_TICK,
    TOUCH, alpha_of, shape_full,
};
use faraday_draw::widgets::Icon;
use faraday_strings::EN;

impl<A: Copy + PartialEq> Ui<'_, A> {
    /// How wide [`Ui::theme_chip`] draws `name`'s chip: its leading slot, its
    /// name and, on the theme a first start is in, the Default mark.
    pub fn theme_chip_width(&self, name: ThemeName) -> f32 {
        let mut w = FILTER_CHIP_LEAD
            + FILTER_CHIP_ICON
            + GAP
            + self.measure(LABEL, W::S, name.name())
            + FILTER_CHIP_TRAIL;
        if name == ThemeName::default() {
            w += GAP + self.measure(LABEL, W::R, EN.ui_default_theme);
        }
        w
    }

    /// A theme as a chip at (x, y), filled in that theme's own
    /// primary-container (in the mode and contrast on screen) with its
    /// name in its own on-primary-container: its leading slot holds the
    /// theme's primary as a dot or, on the chosen one, a tick. The chosen
    /// one is ringed in its own primary, a band of the page between ring
    /// and fill (`docs/decisions/F120.md`). Its state layer is its
    /// own on-primary-container; under it (hovered, focused or held)
    /// what it writes goes to white or black, whichever that colour
    /// leans to, so the label keeps its contrast. Returns its width.
    pub fn theme_chip(&mut self, x: f32, y: f32, name: ThemeName, chosen: bool, action: A) -> f32 {
        let w = self.theme_chip_width(name);
        let h = FILTER_CHIP_H;
        let own = Theme::new(name, self.theme.mode, self.theme.contrast);
        let (fill, ink) = (
            own.role(Role::PrimaryContainer),
            own.role(Role::OnPrimaryContainer),
        );
        if chosen {
            self.theme_chip_ring((x, y, w, h), own.role(Role::Primary));
        }
        self.fill(x, y, w, h, SHAPE_S, fill);
        self.state_layer((x, y, w, h, SHAPE_S), ink.into(), action);
        let layered =
            self.is_pressed(action) || self.is_hovered(action) || self.focused == Some(action);
        let content = if layered { held_ink(fill, ink) } else { ink };
        let r = FILTER_CHIP_ICON / 2.0;
        let (cx, cy) = (x + FILTER_CHIP_LEAD + r, y + h / 2.0);
        if chosen {
            let t = THEME_CHIP_TICK;
            self.icon(cx - t / 2.0, cy - t / 2.0, t, Icon::Done, t, content);
        } else {
            self.dot(cx, cy, THEME_CHIP_DOT, own.role(Role::Primary));
        }
        let lx = x + FILTER_CHIP_LEAD + FILTER_CHIP_ICON + GAP;
        let lw = self.text_mid(lx, y, h, LABEL, W::S, content, name.name());
        if name == ThemeName::default() {
            self.text_mid(
                lx + lw + GAP,
                y,
                h,
                LABEL,
                W::R,
                content,
                EN.ui_default_theme,
            );
        }
        self.focus_ring((x, y, w, h, SHAPE_S), action);
        self.hit_drawn(x, y, w, h, action);
        w
    }

    /// The chosen theme chip's ring round the chip at `at`, in `color`:
    /// [`THEME_CHIP_RING`] thick, [`THEME_CHIP_RING_GAP`] outside it, so
    /// the page shows between ring and fill and the ring stands against
    /// the page on both sides, whatever the fill.
    fn theme_chip_ring(&mut self, (x, y, w, h): (f32, f32, f32, f32), color: Color) {
        let out = THEME_CHIP_RING_GAP + THEME_CHIP_RING;
        let rect = self.rect(x - out, y - out, w + 2.0 * out, h + 2.0 * out);
        let r = (SHAPE_S + out) * self.f;
        self.c
            .stroke_rounded_rect(rect, r, THEME_CHIP_RING * self.f, color);
    }

    /// The ten themes as filter chips from (x, y), wrapping in `w`; the
    /// chosen one is `current`'s. `action` names what a press on a chip
    /// does, given the scheme it would choose: that theme in `current`'s
    /// mode and contrast. Returns the height used.
    pub fn theme_chips(
        &mut self,
        (x, y, w): (f32, f32, f32),
        current: Theme,
        action: impl Fn(Theme) -> A,
    ) -> f32 {
        let (mut cx, mut cy) = (x, y);
        for name in ThemeName::ALL {
            let cw = self.theme_chip_width(name);
            if cx > x && cx + cw > x + w {
                cx = x;
                cy += FILTER_CHIP_H + FILTER_CHIP_GAP;
            }
            let pick = Theme::new(name, current.mode, current.contrast);
            cx +=
                self.theme_chip(cx, cy, name, name == current.name, action(pick)) + FILTER_CHIP_GAP;
        }
        cy + FILTER_CHIP_H - y
    }

    /// How tall [`Ui::appearance`] draws its rows `w` wide, worked out
    /// from the labels' widths with nothing drawn: what a card is sized
    /// by before its surface goes down.
    pub fn appearance_h(&self, w: f32, label_w: Option<f32>) -> f32 {
        let (cw, head) = match label_w {
            Some(lw) => (w - lw, 0.0),
            None => (w, self.line(LABEL, W::R) + APPEARANCE_LABEL_GAP),
        };
        let mut chips = FILTER_CHIP_H;
        let mut cx = 0.0;
        for name in ThemeName::ALL {
            let width = self.theme_chip_width(name);
            if cx > 0.0 && cx + width > cw {
                cx = 0.0;
                chips += FILTER_CHIP_H + FILTER_CHIP_GAP;
            }
            cx += width + FILTER_CHIP_GAP;
        }
        let modes = Mode::ALL.map(Mode::name);
        let levels = Contrast::ALL.map(Contrast::name);
        let rows = [
            chips,
            self.choices_h(cw, SEGMENTED_H, LABEL, &modes),
            self.choices_h(cw, SEGMENTED_H, LABEL, &levels),
        ];
        rows.iter().map(|r| r + head).sum::<f32>() + 2.0 * APPEARANCE_ROW_GAP
    }

    /// Theme, Mode and Contrast (`docs/THEMES.md` §12) from (x, y), `w`
    /// wide: each row's label in a column `label_w` wide with its control
    /// beside it, or, with `label_w` `None`, over it. `action` names the
    /// press that chooses a scheme. Returns the height used.
    pub fn appearance(
        &mut self,
        (x, y, w): (f32, f32, f32),
        label_w: Option<f32>,
        current: Theme,
        action: impl Fn(Theme) -> A,
    ) -> f32 {
        let mut at = y;
        let modes: Vec<(&str, A)> = Mode::ALL
            .into_iter()
            .map(|mode| (mode.name(), action(Theme { mode, ..current })))
            .collect();
        let levels: Vec<(&str, A)> = Contrast::ALL
            .into_iter()
            .map(|contrast| {
                (
                    contrast.name(),
                    action(Theme {
                        contrast,
                        ..current
                    }),
                )
            })
            .collect();
        let rows = [EN.ui_theme_row, EN.ui_mode_row, EN.ui_contrast_row];
        for (i, label) in rows.into_iter().enumerate() {
            if i > 0 {
                at += APPEARANCE_ROW_GAP;
            }
            let (cx, cw, top) = match label_w {
                Some(lw) => {
                    // The label level with the control's first line.
                    let line = if i == 0 { FILTER_CHIP_H } else { SEGMENTED_H };
                    self.text_mid(x, at, line, LABEL, W::R, Role::OnSurfaceVariant, label);
                    (x + lw, w - lw, at)
                }
                None => {
                    self.text(x, at, LABEL, W::R, Role::OnSurfaceVariant, label);
                    (x, w, at + self.line(LABEL, W::R) + APPEARANCE_LABEL_GAP)
                }
            };
            let used = match i {
                0 => self.theme_chips((cx, top, cw), current, &action),
                1 => self.choices(
                    (cx, top, cw, SEGMENTED_H),
                    LABEL,
                    &modes,
                    Some(current.mode as usize),
                ),
                _ => self.choices(
                    (cx, top, cw, SEGMENTED_H),
                    LABEL,
                    &levels,
                    Some(current.contrast as usize),
                ),
            };
            at = top + used;
        }
        at - y
    }

    /// A switch (Material's) at (x, y), [`SWITCH_W`] by [`SWITCH_H`]: on,
    /// its track in primary and its handle in on-primary at the right;
    /// off, its track in surface-container-highest edged in outline and a
    /// smaller handle in outline at the left; pressed, the handle larger.
    /// A press takes `action`, in a target at least [`TOUCH`] tall.
    pub fn switch(&mut self, x: f32, y: f32, on: bool, action: A) {
        let (w, h) = (SWITCH_W, SWITCH_H);
        let r = shape_full(h);
        let pressed = self.is_pressed(action);
        let (handle, layer) = if on {
            self.fill(x, y, w, h, r, Role::Primary);
            let handle = if pressed {
                Role::PrimaryContainer
            } else {
                Role::OnPrimary
            };
            (handle, Role::Primary)
        } else {
            self.fill(x, y, w, h, r, Role::SurfaceContainerHighest);
            let edge = self.rect(x, y, w, h);
            let color = self.col(Role::Outline);
            self.c
                .stroke_rounded_rect(edge, r * self.f, SWITCH_EDGE * self.f, color);
            let handle = if pressed {
                Role::OnSurfaceVariant
            } else {
                Role::Outline
            };
            (handle, Role::OnSurface)
        };
        let d = if pressed {
            SWITCH_HANDLE_PRESSED
        } else if on {
            SWITCH_HANDLE_ON
        } else {
            SWITCH_HANDLE_OFF
        };
        // The handle's centre: a track's height from the end it rests at.
        let cx = if on { x + w - h / 2.0 } else { x + h / 2.0 };
        let cy = y + h / 2.0;
        let opacity = if pressed {
            Some(STATE_PRESS)
        } else if self.is_hovered(action) || self.focused == Some(action) {
            Some(STATE_HOVER)
        } else {
            None
        };
        if let Some(o) = opacity {
            let s = SWITCH_STATE / 2.0;
            self.wash(
                cx - s,
                cy - s,
                2.0 * s,
                2.0 * s,
                s,
                layer.with_alpha(alpha_of(o)),
            );
        }
        self.dot(cx, cy, d / 2.0, handle);
        self.focus_ring((x, y, w, h, r), action);
        self.hit_drawn(x, y, w, h, action);
    }

    /// A list row with a switch at its right end, [`TOUCH`] tall, from
    /// (x, y) `w` wide: `label` at the left, the switch on when `on`.
    /// Returns its height.
    pub fn switch_row(
        &mut self,
        (x, y, w): (f32, f32, f32),
        label: &str,
        on: bool,
        action: A,
    ) -> f32 {
        let room = w - SWITCH_W - GAP;
        let shown = self.fit(LABEL, W::R, label, room);
        self.text_mid(x, y, TOUCH, LABEL, W::R, Role::OnSurface, &shown);
        self.switch(x + w - SWITCH_W, y + (TOUCH - SWITCH_H) / 2.0, on, action);
        TOUCH
    }
}

/// What a theme chip under its state layer writes in: white where its on-colour `ink` is
/// lighter than its `fill`, else black. Material's container pairs sit
/// at 4.5:1 at Standard contrast, and the state layer, laid in `ink`,
/// takes up to a tenth of that while it shows; the label moving away
/// from the fill gives it back (`docs/decisions/F120.md`).
fn held_ink(fill: Color, ink: Color) -> Color {
    let light = |c: Color| 2126 * u32::from(c.r) + 7152 * u32::from(c.g) + 722 * u32::from(c.b);
    if light(ink) > light(fill) {
        Color::WHITE
    } else {
        Color::BLACK
    }
}
