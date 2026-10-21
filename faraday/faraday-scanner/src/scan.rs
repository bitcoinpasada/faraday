//! The preview's reduction, luma and chroma, copied from
//! `opensigner/opensigner-core/src/scan.rs` at upstream commit `d26b84e`.
//! The scanner reads the reduced luma; the stick's camera reduces each
//! frame with these before the core sees it.

use faraday_draw::tokens;

/// The factor a frame `width` by `height` is reduced by so that its
/// shorter side is at most [`tokens::PREVIEW_MAX`] pixels.
pub fn reduce_factor(width: usize, height: usize) -> usize {
    width.min(height).div_ceil(tokens::PREVIEW_MAX).max(1)
}

/// Reduces a frame for the preview: the box average of every
/// `factor × factor` block, where the factor is [`reduce_factor`]. Each
/// source byte is read once and nothing is allocated but the output, so
/// a 640 × 480 frame ten times a second is affordable on a Pi.
///
/// A block at the right or bottom edge that runs off the frame averages
/// the part of it that exists.
pub fn reduce(width: usize, height: usize, luma: &[u8]) -> (u16, u16, Vec<u8>) {
    if width == 0 || height == 0 || luma.len() < width * height {
        return (0, 0, Vec::new());
    }
    let factor = reduce_factor(width, height);
    let (out_w, out_h) = (width.div_ceil(factor), height.div_ceil(factor));
    let mut out = vec![0u8; out_w * out_h];
    for oy in 0..out_h {
        let rows = (oy * factor)..((oy + 1) * factor).min(height);
        for ox in 0..out_w {
            let cols = (ox * factor)..((ox + 1) * factor).min(width);
            let mut sum = 0u32;
            let mut n = 0u32;
            for y in rows.clone() {
                for x in cols.clone() {
                    sum += u32::from(luma[y * width + x]);
                    n += 1;
                }
            }
            out[oy * out_w + ox] = (sum / n.max(1)) as u8;
        }
    }
    (out_w as u16, out_h as u16, out)
}

/// The size of the NV12 chroma plane for a `width` by `height` frame:
/// one U and V pair per 2 x 2 block of luma, so `⌈width / 2⌉` pairs
/// across and `⌈height / 2⌉` down.
fn chroma_size(width: usize, height: usize) -> (usize, usize) {
    (width.div_ceil(2), height.div_ceil(2))
}

/// Reduces a frame's chroma plane by `factor`, the factor [`reduce`]
/// shrank its luma by: the box average of every `factor × factor` block
/// of pairs, U and V averaged separately, so the reduced chroma still
/// carries one pair per 2 x 2 block of the reduced luma.
///
/// `width` and `height` are the luma's, before the reduction.
pub fn reduce_chroma(width: usize, height: usize, chroma: &[u8], factor: usize) -> Vec<u8> {
    let (cw, ch) = chroma_size(width, height);
    let factor = factor.max(1);
    if cw == 0 || ch == 0 || chroma.len() < cw * ch * 2 {
        return Vec::new();
    }
    if factor == 1 {
        return chroma[..cw * ch * 2].to_vec();
    }
    let (out_w, out_h) = (cw.div_ceil(factor), ch.div_ceil(factor));
    let mut out = vec![0u8; out_w * out_h * 2];
    for oy in 0..out_h {
        let rows = (oy * factor)..((oy + 1) * factor).min(ch);
        for ox in 0..out_w {
            let cols = (ox * factor)..((ox + 1) * factor).min(cw);
            let (mut u, mut v, mut n) = (0u32, 0u32, 0u32);
            for y in rows.clone() {
                for x in cols.clone() {
                    let at = (y * cw + x) * 2;
                    u += u32::from(chroma[at]);
                    v += u32::from(chroma[at + 1]);
                    n += 1;
                }
            }
            let at = (oy * out_w + ox) * 2;
            out[at] = (u / n.max(1)) as u8;
            out[at + 1] = (v / n.max(1)) as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_factor_keeps_the_shorter_side_under_the_maximum() {
        assert_eq!(reduce_factor(640, 480), 2, "480 halves to 240");
        assert_eq!(reduce_factor(400, 400), 1, "already at the maximum");
        assert_eq!(reduce_factor(4096, 3072), 8);
        assert_eq!(reduce_factor(1, 1), 1);
    }

    #[test]
    fn reduce_box_averages_every_block() {
        // 4 x 4, values 0..16, reduced by 2: each output is the mean of
        // its 2 x 2 block.
        let luma: Vec<u8> = (0..16).collect();
        let (w, h, out) = reduce(4, 4, &luma);
        assert_eq!((w, h), (4, 4), "a small frame is left alone");
        assert_eq!(out, luma);

        // A frame whose shorter side is over the maximum reduces, and a
        // block that runs off the edge averages what exists.
        let wide = vec![10u8; 801 * 801];
        let (w, h, out) = reduce(801, 801, &wide);
        assert_eq!(reduce_factor(801, 801), 3);
        assert_eq!((w, h), (267, 267));
        assert!(out.iter().all(|&p| p == 10), "a flat frame stays flat");

        // The average itself, on a frame that does reduce: half the rows
        // black and half white gives the mid grey.
        let mut half = vec![0u8; 800 * 800];
        for (y, row) in half.chunks_exact_mut(800).enumerate() {
            if y % 2 == 1 {
                row.fill(200);
            }
        }
        let (_, _, out) = reduce(800, 800, &half);
        assert!(out.iter().all(|&p| p == 100), "0 and 200 average to 100");
    }
}
