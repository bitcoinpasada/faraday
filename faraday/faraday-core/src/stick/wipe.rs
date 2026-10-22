//! Deleting from a stick (owner, 2026-10-11): one way in, the stick
//! visit's **Delete files**, whose sheet chooses what — chosen files, the
//! free space, or the whole files partition — and says before anything is
//! done what cannot be undone, what is written, that nothing outside the
//! files partition is, why, and what zeros do not reach.
//!
//! The disk process does the work. A file is deleted with zeros over its
//! clusters, read back, then removed with its name
//! (`faraday_fat::Volume::wipe`). The free space is zeroed in steps, one a
//! tick, so the screen draws between them (`Volume::zero_free`); the whole
//! partition is first made an empty volume with its layout and label
//! (`Volume::make_empty`), then zeroed the same way, once per pass.
//!
//! What a USB stick or an SD card keeps below its blocks is out of reach:
//! its controller writes a block's new bytes elsewhere in the chip and
//! may keep the old ones until it reuses the place.

use crate::drawing::View;
use crate::strings::EN;
use crate::strings::fill;
use faraday_draw::widgets::Icon;

use crate::compact::{buttons, scroll_sheet};
use crate::screens::wrap_buttons;
use crate::ui::pal::*;
use crate::ui::{Style, Ui, W};
use crate::{Action, Faraday, Sheet, StorageCommand};

/// What zeros do not reach.
const LIMIT: &str = EN.stick_wipe_no_zeroing;
/// What the partition holds of Faraday's.
const REACH: &str = EN.stick_wipe_faraday;
/// What only destroying the stick removes.
const SECRET: &str = EN.stick_wipe_if_a_seed;
/// That nothing outside the files partition is written, and why.
const WITHIN: &str = EN.stick_wipe_only_this;
const BOUNDARY: &str = EN.stick_wipe_why_the;

/// Whether a file on a stick is a vault, by its name.
fn is_vault_name(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".ofv")
}

/// How fast a typical stick is written and read back, MB a second, for
/// the time the sheet says before the first step measures it.
const TYPICAL_MBPS: u64 = 6;

/// What the sheet deletes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum What {
    /// The files ticked.
    Files,
    /// Every part of the files partition no file uses.
    Free,
    /// Every file, then all of the files partition.
    Whole,
}

impl What {
    /// By its place on the sheet.
    pub fn at(k: u8) -> What {
        match k {
            0 => What::Files,
            1 => What::Free,
            _ => What::Whole,
        }
    }
}

/// The sheet: which stick, what is chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteAsk {
    stick: String,
    label: String,
    /// What is deleted.
    pub what: What,
    /// The files ticked, by name.
    pub chosen: Vec<String>,
    /// For the whole partition: one pass or two.
    pub passes: u8,
    /// Free and all bytes, once the stick says.
    space: Option<(u64, u64)>,
}

/// Zeroing under way, from the sheet's button to the last step's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zeroing {
    stick: String,
    label: String,
    /// The whole partition, made empty first.
    pub whole: bool,
    passes: u8,
    pass: u8,
    /// The next step's first cluster, of the volume's.
    next: u32,
    total: u32,
    /// The next step is asked for on the next tick.
    due: bool,
    /// Stop was pressed: no step after the one under way.
    stop: bool,
    /// When this pass's first answer came, and where it stood then.
    since: Option<(u64, u32)>,
    /// The partition was made empty: the first answer came.
    emptied: bool,
    /// The stick's volume serial, to know its copies by once it is gone.
    serial: Option<u32>,
}

impl Zeroing {
    /// How far, of every pass, from 0 to 1.
    fn done_share(&self) -> f32 {
        let pass = if self.total == 0 {
            0.0
        } else {
            (self.next.saturating_sub(2)) as f32 / self.total as f32
        };
        ((f32::from(self.pass.saturating_sub(1)) + pass.min(1.0)) / f32::from(self.passes.max(1)))
            .min(1.0)
    }
}

/// Bytes as a person reads them: GB, MB or KB.
fn size_text(b: u64) -> String {
    if b >= 1 << 30 {
        fill(
            EN.stick_wipe_gb,
            &[&format!("{:.1}", b as f64 / f64::from(1u32 << 30))],
        )
    } else if b >= 1 << 20 {
        fill(EN.stick_wipe_mb, &[&(b >> 20).to_string()])
    } else {
        fill(EN.stick_wipe_kb, &[&((b >> 10).max(1)).to_string()])
    }
}

/// Minutes, at least one, as words.
fn minutes_text(ms: u64) -> String {
    match ms.div_ceil(60_000).max(1) {
        1 => EN.stick_wipe_about_a.to_string(),
        m if m < 90 => fill(EN.stick_wipe_about_minutes, &[&m.to_string()]),
        m => fill(
            EN.stick_wipe_about_hours,
            &[&format!("{:.1}", m as f64 / 60.0)],
        ),
    }
}

impl Faraday {
    /// The stick visit's Delete files: the sheet, on chosen files, with
    /// the stick asked for its space.
    pub(crate) fn delete_open(&mut self) {
        let Some(stick) = self.sticks.get(self.visit.stick) else {
            return;
        };
        self.storage_out.push_back(StorageCommand::StickSpace {
            stick: stick.id.clone(),
        });
        self.delete_ask = Some(DeleteAsk {
            stick: stick.id.clone(),
            label: stick.label.clone(),
            what: What::Files,
            chosen: Vec::new(),
            passes: 1,
            space: None,
        });
        self.sheet = Some(Sheet::Delete);
    }

    /// Whether the sheet's button is a hold: a vault is among the files
    /// chosen (`docs/FLOWS.md` decisions 4 and 61).
    pub(crate) fn delete_holds(&self) -> bool {
        self.delete_ask
            .as_ref()
            .is_some_and(|d| d.what == What::Files && d.chosen.iter().any(|f| is_vault_name(f)))
    }

    /// The sheet's choice of what.
    pub(crate) fn delete_what(&mut self, k: u8) {
        if let Some(d) = self.delete_ask.as_mut() {
            d.what = What::at(k);
        }
    }

    /// A file ticked or unticked, by its place on the stick.
    pub(crate) fn delete_pick(&mut self, k: usize) {
        let name = self
            .sticks
            .get(self.visit.stick)
            .and_then(|s| s.files.get(k))
            .map(|(n, _)| n.clone());
        let (Some(d), Some(name)) = (self.delete_ask.as_mut(), name) else {
            return;
        };
        d.what = What::Files;
        match d.chosen.iter().position(|n| *n == name) {
            Some(p) => {
                d.chosen.remove(p);
            }
            None => d.chosen.push(name),
        }
    }

    /// One pass or two, for the whole partition.
    pub(crate) fn delete_passes(&mut self, n: u8) {
        if let Some(d) = self.delete_ask.as_mut() {
            d.passes = n.clamp(1, 2);
        }
    }

    /// The sheet's button: the files deleted, or zeroing begun.
    pub(crate) fn delete_go(&mut self) {
        let Some(d) = self.delete_ask.take() else {
            return;
        };
        match d.what {
            What::Files => {
                for name in d.chosen {
                    self.storage_out.push_back(StorageCommand::Wipe {
                        stick: d.stick.clone(),
                        name,
                    });
                }
                self.sheet = None;
            }
            What::Free | What::Whole => {
                let whole = d.what == What::Whole;
                self.storage_out.push_back(if whole {
                    StorageCommand::MakeEmpty {
                        stick: d.stick.clone(),
                    }
                } else {
                    StorageCommand::ZeroFree {
                        stick: d.stick.clone(),
                        from: 2,
                    }
                });
                let serial = self
                    .sticks
                    .iter()
                    .find(|s| s.id == d.stick)
                    .and_then(|s| s.serial);
                self.zeroing = Some(Zeroing {
                    serial,
                    stick: d.stick,
                    label: d.label,
                    whole,
                    passes: if whole { d.passes } else { 1 },
                    pass: 1,
                    next: 2,
                    total: 0,
                    due: false,
                    stop: false,
                    since: None,
                    emptied: false,
                });
                self.sheet = Some(Sheet::Zeroing);
            }
        }
    }

    /// A file's delete answered.
    pub(crate) fn wiped(&mut self, stick: &str, name: &str, failed: Option<&str>) {
        match failed {
            None => {
                self.flow_copy_gone(stick, Some(name));
                self.visit.inn.retain(|n| n != name);
                self.visit
                    .log
                    .push((fill(EN.stick_wipe_deleted, &[name]), true));
            }
            Some(why) => self
                .visit
                .log
                .push((fill(EN.stick_wipe_not_deleted, &[name, why]), false)),
        }
    }

    /// The stick's space, for the sheet.
    pub(crate) fn zero_space(&mut self, free: u64, size: u64) {
        if let Some(d) = self.delete_ask.as_mut() {
            d.space = Some((free, size));
        }
    }

    /// Stop: no step after the one under way.
    pub(crate) fn zero_stop(&mut self) {
        let Some(z) = self.zeroing.as_mut() else {
            return;
        };
        if z.due {
            let line = fill(
                EN.stick_wipe_stopped_at,
                &[&((z.done_share() * 100.0) as u32).to_string()],
            );
            self.zero_end(line, false);
        } else {
            z.stop = true;
        }
    }

    /// A step's answer: the next on the next tick, the next pass, or the
    /// end.
    pub(crate) fn zero_step(&mut self, next: u32, total: u32, done: bool) {
        let now = self.now_ms;
        // The partition is empty from the first answer, whatever follows:
        // the copies the backup flow wrote on it are gone.
        if let Some(z) = self.zeroing.as_mut().filter(|z| z.whole && !z.emptied) {
            z.emptied = true;
            let (stick, label, serial) = (z.stick.clone(), z.label.clone(), z.serial);
            self.flow_copy_gone_for(&stick, &label, serial, None);
        }
        let Some(z) = self.zeroing.as_mut() else {
            return;
        };
        z.next = next;
        z.total = z.total.max(total);
        if z.since.is_none() && total > 0 {
            z.since = Some((now, next));
        }
        let share = (z.done_share() * 100.0) as u32;
        if z.stop {
            self.zero_end(fill(EN.stick_wipe_stopped_at, &[&share.to_string()]), false);
            return;
        }
        if done && z.pass < z.passes {
            z.pass += 1;
            z.next = 2;
            z.since = None;
            z.due = true;
        } else if done {
            let line = match (z.whole, z.passes) {
                (false, _) => fill(EN.stick_wipe_free_space, &[&z.label.to_string()]),
                (true, 1) => fill(EN.stick_wipe_s_partition_zeroed, &[&z.label.to_string()]),
                (true, n) => fill(
                    EN.stick_wipe_s_partition,
                    &[&z.label.to_string(), &n.to_string()],
                ),
            };
            self.zero_end(line, true);
        } else {
            z.due = true;
        }
    }

    /// Asks for the next step, once a tick has let a frame draw.
    pub(crate) fn zero_tick(&mut self) {
        let Some(z) = self.zeroing.as_mut().filter(|z| z.due) else {
            return;
        };
        z.due = false;
        self.storage_out.push_back(StorageCommand::ZeroFree {
            stick: z.stick.clone(),
            from: z.next,
        });
        self.dirty = true;
        self.commands.push_back(faraday_shell_api::Command::Draw);
    }

    /// Zeroing failed, or the stick's space could not be read.
    pub(crate) fn zero_failed(&mut self, why: &str) {
        // A whole wipe that fails before its first answer, or whose stick
        // is pulled, may have emptied the FATs: its copies are not trusted
        // (fail closed).
        if let Some(z) = self.zeroing.as_ref().filter(|z| z.whole && !z.emptied) {
            let (stick, label, serial) = (z.stick.clone(), z.label.clone(), z.serial);
            self.flow_copy_gone_for(&stick, &label, serial, None);
        }
        if let Some(z) = self.zeroing.as_ref() {
            let share = (z.done_share() * 100.0) as u32;
            self.zero_end(
                fill(EN.stick_wipe_stopped, &[&share.to_string(), why]),
                false,
            );
        }
    }

    /// The sticks changed: the stick of the sheet or of zeroing gone ends
    /// them.
    pub(crate) fn zero_sticks_changed(&mut self) {
        let here = |id: &str| self.sticks.iter().any(|s| s.id == id);
        if self.delete_ask.as_ref().is_some_and(|d| !here(&d.stick)) {
            self.delete_ask = None;
            if self.sheet == Some(Sheet::Delete) {
                self.sheet = None;
            }
        }
        if self.zeroing.as_ref().is_some_and(|z| !here(&z.stick)) {
            self.zero_failed(&fill(EN.stick_wipe_the_was, &[self.medium.noun()]));
        }
    }

    fn zero_end(&mut self, line: String, ok: bool) {
        self.zeroing = None;
        if self.sheet == Some(Sheet::Zeroing) {
            self.sheet = None;
        }
        self.visit.log.push((line, ok));
        self.dirty = true;
        self.commands.push_back(faraday_shell_api::Command::Draw);
    }
}

/// The sheet: what to delete, what it does, and what it does not reach.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(crate) fn delete_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(d) = app.delete_ask.as_ref() else {
        return;
    };
    let files: &[(String, u64)] = app
        .sticks
        .get(app.visit.stick)
        .map_or(&[], |s| s.files.as_slice());
    let title = fill(EN.stick_wipe_delete_files, &[&d.label.to_string()]);
    let n = d.chosen.len();
    let go = match d.what {
        What::Files => fill(
            EN.stick_wipe_delete_with,
            &[
                &n.to_string(),
                (if n == 1 {
                    EN.stick_wipe_file
                } else {
                    EN.stick_wipe_files
                }),
            ],
        ),
        What::Free => EN.stick_wipe_zero_the_free.to_string(),
        What::Whole => EN.stick_wipe_zero_the.to_string(),
    };
    let hold = app.delete_holds();
    let held_go = fill(
        EN.stick_wipe_hold_delete,
        &[
            &n.to_string(),
            (if n == 1 {
                EN.stick_wipe_file
            } else {
                EN.stick_wipe_files
            }),
        ],
    );
    let go_style = if d.what == What::Files && n == 0 {
        Style::Disabled
    } else {
        Style::Primary
    };
    let amount = |whole: bool| -> String {
        match d.space {
            Some((_, 0)) => EN.stick_wipe_nothing.to_string(),
            Some((free, size)) => {
                let shown = if whole { size } else { free };
                let passes = if whole { u64::from(d.passes) } else { 1 };
                let ms = shown * passes / (TYPICAL_MBPS << 20) * 1000;
                fill(
                    EN.stick_wipe_to_write,
                    &[&size_text(shown).to_string(), &minutes_text(ms).to_string()],
                )
            }
            None => EN.stick_wipe_reading.to_string(),
        }
    };
    let place = if ui.compact {
        (8.0, w - 16.0)
    } else {
        let sw = (w - 48.0).min(640.0);
        ((w - sw) / 2.0, sw)
    };
    let margin = if ui.compact { 8.0 } else { 32.0 };
    let pad = if ui.compact { 16.0 } else { 32.0 };
    scroll_sheet(ui, place, h, margin, pad, &mut |ui, x, y, iw| {
        let mut cy = y;
        // The title.
        if ui.compact {
            cy += crate::compact::sheet_head(ui, x, cy, iw, Icon::Error, ERR, &title);
        } else {
            ui.icon(x, cy, 30.0, Icon::Error, 18.0, ERR);
            let t = ui.fit(20.0, W::S, &title, iw - 42.0);
            ui.text_mid(x + 42.0, cy, 30.0, 20.0, W::S, TEXT, &t);
            cy += 46.0;
        }
        // The danger, in its own band.
        let bh = 40.0;
        ui.fill(x, cy, iw, bh, 8.0, ERR.with_alpha(26));
        ui.stroke(x, cy, iw, bh, 8.0, ERR.with_alpha(110));
        ui.icon(x + 10.0, cy + 10.0, 20.0, Icon::Error, 13.0, ERR);
        let band = ui.fit(13.0, W::S, EN.stick_wipe_danger_zone, iw - 48.0);
        ui.text_mid(x + 38.0, cy, bh, 13.0, W::S, ERR, &band);
        cy += bh + 14.0;
        // What: three choices.
        let choices = [
            (
                What::Files,
                EN.stick_wipe_chosen_files,
                EN.stick_wipe_zeros_over,
            ),
            (What::Free, EN.stick_wipe_the_free, EN.stick_wipe_the_files),
            (
                What::Whole,
                EN.stick_wipe_the_whole,
                EN.stick_wipe_every_file,
            ),
        ];
        for (k, (what, label, line)) in choices.iter().enumerate() {
            let on = d.what == *what;
            let action = Action::DeleteWhat(k as u8);
            ui.fill(
                x,
                cy,
                iw,
                52.0,
                8.0,
                if on { ACCENT.with_alpha(22) } else { BG },
            );
            ui.stroke(x, cy, iw, 52.0, 8.0, if on { ACCENT } else { INNER });
            ui.stroke(
                x + 14.0,
                cy + 17.0,
                18.0,
                18.0,
                9.0,
                if on { ACCENT } else { BORDER },
            );
            if on {
                ui.fill(x + 19.0, cy + 22.0, 8.0, 8.0, 4.0, ACCENT);
            }
            ui.text(x + 44.0, cy + 8.0, 14.0, W::S, TEXT, label);
            let line = ui.fit(12.0, W::R, line, iw - 56.0);
            ui.text(x + 44.0, cy + 29.0, 12.0, W::R, MUTED, &line);
            ui.hit(x, cy, iw, 52.0, action);
            cy += 58.0;
        }
        cy += 6.0;
        match d.what {
            What::Files => {
                if files.is_empty() {
                    let none = fill(EN.stick_wipe_no_files, &[app.medium.noun()]);
                    ui.text(x, cy, 13.0, W::R, DIM, &none);
                    cy += 28.0;
                }
                for (k, (name, size)) in files.iter().enumerate() {
                    let on = d.chosen.contains(name);
                    ui.checkbox(x + 2.0, cy + 10.0, on, true);
                    let sz = size_text(*size);
                    let sw = ui.measure(12.0, W::R, &sz);
                    let n = ui.fit(13.0, W::M, name, iw - 44.0 - sw);
                    ui.text_mid(x + 32.0, cy, 38.0, 13.0, W::M, TEXT, &n);
                    ui.text_right(x + iw, cy, 38.0, 12.0, W::R, MUTED, &sz);
                    ui.hit(x - 4.0, cy, iw + 8.0, 38.0, Action::DeletePick(k));
                    cy += 40.0;
                }
                cy += 8.0;
            }
            What::Free => {
                cy += ui.wrap(x, cy, iw, 13.0, W::S, TEXT, &amount(false)) + 12.0;
            }
            What::Whole => {
                cy += ui.wrap(x, cy, iw, 13.0, W::S, TEXT, &amount(true)) + 10.0;
                let pass_style = |n: u8| {
                    if d.passes == n {
                        Style::Primary
                    } else {
                        Style::Secondary
                    }
                };
                cy += wrap_buttons(
                    ui,
                    x,
                    cy,
                    iw,
                    34.0,
                    &[
                        (
                            EN.stick_wipe_one_pass,
                            pass_style(1),
                            Action::DeletePasses(1),
                        ),
                        (
                            EN.stick_wipe_two_passes_2,
                            pass_style(2),
                            Action::DeletePasses(2),
                        ),
                    ],
                );
                cy += ui.wrap(x, cy, iw, 12.0, W::R, MUTED, EN.stick_wipe_two_passes) + 12.0;
            }
        }
        cy += ui.wrap(x, cy, iw, 13.0, W::S, TEXT, WITHIN) + 8.0;
        cy += ui.wrap(x, cy, iw, 12.0, W::R, MUTED, REACH) + 10.0;
        cy += ui.wrap(x, cy, iw, 12.0, W::R, MUTED, BOUNDARY) + 10.0;
        cy += ui.wrap(x, cy, iw, 12.0, W::R, MUTED, LIMIT) + 10.0;
        cy += ui.wrap(x, cy, iw, 12.0, W::R, MUTED, SECRET) + 12.0;
        // A vault chosen is deleted by a hold (decision 4).
        if hold {
            crate::vault::screen::hold_button_in(
                app,
                ui,
                x,
                cy,
                iw,
                &held_go,
                Action::Vault(crate::vault::VaultAction::HoldWipe),
                ERR,
            );
            cy += 36.0 + 8.0;
            cy += buttons(
                ui,
                x,
                cy,
                iw,
                &[(EN.stick_wipe_cancel, Style::Secondary, Action::Cancel)],
            );
            return cy - y;
        }
        cy += buttons(
            ui,
            x,
            cy,
            iw,
            &[
                (go.as_str(), go_style, Action::DeleteGo),
                (EN.stick_wipe_cancel, Style::Secondary, Action::Cancel),
            ],
        );
        cy - y
    });
}

/// The sheet while zeroing: how far, how long left, and Stop.
pub(crate) fn zeroing_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    let Some(z) = app.zeroing.as_ref() else {
        return;
    };
    let title = if z.whole {
        fill(EN.stick_wipe_zeroing_the, &[&z.label.to_string()])
    } else {
        fill(EN.stick_wipe_zeroing, &[&z.label.to_string()])
    };
    let share = z.done_share();
    let left = z.since.and_then(|(at, from)| {
        let moved = z.next.saturating_sub(from);
        let spent = app.now_ms.saturating_sub(at);
        (moved > 0 && spent > 0).then(|| {
            let per = spent as f64 / f64::from(moved);
            let this = f64::from(z.total.saturating_sub(z.next.saturating_sub(2)));
            let more = f64::from(z.passes.saturating_sub(z.pass)) * f64::from(z.total);
            ((this + more) * per) as u64
        })
    });
    let line = match left {
        Some(ms) => fill(
            EN.stick_wipe_left,
            &[
                &((share * 100.0) as u32).to_string(),
                &minutes_text(ms).to_string(),
            ],
        ),
        None => format!("{}%", (share * 100.0) as u32),
    };
    let pass = (z.passes > 1).then(|| {
        fill(
            EN.stick_wipe_pass_of,
            &[&z.pass.to_string(), &z.passes.to_string()],
        )
    });
    let keep = fill(EN.stick_wipe_keep_the, &[app.medium.noun()]);
    let (stop_label, stop_style) = if z.stop {
        (EN.stick_wipe_stopping, Style::Disabled)
    } else {
        (EN.stick_wipe_stop, Style::Secondary)
    };
    let place = if ui.compact {
        (8.0, w - 16.0)
    } else {
        let sw = (w - 48.0).min(520.0);
        ((w - sw) / 2.0, sw)
    };
    let pad = if ui.compact { 16.0 } else { 32.0 };
    scroll_sheet(ui, place, h, 8.0, pad, &mut |ui, x, y, iw| {
        let mut cy = y;
        let t = ui.fit(18.0, W::S, &title, iw);
        ui.text_mid(x, cy, 30.0, 18.0, W::S, TEXT, &t);
        cy += 44.0;
        ui.fill(x, cy, iw, 10.0, 5.0, INNER);
        ui.fill(x, cy, (iw * share).max(10.0), 10.0, 5.0, ACCENT);
        cy += 24.0;
        ui.text(x, cy, 14.0, W::S, TEXT, &line);
        cy += 26.0;
        if let Some(p) = &pass {
            ui.text(x, cy, 13.0, W::R, MUTED, p);
            cy += 24.0;
        }
        cy += ui.wrap(x, cy, iw, 13.0, W::R, MUTED, &keep) + 14.0;
        cy += buttons(ui, x, cy, iw, &[(stop_label, stop_style, Action::ZeroStop)]);
        cy - y
    });
}

impl Faraday {
    /// Handles the Delete files and zero-a-stick actions (`Delete*`, `ZeroStop`).
    pub(crate) fn wipe_act(&mut self, action: Action) {
        match action {
            Action::DeleteFiles => self.delete_open(),
            Action::DeleteWhat(k) => self.delete_what(k),
            Action::DeletePick(k) => self.delete_pick(k),
            Action::DeletePasses(n) => self.delete_passes(n),
            // A vault chosen is deleted by the hold alone (decision 4).
            Action::DeleteGo if !self.delete_holds() => self.delete_go(),
            Action::DeleteGo => {}
            Action::ZeroStop => self.zero_stop(),
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}
