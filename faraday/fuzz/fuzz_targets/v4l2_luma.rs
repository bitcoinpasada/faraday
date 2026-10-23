//! The V4L2 shell's frame converters over what a driver hands back: a
//! pixel format, a width, a height and a stride, all the driver's word,
//! then a buffer of any length. `luma`, `chroma` and the two downscales
//! turn it into the frame the scanner decodes; the decoder under them is
//! `decode_luma` in the root `fuzz/`. The macOS and Windows shells have
//! converters of the same shape that this does not cover.

#![no_main]

#[path = "../../shells/v4l2/src/abi.rs"]
#[allow(dead_code, missing_docs, unsafe_code)]
mod abi;
#[path = "../../shells/v4l2/src/convert.rs"]
#[allow(dead_code, missing_docs)]
mod convert;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Some((head, buffer)) = data.split_at_checked(7) else {
        return;
    };
    let format = abi::FORMATS[usize::from(head[0]) % abi::FORMATS.len()];
    let width = usize::from(u16::from_le_bytes([head[1], head[2]]));
    let height = usize::from(u16::from_le_bytes([head[3], head[4]]));
    let stride = usize::from(u16::from_le_bytes([head[5], head[6]]));
    let stride = if stride == 0 {
        convert::packed_stride(format, width).unwrap_or(0)
    } else {
        stride
    };
    let Some(step) = convert::luma_step(format) else {
        return;
    };
    let luma = convert::luma(buffer, width, height, stride, step);
    let chroma = convert::chroma(format, buffer, width, height, stride);
    let factor = convert::scale_factor(width);
    if let Some(luma) = luma {
        let _ = convert::downscale(luma, width, height, factor);
    }
    if let Some(chroma) = chroma {
        let _ = convert::downscale_chroma(chroma, width, height, factor);
    }
});
