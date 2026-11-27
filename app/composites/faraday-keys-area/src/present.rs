use faraday_kernel::{Line, Refused, Show, View, Worded};
use faraday_port::values::Anchor;

use crate::{Body, Over, Page, Refusal, State};

impl Worded for Refusal {
    fn words(&self, _show: &Show<'_>) -> Line {
        todo!()
    }
}

impl Refused for Refusal {}

/// What `present` works out for the page the frame's navigation shows.
// Spec: 09 §4.6 gives the composite `Body` and no function that presents it.
pub fn body<'s>(_state: &'s State, _page: Page, _view: &View<'s>, _show: &Show<'s>) -> Body<'s> {
    todo!()
}

/// The sheets over the page, each with its anchor, the lowest first.
// Spec: 09 §4.6 gives the composite `Over` and no function that presents it.
pub fn over<'s>(_state: &'s State, _view: &View<'s>, _show: &Show<'s>) -> Vec<(Anchor, Over<'s>)> {
    todo!()
}
