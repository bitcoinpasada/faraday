//! The row molecule: Material 3's list item, of one, two or three lines (09 §3.2).

use faraday_atoms::hits::Shape;
use faraday_atoms::{Offered, Pressed, Shown, Ui};
use faraday_tokens::dims::{
    BODY_L, BODY_M, GAP, GAP_SMALL, ICON_SMALL, LABEL_S, LIST_1, LIST_2, LIST_3, LIST_3_TOP,
    LIST_PAD, LIST_PAD_V, SHAPE_L,
};
use faraday_tokens::palette::{Paint, Role, on_selected, selected_or};

/// A list item: its headline, its supporting text (none for a one-line item), what stands at
/// its end and in what colour, and what a press on it does (04 §12.1).
pub struct RowModel<'s, A> {
    /// The headline: Material's body large.
    pub headline: Shown<'s>,
    /// The supporting text under it, on one line or two: Material's body medium. Empty for a
    /// one-line item.
    pub supporting: Shown<'s>,
    /// The trailing text, level with the headline: Material's label small.
    pub trailing: Option<(Shown<'s>, Paint)>,
    /// What a press does: `Offered::No` is a row that only states what is there, drawn as usual.
    pub offered: Offered<A>,
    /// Material's selected state (secondary container).
    pub chosen: bool,
    /// The headline is an address already in groups of four: in the mono face, never cut, on
    /// one line where it fits and wrapped between groups where the room requires it.
    pub whole: bool,
    /// A chevron at the end, which says the item opens a page.
    pub chevron: bool,
}

/// How many lines the item takes `w` wide: one with no supporting text, two where it fits on a
/// line, three where it takes two.
fn lines<A: Pressed>(m: &RowModel<'_, A>, ui: &Ui<'_, A>, w: f32) -> usize {
    let sup = m.supporting.as_str();
    if sup.is_empty() {
        1
    } else if ui.measure_as(BODY_M, sup) <= w - 2.0 * LIST_PAD {
        2
    } else {
        3
    }
}

/// The room the headline has `w` wide: the item's inside, less its trailing text and the gap
/// before it.
fn head_room<A: Pressed>(m: &RowModel<'_, A>, ui: &Ui<'_, A>, w: f32) -> f32 {
    let inner = w - 2.0 * LIST_PAD;
    match &m.trailing {
        Some((t, _)) => inner - ui.measure_as(LABEL_S, t.as_str()) - GAP,
        None => inner,
    }
}

/// The height of one headline line: the mono face's where the headline is a whole address.
fn head_line<A: Pressed>(ui: &Ui<'_, A>, whole: bool) -> f32 {
    if whole {
        ui.line_as(BODY_L.mono())
    } else {
        ui.line_as(BODY_L)
    }
}

/// How many lines the headline takes `w` wide: one at Standard, where it is cut to fit; at a
/// larger text size two where it does not fit on one (`docs/TEXT-SIZE.md` §3), the second cut
/// to fit.
fn head_lines<A: Pressed>(m: &RowModel<'_, A>, ui: &Ui<'_, A>, w: f32) -> usize {
    let head = m.headline.as_str();
    if m.whole {
        let room = head_room(m, ui, w);
        return (ui.wrap_height_as(BODY_L.mono(), head, room) / head_line(ui, true))
            .round()
            .max(1.0) as usize;
    }
    if ui.text_factor() <= 1.0 || ui.measure_as(BODY_L, head) <= head_room(m, ui, w) {
        1
    } else {
        2
    }
}

/// The room the chevron takes at the item's end, which the text lays out without.
fn text_width(chevron: bool, w: f32) -> f32 {
    if chevron {
        w - ICON_SMALL - GAP_SMALL
    } else {
        w
    }
}

/// The height the row takes `w` wide: [`LIST_1`], [`LIST_2`] or [`LIST_3`], or its lines and
/// Material's padding where larger text needs more.
pub fn height<A: Pressed>(m: &RowModel<'_, A>, ui: &Ui<'_, A>, w: f32) -> f32 {
    let w = text_width(m.chevron, w);
    let (head, sup) = (head_line(ui, m.whole), ui.line_as(BODY_M));
    // A headline larger text breaks onto a second line adds it.
    let head = head * head_lines(m, ui, w) as f32;
    let (token, lines, pad) = match lines(m, ui, w) {
        1 => (LIST_1, head, LIST_PAD_V),
        2 => (LIST_2, head + sup, LIST_PAD_V),
        _ => (LIST_3, head + 2.0 * sup, LIST_3_TOP),
    };
    // A whole address takes its lines at every text size.
    if m.whole {
        return token.max(lines + 2.0 * pad);
    }
    ui.row_height(token, lines, pad)
}

/// Draws the row from (x, y), `w` wide. The whole item takes its press. Returns its height.
pub fn draw<A: Pressed>(
    m: &RowModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let h = height(m, ui, w);
    if !ui.shows(y, h) {
        return h;
    }
    ui.showing(&m.headline, |ui| {
        ui.showing(&m.supporting, |ui| drawn(m, ui, (x, y, w), h))
    });
    h
}

/// [`draw`] of a row that is in view, `h` tall.
fn drawn<A: Pressed>(
    m: &RowModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, full): (f32, f32, f32),
    h: f32,
) {
    let w = text_width(m.chevron, full);
    let n = lines(m, ui, w);
    if m.chosen {
        ui.fill(x, y, w, h, SHAPE_L, selected_or(true, Role::Surface));
    }
    let ink = on_selected(m.chosen, Role::OnSurface);
    ui.offered(&m.offered, |ui, press, _| {
        if let Some(action) = press {
            ui.press(Shape::new(x, y, full, h, SHAPE_L), ink, action);
        }
    });
    let (line, sup) = (head_line(ui, m.whole), ui.line_as(BODY_M));
    let head_n = head_lines(m, ui, w);
    let head = line * head_n as f32;
    let top = match n {
        1 => y + (h - head) / 2.0,
        2 => y + (h - head - sup) / 2.0,
        _ => y + LIST_3_TOP,
    };
    let (tx, inner) = (x + LIST_PAD, w - 2.0 * LIST_PAD);
    let room = head_room(m, ui, w);
    if let Some((t, tone)) = &m.trailing {
        let tone = if m.chosen { ink } else { *tone };
        ui.text_right_as((x + w - LIST_PAD, top, line), LABEL_S, tone, t.as_str());
    }
    let headline = m.headline.as_str();
    if m.whole {
        ui.wrap_as((tx, top, room), BODY_L.mono(), ink, headline);
    } else if head_n == 1 {
        let name = ui.fit_as(BODY_L, headline, room);
        ui.text_mid_as((tx, top, line), BODY_L, ink, &name);
    } else {
        let end = ui.first_line_end(BODY_L, headline, room);
        let (one, rest) = headline.split_at(end);
        ui.text_mid_as((tx, top, line), BODY_L, ink, one.trim_end());
        let rest = ui.fit_as(BODY_L, rest.trim_start(), room);
        ui.text_mid_as((tx, top + line, line), BODY_L, ink, &rest);
    }
    let muted = on_selected(m.chosen, Role::OnSurfaceVariant);
    let supporting = m.supporting.as_str();
    let first = if n < 3 {
        supporting.len()
    } else {
        ui.first_line_end(BODY_M, supporting, inner)
    };
    let (one, rest) = supporting.split_at(first);
    ui.text_mid_as((tx, top + head, sup), BODY_M, muted, one.trim_end());
    if n == 3 {
        let rest = ui.fit_as(BODY_M, rest.trim_start(), inner);
        ui.text_mid_as((tx, top + head + sup, sup), BODY_M, muted, &rest);
    }
    if m.chevron {
        ui.row_chevron(x + full - LIST_PAD, y, h, muted);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::TextSize;
    use faraday_atoms::ui::testing::{P, at_rest, canvas};
    use faraday_atoms::{MotionStore, Theme};
    use faraday_draw::Rect;
    use faraday_draw::canvas::InkKind;

    /// A row of `headline` and `supporting`, with nothing at its end.
    fn model(headline: &'static str, supporting: &'static str) -> RowModel<'static, P> {
        RowModel {
            headline: Shown::plain(headline),
            supporting: Shown::plain(supporting),
            trailing: Some((Shown::plain("Now"), Role::OnSurfaceVariant.paint())),
            offered: Offered::Yes(P(1)),
            chosen: false,
            whole: false,
            chevron: false,
        }
    }

    /// The text lines the row drew at Standard or `size`, each with its box, and the height the
    /// row took, drawn from (16, 16) `w` wide.
    fn drawn(m: &RowModel<'static, P>, w: f32, size: TextSize) -> (f32, Vec<(String, Rect)>) {
        let mut c = canvas(160);
        let mut store = MotionStore::new();
        let pointer = at_rest(None);
        let mut ui = Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, true);
        ui.set_text_size(size);
        ui.clear(Role::Surface);
        let h = draw(m, &mut ui, (16.0, 16.0, w));
        let lines = ui
            .finish()
            .ink
            .expect("the record is on")
            .0
            .into_iter()
            .filter_map(|i| match i.kind {
                InkKind::Text { text, .. } => Some((text.to_string(), i.rect)),
                _ => None,
            })
            .collect();
        (h, lines)
    }

    /// Invariant: A list item's supporting text too long for one line takes a second inside its item, cut with an ellipsis rather than running past its end, under its headline.
    /// Regression: A long supporting text runs past the item's end, or over its headline.
    /// Owner: kit
    #[test]
    fn a_list_items_long_supporting_text_keeps_inside_its_item() {
        let long = "Three keys at three places, each a different kind, so that no one loss or theft opens it";
        let (_, lines) = drawn(&model("Family vault", long), 300.0, TextSize::Standard);
        let find = |s: &str| {
            lines
                .iter()
                .filter(|(t, _)| t.starts_with(s))
                .collect::<Vec<_>>()
        };
        let first = find("Three keys");
        assert_eq!(first.len(), 1, "{lines:?}");
        let head = find("Family vault");
        assert_eq!(head.len(), 1, "{lines:?}");
        assert!(head[0].1.y < first[0].1.y, "{head:?} not over {first:?}");
        let second: Vec<_> = lines
            .iter()
            .filter(|(t, r)| r.y > first[0].1.y && r.x == first[0].1.x && t.ends_with('…'))
            .collect();
        assert_eq!(second.len(), 1, "{lines:?}");
        for r in [first[0].1, second[0].1] {
            assert!(r.x >= 16 && r.right() <= 316, "{r:?} past the item");
        }
    }

    /// Invariant: At Larger a list item's headline too long for one line is shown whole on two, inside its item, and the item grows to hold them over its supporting line; at Standard the same headline is one line.
    /// Regression: A larger headline is cut to one line, or runs past the item's end or over its supporting line.
    /// Owner: kit
    #[test]
    fn at_larger_a_list_items_long_headline_takes_two_lines_inside_its_item() {
        let head = "Savings for the family";
        let mut m = model(head, "2 of 3");
        m.trailing = None;
        let (_, standard) = drawn(&m, 240.0, TextSize::Standard);
        assert!(standard.iter().any(|(t, _)| t == head), "{standard:?}");
        let (h, larger) = drawn(&m, 240.0, TextSize::Larger);
        let heads: Vec<_> = larger.iter().filter(|(t, _)| t != "2 of 3").collect();
        assert_eq!(heads.len(), 2, "{larger:?}");
        let words: Vec<&str> = heads.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(words.join(" "), head);
        let sup = larger.iter().find(|(t, _)| t == "2 of 3").map(|(_, r)| *r);
        let sup = sup.expect("the supporting line is shown");
        for (_, r) in &heads {
            assert!(r.x >= 16 && r.right() <= 256, "{r:?} past the item");
            assert!(r.bottom() <= sup.y + 1, "{r:?} over the supporting line");
        }
        assert!(
            sup.bottom() as f32 <= 16.0 + h,
            "{sup:?} past the item's foot"
        );
    }
}
