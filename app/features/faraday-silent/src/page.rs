use faraday_organisms::Ui;

use crate::{Model, Press, Silent};

impl faraday_present::Page for Silent {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
