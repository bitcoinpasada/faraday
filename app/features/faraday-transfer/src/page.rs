use faraday_organisms::Ui;

use crate::{Model, Press, Transfer};

impl faraday_present::Page for Transfer {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
