//! The hub's pages, drawn each frame from its state with `faraday-core`'s
//! `Ui`, in the desktop app's palette and sizes. On a wide display a page
//! is a centred column under a strip with the way back; on a small panel
//! (`compact`) a bar across the top carries the way back and the page's
//! name, as the desktop app's small-panel pages do.

use faraday_core::strings::{EN, Strings};
use faraday_core::ui::pal::*;
use faraday_core::ui::{NameSuffix, Style, Ui, W};
use faraday_core::upgrade::{Fit, describe, version};
use opensigner_core::strings::fill;
use osk_ui::Color;
use osk_ui::widgets::Icon;

use faraday_qr::bitlocker::RecoveryKey;

use crate::api::{
    Concern, InUse, InstallItem, KeyShown, Kind, Mode, Os, PayloadKind, Protection, Reason,
    StickFault, WriteStage,
};
use crate::decide::{Item, Refusal, State, Target};
use crate::{Card, Hub, HubAction, LearnId, ModeRefusal, Page, RestartTo, Sheet};

const S: &Strings = &EN;

/// The small panel's side margin.
const M: f32 = 12.0;
/// The small panel's bar.
const BAR_H: f32 = 44.0;
/// The wide strip with the way back.
const STRIP_H: f32 = 52.0;
/// The widest the column grows.
const COL_W: f32 = 720.0;

/// Draws the frame. Returns how far the page can scroll, units.
pub(crate) fn frame(hub: &Hub, ui: &mut Ui) -> f32 {
    let (w, h) = (hub.w, hub.h);
    ui.clear(BG);
    let compact = hub.compact;
    let top = if hub.page == Page::Home {
        0.0
    } else if compact {
        BAR_H
    } else {
        STRIP_H
    };
    let (x, cw) = if compact {
        (M, w - 2.0 * M)
    } else {
        let cw = (w - 96.0).min(COL_W);
        ((w - cw) / 2.0, cw)
    };
    let clip = ui.rect(0.0, top, w, h - top);
    ui.c.push_clip(clip);
    let pad = if compact { 12.0 } else { 28.0 };
    let y0 = top + pad - hub.offset;
    let end = match hub.page {
        Page::Home => home(hub, ui, x, y0, cw),
        Page::Disks => disks(hub, ui, x, y0, cw),
        Page::Mode => mode(hub, ui, x, y0, cw),
        Page::Confirm => confirm(hub, ui, x, y0, cw),
        Page::Writing => writing(hub, ui, x, y0, cw),
        Page::Done => done(hub, ui, x, y0, cw),
        Page::Start => start(hub, ui, x, y0, cw),
        Page::Install => install(hub, ui, x, y0, cw),
        Page::Installed => installed(hub, ui, x, y0, cw),
        Page::About => about(hub, ui, x, y0, cw),
        Page::BitLocker => bitlocker(hub, ui, x, y0, cw),
    };
    ui.c.pop_clip();
    let content = end - y0 + pad + 16.0;
    let max = (content - (h - top)).max(0.0);
    ui.report_scroll(clip, max);
    if hub.page != Page::Home {
        head(hub, ui, w);
    }
    if let Some(sheet) = hub.sheet {
        ui.hits.clear();
        ui.in_sheet = true;
        ui.backdrop();
        match sheet {
            Sheet::Restart(to) => restart_sheet(hub, ui, w, h, to),
            Sheet::Uninstall => uninstall_sheet(hub, ui, w, h),
            Sheet::LetThrough => let_through_sheet(hub, ui, w, h),
        }
        ui.in_sheet = false;
    }
    max
}

// ---------------------------------------------------------------------
// The frame's pieces
// ---------------------------------------------------------------------

/// A page's name, as its title and its bar say it.
fn page_name(hub: &Hub) -> String {
    let target = hub.making.as_ref().map_or(Target::Stick, |m| m.target);
    match hub.page {
        Page::Home => String::new(),
        Page::Disks => match target {
            Target::Stick => S.hub_disks_stick,
            Target::Pi => S.hub_disks_card,
        }
        .to_string(),
        Page::Mode => S.hub_mode_title.to_string(),
        Page::Confirm => S.hub_confirm_title.to_string(),
        Page::Writing => S.hub_writing_title.to_string(),
        Page::Done => match hub.making.as_ref().and_then(|m| m.result.as_ref()) {
            Some(Ok(())) => S.hub_done_title,
            _ => S.hub_failed_title,
        }
        .to_string(),
        Page::Start => S.hub_item_start.to_string(),
        Page::Install if hub.update_offered() => S.hub_row_update.to_string(),
        Page::Install => S.hub_row_install.to_string(),
        Page::Installed => S.hub_row_installed.to_string(),
        Page::About => S.hub_row_about.to_string(),
        Page::BitLocker => S.hub_item_bitlocker.to_string(),
    }
}

/// Where the way back goes, by name; none while a write runs.
fn back_name(hub: &Hub) -> Option<String> {
    match hub.page {
        Page::Home | Page::Writing => None,
        Page::Mode => {
            let target = hub.making.as_ref().map_or(Target::Stick, |m| m.target);
            Some(
                match target {
                    Target::Stick => S.hub_disks_stick,
                    Target::Pi => S.hub_disks_card,
                }
                .to_string(),
            )
        }
        Page::Confirm => Some(S.hub_mode_title.to_string()),
        _ => Some(S.hub_home.to_string()),
    }
}

/// The bar (small panel) or the strip (wide) across the top.
fn head(hub: &Hub, ui: &mut Ui, w: f32) {
    let back = HubAction::Back.host();
    let name = page_name(hub);
    let back_to = back_name(hub);
    if hub.compact {
        ui.fill(0.0, 0.0, w, BAR_H, 0.0, SIDEBAR);
        ui.fill(0.0, BAR_H - 1.0, w, 1.0, 0.0, LINE);
        let x = if back_to.is_some() { BAR_H } else { M };
        if let Some(label) = &back_to {
            if ui.is_pressed(back) {
                ui.fill(4.0, 4.0, BAR_H - 8.0, BAR_H - 8.0, 8.0, INNER);
            }
            ui.icon(4.0, 4.0, BAR_H - 8.0, Icon::ChevronLeft, 13.0, TEXT);
            ui.hit(0.0, 0.0, BAR_H + 8.0, BAR_H, back);
            ui.back(back);
            let label = ui.fit(11.0, W::R, label, w - x - M);
            ui.text(x, 5.0, 11.0, W::R, MUTED, &label);
            let name = ui.fit(16.0, W::S, &name, w - x - M);
            ui.text(x, 19.0, 16.0, W::S, TEXT, &name);
        } else {
            let name = ui.fit(16.0, W::S, &name, w - x - M);
            ui.text_mid(x, 0.0, BAR_H, 16.0, W::S, TEXT, &name);
        }
        return;
    }
    ui.fill(0.0, 0.0, w, STRIP_H, 0.0, BG);
    if let Some(label) = back_to {
        let bw = ui.measure(14.0, W::S, &label) + 44.0;
        let (bx, by) = (24.0, 10.0);
        if ui.is_pressed(back) || ui.is_hovered(back) {
            ui.fill(bx, by, bw, 34.0, 8.0, INNER);
        }
        ui.icon(bx + 4.0, by + 5.0, 24.0, Icon::ChevronLeft, 12.0, MUTED);
        ui.text_mid(bx + 30.0, by, 34.0, 14.0, W::S, MUTED, &label);
        ui.hit(bx, by, bw, 34.0, back);
        ui.back(back);
    }
}

/// A page's title on a wide display; the bar carries it on a small
/// panel. Returns the height it took.
fn title(hub: &Hub, ui: &mut Ui, x: f32, y: f32) -> f32 {
    if hub.compact {
        return 0.0;
    }
    let name = page_name(hub);
    ui.text(x, y, 28.0, W::S, TEXT, &name);
    ui.line(28.0, W::S) + 20.0
}

/// A section's label. Returns its height.
fn label(ui: &mut Ui, x: f32, y: f32, s: &str) -> f32 {
    ui.text(x, y, 13.0, W::S, MUTED, s);
    24.0
}

/// A line of fact, wrapped. Returns its height with the gap under it.
fn line(ui: &mut Ui, x: f32, y: f32, w: f32, s: &str, tone: Color) -> f32 {
    ui.wrap(x, y, w, 13.0, W::R, tone, s) + 10.0
}

/// A label and its value: on one row with the value at the right on a
/// wide display, the value under the label on a small panel. Returns
/// its height.
fn value_row(ui: &mut Ui, x: f32, y: f32, w: f32, key: &str, value: &str, tone: Color) -> f32 {
    if ui.compact {
        ui.text(x, y, 12.0, W::R, MUTED, key);
        let vh = ui.wrap(x, y + 18.0, w, 14.0, W::R, tone, value);
        ui.rule(x, y + 24.0 + vh, w, INNER);
        return vh + 32.0;
    }
    let kw = ui.measure(13.0, W::R, key) + 24.0;
    ui.text_mid(x, y, 36.0, 13.0, W::R, MUTED, key);
    let v = ui.fit(14.0, W::R, value, w - kw);
    ui.text_right(x + w, y, 36.0, 14.0, W::R, tone, &v);
    ui.rule(x, y + 36.0, w, INNER);
    38.0
}

/// A label over a long value in the mono face, in groups of four,
/// wrapped (a SHA-256, a fingerprint, a path). Returns its height.
fn mono_row(ui: &mut Ui, x: f32, y: f32, w: f32, key: &str, value: &str) -> f32 {
    ui.text(x, y + 8.0, 12.0, W::R, MUTED, key);
    let vh = ui.wrap(x, y + 28.0, w, 13.0, W::M, TEXT, value);
    ui.rule(x, y + 36.0 + vh, w, INNER);
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
    icon: Icon,
    head: &str,
    sub: &str,
    tail: &str,
    action: Option<HubAction>,
) -> f32 {
    let compact = ui.compact;
    let h = if compact { 54.0 } else { 58.0 };
    let enabled = action.is_some();
    let pressed = action.is_some_and(|a| ui.is_pressed(a.host()));
    ui.fill(x, y, w, h, 10.0, if pressed { INNER } else { SURFACE });
    ui.stroke(x, y, w, h, 10.0, LINE);
    let ib = if compact { 28.0 } else { 32.0 };
    let (ibg, ifg) = if enabled {
        (ACCENT.with_alpha(26), ACCENT)
    } else {
        (INNER, DIM)
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
    let fg = if enabled { TEXT } else { DIM };
    if sub.is_empty() {
        ui.text_mid(tx, y, h, 14.0, W::S, fg, &head);
    } else {
        ui.text(tx, y + 9.0, 14.0, W::S, fg, &head);
        let sub = ui.fit(12.0, W::R, sub, w - (tx - x) - 12.0 - chevron);
        let tone = if enabled { MUTED } else { WARN };
        ui.text(tx, y + 30.0, 12.0, W::R, tone, &sub);
    }
    if !tail.is_empty() {
        ui.text_right(
            x + w - 12.0 - chevron,
            y,
            if sub.is_empty() { h } else { 30.0 },
            12.0,
            W::R,
            if enabled { MUTED } else { DIM },
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
            DIM,
        );
        ui.hit(x, y, w, h, a.host());
    }
    h + 8.0
}

/// A warning: a tinted box with the line in it. Returns its height with
/// the gap under it.
fn warning(ui: &mut Ui, x: f32, y: f32, w: f32, s: &str, tone: Color) -> f32 {
    let th = ui.wrap(x + 14.0, y + 10.0, w - 28.0, 13.0, W::S, tone, s);
    // Drawn again over its box, which needs the text's height first.
    let bh = th + 20.0;
    ui.fill(x, y, w, bh, 10.0, tone.with_alpha(26));
    ui.stroke(x, y, w, bh, 10.0, tone.with_alpha(110));
    ui.wrap(x + 14.0, y + 10.0, w - 28.0, 13.0, W::S, tone, s);
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
    let action = HubAction::LearnPage(id).host();
    let label = ui.fit(13.0, W::S, label, w);
    let tw = ui.measure(13.0, W::S, &label);
    let tone = if ui.is_pressed(action) || ui.is_hovered(action) {
        TEXT
    } else {
        ACCENT
    };
    ui.text_mid(x, y, 28.0, 13.0, W::S, tone, &label);
    ui.hit(x - 4.0, y, tw + 8.0, 28.0, action);
    32.0
}

/// An item of concern's line (§3.1a), naming it.
pub(crate) fn concern_text(c: &Concern) -> String {
    let fallback = match c.kind {
        Kind::Tv | Kind::WirelessDisplay => S.hub_concern_screen,
        Kind::Kvm | Kind::Receiver | Kind::BoltReceiver => S.hub_concern_device,
        Kind::VirtualMachine => S.hub_concern_computer,
        Kind::RemoteSession => S.hub_concern_session,
    };
    let name = if c.name.trim().is_empty() {
        fallback
    } else {
        c.name.trim()
    };
    let line = match c.kind {
        Kind::Tv => S.hub_concern_tv,
        Kind::Kvm => S.hub_concern_kvm,
        Kind::Receiver => S.devices_receiver_notice,
        Kind::BoltReceiver => S.devices_bolt_notice,
        Kind::VirtualMachine => S.hub_concern_vm,
        Kind::RemoteSession => S.hub_concern_remote,
        Kind::WirelessDisplay => S.hub_concern_wireless_display,
    };
    fill(line, &[name])
}

/// A wireless receiver's line (§3.1a), which holds nothing: the warning,
/// or a Bolt receiver's plain line. Returns its height with the gap
/// under it; nothing without a receiver.
fn receiver_line(hub: &Hub, ui: &mut Ui, x: f32, y: f32, w: f32) -> f32 {
    match hub.receiver_line() {
        Some(l) if l == S.devices_bolt_notice => line(ui, x, y, w, l, MUTED),
        Some(l) => warning(ui, x, y, w, l, WARN),
        None => 0.0,
    }
}

/// What is attached that holds a step (§3.1a): each item's line, its
/// Continue anyway (never for a virtual machine), Check again, and the
/// Learn page. Returns its height with the gap under it; nothing when
/// nothing holds.
fn concern_block(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32, items: &[&Concern]) -> f32 {
    if items.is_empty() {
        return 0.0;
    }
    let mut y = y0;
    for c in items {
        y += warning(ui, x, y, w, &concern_text(c), WARN);
        if c.kind != Kind::VirtualMachine {
            let n = HubAction::ContinueAnyway(hub.concern_number(&c.id));
            y += buttons(
                ui,
                x,
                y - 4.0,
                w,
                &[(S.hub_continue_anyway, Style::Ghost, n)],
            ) + 8.0;
        }
    }
    y += buttons(
        ui,
        x,
        y,
        w,
        &[(S.hub_check_again, Style::Secondary, HubAction::CheckAgain)],
    ) + 8.0;
    y += link(ui, x, y, w, LearnId::ScreensThatRecord);
    y + 12.0 - y0
}

/// One button, or a row of them (a column on a small panel when they do
/// not fit). Returns their height.
fn buttons(ui: &mut Ui, x: f32, y: f32, w: f32, items: &[(&str, Style, HubAction)]) -> f32 {
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
    let th = ui.wrap(x + 38.0, y + 3.0, w - 38.0, 15.0, W::S, TEXT, text);
    th.max(26.0) + 10.0
}

/// What a reason reads as, from `faraday_core::strings`.
pub(crate) fn reason_text(r: &Reason) -> String {
    let n = |v: u64| v.to_string();
    let bytes = |v: u64| fill(S.hub_why_bytes, &[&v.to_string()]);
    match r {
        Reason::NoWriter => S.hub_no_writer.to_string(),
        Reason::NotCarried => S.hub_why_not_carried.to_string(),
        Reason::Changed {
            device,
            serial,
            size,
        } => {
            let serial = if serial.is_empty() {
                S.hub_why_no_serial
            } else {
                serial
            };
            fill(S.hub_why_changed, &[device, serial, &bytes(*size)])
        }
        Reason::InUse(InUse::Mounted(at)) => fill(S.hub_why_mounted, &[at]),
        Reason::InUse(InUse::Swap) => S.hub_why_swap.to_string(),
        Reason::InUse(InUse::System) => S.hub_why_system.to_string(),
        Reason::BootOnly => S.hub_why_boot_only.to_string(),
        Reason::TooSmall { disk, image } => {
            fill(S.hub_why_too_small, &[&bytes(*disk), &bytes(*image)])
        }
        Reason::PayloadSum => S.hub_why_payload_sum.to_string(),
        Reason::ImageChanged { path, sha256 } => fill(S.hub_why_image_changed, &[path, sha256]),
        Reason::Gone => S.hub_why_gone.to_string(),
        Reason::Bus(d) => fill(S.hub_why_bus, &[d]),
        Reason::Unmounting(d) => fill(S.hub_why_unmounting, &[d]),
        Reason::OpenWrite(d) => fill(S.hub_why_open_write, &[d]),
        Reason::OpenRead(d) => fill(S.hub_why_open_read, &[d]),
        Reason::Device { device, detail } => fill(S.hub_why_device, &[device, detail]),
        Reason::Flush { device, detail } => fill(S.hub_why_flush, &[device, detail]),
        Reason::NotStick { device, fault } => {
            fill(S.hub_why_not_stick, &[device, &fault_text(fault)])
        }
        Reason::NotStickImage(fault) => fill(S.hub_why_not_stick_image, &[&fault_text(fault)]),
        Reason::BootLarger { image, stick } => {
            fill(S.hub_why_boot_larger, &[&bytes(*image), &bytes(*stick)])
        }
        Reason::BootLabel => S.hub_why_boot_label.to_string(),
        Reason::BootShort => S.hub_why_boot_short.to_string(),
        Reason::TableChanged(device) => fill(S.hub_why_table_changed, &[device]),
        Reason::ReadingTable(d) => fill(S.hub_why_reading_table, &[d]),
        Reason::BackEnds { device, at } => fill(S.hub_why_back_ends, &[device, &n(*at)]),
        Reason::BackDiffers { device, at } => fill(S.hub_why_back_differs, &[device, &n(*at)]),
        Reason::BackFailed { device, detail } => fill(S.hub_why_back_failed, &[device, detail]),
        Reason::SourceRead(d) => fill(S.hub_why_source_read, &[d]),
        Reason::WriteFailed(d) => fill(S.hub_why_write_failed, &[d]),
        Reason::PayloadChanged => S.hub_why_payload_changed.to_string(),
        Reason::Damaged(d) => fill(S.hub_why_damaged, &[d]),
        Reason::Longer(v) => fill(S.hub_why_longer, &[&n(*v)]),
        Reason::Shorter { expected, actual } => {
            fill(S.hub_why_shorter, &[&n(*actual), &n(*expected)])
        }
        Reason::WrongSum(sum) => fill(S.hub_why_wrong_sum, &[sum]),
        Reason::NoHome => S.hub_why_no_home.to_string(),
        Reason::NoDesktopApp => S.hub_why_no_desktop.to_string(),
        Reason::CacheSum(sum) => fill(S.hub_why_cache_sum, &[sum]),
        Reason::File { path, detail } => fill(S.hub_why_file, &[path, detail]),
        Reason::InTheWay { path } => fill(S.hub_why_in_the_way, &[path]),
        Reason::Start { path, detail } => fill(S.hub_why_start, &[path, detail]),
        Reason::Restart(d) => fill(S.hub_why_restart, &[d]),
        Reason::Missing(program) => fill(S.hub_why_missing, &[program]),
        Reason::BootEntry(d) => fill(S.hub_why_boot_entry, &[d]),
        Reason::NoStick => S.hub_why_no_stick.to_string(),
        Reason::Declined => S.hub_why_declined.to_string(),
        Reason::Unsupported => S.hub_why_unsupported.to_string(),
        Reason::NoAdmin => S.hub_why_no_admin.to_string(),
        Reason::Managed(code) => fill(S.hub_why_managed, &[&format!("0x{code:08X}")]),
        Reason::BitLocker { call, code } => {
            fill(S.hub_why_bitlocker, &[call, &format!("0x{code:08X}")])
        }
        Reason::NotEncrypted => S.hub_why_not_encrypted.to_string(),
        Reason::NotRead => S.hub_why_not_read.to_string(),
    }
}

fn fault_text(f: &StickFault) -> String {
    let bytes = |v: u64| fill(S.hub_why_bytes, &[&v.to_string()]);
    match f {
        StickFault::Reading(d) => fill(S.hub_fault_reading, &[d]),
        StickFault::NoHeader => S.hub_fault_no_header.to_string(),
        StickFault::NotGpt => S.hub_fault_not_gpt.to_string(),
        StickFault::HeaderDamaged => S.hub_fault_header_damaged.to_string(),
        StickFault::HeaderChecksum => S.hub_fault_header_checksum.to_string(),
        StickFault::EntriesDamaged => S.hub_fault_entries_damaged.to_string(),
        StickFault::EntriesPastEnd => S.hub_fault_entries_past_end.to_string(),
        StickFault::EntriesChecksum => S.hub_fault_entries_checksum.to_string(),
        StickFault::DiskUuid => S.hub_fault_disk_uuid.to_string(),
        StickFault::Partitions(n) => fill(S.hub_fault_partitions, &[&n.to_string()]),
        StickFault::BootName => S.hub_fault_boot_name.to_string(),
        StickFault::BootType => S.hub_fault_boot_type.to_string(),
        StickFault::DataType => S.hub_fault_data_type.to_string(),
        StickFault::BootDamaged => S.hub_fault_boot_damaged.to_string(),
        StickFault::BootSmall { boot, image } => {
            fill(S.hub_fault_boot_small, &[&bytes(*boot), &bytes(*image)])
        }
        StickFault::Overlap => S.hub_fault_overlap.to_string(),
        StickFault::BootPastEnd => S.hub_fault_boot_past_end.to_string(),
    }
}

/// A size as people read it: "16 GB", "7.8 GB", "512 MB".
pub(crate) fn size_text(bytes: u64) -> String {
    let gb = bytes as f64 / 1e9;
    if gb >= 10.0 {
        fill(S.hub_size_gb, &[&format!("{gb:.0}")])
    } else if gb >= 1.0 {
        fill(S.hub_size_gb, &[&format!("{gb:.1}")])
    } else {
        fill(S.hub_size_mb, &[&format!("{:.0}", bytes as f64 / 1e6)])
    }
}

/// A SHA-256 in groups of four.
fn grouped(hex: &str) -> String {
    faraday_core::ui::grouped(hex)
}

/// A disk's name: maker and model.
fn disk_name(d: &crate::api::Disk) -> String {
    let name = format!("{} {}", d.vendor.trim(), d.model.trim());
    let name = name.trim();
    if name.is_empty() {
        S.hub_disk_unnamed.to_string()
    } else {
        name.to_string()
    }
}

/// What is on a disk, in a few words (§3.2 step 1).
fn on_it(d: &crate::api::Disk) -> String {
    if let Some(f) = &d.faraday {
        return fill(S.hub_on_faraday, &[&version(f.release.as_deref())]);
    }
    if d.volumes.is_empty() {
        return S.hub_on_empty.to_string();
    }
    let names: Vec<&str> = d
        .volumes
        .iter()
        .filter_map(|v| v.label.as_deref())
        .collect();
    let n = d.volumes.len();
    let what = if names.is_empty() {
        if n == 1 {
            S.hub_on_volume.to_string()
        } else {
            fill(S.hub_on_volumes, &[&n.to_string()])
        }
    } else {
        names.join(", ")
    };
    let files = d.volumes.iter().any(|v| v.files);
    let tail = match (files, n == 1) {
        (true, true) => S.hub_on_files_it,
        (true, false) => S.hub_on_files_them,
        (false, _) => S.hub_on_no_files,
    };
    format!("{what}, {tail}")
}

/// Why a disk cannot be chosen, in one line.
fn refusal_text(r: &Refusal) -> String {
    match r {
        Refusal::InUse(InUse::Mounted(at)) => fill(S.hub_refused_mounted, &[at]),
        Refusal::InUse(InUse::Swap) => S.hub_refused_swap.to_string(),
        Refusal::InUse(InUse::System) => S.hub_refused_system.to_string(),
        Refusal::TooSmall(needs) => fill(S.hub_refused_small, &[&size_text(*needs)]),
    }
}

/// The line Confirm shows when the disk is no longer the one confirmed.
pub(crate) fn not_confirmed(target: Target) -> String {
    match target {
        Target::Stick => S.hub_not_confirmed_stick,
        Target::Pi => S.hub_not_confirmed_card,
    }
    .to_string()
}

// ---------------------------------------------------------------------
// Home
// ---------------------------------------------------------------------

/// What an item is called.
fn item_label(item: Item) -> &'static str {
    match item {
        Item::MakeStick => S.hub_item_make,
        Item::BitLocker => S.hub_item_bitlocker,
        Item::Start => S.hub_item_start,
        Item::SecureBootOn => S.hub_item_secure_boot,
        Item::Resume => S.hub_item_resume,
    }
}

/// What a state reads as.
fn state_text(state: &State) -> String {
    match state {
        State::NoStick => S.hub_state_no_stick.to_string(),
        State::Stick(v) => fill(S.hub_state_stick, &[v]),
        State::Protected => S.hub_state_protected.to_string(),
        State::Suspended => S.hub_state_suspended.to_string(),
        State::Resumed => S.hub_state_resumed.to_string(),
        State::Other => S.hub_state_other.to_string(),
        State::SecureBoot(on) => secure_boot_text(*on),
    }
}

fn secure_boot_text(on: Option<bool>) -> String {
    match on {
        Some(true) => S.hub_secure_boot_on,
        Some(false) => S.hub_secure_boot_off,
        None => S.hub_secure_boot_unknown,
    }
    .to_string()
}

fn home(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    // The title is the wordmark.
    if hub.compact {
        ui.mark(x, y, 20.0, 23.0);
        ui.wordmark_named(x + 28.0, y + 6.0, 11.0, TEXT, NameSuffix::Starter);
        y += 38.0;
    } else {
        ui.mark(x, y, 28.0, 32.0);
        ui.wordmark_named(x + 38.0, y + 11.0, 12.0, TEXT, NameSuffix::Starter);
        y += 56.0;
    }
    let facts = hub.facts();
    let next = facts.next_step();
    let rows = facts.checklist();
    y += computer_card(hub, ui, x, y, w) + 20.0;
    y += concern_block(hub, ui, x, y, w, &hub.restart_holds());
    y += receiver_line(hub, ui, x, y, w);

    // The checklist.
    y += label(ui, x, y, S.hub_checklist);
    ui.rule(x, y, w, INNER);
    for (i, row) in rows.iter().enumerate() {
        let action = HubAction::Item(row.item);
        let is_next = next == Some(row.item);
        let rh = 54.0;
        let pressed = ui.is_pressed(action.host());
        if is_next || pressed {
            let tint = if pressed {
                INNER
            } else {
                ACCENT.with_alpha(22)
            };
            ui.fill(x - 6.0, y + 2.0, w + 12.0, rh - 4.0, 8.0, tint);
        }
        ui.badge(
            x + 4.0,
            y + (rh - 26.0) / 2.0,
            &(i + 1).to_string(),
            row.done,
            is_next,
        );
        let tx = x + 44.0;
        let fg = if row.done { MUTED } else { TEXT };
        let head = ui.fit(14.0, W::S, item_label(row.item), w - 80.0);
        ui.text(tx, y + 8.0, 14.0, W::S, fg, &head);
        let mut state = state_text(&row.state);
        if row.item == Item::Resume && !row.offered {
            state = format!("{state} · {}", resume_needs(hub));
        }
        let state = ui.fit(12.0, W::R, &state, w - 80.0);
        ui.text(tx, y + 30.0, 12.0, W::R, MUTED, &state);
        ui.icon(
            x + w - 24.0,
            y + (rh - 20.0) / 2.0,
            20.0,
            Icon::ChevronRight,
            9.0,
            DIM,
        );
        ui.hit(x - 6.0, y, w + 12.0, rh, action.host());
        y += rh;
        ui.rule(x, y, w, INNER);
    }
    y += 24.0;

    // The firmware's Faraday stick entry, once back from a session.
    if hub.entry_offered() {
        let failed = hub.entry_remove_failed.as_ref().map(reason_text);
        let (sub, action) = if hub.entry_removing {
            (S.hub_removing_entry.to_string(), None)
        } else {
            (
                failed.unwrap_or_else(|| S.hub_remove_entry_sub.to_string()),
                Some(HubAction::RemoveEntry),
            )
        };
        y += nav_row(
            ui,
            x,
            y,
            w,
            Icon::Drive,
            S.hub_remove_entry,
            &sub,
            "",
            action,
        );
        y += 8.0;
    }

    // The other rows.
    let pi = hub.image(Target::Pi).is_some();
    y += nav_row(
        ui,
        x,
        y,
        w,
        Icon::SdCard,
        S.hub_row_pi,
        if pi { "" } else { S.hub_no_pi_image },
        "",
        pi.then_some(HubAction::Make(Target::Pi)),
    );
    y += nav_row(
        ui,
        x,
        y,
        w,
        Icon::Qr,
        S.hub_row_transfer,
        "",
        "",
        Some(HubAction::Transfer),
    );
    if hub.offers_desktop() {
        let failed = hub.desktop_failed.as_ref().map(reason_text);
        let sub = failed.as_deref().unwrap_or(S.hub_row_desktop_sub);
        y += nav_row(
            ui,
            x,
            y,
            w,
            Icon::Tools,
            S.hub_row_desktop,
            sub,
            "",
            Some(HubAction::TryDesktop),
        );
    }
    let (install, action) = match &hub.installed {
        Some(_) if hub.update_offered() => (S.hub_row_update, HubAction::Install),
        Some(_) => (S.hub_row_installed, HubAction::Installed),
        None => (S.hub_row_install, HubAction::Install),
    };
    y += nav_row(ui, x, y, w, Icon::Download, install, "", "", Some(action));
    y += nav_row(
        ui,
        x,
        y,
        w,
        Icon::Info,
        S.hub_row_about,
        "",
        "",
        Some(HubAction::About),
    );
    y += nav_row(
        ui,
        x,
        y,
        w,
        Icon::Learn,
        S.hub_row_learn,
        "",
        "",
        Some(HubAction::Learn),
    );
    y
}

/// The This computer card: what it is, its Secure Boot and encryption,
/// and the next step as its action. Returns its height.
fn computer_card(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let c = &hub.computer;
    let pad = if hub.compact { 12.0 } else { 18.0 };
    let iw = w - 2.0 * pad;
    let name = format!("{} {}", c.maker.trim(), c.model.trim());
    let name = if name.trim().is_empty() {
        S.hub_computer_unknown.to_string()
    } else {
        name.trim().to_string()
    };
    let mut lines: Vec<(String, Color)> = vec![(secure_boot_text(c.secure_boot), TEXT)];
    if c.os == Os::Windows
        && let Some(b) = &hub.bitlocker
    {
        let suspended = hub.facts().suspension_stands();
        let t = match b.protection {
            _ if suspended => S.hub_drive_suspended,
            Protection::None => S.hub_drive_plain,
            Protection::On => S.hub_drive_encrypted,
            Protection::Suspended => S.hub_drive_suspended_other,
            Protection::Unknown => S.hub_drive_unknown,
        };
        lines.push((fill(t, &[&b.drive]), TEXT));
    }
    if c.arm {
        lines.push((S.hub_cannot_start_arm.to_string(), WARN));
    } else if !c.uefi {
        lines.push((S.hub_cannot_start_bios.to_string(), WARN));
    }
    let card = hub.card();
    if card == Card::Firmware {
        // Why the card offers the firmware's settings, not the stick.
        if c.secure_boot == Some(false)
            && let Some(why) = &hub.stick_restart_failed
        {
            lines.push((fill(S.hub_stick_restart_failed, &[&reason_text(why)]), WARN));
        }
        if let Some(why) = &hub.restart_failed {
            lines.push((reason_text(why), ERR));
        }
    }
    let free = hub.restart_holds().is_empty();
    let button = Some(match card {
        Card::Make => (S.hub_item_make, true),
        Card::BitLocker => (S.hub_bitlocker_step, true),
        Card::Resume { offered } => (S.hub_item_resume, offered && hub.guard.busy.is_none()),
        Card::Firmware => (S.hub_restart, free),
        Card::Stick => (S.hub_restart_stick, free),
    });
    let linked = LearnId::SecureBootBitLocker.page().is_some();
    // The lines wrap: measured first, drawing nothing.
    let n = ui.hits.len();
    ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
    let lines_h: f32 = lines
        .iter()
        .map(|(l, _)| ui.wrap(0.0, 0.0, iw, 13.0, W::R, TEXT, l) + 4.0)
        .sum();
    ui.c.pop_clip();
    ui.hits.truncate(n);
    let mut h = pad + 20.0 + 26.0 + lines_h;
    if linked {
        h += 32.0;
    }
    if button.is_some() {
        h += 14.0 + 44.0;
    }
    h += pad;
    ui.card(x, y0, w, h, LINE);
    let mut y = y0 + pad;
    ui.text(x + pad, y, 12.0, W::S, MUTED, S.hub_computer);
    y += 20.0;
    let name = ui.fit(16.0, W::S, &name, iw);
    ui.text(x + pad, y, 16.0, W::S, TEXT, &name);
    y += 26.0;
    for (l, tone) in &lines {
        y += ui.wrap(x + pad, y, iw, 13.0, W::R, *tone, l) + 4.0;
    }
    y += link(ui, x + pad, y, iw, LearnId::SecureBootBitLocker);
    if let Some((label, offered)) = button {
        y += 14.0;
        let style = if offered {
            Style::Primary
        } else {
            Style::Disabled
        };
        let bw = if hub.compact { Some(iw) } else { None };
        let label = ui.fit(15.0, W::S, label, iw - 24.0);
        ui.button(x + pad, y, bw, 44.0, &label, style, HubAction::Next.host());
    }
    h
}

// ---------------------------------------------------------------------
// Make a Faraday stick, make a Pi card
// ---------------------------------------------------------------------

fn target_of(hub: &Hub) -> Target {
    hub.making.as_ref().map_or(Target::Stick, |m| m.target)
}

fn disks(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0 + title(hub, ui, x, y0);
    let target = target_of(hub);
    if target == Target::Pi {
        y += line(ui, x, y, w, S.hub_pi_for, MUTED);
        y += line(ui, x, y, w, S.hub_pi_cameras, MUTED);
    }
    if hub.image(target).is_none() {
        let s = match target {
            Target::Stick => S.hub_no_stick_image,
            Target::Pi => S.hub_no_pi_image,
        };
        return y + line(ui, x, y, w, s, ERR);
    }
    if hub.no_writer {
        y += line(ui, x, y, w, S.hub_no_writer, WARN);
    }
    let offers = hub.offers();
    if offers.is_empty() {
        let s = match target {
            Target::Stick => S.hub_no_disks_stick,
            Target::Pi => S.hub_no_disks_card,
        };
        return y + line(ui, x, y + 8.0, w, s, DIM);
    }
    y += 4.0;
    for o in &offers {
        let d = o.disk;
        let icon = match d.bus {
            crate::api::Bus::SdReader => Icon::SdCard,
            _ => Icon::Drive,
        };
        let (sub, action) = match &o.refused {
            Some(r) => (refusal_text(r), None),
            None if hub.no_writer => (on_it(d), None),
            None => (on_it(d), Some(HubAction::Disk(hub.disk_number(&d.id)))),
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

fn mode(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0 + title(hub, ui, x, y0);
    let target = target_of(hub);
    if let Some(d) = hub.chosen_disk() {
        let s = format!("{} · {}", disk_name(d), size_text(d.size));
        y += line(ui, x, y, w, &s, MUTED);
    }
    y += 4.0;
    let chosen = hub.making.as_ref().and_then(|m| m.mode);
    let whole = match target {
        Target::Stick => S.hub_mode_whole_stick,
        Target::Pi => S.hub_mode_whole_card,
    };
    let options = [
        (Mode::Whole, whole, S.hub_mode_whole_sub),
        (Mode::KeepData, S.hub_mode_keep, S.hub_mode_keep_sub),
    ];
    ui.rule(x, y, w, INNER);
    for (m, head, sub) in options {
        let refused = hub.mode_refusal(m);
        // Keep data is shown only on a Faraday stick (§3.2).
        if m == Mode::KeepData
            && matches!(refused, Some(ModeRefusal::NotFaraday | ModeRefusal::NotYet))
        {
            continue;
        }
        let action = HubAction::Mode(m).host();
        let on = chosen == Some(m);
        let enabled = refused.is_none();
        let rh = 60.0;
        if enabled && (ui.is_pressed(action) || ui.is_hovered(action)) {
            ui.fill(x - 6.0, y + 2.0, w + 12.0, rh - 4.0, 8.0, INNER);
        }
        // The check: a ring, filled when chosen.
        let cy = y + rh / 2.0;
        if on {
            ui.dot(x + 12.0, cy, 9.0, ACCENT);
            ui.dot(x + 12.0, cy, 4.0, ON_ACCENT);
        } else {
            ui.stroke(
                x + 3.0,
                cy - 9.0,
                18.0,
                18.0,
                9.0,
                if enabled { MUTED } else { INNER },
            );
        }
        let tx = x + 36.0;
        ui.text(
            tx,
            y + 10.0,
            15.0,
            W::S,
            if enabled { TEXT } else { DIM },
            head,
        );
        let (sub, tone) = match refused {
            Some(ModeRefusal::Same) => (S.hub_mode_same, WARN),
            Some(ModeRefusal::BootOnly) => (S.hub_mode_boot_only, WARN),
            _ => (sub, MUTED),
        };
        let sub = ui.fit(12.0, W::R, sub, w - 40.0);
        ui.text(tx, y + 34.0, 12.0, W::R, tone, &sub);
        if enabled {
            ui.hit(x - 6.0, y, w + 12.0, rh, action);
        }
        y += rh;
        // The one fact the Learn page explains (§4.4).
        if m == Mode::KeepData {
            y += ui.wrap(tx, y, w - 36.0, 12.0, W::R, MUTED, S.hub_mode_keep_fact) + 6.0;
            y += guide_line(ui, tx, y, w - 36.0, LearnId::KeepData);
        }
        ui.rule(x, y, w, INNER);
    }
    y += 20.0;
    let style = if chosen.is_some() {
        Style::Primary
    } else {
        Style::Disabled
    };
    y + buttons(ui, x, y, w, &[(S.hub_continue, style, HubAction::Continue)])
}

fn confirm(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0 + title(hub, ui, x, y0);
    let Some(m) = hub.making.as_ref() else {
        return y;
    };
    let target = m.target;
    let Some(image) = hub.image(target) else {
        return y;
    };
    let Some((id, device, serial, size)) = m.confirmed.as_ref() else {
        return y;
    };
    let mode = m.mode.unwrap_or(Mode::Whole);
    let disk = hub.disks.iter().find(|d| &d.id == id);
    let noun = match target {
        Target::Stick => S.hub_stick,
        Target::Pi => S.hub_card,
    };
    if let Some(d) = disk {
        y += value_row(ui, x, y, w, noun, &disk_name(d), TEXT);
    }
    y += value_row(ui, x, y, w, S.hub_size, &size_text(*size), TEXT);
    y += value_row(ui, x, y, w, S.hub_device, device, TEXT);
    let serial = if serial.is_empty() {
        S.hub_serial_none
    } else {
        serial.as_str()
    };
    y += value_row(ui, x, y, w, S.hub_serial, serial, TEXT);
    if let Some(d) = disk {
        y += value_row(ui, x, y, w, S.hub_on_it, &on_it(d), TEXT);
    }
    let writes = match (mode, target) {
        (Mode::KeepData, _) => S.hub_mode_keep,
        (Mode::Whole, Target::Stick) => S.hub_mode_whole_stick,
        (Mode::Whole, Target::Pi) => S.hub_mode_whole_card,
    };
    y += value_row(ui, x, y, w, S.hub_writes, writes, TEXT);
    let build = image
        .build
        .as_deref()
        .map_or_else(|| S.hub_build_unknown.to_string(), describe);
    y += value_row(ui, x, y, w, S.hub_build, &build, TEXT);
    let sha_key = if image.dev {
        S.hub_image_sha
    } else {
        S.hub_img_sha
    };
    y += mono_row(ui, x, y, w, sha_key, &grouped(&image.sha256));
    y += 12.0;
    if image.dev {
        y += warning(ui, x, y, w, S.hub_dev_image, WARN);
    }
    if mode == Mode::KeepData
        && let Some(stick) = disk.and_then(|d| d.faraday.as_ref())
    {
        match crate::decide::keep_data(stick, image.build.as_deref()) {
            Fit::Newer => {
                let s = fill(S.hub_keep_newer, &[&version(stick.release.as_deref())]);
                y += warning(ui, x, y, w, &s, WARN);
            }
            Fit::Dev => y += warning(ui, x, y, w, S.hub_keep_dev, WARN),
            _ => {}
        }
    }
    let matches = hub.confirmed_matches();
    if !matches {
        y += line(ui, x, y, w, &not_confirmed(target), ERR);
    } else if let Some(why) = &m.refused {
        y += line(ui, x, y, w, why, ERR);
    }
    if hub.no_writer {
        y += line(ui, x, y, w, S.hub_no_writer, WARN);
    }
    let label = match mode {
        Mode::Whole => S.hub_erase_and_write,
        Mode::KeepData => S.hub_write,
    };
    let style = if matches && !hub.no_writer {
        Style::Primary
    } else {
        Style::Disabled
    };
    y += 4.0;
    y + buttons(ui, x, y, w, &[(label, style, HubAction::Write)])
}

fn writing(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0 + title(hub, ui, x, y0);
    let Some(m) = hub.making.as_ref() else {
        return y;
    };
    if let Some(d) = m
        .confirmed
        .as_ref()
        .and_then(|c| hub.disks.iter().find(|d| d.id == c.0))
    {
        let s = format!("{} · {}", disk_name(d), size_text(d.size));
        y += line(ui, x, y, w, &s, MUTED);
    }
    let (stage, done, total) = m.progress.unwrap_or((WriteStage::Writing, 0, 0));
    y += 8.0;
    let t = if total == 0 {
        0.0
    } else {
        (done as f32 / total as f32).clamp(0.0, 1.0)
    };
    ui.fill(x, y, w, 10.0, 5.0, INNER);
    if t > 0.0 {
        ui.fill(x, y, (w * t).max(10.0), 10.0, 5.0, ACCENT);
    }
    y += 24.0;
    let of = fill(S.hub_of, &[&size_text(done), &size_text(total)]);
    let s = match stage {
        WriteStage::Writing => fill(S.hub_written_of, &[&of]),
        WriteStage::ReadingBack => fill(S.hub_reading_back, &[&of]),
    };
    y + line(ui, x, y, w, &s, TEXT)
}

fn done(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0;
    let Some(m) = hub.making.as_ref() else {
        return y;
    };
    let ok = matches!(m.result, Some(Ok(())));
    let (icon, tone) = if ok {
        (Icon::Success, OK)
    } else {
        (Icon::Error, ERR)
    };
    ui.icon(x - 4.0, y, 40.0, icon, 26.0, tone);
    let name = page_name(hub);
    ui.text_mid(x + 44.0, y, 40.0, 22.0, W::S, tone, &name);
    y += 56.0;
    match &m.result {
        Some(Ok(())) => {
            let s = match m.target {
                Target::Stick => S.hub_pull_stick,
                Target::Pi => S.hub_pull_card,
            };
            y += line(ui, x, y, w, s, TEXT) + 8.0;
            let items: &[(&str, Style, HubAction)] = match m.target {
                Target::Stick => &[
                    (S.hub_item_start, Style::Primary, HubAction::Start),
                    (S.hub_done, Style::Secondary, HubAction::Home),
                ],
                Target::Pi => &[(S.hub_done, Style::Primary, HubAction::Home)],
            };
            y += buttons(ui, x, y, w, items);
        }
        Some(Err(why)) => {
            y += line(ui, x, y, w, &reason_text(why), ERR) + 8.0;
            y += buttons(
                ui,
                x,
                y,
                w,
                &[
                    (S.hub_try_again, Style::Primary, HubAction::Again),
                    (S.hub_done, Style::Secondary, HubAction::Home),
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
            Some(HubAction::LearnPage(id)),
        ),
        None => line(ui, x, y, w, &fill(S.hub_guide_placeholder, &[title]), DIM),
    }
}

/// A guide's title, as its Learn page will carry it; a maker page's is
/// its page's own.
fn learn_title(id: LearnId) -> &'static str {
    match id {
        LearnId::StartWindows => S.hub_learn_start_windows,
        LearnId::StartLinux => S.hub_learn_start_linux,
        LearnId::BitLocker => S.hub_learn_bitlocker,
        LearnId::NoStart => S.hub_learn_no_start,
        LearnId::KeepData => S.hub_learn_keep_data,
        LearnId::Maker(_) => id.page().map_or("", |p| p.title),
        LearnId::SecureBootBitLocker => S.hub_learn_secure_boot_bitlocker,
        LearnId::ScreensThatRecord => S.hub_learn_screens,
        LearnId::BootEntry => S.hub_learn_boot_entry,
    }
}

fn start(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0 + title(hub, ui, x, y0);
    let c = &hub.computer;
    let facts = hub.facts();
    let windows = c.os == Os::Windows;
    if c.tpm_unlock {
        y += warning(ui, x, y, w, S.hub_tpm_line, WARN);
    }
    if c.arm {
        return y + line(ui, x, y, w, S.hub_cannot_start_arm, WARN);
    }
    if !c.uefi {
        y += warning(ui, x, y, w, S.hub_cannot_start_bios, WARN);
    }
    let held = hub.restart_holds();
    y += concern_block(hub, ui, x, y, w, &held);
    y += receiver_line(hub, ui, x, y, w);
    let guide = LearnId::for_computer(c);
    y += guide_line(ui, x, y, w, guide);
    y += link(ui, x, y, w, LearnId::SecureBootBitLocker);
    y += 8.0;
    let ix = x + 38.0;
    let iw = w - 38.0;
    let mut n = 0;
    // 1. BitLocker, before anything else.
    let open = facts.protected_open();
    if windows && (facts.protected() || facts.guard_done()) {
        n += 1;
        y += step_head(ui, x, y, w, n, S.hub_item_bitlocker, facts.guard_done());
        let state = if facts.suspension_stands() {
            State::Suspended
        } else if facts.resumed {
            State::Resumed
        } else {
            State::Protected
        };
        y += line(ui, ix, y, iw, &state_text(&state), MUTED);
        if open {
            y += buttons(
                ui,
                ix,
                y,
                iw,
                &[(S.hub_bitlocker_step, Style::Primary, HubAction::BitLocker)],
            ) + 16.0;
        }
        y += 8.0;
    }
    // 2. Restart into firmware settings.
    n += 1;
    y += step_head(ui, x, y, w, n, S.hub_restart, false);
    if let Some(why) = &hub.restart_failed {
        y += line(ui, ix, y, iw, &reason_text(why), ERR);
    }
    let style = if open || !held.is_empty() {
        Style::Disabled
    } else {
        Style::Secondary
    };
    y += buttons(ui, ix, y, iw, &[(S.hub_restart, style, HubAction::Restart)]) + 16.0;
    // 3. The firmware steps.
    n += 1;
    let sb_off = c.secure_boot == Some(false);
    let head = if sb_off {
        S.hub_secure_boot_already_off
    } else {
        S.hub_secure_boot_turn_off
    };
    y += step_head(ui, x, y, w, n, head, sb_off);
    y += line(ui, ix, y, iw, S.hub_fast_boot, MUTED) + 6.0;
    // 4. Start from the stick.
    n += 1;
    y += step_head(ui, x, y, w, n, S.hub_boot_from_stick, false);
    let how = if windows {
        S.hub_boot_how_windows
    } else {
        S.hub_boot_how
    };
    y += line(ui, ix, y, iw, how, MUTED) + 6.0;
    // 5. Afterwards, unless Secure Boot was off before the hub app came.
    let was_off = hub
        .bitlocker
        .as_ref()
        .and_then(|b| b.marker.as_ref())
        .is_some_and(|m| !m.secure_boot_was_on);
    if !was_off {
        n += 1;
        let s = if windows {
            S.hub_afterwards_windows
        } else {
            S.hub_afterwards
        };
        y += step_head(
            ui,
            x,
            y,
            w,
            n,
            s,
            c.secure_boot == Some(true) && facts.suspension_stands(),
        );
        y += line(ui, ix, y, iw, &secure_boot_text(c.secure_boot), MUTED) + 6.0;
    }
    if windows && facts.suspension_stands() {
        n += 1;
        y += step_head(ui, x, y, w, n, S.hub_item_resume, false);
        if facts.resume_offered() {
            y += buttons(
                ui,
                ix,
                y,
                iw,
                &[(S.hub_item_resume, Style::Primary, HubAction::BitLocker)],
            ) + 16.0;
        } else {
            y += line(ui, ix, y, iw, resume_needs(hub), MUTED);
        }
    }
    // The same page on a phone.
    y += 16.0;
    y += label(ui, x, y, S.hub_on_a_phone);
    let side = if ui.compact { 150.0 } else { 180.0 };
    y + phone_code(ui, x, y, w, side, &guide.address())
}

/// Why Resume BitLocker is not offered: Secure Boot is not as the
/// marker recorded it.
fn resume_needs(hub: &Hub) -> &'static str {
    let marker = hub.bitlocker.as_ref().and_then(|b| b.marker.as_ref());
    if marker.is_some_and(|m| m.this_boot) {
        return S.hub_resume_needs_restart;
    }
    let was_on = marker.is_none_or(|m| m.secure_boot_was_on);
    if was_on {
        S.hub_resume_needs_secure_boot
    } else {
        S.hub_resume_needs_secure_boot_off
    }
}

/// The guide's address as a QR code, with the address under it. Returns
/// its height.
fn phone_code(ui: &mut Ui, x: f32, y: f32, w: f32, side: f32, address: &str) -> f32 {
    let side = side.min(w);
    let Ok(m) = osk_codec::qr::encode(
        osk_codec::qr::Payload::Bytes(address.as_bytes()),
        osk_codec::qr::Ecc::Medium,
    ) else {
        return 0.0;
    };
    faraday_core::qr_code(ui, &m, x, y, side);
    let th = ui.wrap(x, y + side + 10.0, w, 12.0, W::M, MUTED, address);
    side + 10.0 + th + 8.0
}

/// A sheet: a card centred over the frosted page, as tall as what `body`
/// draws (measured first, drawing nothing).
fn sheet(ui: &mut Ui, w: f32, h: f32, body: &mut dyn FnMut(&mut Ui, f32, f32, f32) -> f32) {
    let compact = ui.compact;
    let sw = if compact {
        w - 2.0 * M
    } else {
        460.0f32.min(w - 48.0)
    };
    let pad = if compact { 14.0 } else { 24.0 };
    let n = ui.hits.len();
    ui.c.push_clip(osk_ui::Rect::new(0, 0, 0, 0));
    let bh = body(ui, 0.0, 0.0, sw - 2.0 * pad);
    ui.c.pop_clip();
    ui.hits.truncate(n);
    let sh = bh + 2.0 * pad;
    let sx = (w - sw) / 2.0;
    let sy = ((h - sh) / 2.0).max(8.0);
    ui.shadow(sx, sy, sw, sh, 14.0);
    ui.fill(sx, sy, sw, sh, 14.0, SURFACE);
    ui.stroke(sx, sy, sw, sh, 14.0, LINE);
    body(ui, sx + pad, sy + pad, sw - 2.0 * pad);
}

fn restart_sheet(hub: &Hub, ui: &mut Ui, w: f32, h: f32, to: RestartTo) {
    let address = LearnId::for_computer(&hub.computer).address();
    let os = hub.computer.os;
    let head = match to {
        RestartTo::Firmware => S.hub_restart,
        RestartTo::Stick => S.hub_restart_stick,
    };
    sheet(ui, w, h, &mut |ui, x, y0, iw| {
        let mut y = y0;
        y += ui.wrap(x, y, iw, 17.0, W::S, TEXT, head) + 10.0;
        y += line(ui, x, y, iw, S.hub_restart_save, WARN) + 4.0;
        if to == RestartTo::Stick && os == Os::Linux {
            y += link(ui, x, y, iw, LearnId::BootEntry);
        }
        // Small enough on a small panel that the sheet's buttons fit.
        let side = if ui.compact { 104.0 } else { 160.0 };
        y += phone_code(ui, x, y, iw, side, &address) + 8.0;
        y + buttons(
            ui,
            x,
            y,
            iw,
            &[
                (S.hub_cancel, Style::Secondary, HubAction::Cancel),
                (S.hub_restart_now, Style::Primary, HubAction::RestartNow),
            ],
        ) - y0
    });
}

/// Continue anyway, asked once, naming the item (§3.1a).
fn let_through_sheet(hub: &Hub, ui: &mut Ui, w: f32, h: f32) {
    let Some(c) = hub
        .asking
        .as_ref()
        .and_then(|id| hub.concerns.iter().find(|c| &c.id == id))
    else {
        return;
    };
    let name = if c.name.trim().is_empty() {
        concern_text(c)
    } else {
        c.name.trim().to_string()
    };
    let ask = fill(S.hub_let_through_ask, &[&name]);
    let line_text = concern_text(c);
    sheet(ui, w, h, &mut |ui, x, y0, iw| {
        let mut y = y0;
        y += ui.wrap(x, y, iw, 17.0, W::S, TEXT, &ask) + 10.0;
        y += line(ui, x, y, iw, &line_text, WARN);
        y += line(ui, x, y, iw, S.hub_let_through_until, MUTED) + 4.0;
        y + buttons(
            ui,
            x,
            y,
            iw,
            &[
                (S.hub_cancel, Style::Secondary, HubAction::Cancel),
                (S.hub_continue_anyway, Style::Primary, HubAction::LetThrough),
            ],
        ) - y0
    });
}

// ---------------------------------------------------------------------
// Install, Installed, About, BitLocker
// ---------------------------------------------------------------------

fn install(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0 + title(hub, ui, x, y0);
    if hub.update_offered()
        && let Some(i) = &hub.installed
    {
        y += value_row(ui, x, y, w, S.hub_installed_version, &i.version, TEXT);
        y += value_row(ui, x, y, w, S.hub_this_version, crate::VERSION, TEXT);
        y += 12.0;
    }
    if let Some(Ok(place)) = &hub.install_result {
        y += line(ui, x, y, w, &fill(S.hub_installed_in, &[place]), OK) + 8.0;
        return y + buttons(
            ui,
            x,
            y,
            w,
            &[(S.hub_done, Style::Primary, HubAction::Home)],
        );
    }
    if hub.payload_file(PayloadKind::Desktop).is_some() {
        let rows = [(
            S.hub_install_desktop.to_string(),
            hub.install_desktop,
            !hub.installing,
            HubAction::InstallDesktop.host(),
        )];
        // The label is long: on a small panel, wrapped beside the box.
        if hub.compact {
            let action = rows[0].3;
            let th = ui.wrap(x + 32.0, y + 2.0, w - 32.0, 14.0, W::R, TEXT, &rows[0].0);
            ui.checkbox(x + 2.0, y + 2.0, hub.install_desktop, !hub.installing);
            ui.hit(x, y - 6.0, w, th + 12.0, action);
            y += th + 18.0;
        } else {
            y += ui.multi_list(x, y, w, &rows) + 16.0;
        }
    }
    if let Some(Err(why)) = &hub.install_result {
        y += line(ui, x, y, w, &reason_text(why), ERR);
    }
    let (label, style) = if hub.installing {
        (S.hub_installing, Style::Disabled)
    } else {
        (S.hub_install_now, Style::Primary)
    };
    y + buttons(ui, x, y, w, &[(label, style, HubAction::InstallNow)])
}

fn item_name(item: InstallItem) -> &'static str {
    match item {
        InstallItem::App => S.hub_installed_app,
        InstallItem::Desktop => S.hub_installed_desktop,
        InstallItem::Menu => S.hub_installed_menu,
        InstallItem::DesktopMenu => S.hub_installed_desktop_menu,
        InstallItem::Icon => S.hub_installed_icon,
        InstallItem::UninstallEntry => S.hub_installed_entry,
        InstallItem::Reminder => S.hub_installed_reminder,
    }
}

fn installed(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0 + title(hub, ui, x, y0);
    if let Some(why) = &hub.entry_kept {
        y += line(
            ui,
            x,
            y,
            w,
            &fill(S.hub_entry_kept, &[&reason_text(why)]),
            WARN,
        );
    }
    let Some(i) = &hub.installed else {
        return y + line(ui, x, y, w, S.hub_not_installed, DIM);
    };
    y += value_row(ui, x, y, w, S.hub_installed_version, &i.version, TEXT);
    for (item, place) in &i.items {
        y += mono_row(ui, x, y, w, item_name(*item), place);
    }
    y += 16.0;
    if let Some(why) = &hub.uninstall_failed {
        y += line(ui, x, y, w, &reason_text(why), ERR);
    }
    y + buttons(
        ui,
        x,
        y,
        w,
        &[(S.hub_uninstall, Style::Secondary, HubAction::Uninstall)],
    )
}

/// Uninstall, asked once. While the hub app's BitLocker suspension
/// stands (§5.4) it first offers Resume BitLocker where Resume can be
/// taken now, and otherwise says that protection stays suspended and
/// where Windows resumes it; uninstalling removes the marker and the
/// logon reminder either way.
fn uninstall_sheet(hub: &Hub, ui: &mut Ui, w: f32, h: f32) {
    let f = hub.facts();
    let stands = f.suspension_stands();
    let resume = stands && f.resume_offered();
    sheet(ui, w, h, &mut |ui, x, y0, iw| {
        let mut y = y0;
        y += ui.wrap(x, y, iw, 17.0, W::S, TEXT, S.hub_uninstall_ask) + 10.0;
        y += line(ui, x, y, iw, S.hub_uninstall_keeps, MUTED);
        if resume {
            y += line(ui, x, y, iw, S.hub_uninstall_suspended, WARN);
        } else if stands {
            y += line(ui, x, y, iw, S.hub_uninstall_suspended_where, WARN);
        }
        y += 6.0;
        let items: &[(&str, Style, HubAction)] = if resume {
            &[
                (S.hub_cancel, Style::Secondary, HubAction::Cancel),
                (S.hub_item_resume, Style::Primary, HubAction::Resume),
                (S.hub_uninstall, Style::Secondary, HubAction::UninstallNow),
            ]
        } else {
            &[
                (S.hub_cancel, Style::Secondary, HubAction::Cancel),
                (S.hub_uninstall, Style::Primary, HubAction::UninstallNow),
            ]
        };
        y + buttons(ui, x, y, iw, items) - y0
    });
}

/// The release key's fingerprint, as the README prints it.
const FINGERPRINT: &str = "D1AF A712 ABEF 9010 9A1B  587F F153 6BE0 7E23 4CFD";

fn about(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0 + title(hub, ui, x, y0);
    y += value_row(ui, x, y, w, S.hub_version, crate::VERSION, TEXT);
    match &hub.payload {
        Some(p) => {
            y += value_row(ui, x, y, w, S.hub_release, &p.version, TEXT);
            y += mono_row(ui, x, y, w, S.hub_commit, &p.commit);
            y += 16.0;
            y += label(ui, x, y, S.hub_payload);
            for f in &p.files {
                y += 4.0;
                y += ui.wrap(x, y, w, 14.0, W::S, TEXT, &f.name) + 4.0;
                y += value_row(ui, x, y, w, S.hub_build, &describe(&f.build_id), TEXT);
                y += mono_row(ui, x, y, w, S.hub_sha_xz, &grouped(&f.sha256));
                let inner = fill(S.hub_sha_inner, &[&f.inner]);
                y += mono_row(ui, x, y, w, &inner, &grouped(&f.inner_sha256));
                y += 8.0;
            }
        }
        None => y += line(ui, x, y + 8.0, w, S.hub_no_payload, DIM) + 8.0,
    }
    if let Some(d) = &hub.dev {
        y += 8.0;
        y += warning(ui, x, y, w, S.hub_dev_image, WARN);
        y += mono_row(ui, x, y, w, S.hub_image_file, &d.path);
        y += mono_row(ui, x, y, w, S.hub_image_sha, &grouped(&d.sha256));
    }
    y += 16.0;
    y += label(ui, x, y, S.hub_release_key);
    y += ui.wrap(x, y, w, 13.0, W::M, TEXT, FINGERPRINT) + 18.0;
    y += label(ui, x, y, S.hub_check_title);
    y += line(ui, x, y, w, S.hub_check_text, TEXT);
    for cmd in [
        S.hub_check_cmd_import,
        S.hub_check_cmd_verify,
        S.hub_check_cmd_sums,
    ] {
        y += ui.wrap(x, y, w, 12.0, W::M, MUTED, cmd) + 6.0;
    }
    if hub.computer.os == Os::Windows {
        y += 12.0;
        y += line(ui, x, y, w, S.hub_check_windows, TEXT);
        y += label(ui, x, y + 8.0, S.hub_unsigned_title) + 8.0;
        y += line(ui, x, y, w, S.hub_unsigned_text, TEXT);
    }
    y
}

/// The BitLocker guard (§6.5): the state; Show the recovery key; the
/// key, its QR code and Save; I have kept it; the key hidden and two
/// groups typed back; Suspend BitLocker; Done. The same page resumes
/// later (§6.3). Another encryption product's line comes first (§6.4).
fn bitlocker(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let mut y = y0 + title(hub, ui, x, y0);
    let f = hub.facts();
    let g = &hub.guard;
    if hub.computer.os == Os::Windows
        && let Some(name) = &hub.other
    {
        let found = fill(S.hub_other_found, &[name]);
        if f.other_open {
            y += warning(ui, x, y, w, &found, WARN);
            y += buttons(
                ui,
                x,
                y,
                w,
                &[(S.hub_other_confirm, Style::Secondary, HubAction::Rescue)],
            ) + 20.0;
        } else {
            y += line(ui, x, y, w, &found, MUTED) + 6.0;
        }
    }
    let stands = f.suspension_stands();
    let state = match &hub.bitlocker {
        None => S.hub_drive_unread.to_string(),
        Some(b) => match b.protection {
            _ if stands => fill(S.hub_drive_suspended, &[&b.drive]),
            Protection::On => fill(S.hub_guard_encrypted, &[&b.drive]),
            Protection::None => fill(S.hub_drive_plain, &[&b.drive]),
            Protection::Suspended => fill(S.hub_drive_suspended_other, &[&b.drive]),
            Protection::Unknown => fill(S.hub_drive_unknown, &[&b.drive]),
        },
    };
    y += ui.wrap(x, y, w, 16.0, W::S, TEXT, &state) + 14.0;
    let undone = hub
        .bitlocker
        .as_ref()
        .is_some_and(|b| b.marker.is_some() && b.protection == Protection::On);
    if undone {
        y += warning(ui, x, y, w, S.hub_guard_undone, WARN);
    } else if hub.resumed && !stands {
        y += line(ui, x, y, w, S.hub_guard_resumed, OK);
    }
    if let Some(why) = &g.failed {
        y += line(ui, x, y, w, &reason_text(why), ERR);
    }
    let holds = hub.key_holds();
    if stands {
        y += guard_done(hub, ui, x, y, w);
    } else if f.protected() && !f.other_open && !holds.is_empty() && !g.hidden {
        // Nothing secret on a screen that can keep it.
        y += concern_block(hub, ui, x, y, w, &holds);
    } else if f.protected() && !f.other_open {
        y += match (&g.key, g.hidden) {
            (None, _) => {
                let (label, style) = if g.busy == Some(crate::api::BitLockerStep::ShowKey) {
                    (S.hub_waiting_permission, Style::Disabled)
                } else {
                    (S.hub_show_key, Style::Primary)
                };
                buttons(ui, x, y, w, &[(label, style, HubAction::ShowKey)]) + 16.0
            }
            (Some(k), false) => key_shown(hub, ui, x, y, w, k),
            (Some(_), true) => key_asked(hub, ui, x, y, w),
        };
    }
    y += 12.0;
    y += link(ui, x, y, w, LearnId::SecureBootBitLocker);
    y + guide_line(ui, x, y, w, LearnId::BitLocker)
}

/// The suspension stands: Done's lines, then Start and Resume, whichever
/// comes next first. Returns its height.
fn guard_done(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let f = hub.facts();
    let mut y = y0;
    y += line(ui, x, y, w, S.hub_guard_done, TEXT);
    if hub.installed.is_none() {
        y += line(ui, x, y, w, S.hub_guard_open_again, TEXT);
    }
    y += 8.0;
    let resuming = hub.guard.busy == Some(crate::api::BitLockerStep::Resume);
    let resume = if resuming {
        (S.hub_resuming, Style::Disabled, HubAction::Resume)
    } else {
        (S.hub_item_resume, Style::Primary, HubAction::Resume)
    };
    let start = (S.hub_item_start, Style::Primary, HubAction::Start);
    if !f.resume_offered() {
        y += line(ui, x, y, w, resume_needs(hub), MUTED);
        return y + buttons(ui, x, y, w, &[start]) + 16.0 - y0;
    }
    let items = if hub.suspended_here {
        // Just suspended: the firmware is still ahead.
        let quiet = if resuming {
            Style::Disabled
        } else {
            Style::Secondary
        };
        [start, (resume.0, quiet, resume.2)]
    } else {
        [resume, (start.0, Style::Secondary, start.2)]
    };
    y + buttons(ui, x, y, w, &items) + 16.0 - y0
}

/// The key's eight groups in a secret panel, each under its number;
/// masked, the same panel with dots. Returns its height.
fn key_panel(ui: &mut Ui, x: f32, y: f32, w: f32, key: Option<&RecoveryKey>) -> f32 {
    let cols = if w >= 380.0 { 4 } else { 2 };
    let rows = 8 / cols;
    let size = if ui.compact { 17.0 } else { 20.0 };
    let pad = 14.0;
    let cw = (w - 2.0 * pad) / cols as f32;
    let rh = 18.0 + ui.line(size, W::M) + 8.0;
    let h = 2.0 * pad + rows as f32 * rh - 8.0;
    ui.fill(x, y, w, h, 10.0, BG);
    let edge = if key.is_some() {
        WARN
    } else {
        WARN.with_alpha(110)
    };
    ui.stroke(x, y, w, h, 10.0, edge);
    for i in 0..8 {
        let (c, r) = (i % cols, i / cols);
        let cx = x + pad + c as f32 * cw;
        let cy = y + pad + r as f32 * rh;
        ui.text(cx, cy, 11.0, W::R, MUTED, &(i + 1).to_string());
        match key {
            Some(k) => ui.text(cx, cy + 16.0, size, W::M, TEXT, k.group(i)),
            None => ui.text(cx, cy + 16.0, size, W::M, MUTED, "••••••"),
        };
    }
    h + 12.0
}

/// The key shown: the panel, its ID, its QR code, Save where a disk for
/// it is attached, and I have kept it. Returns its height.
fn key_shown(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32, k: &KeyShown) -> f32 {
    let mut y = y0;
    y += label(ui, x, y, S.hub_recovery_key);
    y += key_panel(ui, x, y, w, Some(&k.text.key));
    y += value_row(ui, x, y, w, S.hub_key_id, &k.key_id(), TEXT) + 16.0;
    if let Some(text) = k.text.write()
        && let Ok(m) = osk_codec::qr::encode(
            osk_codec::qr::Payload::Bytes(text.as_bytes()),
            osk_codec::qr::Ecc::Medium,
        )
    {
        let side = if ui.compact { 180.0f32 } else { 220.0 }.min(w);
        faraday_core::qr_code(ui, &m, x, y, side);
        y += side + 10.0;
    }
    y += line(ui, x, y, w, S.hub_key_photo, MUTED) + 6.0;
    for d in hub.save_targets() {
        let name = fill(S.hub_key_save, &[&disk_name(d)]);
        let (label, style) = if hub.guard.saving {
            (S.hub_key_saving.to_string(), Style::Disabled)
        } else {
            (name, Style::Secondary)
        };
        let n = HubAction::SaveKey(hub.disk_number(&d.id));
        y += buttons(ui, x, y, w, &[(&label, style, n)]) + 10.0;
    }
    match &hub.guard.saved {
        Some(Ok(at)) => y += line(ui, x, y, w, &fill(S.hub_key_saved, &[at]), OK),
        Some(Err(why)) => y += line(ui, x, y, w, &reason_text(why), ERR),
        None => {}
    }
    y += 6.0;
    y + buttons(
        ui,
        x,
        y,
        w,
        &[(S.hub_key_kept, Style::Primary, HubAction::KeptIt)],
    ) + 16.0
        - y0
}

/// The key hidden and two groups asked for, then Suspend BitLocker.
/// Returns its height.
fn key_asked(hub: &Hub, ui: &mut Ui, x: f32, y0: f32, w: f32) -> f32 {
    let g = &hub.guard;
    let mut y = y0;
    y += label(ui, x, y, S.hub_recovery_key);
    y += key_panel(ui, x, y, w, None);
    let Some([a, b]) = g.asked else {
        return y - y0;
    };
    let ask = fill(
        S.hub_key_type,
        &[&(a + 1).to_string(), &(b + 1).to_string()],
    );
    y += ui.wrap(x, y, w, 15.0, W::S, TEXT, &ask) + 12.0;
    let fw = ((w - 12.0) / 2.0).min(180.0);
    for (i, n) in [a, b].into_iter().enumerate() {
        let fx = x + i as f32 * (fw + 12.0);
        ui.text(
            fx,
            y,
            12.0,
            W::R,
            MUTED,
            &fill(S.hub_key_group, &[&(n + 1).to_string()]),
        );
        let by = y + 18.0;
        let focused = g.focus == i && g.busy.is_none();
        ui.fill(fx, by, fw, 44.0, 8.0, BG);
        ui.stroke(fx, by, fw, 44.0, 8.0, if focused { ACCENT } else { BORDER });
        let typed = g.typed(i);
        ui.text_mid(fx + 12.0, by, 44.0, 18.0, W::M, TEXT, typed);
        if focused {
            let tw = ui.measure(18.0, W::M, typed);
            ui.caret(fx + 13.0 + tw, by + 11.0, 22.0);
        }
        ui.hit(fx, by, fw, 44.0, HubAction::Group(i as u8).host());
    }
    y += 18.0 + 44.0 + 14.0;
    if let Some(n) = g.wrong {
        y += line(
            ui,
            x,
            y,
            w,
            &fill(S.hub_key_wrong, &[&(n + 1).to_string()]),
            ERR,
        );
    }
    let suspend = if g.busy == Some(crate::api::BitLockerStep::Suspend) {
        (S.hub_suspending, Style::Disabled, HubAction::Suspend)
    } else {
        (S.hub_suspend, Style::Primary, HubAction::Suspend)
    };
    y + buttons(
        ui,
        x,
        y,
        w,
        &[
            (S.hub_key_show_again, Style::Secondary, HubAction::ShowAgain),
            suspend,
        ],
    ) + 16.0
        - y0
}
