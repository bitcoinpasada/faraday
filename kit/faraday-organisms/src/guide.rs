//! The guide organism (09 §3.3): the guide's sheet, and the search results Tools' Find and Home's
//! search draw.
//!
//! The sheet ([`draw_sheet`]) is the tabs at its head, the page's title pinned under them, the
//! page or the contents scrolled in a region of its own, and Contents and Done at its foot. Held,
//! it is the whole frame, a page under the holder's top app bar. The body is the molecule
//! `guide_block`, or the How blocks (`how`); the words come in the model as data, and the
//! organism names no feature and no page. Where the page has a section the guide is to scroll to,
//! the model's `target` anchor is registered at its place ([`Ui::anchor`]), so a feature can
//! `ViewOrder::Reveal` it. What matches Find is the feature's: the model holds the result.
//!
//! The search results ([`draw`]): under a section label the guide's words that match what is
//! typed, each a list item whose press opens the guide at its definition, or a line saying none
//! matches.

use faraday_atoms::button::{self, ButtonModel, Look};
use faraday_atoms::region::Region;
use faraday_atoms::{Anchor, Offered, Plain, Pressed, Shown, Ui, W};
use faraday_molecules::guide_block;
use faraday_molecules::row::{self, RowModel};
use faraday_molecules::tabs;
use faraday_tokens::dims::{
    BODY_M, BUTTON_H, CARET_MIN, ELEVATION_DIALOG, GAP, GAP_SMALL, GRID, GUIDE_PANE_CLOSE_TOP,
    GUIDE_PANE_PAD, GUIDE_PANE_PARA, GUIDE_PANE_TABS_TOP, GUIDE_SEARCH_AFTER, GUIDE_SEARCH_TOP,
    HOME_RESULTS_LINE, PAD, SHAPE_L, SHAPE_XL, TAB_H, TEXT_FIELD, TITLE_L, TITLE_S, TOUCH,
    TYPED_INSET,
};
use faraday_tokens::palette::Role;
use faraday_tokens::type_roles::TypeRole;

pub use faraday_molecules::guide_block::{
    GuideBlockModel, GuideBody, GuideGroup, GuideLook, GuideNone, GuidePage, GuideSection,
};
pub use faraday_molecules::tabs::{Tab, TabKind, TabsModel};

use crate::how::{self, HowModel};
use crate::nav;

/// The Find field: what is typed, the hint until something is. Typing on its page always lands
/// here, with no press to focus it first, so the caret is drawn whether or not anything is typed.
pub struct Find<'s, A> {
    pub typed: Shown<'s>,
    pub hint: Shown<'s>,
    /// The press that places the caret.
    pub offered: Offered<A>,
}

/// What the sheet's text region holds.
pub enum SheetBody<'s, A> {
    /// A guide page: its lead, its sections and the words it uses.
    Page(GuideBlockModel<'s, A>),
    /// The contents, under the Find field.
    Contents {
        find: Find<'s, A>,
        block: GuideBlockModel<'s, A>,
    },
    /// The How tab: the blocks of the screen's actions.
    How(HowModel<'s>),
}

/// The guide's sheet over the page.
pub struct GuideSheetModel<'s, A> {
    /// Held: the whole frame, a page under the holder's top app bar, no box and no Done.
    pub held: bool,
    /// The Pi's keyboard is up over the contents' Find field: the tabs and the page's title
    /// give way so the field and its matches stay in view above the keyboard.
    pub searching: bool,
    /// The tabs at the head, where the place offers more than the guide.
    pub tabs: Option<TabsModel<'s, A>>,
    /// The page's title, pinned under the tabs while the text scrolls under it.
    pub title: Option<Shown<'s>>,
    /// The type of the pinned title.
    pub head: TypeRole,
    pub body: SheetBody<'s, A>,
    /// **Contents** at the foot's start, where a page is shown.
    pub contents: Option<ButtonModel<'s, A>>,
    /// **Done** at the foot's end; none while held.
    pub done: Option<ButtonModel<'s, A>>,
    /// The press taken on the page beside the sheet.
    pub beside: Option<A>,
    /// Keys the text region's scroll.
    pub scroll: Anchor,
    /// Where the section the guide is to scroll to is registered, when the page has one.
    pub target: Option<Anchor>,
}

/// How the sheet sets its text, and the type of the pinned title.
#[must_use]
pub fn sheet_look() -> (GuideLook, TypeRole) {
    (
        GuideLook {
            heading: TITLE_S,
            body: BODY_M,
            after: [16.0, 8.0, 12.0, 10.0],
        },
        TITLE_L,
    )
}

/// How the supporting pane sets its text, and the type of the pinned title.
#[must_use]
pub fn pane_look() -> (GuideLook, TypeRole) {
    (
        GuideLook {
            heading: TITLE_S,
            body: BODY_M,
            after: [GAP, GAP_SMALL, GUIDE_PANE_PARA, GAP],
        },
        TITLE_L,
    )
}

/// The How tab's text as the guide sets it: the title in the page's head type.
#[must_use]
pub fn how_look(pane: bool) -> how::PageLook {
    let (look, head) = if pane { pane_look() } else { sheet_look() };
    how::PageLook {
        title: head,
        heading: look.heading,
        body: look.body,
        after: look.after,
    }
}

/// The height of the foot row under the text, where Contents and Done stand.
fn foot_height<A: Pressed>(ui: &Ui<'_, A>) -> f32 {
    ui.button_height(BUTTON_H) + 2.0 * GAP_SMALL
}

/// A search field at (x, y), `w` wide: Material's search field in Tools' look, the hint until
/// something is typed.
fn find_field<A: Pressed>(ui: &mut Ui<'_, A>, (fx, fy, fw): (f32, f32, f32), f: &Find<'_, A>) {
    ui.fill(fx, fy, fw, TEXT_FIELD, 8.0, Role::SurfaceContainerHighest);
    ui.stroke(fx, fy, fw, TEXT_FIELD, 8.0, Role::Outline);
    let (tx, room) = (fx + TYPED_INSET, fw - 28.0);
    ui.showing(&f.typed, |ui| {
        ui.offered(&f.offered, |ui, press, _| {
            let Some(here) = press else {
                return;
            };
            ui.press_box(fx, fy, fw, TEXT_FIELD, here.clone());
            if f.typed.as_str().is_empty() {
                let shown = ui.fit_as(BODY_M, f.hint.as_str(), room);
                ui.text_mid_as((tx, fy, TEXT_FIELD), BODY_M, Role::OnSurfaceVariant, &shown);
                ui.caret(tx, fy + (TEXT_FIELD - CARET_MIN) / 2.0, CARET_MIN);
            } else {
                ui.typed(
                    tx,
                    fy,
                    TEXT_FIELD,
                    BODY_M.size,
                    W::R,
                    Role::OnSurface,
                    f.typed.as_str(),
                    room,
                    true,
                    here,
                );
            }
        });
    });
}

/// Draws the sheet in a frame `w` by `h`: on a small panel nearly all of it, elsewhere centred
/// and at most 860 wide; held, the whole frame.
pub fn draw_sheet<A: Pressed>(m: &GuideSheetModel<'_, A>, ui: &mut Ui<'_, A>, (w, h): (f32, f32)) {
    let compact = ui.compact;
    let (sw, sh, pad) = if compact {
        (w - 16.0, h - 16.0, 14.0)
    } else {
        ((w - 80.0).min(860.0), h - 64.0, 28.0)
    };
    let (x, y, sw, sh) = if m.held {
        let sw = w.min(860.0);
        ((w - sw) / 2.0, 0.0, sw, h)
    } else {
        let (x, y) = ((w - sw) / 2.0, (h - sh) / 2.0);
        if let Some(a) = &m.beside {
            ui.press_plain((x, y, sw, sh), Plain::Beside, a.clone());
        }
        ui.shadow(x, y, sw, sh, SHAPE_XL, ELEVATION_DIALOG);
        ui.fill(x, y, sw, sh, SHAPE_XL, Role::SurfaceContainerHigh);
        if ui.theme.is_high_contrast() {
            ui.stroke(x, y, sw, sh, SHAPE_XL, Role::Outline);
        }
        (x, y, sw, sh)
    };
    let look = Metrics {
        pad,
        tabs_edge: pad,
        tabs_top: if m.searching && !m.held {
            GUIDE_SEARCH_TOP
        } else {
            14.0
        },
        after_tabs: if m.searching {
            GUIDE_SEARCH_AFTER
        } else {
            18.0
        },
        rule: 8.0,
        inset: 4.0,
        text_top: 12.0,
        clip_hits: false,
        ground: if m.held {
            Role::Surface
        } else {
            Role::SurfaceContainerHigh
        },
    };
    draw_body(m, ui, (x, y, sw, sh), &look);
}

/// How the body sits in its box: the sheet's and the pane's differ in these and in nothing else.
struct Metrics {
    pad: f32,
    /// The tabs' inset from the box's sides.
    tabs_edge: f32,
    tabs_top: f32,
    after_tabs: f32,
    /// The divider's distance over the text, where there are no tabs.
    rule: f32,
    /// The text's inset within the padding.
    inset: f32,
    text_top: f32,
    /// What is scrolled out of the text's view takes no press.
    clip_hits: bool,
    /// What the text's edges fade into.
    ground: Role,
}

/// The tabs, the pinned title, the scrolled text and the foot's Contents and Done in the box
/// (x, y, w, h), as `look` sets them.
fn draw_body<A: Pressed>(
    m: &GuideSheetModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, sw, sh): (f32, f32, f32, f32),
    look: &Metrics,
) {
    let compact = ui.compact;
    let pad = look.pad;
    let foot = foot_height(ui);
    let foot_top = y + sh - foot + GAP_SMALL;
    let searching = m.searching;
    let row = m
        .tabs
        .as_ref()
        .filter(|t| !searching && !t.items.is_empty());
    let mut tabs_h = 0.0;
    if let Some(t) = row {
        let at = (
            x + look.tabs_edge,
            y + look.tabs_top,
            sw - 2.0 * look.tabs_edge,
            TAB_H,
        );
        tabs_h = tabs::draw(t, ui, at);
    }
    let mut top = y + look.tabs_top + tabs_h + look.after_tabs;
    // Without tabs, a divider of its own over the text.
    if row.is_none() && !searching {
        ui.rule(
            x + pad,
            top - look.rule,
            sw - 2.0 * pad,
            Role::OutlineVariant,
        );
    }
    let (bx, bw) = (x + pad + look.inset, sw - 2.0 * pad - 2.0 * look.inset);
    // The page's title is pinned under the tabs: it stays where it is while the text scrolls
    // under it, so a reader partway down a page sees which page it is.
    let mut text_top = look.text_top;
    if let (Some(title), SheetBody::Page(block), false) = (&m.title, &m.body, searching) {
        let th = ui.wrap_as(
            (bx, top + text_top, bw),
            m.head,
            Role::OnSurface,
            title.as_str(),
        );
        top += text_top + th + block.look.after[0];
        text_top = 0.0;
    }
    let view = sh - (top - y) - foot;
    let clip = ui.rect(x, top, sw, view);
    let mut place = None;
    let mark = ui.presses_mark();
    ui.region(
        Region::new(m.scroll, clip).on(look.ground).bar_shown(),
        |ui| {
            let at = (bx, top + text_top, bw);
            let (used, found) = match &m.body {
                SheetBody::Page(block) => guide_block::draw_found(block, ui, at),
                SheetBody::Contents { find, block } => {
                    find_field(ui, at, find);
                    let below = TEXT_FIELD + block.look.after[3];
                    let (used, found) =
                        guide_block::draw_found(block, ui, (at.0, at.1 + below, at.2));
                    (used + below, found)
                }
                SheetBody::How(how) => how::draw_found(how, ui, at),
            };
            if let Some(off) = found {
                let one = ui.line_as(TITLE_S);
                place = Some(ui.rect(bx, top + text_top + off, bw, one));
            }
            (text_top + used - view).max(0.0)
        },
    );
    if look.clip_hits {
        ui.clip_presses_since(mark, clip);
    }
    if let (Some(id), Some(rect)) = (m.target, place) {
        // Registered outside the view's clip, so a section scrolled out of sight still has its
        // place.
        ui.c.push_clip(rect);
        ui.anchor(id);
        ui.c.pop_clip();
    }
    if let Some(b) = &m.contents {
        let bw = ui.button_width(b.label.as_str(), Look::from(b.style));
        button::draw(b, ui, (x + pad, foot_top, Some(bw), BUTTON_H));
    }
    if let Some(b) = &m.done {
        let dw = if compact { 72.0 } else { 96.0 };
        button::draw(b, ui, (x + sw - pad - dw, foot_top, Some(dw), BUTTON_H));
    }
}

/// The guide as the supporting pane (`docs/LAYOUT.md` §6) in the box (x, y, w, h): its own
/// container, the close button in the header row so that the tabs run the pane's whole width,
/// and nothing of the page under it taking a press. What is scrolled out of the text's view
/// takes none either.
pub fn draw_pane<A: Pressed>(
    m: &GuideSheetModel<'_, A>,
    (close, tip): (A, &Shown<'_>),
    ui: &mut Ui<'_, A>,
    (x, y, w, h): (f32, f32, f32, f32),
) {
    let area = ui.rect(x, y, w, h);
    ui.drop_presses_inside(area);
    ui.fill(x, y, w, h, SHAPE_L, Role::SurfaceContainer);
    let look = Metrics {
        pad: GUIDE_PANE_PAD,
        tabs_edge: 0.0,
        tabs_top: GUIDE_PANE_TABS_TOP,
        after_tabs: PAD,
        rule: GAP,
        inset: 0.0,
        text_top: GAP,
        clip_hits: true,
        ground: Role::SurfaceContainer,
    };
    draw_body(m, ui, (x, y, w, h), &look);
    nav::close_button(
        ui,
        (x + w - GAP - TOUCH, y + GUIDE_PANE_CLOSE_TOP),
        close,
        tip,
    );
}

/// The words the search found in the guide.
pub struct GuidePageModel<'s, A> {
    /// The section label over the words.
    pub label: Shown<'s>,
    /// One row for each word found: its headline the word, its press the guide at its
    /// definition.
    pub words: Vec<RowModel<'s, A>>,
    /// The line shown when nothing at all matched (a tool, a card or a word): `Some` only then.
    pub none: Option<Shown<'s>>,
}

/// Draws the results from (x, y), `w` wide. Returns the height used: none where no word was
/// found and the page found something else.
pub fn draw<A: Pressed>(
    m: &GuidePageModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    if let Some(none) = &m.none {
        ui.text_as(x, y, BODY_M, Role::OnSurfaceVariant, none.as_str());
        return ui.advance_as(BODY_M, HOME_RESULTS_LINE);
    }
    if m.words.is_empty() {
        return 0.0;
    }
    let mut cy = y;
    ui.showing(&m.label, |ui| {
        ui.text_as(x, cy, TITLE_S, Role::OnSurfaceVariant, m.label.as_str());
    });
    cy += ui.line_as(TITLE_S) + GRID;
    for word in &m.words {
        cy += row::draw(word, ui, (x, cy, w));
    }
    cy - y
}

#[cfg(test)]
mod tests {
    use super::*;
    use faraday_atoms::Offered;
    use faraday_atoms::ui::testing::{P, at_rest, canvas};
    use faraday_atoms::{MotionStore, Theme};
    use faraday_draw::canvas::InkKind;

    fn word(headline: &'static str, press: u8) -> RowModel<'static, P> {
        RowModel {
            headline: Shown::plain(headline),
            supporting: Shown::plain(""),
            trailing: None,
            offered: Offered::Yes(P(press)),
            chosen: false,
            whole: false,
            chevron: false,
        }
    }

    /// The text lines `m` draws, and the height it takes.
    fn drawn(m: &GuidePageModel<'static, P>) -> (f32, Vec<String>) {
        let mut c = canvas(160);
        let mut store = MotionStore::new();
        let pointer = at_rest(None);
        let mut ui = Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, true);
        ui.clear(Role::Surface);
        let h = draw(m, &mut ui, (16.0, 16.0, 300.0));
        let lines = ui
            .finish()
            .ink
            .expect("the record is on")
            .0
            .into_iter()
            .filter_map(|i| match i.kind {
                InkKind::Text { text, .. } => Some(text.to_string()),
                _ => None,
            })
            .collect();
        (h, lines)
    }

    /// Invariant: A word typed in the search shows the guide's matching words, each as a row under the section label, and a search that matches nothing shows the one line that says so.
    /// Regression: The words found are left off the results, or the no-match line is drawn over them or left off an empty search.
    /// Owner: kit
    #[test]
    fn a_typed_word_shows_its_matching_guide_rows_and_no_match_shows_the_line() {
        let found = GuidePageModel {
            label: Shown::plain("Words"),
            words: vec![word("Passphrase", 1), word("Seed phrase", 2)],
            none: None,
        };
        let (h, lines) = drawn(&found);
        assert!(h > 0.0);
        for want in ["Words", "Passphrase", "Seed phrase"] {
            assert!(lines.iter().any(|l| l == want), "{want}: {lines:?}");
        }
        assert!(!lines.iter().any(|l| l == "No match"), "{lines:?}");

        let none = GuidePageModel {
            label: Shown::plain("Words"),
            words: Vec::new(),
            none: Some(Shown::plain("No match")),
        };
        let (h, lines) = drawn(&none);
        assert!(h > 0.0);
        assert_eq!(lines, ["No match"]);
    }

    use crate::how::{Cross, Field, FieldBody, HowBlock, Kind, Line, PageLook, Part};
    use faraday_atoms::button::Style;
    use faraday_molecules::flow_text::FlowText;
    use faraday_molecules::guide_block::{GuideBody, GuideLook, GuidePage, GuideSection};
    use faraday_molecules::tabs::{Tab, TabKind};

    const SCROLL: Anchor = Anchor {
        scope: "guide",
        n: 1,
    };
    const TARGET: Anchor = Anchor {
        scope: "guide",
        n: 2,
    };
    const HOW: Anchor = Anchor {
        scope: "guide",
        n: 3,
    };

    fn look() -> GuideLook {
        GuideLook {
            heading: TITLE_S,
            body: BODY_M,
            after: [16.0, 8.0, 12.0, 10.0],
        }
    }

    /// A page of `n` sections, the one numbered `target` marked.
    fn page(n: usize, target: Option<usize>) -> SheetBody<'static, P> {
        let section = |i: usize| GuideSection {
            heading: Shown::plain(if i == 0 { "" } else { "Heading" }),
            paragraphs: vec![FlowText {
                runs: vec![(
                    Shown::plain(
                        "A paragraph of the guide's text, long enough to wrap onto a second line in a narrow sheet.",
                    ),
                    false,
                )],
                role: BODY_M,
            }],
            target: Some(i) == target,
        };
        SheetBody::Page(GuideBlockModel {
            look: look(),
            body: GuideBody::Page(GuidePage {
                sections: (0..n).map(section).collect(),
                words: None,
            }),
        })
    }

    fn button(label: &'static str, style: Style, n: u8) -> ButtonModel<'static, P> {
        ButtonModel {
            label: Shown::plain(label),
            style,
            offered: Offered::Yes(P(n)),
            chosen: false,
            icon: None,
        }
    }

    fn sheet(
        held: bool,
        body: SheetBody<'static, P>,
        target: Option<Anchor>,
    ) -> GuideSheetModel<'static, P> {
        GuideSheetModel {
            held,
            searching: false,
            tabs: Some(TabsModel {
                kind: TabKind::Primary,
                items: vec![
                    Tab {
                        label: Shown::plain("Guide"),
                        on: true,
                        done: false,
                        offered: Offered::Yes(P(1)),
                    },
                    Tab {
                        label: Shown::plain("Cryptography"),
                        on: false,
                        done: false,
                        offered: Offered::Yes(P(2)),
                    },
                ],
            }),
            title: Some(Shown::plain("A page")),
            head: faraday_tokens::type_roles::TITLE_L,
            body,
            contents: Some(button("Contents", Style::Text, 3)),
            done: (!held).then(|| button("Done", Style::Outlined, 4)),
            beside: None,
            scroll: SCROLL,
            target,
        }
    }

    /// The frame `m` draws in a window `w` by `h` pixels at scale 1: its text lines and anchors.
    fn drawn_sheet(
        m: &GuideSheetModel<'static, P>,
        (w, h): (u16, u16),
    ) -> (Vec<String>, Vec<(Anchor, faraday_draw::Rect)>) {
        let mut c = faraday_atoms::ui::testing::canvas_sized(160, (w, h));
        let mut store = MotionStore::new();
        let pointer = at_rest(None);
        let mut ui = Ui::new(&mut c, Theme::default(), 1.0, &mut store, &pointer, true);
        ui.clear(Role::Surface);
        draw_sheet(m, &mut ui, (f32::from(w), f32::from(h)));
        let out = ui.finish();
        let lines = out
            .ink
            .expect("the record is on")
            .0
            .into_iter()
            .filter_map(|i| match i.kind {
                InkKind::Text { text, .. } => Some(text.to_string()),
                _ => None,
            })
            .collect();
        (lines, out.anchors)
    }

    /// Invariant: The guide's sheet offers Contents and Done at its foot and the Cryptography tab at its head, and held it is the page under the holder's bar with no Done.
    /// Regression: The sheet is drawn without a way to the contents or to close it, or a held guide keeps a Done that closes the holder's page.
    /// Owner: kit
    #[test]
    fn the_sheet_offers_contents_and_done_and_held_has_no_done() {
        let (lines, _) = drawn_sheet(&sheet(false, page(2, None), None), (480, 640));
        for want in ["Guide", "Cryptography", "A page", "Contents", "Done"] {
            assert!(lines.iter().any(|l| l == want), "{want}: {lines:?}");
        }
        let (lines, _) = drawn_sheet(&sheet(true, page(2, None), None), (480, 640));
        assert!(lines.iter().any(|l| l == "Contents"), "{lines:?}");
        assert!(!lines.iter().any(|l| l == "Done"), "{lines:?}");
    }

    /// Invariant: The section the guide is to scroll to is given a place a jump can name, even where it lies far below what is in view, and a page with no such section gives none.
    /// Regression: A jump to a definition or a section lands nowhere because the place is only registered while the section is on screen, or is registered for a page that has no such section.
    /// Owner: kit
    #[test]
    fn the_target_section_has_a_place_even_below_the_view() {
        let (_, anchors) = drawn_sheet(&sheet(false, page(12, Some(10)), Some(TARGET)), (480, 640));
        let at = anchors
            .iter()
            .find(|(a, _)| *a == TARGET)
            .expect("its place")
            .1;
        assert!(at.y > 640, "{at:?}");
        let (_, anchors) = drawn_sheet(&sheet(false, page(12, None), Some(TARGET)), (480, 640));
        assert!(anchors.iter().all(|(a, _)| *a != TARGET));
    }

    /// Invariant: The How tab shows each action's name and fields, and a cross-check that does not apply says so with a warning mark and its reason.
    /// Regression: The Cryptography tab draws blocks without their fields, or leaves out why an action is not cross-checked.
    /// Owner: kit
    #[test]
    fn the_how_tab_shows_the_blocks_fields_and_why_not_cross_checked() {
        let line = |t: &'static str, kind| Line {
            text: Shown::plain(t),
            kind,
        };
        let how = HowModel {
            parts: vec![Part::Block(HowBlock {
                name: Shown::plain("Sign"),
                fields: vec![
                    Field {
                        label: Shown::plain("Standards"),
                        body: FieldBody::Lines(vec![line("BIP 174", Kind::Plain)]),
                    },
                    Field {
                        label: Shown::plain("Cross-check"),
                        body: FieldBody::Cross {
                            independence: vec![],
                            outcome: Cross::Not(Shown::plain("No second implementation")),
                        },
                    },
                ],
            })],
            look: PageLook::SHEET,
            id: HOW,
            target: None,
        };
        let mut m = sheet(false, SheetBody::How(how), None);
        m.title = None;
        m.contents = None;
        let (lines, _) = drawn_sheet(&m, (480, 640));
        for want in [
            "Sign",
            "Standards",
            "BIP 174",
            "Cross-check",
            "No second implementation",
        ] {
            assert!(lines.iter().any(|l| l == want), "{want}: {lines:?}");
        }
        assert!(!lines.iter().any(|l| l == "A page"), "{lines:?}");
    }

    /// A How tab of `n` blocks, each a name and one field of a long text, the one numbered
    /// `target` marked.
    fn how_sheet(n: usize, target: Option<usize>) -> GuideSheetModel<'static, P> {
        let block = |i: usize| {
            Part::Block(HowBlock {
                name: Shown::plain(&*Box::leak(format!("Block {i}").into_boxed_str())),
                fields: vec![Field {
                    label: Shown::plain("Standards"),
                    body: FieldBody::Lines(vec![Line {
                        text: Shown::plain(
                            "A standard whose name and number run long enough to wrap onto a second line in a narrow sheet.",
                        ),
                        kind: Kind::Plain,
                    }]),
                }],
            })
        };
        let how = HowModel {
            parts: (0..n).map(block).collect(),
            look: PageLook::SHEET,
            id: HOW,
            target,
        };
        let mut m = sheet(false, SheetBody::How(how), Some(TARGET));
        m.title = None;
        m.contents = None;
        m
    }

    /// The anchors of a frame of `m` drawn into `store`, and how many parts it measured.
    fn frame_into(
        store: &mut MotionStore,
        m: &GuideSheetModel<'static, P>,
        (w, h): (u16, u16),
    ) -> (Vec<(Anchor, faraday_draw::Rect)>, usize) {
        let mut c = faraday_atoms::ui::testing::canvas_sized(160, (w, h));
        let pointer = at_rest(None);
        let mut ui = Ui::new(&mut c, Theme::default(), 1.0, store, &pointer, true);
        ui.clear(Role::Surface);
        crate::how::MEASURED.with(|n| n.set(0));
        draw_sheet(m, &mut ui, (f32::from(w), f32::from(h)));
        let out = ui.finish();
        (out.anchors, crate::how::MEASURED.with(|n| n.get()))
    }

    /// Invariant: A second frame of the How tab at the same width and text size measures only the blocks in view, and a new width measures them again.
    /// Regression: The How tab works every block's height out on every frame, or keeps the heights of another width and puts blocks where they do not fit.
    /// Owner: kit
    #[test]
    fn the_how_tab_keeps_its_heights_for_the_next_frame() {
        let m = how_sheet(40, None);
        let mut store = MotionStore::new();
        let (_, first) = frame_into(&mut store, &m, (480, 640));
        assert_eq!(first, 40);
        let (_, second) = frame_into(&mut store, &m, (480, 640));
        assert!(second < 10, "{second}");
        let (_, narrow) = frame_into(&mut store, &m, (360, 640));
        assert_eq!(narrow, 40);
    }

    /// Invariant: A jump to a How block scrolled far out of sight has the same place on the first frame and on the frames that skip the blocks out of view.
    /// Regression: A Reveal to a block far down the How tab lands short or past it once the heights are kept, because a skipped block was counted at another height.
    /// Owner: kit
    #[test]
    fn a_jump_to_a_how_block_out_of_sight_lands_on_it_with_the_heights_kept() {
        let m = how_sheet(40, Some(30));
        let mut store = MotionStore::new();
        let place = |anchors: &[(Anchor, faraday_draw::Rect)]| {
            anchors
                .iter()
                .find(|(a, _)| *a == TARGET)
                .expect("its place")
                .1
        };
        let (first, _) = frame_into(&mut store, &m, (480, 640));
        let (second, measured) = frame_into(&mut store, &m, (480, 640));
        assert!(place(&first).y > 640, "{:?}", place(&first));
        assert!(measured < 10, "{measured}");
        assert_eq!(place(&first), place(&second));
    }
}
