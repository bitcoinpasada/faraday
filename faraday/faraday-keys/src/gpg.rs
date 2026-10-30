//! OpenPGP keys: a key made from seeds, and what is made from it — the
//! certificate, detached signatures, revocations and the paperkey text —
//! and a key restored from its paperkey and certificate ([`Restore`]).
//! The packets are faraday-pgp's.

use faraday_crypto::work::{Compute, Work};
pub use faraday_pgp::paperkey::LineError;
use faraday_pgp::paperkey::{self, LINE_BYTES, Progress};
use faraday_pgp::{Armor, Certificate, Hash, Reason, Refusal, armor};
use faraday_vault::Record;
use faraday_vault::records::kind;
use zeroize::{Zeroize, Zeroizing};

use crate::vault::{GpgFacts, MadeKey, gpg_field as f};

/// An Ed25519 certification key and its signing subkey, from seeds; the
/// seeds are wiped when it drops.
pub(crate) use faraday_pgp::Key;

/// A key made here ([`create`]): its record, and what is public of it.
pub struct Made {
    /// The record every vault of the plan takes: the seeds, the dates, the
    /// user ID, the expiry, the certificate and the revocation certificate
    /// kept with it, as GnuPG 2.1 and later keep one.
    pub key: MadeKey,
    /// Its fingerprints, dates and user IDs.
    pub facts: GpgFacts,
    /// The public certificate (packets), signed when it was made.
    pub certificate: Vec<u8>,
}

/// Costs Ed25519 signatures. A new key from the fresh seeds `primary` and
/// `subkey`, made at `now` with one user ID and `expiry` (seconds after
/// creation, 0 for none), with its certificate and the revocation
/// certificate kept with it.
pub fn create(
    primary: &Zeroizing<[u8; 32]>,
    subkey: &Zeroizing<[u8; 32]>,
    now: u32,
    user_id: String,
    expiry: u32,
) -> Made {
    let key = make(primary, subkey, now, user_id, expiry);
    let certificate = certificate(&key, now);
    let rev = Zeroizing::new(revocation(&key, now, Reason::Unspecified, ""));
    Made {
        key: MadeKey::of(record_of(&key, &certificate, &rev, None)),
        facts: facts_of(&key),
        certificate,
    }
}

/// The type 7 record of `key` (`docs/VAULT.md` §7), its fields in the
/// order Make writes them: the seeds and dates, the user IDs, the expiry,
/// the certificate, the kept revocation and, for a key revoked, when.
fn record_of(key: &Key, certificate: &[u8], rev: &[u8], revoked: Option<u32>) -> Record {
    let mut r = Record::new(kind::GPG)
        .with(f::PRIMARY, &key.primary_seed)
        .with(f::CREATED, &key.created.to_le_bytes())
        .with(f::SUBKEY, &key.subkey_seed)
        .with(f::SUBKEY_CREATED, &key.subkey_created.to_le_bytes());
    for uid in &key.user_ids {
        r.push(f::USER_ID, uid.as_bytes());
    }
    r.push(f::EXPIRY, &key.expiry.to_le_bytes());
    r.push(f::CERTIFICATE, certificate);
    r.push(f::REVOCATION, rev);
    if let Some(at) = revoked {
        r.push(f::REVOKED, &at.to_le_bytes());
    }
    r
}

/// What is public of `key`.
pub(crate) fn facts_of(key: &Key) -> GpgFacts {
    GpgFacts {
        fingerprint: key.fingerprint(),
        subkey_fingerprint: key.subkey_fingerprint(),
        created: key.created,
        subkey_created: key.subkey_created,
        user_ids: key.user_ids.clone(),
        expiry: key.expiry,
    }
}

/// The key whose seeds are `primary` and `subkey`, made at `created` and
/// `subkey_created`, with `user_ids` and `expiry` (seconds after
/// creation, 0 for none); `None` unless both seeds are 32 bytes.
pub(crate) fn key_of(
    primary: &[u8],
    created: u32,
    subkey: &[u8],
    subkey_created: u32,
    user_ids: Vec<String>,
    expiry: u32,
) -> Option<Key> {
    if primary.len() != 32 || subkey.len() != 32 {
        return None;
    }
    let mut key = Key {
        primary_seed: [0; 32],
        created,
        subkey_seed: [0; 32],
        subkey_created,
        user_ids,
        expiry,
    };
    key.primary_seed.copy_from_slice(primary);
    key.subkey_seed.copy_from_slice(subkey);
    Some(key)
}

/// A new key from fresh seeds, made at `now` with one user ID.
pub(crate) fn make(
    primary: &Zeroizing<[u8; 32]>,
    subkey: &Zeroizing<[u8; 32]>,
    now: u32,
    user_id: String,
    expiry: u32,
) -> Key {
    Key {
        primary_seed: **primary,
        created: now,
        subkey_seed: **subkey,
        subkey_created: now,
        user_ids: vec![user_id],
        expiry,
    }
}

/// The public certificate (packets) of `key`, signed at `at`. Costs
/// Ed25519 signatures.
pub(crate) fn certificate(key: &Key, at: u32) -> Vec<u8> {
    key.certificate(at)
}

/// A revocation certificate (packets) for `key`. Costs an Ed25519
/// signature.
pub(crate) fn revocation(key: &Key, at: u32, reason: Reason, text: &str) -> Vec<u8> {
    key.revocation(at, reason, text)
}

/// A revocation certificate as the file GnuPG keeps it in.
pub(crate) fn revocation_file(key: &Key, revocation: &[u8]) -> String {
    key.revocation_file(revocation)
}

/// A detached signature (packets) over `data` by the signing subkey, at
/// `at`, with SHA-512. Costs an Ed25519 signature.
pub(crate) fn sign(key: &Key, data: &[u8], at: u32) -> Vec<u8> {
    key.sign(data, at, Hash::Sha512)
}

/// The signature over `data`, armoured.
pub(crate) fn sign_armored(key: &Key, data: &[u8], at: u32) -> String {
    armor(Armor::Signature, &sign(key, data, at))
}

/// The paperkey text of the key: secret, made only to be shown.
pub(crate) fn paperkey(key: &Key) -> Zeroizing<String> {
    key.paperkey()
}

/// The most bytes a restore holds: a paperkey of Faraday's is 121, and a
/// line typed may run past where the data ends before it is refused.
const RESTORE_CAP: usize = 256;

/// Why a line typed is not taken (`docs/GPG-PAPERKEY.md` §4.4). Public.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineRefused {
    /// The line does not read: its characters, its number or its CRC.
    Line(LineError),
    /// The line reads and what it says is refused.
    Paperkey(Refusal),
}

/// Why a paperkey read from a code is not taken. Public.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeRefused {
    /// Neither of paperkey's forms: no line numbered 1, and not its raw
    /// form.
    NotPaperkey,
    /// Line `n` of the code is refused, as it would be typed.
    Line(u32, LineRefused),
    /// The data, read whole, is refused: its CRC or what it says.
    Paperkey(Refusal),
    /// The code ends before the whole data's CRC ("CRC of secret is
    /// missing").
    Short,
    /// Line `n` follows the whole data's CRC.
    After(u32),
}

/// Costs a CRC-24 over each line and over the whole. Whether a code is a
/// paperkey, of any key, read as **Scan the paperkey** reads one: its
/// base16 lines whole, or a first line numbered 1 that holds its CRC and
/// starts with format 0 and key version 4 (a paperkey with a later line
/// mistyped is still one), or its raw form whole. Secret key material, so
/// no scanner files it (F167); what was read is wiped here. Public.
pub fn is_paperkey(bytes: &[u8]) -> bool {
    if bytes.starts_with(&[0, 4]) {
        return paperkey::read_raw(bytes).is_ok_and(|d| paperkey::entries(&d).is_ok());
    }
    let Ok(text) = core::str::from_utf8(bytes) else {
        return false;
    };
    if paperkey::read_text(text).is_ok() {
        return true;
    }
    let first = text
        .split('\n')
        .map(str::trim_start)
        .find(|t| !t.is_empty() && !t.starts_with('#'));
    first.is_some_and(|t| {
        t.starts_with("1:") && paperkey::line(1, t).is_ok_and(|l| l.bytes.starts_with(&[0, 4]))
    })
}

/// A paperkey being typed back against its public certificate, line by
/// line (`docs/GPG-PAPERKEY.md` §6): each line ended is checked at once by
/// its CRC and by what the data's headers say, and its bytes kept here, in
/// memory that is wiped, or refused and nothing kept. What it tells is
/// public: the lines checked, the count, the fingerprints read. The lines
/// checked are never shown again; [`Restore::finish`] makes the key.
pub struct Restore {
    certificate: Certificate,
    /// The data of the lines checked, sized once.
    data: Zeroizing<Vec<u8>>,
    /// How many bytes each checked data line holds, in order.
    lines: Vec<usize>,
    /// What the data's headers have said so far.
    progress: Progress,
    /// The whole data's CRC line is checked.
    done: bool,
}

impl Restore {
    /// A restore of the key `certificate` is for, no line typed yet.
    pub fn new(certificate: Certificate) -> Restore {
        Restore {
            certificate,
            data: Zeroizing::new(Vec::with_capacity(RESTORE_CAP)),
            lines: Vec::new(),
            progress: Progress::default(),
            done: false,
        }
    }

    /// The certificate the key is restored against.
    pub fn certificate(&self) -> &Certificate {
        &self.certificate
    }

    /// The number of the line to type next, counting from 1; the CRC line
    /// included.
    pub fn next(&self) -> u32 {
        (self.lines.len() + 1 + usize::from(self.done)) as u32
    }

    /// The lines checked, the CRC line included.
    pub fn checked(&self) -> usize {
        self.lines.len() + usize::from(self.done)
    }

    /// Every line, the CRC line included, is checked.
    pub fn complete(&self) -> bool {
        self.done
    }

    /// How many lines the paperkey has, once its lengths are read: the
    /// lines checked, then 22 bytes a line for the rest, and the CRC line.
    pub fn lines(&self) -> Option<usize> {
        let total = self.progress.total?;
        let rest = total.saturating_sub(self.data.len());
        Some(self.lines.len() + rest.div_ceil(LINE_BYTES) + 1)
    }

    /// The bytes left to type before the CRC line, once the lengths are
    /// read: public, as the lengths are.
    pub fn left(&self) -> Option<usize> {
        Some(self.progress.total?.saturating_sub(self.data.len()))
    }

    /// The fingerprints the lines checked name, in order: public.
    pub fn fingerprints(&self) -> &[[u8; 20]] {
        &self.progress.fingerprints
    }

    /// Costs a CRC-24 over the line. Line `n`, as typed or printed
    /// (`docs/GPG-PAPERKEY.md` §4.5): taken when it is the next line, reads
    /// and its CRC matches, and the data it ends with passes the header
    /// checks (§4.3, steps 5 to 9); otherwise refused, and nothing of it
    /// kept.
    pub fn line(&mut self, n: u32, digits: &str) -> Result<(), LineRefused> {
        let next = self.next();
        if n != next || self.done {
            return Err(LineRefused::Line(LineError::Number { typed: n }));
        }
        let read = paperkey::line(n, digits).map_err(LineRefused::Line)?;
        if read.bytes.is_empty() {
            self.take_whole_crc(read.crc)
        } else {
            self.take_line_bytes(&read.bytes)
        }
        .map_err(LineRefused::Paperkey)
    }

    /// The bytes of a data line read, kept when the data they end with
    /// passes the header checks; otherwise nothing of them kept.
    fn take_line_bytes(&mut self, bytes: &[u8]) -> Result<(), Refusal> {
        let before = self.data.len();
        if before + bytes.len() > RESTORE_CAP {
            return Err(Refusal::Unreadable);
        }
        self.data.extend_from_slice(bytes);
        match paperkey::check(&self.data, &self.cert_keys(), false) {
            Ok(p) => {
                self.progress = p;
                self.lines.push(bytes.len());
                Ok(())
            }
            Err(e) => {
                self.cut(before);
                Err(e)
            }
        }
    }

    /// The whole data's CRC, then the data checked as all there.
    fn take_whole_crc(&mut self, crc: u32) -> Result<(), Refusal> {
        if crc != faraday_pgp::crc24(&self.data) {
            return Err(Refusal::Crc);
        }
        paperkey::check(&self.data, &self.cert_keys(), true)?;
        self.done = true;
        Ok(())
    }

    /// The certificate's two fingerprints, the primary key's first.
    fn cert_keys(&self) -> [[u8; 20]; 2] {
        [
            self.certificate.fingerprint(),
            self.certificate.subkey_fingerprint(),
        ]
    }

    /// Costs a CRC-24 over each line and over the whole. A paperkey read
    /// from a code (`docs/GPG-PAPERKEY.md` §11.2, P5): paperkey's base16
    /// lines, comment lines and all, or its raw form. Every line is checked
    /// as a line typed is, and the whole by its CRC and the headers; taken
    /// whole in place of the lines typed so far, or refused with nothing of
    /// it kept and the lines typed left as they were.
    pub fn paperkey_code(&mut self, bytes: &[u8]) -> Result<(), CodeRefused> {
        let mut read = Restore::new(self.certificate.clone());
        if bytes.starts_with(&[0, 4]) {
            let data = paperkey::read_raw(bytes).map_err(|e| match e {
                paperkey::ReadError::CrcMissing => CodeRefused::Short,
                _ => CodeRefused::Paperkey(Refusal::Crc),
            })?;
            for chunk in data.chunks(LINE_BYTES) {
                read.take_line_bytes(chunk).map_err(CodeRefused::Paperkey)?;
            }
            let crc = faraday_pgp::crc24(&data);
            read.take_whole_crc(crc).map_err(CodeRefused::Paperkey)?;
        } else {
            let text = core::str::from_utf8(bytes).map_err(|_| CodeRefused::NotPaperkey)?;
            for l in text.split('\n') {
                let l = l.strip_suffix('\r').unwrap_or(l);
                let t = l.trim_start();
                if t.is_empty() || t.starts_with('#') {
                    continue;
                }
                let n = read.next();
                // The first line of a paperkey is numbered 1.
                if n == 1 && !t.starts_with("1:") {
                    return Err(CodeRefused::NotPaperkey);
                }
                if read.done {
                    return Err(CodeRefused::After(n));
                }
                read.line(n, l).map_err(|e| CodeRefused::Line(n, e))?;
            }
            if read.next() == 1 {
                return Err(CodeRefused::NotPaperkey);
            }
            if !read.done {
                return Err(CodeRefused::Short);
            }
        }
        *self = read;
        Ok(())
    }

    /// The data cut back to its first `len` bytes, the rest wiped.
    fn cut(&mut self, len: usize) {
        self.data[len..].zeroize();
        self.data.truncate(len);
    }

    /// Line `n` and every line after it cleared, to be typed again
    /// (**Type line {n} again**).
    pub fn again(&mut self, n: u32) {
        let n = n.max(1) as usize;
        // The CRC line is the one after the data's.
        if n <= self.lines.len() + 1 {
            self.done = false;
        }
        let keep = (n - 1).min(self.lines.len());
        self.lines.truncate(keep);
        let len = self.lines.iter().sum();
        self.cut(len);
        self.progress = paperkey::check(&self.data, &self.cert_keys(), false).unwrap_or_default();
    }

    /// Every line forgotten and wiped, the certificate kept: what a lock
    /// leaves of a restore (`docs/GPG-PAPERKEY.md` §6).
    pub fn wipe(&mut self) {
        self.again(1);
    }

    /// Costs two Ed25519 public keys and an Ed25519 signature for each
    /// signature of the key's and each made for the record. The key
    /// restored from the lines checked (`docs/GPG-PAPERKEY.md` §4.3, steps
    /// 10 to 12), as Make gives one: its record, with the seeds, the
    /// certificate's dates and user IDs, the newest expiry, the certificate
    /// made again and the revocation Make keeps; refused while a line is
    /// left to type.
    pub fn finish(&self, c: &Compute) -> Result<Restored, Refusal> {
        c.count(Work::Sign);
        if !self.done {
            return Err(Refusal::Unreadable);
        }
        let r = faraday_pgp::restore(&self.certificate, &self.data)?;
        Ok(Restored {
            made: Made {
                key: MadeKey::of(record_of(&r.key, &r.certificate, &r.revocation, r.revoked)),
                facts: facts_of(&r.key),
                certificate: r.certificate.clone(),
            },
            revoked: r.revoked,
            set_aside: r.set_aside,
        })
    }
}

/// A key restored ([`Restore::finish`]).
pub struct Restored {
    /// The record, its facts and its certificate, as [`create`] gives them.
    pub made: Made,
    /// When the certificate's revocation by the key was made, if it has
    /// one: the record is marked revoked then.
    pub revoked: Option<u32>,
    /// The signatures by other keys, not kept.
    pub set_aside: usize,
}
