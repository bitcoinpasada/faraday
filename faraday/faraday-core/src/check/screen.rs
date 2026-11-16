//! `Screen::Check`: what to hold up against a hardware wallet, a tile for
//! each in Tools' grid, a subheader per group and a divider between them.

use super::{FORMS, GROUPS, Press};
use crate::Action;
use crate::drawing::View;
use crate::screens::catalog::{Face, grid};
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Region, Style, Ui};
use faraday_draw::tokens::{
    BODY_M, CHECK_TAIL, CHECK_TOP, GAP_SMALL, LIST_1, PAD, PAGE_INSET, RADIUS_SMALL, TITLE_S,
};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::section_label;

/// Check: the tiles in the page, scrolling where the panel is
/// short. The top app bar carries the name.
pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let compact = ui.compact;
    let (x, w) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + PAGE_INSET, cw - 2.0 * PAGE_INSET)
    };
    let clip = ui.rect(x0, 0.0, cw, h);
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    let top = if compact {
        CHECK_TOP
    } else {
        crate::screens::bar_head_back(app, ui, EN.compact_check)
    };
    let mut y = top;
    for (g, (name, items)) in GROUPS.iter().enumerate() {
        if g > 0 {
            ui.fill(x, y, w, 1.0, 0.0, Role::OutlineVariant);
            y += GAP_SMALL;
        }
        section_label(ui, x, y, name);
        y += ui.line_as(TITLE_S) + GAP_SMALL;
        let g = grid(compact, w);
        for (k, it) in items.iter().enumerate() {
            let (tx, ty) = g.at(x, y, k);
            let tags: &[&str] = if it.seed { &[EN.check_seed] } else { &[] };
            Face {
                name: it.label,
                line: it.line,
                warn: false,
                tags,
                opens: true,
                action: Action::Check(Press::Row(it.row)),
            }
            .draw(ui, tx, ty, g.tile_w);
        }
        y += g.height(items.len());
        y += GAP_SMALL;
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
        EN.check_backup,
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
