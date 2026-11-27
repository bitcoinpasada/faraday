//! The integer network's arithmetic: the weight files, the convolutions, the
//! pooling and the requantization.
//!
//! Activations are 12-bit values in `i16`, channels last. A layer has `i16`
//! weights, `i32` biases and `i32` requantization multipliers (16 fractional
//! bits). Sums wrap on overflow as the reference's pairs of `i16` products
//! do. Every buffer that holds an image, a map or an activation is wiped when
//! it is dropped.

use zeroize::Zeroizing;

use crate::Error;

/// Largest value an activation takes after requantization.
const AMAX: i64 = 2047;
/// Fractional bits of the multipliers.
const SHIFT: u32 = 16;

/// A work buffer, wiped when dropped.
pub(crate) type Buf<T> = Zeroizing<Vec<T>>;

/// Reads the weight file front to back.
pub(crate) struct Reader {
    data: &'static [u8],
}

/// One layer's parameters, all little-endian bytes of the weight file.
pub(crate) struct Layer {
    /// `i16` weights, `[O][..taps..][C]`.
    pub(crate) w: &'static [u8],
    /// `i32` biases (or `f32` scales for an output layer), `O` of them.
    pub(crate) bias: &'static [u8],
    /// `i32` multipliers (or `f32` biases for an output layer), `O` of them.
    pub(crate) mul: &'static [u8],
}

impl Reader {
    pub(crate) fn new(data: &'static [u8]) -> Self {
        Self { data }
    }

    fn take(&mut self, n: usize) -> Result<&'static [u8], Error> {
        let (head, rest) = self.data.split_at_checked(n).ok_or(Error::Internal)?;
        self.data = rest;
        Ok(head)
    }

    /// The next layer: `o` outputs, `k` weights each.
    pub(crate) fn layer(&mut self, o: usize, k: usize) -> Result<Layer, Error> {
        Ok(Layer {
            w: self.take(2 * o * k)?,
            bias: self.take(4 * o)?,
            mul: self.take(4 * o)?,
        })
    }

    /// Whether every byte was read.
    pub(crate) fn done(&self) -> bool {
        self.data.is_empty()
    }
}

fn four(b: &[u8], i: usize) -> [u8; 4] {
    b.get(4 * i..4 * i + 4)
        .and_then(|s| <[u8; 4]>::try_from(s).ok())
        .unwrap_or([0; 4])
}

/// The `i`-th little-endian `i32` of `b` (0 past the end).
pub(crate) fn i32_at(b: &[u8], i: usize) -> i32 {
    i32::from_le_bytes(four(b, i))
}

/// The `i`-th little-endian `f32` of `b` (0 past the end).
pub(crate) fn f32_at(b: &[u8], i: usize) -> f32 {
    f32::from_le_bytes(four(b, i))
}

/// Sum of products of activations and little-endian `i16` weights, wrapping.
fn dot(a: &[i16], w: &[u8]) -> i32 {
    a.iter().zip(w.chunks_exact(2)).fold(0i32, |s, (&x, b)| {
        let k = i16::from_le_bytes([b[0], b[1]]);
        s.wrapping_add(i32::from(x).wrapping_mul(i32::from(k)))
    })
}

/// The weights of output `o`, tap `tap` of `taps`, for `c` input channels.
fn kernel(w: &[u8], o: usize, taps: usize, tap: usize, c: usize) -> &[u8] {
    let start = (o * taps + tap) * c * 2;
    w.get(start..start + 2 * c).unwrap_or(&[])
}

/// The input image as activations: zero-padded by one on every side,
/// `(rows + 2) x (cols + 2)`, values 0..=255 scaled to 0..=2040. `cols` may
/// be wider than `width`: the rest stays zero.
pub(crate) fn padded_input(img: &[u8], rows: usize, width: usize, cols: usize) -> Buf<i16> {
    let stride = cols + 2;
    let mut out = Zeroizing::new(vec![0i16; (rows + 2) * stride]);
    for (y, row) in img.chunks_exact(width.max(1)).take(rows).enumerate() {
        let start = (y + 1) * stride + 1;
        if let Some(dst) = out.get_mut(start..start + width) {
            for (d, &p) in dst.iter_mut().zip(row) {
                *d = i16::from(p) << 3;
            }
        }
    }
    out
}

/// The first layer's convolution: one input channel on the padded input,
/// weights `[O][9]`, accumulators `rows x cols x O`.
pub(crate) fn conv_first(input: &[i16], rows: usize, cols: usize, l: &Layer, o: usize) -> Buf<i32> {
    let stride = cols + 2;
    let mut acc = Zeroizing::new(vec![0i32; rows * cols * o]);
    for y in 0..rows {
        for x in 0..cols {
            for k in 0..o {
                let mut s = i32_at(l.bias, k);
                for tap in 0..9 {
                    let (dy, dx) = (tap / 3, tap % 3);
                    let p = input.get((y + dy) * stride + x + dx).copied().unwrap_or(0);
                    let kw = kernel(l.w, k, 9, tap, 1);
                    let kv = i16::from_le_bytes([
                        kw.first().copied().unwrap_or(0),
                        kw.get(1).copied().unwrap_or(0),
                    ]);
                    s = s.wrapping_add(i32::from(p).wrapping_mul(i32::from(kv)));
                }
                if let Some(a) = acc.get_mut((y * cols + x) * o + k) {
                    *a = s;
                }
            }
        }
    }
    acc
}

/// 3x3 convolution with zero padding: input `rows x cols x c`, weights
/// `[O][3][3][C]`, accumulators `rows x cols x O`; `biased` adds the layer's
/// biases (an output layer has none).
pub(crate) fn conv3x3(
    input: &[i16],
    rows: usize,
    cols: usize,
    c: usize,
    l: &Layer,
    o: usize,
    biased: bool,
) -> Buf<i32> {
    let mut acc = Zeroizing::new(vec![0i32; rows * cols * o]);
    for (px, out) in acc.chunks_exact_mut(o.max(1)).enumerate() {
        let (y, x) = (px / cols.max(1), px % cols.max(1));
        for (k, a) in out.iter_mut().enumerate() {
            *a = if biased { i32_at(l.bias, k) } else { 0 };
        }
        for tap in 0..9 {
            let (yy, xx) = ((y + tap / 3).wrapping_sub(1), (x + tap % 3).wrapping_sub(1));
            if yy >= rows || xx >= cols {
                continue;
            }
            let start = (yy * cols + xx) * c;
            let Some(p) = input.get(start..start + c) else {
                continue;
            };
            for (k, a) in out.iter_mut().enumerate() {
                *a = a.wrapping_add(dot(p, kernel(l.w, k, 9, tap, c)));
            }
        }
    }
    acc
}

/// ReLU and requantization of an accumulator to the 12-bit range.
fn requant(acc: i32, mul: i32) -> i16 {
    if acc <= 0 {
        return 0;
    }
    let v = (i64::from(acc) * i64::from(mul) + (1 << (SHIFT - 1))) >> SHIFT;
    v.min(AMAX) as i16
}

/// Max pooling `ph x pw` of accumulators `rows x cols x o`, then requantization
/// with the layer's multipliers.
pub(crate) fn pool(
    acc: &[i32],
    rows: usize,
    cols: usize,
    o: usize,
    ph: usize,
    pw: usize,
    l: &Layer,
) -> Buf<i16> {
    let (h2, w2) = (rows / ph, cols / pw);
    let mut out = Zeroizing::new(vec![0i16; h2 * w2 * o]);
    for y in 0..h2 {
        for x in 0..w2 {
            for k in 0..o {
                let mut m = i32::MIN;
                for dy in 0..ph {
                    for dx in 0..pw {
                        let v = acc
                            .get(((y * ph + dy) * cols + x * pw + dx) * o + k)
                            .copied()
                            .unwrap_or(i32::MIN);
                        m = m.max(v);
                    }
                }
                if let Some(d) = out.get_mut((y * w2 + x) * o + k) {
                    *d = requant(m, i32_at(l.mul, k));
                }
            }
        }
    }
    out
}

/// 1D convolution, kernel 3, dilation `dil`, zero padding: input `t x c`,
/// weights `[O][3][C]`, output `t x O` requantized.
pub(crate) fn conv1d(
    input: &[i16],
    t: usize,
    c: usize,
    l: &Layer,
    o: usize,
    dil: usize,
) -> Buf<i16> {
    let mut out = Zeroizing::new(vec![0i16; t * o]);
    for ti in 0..t {
        for k in 0..o {
            let mut s = i32_at(l.bias, k);
            for j in 0..3 {
                let tt = (ti + j * dil).wrapping_sub(dil);
                if tt >= t {
                    continue;
                }
                if let Some(p) = input.get(tt * c..tt * c + c) {
                    s = s.wrapping_add(dot(p, kernel(l.w, k, 3, j, c)));
                }
            }
            if let Some(d) = out.get_mut(ti * o + k) {
                *d = requant(s, i32_at(l.mul, k));
            }
        }
    }
    out
}

/// Dot product of activations with output row `k` of an output layer's
/// weights `[O][n]`.
pub(crate) fn dense(a: &[i16], l: &Layer, k: usize) -> i32 {
    dot(a, kernel(l.w, k, 1, 0, a.len()))
}
