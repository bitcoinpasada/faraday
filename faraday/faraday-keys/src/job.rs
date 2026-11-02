//! Key stretches that run on another thread (`docs/WORKERS.md` §6,
//! `docs/KEYS.md` §2, "Transient input").
//!
//! A [`StretchJob`] owns what one Argon2id run needs, the passphrase
//! moved in rather than copied, and is `Send`. Its answer, [`Stretched`],
//! holds the slot key and the decrypted records of a vault opened, or the
//! bytes of a vault made. Both are faraday-keys' types: the key material
//! never leaves them, and joins the store on the main thread
//! ([`crate::vault::OpenVaults::adopt`]). Everything wipes as it drops, at
//! the job's end on the worker or where a stale answer is dropped.

use faraday_crypto::work::Compute;
use faraday_intake::secret_buf::SecretText;
use faraday_vault::{self as fv, Contents};
use zeroize::Zeroizing;

/// The costly part of opening or making a vault, with what it needs.
pub struct StretchJob {
    what: What,
}

enum What {
    Open {
        file: Vec<u8>,
        passphrase: SecretText,
    },
    Make {
        slot_len: u32,
        cost: fv::Cost,
        passphrases: Vec<SecretText>,
        contents: Vec<Contents>,
        seed: Zeroizing<[u8; 32]>,
    },
}

/// What a [`StretchJob`] gives back.
pub struct Stretched {
    out: Out,
}

enum Out {
    Opened(Result<Box<OpenedSlot>, fv::Error>),
    Made(Result<Vec<u8>, fv::Error>),
}

/// A slot opened: the file it came from, its key and its records.
pub(crate) struct OpenedSlot {
    pub(crate) file: Vec<u8>,
    pub(crate) opened: fv::Opened,
    pub(crate) records: Contents,
}

impl StretchJob {
    /// A vault `file` to open with `passphrase`, which is moved in.
    pub fn open(file: Vec<u8>, passphrase: SecretText) -> StretchJob {
        StretchJob {
            what: What::Open { file, passphrase },
        }
    }

    /// A vault to make (`faraday_vault::create`): one to four passphrases,
    /// moved in, each with the contents of its own slot, with everything
    /// random drawn from `seed`.
    pub fn make(
        slot_len: u32,
        cost: fv::Cost,
        passphrases: Vec<SecretText>,
        contents: Vec<Contents>,
        seed: Zeroizing<[u8; 32]>,
    ) -> StretchJob {
        StretchJob {
            what: What::Make {
                slot_len,
                cost,
                passphrases,
                contents,
                seed,
            },
        }
    }

    /// Costs an Argon2id run over the vault's memory for each passphrase
    /// (the same call as `faraday_vault::open` or `create`). Everything
    /// the job held is wiped as it ends.
    pub fn run(self, c: &Compute) -> Stretched {
        let out = match self.what {
            What::Open { file, passphrase } => Out::Opened(
                fv::open(c, &file, passphrase.as_str()).map(|(opened, records)| {
                    Box::new(OpenedSlot {
                        file,
                        opened,
                        records,
                    })
                }),
            ),
            What::Make {
                slot_len,
                cost,
                passphrases,
                contents,
                seed,
            } => {
                let phrases: Vec<&str> = passphrases.iter().map(SecretText::as_str).collect();
                Out::Made(fv::create(c, slot_len, cost, &phrases, &contents, &seed))
            }
        };
        Stretched { out }
    }
}

impl Stretched {
    /// Why the run failed, if it did. A lookup.
    pub fn error(&self) -> Option<fv::Error> {
        match &self.out {
            Out::Opened(Err(e)) | Out::Made(Err(e)) => Some(*e),
            _ => None,
        }
    }

    /// The vault file a make gave: encrypted, so public. An open's answer
    /// joins the store through [`crate::vault::OpenVaults::adopt`] and
    /// gives `Err(Error::Passphrases)` here.
    pub fn into_made(self) -> Result<Vec<u8>, fv::Error> {
        match self.out {
            Out::Made(r) => r,
            Out::Opened(Err(e)) => Err(e),
            Out::Opened(Ok(_)) => Err(fv::Error::Passphrases),
        }
    }

    pub(crate) fn into_opened(self) -> Result<Box<OpenedSlot>, fv::Error> {
        match self.out {
            Out::Opened(r) => r,
            Out::Made(Err(e)) => Err(e),
            Out::Made(Ok(_)) => Err(fv::Error::Passphrases),
        }
    }
}
