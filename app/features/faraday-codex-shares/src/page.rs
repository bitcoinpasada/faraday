use faraday_organisms::Ui;

use crate::{Model, Press, CodexShares};

impl faraday_present::Page for CodexShares {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
