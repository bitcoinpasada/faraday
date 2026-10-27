//! `Screen::SetTime`: setting the time by hand (`docs/TOTP.md` section 3).

use super::parts::{section, tz_row};
use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{BODY_S, BUTTON_H, GAP, PAD, TITLE_M};
use faraday_draw::widgets::Icon;
use faraday_ui::parts::{section_label, title, wrap_buttons};

/// The back link at the top of the page.
fn back_link(app: &View, ui: &mut Ui, x: f32, y: f32) {
    let back = Action::SetTimeBack;
    let muted = Role::OnSurfaceVariant;
    ui.icon(x - 4.0, y, 16.0, Icon::ChevronLeft, 10.0, muted);
    let name = app.back_link_name(app.set_time.back_to());
    ui.text_as(x + 14.0, y, BODY_S, muted, app.named_back((name, back)).0);
    ui.hit(x - 4.0, y - 4.0, 70.0, 24.0, back);
    ui.back(back);
}

/// `Screen::SetTime` (`docs/TOTP.md` §3): the Enter screen that sets the
/// time by hand.
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return set_time_compact(app, ui, x0, cw, h);
    }
    let x = x0 + 56.0;
    let width = (cw - 112.0).min(520.0);
    let mut y = 28.0;
    back_link(app, ui, x, y);
    y += 26.0;
    title(ui, x, y, EN.set_time_set_the_time);
    y += 64.0;
    // Context: Now, and the time zone.
    ui.card(x, y, width, 112.0, Role::OutlineVariant);
    ui.text_mid_as(
        (x + 22.0, y + 16.0, 32.0),
        BODY_S,
        Role::OnSurfaceVariant,
        EN.set_time_now,
    );
    let now = app.now_local_text();
    ui.text_right_as(
        (x + width - 22.0, y + 16.0, 32.0),
        TITLE_M,
        Role::OnSurface,
        &now,
    );
    tz_row(ui, x + 22.0, y + 60.0, width - 44.0, app.tz_min());
    y += 132.0;
    // What the time starts from, when it is not this computer's clock.
    if let Some(from) = app.clock_source() {
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, from);
        y += PAD + GAP;
    }
    // Entry: Date and Time.
    section_label(ui, x, y, EN.set_time_date);
    y += 26.0;
    crate::vault::screen::text_box(
        ui,
        x,
        y,
        220.0,
        app.set_time.date(),
        false,
        app.set_time.focus() == Some(crate::set_time::Focus::Date),
        Action::SetTimeFocus(crate::set_time::Focus::Date),
    );
    y += 56.0;
    section_label(ui, x, y, EN.set_time_time);
    y += 26.0;
    crate::vault::screen::text_box(
        ui,
        x,
        y,
        220.0,
        app.set_time.time(),
        false,
        app.set_time.focus() == Some(crate::set_time::Focus::Time),
        Action::SetTimeFocus(crate::set_time::Focus::Time),
    );
    y += 70.0;
    let ready = app.set_time_ready();
    let mut bx = x;
    bx += ui.button(
        bx,
        y,
        Some(120.0),
        BUTTON_H,
        EN.set_time_set,
        Style::Primary.when(ready),
        Action::SetTimeSet,
    ) + 10.0;
    if app.clock_now().is_some() {
        bx += ui.button(
            bx,
            y,
            None,
            BUTTON_H,
            EN.set_time_keep_button,
            Style::Secondary,
            Action::SetTimeKeep,
        ) + 10.0;
    }
    ui.button(
        bx,
        y,
        None,
        BUTTON_H,
        EN.set_time_back,
        Style::Ghost,
        Action::SetTimeBack,
    );
}

/// [`set_time_screen`] on a small panel.
fn set_time_compact(app: &View, ui: &mut Ui, x0: f32, cw: f32, _h: f32) {
    let (x, w) = (x0 + crate::compact::M, cw - 2.0 * crate::compact::M);
    let top = 12.0 - app.list_offset;
    let mut y = top;
    y += section(ui, x, y, w, EN.set_time_now, &|ui, x, y0, w| {
        let mut y = y0;
        y += ui.wrap_as((x, y, w), TITLE_M, Role::OnSurface, &app.now_local_text()) + 10.0;
        if let Some(from) = app.clock_source() {
            y += ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, from) + GAP;
        }
        y += tz_row(ui, x, y, w, app.tz_min());
        y - y0
    });
    section_label(ui, x, y, EN.set_time_date);
    y += 26.0;
    crate::vault::screen::text_box(
        ui,
        x,
        y,
        w,
        app.set_time.date(),
        false,
        app.set_time.focus() == Some(crate::set_time::Focus::Date),
        Action::SetTimeFocus(crate::set_time::Focus::Date),
    );
    y += 56.0;
    section_label(ui, x, y, EN.set_time_time);
    y += 26.0;
    crate::vault::screen::text_box(
        ui,
        x,
        y,
        w,
        app.set_time.time(),
        false,
        app.set_time.focus() == Some(crate::set_time::Focus::Time),
        Action::SetTimeFocus(crate::set_time::Focus::Time),
    );
    y += 66.0;
    let ready = app.set_time_ready();
    let mut items: Vec<(&str, Style, Action)> = vec![(
        EN.set_time_set,
        Style::Primary.when(ready),
        Action::SetTimeSet,
    )];
    if app.clock_now().is_some() {
        items.push((
            EN.set_time_keep_button,
            Style::Secondary,
            Action::SetTimeKeep,
        ));
    }
    items.push((EN.set_time_back, Style::Ghost, Action::SetTimeBack));
    wrap_buttons(ui, x, y, w, BUTTON_H, &items);
}
