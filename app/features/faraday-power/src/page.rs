use faraday_organisms::Ui;

use crate::{Model, Power, Press};

impl faraday_present::Page for Power {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
