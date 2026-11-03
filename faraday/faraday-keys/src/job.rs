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

use std::sync::Arc;

use faraday_bip::keys::{Fingerprint, MasterKey, Network};
use faraday_bip::policy::WalletPolicy;
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

/// A KeePass database to write on another thread: the entry's text and
/// password and the passphrase it is sealed under, moved in, with the
/// randomness the file needs. Everything wipes as the job ends.
pub struct KdbxJob {
    title: Vec<u8>,
    notes: Zeroizing<Vec<u8>>,
    password: Zeroizing<Vec<u8>>,
    passphrase: SecretText,
    seed: Zeroizing<[u8; crate::vault::KDBX_SEED_LEN]>,
}

impl KdbxJob {
    pub(crate) fn new(
        title: Vec<u8>,
        notes: Zeroizing<Vec<u8>>,
        password: Zeroizing<Vec<u8>>,
        passphrase: SecretText,
        seed: Zeroizing<[u8; crate::vault::KDBX_SEED_LEN]>,
    ) -> KdbxJob {
        KdbxJob {
            title,
            notes,
            password,
            passphrase,
            seed,
        }
    }

    /// Costs two Argon2id runs: a KDBX 4 database's own and the check after
    /// it ([`crate::vault::write_kdbx`], then `faraday_backup::kdbx::verify`
    /// under the same passphrase). The file, encrypted so public; an error
    /// where the entry does not fit or the file written does not verify,
    /// and then no file is given back.
    pub fn run(self, c: &Compute) -> Result<Vec<u8>, KdbxError> {
        let file = crate::vault::write_kdbx(
            c,
            &self.title,
            &self.notes,
            &self.password,
            self.passphrase.as_bytes(),
            &self.seed,
        )
        .ok_or(KdbxError::TooLarge)?;
        if !faraday_backup::kdbx::verify(c, &file, self.passphrase.as_bytes()) {
            return Err(KdbxError::NotVerified);
        }
        Ok(file)
    }
}

/// Why a KeePass export gave no file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KdbxError {
    /// The entry does not fit in the database.
    TooLarge,
    /// The file written did not verify under its passphrase.
    NotVerified,
}

/// A PSBT to sign on another thread (`docs/WORKERS.md` §6).
///
/// It holds a clone of the `Arc` of each selected key, not a copy of the
/// key: a key forgotten or a lock while it runs drops the store's `Arc`,
/// the job's keeps the key until the job ends, and the key is wiped as the
/// last one drops. The PSBT, the context's key references and policies (all
/// public), the dangers held past, the seed (wiped as it drops) and the
/// MuSig2 round, if any, are owned. The round comes back with the answer
/// ([`Signing::into_parts`]); a stale answer drops it with its nonces.
pub struct SignJob {
    psbt: faraday_psbt::Psbt,
    masters: Vec<Arc<MasterKey>>,
    selection: Vec<Fingerprint>,
    network: Network,
    refs: Vec<faraday_psbt::KeyRef>,
    policies: Vec<WalletPolicy>,
    accepted: Vec<faraday_psbt::WarningKind>,
    musig: Option<faraday_psbt::MusigSession>,
    seed: Zeroizing<[u8; 32]>,
}

/// What a [`SignJob`] gives back: the PSBT, as far as it was signed, the
/// MuSig2 round as it stands, and the result.
pub struct Signing {
    psbt: faraday_psbt::Psbt,
    musig: Option<faraday_psbt::MusigSession>,
    result: Result<faraday_psbt::SignResult, faraday_psbt::Error>,
}

impl SignJob {
    pub(crate) fn new(
        psbt: faraday_psbt::Psbt,
        masters: Vec<Arc<MasterKey>>,
        selection: Vec<Fingerprint>,
        (network, refs, policies): (Network, Vec<faraday_psbt::KeyRef>, Vec<WalletPolicy>),
        (accepted, musig, seed): (
            Vec<faraday_psbt::WarningKind>,
            Option<faraday_psbt::MusigSession>,
            Zeroizing<[u8; 32]>,
        ),
    ) -> SignJob {
        SignJob {
            psbt,
            masters,
            selection,
            network,
            refs,
            policies,
            accepted,
            musig,
            seed,
        }
    }

    /// Costs what `faraday_psbt::sign` costs (the same call as before the
    /// job existed, over the same keys).
    pub fn run(mut self, c: &Compute) -> Signing {
        let masters: Vec<&MasterKey> = self.masters.iter().map(|m| &**m).collect();
        let ctx = faraday_psbt::Context {
            network: self.network,
            keys: &self.refs,
            wallets: &self.policies,
            musig_session: None,
        };
        let result = crate::psbt::sign(
            c,
            &mut self.psbt,
            &masters,
            &self.selection,
            (&ctx, &self.accepted),
            &mut self.musig,
            &self.seed,
        );
        Signing {
            psbt: self.psbt,
            musig: self.musig,
            result,
        }
    }
}

impl Signing {
    /// The PSBT, the MuSig2 round as it stands and the result.
    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        faraday_psbt::Psbt,
        Option<faraday_psbt::MusigSession>,
        Result<faraday_psbt::SignResult, faraday_psbt::Error>,
    ) {
        (self.psbt, self.musig, self.result)
    }
}
