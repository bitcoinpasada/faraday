use faraday_organisms::Ui;

use crate::{Home, Model, Press};

impl faraday_present::Page for Home {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
