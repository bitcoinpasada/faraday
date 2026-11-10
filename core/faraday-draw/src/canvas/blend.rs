//! A cross-fade's blend of two frames (`docs/MOTION.md` §3.5), in bands
//! of rows.
//!
//! [`Canvas::blend_noting_banded`] and [`Canvas::blend_within_banded`]
//! split the rows they blend into bands, each a `&mut [u8]` of the
//! canvas's rows of its own with the same rows of the two frames, and
//! hand them to a caller that blends them side by side (threads, or one
//! after the other), as a recorded frame is painted in bands
//! (`docs/FRAMES.md` §4). A band reads the two frames and writes its own
//! rows and the spans of its own rows, nothing else, so it is `Send`.
//! Each pixel is blended from the same pixel of the two frames alone, so
//! the bytes are the same for any count of bands.

use alloc::vec;
use alloc::vec::Vec;
use core::ops::Range;

use faraday_crypto::work::Work;

use super::Canvas;
use crate::geom::Rect;

/// One band of rows of a fade's blend: the canvas's rows it writes, the
/// same rows of the frame faded from and the frame faded to, and what it
/// blends there.
pub struct BlendBand<'a> {
    now: &'a mut [u8],
    was: &'a [u8],
    will: &'a [u8],
    /// The canvas row of the band's first row.
    top: usize,
    /// A row's bytes.
    stride: usize,
    /// Of 256, how far from `was` to `will`.
    t: u16,
    what: What<'a>,
}

/// What a band blends.
enum What<'a> {
    /// Every pixel; for each of its rows, the span inside a ring `ring`
    /// pixels wide round a canvas of `rows` rows where the two frames
    /// differ.
    Noting {
        ring: usize,
        rows: usize,
        spans: &'a mut [Option<(usize, usize)>],
    },
    /// The pixels of these rectangles, each inside the canvas, alone.
    Within(&'a [Rect]),
}

/// Blends the bands one after the other, on this thread.
fn each(bands: &mut [BlendBand<'_>]) {
    for band in bands {
        band.blend();
    }
}

/// One band of [`split`]: its first row, its canvas rows, and the same
/// rows of the two frames.
type Part<'a> = (usize, &'a mut [u8], &'a [u8], &'a [u8]);

/// `rows` of the canvas's pixels and of the two frames, split into `n`
/// bands of about as many rows each, as (first row, canvas rows, rows of
/// `was`, rows of `will`).
fn split<'a>(
    data: &'a mut [u8],
    was: &'a [u8],
    will: &'a [u8],
    stride: usize,
    rows: Range<usize>,
    n: usize,
) -> Vec<Part<'a>> {
    let count = rows.len();
    let n = n.clamp(1, count.max(1));
    let mut rest = &mut data[rows.start * stride..rows.end * stride];
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let a = rows.start + i * count / n;
        let b = rows.start + (i + 1) * count / n;
        let (mine, tail) = core::mem::take(&mut rest).split_at_mut((b - a) * stride);
        rest = tail;
        let at = a * stride..b * stride;
        out.push((a, mine, &was[at.clone()], &will[at]));
    }
    out
}

impl Canvas {
    /// Shows `t` (0 to 1) of the way from `before` to `after`, two
    /// [`Canvas::snapshot`]s of this canvas, without drawing either
    /// again: a step of a cross-fade whose new frame is kept aside
    /// ([`Canvas::swap_pixels`]). Every pixel is written; snapshots of
    /// another size leave the canvas as it is.
    pub fn blend_between(&mut self, before: &[u8], after: &[u8], t: f32) {
        let mut bands = Vec::new();
        self.blend_noting(before, after, t, 0, &mut bands);
    }

    /// Costs a pass over the whole canvas. [`Canvas::blend_between`],
    /// which also sets `bands` to rectangles that hold every pixel where
    /// `before` and `after` differ, but for a ring `inset` pixels wide
    /// round the canvas's edge, which whoever holds the canvas draws over
    /// every frame and is never noted: for the fade's later steps to blend
    /// those alone ([`Canvas::blend_within`]). Differences are found 16
    /// pixels at a time from the ring's inner edge, so a band may hold a
    /// few pixels that agree; rows next to each other whose differences
    /// span about the same columns share a band.
    pub fn blend_noting(
        &mut self,
        before: &[u8],
        after: &[u8],
        t: f32,
        inset: i32,
        bands: &mut Vec<Rect>,
    ) {
        self.blend_noting_banded(before, after, (t, inset), bands, 1, &mut each);
    }

    /// Costs a pass over the whole canvas, in `n` bands of rows.
    /// [`Canvas::blend_noting`] with `t` and `inset` as `how`: `run` is
    /// given the bands, each ready to [`BlendBand::blend`], and must blend
    /// them all (on any threads, in any order) before it returns.
    pub fn blend_noting_banded(
        &mut self,
        before: &[u8],
        after: &[u8],
        how: (f32, i32),
        bands: &mut Vec<Rect>,
        n: usize,
        run: &mut dyn FnMut(&mut [BlendBand<'_>]),
    ) {
        let (t, inset) = how;
        self.sync();
        (self.counter)(Work::FullCanvas);
        bands.clear();
        let len = self.visible().len();
        if before.len() != len || after.len() != len {
            return;
        }
        // Each channel of a premultiplied pixel on its own, in 16 bits:
        // 255 x 256 fits, and a loop over bytes is one the compiler runs
        // many lanes at a time. A frosted page under a sheet differs
        // from the page before it everywhere, so most pixels of such a
        // fade are blended.
        let t = (t.clamp(0.0, 1.0) * 256.0) as u16;
        self.damaged(self.bounds());
        let (w, h) = (self.pixmap.width() as usize, self.rows as usize);
        let ring = usize::try_from(inset.max(0))
            .unwrap_or(0)
            .min(w / 2)
            .min(h / 2);
        let stride = w * 4;
        let mut spans = vec![None; h];
        {
            let mut left = spans.as_mut_slice();
            let data = self.pixmap.data_mut();
            let mut parts: Vec<BlendBand<'_>> = split(data, before, after, stride, 0..h, n)
                .into_iter()
                .map(|(top, now, was, will)| {
                    let (mine, tail) = core::mem::take(&mut left).split_at_mut(now.len() / stride);
                    left = tail;
                    BlendBand {
                        now,
                        was,
                        will,
                        top,
                        stride,
                        t,
                        what: What::Noting {
                            ring,
                            rows: h,
                            spans: mine,
                        },
                    }
                })
                .collect();
            run(&mut parts);
        }
        let mut gathered = Bands::default();
        for (y, span) in spans.into_iter().enumerate() {
            gathered.row(y, span);
        }
        gathered.into_rects(bands);
    }

    /// Costs a pass over the pixels of `bands`. The next step of a fade
    /// [`Canvas::blend_noting`] began: `t` of the way from `before` to
    /// `after` inside `bands`, the rectangles that call noted, the rest
    /// left as it is, where the two agree and the canvas already shows
    /// them (or the ring it left out, which is drawn over). Snapshots of
    /// another size leave the canvas as it is.
    pub fn blend_within(&mut self, before: &[u8], after: &[u8], t: f32, bands: &[Rect]) {
        self.blend_within_banded(before, after, t, bands, 1, &mut each);
    }

    /// Costs a pass over the pixels of `bands`, in `n` bands of the rows
    /// they span. [`Canvas::blend_within`]: `run` is given the bands of
    /// rows, each ready to [`BlendBand::blend`], and must blend them all
    /// (on any threads, in any order) before it returns.
    pub fn blend_within_banded(
        &mut self,
        before: &[u8],
        after: &[u8],
        t: f32,
        bands: &[Rect],
        n: usize,
        run: &mut dyn FnMut(&mut [BlendBand<'_>]),
    ) {
        self.sync();
        (self.counter)(Work::FullCanvas);
        let len = self.visible().len();
        if before.len() != len || after.len() != len {
            return;
        }
        let t = (t.clamp(0.0, 1.0) * 256.0) as u16;
        let bounds = self.bounds();
        let mut rows: Option<(i32, i32)> = None;
        for band in bands {
            let b = band.intersect(&bounds);
            if b.is_empty() {
                continue;
            }
            self.damaged(b);
            rows = Some(rows.map_or((b.y, b.bottom()), |(a, z)| (a.min(b.y), z.max(b.bottom()))));
        }
        let Some((a, z)) = rows else {
            return;
        };
        let stride = self.pixmap.width() as usize * 4;
        let data = self.pixmap.data_mut();
        let rows = a as usize..z as usize;
        let mut parts: Vec<BlendBand<'_>> = split(data, before, after, stride, rows, n)
            .into_iter()
            .map(|(top, now, was, will)| BlendBand {
                now,
                was,
                will,
                top,
                stride,
                t,
                what: What::Within(bands),
            })
            .collect();
        run(&mut parts);
    }
}

impl BlendBand<'_> {
    /// Costs a pass over the band's pixels to blend. Blends the band's
    /// rows.
    pub fn blend(&mut self) {
        let (stride, t, top) = (self.stride, self.t, self.top);
        let rows = self.now.len() / stride.max(1);
        match &mut self.what {
            What::Noting {
                ring,
                rows: h,
                spans,
            } => {
                let (ring, h) = (*ring, *h);
                let lines = self
                    .now
                    .chunks_mut(stride)
                    .zip(self.was.chunks(stride))
                    .zip(self.will.chunks(stride))
                    .zip(spans.iter_mut())
                    .enumerate();
                for (i, (((now, was), will), span)) in lines {
                    let y = top + i;
                    if y < ring || y >= h - ring {
                        blend_runs(now, was, will, t, true);
                        continue;
                    }
                    let (l, r) = (ring * 4, stride - ring * 4);
                    blend_runs(&mut now[..l], &was[..l], &will[..l], t, true);
                    blend_runs(&mut now[r..], &was[r..], &will[r..], t, true);
                    let s = blend_runs(&mut now[l..r], &was[l..r], &will[l..r], t, true);
                    *span = s.map(|(a, b)| (a + ring, b + ring));
                }
            }
            What::Within(rects) => {
                let (y0, y1) = (top as i32, (top + rows) as i32);
                for b in rects.iter() {
                    let (from, to) = (b.y.max(y0).max(0), b.bottom().min(y1));
                    let (x0, x1) = (b.x.max(0) as usize, b.right().max(0) as usize);
                    let x1 = x1.min(stride / 4);
                    if from >= to || x0 >= x1 {
                        continue;
                    }
                    for y in from..to {
                        let at = (y as usize - top) * stride + x0 * 4;
                        let end = at + (x1 - x0) * 4;
                        blend_runs(
                            &mut self.now[at..end],
                            &self.was[at..end],
                            &self.will[at..end],
                            t,
                            false,
                        );
                    }
                }
            }
        }
    }
}

/// Bytes a fade compares at a time, 16 pixels: where the two frames
/// agree over a run, the new one is copied, most of a change of screen
/// (the sidebar, the page's ground).
const RUN: usize = 4 * 16;

/// Costs a pass over the pixels. `now` set `t` (of 256) of the way from
/// `was` to `will`, three runs of the same pixels, [`RUN`] bytes at a
/// time; returns the first pixel and the end of the runs where `was` and
/// `will` differ, if any do. Where they agree `will` is copied only with
/// `agreeing`: a later step's canvas already shows it there.
fn blend_runs(
    now: &mut [u8],
    was: &[u8],
    will: &[u8],
    t: u16,
    agreeing: bool,
) -> Option<(usize, usize)> {
    let u = 256 - t;
    let mut span: Option<(usize, usize)> = None;
    let runs = now
        .chunks_mut(RUN)
        .zip(was.chunks(RUN))
        .zip(will.chunks(RUN))
        .enumerate();
    for (i, ((now, was), will)) in runs {
        if was == will {
            if agreeing {
                now.copy_from_slice(will);
            }
            continue;
        }
        let px = (i * RUN / 4, (i * RUN + will.len()) / 4);
        span = Some(span.map_or(px, |(a, _)| (a, px.1)));
        if t == 256 {
            now.copy_from_slice(will);
            continue;
        }
        for ((now, &w), &n) in now.iter_mut().zip(was).zip(will) {
            *now = ((u16::from(w) * u + u16::from(n) * t) >> 8) as u8;
        }
    }
    span
}

/// The bands [`Canvas::blend_noting`] gathers a row at a time, as columns
/// `[x0, x1)` and rows `[y0, y1)`.
#[derive(Default)]
struct Bands(Vec<(usize, usize, usize, usize)>);

impl Bands {
    /// At most this many bands: past it, bands next to each other are
    /// joined two by two.
    const MOST: usize = 256;
    /// A row joins the band above it when each edge of its columns is
    /// within this many pixels of the band's.
    const SLACK: usize = 64;

    /// Row `y` differs over the columns of `span`, or nowhere.
    fn row(&mut self, y: usize, span: Option<(usize, usize)>) {
        let Some((a, b)) = span else {
            return;
        };
        if let Some(last) = self.0.last_mut()
            && last.3 == y
            && a.abs_diff(last.0) <= Self::SLACK
            && b.abs_diff(last.1) <= Self::SLACK
        {
            *last = (last.0.min(a), last.1.max(b), last.2, y + 1);
            return;
        }
        self.0.push((a, b, y, y + 1));
    }

    /// The bands as rectangles into `out`.
    fn into_rects(mut self, out: &mut Vec<Rect>) {
        while self.0.len() > Self::MOST {
            self.0 = self
                .0
                .chunks(2)
                .map(|p| {
                    p.iter().skip(1).fold(p[0], |a, b| {
                        (a.0.min(b.0), a.1.max(b.1), a.2.min(b.2), a.3.max(b.3))
                    })
                })
                .collect();
        }
        out.extend(self.0.into_iter().map(|(x0, x1, y0, y1)| {
            Rect::new(x0 as i32, y0 as i32, (x1 - x0) as i32, (y1 - y0) as i32)
        }));
    }
}
