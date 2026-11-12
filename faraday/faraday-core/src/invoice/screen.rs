//! `Screen::Invoice`: a Lightning invoice taken apart.

use crate::drawing::View;
use crate::strings::EN;
use crate::ui::Region;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, grouped};
use crate::{Action, Screen};
use faraday_draw::tokens::{
    BODY_M, BODY_S, BUTTON_H, FIELD_EDGE_ACTIVE, FIELD_RADIUS, INVOICE_BLOCK_GAP, INVOICE_FIELD_H,
    INVOICE_FIELD_PAD_X, INVOICE_FIELD_PAD_Y, INVOICE_GAP, INVOICE_LABEL_W, INVOICE_NAME_DROP,
    INVOICE_NAME_H, INVOICE_PAGE_W, INVOICE_REFUSAL_GAP, INVOICE_ROW_STEP, INVOICE_RULE_DROP,
    INVOICE_TAIL, INVOICE_TOP, INVOICE_WIDE_ROW_H, INVOICE_WIDE_RULE_GAP, INVOICE_WIDE_STEP,
    PAGE_INSET,
};
use faraday_ui::parts::wrap_buttons;

use super::InvoicePress;

/// A small panel's title, and the way back: to the screen it was opened
/// from.
pub(crate) fn back_of(app: &View) -> (&'static str, Action) {
    let back = app.invoice().map_or(Screen::Catalog, |s| s.back());
    (crate::screens::decode_back(back), Action::Nav(back))
}

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = app.invoice() else {
        return;
    };
    let compact = ui.compact;
    let (x, w) = if compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (x0 + PAGE_INSET, (cw - 2.0 * PAGE_INSET).min(INVOICE_PAGE_W))
    };
    let clip = ui.rect(x0, 0.0, cw, h);
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    let top = if compact {
        INVOICE_TOP
    } else {
        let back = app.named_back(back_of(app));
        crate::screens::bar_head(ui, Some(back), EN.invoice_title)
    };
    let mut y = top;
    let guide = EN.invoice_about;
    // On a small panel the About page comes after what it explains.
    if !compact {
        y += crate::screens::parts::guide_text(app, ui, x, y, w, guide);
    }

    // Material's outlined text field, its edge thicker and in primary
    // while typing goes to it, which is always here.
    let here = Action::TypeHere(crate::edit::Here::Invoice);
    let field = (x, y, w, INVOICE_FIELD_H);
    crate::menu::outline(
        ui,
        field,
        FIELD_RADIUS,
        FIELD_EDGE_ACTIVE,
        Role::Primary.paint(),
    );
    ui.press_box(x, y, w, INVOICE_FIELD_H, here);
    // The text stops above the bottom inset, so no line shows cut.
    ui.c.push_clip(ui.rect(x, y, w, INVOICE_FIELD_H - INVOICE_FIELD_PAD_Y));
    let room = (
        x + INVOICE_FIELD_PAD_X,
        y + INVOICE_FIELD_PAD_Y,
        w - 2.0 * INVOICE_FIELD_PAD_X,
    );
    if s.text().is_empty() {
        ui.wrap_as(room, BODY_M, Role::OnSurfaceVariant, EN.invoice_field);
        let ch = BODY_M.size * ui.text_factor();
        ui.caret(room.0, room.1 + (ui.line_as(BODY_M) - ch) / 2.0, ch);
    } else {
        ui.typed_lines(room, BODY_M.mono(), Role::OnSurface, s.text(), (true, here));
    }
    ui.c.pop_clip();
    y += INVOICE_FIELD_H + INVOICE_GAP;

    y += wrap_buttons(
        ui,
        x,
        y,
        w,
        BUTTON_H,
        &[
            (
                EN.invoice_decode,
                Style::Primary,
                Action::Invoice(InvoicePress::Decode),
            ),
            (
                EN.invoice_scan,
                Style::Secondary,
                Action::Invoice(InvoicePress::Scan),
            ),
            (
                EN.invoice_clear,
                Style::Secondary,
                Action::Invoice(InvoicePress::Clear),
            ),
        ],
    ) + INVOICE_BLOCK_GAP;

    match s.read() {
        None => {}
        Some(Err(why)) => {
            y += ui.wrap_as((x, y, w), BODY_M, Role::Error, why) + INVOICE_REFUSAL_GAP;
        }
        Some(Ok(rows)) => y = draw_rows(ui, rows, (x, y, w)),
    }
    if compact {
        y += INVOICE_BLOCK_GAP
            + crate::compact::screen::about(ui, x, y + INVOICE_BLOCK_GAP, w, guide);
    }
    app.scroll.content_h().set(y - top + INVOICE_TAIL);
    ui.region_close(open, app.scroll.content_h().get() - h);
}

/// The rows of a decoded invoice, name and value, from `y`; returns the
/// `y` under the last.
fn draw_rows(ui: &mut Ui, rows: &[super::Row], (x, mut y, w): (f32, f32, f32)) -> f32 {
    let compact = ui.compact;
    for row in rows {
        let face = if row.mono { BODY_M.mono() } else { BODY_M };
        let value = if row.mono {
            grouped(&row.value)
        } else {
            row.value.clone()
        };
        if compact {
            ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, &row.label);
            let vh = ui.wrap_as((x, y + INVOICE_NAME_H, w), face, Role::OnSurface, &value);
            ui.rule(x, y + vh + INVOICE_RULE_DROP, w, Role::OutlineVariant);
            y += vh + INVOICE_ROW_STEP;
        } else {
            ui.text_as(
                x,
                y + INVOICE_NAME_DROP,
                BODY_S,
                Role::OnSurfaceVariant,
                &row.label,
            );
            let vh = ui.wrap_as(
                (x + INVOICE_LABEL_W, y, w - INVOICE_LABEL_W),
                face,
                Role::OnSurface,
                &value,
            );
            let rh = vh.max(INVOICE_WIDE_ROW_H);
            ui.rule(x, y + rh + INVOICE_WIDE_RULE_GAP, w, Role::OutlineVariant);
            y += rh + INVOICE_WIDE_STEP;
        }
    }
    y
}
