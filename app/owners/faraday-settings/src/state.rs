use core::borrow::Borrow;

use faraday_kernel::AtLock;
use faraday_kernel::settings::Settings;

#[derive(AtLock)]
pub struct State {
    #[lock(public)]
    skeleton: (),
}

impl Borrow<Settings> for State {
    fn borrow(&self) -> &Settings {
        todo!()
    }
}
