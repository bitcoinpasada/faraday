//! Turning a locked pixel buffer's two planes into the packed 8-bit
//! luma the core decodes and the NV12 chroma it draws the preview from,
//! and shrinking both when the device hands us more pixels than QR
//! detection needs.
//!
//! The bi-planar video-range format the session asks for already lays
//! plane 1 out as interleaved Cb and Cr pairs, one per 2 x 2 block of
//! luma, which is the order the core takes; the only work is the row
//! stride and the shrink.
//!
//! None of this is macOS-specific and none of it is `unsafe`, so it is
//! compiled and tested on every platform: it is the half of the capture
//! path that can be checked on the build box.

/// The widest frame we hand the core. Above this a frame is shrunk by an
/// integer factor: `rqrr` needs no more than about 640 pixels across to
/// find a code, and a Mac's built-in camera offers 1280 × 720 and more.
pub const MAX_WIDTH: usize = 800;

/// Copies plane 0 out of a bi-planar 4:2:0 buffer into a packed
/// `width × height` buffer. One byte per pixel, so a row is `width`
/// bytes of the `stride` bytes AVFoundation lays out.
///
/// Returns `None` when the plane is shorter than the rows it claims,
/// which is not something to guess at.
pub fn luma(plane: &[u8], width: usize, height: usize, stride: usize) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || stride < width {
        return None;
    }
    // The last row needs only its own pixels, not a whole stride.
    let needed = stride * (height - 1) + width;
    if plane.len() < needed {
        return None;
    }
    let mut out = vec![0u8; width * height];
    for y in 0..height {
        let row = &plane[y * stride..y * stride + width];
        out[y * width..(y + 1) * width].copy_from_slice(row);
    }
    Some(out)
}

/// The size of the chroma plane for a `width` by `height` frame: one Cb
/// and Cr pair per 2 x 2 block of luma.
pub fn chroma_size(width: usize, height: usize) -> (usize, usize) {
    (width.div_ceil(2), height.div_ceil(2))
}

/// Copies plane 1 out of a bi-planar 4:2:0 buffer into a packed
/// `⌈width / 2⌉ × ⌈height / 2⌉` plane of interleaved pairs. `width` and
/// `height` are the luma's; `stride` is plane 1's own bytes per row.
///
/// Returns `None` when the plane is shorter than the rows it claims.
pub fn chroma(plane: &[u8], width: usize, height: usize, stride: usize) -> Option<Vec<u8>> {
    let (cw, ch) = chroma_size(width, height);
    if cw == 0 || ch == 0 || stride < cw * 2 {
        return None;
    }
    // The last row needs only its own pairs, not a whole stride.
    if plane.len() < stride * (ch - 1) + cw * 2 {
        return None;
    }
    let mut out = vec![0u8; cw * ch * 2];
    for y in 0..ch {
        let row = &plane[y * stride..y * stride + cw * 2];
        out[y * cw * 2..(y + 1) * cw * 2].copy_from_slice(row);
    }
    Some(out)
}

/// Shrinks a chroma plane by the factor [`downscale`] shrank its luma
/// by, averaging the Cb and the Cr of each `factor × factor` block of
/// pairs separately. `width` and `height` are the luma's, before the
/// shrink.
pub fn downscale_chroma(chroma: Vec<u8>, width: usize, height: usize, factor: usize) -> Vec<u8> {
    let (cw, ch) = chroma_size(width, height);
    let (w, h) = (width / factor.max(1), height / factor.max(1));
    if factor <= 1 || w == 0 || h == 0 || chroma.len() < cw * ch * 2 {
        return chroma;
    }
    let (out_w, out_h) = chroma_size(w, h);
    let mut out = vec![0u8; out_w * out_h * 2];
    for y in 0..out_h {
        for x in 0..out_w {
            let (mut cb, mut cr, mut n) = (0u32, 0u32, 0u32);
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
                    cb += u32::from(chroma[at]);
                    cr += u32::from(chroma[at + 1]);
                    n += 1;
                }
            }
            let at = (y * out_w + x) * 2;
            out[at] = (cb / n.max(1)) as u8;
            out[at + 1] = (cr / n.max(1)) as u8;
        }
    }
    out
}

/// The integer factor that brings `width` down to at most [`MAX_WIDTH`].
/// Always at least 1, so a 640 × 480 frame is untouched.
pub fn scale_factor(width: usize) -> usize {
    if width <= MAX_WIDTH {
        return 1;
    }
    width.div_ceil(MAX_WIDTH)
}

/// Shrinks a packed luma buffer by an integer factor, averaging each
/// `factor × factor` block. Averaging rather than picking a corner keeps
/// a QR module's edge halfway grey instead of aliasing it away.
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
