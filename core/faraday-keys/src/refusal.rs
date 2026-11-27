//! Why a key or a wallet was refused joining the session: a duplicate, a wrong
//! key, a stick attached, a key the Practice does not take. Moved here from
//! `faraday-intake`'s `wallet` module (`docs/spec/01-layers.md` §2.3), which
//! keeps its own answer for what it parses; the application maps either one to
//! wording.

use faraday_bip::keys::Network;
use faraday_text::Text;

/// Why a key or a wallet was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The words are not a valid BIP-39 mnemonic.
    Words(Text),
    /// The key is already in the session: its fingerprint, which is
    /// public.
    Duplicate(faraday_bip::keys::Fingerprint),
    /// A key for a slot was asked for and this is another key.
    WrongKey {
        /// The fingerprint that was expected.
        wanted: Text,
        /// The fingerprint the words give.
        got: Text,
    },
    /// Words for a wallet's missing key give a key that is none of the
    /// wallet's.
    NotOfWallet {
        /// The fingerprint the words give.
        got: Text,
        /// The wallet's name.
        wallet: Text,
    },
    /// The text is not a wallet this build reads.
    NotAWallet(Text),
    /// The wallet or transaction is on another network than the wallets
    /// already loaded.
    Network {
        /// The network it is on.
        offered: Network,
        /// The network of the loaded wallets.
        loaded: Network,
    },
    /// The demo is on, and this key is not one of its public seeds.
    NotAllowed(Text),
    /// A stick is attached, called this ("stick", "SD card"): keys load
    /// only with none (`docs/SECURITY-MODEL.md`).
    StickAttached(&'static str),
    /// A silent payments wallet whose scan key or spend key is not what
    /// the loaded key with its fingerprint (this one) gives at its
    /// account.
    SilentDiffers(Text),
    /// A silent payments wallet whose record names a fingerprint no loaded
    /// key has, over the scan key and spend key of a loaded key (this
    /// one) that has another.
    SilentOtherKey {
        /// The loaded key's fingerprint.
        key: Text,
        /// The fingerprint the record names.
        record: Text,
    },
    /// A key of the wallet has a loaded key's master fingerprint (this
    /// one) and is not what that key gives at its origin path.
    NotYourKey(Text),
    /// A key of the wallet is under a fingerprint no loaded key has (this
    /// one) and is the extended public key of a key made here and not
    /// backed up yet, at its path.
    NotBackedUp(Text),
    /// A key that is not one of the wallet's spends it alone by the key
    /// path, and the person has not confirmed it yet (F132).
    Unconfirmed,
    /// A Coldcard export holding a single-sig account and a silent
    /// payments account, and the person has not chosen which loads yet.
    AccountUnchosen,
    /// A MuSig2 wallet two of whose keys come from one seed (one master
    /// fingerprint): the MuSig2 check matches one participant to each
    /// master fingerprint, so a second account of one seed never signs.
    MuSigOneSeed,
}
