//! `Screen::Check`: what to hold up against a hardware wallet, a tile for
//! each in Tools' grid and group layout (`menu_page::section`).

use super::{FORMS, GROUPS, Press};
use crate::Action;
use crate::drawing::View;
use crate::screens::catalog::Face;
use crate::screens::menu_page::{self, Page, Section};
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{BODY_M, GAP_SMALL, LIST_1, PAD, RADIUS_SMALL};
use faraday_draw::widgets::Icon;

/// Check: the tiles on the menu page container, scrolling where the panel
/// is short. The top app bar carries the name and the way back.
pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let seed_tag = [EN.check_seed];
    let sections: Vec<Section> = GROUPS
        .iter()
        .map(|(name, items)| {
            let faces = items
                .iter()
                .map(|it| Face {
                    name: it.label,
                    line: it.line,
                    warn: false,
                    tags: if it.seed { &seed_tag } else { &[] },
                    opens: true,
                    action: Action::Check(Press::Row(it.row)),
                })
                .collect();
            Section::tiles(Some(name), faces)
        })
        .collect();
    let page = Page {
        name: EN.compact_check,
        back: true,
        search: None,
        most_columns: usize::MAX,
    };
    menu_page::draw(app, ui, (x0, cw, h), &page, &sections);
}

/// The forms of a backup in another form, as a dialog of rows and Cancel.
pub(crate) fn backups_dialog(ui: &mut Ui, w: f32, h: f32) {
    use faraday_draw::tokens::DIALOG_MAX;
    let head = (
        Icon::Checklist.into(),
        Role::Primary.paint(),
        EN.catalog_check_a_backup,
    );
    let actions = [(EN.vault_cancel, Style::Secondary, Action::Cancel)];
    crate::compact::dialog(
        ui,
        (w, h, DIALOG_MAX),
        head,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            for (f, label) in FORMS {
                ui.fill(
                    x,
                    cy,
                    iw,
                    LIST_1,
                    RADIUS_SMALL,
                    Role::SurfaceContainerHighest,
                );
                ui.text_mid_as((x + PAD, cy, LIST_1), BODY_M, Role::OnSurface, label);
                ui.press_box(x, cy, iw, LIST_1, Action::Check(Press::Form(f)));
                cy += LIST_1 + GAP_SMALL;
            }
            cy - GAP_SMALL - y
        },
        &actions,
    );
}
