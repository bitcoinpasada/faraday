use faraday_organisms::Ui;

use crate::{Model, Press, StickVisit};

impl faraday_present::Page for StickVisit {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
