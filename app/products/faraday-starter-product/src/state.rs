use faraday_kernel::{AtLock, Frame};

/// The Starter's `Outside` (06 §4.1).
#[derive(AtLock)]
pub struct StarterApp {
    #[lock(kept)]
    starter: faraday_starter::State,
    #[lock(public)]
    frame: Frame<faraday_starter::Page>,
}
