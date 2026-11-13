//! What Faraday Demo's host strip offers for an app, as the tests of the
//! app's screens ask it.

use faraday_core::{Action, Faraday};
use faraday_host::Host;

/// What Faraday Demo's host strip offers for `app` as it stands, its
/// demo aids among them (`docs/DESKTOP-HOST.md` §3.1): the app held by
/// the host for one frame and given back.
pub fn strip(app: &mut Faraday) -> Vec<String> {
    Host::strip_for(app)
        .iter()
        .map(|t| String::from(&**t))
        .collect()
}

/// Whether Faraday Demo's host strip offers the press `a` for `app` as
/// it stands (`docs/DESKTOP-HOST.md` §3.1).
pub fn strip_offers(app: &mut Faraday, a: Action) -> bool {
    Host::strip_actions_for(app).contains(&a)
}
