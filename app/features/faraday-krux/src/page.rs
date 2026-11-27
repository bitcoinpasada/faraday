use faraday_organisms::Ui;

use crate::{Model, Press, Krux};

impl faraday_present::Page for Krux {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
