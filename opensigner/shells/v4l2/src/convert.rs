//! Turning a captured buffer into the 8-bit luma the core decodes and
//! the NV12 chroma it draws the preview from, and shrinking both so a
//! large sensor mode does not cost a Pi 3B+ a second a frame.
//!
//! All four formats we accept put the luma first and one byte per pixel:
//! `GREY` is luma alone, `NV12` and `YU12` start with a luma plane, and
//! `YUYV` interleaves it every second byte. So one routine covers them
//! all, parameterised by the step between neighbouring luma bytes, and
//! all four honour a row stride wider than the row.
//!
//! The colour is laid out three different ways, so [`chroma`] has a
//! branch each: `YUYV` carries a U and a V for every pair of pixels of
//! every row, `NV12` an interleaved plane after the luma, `YU12` a U
//! plane then a V plane, and `GREY` no colour at all. All three become
//! the one NV12 order the core takes.

use crate::abi;

/// The widest frame we hand the core. Above this a frame is shrunk by an
/// integer factor: `rqrr` needs no more than about 640 pixels across to
/// find a code, and a 1296 × 972 sensor mode is five times the pixels of
/// the 640 × 480 we ask for.
pub const MAX_WIDTH: usize = 800;

/// Bytes between neighbouring luma samples for a format we accept, or
/// `None` for one we do not.
pub fn luma_step(format: u32) -> Option<usize> {
    match format {
        abi::PIX_FMT_GREY | abi::PIX_FMT_NV12 | abi::PIX_FMT_YU12 => Some(1),
        abi::PIX_FMT_YUYV => Some(2),
        _ => None,
    }
}

/// The packed row length for a format, used when the driver reports a
/// `bytesperline` of zero.
pub fn packed_stride(format: u32, width: usize) -> Option<usize> {
    luma_step(format).map(|step| width * step)
}

/// Copies the luma plane out of `src` into a packed `width × height`
/// buffer.
///
/// `stride` is the driver's `bytesperline`, which may be wider than the
/// row; `step` is [`luma_step`]. Returns `None` when the buffer is
/// shorter than the rows it claims, which is a driver that wrote less
/// than it said and not something to guess at.
pub fn luma(
    src: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    step: usize,
) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || step == 0 || stride < width * step {
        return None;
    }
    // The last row needs only its own pixels, not a whole stride.
    let needed = stride * (height - 1) + width * step;
    if src.len() < needed {
        return None;
    }
    let mut out = vec![0u8; width * height];
    for y in 0..height {
        let row = &src[y * stride..];
        let dst = &mut out[y * width..(y + 1) * width];
        if step == 1 {
            // GREY and the two planar formats: the row is already the
            // luma, so this is one memcpy a row rather than a loop over
            // three hundred thousand pixels.
            dst.copy_from_slice(&row[..width]);
        } else {
            for (x, d) in dst.iter_mut().enumerate() {
                *d = row[x * step];
            }
        }
    }
    Some(out)
}

/// The size of the NV12 chroma plane for a `width` by `height` frame:
/// one U and V pair per 2 x 2 block of luma.
pub fn chroma_size(width: usize, height: usize) -> (usize, usize) {
    (width.div_ceil(2), height.div_ceil(2))
}

/// Copies the colour out of `src` as one NV12 chroma plane:
/// `⌈width / 2⌉ × ⌈height / 2⌉` pairs of interleaved U and V, row-major.
///
/// `None` for `GREY`, which has no colour, and for a buffer shorter than
/// the planes the format claims — a driver that wrote less than it said
/// is not something to guess at, and the preview is grey instead.
///
/// `stride` is the driver's `bytesperline` for the luma. The two planar
/// formats lay their chroma out at half that stride, rounded up, which
/// is what every driver that pads does.
pub fn chroma(
    format: u32,
    src: &[u8],
    width: usize,
    height: usize,
    stride: usize,
) -> Option<Vec<u8>> {
    let (cw, ch) = chroma_size(width, height);
    if width == 0 || height == 0 {
        return None;
    }
    let mut out = vec![0u8; cw * ch * 2];
    match format {
        // Packed 4:2:2: Y U Y V, so every second row of pixel pairs
        // gives the pair a 4:2:0 block wants.
        abi::PIX_FMT_YUYV => {
            if stride < width * 2 || src.len() < stride * (height - 1) + width * 2 {
                return None;
            }
            for y in 0..ch {
                let row = &src[(y * 2) * stride..];
                for x in 0..cw {
                    let at = x * 4;
                    let to = (y * cw + x) * 2;
                    out[to] = row[at + 1];
                    out[to + 1] = row[(at + 3).min(width * 2 - 1)];
                }
            }
        }
        // Semi-planar 4:2:0: the plane after the luma is already the
        // order the core wants, less any row padding.
        abi::PIX_FMT_NV12 => {
            let c_stride = stride.max(width);
            let plane = src.get(c_stride * height..)?;
            if c_stride < cw * 2 || plane.len() < c_stride * (ch - 1) + cw * 2 {
                return None;
            }
            for y in 0..ch {
                let row = &plane[y * c_stride..];
                out[y * cw * 2..(y + 1) * cw * 2].copy_from_slice(&row[..cw * 2]);
            }
        }
        // Planar 4:2:0: a U plane then a V plane, each at half the
        // luma's stride, interleaved into NV12 order.
        abi::PIX_FMT_YU12 => {
            let y_stride = stride.max(width);
            let c_stride = y_stride.div_ceil(2).max(cw);
            let planes = src.get(y_stride * height..)?;
            let plane_len = c_stride * ch;
            if planes.len() < plane_len * 2 {
                return None;
            }
            let (u, v) = planes.split_at(plane_len);
            for y in 0..ch {
                for x in 0..cw {
                    let from = y * c_stride + x;
                    let to = (y * cw + x) * 2;
                    out[to] = u[from];
                    out[to + 1] = v[from];
                }
            }
        }
        _ => return None,
    }
    Some(out)
}

/// The integer factor that brings `width` down to at most [`MAX_WIDTH`].
/// Always at least 1, so the usual 640 × 480 frame is untouched.
pub fn scale_factor(width: usize) -> usize {
    if width <= MAX_WIDTH {
        return 1;
    }
    width.div_ceil(MAX_WIDTH)
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

    #[test]
    fn steps_match_the_four_formats() {
        assert_eq!(luma_step(abi::PIX_FMT_GREY), Some(1));
        assert_eq!(luma_step(abi::PIX_FMT_NV12), Some(1));
        assert_eq!(luma_step(abi::PIX_FMT_YU12), Some(1));
        assert_eq!(luma_step(abi::PIX_FMT_YUYV), Some(2));
        assert_eq!(luma_step(0x1234_5678), None);
        assert_eq!(packed_stride(abi::PIX_FMT_YUYV, 640), Some(1280));
        assert_eq!(packed_stride(abi::PIX_FMT_GREY, 640), Some(640));
    }

    /// GREY is the buffer itself, and a stride wider than the row is
    /// honoured rather than assumed away.
    #[test]
    fn grey_with_and_without_padding() {
        let src: Vec<u8> = (0..12).collect();
        assert_eq!(
            luma(&src, 4, 3, 4, 1).unwrap(),
            (0..12).collect::<Vec<u8>>()
        );
        // 4 pixels a row, 6 bytes a row: two bytes of padding.
        let src: Vec<u8> = (0..18).collect();
        assert_eq!(
            luma(&src, 4, 3, 6, 1).unwrap(),
            vec![0, 1, 2, 3, 6, 7, 8, 9, 12, 13, 14, 15]
        );
    }

    /// YUYV takes every second byte from zero: Y0 U0 Y1 V0.
    #[test]
    fn yuyv_takes_every_second_byte() {
        // 2×2, packed: each row is 4 bytes.
        let src = vec![10, 200, 11, 201, 12, 202, 13, 203];
        assert_eq!(luma(&src, 2, 2, 4, 2).unwrap(), vec![10, 11, 12, 13]);
        // The same rows with two bytes of stride padding.
        let src = vec![10, 200, 11, 201, 0, 0, 12, 202, 13, 203, 0, 0];
        assert_eq!(luma(&src, 2, 2, 6, 2).unwrap(), vec![10, 11, 12, 13]);
    }

    /// NV12 and YU12 are the first `width × height` bytes; the chroma
    /// plane that follows is a separate copy.
    #[test]
    fn planar_reads_only_the_luma_plane() {
        // 4×2 luma then a half-height chroma plane of a different value.
        let mut src: Vec<u8> = (0..8).collect();
        src.extend(std::iter::repeat_n(0xFF, 8));
        assert_eq!(luma(&src, 4, 2, 4, 1).unwrap(), (0..8).collect::<Vec<u8>>());
        // A padded luma plane: 4 pixels a row in 8 bytes.
        let mut src: Vec<u8> = Vec::new();
        for row in 0..2u8 {
            src.extend([row * 10, row * 10 + 1, row * 10 + 2, row * 10 + 3]);
            src.extend([0u8; 4]);
        }
        src.extend([0xFFu8; 8]);
        assert_eq!(
            luma(&src, 4, 2, 8, 1).unwrap(),
            vec![0, 1, 2, 3, 10, 11, 12, 13]
        );
    }

    /// A short buffer, a stride narrower than the row and an empty frame
    /// are refused rather than indexed past the end.
    #[test]
    fn refuses_impossible_buffers() {
        assert!(luma(&[0; 4], 4, 2, 4, 1).is_none());
        assert!(luma(&[0; 16], 4, 2, 3, 1).is_none());
        assert!(luma(&[0; 16], 0, 2, 4, 1).is_none());
        assert!(luma(&[0; 16], 4, 0, 4, 1).is_none());
        assert!(luma(&[0; 16], 4, 2, 4, 0).is_none());
        // The last row needs its pixels only, not a whole stride.
        assert!(luma(&[0; 10], 4, 2, 6, 1).is_some());
    }

    /// YUYV carries a U and a V for every pair of pixels of every row;
    /// the chroma a 4:2:0 block wants is the pair from every second row.
    #[test]
    fn yuyv_chroma_takes_every_second_row() {
        // 2x2 pixels: each row is one Y U Y V group of 4 bytes.
        let src = vec![10, 200, 11, 201, 12, 202, 13, 203];
        let uv = chroma(abi::PIX_FMT_YUYV, &src, 2, 2, 4).expect("a colour format");
        assert_eq!(uv, vec![200, 201], "row 0's U and V, row 1's dropped");
        // The same rows with two bytes of stride padding.
        let src = vec![10, 200, 11, 201, 0, 0, 12, 202, 13, 203, 0, 0];
        assert_eq!(
            chroma(abi::PIX_FMT_YUYV, &src, 2, 2, 6).unwrap(),
            vec![200, 201]
        );
    }

    /// NV12's second plane is already the order the core takes, and
    /// YU12's two planes are interleaved into it.
    #[test]
    fn planar_chroma_comes_out_in_nv12_order() {
        // 4x2 luma, then one row of 2 pairs.
        let mut src: Vec<u8> = (0..8).collect();
        src.extend([100, 200, 101, 201]);
        let uv = chroma(abi::PIX_FMT_NV12, &src, 4, 2, 4).expect("a colour format");
        assert_eq!(uv, vec![100, 200, 101, 201]);
        // The same frame as YU12: a U plane of 2 bytes then a V plane.
        let mut src: Vec<u8> = (0..8).collect();
        src.extend([100, 101]);
        src.extend([200, 201]);
        let uv = chroma(abi::PIX_FMT_YU12, &src, 4, 2, 4).expect("a colour format");
        assert_eq!(uv, vec![100, 200, 101, 201]);
        // GREY has no colour, and a buffer with no chroma in it is
        // refused rather than read past.
        assert_eq!(chroma(abi::PIX_FMT_GREY, &src, 4, 2, 4), None);
        assert_eq!(chroma(abi::PIX_FMT_NV12, &src[..8], 4, 2, 4), None);
    }

    #[test]
    fn downscale_chroma_averages_each_block_of_pairs() {
        // 4x4 luma, so 2x2 pairs; a factor of 2 leaves one pair.
        let uv = vec![10, 20, 12, 22, 14, 24, 16, 26];
        let out = downscale_chroma(uv, 4, 4, 2);
        assert_eq!(out, vec![13, 23], "U and V averaged separately");
    }

    #[test]
    fn scale_factor_only_shrinks_what_is_too_wide() {
        assert_eq!(scale_factor(640), 1);
        assert_eq!(scale_factor(MAX_WIDTH), 1);
        assert_eq!(scale_factor(MAX_WIDTH + 1), 2);
        // The Pi camera's 1296x972 mode halves to 648.
        assert_eq!(scale_factor(1296), 2);
        assert_eq!(1296 / scale_factor(1296), 648);
        // A 2592-wide still mode thirds to 864, which is still too wide,
        // so it quarters: 4 is the smallest factor under the cap.
        assert_eq!(scale_factor(2592), 4);
        assert!(2592 / scale_factor(2592) <= MAX_WIDTH);
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
