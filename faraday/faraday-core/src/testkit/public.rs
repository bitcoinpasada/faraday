//! The test kit's key texts for a master key a test holds. Everything else
//! the kit makes that is plain text is in `plain.rs`.

use faraday_bip::keys::MasterKey;
use faraday_text::Text;

use super::KitError;

/// A key kind's key text for a master key a test holds
/// (`Keys::master_for_tests`); the session's keys go through
/// [`crate::wallet::KeyTexts`]. Bring it in with `use
/// faraday_core::testkit::KindText`.
pub trait KindText {
    /// Costs a key derivation. `[fingerprint/path]xpub` of `master` at the
    /// kind's path, account 0.
    fn key_text(
        self,
        c: &faraday_crypto::work::Compute,
        master: &MasterKey,
    ) -> Result<Text, KitError>;

    /// Costs a key derivation. `[fingerprint/path]xpub` of `master` at the
    /// kind's path, at `account`.
    fn key_text_at(
        self,
        c: &faraday_crypto::work::Compute,
        master: &MasterKey,
        account: u32,
    ) -> Result<Text, KitError>;
}

impl KindText for crate::create::NewKind {
    fn key_text(
        self,
        c: &faraday_crypto::work::Compute,
        master: &MasterKey,
    ) -> Result<Text, KitError> {
        self.key_text_at(c, master, 0)
    }

    fn key_text_at(
        self,
        c: &faraday_crypto::work::Compute,
        master: &MasterKey,
        account: u32,
    ) -> Result<Text, KitError> {
        Ok(faraday_keys::texts::key_text(
            c,
            master,
            &self.spec(master.network(), account)?,
        )?)
    }
}
