use faraday_organisms::Ui;

use crate::{Model, Press, StarterGuide};

impl faraday_present::Page for StarterGuide {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
