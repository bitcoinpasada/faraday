use faraday_organisms::Ui;

use crate::{Model, Press, OtsCheck};

impl faraday_present::Page for OtsCheck {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
