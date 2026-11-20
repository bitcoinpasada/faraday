//! `Screen::Check`: what to hold up against a hardware wallet, a tile for
//! each in Tools' grid and group layout (`catalog::section`).

use super::{FORMS, GROUPS, Press};
use crate::Action;
use crate::drawing::View;
use crate::screens::catalog::{Face, grid, page_span, section};
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Region, Style, Ui};
use faraday_draw::tokens::{BODY_M, CHECK_TAIL, CHECK_TOP, GAP_SMALL, LIST_1, PAD, RADIUS_SMALL};
use faraday_draw::widgets::Icon;

/// Check: the tiles in the page, scrolling where the panel is
/// short. The top app bar carries the name.
pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let compact = ui.compact;
    let (x, w) = page_span(compact, x0, cw);
    let clip = ui.rect(x0, 0.0, cw, h);
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    let top = if compact {
        CHECK_TOP
    } else {
        crate::screens::bar_head_back(app, ui, EN.compact_check)
    };
    let mut y = top;
    let seed_tag = [EN.check_seed];
    let groups: Vec<(&str, Vec<Face>)> = GROUPS
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
            (*name, faces)
        })
        .collect();
    // Every tile on the page is one height.
    let g = grid(compact, w).matching(ui, groups.iter().flat_map(|(_, f)| f));
    for (name, faces) in &groups {
        y += section(ui, (x, y), &g, name, faces);
    }
    app.scroll.content_h().set(y - top + CHECK_TAIL);
    ui.region_close(open, app.scroll.content_h().get() - h);
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
