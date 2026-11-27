//! The shared owners: the session, the vaults and the settings, read through `View`'s traits and
//! changed only by a change sent to its owner (03 §7).

use faraday_bip::keys::Network;
use faraday_keys::Keys;
use faraday_keys::vault::{OpenVaults, VaultId};

use crate::cx::Edits;
use crate::feature::Feature;
use crate::handle::{KeyId, WalletId};
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
    #[expect(
        clippy::result_large_err,
        reason = "the kernel's enums carry the domain's real types, which differ in size; a value is moved once"
    )]
    fn take(done: Done) -> Result<Self::Done, Done>;
    fn fault() -> Self::Done;
}

#[expect(
    clippy::large_enum_variant,
    reason = "the kernel's enums carry the domain's real types, which differ in size; a value is moved once"
)]
pub enum AnyChange {
    Session(session::Change),
    Vaults(vaults::Change),
    Settings(settings::Change),
}

#[expect(
    clippy::large_enum_variant,
    reason = "the kernel's enums carry the domain's real types, which differ in size; a value is moved once"
)]
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

/// The empty readers (A19): what a part is lent where it has no session or vaults to read, and
/// what an owner is lent in its own slot (05 R13, R26). It holds nothing and changes never.
pub struct Empty {
    keys: Keys,
    open: OpenVaults,
}

impl Empty {
    pub fn new() -> Empty {
        Empty {
            keys: Keys::on(Network::Mainnet),
            open: OpenVaults::default(),
        }
    }
}

impl Default for Empty {
    fn default() -> Empty {
        Empty::new()
    }
}

impl session::Read for Empty {
    fn keys(&self) -> &Keys {
        &self.keys
    }

    fn key(&self, _id: KeyId) -> Option<session::KeyView<'_>> {
        None
    }

    fn wallets(&self) -> &[session::WalletView] {
        &[]
    }

    fn wallet(&self, _id: WalletId) -> Option<&session::WalletView> {
        None
    }

    fn waiting(&self) -> &[session::WaitingItem] {
        &[]
    }

    fn inbox(&self) -> &[session::FileView] {
        &[]
    }

    fn outbox(&self) -> &[session::FileView] {
        &[]
    }

    fn seeded(&self) -> bool {
        false
    }

    fn edits(&self) -> Edits {
        Edits::NONE
    }
}

impl vaults::Read for Empty {
    fn open(&self) -> &OpenVaults {
        &self.open
    }

    fn current(&self) -> Option<VaultId> {
        None
    }

    fn edits(&self) -> Edits {
        Edits::NONE
    }
}
