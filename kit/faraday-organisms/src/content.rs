//! The content organism: how a page draws controls (04 §12.3).

use faraday_atoms::button::ButtonModel;
use faraday_atoms::field::FieldModel;
use faraday_atoms::hold::HoldModel;
use faraday_atoms::{Pressed, Shown, Ui};
use faraday_molecules::flow_text::FlowText;
use faraday_molecules::options::ChoiceModel;
use faraday_molecules::pads::PadModel;
use faraday_molecules::pass::PassModel;
use faraday_molecules::qr::QrModel;
use faraday_molecules::row::RowModel;
use faraday_molecules::status::BusyModel;
use faraday_molecules::words::WordGridModel;

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

/// Draws the content.
pub fn draw<A: Pressed>(_m: &Content<'_, A>, _ui: &mut Ui<'_, A>) {
    todo!()
}
