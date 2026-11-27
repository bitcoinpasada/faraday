//! The command line (S6: no event is made here).

use faraday_demo_product::DemoOptions;
use faraday_window::WindowOptions;

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
