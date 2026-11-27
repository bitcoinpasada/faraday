//! The NV12 planes read into luma and chroma.

/// The luma plane, row by row without the stride's padding.
#[must_use]
pub fn luma(_plane: &[u8], _width: usize, _height: usize, _stride: usize) -> Option<Vec<u8>> {
    todo!()
}

/// The interleaved UV plane, row by row without the stride's padding.
#[must_use]
pub fn chroma(_plane: &[u8], _width: usize, _height: usize, _stride: usize) -> Option<Vec<u8>> {
    todo!()
}
