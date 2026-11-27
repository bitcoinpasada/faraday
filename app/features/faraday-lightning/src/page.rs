use faraday_organisms::Ui;

use crate::{Lightning, Model, Press};

impl faraday_present::Page for Lightning {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
