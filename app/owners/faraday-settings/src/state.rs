use core::borrow::Borrow;

use faraday_kernel::AtLock;
use faraday_kernel::settings::Settings;

/// The person's choices as the owner holds them.
#[derive(AtLock)]
pub struct State {
    #[lock(public)]
    settings: Settings,
}

impl State {
    /// The first start: the defaults, until the settings file is read.
    pub(crate) fn new(settings: Settings) -> State {
        State { settings }
    }
}

impl Borrow<Settings> for State {
    fn borrow(&self) -> &Settings {
        &self.settings
    }
}
