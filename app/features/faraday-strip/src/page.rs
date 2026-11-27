use faraday_organisms::Ui;

use crate::{Model, Press, Strip};

impl faraday_present::Page for Strip {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
