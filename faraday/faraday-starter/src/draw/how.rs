//! **How Faraday does it**'s sheet (`docs/CRYPTO-LEARN.md` §8.4).

use faraday_draw::tokens::{MARGIN_COMPACT, MARGIN_WIDE};
use faraday_draw::widgets::Icon;
use faraday_ui::{Role, Style};

use super::{S, Ui};
use crate::{Starter, StarterAction};

/// **How Faraday does it**: a sheet in the Starter's look
/// (`faraday_ui::sheet::scroll_sheet`), the window's width less its margin
/// on a compact window, centred and at most `DIALOG_MAX` wide
/// otherwise; its head, the blocks of the page's computing actions
/// scrolling in it, and **Close** at its foot. A press beside it closes
/// it (`docs/CRYPTO-LEARN.md` §8.4).
pub(super) fn sheet(starter: &Starter, ui: &mut Ui, w: f32, h: f32) {
    let margin = if starter.compact() {
        MARGIN_COMPACT
    } else {
        MARGIN_WIDE
    };
    let sw = if starter.compact() {
        w - 2.0 * margin
    } else {
        (w - 2.0 * margin).min(faraday_draw::tokens::DIALOG_MAX)
    };
    let blocks = starter.how_blocks();
    let release = crate::VERSION;
    let outside = ui.outside.replace(StarterAction::Cancel.host());
    faraday_ui::sheet::scroll_sheet(
        ui,
        ((w - sw) / 2.0, sw),
        h,
        margin,
        faraday_draw::tokens::PAD,
        &mut |ui, x, y, iw| {
            let mut cy = y;
            cy += faraday_ui::sheet::sheet_head(
                ui,
                x,
                cy,
                iw,
                Icon::Method,
                Role::OnSurface,
                S.starter_how_title,
            );
            cy += faraday_ui::how::blocks(ui, (x, cy, iw), &blocks, release);
            // Pinned to the sheet's foot.
            faraday_ui::sheet::buttons(
                ui,
                x,
                cy,
                iw,
                &[(
                    S.starter_how_close,
                    Style::Ghost,
                    StarterAction::Cancel.host(),
                )],
            );
            cy - y
        },
    );
    ui.outside = outside;
}
