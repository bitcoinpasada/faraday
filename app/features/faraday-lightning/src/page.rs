use faraday_organisms::Ui;

use crate::{Model, Press, Lightning};

impl faraday_present::Page for Lightning {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
