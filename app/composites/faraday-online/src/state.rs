use faraday_kernel::{AtLock, Frame};

use crate::Page;

#[derive(AtLock)]
pub struct State {
    #[lock(kept)]
    device: faraday_device::State,
    #[lock(kept)]
    strip: faraday_strip::State,
    #[lock(kept)]
    transfer: faraday_transfer::State,
}

/// Faraday Demo's `App`, and the Starter's inside its lock scope (06 §4.1).
#[derive(AtLock)]
pub struct OnlineApp {
    #[lock(kept)]
    online: State,
    #[lock(public)]
    frame: Frame<Page>,
}
