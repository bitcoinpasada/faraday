use faraday_kernel::{AtLock, Frame};

use crate::Page;

/// The device app's `App` (06 §4.1).
#[derive(AtLock)]
pub struct DeviceApp {
    #[lock(kept)]
    device: faraday_device::State,
    #[lock(public)]
    frame: Frame<Page>,
    /// What Home's search field holds, as the bar's search shows it (03 §6).
    #[lock(public)]
    find: faraday_text::Text,
    #[lock(public)]
    find_focused: bool,
}

impl DeviceApp {
    /// A build of the scope: the device composite and a frame showing Home.
    pub(crate) fn new(device: faraday_device::State) -> DeviceApp {
        DeviceApp {
            device,
            frame: Frame::new(Page::Device(faraday_device::Page::Home)),
            find: faraday_text::Text::default(),
            find_focused: false,
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

    /// The words in the bar's search.
    #[must_use]
    pub fn find(&self) -> &faraday_text::Text {
        &self.find
    }

    /// The typing goes to the bar's search.
    #[must_use]
    pub fn find_focused(&self) -> bool {
        self.find_focused
    }

    pub(crate) fn set_find(&mut self, typed: faraday_text::Text, focused: bool) {
        self.find = typed;
        self.find_focused = focused;
    }

    pub(crate) fn parts(&mut self) -> (&mut faraday_device::State, &mut Frame<Page>) {
        (&mut self.device, &mut self.frame)
    }
}
