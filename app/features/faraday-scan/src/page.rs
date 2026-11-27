use faraday_organisms::Ui;

use crate::{Model, Press, Scan};

impl faraday_present::Page for Scan {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
