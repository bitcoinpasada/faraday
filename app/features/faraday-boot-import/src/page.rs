use faraday_organisms::Ui;

use crate::{Model, Press, BootImport};

impl faraday_present::Page for BootImport {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
