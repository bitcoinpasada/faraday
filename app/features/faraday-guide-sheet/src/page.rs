use faraday_organisms::Ui;

use crate::{GuideSheet, Model, Press};

impl faraday_present::Page for GuideSheet {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
