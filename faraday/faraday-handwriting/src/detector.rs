//! The word detector: a fully convolutional network that finds the words, the
//! line numbers and the cards on a photo reduced to
//! `DETECTOR_WIDTH x DETECTOR_HEIGHT`.
//!
//! Encoder e1-e4 (conv 3x3, max 2x2), e5 and e6 (conv 3x3), e7 (conv 3x3, max
//! 2x2), e8 (conv 3x3); decoder d4 = conv(e6 + up(e8)), d3 = conv(e3 +
//! up(d4)), d2 = conv(e2 + up(d3)), d1 = conv(e1 + up(d2)); the output
//! layer is a conv 3x3 on d1. `up` is nearest upsampling by 2 and `+` a
//! channel concatenation.

use core::fmt;

use zeroize::Zeroizing;

use crate::nn::{self, Buf, Reader, f32_at};
use crate::{DETECTOR_HEIGHT, DETECTOR_OUTPUTS, DETECTOR_WIDTH, Error};

const LAYERS: usize = 12;

static WEIGHTS: &[u8] = include_bytes!("../weights/detector.bin");

/// One layer: the layer feeding it (None: the image), the layer upsampled
/// and concatenated after it, whether a 2x2 max pooling follows, and the
/// output channels.
struct Spec {
    a: Option<usize>,
    b: Option<usize>,
    pool: bool,
    out: usize,
}

const fn spec(a: Option<usize>, b: Option<usize>, pool: bool, out: usize) -> Spec {
    Spec { a, b, pool, out }
}

const SPECS: [Spec; LAYERS] = [
    spec(None, None, true, 8),
    spec(Some(0), None, true, 16),
    spec(Some(1), None, true, 32),
    spec(Some(2), None, true, 48),
    spec(Some(3), None, false, 64),
    spec(Some(4), None, false, 64),
    spec(Some(5), None, true, 64),
    spec(Some(6), None, false, 64),
    spec(Some(5), Some(7), false, 32),
    spec(Some(2), Some(8), false, 32),
    spec(Some(1), Some(9), false, 16),
    spec(Some(0), Some(10), false, 8),
];

/// Input channels of layer `i`.
const fn inputs(i: usize) -> usize {
    match (SPECS[i].a, SPECS[i].b) {
        (None, _) => 1,
        (Some(a), None) => SPECS[a].out,
        (Some(a), Some(b)) => SPECS[a].out + SPECS[b].out,
    }
}

const HEAD_TAPS: usize = 9 * SPECS[LAYERS - 1].out;

const fn weights_len() -> usize {
    let mut n = 2 * DETECTOR_OUTPUTS * HEAD_TAPS + 8 * DETECTOR_OUTPUTS;
    let mut i = 0;
    while i < LAYERS {
        n += 2 * SPECS[i].out * 9 * inputs(i) + 8 * SPECS[i].out;
        i += 1;
    }
    n
}

const _: () = assert!(WEIGHTS.len() == weights_len());

/// The detector's output maps: `DETECTOR_OUTPUTS` channels of
/// `DETECTOR_HEIGHT / 2` rows by `DETECTOR_WIDTH / 2` columns, channel-major.
/// Wiped when dropped.
pub struct Maps {
    pub(crate) data: Buf<f32>,
}

impl Maps {
    /// Rows of each map.
    pub const ROWS: usize = DETECTOR_HEIGHT / 2;
    /// Columns of each map.
    pub const COLUMNS: usize = DETECTOR_WIDTH / 2;

    /// All channels, one after the other.
    pub fn values(&self) -> &[f32] {
        &self.data
    }

    /// Channel `k` (row-major, `ROWS x COLUMNS`); empty past the last.
    pub fn channel(&self, k: usize) -> &[f32] {
        let n = Self::ROWS * Self::COLUMNS;
        match (
            k.checked_mul(n),
            k.checked_add(1).and_then(|j| j.checked_mul(n)),
        ) {
            (Some(a), Some(b)) => self.data.get(a..b).unwrap_or(&[]),
            _ => &[],
        }
    }
}

impl fmt::Debug for Maps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Maps({} channels)", DETECTOR_OUTPUTS)
    }
}

/// Copies `t` (`rows/up x cols/up x c`, nearest upsampling by `up`) into
/// channels `offset..offset + c` of `out` (`rows x cols x total`).
fn gather(
    t: &[i16],
    c: usize,
    up: usize,
    shape: (usize, usize, usize),
    offset: usize,
    out: &mut [i16],
) {
    let (rows, cols, total) = shape;
    let tw = cols / up;
    for y in 0..rows {
        for x in 0..cols {
            let src = ((y / up) * tw + x / up) * c;
            let dst = (y * cols + x) * total + offset;
            if let (Some(s), Some(d)) = (t.get(src..src + c), out.get_mut(dst..dst + c)) {
                d.copy_from_slice(s);
            }
        }
    }
}

/// Costs one run of the detector network (hundreds of milliseconds on the
/// Pi's panel); the output maps of `image`, `DETECTOR_HEIGHT` rows by
/// `DETECTOR_WIDTH` columns of grayscale, row-major.
pub fn detector_maps(image: &[u8]) -> Result<Maps, Error> {
    if image.len() != DETECTOR_HEIGHT * DETECTOR_WIDTH {
        return Err(Error::Size);
    }
    let mut r = Reader::new(WEIGHTS);
    let mut layers = Vec::with_capacity(LAYERS);
    for (i, s) in SPECS.iter().enumerate() {
        layers.push(r.layer(s.out, 9 * inputs(i))?);
    }
    let head = r.layer(DETECTOR_OUTPUTS, HEAD_TAPS)?;
    if !r.done() {
        return Err(Error::Internal);
    }

    // Outputs and sizes (rows, columns) of the layers so far.
    let mut outs: Vec<Buf<i16>> = Vec::with_capacity(LAYERS);
    let mut dims: Vec<(usize, usize)> = Vec::with_capacity(LAYERS);
    for (i, (s, l)) in SPECS.iter().zip(&layers).enumerate() {
        let p = if s.pool { 2 } else { 1 };
        let (rows, cols, acc) = match s.a {
            None => {
                let input =
                    nn::padded_input(image, DETECTOR_HEIGHT, DETECTOR_WIDTH, DETECTOR_WIDTH);
                let acc = nn::conv_first(&input, DETECTOR_HEIGHT, DETECTOR_WIDTH, l, s.out);
                (DETECTOR_HEIGHT, DETECTOR_WIDTH, acc)
            }
            Some(a) => {
                let (rows, cols) = dims.get(a).copied().ok_or(Error::Internal)?;
                let ina = outs.get(a).ok_or(Error::Internal)?;
                let acc = match s.b {
                    None => nn::conv3x3(ina, rows, cols, inputs(i), l, s.out, true),
                    Some(b) => {
                        let inb = outs.get(b).ok_or(Error::Internal)?;
                        let total = inputs(i);
                        let ca = SPECS[a].out;
                        let mut cat = Zeroizing::new(vec![0i16; rows * cols * total]);
                        gather(ina, ca, 1, (rows, cols, total), 0, &mut cat);
                        gather(inb, total - ca, 2, (rows, cols, total), ca, &mut cat);
                        nn::conv3x3(&cat, rows, cols, total, l, s.out, true)
                    }
                };
                (rows, cols, acc)
            }
        };
        outs.push(nn::pool(&acc, rows, cols, s.out, p, p, l));
        dims.push((rows / p, cols / p));
    }

    let (rows, cols) = dims.get(LAYERS - 1).copied().ok_or(Error::Internal)?;
    let last = outs.get(LAYERS - 1).ok_or(Error::Internal)?;
    let acc = nn::conv3x3(
        last,
        rows,
        cols,
        SPECS[LAYERS - 1].out,
        &head,
        DETECTOR_OUTPUTS,
        false,
    );
    let n = rows * cols;
    let mut data = Zeroizing::new(vec![0f32; DETECTOR_OUTPUTS * n]);
    for i in 0..n {
        for o in 0..DETECTOR_OUTPUTS {
            let s = acc.get(i * DETECTOR_OUTPUTS + o).copied().unwrap_or(0);
            if let Some(d) = data.get_mut(o * n + i) {
                *d = (s as f32) * f32_at(head.bias, o) + f32_at(head.mul, o);
            }
        }
    }
    Ok(Maps { data })
}
