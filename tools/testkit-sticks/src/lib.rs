//! A tool or test-support crate: writes the test kit's sticks (09 §8). The backup stick holds
//! what `faraday-backup` writes.

use std::io;
use std::path::Path;

/// Writes the test kit's sticks, one folder each, under `out`.
pub fn write(_out: &Path) -> io::Result<()> {
    todo!()
}
