//! The stick pills across the top of the desktop app and the website
//! demo: the boot stick and Stick 1, 2, 3 and on, each a small pill with
//! its state, in or out. A press on a pill asks the shell to insert the
//! stick or pull it; the shell's sticks are folders, one for each pill,
//! so a stick keeps what was written to it.
//!
//! The pills are the boot stick and Stick 1 when nothing asks for more.
//! While a backup flow is on screen there is a pill for each stick it
//! needs, and a stick that is in keeps its pill.

use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Ui, W};
use crate::{Action, Faraday};

/// The boot stick's folder.
pub const BOOT_FOLDER: &str = "BOOTSTICK";

/// The folder of backup Stick `n` (1 and on).
pub fn backup_folder(n: usize) -> String {
    format!("STICK-{n}")
}

/// The number a backup stick's folder is for, when it is one.
fn folder_number(folder: &str) -> Option<usize> {
    folder
        .strip_prefix("STICK-")?
        .parse()
        .ok()
        .filter(|&n| n >= 1)
}

/// The sticks a backup flow waits for, for the pills and for a demo that
/// inserts them itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SticksWanted {
    /// How many sticks the flow uses, the boot stick not counted.
    pub count: usize,
    /// The stick it asks for now, counted from 1 as the pills are: it
    /// asks for one at a time. None while it asks for none (making the
    /// keys, locking).
    pub now: Option<usize>,
}

/// One stick's pill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pill {
    /// 0 the boot stick, `n` Stick `n`.
    pub n: usize,
    /// What it is called: Boot stick, Stick 1.
    pub label: String,
    /// The stick's folder in the shell.
    pub folder: String,
    /// The stick is in.
    pub inserted: bool,
    /// The flow asks for this stick now.
    pub asked: bool,
}

/// A pill's height, and the gap between pills and between rows, in
/// design units.
const PILL_H: f32 = 20.0;
const PILL_GAP: f32 = 6.0;
/// The room round a row of pills, above and below.
const STRIP_PAD: f32 = 2.0;
/// A pill's room left and right of its text, and the colour dot's size
/// and the room after it.
const PILL_PAD: f32 = 10.0;
const DOT_R: f32 = 3.0;
const DOT_ROOM: f32 = 8.0;
const PILL_TEXT: f32 = 11.0;
/// The room kept clear at each side of a row drawn across the window:
/// the page's own buttons sit at the right.
const SIDE_ROOM: f32 = 70.0;

impl Faraday {
    /// The sticks the backup flow on screen waits for: how many, and the
    /// one it asks for now. None with no flow on screen.
    pub fn sticks_wanted(&self) -> Option<SticksWanted> {
        use crate::backup_flow::Step;
        let f = self.flow()?;
        if f.step == Step::Done || f.slots.is_empty() {
            return None;
        }
        let now = match f.step {
            Step::Sticks => f.slots.iter().position(|s| s.bound.is_none()),
            Step::Write => f.slots.iter().position(|s| s.written.is_none()),
            _ => None,
        };
        Some(SticksWanted {
            count: f.slots.len(),
            now: now.map(|k| k + 1),
        })
    }

    /// The pills across the top, the boot stick first. None where the
    /// shell has no pills.
    pub fn stick_pills(&self) -> Vec<Pill> {
        if !self.shell_sticks {
            return Vec::new();
        }
        let wanted = self.sticks_wanted();
        let held = self
            .sticks
            .iter()
            .filter_map(|s| folder_number(&s.label))
            .max()
            .unwrap_or(0);
        let count = wanted.map_or(0, |w| w.count).max(held).max(1);
        let asked = wanted.and_then(|w| w.now);
        let is_in = |folder: &str| self.sticks.iter().any(|s| s.label == folder);
        let mut pills = vec![Pill {
            n: 0,
            label: EN.stick_pill_boot.to_string(),
            folder: BOOT_FOLDER.to_string(),
            // The core relabels the boot stick, so it is found by what it is.
            inserted: self.sticks.iter().any(|s| s.boot),
            asked: false,
        }];
        for n in 1..=count {
            let folder = backup_folder(n);
            pills.push(Pill {
                n,
                label: fill(EN.stick_pill_n, &[&n.to_string()]),
                inserted: is_in(&folder),
                folder,
                asked: asked == Some(n),
            });
        }
        pills
    }

    /// The sticks the person pressed since the last call, as folder
    /// names: the shell inserts each one that is out and pulls each one
    /// that is in.
    pub fn take_stick_toggles(&mut self) -> Vec<String> {
        std::mem::take(&mut self.stick_toggles)
    }

    /// A press on pill `n`.
    pub(crate) fn stick_pill_pressed(&mut self, n: u8) {
        let n = usize::from(n);
        let folder = if n == 0 {
            BOOT_FOLDER.to_string()
        } else {
            backup_folder(n)
        };
        self.stick_toggles.push(folder);
    }
}

/// How a row of pills is set.
struct Row {
    /// The pills in the row, by index, and the row's width.
    items: Vec<usize>,
    width: f32,
}

/// The pills' texts and widths in the longest form that fits in `avail`
/// on one row; failing that, the short form, wrapped into rows.
struct Layout {
    texts: Vec<String>,
    widths: Vec<f32>,
    rows: Vec<Row>,
}

fn layout(pills: &[Pill], ui: &Ui, avail: f32) -> Layout {
    let build = |long: bool| -> (Vec<String>, Vec<f32>) {
        let texts: Vec<String> = pills
            .iter()
            .map(|p| {
                if long {
                    let state = if p.inserted {
                        EN.stick_pill_in
                    } else {
                        EN.stick_pill_out
                    };
                    fill(EN.stick_pill_state, &[&p.label, state])
                } else {
                    p.label.clone()
                }
            })
            .collect();
        let widths = texts
            .iter()
            .map(|t| 2.0 * PILL_PAD + DOT_ROOM + ui.measure(PILL_TEXT, W::S, t))
            .collect();
        (texts, widths)
    };
    let total = |w: &[f32]| w.iter().sum::<f32>() + PILL_GAP * (w.len().saturating_sub(1)) as f32;
    let (mut texts, mut widths) = build(true);
    if total(&widths) > avail {
        (texts, widths) = build(false);
    }
    let mut rows: Vec<Row> = Vec::new();
    for (k, w) in widths.iter().enumerate() {
        match rows.last_mut() {
            Some(r) if r.width + PILL_GAP + w <= avail => {
                r.width += PILL_GAP + w;
                r.items.push(k);
            }
            _ => rows.push(Row {
                items: vec![k],
                width: *w,
            }),
        }
    }
    Layout {
        texts,
        widths,
        rows,
    }
}

/// How tall the strip of pills across a window `window_w` wide is, in
/// design units; 0 where there are none.
pub(crate) fn strip_h(app: &Faraday, ui: &Ui, window_w: f32) -> f32 {
    let pills = app.stick_pills();
    if pills.is_empty() {
        return 0.0;
    }
    let l = layout(&pills, ui, window_w - 2.0 * SIDE_ROOM);
    let rows = l.rows.len() as f32;
    2.0 * STRIP_PAD + rows * PILL_H + (rows - 1.0) * PILL_GAP
}

/// Draws the pills across the top of the window, over the page and any
/// sheet, and offers each as a press.
pub(crate) fn draw(app: &Faraday, ui: &mut Ui, window_w: f32) {
    let pills = app.stick_pills();
    if pills.is_empty() {
        return;
    }
    let l = layout(&pills, ui, window_w - 2.0 * SIDE_ROOM);
    let light = ui.theme.is_light();
    let was = (ui.ox, ui.oy, ui.in_sheet);
    ui.ox = 0.0;
    ui.oy = 0.0;
    ui.in_sheet = false;
    for (r, row) in l.rows.iter().enumerate() {
        let y = STRIP_PAD + r as f32 * (PILL_H + PILL_GAP);
        let mut x = (window_w - row.width) / 2.0;
        for &k in &row.items {
            let (p, w) = (&pills[k], l.widths[k]);
            let tone = stick_tone(p.n, light);
            let action = Action::StickPill(p.n as u8);
            let pressed = ui.is_pressed(action);
            let (fg, edge, body) = if p.inserted {
                (
                    tone,
                    tone.with_alpha(170),
                    tone.with_alpha(if pressed { 70 } else { 34 }),
                )
            } else if p.asked {
                (tone, tone, tone.with_alpha(if pressed { 70 } else { 18 }))
            } else {
                (DIM, BORDER, if pressed { INNER } else { BG })
            };
            ui.fill(x, y, w, PILL_H, PILL_H / 2.0, body);
            ui.stroke(x, y, w, PILL_H, PILL_H / 2.0, edge);
            ui.dot(x + PILL_PAD + DOT_R, y + PILL_H / 2.0, DOT_R, tone);
            ui.text_mid(
                x + PILL_PAD + DOT_ROOM,
                y,
                PILL_H,
                PILL_TEXT,
                W::S,
                fg,
                &l.texts[k],
            );
            ui.hit(x, y, w, PILL_H, action);
            x += w + PILL_GAP;
        }
    }
    (ui.ox, ui.oy, ui.in_sheet) = was;
}
