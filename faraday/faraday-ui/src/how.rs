//! A block (`docs/CRYPTO-LEARN.md` §8.5) drawn as a Material list: the
//! same list in Faraday's Learn tab and in the Starter's sheet, at every
//! width. A field is one list item, its headline the field's label in
//! Body L and its supporting text the value in Body M; an item is as
//! tall as its text and cuts nothing (F148 records this difference from
//! Material, whose item stops at three lines). A block is read, not
//! acted on: its items take no press.

use faraday_draw::tokens::{
    BODY_L, BODY_M, GAP, GAP_SMALL, ICON_GLYPH_SMALL, LIST_PAD_V, PAD, TITLE_S, TypeRole,
};
use faraday_draw::widgets::Icon;
use faraday_learn::block::{Area, Block, CrossCheck, Kind, Line};
use faraday_learn::construction::{Construction, Item, Table};
use faraday_strings::{EN, fill};
use std::sync::{Arc, Mutex};

use crate::{Role, Ui};

/// The blocks in `blocks`, one after another from (x, y), `w` wide, a
/// full-width divider with `GAP` above and below between two. `release`
/// is the build's own release string, put before each Code entry.
/// Returns the height used. A block's height is kept for the width and
/// text size it was drawn at, and a block wholly outside the clip is not
/// drawn again: it costs a lookup. Costs a pass over the text of the
/// blocks in view, and of those not yet drawn.
pub fn blocks<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, w): (f32, f32, f32),
    blocks: &[&Block],
    release: &str,
) -> f32 {
    let mut cy = y;
    for (i, b) in blocks.iter().enumerate() {
        if i > 0 {
            cy += GAP;
            ui.rule(x, cy, w, Role::OutlineVariant);
            cy += GAP;
        }
        cy += kept_block(ui, (x, cy, w), b, release);
    }
    cy - y
}

/// What a kept height is kept for: the width and scale in pixels and the
/// two line heights the block's text is set in.
type Key = (i32, i32, u32, u32);

/// The kept height of each part (0 the whole) at a `Key`, as drawn.
type Heights = Mutex<Vec<(Key, usize, f32)>>;

/// [`block`], or its kept height where the block lies wholly outside the
/// clip and has been drawn at this width and text size before.
fn kept_block<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    at: (f32, f32, f32),
    b: &Block,
    release: &str,
) -> f32 {
    let key = (
        ui.px(at.2),
        ui.px(GAP),
        ui.line_as(BODY_M).to_bits(),
        ui.line_as(BODY_L).to_bits(),
    );
    let cooked = cooked(b, release);
    kept(ui, &cooked.heights, key, 0, at.1, &mut |ui, y| {
        block(ui, (at.0, y, at.2), b, &cooked, key)
    })
}

/// `draw` at `y`, which returns the height it took, unless the part
/// `index` of the block is known to be that tall at `key` and lies wholly
/// outside the clip: then only its kept height. A lookup in that case.
fn kept<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    store: &Heights,
    key: Key,
    index: usize,
    y: f32,
    draw: &mut dyn FnMut(&mut Ui<A>, f32) -> f32,
) -> f32 {
    let known = {
        let heights = store.lock().unwrap_or_else(|e| e.into_inner());
        heights
            .iter()
            .find(|(k, n, _)| *k == key && *n == index)
            .map(|(_, _, h)| *h)
    };
    if let Some(h) = known {
        let clip = ui.c.clip();
        let (top, bottom) = (ui.px(y + ui.oy), ui.px(y + ui.oy + h));
        if bottom < clip.y || top > clip.y + clip.h {
            return h;
        }
    }
    let h = draw(ui, y);
    if known.is_none() {
        let mut heights = store.lock().unwrap_or_else(|e| e.into_inner());
        heights.push((key, index, h));
    }
    h
}

/// One block from (x, y): the action's name in Title S, then the nine
/// fields in order, each kept apart (`kept`). Returns the height used.
fn block<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, w): (f32, f32, f32),
    b: &Block,
    cooked: &Cooked,
    key: Key,
) -> f32 {
    let mut cy = y;
    cy += kept(ui, &cooked.heights, key, 1, cy, &mut |ui, y| {
        ui.wrap_as((x, y, w), TITLE_S, Role::OnSurface, b.name)
    });
    let one = [Line {
        text: b.action,
        kind: Kind::Plain,
    }];
    let fields: [(&str, &[Line]); 4] = [
        (EN.how_field_action, &one),
        (EN.how_field_standards, b.standards),
        (EN.how_field_implementation, b.implementation),
        (EN.how_field_checked, b.checked),
    ];
    for (n, (label, lines)) in fields.into_iter().enumerate() {
        cy += kept(ui, &cooked.heights, key, 2 + n, cy, &mut |ui, y| {
            field(ui, (x, y, w), label, lines)
        });
    }
    cy += kept(ui, &cooked.heights, key, 6, cy, &mut |ui, y| {
        cross_check(ui, (x, y, w), &b.cross_check, &cooked.run)
    });
    cy += kept(ui, &cooked.heights, key, 7, cy, &mut |ui, y| {
        item(ui, (x, y, w), EN.how_field_code, |ui, at| {
            let all = cooked.code.iter().map(String::as_str);
            entries(
                ui,
                at,
                all.chain([EN.how_repository]).map(|t| (t, Kind::Mono)),
            )
        })
    });
    let rest: [(&str, &[Line]); 3] = [
        (EN.how_field_secret, b.secret),
        (EN.how_field_limits, b.limits),
        (EN.how_field_own_calls, b.own_calls),
    ];
    for (n, (label, lines)) in rest.into_iter().enumerate() {
        cy += kept(ui, &cooked.heights, key, 8 + n, cy, &mut |ui, y| {
            field(ui, (x, y, w), label, lines)
        });
    }
    cy - y
}

/// A field whose value is `lines`.
fn field<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    at: (f32, f32, f32),
    label: &str,
    lines: &[Line],
) -> f32 {
    item(ui, at, label, |ui, at| {
        entries(ui, at, lines.iter().map(|l| (l.text, l.kind)))
    })
}

/// Cross-check: one item, its parts each a paragraph led by the part's
/// name in the medium face of Body M. An action with no area has the one
/// paragraph that says why.
fn cross_check<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    at: (f32, f32, f32),
    cc: &CrossCheck,
    run: &str,
) -> f32 {
    item(ui, at, EN.how_field_crosscheck, |ui, (x, y, w)| match cc {
        CrossCheck::Not(why) => line(ui, (x, y, w), why, Kind::Warn),
        CrossCheck::Area(a) => area(ui, (x, y, w), a, run),
    })
}

/// An area's parts, a `GAP_SMALL` apart.
fn area<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, w): (f32, f32, f32),
    a: &Area,
    run: &str,
) -> f32 {
    let name = |ui: &mut Ui<A>, cy: f32, name: &str| {
        GAP_SMALL
            + ui.wrap_as(
                (x, cy + GAP_SMALL, w),
                BODY_M.medium(),
                Role::OnSurfaceVariant,
                name,
            )
    };
    let mut cy = y;
    cy += ui.wrap_as(
        (x, cy, w),
        BODY_M.medium(),
        Role::OnSurfaceVariant,
        EN.how_cc_area,
    );
    cy += line(ui, (x, cy, w), a.area, Kind::Plain);
    let parts: [(&str, &[Line]); 6] = [
        (EN.how_cc_code, a.code),
        (EN.how_cc_second, a.second),
        (EN.how_cc_vectors, a.vectors),
        (EN.how_cc_random, a.random),
        (EN.how_cc_differences, a.differences),
        (EN.how_cc_when, a.when),
    ];
    for (label, lines) in parts {
        cy += name(ui, cy, label);
        for l in lines {
            cy += line(ui, (x, cy, w), l.text, l.kind);
        }
    }
    cy += name(ui, cy, EN.how_cc_last_run);
    cy += line(ui, (x, cy, w), run, Kind::Mono);
    cy - y
}

/// What a block's drawing formats from the release and its manifest text:
/// the Code entries and the Cross-check's last run.
struct Cooked {
    /// The height of each part of the block (0 the whole) at a `Key`, as drawn.
    heights: Heights,
    release: String,
    code: Vec<String>,
    run: String,
}

/// The formatted Code entries and last run of `b` for `release`. Costs the
/// formats the first time a block is drawn in a process (the release does
/// not change within one); a lookup, and a reference count, after it.
fn cooked(b: &Block, release: &str) -> Arc<Cooked> {
    static KEPT: Mutex<Vec<(&'static str, Arc<Cooked>)>> = Mutex::new(Vec::new());
    let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, c)) = kept
        .iter()
        .find(|(id, c)| *id == b.id && c.release == release)
    {
        return Arc::clone(c);
    }
    let code = b
        .code
        .iter()
        .map(|c| fill(EN.how_code_entry, &[release, c.file, c.function]))
        .collect();
    let run = match b.cross_check {
        CrossCheck::Area(a) => fill(EN.how_cc_seed_cases, &[a.seed, &a.cases.to_string()]),
        CrossCheck::Not(_) => String::new(),
    };
    let c = Arc::new(Cooked {
        heights: Mutex::new(Vec::new()),
        release: release.to_string(),
        code,
        run,
    });
    kept.retain(|(id, _)| *id != b.id);
    kept.push((b.id, Arc::clone(&c)));
    c
}

/// A list item at (x, y): its headline `label` in Body L, `value` drawn
/// under it at the supporting text's place, the item's vertical padding
/// above and below. Returns the item's height.
fn item<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, w): (f32, f32, f32),
    label: &str,
    value: impl FnOnce(&mut Ui<A>, (f32, f32, f32)) -> f32,
) -> f32 {
    let top = y + LIST_PAD_V;
    let head = ui.wrap_as((x, top, w), BODY_L, Role::OnSurface, label);
    let body = value(ui, (x, top + head, w));
    2.0 * LIST_PAD_V + head + body
}

/// Each entry its own paragraph, `GAP_SMALL` between. Returns the height.
fn entries<'a, A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, w): (f32, f32, f32),
    all: impl Iterator<Item = (&'a str, Kind)>,
) -> f32 {
    let mut cy = y;
    for (i, (text, kind)) in all.enumerate() {
        if i > 0 {
            cy += GAP_SMALL;
        }
        cy += line(ui, (x, cy, w), text, kind);
    }
    cy - y
}

/// One entry: Body M in on-surface-variant, in the mono face where it is
/// read character by character, and what is not checked in the Warning
/// role with `Icon::Warning` before it, so that colour is not all that
/// tells it. Returns its height.
fn line<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, w): (f32, f32, f32),
    text: &str,
    kind: Kind,
) -> f32 {
    match kind {
        Kind::Plain => ui.wrap_as((x, y, w), BODY_M, Role::OnSurfaceVariant, text),
        Kind::Mono => ui.wrap_as((x, y, w), BODY_M.mono(), Role::OnSurfaceVariant, text),
        Kind::Warn => {
            let line = ui.line_as(BODY_M);
            let iy = y + (line - ICON_GLYPH_SMALL) / 2.0;
            ui.icon(
                x,
                iy,
                ICON_GLYPH_SMALL,
                Icon::Warning,
                ICON_GLYPH_SMALL,
                Role::Warning,
            );
            let tx = x + ICON_GLYPH_SMALL + GAP_SMALL;
            ui.wrap_as((tx, y, w - (tx - x)), BODY_M, Role::Warning, text)
        }
    }
}

/// The type and the room a construction is drawn with: a Learn page's
/// own (`docs/CRYPTO-LEARN.md` §18.2), so its sections read as more of
/// the page's.
#[derive(Debug, Clone, Copy)]
pub struct PageLook {
    /// The construction's title, as a page's title.
    pub title: TypeRole,
    /// A section's heading.
    pub heading: TypeRole,
    /// A paragraph.
    pub body: TypeRole,
    /// The room under the title, a heading, a paragraph and a section.
    pub after: [f32; 4],
}

/// A construction from (x, y), `w` wide, after a Learn page's own
/// sections: its title, then each section's heading and its paragraphs
/// and tables. It is read, not acted on: nothing in it takes a press.
/// Returns the height used. Each part's height is kept for the width and
/// text size it was drawn at, as a block's is, and a part wholly outside
/// the clip is not drawn again: it costs a lookup. Costs a pass over the
/// text of the parts in view, and of those not yet drawn.
pub fn construction<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, w): (f32, f32, f32),
    c: &Construction,
    look: &PageLook,
) -> f32 {
    let key = (
        ui.px(w),
        ui.px(ui.line_as(look.heading)),
        ui.line_as(look.body).to_bits(),
        ui.line_as(BODY_M).to_bits(),
    );
    let store = construction_heights(c.id);
    kept(ui, &store, key, 0, y, &mut |ui, y| {
        construction_parts(ui, (x, y, w), c, look, &store, key)
    })
}

/// The parts of a construction, each kept apart (`kept`), numbered from 1
/// in the order drawn. Returns the height used.
fn construction_parts<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, w): (f32, f32, f32),
    c: &Construction,
    look: &PageLook,
    store: &Heights,
    key: Key,
) -> f32 {
    let [after_title, after_heading, after_para, after_section] = look.after;
    let mut n = 1;
    let mut cy = y;
    cy += kept(ui, store, key, n, cy, &mut |ui, y| {
        ui.wrap_as((x, y, w), look.title, Role::OnSurface, c.title)
    }) + after_title;
    for part in c.sections {
        n += 1;
        cy += kept(ui, store, key, n, cy, &mut |ui, y| {
            ui.wrap_as((x, y, w), look.heading, Role::Primary, part.heading)
        }) + after_heading;
        for it in part.items {
            n += 1;
            cy += kept(ui, store, key, n, cy, &mut |ui, y| match it {
                Item::Para(text) => ui.wrap_as((x, y, w), look.body, Role::OnSurface, text),
                Item::Table(t) => table(ui, (x, y, w), t),
            }) + after_para;
        }
        cy += after_section;
    }
    cy - y
}

/// The kept heights of the construction `id`. A lookup, and a reference
/// count, after the first time it is drawn in a process.
fn construction_heights(id: &'static str) -> Arc<Heights> {
    static KEPT: Mutex<Vec<(&'static str, Arc<Heights>)>> = Mutex::new(Vec::new());
    let mut kept = KEPT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((_, h)) = kept.iter().find(|(i, _)| *i == id) {
        return Arc::clone(h);
    }
    let h = Arc::new(Mutex::new(Vec::new()));
    kept.push((id, Arc::clone(&h)));
    h
}

/// A table: its heads in Title S on-surface-variant, its cells in Body M,
/// a divider under the heads and under each row (Material's data table),
/// columns as wide as their text where the row fits and shared out where
/// it does not, a cell's text wrapping in its column. Returns the height.
fn table<A: Copy + PartialEq>(ui: &mut Ui<A>, (x, y, w): (f32, f32, f32), t: &Table) -> f32 {
    let n = t.head.len();
    if n == 0 {
        return 0.0;
    }
    let natural: Vec<f32> = (0..n)
        .map(|i| {
            let head = ui.measure_as(TITLE_S, t.head[i]);
            t.rows
                .iter()
                .filter_map(|r| r.get(i))
                .map(|cell| ui.measure_as(BODY_M, cell))
                .fold(head, f32::max)
        })
        .collect();
    let widths = shared(&natural, (w - PAD * (n - 1) as f32).max(n as f32));
    let mut cy = y;
    let rows = std::iter::once((t.head, TITLE_S, Role::OnSurfaceVariant))
        .chain(t.rows.iter().map(|r| (*r, BODY_M, Role::OnSurface)));
    for (cells, role, colour) in rows {
        let mut cx = x;
        let mut tall: f32 = 0.0;
        for (cell, cw) in cells.iter().zip(&widths) {
            tall = tall.max(ui.wrap_as((cx, cy, *cw), role, colour, cell));
            cx += cw + PAD;
        }
        cy += tall + GAP_SMALL;
        ui.rule(x, cy, w, Role::OutlineVariant);
        cy += GAP_SMALL;
    }
    cy - y
}

/// Column widths in `room`: each column's natural width where all fit;
/// otherwise the narrow columns keep theirs and the rest share what is
/// left equally.
fn shared(natural: &[f32], room: f32) -> Vec<f32> {
    if natural.iter().sum::<f32>() <= room {
        return natural.to_vec();
    }
    let mut order: Vec<usize> = (0..natural.len()).collect();
    order.sort_by(|a, b| natural[*a].total_cmp(&natural[*b]));
    let mut out = vec![0.0; natural.len()];
    let mut left = room;
    for (k, &i) in order.iter().enumerate() {
        let share = left / (natural.len() - k) as f32;
        out[i] = natural[i].min(share);
        left -= out[i];
    }
    out
}
