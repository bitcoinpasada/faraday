//! Buttons in each style (09 §3.2).

use faraday_tokens::palette::{Paint, Role};

use crate::Pressed;
use crate::hits::{Shape, Unpressed};
use crate::offered::Offered;
use crate::shown::Shown;
use crate::ui::Ui;
use crate::ui::W;
use faraday_draw::icons::Icon;
use faraday_tokens::dims::{BUTTON_PAD, GAP, ICON_GAP, ICON_GLYPH_BUTTON, LABEL_L, shape_full};

/// A button, with its style and what a press offers (04 §12.3).
pub struct ButtonModel<'s, A> {
    pub label: Shown<'s>,
    pub style: Style,
    pub offered: Offered<A>,
    /// The chosen one of a set of options: secondary-container, with a tick.
    pub chosen: bool,
    /// An icon before the label.
    pub icon: Option<Icon>,
}

/// Draws the button in the box (x, y, width or its label's, h), design units: taller where
/// larger text needs it. Returns its width.
pub fn draw<A: Pressed>(
    m: &ButtonModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w, h): (f32, f32, Option<f32>, f32),
) -> f32 {
    let look = if m.chosen {
        Look::Selected
    } else {
        Look::from(m.style)
    };
    ui.showing(m.label, |ui| {
        ui.offered(&m.offered, |ui, press, off| {
            ui.button_offered(
                (x, y, w, h),
                m.label.as_str(),
                m.icon,
                look.when(!off),
                press,
            )
        })
    })
}

/// How a button is drawn (04 §12.3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    Filled,
    Tonal,
    Outlined,
    Text,
    Elevated,
    Error,
}

impl From<Style> for Look {
    fn from(style: Style) -> Look {
        match style {
            Style::Filled => Look::Primary,
            // Material's elevated button is a tonal one lifted by its shadow; it is drawn as
            // the tonal one, flat, until the shadow is part of the button.
            Style::Tonal | Style::Elevated => Look::Tonal,
            Style::Outlined => Look::Secondary,
            Style::Text => Look::Ghost,
            Style::Error => Look::Danger,
        }
    }
}

/// A button's look as `main` drew it: a style, a chosen option's tick, and each kind not
/// available now in its own shape dimmed.
/// How a button looks: Material's filled, tonal, outlined and text
/// buttons, a chosen option, and each kind not available now, drawn in
/// its own shape dimmed (`docs/DESIGN.md` §4.13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    /// The one action a screen leads with: filled in primary.
    Primary,
    /// A second action that matters: filled in secondary-container, no
    /// edge.
    Tonal,
    /// Any other action: outlined in the outline role, its label in
    /// primary.
    Secondary,
    /// A quiet action beside others: a text button, its label in
    /// primary.
    Ghost,
    /// The first press of a dangerous act (what removes or deletes): an
    /// outlined button in the error tone; the last press is still a hold
    /// (F332).
    Danger,
    /// The chosen one of a set of options: secondary-container, with a
    /// tick.
    Selected,
    /// An outlined button shown but not available now.
    Disabled,
    /// A filled button not available now: its fill kept, faint.
    PrimaryOff,
    /// A tonal button not available now.
    TonalOff,
    /// A text button not available now.
    GhostOff,
}

impl Look {
    /// This kind not available now, in its own shape.
    pub const fn off(self) -> Look {
        match self {
            Look::Primary | Look::PrimaryOff => Look::PrimaryOff,
            Look::Tonal | Look::TonalOff => Look::TonalOff,
            Look::Ghost | Look::GhostOff => Look::GhostOff,
            Look::Secondary | Look::Danger | Look::Selected | Look::Disabled => Look::Disabled,
        }
    }

    /// This kind, or [`Look::off`] when `on` is false.
    pub const fn when(self, on: bool) -> Look {
        if on { self } else { self.off() }
    }

    /// An option of a set: [`Look::Selected`] when it is the one
    /// chosen, an outlined button when not.
    pub const fn chosen(on: bool) -> Look {
        if on { Look::Selected } else { Look::Secondary }
    }

    /// Whether it is shown but not available now.
    pub const fn is_off(self) -> bool {
        matches!(
            self,
            Look::Disabled | Look::PrimaryOff | Look::TonalOff | Look::GhostOff
        )
    }

    /// The colour of its label, and of its state layer (Material's
    /// component specs, `docs/THEMES.md` §7); a kind not available now
    /// has on-surface at the disabled opacity (§8).
    pub fn label(self) -> Paint {
        match self {
            Look::Primary => Role::OnPrimary.paint(),
            Look::Tonal | Look::Selected => Role::OnSecondaryContainer.paint(),
            Look::Secondary | Look::Ghost => Role::Primary.paint(),
            Look::Danger => Role::Error.paint(),
            Look::Disabled | Look::PrimaryOff | Look::TonalOff | Look::GhostOff => {
                Role::OnSurface.at(faraday_tokens::dims::DISABLED_CONTENT)
            }
        }
    }
}

impl<A: Pressed> Ui<'_, A> {
    /// A button (`docs/DESIGN.md` §4.13): Material's filled, tonal,
    /// outlined or text button by `style`, round at its ends, its state
    /// layer in its label's colour and the focus ring round it; a kind
    /// not available now keeps its shape, faint, and takes no press. A
    /// chosen option carries a tick before its label. Returns its width.
    #[allow(
        clippy::too_many_arguments,
        reason = "a button's position, size, label, style and action, as every button helper takes them"
    )]
    pub fn button(
        &mut self,
        x: f32,
        y: f32,
        w: Option<f32>,
        h: f32,
        label: &str,
        style: Look,
        action: A,
    ) -> f32 {
        // Taller where larger text needs it, about the same middle
        // (`docs/TEXT-SIZE.md` §5).
        let grown = self.button_height(h);
        let y = y - (grown - h) / 2.0;
        let r = faraday_tokens::dims::shape_full(grown);
        let size = faraday_tokens::dims::LABEL_L.size;
        self.button_drawn(
            (x, y, w, grown, r),
            size,
            label,
            style,
            Some(action),
            None,
            None,
            false,
            &[],
        )
    }

    /// A button in the box (x, y, w, h), taller where larger text needs it, with `icon` before
    /// its label, that takes `action` when there is one. Returns its width.
    pub fn button_offered(
        &mut self,
        (x, y, w, h): (f32, f32, Option<f32>, f32),
        label: &str,
        icon: Option<Icon>,
        look: Look,
        action: Option<A>,
    ) -> f32 {
        let grown = self.button_height(h);
        let y = y - (grown - h) / 2.0;
        let r = shape_full(grown);
        let size = LABEL_L.size;
        self.button_drawn(
            (x, y, w, grown, r),
            size,
            label,
            look,
            action,
            None,
            icon,
            false,
            &[],
        )
    }

    /// [`Ui::button`] `h` tall as given, its label broken onto the
    /// centred `lines` ([`sheet::buttons`] where it does not fit one).
    pub fn button_wrapped(
        &mut self,
        (x, y, w, h): (f32, f32, f32, f32),
        lines: &[&str],
        style: Look,
        action: A,
    ) {
        let size = faraday_tokens::dims::LABEL_L.size;
        let r = faraday_tokens::dims::shape_full(h);
        let at = (x, y, Some(w), h, r);
        self.button_drawn(at, size, "", style, Some(action), None, None, false, lines);
    }

    /// [`Ui::button`] with `icon` before its label, as Material's
    /// buttons with an icon are drawn (`docs/DESIGN.md` §4.13). Returns
    /// its width.
    #[allow(
        clippy::too_many_arguments,
        reason = "a button's position, size, icon, label, style and action, as every button helper takes them"
    )]
    pub fn button_icon(
        &mut self,
        (x, y, w, h): (f32, f32, Option<f32>, f32),
        icon: Icon,
        label: &str,
        style: Look,
        action: A,
    ) -> f32 {
        let grown = self.button_height(h);
        let y = y - (grown - h) / 2.0;
        let r = faraday_tokens::dims::shape_full(grown);
        let size = faraday_tokens::dims::LABEL_L.size;
        self.button_drawn(
            (x, y, w, grown, r),
            size,
            label,
            style,
            Some(action),
            None,
            Some(icon),
            false,
            &[],
        )
    }

    /// [`Ui::button_icon`] with the icon after its label: a button that
    /// opens a menu under it, with the down chevron. Returns its width.
    pub fn button_icon_after(
        &mut self,
        (x, y, w, h): (f32, f32, Option<f32>, f32),
        icon: Icon,
        label: &str,
        style: Look,
        action: A,
    ) -> f32 {
        let grown = self.button_height(h);
        let y = y - (grown - h) / 2.0;
        let r = faraday_tokens::dims::shape_full(grown);
        let size = faraday_tokens::dims::LABEL_L.size;
        self.button_drawn(
            (x, y, w, grown, r),
            size,
            label,
            style,
            Some(action),
            None,
            Some(icon),
            true,
            &[],
        )
    }

    /// How wide [`Ui::button_icon`] draws `label` in `style` when given
    /// no width.
    pub fn button_icon_width(&self, label: &str, style: Look) -> f32 {
        use faraday_tokens::dims::{ICON_GAP, ICON_GLYPH_BUTTON};
        self.button_width(label, style) + ICON_GLYPH_BUTTON + ICON_GAP
    }

    /// A text button standing alone in a column of content, or first in
    /// a row at its edge (`docs/DESIGN.md` §4.13): its label starts at
    /// `x`, the content's start edge, and the button's state layer and
    /// touch target reach out past it by Material's container padding.
    /// `style` is [`Look::Ghost`] or [`Look::GhostOff`]. Returns how far
    /// the button's right edge is past `x`.
    pub fn text_button(&mut self, x: f32, y: f32, label: &str, style: Look, action: A) -> f32 {
        use faraday_tokens::dims::{BUTTON_H, BUTTON_PAD};
        // Where the content starts nearer the panel's edge than the
        // padding, the container reaches to the edge and no further.
        let pad = BUTTON_PAD.min(x.max(0.0));
        let w = self.button_width(label, style) - 2.0 * (BUTTON_PAD - pad);
        self.button(x - pad, y, Some(w), BUTTON_H, label, style, action) - pad
    }

    /// [`Ui::button`] with corners of `r` rather than round ends: a key
    /// of the on-screen keyboard, its label the key's
    /// ([`faraday_tokens::dims::key_label`]).
    pub fn button_in(
        &mut self,
        at: (f32, f32, Option<f32>, f32, f32),
        label: &str,
        style: Look,
        action: A,
    ) -> f32 {
        // A key keeps its size whatever the text size: the keyboards
        // and pads are set by what must fit on the panel.
        let size = faraday_tokens::dims::key_label(at.3);
        self.fixed_text(|ui| {
            ui.button_drawn(at, size, label, style, Some(action), None, None, false, &[])
        })
    }

    /// [`Ui::button_in`] for a key of the on-screen keyboard: it shows
    /// hover and focus, and no pressed layer, so a watcher reads nothing
    /// from a press ([`Unpressed::Keyboard`]).
    pub fn key_in(
        &mut self,
        at: (f32, f32, Option<f32>, f32, f32),
        label: &str,
        style: Look,
        action: A,
    ) -> f32 {
        let size = faraday_tokens::dims::key_label(at.3);
        self.fixed_text(|ui| {
            ui.button_drawn(
                at,
                size,
                label,
                style,
                Some(action),
                Some(Unpressed::Keyboard),
                None,
                false,
                &[],
            )
        })
    }

    /// How wide [`Ui::button`] draws `label` in `style` when given no
    /// width: its label as the button measures it, the padding and the
    /// room for the tick.
    pub fn button_width(&self, label: &str, style: Look) -> f32 {
        use faraday_tokens::dims::{BUTTON_PAD, ICON_GAP, ICON_GLYPH_BUTTON};
        let tick = if style == Look::Selected {
            ICON_GLYPH_BUTTON + ICON_GAP
        } else {
            0.0
        };
        let size = faraday_tokens::dims::LABEL_L.size;
        self.measure(size, W::S, label) + 2.0 * BUTTON_PAD + tick
    }

    /// A button or a key with its label at `size` in the medium face:
    /// [`Ui::button`] and [`Ui::button_in`].
    #[allow(
        clippy::too_many_arguments,
        reason = "a button's place, size, label, style, action, press kind and icon"
    )]
    fn button_drawn(
        &mut self,
        (x, y, w, h, r): (f32, f32, Option<f32>, f32, f32),
        size: f32,
        label: &str,
        style: Look,
        action: Option<A>,
        unpressed: Option<Unpressed>,
        icon: Option<Icon>,
        after: bool,
        lines: &[&str],
    ) -> f32 {
        use faraday_tokens::dims::{BUTTON_PAD, ICON_GAP, ICON_GLYPH_BUTTON};
        let weight = W::S;
        let style = if self.answering || action.is_none() {
            style.off()
        } else {
            style
        };
        // A chosen option's tick, or the icon a button was given.
        let mark = if style == Look::Selected {
            Some(Icon::Done)
        } else {
            icon
        };
        let tick = if mark.is_some() {
            ICON_GLYPH_BUTTON + ICON_GAP
        } else {
            0.0
        };
        // The label's start past the button's content start: after an icon
        // that stands before it, none where the icon follows.
        let lead = if after { 0.0 } else { tick };
        let tw = self.measure(size, weight, label);
        let width = w.unwrap_or(tw + 2.0 * BUTTON_PAD + tick);
        let faint = Role::OnSurface.at(faraday_tokens::dims::DISABLED_CONTAINER);
        match style {
            Look::Primary => {
                self.primaries += 1;
                self.fill(x, y, width, h, r, Role::Primary);
            }
            Look::Tonal | Look::Selected => {
                self.fill(x, y, width, h, r, Role::SecondaryContainer);
                // High contrast edges every control at 3:1.
                if self.theme.is_high_contrast() {
                    self.stroke(x, y, width, h, r, Role::Outline);
                }
            }
            Look::Secondary => self.stroke(x, y, width, h, r, Role::Outline),
            Look::Danger => self.stroke(x, y, width, h, r, Role::Error),
            Look::Disabled => {
                // Faint in every theme: it is not available now.
                let (rect, edge) = (self.rect(x, y, width, h), self.col(faint));
                let line = self.f.max(1.0);
                self.c.stroke_rounded_rect(rect, r * self.f, line, edge);
            }
            Look::PrimaryOff | Look::TonalOff => self.wash(x, y, width, h, r, faint),
            Look::Ghost | Look::GhostOff => {}
        }
        let fg = style.label();
        if !style.is_off() {
            let shape = Shape::new(x, y, width, h, r);
            if let Some(action) = action {
                match unpressed {
                    Some(why) => self.press_unpressed(shape, fg, why, action),
                    None => self.press(shape, fg, action),
                }
            }
        } else if let Some(action) = action {
            self.answer_hit((x, y, width, h), action);
        }
        if !lines.is_empty() {
            let line = self.line_as(faraday_tokens::dims::LABEL_L);
            let top = y + (h - lines.len() as f32 * line) / 2.0;
            let widths: Vec<f32> = lines
                .iter()
                .map(|l| self.measure(size, weight, l))
                .collect();
            self.centred_lines((x, width, top, line), lines, &widths, fg);
            return width;
        }
        // At a larger text size a label wider than its button fits down
        // to Standard, and is shortened there only where it still does
        // not fit (`docs/TEXT-SIZE.md` §3).
        let room = width - tick - 2.0 * BUTTON_PAD;
        if self.text > 1.0 && tw > room {
            let size = size * self.fit_factor_of(tw, room);
            return self.fixed_text(|ui| {
                let shown = ui.fit(size, weight, label, room);
                let tw = ui.measure(size, weight, &shown);
                let lx = x + (width - tw - tick) / 2.0;
                if let Some(mark) = mark {
                    let iy = y + (h - ICON_GLYPH_BUTTON) / 2.0;
                    let ix = if after { lx + tw + ICON_GAP } else { lx };
                    ui.icon(ix, iy, ICON_GLYPH_BUTTON, mark, ICON_GLYPH_BUTTON, fg);
                }
                ui.text_mid(lx + lead, y, h, size, weight, fg, &shown);
                width
            });
        }
        let lx = x + (width - tw - tick) / 2.0;
        if let Some(mark) = mark {
            let iy = y + (h - ICON_GLYPH_BUTTON) / 2.0;
            let ix = if after { lx + tw + ICON_GAP } else { lx };
            self.icon(ix, iy, ICON_GLYPH_BUTTON, mark, ICON_GLYPH_BUTTON, fg);
        }
        self.text_mid(lx + lead, y, h, size, weight, fg, label);
        width
    }

    /// A chip inside the empty entry field at (`x`, `y`, `w`, `h`), at its
    /// text inset, secondary-styled and vertically centred
    /// (`docs/MODES.md` §6.1). Its hit target is the chip, at the touch
    /// floor's height on a small panel and the field's on a wide one.
    /// Returns the chip's width; the caller passes `label`, the shorter
    /// one where the field is narrower than the chip.
    /// A choice of one among `options` in a width `w` from (x, y): a
    /// segmented button `h` tall where it fits, else the options as
    /// buttons wrapping onto rows, the chosen one [`Look::Selected`].
    /// Returns the height it took.
    pub fn choices(
        &mut self,
        at: (f32, f32, f32, f32),
        size: f32,
        options: &[(&str, A)],
        chosen: Option<usize>,
    ) -> f32 {
        self.choices_with(at, size, options, chosen, &[])
    }

    /// [`Ui::choices`] with the options `off` marks (by index; one past
    /// its end is enabled) disabled: a dimmed segment, or a
    /// [`Look::Disabled`] button, taking no press.
    pub fn choices_with(
        &mut self,
        (x, y, w, h): (f32, f32, f32, f32),
        size: f32,
        options: &[(&str, A)],
        chosen: Option<usize>,
        off: &[bool],
    ) -> f32 {
        let sw = self.segment_width_of(size, options.iter().map(|o| o.0));
        // A row as tall as larger text needs (`docs/TEXT-SIZE.md` §5).
        let h = self.button_height(h);
        if sw * options.len() as f32 <= w {
            self.segmented_with((x, y, h), size, options, chosen, None, off);
            return h;
        }
        // Each is a button of its own, padded as [`Ui::button`] pads
        // one, with room for the chosen one's tick.
        let pad = 2.0 * BUTTON_PAD + ICON_GLYPH_BUTTON + ICON_GAP;
        let (mut bx, mut by) = (x, y);
        for (i, (label, action)) in options.iter().enumerate() {
            let label = *label;
            let whole = self.measure(size, W::S, label) + pad;
            let bw = whole.min(w);
            if bx > x && bx + bw > x + w {
                bx = x;
                by += h + GAP;
            }
            // Cut only when the label is wider than the row: `bw - pad`
            // can come out a hair under the label's own width.
            let fitted;
            let shown: &str = if whole <= w {
                label
            } else {
                fitted = self.fit(size, W::S, label, w - pad);
                &fitted
            };
            let style = if off.get(i).copied().unwrap_or(false) {
                Look::Disabled
            } else {
                Look::chosen(chosen == Some(i))
            };
            bx += self.button(bx, by, Some(bw), h, shown, style, action.clone()) + GAP;
        }
        by - y + h
    }

    /// How tall [`Ui::choices`] draws `labels` in a width `w`, `h` a row,
    /// worked out from their widths with nothing drawn.
    pub fn choices_h(&self, w: f32, h: f32, size: f32, labels: &[&str]) -> f32 {
        let h = self.button_height(h);
        if self.segment_width(size, labels) * labels.len() as f32 <= w {
            return h;
        }
        let pad = 2.0 * BUTTON_PAD + ICON_GLYPH_BUTTON + ICON_GAP;
        let (mut bx, mut by) = (0.0, 0.0);
        for label in labels {
            let bw = (self.measure(size, W::S, label) + pad).min(w);
            if bx > 0.0 && bx + bw > w {
                bx = 0.0;
                by += h + GAP;
            }
            bx += bw + GAP;
        }
        by + h
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::testing::*;

    /// What is drawn just outside a button's left end, with the keyboard on `focused`.
    fn beside_a_button(focused: Option<P>) -> (u8, u8, u8) {
        let mut c = canvas(160);
        frame(&mut c, &at_rest(focused), |ui| {
            ui.clear(Role::Surface);
            ui.button(100.0, 100.0, Some(80.0), 40.0, "Go", Look::Secondary, P(1));
        });
        pixel(&c, 96, 120)
    }

    /// Invariant: The control the keyboard is on is ringed, and no other is.
    /// Regression: The focus ring is drawn round every button, or round none.
    /// Owner: kit
    #[test]
    fn the_control_the_keyboard_is_on_is_ringed() {
        assert_ne!(beside_a_button(Some(P(1))), beside_a_button(None));
        assert_eq!(beside_a_button(Some(P(2))), beside_a_button(None));
    }

    /// Whether a button of `look` took a press.
    fn takes_a_press(look: Look) -> bool {
        let mut c = canvas(160);
        let out = frame(&mut c, &at_rest(None), |ui| {
            ui.clear(Role::Surface);
            ui.button(100.0, 100.0, Some(120.0), 40.0, "", look, P(1));
        });
        !out.hits.is_empty()
    }

    /// Invariant: A button not available now takes no press, and an available one does.
    /// Regression: A dimmed button still sends its press.
    /// Owner: kit
    #[test]
    fn a_button_not_available_takes_no_press() {
        assert!(!takes_a_press(Look::Primary.when(false)));
        assert!(!takes_a_press(Look::Secondary.when(false)));
        assert!(takes_a_press(Look::Primary));
    }

    /// Invariant: A button the app refuses and answers draws dimmed, takes a press and has no place in the Tab order; one not offered takes none.
    /// Regression: A refused button drawn dimmed is dead, so the person gets no word on why.
    /// Owner: kit
    #[test]
    fn a_refused_button_drawn_dimmed_still_takes_its_press_but_is_not_tabbed_to() {
        use crate::offered::{Drawn, Offered};
        let model = |offered| ButtonModel {
            label: Shown::plain("Go"),
            style: Style::Filled,
            offered,
            chosen: false,
            icon: None,
        };
        let drawn = |offered| {
            let mut c = canvas(160);
            frame(&mut c, &at_rest(None), |ui| {
                draw(&model(offered), ui, (100.0, 100.0, Some(80.0), 40.0));
            })
        };
        let yes = drawn(Offered::Yes(P(1)));
        assert_eq!((yes.hits.len(), yes.reach.len()), (1, 1));
        let refused = drawn(Offered::Refused(P(1), Drawn::Dimmed));
        assert_eq!((refused.hits.len(), refused.reach.len()), (1, 0));
        let as_usual = drawn(Offered::Refused(P(1), Drawn::AsUsual));
        assert_eq!((as_usual.hits.len(), as_usual.reach.len()), (1, 1));
        let no = drawn(Offered::No);
        assert!(no.hits.is_empty());
    }
}
