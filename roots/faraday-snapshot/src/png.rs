//! The picture written at each step, with no metadata.

use std::io;
use std::path::Path;

/// Writes `rgba` (`width` by `height`) as a PNG with no text, time or colour-profile chunk.
pub fn write(_path: &Path, _width: u32, _height: u32, _rgba: &[u8]) -> io::Result<()> {
    todo!()
}
