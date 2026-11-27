//! The command line (S6: no event is made here).

use faraday_demo_product::DemoOptions;
use faraday_window::WindowOptions;

// Spec: 09 §7 names `options.rs` and 06 §5.2 the flags: `--text` and `--transfer` reach the
// product, `--sticks` and `--downloads` the desktop adapter, the rest the window.
pub struct Options {
    pub product: DemoOptions,
    pub window: WindowOptions,
    pub sticks: String,
    pub downloads: Option<String>,
}

#[must_use]
pub fn parse() -> Options {
    todo!()
}
