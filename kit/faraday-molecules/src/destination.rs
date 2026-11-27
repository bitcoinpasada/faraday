//! The destination molecule: one destination of the navigation rail or bar, its icon on a pill
//! over its label, with its badge (04 §12.1, 09 §3.2).

use faraday_atoms::hits::Shape;
use faraday_atoms::{Glyph, Offered, Pressed, Shown, Ui};
use faraday_draw::canvas::InkKind;
use faraday_tokens::dims::{
    BADGE_LABEL, BADGE_LARGE, BADGE_PAD, BADGE_SMALL, GAP_SMALL, LABEL_M, NAV_ICON,
    NAV_LABEL_GAP, NAV_PILL_H, NAV_PILL_W,
};
use faraday_tokens::palette::{BADGE, ON_BADGE, Paint, Role};

/// Material's badge on a destination's icon: a small dot where something waits, or a large
/// badge with a count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Badge {
    /// None.
    #[default]
    None,
    /// The small badge: a dot.
    Dot,
    /// The large badge with a count, the model's `many` past 999 (Material's most characters,
    /// four).
    Count(usize),
}

/// One destination: its icon, its label, what a press on it does, whether it is the one shown
/// now, and its badge.
pub struct DestinationModel<'s, A> {
    /// Its icon.
    pub icon: Glyph,
    /// Its label.
    pub label: Shown<'s>,
    /// What a press on it does.
    pub offered: Offered<A>,
    /// It is the destination shown now.
    pub current: bool,
    /// What waits there, on its icon.
    pub badge: Badge,
    /// How many flows in progress stand there, on its icon too.
    pub flows: usize,
    /// What a count past 999 is written as ("999+").
    pub many: Shown<'s>,
}

/// A large badge's count as it is written.
fn count_label(m: &DestinationModel<'_, impl Pressed>, n: usize) -> String {
    if n > 999 {
        m.many.as_str().to_string()
    } else {
        n.to_string()
    }
}

/// A large badge's width for `label`.
fn count_width<A: Pressed>(ui: &Ui<'_, A>, label: &str) -> f32 {
    (ui.measure(BADGE_LABEL, faraday_atoms::W::S, label) + 2.0 * BADGE_PAD).max(BADGE_LARGE)
}

/// A large badge's pill at (x, y), `label` centred in it.
fn count_pill<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y): (f32, f32),
    label: &str,
    (fill, ink): (Paint, Paint),
) {
    let h = BADGE_LARGE;
    let w = count_width(ui, label);
    ui.fill(x, y, w, h, h / 2.0, fill);
    let tw = ui.measure(BADGE_LABEL, faraday_atoms::W::S, label);
    ui.text_mid(x + (w - tw) / 2.0, y, h, BADGE_LABEL, faraday_atoms::W::S, ink, label);
}

/// Draws the badge on an icon glyph whose box is `glyph` (x, y, side): the dot on its top
/// right corner, the large badge from its middle across its top right, as Material places them,
/// in Bitcoin orange with a white count, in every theme.
fn badge<A: Pressed>(
    m: &DestinationModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (gx, gy, gs): (f32, f32, f32),
) {
    match m.badge {
        Badge::None | Badge::Count(0) => {}
        Badge::Dot => {
            let d = BADGE_SMALL;
            let (x, y) = (gx + gs - d / 2.0, gy - d / 2.0);
            ui.fill(x, y, d, d, d / 2.0, BADGE);
            let r = ui.rect(x, y, d, d);
            ui.c.note(InkKind::Dot, r);
        }
        Badge::Count(n) => {
            let label = count_label(m, n);
            let at = (gx + gs / 2.0, gy - BADGE_LARGE / 2.0);
            count_pill(ui, at, &label, (BADGE, ON_BADGE));
        }
    }
}

/// The mark of the flows in progress on the icon: a pill in the primary colour at the corner,
/// beside the destination's own badge where it has one.
fn flow_pill<A: Pressed>(
    m: &DestinationModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (gx, gy, gs): (f32, f32, f32),
) {
    if m.flows == 0 {
        return;
    }
    let corner = gx + gs / 2.0;
    let x = match m.badge {
        Badge::None | Badge::Count(0) => corner,
        Badge::Dot => gx + gs + BADGE_SMALL / 2.0 + GAP_SMALL,
        Badge::Count(n) => corner + count_width(ui, &count_label(m, n)) + GAP_SMALL,
    };
    let at = (x, gy - BADGE_LARGE / 2.0);
    count_pill(ui, at, &count_label(m, m.flows), (Role::Primary.paint(), Role::OnPrimary.paint()));
}

/// The height a destination's pill and label take together, its label's line at the text size.
fn content<A: Pressed>(ui: &Ui<'_, A>) -> f32 {
    NAV_PILL_H + NAV_LABEL_GAP + ui.line_as(LABEL_M)
}

/// Draws the destination centred in the box at (x, y), `w` by `h`, the whole box pressable.
pub fn draw<A: Pressed>(
    m: &DestinationModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w, h): (f32, f32, f32, f32),
) {
    let top = y + ((h - content(ui)) / 2.0).max(0.0);
    let px = x + (w - NAV_PILL_W) / 2.0;
    if m.current {
        ui.fill(px, top, NAV_PILL_W, NAV_PILL_H, NAV_PILL_H / 2.0, Role::SecondaryContainer);
    }
    let (icon_fg, fg) = if m.current {
        (Role::OnSecondaryContainer, Role::OnSurface)
    } else {
        (Role::OnSurfaceVariant, Role::OnSurfaceVariant)
    };
    // Ringed round its pill, the shape its state layer takes.
    ui.offered(&m.offered, |ui, press, _| {
        if let Some(a) = press {
            let shape = Shape::new(px, top, NAV_PILL_W, NAV_PILL_H, NAV_PILL_H / 2.0);
            ui.press_to(shape, (x, y, w, h), icon_fg.paint(), a);
        }
    });
    let ix = x + (w - NAV_PILL_H) / 2.0;
    ui.icon(ix, top, NAV_PILL_H, m.icon, NAV_ICON, icon_fg);
    let inset = (NAV_PILL_H - NAV_ICON) / 2.0;
    let glyph = (ix + inset, top + inset, NAV_ICON);
    ui.showing(&m.many, |ui| {
        badge(m, ui, glyph);
        flow_pill(m, ui, glyph);
    });
    // Material's label medium, the current destination's heavier.
    let role = if m.current { LABEL_M } else { LABEL_M.regular() };
    // A one-line slot: fitted down to Standard (`docs/TEXT-SIZE.md` §3).
    let room = w - 2.0 * GAP_SMALL;
    let label = m.label.as_str();
    ui.showing(&m.label, |ui| {
        let lw = ui.text_fit_width(role, label, room);
        let ly = top + NAV_PILL_H + NAV_LABEL_GAP;
        // The band its own line tall, so the label's top is at `ly`.
        let band = (x + (w - lw) / 2.0, ly, ui.line(role.size, role.face.into()), room);
        ui.text_fit_as(band, role, fg, label);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas};
    use faraday_atoms::{MotionStore, Theme};
    use faraday_draw::canvas::InkKind;
    use faraday_tokens::palette::Role;

    /// The text the destination drew, drawn in a box 80 by 80 with a count of `n` on it.
    fn texts(n: usize) -> Vec<String> {
        let mut c = canvas(160);
        let mut store = MotionStore::new();
        let pointer = at_rest(None);
        let mut ui: Ui<'_, P> = Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, true);
        ui.clear(Role::Surface);
        let m = DestinationModel {
            icon: Glyph::Icon(faraday_draw::icons::Icon::Done),
            label: Shown::plain("Home"),
            offered: Offered::Yes(P(1)),
            current: false,
            badge: Badge::Count(n),
            flows: 0,
            many: Shown::plain("999+"),
        };
        draw(&m, &mut ui, (0.0, 0.0, 80.0, 80.0));
        ui.finish()
            .ink
            .expect("the record is on")
            .0
            .into_iter()
            .filter_map(|i| match i.kind {
                InkKind::Text { text, .. } => Some(text.to_string()),
                _ => None,
            })
            .collect()
    }

    /// Invariant: A destination's badge writes its count, and past 999 the wording given for it, never the number.
    /// Regression: A count past 999 is drawn in full and widens the badge past the icon, or the count is dropped.
    /// Owner: kit
    #[test]
    fn a_badge_writes_its_count_and_past_999_the_wording_for_many() {
        assert!(texts(7).contains(&"7".to_string()));
        let many = texts(1200);
        assert!(many.contains(&"999+".to_string()), "{many:?}");
        assert!(!many.contains(&"1200".to_string()));
    }
}
