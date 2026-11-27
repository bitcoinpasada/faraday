//! The command line (S6: no event is made here).

use faraday_stick_adapter::StickOptions;

// Spec: 09 §7 names `options.rs` and 06 §5.1 the flags (`--fb`, `--size`, `--depth`, `--dpi`,
// and the rest); the storage flags are the two of `Storage::disk_process`.
/// What the command line gives: the stick adapter's options and the disk process's two paths.
pub struct Options {
    pub stick: StickOptions,
    pub fifo: String,
    pub ram_dir: String,
}

#[must_use]
pub fn parse() -> Options {
    todo!()
}
