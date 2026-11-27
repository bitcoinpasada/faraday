//! A vault longer than a frame is read from a stick in pieces; any other
//! file that long is refused, and the frame keeps its limit.

use faraday_files::proto::{MAX_FRAME, PIECE, Response};

#[test]
fn a_piece_fits_a_frame_whose_limit_is_unchanged() {
    let piece = Response::Part {
        total: 1 << 28,
        bytes: vec![0; PIECE],
    }
    .encode();
    assert!(piece.len() as u32 <= MAX_FRAME);
}
