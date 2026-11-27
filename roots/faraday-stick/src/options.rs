//! The command line (S6: no event is made here).

use faraday_stick_adapter::StickOptions;

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
