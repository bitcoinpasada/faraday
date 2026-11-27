//! The reduced copy a preview and a pass read.

/// The whole factor a frame of this size is reduced by for the preview.
#[must_use]
pub fn reduce_factor(_width: usize, _height: usize) -> usize {
    todo!()
}

/// The box average of every `factor` by `factor` block: the new width, height and luma.
#[must_use]
pub fn reduce(_width: usize, _height: usize, _luma: &[u8]) -> (u16, u16, Vec<u8>) {
    todo!()
}

/// The chroma plane reduced by `factor`.
#[must_use]
pub fn reduce_chroma(_width: usize, _height: usize, _chroma: &[u8], _factor: usize) -> Vec<u8> {
    todo!()
}
