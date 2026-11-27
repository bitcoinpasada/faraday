//! The content organism: how a page draws controls (04 §12.3).

use faraday_atoms::button::ButtonModel;
use faraday_atoms::field::FieldModel;
use faraday_atoms::hold::HoldModel;
use faraday_atoms::{Pressed, Shown, Ui};
use faraday_molecules::buttons::draw_items;
use faraday_molecules::flow_text::FlowText;
use faraday_molecules::options::ChoiceModel;
use faraday_molecules::pads::PadModel;
use faraday_molecules::pass::PassModel;
use faraday_molecules::qr::QrModel;
use faraday_molecules::row::RowModel;
use faraday_molecules::status::BusyModel;
use faraday_molecules::words::WordGridModel;
use faraday_molecules::{flow_text, row, status};
use faraday_tokens::dims::{BODY_M, BODY_S, GAP, GAP_SMALL};
use faraday_tokens::palette::Role;
use faraday_tokens::type_roles::sheet_head;

use crate::board::BoardModel;
use crate::chart::{ChartModel, WalletCardModel};
use crate::demo_only::DemoOnlyModel;
use crate::qr_grid::QrGridModel;
use crate::ways_in::WaysInModel;
use crate::worksheet::WorksheetModel;

/// A column of blocks, drawn in order in the width given.
pub struct Content<'s, A>(pub Vec<Block<'s, A>>);

pub enum Block<'s, A> {
    Heading(Shown<'s>),
    /// A headline: a dialog's head in Material's headline (headline small, title large on a
    /// small panel), for a feature that says "headline" without naming a type role.
    Headline(Shown<'s>),
    /// Bold and plain runs.
    Para(FlowText<'s>),
    Fact {
        label: Shown<'s>,
        value: Shown<'s>,
        mono: bool,
    },
    /// List items: icon, headline, line, tags, chevron, press.
    Rows(Vec<RowModel<'s, A>>),
    /// One or several of a list; segmented.
    Choose(ChoiceModel<'s, A>),
    /// Label, the typed line, its caret, an action in the field.
    Field(FieldModel<'s, A>),
    /// The passphrase field part's drawing, from its read view.
    Pass(PassModel<'s, A>),
    Words(WordGridModel<'s, A>),
    /// Dice, cards, coin.
    Pad(PadModel<A>),
    Code(QrModel<'s>),
    Address(Shown<'s>),
    /// Each with its `Style` and `Offered`.
    Buttons(Vec<ButtonModel<'s, A>>),
    Hold(HoldModel<'s, A>),
    Busy(BusyModel<'s>),
    Caution(Shown<'s>),
    /// A bespoke organism: worksheet, chart, board, ways-in.
    Part(Organism<'s, A>),
}

/// The bespoke organisms a `Block::Part` holds, one variant per model: every organism a step's
/// card draws, since a card is content alone (09 §3.3).
pub enum Organism<'s, A> {
    Worksheet(WorksheetModel<'s, A>),
    Chart(ChartModel<'s, A>),
    WalletCard(WalletCardModel<'s, A>),
    Board(BoardModel<'s, A>),
    WaysIn(WaysInModel<'s, A>),
    QrGrid(QrGridModel<'s, A>),
    DemoOnly(DemoOnlyModel<'s, A>),
}

/// A fact: its label over its value, each wrapped, a rule under. Returns the height it took,
/// without the gap that follows a block.
fn fact<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    (label, value, mono): (&Shown<'_>, &Shown<'_>, bool),
) -> f32 {
    let value_role = if mono { BODY_M.mono() } else { BODY_M };
    ui.showing(label, |ui| {
        ui.showing(value, |ui| {
            let vy =
                ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, label.as_str()) + GAP_SMALL;
            let vh = ui.wrap_as((x, y + vy, w), value_role, Role::OnSurface, value.as_str());
            ui.rule(x, y + vy + vh + GAP_SMALL, w, Role::OutlineVariant);
            vy + vh + GAP_SMALL
        })
    })
}

/// Draws one block from (x, y), `w` wide. Returns the height it took.
fn block<A: Pressed>(b: &Block<'_, A>, ui: &mut Ui<'_, A>, at: (f32, f32, f32)) -> f32 {
    let (x, y, w) = at;
    match b {
        Block::Heading(s) => ui.showing(s, |ui| {
            ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, s.as_str());
            ui.advance_as(BODY_S, 22.0)
        }),
        Block::Headline(s) => ui.showing(s, |ui| {
            let role = sheet_head(ui.compact);
            let h = ui.wrap_as((x, y, w), role, Role::OnSurface, s.as_str());
            h + GAP
        }),
        Block::Para(m) => flow_text::draw(m, ui, at),
        Block::Fact { label, value, mono } => fact(ui, at, (label, value, *mono)),
        Block::Rows(rows) => {
            let mut cy = y;
            for r in rows {
                cy += row::draw(r, ui, (x, cy, w));
            }
            cy - y
        }
        Block::Caution(s) => ui.showing(s, |ui| {
            ui.wrap_as((x, y, w), BODY_M, Role::Warning, s.as_str())
        }),
        Block::Buttons(items) => draw_items(items, &[], false, ui, at),
        Block::Busy(m) => status::draw_busy(m, ui, at),
        Block::Hold(m) => faraday_atoms::hold::draw(m, ui, at),
        Block::Field(m) => faraday_atoms::field::draw(m, ui, at),
        // The blocks below draw in the molecules and organisms that are not yet ported.
        Block::Choose(m) => {
            faraday_molecules::options::draw(m, ui);
            0.0
        }
        Block::Pass(m) => {
            faraday_molecules::pass::draw(m, ui);
            0.0
        }
        Block::Words(m) => {
            faraday_molecules::words::draw(m, ui);
            0.0
        }
        Block::Pad(m) => {
            faraday_molecules::pads::draw(m, ui);
            0.0
        }
        Block::Code(m) => {
            faraday_molecules::qr::draw(m, ui);
            0.0
        }
        Block::Address(_) => 0.0,
        Block::Part(o) => {
            part(o, ui);
            0.0
        }
    }
}

/// A bespoke organism: its own draw, which is not yet ported.
fn part<A: Pressed>(o: &Organism<'_, A>, ui: &mut Ui<'_, A>) {
    match o {
        Organism::Worksheet(m) => crate::worksheet::draw(m, ui),
        Organism::Chart(m) => crate::chart::draw(m, ui),
        Organism::WalletCard(_) => {}
        Organism::Board(m) => crate::board::draw(m, ui),
        Organism::WaysIn(m) => crate::ways_in::draw(m, ui),
        Organism::QrGrid(m) => crate::qr_grid::draw(m, ui),
        Organism::DemoOnly(m) => crate::demo_only::draw(m, ui),
    }
}

/// Draws the content from (x, y), `w` wide: its blocks in order, a gap between them (a list's
/// rows, which carry their own padding, apart). Returns the height it took.
pub fn draw<A: Pressed>(m: &Content<'_, A>, ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32)) -> f32 {
    let mut cy = y;
    for (i, b) in m.0.iter().enumerate() {
        if i > 0 {
            cy += GAP;
        }
        cy += block(b, ui, (x, cy, w));
    }
    cy - y
}

/// The height [`draw`] takes `w` wide, with nothing drawn.
pub fn height<A: Pressed>(m: &Content<'_, A>, ui: &mut Ui<'_, A>, w: f32) -> f32 {
    ui.measured(|ui| draw(m, ui, (0.0, 0.0, w)))
}
