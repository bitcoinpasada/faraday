//! The shared owners: the session, the vaults and the settings, read through `View`'s traits and
//! changed only by a change sent to its owner (03 §7).

use crate::feature::Feature;
use crate::message::Public;

pub mod session;
pub mod settings;
pub mod vaults;

/// Gives a change's own type its `From` into `AnyChange` and its `shared::Change` (every body
/// `todo!()` until the owners are written).
macro_rules! change {
    ($change:ty => $done:ty) => {
        impl From<$change> for $crate::shared::AnyChange {
            fn from(_change: $change) -> $crate::shared::AnyChange {
                todo!()
            }
        }

        impl $crate::shared::Change for $change {
            type Done = $done;

            fn take(_done: $crate::shared::Done) -> Result<Self::Done, $crate::shared::Done> {
                todo!()
            }

            fn fault() -> Self::Done {
                todo!()
            }
        }
    };
}
pub(crate) use change;

/// Every change's own type implements this; the mirror of `port::Request`.
pub trait Change: Into<AnyChange> {
    type Done;
    fn take(done: Done) -> Result<Self::Done, Done>;
    fn fault() -> Self::Done;
}

pub enum AnyChange {
    Session(session::Change),
    Vaults(vaults::Change),
    Settings(settings::Change),
}

pub enum Done {
    Session(session::Done),
    Vaults(vaults::Done),
    Settings(settings::Done),
}

/// What a shared owner says came, went or changed; broadcast as `Notice::Shared`.
pub enum Changed {
    Session(session::Changed),
    Vaults(vaults::Changed),
    Settings(settings::Changed),
}

/// Minted by the runtime, as `AskId` is.
#[derive(Public, Copy, PartialEq, Eq)]
pub struct ChangeId {
    pub generation: u32,
    pub n: u32,
}

/// A shared owner: its `Msg` can carry a change it is given.
pub trait Owner: Feature {
    fn given(id: ChangeId, change: AnyChange) -> Self::Msg;
}

pub use settings::Settings;
