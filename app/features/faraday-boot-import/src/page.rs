use faraday_organisms::Ui;

use crate::{BootImport, Model, Press};

impl faraday_present::Page for BootImport {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
