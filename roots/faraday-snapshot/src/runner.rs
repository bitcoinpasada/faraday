//! The driver and its command line.

use std::process::ExitCode;

use faraday_fixture_adapters::Fixtures;

/// Which product's model a run builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Product {
    Device,
    Demo,
    Starter,
}

// Spec: 09 §7 names `runner` ("the driver and its command line") without a shape; the
// command is the product, the size, the tour and where the pictures go.
/// What the command line asks of one run.
pub struct Command {
    pub product: Product,
    pub width: u16,
    pub height: u16,
    pub dpi: u16,
    /// The tour's name; the default tour when `None`.
    pub tour: Option<String>,
    pub out: String,
}

/// One tour's run: the runtime over the fixture adapters, and the pictures taken.
pub struct Tour {
    pub fixtures: Fixtures,
}

#[must_use]
pub fn parse() -> Command {
    todo!()
}

pub fn run(_command: &Command) -> ExitCode {
    todo!()
}
