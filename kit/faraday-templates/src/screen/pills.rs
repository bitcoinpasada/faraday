//! The stick pills: a strip across the top of the window on the one-column layouts, or in one
//! row in the top app bar beside the sidebar. A pill is the stick-pill molecule; this lays them
//! out: the longest form of their words that fits, in rows.

use faraday_atoms::{Offered, Pressed, Shown, Ui};
use faraday_molecules::stick_pill::{self, PillAt, StickPillModel, StickState};
use faraday_tokens::dims::{APP_BAR_H, APP_BAR_PILLS_TITLE_ROOM, GAP, LABEL_M};
use faraday_tokens::dims::{MODE_PILL_HEIGHT, MODE_PILL_PAD};

/// One stick's pill, in the three forms of its words (long, short, then its number).
pub struct StripPill<'s, A> {
    /// The stick's tone, counted from the boot stick's, which is 0.
    pub n: usize,
    pub state: StickState,
    pub long: Shown<'s>,
    pub short: Shown<'s>,
    pub number: Shown<'s>,
    /// A press on it, where the shell's sticks are its own; `Offered::No` for none.
    pub offered: Offered<A>,
}

/// Where the pills stand.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PillsPlace {
    /// A strip across the top of the window, the page under it.
    Strip,
    /// One row in the top app bar, ending at `right` (left of the bar's actions); the bar's
    /// `right` is left of them, by [`bar_room`].
    Bar { right: f32 },
}

/// The pills to draw and where.
pub struct PillsModel<'s, A> {
    pub pills: Vec<StripPill<'s, A>>,
    pub place: PillsPlace,
}

/// A pill's measures, in design units.
struct Size {
    h: f32,
    pad: f32,
    dot_r: f32,
    dot_room: f32,
    gap: f32,
    strip_pad: f32,
    side: f32,
}

/// On a wide display the page's own buttons sit at the right, so the row keeps clear of them.
const WIDE: Size = Size {
    h: MODE_PILL_HEIGHT,
    pad: MODE_PILL_PAD,
    dot_r: 4.0,
    dot_room: 12.0,
    gap: 8.0,
    strip_pad: 4.0,
    side: 70.0,
};

/// On a small panel the strip is a band of its own above the page.
const SMALL: Size = Size {
    h: 24.0,
    pad: 9.0,
    dot_r: 4.0,
    dot_room: 10.0,
    gap: 6.0,
    strip_pad: 4.0,
    side: 8.0,
};

fn size<A: Pressed>(ui: &Ui<'_, A>) -> &'static Size {
    if ui.compact { &SMALL } else { &WIDE }
}

#[derive(Clone, Copy, PartialEq)]
enum Form {
    Long,
    Short,
    Number,
}

struct Layout {
    form: Form,
    widths: Vec<f32>,
    /// The pills of each row, by index, and the row's width.
    rows: Vec<(Vec<usize>, f32)>,
}

fn words<'a, 's, A>(p: &'a StripPill<'s, A>, form: Form) -> &'a Shown<'s> {
    match form {
        Form::Long => &p.long,
        Form::Short => &p.short,
        Form::Number => &p.number,
    }
}

fn layout<A: Pressed>(pills: &[StripPill<'_, A>], ui: &Ui<'_, A>, z: &Size, avail: f32) -> Layout {
    let build = |form: Form| -> Vec<f32> {
        pills
            .iter()
            .map(|p| 2.0 * z.pad + z.dot_room + ui.measure_as(LABEL_M, words(p, form).as_str()))
            .collect()
    };
    let total = |w: &[f32]| w.iter().sum::<f32>() + z.gap * w.len().saturating_sub(1) as f32;
    let mut form = Form::Long;
    let mut widths = build(form);
    for next in [Form::Short, Form::Number] {
        if total(&widths) <= avail {
            break;
        }
        form = next;
        widths = build(form);
    }
    if total(&widths) > avail {
        form = Form::Short;
        widths = build(form);
    }
    let mut rows: Vec<(Vec<usize>, f32)> = Vec::new();
    for (k, w) in widths.iter().enumerate() {
        match rows.last_mut() {
            Some((items, width)) if *width + z.gap + w <= avail => {
                *width += z.gap + w;
                items.push(k);
            }
            _ => rows.push((vec![k], *w)),
        }
    }
    Layout { form, widths, rows }
}

fn one<A: Pressed>(p: &StripPill<'_, A>, form: Form, ui: &mut Ui<'_, A>, at: PillAt) {
    let m = StickPillModel {
        n: p.n.saturating_sub(1),
        state: p.state,
        label: words(p, form).clone(),
    };
    stick_pill::draw(&m, ui, at, &p.offered);
}

fn at(z: &Size, x: f32, y: f32, w: f32) -> PillAt {
    PillAt {
        x,
        y,
        w,
        h: z.h,
        pad: z.pad,
        dot_r: z.dot_r,
        dot_room: z.dot_room,
    }
}

/// How tall the strip across a window `window_w` wide is, in units; 0 for none.
pub fn strip_height<A: Pressed>(m: &PillsModel<'_, A>, ui: &Ui<'_, A>, window_w: f32) -> f32 {
    if m.pills.is_empty() || !matches!(m.place, PillsPlace::Strip) {
        return 0.0;
    }
    let z = size(ui);
    let rows = layout(&m.pills, ui, z, window_w - 2.0 * z.side).rows.len() as f32;
    2.0 * z.strip_pad + rows * z.h + (rows - 1.0) * z.gap
}

/// How much of the bar's width the pills in it take, the gap before them counted: what a page
/// subtracts from the bar's `right` for the title. Nothing where they do not stand in the bar.
pub fn bar_room<A: Pressed>(m: &PillsModel<'_, A>, ui: &Ui<'_, A>) -> f32 {
    let PillsPlace::Bar { right } = m.place else {
        return 0.0;
    };
    if m.pills.is_empty() {
        return 0.0;
    }
    let z = size(ui);
    let l = layout(&m.pills, ui, z, right - APP_BAR_PILLS_TITLE_ROOM);
    l.widths.iter().sum::<f32>() + z.gap * (m.pills.len() - 1) as f32 + GAP
}

/// Draws the pills where they stand: centred in rows across `window_w` for the strip, in one row
/// in the bar's height for the bar. The caller sets the origin: the window's for the strip, the
/// top app bar's for the bar.
pub fn draw<A: Pressed>(m: &PillsModel<'_, A>, ui: &mut Ui<'_, A>, window_w: f32) {
    if m.pills.is_empty() {
        return;
    }
    let z = size(ui);
    match m.place {
        PillsPlace::Strip => {
            let l = layout(&m.pills, ui, z, window_w - 2.0 * z.side);
            for (r, (items, width)) in l.rows.iter().enumerate() {
                let y = z.strip_pad + r as f32 * (z.h + z.gap);
                let mut x = (window_w - width) / 2.0;
                for &k in items {
                    one(&m.pills[k], l.form, ui, at(z, x, y, l.widths[k]));
                    x += l.widths[k] + z.gap;
                }
            }
        }
        PillsPlace::Bar { right } => {
            let l = layout(&m.pills, ui, z, right - APP_BAR_PILLS_TITLE_ROOM);
            let total = l.widths.iter().sum::<f32>() + z.gap * (m.pills.len() - 1) as f32;
            let (mut x, y) = (right - GAP - total, (APP_BAR_H - z.h) / 2.0);
            for (k, p) in m.pills.iter().enumerate() {
                one(p, l.form, ui, at(z, x, y, l.widths[k]));
                x += l.widths[k] + z.gap;
            }
        }
    }
}
