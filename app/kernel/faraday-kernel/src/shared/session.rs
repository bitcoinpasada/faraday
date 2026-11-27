//! The session's reads and changes: the key store, wallets, what waits, Inbox and Outbox, the
//! session's randomness (03 §7.1 to §7.3).

use faraday_bip::keys::{Fingerprint, Network};
use faraday_bip::policy::WalletPolicy;
use faraday_keys::{Keys, Refusal};
use faraday_port::bytes::{Bytes, Entropy, EntropyBytes};
use faraday_port::storage::Sha256;
use faraday_text::{SecretText, Text};

use super::change;
use crate::cx::Edits;
use crate::handle::{FileId, KeyId, WalletId};

pub trait Read {
    /// Reveals are `faraday-keys`' (K-1).
    fn keys(&self) -> &Keys;
    /// Fingerprint, name, held, sample, made online.
    fn key(&self, id: KeyId) -> Option<KeyView<'_>>;
    /// Public data only.
    fn wallets(&self) -> &[WalletView];
    fn wallet(&self, id: WalletId) -> Option<&WalletView>;
    /// PSBTs to sign, keys held for backup.
    fn waiting(&self) -> &[WaitingItem];
    fn inbox(&self) -> &[FileView];
    fn outbox(&self) -> &[FileView];
    fn seeded(&self) -> bool;
    /// Counts every change to what the session holds; a memo's key (04, V13).
    fn edits(&self) -> Edits;
}

/// What the session shows of a key: public data only.
pub struct KeyView<'a> {
    pub fingerprint: Fingerprint,
    pub name: &'a Text,
    pub held: bool,
    pub sample: bool,
    pub made_online: bool,
}

/// Public data only: `main`'s `Wallet` holds no secret.
pub struct WalletView {
    pub id: WalletId,
    pub name: Text,
    pub policy: WalletPolicy,
    pub origin: WalletOrigin,
    pub source: WalletSource,
    pub checked: bool,
    pub network: Network,
}

/// Where a wallet's keys come from.
// Spec: 03 §7.2 lists `origin` and `source` in `WalletView` without their types.
pub struct WalletOrigin(());

/// How a wallet came in.
// Spec: 03 §7.2 lists `origin` and `source` in `WalletView` without their types.
pub struct WalletSource(());

/// A file held: its name, kind and whether it holds a secret; never its bytes or any text read
/// from them.
// Spec: 03 §7.2 describes `FileView` in prose.
pub struct FileView {
    pub id: FileId,
    pub name: Text,
    pub kind: FileKind,
    pub secret: bool,
}

// Spec: 03 §7.2 says `FileView` has a kind without shaping it.
pub struct FileKind(());

/// One thing the session holds until it is dealt with.
// Spec: 03 §7.2 writes this type `Waiting`, which `handle` gives to the ticket holder (§8.1).
pub enum WaitingItem {
    /// A PSBT waiting to be signed.
    Psbt(WalletId),
    /// A key held until its backup is written.
    Held(KeyId),
}

pub enum Changed {
    KeyCame(KeyId),
    KeyGone(KeyId),
    WalletCame(WalletId),
    WalletGone(WalletId),
    FileCame(FileId),
    FileGone(FileId),
    HeldReleased(KeyId),
    Network(Network),
}

/// A wallet of several keys comes in only through the sheet that shows its keys (F254).
// Spec: 03 §7.3 names `Confirm` without a shape.
pub struct Confirm(());

pub enum AddRefusal {
    Unconfirmed(Confirm),
    Refused(Refusal),
}

/// A key's input type: words, seed, xprv, share set.
// Spec: 03 §7.3 says `faraday-keys`' input type and names none.
pub struct KeyInput(());

/// The spend's MuSig2 round; its secret nonces move in and back.
// Spec: 03 §7.3 names the round without a type.
pub struct MusigRound(());

/// `Draw`'s answer when the seed never came.
pub struct NoRandomness;

/// What `Sign` is answered with: the PSBT and the round, moved back.
// Spec: 03 §7.3 says "the PSBT and the round, moved back".
pub struct Signed {
    pub psbt: Bytes,
    pub round: Option<MusigRound>,
}

pub struct AddKey {
    pub key: KeyInput,
    pub passphrase: Option<SecretText>,
}

pub struct RemoveKey(pub KeyId);

pub struct RemoveAll;

pub struct AddWallet {
    pub policy: WalletPolicy,
    pub name: Option<Text>,
    pub source: WalletSource,
}

pub struct ConfirmWallet(pub Confirm);

pub struct RemoveWallet(pub WalletId);

pub struct RenameWallet {
    pub wallet: WalletId,
    pub name: Text,
}

pub struct MarkChecked(pub WalletId);

pub struct Follow(pub Network);

pub struct Sign {
    pub wallet: WalletId,
    pub psbt: Bytes,
    pub round: Option<MusigRound>,
}

pub struct Draw {
    /// What the bytes are for.
    pub what: &'static str,
    pub n: usize,
}

/// The adapter's entropy.
pub struct Seed(pub Entropy);

/// The person's own accumulator.
pub struct AddOwn(pub EntropyBytes);

pub struct PutOutbox {
    pub name: Text,
    pub bytes: Bytes,
}

pub struct TakeFile(pub FileId);

pub struct RemoveFile(pub FileId);

pub struct RenameFile {
    pub file: FileId,
    pub name: Text,
}

pub struct MarkWritten {
    pub file: FileId,
    pub sha: Sha256,
}

/// A key and its copy written.
pub struct Hold(pub KeyId);

pub struct Release(pub KeyId);

pub enum Change {
    AddKey(AddKey),
    RemoveKey(RemoveKey),
    RemoveAll(RemoveAll),
    AddWallet(AddWallet),
    ConfirmWallet(ConfirmWallet),
    RemoveWallet(RemoveWallet),
    RenameWallet(RenameWallet),
    MarkChecked(MarkChecked),
    Follow(Follow),
    Sign(Sign),
    Draw(Draw),
    Seed(Seed),
    AddOwn(AddOwn),
    PutOutbox(PutOutbox),
    TakeFile(TakeFile),
    RemoveFile(RemoveFile),
    RenameFile(RenameFile),
    MarkWritten(MarkWritten),
    Hold(Hold),
    Release(Release),
}

pub enum Done {
    AddKey(Result<KeyId, Refusal>),
    RemoveKey(Result<(), Refusal>),
    RemoveAll(Result<(), Refusal>),
    AddWallet(Result<WalletId, AddRefusal>),
    ConfirmWallet(Result<WalletId, Refusal>),
    RemoveWallet(Result<(), Refusal>),
    RenameWallet(Result<(), Refusal>),
    MarkChecked(Result<(), Refusal>),
    Follow(Result<(), Refusal>),
    Sign(Result<Signed, Refusal>),
    Draw(Result<EntropyBytes, NoRandomness>),
    Seed(()),
    AddOwn(()),
    PutOutbox(Result<FileId, Refusal>),
    TakeFile(Result<Bytes, Refusal>),
    RemoveFile(Result<(), Refusal>),
    RenameFile(Result<(), Refusal>),
    MarkWritten(Result<(), Refusal>),
    Hold(()),
    Release(()),
}

change!(AddKey => Result<KeyId, Refusal>);
change!(RemoveKey => Result<(), Refusal>);
change!(RemoveAll => Result<(), Refusal>);
change!(AddWallet => Result<WalletId, AddRefusal>);
change!(ConfirmWallet => Result<WalletId, Refusal>);
change!(RemoveWallet => Result<(), Refusal>);
change!(RenameWallet => Result<(), Refusal>);
change!(MarkChecked => Result<(), Refusal>);
change!(Follow => Result<(), Refusal>);
change!(Sign => Result<Signed, Refusal>);
change!(Draw => Result<EntropyBytes, NoRandomness>);
change!(Seed => ());
change!(AddOwn => ());
change!(PutOutbox => Result<FileId, Refusal>);
change!(TakeFile => Result<Bytes, Refusal>);
change!(RemoveFile => Result<(), Refusal>);
change!(RenameFile => Result<(), Refusal>);
change!(MarkWritten => Result<(), Refusal>);
change!(Hold => ());
change!(Release => ());
