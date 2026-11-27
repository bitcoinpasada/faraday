use faraday_organisms::Ui;

use crate::{Calculators, Model, Press};

impl faraday_present::Page for Calculators {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
