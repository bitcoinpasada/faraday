//! Turning a locked NV12 buffer's two planes into the packed 8-bit luma
//! the core decodes and the NV12 chroma it draws the preview from.
//!
//! NV12 (AVFoundation's bi-planar video-range format, and Media
//! Foundation's NV12) already lays its second plane out as interleaved
//! Cb and Cr pairs, one per 2 x 2 block of luma, which is the order the
//! core takes; the only work is the row stride.

use crate::scale::chroma_size;

/// Copies plane 0 (Y) out of an NV12 buffer into a packed `width ×
/// height` buffer. One byte per pixel, so a row is `width` bytes of the
/// `stride` bytes the device lays out.
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

/// Copies plane 1 (UV) out of an NV12 buffer into a packed
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
