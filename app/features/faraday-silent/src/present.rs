use faraday_kernel::{Line, Refused, Show, View, Worded};
use faraday_present::Present;

use crate::{Model, Refusal, State, Silent};

impl Worded for Refusal {
    fn words(&self, _show: &Show<'_>) -> Line {
        todo!()
    }
}

impl Refused for Refusal {}

impl Present for Silent {
    type Model<'s> = Model<'s>;

    fn present<'s>(_state: &'s State, _view: &View<'s>, _show: &Show<'s>) -> Model<'s> {
        todo!()
    }
}
