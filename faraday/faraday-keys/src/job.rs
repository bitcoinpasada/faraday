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
#![cfg_attr(not(test), warn(clippy::disallowed_methods))]

use std::sync::Arc;

use faraday_bip::keys::{Fingerprint, MasterKey, Network};
use faraday_bip::policy::WalletPolicy;
use faraday_crypto::work::Compute;
use faraday_intake::secret_buf::SecretText;
use faraday_vault as fv;
use zeroize::Zeroizing;

use crate::session::{Held, Part, Reveal};
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
    /// The network an open works its key records' identities out on; none:
    /// they are not worked out.
    identities: Option<Network>,
}

/// A key record's identity: its id ([`crate::vault::key_record_id`]), its
/// fingerprint and its [`faraday_bip::keys::MasterKey`]-derived master id.
/// Public: no secret is in it.
pub type KeyIdentity = ([u8; 32], Fingerprint, [u8; 32]);

enum What {
    Open {
        file: Zeroizing<Vec<u8>>,
        /// The bytes the document's structures may take; none: the
        /// format's limit alone.
        room: Option<usize>,
        /// What a compressed file's plaintext may take; none: any length
        /// up to the ceiling.
        budget: Option<Budget>,
        /// What the whole body may take; none: no check.
        parsed: Option<Parsed>,
        passphrase: SecretText,
        /// The key file's key and the YubiKey's secret the file needs.
        components: fv::kdbx::Components,
    },
    Make {
        id: [u8; 16],
        label: Box<str>,
        cost: fv::Cost,
        passphrase: SecretText,
        components: fv::kdbx::Components,
        /// The KeePass document of a key file drawn for this make.
        document: Option<Zeroizing<Vec<u8>>>,
        random: Zeroizing<[u8; SEAL_RANDOM_LEN]>,
        now: Option<u64>,
    },
}

/// What a [`StretchJob`] gives back.
pub struct Stretched {
    out: Out,
    /// The KeePass document of the key file a make drew.
    document: Option<Zeroizing<Vec<u8>>>,
}

/// A make was given a YubiKey's secret: Faraday never sets one up (F208).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyedRefused;

enum Out {
    Opened(Result<Box<OpenedSlot>, fv::file::Error>),
    Made(Result<Vec<u8>, fv::file::Error>),
}

/// A vault opened: the digest of the file it came from, the opened file and the
/// records built from it.
pub(crate) struct OpenedSlot {
    pub(crate) identities: Vec<KeyIdentity>,
    pub(crate) digest: [u8; 32],
    pub(crate) base: usize,
    pub(crate) opened: Arc<fv::file::Opened>,
    pub(crate) built: Built,
}

impl StretchJob {
    /// A vault `file` to open with `passphrase`, which is moved in.
    pub fn open(file: impl Into<Zeroizing<Vec<u8>>>, passphrase: SecretText) -> StretchJob {
        Self::open_within(file, passphrase, None, None, None)
    }

    /// [`StretchJob::open`] for a caller that says what memory the open may
    /// take beyond what it holds already, in bytes (`room`, see
    /// `faraday_vault::file::open_within`), and what a compressed
    /// file's plaintext may take (`budget`): past either the open ends with
    /// `Error::Memory`; `parsed` is what the body may take once it is whole.
    pub fn open_within(
        file: impl Into<Zeroizing<Vec<u8>>>,
        passphrase: SecretText,
        room: Option<usize>,
        budget: Option<Budget>,
        parsed: Option<Parsed>,
    ) -> StretchJob {
        StretchJob {
            what: What::Open {
                file: file.into(),
                passphrase,
                components: fv::kdbx::Components::none(),
                room,
                budget,
                parsed,
            },
            identities: None,
        }
    }

    /// This job for a vault that also needs a key file's key or a
    /// YubiKey's secret (`docs/VAULT.md` §3.2, F208): an open gives them to
    /// the file, a make writes the file under the key file. They are moved
    /// in and wipe as the job ends; the opened vault keeps them for its
    /// seals. Faraday never sets a YubiKey up (F208): a make given a secret
    /// is refused and gives nothing back; the job and its components wipe
    /// as they drop.
    pub fn keyed(
        mut self,
        with: impl Into<crate::parts::UnlockParts>,
    ) -> Result<StretchJob, KeyedRefused> {
        let with = with.into().0;
        match &mut self.what {
            What::Open { components, .. } => *components = with,
            What::Make {
                components,
                document,
                ..
            } => {
                if with.has_yubikey() {
                    return Err(KeyedRefused);
                }
                *components = with;
                // A document only comes from the draw that made the key.
                *document = None;
            }
        }
        Ok(self)
    }

    /// A make that also draws a new key file from `random` (drawn by the
    /// caller for this and nothing else): the file needs it, and
    /// [`Stretched::key_file_document`] gives its KeePass document. Not for
    /// an open: the job is returned as it was.
    pub fn making_key_file(mut self, random: &[u8; 32]) -> StretchJob {
        if let What::Make {
            components,
            document,
            ..
        } = &mut self.what
        {
            *components = fv::kdbx::Components::with_key_file(Zeroizing::new(*random));
            *document = Some(fv::kdbx::keyfile::write(random));
        }
        self
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
                components: fv::kdbx::Components::none(),
                document: None,
                random,
                now,
            },
            identities: None,
        }
    }

    /// This job for an open that also works out each key record's
    /// fingerprint and master id on `network`, in the job, so the thread
    /// that adopts the vault finds them ([`Stretched::identities`]). A make
    /// is not changed.
    pub fn with_identities(mut self, network: Network) -> StretchJob {
        self.identities = Some(network);
        self
    }

    /// Costs an Argon2id run over the vault's memory (the same call as
    /// `faraday_vault::file::open` or `create`), and the reading of each
    /// GPG key of an opened file; with [`StretchJob::with_identities`],
    /// also a PBKDF2 and two elliptic-curve multiplications for each
    /// distinct key record. Everything the job held is wiped as it
    /// ends.
    pub fn run(self, c: &Compute) -> Stretched {
        let network = self.identities;
        let mut document = None;
        let out = match self.what {
            What::Open {
                file,
                passphrase,
                components,
                room,
                budget,
                parsed,
            } => Out::Opened(
                fv::file::open_with(
                    c,
                    &file,
                    passphrase.as_str(),
                    components,
                    &mut |plain| budget.as_ref().map_or(Ok(()), |b| b(plain)),
                    &mut |doc, att| parsed.as_ref().map_or(Ok(None), |p| p(doc, att)),
                    room,
                )
                .map(|mut opened| {
                    let built = crate::vault::build(&opened);
                    let identities = network.map_or_else(Vec::new, |n| identities_of(c, n, &built));
                    let digest = faraday_crypto::sha256_whole(c, &file);
                    // The file is not needed past its digest.
                    drop(file);
                    Box::new(OpenedSlot {
                        identities,
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
                components,
                document: made,
                random,
                now,
            } => {
                document = made;
                Out::Made(fv::file::create_holding(
                    c,
                    id,
                    &label,
                    cost,
                    passphrase.as_str(),
                    &components,
                    &random,
                    now,
                    &[],
                ))
            }
        };
        Stretched { out, document }
    }
}

/// Costs a PBKDF2 and two elliptic-curve multiplications for each key
/// record of `built` with a key, once for records of one id. Each key
/// record's identity on `network`, worked out fresh; only the fingerprint
/// and master id leave.
fn identities_of(c: &Compute, network: Network, built: &Built) -> Vec<KeyIdentity> {
    let mut out: Vec<KeyIdentity> = Vec::new();
    for r in built
        .records
        .iter()
        .filter(|r| r.kind == fv::records::kind::KEY)
    {
        let view = crate::vault::RecordView::of(r);
        let Some(id) = crate::vault::key_record_id(view) else {
            continue;
        };
        if out.iter().any(|(k, ..)| *k == id) {
            continue;
        }
        if let Some((fp, mid)) = crate::vault::key_identity(c, network, view) {
            out.push((id, fp, mid));
        }
    }
    out
}

impl Stretched {
    /// The identities of the key records an open worked out, when asked
    /// ([`StretchJob::with_identities`]): public, no secret. Empty for a
    /// make, a failed open or an open not asked. A lookup.
    pub fn identities(&self) -> &[KeyIdentity] {
        match &self.out {
            Out::Opened(Ok(slot)) => &slot.identities,
            _ => &[],
        }
    }

    /// Reveal: the KeePass key file of the key a make drew
    /// ([`StretchJob::making_key_file`]), for `why` (`Reveal::OutboxFile`
    /// of `Part::KeyFile`), to be written to a stick (`docs/KEYS.md` §2).
    /// `None` for a make that drew none or failed, an open, or another reason.
    #[expect(
        clippy::disallowed_methods,
        reason = "the document copied is Zeroizing, wiped on drop"
    )]
    pub fn key_file_document(&self, why: Reveal) -> Option<Held<Zeroizing<Vec<u8>>>> {
        if !matches!(why, Reveal::OutboxFile(Part::KeyFile)) {
            return None;
        }
        if !matches!(self.out, Out::Made(Ok(_))) {
            return None;
        }
        let d = self.document.as_ref()?;
        Some(Held::of(d.clone()))
    }

    /// Why the run failed, if it did. A lookup.
    #[expect(clippy::disallowed_methods, reason = "an error is public")]
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
    /// The key file's key the seal writes (`Some(None)`: none) and whether
    /// it leaves out the YubiKey's secret; wiped with the job.
    parts: (Option<Option<Zeroizing<[u8; 32]>>>, bool),
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
        parts: (Option<Option<Zeroizing<[u8; 32]>>>, bool),
        random: &[u8; SEAL_RANDOM_LEN],
        now: Option<u64>,
    ) -> SealJob {
        SealJob {
            opened,
            ops,
            names,
            rekey,
            parts,
            random: Zeroizing::new(*random),
            now,
        }
    }

    /// Costs an Argon2id run at the vault's next cost, a document write for
    /// each entry changed, a PBKDF2 for each new key with no label and a
    /// read back of the document and of the file made (the same calls as
    /// `faraday_vault::file::seal_with`).
    pub fn run(self, c: &Compute) -> Sealed {
        #[expect(clippy::disallowed_methods, reason = "an error is public")]
        let out = match &self.ops {
            Ok(ops) => with_changes(c, &self.names, ops, true, self.now, |changes| {
                let rekey = fv::file::Rekey {
                    passphrase: self.rekey.0.as_ref().map(SecretText::as_str),
                    memory_kib: self.rekey.1,
                    key_file: self.parts.0.as_ref().map(|k| k.as_deref()),
                    drop_yubikey: self.parts.1,
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
            record: self.record,
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
    seed: Option<Zeroizing<[u8; 32]>>,
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
            Option<Zeroizing<[u8; 32]>>,
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
            self.seed.as_ref(),
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

/// A key record of words to load: the words as the record keeps them and
/// the passphrase to stretch them with. The mnemonic is held inline: the
/// vector of these is reserved at its full size, so no growth copies it
/// to the heap and frees the old one. Stated limit (`docs/SECRET-LEAKS.md`
/// §8, stack copies, reviewed and not tested): moving the mnemonic into
/// the record and the record into the vector leaves the word indices once
/// in the frame that built it, which is not wiped.
pub(crate) struct WordsToLoad {
    pub(crate) phrase: Zeroizing<Vec<u8>>,
    pub(crate) mnemonic: faraday_bip::bip39::Mnemonic,
    /// The passphrase the record stores with its key; none: the one an
    /// open vault keeps alone for the key, if one is.
    pub(crate) own: Option<Zeroizing<Vec<u8>>>,
    /// Which of the passphrases kept alone for the key is wanted.
    pub(crate) chosen: usize,
    pub(crate) label: Zeroizing<Vec<u8>>,
    pub(crate) codex32: Option<fv::codex32::SetFacts>,
    pub(crate) child: Option<fv::child::ChildOf>,
}

/// A key record of a master seed to load.
pub(crate) struct SeedToLoad {
    pub(crate) seed: Zeroizing<Vec<u8>>,
    pub(crate) label: Zeroizing<Vec<u8>>,
    pub(crate) codex32: Option<fv::codex32::SetFacts>,
}

/// The costly part of loading the key records of open vaults into the
/// session ([`crate::vault::OpenVaults::load_job`]): the stretch of each
/// key's words, its master key and its id. The keys come out whole, for
/// the thread that holds the session to add
/// ([`crate::Keys::adopt_loaded`]); no secret leaves faraday-keys.
pub struct KeysJob {
    pub(crate) network: Network,
    pub(crate) words: Vec<WordsToLoad>,
    pub(crate) seeds: Vec<SeedToLoad>,
    /// The passphrase records the open vaults keep alone, each with the
    /// fingerprint it names, in the order the vaults are held.
    pub(crate) kept: Vec<(Vec<u8>, Zeroizing<Vec<u8>>)>,
}

/// The keys a [`KeysJob`] worked out, in the order they are added.
pub struct KeysLoaded {
    /// The network the keys were worked out on; a session on another adds none.
    pub(crate) network: Network,
    pub(crate) items: Vec<LoadedKey>,
}

pub(crate) struct LoadedKey {
    pub(crate) entry: crate::session::Entry,
    pub(crate) codex32: Option<fv::codex32::SetFacts>,
    pub(crate) child: Option<fv::child::ChildOf>,
}

/// The text a held byte string stands for: what was stored as text, so it
/// is text; empty if it is not.
fn text(bytes: &[u8]) -> &str {
    core::str::from_utf8(bytes).unwrap_or("")
}

impl KeysJob {
    pub(crate) fn new(network: Network) -> KeysJob {
        KeysJob {
            network,
            words: Vec::new(),
            seeds: Vec::new(),
            kept: Vec::new(),
        }
    }

    /// Whether the job has no key to work out: a load of wallets alone.
    pub fn is_empty(&self) -> bool {
        self.words.is_empty() && self.seeds.is_empty()
    }

    /// Costs a PBKDF2 for each passphrase kept alone that a words record
    /// has no passphrase of its own to be tried with, and what
    /// `add_words_with` and `add_seed` cost for each key: a parse of the
    /// words, a PBKDF2, a master key and its id. A key that does not
    /// stretch is left out. Everything the job held is wiped as it ends.
    pub fn run(self, c: &Compute) -> KeysLoaded {
        let network = self.network;
        let mut items = Vec::new();
        for w in &self.words {
            #[expect(
                clippy::disallowed_methods,
                reason = "the passphrase copied is Zeroizing, wiped on drop"
            )]
            let pass = match &w.own {
                Some(p) => p.clone(),
                None => self.kept_for(c, w).unwrap_or_default(),
            };
            let (phrase, pass) = (text(&w.phrase), text(&pass));
            let Ok(m) = crate::session::parse_words(c, phrase) else {
                continue;
            };
            let Ok(master) = crate::session::mnemonic_master(c, &m, pass, network) else {
                continue;
            };
            #[expect(
                clippy::disallowed_methods,
                reason = "a codex32 set's facts are public"
            )]
            items.push(LoadedKey {
                entry: crate::session::mnemonic_entry(c, master, &m, pass, text(&w.label)),
                codex32: w.codex32.clone(),
                child: w.child,
            });
        }
        for s in &self.seeds {
            let Ok(master) = crate::session::seed_master(c, &s.seed, network) else {
                continue;
            };
            #[expect(
                clippy::disallowed_methods,
                reason = "a codex32 set's facts are public"
            )]
            items.push(LoadedKey {
                entry: crate::session::seed_entry(c, master, &s.seed, text(&s.label)),
                codex32: s.codex32.clone(),
                child: None,
            });
        }
        KeysLoaded { network, items }
    }

    /// Costs a PBKDF2 for each passphrase kept alone. The `chosen`th of
    /// the passphrases kept alone for the words of `w` (the first when it
    /// names none), found by the fingerprint each record names.
    #[expect(
        clippy::disallowed_methods,
        reason = "the passphrase copied is Zeroizing, wiped on drop"
    )]
    fn kept_for(&self, c: &Compute, w: &WordsToLoad) -> Option<Zeroizing<Vec<u8>>> {
        use crate::identity::{Material, fingerprint};
        let found: Vec<&Zeroizing<Vec<u8>>> = self
            .kept
            .iter()
            .filter(|(fp, p)| {
                fingerprint(c, Material::Words(&w.mnemonic), text(p), self.network)
                    .is_some_and(|got| got.0[..] == fp[..])
            })
            .map(|(_, p)| p)
            .collect();
        Some((*found.get(w.chosen).or_else(|| found.first())?).clone())
    }
}
