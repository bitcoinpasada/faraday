//! The device app's product crate: the owners, the device app (its frame and the device
//! composite), and the runtime's `Root` for it.
//! Section 09 §4.8: the product's shape (06 §2, §4, §5.1).

#![expect(dead_code, reason = "the fields are read once the bodies are written")]

mod l1;
mod model;
mod msg;
mod present;
mod root;
mod state;
mod update;

use faraday_kernel::Public;

pub use model::{Body, Over};
pub use msg::{AppPress, Msg, Press};
pub use root::{Device, MODES, NoOptions};
pub use state::DeviceApp;
pub use update::Refusal;

/// The app: `Feature<State = DeviceApp, Out = Top>`.
pub struct App;

/// Its pages (06 §4.2).
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Page {
    Device(faraday_device::Page),
    Vaults(faraday_vaults::Page),
    Settings(faraday_settings::Page),
    SelfTestFailed,
}
