use faraday_organisms::Ui;

use crate::{Model, Press, VaultList};

impl faraday_present::Page for VaultList {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
