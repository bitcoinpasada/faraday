//! What every flow shares. A flow is drawn by [`container`]
//! (`docs/FLOW-CONTAINER.md`): one card at one place, with Your choices
//! ([`choices`]) at its right; it declares what only it knows through
//! [`Contained`]. This file holds that trait, the room a card is drawn in,
//! the scroll a page keeps and the small helpers flows draw with
//! ([`about`], [`guide_text`], [`small_touch`], [`past_end`]).

pub(crate) mod act;
mod answers;
mod choices;
mod container;
mod take_away;

pub(crate) use choices::{Row, Title};
pub(crate) use container::{Page, contained, pair_left};

use faraday_draw::widgets::Icon;

use crate::Action;
use crate::ui::Ui;
use crate::ui::pal::*;
use faraday_draw::tokens;
use faraday_draw::tokens::{BODY_M, BODY_S, LABEL_M};

/// How far in from a step's left edge its body is drawn, under its title.
pub(crate) const BODY_INDENT: f32 = 58.0;
/// How far in from a step's right edge its body stops.
pub(crate) const BODY_RIGHT: f32 = 18.0;

/// What a flow on the container declares each frame, by lookups only
/// (`docs/FLOW-CONTAINER.md` §1). Its steps, the current one, where Back
/// stops and whether a step runs are [`crate::trail::Steps`]; the title of
/// a step is [`crate::trail::StepFlow::name`]. The card's body and what
/// stands under the rows are closures the screen gives the drawing.
pub(crate) trait Contained: crate::trail::StepFlow {
    /// The rows of Your choices, in order.
    fn rows(&self, app: &crate::Faraday) -> Vec<Row>;
    /// Whether the number of steps is known before the first and never
    /// changes ("Step N of M"), or grows as answers are given ("Step N").
    fn count_fixed(&self, app: &crate::Faraday) -> bool;
    /// Turns off every "shown" switch the flow's steps hold (an eye on a
    /// passphrase, a typed secret shown): the container calls it when a
    /// step is left by any way out, so a step reopens with its secrets
    /// hidden (F104). What was typed stays.
    fn hide_shown(&self, app: &mut crate::Faraday);
    /// After step `changed` was answered again: whether the answer of step
    /// `later` still holds. The container keeps the answers that do and
    /// clears the rest; false clears every later answer.
    #[expect(dead_code, reason = "filled by the answers' brief, which reads it")]
    fn still_fits(&self, _app: &crate::Faraday, _changed: u8, _later: u8) -> bool {
        false
    }
    /// Whether the current step takes the whole room beside the
    /// navigation, with no pane (`docs/FLOW-CONTAINER.md` §5).
    fn full_width(&self, _app: &crate::Faraday) -> bool {
        false
    }
    /// The title of step `k` on the card's heading: the step's name unless
    /// the flow builds it (a title with the item's own name in it).
    fn title(&self, app: &crate::Faraday, k: u8) -> Title {
        Title::of(self.name(app, k))
    }
    /// How many steps at the head of `Steps::order` come before step 1: the
    /// heading reads the title alone, and "Step N of M" counts from the
    /// step after them.
    fn heads(&self, _app: &crate::Faraday) -> usize {
        0
    }
    /// Whether `Steps::order[0]` is a checklist card and the rest are its
    /// items: Back from an item opens the checklist, from the checklist it
    /// leaves the flow, and a row takes a press whenever its item is not
    /// current and the step is not running. A flow keeps an item it cannot
    /// open yet out of the press by giving its row an `opens` that is not
    /// in `Steps::order`.
    fn checklist(&self, _app: &crate::Faraday) -> bool {
        false
    }
    /// Whether the flow's card holds wide content (`docs/LAYOUT.md` §14):
    /// it stands at the navigation's edge plus `MARGIN_WIDE` on every step,
    /// not centred, and its body reads [`Ui::flow_work`]. A flow with a
    /// step that is [`Contained::full_width`] answers true on every step.
    fn wide_content(&self, _app: &crate::Faraday) -> bool {
        false
    }
    /// The wallet or key the flow is on, when the chip at the card's head
    /// names it: where the pair stands the pick is Your choices' first row
    /// and the chip goes. The second is what a press on the row does (the
    /// chip's own press). A chip that says where the flow goes next is not
    /// one, nor is a step's own name.
    fn pick_row(&self, _app: &crate::Faraday) -> Option<(Row, Option<Action>)> {
        None
    }
    /// The pick of step `k`, when that step is a pick: where the pair
    /// stands its rows are drawn at the left in place of Your choices, and
    /// the card keeps Next. On every other size the step's card draws the
    /// rows itself. Labels and fingerprints only (`docs/FLOW-CONTAINER.md`
    /// §4).
    fn pick(&self, _app: &crate::Faraday, _k: u8) -> Option<Pick> {
        None
    }
    /// Whether the pane lists nothing of its own (no answers, no lead
    /// row): it then draws no heading, only what stands under it.
    fn lists_nothing(&self, _app: &crate::Faraday) -> bool {
        false
    }

    /// What the flow's last step gives to take away, when step `k` is that
    /// step and the flow has made something: where the pair stands the
    /// pane draws it in place of Your choices (`flow/take_away.rs`, `docs/
    /// FLOW-CONTAINER.md` §7), and the step's card keeps the outcome. Public
    /// things only: names and actions, never a key, a seed or a
    /// passphrase.
    fn take_away(&self, _app: &crate::Faraday, _k: u8) -> Option<TakeAway> {
        None
    }
}

/// What a flow's end gives to take away: the files it made, by name in
/// the outbox, and the codes to scan.
pub(crate) struct TakeAway {
    /// The files made, in order.
    pub(crate) files: Vec<TakeFile>,
    /// The codes to scan.
    pub(crate) codes: Vec<TakeCode>,
}

/// One file in the take-away: the name it went out under and the SHA-256
/// of its bytes taken when it was made, so that only a file of that name
/// and those bytes counts as written; and what offers it again when it is
/// no longer in the outbox.
pub(crate) struct TakeFile {
    /// The name it went out under.
    pub(crate) name: Title,
    /// The SHA-256 of its bytes.
    pub(crate) sha: [u8; 32],
    /// The name a new file is made under, drawn while this one is not there.
    pub(crate) again: Title,
    /// What the row says that makes it (the first time).
    pub(crate) make_label: &'static str,
    /// The press that makes it.
    pub(crate) make: crate::Action,
}

/// One code in the take-away: what it is, and the press that shows it.
pub(crate) struct TakeCode {
    pub(crate) name: &'static str,
    pub(crate) show: crate::Action,
}

/// A step's pick as the pane holds it: the question as its heading, then
/// the groups of rows, each under its own heading.
pub(crate) struct Pick {
    /// The step's question: the pane's heading.
    pub(crate) question: &'static str,
    /// The groups, in order.
    pub(crate) groups: Vec<PickGroup>,
}

/// One group of a pick's rows, under a heading.
pub(crate) struct PickGroup {
    pub(crate) head: &'static str,
    pub(crate) rows: Vec<crate::pick::PickRow>,
}

/// Where a flow on the container drew its card: the room it may draw and
/// clip in, and the column the card's controls stand in, which is what a
/// panel at the page's foot lines up with.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Room {
    /// The left edge of the room the page draws and clips in.
    pub(crate) area_x: f32,
    /// That room's width.
    pub(crate) area_w: f32,
    /// The card column's left edge.
    pub(crate) x: f32,
    /// The card column's width.
    pub(crate) w: f32,
}

/// One card of a flow.
pub struct Card {
    /// The card's title.
    pub title: String,
    /// The line shown while it is closed.
    pub summary: String,
    /// Set the summary in the mono face: it is data.
    pub mono: bool,
    /// Closed as done.
    pub done: bool,
    /// Open now.
    pub open: bool,
    /// The card's value has a default, so a flow may open closed on it
    /// at entry; closed, it draws a **Change** text button at its right
    /// in place of the chevron.
    pub default: bool,
    /// What pressing its header does.
    pub toggle: Action,
    /// A line of the card's own, shown while it is open.
    pub lead: Option<String>,
}

/// Where the column is scrolled, and whether to bring the open card into
/// view on this frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct Scroll {
    /// Design units scrolled.
    pub y: f32,
    /// The open card changed.
    pub follow: bool,
}

/// What is drawn under a flow's last card: at (x, y) in a width,
/// returning its height.
pub type Foot<'a> = &'a mut dyn FnMut(&mut Ui, f32, f32, f32) -> f32;

/// Whether a page's offset `y` is past its end `max`, both in design
/// units. The frame rounds an offset to whole pixels at or under the end
/// (`Faraday::whole_offset`), which may be a hair over `max` as a float:
/// that is not past it, or the page would clamp and draw again for ever.
pub(crate) fn past_end(y: f32, max: f32) -> bool {
    y > max + 1e-3
}

/// The touch target's side on the small panel's own, shorter one, 0 where
/// the target is the usual size: what a row grows to there. A lookup.
pub(crate) fn small_touch(ui: &Ui) -> f32 {
    if ui.touch < tokens::TOUCH {
        ui.touch
    } else {
        0.0
    }
}

/// A walk-through behind a tap, after a small panel's controls: a row
/// that opens it and closes it, and the text while open. `k` is the
/// step's place, or [`crate::ABOUT_PAGE`]. Returns its height.
pub(crate) fn about(ui: &mut Ui, x: f32, y: f32, w: f32, label: &str, k: u8, text: &str) -> f32 {
    let open = ui.about_open == Some(k);
    let action = Action::About(k);
    let mut cy = y + 8.0;
    ui.rule(x, cy, w, Role::OutlineVariant);
    cy += 6.0;
    // On the small panel the row keeps a full touch target.
    let rh = 36.0_f32.max(small_touch(ui));
    ui.icon(
        x,
        cy + (rh - 20.0) / 2.0,
        20.0,
        Icon::Info,
        11.0,
        Role::OnSurfaceVariant,
    );
    ui.text_mid_as((x + 26.0, cy, rh), LABEL_M, Role::OnSurfaceVariant, label);
    ui.icon(
        x + w - 22.0,
        cy + (rh - 20.0) / 2.0,
        20.0,
        if open {
            Icon::ChevronUp
        } else {
            Icon::ChevronRight
        },
        9.0,
        Role::OnSurfaceVariant,
    );
    ui.press_box(x, cy, w, rh, action);
    cy += rh + 6.0;
    if open {
        let gh = ui.wrap_as(
            (x + 12.0, cy, w - 12.0),
            BODY_S,
            Role::OnSurfaceVariant,
            text,
        );
        ui.fill(x, cy, 3.0, gh, 1.5, Role::Primary.paint());
        cy += gh;
        cy += 8.0;
    }
    cy - y
}

/// A card's text from (x, y), `w` wide: its words in body text beside a
/// bar in the accent, as a flow's open card and Settings' cards show it.
/// Returns its height.
pub(crate) fn guide_text(ui: &mut Ui, (x, y, w): (f32, f32, f32), text: &str) -> f32 {
    use tokens::{GUIDE_BAR, GUIDE_INDENT};
    let tx = x + GUIDE_INDENT;
    let gh = ui.wrap_as((tx, y, w - GUIDE_INDENT), BODY_M, Role::OnSurface, text);
    let bar = Role::Primary.paint();
    ui.fill(x, y, GUIDE_BAR, gh, GUIDE_BAR / 2.0, bar);
    gh
}
