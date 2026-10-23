//! A vault's key and wallet, shown and saved (`docs/BACKUP-FLOW.md`
//! §17.7): a key record's SeedQR behind the hold that shows its words,
//! a wallet record's descriptor as the Wallets page shows it, and Save to
//! a stick for either, so that the picture and text forms need not be
//! kept in the vault. Nothing here is stored in the vault: what is made
//! goes to the Outbox, for the stick, as Files' files do.
//!
//! Its presses are the vault's glue, in `vault/mod.rs`.

/// What a file of the popup is made from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveWhat {
    /// The key's words, one line.
    Words,
    /// The key's SeedQR, as a labelled picture.
    SeedQr,
    /// A wallet's public files, already made.
    Public,
}

/// One line of the popup: a form, the files it is written as, ticked or
/// not.
pub struct SaveFile {
    /// The form, as the popup names it.
    pub label: String,
    /// What it is made from.
    pub what: SaveWhat,
    /// The files' names.
    pub names: Vec<String>,
    /// A wallet's files, made when the popup opened: public, so kept
    /// until it closes. Empty for a seed's, which are made at Save.
    pub made: Vec<(String, Vec<u8>)>,
    /// To be saved.
    pub ticked: bool,
}

/// Save to a stick's popup.
pub struct StickSave {
    /// The vault record it is for, by index into the open vault's records.
    pub record: usize,
    /// The record is a key: the popup says what the stick gives whoever
    /// finds it.
    pub seed: bool,
    /// Its lines.
    pub files: Vec<SaveFile>,
}
