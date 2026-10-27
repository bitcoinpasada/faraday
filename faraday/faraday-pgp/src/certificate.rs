//! A public certificate read for a restore, and the key restored from it
//! and a paperkey's data (`docs/GPG-PAPERKEY.md` §4.3, §5).
//!
//! [`Certificate::read`] takes one armoured public key block that
//! [`read_public`](crate::read_public) takes, of the shape Faraday makes:
//! a version-4 EdDSALegacy key on Ed25519 with one such subkey that
//! signs, and at least one user ID. A key of another algorithm is named as
//! `gpg --list-keys` names it. Each signature is sorted by its issuer: the
//! key's own are kept to be made again, and those by other keys are set
//! aside and counted.
//!
//! [`restore`] joins the paperkey's entries to the certificate and proves
//! every byte of what it gives: each secret makes its key's public key,
//! and each signature of the key's in the certificate is the one Faraday
//! makes from the times, expiry and reason it states, byte for byte.
//! Ed25519 is deterministic, so a certificate Faraday made is made again
//! exactly; any other is refused. Nothing is taken on a parser's word.

use alloc::string::String;
use alloc::vec::Vec;

use zeroize::Zeroizing;

use crate::{
    ED25519_OID, EDDSA, Key, Reason, fingerprint_of, next_packet, paperkey, public_body,
    read_block_packets, sig, sub, tag,
};

/// Why a certificate, a paperkey or the pair is not restored
/// (`docs/GPG-PAPERKEY.md` §4.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Not one armoured public key block that Faraday takes as public.
    NotCertificate,
    /// A key of another algorithm, named as `gpg --list-keys` names it
    /// (`rsa3072`, `cv25519`, `nistp256`, …).
    Algorithm(String),
    /// A key, or a paperkey's entry, of another OpenPGP version.
    Version(u8),
    /// Not one Ed25519 primary key with one Ed25519 signing subkey.
    Shape,
    /// The certificate has no user ID.
    NoUserId,
    /// A paperkey's format version other than 0.
    FormatVersion(u8),
    /// The paperkey is for the key with this fingerprint.
    OtherKey([u8; 20]),
    /// The paperkey's subkey has this fingerprint.
    OtherSubkey([u8; 20]),
    /// The paperkey holds this many keys, where the certificate has 2.
    Count(usize),
    /// A secret part is protected by a passphrase.
    Protected,
    /// The whole data's CRC does not match.
    Crc,
    /// A secret part does not read as an unprotected Ed25519 seed.
    Unreadable,
    /// A secret does not make its key's public key.
    Mismatch,
    /// A signature of the key's is not the one Faraday makes.
    NotOurs,
}

/// Where a signature sits in the certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// After the primary key, before any user ID.
    Key,
    /// After user ID `n`.
    UserId(usize),
    /// After the subkey.
    Subkey,
}

/// Which of the key's two keys made a signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum By {
    Primary,
    Subkey,
}

/// A signature by the key, as the certificate states it.
#[derive(Debug, Clone)]
struct Own {
    place: Place,
    by: By,
    kind: u8,
    /// The packet's body, compared with the one made again.
    body: Vec<u8>,
    created: Option<u32>,
    expires: Option<u32>,
    flags: Option<u8>,
    reason: Option<(u8, Vec<u8>)>,
}

/// A certificate of Faraday's shape, read: public, all of it. What it
/// states (dates, expiry, revocation) is a claim until [`restore`] has
/// made every signature of the key's again.
#[derive(Debug, Clone)]
pub struct Certificate {
    primary: Vec<u8>,
    fingerprint: [u8; 20],
    subkey: Vec<u8>,
    subkey_fingerprint: [u8; 20],
    user_ids: Vec<String>,
    own: Vec<Own>,
    set_aside: usize,
}

/// A signature's fields, as far as a restore or a verification reads
/// them.
pub(crate) struct SigFields {
    pub(crate) kind: u8,
    pub(crate) created: Option<u32>,
    expires: Option<u32>,
    flags: Option<u8>,
    reason: Option<(u8, Vec<u8>)>,
    pub(crate) issuer_fp: Option<[u8; 20]>,
    pub(crate) issuer_id: Option<[u8; 8]>,
    /// The hashed area holds what a verification does not honour: a
    /// signature expiry, or a subpacket marked critical other than the
    /// creation time and the issuer's fingerprint or key ID (RFC 9580
    /// §5.2.3.7: a critical subpacket not understood makes the signature
    /// invalid; an issuer only names the key to try, so marking it
    /// critical changes nothing).
    pub(crate) unhonoured: bool,
}

/// The fields of a version-4 signature body that [`read_block_packets`]
/// has read to its end: the hashed area gives the time, the expiry, the
/// flags and the reason; either area the issuer.
pub(crate) fn sig_fields(body: &[u8]) -> Option<SigFields> {
    let mut f = SigFields {
        kind: *body.get(1)?,
        created: None,
        expires: None,
        flags: None,
        reason: None,
        issuer_fp: None,
        issuer_id: None,
        unhonoured: false,
    };
    let hashed_len = usize::from(u16::from_be_bytes([*body.get(4)?, *body.get(5)?]));
    let hashed = body.get(6..6 + hashed_len)?;
    let at = 6 + hashed_len;
    let unhashed_len = usize::from(u16::from_be_bytes([*body.get(at)?, *body.get(at + 1)?]));
    let unhashed = body.get(at + 2..at + 2 + unhashed_len)?;
    for (area, is_hashed) in [(hashed, true), (unhashed, false)] {
        let mut r = 0usize;
        while r < area.len() {
            let first = usize::from(area[r]);
            let (n, head) = match first {
                0..=191 => (first, 1),
                192..=254 => (
                    ((first - 192) << 8) + usize::from(*area.get(r + 1)?) + 192,
                    2,
                ),
                _ => {
                    let s = area.get(r + 1..r + 5)?;
                    (u32::from_be_bytes([s[0], s[1], s[2], s[3]]) as usize, 5)
                }
            };
            let sp = area.get(r + head..r + head + n)?;
            let (&kind, data) = sp.split_first()?;
            if is_hashed
                && (kind & 0x7f == SIG_EXPIRES
                    || (kind & 0x80 != 0
                        && !matches!(
                            kind & 0x7f,
                            sub::CREATED | sub::ISSUER | sub::ISSUER_FINGERPRINT
                        )))
            {
                f.unhonoured = true;
            }
            match (kind & 0x7f, is_hashed) {
                (sub::CREATED, true) => f.created = Some(u32::from_be_bytes(data.try_into().ok()?)),
                (sub::KEY_EXPIRES, true) => {
                    f.expires = Some(u32::from_be_bytes(data.try_into().ok()?));
                }
                (sub::KEY_FLAGS, true) => f.flags = data.first().copied(),
                (sub::REVOCATION_REASON, true) => {
                    let (&code, text) = data.split_first()?;
                    f.reason = Some((code, text.to_vec()));
                }
                (sub::ISSUER_FINGERPRINT, _) if data.len() == 21 && data[0] == 4 => {
                    f.issuer_fp = Some(data[1..].try_into().ok()?);
                }
                (sub::ISSUER, _) => f.issuer_id = Some(data.try_into().ok()?),
                _ => {}
            }
            r += head + n;
        }
    }
    Some(f)
}

/// The signature expiry subpacket (RFC 9580 §5.2.3.18).
const SIG_EXPIRES: u8 = 3;

/// The dotted form of an OID's bytes, as GnuPG prints a curve it does not
/// name.
fn oid_text(oid: &[u8]) -> String {
    use core::fmt::Write;
    let mut out = String::new();
    let Some((&first, rest)) = oid.split_first() else {
        return out;
    };
    let _ = write!(out, "{}.{}", first / 40, first % 40);
    let mut v = 0u64;
    for &b in rest {
        v = v << 7 | u64::from(b & 0x7f);
        if b & 0x80 == 0 {
            let _ = write!(out, ".{v}");
            v = 0;
        }
    }
    out
}

/// GnuPG's name for a curve's OID (`openpgp_oid_to_curve`), or `E_` and
/// the OID as GnuPG prints one it does not know.
fn curve_name(oid: &[u8]) -> String {
    let name = match oid {
        [0x2B, 0x06, 0x01, 0x04, 0x01, 0xDA, 0x47, 0x0F, 0x01] => "ed25519",
        [0x2B, 0x06, 0x01, 0x04, 0x01, 0x97, 0x55, 0x01, 0x05, 0x01] => "cv25519",
        [0x2B, 0x65, 0x71] => "ed448",
        [0x2B, 0x65, 0x6F] => "cv448",
        [0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x03, 0x01, 0x07] => "nistp256",
        [0x2B, 0x81, 0x04, 0x00, 0x22] => "nistp384",
        [0x2B, 0x81, 0x04, 0x00, 0x23] => "nistp521",
        [0x2B, 0x24, 0x03, 0x03, 0x02, 0x08, 0x01, 0x01, 0x07] => "brainpoolP256r1",
        [0x2B, 0x24, 0x03, 0x03, 0x02, 0x08, 0x01, 0x01, 0x0B] => "brainpoolP384r1",
        [0x2B, 0x24, 0x03, 0x03, 0x02, 0x08, 0x01, 0x01, 0x0D] => "brainpoolP512r1",
        [0x2B, 0x81, 0x04, 0x00, 0x0A] => "secp256k1",
        _ => {
            let mut s = String::from("E_");
            s.push_str(&oid_text(oid));
            return s;
        }
    };
    String::from(name)
}

/// The bit count of the MPI at `at` in `body`.
fn mpi_bits(body: &[u8], at: usize) -> u16 {
    match body.get(at..at + 2) {
        Some(b) => u16::from_be_bytes([b[0], b[1]]),
        None => 0,
    }
}

/// `None` for a key of Faraday's algorithm, EdDSALegacy on Ed25519;
/// otherwise the refusal, naming the algorithm as GnuPG 2.4's
/// `pubkey_string` does.
fn algorithm_refusal(body: &[u8]) -> Option<Refusal> {
    use core::fmt::Write;
    let version = *body.first()?;
    if version != 4 {
        return Some(Refusal::Version(version));
    }
    let alg = *body.get(5)?;
    let mut name = String::new();
    match alg {
        1..=3 => {
            let _ = write!(name, "rsa{}", mpi_bits(body, 6));
        }
        16 | 20 => {
            let _ = write!(name, "elg{}", mpi_bits(body, 6));
        }
        17 => {
            let _ = write!(name, "dsa{}", mpi_bits(body, 6));
        }
        18 | 19 | 22 => {
            let len = usize::from(*body.get(6)?);
            let oid = body.get(7..7 + len)?;
            if alg == EDDSA && body.get(6..7 + len) == Some(&ED25519_OID[..]) {
                return None;
            }
            name = curve_name(oid);
        }
        25 => name.push_str("cv25519"),
        26 => name.push_str("cv448"),
        28 => name.push_str("ed448"),
        _ => {
            let _ = write!(name, "unknown_{alg}");
        }
    }
    // An Ed25519 key under another algorithm number is not of Faraday's
    // shape, and is not "ed25519 keys are not restored".
    if name == "ed25519" || alg == 27 {
        return Some(Refusal::Shape);
    }
    Some(Refusal::Algorithm(name))
}

/// When a key was made, from its public key body.
fn created_of(body: &[u8]) -> u32 {
    u32::from_be_bytes([body[1], body[2], body[3], body[4]])
}

impl Certificate {
    /// Costs a SHA-1 of each key. `bytes` read as a certificate to restore
    /// from (`docs/GPG-PAPERKEY.md` §4.3, steps 1 to 4).
    pub fn read(bytes: &[u8]) -> Result<Certificate, Refusal> {
        let block = read_block_packets(bytes).ok_or(Refusal::NotCertificate)?;
        if !block.crc_ok || block.armor.kind != crate::Armor::PublicKey {
            return Err(Refusal::NotCertificate);
        }
        let body = |r: &core::ops::Range<usize>| block.data[r.clone()].to_vec();
        let mut primary: Option<Vec<u8>> = None;
        let mut subkeys: Vec<Vec<u8>> = Vec::new();
        let mut user_ids: Vec<String> = Vec::new();
        let mut sigs: Vec<(Place, Vec<u8>)> = Vec::new();
        let mut place = Place::Key;
        for (t, r) in &block.packets {
            match *t {
                tag::PUBLIC_KEY if primary.is_none() => primary = Some(body(r)),
                // A second key in the block: a keyring, not one key.
                tag::PUBLIC_KEY => return Err(Refusal::Shape),
                tag::PUBLIC_SUBKEY => {
                    subkeys.push(body(r));
                    place = Place::Subkey;
                }
                tag::USER_ID => {
                    if place == Place::Subkey {
                        return Err(Refusal::Shape);
                    }
                    user_ids.push(String::from_utf8(body(r)).map_err(|_| Refusal::NotCertificate)?);
                    place = Place::UserId(user_ids.len() - 1);
                }
                tag::SIGNATURE => sigs.push((place, body(r))),
                _ => return Err(Refusal::NotCertificate),
            }
        }
        let primary = primary.ok_or(Refusal::NotCertificate)?;
        for key in core::iter::once(&primary).chain(subkeys.iter()) {
            if let Some(refusal) = algorithm_refusal(key) {
                return Err(refusal);
            }
        }
        let [subkey] = <[Vec<u8>; 1]>::try_from(subkeys).map_err(|_| Refusal::Shape)?;
        if user_ids.is_empty() {
            return Err(Refusal::NoUserId);
        }
        let fingerprint = fingerprint_of(&primary);
        let subkey_fingerprint = fingerprint_of(&subkey);
        let mut own = Vec::new();
        let mut set_aside = 0;
        for (place, body) in sigs {
            let f = sig_fields(&body).ok_or(Refusal::NotCertificate)?;
            let is = |fp: &[u8; 20]| match (f.issuer_fp, f.issuer_id) {
                (Some(i), _) => &i == fp,
                (None, Some(id)) => id[..] == fp[12..],
                (None, None) => false,
            };
            let by = if is(&fingerprint) {
                By::Primary
            } else if is(&subkey_fingerprint) {
                By::Subkey
            } else {
                set_aside += 1;
                continue;
            };
            own.push(Own {
                place,
                by,
                kind: f.kind,
                body,
                created: f.created,
                expires: f.expires,
                flags: f.flags,
                reason: f.reason,
            });
        }
        let cert = Certificate {
            primary,
            fingerprint,
            subkey,
            subkey_fingerprint,
            user_ids,
            own,
            set_aside,
        };
        // The subkey's newest binding by the primary key gives it the
        // signing flag, and no other.
        let binding = cert
            .self_signatures()
            .filter(|s| s.place == Place::Subkey && s.kind == sig::SUBKEY_BINDING)
            .max_by_key(|s| s.created);
        match binding.and_then(|b| b.flags) {
            Some(0x02) => Ok(cert),
            _ => Err(Refusal::Shape),
        }
    }

    fn self_signatures(&self) -> impl Iterator<Item = &Own> {
        self.own.iter().filter(|s| s.by == By::Primary)
    }

    /// The primary key's fingerprint.
    pub fn fingerprint(&self) -> [u8; 20] {
        self.fingerprint
    }

    /// The signing subkey's fingerprint.
    pub fn subkey_fingerprint(&self) -> [u8; 20] {
        self.subkey_fingerprint
    }

    /// When the primary key was made.
    pub fn created(&self) -> u32 {
        created_of(&self.primary)
    }

    /// When the subkey was made.
    pub fn subkey_created(&self) -> u32 {
        created_of(&self.subkey)
    }

    /// The user IDs, in their order.
    pub fn user_ids(&self) -> &[String] {
        &self.user_ids
    }

    /// The signatures by other keys, which a restore does not keep.
    pub fn set_aside(&self) -> usize {
        self.set_aside
    }

    /// When the newest self-signature says it was made, if any.
    pub fn certified_at(&self) -> Option<u32> {
        self.self_signatures()
            .filter(|s| matches!(s.place, Place::UserId(_) | Place::Subkey))
            .filter_map(|s| s.created)
            .max()
    }

    /// The expiry, seconds after creation, that the newest user ID
    /// self-signature states; 0 for none.
    pub fn expiry(&self) -> u32 {
        self.self_signatures()
            .filter(|s| matches!(s.place, Place::UserId(_)))
            .max_by_key(|s| s.created)
            .and_then(|s| s.expires)
            .unwrap_or(0)
    }

    /// The primary key's public key body.
    pub(crate) fn primary_body(&self) -> &[u8] {
        &self.primary
    }

    /// The signing subkey's public key body.
    pub(crate) fn subkey_body(&self) -> &[u8] {
        &self.subkey
    }

    /// The subkey's newest binding by the primary key: its signature body
    /// and the expiry it states, seconds after the subkey was made (0 or
    /// none for none).
    pub(crate) fn subkey_binding(&self) -> Option<(&[u8], Option<u32>)> {
        self.self_signatures()
            .filter(|s| s.place == Place::Subkey && s.kind == sig::SUBKEY_BINDING)
            .max_by_key(|s| s.created)
            .map(|s| (s.body.as_slice(), s.expires))
    }

    /// Whether the certificate carries a revocation by the key of either
    /// key: a key revocation, or a subkey revocation.
    pub(crate) fn carries_revocation(&self) -> bool {
        self.own
            .iter()
            .any(|s| s.kind == sig::KEY_REVOCATION || s.kind == sig::SUBKEY_REVOCATION)
    }

    /// When the earliest revocation by the key says it was made.
    pub fn revoked_at(&self) -> Option<u32> {
        self.self_signatures()
            .filter(|s| s.place == Place::Key && s.kind == sig::KEY_REVOCATION)
            .filter_map(|s| s.created)
            .min()
    }
}

/// The key restored, with the record's public values (`docs/GPG-PAPERKEY.md`
/// §5.1). The key's seeds and the kept revocation are wiped on drop.
pub struct Restored {
    /// The key: its seeds, dates, user IDs and the newest expiry.
    pub key: Key,
    /// The certificate the record keeps: made at the newest
    /// self-signature's time, and equal to the one the certificate holds.
    pub certificate: Vec<u8>,
    /// The revocation certificate Make keeps: at the creation time, with
    /// no reason and no text.
    pub revocation: Zeroizing<Vec<u8>>,
    /// The time of the earliest revocation by the key in the certificate.
    pub revoked: Option<u32>,
    /// The newest self-signature's time.
    pub certified_at: u32,
    /// The signatures by other keys, not kept.
    pub set_aside: usize,
}

/// An unprotected secret part's Ed25519 seed (`docs/GPG-PAPERKEY.md`
/// §4.3, steps 7 and 10): the usage octet 0, an MPI of at most 256 bits
/// written in full, nothing after its checksum, and the checksum the
/// sum of the MPI's bytes.
fn seed_of(secret: &[u8]) -> Result<Zeroizing<[u8; 32]>, Refusal> {
    let (&usage, rest) = secret.split_first().ok_or(Refusal::Unreadable)?;
    if usage != 0 {
        return Err(Refusal::Protected);
    }
    if rest.len() < 4 {
        return Err(Refusal::Unreadable);
    }
    let bits = usize::from(u16::from_be_bytes([rest[0], rest[1]]));
    let n = bits.div_ceil(8);
    if bits > 256 || rest.len() != 2 + n + 2 {
        return Err(Refusal::Unreadable);
    }
    let m = &rest[2..2 + n];
    if bits > 0 && (m[0] == 0 || n * 8 - m[0].leading_zeros() as usize != bits) {
        return Err(Refusal::Unreadable);
    }
    let sum = rest[..2 + n]
        .iter()
        .fold(0u16, |s, b| s.wrapping_add(u16::from(*b)));
    if sum.to_be_bytes() != rest[2 + n..] {
        return Err(Refusal::Unreadable);
    }
    let mut seed = Zeroizing::new([0u8; 32]);
    seed[32 - n..].copy_from_slice(m);
    Ok(seed)
}

/// The signature bodies of a certificate Faraday made: each user ID's
/// self-signature in order, then the subkey's binding.
fn signature_bodies(cert: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < cert.len() {
        let (t, from, to) = next_packet(cert, at);
        if t == tag::SIGNATURE {
            out.push(cert[from..to].to_vec());
        }
        at = to;
    }
    out
}

/// The self-signatures Faraday makes at `at` with `expiry`, kept so that
/// each pair is made once.
struct Made {
    at: u32,
    expiry: u32,
    bodies: Vec<Vec<u8>>,
}

/// Costs an Ed25519 signature for each user ID and two for the subkey,
/// for each time and expiry not made before. Faraday's self-signatures at
/// `at` with `expiry`.
fn made_at<'a>(key: &mut Key, cache: &'a mut Vec<Made>, at: u32, expiry: u32) -> &'a [Vec<u8>] {
    let i = match cache.iter().position(|m| m.at == at && m.expiry == expiry) {
        Some(i) => i,
        None => {
            key.expiry = expiry;
            cache.push(Made {
                at,
                expiry,
                bodies: signature_bodies(&key.certificate(at)),
            });
            cache.len() - 1
        }
    };
    &cache[i].bodies
}

/// The expiry, seconds after the primary's creation, that a binding's
/// subkey expiry `b` stands for: Faraday counts a subkey's from its own
/// creation (`Key::certificate`).
fn expiry_of_binding(cert: &Certificate, b: Option<u32>) -> Option<u32> {
    match b {
        None => Some(0),
        Some(b) => b
            .checked_add(cert.subkey_created())?
            .checked_sub(cert.created()),
    }
}

/// Costs an Ed25519 signature for each signature of the key's, and more
/// for each self-signature's time. Every signature of the key's in
/// `cert` made again with `key` and compared byte for byte
/// (`docs/GPG-PAPERKEY.md` §4.3, step 12); the newest self-signatures'
/// time and expiry, with each of those present, and the earliest
/// revocation's time.
fn rebuild(cert: &Certificate, key: &mut Key) -> Result<(u32, u32, Option<u32>), Refusal> {
    let mut cache: Vec<Made> = Vec::new();
    let n = cert.user_ids.len();
    // Each user ID self-signature's time and expiry, to read a binding
    // made with it.
    let stated: Vec<(u32, u32)> = cert
        .self_signatures()
        .filter(|s| matches!(s.place, Place::UserId(_)))
        .filter_map(|s| Some((s.created?, s.expires.unwrap_or(0))))
        .collect();
    let mut newest: Option<(u32, u32)> = None;
    let mut seen: Vec<(u32, u32, usize)> = Vec::new();
    let mut revoked: Option<u32> = None;
    for s in &cert.own {
        let at = s.created.ok_or(Refusal::NotOurs)?;
        if s.by != By::Primary {
            return Err(Refusal::NotOurs);
        }
        let (expiry, index) = match (s.place, s.kind) {
            (Place::UserId(k), sig::POSITIVE_CERTIFICATION) => (s.expires.unwrap_or(0), k),
            (Place::Subkey, sig::SUBKEY_BINDING) => {
                let e = match stated.iter().find(|(t, _)| *t == at) {
                    Some((_, e)) => *e,
                    None => expiry_of_binding(cert, s.expires).ok_or(Refusal::NotOurs)?,
                };
                (e, n)
            }
            (Place::Key, sig::KEY_REVOCATION) => {
                let (code, text) = s.reason.as_ref().ok_or(Refusal::NotOurs)?;
                let reason = Reason::from_code(*code).ok_or(Refusal::NotOurs)?;
                let text = core::str::from_utf8(text).map_err(|_| Refusal::NotOurs)?;
                let made = key.revocation(at, reason, text);
                let (_, from, to) = next_packet(&made, 0);
                if made[from..to] != s.body[..] {
                    return Err(Refusal::NotOurs);
                }
                revoked = Some(revoked.map_or(at, |r| r.min(at)));
                continue;
            }
            _ => return Err(Refusal::NotOurs),
        };
        let bodies = made_at(key, &mut cache, at, expiry);
        if bodies.get(index) != Some(&s.body) {
            return Err(Refusal::NotOurs);
        }
        seen.push((at, expiry, index));
        if newest.is_none_or(|(t, _)| at > t) {
            newest = Some((at, expiry));
        }
    }
    let (at, expiry) = newest.ok_or(Refusal::NotOurs)?;
    // The certificate the record keeps is one the certificate holds whole:
    // every user ID's self-signature and the binding at that time.
    if (0..=n).any(|i| !seen.contains(&(at, expiry, i))) {
        return Err(Refusal::NotOurs);
    }
    key.expiry = expiry;
    Ok((at, expiry, revoked))
}

/// Costs two Ed25519 public keys, and an Ed25519 signature for each
/// signature of the key's in the certificate and each made for the
/// record. The key whose certificate is `cert` and whose paperkey's data
/// is `data`, restored (`docs/GPG-PAPERKEY.md` §4.3, steps 6 to 12): the
/// entries are the certificate's two keys, each secret unprotected and
/// making its key's public key, and every signature of the key's made
/// again and equal.
pub fn restore(cert: &Certificate, data: &[u8]) -> Result<Restored, Refusal> {
    let entries = paperkey::entries(data)?;
    if entries.len() != 2 {
        return Err(Refusal::Count(entries.len()));
    }
    if entries[0].fingerprint != cert.fingerprint {
        return Err(Refusal::OtherKey(entries[0].fingerprint));
    }
    if entries[1].fingerprint != cert.subkey_fingerprint {
        return Err(Refusal::OtherSubkey(entries[1].fingerprint));
    }
    let primary_seed = seed_of(entries[0].secret)?;
    let subkey_seed = seed_of(entries[1].secret)?;
    let (created, subkey_created) = (cert.created(), cert.subkey_created());
    if public_body(&primary_seed, created) != cert.primary
        || public_body(&subkey_seed, subkey_created) != cert.subkey
    {
        return Err(Refusal::Mismatch);
    }
    let mut key = Key {
        primary_seed: *primary_seed,
        created,
        subkey_seed: *subkey_seed,
        subkey_created,
        user_ids: cert.user_ids.clone(),
        expiry: 0,
    };
    let (at, _, revoked) = rebuild(cert, &mut key)?;
    let certificate = key.certificate(at);
    let revocation = Zeroizing::new(key.revocation(created, Reason::Unspecified, ""));
    Ok(Restored {
        key,
        certificate,
        revocation,
        revoked,
        certified_at: at,
        set_aside: cert.set_aside,
    })
}
