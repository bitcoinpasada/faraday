//! What a passphrase field takes, from the standard it feeds (`main`'s `pass_field::Kind`).

use faraday_kernel::Public;

/// What a passphrase field takes.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A BIP-39 passphrase: printable ASCII (F155), at most `MAX_PASSPHRASE_BYTES`.
    Bip39,
    /// A Krux code's passphrase being made: printable ASCII, at most `PASSPHRASE_MAX`.
    KefMake,
    /// A Krux code's passphrase being read: any non-control character (the code was made
    /// elsewhere), with no most.
    KefRead,
    /// An aezeed's or a `hsm_secret`'s passphrase: printable ASCII as before, no most (their
    /// standards set none).
    Lightning,
    /// A vault's passphrase: any non-control character, no most (none is set).
    Vault,
    /// A KeePass export's passphrase: as a vault's.
    Kdbx,
    /// A Bitcoin Core wallet's passphrase: any non-control character, 1024 bytes.
    CoreWallet,
}

impl Kind {
    /// Whether the kind takes `c`.
    pub(crate) fn takes(self, c: char) -> bool {
        match self {
            Kind::Bip39 | Kind::KefMake | Kind::Lightning => (' '..='~').contains(&c),
            Kind::KefRead | Kind::Vault | Kind::Kdbx | Kind::CoreWallet => !c.is_control(),
        }
    }

    /// The most bytes the kind holds, where there is a most.
    pub(crate) fn most(self) -> Option<usize> {
        match self {
            Kind::Bip39 => Some(faraday_bip::bip39::MAX_PASSPHRASE_BYTES),
            Kind::KefMake => Some(faraday_keys::kef::PASSPHRASE_MAX),
            Kind::CoreWallet => Some(1024),
            Kind::KefRead | Kind::Lightning | Kind::Vault | Kind::Kdbx => None,
        }
    }
}

/// Why a key was refused.
#[derive(Public, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The character is outside printable ASCII.
    Ascii,
    /// The passphrase is at its most.
    TooLong,
    /// A stick is attached: a passphrase takes no typing.
    Stick,
}
