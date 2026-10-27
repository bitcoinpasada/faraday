//! A detached signature checked against a certificate of Faraday's shape
//! (RFC 9580 §5.2.4, as version 4 has it): what Faraday reads to take a
//! release's `SHA256SUMS` as the release's own (`docs/decisions/F134.md`).
//!
//! [`verify_detached`] takes one armoured signature block holding one
//! version-4 binary signature (type 0x00) by the certificate's signing
//! subkey, EdDSALegacy over SHA-256 or SHA-512, and checks it over the
//! data. It checks the certificate too: its subkey's newest binding is
//! verified with the primary key, and the binding must carry the
//! subkey's own primary-key binding (type 0x19, the "back signature"
//! RFC 9580 §5.2.1 requires of a signing subkey), verified with the
//! subkey; a certificate carrying a revocation is refused, and a
//! signature made before the subkey or after either key's expiry is
//! refused. Anything else is refused rather than read: another signature
//! type (a text-mode 0x01 signature included), version or algorithm, an
//! armour block without its checksum or with a wrong one, a hashed
//! signature expiry, a creation time outside the hashed area, or a
//! critical subpacket other than the creation time and the issuer's
//! fingerprint or key ID (which name a key and never decide validity).

use alloc::vec::Vec;

use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256, Sha512};

use crate::certificate::sig_fields;
use crate::{Armor, Certificate, ED25519_OID, EDDSA, Hash, read_block_packets, sig, tag};

/// Why a detached signature is not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unverified {
    /// Not one armoured signature block of one version-4 binary
    /// signature, EdDSALegacy over SHA-256 or SHA-512, with its creation
    /// time and nothing this does not honour.
    NotSignature,
    /// Made by another key than the certificate's signing subkey.
    OtherKey,
    /// The certificate carries a revocation.
    Revoked,
    /// Made before the subkey was, or after it or the primary key
    /// expired.
    OutOfTime,
    /// The signature, or the subkey's binding, does not verify.
    Bad,
}

/// Costs a hash of `data` and three Ed25519 verifications. Checks
/// `asc`, an armoured detached signature, over `data` by `cert`'s signing
/// subkey; returns when the signature says it was made.
pub fn verify_detached(cert: &Certificate, data: &[u8], asc: &[u8]) -> Result<u32, Unverified> {
    let block = read_block_packets(asc)
        .filter(|b| b.crc_ok && b.armor.kind == Armor::Signature)
        .ok_or(Unverified::NotSignature)?;
    let [(packet_tag, range)] = block.packets.as_slice() else {
        return Err(Unverified::NotSignature);
    };
    let body = &block.data[range.clone()];
    if *packet_tag != tag::SIGNATURE || body.get(1) != Some(&sig::BINARY) {
        return Err(Unverified::NotSignature);
    }
    let f = sig_fields(body).ok_or(Unverified::NotSignature)?;
    let created = f.created.ok_or(Unverified::NotSignature)?;
    if f.unhonoured {
        return Err(Unverified::NotSignature);
    }
    let subkey_fp = cert.subkey_fingerprint();
    let by_subkey = match (f.issuer_fp, f.issuer_id) {
        (Some(fp), _) => fp == subkey_fp,
        (None, Some(id)) => id[..] == subkey_fp[12..],
        (None, None) => false,
    };
    if !by_subkey {
        return Err(Unverified::OtherKey);
    }
    let subkey = bound_subkey(cert)?;
    if !within(cert, created) {
        return Err(Unverified::OutOfTime);
    }
    if !verifies(&subkey, &[data], body) {
        return Err(Unverified::Bad);
    }
    Ok(created)
}

/// Costs two Ed25519 verifications. The certificate's signing subkey's
/// point, once its newest binding verifies with the primary key, the
/// back signature the binding carries verifies with the subkey, and no
/// revocation is in the certificate.
fn bound_subkey(cert: &Certificate) -> Result<[u8; 32], Unverified> {
    if cert.carries_revocation() {
        return Err(Unverified::Revoked);
    }
    let primary = point(cert.primary_body()).ok_or(Unverified::Bad)?;
    let subkey = point(cert.subkey_body()).ok_or(Unverified::Bad)?;
    let (binding, _) = cert.subkey_binding().ok_or(Unverified::Bad)?;
    let p = cert.primary_body();
    let s = cert.subkey_body();
    let (pl, sl) = (len16(p)?, len16(s)?);
    let key: [&[u8]; 6] = [&[0x99], &pl, p, &[0x99], &sl, s];
    let back = embedded(binding).ok_or(Unverified::Bad)?;
    let back_ok = back.get(1) == Some(&sig::PRIMARY_BINDING) && verifies(&subkey, &key, back);
    if verifies(&primary, &key, binding) && back_ok {
        Ok(subkey)
    } else {
        Err(Unverified::Bad)
    }
}

/// The signature a binding embeds (subpacket 32), from either area: the
/// back signature is self-authenticating, and GnuPG writes it unhashed.
/// `None` for none, or more than one.
fn embedded(binding: &[u8]) -> Option<&[u8]> {
    let hashed_len = usize::from(u16::from_be_bytes([*binding.get(4)?, *binding.get(5)?]));
    let hashed = binding.get(6..6 + hashed_len)?;
    let at = 6 + hashed_len;
    let unhashed_len = usize::from(u16::from_be_bytes([
        *binding.get(at)?,
        *binding.get(at + 1)?,
    ]));
    let unhashed = binding.get(at + 2..at + 2 + unhashed_len)?;
    let mut found = None;
    for area in [hashed, unhashed] {
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
            if kind & 0x7f == crate::sub::EMBEDDED_SIGNATURE {
                if found.is_some() {
                    return None;
                }
                found = Some(data);
            }
            r += head + n;
        }
    }
    found
}

/// A key body's length as a key is hashed with it.
fn len16(body: &[u8]) -> Result<[u8; 2], Unverified> {
    u16::try_from(body.len())
        .map(u16::to_be_bytes)
        .map_err(|_| Unverified::Bad)
}

/// Whether `at` is no earlier than the subkey and before the primary
/// key's and the subkey's expiry, where they have one.
fn within(cert: &Certificate, at: u32) -> bool {
    let sub_made = cert.subkey_created();
    let sub_ends = match cert.subkey_binding().and_then(|(_, e)| e) {
        Some(e) if e > 0 => u64::from(sub_made) + u64::from(e),
        _ => u64::MAX,
    };
    let primary_ends = match cert.expiry() {
        0 => u64::MAX,
        e => u64::from(cert.created()) + u64::from(e),
    };
    at >= sub_made && u64::from(at) < sub_ends.min(primary_ends)
}

/// The Ed25519 point of an EdDSALegacy public key body: the curve's OID,
/// then an MPI of 263 bits, 0x40 and the 32 bytes.
fn point(body: &[u8]) -> Option<[u8; 32]> {
    if body.first() != Some(&4) || body.get(5) != Some(&EDDSA) {
        return None;
    }
    let rest = body.get(6..)?.strip_prefix(&ED25519_OID[..])?;
    let rest = rest.strip_prefix(&[0x01, 0x07, 0x40][..])?;
    rest.try_into().ok()
}

/// An MPI at `at` in `body`, at most 32 bytes, as 32 bytes with the
/// leading zeros it dropped put back; and where the next field starts.
fn mpi32(body: &[u8], at: usize) -> Option<([u8; 32], usize)> {
    let bits = usize::from(u16::from_be_bytes([*body.get(at)?, *body.get(at + 1)?]));
    let n = bits.div_ceil(8);
    if n > 32 {
        return None;
    }
    let bytes = body.get(at + 2..at + 2 + n)?;
    let mut out = [0u8; 32];
    out[32 - n..].copy_from_slice(bytes);
    Some((out, at + 2 + n))
}

/// Costs a hash of `prefix` and an Ed25519 verification. Whether the
/// version-4 signature `body` verifies with `key` over the parts of
/// `prefix`, in order, then its hashed fields and trailer.
fn verifies(key: &[u8; 32], prefix: &[&[u8]], body: &[u8]) -> bool {
    (|| {
        if body.first() != Some(&4) || body.get(2) != Some(&EDDSA) {
            return None;
        }
        let hash = match body.get(3)? {
            8 => Hash::Sha256,
            10 => Hash::Sha512,
            _ => return None,
        };
        let hashed_len = usize::from(u16::from_be_bytes([*body.get(4)?, *body.get(5)?]));
        let fields = body.get(..6 + hashed_len)?;
        let at = 6 + hashed_len;
        let unhashed_len = usize::from(u16::from_be_bytes([*body.get(at)?, *body.get(at + 1)?]));
        let at = at + 2 + unhashed_len;
        let left = body.get(at..at + 2)?;
        let (r, at) = mpi32(body, at + 2)?;
        let (s, at) = mpi32(body, at)?;
        if at != body.len() {
            return None;
        }
        let trailer_len = u32::try_from(fields.len()).ok()?.to_be_bytes();
        let mut parts: Vec<&[u8]> = prefix.to_vec();
        parts.extend_from_slice(&[fields, &[0x04, 0xff][..], &trailer_len[..]]);
        let digest = digest(hash, &parts);
        if digest.get(..2)? != left {
            return None;
        }
        let mut rs = [0u8; 64];
        rs[..32].copy_from_slice(&r);
        rs[32..].copy_from_slice(&s);
        // EdDSA in OpenPGP signs the digest, not the data.
        VerifyingKey::from_bytes(key)
            .ok()?
            .verify_strict(&digest, &Signature::from_bytes(&rs))
            .ok()
    })()
    .is_some()
}

/// Costs a hash of `parts`. Their digest, taken in order.
fn digest(hash: Hash, parts: &[&[u8]]) -> Vec<u8> {
    match hash {
        Hash::Sha256 => {
            let mut h = Sha256::new();
            parts.iter().for_each(|p| h.update(p));
            h.finalize().to_vec()
        }
        Hash::Sha512 => {
            let mut h = Sha512::new();
            parts.iter().for_each(|p| h.update(p));
            h.finalize().to_vec()
        }
    }
}
