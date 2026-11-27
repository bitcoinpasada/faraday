use faraday_kernel::{AtLock, Frame};

use crate::Page;

/// The device app's `App` (06 §4.1).
#[derive(AtLock)]
pub struct DeviceApp {
    #[lock(kept)]
    device: faraday_device::State,
    #[lock(public)]
    frame: Frame<Page>,
}
