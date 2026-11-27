//! The command line (S6: no event is made here).

use faraday_starter_product::StarterOptions;
use faraday_window::WindowOptions;

// Spec: 09 §7 names `options.rs` and 06 §5.3 the flags: `--uninstall` and `--image` reach the
// product, `--downloads` the desktop adapter, `--size` and `--timings` the window.
pub struct Options {
    pub product: StarterOptions,
    pub window: WindowOptions,
    pub sticks: String,
    pub downloads: Option<String>,
}

#[must_use]
pub fn parse() -> Options {
    todo!()
}
