use faraday_organisms::Ui;

use crate::{Model, Press, Vanity};

impl faraday_present::Page for Vanity {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
