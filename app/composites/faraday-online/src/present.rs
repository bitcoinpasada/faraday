use faraday_kernel::{Line, Refused, Show, View, Worded};
use faraday_organisms::{Pressed, Ui};
use faraday_port::values::Anchor;
use faraday_present::Drawn;
use faraday_templates::screen::FrameModel;

use crate::{Body, OnlineApp, Over, Page, Press, Refusal};

impl Worded for Refusal {
    fn words(&self, _show: &Show<'_>) -> Line {
        todo!()
    }
}

impl Refused for Refusal {}

/// The page the online app's `Nav` shows; an owner's is the product's to present (06 §4.2).
pub fn shown(_app: &OnlineApp) -> Page {
    todo!()
}

/// The frame's model, its presses lifted into the product's (06 §4.2).
pub fn frame_model<'s, R: Pressed>(
    _app: &'s OnlineApp,
    _view: &View<'s>,
    _show: &Show<'s>,
    _lift: fn(Press) -> R,
) -> FrameModel<'s, R> {
    todo!()
}

/// The shown page's model; `None` when the shown page is an owner's.
pub fn body<'s>(_app: &'s OnlineApp, _view: &View<'s>, _show: &Show<'s>) -> Option<Body<'s>> {
    todo!()
}

/// The sheets over the page, each with its anchor, the lowest first.
pub fn over<'s>(_app: &'s OnlineApp, _view: &View<'s>, _show: &Show<'s>) -> Vec<(Anchor, Over<'s>)> {
    todo!()
}

impl Drawn<Press> for Body<'_> {
    fn draw(&self, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}

impl Drawn<Press> for Over<'_> {
    fn draw(&self, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
