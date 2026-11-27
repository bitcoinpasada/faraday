use faraday_organisms::Ui;
use faraday_templates::menu_page;

use crate::{Home, Model, Press};

impl faraday_present::Page for Home {
    type Press = Press;

    fn draw(model: &Model<'_>, ui: &mut Ui<'_, Press>) {
        menu_page::draw(&model.page, ui);
    }
}
