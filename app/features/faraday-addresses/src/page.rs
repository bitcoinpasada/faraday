use faraday_organisms::Ui;

use crate::{Addresses, Model, Press};

impl faraday_present::Page for Addresses {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
