use faraday_organisms::Ui;

use crate::{Model, Press, VaultContents};

impl faraday_present::Page for VaultContents {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
