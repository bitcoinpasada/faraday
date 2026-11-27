//! Image clean-up: a grey crop of one handwritten line to the ink map the word
//! network reads, and the faint ink left out of it.
//!
//! The steps follow Haller's `preprocess.py` (see the README): reduce a tall
//! crop by a block mean, take the ink as the darkness against the local paper
//! brightness, keep the pixels above a threshold taken from the darkest
//! strokes, remove long horizontal runs (ruled and printed lines), find the
//! rows and columns of the word, and scale that box to 32 rows with a
//! triangle filter. No libm function is used (floor, round and powers are
//! done in integers or by casts), so the build links no libm symbol.

use core::fmt;

use zeroize::Zeroizing;

use crate::nn::Buf;
use crate::{Error, INK_HEIGHT};

/// A crop taller than this is reduced first: rows.
const WORKING_HEIGHT: usize = 64;
/// A crop wider than this is reduced as well, so that the work is bounded
/// whatever the size of the crop: columns.
const WORKING_WIDTH: usize = 4096;
/// The widest ink map made: columns.
pub const MAX_INK_WIDTH: usize = 256;
/// The narrowest ink map made: columns.
const MIN_INK_WIDTH: usize = 8;
/// Percentile of the ink taken as the strength of the strokes.
const STRONGEST_PERCENTILE: f64 = 0.995;
/// Darkest ink (share of the paper brightness) that counts as a word at all.
const MIN_STRONGEST: f32 = 0.08;
/// Lowest ink threshold.
const MIN_THRESHOLD: f32 = 0.06;
/// Share of the strongest ink that is the threshold.
const THRESHOLD_SHARE: f32 = 0.35;
/// Share of the strongest ink that `unread_ink` takes as faint.
const FAINT_SHARE: f32 = 0.14;
/// Lowest faint threshold of `unread_ink`.
const MIN_FAINT_THRESHOLD: f32 = 0.03;
/// How near a removed line is to the word's rows to count as on them: rows.
const LINE_REACH: usize = 2;

/// The ink map of a word and where the word stands in the crop.
pub struct Cleaned {
    ink: Buf<u8>,
    width: usize,
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}

impl Cleaned {
    /// The map: `INK_HEIGHT` rows by `width()` columns, row-major, 0 paper
    /// and 255 ink.
    pub fn ink(&self) -> &[u8] {
        &self.ink
    }

    /// Columns of the map: 8 to 256.
    pub fn width(&self) -> usize {
        self.width
    }

    /// The word's box in the crop: left, top, right, bottom (the last two
    /// exclusive), clamped to the crop.
    pub fn bounds(&self) -> (usize, usize, usize, usize) {
        (self.left, self.top, self.right, self.bottom)
    }
}

impl fmt::Debug for Cleaned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Cleaned(..)")
    }
}

/// A crop reduced to the working size, with its ink.
pub(crate) struct Prepared {
    crop_width: usize,
    crop_height: usize,
    factor: usize,
    w: usize,
    h: usize,
    ink: Buf<f32>,
    strongest: f32,
}

fn ceil_div(a: usize, b: usize) -> usize {
    a / b + usize::from(!a.is_multiple_of(b))
}

/// Block mean of `factor` by `factor` pixels, the blocks at the right and
/// bottom edge holding what is left, rounded to the nearest integer.
fn reduce(gray: &[u8], width: usize, height: usize, factor: usize) -> (Buf<f32>, usize, usize) {
    let (w, h) = (ceil_div(width, factor), ceil_div(height, factor));
    let mut out: Buf<f32> = Zeroizing::new(vec![0.0; w * h]);
    for by in 0..h {
        let (y0, y1) = (by * factor, ((by + 1) * factor).min(height));
        for bx in 0..w {
            let (x0, x1) = (bx * factor, ((bx + 1) * factor).min(width));
            let mut sum = 0u64;
            for y in y0..y1 {
                if let Some(row) = gray.get(y * width + x0..y * width + x1) {
                    sum += row.iter().map(|&v| u64::from(v)).sum::<u64>();
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as u64;
            if let Some(o) = out.get_mut(by * w + bx) {
                *o = ((sum + n / 2) / n.max(1)) as f32;
            }
        }
    }
    (out, w, h)
}

/// Mean over a (2r+1) by (2r+1) window, the edges replicated, by an integral
/// image of the padded map in f64, as the reference does.
fn box_mean(src: &[f32], h: usize, w: usize, radius: usize) -> Buf<f32> {
    let k = 2 * radius + 1;
    let (ph, pw) = (h + 2 * radius + 2, w + 2 * radius + 2);
    let mut p: Zeroizing<Vec<f64>> = Zeroizing::new(vec![0.0; ph * pw]);
    for y in 0..ph {
        let sy = y.saturating_sub(radius + 1).min(h - 1);
        for x in 0..pw {
            let sx = x.saturating_sub(radius + 1).min(w - 1);
            if let (Some(d), Some(&s)) = (p.get_mut(y * pw + x), src.get(sy * w + sx)) {
                *d = f64::from(s);
            }
        }
    }
    for y in 1..ph {
        for x in 0..pw {
            let up = p[(y - 1) * pw + x];
            p[y * pw + x] += up;
        }
    }
    for y in 0..ph {
        for x in 1..pw {
            let left = p[y * pw + x - 1];
            p[y * pw + x] += left;
        }
    }
    let area = (k * k) as f64;
    let mut out: Buf<f32> = Zeroizing::new(vec![0.0; h * w]);
    for y in 0..h {
        for x in 0..w {
            let total =
                p[(y + k) * pw + x + k] - p[y * pw + x + k] - p[(y + k) * pw + x] + p[y * pw + x];
            out[y * w + x] = (total / area) as f32;
        }
    }
    out
}

/// Linear-interpolated percentile `q` (0 to 1) of `values`, which must not
/// be empty.
fn percentile(values: &[f32], q: f64) -> f32 {
    let mut sorted: Buf<f32> = Zeroizing::new(values.to_vec());
    sorted.sort_unstable_by(f32::total_cmp);
    let last = sorted.len().saturating_sub(1);
    let pos = last as f64 * q;
    let lo = (pos as usize).min(last);
    let hi = (lo + 1).min(last);
    let frac = pos - lo as f64;
    let (a, b) = (
        f64::from(sorted.get(lo).copied().unwrap_or(0.0)),
        f64::from(sorted.get(hi).copied().unwrap_or(0.0)),
    );
    (a + (b - a) * frac) as f32
}

/// Reduces the crop and takes its ink: the darkness against the local paper
/// brightness, 0 to 1. `None` if the crop is empty or holds no ink worth
/// reading.
pub(crate) fn prepare(width: usize, height: usize, gray: &[u8]) -> Result<Option<Prepared>, Error> {
    if width.checked_mul(height) != Some(gray.len()) {
        return Err(Error::Size);
    }
    if gray.is_empty() {
        return Ok(None);
    }
    let factor = ceil_div(height, WORKING_HEIGHT)
        .max(ceil_div(width, WORKING_WIDTH))
        .max(1);
    let (a, w, h) = reduce(gray, width, height, factor);
    let background = box_mean(&a, h, w, (h / 4).max(4));
    let mut ink: Buf<f32> = Zeroizing::new(vec![0.0; w * h]);
    for ((o, &v), &bg) in ink.iter_mut().zip(a.iter()).zip(background.iter()) {
        let scale = (0.5 * bg).max(1.0);
        *o = ((bg - v) / scale).clamp(0.0, 1.0);
    }
    let strongest = percentile(&ink, STRONGEST_PERCENTILE);
    if strongest < MIN_STRONGEST {
        return Ok(None);
    }
    Ok(Some(Prepared {
        crop_width: width,
        crop_height: height,
        factor,
        w,
        h,
        ink,
        strongest,
    }))
}

/// The cells that lie in a horizontal run of `min_length` or more.
fn long_runs(mask: &[bool], w: usize, min_length: usize) -> Buf<bool> {
    let mut out: Buf<bool> = Zeroizing::new(vec![false; mask.len()]);
    for (row, dst) in mask.chunks(w).zip(out.chunks_mut(w)) {
        let mut x = 0;
        while x < w {
            if row.get(x).copied() != Some(true) {
                x += 1;
                continue;
            }
            let start = x;
            while row.get(x).copied() == Some(true) {
                x += 1;
            }
            if x - start >= min_length {
                for d in dst.iter_mut().take(x).skip(start) {
                    *d = true;
                }
            }
        }
    }
    out
}

/// Long runs plus the row above and below each cell of them.
fn long_runs_widened(mask: &[bool], h: usize, w: usize, min_length: usize) -> Buf<bool> {
    let runs = long_runs(mask, w, min_length);
    let mut out: Buf<bool> = Zeroizing::new(runs.to_vec());
    for y in 0..h {
        for x in 0..w {
            let at = |yy: usize| runs.get(yy * w + x).copied().unwrap_or(false);
            let on = at(y) || (y > 0 && at(y - 1)) || (y + 1 < h && at(y + 1));
            if let Some(o) = out.get_mut(y * w + x) {
                *o = on;
            }
        }
    }
    out
}

/// Rows of the main line of text: the heaviest block of consecutive inked
/// rows, extended while ink continues.
fn text_rows(mask: &[bool], h: usize, w: usize) -> Option<(usize, usize)> {
    let profile: Vec<u32> = mask
        .chunks(w)
        .map(|r| r.iter().filter(|&&m| m).count() as u32)
        .collect();
    let peak = profile.iter().copied().max().unwrap_or(0);
    if peak == 0 {
        return None;
    }
    let limit = (0.08 * f64::from(peak)).max(2.0);
    let mut blocks: Vec<(usize, usize)> = Vec::new();
    let mut start: Option<usize> = None;
    for y in 0..=h {
        let on = profile.get(y).is_some_and(|&p| f64::from(p) > limit);
        if on && start.is_none() {
            start = Some(y);
        } else if !on && let Some(s) = start.take() {
            match blocks.last_mut() {
                Some(last) if s - last.1 <= 2 => last.1 = y,
                _ => blocks.push((s, y)),
            }
        }
    }
    let weight = |b: &(usize, usize)| -> u64 {
        profile
            .get(b.0..b.1)
            .map_or(0, |p| p.iter().map(|&v| u64::from(v)).sum())
    };
    let mut best: Option<(usize, usize)> = None;
    for b in &blocks {
        if best.is_none_or(|c| weight(b) > weight(&c)) {
            best = Some(*b);
        }
    }
    let (mut y0, mut y1) = best?;
    while y0 > 0 && profile.get(y0 - 1).is_some_and(|&p| p > 0) {
        y0 -= 1;
    }
    while profile.get(y1).is_some_and(|&p| p > 0) {
        y1 += 1;
    }
    Some((y0, y1))
}

/// First and last inked column of the word in the rows `y0..y1`: groups of
/// columns separated by gaps wider than `height`, groups with under 3% of the
/// ink dropped.
fn word_columns(
    mask: &[bool],
    w: usize,
    y0: usize,
    y1: usize,
    height: usize,
) -> Option<(usize, usize)> {
    let mut per_column = vec![0u64; w];
    for row in mask.chunks(w).take(y1).skip(y0) {
        for (c, &m) in per_column.iter_mut().zip(row) {
            *c += u64::from(m);
        }
    }
    let total: u64 = per_column.iter().sum();
    // (first column, last column, ink) of each group.
    let mut groups: Vec<(usize, usize, u64)> = Vec::new();
    for (x, &c) in per_column.iter().enumerate() {
        if c == 0 {
            continue;
        }
        match groups.last_mut() {
            Some(g) if x - g.1 <= height => {
                g.1 = x;
                g.2 += c;
            }
            _ => groups.push((x, x, c)),
        }
    }
    let kept: Vec<&(usize, usize, u64)> = groups
        .iter()
        .filter(|g| g.2 as f64 >= 0.03 * total as f64)
        .collect();
    Some((kept.first()?.0, kept.last()?.1))
}

/// Triangle-filter coefficients of one output index, as Pillow's bilinear
/// resize: the support widens with the scale when shrinking.
fn coefficients(input: usize, output: usize, index: usize) -> (usize, Vec<f64>) {
    let scale = input as f64 / output as f64;
    let filter_scale = if scale > 1.0 { scale } else { 1.0 };
    let center = (index as f64 + 0.5) * scale;
    let lo = ((center - filter_scale + 0.5) as i64).max(0) as usize;
    let hi = (((center + filter_scale + 0.5) as i64).max(0) as usize).min(input);
    let mut weights: Vec<f64> = (lo..hi)
        .map(|x| {
            let v = (x as f64 - center + 0.5) / filter_scale;
            let v = if v < 0.0 { -v } else { v };
            if v < 1.0 { 1.0 - v } else { 0.0 }
        })
        .collect();
    let sum: f64 = weights.iter().sum();
    if sum != 0.0 {
        for w in &mut weights {
            *w /= sum;
        }
    }
    (lo, weights)
}

fn to_byte(v: f64) -> u8 {
    let r = v + 0.5;
    if r <= 0.0 {
        0
    } else if r >= 255.0 {
        255
    } else {
        r as u8
    }
}

/// `src` (`sw` by `sh`) scaled to `dw` by `dh`: a horizontal pass, then a
/// vertical one, each rounded to bytes.
fn resize(src: &[u8], sw: usize, sh: usize, dw: usize, dh: usize) -> Buf<u8> {
    let mut mid: Buf<u8> = Zeroizing::new(vec![0; dw * sh]);
    for x in 0..dw {
        let (lo, weights) = coefficients(sw, dw, x);
        for y in 0..sh {
            let mut acc = 0.0;
            for (i, wt) in weights.iter().enumerate() {
                acc += wt * f64::from(src.get(y * sw + lo + i).copied().unwrap_or(0));
            }
            if let Some(d) = mid.get_mut(y * dw + x) {
                *d = to_byte(acc);
            }
        }
    }
    let mut out: Buf<u8> = Zeroizing::new(vec![0; dw * dh]);
    for y in 0..dh {
        let (lo, weights) = coefficients(sh, dh, y);
        for x in 0..dw {
            let mut acc = 0.0;
            for (i, wt) in weights.iter().enumerate() {
                acc += wt * f64::from(mid.get((lo + i) * dw + x).copied().unwrap_or(0));
            }
            if let Some(d) = out.get_mut(y * dw + x) {
                *d = to_byte(acc);
            }
        }
    }
    out
}

/// `round(a / b)` with ties to even, in integers.
fn div_round_even(a: usize, b: usize) -> usize {
    let (q, r) = (a / b, a % b);
    if 2 * r > b || (2 * r == b && q % 2 == 1) {
        q + 1
    } else {
        q
    }
}

fn threshold_of(strongest: f32) -> f32 {
    MIN_THRESHOLD.max(THRESHOLD_SHARE * strongest)
}

/// The mask `locate` reads the word from: ink over the threshold, with the
/// ruled and printed lines (found on a more permissive mask where even a faint
/// line stays continuous, widened by one pixel) taken off.
fn read_mask(p: &Prepared) -> Buf<bool> {
    let (w, h) = (p.w, p.h);
    let threshold = threshold_of(p.strongest);
    let mut mask: Buf<bool> = Zeroizing::new(p.ink.iter().map(|&v| v > threshold).collect());
    let faint: Buf<bool> = Zeroizing::new(p.ink.iter().map(|&v| v > 0.5 * threshold).collect());
    let lines = long_runs_widened(&faint, h, w, (w / 4).max(24));
    for (m, &l) in mask.iter_mut().zip(lines.iter()) {
        *m = *m && !l;
    }
    mask
}

/// The word's ink map and box in the crop.
pub(crate) fn locate(p: &Prepared) -> Option<Cleaned> {
    let (w, h) = (p.w, p.h);
    let mask = read_mask(p);
    let (y0, y1) = text_rows(&mask, h, w)?;
    let height = y1 - y0;
    let (c0, c1) = word_columns(&mask, w, y0, y1, height)?;
    let margin_y = (0.15 * height as f64) as usize;
    let margin_x = (0.2 * height as f64) as usize;
    let y0 = y0.saturating_sub(margin_y);
    let y1 = (y1 + margin_y + 1).min(h);
    let x0 = c0.saturating_sub(margin_x);
    let x1 = (c1 + margin_x + 1).min(w);
    let (bw, bh) = (x1 - x0, y1 - y0);
    let mut word: Buf<u8> = Zeroizing::new(Vec::with_capacity(bw * bh));
    for y in y0..y1 {
        for x in x0..x1 {
            let i = y * w + x;
            let keep = mask.get(i).copied().unwrap_or(false);
            let v = if keep {
                p.ink.get(i).copied().unwrap_or(0.0) * 255.0
            } else {
                0.0
            };
            word.push(v as u8);
        }
    }
    let width = div_round_even(bw * INK_HEIGHT, bh).clamp(MIN_INK_WIDTH, MAX_INK_WIDTH);
    let ink = resize(&word, bw, bh, width, INK_HEIGHT);
    let f = p.factor;
    Some(Cleaned {
        ink,
        width,
        left: (x0 * f).min(p.crop_width),
        top: (y0 * f).min(p.crop_height),
        right: (x1 * f).min(p.crop_width),
        bottom: (y1 * f).min(p.crop_height),
    })
}

/// Width, in heights of the text, of the faint ink left out of the word:
/// strokes on the rows of the word, beyond its first or last column, lighter
/// than its threshold but darker than a share of the darkest strokes once
/// averaged over 3 by 3 pixels.
///
/// `None` if it cannot be told: the stricter mask finds no rows or columns,
/// the rows `locate` reads are other ones, or a removed line is on the rows of the word or within two of them, where it
/// may have erased strokes or cut the word in two.
pub(crate) fn unread(p: &Prepared) -> Option<f32> {
    let (w, h) = (p.w, p.h);
    let threshold = threshold_of(p.strongest);
    let faint_threshold = MIN_FAINT_THRESHOLD.max(FAINT_SHARE * p.strongest);
    let near = box_mean(&p.ink, h, w, 1);
    let mut faint: Buf<bool> = Zeroizing::new(
        near.iter()
            .zip(p.ink.iter())
            .map(|(&m, &v)| m > faint_threshold && v > 0.7 * faint_threshold)
            .collect(),
    );
    let lines: Buf<bool> = Zeroizing::new(
        faint
            .iter()
            .zip(p.ink.iter())
            .map(|(&f, &v)| f || v > 0.5 * threshold)
            .collect(),
    );
    let lines = long_runs_widened(&lines, h, w, (w / 4).max(24));
    let mask: Buf<bool> = Zeroizing::new(
        p.ink
            .iter()
            .zip(lines.iter())
            .map(|(&v, &l)| v > threshold && !l)
            .collect(),
    );
    for (f, &l) in faint.iter_mut().zip(lines.iter()) {
        *f = *f && !l;
    }
    let (y0, y1) = text_rows(&mask, h, w)?;
    // The rows `locate` reads must be these: if they differ (a pale line
    // splits the word there), the faint ink of another part is not the word's.
    if text_rows(&read_mask(p), h, w) != Some((y0, y1)) {
        return None;
    }
    // A line through a word splits it, and the rows are then one part's: a
    // line beside the rows counts as one on them.
    if lines
        .chunks(w)
        .take(y1.saturating_add(LINE_REACH))
        .skip(y0.saturating_sub(LINE_REACH))
        .any(|r| r.iter().any(|&l| l))
    {
        return None;
    }
    let height = y1 - y0;
    let (c0, c1) = word_columns(&mask, w, y0, y1, height)?;
    let (p0, p1) = word_columns(&faint, w, y0, y1, height)?;
    let left = c0 as i64 - p0 as i64;
    let right = p1 as i64 - c1 as i64;
    Some(left.max(right).max(0) as f32 / height.max(1) as f32)
}

/// Costs a few passes over the crop (reduced first to at most 64 rows by 4,096
/// columns) and one resize to the ink map (not timed on the Pi).
///
/// Cleans a grey crop (`width` by `height`, row-major, 0 black to 255 white)
/// of one handwritten line into the word's ink map: at most 256 columns by
/// 32 rows, 0 paper and 255 ink, and the word's box in the crop. `None` if
/// the crop is empty or holds no word. `Error::Size` if the bytes are not
/// `width * height`.
///
/// The work is bounded: a crop larger than 64 rows or 4,096 columns is first
/// reduced by a block mean.
pub fn clean_crop(width: usize, height: usize, gray: &[u8]) -> Result<Option<Cleaned>, Error> {
    Ok(prepare(width, height, gray)?.and_then(|p| locate(&p)))
}

/// Costs the passes of `clean_crop` and one more of the stricter mask, on the
/// crop reduced first to at most 64 rows by 4,096 columns.
///
/// Width, in heights of the text, of the faint ink of the crop that
/// `clean_crop` leaves out of the word (faded or broken letters beside it);
/// `None` if it cannot be told: no word is found, or lines were taken off the
/// word's own rows or beside them.
pub fn unread_ink(width: usize, height: usize, gray: &[u8]) -> Result<Option<f32>, Error> {
    Ok(prepare(width, height, gray)?.and_then(|p| unread(&p)))
}
