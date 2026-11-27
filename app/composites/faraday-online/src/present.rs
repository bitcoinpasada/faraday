use faraday_kernel::{Line, Refused, Show, TopsView, View, Worded};
use faraday_organisms::Ui;
use faraday_present::Drawn;

use crate::{Body, OnlineApp, Over, Press, Refusal};

impl Worded for Refusal {
    fn words(&self, _show: &Show<'_>) -> Line {
        todo!()
    }
}

impl Refused for Refusal {}

/// The online app's frame: present, then draw (06 §4.4).
pub fn frame(
    _app: &OnlineApp,
    _tops: TopsView<'_>,
    _view: &View<'_>,
    _show: &Show<'_>,
    _ui: &mut Ui<'_, Press>,
) {
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
