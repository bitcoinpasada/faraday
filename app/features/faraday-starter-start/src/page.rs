use faraday_organisms::Ui;

use crate::{Model, Press, StarterStart};

impl faraday_present::Page for StarterStart {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
