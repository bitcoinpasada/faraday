use faraday_organisms::Ui;

use crate::{Model, Press, SecureBoot};

impl faraday_present::Page for SecureBoot {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
