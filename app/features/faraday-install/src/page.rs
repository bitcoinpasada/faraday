use faraday_organisms::Ui;

use crate::{Model, Press, Install};

impl faraday_present::Page for Install {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
