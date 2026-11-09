//! Key stretches that run on another thread (`docs/WORKERS.md` §6,
//! `docs/KEYS.md` §2, "Transient input").
//!
//! A [`StretchJob`] owns what one Argon2id run needs, the passphrase
//! moved in rather than copied, and is `Send`. Its answer, [`Stretched`],
//! holds the opened vault file and its records, or the bytes of a vault
//! made. A [`SealJob`] is the same for a seal: the store hands it the
//! changes and the opened file, and its answer, [`Sealed`], is the new
//! file. All are faraday-keys' types: the key material never leaves them,
//! and an opened file joins the store on the main thread
//! ([`crate::vault::OpenVaults::adopt`]). Everything wipes as it drops, at
//! the job's end on the worker or where a stale answer is dropped.

use std::sync::Arc;

use faraday_bip::keys::{Fingerprint, MasterKey, Network};
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;
use faraday_intake::secret_buf::SecretText;
use faraday_vault as fv;
use zeroize::Zeroizing;

use crate::vault::{Built, Names, Op, SEAL_RANDOM_LEN, with_changes};

/// What a compressed file's plaintext may take: asked with the plaintext
/// length the file states once its HMACs have verified; `Err(mib)` is the
/// memory that was not found (`faraday_vault::file::open_within`).
pub type Budget = Box<dyn Fn(usize) -> Result<(), u32> + Send>;

/// What a file's body may take once it is whole: asked with the XML
/// document's length and the attachments' together, before the document is
/// parsed; `Err(mib)` is the memory that was not found, and `Ok(Some(bytes))`
/// the most the open may take beyond what the caller holds, whatever is free
/// (`faraday_vault::file::open_within`).
pub type Parsed = Box<dyn Fn(usize, usize) -> Result<Option<usize>, u32> + Send>;

/// The costly part of opening or making a vault, with what it needs.
pub struct StretchJob {
    what: What,
}

enum What {
    Open {
        file: Vec<u8>,
        /// The bytes the document's structures may take; none: the
        /// format's limit alone.
        room: Option<usize>,
        /// What a compressed file's plaintext may take; none: any length
        /// up to the ceiling.
        budget: Option<Budget>,
        /// What the whole body may take; none: no check.
        parsed: Option<Parsed>,
        passphrase: SecretText,
    },
    Make {
        id: [u8; 16],
        label: Box<str>,
        cost: fv::Cost,
        passphrase: SecretText,
        random: Zeroizing<[u8; SEAL_RANDOM_LEN]>,
        now: Option<u64>,
    },
}

/// What a [`StretchJob`] gives back.
pub struct Stretched {
    out: Out,
}

enum Out {
    Opened(Result<Box<OpenedSlot>, fv::file::Error>),
    Made(Result<Vec<u8>, fv::file::Error>),
}

/// A vault opened: the digest of the file it came from, the opened file and the
/// records built from it.
pub(crate) struct OpenedSlot {
    pub(crate) digest: [u8; 32],
    pub(crate) base: usize,
    pub(crate) opened: Arc<fv::file::Opened>,
    pub(crate) built: Built,
}

impl StretchJob {
    /// A vault `file` to open with `passphrase`, which is moved in.
    pub fn open(file: Vec<u8>, passphrase: SecretText) -> StretchJob {
        Self::open_within(file, passphrase, None, None, None)
    }

    /// [`StretchJob::open`] for a caller that says what memory the open may
    /// take beyond what it holds already, in bytes (`room`, see
    /// `faraday_vault::file::open_within`), and what a compressed
    /// file's plaintext may take (`budget`): past either the open ends with
    /// `Error::Memory`; `parsed` is what the body may take once it is whole.
    pub fn open_within(
        file: Vec<u8>,
        passphrase: SecretText,
        room: Option<usize>,
        budget: Option<Budget>,
        parsed: Option<Parsed>,
    ) -> StretchJob {
        StretchJob {
            what: What::Open {
                file,
                passphrase,
                room,
                budget,
                parsed,
            },
        }
    }

    /// A vault to make (`faraday_vault::file::create`): the root group's
    /// UUID `id`, named `label`, at `cost`, under one
    /// `passphrase`, moved in, with `random` drawn for the seal and `now`
    /// the time of the first seal.
    pub fn make(
        id: [u8; 16],
        label: Box<str>,
        cost: fv::Cost,
        passphrase: SecretText,
        random: Zeroizing<[u8; SEAL_RANDOM_LEN]>,
        now: Option<u64>,
    ) -> StretchJob {
        StretchJob {
            what: What::Make {
                id,
                label,
                cost,
                passphrase,
                random,
                now,
            },
        }
    }

    /// Costs an Argon2id run over the vault's memory (the same call as
    /// `faraday_vault::file::open` or `create`), and the reading of each
    /// GPG key of an opened file. Everything the job held is wiped as it
    /// ends.
    pub fn run(self, c: &Compute) -> Stretched {
        let out = match self.what {
            What::Open {
                file,
                passphrase,
                room,
                budget,
                parsed,
            } => Out::Opened(
                fv::file::open_within(
                    c,
                    &file,
                    passphrase.as_str(),
                    &mut |plain| budget.as_ref().map_or(Ok(()), |b| b(plain)),
                    &mut |doc, att| parsed.as_ref().map_or(Ok(None), |p| p(doc, att)),
                    room,
                )
                .map(|mut opened| {
                    let built = crate::vault::build(&opened);
                    let digest = faraday_crypto::sha256_whole(c, &file);
                    // The file is not needed past its digest.
                    drop(file);
                    Box::new(OpenedSlot {
                        digest,
                        base: crate::vault::base_of(&mut opened),
                        opened: Arc::new(opened),
                        built,
                    })
                }),
            ),
            What::Make {
                id,
                label,
                cost,
                passphrase,
                random,
                now,
            } => Out::Made(fv::file::create(
                c,
                id,
                &label,
                cost,
                passphrase.as_str(),
                &random,
                now,
            )),
        };
        Stretched { out }
    }
}

impl Stretched {
    /// Why the run failed, if it did. A lookup.
    pub fn error(&self) -> Option<fv::file::Error> {
        match &self.out {
            Out::Opened(Err(e)) | Out::Made(Err(e)) => Some(e.clone()),
            _ => None,
        }
    }

    /// The vault file a make gave: encrypted, so public. An open's answer
    /// joins the store through [`crate::vault::OpenVaults::adopt`] and
    /// gives `Err(Error::ReadOnly)` here.
    pub fn into_made(self) -> Result<Vec<u8>, fv::file::Error> {
        match self.out {
            Out::Made(r) => r,
            Out::Opened(Err(e)) => Err(e),
            Out::Opened(Ok(_)) => Err(fv::file::Error::ReadOnly),
        }
    }

    pub(crate) fn into_opened(self) -> Result<Box<OpenedSlot>, fv::file::Error> {
        match self.out {
            Out::Opened(r) => r,
            Out::Made(Err(e)) => Err(e),
            Out::Made(Ok(_)) => Err(fv::file::Error::ReadOnly),
        }
    }
}

/// A vault to seal on another thread: the opened file (shared with
/// nothing that changes it), the writes, the randomness and the time. Made
/// by [`crate::vault::OpenVaults::seal_job`]; everything wipes as the job
/// ends or is dropped.
pub struct SealJob {
    opened: Arc<fv::file::Opened>,
    ops: Result<Vec<Op>, fv::file::Error>,
    names: Names,
    /// A new passphrase (a copy that wipes with the job) and a new memory
    /// step, when the seal takes them.
    rekey: (Option<SecretText>, Option<u32>),
    random: Zeroizing<[u8; SEAL_RANDOM_LEN]>,
    now: Option<u64>,
}

/// What a [`SealJob`] gives back: the new file, or why there is none.
/// Wiped if dropped.
pub struct Sealed {
    out: Result<Zeroizing<Vec<u8>>, fv::file::Error>,
}

impl SealJob {
    pub(crate) fn new(
        opened: Arc<fv::file::Opened>,
        ops: Result<Vec<Op>, fv::file::Error>,
        names: Names,
        rekey: (Option<SecretText>, Option<u32>),
        random: &[u8; SEAL_RANDOM_LEN],
        now: Option<u64>,
    ) -> SealJob {
        SealJob {
            opened,
            ops,
            names,
            rekey,
            random: Zeroizing::new(*random),
            now,
        }
    }

    /// Costs an Argon2id run at the vault's next cost, a document write for
    /// each entry changed, a PBKDF2 for each new key with no label and a
    /// read back of the document and of the file made (the same calls as
    /// `faraday_vault::file::seal_with`).
    pub fn run(self, c: &Compute) -> Sealed {
        let out = match &self.ops {
            Ok(ops) => with_changes(c, &self.names, ops, true, self.now, |changes| {
                let rekey = fv::file::Rekey {
                    passphrase: self.rekey.0.as_ref().map(SecretText::as_str),
                    memory_kib: self.rekey.1,
                };
                fv::file::seal_with(c, &self.opened, changes, rekey, &self.random, self.now)
            }),
            Err(e) => Err(e.clone()),
        };
        Sealed {
            out: out.map(Zeroizing::new),
        }
    }
}

impl Sealed {
    /// Why the seal gave no file, if it did not. A lookup.
    pub fn error(&self) -> Option<fv::file::Error> {
        self.out.as_ref().err().cloned()
    }

    /// The vault file: encrypted, so public.
    pub fn into_file(self) -> Result<Vec<u8>, fv::file::Error> {
        self.out.map(|mut f| core::mem::take(&mut *f))
    }
}

/// A vault holding one item to write on another thread, for a KeePass
/// app: the item, the passphrase it is sealed under and the randomness the
/// file needs, moved in. Everything wipes as the job ends.
pub struct KdbxJob {
    record: fv::Record,
    names: Names,
    vault: ([u8; 16], Box<str>, Option<u64>),
    uuids: ([u8; 16], [u8; 16]),
    passphrase: SecretText,
    cost: fv::Cost,
    random: Zeroizing<[u8; SEAL_RANDOM_LEN]>,
}

impl KdbxJob {
    pub(crate) fn new(
        record: fv::Record,
        names: Names,
        vault: ([u8; 16], Box<str>, Option<u64>),
        uuids: ([u8; 16], [u8; 16]),
        passphrase: SecretText,
        cost: fv::Cost,
        random: Zeroizing<[u8; SEAL_RANDOM_LEN]>,
    ) -> KdbxJob {
        KdbxJob {
            record,
            names,
            vault,
            uuids,
            passphrase,
            cost,
            random,
        }
    }

    /// Costs an Argon2id run at the job's cost and a document write (the
    /// same calls as `faraday_vault::file::create_holding`). The vault
    /// file, encrypted so public; an error where the item does not fit the
    /// largest size, and then no file is given back.
    pub fn run(self, c: &Compute) -> Result<Vec<u8>, fv::file::Error> {
        let (id, label, now) = &self.vault;
        let op = Op::Put {
            uuid: self.uuids.0,
            group_uuid: self.uuids.1,
            new: true,
            record: self.record.clone(),
        };
        with_changes(c, &self.names, &[op], true, *now, |changes| {
            fv::file::create_holding(
                c,
                *id,
                label,
                self.cost,
                self.passphrase.as_str(),
                &fv::kdbx::Components::none(),
                &self.random,
                *now,
                changes,
            )
        })
    }
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
    /// The MuSig2 round's public facts, no nonce: `None` where the round is
    /// empty (every nonce has signed, or none was drawn), which is the
    /// round the spend does not keep.
    pub fn round_view(&self) -> Option<faraday_psbt::MusigSessionView> {
        self.musig
            .as_ref()
            .filter(|m| !m.is_empty())
            .map(|m| m.view())
    }

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
