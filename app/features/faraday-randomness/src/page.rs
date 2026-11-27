use faraday_organisms::Ui;

use crate::{Model, Press, Randomness};

impl faraday_present::Page for Randomness {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
