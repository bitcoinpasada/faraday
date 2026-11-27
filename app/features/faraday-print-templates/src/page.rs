use faraday_organisms::Ui;

use crate::{Model, Press, PrintTemplates};

impl faraday_present::Page for PrintTemplates {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
