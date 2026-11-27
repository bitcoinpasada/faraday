use faraday_organisms::Ui;

use crate::{Model, Press, VaultSeal};

impl faraday_present::Page for VaultSeal {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
