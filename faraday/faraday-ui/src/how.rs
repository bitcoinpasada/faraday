//! A block (`docs/CRYPTO-LEARN.md` §8.5) drawn as a Material list: the
//! same list in Faraday's Learn tab and in the Starter's sheet, at every
//! width. A field is one list item, its headline the field's label in
//! Body L and its supporting text the value in Body M; an item is as
//! tall as its text and cuts nothing (F148 records this difference from
//! Material, whose item stops at three lines). A block is read, not
//! acted on: its items take no press.

use faraday_draw::tokens::{BODY_L, BODY_M, GAP, GAP_SMALL, ICON_GLYPH_SMALL, LIST_PAD_V, TITLE_S};
use faraday_draw::widgets::Icon;
use faraday_learn::block::{Area, Block, CrossCheck, Kind, Line};
use faraday_strings::{EN, fill};

use crate::{Role, Ui};

/// The blocks in `blocks`, one after another from (x, y), `w` wide, a
/// full-width divider with `GAP` above and below between two. `release`
/// is the build's own release string, put before each Code entry.
/// Returns the height used. Costs a pass over the blocks' text; the
/// strings it makes are the Code entries'.
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
        cy += block(ui, (x, cy, w), b, release);
    }
    cy - y
}

/// One block from (x, y): the action's name in Title S, then the nine
/// fields in order. Returns the height used.
fn block<A: Copy + PartialEq>(
    ui: &mut Ui<A>,
    (x, y, w): (f32, f32, f32),
    b: &Block,
    release: &str,
) -> f32 {
    let mut cy = y;
    cy += ui.wrap_as((x, cy, w), TITLE_S, Role::OnSurface, b.name);
    let one = [Line {
        text: b.action,
        kind: Kind::Plain,
    }];
    cy += field(ui, (x, cy, w), EN.how_field_action, &one);
    cy += field(ui, (x, cy, w), EN.how_field_standards, b.standards);
    cy += field(
        ui,
        (x, cy, w),
        EN.how_field_implementation,
        b.implementation,
    );
    cy += field(ui, (x, cy, w), EN.how_field_checked, b.checked);
    cy += cross_check(ui, (x, cy, w), &b.cross_check);
    let mut code: Vec<(String, Kind)> = b
        .code
        .iter()
        .map(|c| {
            (
                fill(EN.how_code_entry, &[release, c.file, c.function]),
                Kind::Mono,
            )
        })
        .collect();
    code.push((EN.how_repository.to_string(), Kind::Mono));
    cy += item(ui, (x, cy, w), EN.how_field_code, |ui, at| {
        entries(ui, at, code.iter().map(|(t, k)| (t.as_str(), *k)))
    });
    cy += field(ui, (x, cy, w), EN.how_field_secret, b.secret);
    cy += field(ui, (x, cy, w), EN.how_field_limits, b.limits);
    cy += field(ui, (x, cy, w), EN.how_field_own_calls, b.own_calls);
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
fn cross_check<A: Copy + PartialEq>(ui: &mut Ui<A>, at: (f32, f32, f32), cc: &CrossCheck) -> f32 {
    item(ui, at, EN.how_field_crosscheck, |ui, (x, y, w)| match cc {
        CrossCheck::Not(why) => line(ui, (x, y, w), why, Kind::Warn),
        CrossCheck::Area(a) => area(ui, (x, y, w), a),
    })
}

/// An area's parts, a `GAP_SMALL` apart.
fn area<A: Copy + PartialEq>(ui: &mut Ui<A>, (x, y, w): (f32, f32, f32), a: &Area) -> f32 {
    let run = fill(EN.how_cc_seed_cases, &[a.seed, &a.cases.to_string()]);
    let of = |lines: &'static [Line]| -> Vec<(&str, Kind)> {
        lines.iter().map(|l| (l.text, l.kind)).collect()
    };
    let parts: [(&str, Vec<(&str, Kind)>); 8] = [
        (EN.how_cc_area, vec![(a.area, Kind::Plain)]),
        (EN.how_cc_code, of(a.code)),
        (EN.how_cc_second, of(a.second)),
        (EN.how_cc_vectors, of(a.vectors)),
        (EN.how_cc_random, of(a.random)),
        (EN.how_cc_differences, of(a.differences)),
        (EN.how_cc_when, of(a.when)),
        (EN.how_cc_last_run, vec![(run.as_str(), Kind::Mono)]),
    ];
    let mut cy = y;
    for (i, (name, lines)) in parts.iter().enumerate() {
        if i > 0 {
            cy += GAP_SMALL;
        }
        cy += ui.wrap_as((x, cy, w), BODY_M.medium(), Role::OnSurfaceVariant, name);
        for &(text, kind) in lines {
            cy += line(ui, (x, cy, w), text, kind);
        }
    }
    cy - y
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
