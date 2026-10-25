//! The stick pills: the boot stick, which is Stick 1, then Stick 2, 3
//! and on, each a small pill with its state, in or out.
//!
//! On Faraday Desktop and the website demo they stand in the top bar,
//! left of its other actions (`docs/LAYOUT.md` §3): as many as fit in the
//! room the bar leaves beside the page's title, the boot stick's always,
//! the rest folded into one pill, "Sticks · N", that opens the Sticks
//! menu listing every stick (Material's overflow). A press on a pill, or
//! on a stick in the menu, asks the shell to insert the stick or pull it;
//! the shell's sticks are folders, one for each pill, so a stick keeps
//! what was written to it.
//!
//! The pills are the boot stick and Stick 2 when nothing asks for more.
//! While a backup flow is on screen there is a pill for each stick it
//! needs, and a stick that is in keeps its pill.
//!
//! A backup's sticks are numbered from 2 on every shell: the boot stick
//! is Stick 1, still called Boot stick (Card 1 and Boot card on the Pi).
//! On a device (the PC stick, the Pi) the pills are the boot stick's and
//! the backup flow's, in a strip across the top while a flow on screen
//! uses sticks: the boot pill,
//! then a pill per stick in the flow's order, lit only while the stick in reads as that one, and
//! no press, since a real stick is put in by hand (`docs/BACKUP-FLOW.md`
//! §16).

use crate::backup_flow::{Purpose, Step};
use crate::drawing::View;
use crate::medium::Medium;
use crate::strings::{EN, fill};
use crate::ui::pal::*;
use crate::ui::{Ui, W};
use crate::{Action, Faraday, StickInfo};
use faraday_draw::tokens::{CAPTION, GAP, GAP_SMALL, GRID, MODE_PILL_HEIGHT, MODE_PILL_PAD};

/// The boot stick's folder.
pub const BOOT_FOLDER: &str = "BOOTSTICK";

/// The number of the first backup stick: the boot stick is Stick 1.
pub const FIRST_BACKUP: usize = 2;

/// The folder of backup Stick `n` (2 and on: Stick 1 is the boot stick,
/// whose folder is [`BOOT_FOLDER`]).
pub fn backup_folder(n: usize) -> String {
    format!("STICK-{n}")
}

/// The number a backup stick's folder is for, when it is one.
fn folder_number(folder: &str) -> Option<usize> {
    folder
        .strip_prefix("STICK-")?
        .parse()
        .ok()
        .filter(|&n| n >= FIRST_BACKUP)
}

/// The sticks a backup flow waits for, for the pills and for a demo that
/// inserts them itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SticksWanted {
    /// How many sticks the flow uses, the boot stick not counted.
    pub count: usize,
    /// The stick it asks for now, by its pill's number (the first
    /// backup stick is 2, the boot stick being 1): it asks for one at a
    /// time. None while it asks for none (making the
    /// keys, locking).
    pub now: Option<usize>,
}

/// One stick's pill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pill {
    /// 1 the boot stick, `n` Stick `n` from 2.
    pub n: usize,
    /// What it is called: Boot stick, Stick 2.
    pub label: String,
    /// The stick's folder in the shell.
    pub folder: String,
    /// The stick is in.
    pub inserted: bool,
    /// The flow asks for this stick now.
    pub asked: bool,
}

/// A pill's measures, in design units: its height, its text, its room
/// left and right of the text, the colour dot's radius and the room it
/// takes before the text, the gap between pills and between rows, the
/// room round a row above and below, and the room kept clear at each
/// side of a row drawn across the window.
struct Size {
    h: f32,
    text: f32,
    pad: f32,
    dot_r: f32,
    dot_room: f32,
    gap: f32,
    strip_pad: f32,
    side: f32,
}

/// On a wide display: the page's own buttons sit at the right, under the
/// strip, so the row keeps clear of them.
const WIDE: Size = Size {
    h: 28.0,
    text: 13.0,
    pad: 14.0,
    dot_r: 4.0,
    dot_room: 12.0,
    gap: 8.0,
    strip_pad: 4.0,
    side: 70.0,
};

/// On a small panel: the strip is a band of its own above the page.
const SMALL: Size = Size {
    h: 24.0,
    text: 12.0,
    pad: 9.0,
    dot_r: 4.0,
    dot_room: 10.0,
    gap: 6.0,
    strip_pad: 4.0,
    side: 8.0,
};

fn size(app: &View) -> &'static Size {
    if app.is_compact() { &SMALL } else { &WIDE }
}

impl Faraday {
    /// The sticks the backup flow on screen waits for: how many, and the
    /// one it asks for now. None with no flow on screen.
    pub fn sticks_wanted(&self) -> Option<SticksWanted> {
        use crate::backup_flow::Step;
        let f = self.flow()?;
        if f.step == Step::Done || f.slots.is_empty() {
            return None;
        }
        Some(SticksWanted {
            count: f.own_sticks(),
            now: self.flow_wanted_slot().map(|k| f.stick_number(k)),
        })
    }

    /// The pills across the top, the boot stick first. None where the
    /// shell has no pills.
    pub fn stick_pills(&self) -> Vec<Pill> {
        if !self.shell_sticks {
            return self.device_pills();
        }
        let wanted = self.sticks_wanted();
        let held = self
            .stick
            .iter()
            .filter_map(|s| folder_number(&s.label))
            .max()
            .unwrap_or(0);
        let last = wanted
            .map_or(0, |w| w.count + FIRST_BACKUP - 1)
            .max(held)
            .max(FIRST_BACKUP);
        let asked = wanted.and_then(|w| w.now);
        let is_in = |folder: &str| self.stick.iter().any(|s| s.label == folder);
        let mut boot = self.boot_pill();
        boot.asked = asked == Some(1);
        let mut pills = vec![boot];
        for n in FIRST_BACKUP..=last {
            let folder = backup_folder(n);
            pills.push(Pill {
                n,
                label: self.pill_name(n),
                inserted: is_in(&folder),
                folder,
                asked: asked == Some(n),
            });
        }
        pills
    }

    /// The boot stick's pill, Stick 1.
    fn boot_pill(&self) -> Pill {
        Pill {
            n: 1,
            label: match self.medium {
                Medium::Stick => EN.stick_pill_boot,
                Medium::SdCard => EN.backup_flow_pill_boot_card,
            }
            .to_string(),
            folder: BOOT_FOLDER.to_string(),
            // The core relabels the boot stick, so it is found by what it is.
            inserted: self.stick.iter().any(|s| s.boot),
            asked: false,
        }
    }

    /// A device's pills, while the backup flow on screen uses sticks: the
    /// boot stick's, then one per stick of the flow, lit while the stick
    /// in reads as that one.
    fn device_pills(&self) -> Vec<Pill> {
        if !self.flow_screen() {
            return Vec::new();
        }
        let Some(wanted) = self.sticks_wanted() else {
            return Vec::new();
        };
        let Some(f) = self.flow() else {
            return Vec::new();
        };
        // The numbers of the sticks in that read as one of the flow's.
        let lit: Vec<usize> = self
            .stick
            .iter()
            .filter_map(|st| self.flow_stick_slot(st))
            .map(|k| f.stick_number(k))
            .collect();
        let mut boot = self.boot_pill();
        boot.folder = String::new();
        boot.asked = wanted.now == Some(1);
        let mut pills = vec![boot];
        pills.extend((0..wanted.count).map(|j| {
            let n = j + FIRST_BACKUP;
            Pill {
                n,
                label: self.pill_name(n),
                folder: String::new(),
                inserted: lit.contains(&n),
                asked: wanted.now == Some(n),
            }
        }));
        pills
    }

    /// The word a pill uses for its medium: Stick, Card.
    pub(crate) fn pill_word(&self) -> &'static str {
        match self.medium {
            Medium::Stick => EN.backup_flow_pill_stick,
            Medium::SdCard => EN.backup_flow_pill_card,
        }
    }

    /// Pill `n`: Stick 2, Card 2.
    fn pill_name(&self, n: usize) -> String {
        fill(EN.backup_flow_pill, &[self.pill_word(), &n.to_string()])
    }

    /// The slot of the flow on screen that the stick `st` reads as, when
    /// it is recognised as one (§18): the stick being read, once the flow
    /// has taken it for its slot or checked it; else the one slot whose
    /// record it matches, by label, serial, listing and the vault files'
    /// SHA-256 (§4.2, §4.6). A stick that matches no slot, or several
    /// alike, is none.
    pub(crate) fn flow_stick_slot(&self, st: &StickInfo) -> Option<usize> {
        use crate::backup_flow::Problem;
        let f = self.flow()?;
        let reading = f.live.reading.as_ref().filter(|r| r.id == st.id);
        if let Some(r) = reading {
            let settled = r.pending.is_empty() && !r.space_pending;
            match &r.problem {
                None if settled && (r.bound || r.purpose == Purpose::Check) => {
                    return Some(r.slot);
                }
                // Still read, or asked which vault or which place.
                None => return None,
                Some(Problem::BoundTo(t)) => {
                    return f.slots.iter().position(|s| s.target == *t);
                }
                Some(Problem::NotRead(_) | Problem::HasFirst | Problem::HasFilesOf(_)) => {}
                // Unusable: not one of the flow's sticks as far as is known.
                Some(_) => return None,
            }
        }
        self.flow_record_slot(st)
    }

    /// The one slot of the flow on screen whose record the stick `st`
    /// matches, by label, serial, listing and, once read, the vault files'
    /// SHA-256; none where it matches none or several alike.
    fn flow_record_slot(&self, st: &StickInfo) -> Option<usize> {
        let f = self.flow()?;
        let reading = f.live.reading.as_ref().filter(|r| r.id == st.id);
        let read: Option<Vec<(&str, [u8; 32])>> =
            reading.filter(|r| r.pending.is_empty()).map(|r| {
                r.files
                    .iter()
                    .map(|(v, _)| (v.name.as_str(), v.sha))
                    .collect()
            });
        let mut listing = st.files.clone();
        listing.sort();
        let matches = |k: usize| -> bool {
            let s = &f.slots[k];
            if let Some(w) = &s.written {
                return w.label == st.label
                    && w.serial == st.serial
                    && w.files.iter().all(|(n, sha)| {
                        st.files.iter().any(|(x, _)| x == n)
                            && read.as_ref().is_none_or(|rd| {
                                !n.to_ascii_lowercase().ends_with(".ofv")
                                    || rd.iter().any(|(x, h)| x == n && h == sha)
                            })
                    });
            }
            let Some(b) = &s.bound else {
                return false;
            };
            let mut was = b.listing.clone();
            was.sort();
            b.label == st.label
                && b.serial == st.serial
                && was == listing
                && read.as_ref().is_none_or(|rd| {
                    b.vaults
                        .iter()
                        .all(|v| rd.iter().any(|(x, h)| *x == v.name && *h == v.sha))
                })
        };
        let mut found = (0..f.slots.len()).filter(|&k| matches(k));
        let k = found.next()?;
        found.next().is_none().then_some(k)
    }

    /// On a device, the name of slot `k`'s stick in the flow on screen,
    /// which every line naming that stick leads with (§18): "Stick 3", or
    /// "Boot stick" where the boot stick holds it. None on Desktop and the
    /// demo, whose sticks are named by their folders.
    pub(crate) fn flow_slot_name(&self, k: usize) -> Option<String> {
        if self.shell_sticks {
            return None;
        }
        let n = self.flow()?.stick_number(k);
        Some(if n == 1 {
            self.boot_pill().label
        } else {
            self.pill_name(n)
        })
    }

    /// Slot `k`'s number written out with the pill's word, for the lines
    /// that say it in a sentence: "Stick", "3".
    fn flow_slot_word_n(&self, k: usize) -> Option<(&'static str, String)> {
        let n = self.flow()?.stick_number(k);
        Some((self.pill_word(), n.to_string()))
    }

    /// On a device, the row naming slot `k` of the flow on screen: "Stick
    /// 2 · Vault 1 (Place 1)".
    pub(crate) fn flow_slot_row(&self, k: usize) -> Option<String> {
        let name = self.flow_slot_name(k)?;
        let f = self.flow()?;
        let t = f.slots.get(k)?.target;
        Some(fill(
            EN.backup_flow_slot_row,
            &[&name, &f.target_what(&t, self.medium.noun())],
        ))
    }

    /// The Insert line for slot `k`: on a device "Insert Stick 3: Vault 2
    /// (Place 2)", else "Insert the stick for Vault 2 (Place 2)".
    pub(crate) fn flow_insert_line(&self, k: usize) -> String {
        let m = self.medium.noun();
        let Some(f) = self.flow() else {
            return String::new();
        };
        let Some(t) = f.slots.get(k).map(|s| s.target) else {
            return String::new();
        };
        match self.flow_slot_name(k) {
            Some(name) => fill(EN.backup_flow_insert_n, &[&name, &f.target_what(&t, m)]),
            None => fill(EN.backup_flow_insert, &[&f.target_name(&t, m)]),
        }
    }

    /// Above the hold on a device, for a stick not yet settled that reads
    /// as another slot's stick (a blank stick read alike, §18): "This reads
    /// as Stick 2".
    pub(crate) fn flow_stick_reads_as(&self) -> Option<String> {
        if self.shell_sticks {
            return None;
        }
        let f = self.flow()?;
        let r = f.live.reading.as_ref()?;
        if r.problem.is_some() || r.bound || r.purpose != Purpose::Bind {
            return None;
        }
        if !r.pending.is_empty() || r.space_pending {
            return None;
        }
        let st = self.stick.iter().find(|s| s.id == r.id)?;
        let m = self.flow_record_slot(st).filter(|&m| m != r.slot)?;
        let (word, n) = self.flow_slot_word_n(m)?;
        Some(fill(EN.backup_flow_reads_as, &[word, &n]))
    }

    /// The line under a refused stick's reason on a device (§18): the
    /// stick it reads as, "This is Stick 2", or "Not one of this
    /// backup's sticks". None where it is the stick wanted, or where the
    /// refusal is not about which stick it is.
    pub(crate) fn flow_stick_from_elsewhere(&self) -> Option<String> {
        use crate::backup_flow::Problem;
        if self.shell_sticks {
            return None;
        }
        let f = self.flow()?;
        let r = f.live.reading.as_ref()?;
        if !matches!(
            r.problem,
            Some(
                Problem::BoundTo(_)
                    | Problem::NotRead(_)
                    | Problem::HasFirst
                    | Problem::HasFilesOf(_)
            )
        ) {
            return None;
        }
        let st = self.stick.iter().find(|s| s.id == r.id)?;
        match self.flow_stick_slot(st) {
            Some(m) if m == r.slot => None,
            Some(m) => {
                let (word, n) = self.flow_slot_word_n(m)?;
                Some(fill(EN.backup_flow_this_is_n, &[word, &n]))
            }
            None => Some(fill(EN.backup_flow_not_ours, &[self.medium.nouns()])),
        }
    }

    /// The line under slot `k`'s row on a device while the stick just
    /// taken for it is in (§18): "Mark it as Stick 2".
    pub(crate) fn flow_mark_line(&self, k: usize) -> Option<String> {
        if self.shell_sticks {
            return None;
        }
        let f = self.flow()?;
        if !matches!(f.step, Step::Sticks | Step::Write) {
            return None;
        }
        let r = f.live.reading.as_ref()?;
        // The boot stick is Stick 1 already: never a mark line.
        if !r.first || r.boot || r.slot != k || f.boot_holds(k) {
            return None;
        }
        if f.slots.get(k).is_none_or(|s| s.bound.is_none()) {
            return None;
        }
        let (word, n) = self.flow_slot_word_n(k)?;
        Some(fill(EN.backup_flow_mark_as, &[word, &n]))
    }

    /// The sticks the person pressed since the last call, as folder
    /// names: the shell inserts each one that is out and pulls each one
    /// that is in.
    pub fn take_stick_toggles(&mut self) -> Vec<String> {
        std::mem::take(&mut self.stick.toggles)
    }

    /// A press on pill `n`; none on a device, whose sticks are put in by
    /// hand.
    pub(crate) fn stick_pill_pressed(&mut self, n: u8) {
        if !self.shell_sticks {
            return;
        }
        let n = usize::from(n);
        let folder = if n <= 1 {
            BOOT_FOLDER.to_string()
        } else {
            backup_folder(n)
        };
        self.stick.toggles.push(folder);
    }
}

/// How a row of pills is set.
struct Row {
    /// The pills in the row, by index, and the row's width.
    items: Vec<usize>,
    width: f32,
}

/// The pills' texts and widths in the longest form that fits in `avail`
/// on one row: with its state ("Stick 2 · in"), short ("Stick 2"), the
/// number alone ("1"); failing all three, the short form, wrapped into
/// rows.
struct Layout {
    texts: Vec<String>,
    widths: Vec<f32>,
    rows: Vec<Row>,
}

/// How a pill's text is put.
#[derive(Clone, Copy, PartialEq)]
enum Form {
    Long,
    Short,
    Number,
}

fn layout(pills: &[Pill], ui: &Ui, z: &Size, avail: f32) -> Layout {
    let build = |form: Form| -> (Vec<String>, Vec<f32>) {
        let texts: Vec<String> = pills
            .iter()
            .map(|p| match form {
                Form::Long => {
                    let state = if p.inserted {
                        EN.stick_pill_inserted
                    } else {
                        EN.stick_pill_out
                    };
                    fill(EN.stick_pill_state, &[&p.label, state])
                }
                Form::Number => p.n.to_string(),
                Form::Short => p.label.clone(),
            })
            .collect();
        let widths = texts
            .iter()
            .map(|t| 2.0 * z.pad + z.dot_room + ui.measure(z.text, W::S, t))
            .collect();
        (texts, widths)
    };
    let total = |w: &[f32]| w.iter().sum::<f32>() + z.gap * (w.len().saturating_sub(1)) as f32;
    let (mut texts, mut widths) = build(Form::Long);
    for form in [Form::Short, Form::Number] {
        if total(&widths) <= avail {
            break;
        }
        (texts, widths) = build(form);
    }
    if total(&widths) > avail {
        (texts, widths) = build(Form::Short);
    }
    let mut rows: Vec<Row> = Vec::new();
    for (k, w) in widths.iter().enumerate() {
        match rows.last_mut() {
            Some(r) if r.width + z.gap + w <= avail => {
                r.width += z.gap + w;
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
/// design units: a device's, and Faraday Desktop's on the Pi panel's
/// size; 0 where there are none, or where they stand in the top bar.
pub(crate) fn strip_h(app: &View, ui: &Ui, window_w: f32) -> f32 {
    if in_bar(app) {
        return 0.0;
    }
    let pills = app.stick_pills();
    if pills.is_empty() {
        return 0.0;
    }
    let z = size(app);
    let l = layout(&pills, ui, z, window_w - 2.0 * z.side);
    let rows = l.rows.len() as f32;
    2.0 * z.strip_pad + rows * z.h + (rows - 1.0) * z.gap
}

/// The room a wide page leaves above its first line, which the strip may
/// lie over, in design units.
const PAGE_TOP_ROOM: f32 = 24.0;

/// How far a wide page moves down under a strip `strip` tall: by what
/// the strip takes beyond the room the page leaves at its top.
pub(crate) fn page_drop(strip: f32) -> f32 {
    (strip - PAGE_TOP_ROOM).max(0.0)
}

/// A pill's colours: its text, its edge and its body, and the tone of its
/// dot. Lit in its own tone while the stick is in, edged in it while the
/// flow asks for it, else quiet.
fn paints(p: &Pill, pressed: bool, light: bool) -> (Paint, Paint, Paint, Paint) {
    // The tones are counted from the boot stick's, the first.
    let tone = Paint::from(stick_tone(p.n.saturating_sub(1), light));
    let (fg, edge, body) = if p.inserted {
        (
            tone,
            tone.with_alpha(170),
            tone.with_alpha(if pressed { 70 } else { 34 }),
        )
    } else if p.asked {
        (tone, tone, tone.with_alpha(if pressed { 70 } else { 18 }))
    } else {
        quiet(pressed)
    };
    (fg, edge, body, tone)
}

/// A quiet pill's text, edge and body.
fn quiet(pressed: bool) -> (Paint, Paint, Paint) {
    (
        Role::OnSurfaceVariant.paint(),
        Role::OutlineVariant.paint(),
        if pressed {
            Role::SurfaceContainerHigh.paint()
        } else {
            Role::SurfaceContainerHighest.paint()
        },
    )
}

/// Draws the strip of pills across the top of the window, over the page
/// and any sheet: a device's, and Faraday Desktop's on the Pi panel's
/// size, where its 44-unit bar has no room for them; none where they
/// stand in the top bar ([`bar`]). Each takes a press where the shell's
/// sticks are its own; a device's are put in by hand.
pub(crate) fn draw(app: &View, ui: &mut Ui, window_w: f32) {
    if in_bar(app) {
        return;
    }
    let pills = app.stick_pills();
    if pills.is_empty() {
        return;
    }
    let z = size(app);
    let l = layout(&pills, ui, z, window_w - 2.0 * z.side);
    let light = ui.theme.is_light();
    let presses = app.shell_sticks;
    let was = (ui.ox, ui.oy, ui.in_sheet);
    ui.ox = 0.0;
    ui.oy = 0.0;
    ui.in_sheet = false;
    for (r, row) in l.rows.iter().enumerate() {
        let y = z.strip_pad + r as f32 * (z.h + z.gap);
        let mut x = (window_w - row.width) / 2.0;
        for &k in &row.items {
            let (p, w) = (&pills[k], l.widths[k]);
            let action = Action::StickPill(p.n as u8);
            let (fg, edge, body, tone) = paints(p, presses && ui.is_pressed(action), light);
            ui.fill(x, y, w, z.h, z.h / 2.0, body);
            ui.stroke(x, y, w, z.h, z.h / 2.0, edge);
            ui.dot(x + z.pad + z.dot_r, y + z.h / 2.0, z.dot_r, tone);
            ui.text_mid(
                x + z.pad + z.dot_room,
                y,
                z.h,
                z.text,
                W::S,
                fg,
                &l.texts[k],
            );
            if presses {
                ui.hit(x, y, w, z.h, action);
            }
            x += w + z.gap;
        }
    }
    (ui.ox, ui.oy, ui.in_sheet) = was;
}

// ---- Faraday Desktop's top bar (`docs/LAYOUT.md` §3) ----

/// Whether the pills stand in the top bar: Faraday Desktop's and the
/// demo's, on every window but the Pi panel's size, whose 44-unit bar
/// holds the way back, Load samples, the mode and the ? already (there
/// they keep their strip). A lookup.
fn in_bar(app: &View) -> bool {
    app.shell_sticks && app.nav() != crate::nav::Nav::Menu
}

/// One pill in the top bar: a stick's, by its index among the pills, or
/// the Sticks pill (`None`) that the rest fold into; its text and width;
/// whether a press opens the Sticks menu rather than inserting or
/// pulling the stick.
#[derive(Clone)]
struct BarItem {
    pill: Option<usize>,
    text: String,
    w: f32,
    menu: bool,
}

/// The pills the top bar holds, in order from the left, and their width
/// with the gaps between them.
struct BarFit {
    pills: Vec<Pill>,
    items: Vec<BarItem>,
    width: f32,
}

/// A pill's text with its state: "Stick 2 · in".
fn with_state(p: &Pill) -> String {
    let state = if p.inserted {
        EN.stick_pill_inserted
    } else {
        EN.stick_pill_out
    };
    fill(EN.stick_pill_state, &[&p.label, state])
}

/// A bar pill's width for `text`, with the room of a stick's dot when
/// `dot`: the mode pill's measures.
fn bar_pill_w(ui: &Ui, text: &str, dot: bool) -> f32 {
    let dot = if dot { GRID + GAP_SMALL } else { 0.0 };
    2.0 * MODE_PILL_PAD + dot + ui.measure(CAPTION, W::S, text)
}

/// Costs a text measure per pill. The pills in the top bar, in `room`:
/// every stick with its state where all fit; else the boot stick and as
/// many after it as fit beside the Sticks pill, which holds the rest;
/// else, narrowest, the boot stick's pill alone, by its name, opening the
/// Sticks menu. The boot stick's pill always shows, and the bar never
/// takes a second row.
/// None where they keep their strip ([`draw`]): a device, the Pi panel's size.
fn bar_fit(app: &View, ui: &Ui, room: f32) -> Option<BarFit> {
    if !in_bar(app) {
        return None;
    }
    let pills = app.stick_pills();
    if pills.is_empty() {
        return None;
    }
    let item = |k: usize, text: String| BarItem {
        pill: Some(k),
        w: bar_pill_w(ui, &text, true),
        text,
        menu: false,
    };
    let more = |n: usize| {
        let text = fill(EN.stick_pill_more, &[&n.to_string()]);
        BarItem {
            pill: None,
            w: bar_pill_w(ui, &text, false),
            text,
            menu: true,
        }
    };
    let total = |v: &[BarItem]| -> f32 {
        v.iter().map(|i| i.w).sum::<f32>() + GAP * v.len().saturating_sub(1) as f32
    };
    let long: Vec<BarItem> = pills
        .iter()
        .enumerate()
        .map(|(k, p)| item(k, with_state(p)))
        .collect();
    let n = long.len();
    let items = if n == 1 || total(&long) <= room {
        long
    } else {
        // The boot stick, then each after it while it and the Sticks
        // pill for the rest still fit.
        let mut kept = vec![long[0].clone()];
        for it in &long[1..n - 1] {
            let mut next = kept.clone();
            next.push(it.clone());
            next.push(more(n - next.len() + 1));
            if total(&next) > room {
                break;
            }
            kept.push(it.clone());
        }
        let rest = n - kept.len();
        kept.push(more(rest));
        if total(&kept) <= room {
            kept
        } else {
            // Narrowest: the boot stick's pill alone, by its name, which
            // opens the menu of every stick.
            let mut boot = item(0, pills[0].label.clone());
            boot.menu = true;
            vec![boot]
        }
    };
    let width = total(&items);
    Some(BarFit {
        pills,
        items,
        width,
    })
}

/// The room the top bar's stick pills take at its right, with the gap
/// after them, in `room`; 0 with none. Costs a text measure per pill.
pub(crate) fn bar_width(app: &View, ui: &Ui, room: f32) -> f32 {
    bar_fit(app, ui, room).map_or(0.0, |f| f.width + GAP)
}

/// Costs a text measure per pill. Draws the top bar's stick pills
/// ending at `right`, their top at `y`, in `room` ([`bar_fit`]), and
/// notes where, so they are drawn again over a sheet. Returns where the
/// next action ends: their left edge a gap to the left, or `right` with
/// none.
pub(crate) fn bar(app: &View, ui: &mut Ui, right: f32, y: f32, room: f32) -> f32 {
    let Some(fit) = bar_fit(app, ui, room) else {
        return right;
    };
    app.stick.note_bar(Some((right + ui.ox, y + ui.oy, room)));
    bar_draw(app, ui, &fit, right - fit.width, y);
    right - fit.width - GAP
}

/// The pills of `fit` from (`x`, `y`), each a press: a stick's inserts it
/// or pulls it, the Sticks pill opens or closes the menu.
fn bar_draw(app: &View, ui: &mut Ui, fit: &BarFit, mut x: f32, y: f32) {
    let h = MODE_PILL_HEIGHT;
    let light = ui.theme.is_light();
    let open = app.sheet == Some(crate::Sheet::SticksMenu);
    for it in &fit.items {
        let w = it.w;
        let (fg, edge, body, dot, action) = match it.pill {
            Some(k) => {
                let p = &fit.pills[k];
                let action = if it.menu {
                    Action::SticksMenu
                } else {
                    Action::StickPill(p.n as u8)
                };
                let (fg, edge, body, tone) = paints(p, ui.is_pressed(action), light);
                (fg, edge, body, Some(tone), action)
            }
            None => {
                let action = Action::SticksMenu;
                let (fg, edge, body) = if open {
                    (
                        Role::OnSecondaryContainer.paint(),
                        Role::Outline.paint(),
                        Role::SecondaryContainer.paint(),
                    )
                } else {
                    quiet(ui.is_pressed(action))
                };
                // A stick in the menu that the flow asks for edges the
                // pill in its tone.
                let shown: Vec<usize> = fit.items.iter().filter_map(|i| i.pill).collect();
                let asked = (0..fit.pills.len())
                    .find(|k| !shown.contains(k) && fit.pills[*k].asked)
                    .map(|k| paints(&fit.pills[k], false, light).3);
                (fg, asked.unwrap_or(edge), body, None, action)
            }
        };
        ui.fill(x, y, w, h, h / 2.0, body);
        ui.stroke(x, y, w, h, h / 2.0, edge);
        let mut tx = x + MODE_PILL_PAD;
        if let Some(tone) = dot {
            ui.dot(tx + GAP_SMALL, y + h / 2.0, GAP_SMALL, tone);
            tx += GRID + GAP_SMALL;
        }
        ui.text_mid(tx, y, h, CAPTION, W::S, fg, &it.text);
        ui.hit(x, y, w, h, action);
        x += w + GAP;
    }
}

/// Costs a text measure per pill. Over an open sheet, the top bar's stick
/// pills again where the page drew them, still taking a press: a sheet
/// may wait for a stick, which is put in from its pill.
pub(crate) fn over_sheet(app: &View, ui: &mut Ui) {
    if app.sheet.is_none() {
        return;
    }
    let Some((right, y, room)) = app.stick.bar_spot() else {
        return;
    };
    let Some(fit) = bar_fit(app, ui, room) else {
        return;
    };
    let was = (ui.ox, ui.oy, ui.in_sheet);
    (ui.ox, ui.oy, ui.in_sheet) = (0.0, 0.0, false);
    bar_draw(app, ui, &fit, right - fit.width, y);
    (ui.ox, ui.oy, ui.in_sheet) = was;
}

/// How much of the Sticks menu shows while it moves, 0 to 1: opening on
/// Material's emphasized decelerate, closing on its emphasized
/// accelerate; 1 at rest. Arithmetic.
fn menu_shown(app: &View) -> f32 {
    use crate::motion::{MENU_IN_MS, MENU_OUT_MS, ease, progress};
    use faraday_draw::tokens::{EASE_EMPHASIZED_ACCELERATE, EASE_EMPHASIZED_DECELERATE};
    match app.stick.menu_motion() {
        Some((true, at)) => ease(
            EASE_EMPHASIZED_DECELERATE,
            progress(at, app.now_ms, MENU_IN_MS),
        ),
        Some((false, at)) => {
            1.0 - ease(
                EASE_EMPHASIZED_ACCELERATE,
                progress(at, app.now_ms, MENU_OUT_MS),
            )
        }
        None => 1.0,
    }
}

/// Costs a text measure per stick. The Sticks menu: every stick with its
/// state, a press inserting or pulling it; a popup under its pill, a
/// sheet on a compact window (Material's menu, `crate::menu`).
pub(crate) fn menu_sheet(app: &View, ui: &mut Ui, w: f32, h: f32) {
    menu_drawn(app, ui, w, h, menu_shown(app));
}

/// The menu drawn `shown` of the way open.
fn menu_drawn(app: &View, ui: &mut Ui, w: f32, h: f32, shown: f32) {
    let lines: Vec<crate::menu::Line> = app
        .stick_pills()
        .iter()
        .map(|p| crate::menu::Line::Item(with_state(p), Action::StickPill(p.n as u8)))
        .collect();
    crate::menu::draw(
        ui,
        w,
        h,
        &crate::menu::View {
            title: EN.stick_menu_title,
            icon: faraday_draw::widgets::Icon::Ellipsis,
            lines: &lines,
            at: 0,
            keys: false,
            anchor: app.stick.menu_anchor(),
            cancel: (EN.nav_close, Action::Cancel),
            shown,
        },
    );
}

/// Costs a text measure per stick. The Sticks menu closing: drawn over the
/// page as it leaves, taking no press, until its motion ends.
pub(crate) fn menu_leaving(app: &View, ui: &mut Ui, w: f32, h: f32) {
    if app.sheet == Some(crate::Sheet::SticksMenu) || ui.compact {
        return;
    }
    if !matches!(app.stick.menu_motion(), Some((false, _))) {
        return;
    }
    let mark = (ui.hits.len(), ui.outside.take());
    let was = (ui.ox, ui.oy);
    (ui.ox, ui.oy) = (0.0, 0.0);
    menu_drawn(app, ui, w, h, menu_shown(app));
    (ui.ox, ui.oy) = was;
    ui.hits.truncate(mark.0);
    ui.outside = mark.1;
}

impl Faraday {
    /// A press that is the shell's, not the page's, so it acts under any
    /// sheet and waits for nothing: a stick's pill (a stick chosen in the
    /// Sticks menu closes it), the Sticks pill, and the menu's way out.
    /// Returns whether it was one.
    pub(crate) fn stick_bar_press(&mut self, action: Action) -> bool {
        match action {
            Action::StickPill(n) => {
                self.sticks_menu_close();
                self.stick_pill_pressed(n);
            }
            Action::SticksMenu => self.sticks_menu_pressed(),
            Action::Cancel if self.sheet == Some(crate::Sheet::SticksMenu) => {
                self.sticks_menu_close();
            }
            _ => return false,
        }
        true
    }

    /// The Sticks pill pressed: the menu opens under it, or closes when
    /// open. It moves as Material's menu does, at once under Reduce
    /// motion and on a compact window, where it is a sheet.
    pub(crate) fn sticks_menu_pressed(&mut self) {
        if self.sheet == Some(crate::Sheet::SticksMenu) {
            self.sticks_menu_close();
            return;
        }
        let anchor = self
            .pointer
            .rect_of(Action::SticksMenu)
            .map(|r| (r.x, r.y, r.w, r.h));
        let moving = !self.reduce_motion() && !self.compact;
        self.stick.set_menu(anchor, moving);
        self.sheet = Some(crate::Sheet::SticksMenu);
        self.scroll.reset_sheet();
    }

    /// The Sticks menu closes: a stick chosen in it, a press beside it,
    /// Escape or its pill again.
    pub(crate) fn sticks_menu_close(&mut self) {
        if self.sheet != Some(crate::Sheet::SticksMenu) {
            return;
        }
        self.sheet = None;
        let moving = !self.reduce_motion() && !self.compact;
        self.stick.close_menu(moving);
    }

    /// The Sticks menu's motion at a tick: whether it still moves, so
    /// the frame is drawn again.
    pub(crate) fn sticks_menu_tick(&mut self, now_ms: u64) -> bool {
        use crate::motion::{MENU_IN_MS, MENU_OUT_MS};
        self.stick
            .menu_tick(now_ms, |open| if open { MENU_IN_MS } else { MENU_OUT_MS })
    }
}
