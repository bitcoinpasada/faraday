use faraday_organisms::Ui;

use crate::{Model, Press, PublicFiles};

impl faraday_present::Page for PublicFiles {
    type Press = Press;

    fn draw(_model: &Model<'_>, _ui: &mut Ui<'_, Press>) {
        todo!()
    }
}
