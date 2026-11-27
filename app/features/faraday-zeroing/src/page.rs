use faraday_organisms::Ui;

use crate::{Model, Press, Zeroing};

impl faraday_present::Page for Zeroing {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
