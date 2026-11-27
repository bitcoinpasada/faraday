//! The command line (S6: no event is made here).

use faraday_starter_product::StarterOptions;
use faraday_window::WindowOptions;

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
