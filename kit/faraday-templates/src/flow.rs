//! The step-flow template (04 §5, §12.4): the heading and title, the card, Your choices, what
//! stands under it, the foot (the refusal, the busy line, Back and Cancel) and the asking
//! dialog. The small panel's page per step and its pin bar are `compact`.
//!
//! Left: the open step's pick, the last step's take-away and the lead row of Your choices
//! (`FlowModel` has no fields for them), Your choices as the top bar's button and sheet on the
//! small panel, the card's raised look at High contrast, and the fade through itself (the
//! runtime's: the template only names the place, `Ui::anchor`).

use faraday_atoms::button::{ButtonModel, Style};
use faraday_atoms::hits::Shape;
use faraday_atoms::region::Region;
use faraday_atoms::{Anchor, Offered, Pressed, Shown, Ui};
use faraday_molecules::buttons::draw_items;
use faraday_molecules::status::{self, BusyModel};
use faraday_organisms::content::{self, Content};
use faraday_organisms::sheet::{self, DialogModel};
use faraday_tokens::dims::{
    BODY_S, CHOICES_INSET, CHOICES_ROW, COLUMN_MAX, FLOW_END_PAD, GAP, GRID, LABEL_L, MARGIN_WIDE,
    PAD, PAGE_TOP_BAR, SHAPE_L, TITLE_M,
};
use faraday_tokens::palette::Role;

use crate::panes;
use crate::screen::page_room;

/// How wide a step's card is (LAYOUT §14): the kernel's `Width` (03 §9.1), mapped by the
/// presentation kernel, since the kit may not name the kernel (04 §5).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Width {
    Column,
    Wide,
    Full,
}

/// One row of Your choices (04 §5).
pub struct ChoiceRow<'s, A> {
    /// The step's title.
    pub title: Shown<'s>,
    /// The step's answer worded by `ShowFlow::choice`, or "Not chosen yet".
    pub answer: Shown<'s>,
    /// A press opens the step.
    pub offered: Offered<A>,
}

/// The words the template draws itself: the template names no strings crate, so the fill gives
/// them.
pub struct FlowWords<'s> {
    /// The heading of Your choices.
    pub your_choices: Shown<'s>,
    /// Back's label.
    pub back: Shown<'s>,
    /// Cancel's label.
    pub cancel: Shown<'s>,
}

/// What the step-flow template draws, filled once for every flow (04 §5, rule V12). Its fields
/// are private and it is built only by `new` and the `with_*` methods, which
/// `disallowed-methods` lets only the presentation kernel's fill call.
pub struct FlowModel<'s, A> {
    heading: Shown<'s>,
    title: Shown<'s>,
    card: Content<'s, A>,
    choices: Vec<ChoiceRow<'s, A>>,
    under: Option<Content<'s, A>>,
    back: Offered<A>,
    cancel: Option<Offered<A>>,
    asking: Option<Box<DialogModel<'s, A>>>,
    refused: Option<Shown<'s>>,
    busy: Option<BusyModel<'s>>,
    width: Width,
    /// The anchor of the fade through.
    step: Anchor,
    words: FlowWords<'s>,
}

impl<'s, A> FlowModel<'s, A> {
    /// The fields every flow has: the heading, the step's title and card, Back, the card's width,
    /// the anchor of the fade through and the words the template draws. The rest is named by the `with_*` methods.
    pub fn new(
        heading: Shown<'s>,
        title: Shown<'s>,
        card: Content<'s, A>,
        back: Offered<A>,
        width: Width,
        step: Anchor,
        words: FlowWords<'s>,
    ) -> Self {
        Self {
            heading,
            title,
            card,
            choices: Vec::new(),
            under: None,
            back,
            cancel: None,
            asking: None,
            refused: None,
            busy: None,
            width,
            step,
            words,
        }
    }

    /// Your choices: one row for each step answered or still to answer.
    pub fn with_choices(mut self, choices: Vec<ChoiceRow<'s, A>>) -> Self {
        self.choices = choices;
        self
    }

    /// The content under the card.
    pub fn with_under(mut self, under: Content<'s, A>) -> Self {
        self.under = Some(under);
        self
    }

    /// The Cancel the flow offers.
    pub fn with_cancel(mut self, cancel: Offered<A>) -> Self {
        self.cancel = Some(cancel);
        self
    }

    /// The dialog the flow is asking.
    pub fn with_asking(mut self, asking: Box<DialogModel<'s, A>>) -> Self {
        self.asking = Some(asking);
        self
    }

    /// The refusal the flow shows.
    pub fn with_refused(mut self, refused: Shown<'s>) -> Self {
        self.refused = Some(refused);
        self
    }

    /// The busy line the flow shows while it works.
    pub fn with_busy(mut self, busy: BusyModel<'s>) -> Self {
        self.busy = Some(busy);
        self
    }
}

/// The anchor the card's scroll is kept by: one for each step, so a step just opened starts at
/// its top (the fade through is `FlowModel::step` itself).
fn card_scroll(step: Anchor) -> Anchor {
    Anchor {
        scope: "flow card",
        n: step.n,
    }
}

/// The step count at the right of the title line, and the title wrapped in what it leaves.
/// Returns the height, the gap under it counted.
fn heading<A: Pressed>(
    m: &FlowModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let lh = ui.line_as(TITLE_M);
    let count = m.heading.as_str();
    let count_w = ui.showing(&m.heading, |ui| ui.measure_as(BODY_S, count));
    let th = ui.showing(&m.title, |ui| {
        ui.wrap_as(
            (x, y, w - count_w - GAP - 2.0),
            TITLE_M,
            Role::OnSurface,
            m.title.as_str(),
        )
    });
    ui.showing(&m.heading, |ui| {
        ui.text_right_as((x + w, y, lh), BODY_S, Role::OnSurfaceVariant, count);
    });
    th + GAP + 2.0
}

/// A text button for Back or Cancel.
fn text_button<'s, A: Pressed>(label: &Shown<'s>, offered: &Offered<A>) -> ButtonModel<'s, A> {
    ButtonModel {
        label: label.clone(),
        style: Style::Text,
        offered: offered.clone(),
        chosen: false,
        icon: None,
    }
}

/// Back, and Cancel where the flow offers it, in one row: at the row's end on the wide layout,
/// sharing its width on the small panel. Returns the height.
fn nav_row<A: Pressed>(m: &FlowModel<'_, A>, ui: &mut Ui<'_, A>, at: (f32, f32, f32)) -> f32 {
    let mut items = vec![text_button(&m.words.back, &m.back)];
    if let Some(cancel) = &m.cancel {
        items.push(text_button(&m.words.cancel, cancel));
    }
    draw_items(&items, &[], !ui.compact, ui, at)
}

/// The busy line as it is drawn: the moving bar holds still under Reduce motion, whatever phase
/// the model carries.
fn busy_line<A: Pressed>(busy: &BusyModel<'_>, ui: &mut Ui<'_, A>, at: (f32, f32, f32)) -> f32 {
    if ui.reduce_motion() {
        let still = BusyModel {
            line: busy.line.clone(),
            done: busy.done,
            phase: None,
        };
        return status::draw_busy(&still, ui, at);
    }
    status::draw_busy(busy, ui, at)
}

/// The foot of a step: the refusal's line, the busy line while the flow works, then Back and
/// Cancel. Returns the height.
fn foot<A: Pressed>(m: &FlowModel<'_, A>, ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    if let Some(refused) = &m.refused {
        cy += ui.showing(refused, |ui| {
            ui.wrap_as((x, cy, w), BODY_S, Role::Error, refused.as_str())
        }) + GAP;
    }
    if let Some(busy) = &m.busy {
        cy += busy_line(busy, ui, (x, cy, w)) + GAP;
    }
    cy += nav_row(m, ui, (x, cy, w));
    cy - y
}

/// The step: its heading and title, the card (all its blocks, or those before `pinned` ones the
/// small panel keeps at its foot), what is under it where no pane holds that, and the foot.
/// Returns the height.
pub(crate) fn column<A: Pressed>(
    m: &FlowModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    (pinned, under): (bool, bool),
) -> f32 {
    let mut cy = y;
    cy += heading(m, ui, (x, cy, w));
    let blocks = match (pinned, m.card.0.split_last()) {
        (true, Some((_, rest))) => rest,
        _ => &m.card.0[..],
    };
    cy += content::draw_blocks(blocks, ui, (x, cy, w)) + GAP;
    if let (true, Some(under)) = (under, &m.under) {
        cy += content::draw(under, ui, (x, cy, w)) + GAP;
    }
    cy += foot(m, ui, (x, cy, w));
    cy - y
}

/// One row of Your choices: the step's title over its answer, a press over the row.
fn choice_row<A: Pressed>(
    row: &ChoiceRow<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    ui.offered(&row.offered, |ui, press, dimmed| {
        if let Some(action) = press {
            ui.press(
                Shape::new(x - GAP, y, w + 2.0 * GAP, CHOICES_ROW, 0.0),
                Role::OnSurface.paint(),
                action,
            );
        }
        let lh = ui.line_as(BODY_S);
        let top = y + (CHOICES_ROW - 2.0 * lh) / 2.0;
        let (label, value) = if dimmed {
            (Role::OnSurfaceVariant, Role::OnSurfaceVariant)
        } else {
            (Role::OnSurfaceVariant, Role::OnSurface)
        };
        ui.showing(&row.title, |ui| {
            let t = ui.fit_as(BODY_S, row.title.as_str(), w);
            ui.text_as(x, top, BODY_S, label, &t);
        });
        ui.showing(&row.answer, |ui| {
            let a = ui.fit_as(BODY_S, row.answer.as_str(), w);
            ui.text_as(x, top + lh, BODY_S, value, &a);
        });
    });
    CHOICES_ROW
}

/// Your choices: its heading and a row for each step, then, where `with_under`, what the flow
/// keeps under them.
/// Returns the height.
fn choices<A: Pressed>(
    m: &FlowModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    with_under: bool,
) -> f32 {
    let mut cy = y;
    if !m.choices.is_empty() {
        ui.text_as(
            x,
            cy,
            LABEL_L,
            Role::OnSurfaceVariant,
            m.words.your_choices.as_str(),
        );
        cy += ui.line_as(LABEL_L) + GRID;
        for row in &m.choices {
            cy += choice_row(row, ui, (x, cy, w));
        }
    }
    if let (true, Some(under)) = (with_under, &m.under) {
        cy += GAP + content::draw(under, ui, (x, cy + GAP, w));
    }
    cy - y
}

/// The cancel dialog over the page: only its presses are taken.
fn asking<A: Pressed>(m: &FlowModel<'_, A>, ui: &mut Ui<'_, A>, (cw, h): (f32, f32)) {
    let Some(dialog) = &m.asking else { return };
    let id = Anchor {
        scope: "flow asking",
        n: m.step.n,
    };
    ui.clear_hits();
    ui.backdrop(id);
    ui.add_sheet(id);
    ui.layer += 1;
    sheet::dialog(dialog, ui, (0.0, cw, h));
}

/// Draws the step-flow template in the room the screen template left it: on the small panel the
/// page of the step (`compact`), else the card with Your choices beside it where the room holds
/// both, under it where it does not; over them the asking dialog.
pub fn draw<A: Pressed>(m: &FlowModel<'_, A>, ui: &mut Ui<'_, A>) {
    let room = page_room(ui);
    if ui.compact {
        compact::page(m, ui, room);
    } else {
        wide(m, ui, room);
    }
    asking(m, ui, room);
}

/// The wide layout: the card at its place and Your choices in a pane beside it.
fn wide<A: Pressed>(m: &FlowModel<'_, A>, ui: &mut Ui<'_, A>, (cw, h): (f32, f32)) {
    let full = m.width == Width::Full;
    let wants_pane = !m.choices.is_empty() || m.under.is_some();
    let pair = if full || !wants_pane {
        None
    } else {
        panes::pair(cw)
    };
    let (x, w) = match (&pair, m.width) {
        (Some(p), _) => (p.card_x, p.card_w),
        (None, Width::Column) => {
            let w = (cw - 2.0 * MARGIN_WIDE).min(COLUMN_MAX);
            ((cw - w) / 2.0, w)
        }
        (None, Width::Wide) => (MARGIN_WIDE, (cw - 2.0 * MARGIN_WIDE).min(COLUMN_MAX * 2.0)),
        (None, Width::Full) => (MARGIN_WIDE, cw - 2.0 * MARGIN_WIDE),
    };
    let under = pair.is_none();
    let top = PAGE_TOP_BAR;
    let raised = pair.is_some();
    // The raised card stands a padding round its content.
    let (cy, inset) = if raised { (top + PAD, PAD) } else { (top, 0.0) };
    let view = (x - inset, top, w + 2.0 * inset, h - top);
    if raised {
        let inner = ui.measured(|ui| column(m, ui, (x, 0.0, w), (false, under)));
        let bh = (inner + 2.0 * PAD).min(view.3 - GAP);
        ui.fill(view.0, top, view.2, bh, SHAPE_L, Role::SurfaceContainerLow);
    }
    ui.anchor(m.step);
    let clip = ui.rect(view.0, view.1, view.2, view.3);
    let open = ui.region_open(Region::new(card_scroll(m.step), clip));
    let mut used = column(m, ui, (x, cy, w), (false, under));
    // The room holds no pair: Your choices stand under the card.
    if pair.is_none() && !m.choices.is_empty() {
        used += GAP + choices(m, ui, (x, cy + used + GAP, w), false);
    }
    ui.region_close(open, cy + used + FLOW_END_PAD - (view.1 + view.3));
    if let Some(p) = pair {
        let (px, pw) = (p.pane_x + CHOICES_INSET, p.pane_w - 2.0 * CHOICES_INSET);
        let clip = ui.rect(p.pane_x, top, p.pane_w, h - top);
        let id = Anchor {
            scope: "flow choices",
            n: m.step.n,
        };
        let open = ui.region_open(Region::new(id, clip));
        let used = choices(m, ui, (px, cy, pw), true);
        ui.region_close(open, cy + used + CHOICES_INSET - h);
        ui.fill(
            p.pane_x + p.pane_w + p.gap / 2.0,
            top,
            1.0,
            h - top - GAP,
            0.0,
            Role::OutlineVariant,
        );
    }
}

mod compact;

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "V12: the tests stand in for the presentation kernel's fill"
)]
mod tests {
    use super::*;
    use faraday_atoms::ui::testing::{P, at_rest, canvas, frame};
    use faraday_molecules::buttons::ButtonsModel;
    use faraday_organisms::content::Block;
    use faraday_organisms::sheet::{Head, SheetModel};

    const STEP: Anchor = Anchor {
        scope: "test step",
        n: 3,
    };

    fn button(label: &'static str, style: Style, press: u8) -> ButtonModel<'static, P> {
        ButtonModel {
            label: Shown::plain(label),
            style,
            offered: Offered::Yes(P(press)),
            chosen: false,
            icon: None,
        }
    }

    /// A flow of a card of `lines` headings ending in its forward button (press 1), one row of Your choices
    /// (press 2), Back (3) and Cancel (4).
    fn flow(lines: usize) -> FlowModel<'static, P> {
        let mut blocks: Vec<Block<'static, P>> = (0..lines)
            .map(|_| Block::Heading(Shown::plain("A line of what the step says")))
            .collect();
        blocks.push(Block::Buttons(vec![button("Next", Style::Filled, 1)]));
        FlowModel::new(
            Shown::plain("Step 2 of 4"),
            Shown::plain("Choose"),
            Content(blocks),
            Offered::Yes(P(3)),
            Width::Column,
            STEP,
            FlowWords {
                your_choices: Shown::plain("Your choices"),
                back: Shown::plain("Back"),
                cancel: Shown::plain("Cancel"),
            },
        )
        .with_choices(vec![ChoiceRow {
            title: Shown::plain("Kind"),
            answer: Shown::plain("Single"),
            offered: Offered::Yes(P(2)),
        }])
        .with_cancel(Offered::Yes(P(4)))
    }

    /// Invariant: On the small panel the card's last row of buttons stands in the pin bar at the panel's foot however long the card is, and the card scrolls above it.
    /// Regression: The forward action drawn at the card's end, scrolled out of reach on a long step.
    /// Owner: kit
    #[test]
    fn the_small_panels_forward_action_stands_at_the_foot() {
        let mut c = canvas(160);
        let m = flow(40);
        let out = frame(&mut c, &at_rest(None), |ui| {
            ui.compact = true;
            draw(&m, ui);
        });
        let next = out.hits.iter().find(|h| h.press == P(1)).expect("Next");
        assert!(next.rect.y > 480 - 64, "Next at {}", next.rect.y);
    }

    /// Invariant: With the cancel dialog open only the dialog's presses are taken; the card, Your choices, Back and Cancel under it take none.
    /// Regression: A press goes through the dialog to the step under it.
    /// Owner: kit
    #[test]
    fn a_press_under_the_asking_dialog_is_not_taken() {
        let mut c = canvas(160);
        let m = flow(40).with_asking(Box::new(DialogModel {
            sheet: SheetModel {
                head: Some(Head {
                    icon: faraday_atoms::Glyph::Stick,
                    tone: faraday_atoms::ui::testing::page(),
                    title: Shown::plain("Cancel this?"),
                }),
                body: Content(vec![Block::Heading(Shown::plain(
                    "What is entered is lost",
                ))]),
                foot: ButtonsModel::row(vec![button("Keep going", Style::Text, 9)]),
                beside: None,
                scroll: Anchor {
                    scope: "test dialog",
                    n: 1,
                },
            },
            want: 360.0,
        }));
        let out = frame(&mut c, &at_rest(None), |ui| {
            ui.compact = true;
            draw(&m, ui);
        });
        let presses: Vec<P> = out.hits.iter().map(|h| h.press).collect();
        assert!(presses.contains(&P(9)));
        for under in [1, 2, 3, 4] {
            assert!(
                !presses.contains(&P(under)),
                "P({under}) taken under the dialog"
            );
        }
    }

    /// Invariant: While Reduce motion is on the busy bar holds still, whatever phase the model carries; without it the bar moves with the phase.
    /// Regression: The busy bar keeps crossing its track for a person who asked for no motion.
    /// Owner: kit
    #[test]
    fn the_busy_bar_holds_still_under_reduce_motion() {
        let drawn = |phase: f32, reduce: bool| {
            let mut c = canvas(160);
            let m = flow(2).with_busy(BusyModel {
                line: Shown::plain("Working"),
                done: None,
                phase: Some(phase),
            });
            frame(&mut c, &at_rest(None), |ui| {
                ui.compact = true;
                ui.set_reduce_motion(reduce);
                draw(&m, ui);
            });
            c.pixels().rgba.to_vec()
        };
        assert_eq!(drawn(0.2, true), drawn(0.8, true));
        assert_ne!(drawn(0.2, false), drawn(0.8, false));
    }

    /// Invariant: On the wide layout where the room holds no pair Your choices stand under the card, each row taking its press, and the card's foot keeps Back and Cancel.
    /// Regression: Your choices dropped from a window too narrow for the pane, so no step could be reopened.
    /// Owner: kit
    #[test]
    fn a_room_too_narrow_for_the_pair_puts_your_choices_under_the_card() {
        let mut c = canvas(160);
        let m = flow(2);
        let out = frame(&mut c, &at_rest(None), |ui| {
            ui.compact = false;
            draw(&m, ui);
        });
        let at = |n: u8| {
            out.hits
                .iter()
                .find(|h| h.press == P(n))
                .expect("a control")
                .rect
                .y
        };
        assert!(at(2) > at(3));
        assert!(out.hits.iter().any(|h| h.press == P(4)));
    }
}
