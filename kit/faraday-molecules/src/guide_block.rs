//! The guide block molecule: the text of a guide page, or the guide's contents, laid out from a
//! point and `w` wide (`docs/GUIDE-IN-APP.md`; 09 §3.2). The tabs, the title pinned over the
//! text, the search field, the scroll and the foot are the guide organism's.

use faraday_atoms::button::Look;
use faraday_atoms::{Offered, Pressed, Shown, Ui};
use faraday_tokens::dims::BUTTON_H;
use faraday_tokens::palette::Role;
use faraday_tokens::type_roles::TypeRole;

use crate::flow_text::{self, FlowText};
use crate::row::{self, RowModel};

/// How a guide's text is set: the type roles and the room after each part.
pub struct GuideLook {
    /// A section's heading, or a group's.
    pub heading: TypeRole,
    /// The running text and the line that says nothing matched.
    pub body: TypeRole,
    /// The room under the title, a heading, a paragraph and a section, in that order.
    pub after: [f32; 4],
}

/// One section of a page: its heading and its paragraphs. The page's first section, its lead,
/// has no heading (the title is the organism's).
pub struct GuideSection<'s> {
    /// The heading; empty for the lead.
    pub heading: Shown<'s>,
    /// The paragraphs: runs of text, the bold ones in the medium face.
    pub paragraphs: Vec<FlowText<'s>>,
    /// The scroll goes to this section: its top, within the block, is returned by
    /// [`draw_found`].
    pub target: bool,
}

/// What a page's block holds.
pub struct GuidePage<'s, A> {
    /// The sections, the lead first.
    pub sections: Vec<GuideSection<'s>>,
    /// The heading over the words the page's screen uses, and a row for each, a press of which
    /// shows the word's definition; none for a page with no words. Drawn after the lead.
    pub words: Option<(Shown<'s>, Vec<RowModel<'s, A>>)>,
}

/// A group of the contents: its heading and a one-line row for each page.
pub struct GuideGroup<'s, A> {
    /// The heading; empty for none (the rows alone, as while the keyboard is up).
    pub heading: Shown<'s>,
    /// One row for each page.
    pub rows: Vec<RowModel<'s, A>>,
}

/// The line that says no page matched, and the button that empties the search.
pub struct GuideNone<'s, A> {
    /// What it says.
    pub line: Shown<'s>,
    /// The button's label.
    pub clear: Shown<'s>,
    /// What the button does.
    pub offered: Offered<A>,
}

/// What the block shows.
pub enum GuideBody<'s, A> {
    /// A page's text.
    Page(GuidePage<'s, A>),
    /// The contents: the groups that matched, or `none` where none did.
    Contents {
        groups: Vec<GuideGroup<'s, A>>,
        none: Option<GuideNone<'s, A>>,
    },
}

/// A guide page's text or the guide's contents.
pub struct GuideBlockModel<'s, A> {
    /// How it is set.
    pub look: GuideLook,
    /// What it shows.
    pub body: GuideBody<'s, A>,
}

/// `text` in `role` wrapped to `w` from (x, y), only the lines in view drawn. Returns the height.
fn one<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    role: TypeRole,
    text: &str,
) -> f32 {
    let laid = ui.wrapped_as(role, text, w);
    ui.draw_wrapped_seen((x, y), Role::OnSurface, &laid)
}

/// A paragraph from (x, y), `w` wide: a single run as one wrapped text, several as a flow in
/// which the bold runs stand in the medium face. Only the lines in view are drawn.
fn paragraph<A: Pressed>(ui: &mut Ui<'_, A>, at: (f32, f32, f32), p: &FlowText<'_>) -> f32 {
    if let [(run, bold)] = p.runs.as_slice() {
        let role = if *bold { p.role.medium() } else { p.role };
        return ui.showing(run, |ui| one(ui, at, role, run.as_str()));
    }
    if !ui.shows(at.1, flow_text::height(p, ui, at.2)) {
        return flow_text::height(p, ui, at.2);
    }
    flow_text::draw(p, ui, at)
}

/// Draws the block from (x, y), `w` wide. Returns the height.
pub fn draw<A: Pressed>(
    m: &GuideBlockModel<'_, A>,
    ui: &mut Ui<'_, A>,
    at: (f32, f32, f32),
) -> f32 {
    draw_found(m, ui, at).0
}

/// [`draw`], and where within the block the section marked `target` begins, if the page has one.
pub fn draw_found<A: Pressed>(
    m: &GuideBlockModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> (f32, Option<f32>) {
    let [_, after_heading, after_para, after_section] = m.look.after;
    let look = &m.look;
    let mut cy = y;
    let mut found = None;
    match &m.body {
        GuideBody::Page(page) => {
            for (i, section) in page.sections.iter().enumerate() {
                if section.target {
                    found = Some(cy - y);
                }
                if i > 0 {
                    let head = &section.heading;
                    cy += ui.showing(head, |ui| one(ui, (x, cy, w), look.heading, head.as_str()))
                        + after_heading;
                }
                for p in &section.paragraphs {
                    cy += paragraph(ui, (x, cy, w), p) + after_para;
                }
                if i == 0
                    && let Some((head, rows)) = page.words.as_ref().filter(|(_, r)| !r.is_empty())
                {
                    cy += ui.wrap_as((x, cy, w), look.heading, Role::OnSurface, head.as_str())
                        + after_heading;
                    for r in rows {
                        cy += row::draw(r, ui, (x, cy, w));
                    }
                }
                cy += after_section;
            }
        }
        GuideBody::Contents { groups, none } => {
            for g in groups {
                if !g.heading.as_str().is_empty() {
                    cy += ui.wrap_as(
                        (x, cy, w),
                        look.heading,
                        Role::OnSurface,
                        g.heading.as_str(),
                    ) + after_heading;
                }
                for r in &g.rows {
                    cy += row::draw(r, ui, (x, cy, w));
                }
                cy += after_section;
            }
            if let Some(n) = none {
                // An empty result carries its way on: the button that empties the field.
                cy += ui.wrap_as(
                    (x, cy, w),
                    look.body,
                    Role::OnSurfaceVariant,
                    n.line.as_str(),
                );
                cy += after_heading;
                let bw = ui.button_width(n.clear.as_str(), Look::Secondary);
                ui.offered(&n.offered, |ui, press, _| {
                    if let Some(a) = press {
                        ui.button(
                            x,
                            cy,
                            Some(bw),
                            BUTTON_H,
                            n.clear.as_str(),
                            Look::Secondary,
                            a,
                        );
                    }
                });
                cy += BUTTON_H + after_section;
            }
        }
    }
    (cy - y, found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas};
    use faraday_atoms::{MotionStore, Theme};
    use faraday_tokens::dims::{BODY_M, TITLE_S};

    fn page(target: usize) -> GuideBlockModel<'static, P> {
        let para = |s: &'static str| FlowText {
            runs: vec![(Shown::plain(s), false), (Shown::plain(" and more"), true)],
            role: BODY_M,
        };
        let section = |head: &'static str, t: bool| GuideSection {
            heading: Shown::plain(head),
            paragraphs: vec![para("A paragraph of the guide's text")],
            target: t,
        };
        GuideBlockModel {
            look: GuideLook {
                heading: TITLE_S,
                body: BODY_M,
                after: [16.0, 8.0, 12.0, 10.0],
            },
            body: GuideBody::Page(GuidePage {
                sections: vec![
                    section("", false),
                    section("Second", target == 1),
                    section("Third", target == 2),
                ],
                words: None,
            }),
        }
    }

    /// Invariant: A guide page tells where the section the guide is to scroll to begins, below the sections before it, and nothing where the page has no such section.
    /// Regression: The guide opens a definition at the top of the page, or a section's offset is lost so the scroll lands on the wrong text.
    /// Owner: kit
    #[test]
    fn a_page_tells_where_the_section_to_scroll_to_begins() {
        let mut c = canvas(160);
        let mut store = MotionStore::new();
        let pointer = at_rest(None);
        let mut ui: Ui<'_, P> = Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, true);
        let (h1, at1) = draw_found(&page(1), &mut ui, (0.0, 0.0, 300.0));
        let (_, at2) = draw_found(&page(2), &mut ui, (0.0, 0.0, 300.0));
        let (_, none) = draw_found(&page(9), &mut ui, (0.0, 0.0, 300.0));
        let (at1, at2) = (at1.expect("the second section"), at2.expect("the third"));
        assert!(at1 > 0.0 && at2 > at1 && at2 < h1);
        assert_eq!(none, None);
    }
}
