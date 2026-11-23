//! The word network: an ink map of one handwritten word to per-frame class
//! logits.
//!
//! conv 3x3 1->16, max 2x2; conv 3x3 16->32, max 2x2; conv 3x3 32->64, max
//! (2,1); conv 3x3 64->64, max (2,1); the two rows of columns joined into
//! vectors of 128; conv1d k3 -> 128; conv1d k3 dilation 2 -> 128; k1 -> 37.

use core::fmt;

use zeroize::Zeroizing;

use crate::nn::{self, Buf, Reader, f32_at};
use crate::{Error, INK_HEIGHT, MAX_WIDTH};

/// Classes: 0 is the CTC blank, 1-26 are a-z, 27-36 are 0-9.
pub const CLASSES: usize = 37;

const C1: usize = 16;
const C2: usize = 32;
const C3: usize = 64;
const HIDDEN: usize = 128;

static WEIGHTS: &[u8] = include_bytes!("../weights/word.bin");

/// Outputs and weights per output of the six convolutions.
const LAYERS: [(usize, usize); 6] = [
    (C1, 9),
    (C2, 9 * C1),
    (C3, 9 * C2),
    (C3, 9 * C3),
    (HIDDEN, 3 * 2 * C3),
    (HIDDEN, 3 * HIDDEN),
];

const fn weights_len() -> usize {
    let mut n = 2 * CLASSES * HIDDEN + 8 * CLASSES;
    let mut i = 0;
    while i < LAYERS.len() {
        n += 2 * LAYERS[i].0 * LAYERS[i].1 + 8 * LAYERS[i].0;
        i += 1;
    }
    n
}

const _: () = assert!(WEIGHTS.len() == weights_len());

/// Frames the network gives for an ink map of `width` columns.
pub const fn frames(width: usize) -> usize {
    if width / 4 > 0 { width / 4 } else { 1 }
}

/// The logits of one strip: `CLASSES` rows of `frames` values, class-major.
/// Wiped when dropped.
pub struct Logits {
    pub(crate) data: Buf<f32>,
    pub(crate) frames: usize,
}

impl Logits {
    /// Number of frames (columns of the table).
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// The values, `CLASSES` rows of `frames()` each: class `k` at frame `t`
    /// is `values()[k * frames() + t]`.
    pub fn values(&self) -> &[f32] {
        &self.data
    }
}

impl fmt::Debug for Logits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Logits({} frames)", self.frames)
    }
}

/// Costs one run of the word network over the ink map (a few milliseconds a
/// hundred columns); the logits of the ink map `ink` (`INK_HEIGHT` rows by
/// `width` columns, row-major, 0 paper and 255 ink).
pub fn word_logits(ink: &[u8], width: usize) -> Result<Logits, Error> {
    if width == 0 || INK_HEIGHT.checked_mul(width) != Some(ink.len()) {
        return Err(Error::Size);
    }
    if width > MAX_WIDTH {
        return Err(Error::TooWide);
    }
    // Padded on the right with 16 empty columns, to a multiple of 4.
    let w = (width + 16).div_ceil(4) * 4;
    let (w2, w4, t) = (w / 2, w / 4, frames(width));

    let mut r = Reader::new(WEIGHTS);
    let l = [
        r.layer(LAYERS[0].0, LAYERS[0].1)?,
        r.layer(LAYERS[1].0, LAYERS[1].1)?,
        r.layer(LAYERS[2].0, LAYERS[2].1)?,
        r.layer(LAYERS[3].0, LAYERS[3].1)?,
        r.layer(LAYERS[4].0, LAYERS[4].1)?,
        r.layer(LAYERS[5].0, LAYERS[5].1)?,
    ];
    let out = r.layer(CLASSES, HIDDEN)?;
    if !r.done() {
        return Err(Error::Internal);
    }

    let input = nn::padded_input(ink, INK_HEIGHT, width, w);
    let acc = nn::conv_first(&input, INK_HEIGHT, w, &l[0], C1);
    drop(input);
    let a = nn::pool(&acc, INK_HEIGHT, w, C1, 2, 2, &l[0]);
    let acc = nn::conv3x3(&a, 16, w2, C1, &l[1], C2, true);
    let b = nn::pool(&acc, 16, w2, C2, 2, 2, &l[1]);
    let acc = nn::conv3x3(&b, 8, w4, C2, &l[2], C3, true);
    let a = nn::pool(&acc, 8, w4, C3, 2, 1, &l[2]);
    let acc = nn::conv3x3(&a, 4, w4, C3, &l[3], C3, true);
    let b = nn::pool(&acc, 4, w4, C3, 2, 1, &l[3]);
    drop(acc);

    // Columns: a[t][h * C3 + c] = b[h][t][c].
    let mut cols = Zeroizing::new(vec![0i16; w4 * 2 * C3]);
    for (ti, dst) in cols.chunks_exact_mut(2 * C3).enumerate() {
        for (h, half) in dst.chunks_exact_mut(C3).enumerate() {
            if let Some(src) = b.get((h * w4 + ti) * C3..(h * w4 + ti + 1) * C3) {
                half.copy_from_slice(src);
            }
        }
    }
    let h1 = nn::conv1d(&cols, w4, 2 * C3, &l[4], HIDDEN, 1);
    let h2 = nn::conv1d(&h1, w4, HIDDEN, &l[5], HIDDEN, 2);

    let mut logits = Zeroizing::new(vec![0f32; CLASSES * t]);
    for ti in 0..t {
        let Some(p) = h2.get(ti * HIDDEN..(ti + 1) * HIDDEN) else {
            continue;
        };
        for k in 0..CLASSES {
            let s = nn::dense(p, &out, k);
            if let Some(d) = logits.get_mut(k * t + ti) {
                *d = (s as f32) * f32_at(out.bias, k) + f32_at(out.mul, k);
            }
        }
    }
    Ok(Logits {
        data: logits,
        frames: t,
    })
}
