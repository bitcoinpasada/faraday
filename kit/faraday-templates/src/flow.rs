//! The step-flow template (04 §5, §12.4): the heading and title, the card, Your choices, what
//! stands under it, the foot (the refusal, the busy line, Back and Cancel) and the asking
//! dialog. The small panel's page per step and its pin bar are `compact`.
//!
//! Left: Your choices as the top bar's button and sheet on the small panel, and the fade through
//! itself (the runtime's: the template only names the place, `Ui::anchor`). Back and Cancel at
//! the card's foot and the asking dialog over the page room are the screen frame's to take over.

use faraday_atoms::button::{ButtonModel, Style};
use faraday_atoms::hits::Shape;
use faraday_atoms::region::Region;
use faraday_atoms::{Anchor, Offered, Pressed, Shown, Ui};
use faraday_molecules::buttons::{ButtonsModel, draw_items};
use faraday_molecules::row::{self, RowModel};
use faraday_molecules::status::{self, BusyModel};
use faraday_organisms::content::{self, Content};
use faraday_organisms::sheet::{self, DialogModel};
use faraday_tokens::dims::{
    BODY_S, CHOICES_INSET, CHOICES_ROW, COLUMN_MAX, FLOW_END_PAD, GAP, GRID, LABEL_L, MARGIN_WIDE,
    PAD, PAGE_TOP_BAR, SHAPE_L, SHAPE_S, TAG_H, TITLE_M,
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
    /// The row stands for the step now open: drawn in the secondary container.
    pub current: bool,
    /// The answer is data: set in the mono face.
    pub mono: bool,
}

/// The chip at the head of the card: where the flow goes next.
pub struct Chip<'s, A> {
    pub label: Shown<'s>,
    /// What a press on it does; none for a chip that only says.
    pub offered: Option<Offered<A>>,
}

/// A step's pick as the pane holds it in place of Your choices: the question, then groups of
/// rows each under a heading.
pub struct Pick<'s, A> {
    pub question: Shown<'s>,
    pub groups: Vec<(Shown<'s>, Vec<RowModel<'s, A>>)>,
}

/// The last step's take-away as the pane holds it in place of Your choices: a heading over
/// content (rows, and the button that writes them).
pub struct Take<'s, A> {
    pub head: Shown<'s>,
    pub body: Content<'s, A>,
}

/// What stands in the pane in place of Your choices.
enum Instead<'s, A> {
    Pick(Pick<'s, A>),
    Take(Take<'s, A>),
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
    lead: Option<ChoiceRow<'s, A>>,
    chip: Option<Chip<'s, A>>,
    note: Option<Shown<'s>>,
    forward: Option<ButtonsModel<'s, A>>,
    instead: Option<Instead<'s, A>>,
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
            lead: None,
            chip: None,
            note: None,
            forward: None,
            instead: None,
        }
    }

    /// Your choices: one row for each step answered or still to answer.
    pub fn with_choices(mut self, choices: Vec<ChoiceRow<'s, A>>) -> Self {
        self.choices = choices;
        self
    }

    /// The wallet or key the flow is on: the first row of Your choices.
    pub fn with_lead(mut self, lead: ChoiceRow<'s, A>) -> Self {
        self.lead = Some(lead);
        self
    }

    /// The chip at the head of the card.
    pub fn with_chip(mut self, chip: Chip<'s, A>) -> Self {
        self.chip = Some(chip);
        self
    }

    /// The line under the chip, with a primary bar at its left.
    pub fn with_note(mut self, note: Shown<'s>) -> Self {
        self.note = Some(note);
        self
    }

    /// The forward action: at the card's end on the wide layout, pinned at the small panel's
    /// foot in the pin bar.
    pub fn with_forward(mut self, forward: ButtonsModel<'s, A>) -> Self {
        self.forward = Some(forward);
        self
    }

    /// The open step's pick, which takes the pane's place where the pair stands.
    pub fn with_pick(mut self, pick: Pick<'s, A>) -> Self {
        self.instead = Some(Instead::Pick(pick));
        self
    }

    /// The last step's take-away, which takes the pane's place where the pair stands.
    pub fn with_take(mut self, take: Take<'s, A>) -> Self {
        self.instead = Some(Instead::Take(take));
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

/// The chip at the head of the card and the line under it. Returns the height.
fn chip_and_note<A: Pressed>(
    m: &FlowModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> f32 {
    let mut cy = y;
    if let Some(chip) = &m.chip {
        let press = match &chip.offered {
            Some(Offered::Yes(a) | Offered::Refused(a, _)) => Some(a.clone()),
            _ => None,
        };
        ui.showing(&chip.label, |ui| {
            let label = ui.fit_as(BODY_S, chip.label.as_str(), w - 34.0);
            ui.chip_press(
                x,
                cy,
                label.as_str(),
                (
                    Role::OnSecondaryContainer.paint(),
                    Role::SecondaryContainer.paint(),
                ),
                press,
            );
        });
        cy += if ui.compact { 36.0 } else { TAG_H + PAD };
    }
    if let Some(note) = &m.note {
        let (inset, gap) = if ui.compact {
            (12.0, 14.0)
        } else {
            (14.0, 20.0)
        };
        let nh = ui.showing(note, |ui| {
            ui.wrap_as(
                (x + inset, cy, w - inset),
                BODY_S,
                Role::OnSurfaceVariant,
                note.as_str(),
            )
        });
        ui.fill(x, cy, 3.0, nh, 1.5, Role::Primary);
        cy += nh + gap;
    }
    cy - y
}

/// The step: its chip and note, heading and title, the card, the forward action at the card's
/// end (unless `pinned` in the small panel's bar), what is under the card where no pane holds
/// that, and the foot. Returns the height.
pub(crate) fn column<A: Pressed>(
    m: &FlowModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    (pinned, under): (bool, bool),
) -> f32 {
    let mut cy = y;
    cy += chip_and_note(m, ui, (x, cy, w));
    cy += heading(m, ui, (x, cy, w));
    cy += content::draw(&m.card, ui, (x, cy, w)) + GAP;
    if let (false, Some(forward)) = (pinned, &m.forward) {
        cy += draw_items(&forward.items, &forward.rule, !ui.compact, ui, (x, cy, w)) + GAP;
    }
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
        if row.current {
            ui.fill(
                x - GAP,
                y,
                w + 2.0 * GAP,
                CHOICES_ROW,
                SHAPE_S,
                Role::SecondaryContainer,
            );
        }
        let (label, value) = match (row.current, dimmed) {
            (true, _) => (Role::OnSecondaryContainer, Role::OnSecondaryContainer),
            (false, true) => (Role::OnSurfaceVariant, Role::OnSurfaceVariant),
            (false, false) => (Role::OnSurfaceVariant, Role::OnSurface),
        };
        let answer_role = if row.mono { BODY_S.mono() } else { BODY_S };
        ui.showing(&row.title, |ui| {
            let t = ui.fit_as(BODY_S, row.title.as_str(), w);
            ui.text_as(x, top, BODY_S, label, &t);
        });
        ui.showing(&row.answer, |ui| {
            let a = ui.fit_as(answer_role, row.answer.as_str(), w);
            ui.text_as(x, top + lh, answer_role, value, &a);
        });
    });
    CHOICES_ROW
}

/// The pane: Your choices (the lead row, then a row for each step) or, where the flow has one,
/// the open step's pick or the take-away in their place (`instead`); then, where `with_under`,
/// what the flow keeps under them. Returns the height.
fn choices<A: Pressed>(
    m: &FlowModel<'_, A>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    (with_under, instead): (bool, bool),
) -> f32 {
    let mut cy = y;
    let label = |ui: &mut Ui<'_, A>, y: f32, s: &Shown<'_>| {
        ui.showing(s, |ui| {
            ui.text_as(x, y, LABEL_L, Role::OnSurfaceVariant, s.as_str());
        });
        ui.line_as(LABEL_L) + GRID
    };
    match (&m.instead, instead) {
        (Some(Instead::Pick(pick)), true) => {
            cy += label(ui, cy, &pick.question);
            for (head, rows) in &pick.groups {
                cy += label(ui, cy, head);
                for r in rows {
                    cy += row::draw(r, ui, (x, cy, w));
                }
            }
        }
        (Some(Instead::Take(take)), true) => {
            cy += label(ui, cy, &take.head);
            cy += content::draw(&take.body, ui, (x, cy, w));
        }
        _ if m.lead.is_some() || !m.choices.is_empty() => {
            cy += label(ui, cy, &m.words.your_choices);
            for row in m.lead.iter().chain(&m.choices) {
                cy += choice_row(row, ui, (x, cy, w));
            }
        }
        _ => {}
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
    let wants_pane =
        !m.choices.is_empty() || m.lead.is_some() || m.instead.is_some() || m.under.is_some();
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
        // The raised card is outlined at High contrast alone.
        if ui.theme.is_high_contrast() {
            ui.stroke(view.0, top, view.2, bh, SHAPE_L, Role::Outline);
        }
    }
    ui.anchor(m.step);
    let clip = ui.rect(view.0, view.1, view.2, view.3);
    let open = ui.region_open(Region::new(card_scroll(m.step), clip));
    let mut used = column(m, ui, (x, cy, w), (false, under));
    // The room holds no pair: Your choices stand under the card.
    if pair.is_none() && (m.lead.is_some() || !m.choices.is_empty()) {
        used += GAP + choices(m, ui, (x, cy + used + GAP, w), (false, false));
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
        let used = choices(m, ui, (px, cy, pw), (true, true));
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
    use faraday_atoms::ui::testing::{P, at_rest, canvas, canvas_sized, frame};
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

    /// A flow of a card of `lines` headings and a forward button (press 1), one row of Your choices
    /// (press 2), Back (3) and Cancel (4).
    fn flow(lines: usize) -> FlowModel<'static, P> {
        let blocks: Vec<Block<'static, P>> = (0..lines)
            .map(|_| Block::Heading(Shown::plain("A line of what the step says")))
            .collect();
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
            current: false,
            mono: false,
        }])
        .with_cancel(Offered::Yes(P(4)))
        .with_forward(ButtonsModel::row(vec![button("Next", Style::Filled, 1)]))
    }

    /// Invariant: On the small panel the forward action stands in the pin bar at the panel's foot however long the card is, and the card scrolls above it.
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

    type Seen = (Vec<u8>, Vec<(P, i32, i32, i32, i32)>);

    /// A wide frame 1000 by 700 (the pair fits): its pixels and each hit's press and box.
    fn wide_frame(m: &FlowModel<'static, P>, high: bool) -> Seen {
        let mut c = canvas_sized(160, (1000, 700));
        let out = frame(&mut c, &at_rest(None), |ui| {
            ui.compact = false;
            if high {
                ui.theme = faraday_atoms::Theme::all()
                    .find(|t| t.is_high_contrast())
                    .expect("a high contrast theme");
            }
            draw(m, ui);
        });
        let hits = out
            .hits
            .iter()
            .map(|h| (h.press, h.rect.x, h.rect.y, h.rect.w, h.rect.h))
            .collect();
        (c.pixels().rgba.to_vec(), hits)
    }

    fn pixel_at(rgba: &[u8], (x, y): (i32, i32)) -> [u8; 4] {
        let i = ((y * 1000 + x) * 4) as usize;
        [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
    }

    /// Invariant: Where the room holds the pair the card is raised: its edge is drawn at High contrast alone, so a pixel on its top edge equals the card's fill in the standard theme and differs from it at High contrast.
    /// Regression: The raised card outlined in every theme, or its outline lost at High contrast.
    /// Owner: kit
    #[test]
    fn the_raised_card_is_outlined_at_high_contrast_alone() {
        let m = flow(2);
        let place = panes::pair(1000.0).expect("the pair fits");
        let x = (place.card_x + 100.0) as i32;
        let edge = 16; // the card's top: the page's top margin
        let (std, _) = wide_frame(&m, false);
        let (hc, _) = wide_frame(&m, true);
        let on_edge = |px: &[u8]| {
            (edge - 1..=edge + 1)
                .map(|y| pixel_at(px, (x, y)))
                .collect::<Vec<_>>()
        };
        let inside = |px: &[u8]| pixel_at(px, (x, edge + 6));
        assert!(on_edge(&std).iter().skip(1).all(|p| *p == inside(&std)));
        assert!(on_edge(&hc).iter().any(|p| *p != inside(&hc)));
    }

    /// Invariant: The row of the open step is drawn in the secondary container, a row of another step is not.
    /// Regression: Your choices marks no row as the step now open.
    /// Owner: kit
    #[test]
    fn the_current_choice_row_is_tinted() {
        let tint = |current: bool| {
            let mut m = flow(2);
            m.choices[0].current = current;
            let (px, hits) = wide_frame(&m, false);
            let row = hits.iter().find(|h| h.0 == P(2)).expect("a row");
            pixel_at(&px, (row.1 + row.3 - 2, row.2 + 3))
        };
        assert_ne!(tint(true), tint(false));
    }

    /// Invariant: Where the pair stands a take-away takes the pane's place: its press is taken and Your choices' rows are not drawn; the lead row stands first in Your choices where there is no take-away.
    /// Regression: The pane draws Your choices over a take-away, or drops the lead row.
    /// Owner: kit
    #[test]
    fn a_take_away_or_the_lead_row_stands_in_the_pane() {
        let row = |press| ChoiceRow {
            title: Shown::plain("Wallet"),
            answer: Shown::plain("A wallet"),
            offered: Offered::Yes(P(press)),
            current: false,
            mono: false,
        };
        let (_, led) = wide_frame(&flow(2).with_lead(row(5)), false);
        let y = |hits: &[(P, i32, i32, i32, i32)], n: u8| {
            hits.iter().find(|h| h.0 == P(n)).map(|h| h.2)
        };
        assert!(y(&led, 5) < y(&led, 2));
        let take = Take {
            head: Shown::plain("Take away"),
            body: Content(vec![Block::Buttons(vec![button(
                "Write",
                Style::Filled,
                7,
            )])]),
        };
        let (_, taken) = wide_frame(&flow(2).with_lead(row(5)).with_take(take), false);
        assert!(y(&taken, 7).is_some());
        assert!(y(&taken, 2).is_none() && y(&taken, 5).is_none());
    }

    /// Invariant: The chip at the head of the card takes its press and stands above the title's card; the note draws without taking one.
    /// Regression: The chip left out of the card, or drawn under it.
    /// Owner: kit
    #[test]
    fn the_chip_stands_above_the_card() {
        let m = flow(2)
            .with_chip(Chip {
                label: Shown::plain("Next: Receive"),
                offered: Some(Offered::Yes(P(6))),
            })
            .with_note(Shown::plain("Part of a backup"));
        let (_, hits) = wide_frame(&m, false);
        let y = |n: u8| hits.iter().find(|h| h.0 == P(n)).expect("a control").2;
        assert!(y(6) < y(1));
    }
}
