//! OpenPGP keys: a key made from seeds, and what is made from it — the
//! certificate, detached signatures, revocations and the paperkey text —
//! and a key restored from its paperkey and certificate ([`Restore`]).
//! The packets are faraday-pgp's.

use faraday_crypto::work::{Compute, Work};
pub use faraday_pgp::paperkey::LineError;
use faraday_pgp::paperkey::{self, LINE_BYTES, Progress};
use faraday_pgp::{Armor, Certificate, Hash, Reason, Refusal, armor};
use faraday_vault::Record;
use faraday_vault::kdbx::items::GpgKey;
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
        key: MadeKey::of(key_record(&key, &certificate, Some(&rev), None)),
        facts: facts_of(&key),
        certificate,
    }
}

/// The type 7 record of `key` (`docs/VAULT.md` §7), its fields in the
/// order Make writes them: the seeds and dates, the user IDs, the expiry,
/// the certificate, the kept revocation and, for a key revoked, when.
fn key_record(key: &Key, certificate: &[u8], rev: Option<&[u8]>, revoked: Option<u32>) -> Record {
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
    if let Some(rev) = rev {
        r.push(f::REVOCATION, rev);
    }
    if let Some(at) = revoked {
        r.push(f::REVOKED, &at.to_le_bytes());
    }
    r
}

/// Costs Ed25519 signatures. The type 7 record of a GPG key as a KDBX
/// vault holds it (`docs/VAULT.md` §7, `gpg`), or `None` when the key is
/// not in Faraday's own shape. The secret key must be what [`gpg_key_of`]
/// writes (`faraday_pgp::read_secret_at`), a public certificate kept with
/// it must be exactly the certificate the key makes at its signing time,
/// a revocation certificate must be exactly the one signature the key
/// makes for the time, reason and text it states, and the date a key was
/// revoked must be an RFC 3339 time in UTC to the second. A certificate missing is
/// made again at the time the key's self-signatures say.
pub fn record_of(g: &GpgKey) -> Option<Record> {
    let (key, at) = faraday_pgp::read_secret_at(g.key.as_bytes())?;
    let certificate = match &g.public {
        Some(text) => {
            let packets = public_packets(text)?;
            let read = Certificate::read(text.as_bytes()).ok()?;
            if read.fingerprint() != key.fingerprint()
                || read.subkey_fingerprint() != key.subkey_fingerprint()
                || packets != key.certificate(faraday_pgp::signature_time(&packets)?)
            {
                return None;
            }
            packets
        }
        None => key.certificate(at),
    };
    let revocation = match &g.revocation {
        Some(text) => {
            let packets = public_packets(text)?;
            let (t, reason, why) = revocation_parts(&packets)?;
            if packets != key.revocation(t, reason, &why) {
                return None;
            }
            Some(packets)
        }
        None => None,
    };
    let revoked = match &g.revoked {
        Some(t) => Some(unix_of(t)?),
        None => None,
    };
    Some(key_record(
        &key,
        &certificate,
        revocation.as_deref(),
        revoked,
    ))
}

/// Costs Ed25519 signatures. The key a type 7 record holds as a KDBX
/// vault's entry fields: the armoured secret key (self-signatures made at
/// `at`, which a certificate states: `faraday_pgp::signature_time`), the
/// certificate and the revocation certificate armoured as public blocks,
/// and the date revoked in RFC 3339. `None` for a record without both
/// seeds.
pub fn gpg_key_of(r: &Record, at: u32) -> Option<GpgKey> {
    let key = crate::vault::gpg_key(r)?;
    // The armoured block is bytes in a wiping buffer; its ASCII moves into
    // the wiping text without a copy.
    let mut armored = faraday_pgp::armor_secret(&key, at);
    let text = String::from_utf8(core::mem::take(&mut *armored)).ok()?;
    let public = r.field(f::CERTIFICATE).map(|c| armor(Armor::PublicKey, c));
    let revocation = r
        .field(f::REVOCATION)
        .map(|c| Zeroizing::new(armor(Armor::PublicKey, c)));
    let revoked = r
        .field(f::REVOKED)
        .and_then(|b| Some(utc_of(u32::from_le_bytes(b.try_into().ok()?))));
    Some(GpgKey {
        title: key.user_ids.first().cloned().unwrap_or_default(),
        key: Zeroizing::new(text),
        public,
        revocation,
        revoked,
    })
}

/// The time, reason and text a revocation certificate states: one
/// signature packet, its creation time first, then the reason subpacket.
fn revocation_parts(packets: &[u8]) -> Option<(u32, Reason, String)> {
    let first = usize::from(*packets.get(1)?);
    let (len, head) = match first {
        0..=191 => (first, 2),
        192..=223 => (((first - 192) << 8) + usize::from(*packets.get(2)?) + 192, 3),
        _ => return None,
    };
    if packets.first()? & 0x3f != 2 || packets.len() != head + len {
        return None;
    }
    let body = &packets[head..];
    // Version, type, algorithm, hash, hashed length, then the creation
    // time subpacket (length 5, type 2, four bytes).
    let at = u32::from_be_bytes(body.get(8..12)?.try_into().ok()?);
    let rest = body.get(12..)?;
    let n = usize::from(*rest.first()?);
    if n > 191 || *rest.get(1)? != 29 {
        return None;
    }
    let why = rest.get(2..1 + n)?;
    let (&code, text) = why.split_first()?;
    Some((
        at,
        Reason::from_code(code)?,
        String::from_utf8(text.to_vec()).ok()?,
    ))
}

/// The packets of an armoured public block, any packets (the revocation
/// certificate is one signature).
fn public_packets(text: &str) -> Option<Vec<u8>> {
    let packets = faraday_pgp::dearmor(
        b"-----BEGIN PGP PUBLIC KEY BLOCK-----",
        b"-----END PGP PUBLIC KEY BLOCK-----",
        text.as_bytes(),
    )?;
    Some(packets.to_vec())
}

/// Days since 1970-01-01 of a civil date (proleptic Gregorian).
fn days_of(y: i64, m: i64, d: i64) -> i64 {
    let y = y - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// A Unix time as an RFC 3339 time in UTC to the second:
/// `2026-10-28T14:02:00Z`.
fn utc_of(unix: u32) -> String {
    let (day, sec) = (unix / 86_400, unix % 86_400);
    format!(
        "{}T{:02}:{:02}:{:02}Z",
        faraday_pgp::utc_date(u64::from(day) * 86_400),
        sec / 3600,
        sec % 3600 / 60,
        sec % 60
    )
}

/// The Unix time of an RFC 3339 time in UTC to the second, as [`utc_of`]
/// writes it, if it is one and is a valid date within a `u32`.
fn unix_of(text: &str) -> Option<u32> {
    let b = text.as_bytes();
    let num = |r: core::ops::Range<usize>| -> Option<i64> {
        let s = b.get(r)?;
        s.iter()
            .all(u8::is_ascii_digit)
            .then(|| s.iter().fold(0i64, |v, d| v * 10 + i64::from(d - b'0')))
    };
    if b.len() != 20
        || [
            (4, b'-'),
            (7, b'-'),
            (10, b'T'),
            (13, b':'),
            (16, b':'),
            (19, b'Z'),
        ]
        .iter()
        .any(|&(i, c)| b[i] != c)
    {
        return None;
    }
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, s) = (num(11..13)?, num(14..16)?, num(17..19)?);
    if !(1..=12).contains(&mo) || d < 1 || h > 23 || mi > 59 || s > 59 {
        return None;
    }
    let unix = days_of(y, mo, d) * 86_400 + h * 3600 + mi * 60 + s;
    let unix = u32::try_from(unix).ok()?;
    // A day past the month's end rolls over: the text must be the one
    // written for the time.
    (utc_of(unix) == text).then_some(unix)
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
                key: MadeKey::of(key_record(
                    &r.key,
                    &r.certificate,
                    Some(&r.revocation),
                    r.revoked,
                )),
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
