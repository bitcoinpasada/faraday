//! The vault's last section: the items that could not be read.

use crate::strings::{EN, fill};
use crate::ui::Ui;
use crate::ui::pal::*;
use crate::vault::{OpenVault, errors::fault_line};
use faraday_draw::tokens::{self, BODY_M, BODY_S, TITLE_S};

use crate::drawing::View;

/// Draws "Not read here · n" and a row for each item the store could not
/// read, at (x, y) and `w` wide: the title, then the field or key and the
/// fault. No actions. Returns the height used, 0 when there is none.
pub(super) fn section(app: &View, ui: &mut Ui, v: &OpenVault, at: (f32, f32, f32)) -> f32 {
    let (x, y0, w) = at;
    let list = app.vaults.store().unreadable(v.id);
    if list.is_empty() {
        return 0.0;
    }
    let mut y = y0 + tokens::GAP;
    let head = fill(EN.vault_not_read, &[&list.len().to_string()]);
    ui.rule(x, y, w, Role::OutlineVariant);
    y += tokens::GAP;
    ui.text_as(x, y, TITLE_S, Role::OnSurfaceVariant, &head);
    y += ui.line_as(TITLE_S) + tokens::GAP_SMALL;
    for u in list {
        let title = if u.title.is_empty() {
            EN.vault_no_title
        } else {
            u.title.as_str()
        };
        y += ui.wrap_as((x, y, w), BODY_M, Role::OnSurface, title);
        let why = format!("{} · {}", &*u.what.at, fault_line(u.what.fault));
        y += ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, &why);
        y += tokens::GAP_SMALL;
    }
    y - y0
}
