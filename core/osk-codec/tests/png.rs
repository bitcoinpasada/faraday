//! A picture written as a PNG reads back, through the decoder the
//! shells and the disk process use, as the same pixels.

use osk_codec::png::grey_png;

/// The picture's size and its greyscale bytes, as a PNG decoder reads
/// them.
fn decoded(file: &[u8]) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(file));
    let mut reader = decoder.read_info().expect("a PNG");
    let mut buf = vec![0; reader.output_buffer_size().expect("a size")];
    let info = reader.next_frame(&mut buf).expect("its pixels");
    assert_eq!(info.color_type, png::ColorType::Grayscale);
    assert_eq!(info.bit_depth, png::BitDepth::Eight);
    buf.truncate(info.buffer_size());
    (info.width, info.height, buf)
}

#[test]
fn a_grey_picture_reads_back_as_its_pixels() {
    // Wide enough that the rows run past one stored block.
    let (w, h) = (301, 257);
    let pixels: Vec<u8> = (0..w * h).map(|i| (i * 7 % 256) as u8).collect();
    let (dw, dh, got) = decoded(&grey_png(w, h, &pixels));
    assert_eq!((dw, dh), (w as u32, h as u32));
    assert_eq!(got, pixels);
}

#[test]
fn pixels_a_short_buffer_lacks_are_white() {
    let (dw, dh, got) = decoded(&grey_png(3, 2, &[0, 10, 20, 30]));
    assert_eq!((dw, dh), (3, 2));
    assert_eq!(got, [0, 10, 20, 30, 255, 255]);
}
