use faraday_organisms::Ui;
use faraday_organisms::guide;

use crate::{GuideSheet, Model, Press};

impl faraday_present::Page for GuideSheet {
    type Press = Press;

    /// The sheet over the page. The pane is drawn at its place by [`GuideSheet::draw_pane`].
    fn draw(model: &Model<'_>, ui: &mut Ui<'_, Press>) {
        let (Some(sheet), None) = (&model.guide, &model.pane) else {
            return;
        };
        let (w, h) = (ui.c.width() as f32 / ui.f, ui.c.height() as f32 / ui.f);
        guide::draw_sheet(sheet, ui, (w, h));
    }
}

impl GuideSheet {
    /// The supporting pane in the box (x, y, w, h) the pane container gives it, where the model
    /// is the pane's. Nothing of the page under it takes a press.
    pub fn draw_pane(model: &Model<'_>, ui: &mut Ui<'_, Press>, at: (f32, f32, f32, f32)) {
        let (Some(sheet), Some(pane)) = (&model.guide, &model.pane) else {
            return;
        };
        guide::draw_pane(sheet, (Press::Close, &pane.close), ui, at);
    }
}
