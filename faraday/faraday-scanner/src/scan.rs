//! The preview's reduction, copied from
//! `opensigner/opensigner-core/src/scan.rs` at upstream commit `d26b84e`,
//! so that the scanner holds no part of OpenSigner's core.

use osk_ui::tokens;

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
