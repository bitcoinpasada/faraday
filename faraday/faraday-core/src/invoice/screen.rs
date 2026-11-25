//! `Screen::Invoice`: a Lightning invoice taken apart.

use crate::drawing::View;
use crate::pick;
use crate::strings::EN;
use crate::ui::Region;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, grouped};
use crate::{Action, Screen};
use faraday_draw::tokens::{
    BODY_M, BODY_S, BUTTON_H, FIELD_EDGE_ACTIVE, FIELD_RADIUS, INVOICE_BLOCK_GAP,
    INVOICE_CLEAR_ROOM, INVOICE_CLEAR_TOP, INVOICE_CLEAR_W, INVOICE_FIELD_H, INVOICE_FIELD_PAD_X,
    INVOICE_FIELD_PAD_Y, INVOICE_GAP, INVOICE_LABEL_W, INVOICE_NAME_DROP, INVOICE_NAME_H,
    INVOICE_PAGE_W, INVOICE_REFUSAL_GAP, INVOICE_ROW_STEP, INVOICE_RULE_DROP, INVOICE_TAIL,
    INVOICE_TOP, INVOICE_WIDE_ROW_H, INVOICE_WIDE_RULE_GAP, INVOICE_WIDE_STEP, PAGE_INSET,
};
use faraday_ui::parts::wrap_buttons;

use super::InvoicePress;

/// A small panel's title, and the way back: to the screen it was opened
/// from.
pub(crate) fn back_of(app: &View) -> (&'static str, Action) {
    let back = app.invoice().map_or(Screen::Catalog, |s| s.back());
    (crate::screens::decode_back(back), Action::Nav(back))
}

/// Material's outlined text field, its edge thicker and in primary while
/// typing goes to it; returns its height with the gap under it.
fn field(ui: &mut Ui, s: &super::InvoiceState, (x, y, w): (f32, f32, f32), card: bool) -> f32 {
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
    // On the card Clear stands at the field's right, as the tools' do.
    let beside = if card { INVOICE_CLEAR_ROOM } else { 0.0 };
    let room = (
        x + INVOICE_FIELD_PAD_X,
        y + INVOICE_FIELD_PAD_Y,
        w - 2.0 * INVOICE_FIELD_PAD_X - beside,
    );
    if s.text().is_empty() {
        ui.wrap_as(room, BODY_M, Role::OnSurfaceVariant, EN.invoice_field);
        let ch = BODY_M.size * ui.text_factor();
        ui.caret(room.0, room.1 + (ui.line_as(BODY_M) - ch) / 2.0, ch);
    } else {
        ui.typed_lines(room, BODY_M.mono(), Role::OnSurface, s.text(), (true, here));
    }
    ui.c.pop_clip();
    if card {
        ui.button(
            x + w - INVOICE_CLEAR_ROOM,
            y + INVOICE_CLEAR_TOP,
            Some(INVOICE_CLEAR_W),
            BUTTON_H,
            EN.invoice_clear,
            Style::Ghost,
            Action::Invoice(InvoicePress::Clear),
        );
    }
    INVOICE_FIELD_H + INVOICE_GAP
}

/// The three buttons under the field; returns their height.
fn buttons(ui: &mut Ui, (x, y, w): (f32, f32, f32), card: bool) -> f32 {
    let decode = (
        EN.invoice_decode,
        Style::Primary,
        Action::Invoice(InvoicePress::Decode),
    );
    // On the card the pick at the left holds Scan; Decode stays.
    if card {
        return wrap_buttons(ui, x, y, w, BUTTON_H, &[decode]);
    }
    wrap_buttons(
        ui,
        x,
        y,
        w,
        BUTTON_H,
        &[
            decode,
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
    )
}

/// What the text read as, from `y`: the refusal, or the lines and the
/// end; returns the height.
fn reading(ui: &mut Ui, s: &super::InvoiceState, (x, y, w): (f32, f32, f32)) -> f32 {
    match s.read() {
        None => 0.0,
        Some(Err(why)) => ui.wrap_as((x, y, w), BODY_M, Role::Error, why) + INVOICE_REFUSAL_GAP,
        Some(Ok(rows)) => {
            let under = draw_rows(ui, rows, (x, y, w));
            // The end: the tool again, empty, or Done to where it was
            // opened.
            under - y
                + crate::screens::parts::ending(
                    ui,
                    (x, under, w),
                    None,
                    (Action::Invoice(InvoicePress::Clear), Action::Nav(s.back())),
                )
                + INVOICE_BLOCK_GAP
        }
    }
}

/// The tool as the flow's one card: Input, then Result
/// (`docs/FLOW-CONTAINER.md` §2).
fn card(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = app.invoice() else { return };
    let back = Some(app.named_back(back_of(app)));
    let page = crate::flow::Page {
        area: (x0, cw),
        h,
        foot_h: 0.0,
        back,
        heading: EN.invoice_title,
        chip: None,
        note: None,
        chip_tap: None,
        step: None,
        moves: crate::ui::Moves::No,
    };
    let Some(contained) = crate::trail::StepFlow::contained(&super::flow::INVOICE_STEPS) else {
        return;
    };
    let scroll = s.scroll;
    let (next, again, _) = {
        let mut body = |ui: &mut Ui, step: u8, x: f32, y: f32, w: f32| -> f32 {
            let area_w = cw;
            if step == super::flow::INPUT {
                let mut used = 0.0;
                // Where the pair stands the pane holds the ways in.
                if !crate::flow::pair_left(app, &super::flow::INVOICE_STEPS, ui.compact, area_w) {
                    let ways = pick::Decl::new(EN.invoice_title).ways(super::flow::ways_in());
                    used += pick::draw_groups(ui, x, y, w, &ways.groups());
                }
                used += field(ui, s, (x, y + used, w), true);
                used += buttons(ui, (x, y + used, w), true);
                used
            } else {
                reading(ui, s, (x, y, w))
            }
        };
        let mut under = |_: &mut Ui, _: f32, _: f32, _: f32| 0.0;
        crate::flow::contained(ui, app, contained, &page, scroll, &mut body, &mut under)
    };
    app.scroll(crate::drawing::Scrolled::Invoice, next, again);
}

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = app.invoice() else {
        return;
    };
    if !ui.compact {
        return card(app, ui, x0, cw, h);
    }
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

    y += field(ui, s, (x, y, w), false);

    y += buttons(ui, (x, y, w), false) + INVOICE_BLOCK_GAP;

    y += reading(ui, s, (x, y, w));
    if compact {
        let guide = EN.invoice_about;
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
            String::from(&*grouped(&row.value))
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
