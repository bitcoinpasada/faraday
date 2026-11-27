use faraday_organisms::Ui;

use crate::{Gpg, Model, Press};

impl faraday_present::Page for Gpg {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
