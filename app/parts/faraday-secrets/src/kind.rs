//! Who may read a file, and what secret a text is (`main`'s `secrets::Exposure` and
//! `scan_read::Kind::secret`).

use faraday_kernel::session::FileKind;

/// Who may read a file.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Exposure {
    /// Anyone: public keys, PSBTs, signed messages, certificates.
    Public,
    /// Only who has the passphrase: a vault, a KDBX database or a Krux encrypted code.
    Sealed,
    /// Anyone who has the file can use the secret in it.
    Secret,
}

/// Who may read a file of this kind.
pub fn exposure(kind: FileKind) -> Exposure {
    match kind {
        FileKind::Vault | FileKind::Kdbx | FileKind::KdbxOther | FileKind::KruxCode => {
            Exposure::Sealed
        }
        FileKind::Entries
        | FileKind::Words
        | FileKind::SeedPart
        | FileKind::KeyLine
        | FileKind::SecretWallet
        | FileKind::KeyWithSecret
        | FileKind::CoreWallet => Exposure::Secret,
        FileKind::Psbt
        | FileKind::Wallet
        | FileKind::Transaction
        | FileKind::Sheet
        | FileKind::Message
        | FileKind::Key
        | FileKind::Share
        | FileKind::EfiImage
        | FileKind::Pdf
        | FileKind::Text
        | FileKind::Gpg
        | FileKind::Other => Exposure::Public,
    }
}

/// A file of this kind may hold a secret: the secret kinds, and text or a file Faraday does not
/// read, which it cannot tell. A lock drops these from the Inbox (`docs/VAULT.md`).
pub fn may_be_secret(kind: FileKind) -> bool {
    matches!(
        kind,
        FileKind::Words
            | FileKind::SeedPart
            | FileKind::KeyLine
            | FileKind::SecretWallet
            | FileKind::KeyWithSecret
            | FileKind::KruxCode
            | FileKind::Entries
            | FileKind::CoreWallet
            | FileKind::Text
            | FileKind::Other
    )
}

/// The secret a text is or holds: the kinds of `main`'s `scan_read::Kind` that `Kind::secret`
/// names, and `Wallet` for a wallet file holding a secret.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SecretKind {
    SeedWords,
    SeedQr,
    CompactSeedQr,
    Codex32Secret,
    Codex32Share,
    Slip39Share,
    BadChecksum,
    Aezeed,
    Wif,
    Xprv,
    UrSeed,
    Nostr,
    SpendKey,
    Paperkey,
    Totp,
    Entries,
    Wallet,
}
