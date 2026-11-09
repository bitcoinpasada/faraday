//! Shrinking a captured frame's luma and chroma by an integer factor
//! when a device hands over more pixels than QR detection needs. Each
//! shell passes the widest frame it hands the core.

/// The size of the NV12 chroma plane for a `width` by `height` frame:
/// one U and V pair per 2 x 2 block of luma.
pub fn chroma_size(width: usize, height: usize) -> (usize, usize) {
    (width.div_ceil(2), height.div_ceil(2))
}

/// The integer factor that brings `width` down to at most `max_width`.
/// Always at least 1, so the usual 640 × 480 frame is untouched.
pub fn scale_factor(width: usize, max_width: usize) -> usize {
    if width <= max_width {
        return 1;
    }
    width.div_ceil(max_width)
}

/// Shrinks a packed luma buffer by an integer factor, averaging each
/// `factor × factor` block. Averaging rather than picking a corner
/// keeps a QR module's edge halfway grey instead of aliasing it away.
///
/// Returns the new size and buffer; a factor of 1 hands the buffer back
/// untouched. Rows and columns that do not fill a whole block are
/// dropped, so the result is `width / factor` by `height / factor`.
pub fn downscale(
    luma: Vec<u8>,
    width: usize,
    height: usize,
    factor: usize,
) -> (usize, usize, Vec<u8>) {
    let (w, h) = (width / factor.max(1), height / factor.max(1));
    if factor <= 1 || w == 0 || h == 0 || luma.len() < width * height {
        return (width, height, luma);
    }
    let area = (factor * factor) as u32;
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut sum = 0u32;
            for dy in 0..factor {
                let row = (y * factor + dy) * width + x * factor;
                sum += luma[row..row + factor]
                    .iter()
                    .map(|&v| u32::from(v))
                    .sum::<u32>();
            }
            out[y * w + x] = (sum / area) as u8;
        }
    }
    (w, h, out)
}

/// Shrinks an NV12 chroma plane by the factor [`downscale`] shrank its
/// luma by, averaging the U and the V of each `factor × factor` block of
/// pairs separately.
///
/// `width` and `height` are the luma's, before the shrink; the result is
/// the chroma plane for the shrunk luma. A factor of 1 hands the plane
/// back untouched.
pub fn downscale_chroma(chroma: Vec<u8>, width: usize, height: usize, factor: usize) -> Vec<u8> {
    let (cw, ch) = chroma_size(width, height);
    let (w, h) = (width / factor.max(1), height / factor.max(1));
    if factor <= 1 || w == 0 || h == 0 || chroma.len() < cw * ch * 2 {
        return chroma;
    }
    // The shrunk luma is `w` by `h`, so its chroma is that halved.
    let (out_w, out_h) = chroma_size(w, h);
    let mut out = vec![0u8; out_w * out_h * 2];
    for y in 0..out_h {
        for x in 0..out_w {
            let (mut u, mut v, mut n) = (0u32, 0u32, 0u32);
            for dy in 0..factor {
                let sy = y * factor + dy;
                if sy >= ch {
                    break;
                }
                for dx in 0..factor {
                    let sx = x * factor + dx;
                    if sx >= cw {
                        break;
                    }
                    let at = (sy * cw + sx) * 2;
                    u += u32::from(chroma[at]);
                    v += u32::from(chroma[at + 1]);
                    n += 1;
                }
            }
            let at = (y * out_w + x) * 2;
            out[at] = (u / n.max(1)) as u8;
            out[at + 1] = (v / n.max(1)) as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The widest a shell passes: the v4l2 shell's.
    const MAX_WIDTH: usize = 1920;

    #[test]
    fn downscale_chroma_averages_each_block_of_pairs() {
        // 4x4 luma, so 2x2 pairs; a factor of 2 leaves one pair.
        let uv = vec![10, 20, 12, 22, 14, 24, 16, 26];
        let out = downscale_chroma(uv, 4, 4, 2);
        assert_eq!(out, vec![13, 23], "U and V averaged separately");
    }

    #[test]
    fn scale_factor_only_shrinks_what_is_too_wide() {
        assert_eq!(scale_factor(640, MAX_WIDTH), 1);
        assert_eq!(scale_factor(MAX_WIDTH, MAX_WIDTH), 1);
        assert_eq!(scale_factor(MAX_WIDTH + 1, MAX_WIDTH), 2);
        // A 1080p frame and the Pi camera's 1296x972 mode pass whole.
        assert_eq!(scale_factor(1920, MAX_WIDTH), 1);
        assert_eq!(scale_factor(1296, MAX_WIDTH), 1);
        // A 2592-wide still mode halves to 1296.
        assert_eq!(scale_factor(2592, MAX_WIDTH), 2);
        assert!(2592 / scale_factor(2592, MAX_WIDTH) <= MAX_WIDTH);
    }

    #[test]
    fn downscale_averages_each_block() {
        // 4×4 of two values: every 2×2 block averages to 5.
        let src = vec![0, 10, 0, 10, 10, 0, 10, 0, 0, 10, 0, 10, 10, 0, 10, 0];
        let (w, h, out) = downscale(src, 4, 4, 2);
        assert_eq!((w, h), (2, 2));
        assert_eq!(out, vec![5, 5, 5, 5]);
    }

    #[test]
    fn downscale_keeps_a_factor_of_one_and_drops_partial_blocks() {
        let src: Vec<u8> = (0..12).collect();
        let (w, h, out) = downscale(src.clone(), 4, 3, 1);
        assert_eq!((w, h, out), (4, 3, src));
        // 5 columns by 3 rows at factor 2: one column and one row fall off.
        let src: Vec<u8> = vec![1; 15];
        let (w, h, out) = downscale(src, 5, 3, 2);
        assert_eq!((w, h), (2, 1));
        assert_eq!(out, vec![1, 1]);
        // Too small to shrink at all: handed back as it is.
        let (w, h, out) = downscale(vec![7; 4], 2, 2, 4);
        assert_eq!((w, h, out), (2, 2, vec![7; 4]));
    }
}
