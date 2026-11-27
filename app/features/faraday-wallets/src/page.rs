use faraday_organisms::Ui;

use crate::{Model, Press, Wallets};

impl faraday_present::Page for Wallets {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
