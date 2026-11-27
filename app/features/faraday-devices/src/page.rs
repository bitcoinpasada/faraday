use faraday_organisms::Ui;

use crate::{Devices, Model, Press};

impl faraday_present::Page for Devices {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
