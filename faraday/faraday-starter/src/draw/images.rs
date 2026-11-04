//! Make a Faraday stick's and Make a Pi card's Image (`docs/STARTER.md`
//! §3.2, §3.3): the images that can be written, one ticked, then the
//! files to download for hardware whose image was not found. A Material 3
//! list: two-line items, a radio on the choices, a file icon on the rest.

use faraday_draw::tokens::{IMAGE_LIST_GAP, LIST_2};
use faraday_draw::widgets::Icon;
use faraday_strings::fill;
use faraday_ui::Role;

use super::row::{ROW_ROOM, ROW_TEXT, press_row};
use super::{S, Ui, label, nav_row, radio_mark, two_lines};
use crate::decide::Target;
use crate::{Starter, StarterAction};

/// The section from (x, y0), `w` wide: the choices, then what is not
/// found. Nothing while there is nothing found and nothing to download.
/// Returns its height.
pub(super) fn section(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32, t: Target) -> f32 {
    let choices = starter.image_choices(t);
    let missing = starter.images_to_download(t);
    let refused = starter.images_refused(t);
    if missing.is_empty() && refused.is_empty() && choices.is_empty() {
        return 0.0;
    }
    let mut y = y0;
    if !choices.is_empty() {
        y += label(ui, x, y, S.starter_images);
        ui.rule(x, y, w, Role::OutlineVariant);
        for (n, c) in choices.iter().enumerate() {
            let sub = fill(S.starter_image_found, &[&c.folder, &c.build]);
            let action = StarterAction::Image(n as u16);
            press_row(ui, x, y, w, LIST_2, action);
            radio_mark(ui, x, y + LIST_2 / 2.0, c.selected, true);
            two_lines(
                ui,
                (x + ROW_TEXT, y, LIST_2),
                (w - ROW_ROOM, w - ROW_ROOM),
                (&c.head, Role::OnSurface.paint()),
                (&sub, Role::OnSurfaceVariant.paint()),
            );
            y += LIST_2;
            ui.rule(x, y, w, Role::OutlineVariant);
        }
        y += IMAGE_LIST_GAP;
    }
    if !missing.is_empty() || !refused.is_empty() {
        y += label(ui, x, y, S.starter_images_missing);
        ui.rule(x, y, w, Role::OutlineVariant);
        for name in missing {
            y += nav_row(
                ui,
                x,
                y,
                w,
                Icon::File,
                name,
                S.starter_image_download,
                "",
                None,
            );
        }
        for (name, needs) in refused {
            let sub = needs.map_or_else(
                || S.starter_image_mismatch.to_string(),
                |v| fill(S.starter_image_other, &[v]),
            );
            y += nav_row(ui, x, y, w, Icon::File, name, &sub, "", None);
        }
        y += IMAGE_LIST_GAP;
    }
    y - y0
}
