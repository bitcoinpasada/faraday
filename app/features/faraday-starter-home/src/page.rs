use faraday_organisms::Ui;

use crate::{Model, Press, StarterHome};

impl faraday_present::Page for StarterHome {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
