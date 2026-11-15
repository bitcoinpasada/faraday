//! The seed My seed makes my wallet holds (`docs/FLOWS.md`): the words
//! typed or scanned and their BIP-39 passphrase, held for the check alone
//! as the Sweep holds its private key ([`crate::sweep`]): one at most, in
//! a slot of its own beside the session's keys, never one of them, used
//! only through the public reads below, and erased by
//! [`Keys::check_forget`] and with the session's keys. Only
//! [`Keys::check_keep`] makes it one of the keys, as Add a key does.
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use faraday_bip::bitcoin::bip32::{DerivationPath, Xpub};
use faraday_bip::keys::{Fingerprint, MasterKey};
use faraday_crypto::work::Compute;
use faraday_intake::wallet::Refusal;
use faraday_text::Text;

use crate::session::Keys;

/// What the check shows of the key held: public values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckFacts {
    /// The key's fingerprint, the passphrase applied.
    pub fingerprint: Fingerprint,
    /// The fingerprint the same words make with no passphrase, when a
    /// passphrase was given.
    pub without: Option<Fingerprint>,
    /// How many words.
    pub words: usize,
}

/// The key held: its master key, and the words and passphrase it was made
/// of, so that Keep this key can load it.
pub(crate) struct HeldCheck {
    master: MasterKey,
    words: Text,
    passphrase: Text,
    facts: CheckFacts,
}

impl Keys {
    /// Costs a parse of the words, a PBKDF2 and an elliptic-curve
    /// multiplication, and the same again for the fingerprint without the
    /// passphrase when one is given. The key `words` and `passphrase` make
    /// becomes the key the check holds, in place of any other; refused while
    /// a stick is attached and for words no list reads. In the demo only a
    /// sample key is held.
    pub fn check_read(
        &mut self,
        c: &Compute,
        words: &str,
        passphrase: &str,
    ) -> Result<CheckFacts, Refusal> {
        self.no_stick()?;
        let m = crate::session::parse_words(c, words)?;
        let master = crate::session::mnemonic_master(c, &m, passphrase, self.network())?;
        let fingerprint = master.fingerprint();
        self.allowed(fingerprint)?;
        let without = if passphrase.is_empty() {
            None
        } else {
            crate::identity::fingerprint_of_mnemonic(c, &m, "").ok()
        };
        let facts = CheckFacts {
            fingerprint,
            without,
            words: m.word_count(),
        };
        self.check = Some(HeldCheck {
            master,
            words: crate::texts::normal_words(words),
            passphrase: crate::texts::wiped(passphrase),
            facts,
        });
        Ok(facts)
    }

    /// What the check shows of the key it holds; `None` when it holds none.
    /// A lookup.
    pub fn check_facts(&self) -> Option<CheckFacts> {
        self.check.as_ref().map(|h| h.facts)
    }

    /// Costs a key derivation of at most
    /// [`faraday_bip::keys::MAX_PATH_STEPS`] steps and an elliptic-curve
    /// multiplication. The extended public key at `path` below the key the
    /// check holds; `None` when it holds none or for a longer path.
    pub fn check_xpub_at(&self, c: &Compute, path: &DerivationPath) -> Option<Xpub> {
        if path.len() > faraday_bip::keys::MAX_PATH_STEPS {
            return None;
        }
        Some(self.check.as_ref()?.master.derive(c, path).to_xpub(c))
    }

    /// Costs what `add_words_with` costs. The key the check holds becomes
    /// one of the session's keys, as Add a key loads typed words and a
    /// passphrase (the same refusals: a stick attached, not a sample in the
    /// demo, already loaded), labelled `label`. The check's own copy is
    /// erased once it is loaded.
    pub fn check_keep(&mut self, c: &Compute, label: &str) -> Result<Fingerprint, Refusal> {
        let Some(h) = self.check.take() else {
            return Err(Refusal::Words(Text::default()));
        };
        let fp = self.add_words_with(c, &h.words, &h.passphrase, label, None);
        if fp.is_err() {
            self.check = Some(h);
        }
        fp
    }

    /// The key the check holds is erased: the check is done or left.
    pub fn check_forget(&mut self) {
        self.check = None;
    }
}
