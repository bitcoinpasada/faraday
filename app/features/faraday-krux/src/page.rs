use faraday_organisms::Ui;

use crate::{Krux, Model, Press};

impl faraday_present::Page for Krux {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
