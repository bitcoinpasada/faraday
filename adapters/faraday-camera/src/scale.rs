//! Scaling a frame down by a whole factor.

/// The chroma plane's size for a luma of `width` by `height`.
#[must_use]
pub fn chroma_size(_width: usize, _height: usize) -> (usize, usize) {
    todo!()
}

/// The whole factor that brings `width` to `max_side` or under.
#[must_use]
pub fn scale_factor(_width: usize, _height: usize, _max_side: usize) -> usize {
    todo!()
}

/// The luma scaled down by `factor`: the new width, height and plane.
#[must_use]
pub fn downscale(
    _luma: Vec<u8>,
    _width: usize,
    _height: usize,
    _factor: usize,
) -> (usize, usize, Vec<u8>) {
    todo!()
}

/// The chroma plane scaled down by `factor`.
#[must_use]
pub fn downscale_chroma(
    _chroma: Vec<u8>,
    _width: usize,
    _height: usize,
    _factor: usize,
) -> Vec<u8> {
    todo!()
}
