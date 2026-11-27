use faraday_organisms::Ui;

use crate::{Model, Press, Decode};

impl faraday_present::Page for Decode {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
