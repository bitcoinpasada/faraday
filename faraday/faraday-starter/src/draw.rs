//! The Starter's pages, drawn each frame from its state with `faraday-ui`'s
//! `Ui`, in the desktop app's palette and sizes (`docs/STARTER.md` §3.1d):
//! the three destinations under a top app bar, beside the navigation rail
//! or above the navigation bar by the window's class, a task page under a
//! bar with the way back.

use faraday_core::Action;
use faraday_core::upgrade::{Fit, describe, version};
use faraday_draw::widgets::Icon;
use faraday_strings::fill;
use faraday_strings::{EN, Strings};
use faraday_ui::Role;
use faraday_ui::parts::qr_code;
use faraday_ui::{Glyph, NameSuffix, Style, W};

use crate::api::{Concern, InUse, InstallItem, Kind, Mode, Os, Reason, StickFault, WriteStage};
use crate::decide::{Item, Refusal, State, Target};
use crate::macs::{self, StickFor};
use crate::notes;
use crate::template::{CODEX_LENGTHS, CodexCopy, Copy, Form};
use crate::{
    Card, Destination, LearnId, ModeRefusal, Page, RestartTo, Sheet, Starter, StarterAction,
    TemplateChoice,
};

/// Faraday's drawing, a press being the desktop app's action that carries
/// the Starter's own ([`StarterAction::host`]).
type Ui<'a> = faraday_ui::Ui<'a, Action>;

const S: &Strings = &EN;

mod appearance;
mod home;
mod nav;

use faraday_draw::tokens::{
    CONTENT_MAX, DISABLED_CONTAINER, DISABLED_CONTENT, GAP, MARGIN_COMPACT, MARGIN_WIDE, NAV_BAR_H,
    PROGRESS_H, RAIL_W, STATE_PRESS,
};

/// The widest a task page's column grows (a text column, not a grid).
const COL_W: f32 = 720.0;

/// Draws the frame (§3.1d): the page under the top app bar, beside the
/// rail or above the bar on the three destinations, and under a bar with
/// the way back on a task page. Returns how far the page can scroll,
/// units.
pub(crate) fn frame(starter: &Starter, ui: &mut Ui) -> f32 {
    let (w, h) = (starter.w, starter.h);
    // Under a sheet that has its frosted copy of the page, the page is
    // not drawn again: the copy stands for it, and so does the scroll
    // it was drawn at (`Starter::frost`).
    if let Some(sheet) = starter.sheet
        && ui.frost.is_some()
    {
        sheet_over(starter, ui, w, h, sheet);
        return starter.max;
    }
    if starter.showing {
        held_chrome(starter, ui);
        return starter.max;
    }
    ui.clear(Role::Surface);
    let dest = starter.page.destination();
    let compact = starter.compact();
    let rail_w = if dest.is_some() && !compact {
        RAIL_W
    } else {
        0.0
    };
    let bar_h = if dest.is_some() && compact {
        NAV_BAR_H
    } else {
        0.0
    };
    let margin = if compact { MARGIN_COMPACT } else { MARGIN_WIDE };
    let area_w = w - rail_w;
    let widest = if dest.is_some() { CONTENT_MAX } else { COL_W };
    let cw = (area_w - 2.0 * margin).min(widest);
    let x = rail_w + (area_w - cw) / 2.0;
    let top = faraday_draw::tokens::APP_BAR_H;
    let view_h = h - top - bar_h;
    let clip = ui.rect(rail_w, top, area_w, view_h);
    ui.c.push_clip(clip);
    let y0 = top + margin - starter.offset;
    let end = match starter.page {
        Page::Home => home::home(starter, ui, x, y0, cw),
        Page::Make => home::make(starter, ui, x, y0, cw),
        Page::Tools => home::tools(starter, ui, x, y0, cw),
        Page::Which => which(starter, ui, x, y0, cw),
        Page::Disks => disks(starter, ui, x, y0, cw),
        Page::Mode => mode(starter, ui, x, y0, cw),
        Page::Confirm => confirm(starter, ui, x, y0, cw),
        Page::Writing => writing(starter, ui, x, y0, cw),
        Page::Done => done(starter, ui, x, y0, cw),
        Page::Start => start(starter, ui, x, y0, cw),
        Page::Install => install(starter, ui, x, y0, cw),
        Page::Installed => installed(starter, ui, x, y0, cw),
        Page::About => about(starter, ui, x, y0, cw),
        Page::Appearance => appearance::page(starter, ui, x, y0, cw),
        Page::Templates => templates(starter, ui, x, y0, cw),
        Page::TemplateWords => template_words(starter, ui, x, y0, cw),
        Page::TemplateCopy => template_copy(starter, ui, x, y0, cw),
        Page::TemplatePlaces => template_places(starter, ui, x, y0, cw),
        Page::TemplateForm => template_form(starter, ui, x, y0, cw),
        Page::TemplateLength => template_length(starter, ui, x, y0, cw),
        Page::TemplateWorksheets => template_worksheets(starter, ui, x, y0, cw),
    };
    ui.c.pop_clip();
    let content = end - y0 + 2.0 * margin;
    let max = (content - view_h).max(0.0);
    ui.report_scroll(clip, max);
    nav::top_bar(starter, ui, rail_w, area_w);
    if dest.is_some() {
        if compact {
            nav::bottom_bar(starter, ui, h, w);
        } else {
            nav::rail(starter, ui, h);
        }
    }
    if starter.menu {
        nav::menu(starter, ui, w);
    }
    if let Some(sheet) = starter.sheet {
        sheet_over(starter, ui, w, h, sheet);
    }
    max
}
/// The Starter's own frame round the held desktop app (§3.1d): the top
/// app bar over it, with the way back on Transfer and a Learn page opened
/// from a page, and on the Learn destination the rail or the bar at the
/// foot. The held app's frame is copied in under it ([`Starter::compose`]).
fn held_chrome(starter: &Starter, ui: &mut Ui) {
    let (w, h) = (starter.w, starter.h);
    let compact = starter.compact();
    let rail_w = if starter.learn_dest && !compact {
        RAIL_W
    } else {
        0.0
    };
    nav::top_bar(starter, ui, rail_w, w - rail_w);
    if starter.learn_dest {
        if compact {
            nav::bottom_bar(starter, ui, h, w);
        } else {
            nav::rail(starter, ui, h);
        }
    }
    if starter.menu {
        nav::menu(starter, ui, w);
    }
}

/// The open sheet over the frosted page: frosted now from what the frame
/// drew, or put back from the copy kept.
fn sheet_over(starter: &Starter, ui: &mut Ui, w: f32, h: f32, sheet: Sheet) {
    ui.clear_hits();
    ui.in_sheet = true;
    // The sheet scrolls by its own offset; the page under it keeps its.
    ui.offset = starter.sheet_offset();
    ui.backdrop();
    match sheet {
        Sheet::Restart(to) => restart_sheet(starter, ui, w, h, to),
        Sheet::Uninstall => uninstall_sheet(starter, ui, w, h),
        Sheet::LetThrough => let_through_sheet(starter, ui, w, h),
    }
    ui.in_sheet = false;
}

// ---------------------------------------------------------------------
// The frame's pieces
// ---------------------------------------------------------------------

/// A page's name, as its title and its bar say it.
fn page_name(starter: &Starter) -> String {
    if starter.showing {
        return if starter.held.screen == faraday_core::Screen::Transfer {
            S.starter_row_transfer
        } else {
            S.starter_row_learn
        }
        .to_string();
    }
    page_name_of(starter, starter.page)
}

/// What page `page` is called, as its title and a way back to it say it.
fn page_name_of(starter: &Starter, page: Page) -> String {
    let target = starter.making.as_ref().map_or(Target::Stick, |m| m.target);
    match page {
        Page::Home => String::new(),
        Page::Make => S.starter_nav_make.to_string(),
        Page::Tools => S.starter_nav_tools.to_string(),
        Page::Which => S.starter_which_title.to_string(),
        Page::Disks => match target {
            Target::Stick => S.starter_disks_stick,
            Target::Pi => S.starter_disks_card,
        }
        .to_string(),
        Page::Mode => S.starter_mode_title.to_string(),
        Page::Confirm => S.starter_confirm_title.to_string(),
        Page::Writing => S.starter_writing_title.to_string(),
        Page::Done => match starter.making.as_ref().and_then(|m| m.result.as_ref()) {
            Some(Ok(())) => S.starter_done_title,
            _ => S.starter_failed_title,
        }
        .to_string(),
        Page::Start => start_label(starter).to_string(),
        Page::Install if starter.update_offered() => S.starter_row_update.to_string(),
        Page::Install => S.starter_row_install.to_string(),
        Page::Installed => S.starter_row_installed.to_string(),
        Page::About => S.starter_row_about.to_string(),
        Page::Appearance => S.starter_row_appearance.to_string(),
        Page::Templates => S.starter_row_templates.to_string(),
        Page::TemplateWords => S.starter_tpl_words.to_string(),
        Page::TemplateCopy => S.starter_tpl_copy.to_string(),
        Page::TemplatePlaces => S.starter_tpl_places.to_string(),
        Page::TemplateForm => S.starter_tpl_form.to_string(),
        Page::TemplateLength => S.starter_tpl_length.to_string(),
        Page::TemplateWorksheets => S.starter_tpl_worksheets.to_string(),
    }
}

/// Where the way back goes, by name; none while a write runs.
fn back_name(starter: &Starter) -> Option<String> {
    if starter.showing {
        // Transfer or a Learn page goes back to the page it was opened
        // from; the Learn destination has none.
        return (!starter.learn_dest).then(|| match starter.page.destination() {
            Some(_) => destination_name(starter.page),
            None => page_name_of(starter, starter.page),
        });
    }
    match starter.page {
        Page::Home | Page::Make | Page::Tools | Page::Writing => None,
        Page::Mode => {
            let target = starter.making.as_ref().map_or(Target::Stick, |m| m.target);
            Some(
                match target {
                    Target::Stick => S.starter_disks_stick,
                    Target::Pi => S.starter_disks_card,
                }
                .to_string(),
            )
        }
        Page::Confirm => Some(S.starter_mode_title.to_string()),
        Page::TemplateWords
        | Page::TemplateCopy
        | Page::TemplatePlaces
        | Page::TemplateForm
        | Page::TemplateLength
        | Page::TemplateWorksheets => Some(S.starter_row_templates.to_string()),
        Page::Disks if starter.asks_which() && target_of(starter) == Target::Stick => {
            Some(S.starter_which_title.to_string())
        }
        _ => Some(destination_name(
            starter.origin.map_or(Page::Home, |(p, _)| p),
        )),
    }
}

/// What a destination's page is called, where a way back names it.
fn destination_name(page: Page) -> String {
    match page {
        Page::Make => S.starter_nav_make,
        Page::Tools => S.starter_nav_tools,
        _ => S.starter_home,
    }
    .to_string()
}

/// A section's label. Returns its height.
fn label(ui: &mut Ui, x: f32, y: f32, s: &str) -> f32 {
    ui.text(x, y, 13.0, W::S, Role::OnSurfaceVariant, s);
    24.0
}

/// A line of fact, wrapped. Returns its height with the gap under it.
fn line(ui: &mut Ui, x: f32, y: f32, w: f32, s: &str, tone: Role) -> f32 {
    ui.wrap(x, y, w, 13.0, W::R, tone, s) + 10.0
}

/// A label and its value: on one row with the value at the right on a
/// wide display, the value under the label on a small panel. Returns
/// its height.
fn value_row(ui: &mut Ui, x: f32, y: f32, w: f32, key: &str, value: &str, tone: Role) -> f32 {
    if ui.compact {
        ui.text(x, y, 12.0, W::R, Role::OnSurfaceVariant, key);
        let vh = ui.wrap(x, y + 18.0, w, 14.0, W::R, tone, value);
        ui.rule(x, y + 24.0 + vh, w, Role::OutlineVariant);
        return vh + 32.0;
    }
    let kw = ui.measure(13.0, W::R, key) + 24.0;
    ui.text_mid(x, y, 36.0, 13.0, W::R, Role::OnSurfaceVariant, key);
    let v = ui.fit(14.0, W::R, value, w - kw);
    ui.text_right(x + w, y, 36.0, 14.0, W::R, tone, &v);
    ui.rule(x, y + 36.0, w, Role::OutlineVariant);
    38.0
}

/// [`value_row`] with the value in on-surface. Returns its height.
fn fact_row(ui: &mut Ui, x: f32, y: f32, w: f32, key: &str, value: &str) -> f32 {
    value_row(ui, x, y, w, key, value, Role::OnSurface)
}

/// A radio mark centred on (x + 12, cy): a ring, filled when chosen, its
/// edge on-surface-variant, or outline-variant where it is not available.
fn radio_mark(ui: &mut Ui, x: f32, cy: f32, on: bool, enabled: bool) {
    if on {
        ui.dot(x + 12.0, cy, 9.0, Role::Primary);
        ui.dot(x + 12.0, cy, 4.0, Role::OnPrimary);
    } else {
        let edge = if enabled {
            Role::OnSurfaceVariant
        } else {
            Role::OutlineVariant
        };
        ui.stroke(x + 3.0, cy - 9.0, 18.0, 18.0, 9.0, edge);
    }
}

/// A label over a long value in the mono face, in groups of four,
/// wrapped (a SHA-256, a fingerprint, a path). Returns its height.
fn mono_row(ui: &mut Ui, x: f32, y: f32, w: f32, key: &str, value: &str) -> f32 {
    ui.text(x, y + 8.0, 12.0, W::R, Role::OnSurfaceVariant, key);
    let vh = ui.wrap(x, y + 28.0, w, 13.0, W::M, Role::OnSurface, value);
    ui.rule(x, y + 36.0 + vh, w, Role::OutlineVariant);
    vh + 40.0
}

/// A row that opens something: an icon, its label, a second line, and a
/// value at the right. Dimmed, it takes no press and its second line is
/// the reason. Returns its height with the gap under it.
#[allow(clippy::too_many_arguments)]
fn nav_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    icon: impl Into<Glyph>,
    head: &str,
    sub: &str,
    tail: &str,
    action: Option<StarterAction>,
) -> f32 {
    let compact = ui.compact;
    let h = if compact { 54.0 } else { 58.0 };
    let enabled = action.is_some();
    let pressed = action.is_some_and(|a| ui.is_pressed(a.host()));
    ui.card(x, y, w, h, Role::OutlineVariant);
    if pressed {
        ui.wash(x, y, w, h, 10.0, Role::OnSurface.at(STATE_PRESS));
    }
    let ib = if compact { 28.0 } else { 32.0 };
    let (ibg, ifg) = if enabled {
        (
            Role::SecondaryContainer.paint(),
            Role::OnSecondaryContainer.paint(),
        )
    } else {
        (
            Role::OnSurface.at(DISABLED_CONTAINER),
            Role::OnSurface.at(DISABLED_CONTENT),
        )
    };
    ui.fill(x + 10.0, y + (h - ib) / 2.0, ib, ib, 8.0, ibg);
    ui.icon(x + 10.0, y + (h - ib) / 2.0, ib, icon, 14.0, ifg);
    let tx = x + 20.0 + ib;
    let tail_w = if tail.is_empty() {
        0.0
    } else {
        ui.measure(12.0, W::R, tail) + 12.0
    };
    let chevron = if enabled { 22.0 } else { 0.0 };
    let room = w - (tx - x) - 12.0 - tail_w - chevron;
    let head = ui.fit(14.0, W::S, head, room);
    let fg = if enabled {
        Role::OnSurface.paint()
    } else {
        Role::OnSurface.at(DISABLED_CONTENT)
    };
    if sub.is_empty() {
        ui.text_mid(tx, y, h, 14.0, W::S, fg, &head);
    } else {
        ui.text(tx, y + 9.0, 14.0, W::S, fg, &head);
        let sub = ui.fit(12.0, W::R, sub, w - (tx - x) - 12.0 - chevron);
        let tone = if enabled {
            Role::OnSurfaceVariant
        } else {
            Role::Warning
        };
        ui.text(tx, y + 30.0, 12.0, W::R, tone, &sub);
    }
    if !tail.is_empty() {
        ui.text_right(
            x + w - 12.0 - chevron,
            y,
            if sub.is_empty() { h } else { 30.0 },
            12.0,
            W::R,
            if enabled {
                Role::OnSurfaceVariant.paint()
            } else {
                Role::OnSurface.at(DISABLED_CONTENT)
            },
            tail,
        );
    }
    if let Some(a) = action {
        ui.icon(
            x + w - 28.0,
            y + (h - 20.0) / 2.0,
            20.0,
            Icon::ChevronRight,
            9.0,
            Role::OnSurfaceVariant,
        );
        ui.hit(x, y, w, h, a.host());
    }
    h + 8.0
}

/// A warning: a band in warning-container with the line in it, in
/// on-warning-container. Returns its height with the gap under it.
fn warning(ui: &mut Ui, x: f32, y: f32, w: f32, s: &str) -> f32 {
    // Measured first, where nothing shows: the band needs the text's height.
    ui.c.push_clip(faraday_draw::Rect::new(0, 0, 0, 0));
    let th = ui.wrap(
        x + 14.0,
        y + 10.0,
        w - 28.0,
        13.0,
        W::S,
        Role::OnWarningContainer,
        s,
    );
    ui.c.pop_clip();
    let bh = th + 20.0;
    ui.fill(x, y, w, bh, 10.0, Role::WarningContainer);
    ui.stroke(x, y, w, bh, 10.0, Role::Warning.with_alpha(110));
    ui.wrap(
        x + 14.0,
        y + 10.0,
        w - 28.0,
        13.0,
        W::S,
        Role::OnWarningContainer,
        s,
    );
    bh + 12.0
}

/// A link to a Learn page: a line in the accent colour that opens it,
/// drawn only when the page exists (another batch writes some of them).
/// Returns its height, nothing when the page does not exist.
pub(crate) fn link(ui: &mut Ui, x: f32, y: f32, w: f32, id: LearnId) -> f32 {
    if id.page().is_none() {
        return 0.0;
    }
    let label = learn_title(id);
    let action = StarterAction::LearnPage(id).host();
    let label = ui.fit(13.0, W::S, label, w);
    let tw = ui.measure(13.0, W::S, &label);
    let tone = if ui.is_pressed(action) || ui.is_hovered(action) {
        Role::OnSurface
    } else {
        Role::Primary
    };
    ui.text_mid(x, y, 28.0, 13.0, W::S, tone, &label);
    ui.hit(x - 4.0, y, tw + 8.0, 28.0, action);
    32.0
}

/// An item of concern's line (§3.1a), naming it.
pub(crate) fn concern_text(c: &Concern) -> String {
    let fallback = match c.kind {
        Kind::Tv => S.starter_concern_screen,
        Kind::Kvm | Kind::Receiver | Kind::BoltReceiver => S.starter_concern_device,
        Kind::VirtualMachine => S.starter_concern_computer,
    };
    let name = if c.name.trim().is_empty() {
        fallback
    } else {
        c.name.trim()
    };
    let line = match c.kind {
        Kind::Tv => S.starter_concern_tv,
        Kind::Kvm => S.starter_concern_kvm,
        Kind::Receiver => S.devices_receiver_notice,
        Kind::BoltReceiver => S.devices_bolt_notice,
        Kind::VirtualMachine => S.starter_concern_vm,
    };
    fill(line, &[name])
}

/// A wireless receiver's line (§3.1a), which holds nothing: the warning,
/// or a Bolt receiver's plain line. Returns its height with the gap
/// under it; nothing without a receiver.
fn receiver_line(starter: &Starter, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    match starter.receiver_line() {
        Some(l) if l == S.devices_bolt_notice => line(ui, x, y, w, l, Role::OnSurfaceVariant),
        Some(l) => warning(ui, x, y, w, l),
        None => 0.0,
    }
}

/// What is attached that holds a step (§3.1a): each item's line, its
/// Continue anyway (never for a virtual machine), Check again, and the
/// Learn page. Returns its height with the gap under it; nothing when
/// nothing holds.
fn concern_block(
    starter: &Starter,
    ui: &mut Ui,
    x: f32,
    y0: f32,
    w: f32,
    items: &[&Concern],
) -> f32 {
    if items.is_empty() {
        return 0.0;
    }
    let mut y = y0;
    for c in items {
        y += warning(ui, x, y, w, &concern_text(c));
        if c.kind != Kind::VirtualMachine {
            let n = StarterAction::ContinueAnyway(starter.concern_number(&c.id));
            y += buttons(
                ui,
                x,
                y - 4.0,
                w,
                &[(S.starter_continue_anyway, Style::Ghost, n)],
            ) + 8.0;
        }
    }
    y += buttons(
        ui,
        x,
        y,
        w,
        &[(
            S.starter_check_again,
            Style::Secondary,
            StarterAction::CheckAgain,
        )],
    ) + 8.0;
    y += link(ui, x, y, w, LearnId::ScreensThatRecord);
    y + 12.0 - y0
}

/// One button, or a row of them (a column on a small panel when they do
/// not fit). Returns their height.
fn buttons(ui: &mut Ui, x: f32, y: f32, w: f32, items: &[(&str, Style, StarterAction)]) -> f32 {
    if items.is_empty() {
        return 0.0;
    }
    if ui.compact {
        let n = items.len() as f32;
        let share = (w - (n - 1.0) * 8.0) / n;
        let fits = items
            .iter()
            .all(|(l, ..)| ui.measure(15.0, W::S, l) + 24.0 <= share);
        if fits {
            for (i, &(l, s, a)) in items.iter().enumerate() {
                ui.button(
                    x + i as f32 * (share + 8.0),
                    y,
                    Some(share),
                    46.0,
                    l,
                    s,
                    a.host(),
                );
            }
            return 46.0;
        }
        for (i, &(l, s, a)) in items.iter().enumerate() {
            let l = ui.fit(15.0, W::S, l, w - 24.0);
            ui.button(x, y + i as f32 * 54.0, Some(w), 46.0, &l, s, a.host());
        }
        return n * 54.0 - 8.0;
    }
    let mut bx = x;
    for &(l, s, a) in items {
        bx += ui.button(bx, y, None, 44.0, l, s, a.host()) + 10.0;
    }
    44.0
}

/// A numbered step's heading, or a tick once it is done. Returns its
/// height.
fn step_head(ui: &mut Ui, x: f32, y: f32, w: f32, n: usize, text: &str, done: bool) -> f32 {
    ui.badge(x, y, &n.to_string(), done, !done);
    let th = ui.wrap(
        x + 38.0,
        y + 3.0,
        w - 38.0,
        15.0,
        W::S,
        Role::OnSurface,
        text,
    );
    th.max(26.0) + 10.0
}

/// What a reason reads as, from `faraday_strings`.
pub(crate) fn reason_text(r: &Reason) -> String {
    let n = |v: u64| v.to_string();
    let bytes = |v: u64| fill(S.starter_why_bytes, &[&v.to_string()]);
    match r {
        Reason::NoWriter => S.starter_no_writer.to_string(),
        Reason::NotCarried => S.starter_why_not_carried.to_string(),
        Reason::Changed {
            device,
            serial,
            size,
        } => {
            let serial = if serial.is_empty() {
                S.starter_why_no_serial
            } else {
                serial
            };
            fill(S.starter_why_changed, &[device, serial, &bytes(*size)])
        }
        Reason::InUse(InUse::Mounted(at)) => fill(S.starter_why_mounted, &[at]),
        Reason::InUse(InUse::Swap) => S.starter_why_swap.to_string(),
        Reason::InUse(InUse::System) => S.starter_why_system.to_string(),
        Reason::BootOnly => S.starter_why_boot_only.to_string(),
        Reason::TooSmall { disk, image } => {
            fill(S.starter_why_too_small, &[&bytes(*disk), &bytes(*image)])
        }
        Reason::PayloadSum => S.starter_why_payload_sum.to_string(),
        Reason::ImageChanged { path, sha256 } => fill(S.starter_why_image_changed, &[path, sha256]),
        Reason::Gone => S.starter_why_gone.to_string(),
        Reason::Bus(d) => fill(S.starter_why_bus, &[d]),
        Reason::Unmounting(d) => fill(S.starter_why_unmounting, &[d]),
        Reason::OpenWrite(d) => fill(S.starter_why_open_write, &[d]),
        Reason::OpenRead(d) => fill(S.starter_why_open_read, &[d]),
        Reason::Device { device, detail } => fill(S.starter_why_device, &[device, detail]),
        Reason::Flush { device, detail } => fill(S.starter_why_flush, &[device, detail]),
        Reason::NotStick { device, fault } => {
            fill(S.starter_why_not_stick, &[device, &fault_text(fault)])
        }
        Reason::NotStickImage(fault) => fill(S.starter_why_not_stick_image, &[&fault_text(fault)]),
        Reason::BootLarger { image, stick } => {
            fill(S.starter_why_boot_larger, &[&bytes(*image), &bytes(*stick)])
        }
        Reason::BootLabel => S.starter_why_boot_label.to_string(),
        Reason::BootShort => S.starter_why_boot_short.to_string(),
        Reason::TableChanged(device) => fill(S.starter_why_table_changed, &[device]),
        Reason::ReadingTable(d) => fill(S.starter_why_reading_table, &[d]),
        Reason::BackEnds { device, at } => fill(S.starter_why_back_ends, &[device, &n(*at)]),
        Reason::BackDiffers { device, at } => fill(S.starter_why_back_differs, &[device, &n(*at)]),
        Reason::BackFailed { device, detail } => fill(S.starter_why_back_failed, &[device, detail]),
        Reason::SourceRead(d) => fill(S.starter_why_source_read, &[d]),
        Reason::WriteFailed(d) => fill(S.starter_why_write_failed, &[d]),
        Reason::PayloadChanged => S.starter_why_payload_changed.to_string(),
        Reason::Damaged(d) => fill(S.starter_why_damaged, &[d]),
        Reason::Longer(v) => fill(S.starter_why_longer, &[&n(*v)]),
        Reason::Shorter { expected, actual } => {
            fill(S.starter_why_shorter, &[&n(*actual), &n(*expected)])
        }
        Reason::WrongSum(sum) => fill(S.starter_why_wrong_sum, &[sum]),
        Reason::NoHome => S.starter_why_no_home.to_string(),
        Reason::NoDesktopApp => S.starter_why_no_desktop.to_string(),
        Reason::CacheSum(sum) => fill(S.starter_why_cache_sum, &[sum]),
        Reason::File { path, detail } => fill(S.starter_why_file, &[path, detail]),
        Reason::InTheWay { path } => fill(S.starter_why_in_the_way, &[path]),
        Reason::Start { path, detail } => fill(S.starter_why_start, &[path, detail]),
        Reason::Restart(d) => fill(S.starter_why_restart, &[d]),
        Reason::Missing(program) => fill(S.starter_why_missing, &[program]),
        Reason::BootEntry(d) => fill(S.starter_why_boot_entry, &[d]),
        Reason::NoStick => S.starter_why_no_stick.to_string(),
        Reason::SameBuild => S.starter_why_same_build.to_string(),
        Reason::Declined => S.starter_why_declined.to_string(),
        Reason::Unsupported => S.starter_why_unsupported.to_string(),
        Reason::NoAdmin => S.starter_why_no_admin.to_string(),
        Reason::Managed(code) => fill(S.starter_why_managed, &[&format!("0x{code:08X}")]),
        Reason::BitLocker { call, code } => {
            fill(S.starter_why_bitlocker, &[call, &format!("0x{code:08X}")])
        }
        Reason::NotEncrypted => S.starter_why_not_encrypted.to_string(),
        Reason::NotRead => S.starter_why_not_read.to_string(),
        Reason::NoRecoveryKey => S.starter_why_no_recovery_key.to_string(),
    }
}

fn fault_text(f: &StickFault) -> String {
    let bytes = |v: u64| fill(S.starter_why_bytes, &[&v.to_string()]);
    match f {
        StickFault::Reading(d) => fill(S.starter_fault_reading, &[d]),
        StickFault::NoHeader => S.starter_fault_no_header.to_string(),
        StickFault::NotGpt => S.starter_fault_not_gpt.to_string(),
        StickFault::HeaderDamaged => S.starter_fault_header_damaged.to_string(),
        StickFault::HeaderChecksum => S.starter_fault_header_checksum.to_string(),
        StickFault::EntriesDamaged => S.starter_fault_entries_damaged.to_string(),
        StickFault::EntriesPastEnd => S.starter_fault_entries_past_end.to_string(),
        StickFault::EntriesChecksum => S.starter_fault_entries_checksum.to_string(),
        StickFault::DiskUuid => S.starter_fault_disk_uuid.to_string(),
        StickFault::Partitions(n) => fill(S.starter_fault_partitions, &[&n.to_string()]),
        StickFault::BootName => S.starter_fault_boot_name.to_string(),
        StickFault::BootType => S.starter_fault_boot_type.to_string(),
        StickFault::DataType => S.starter_fault_data_type.to_string(),
        StickFault::BootDamaged => S.starter_fault_boot_damaged.to_string(),
        StickFault::BootSmall { boot, image } => {
            fill(S.starter_fault_boot_small, &[&bytes(*boot), &bytes(*image)])
        }
        StickFault::Overlap => S.starter_fault_overlap.to_string(),
        StickFault::BootPastEnd => S.starter_fault_boot_past_end.to_string(),
    }
}

/// A size as people read it: "16 GB", "7.8 GB", "512 MB".
pub(crate) fn size_text(bytes: u64) -> String {
    let gb = bytes as f64 / 1e9;
    if gb >= 10.0 {
        fill(S.starter_size_gb, &[&format!("{gb:.0}")])
    } else if gb >= 1.0 {
        fill(S.starter_size_gb, &[&format!("{gb:.1}")])
    } else {
        fill(S.starter_size_mb, &[&format!("{:.0}", bytes as f64 / 1e6)])
    }
}

/// A SHA-256 in groups of four.
fn grouped(hex: &str) -> String {
    faraday_ui::grouped(hex)
}

/// A disk's name: maker and model.
fn disk_name(d: &crate::api::Disk) -> String {
    let name = format!("{} {}", d.vendor.trim(), d.model.trim());
    let name = name.trim();
    if name.is_empty() {
        S.starter_disk_unnamed.to_string()
    } else {
        name.to_string()
    }
}

/// What is on a disk, in a few words (§3.2 step 1).
fn on_it(d: &crate::api::Disk) -> String {
    if let Some(f) = &d.faraday {
        return fill(S.starter_on_faraday, &[&crate::decide::version_of(f)]);
    }
    if d.volumes.is_empty() {
        return S.starter_on_empty.to_string();
    }
    let names: Vec<&str> = d
        .volumes
        .iter()
        .filter_map(|v| v.label.as_deref())
        .collect();
    let n = d.volumes.len();
    let what = if names.is_empty() {
        if n == 1 {
            S.starter_on_volume.to_string()
        } else {
            fill(S.starter_on_volumes, &[&n.to_string()])
        }
    } else {
        names.join(", ")
    };
    let files = d.volumes.iter().any(|v| v.files);
    let tail = match (files, n == 1) {
        (true, true) => S.starter_volume_has_files,
        (true, false) => S.starter_on_files_them,
        (false, _) => S.starter_on_no_files,
    };
    format!("{what}, {tail}")
}

/// Why a disk cannot be chosen, in one line.
fn refusal_text(r: &Refusal) -> String {
    match r {
        Refusal::InUse(InUse::Mounted(at)) => fill(S.starter_refused_mounted, &[at]),
        Refusal::InUse(InUse::Swap) => S.starter_refused_swap.to_string(),
        Refusal::InUse(InUse::System) => S.starter_refused_system.to_string(),
        Refusal::TooSmall(needs) => fill(S.starter_refused_small, &[&size_text(*needs)]),
    }
}

/// The line Confirm shows when the disk is no longer the one confirmed.
pub(crate) fn not_confirmed(target: Target) -> String {
    match target {
        Target::Stick => S.starter_not_confirmed_stick,
        Target::Pi => S.starter_not_confirmed_card,
    }
    .to_string()
}

// ---------------------------------------------------------------------
// Home
// ---------------------------------------------------------------------

/// Start this computer from the stick, or on a Mac, Start this Mac
/// from the stick.
fn start_label(starter: &Starter) -> &'static str {
    if starter.facts().mac().is_some() {
        S.starter_item_start_mac
    } else {
        S.starter_item_start
    }
}

/// What an item is called.
fn item_label(starter: &Starter, item: Item) -> &'static str {
    match item {
        Item::MakeStick => S.starter_item_make,
        Item::Start => start_label(starter),
        Item::SecureBootOn => S.starter_item_secure_boot,
    }
}

/// What a state reads as.
fn state_text(state: &State) -> String {
    match state {
        State::NoStick => S.starter_state_no_stick.to_string(),
        State::Stick(v) => fill(S.starter_state_stick, &[v]),
        State::SecureBoot(on) => secure_boot_text(*on),
        State::MacStart(Some(false)) => S.starter_state_mac_option.to_string(),
        State::MacStart(Some(true)) => S.starter_state_mac_t2.to_string(),
        State::MacStart(None) => S.starter_state_mac_unknown.to_string(),
    }
}

fn secure_boot_text(on: Option<bool>) -> String {
    match on {
        Some(true) => S.starter_secure_boot_enabled,
        Some(false) => S.starter_secure_boot_off,
        None => S.starter_secure_boot_unknown,
    }
    .to_string()
}

// ---------------------------------------------------------------------
// Make a Faraday stick, make a Pi card
// ---------------------------------------------------------------------

fn target_of(starter: &Starter) -> Target {
    starter.making.as_ref().map_or(Target::Stick, |m| m.target)
}

/// For which computer? (§3.2 step 0): the two stick images, nothing
/// chosen, the row that fits this computer tagged.
fn which(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    let fits = macs::stick_for(&starter.computer);
    for (k, head) in [
        (StickFor::Pc, S.starter_which_pc),
        (StickFor::AppleSilicon, S.starter_which_apple),
    ] {
        let tag = if fits == Some(k) {
            S.starter_computer
        } else {
            ""
        };
        y += nav_row(
            ui,
            x,
            y,
            w,
            Glyph::Stick,
            head,
            tag,
            "",
            Some(StarterAction::StickFor(k)),
        );
    }
    y
}

fn disks(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    let target = target_of(starter);
    if target == Target::Pi {
        y += line(
            ui,
            x,
            y,
            w,
            S.starter_pi_supported_boards,
            Role::OnSurfaceVariant,
        );
        y += line(ui, x, y, w, S.starter_pi_cameras, Role::OnSurfaceVariant);
    }
    if starter.image(target).is_none() {
        let s = match target {
            Target::Stick => S.starter_no_stick_image,
            Target::Pi => S.starter_no_pi_image,
        };
        return y + line(ui, x, y, w, s, Role::Error);
    }
    if starter.no_writer {
        y += line(ui, x, y, w, S.starter_no_writer, Role::Warning);
    }
    let offers = starter.offers();
    if offers.is_empty() {
        let s = match target {
            Target::Stick => S.starter_no_disks_stick,
            Target::Pi => S.starter_no_disks_card,
        };
        return y + line(ui, x, y + 8.0, w, s, Role::OnSurfaceVariant);
    }
    y += 4.0;
    for o in &offers {
        let d = o.disk;
        // A stick's glyph for a USB disk, the SD card's for the
        // computer's reader, the drive's for anything else
        // (`docs/BACKUP-FLOW.md` §17.17 item 4).
        let icon: Glyph = match d.bus {
            crate::api::Bus::Usb => Glyph::Stick,
            crate::api::Bus::SdReader => Icon::SdCard.into(),
            crate::api::Bus::Other => Icon::Drive.into(),
        };
        let (sub, action) = match &o.refused {
            Some(r) => (refusal_text(r), None),
            None if starter.no_writer => (on_it(d), None),
            None => (
                on_it(d),
                Some(StarterAction::Disk(starter.disk_number(&d.id))),
            ),
        };
        y += nav_row(
            ui,
            x,
            y,
            w,
            icon,
            &disk_name(d),
            &sub,
            &size_text(d.size),
            action,
        );
    }
    y
}

fn mode(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    let target = target_of(starter);
    if let Some(d) = starter.chosen_disk() {
        let s = format!("{} · {}", disk_name(d), size_text(d.size));
        y += line(ui, x, y, w, &s, Role::OnSurfaceVariant);
    }
    y += 4.0;
    let chosen = starter.making.as_ref().and_then(|m| m.mode);
    let whole = match target {
        Target::Stick => S.starter_mode_whole_stick,
        Target::Pi => S.starter_mode_whole_card,
    };
    let options = [
        (Mode::Whole, whole, S.starter_mode_whole_sub),
        (Mode::KeepData, S.starter_mode_keep, S.starter_mode_keep_sub),
    ];
    ui.rule(x, y, w, Role::OutlineVariant);
    for (m, head, sub) in options {
        let refused = starter.mode_refusal(m);
        // Keep data is shown only on a Faraday stick (§3.2).
        if m == Mode::KeepData
            && matches!(refused, Some(ModeRefusal::NotFaraday | ModeRefusal::NotYet))
        {
            continue;
        }
        let action = StarterAction::Mode(m).host();
        let on = chosen == Some(m);
        let enabled = refused.is_none();
        let rh = 60.0;
        if enabled {
            ui.state_layer(
                (x - 6.0, y + 2.0, w + 12.0, rh - 4.0, 8.0),
                Role::OnSurface.paint(),
                action,
            );
        }
        // The check: a ring, filled when chosen.
        let cy = y + rh / 2.0;
        radio_mark(ui, x, cy, on, enabled);
        let tx = x + 36.0;
        ui.text(
            tx,
            y + 10.0,
            15.0,
            W::S,
            if enabled {
                Role::OnSurface.paint()
            } else {
                Role::OnSurface.at(DISABLED_CONTENT)
            },
            head,
        );
        let (sub, tone) = match refused {
            Some(ModeRefusal::Same) => (S.starter_mode_same, Role::Warning),
            Some(ModeRefusal::BootOnly) => (S.starter_mode_boot_only, Role::Warning),
            None if m == Mode::KeepData && starter.version_reading.is_some() => {
                (S.starter_mode_reading, Role::OnSurfaceVariant)
            }
            _ => (sub, Role::OnSurfaceVariant),
        };
        let sub = ui.fit(12.0, W::R, sub, w - 40.0);
        ui.text(tx, y + 34.0, 12.0, W::R, tone, &sub);
        if enabled {
            ui.hit(x - 6.0, y, w + 12.0, rh, action);
        }
        y += rh;
        // The one fact the Learn page explains (§4.4).
        if m == Mode::KeepData {
            y += ui.wrap(
                tx,
                y,
                w - 36.0,
                12.0,
                W::R,
                Role::OnSurfaceVariant,
                S.starter_mode_keep_fact,
            ) + 6.0;
            y += guide_line(ui, tx, y, w - 36.0, LearnId::KeepData);
        }
        ui.rule(x, y, w, Role::OutlineVariant);
    }
    if let Some(why) = &starter.version_failed {
        y += 12.0;
        y += line(ui, x, y, w, &reason_text(why), Role::Error);
    }
    y += 20.0;
    let style = if chosen.is_some() && !starter.keep_unread() {
        Style::Primary
    } else {
        Style::Disabled
    };
    y + buttons(
        ui,
        x,
        y,
        w,
        &[(S.starter_continue, style, StarterAction::Continue)],
    )
}

fn confirm(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    let Some(m) = starter.making.as_ref() else {
        return y;
    };
    let target = m.target;
    let Some(image) = starter.image(target) else {
        return y;
    };
    let Some((id, device, serial, size)) = m.confirmed.as_ref() else {
        return y;
    };
    let mode = m.mode.unwrap_or(Mode::Whole);
    let disk = starter.disks.iter().find(|d| &d.id == id);
    let noun = match target {
        Target::Stick => S.starter_stick,
        Target::Pi => S.starter_card,
    };
    if let Some(d) = disk {
        y += fact_row(ui, x, y, w, noun, &disk_name(d));
    }
    y += fact_row(ui, x, y, w, S.starter_size, &size_text(*size));
    y += fact_row(ui, x, y, w, S.starter_device, device);
    let serial = if serial.is_empty() {
        S.starter_serial_none
    } else {
        serial.as_str()
    };
    y += fact_row(ui, x, y, w, S.starter_serial, serial);
    if let Some(d) = disk {
        y += fact_row(ui, x, y, w, S.starter_disk_contents_row, &on_it(d));
    }
    let writes = match (mode, target) {
        (Mode::KeepData, _) => S.starter_mode_keep,
        (Mode::Whole, Target::Stick) => S.starter_mode_whole_stick,
        (Mode::Whole, Target::Pi) => S.starter_mode_whole_card,
    };
    y += fact_row(ui, x, y, w, S.starter_writes, writes);
    let build = image
        .build
        .as_deref()
        .map_or_else(|| S.starter_build_unknown.to_string(), describe);
    y += fact_row(ui, x, y, w, S.starter_build, &build);
    let sha_key = if image.dev {
        S.starter_image_sha
    } else {
        S.starter_img_sha
    };
    y += mono_row(ui, x, y, w, sha_key, &grouped(&image.sha256));
    y += 12.0;
    if image.dev {
        y += warning(ui, x, y, w, S.starter_dev_image);
    }
    if mode == Mode::KeepData
        && let Some(stick) = disk.and_then(|d| d.faraday.as_ref())
    {
        match crate::decide::keep_data(stick, image.build.as_deref()) {
            Fit::Newer => {
                let s = fill(S.starter_keep_newer, &[&version(stick.release.as_deref())]);
                y += warning(ui, x, y, w, &s);
            }
            Fit::Dev => y += warning(ui, x, y, w, S.starter_keep_dev),
            _ => {}
        }
    }
    let matches = starter.confirmed_matches();
    if !matches {
        y += line(ui, x, y, w, &not_confirmed(target), Role::Error);
    } else if let Some(why) = &m.refused {
        y += line(ui, x, y, w, why, Role::Error);
    }
    if starter.no_writer {
        y += line(ui, x, y, w, S.starter_no_writer, Role::Warning);
    }
    let label = match mode {
        Mode::Whole => S.starter_erase_and_write,
        Mode::KeepData => S.starter_write,
    };
    let style = if matches && !starter.no_writer {
        Style::Primary
    } else {
        Style::Disabled
    };
    y += 4.0;
    if !matches {
        // The disk is gone or changed: the only way on is to choose again.
        return y + buttons(
            ui,
            x,
            y,
            w,
            &[
                (S.starter_choose_again, Style::Primary, StarterAction::Again),
                (label, style, StarterAction::Write),
            ],
        );
    }
    y + buttons(ui, x, y, w, &[(label, style, StarterAction::Write)])
}

fn writing(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    let Some(m) = starter.making.as_ref() else {
        return y;
    };
    if let Some(d) = m
        .confirmed
        .as_ref()
        .and_then(|c| starter.disks.iter().find(|d| d.id == c.0))
    {
        let s = format!("{} · {}", disk_name(d), size_text(d.size));
        y += line(ui, x, y, w, &s, Role::OnSurfaceVariant);
    }
    let (stage, done, total) = m.progress.unwrap_or((WriteStage::Writing, 0, 0));
    y += GAP;
    // Material's linear indicator: the bytes written (or read back) of
    // the image's; before the writer has said how many, still.
    let t = (total > 0).then(|| done as f32 / total as f32);
    ui.progress((x, y, w), t, None);
    y += PROGRESS_H + 2.0 * GAP;
    let of = fill(
        S.starter_progress_bytes,
        &[&size_text(done), &size_text(total)],
    );
    let s = match stage {
        WriteStage::Writing => fill(S.starter_progress_writing, &[&of]),
        WriteStage::ReadingBack => fill(S.starter_reading_back, &[&of]),
    };
    y + line(ui, x, y, w, &s, Role::OnSurface)
}

fn done(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    let Some(m) = starter.making.as_ref() else {
        return y;
    };
    let ok = matches!(m.result, Some(Ok(())));
    let (icon, tone) = if ok {
        (Icon::Success, Role::Ok)
    } else {
        (Icon::Error, Role::Error)
    };
    ui.icon(x - 4.0, y, 40.0, icon, 26.0, tone);
    let name = page_name(starter);
    ui.text_mid(x + 44.0, y, 40.0, 22.0, W::S, tone, &name);
    y += 56.0;
    match &m.result {
        Some(Ok(())) => {
            let s = match m.target {
                Target::Stick => S.starter_pull_stick,
                Target::Pi => S.starter_pull_card,
            };
            y += line(ui, x, y, w, s, Role::OnSurface) + 8.0;
            if m.target == Target::Stick
                && let Some(disk) = starter.chosen_disk()
                && let Some(entry) =
                    crate::decide::boot_menu_entry(&disk.vendor, &disk.model, &starter.computer)
            {
                y += value_row(
                    ui,
                    x,
                    y,
                    w,
                    S.starter_boot_menu_label,
                    &fill(S.starter_boot_menu_value, &[&entry]),
                    Role::OnSurface,
                ) + 8.0;
            }
            let items: &[(&str, Style, StarterAction)] = match m.target {
                Target::Stick => &[
                    (start_label(starter), Style::Primary, StarterAction::Start),
                    (S.starter_done, Style::Secondary, StarterAction::Home),
                ],
                Target::Pi => &[(S.starter_done, Style::Primary, StarterAction::Home)],
            };
            y += buttons(ui, x, y, w, items);
        }
        Some(Err(why)) => {
            y += line(ui, x, y, w, &reason_text(why), Role::Error) + 8.0;
            y += buttons(
                ui,
                x,
                y,
                w,
                &[
                    (S.starter_try_again, Style::Primary, StarterAction::Again),
                    (S.starter_done, Style::Secondary, StarterAction::Home),
                ],
            );
        }
        None => {}
    }
    y
}

// ---------------------------------------------------------------------
// Start this computer from the stick
// ---------------------------------------------------------------------

/// The guide's line: a row that opens its Learn page, or until batch G
/// writes it, its title as a line.
fn guide_line(ui: &mut Ui, x: f32, y: f32, w: f32, id: LearnId) -> f32 {
    let title = learn_title(id);
    match id.page() {
        Some(_) => nav_row(
            ui,
            x,
            y,
            w,
            Icon::Learn,
            title,
            "",
            "",
            Some(StarterAction::LearnPage(id)),
        ),
        None => line(
            ui,
            x,
            y,
            w,
            &fill(S.starter_guide_placeholder, &[title]),
            Role::OnSurfaceVariant,
        ),
    }
}

/// A guide's title, as its Learn page will carry it; a maker page's is
/// its page's own.
fn learn_title(id: LearnId) -> &'static str {
    match id {
        LearnId::StartWindows => S.starter_learn_start_windows,
        LearnId::StartLinux => S.starter_learn_start_linux,
        LearnId::NoStart => S.starter_learn_no_start,
        LearnId::KeepData => S.starter_learn_keep_data,
        LearnId::Maker(_) => id.page().map_or("", |p| p.title),
        LearnId::SecureBootBitLocker => S.starter_learn_secure_boot_bitlocker,
        LearnId::ScreensThatRecord => S.starter_learn_screens,
        LearnId::WhatElseRuns => S.starter_learn_what_else_runs,
        LearnId::BootEntry => S.starter_learn_boot_entry,
        LearnId::MacIntel => S.starter_learn_mac_intel,
    }
}

#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
fn start(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    let c = &starter.computer;
    let facts = starter.facts();
    let windows = c.os == Os::Windows;
    if c.tpm_unlock {
        y += warning(ui, x, y, w, S.starter_tpm_line);
    }
    if let Some(mac) = facts.mac() {
        return mac_start(starter, ui, x, y, w, mac);
    }
    if c.arm {
        return y + line(ui, x, y, w, S.starter_cannot_start_arm, Role::Warning);
    }
    if !c.uefi {
        y += warning(ui, x, y, w, S.starter_cannot_start_bios);
    }
    let held = starter.restart_holds();
    y += concern_block(starter, ui, x, y, w, &held);
    y += receiver_line(starter, ui, x, y, w);
    let guide = LearnId::for_computer(c);
    y += guide_line(ui, x, y, w, guide);
    y += link(ui, x, y, w, LearnId::SecureBootBitLocker);
    y += 8.0;
    let ix = x + 38.0;
    let iw = w - 38.0;
    let mut n = 0;
    let open = facts.protected_open();
    if open && let Some(name) = &starter.other {
        y += warning(ui, x, y, w, &fill(S.starter_other_found, &[name]));
        y += buttons(
            ui,
            x,
            y,
            w,
            &[(
                S.starter_other_confirm,
                Style::Secondary,
                StarterAction::Rescue,
            )],
        ) + 20.0;
    }
    y += notes_block(starter, ui, x, y, w);
    y += bitlocker_block(starter, ui, x, y, w);
    // 1. Restart into firmware settings.
    n += 1;
    y += step_head(ui, x, y, w, n, S.starter_restart, false);
    if let Some(why) = &starter.restart_failed {
        y += line(ui, ix, y, iw, &reason_text(why), Role::Error);
    }
    let style = if open || !held.is_empty() {
        Style::Disabled
    } else {
        Style::Secondary
    };
    y += buttons(
        ui,
        ix,
        y,
        iw,
        &[(S.starter_restart, style, StarterAction::Restart)],
    ) + 16.0;
    // 2. The firmware steps.
    n += 1;
    let sb_off = c.secure_boot == Some(false);
    let head = if sb_off {
        S.starter_secure_boot_already_off
    } else {
        S.starter_secure_boot_turn_off
    };
    y += step_head(ui, x, y, w, n, head, sb_off);
    y += line(ui, ix, y, iw, S.starter_fast_boot, Role::OnSurfaceVariant);
    y += line(
        ui,
        ix,
        y,
        iw,
        notes::not_found(c.os),
        Role::OnSurfaceVariant,
    ) + 6.0;
    // 3. Start from the stick.
    n += 1;
    y += step_head(ui, x, y, w, n, S.starter_boot_from_stick, false);
    // Use a device is reached from Windows, and is offered only while
    // Windows reports Secure Boot off (F33): the boot menu key otherwise.
    let how = if facts.use_a_device() {
        S.starter_boot_how_windows
    } else {
        S.starter_boot_how
    };
    y += line(ui, ix, y, iw, how, Role::OnSurfaceVariant) + 6.0;
    if facts.use_a_device() && facts.stick().is_some() {
        let style = if open || !held.is_empty() {
            Style::Disabled
        } else {
            Style::Secondary
        };
        y += buttons(
            ui,
            ix,
            y,
            iw,
            &[(
                S.starter_restart_choose_device,
                style,
                StarterAction::RestartStick,
            )],
        ) + 12.0;
    }
    if let Some(stick) = facts.stick()
        && let Some(entry) = crate::decide::boot_menu_entry(&stick.vendor, &stick.model, c)
    {
        y += value_row(
            ui,
            ix,
            y,
            iw,
            S.starter_boot_menu_label,
            &fill(S.starter_boot_menu_value, &[&entry]),
            Role::OnSurface,
        ) + 8.0;
    }
    // 4. Afterwards: the last step, shown always (F33).
    n += 1;
    let s = if windows {
        S.starter_afterwards_windows
    } else {
        S.starter_afterwards
    };
    y += step_head(ui, x, y, w, n, s, false) + 6.0;
    // The same page on a phone.
    y += 16.0;
    y += label(ui, x, y, S.starter_on_a_phone);
    let side = if ui.compact { 150.0 } else { 180.0 };
    y + phone_code(starter, ui, x, y, w, side, &guide.address())
}

/// The Start page on a Mac (§3.4, `docs/MAC.md` §3.1–§3.2): the Mac's
/// steps, none of them a button. On a T2 Mac, Startup Security lowered
/// in macOS Recovery; then Option at power-on and EFI Boot; on a T2 Mac,
/// Startup Security back afterwards. A Mac the table does not know gets
/// the T2 steps marked as for a T2 Mac only..
fn mac_start(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32, mac: macs::Start) -> f32 {
    let mut y = y0;
    let t2 = match mac {
        macs::Start::Intel { t2 } => t2,
        macs::Start::Firmware32 => {
            return y + line(
                ui,
                x,
                y,
                w,
                S.starter_cannot_start_mac_firmware,
                Role::Warning,
            );
        }
        macs::Start::NotYet => {
            return y + line(ui, x, y, w, S.starter_cannot_start_mac_yet, Role::Warning);
        }
    };
    let held = starter.restart_holds();
    y += concern_block(starter, ui, x, y, w, &held);
    y += receiver_line(starter, ui, x, y, w);
    let guide = LearnId::MacIntel;
    y += guide_line(ui, x, y, w, guide);
    y += 8.0;
    y += notes_block(starter, ui, x, y, w);
    let ix = x + 38.0;
    let iw = w - 38.0;
    let mut n = 0;
    let t2_steps = t2 != Some(false);
    if t2_steps {
        n += 1;
        y += step_head(ui, x, y, w, n, S.starter_mac_lower, false);
        if t2.is_none() {
            y += line(ui, ix, y, iw, S.starter_mac_t2_only, Role::Warning);
        }
        y += line(
            ui,
            ix,
            y,
            iw,
            S.starter_mac_recovery,
            Role::OnSurfaceVariant,
        );
        y += line(
            ui,
            ix,
            y,
            iw,
            S.starter_mac_utility_lower,
            Role::OnSurfaceVariant,
        ) + 6.0;
    }
    n += 1;
    y += step_head(ui, x, y, w, n, S.starter_boot_from_stick, false);
    y += line(ui, ix, y, iw, S.starter_mac_option, Role::OnSurfaceVariant);
    y += line(
        ui,
        ix,
        y,
        iw,
        S.starter_mac_firmware_password,
        Role::OnSurfaceVariant,
    ) + 6.0;
    if t2_steps {
        n += 1;
        y += step_head(ui, x, y, w, n, S.starter_mac_afterwards, false);
        if t2.is_none() {
            y += line(ui, ix, y, iw, S.starter_mac_t2_only, Role::Warning);
        }
        y += line(
            ui,
            ix,
            y,
            iw,
            S.starter_mac_utility_full,
            Role::OnSurfaceVariant,
        ) + 6.0;
    }
    y += 16.0;
    y += label(ui, x, y, S.starter_on_a_phone);
    let side = if ui.compact { 150.0 } else { 180.0 };
    y + phone_code(starter, ui, x, y, w, side, &guide.address())
}

/// What to write down before the restart (§3.4, F47): the three rows,
/// a value not known read as such, then Print. Returns its height.
fn notes_block(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let Some(notes) = notes::Notes::for_computer(&starter.computer) else {
        return 0.0;
    };
    let mut y = y0;
    y += label(ui, x, y, S.starter_notes_label);
    for row in &notes.rows {
        let (value, tone) = match row.value {
            Some(v) => (v.to_string(), Role::OnSurface),
            None if row.try_keys.is_empty() => (
                S.starter_notes_not_known.to_string(),
                Role::OnSurfaceVariant,
            ),
            None => (
                fill(S.starter_notes_try, &[&row.try_keys.join(" · ")]),
                Role::OnSurfaceVariant,
            ),
        };
        y += note_row(ui, x, y, w, &row.label, &value, tone);
    }
    y += 10.0;
    if let Some(why) = &starter.notes_open_failed {
        y += line(ui, x, y, w, &reason_text(why), Role::Error);
    }
    let print = if starter.key_reading {
        Style::Disabled
    } else {
        Style::Secondary
    };
    if !starter.bitlocker_offered() {
        y += buttons(
            ui,
            x,
            y,
            w,
            &[(S.starter_notes_print, print, StarterAction::PrintNotes)],
        );
        return y + 24.0 - y0;
    }
    // Beside Print, on Windows with BitLocker on (F49): the tick box
    // that puts the recovery key on the sheet, unticked until ticked.
    let tick_w = 18.0 + 12.0 + ui.measure(14.0, W::R, S.starter_notes_include_key);
    let (tx, ty) = if ui.compact {
        let bh = buttons(
            ui,
            x,
            y,
            w,
            &[(S.starter_notes_print, print, StarterAction::PrintNotes)],
        );
        y += bh + 8.0;
        (x, y)
    } else {
        let bw = ui.button(
            x,
            y,
            None,
            44.0,
            S.starter_notes_print,
            print,
            StarterAction::PrintNotes.host(),
        );
        (x + bw + 20.0, y)
    };
    let a = StarterAction::IncludeKey.host();
    ui.state_layer(
        (tx - 6.0, ty, tick_w + 12.0, 44.0, 8.0),
        Role::OnSurface.paint(),
        a,
    );
    ui.checkbox(tx, ty + 13.0, starter.include_key, true);
    ui.text_mid(
        tx + 30.0,
        ty,
        44.0,
        14.0,
        W::R,
        Role::OnSurface,
        S.starter_notes_include_key,
    );
    ui.hit(tx - 6.0, ty, tick_w + 12.0, 44.0, a);
    y = ty + 44.0;
    if starter.key_reading {
        y += 8.0;
        y += line(
            ui,
            x,
            y,
            w,
            S.starter_waiting_permission,
            Role::OnSurfaceVariant,
        );
    }
    y + 24.0 - y0
}

/// BitLocker on the system drive (§6, F49), on Windows with it on or
/// suspended: the drive's state as Windows reads it, and the one step it
/// allows, never required: Temporarily suspend BitLocker while on,
/// Resume BitLocker while suspended. Returns its height.
fn bitlocker_block(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let Some(b) = starter
        .bitlocker
        .as_ref()
        .filter(|_| starter.bitlocker_offered())
    else {
        return 0.0;
    };
    let mut y = y0;
    let on = b.protection == crate::api::Protection::On;
    let state = if on {
        S.starter_bitlocker_enabled.to_string()
    } else {
        match b.starts_left {
            Some(0) => S.starter_bitlocker_suspended_until.to_string(),
            Some(1) => S.starter_bitlocker_suspended_once.to_string(),
            Some(n) => fill(S.starter_bitlocker_suspended_times, &[&n.to_string()]),
            None => S.starter_bitlocker_suspended.to_string(),
        }
    };
    y += note_row(
        ui,
        x,
        y,
        w,
        &fill(S.starter_bitlocker_row, &[&b.drive]),
        &state,
        Role::OnSurface,
    );
    y += 10.0;
    if let Some(why) = &starter.bitlocker_failed {
        y += line(ui, x, y, w, &reason_text(why), Role::Error);
    }
    let busy = starter.bitlocker_step.is_some();
    let style = if busy {
        Style::Disabled
    } else {
        Style::Secondary
    };
    let (label, action) = if on {
        (S.starter_bitlocker_suspend, StarterAction::SuspendBitLocker)
    } else {
        (S.starter_bitlocker_resume, StarterAction::ResumeBitLocker)
    };
    y += buttons(ui, x, y, w, &[(label, style, action)]) + 8.0;
    if busy {
        y += line(
            ui,
            x,
            y,
            w,
            S.starter_waiting_permission,
            Role::OnSurfaceVariant,
        );
    } else if on {
        y += line(
            ui,
            x,
            y,
            w,
            &fill(
                S.starter_bitlocker_suspend_note,
                &[&crate::api::SUSPEND_STARTS.to_string()],
            ),
            Role::OnSurfaceVariant,
        );
    }
    y + 16.0 - y0
}

/// A value row whose value is never cut: under its label, wrapped, when
/// it does not fit beside it (a long menu path). Returns its height.
fn note_row(ui: &mut Ui, x: f32, y: f32, w: f32, key: &str, value: &str, tone: Role) -> f32 {
    let kw = ui.measure(13.0, W::R, key) + 24.0;
    if ui.compact || ui.measure(14.0, W::R, value) <= w - kw {
        return value_row(ui, x, y, w, key, value, tone);
    }
    ui.text(x, y + 8.0, 13.0, W::R, Role::OnSurfaceVariant, key);
    let vh = ui.wrap(x, y + 30.0, w, 14.0, W::R, tone, value);
    ui.rule(x, y + 38.0 + vh, w, Role::OutlineVariant);
    vh + 40.0
}

/// The guide's address as a QR code, with the address under it: the
/// code made when the computer was reported ([`Starter::phone_code`]).
/// Returns its height.
#[allow(clippy::too_many_arguments)]
fn phone_code(
    starter: &Starter,
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    side: f32,
    address: &str,
) -> f32 {
    let side = side.min(w);
    let Some(m) = starter.phone_code(address) else {
        return 0.0;
    };
    qr_code(ui, m, x, y, side);
    let th = ui.wrap(
        x,
        y + side + 10.0,
        w,
        12.0,
        W::M,
        Role::OnSurfaceVariant,
        address,
    );
    side + 10.0 + th + 8.0
}

/// A dialog over the frosted page (`faraday_ui::sheet::dialog`, the one
/// Faraday draws): `head` with its icon, what `body` draws under it, and
/// `actions` at its foot.
fn dialog(
    ui: &mut Ui,
    (w, h): (f32, f32),
    head: (Icon, Role, &str),
    body: &mut dyn FnMut(&mut Ui, f32, f32, f32) -> f32,
    actions: &[(&str, Style, StarterAction)],
) {
    let head = (Glyph::from(head.0), head.1.paint(), head.2);
    let frame = (w, h, faraday_draw::tokens::DIALOG_MAX);
    let actions: Vec<(&str, Style, Action)> =
        actions.iter().map(|&(l, s, a)| (l, s, a.host())).collect();
    faraday_ui::sheet::dialog(
        ui,
        frame,
        head,
        &mut |ui, x, y, iw| body(ui, x, y, iw),
        &actions,
    );
}

fn restart_sheet(starter: &Starter, ui: &mut Ui, w: f32, h: f32, to: RestartTo) {
    let address = LearnId::for_computer(&starter.computer).address();
    let os = starter.computer.os;
    let head = match to {
        RestartTo::Firmware => S.starter_restart,
        RestartTo::Stick => S.starter_restart_stick,
    };
    let actions = [
        (S.starter_cancel, Style::Secondary, StarterAction::Cancel),
        (
            S.starter_restart_now,
            Style::Primary,
            StarterAction::RestartNow,
        ),
    ];
    let title = (Icon::Power, Role::OnSurface, head);
    dialog(
        ui,
        (w, h),
        title,
        &mut |ui, x, y0, iw| {
            let mut y = y0;
            y += line(ui, x, y, iw, S.starter_restart_save, Role::Warning) + 4.0;
            y += line(
                ui,
                x,
                y,
                iw,
                S.starter_restart_keep_stick,
                Role::OnSurfaceVariant,
            );
            if to == RestartTo::Stick && os == Os::Windows {
                y += line(
                    ui,
                    x,
                    y,
                    iw,
                    S.starter_restart_windows_menu,
                    Role::OnSurfaceVariant,
                );
            }
            y += 4.0;
            if to == RestartTo::Stick && os == Os::Linux {
                y += link(ui, x, y, iw, LearnId::BootEntry);
            }
            // Small enough on a small panel that the sheet's buttons fit.
            let side = if ui.compact { 104.0 } else { 160.0 };
            y += phone_code(starter, ui, x, y, iw, side, &address);
            y - y0
        },
        &actions,
    );
}

/// Continue anyway, asked once, naming the item (§3.1a).
fn let_through_sheet(starter: &Starter, ui: &mut Ui, w: f32, h: f32) {
    let Some(c) = starter
        .asking
        .as_ref()
        .and_then(|id| starter.concerns.iter().find(|c| &c.id == id))
    else {
        return;
    };
    let name = if c.name.trim().is_empty() {
        concern_text(c)
    } else {
        c.name.trim().to_string()
    };
    let ask = fill(S.starter_let_through_ask, &[&name]);
    let line_text = concern_text(c);
    let actions = [
        (S.starter_cancel, Style::Secondary, StarterAction::Cancel),
        (
            S.starter_continue_anyway,
            Style::Primary,
            StarterAction::LetThrough,
        ),
    ];
    let title = (Icon::Warning, Role::Warning, ask.as_str());
    dialog(
        ui,
        (w, h),
        title,
        &mut |ui, x, y0, iw| {
            let mut y = y0;
            y += line(ui, x, y, iw, &line_text, Role::Warning);
            y += line(
                ui,
                x,
                y,
                iw,
                S.starter_let_through_until,
                Role::OnSurfaceVariant,
            );
            y - y0
        },
        &actions,
    );
}

// ---------------------------------------------------------------------
// Install, Installed, About
// ---------------------------------------------------------------------

fn install(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    if starter.update_offered()
        && let Some(i) = &starter.installed
    {
        y += fact_row(ui, x, y, w, S.starter_installed_version, &i.version);
        y += fact_row(ui, x, y, w, S.starter_this_version, crate::VERSION);
        y += 12.0;
    }
    if let Some(Ok(place)) = &starter.install_result {
        y += line(
            ui,
            x,
            y,
            w,
            &fill(S.starter_install_done_path, &[place]),
            Role::Ok,
        ) + 8.0;
        return y + buttons(
            ui,
            x,
            y,
            w,
            &[(S.starter_done, Style::Primary, StarterAction::Home)],
        );
    }
    if let Some(Err(why)) = &starter.install_result {
        y += line(ui, x, y, w, &reason_text(why), Role::Error);
    }
    let (label, style) = if starter.installing {
        (S.starter_installing, Style::Disabled)
    } else {
        (S.starter_install_now, Style::Primary)
    };
    y + buttons(ui, x, y, w, &[(label, style, StarterAction::InstallNow)])
}

fn item_name(item: InstallItem) -> &'static str {
    match item {
        InstallItem::App => S.starter_installed_app,
        InstallItem::Desktop => S.starter_installed_desktop,
        InstallItem::Menu => S.starter_installed_menu,
        InstallItem::DesktopMenu => S.starter_installed_desktop_menu,
        InstallItem::Icon => S.starter_installed_icon,
        InstallItem::UninstallEntry => S.starter_installed_entry,
    }
}

fn installed(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    if let Some(why) = &starter.entry_kept {
        y += line(
            ui,
            x,
            y,
            w,
            &fill(S.starter_entry_kept, &[&reason_text(why)]),
            Role::Warning,
        );
    }
    let Some(i) = &starter.installed else {
        return y + line(ui, x, y, w, S.starter_not_installed, Role::OnSurfaceVariant);
    };
    y += fact_row(ui, x, y, w, S.starter_installed_version, &i.version);
    for (item, place) in &i.items {
        y += mono_row(ui, x, y, w, item_name(*item), place);
    }
    y += 16.0;
    if let Some(why) = &starter.uninstall_failed {
        y += line(ui, x, y, w, &reason_text(why), Role::Error);
    }
    y + buttons(
        ui,
        x,
        y,
        w,
        &[(
            S.starter_uninstall,
            Style::Secondary,
            StarterAction::Uninstall,
        )],
    )
}

/// Uninstall, asked once.
fn uninstall_sheet(_starter: &Starter, ui: &mut Ui, w: f32, h: f32) {
    let actions = [
        (S.starter_cancel, Style::Secondary, StarterAction::Cancel),
        (
            S.starter_uninstall,
            Style::Primary,
            StarterAction::UninstallNow,
        ),
    ];
    let title = (Icon::Trash, Role::OnSurface, S.starter_uninstall_ask);
    dialog(
        ui,
        (w, h),
        title,
        &mut |ui, x, y0, iw| {
            line(
                ui,
                x,
                y0,
                iw,
                S.starter_uninstall_keeps,
                Role::OnSurfaceVariant,
            )
        },
        &actions,
    );
}

/// The release key's fingerprint, as the README prints it.
const FINGERPRINT: &str = "D1AF A712 ABEF 9010 9A1B  587F F153 6BE0 7E23 4CFD";

fn about(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    y += fact_row(ui, x, y, w, S.starter_version, crate::VERSION);
    match &starter.payload {
        Some(p) => {
            y += fact_row(ui, x, y, w, S.starter_release, &p.version);
            y += mono_row(ui, x, y, w, S.starter_commit, &p.commit);
            y += 16.0;
            y += label(ui, x, y, S.starter_payload);
            for f in &p.files {
                y += 4.0;
                y += ui.wrap(x, y, w, 14.0, W::S, Role::OnSurface, &f.name) + 4.0;
                y += fact_row(ui, x, y, w, S.starter_build, &describe(&f.build_id));
                y += mono_row(ui, x, y, w, S.starter_sha_xz, &grouped(&f.sha256));
                let inner = fill(S.starter_sha_inner, &[&f.inner]);
                y += mono_row(ui, x, y, w, &inner, &grouped(&f.inner_sha256));
                y += 8.0;
            }
        }
        None => {
            y += line(
                ui,
                x,
                y + 8.0,
                w,
                S.starter_no_payload,
                Role::OnSurfaceVariant,
            ) + 8.0
        }
    }
    if let Some(d) = &starter.dev {
        y += 8.0;
        y += warning(ui, x, y, w, S.starter_dev_image);
        y += mono_row(ui, x, y, w, S.starter_image_file, &d.path);
        y += mono_row(ui, x, y, w, S.starter_image_sha, &grouped(&d.sha256));
    }
    y += 16.0;
    y += label(ui, x, y, S.starter_release_key);
    y += ui.wrap(x, y, w, 13.0, W::M, Role::OnSurface, FINGERPRINT) + 18.0;
    y += label(ui, x, y, S.starter_check_title);
    y += line(ui, x, y, w, S.starter_check_text, Role::OnSurface);
    for cmd in [
        S.starter_check_cmd_import,
        S.starter_check_cmd_verify,
        S.starter_check_cmd_sums,
    ] {
        y += ui.wrap(x, y, w, 12.0, W::M, Role::OnSurfaceVariant, cmd) + 6.0;
    }
    if starter.computer.os == Os::Windows {
        y += 12.0;
        y += line(ui, x, y, w, S.starter_check_windows, Role::OnSurface);
        y += label(ui, x, y + 8.0, S.starter_unsigned_title) + 8.0;
        y += line(ui, x, y, w, S.starter_unsigned_text, Role::OnSurface);
    }
    y
}

// ---------------------------------------------------------------------
// Blank templates (§3.8)
// ---------------------------------------------------------------------

/// What Copy reads as.
fn copy_text(c: Copy) -> &'static str {
    match c {
        Copy::Words => S.starter_tpl_words,
        Copy::SeedQr => S.starter_tpl_seedqr,
        Copy::Both => S.starter_tpl_both,
    }
}

/// What a Form reads as.
fn form_text(f: Form) -> &'static str {
    match f {
        Form::Words => S.starter_tpl_form_words,
        Form::Codex32 => S.starter_tpl_form_codex32,
    }
}

/// What a codex32 copy reads as.
fn codex_copy_text(c: CodexCopy) -> &'static str {
    match c {
        CodexCopy::String => S.starter_tpl_string,
        CodexCopy::Codex32Qr => S.starter_tpl_codex32qr,
        CodexCopy::Both => S.starter_tpl_both,
        CodexCopy::Grids => S.starter_tpl_grids,
    }
}

/// A codex32 length as the menu has it.
fn chars_text(n: usize) -> String {
    fill(S.starter_tpl_chars, &[&n.to_string()])
}

/// What Worksheets reads as.
fn worksheets_text(on: bool) -> &'static str {
    if on {
        S.starter_tpl_ws_all
    } else {
        S.starter_tpl_ws_none
    }
}

/// The menu's rows: head, value and the choice each opens, by form.
fn template_rows(starter: &Starter) -> Vec<(&'static str, String, TemplateChoice)> {
    let t = &starter.template;
    let form = (
        S.starter_tpl_form,
        form_text(t.form).to_string(),
        TemplateChoice::Form,
    );
    let places = (
        S.starter_tpl_places,
        t.places.to_string(),
        TemplateChoice::Places,
    );
    match t.form {
        Form::Words => vec![
            form,
            (
                S.starter_tpl_words,
                t.words.to_string(),
                TemplateChoice::Words,
            ),
            (
                S.starter_tpl_copy,
                copy_text(t.copy).to_string(),
                TemplateChoice::Copy,
            ),
            places,
        ],
        Form::Codex32 => vec![
            form,
            (
                S.starter_tpl_length,
                chars_text(t.chars),
                TemplateChoice::Length,
            ),
            (
                S.starter_tpl_copy,
                codex_copy_text(t.codex_copy).to_string(),
                TemplateChoice::Copy,
            ),
            (
                S.starter_tpl_worksheets,
                worksheets_text(t.worksheets).to_string(),
                TemplateChoice::Worksheets,
            ),
            places,
        ],
    }
}

/// The blank templates menu: Words, Copy and Places as value rows, Print,
/// Save the PDF, and what became of the last save.
fn templates(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    let t = &starter.template;
    let rows = template_rows(starter);
    for (head, tail, choice) in rows {
        y += nav_row(
            ui,
            x,
            y,
            w,
            Icon::Learn,
            head,
            "",
            &tail,
            Some(StarterAction::TemplateChoice(choice)),
        );
    }
    y += 12.0;
    y += buttons(
        ui,
        x,
        y,
        w,
        &[
            (
                S.starter_tpl_print,
                Style::Primary,
                StarterAction::PrintTemplate,
            ),
            (
                S.starter_tpl_save,
                Style::Secondary,
                StarterAction::SaveTemplate,
            ),
        ],
    );
    y += 14.0;
    match &t.saved {
        Some(Ok(path)) => {
            y += line(ui, x, y, w, &fill(S.starter_tpl_saved, &[path]), Role::Ok);
            y += buttons(
                ui,
                x,
                y,
                w,
                &[(
                    S.starter_tpl_open,
                    Style::Secondary,
                    StarterAction::OpenTemplate,
                )],
            );
        }
        Some(Err(why)) => y += line(ui, x, y, w, &reason_text(why), Role::Error),
        None => {}
    }
    if let Some(why) = &t.open_failed {
        y += line(ui, x, y, w, &reason_text(why), Role::Error);
    }
    y
}

/// A Choice row: a ring, filled when chosen, and the option.
fn tick_row(
    ui: &mut Ui,
    x: f32,
    y: f32,
    w: f32,
    head: &str,
    on: bool,
    action: StarterAction,
) -> f32 {
    let a = action.host();
    let rh = 52.0;
    ui.state_layer(
        (x - 6.0, y + 2.0, w + 12.0, rh - 4.0, 8.0),
        Role::OnSurface.paint(),
        a,
    );
    let cy = y + rh / 2.0;
    radio_mark(ui, x, cy, on, true);
    ui.text_mid(x + 36.0, y, rh, 15.0, W::S, Role::OnSurface, head);
    ui.hit(x - 6.0, y, w + 12.0, rh, a);
    ui.rule(x, y + rh, w, Role::OutlineVariant);
    rh
}

/// Back to the menu, the Choice's Continue.
fn choice_done(ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    y + 20.0
        + buttons(
            ui,
            x,
            y + 20.0,
            w,
            &[(S.starter_continue, Style::Primary, StarterAction::Back)],
        )
}

fn template_words(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    ui.rule(x, y, w, Role::OutlineVariant);
    for n in [12usize, 24] {
        y += tick_row(
            ui,
            x,
            y,
            w,
            &n.to_string(),
            starter.template.words == n,
            StarterAction::TemplateWords(n),
        );
    }
    choice_done(ui, x, y, w)
}

fn template_copy(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    ui.rule(x, y, w, Role::OutlineVariant);
    if starter.template.form == Form::Codex32 {
        // The Codex32QR grid is for 48 characters only.
        for c in CodexCopy::ALL {
            if c != CodexCopy::String && starter.template.chars != 48 {
                continue;
            }
            y += tick_row(
                ui,
                x,
                y,
                w,
                codex_copy_text(c),
                starter.template.codex_copy == c,
                StarterAction::TemplateCodexCopy(c),
            );
        }
        return choice_done(ui, x, y, w);
    }
    for c in Copy::ALL {
        y += tick_row(
            ui,
            x,
            y,
            w,
            copy_text(c),
            starter.template.copy == c,
            StarterAction::TemplateCopy(c),
        );
    }
    choice_done(ui, x, y, w)
}

fn template_form(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    ui.rule(x, y, w, Role::OutlineVariant);
    for f in [Form::Words, Form::Codex32] {
        y += tick_row(
            ui,
            x,
            y,
            w,
            form_text(f),
            starter.template.form == f,
            StarterAction::TemplateForm(f),
        );
    }
    choice_done(ui, x, y, w)
}

fn template_length(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    ui.rule(x, y, w, Role::OutlineVariant);
    for n in CODEX_LENGTHS {
        y += tick_row(
            ui,
            x,
            y,
            w,
            &chars_text(n),
            starter.template.chars == n,
            StarterAction::TemplateLength(n),
        );
    }
    choice_done(ui, x, y, w)
}

fn template_worksheets(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    ui.rule(x, y, w, Role::OutlineVariant);
    for on in [false, true] {
        // None are published at 127 characters.
        if on && starter.template.chars == 127 {
            continue;
        }
        y += tick_row(
            ui,
            x,
            y,
            w,
            worksheets_text(on),
            starter.template.worksheets == on,
            StarterAction::TemplateWorksheets(on),
        );
    }
    choice_done(ui, x, y, w)
}

fn template_places(starter: &Starter, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    y += fact_row(
        ui,
        x,
        y,
        w,
        S.starter_tpl_places,
        &starter.template.places.to_string(),
    );
    y += 12.0;
    y += buttons(
        ui,
        x,
        y,
        w,
        &[
            (
                S.starter_tpl_fewer,
                Style::Secondary,
                StarterAction::PlacesFewer,
            ),
            (
                S.starter_tpl_more,
                Style::Secondary,
                StarterAction::PlacesMore,
            ),
        ],
    );
    choice_done(ui, x, y + 14.0, w)
}

/// What a test or a snapshot sets and reads of the frame's drawing.
impl Starter {
    /// Everything the current frame draws that leaves ink (text, icons,
    /// progress), with the box of each in pixels, for a test of the
    /// colours it is drawn in.
    pub fn drawn_ink(&mut self) -> Vec<faraday_draw::canvas::Ink> {
        // Under a sheet, the page is drawn once to make its frosted copy,
        // then the frame is drawn from the copy as it is on screen; only
        // that second draw is recorded.
        self.frost = None;
        self.render();
        if let Some(c) = self.canvas.as_mut() {
            c.record_ink(true);
        }
        self.render();
        let ink = self
            .canvas
            .as_ref()
            .map_or_else(Vec::new, |c| c.ink().to_vec());
        if let Some(c) = self.canvas.as_mut() {
            c.record_ink(false);
        }
        self.dirty = true;
        ink
    }
}
