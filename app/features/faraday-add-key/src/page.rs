use faraday_organisms::Ui;

use crate::{Model, Press, AddKey};

impl faraday_present::Page for AddKey {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
