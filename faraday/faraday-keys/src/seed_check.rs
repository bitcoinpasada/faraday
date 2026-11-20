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
    /// How many words; `None` for a master seed that has no words (a
    /// SLIP-39 or codex32 backup).
    pub words: Option<usize>,
}

/// The key held: its master key, and the words and passphrase it was made
/// of, so that Keep this key can load it.
pub(crate) struct HeldCheck {
    master: std::sync::Arc<MasterKey>,
    /// Empty for a master seed with no words.
    words: Text,
    passphrase: Text,
    /// The master seed the key was made of, for a key with no words;
    /// wiped as it drops.
    seed: Option<zeroize::Zeroizing<Vec<u8>>>,
    /// The codex32 set a master seed was read from and the strings typed,
    /// for Keep this key to keep them with the key, as Add a key does.
    codex32: Option<(faraday_vault::codex32::SetFacts, Vec<Text>)>,
    facts: CheckFacts,
}

impl HeldCheck {
    /// What a job that compares reads, for the time it runs: the master
    /// behind the same `Arc`, and a copy of the words, wiped as it drops,
    /// for the key the same words make with no passphrase. No passphrase
    /// comes with it.
    #[expect(
        clippy::disallowed_methods,
        reason = "an Arc count, and the words for the job's time only"
    )]
    pub(crate) fn job_copy(&self) -> HeldCheck {
        HeldCheck {
            master: std::sync::Arc::clone(&self.master),
            words: self.words.clone(),
            passphrase: Text::default(),
            seed: None,
            codex32: None,
            facts: self.facts,
        }
    }
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
        let master = std::sync::Arc::new(master);
        self.allowed(fingerprint)?;
        let without = if passphrase.is_empty() {
            None
        } else {
            crate::identity::fingerprint_of_mnemonic(c, &m, "").ok()
        };
        let facts = CheckFacts {
            fingerprint,
            without,
            words: Some(m.word_count()),
        };
        self.check = Some(HeldCheck {
            master,
            words: crate::texts::normal_words(words),
            passphrase: crate::texts::wiped(passphrase),
            seed: None,
            codex32: None,
            facts,
        });
        Ok(facts)
    }

    /// Costs what `from_seed_bytes` costs. The key the master seed `seed`
    /// makes (a SLIP-39 or codex32 backup: no words, no BIP-39
    /// passphrase) becomes the key the check holds, in place of any other;
    /// the same refusals as `check_read`.
    pub fn check_read_seed(&mut self, c: &Compute, seed: &[u8]) -> Result<CheckFacts, Refusal> {
        self.no_stick()?;
        let master = crate::session::seed_master(c, seed, self.network())?;
        let fingerprint = master.fingerprint();
        self.allowed(fingerprint)?;
        let facts = CheckFacts {
            fingerprint,
            without: None,
            words: None,
        };
        self.check = Some(HeldCheck {
            master: std::sync::Arc::new(master),
            words: Text::default(),
            passphrase: Text::default(),
            seed: Some(crate::texts::wiped_bytes(seed)),
            codex32: None,
            facts,
        });
        Ok(facts)
    }

    /// Costs what `check_read` costs. The words the check holds are read
    /// again with `passphrase` in place of the one given before, so that
    /// the passphrase is taken again without the words coming out; refused
    /// when the check holds none. The check keeps its key when this
    /// refuses.
    #[expect(
        clippy::disallowed_methods,
        reason = "the words held, copied for the read and wiped as the copy drops"
    )]
    pub fn check_reread(&mut self, c: &Compute, passphrase: &str) -> Result<CheckFacts, Refusal> {
        let Some(h) = self.check.as_ref().filter(|h| h.seed.is_none()) else {
            return Err(Refusal::Words(Text::default()));
        };
        let words = h.words.clone();
        self.check_read(c, &words, passphrase)
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

    /// Costs a parse of the words, a PBKDF2, a key derivation of at most
    /// [`faraday_bip::keys::MAX_PATH_STEPS`] steps and an elliptic-curve
    /// multiplication. The extended public key at `path` below what the
    /// words the check holds make with no passphrase; `None` when it holds
    /// none, for a key with no words and for a longer path.
    pub fn check_xpub_without_passphrase_at(
        &self,
        c: &Compute,
        path: &DerivationPath,
    ) -> Option<Xpub> {
        if path.len() > faraday_bip::keys::MAX_PATH_STEPS {
            return None;
        }
        let h = self.check.as_ref().filter(|h| h.seed.is_none())?;
        let m = crate::session::parse_words(c, &h.words).ok()?;
        crate::identity::xpub_at(
            c,
            crate::identity::Material::Words(&m),
            "",
            self.network(),
            path,
        )
    }

    /// The codex32 set the check's master seed was read from, with the
    /// strings typed, kept for [`Keys::check_keep`]; wiped with the check.
    /// Nothing while the check holds no key.
    pub fn check_set_codex32(&mut self, facts: faraday_vault::codex32::SetFacts, strings: &[&str]) {
        if let Some(h) = self.check.as_mut() {
            h.codex32 = Some((
                facts,
                strings.iter().map(|s| crate::texts::wiped(s)).collect(),
            ));
        }
    }

    /// Costs what `add_words_with` costs. The key the check holds becomes
    /// one of the session's keys, as Add a key loads typed words and a
    /// passphrase (the same refusals: a stick attached, not a sample in the
    /// demo, already loaded), labelled `label`. The check's own copy is
    /// erased once it is loaded.
    pub fn check_keep(&mut self, c: &Compute, label: &str) -> Result<Fingerprint, Refusal> {
        let Some(mut h) = self.check.take() else {
            return Err(Refusal::Words(Text::default()));
        };
        let fp = match &h.seed {
            Some(seed) => self.add_seed(c, seed, label),
            None => self.add_words_with(c, &h.words, &h.passphrase, label, None),
        };
        if let Ok(fp) = &fp
            && let Some((facts, strings)) = h.codex32.take()
            && let Some(i) = self.position(*fp)
        {
            let refs: Vec<&str> = strings.iter().map(|t| t.as_str()).collect();
            self.set_codex32(i, Some(facts));
            self.keep_codex32(c, i, &refs);
        }
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
