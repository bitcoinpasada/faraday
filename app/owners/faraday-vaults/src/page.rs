use faraday_organisms::Ui;

use crate::{Model, Press, Vaults};

impl faraday_present::Page for Vaults {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
