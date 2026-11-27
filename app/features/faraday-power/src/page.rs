use faraday_organisms::Ui;
use faraday_organisms::content::{self, Block, Content};
use faraday_organisms::sheet;
use faraday_templates::menu_page::page_span;
use faraday_templates::screen::page_room;

use crate::{FailedModel, Model, Power, Press};

impl faraday_present::Page for Power {
    type Press = Press;

    /// The sheet on show, over the page: a dialog on a wide panel, the small panel's sheet on a
    /// small one.
    fn draw(model: &Model<'_>, ui: &mut Ui<'_, Press>) {
        let Some(dialog) = &model.sheet else { return };
        let (w, h) = (ui.c.width() as f32 / ui.f, ui.c.height() as f32 / ui.f);
        sheet::dialog(dialog, ui, (0.0, w, h));
    }
}

impl Power {
    /// The failed self-test's page: its title, the check and the one button, in the page's
    /// width.
    pub fn draw_failed(model: &FailedModel<'_>, ui: &mut Ui<'_, Press>) {
        let (cw, _) = page_room(ui);
        let (x, width) = page_span(ui.compact, 0.0, cw);
        let page = Content(vec![
            Block::Heading(model.title),
            Block::Fact {
                label: model.row,
                value: model.check,
                mono: false,
            },
            Block::Buttons(vec![faraday_organisms::ButtonModel {
                label: model.exit.label,
                style: model.exit.style,
                offered: model.exit.offered.clone(),
                chosen: false,
                icon: None,
            }]),
        ]);
        content::draw(&page, ui, (x, 0.0, width));
    }
}
