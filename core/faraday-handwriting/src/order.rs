//! The reading order of the words on a page, from a small rule of its own.
//!
//! A line number goes with the word to its right. Words are put in columns
//! (boxes that overlap across the line) and a column is cut into pieces where
//! a clear gap stands between two words or where the numbering starts again.
//! A chain of boxes is one column only when each word overlaps the next one
//! down and the left edges stay near one straight line; otherwise it is cut
//! at the wide gaps between left edges, and a part that is still not a column
//! makes the whole page one list whose order is asked.
//!
//! * Every word numbered, the numbers read clearly: the pieces are taken in
//!   turn and a list is the longest run of them whose numbers are exactly
//!   1 to n, in the order of the numbers. The numbering restarts cut lists
//!   only here. A single column whose numbers do not run down it is flagged.
//! * Otherwise a single column is read top to bottom (each piece of it is a
//!   list), and several columns are one list whose two orders, down the
//!   columns and across the rows, are both returned: nothing is chosen. The
//!   numbers are judged on the order returned (the one they run along comes
//!   first, and `Group::rows_first` says which it is): each must equal its
//!   place plus one.
//!
//! Work is quadratic in the number of words, which `find_words` bounds.

use crate::find::Rect;

/// A number is the line number of the word to its right when the gap between
/// them is at most this many heights.
const NUMBER_REACH: f32 = 3.0;
/// Two boxes are on one line when their centres are closer than this share
/// of the lower height.
const SAME_ROW: f32 = 0.5;
/// A gap between two words of a column is clear when it is more than this
/// many line pitches.
const CLEAR_GAP_PITCHES: f32 = 2.0;
/// The largest line number read.
pub const MAX_NUMBER: u8 = 60;

/// A word box and the line number clearly read beside it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Item {
    pub rect: Rect,
    pub value: Option<u8>,
}

/// One list the layout found: words in the order read, and, when the order
/// is a question, the other order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Group {
    /// Indices of the items.
    pub order: Vec<usize>,
    /// The other order, if the layout does not decide between two.
    pub across: Option<Vec<usize>>,
    /// A number read beside a word is not its place in `order` plus one.
    pub numbers_disagree: bool,
    /// The list is one of several that only a vertical gap cut apart.
    pub cut_by_gap: bool,
    /// The sizes of the columns this list stands in, when it is several.
    pub sides: Vec<usize>,
    /// The sizes, top to bottom, of the parts of several columns that a blank
    /// band across all of them cuts apart (empty if there is no such band).
    pub bands: Vec<usize>,
    /// `order` goes row by row across the columns (the numbers run that way),
    /// not down them.
    pub rows_first: bool,
    /// The layout could not be told: boxes that are neither one column nor
    /// several; the page may hold one list or several.
    pub unclear: bool,
}

fn cy(r: &Rect) -> f32 {
    (r.y0 + r.y1) / 2.0
}

fn cx(r: &Rect) -> f32 {
    (r.x0 + r.x1) / 2.0
}

fn median(v: &mut [f32]) -> f32 {
    v.sort_by(f32::total_cmp);
    let n = v.len();
    if n == 0 {
        return 0.0;
    }
    (v.get((n - 1) / 2).copied().unwrap_or(0.0) + v.get(n / 2).copied().unwrap_or(0.0)) / 2.0
}

/// Two distances closer than this share of the number's height are alike.
const ALIKE: f32 = 0.25;

/// A word a number may go with, and how far: vertical distance between the
/// centres, then the gap along the line.
#[derive(Clone, Copy)]
struct Near {
    word: usize,
    dy: f32,
    gap: f32,
    /// The word's centre and height.
    wy: f32,
    wh: f32,
}

/// Which of two candidates is nearer the number: by the vertical distance
/// first, by the gap when that is alike.
fn nearer(a: &Near, b: &Near, tol: f32) -> core::cmp::Ordering {
    if (a.dy - b.dy).abs() > tol {
        a.dy.total_cmp(&b.dy)
    } else {
        a.gap.total_cmp(&b.gap)
    }
}

/// For each word, the number box (index) that goes with it: the one on its
/// left, on its line, within `NUMBER_REACH` heights, nearest by the vertical
/// distance of the centres and then by the gap. A number about as near two
/// words goes with neither (two words on one line, as far in height and in
/// gap; two words on other lines, as far in height). A number that loses its word to a nearer number
/// takes its second choice only if that word has no number and no other
/// number that lost wants it too. Also how many
/// numbers go with no word.
pub(crate) fn pair_numbers(words: &[Rect], numbers: &[Rect]) -> (Vec<Option<usize>>, usize) {
    // For each number its best and second candidates, none when two are alike.
    let mut choices: Vec<[Option<Near>; 2]> = Vec::with_capacity(numbers.len());
    let mut tols: Vec<f32> = Vec::with_capacity(numbers.len());
    for n in numbers {
        let tol = ALIKE * n.height();
        let mut best: Option<Near> = None;
        let mut second: Option<Near> = None;
        for (wi, w) in words.iter().enumerate() {
            let low = n.height().min(w.height());
            let across = n.y1.min(w.y1) - n.y0.max(w.y0);
            let dy = (cy(w) - cy(n)).abs();
            let gap = (w.x0 - n.x1).max(0.0);
            let reach = NUMBER_REACH * n.height().max(w.height());
            let usable = cx(w) > cx(n)
                && across >= SAME_ROW * low
                && dy.is_finite()
                && gap.is_finite()
                && gap <= reach;
            if !usable {
                continue;
            }
            let c = Near {
                word: wi,
                dy,
                gap,
                wy: cy(w),
                wh: w.height(),
            };
            match best {
                Some(b) if nearer(&c, &b, tol).is_lt() => {
                    second = best;
                    best = Some(c);
                }
                Some(_) => {
                    if second.is_none_or(|s| nearer(&c, &s, tol).is_lt()) {
                        second = Some(c);
                    }
                }
                None => best = Some(c),
            }
        }
        let alike = match (best, second) {
            // Two words on one line are told apart by the gap; two on other
            // lines, as near the number in height, by nothing.
            (Some(b), Some(s)) => {
                let one_line = (b.wy - s.wy).abs() < SAME_ROW * b.wh.min(s.wh);
                (b.dy - s.dy).abs() <= tol && (!one_line || (b.gap - s.gap).abs() <= tol)
            }
            _ => false,
        };
        choices.push(if alike { [None, None] } else { [best, second] });
        tols.push(tol);
    }
    let mut owner: Vec<Option<(usize, Near)>> = vec![None; words.len()];
    // Round one: each number claims its best word; the nearer keeps it.
    let mut lost: Vec<usize> = Vec::new();
    for ni in 0..numbers.len() {
        let Some(Some(c)) = choices.get(ni).map(|ch| ch.first().copied().flatten()) else {
            continue;
        };
        let tol = tols.get(ni).copied().unwrap_or(0.0);
        let Some(slot) = owner.get_mut(c.word) else {
            continue;
        };
        match *slot {
            None => *slot = Some((ni, c)),
            Some((other, o)) => {
                // The nearer number keeps the word; the other may try again.
                if nearer(&c, &o, tol.max(tols.get(other).copied().unwrap_or(0.0))).is_lt() {
                    *slot = Some((ni, c));
                    lost.push(other);
                } else {
                    lost.push(ni);
                }
            }
        }
    }
    // Round two: a number that lost takes its second choice only if that word
    // has no number; two that lost and want the same free word leave it to
    // neither.
    let mut wanted: Vec<Vec<(usize, Near)>> = vec![Vec::new(); words.len()];
    for ni in lost {
        let Some(Some(c)) = choices.get(ni).map(|ch| ch.get(1).copied().flatten()) else {
            continue;
        };
        if let Some(w) = wanted.get_mut(c.word) {
            w.push((ni, c));
        }
    }
    for (slot, want) in owner.iter_mut().zip(wanted) {
        if slot.is_none()
            && let [only] = want.as_slice()
        {
            *slot = Some(*only);
        }
    }
    let paired: Vec<Option<usize>> = owner.iter().map(|o| o.map(|(n, _)| n)).collect();
    let used = paired.iter().flatten().count();
    (paired, numbers.len().saturating_sub(used))
}

/// A column cut: item indices from top to bottom.
struct Piece {
    column: usize,
    items: Vec<usize>,
}

/// Each word of a column overlaps the next one down in x by at least this
/// share of the narrower.
const COLUMN_OVERLAP: f32 = 0.5;
/// The left edges of a column stand within this many heights of one straight
/// line (a slanted page's column is straight).
const EDGE_HEIGHTS: f32 = 2.0;
/// Two left edges more than this many heights apart are in two columns.
const COLUMN_GAP_HEIGHTS: f32 = 2.0;

fn by_y(items: &[Item], m: &mut [usize]) {
    m.sort_by(|&a, &b| {
        let ya = items.get(a).map_or(0.0, |i| cy(&i.rect));
        let yb = items.get(b).map_or(0.0, |i| cy(&i.rect));
        ya.total_cmp(&yb)
    });
}

/// True if `members` are one column: one word on each line, each word over
/// the next one down in x, and the left edges near one straight line.
fn is_column(items: &[Item], members: &[usize]) -> bool {
    let mut m = members.to_vec();
    by_y(items, &mut m);
    let rects: Vec<Rect> = m
        .iter()
        .filter_map(|&i| items.get(i).map(|it| it.rect))
        .collect();
    if rects.len() <= 1 {
        return true;
    }
    if rows_of(items, &m).iter().any(|r| r.len() > 1) {
        return false;
    }
    let linked = rects.windows(2).all(|p| {
        let over = p[0].x1.min(p[1].x1) - p[0].x0.max(p[1].x0);
        let narrow = p[0].width().min(p[1].width());
        over >= COLUMN_OVERLAP * narrow
    });
    if !linked {
        return false;
    }
    // Least-squares line of the left edge against the line's height.
    let n = rects.len() as f32;
    let my = rects.iter().map(cy).sum::<f32>() / n;
    let mx = rects.iter().map(|r| r.x0).sum::<f32>() / n;
    let var: f32 = rects.iter().map(|r| (cy(r) - my) * (cy(r) - my)).sum();
    let cov: f32 = rects.iter().map(|r| (cy(r) - my) * (r.x0 - mx)).sum();
    let slope = if var > 0.0 { cov / var } else { 0.0 };
    let mut hs: Vec<f32> = rects.iter().map(Rect::height).collect();
    let tol = EDGE_HEIGHTS * median(&mut hs);
    rects
        .iter()
        .all(|r| (r.x0 - (mx + slope * (cy(r) - my))).abs() <= tol)
}

/// A part of a chain cut at the wide gaps between left edges.
fn split_by_x(items: &[Item], members: &[usize]) -> Vec<Vec<usize>> {
    let x0 = |i: usize| items.get(i).map_or(0.0, |it| it.rect.x0);
    let mut hs: Vec<f32> = members
        .iter()
        .filter_map(|&i| items.get(i).map(|it| it.rect.height()))
        .collect();
    let tol = COLUMN_GAP_HEIGHTS * median(&mut hs);
    let mut by_x = members.to_vec();
    by_x.sort_by(|&a, &b| x0(a).total_cmp(&x0(b)));
    let mut out: Vec<Vec<usize>> = Vec::new();
    for i in by_x {
        match out.last_mut() {
            Some(cur) if cur.last().is_some_and(|&l| x0(i) - x0(l) <= tol) => cur.push(i),
            _ => out.push(vec![i]),
        }
    }
    out
}

/// Columns: items whose boxes overlap across the line, chained, left to
/// right, each column's items top to bottom. A chain that is not a column is
/// cut at wide gaps between left edges; `true` if a part is still not a
/// column, so that the layout is unclear.
fn columns(items: &[Item]) -> (Vec<Vec<usize>>, bool) {
    let mut idx: Vec<usize> = (0..items.len()).collect();
    idx.sort_by(|&a, &b| {
        let (ra, rb) = (items.get(a).map(|i| i.rect), items.get(b).map(|i| i.rect));
        match (ra, rb) {
            (Some(ra), Some(rb)) => ra.x0.total_cmp(&rb.x0),
            _ => core::cmp::Ordering::Equal,
        }
    });
    let mut chains: Vec<(f32, Vec<usize>)> = Vec::new();
    for i in idx {
        let Some(r) = items.get(i).map(|it| it.rect) else {
            continue;
        };
        match chains.last_mut() {
            Some((right, members)) if r.x0 < *right => {
                *right = right.max(r.x1);
                members.push(i);
            }
            _ => chains.push((r.x1, vec![i])),
        }
    }
    let mut unclear = false;
    let mut out: Vec<Vec<usize>> = Vec::new();
    for (_, chain) in chains {
        let parts = if is_column(items, &chain) {
            vec![chain]
        } else {
            split_by_x(items, &chain)
        };
        for part in parts {
            if is_column(items, &part) {
                out.push(part);
                continue;
            }
            for sub in split_crowded(items, part) {
                unclear |= !is_column(items, &sub);
                out.push(sub);
            }
        }
    }
    for m in &mut out {
        by_y(items, m);
    }
    (out, unclear)
}

/// A chain of overlapping boxes that holds several words on one line is
/// several columns whose words run into each other: it is cut again at the
/// widest gaps between the words' left edges, as many times as the most words
/// on one line say.
fn split_crowded(items: &[Item], members: Vec<usize>) -> Vec<Vec<usize>> {
    let per_row = rows_of(items, &members)
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(1);
    if per_row <= 1 {
        return vec![members];
    }
    let x0 = |i: usize| items.get(i).map_or(0.0, |it| it.rect.x0);
    let mut by_x = members;
    by_x.sort_by(|&a, &b| x0(a).total_cmp(&x0(b)));
    let mut gaps: Vec<(f32, usize)> = by_x
        .windows(2)
        .enumerate()
        .map(|(k, p)| (x0(p[1]) - x0(p[0]), k + 1))
        .collect();
    gaps.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut cuts: Vec<usize> = gaps.iter().take(per_row - 1).map(|g| g.1).collect();
    cuts.sort_unstable();
    let mut out = Vec::new();
    let mut from = 0;
    for c in cuts.into_iter().chain(core::iter::once(by_x.len())) {
        if let Some(part) = by_x.get(from..c) {
            out.push(part.to_vec());
        }
        from = c;
    }
    out
}

/// A column cut where the gap between two words is clear and, if
/// `cut_restarts`, where a line number 1 comes again.
fn pieces(items: &[Item], cols: &[Vec<usize>], cut_restarts: bool) -> Vec<Piece> {
    let mut out = Vec::new();
    for (column, col) in cols.iter().enumerate() {
        let rects: Vec<Rect> = col
            .iter()
            .filter_map(|&i| items.get(i).map(|it| it.rect))
            .collect();
        let mut pitches: Vec<f32> = rects.windows(2).map(|p| cy(&p[1]) - cy(&p[0])).collect();
        let pitch = if pitches.len() >= 2 {
            median(&mut pitches)
        } else {
            f32::INFINITY
        };
        let mut cur: Vec<usize> = Vec::new();
        let mut seen_one = false;
        let mut last: Option<Rect> = None;
        for (&i, r) in col.iter().zip(rects.iter()) {
            let gap_cut = last.is_some_and(|p| r.y0 - p.y1 > CLEAR_GAP_PITCHES * pitch);
            let value = items.get(i).and_then(|it| it.value);
            let restart = cut_restarts && value == Some(1) && seen_one;
            if (gap_cut || restart) && !cur.is_empty() {
                out.push(Piece {
                    column,
                    items: core::mem::take(&mut cur),
                });
                seen_one = false;
            }
            seen_one |= value == Some(1);
            cur.push(i);
            last = Some(*r);
        }
        if !cur.is_empty() {
            out.push(Piece { column, items: cur });
        }
    }
    out
}

/// The numbers of `members`, if they are exactly 1 to the count.
fn exactly_one_to_n(items: &[Item], members: &[usize]) -> bool {
    let mut mask = 0u64;
    for &i in members {
        let Some(v) = items.get(i).and_then(|it| it.value) else {
            return false;
        };
        if v == 0 || v > MAX_NUMBER || mask >> v & 1 == 1 {
            return false;
        }
        mask |= 1 << v;
    }
    let n = members.len();
    n > 0 && n <= usize::from(MAX_NUMBER) && mask == ((1u64 << (n + 1)) - 2)
}

/// Lists by the numbers: the longest run of pieces, from the first left,
/// whose numbers are exactly 1 to n, and again from the piece after it.
/// `None` if some word has no number or a run is not 1 to n. A run in one
/// column whose numbers do not run down it is flagged, never reordered
/// silently.
fn by_numbers(items: &[Item], ps: &[Piece]) -> Option<Vec<Group>> {
    let mut groups = Vec::new();
    let mut at = 0;
    while at < ps.len() {
        let mut best: Option<(usize, Vec<usize>)> = None;
        let mut members: Vec<usize> = Vec::new();
        for (k, p) in ps.iter().enumerate().skip(at) {
            members.extend_from_slice(&p.items);
            if exactly_one_to_n(items, &members) {
                best = Some((k + 1, members.clone()));
            }
        }
        let (next, by_position) = best?;
        let first_column = ps.get(at).map(|p| p.column);
        let one_column = ps
            .get(at..next)
            .is_some_and(|run| run.iter().all(|p| Some(p.column) == first_column));
        let mut order = by_position.clone();
        order.sort_by_key(|&i| items.get(i).and_then(|it| it.value));
        groups.push(Group {
            numbers_disagree: one_column && order != by_position,
            order,
            across: None,
            cut_by_gap: false,
            sides: Vec::new(),
            bands: Vec::new(),
            rows_first: false,
            unclear: false,
        });
        at = next;
    }
    Some(groups)
}

/// True if every number read along `order` is its place plus one.
fn numbers_agree(items: &[Item], order: &[usize]) -> bool {
    order
        .iter()
        .enumerate()
        .all(|(place, &i)| match items.get(i).and_then(|it| it.value) {
            Some(v) => usize::from(v) == place.saturating_add(1),
            None => true,
        })
}

/// Rows of `all` (boxes whose centres stand within `SAME_ROW` of a height),
/// top to bottom, each left to right.
fn rows_of(items: &[Item], all: &[usize]) -> Vec<Vec<usize>> {
    let mut idx: Vec<usize> = all.to_vec();
    let y = |i: usize| items.get(i).map_or(0.0, |it| cy(&it.rect));
    idx.sort_by(|&a, &b| y(a).total_cmp(&y(b)));
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut row_y = 0.0f32;
    for i in idx {
        let h = items.get(i).map_or(1.0, |it| it.rect.height());
        match rows.last_mut() {
            Some(row) if (y(i) - row_y).abs() < SAME_ROW * h => {
                let n = row.len() as f32;
                row_y = (row_y * n + y(i)) / (n + 1.0);
                row.push(i);
            }
            _ => {
                row_y = y(i);
                rows.push(vec![i]);
            }
        }
    }
    for row in &mut rows {
        row.sort_by(|&a, &b| {
            let xa = items.get(a).map_or(0.0, |it| it.rect.x0);
            let xb = items.get(b).map_or(0.0, |it| it.rect.x0);
            xa.total_cmp(&xb)
        });
    }
    rows
}

/// Rows of all the items, top to bottom, each left to right.
fn across_order(items: &[Item], all: &[usize]) -> Vec<usize> {
    rows_of(items, all).into_iter().flatten().collect()
}

/// How many words stand in each part, top to bottom, of `all` that a blank
/// band cuts apart (a gap of more than `CLEAR_GAP_PITCHES` line pitches across
/// every box of `all`); empty if there is no such band.
fn bands_of(items: &[Item], all: &[usize]) -> Vec<usize> {
    let rows = rows_of(items, all);
    let mut pitches: Vec<f32> = rows
        .windows(2)
        .filter_map(|p| {
            let a = items.get(*p[0].first()?)?;
            let b = items.get(*p[1].first()?)?;
            Some(cy(&b.rect) - cy(&a.rect))
        })
        .collect();
    if pitches.len() < 2 {
        return Vec::new();
    }
    let pitch = median(&mut pitches);
    let mut idx: Vec<usize> = all.to_vec();
    idx.sort_by(|&a, &b| {
        let ya = items.get(a).map_or(0.0, |i| i.rect.y0);
        let yb = items.get(b).map_or(0.0, |i| i.rect.y0);
        ya.total_cmp(&yb)
    });
    let mut out: Vec<usize> = Vec::new();
    let mut reach = f32::NEG_INFINITY;
    for i in idx {
        let Some(r) = items.get(i).map(|it| it.rect) else {
            continue;
        };
        match out.last_mut() {
            Some(n) if r.y0 - reach <= CLEAR_GAP_PITCHES * pitch => {
                *n += 1;
                reach = reach.max(r.y1);
            }
            _ => {
                out.push(1);
                reach = r.y1;
            }
        }
    }
    if out.len() > 1 { out } else { Vec::new() }
}

/// The lists on a page of word boxes, each with its order (see the module).
pub(crate) fn arrange(items: &[Item]) -> Vec<Group> {
    if items.is_empty() {
        return Vec::new();
    }
    let (cols, unclear) = columns(items);
    if unclear {
        let down: Vec<usize> = cols.iter().flatten().copied().collect();
        return vec![Group {
            numbers_disagree: !numbers_agree(items, &down),
            across: Some(across_order(items, &down)),
            order: down,
            cut_by_gap: false,
            sides: Vec::new(),
            bands: Vec::new(),
            rows_first: false,
            unclear: true,
        }];
    }
    if let Some(g) = by_numbers(items, &pieces(items, &cols, true)) {
        return g;
    }
    let ps = pieces(items, &cols, false);
    if cols.len() == 1 {
        let cut = ps.len() > 1;
        return ps
            .into_iter()
            .map(|p| Group {
                numbers_disagree: !numbers_agree(items, &p.items),
                order: p.items,
                across: None,
                cut_by_gap: cut,
                sides: Vec::new(),
                bands: Vec::new(),
                rows_first: false,
                unclear: false,
            })
            .collect();
    }
    let mut down: Vec<usize> = cols.iter().flatten().copied().collect();
    let mut across = across_order(items, &down);
    // The order the numbers run along comes first.
    let rows_first = !numbers_agree(items, &down) && numbers_agree(items, &across);
    if rows_first {
        core::mem::swap(&mut down, &mut across);
    }
    let numbers_disagree = !numbers_agree(items, &down);
    let across = (across != down).then_some(across);
    vec![Group {
        order: down,
        across,
        numbers_disagree,
        cut_by_gap: false,
        sides: cols.iter().map(Vec::len).collect(),
        bands: bands_of(items, &cols.iter().flatten().copied().collect::<Vec<_>>()),
        rows_first,
        unclear: false,
    }]
}
