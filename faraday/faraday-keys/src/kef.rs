//! Krux's encrypted code sealed and opened on another thread
//! (`docs/KEF.md` §7.2, `docs/WORKERS.md` §4).
//!
//! A [`SealJob`] or an [`OpenJob`] owns what one run needs, the
//! passphrase and the plaintext moved in and wiped on drop, and is `Send`.
//! Its [`Progress`] is shared with the main thread, which reads the rounds
//! done for a determinate bar and sets the flag that stops the work at its
//! next step.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use faraday_crypto::work::Compute;
use faraday_kef::spec_of;
use zeroize::Zeroizing;

pub use faraday_codec::kef::ID_MAX;
pub use faraday_kef::{
    Envelope, OpenError, Opened, Refusal, SealError, Sealing, count_with_change,
    reading_passphrases, unwrap,
};

/// How far a running job is, and whether it is told to stop: shared by
/// the main thread and the worker's.
#[derive(Default)]
pub struct Progress {
    cancel: AtomicBool,
    done: AtomicU32,
}

impl Progress {
    /// Tells the job to stop at its next step.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// The rounds the job has done.
    pub fn rounds(&self) -> u32 {
        self.done.load(Ordering::SeqCst)
    }

    /// The callback of `faraday_kef`'s `between`: notes the rounds done
    /// and goes on until told to stop.
    fn between(&self, rounds: u32) -> bool {
        self.done.store(rounds, Ordering::SeqCst);
        !self.cancel.load(Ordering::SeqCst)
    }
}

/// The length of the IV a version carries: none for ECB, else 12 or 16.
/// `None` for a version Krux does not have.
pub fn iv_len(version: u8) -> Option<usize> {
    spec_of(version).map(|s| s.iv_len())
}

/// A code to seal, with what it needs.
pub struct SealJob {
    passphrase: Zeroizing<Vec<u8>>,
    plaintext: Zeroizing<Vec<u8>>,
    version: u8,
    id: Vec<u8>,
    count: u32,
    iv: Vec<u8>,
    progress: Arc<Progress>,
}

impl SealJob {
    /// A seal of `plaintext` under `passphrase` as `version`, with `id`,
    /// `count` and `iv` (fresh bytes of the session's), reporting to
    /// `progress`.
    pub fn new(
        passphrase: Zeroizing<Vec<u8>>,
        plaintext: Zeroizing<Vec<u8>>,
        how: &Sealing<'_>,
        progress: Arc<Progress>,
    ) -> Self {
        Self {
            passphrase,
            plaintext,
            version: how.version,
            id: how.id.to_vec(),
            count: how.count,
            iv: how.iv.to_vec(),
            progress,
        }
    }

    /// Costs a PBKDF2 of the count's rounds, stopped by the progress's
    /// flag: the envelope's bytes, or why there are none
    /// ([`SealError::Cancelled`] when stopped).
    pub fn run(self, c: &Compute) -> Result<Vec<u8>, SealError> {
        let how = Sealing {
            version: self.version,
            id: &self.id,
            count: self.count,
            iv: &self.iv,
        };
        let progress = &self.progress;
        faraday_kef::seal(c, &self.passphrase, &self.plaintext, &how, &mut |n| {
            progress.between(n)
        })
    }
}

/// A code to open, with what it needs.
pub struct OpenJob {
    passphrase: Zeroizing<Vec<u8>>,
    envelope: Envelope,
    progress: Arc<Progress>,
}

impl OpenJob {
    /// An open of `envelope` under `passphrase` (bytes as typed),
    /// reporting to `progress`.
    pub fn new(
        passphrase: Zeroizing<Vec<u8>>,
        envelope: Envelope,
        progress: Arc<Progress>,
    ) -> Self {
        Self {
            passphrase,
            envelope,
            progress,
        }
    }

    /// Costs a PBKDF2 of the envelope's count of rounds, stopped by the
    /// progress's flag: the contents, or why there are none
    /// ([`OpenError::Cancelled`] when stopped).
    pub fn run(self, c: &Compute) -> Result<Opened, OpenError> {
        let progress = &self.progress;
        faraday_kef::open(c, &self.passphrase, &self.envelope, &mut |n| {
            progress.between(n)
        })
    }
}
