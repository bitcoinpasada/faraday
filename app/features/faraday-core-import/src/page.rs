use faraday_organisms::Ui;

use crate::{CoreImport, Model, Press};

impl faraday_present::Page for CoreImport {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
