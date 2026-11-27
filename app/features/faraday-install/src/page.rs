use faraday_organisms::Ui;

use crate::{Install, Model, Press};

impl faraday_present::Page for Install {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
