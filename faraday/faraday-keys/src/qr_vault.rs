//! The QR vault tool's secrets (`docs/VAULT.md` §13, F226): a note, a
//! loaded key's words or an open file vault's item sealed as a QR vault,
//! and a code opened into one held item that leaves only through the
//! offers below (`Reveal::QrVault`, `docs/KEYS.md`).
//!
//! Every seal and open of a QR vault in the app is here, over
//! [`faraday_vault::qr`]. A seal is read back under the same passphrase
//! before its bytes are given. The item an open gives is held in
//! [`Keys`], one at a time: a new open or [`Keys::qr_forget`] wipes the
//! old, and [`Keys::clear`] (a lock, the idle warning) wipes it with the
//! rest of the session's secrets.

use faraday_bip::bip39::{Language, Mnemonic};
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::Compute;
use faraday_intake::secret_buf::SecretText;
use faraday_vault::contents::{self, Why, write_qr};
use faraday_vault::records::{self, field, kind};
use faraday_vault::{Record, qr};
use zeroize::Zeroizing;

use crate::session::{Keys, Reveal, Revealed};
use crate::vault::{OpenVaults, Room, VaultId};

pub use faraday_pgp::message::{
    Cost as QrCost, RANDOM_LEN as QR_RANDOM_LEN, SEAL_EXPONENTS as QR_SEAL_EXPONENTS,
    Unsupported as QrUnsupported,
};

/// Whether a file vault's item of record kind `kind` has the **Save as a
/// QR vault** button (`docs/VAULT.md` §13): a seed, a codex32 share, a
/// BIP-39 passphrase and a note, and nothing else. A lookup.
pub fn qr_item_offered(kind: u8) -> bool {
    matches!(
        kind,
        kind::KEY | kind::CODEX32_SHARE | kind::PASSPHRASE | kind::NOTE
    )
}

/// Whether a file vault's item fits a QR vault, or why not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrFit {
    /// It fits.
    Fits,
    /// The item has no **Save as a QR vault** button
    /// ([`qr_item_offered`]), or there is no such item.
    NotOffered,
    /// A seed that is not 12 or 24 words of the English list, held as
    /// words.
    SeedNotQr,
    /// A passphrase whose TOML text is over `QR_PASSPHRASE_MAX`.
    PassphraseTooLong,
    /// A note too long by this many characters: the fewest, each one a
    /// Unicode scalar value, taken off its end to make it fit.
    NoteTooLong(usize),
    /// The item is one the contents refuse for another reason (text that
    /// is not UTF-8).
    Unwritable,
}

/// Why a QR vault was not sealed or opened, or an item not offered on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QrError {
    /// The passphrase is empty.
    EmptyPassphrase,
    /// The passphrase or the Argon2id memory could not be allocated: the
    /// MiB it needs.
    Memory(u32),
    /// The passphrase is wrong (the session key's tag did not check). May
    /// be tried again.
    WrongPassphrase,
    /// An OpenPGP message Faraday does not open (`docs/VAULT.md` §1.2),
    /// named by the rule it is refused for.
    NotQrVault(QrUnsupported),
    /// Not the packets of an OpenPGP message.
    NotAMessage,
    /// The stated Argon2id cost is outside the limits, or a memory step
    /// is not one of `QR_SEAL_EXPONENTS`.
    Cost,
    /// The passphrase is right and a data tag did not check.
    Damaged,
    /// The message opened and its contents are refused (§2.1), or the
    /// item does not fit.
    Contents(contents::Refusal),
    /// The message opened to something that is none of the five items.
    Unreadable,
    /// The seal did not read back as the item sealed: no code is given.
    ReadBack,
    /// The item does not fit a QR vault, or has no place in one.
    NotFit(QrFit),
    /// There is no such key, vault or item.
    NoItem,
    /// The offer is not of what is held (nothing is held, or another
    /// item).
    NotHeld,
    /// No loaded key with words makes the passphrase's key with it.
    NotLoaded,
    /// The key was refused.
    Key(faraday_intake::wallet::Refusal),
    /// The open vault has no room for the note, or is read-only or not
    /// held.
    NoRoom,
    /// Whether the note fits is not known: the copy of the document of this
    /// many bytes that the exact count takes is more than this computer has
    /// free. The vault is unchanged.
    Short(usize),
}

fn from_message(e: faraday_pgp::message::Error) -> QrError {
    use faraday_pgp::message::Error;
    match e {
        Error::Unsupported(u) => QrError::NotQrVault(u),
        Error::Literal => QrError::NotQrVault(QrUnsupported::Other),
        Error::Malformed | Error::TooLong => QrError::NotAMessage,
        Error::Cost => QrError::Cost,
        Error::Memory(n) => QrError::Memory(n),
        Error::WrongPassphrase => QrError::WrongPassphrase,
        Error::Damaged => QrError::Damaged,
    }
}

fn from_qr(e: qr::Error) -> QrError {
    match e {
        qr::Error::Message(m) => from_message(m),
        qr::Error::Contents(r) => QrError::Contents(r),
        qr::Error::Memory(n) => QrError::Memory(n),
        qr::Error::EmptyPassphrase => QrError::EmptyPassphrase,
    }
}

/// What the QR vault `bytes` state their S2K costs, read with no
/// passphrase and no Argon2id run: for the caller's memory check (`docs/VAULT.md` §4) before it opens. Refused as
/// [`Keys::qr_open`] refuses what is not a QR vault Faraday opens
/// ([`QrError::NotQrVault`], [`QrError::NotAMessage`], [`QrError::Cost`]).
pub fn qr_cost(bytes: &[u8]) -> Result<QrCost, QrError> {
    faraday_pgp::message::parse(bytes)
        .map(|m| m.cost())
        .map_err(from_message)
}

/// Costs what `qr::seal` and `qr::open` cost, two Argon2id runs. Seals
/// `record` and reads the seal back under the same passphrase: the
/// code's bytes only when it opens to the same contents.
fn seal_record(
    c: &Compute,
    record: &Record,
    passphrase: &str,
    exponent: u8,
    random: &[u8; QR_RANDOM_LEN],
) -> Result<Vec<u8>, QrError> {
    let bytes = qr::seal(c, passphrase, exponent, random, record).map_err(from_qr)?;
    let back = qr::open(c, passphrase, &bytes).map_err(from_qr)?;
    let want = write_qr(record).map_err(QrError::Contents)?;
    let got = write_qr(&back).map_err(QrError::Contents)?;
    if *want != *got {
        return Err(QrError::ReadBack);
    }
    Ok(bytes)
}

/// A seal of a QR vault to run on another thread (`docs/WORKERS.md` §3):
/// the item as its one record, the passphrase and the memory step, and the
/// 95 random bytes, all owned, so the job is `Send`. The record's fields,
/// the passphrase and the random bytes are wiped as the job ends or is
/// dropped; a key's words are copied in by [`Keys::qr_seal_key_job`] and
/// leave only in the code sealed.
pub struct QrSealJob {
    record: Record,
    passphrase: SecretText,
    exponent: u8,
    random: Zeroizing<[u8; QR_RANDOM_LEN]>,
}

impl QrSealJob {
    /// Costs two Argon2id runs at `exponent`, the seal and the read-back
    /// under the same passphrase. The code's bytes, 396 of them, only
    /// when the seal opens to the same contents; the job's secrets wipe as
    /// it ends. `exponent` is one of `QR_SEAL_EXPONENTS`; a note over
    /// `QR_NOTE_MAX` is [`QrError::Contents`].
    pub fn run(self, c: &Compute) -> Result<Vec<u8>, QrError> {
        seal_record(
            c,
            &self.record,
            self.passphrase.as_str(),
            self.exponent,
            &self.random,
        )
    }
}

/// The seal of a typed note as a QR vault under `passphrase`
/// (`docs/VAULT.md` §13 Make), to run with [`QrSealJob::run`]. `random` is
/// the caller's 95 bytes, moved in. An empty note seals the empty vault.
/// A pure copy: nothing is worked out until the job runs.
pub fn qr_seal_note_job(
    note: &str,
    passphrase: &str,
    exponent: u8,
    random: Zeroizing<[u8; QR_RANDOM_LEN]>,
) -> QrSealJob {
    QrSealJob {
        record: Record::new(kind::NOTE).with(field::NOTE, note.as_bytes()),
        passphrase: SecretText::of(passphrase),
        exponent,
        random,
    }
}

/// How many characters to take off the end of `note` for it to fit a QR
/// vault (`QR_NOTE_MAX`), counted as the contents write it; 0 for a note
/// that fits. A pure count.
pub fn qr_note_over(note: &str) -> usize {
    contents::qr_note_over(note)
}

/// A file vault's item as the one record a QR vault holds: its secret
/// fields and no other. `None` for a kind with no button.
fn item_record(r: &Record) -> Option<Record> {
    let copy = |mut out: Record, numbers: &[u8]| {
        for &n in numbers {
            if let Some(b) = r.field(n) {
                out.push(n, b);
            }
        }
        out
    };
    match r.kind {
        kind::KEY => Some(copy(Record::new(kind::KEY), &[field::KEY])),
        kind::CODEX32_SHARE => Some(copy(
            Record::new(kind::CODEX32_SHARE),
            &[field::CODEX32_STRING, field::CODEX32_KEY],
        )),
        kind::PASSPHRASE => Some(copy(
            Record::new(kind::PASSPHRASE),
            &[field::PASSPHRASE, field::PASSPHRASE_KEY],
        )),
        kind::NOTE => Some(Record::new(kind::NOTE).with(field::NOTE, r.field(field::NOTE)?)),
        _ => None,
    }
}

impl Keys {
    /// Whether key `i` can be sealed as a QR vault: 12 or 24 words of the
    /// English list. A lookup; no word is parsed.
    pub fn qr_key_offered(&self, i: usize) -> bool {
        self.info(i).is_some_and(|k| k.kef_offered())
    }

    /// Costs a parse of the words. Key `i`'s words as a QR vault's seed
    /// under `passphrase` (`Reveal::QrVault`), the job to run with
    /// [`QrSealJob::run`]: the words are copied into the job and never leave
    /// this crate otherwise. [`QrError::NotFit`] with [`QrFit::SeedNotQr`]
    /// for a key of other words, and [`QrError::NoItem`] for no such key.
    pub fn qr_seal_key_job(
        &self,
        c: &Compute,
        i: usize,
        passphrase: &str,
        exponent: u8,
        random: Zeroizing<[u8; QR_RANDOM_LEN]>,
    ) -> Result<QrSealJob, QrError> {
        self.info(i).ok_or(QrError::NoItem)?;
        if !self.qr_key_offered(i) {
            return Err(QrError::NotFit(QrFit::SeedNotQr));
        }
        let s = self.secrets(i).ok_or(QrError::NoItem)?;
        let words = s.words.ok_or(QrError::NotFit(QrFit::SeedNotQr))?;
        let m =
            Mnemonic::parse(c, s.language, words).map_err(|_| QrError::NotFit(QrFit::SeedNotQr))?;
        Ok(QrSealJob {
            record: Record::new(kind::KEY).with(field::KEY, &records::words_payload(&m)),
            passphrase: SecretText::of(passphrase),
            exponent,
            random,
        })
    }

    // ---- an opened code, held ----

    /// Costs the Argon2id run the message states (the caller checked
    /// [`qr_cost`] against the memory it has) and a parse of the words.
    /// Opens the QR vault `bytes` under `passphrase` and holds what it
    /// holds, in place of any item held before (wiped first, so a refusal
    /// leaves none). Says what it holds, with no secret in it. Errors are
    /// kept apart: [`QrError::WrongPassphrase`] (try again),
    /// [`QrError::NotQrVault`] (a message Faraday does not open),
    /// [`QrError::Contents`], [`QrError::Memory`],
    /// [`QrError::EmptyPassphrase`].
    pub fn qr_open(
        &mut self,
        c: &Compute,
        passphrase: &str,
        bytes: &[u8],
    ) -> Result<QrHolds, QrError> {
        self.qr_item = None;
        let record = qr::open(c, passphrase, bytes).map_err(from_qr)?;
        let item = HeldItem::of(&record).ok_or(QrError::Unreadable)?;
        let holds = item.holds();
        self.qr_item = Some(item);
        Ok(holds)
    }

    /// What is held, with no secret in it (a lookup); `None` when no code
    /// is open.
    pub fn qr_holds(&self) -> Option<QrHolds> {
        self.qr_item.as_ref().map(HeldItem::holds)
    }

    /// The held item is wiped and dropped.
    pub fn qr_forget(&mut self) {
        self.qr_item = None;
    }

    /// Costs what `add_words_with` costs. Loads the held seed's words as a
    /// key, as a scanned SeedQR's are, with `passphrase` (empty: none)
    /// and `label`. The item stays held. [`QrError::NotHeld`] when
    /// another item or none is held.
    pub fn qr_load_words(
        &mut self,
        c: &Compute,
        passphrase: &str,
        label: &str,
    ) -> Result<Fingerprint, QrError> {
        let Some(HeldItem::Words(words, _)) = &self.qr_item else {
            return Err(QrError::NotHeld);
        };
        let words = words.clone();
        self.add_words_with(c, &words, passphrase, label, None)
            .map_err(QrError::Key)
    }

    /// Costs a PBKDF2 and an elliptic-curve multiplication for each key
    /// with words, and what `add_with_passphrase` costs. Adds the held
    /// passphrase to the loaded key whose words make the fingerprint the
    /// item names with it (as a file vault's passphrase item is read), as
    /// a key of the same words with that passphrase. [`QrError::NotLoaded`]
    /// when no key with words makes it; [`QrError::Key`] with
    /// `Refusal::Duplicate` when the key with the passphrase is loaded
    /// already (as for any key added twice); [`QrError::NotHeld`] when
    /// another item or none is held. The item stays held.
    pub fn qr_add_passphrase(&mut self, c: &Compute, label: &str) -> Result<Fingerprint, QrError> {
        let Some(HeldItem::Passphrase(text, key)) = &self.qr_item else {
            return Err(QrError::NotHeld);
        };
        let (text, key) = (text.clone(), *key);
        if self.position_of(key).is_some() {
            return Err(QrError::Key(faraday_intake::wallet::Refusal::Duplicate(
                Fingerprint(key),
            )));
        }
        let i = (0..self.len())
            .filter(|&i| self.info(i).is_some_and(|k| k.has_words()))
            .find(|&i| self.fingerprint_with(c, i, &text) == Some(Fingerprint(key)))
            .ok_or(QrError::NotLoaded)?;
        self.add_with_passphrase(c, i, &text, label)
            .map_err(QrError::Key)
    }

    /// The held seed's words, lent (`Reveal::QrVault`), for the seed
    /// sheet. Empty for another reason, another item or none held.
    pub fn qr_words(&self, why: Reveal) -> Revealed<'_> {
        match (&self.qr_item, why) {
            (Some(HeldItem::Words(w, _)), Reveal::QrVault) => Revealed::lent(w),
            _ => Revealed::empty(),
        }
    }

    /// The held codex32 share's string, lent (`Reveal::QrVault`), for
    /// Add a key's codex32 form. Empty for another reason, another item
    /// or none held.
    pub fn qr_share(&self, why: Reveal) -> Revealed<'_> {
        match (&self.qr_item, why) {
            (Some(HeldItem::Share(s, _)), Reveal::QrVault) => Revealed::lent(s),
            _ => Revealed::empty(),
        }
    }

    /// The held passphrase, lent (`Reveal::QrVault`), behind its eye.
    /// Empty for another reason, another item or none held.
    pub fn qr_passphrase(&self, why: Reveal) -> Revealed<'_> {
        match (&self.qr_item, why) {
            (Some(HeldItem::Passphrase(p, _)), Reveal::QrVault) => Revealed::lent(p),
            _ => Revealed::empty(),
        }
    }

    /// The held note's text, lent (`Reveal::QrVault`), behind its eye.
    /// Empty for another reason, another item or none held.
    pub fn qr_note(&self, why: Reveal) -> Revealed<'_> {
        match (&self.qr_item, why) {
            (Some(HeldItem::Note(n)), Reveal::QrVault) => Revealed::lent(n),
            _ => Revealed::empty(),
        }
    }
}

impl OpenVaults {
    /// Costs what `word_count` costs for a seed. Whether item `i` of open
    /// vault `id` fits a QR vault, or why not (`docs/VAULT.md` §13's
    /// table): pure, nothing is sealed. [`QrFit::NotOffered`] for a kind
    /// with no button ([`qr_item_offered`]) or no such item.
    pub fn qr_fit(&self, c: &Compute, id: VaultId, i: usize) -> QrFit {
        let Some(held) = self.copy_out(id, i, &[]) else {
            return QrFit::NotOffered;
        };
        let Some(item) = item_record(&held) else {
            return QrFit::NotOffered;
        };
        match item.kind {
            kind::KEY => {
                let words = self.word_count(c, id, i);
                let english = self.language(c, id, i) == Some(Language::English);
                if english && matches!(words, Some(12 | 24)) {
                    QrFit::Fits
                } else {
                    QrFit::SeedNotQr
                }
            }
            kind::CODEX32_SHARE => QrFit::Fits,
            kind::NOTE => match item.text(field::NOTE) {
                Some(note) => match contents::qr_note_over(note) {
                    0 => QrFit::Fits,
                    n => QrFit::NoteTooLong(n),
                },
                None => QrFit::Unwritable,
            },
            _ => match write_qr(&item) {
                Ok(_) => QrFit::Fits,
                Err(r) if r.why == Why::TooLong => QrFit::PassphraseTooLong,
                Err(_) => QrFit::Unwritable,
            },
        }
    }

    /// Costs what `qr_fit` costs, an Argon2id run at `exponent` and one to
    /// read the seal back. Item `i` of open vault `id` (a seed, a codex32
    /// share, a BIP-39 passphrase or a note) sealed as that item of a QR
    /// vault under `passphrase`: the code's bytes after the read-back. The
    /// item stays in the vault. [`QrError::NotFit`] with the reason
    /// `qr_fit` gives when it does not fit.
    pub fn qr_seal_item(
        &self,
        c: &Compute,
        id: VaultId,
        i: usize,
        passphrase: &str,
        exponent: u8,
        random: &[u8; QR_RANDOM_LEN],
    ) -> Result<Vec<u8>, QrError> {
        match self.qr_fit(c, id, i) {
            QrFit::Fits => {}
            QrFit::NotOffered if self.copy_out(id, i, &[]).is_none() => {
                return Err(QrError::NoItem);
            }
            other => return Err(QrError::NotFit(other)),
        }
        let held = self.copy_out(id, i, &[]).ok_or(QrError::NoItem)?;
        let item = item_record(&held).ok_or(QrError::NoItem)?;
        seal_record(c, &item, passphrase, exponent, random)
    }

    /// Costs what `fits` costs. Saves the note `keys` holds from a code
    /// (`Reveal::QrVault`) as a new note last in open vault `id`: its
    /// place. [`QrError::NotHeld`] when `keys` holds no note;
    /// [`QrError::NoRoom`] when the vault is read-only, not held or full;
    /// [`QrError::Short`] when this computer has not the memory to count.
    pub fn qr_save_note(
        &mut self,
        c: &Compute,
        keys: &Keys,
        id: VaultId,
    ) -> Result<usize, QrError> {
        let Some(HeldItem::Note(text)) = &keys.qr_item else {
            return Err(QrError::NotHeld);
        };
        let record = Record::new(kind::NOTE).with(field::NOTE, text.as_bytes());
        match self.fit(c, id, &[], &[&record]) {
            Room::Fits => {}
            Room::Full => return Err(QrError::NoRoom),
            Room::Short(len) => return Err(QrError::Short(len)),
        }
        self.insert(id, record).ok_or(QrError::NoRoom)
    }
}

/// What a held code holds, as a screen may know it: no secret in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QrHolds {
    /// The empty vault.
    Empty,
    /// A seed's words: how many.
    Seed {
        /// 12 or 24.
        words: u8,
    },
    /// A codex32 share and the fingerprint of its key.
    Share {
        /// The key's fingerprint.
        key: Fingerprint,
    },
    /// A BIP-39 passphrase and the fingerprint of its key.
    Passphrase {
        /// The key's fingerprint.
        key: Fingerprint,
    },
    /// A note.
    Note,
}

/// The one item a code was opened to, wiped when dropped.
pub(crate) enum HeldItem {
    Empty,
    Words(SecretText, u8),
    Share(SecretText, [u8; 4]),
    Passphrase(SecretText, [u8; 4]),
    Note(SecretText),
}

impl HeldItem {
    /// The item a record read from a QR vault's contents is.
    fn of(r: &Record) -> Option<HeldItem> {
        let text = |n: u8| r.text(n).map(SecretText::of);
        let key = |n: u8| -> Option<[u8; 4]> { r.field(n)?.try_into().ok() };
        Some(match r.kind {
            kind::KEY => {
                let m = records::words_of(r)?;
                let lang = m.language();
                let mut phrase = SecretText::new();
                for (k, &w) in m.indices().iter().enumerate() {
                    if k > 0 {
                        phrase.push(' ');
                    }
                    phrase.push_str(lang.word(w));
                }
                HeldItem::Words(phrase, u8::try_from(m.word_count()).ok()?)
            }
            kind::CODEX32_SHARE => {
                HeldItem::Share(text(field::CODEX32_STRING)?, key(field::CODEX32_KEY)?)
            }
            kind::PASSPHRASE => {
                HeldItem::Passphrase(text(field::PASSPHRASE)?, key(field::PASSPHRASE_KEY)?)
            }
            kind::NOTE => {
                let note = text(field::NOTE)?;
                if note.is_empty() {
                    HeldItem::Empty
                } else {
                    HeldItem::Note(note)
                }
            }
            _ => return None,
        })
    }

    fn holds(&self) -> QrHolds {
        match self {
            HeldItem::Empty => QrHolds::Empty,
            HeldItem::Words(_, n) => QrHolds::Seed { words: *n },
            HeldItem::Share(_, k) => QrHolds::Share {
                key: Fingerprint(*k),
            },
            HeldItem::Passphrase(_, k) => QrHolds::Passphrase {
                key: Fingerprint(*k),
            },
            HeldItem::Note(_) => QrHolds::Note,
        }
    }
}
