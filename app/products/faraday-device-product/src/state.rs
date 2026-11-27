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

impl DeviceApp {
    /// A build of the scope: the device composite and a frame showing Home.
    pub(crate) fn new(device: faraday_device::State) -> DeviceApp {
        DeviceApp {
            device,
            frame: Frame::new(Page::Device(faraday_device::Page::Home)),
        }
    }

    #[must_use]
    pub fn device(&self) -> &faraday_device::State {
        &self.device
    }

    #[must_use]
    pub fn frame(&self) -> &Frame<Page> {
        &self.frame
    }

    pub(crate) fn parts(&mut self) -> (&mut faraday_device::State, &mut Frame<Page>) {
        (&mut self.device, &mut self.frame)
    }
}
