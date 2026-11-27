use faraday_organisms::Ui;

use crate::{Model, Press, Messages};

impl faraday_present::Page for Messages {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
