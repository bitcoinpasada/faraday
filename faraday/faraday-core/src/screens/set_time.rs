//! `Screen::SetTime`: setting the time by hand (`docs/TOTP.md` section 3).

use super::parts::{section, tz_row, tz_steppers};
use crate::Action;
use crate::drawing::View;
use crate::strings::EN;
use crate::ui::Region;
use crate::ui::pal::*;
use crate::ui::{Style, Ui};
use faraday_draw::tokens::{
    BODY_S, BUTTON_H, FIELD, FIELD_H, GAP, GAP_SMALL, PAD, SHAPE_S, TITLE_M, TITLE_S, TOUCH,
};
use faraday_ui::parts::{section_label, wrap_buttons};

/// One row of the list of zones: a radio mark, the offset and the cities
/// that keep it. Single-choice, as the app's other pick lists are
/// (Material 3, Lists: a list of single-choice items with a radio
/// button).
fn zone_row(ui: &mut Ui, (x, y, w): (f32, f32, f32), (min, clue): (i16, &str), on: bool) {
    let h = FIELD;
    ui.fill(
        x,
        y,
        w,
        h,
        SHAPE_S,
        selected_or(on, Role::SurfaceContainerHighest),
    );
    ui.stroke(x, y, w, h, SHAPE_S, selected_or(on, Role::OutlineVariant));
    let (cx, cy) = (x + PAD + GAP, y + h / 2.0);
    ui.dot(cx, cy, GAP, on_selected(on, Role::OutlineVariant));
    if !on {
        ui.dot(cx, cy, GAP - 2.0, Role::SurfaceContainerHighest);
    }
    let mut name = faraday_text::Text::of("UTC");
    name.push_str(&crate::vault::tz_offset_text(min));
    ui.text_as(
        x + TOUCH,
        y + GAP,
        TITLE_S,
        on_selected(on, Role::OnSurface),
        &name,
    );
    let line = ui.fit_as(BODY_S, clue, w - TOUCH - PAD);
    let ly = cy + GAP_SMALL;
    ui.text_as(
        x + TOUCH,
        ly,
        BODY_S,
        on_selected(on, Role::OnSurfaceVariant),
        &line,
    );
    ui.press_box(x, y, w, h, Action::TzSet(min));
}

/// The list of zones in place of the fields: one row for each offset in
/// the table, then the steppers for an offset it lacks, and Back to the
/// fields. `(x, w)` is the column the rows take.
fn zone_list(app: &View, ui: &mut Ui, (x0, cw, h): (f32, f32, f32), (x, w): (f32, f32)) {
    let clip = ui.rect(x0, 0.0, cw, h);
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    let top = if ui.compact {
        12.0
    } else {
        let name = EN.set_time_set_the_time;
        let back = app.named_back((name, Action::TzList(false)));
        super::bar_head(ui, Some(back), EN.tz_row_time_zone)
    };
    let mut y = top;
    let chosen = app.tz_chosen();
    for &(min, clue) in faraday_strings::zones::ZONES {
        zone_row(ui, (x, y, w), (min, clue), chosen == Some(min));
        y += FIELD + GAP_SMALL;
    }
    y += GAP;
    section_label(ui, x, y, EN.set_time_other_offset);
    y += ui.advance_as(TITLE_S, 26.0);
    y += tz_steppers(ui, x, y, chosen) + PAD;
    ui.button(
        x,
        y,
        None,
        BUTTON_H,
        EN.set_time_back,
        Style::Ghost,
        Action::TzList(false),
    );
    y += ui.button_height(BUTTON_H);
    app.scroll.content_h().set(y - top + PAD);
    ui.region_close(open, app.scroll.content_h().get() - h);
}

/// The Date and Time fields, each under its label, `fw` wide. Returns
/// the height they take.
fn fields(app: &View, ui: &mut Ui, (x, y0): (f32, f32), fw: f32) -> f32 {
    use crate::set_time::Focus;
    let mut y = y0;
    for (label, text, focus) in [
        (EN.set_time_date, app.set_time.date(), Focus::Date),
        (EN.set_time_time, app.set_time.time(), Focus::Time),
    ] {
        section_label(ui, x, y, label);
        y += ui.advance_as(TITLE_S, 26.0);
        crate::vault::screen::text_box(
            ui,
            x,
            y,
            fw,
            text,
            false,
            app.set_time.focus() == Some(focus),
            Action::SetTimeFocus(focus),
        );
        y += FIELD_H + PAD;
    }
    y - y0
}

/// The line that says what Set would set, both ways, when Set is offered.
/// Returns the height it takes.
fn result_line(app: &View, ui: &mut Ui, (x, y, w): (f32, f32, f32)) -> f32 {
    match app.set_time_result() {
        Some(r) => ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, &r) + GAP,
        None => 0.0,
    }
}

/// `Screen::SetTime` (`docs/TOTP.md` §3): the Enter screen that sets the
/// time by hand.
pub(super) fn draw(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    if ui.compact {
        return set_time_compact(app, ui, x0, cw, h);
    }
    let x = x0 + 56.0;
    let width = (cw - 112.0).min(520.0);
    if app.set_time.zones() {
        return zone_list(app, ui, (x0, cw, h), (x, width));
    }
    let name = app.back_link_name(app.set_time.back_to());
    let back = app.named_back((name, Action::SetTimeBack));
    let mut y = super::bar_head(ui, Some(back), EN.set_time_set_the_time);
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
    tz_row(ui, x + 22.0, y + 60.0, width - 44.0, app.tz_chosen());
    y += 132.0;
    // What the time starts from, when it is not this computer's clock.
    if let Some(from) = app.clock_source() {
        ui.text_as(x, y, BODY_S, Role::OnSurfaceVariant, from);
        y += PAD + GAP;
    }
    // Entry: Date and Time.
    y += fields(app, ui, (x, y), 220.0) + 14.0;
    y += result_line(app, ui, (x, y, width));
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
    if app.clock_now().is_some() && app.tz_known() {
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
        Action::Back,
    );
}

/// [`set_time_screen`] on a small panel.
fn set_time_compact(app: &View, ui: &mut Ui, x0: f32, cw: f32, h: f32) {
    let (x, w) = (x0 + crate::compact::M, cw - 2.0 * crate::compact::M);
    if app.set_time.zones() {
        return zone_list(app, ui, (x0, cw, h), (x, w));
    }
    // Scrolled as far as the end it last drew to, so its fields come up
    // above the keyboard.
    let clip = ui.rect(x0, 0.0, cw, h);
    let open = ui.region_open(Region::page(clip).from_zero(app.list_offset));
    let top = 12.0;
    let mut y = top;
    y += section(ui, x, y, w, EN.set_time_now, &|ui, x, y0, w| {
        let mut y = y0;
        y += ui.wrap_as((x, y, w), TITLE_M, Role::OnSurface, &app.now_local_text()) + 10.0;
        if let Some(from) = app.clock_source() {
            y += ui.wrap_as((x, y, w), BODY_S, Role::OnSurfaceVariant, from) + GAP;
        }
        y += tz_row(ui, x, y, w, app.tz_chosen());
        y - y0
    });
    y += fields(app, ui, (x, y), w) + 10.0;
    y += result_line(app, ui, (x, y, w));
    let ready = app.set_time_ready();
    let mut items: Vec<(&str, Style, Action)> = vec![(
        EN.set_time_set,
        Style::Primary.when(ready),
        Action::SetTimeSet,
    )];
    if app.clock_now().is_some() && app.tz_known() {
        items.push((
            EN.set_time_keep_button,
            Style::Secondary,
            Action::SetTimeKeep,
        ));
    }
    items.push((EN.set_time_back, Style::Ghost, Action::Back));
    y += wrap_buttons(ui, x, y, w, BUTTON_H, &items);
    app.scroll.content_h().set(y - top + PAD);
    ui.region_close(open, app.scroll.content_h().get() - h);
}
