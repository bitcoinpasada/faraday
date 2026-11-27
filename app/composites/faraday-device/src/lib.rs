//! The device composite: every feature of the device app that is not a shared owner.
//! Section 09 §4.6: the composite's shape (03 §6, 06 §3.1, §4.2).

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod frame;
mod model;
mod msg;
mod page;
mod present;
mod state;
mod update;

use faraday_kernel::{CarryName, JobKind, Public};
use faraday_port::values::Anchor;

pub use frame::FailedModel;
pub use model::{Body, Over};
pub use msg::{Msg, Out, Press};
pub use present::{anchor, body, over};
pub use state::State;
pub use update::Refusal;

/// The feature: `Feature`.
pub struct Device;

/// Its screens (06 §4.2): each a child's own page type.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Keys(faraday_keys_area::Page),
    Wallets(faraday_wallets_area::Page),
    Signing(faraday_signing_area::Page),
    Checks(faraday_checks_area::Page),
    Tools(faraday_tools_area::Page),
    Files(faraday_files_area::Page),
}

impl Page {
    /// The page the Check destination opens.
    pub const CHECK: Page = Page::Checks(faraday_checks_area::Page::Check);
    /// The page the Wallets destination opens.
    pub const WALLETS: Page = Page::Wallets(faraday_wallets_area::Page::WalletsStart);
    /// The page the Files destination opens.
    pub const FILES: Page = Page::Files(faraday_files_area::Page::Files);
    /// The page the Tools destination opens.
    pub const TOOLS: Page = Page::Tools(faraday_tools_area::Page::Catalog);
}

pub const JOBS: &[JobKind] = &[];
pub const CARRY: &[CarryName] = &[];
pub const ANCHORS: &[Anchor] = &[];
