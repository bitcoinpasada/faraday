//! The How blocks organism: the guide's Cryptography tab (`docs/CRYPTO-LEARN.md` §8.5, 09 §3.3).
//! A block is drawn as a Material list: a field is one list item, its headline the field's label
//! in Body L and its value under it in Body M; an item is as tall as its text and cuts nothing.
//! A block is read, not acted on: nothing in it takes a press. A construction is drawn after the
//! last block of the actions it belongs to, in the page's own look.
//!
//! The words come in the model as data; the organism names no feature and no page. Only the
//! lines in view are drawn (`Ui::wrap_seen`); the height of every part is still counted.

use faraday_atoms::{Anchor, HeightsKey, Icon, Pressed, Shown, Ui};
use faraday_tokens::dims::{BODY_L, BODY_M, GAP, GAP_SMALL, ICON_GLYPH_SMALL, LIST_PAD_V, PAD};
use faraday_tokens::dims::{TITLE_M, TITLE_S};
use faraday_tokens::palette::Role;
use faraday_tokens::type_roles::TypeRole;

/// How an entry's text is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Body M in on-surface-variant.
    Plain,
    /// The mono face, where it is read character by character.
    Mono,
    /// What is not checked: the warning role with an icon before it, so that colour is not all
    /// that tells it.
    Warn,
}

/// One entry of a field: a paragraph of its own.
pub struct Line<'s> {
    pub text: Shown<'s>,
    pub kind: Kind,
}

/// What a field holds.
pub enum FieldBody<'s> {
    /// Entries, each a paragraph, a small gap apart.
    Lines(Vec<Line<'s>>),
    /// The cross-check: the independence lines first, then what it says.
    Cross {
        independence: Vec<Line<'s>>,
        outcome: Cross<'s>,
    },
}

/// What the cross-check says.
pub enum Cross<'s> {
    /// The action has no area: the one line that says why.
    Not(Shown<'s>),
    /// The area's parts: the heading, the area's line and each part, a part's name in the medium
    /// face over its lines.
    Area {
        heading: Shown<'s>,
        area: Shown<'s>,
        parts: Vec<(Shown<'s>, Vec<Line<'s>>)>,
    },
}

/// A field: its label and what it holds.
pub struct Field<'s> {
    pub label: Shown<'s>,
    pub body: FieldBody<'s>,
}

/// One action's block: its name in Title S, then its fields in order.
pub struct HowBlock<'s> {
    pub name: Shown<'s>,
    pub fields: Vec<Field<'s>>,
}

/// A table: its heads in Title S, its cells in Body M.
pub struct Table<'s> {
    pub head: Vec<Shown<'s>>,
    pub rows: Vec<Vec<Shown<'s>>>,
}

/// What a section of a construction holds.
pub enum Item<'s> {
    Para(Shown<'s>),
    Table(Table<'s>),
}

/// A construction's section: its heading and its paragraphs and tables.
pub struct Section<'s> {
    pub heading: Shown<'s>,
    pub items: Vec<Item<'s>>,
}

/// A construction: its title and its sections.
pub struct Construction<'s> {
    pub title: Shown<'s>,
    pub sections: Vec<Section<'s>>,
}

/// What follows what, top down.
pub enum Part<'s> {
    Block(HowBlock<'s>),
    /// Drawn after the last block of the actions it belongs to.
    Construction(Construction<'s>),
}

/// The type and the room a construction is drawn with, so that its sections read as the tab's
/// own text.
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

impl PageLook {
    /// The look of the Starter's Cryptography sheet.
    pub const SHEET: PageLook = PageLook {
        title: TITLE_M,
        heading: TITLE_S,
        body: BODY_M,
        after: [PAD, GAP, GAP + GAP_SMALL, GAP + GAP_SMALL],
    };
}

/// The How tab's body.
pub struct HowModel<'s> {
    pub parts: Vec<Part<'s>>,
    pub look: PageLook,
    /// Keys the parts' heights in the motion store, which `draw` writes and reads (04 V15).
    pub id: Anchor,
    /// The part a jump is to reach: `draw_found` says where it begins.
    pub target: Option<usize>,
}

/// Draws the parts one after another from (x, y), `w` wide, a full-width divider with a gap above
/// and below between two. Returns the height used.
pub fn draw<A: Pressed>(m: &HowModel<'_>, ui: &mut Ui<'_, A>, at: (f32, f32, f32)) -> f32 {
    draw_found(m, ui, at).0
}

// What `draw_found` counts, so that a test can see a part measured.
#[cfg(test)]
thread_local! {
    pub(crate) static MEASURED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// What the kept heights depend on: the width, the text size and the scale, and the parts
/// (their count and the letters of their titles).
fn key<A: Pressed>(m: &HowModel<'_>, ui: &Ui<'_, A>, w: f32) -> HeightsKey {
    let mut shape: u32 = 0x811c_9dc5;
    let mut eat = |b: u8| shape = (shape ^ u32::from(b)).wrapping_mul(0x0100_0193);
    for part in &m.parts {
        let title = match part {
            Part::Block(b) => b.name.as_str(),
            Part::Construction(c) => c.title.as_str(),
        };
        title.bytes().for_each(&mut eat);
        eat(0);
    }
    [
        w.to_bits(),
        ui.line_as(BODY_M).to_bits(),
        ui.f.to_bits(),
        shape ^ (m.parts.len() as u32),
    ]
}

/// [`draw`], and where within the parts the one marked `target` begins. Each part's height is
/// kept in the motion store (`Ui::keep_heights`); the next frame under the same width, text size
/// and parts reads them and passes over a part out of view without measuring it.
pub fn draw_found<A: Pressed>(
    m: &HowModel<'_>,
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
) -> (f32, Option<f32>) {
    let key = key(m, ui, w);
    let kept: Vec<f32> = match ui.kept_heights(m.id, key) {
        Some(h) if h.len() == m.parts.len() => h.to_vec(),
        _ => Vec::new(),
    };
    let mut heights = Vec::with_capacity(m.parts.len());
    let mut found = None;
    let mut cy = y;
    for (i, part) in m.parts.iter().enumerate() {
        if i > 0 {
            cy += GAP;
            ui.rule(x, cy, w, Role::OutlineVariant);
            cy += GAP;
        }
        if m.target == Some(i) {
            found = Some(cy - y);
        }
        let h = match kept.get(i) {
            Some(&h) if !ui.shows(cy, h) => h,
            _ => {
                #[cfg(test)]
                MEASURED.with(|c| c.set(c.get() + 1));
                match part {
                    Part::Block(b) => block(ui, (x, cy, w), b),
                    Part::Construction(c) => construction(ui, (x, cy, w), c, &m.look),
                }
            }
        };
        heights.push(h);
        cy += h;
    }
    ui.keep_heights(m.id, key, &heights);
    (cy - y, found)
}

/// One block from (x, y): the action's name in Title S, then the fields.
fn block<A: Pressed>(ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32), b: &HowBlock<'_>) -> f32 {
    let mut cy = y;
    cy += ui.wrap_seen((x, cy, w), TITLE_S, Role::OnSurface, b.name.as_str());
    for f in &b.fields {
        cy += field(ui, (x, cy, w), f);
    }
    cy - y
}

/// A list item at (x, y): its headline `label` in Body L, the value drawn under it, the item's
/// vertical padding above and below. Returns the item's height.
fn item<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    label: &str,
    value: impl FnOnce(&mut Ui<'_, A>, (f32, f32, f32)) -> f32,
) -> f32 {
    let top = y + LIST_PAD_V;
    let head = ui.wrap_seen((x, top, w), BODY_L, Role::OnSurface, label);
    let body = value(ui, (x, top + head, w));
    2.0 * LIST_PAD_V + head + body
}

fn field<A: Pressed>(ui: &mut Ui<'_, A>, at: (f32, f32, f32), f: &Field<'_>) -> f32 {
    item(ui, at, f.label.as_str(), |ui, at| match &f.body {
        FieldBody::Lines(lines) => entries(ui, at, lines),
        FieldBody::Cross {
            independence,
            outcome,
        } => cross(ui, at, independence, outcome),
    })
}

/// The cross-check: the independence lines, each a paragraph, then the outcome.
fn cross<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    independence: &[Line<'_>],
    outcome: &Cross<'_>,
) -> f32 {
    let mut cy = y;
    if !independence.is_empty() {
        cy += entries(ui, (x, cy, w), independence);
        cy += GAP_SMALL;
    }
    cy += match outcome {
        Cross::Not(why) => line(ui, (x, cy, w), why.as_str(), Kind::Warn),
        Cross::Area {
            heading,
            area,
            parts,
        } => {
            let mut ay = cy;
            ay += ui.wrap_seen(
                (x, ay, w),
                BODY_M.medium(),
                Role::OnSurfaceVariant,
                heading.as_str(),
            );
            ay += line(ui, (x, ay, w), area.as_str(), Kind::Plain);
            for (name, lines) in parts {
                ay += GAP_SMALL
                    + ui.wrap_seen(
                        (x, ay + GAP_SMALL, w),
                        BODY_M.medium(),
                        Role::OnSurfaceVariant,
                        name.as_str(),
                    );
                for l in lines {
                    ay += line(ui, (x, ay, w), l.text.as_str(), l.kind);
                }
            }
            ay - cy
        }
    };
    cy - y
}

/// Each entry its own paragraph, a small gap between. Returns the height.
fn entries<A: Pressed>(ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32), all: &[Line<'_>]) -> f32 {
    let mut cy = y;
    for (i, l) in all.iter().enumerate() {
        if i > 0 {
            cy += GAP_SMALL;
        }
        cy += line(ui, (x, cy, w), l.text.as_str(), l.kind);
    }
    cy - y
}

/// One entry. Returns its height.
fn line<A: Pressed>(ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32), text: &str, kind: Kind) -> f32 {
    match kind {
        Kind::Plain => ui.wrap_seen((x, y, w), BODY_M, Role::OnSurfaceVariant, text),
        Kind::Mono => ui.wrap_seen((x, y, w), BODY_M.mono(), Role::OnSurfaceVariant, text),
        Kind::Warn => {
            let one = ui.line_as(BODY_M);
            let iy = y + (one - ICON_GLYPH_SMALL) / 2.0;
            ui.icon(
                x,
                iy,
                ICON_GLYPH_SMALL,
                Icon::Warning,
                ICON_GLYPH_SMALL,
                Role::Warning,
            );
            let tx = x + ICON_GLYPH_SMALL + GAP_SMALL;
            ui.wrap_seen((tx, y, w - (tx - x)), BODY_M, Role::Warning, text)
        }
    }
}

/// A construction from (x, y), `w` wide: its title, then each section's heading and its
/// paragraphs and tables. Returns the height used.
fn construction<A: Pressed>(
    ui: &mut Ui<'_, A>,
    (x, y, w): (f32, f32, f32),
    c: &Construction<'_>,
    look: &PageLook,
) -> f32 {
    let [after_title, after_heading, after_para, after_section] = look.after;
    let mut cy = y;
    cy += ui.wrap_seen((x, cy, w), look.title, Role::OnSurface, c.title.as_str()) + after_title;
    for part in &c.sections {
        cy += ui.wrap_seen(
            (x, cy, w),
            look.heading,
            Role::Primary,
            part.heading.as_str(),
        ) + after_heading;
        for it in &part.items {
            cy += match it {
                Item::Para(text) => {
                    ui.wrap_seen((x, cy, w), look.body, Role::OnSurface, text.as_str())
                }
                Item::Table(t) => table(ui, (x, cy, w), t),
            } + after_para;
        }
        cy += after_section;
    }
    cy - y
}

/// A table (Material's data table): its heads in Title S on-surface-variant, its cells in
/// Body M, a divider under the heads and under each row, columns as wide as their text where the
/// row fits and shared out where it does not, a cell's text wrapping in its column.
fn table<A: Pressed>(ui: &mut Ui<'_, A>, (x, y, w): (f32, f32, f32), t: &Table<'_>) -> f32 {
    let n = t.head.len();
    if n == 0 {
        return 0.0;
    }
    let natural: Vec<f32> = (0..n)
        .map(|i| {
            let head = ui.measure_as(TITLE_S, t.head[i].as_str());
            t.rows
                .iter()
                .filter_map(|r| r.get(i))
                .map(|cell| ui.measure_as(BODY_M, cell.as_str()))
                .fold(head, f32::max)
        })
        .collect();
    let widths = shared(&natural, (w - PAD * (n - 1) as f32).max(n as f32));
    let mut cy = y;
    let heads = (t.head.as_slice(), TITLE_S, Role::OnSurfaceVariant);
    let rows = t
        .rows
        .iter()
        .map(|r| (r.as_slice(), BODY_M, Role::OnSurface));
    for (cells, role, colour) in std::iter::once(heads).chain(rows) {
        let mut cx = x;
        let mut tall: f32 = 0.0;
        for (cell, cw) in cells.iter().zip(&widths) {
            tall = tall.max(ui.wrap_seen((cx, cy, *cw), role, colour, cell.as_str()));
            cx += cw + PAD;
        }
        cy += tall + GAP_SMALL;
        ui.rule(x, cy, w, Role::OutlineVariant);
        cy += GAP_SMALL;
    }
    cy - y
}

/// Column widths in `room`: each column's natural width where all fit; otherwise the narrow
/// columns keep theirs and the rest share what is left equally.
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
