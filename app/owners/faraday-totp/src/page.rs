use faraday_organisms::Ui;

use crate::{Model, Press, Totp};

impl faraday_present::Page for Totp {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
