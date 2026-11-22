//! `Screen::Check`: what to hold up against a hardware wallet, a tile for
//! each in Tools' grid and group layout (`menu_page::section`).

use super::{GROUPS, Press, backup_of};
use crate::Action;
use crate::drawing::View;
use crate::screens::catalog::Face;
use crate::screens::menu_page::{self, Page, Section};
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{
    BODY_M, DISCLOSE_GLYPH, DISCLOSE_ICON_Y, GAP_SMALL, ICON_GLYPH_SMALL, LIST_1, PAD,
    RADIUS_SMALL, TOUCH,
};
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
                    mark: crate::screens::catalog::Mark::None,
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
        fixed_columns: None,
        above: None,
        bar_still: false,
        ways: false,
    };
    menu_page::draw(app, ui, (x0, cw, h), &page, &sections);
}

/// The forms of a backup in another form, as a dialog of rows and Cancel.
pub(crate) fn backups_dialog(ui: &mut Ui, w: f32, h: f32, other: bool) {
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
            let opener = &crate::ways_in::CHECK_BACKUP;
            let first = opener.up_front.iter().filter_map(|w| backup_of(*w));
            let rest = opener
                .other_forms()
                .into_iter()
                .filter_map(|f| backup_of(crate::ways_in::Way::Form(f)));
            let row = |ui: &mut Ui, cy: &mut f32, f: super::Backup| {
                ui.fill(
                    x,
                    *cy,
                    iw,
                    LIST_1,
                    RADIUS_SMALL,
                    Role::SurfaceContainerHighest,
                );
                ui.text_mid_as((x + PAD, *cy, LIST_1), BODY_M, Role::OnSurface, f.label());
                ui.press_box(x, *cy, iw, LIST_1, Action::Check(Press::Form(f)));
                *cy += LIST_1 + GAP_SMALL;
            };
            for f in first {
                row(ui, &mut cy, f);
            }
            let lw = ui.button(
                x,
                cy,
                None,
                TOUCH,
                EN.entry_other_forms,
                Style::Ghost,
                Action::Check(Press::Other),
            );
            ui.icon(
                x + lw + GAP_SMALL,
                cy + DISCLOSE_ICON_Y,
                ICON_GLYPH_SMALL,
                if other {
                    Icon::ChevronUp
                } else {
                    Icon::ChevronRight
                },
                DISCLOSE_GLYPH,
                Role::OnSurfaceVariant,
            );
            cy += TOUCH + GAP_SMALL;
            if other {
                for f in rest {
                    row(ui, &mut cy, f);
                }
            }
            cy - GAP_SMALL - y
        },
        &actions,
    );
}
