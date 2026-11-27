use faraday_organisms::Ui;

use crate::{Model, Press, Settings};

impl faraday_present::Page for Settings {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
