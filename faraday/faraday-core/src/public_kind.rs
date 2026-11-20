//! The kinds of file Faraday writes for a wallet, one row each: what the
//! file is, the wallet app that reads it (each variant's comment), and
//! whether making it is a job on the worker ([`ROWS`]). Another wallet
//! app's file is one new variant and row here and its making in
//! [`crate::public_files::files`].
//!
//! The numbers are the kinds' wire values, carried by `Action::PublicOut`,
//! `Action::PublicRemove` and `Action::BOut`; they do not change.

/// A kind of public file.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub(crate) enum PublicFile {
    /// The descriptor as text; any wallet app.
    Descriptor = 1,
    /// The multisig config; Coldcard, Keystone and Passport.
    MultisigConfig = 2,
    /// The backup sheet as a PDF; to print.
    BackupSheet = 3,
    /// The split shares, their sheets and pictures; Faraday's own.
    SplitShares = 4,
    /// The wallet's `.json`; Sparrow.
    WalletJson = 5,
    /// The BIP 129 descriptor record; Nunchuk.
    Bsms = 6,
    /// The `importdescriptors` file; Bitcoin Core.
    CoreImport = 7,
    /// The descriptor's code as a labelled picture; any wallet app.
    DescriptorQr = 8,
    /// The import for Bitcoin Core before 29.
    CoreBefore29 = 9,
    /// The wallet file; Electrum.
    Electrum = 10,
    /// The config; Caravan.
    Caravan = 11,
    /// The `addwallet` command; Specter DIY.
    SpecterDiy = 12,
    /// The wallet file; Wasabi.
    Wasabi = 13,
    /// The `addwallet` command's codes as labelled pictures; Specter DIY.
    SpecterDiyQr = 14,
    /// A silent payments wallet's BIP-392 descriptor: not written here (it
    /// carries the scan key and goes through the secret sheet).
    SpDescriptor = 15,
}

/// One kind's row.
struct Row {
    kind: PublicFile,
    /// Making it rebuilds a wallet to check it, or makes several pictures or
    /// PDFs: a job on the worker when a person asks.
    slow: bool,
}

const fn row(kind: PublicFile, slow: bool) -> Row {
    Row { kind, slow }
}

/// Every kind.
const ROWS: [Row; 15] = [
    row(PublicFile::Descriptor, false),
    row(PublicFile::MultisigConfig, true),
    row(PublicFile::BackupSheet, false),
    row(PublicFile::SplitShares, true),
    row(PublicFile::WalletJson, false),
    row(PublicFile::Bsms, false),
    row(PublicFile::CoreImport, false),
    row(PublicFile::DescriptorQr, false),
    row(PublicFile::CoreBefore29, false),
    row(PublicFile::Electrum, false),
    row(PublicFile::Caravan, false),
    row(PublicFile::SpecterDiy, false),
    row(PublicFile::Wasabi, false),
    row(PublicFile::SpecterDiyQr, true),
    row(PublicFile::SpDescriptor, false),
];

impl PublicFile {
    /// The kind a wire value names; `None` for a value that names none.
    pub(crate) fn from_wire(n: u8) -> Option<Self> {
        ROWS.iter().map(|r| r.kind).find(|k| *k as u8 == n)
    }

    /// The wire value of the kind.
    pub(crate) fn wire(self) -> u8 {
        self as u8
    }

    /// Whether a person's press on the kind is a job on the worker.
    pub(crate) fn slow(self) -> bool {
        ROWS.iter().any(|r| r.kind == self && r.slow)
    }
}
