//! `Screen::CheckAddress`: Check an address (`docs/DESIGN.md` §5): the
//! outlined field, the ways in under it, then the result card or what
//! the page says in its place. Drawing reads the state and works nothing
//! out.

use super::AddressPress;
use crate::drawing::View;
use crate::scan_read::Way;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::{Action, Screen};
use faraday_draw::tokens::{
    BODY_L, BODY_M, BODY_S, BUTTON_H, COLUMN_MAX, EXPLORE_INSET, FIELD_H, FIELD_PAD, GAP,
    GAP_SMALL, PAD, PAGE_TOP_BAR,
};
use faraday_draw::widgets::Icon;
use faraday_text::Text;
use faraday_ui::parts::measured;
use faraday_ui::roles::ListItem;

/// A small panel's title, and the way back: to the screen it was opened
/// from.
pub(crate) fn back_of(app: &View) -> (&'static str, Action) {
    if app.address_check_state().is_some_and(|s| s.row_shown()) {
        return (
            EN.address_check_title,
            Action::AddressCheck(AddressPress::Back),
        );
    }
    let back = app
        .address_check_state()
        .map_or(Screen::Catalog, |s| s.back());
    (crate::screens::decode_back(back), Action::Nav(back))
}

pub(crate) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let Some(s) = app.address_check_state() else {
        return;
    };
    let (x, width) = if ui.compact {
        (x0 + crate::compact::M, cw - 2.0 * crate::compact::M)
    } else {
        (
            x0 + EXPLORE_INSET,
            (cw - 2.0 * EXPLORE_INSET).min(COLUMN_MAX),
        )
    };
    ui.page_begin((x0, cw, h), app.list_offset);
    let top = crate::compact::M;
    let mut y = if ui.compact {
        top
    } else {
        crate::screens::bar_head_back(app, ui, EN.address_check_title);
        PAGE_TOP_BAR
    };

    // The field, its refusal as its supporting text.
    ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, EN.address_check_field);
    y += PAD + GAP_SMALL;
    let here = Action::TypeHere(crate::edit::Here::CheckAddress);
    let used = ui.field_box((x, y, width), true, s.refusal().map(|t| t.as_str()));
    ui.press_box(x, y, width, FIELD_H, here);
    ui.typed(
        x + FIELD_PAD,
        y,
        FIELD_H,
        BODY_L.size,
        W::M,
        Role::OnSurface,
        s.typed(),
        width - FIELD_PAD * 2.0,
        true,
        here,
    );
    y += used + GAP;

    // The other ways in: the empty part or the files to choose from while
    // the page holds nothing, else each an icon and a label.
    let waiting = super::file::waiting(app);
    if s.is_empty() && waiting.is_empty() {
        y += crate::screens::parts::nothing_to_work_on(
            app,
            ui,
            (x, y, width),
            EN.address_check_no_address,
            (
                Some(Action::AddressCheck(AddressPress::Scan)),
                Screen::CheckAddress,
            ),
        ) + GAP;
        if app.clipboard {
            y += icon_rows(
                ui,
                x,
                y,
                width,
                &[(
                    Icon::Paste,
                    EN.scan_paste,
                    Style::Ghost,
                    Action::AddressCheck(AddressPress::Paste),
                )],
            );
        }
    } else if s.is_empty() {
        let rows: Vec<(&str, Action)> = waiting
            .iter()
            .map(|(i, name)| {
                (
                    name.as_str(),
                    Action::AddressCheck(AddressPress::File(*i as u16)),
                )
            })
            .collect();
        y += crate::screens::parts::file_choices(
            ui,
            (x, y, width),
            EN.address_check_choose_file,
            &rows,
        ) + GAP;
    } else {
        let mut ins = vec![(
            Icon::Scan,
            EN.address_check_scan,
            Style::Tonal,
            Action::AddressCheck(AddressPress::Scan),
        )];
        if app.clipboard {
            ins.push((
                Icon::Paste,
                EN.scan_paste,
                Style::Ghost,
                Action::AddressCheck(AddressPress::Paste),
            ));
        }
        if !super::file::files(app).is_empty() {
            ins.push((
                Icon::File,
                EN.address_check_file,
                Style::Ghost,
                Action::AddressCheck(AddressPress::Files),
            ));
        }
        y += icon_rows(ui, x, y, width, &ins);
    }

    if s.load_first() {
        y += ui.wrap_as(
            (x, y, width),
            BODY_M,
            Role::OnSurfaceVariant,
            EN.address_check_load_first,
        ) + GAP;
    } else if let (Some(list), false) = (s.list(), s.row_shown()) {
        y += listed(ui, (x, y, width), list);
        y += ending(ui, (x, y, width), back_of_page(s));
    } else if let Some(a) = s.answer() {
        // The result in a card, measured before its surface goes down.
        let inner = (x + PAD, y + PAD, width - 2.0 * PAD);
        let used = measured(ui, |ui| crate::address_find::screen::result(ui, inner, a));
        let edge = if a.found { Role::Ok } else { Role::Warning };
        ui.card(x, y, width, used + 2.0 * PAD, edge.paint());
        crate::address_find::screen::result(ui, inner, a);
        y += used + 2.0 * PAD + GAP;
        let ways: Vec<(String, Style, Action)> = s
            .ways()
            .iter()
            .enumerate()
            .map(|(i, way)| {
                let style = match way {
                    Way::MakeWallet | Way::OpenWallet(_) => Style::Primary,
                    _ => Style::Secondary,
                };
                (
                    way.label().to_string(),
                    style,
                    Action::AddressCheck(AddressPress::Way(i as u8)),
                )
            })
            .collect();
        if !ways.is_empty() {
            y += crate::screens::button_rows(ui, x, y, width, &ways);
        }
        y += ending(ui, (x, y, width), back_of_page(s));
    }
    crate::compact::screen::finish(app, ui, h, y - top + PAD);
}

/// The page's Another and Done actions.
fn back_of_page(s: &super::AddressCheckState) -> (Action, Action) {
    (
        Action::AddressCheck(AddressPress::Another),
        Action::Nav(s.back()),
    )
}

/// The end of a result: Another, and Done back to where the page was
/// opened from. Returns the height used, with the gap above.
fn ending(ui: &mut Ui, (x, y, w): (f32, f32, f32), ends: (Action, Action)) -> f32 {
    GAP + crate::screens::parts::ending(ui, (x, y + GAP, w), None, ends)
}

/// The addresses of a file, counted and listed, each a two-line item: the
/// address in short form, then its answer. Returns the height used.
fn listed(ui: &mut Ui, (x, y, w): (f32, f32, f32), list: &super::file::Listed) -> f32 {
    let count = Text::fill(
        EN.address_check_read_count,
        &[
            &Text::num(list.rows().len() as u64),
            &Text::num(list.total() as u64),
        ],
    );
    let mut cy = y + ui.wrap_as((x, y, w), BODY_M, Role::OnSurfaceVariant, &count) + GAP;
    for (i, row) in list.rows().iter().enumerate() {
        let head = faraday_ui::short(&row.address);
        let it = ListItem {
            headline: &head,
            supporting: &row.line,
            trailing: None,
            action: Action::AddressCheck(if row.opens() {
                AddressPress::Row(i as u8)
            } else {
                AddressPress::Back
            }),
        };
        cy += if row.opens() {
            ui.list_item((x, cy, w), &it, false)
        } else {
            ui.list_item_still((x, cy, w), &it)
        };
    }
    cy - y + GAP
}

/// From a file's menu, under the button that opened it: the text files in
/// Files, newest first, as every menu is drawn ([`crate::menu::draw`]).
pub(crate) fn files_menu(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(s) = app.address_check_state() else {
        return;
    };
    let lines: Vec<crate::menu::Line> = super::file::files(app)
        .into_iter()
        .map(|(i, name)| {
            crate::menu::Line::Item(
                name.as_str().into(),
                Action::AddressCheck(AddressPress::File(i as u16)),
            )
        })
        .collect();
    crate::menu::draw(
        ui,
        w,
        h,
        &crate::menu::View {
            title: EN.address_check_file,
            icon: Icon::File,
            lines: &lines,
            at: 0,
            keys: false,
            anchor: s.files_at(),
            cancel: (EN.keygen_back, Action::Cancel),
            shown: 1.0,
        },
    );
}

/// Buttons with an icon before their label, left to right, wrapping at
/// the width. Returns the height they took.
fn icon_rows(ui: &mut Ui, x: f32, y: f32, w: f32, items: &[(Icon, &str, Style, Action)]) -> f32 {
    let bh = ui.button_height(BUTTON_H);
    let pitch = (bh + GAP).max(ui.touch);
    let (mut bx, mut by) = (x, y);
    for &(icon, label, style, action) in items {
        let bw = ui.button_icon_width(label, style).min(w);
        if bx > x && bx + bw > x + w {
            bx = x;
            by += pitch;
        }
        bx += ui.button_icon((bx, by, Some(bw), BUTTON_H), icon, label, style, action) + GAP;
    }
    by - y + pitch
}
