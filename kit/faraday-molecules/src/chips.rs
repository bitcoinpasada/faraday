//! The chips molecule: the themes as filter chips, each filled in that theme's own colours
//! (09 §3.2; `docs/THEMES.md` §12).

use faraday_atoms::hits::Shape;
use faraday_atoms::{Glyph, Pressed, Shown, Ui};

use faraday_tokens::Theme;
use faraday_tokens::dims::{
    FILTER_CHIP_GAP, FILTER_CHIP_H, FILTER_CHIP_ICON, FILTER_CHIP_LEAD, FILTER_CHIP_TRAIL, GAP,
    LABEL_L, SHAPE_S, THEME_CHIP_DOT, THEME_CHIP_RING, THEME_CHIP_RING_GAP, THEME_CHIP_TICK,
    TOUCH,
};
use faraday_tokens::palette::{Color, Role, ThemeName};

/// One theme's chip: the theme, its name, and what a press on it sends.
pub struct ThemeChip<'s, A> {
    pub name: ThemeName,
    /// What it reads: the theme's name.
    pub label: Shown<'s>,
    /// The press that chooses this theme in the current mode and contrast.
    pub press: A,
}

/// The themes as filter chips, wrapping in the width given; the chosen one is `current`'s.
pub struct ChipsModel<'s, A> {
    /// The scheme on screen: the chosen theme, and the mode and contrast every chip is drawn in.
    pub current: Theme,
    pub items: Vec<ThemeChip<'s, A>>,
    /// The mark the theme a first start is in carries after its name.
    pub default_mark: Shown<'s>,
}

/// How wide a chip draws `item`'s: its leading slot, its name and, on the theme a first start is
/// in, the Default mark.
fn chip_width<A: Pressed>(m: &ChipsModel<'_, A>, ui: &Ui<'_, A>, item: &ThemeChip<'_, A>) -> f32 {
    let mut w = FILTER_CHIP_LEAD
        + FILTER_CHIP_ICON
        + GAP
        + ui.measure_as(LABEL_L, item.label.as_str())
        + FILTER_CHIP_TRAIL;
    if item.name == ThemeName::default() {
        w += GAP + ui.measure_as(LABEL_L.regular(), m.default_mark.as_str());
    }
    w
}

/// Draws the chips from (x, y), wrapping in `w`. Rows are [`TOUCH`] apart, so each chip's
/// target, grown to [`TOUCH`], meets the next row's and is not cut. Returns the height used.
pub fn draw<A: Pressed>(m: &ChipsModel<'_, A>, ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32)) -> f32 {
    let (mut cx, mut cy) = (x, y);
    for item in &m.items {
        let cw = chip_width(m, ui, item);
        if cx > x && cx + cw > x + w {
            cx = x;
            cy += TOUCH;
        }
        cx += chip(m, ui, item, (cx, cy, cw)) + FILTER_CHIP_GAP;
    }
    cy + FILTER_CHIP_H - y
}

/// A theme as a chip at (x, y), `w` wide, filled in that theme's own primary-container (in the
/// mode and contrast on screen) with its name in its own on-primary-container: its leading slot
/// holds the theme's primary as a dot or, on the chosen one, a tick. The chosen one is ringed in
/// its own primary, a band of the page between ring and fill (`docs/decisions/F120.md`). Its state
/// layer is its own on-primary-container; under it (hovered, focused or held) what it writes goes
/// to white or black, whichever that colour leans to, so the label keeps its contrast. Returns
/// its width.
fn chip<A: Pressed>(
    m: &ChipsModel<'_, A>,
    ui: &mut Ui<'_, A>,
    item: &ThemeChip<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let h = FILTER_CHIP_H;
    let chosen = item.name == m.current.name;
    let own = Theme::new(item.name, m.current.mode, m.current.contrast);
    let (fill, ink) = (
        own.role(Role::PrimaryContainer),
        own.role(Role::OnPrimaryContainer),
    );
    if chosen {
        ring(ui, (x, y, w, h), own.role(Role::Primary));
    }
    ui.fill(x, y, w, h, SHAPE_S, fill);
    ui.press(Shape::new(x, y, w, h, SHAPE_S), ink.into(), item.press.clone());
    let a = &item.press;
    let layered = ui.is_pressed(a) || ui.is_hovered(a) || ui.is_focused(a);
    let content = if layered { held_ink(fill, ink) } else { ink };
    let r = FILTER_CHIP_ICON / 2.0;
    let (cx, cy) = (x + FILTER_CHIP_LEAD + r, y + h / 2.0);
    if chosen {
        let t = THEME_CHIP_TICK;
        let tick = Glyph::Icon(faraday_atoms::Icon::Done);
        ui.icon(cx - t / 2.0, cy - t / 2.0, t, tick, t, content);
    } else {
        ui.dot(cx, cy, THEME_CHIP_DOT, own.role(Role::Primary));
    }
    let lx = x + FILTER_CHIP_LEAD + FILTER_CHIP_ICON + GAP;
    let lw = ui.text_mid_as((lx, y, h), LABEL_L, content, item.label.as_str());
    if item.name == ThemeName::default() {
        let at = (lx + lw + GAP, y, h);
        ui.text_mid_as(at, LABEL_L.regular(), content, m.default_mark.as_str());
    }
    w
}

/// The chosen theme chip's ring round the chip at `at`, in `color`: [`THEME_CHIP_RING`] thick,
/// [`THEME_CHIP_RING_GAP`] outside it, so the page shows between ring and fill and the ring
/// stands against the page on both sides, whatever the fill.
fn ring<A: Pressed>(ui: &mut Ui<'_, A>, (x, y, w, h): (f32, f32, f32, f32), color: Color) {
    let out = THEME_CHIP_RING_GAP + THEME_CHIP_RING;
    let rect = ui.rect(x - out, y - out, w + 2.0 * out, h + 2.0 * out);
    let r = (SHAPE_S + out) * ui.f;
    let f = ui.f;
    ui.c.stroke_rounded_rect(rect, r, THEME_CHIP_RING * f, color);
}

/// What a chip writes while it is under a layer: white or black, whichever `ink` leans to
/// against `fill`.
fn held_ink(fill: Color, ink: Color) -> Color {
    let light = |c: Color| 2126 * u32::from(c.r) + 7152 * u32::from(c.g) + 722 * u32::from(c.b);
    if light(ink) > light(fill) {
        Color::WHITE
    } else {
        Color::BLACK
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::Pointer;
    use faraday_atoms::ui::testing::*;
    use faraday_draw::canvas::InkKind;
    use faraday_text::Text;

    /// The chips of `current`'s mode and contrast, each in a row of its own.
    fn model<'s>(names: &'s [Text], current: Theme) -> ChipsModel<'s, P> {
        let items = Theme::all()
            .filter(|t| t.mode == current.mode && t.contrast == current.contrast)
            .enumerate()
            .map(|(i, t)| ThemeChip {
                name: t.name,
                label: Shown::plain(&names[i]),
                press: P(i as u8),
            })
            .collect();
        ChipsModel {
            current,
            items,
            default_mark: Shown::plain("Default"),
        }
    }

    fn names(current: Theme) -> Vec<Text> {
        Theme::all()
            .filter(|t| t.mode == current.mode && t.contrast == current.contrast)
            .map(|t| Text::of(&format!("{:?}", t.name)))
            .collect()
    }

    fn drawn(current: Theme, pointer: &Pointer<P>) -> (faraday_draw::Canvas, f32) {
        let names = names(current);
        let m = model(&names, current);
        let mut canvas = canvas(160);
        let mut h = 0.0;
        frame(&mut canvas, pointer, |ui| {
            ui.clear(page());
            // One chip to a row: a width no two share.
            h = draw(&m, ui, (24.0, 16.0, 1.0));
        });
        (canvas, h)
    }

    /// Invariant: In every scheme the chosen theme's chip alone looks different from the others; moving the choice changes those two chips and no other.
    /// Regression: The chosen chip is drawn like the rest, or choosing one redraws another's chip.
    /// Owner: kit
    #[test]
    fn the_chosen_theme_chip_alone_looks_different_from_the_others() {
        let mut short = Vec::new();
        for theme in Theme::all() {
            let other = Theme::all()
                .find(|t| t.mode == theme.mode && t.contrast == theme.contrast && t.name != theme.name)
                .expect("ten themes");
            let (a, _) = drawn(theme, &at_rest(None));
            let (b, _) = drawn(other, &at_rest(None));
            let (fa, fb) = (a.pixels(), b.pixels());
            let w = usize::from(fa.width);
            for (k, t) in Theme::all()
                .filter(|t| t.mode == theme.mode && t.contrast == theme.contrast)
                .enumerate()
            {
                let top = (16.0 + k as f32 * 48.0 - 6.0) as usize * w * 4;
                let end = ((16.0 + k as f32 * 48.0 + 38.0) as usize * w * 4).min(fa.rgba.len());
                let differs = fa.rgba[top..end] != fb.rgba[top..end];
                if differs != (t.name == theme.name || t.name == other.name) {
                    short.push(format!("{theme:?}: {:?}'s chip differs {differs} ", t.name));
                }
            }
        }
        assert!(short.is_empty(), "{short:#?}");
    }

    /// Invariant: A chip held down writes its label in white or black, whichever its colour leans to, in every scheme; at rest it writes in its own colour.
    /// Regression: A pressed chip keeps its own on-container ink under the state layer and its label loses its contrast.
    /// Owner: kit
    #[test]
    fn a_pressed_theme_chip_writes_in_white_or_black() {
        let mut short = Vec::new();
        for theme in Theme::all() {
            let held = Pointer {
                pressed: Some(P(0)),
                ..at_rest(None)
            };
            let first = format!("{:?}", Theme::all().next().map(|t| t.name).unwrap());
            for (pointer, expect) in [(&held, true), (&at_rest(None), false)] {
                let (canvas, _) = drawn(theme, pointer);
                let ink = canvas.ink().iter().find_map(|i| match &i.kind {
                    InkKind::Text { text, color, .. } if text.as_str() == first => Some(*color),
                    _ => None,
                });
                let Some(c) = ink else {
                    short.push(format!("{theme:?}: {first} not drawn"));
                    continue;
                };
                let plain = matches!((c.r, c.g, c.b), (255, 255, 255) | (0, 0, 0));
                if expect && !plain {
                    short.push(format!("{theme:?}: pressed label is not white or black"));
                }
            }
        }
        assert!(short.is_empty(), "{short:#?}");
    }
}
