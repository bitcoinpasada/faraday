use faraday_organisms::Ui;
use faraday_organisms::content;
use faraday_templates::menu_page::{self, page_span};
use faraday_templates::screen::page_room;

use crate::{Model, Press, Settings};

impl faraday_present::Page for Settings {
    type Press = Press;

    fn draw(model: &Model<'_>, ui: &mut Ui<'_, Press>) {
        match model {
            Model::Settings(page) => menu_page::draw(page, ui),
            Model::SetTime(body) => {
                let (cw, _) = page_room(ui);
                let (x, width) = page_span(ui.compact, 0.0, cw);
                content::draw(body, ui, (x, 0.0, width));
            }
        }
    }
}
