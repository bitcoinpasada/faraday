use faraday_organisms::Ui;
use faraday_organisms::sheet;

use crate::{Devices, Model, Press};

impl faraday_present::Page for Devices {
    type Press = Press;

    /// The sheet on show: a dialog on a wide panel, the small panel's sheet on a small one.
    fn draw(model: &Model<'_>, ui: &mut Ui<'_, Press>) {
        let Some(dialog) = &model.sheet else { return };
        let (w, h) = (ui.c.width() as f32 / ui.f, ui.c.height() as f32 / ui.f);
        sheet::dialog(dialog, ui, (0.0, w, h));
    }
}
