use faraday_kernel::{Line, Refused, Show, View, Worded};
use faraday_port::values::Anchor;
use faraday_present::Present;

use crate::{Model, Refusal, SheetModel, State, Vaults};

impl Worded for Refusal {
    fn words(&self, _show: &Show<'_>) -> Line {
        todo!()
    }
}

impl Refused for Refusal {}

impl Present for Vaults {
    type Model<'s> = Model<'s>;

    fn present<'s>(_state: &'s State, _view: &View<'s>, _show: &Show<'s>) -> Model<'s> {
        todo!()
    }
}

/// The vault area's sheets on show, each with its anchor, the lowest first (06 §4.2).
pub fn sheets<'s>(_state: &'s State, _view: &View<'s>, _show: &Show<'s>) -> Vec<(Anchor, SheetModel<'s>)> {
    todo!()
}
