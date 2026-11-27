use faraday_organisms::Ui;

use crate::{Model, Press, VaultUnlock};

impl faraday_present::Page for VaultUnlock {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
