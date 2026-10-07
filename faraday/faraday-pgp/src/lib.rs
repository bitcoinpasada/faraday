//! The OpenPGP Faraday writes (`PLAN.md` §7): version-4 packets for one
//! kind of key, an Ed25519 certification key with an Ed25519 signing
//! subkey, and what is made with it: the public certificate, detached
//! signatures, a revocation certificate, and the secret parts in
//! `paperkey`'s text form.
//!
//! Nothing here reads OpenPGP from outside. The inputs are the key's own
//! seeds, its creation times, its user IDs and its expiry, which a vault
//! keeps (`docs/VAULT.md` §7, type 7). Every signature is Ed25519's and so
//! deterministic: the same key and time give the same bytes.
//!
//! The formats are RFC 4880 as RFC 9580 keeps them for version 4: the
//! EdDSALegacy public-key algorithm (22) over the Ed25519 curve, SHA-1
//! fingerprints, new-format packet headers and ASCII armour.
//!
//! `no_std` + `alloc`.

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use ed25519_dalek::{Signer, SigningKey};
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use zeroize::Zeroize;

/// The EdDSALegacy public-key algorithm.
const EDDSA: u8 = 22;
/// The Ed25519 curve's OID, as its length and bytes.
const ED25519_OID: [u8; 10] = [0x09, 0x2B, 0x06, 0x01, 0x04, 0x01, 0xDA, 0x47, 0x0F, 0x01];

/// Packet tags.
mod tag {
    pub const SIGNATURE: u8 = 2;
    pub const SECRET_KEY: u8 = 5;
    pub const PUBLIC_KEY: u8 = 6;
    pub const SECRET_SUBKEY: u8 = 7;
    pub const USER_ID: u8 = 13;
    pub const PUBLIC_SUBKEY: u8 = 14;
}

/// Signature types.
mod sig {
    pub const BINARY: u8 = 0x00;
    pub const POSITIVE_CERTIFICATION: u8 = 0x13;
    pub const SUBKEY_BINDING: u8 = 0x18;
    pub const PRIMARY_BINDING: u8 = 0x19;
    pub const KEY_REVOCATION: u8 = 0x20;
}

/// Signature subpacket types.
mod sub {
    pub const CREATED: u8 = 2;
    pub const KEY_EXPIRES: u8 = 9;
    pub const PREFERRED_SYMMETRIC: u8 = 11;
    pub const ISSUER: u8 = 16;
    pub const PREFERRED_HASH: u8 = 21;
    pub const PREFERRED_COMPRESSION: u8 = 22;
    pub const PRIMARY_USER_ID: u8 = 25;
    pub const KEY_FLAGS: u8 = 27;
    pub const REVOCATION_REASON: u8 = 29;
    pub const FEATURES: u8 = 30;
    pub const EMBEDDED_SIGNATURE: u8 = 32;
    pub const ISSUER_FINGERPRINT: u8 = 33;
}

/// The hash a signature is made over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hash {
    /// SHA-256, algorithm 8.
    Sha256,
    /// SHA-512, algorithm 10.
    Sha512,
}

impl Hash {
    fn id(self) -> u8 {
        match self {
            Hash::Sha256 => 8,
            Hash::Sha512 => 10,
        }
    }

    fn digest(self, data: &[u8]) -> Vec<u8> {
        match self {
            Hash::Sha256 => Sha256::digest(data).to_vec(),
            Hash::Sha512 => Sha512::digest(data).to_vec(),
        }
    }
}

/// Why a key was revoked (RFC 4880 §5.2.3.23).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// No reason given.
    Unspecified,
    /// The key is replaced by another.
    Superseded,
    /// The secret key may be known to someone else.
    Compromised,
    /// The key is no longer used.
    Retired,
}

impl Reason {
    fn code(self) -> u8 {
        match self {
            Reason::Unspecified => 0,
            Reason::Superseded => 1,
            Reason::Compromised => 2,
            Reason::Retired => 3,
        }
    }
}

/// One key: the primary that certifies, the subkey that signs, and what
/// the self-signatures state. The seeds are secret and wiped on drop.
pub struct Key {
    /// The primary key's Ed25519 seed.
    pub primary_seed: [u8; 32],
    /// When the primary key was made, Unix seconds.
    pub created: u32,
    /// The signing subkey's Ed25519 seed.
    pub subkey_seed: [u8; 32],
    /// When the subkey was made, Unix seconds.
    pub subkey_created: u32,
    /// The user IDs, the first one primary.
    pub user_ids: Vec<String>,
    /// Seconds after creation that the key expires; 0 for never.
    pub expiry: u32,
}

impl Drop for Key {
    fn drop(&mut self) {
        self.primary_seed.zeroize();
        self.subkey_seed.zeroize();
    }
}

/// An MPI: its length in bits, then the bytes, leading zeros dropped.
fn mpi(bytes: &[u8]) -> Vec<u8> {
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
    let b = &bytes[start..];
    let bits = if b.is_empty() {
        0
    } else {
        (b.len() as u16 - 1) * 8 + (8 - b[0].leading_zeros() as u16)
    };
    let mut out = Vec::with_capacity(2 + b.len());
    out.extend_from_slice(&bits.to_be_bytes());
    out.extend_from_slice(b);
    out
}

/// A length as packet headers and subpackets write it.
fn length(n: usize) -> Vec<u8> {
    if n < 192 {
        alloc::vec![n as u8]
    } else if n < 8384 {
        let m = n - 192;
        alloc::vec![((m >> 8) + 192) as u8, (m & 0xff) as u8]
    } else {
        let mut v = alloc::vec![0xff];
        v.extend_from_slice(&(n as u32).to_be_bytes());
        v
    }
}

/// A packet with a new-format header.
fn packet(tag: u8, body: &[u8]) -> Vec<u8> {
    let mut out = alloc::vec![0xC0 | tag];
    out.extend(length(body.len()));
    out.extend_from_slice(body);
    out
}

/// A signature subpacket.
fn subpacket(kind: u8, data: &[u8]) -> Vec<u8> {
    let mut out = length(data.len() + 1);
    out.push(kind);
    out.extend_from_slice(data);
    out
}

/// A public key's packet body, primary or subkey alike.
fn public_body(seed: &[u8; 32], created: u32) -> Vec<u8> {
    let public = SigningKey::from_bytes(seed).verifying_key();
    let mut point = alloc::vec![0x40];
    point.extend_from_slice(public.as_bytes());
    let mut body = alloc::vec![4];
    body.extend_from_slice(&created.to_be_bytes());
    body.push(EDDSA);
    body.extend_from_slice(&ED25519_OID);
    body.extend(mpi(&point));
    body
}

/// What a key is hashed as when a signature covers it.
fn key_hashed(body: &[u8]) -> Vec<u8> {
    let mut out = alloc::vec![0x99];
    out.extend_from_slice(&(body.len() as u16).to_be_bytes());
    out.extend_from_slice(body);
    out
}

/// The version-4 fingerprint of a public key body.
fn fingerprint_of(body: &[u8]) -> [u8; 20] {
    Sha1::digest(key_hashed(body)).into()
}

/// A version-4 signature packet's body: the hashed fields, the signature
/// over `prefix` and them, and the unhashed fields.
fn signature(
    seed: &[u8; 32],
    kind: u8,
    hash: Hash,
    prefix: &[u8],
    hashed: &[u8],
    unhashed: &[u8],
) -> Vec<u8> {
    let mut fields = alloc::vec![4, kind, EDDSA, hash.id()];
    fields.extend_from_slice(&(hashed.len() as u16).to_be_bytes());
    fields.extend_from_slice(hashed);
    let mut data = Vec::with_capacity(prefix.len() + fields.len() + 6);
    data.extend_from_slice(prefix);
    data.extend_from_slice(&fields);
    data.extend_from_slice(&[0x04, 0xff]);
    data.extend_from_slice(&(fields.len() as u32).to_be_bytes());
    let digest = hash.digest(&data);
    // EdDSA in OpenPGP signs the digest, not the data.
    let sig = SigningKey::from_bytes(seed).sign(&digest).to_bytes();
    let mut body = fields;
    body.extend_from_slice(&(unhashed.len() as u16).to_be_bytes());
    body.extend_from_slice(unhashed);
    body.extend_from_slice(&digest[..2]);
    body.extend(mpi(&sig[..32]));
    body.extend(mpi(&sig[32..]));
    body
}

impl Key {
    fn primary_body(&self) -> Vec<u8> {
        public_body(&self.primary_seed, self.created)
    }

    fn subkey_body(&self) -> Vec<u8> {
        public_body(&self.subkey_seed, self.subkey_created)
    }

    /// The primary key's fingerprint.
    pub fn fingerprint(&self) -> [u8; 20] {
        fingerprint_of(&self.primary_body())
    }

    /// The signing subkey's fingerprint.
    pub fn subkey_fingerprint(&self) -> [u8; 20] {
        fingerprint_of(&self.subkey_body())
    }

    fn issued_by(fp: &[u8; 20]) -> (Vec<u8>, Vec<u8>) {
        let mut issuer = alloc::vec![4];
        issuer.extend_from_slice(fp);
        (
            subpacket(sub::ISSUER_FINGERPRINT, &issuer),
            subpacket(sub::ISSUER, &fp[12..]),
        )
    }

    /// The public certificate: the primary key, each user ID with its
    /// self-signature, and the signing subkey with its binding signature
    /// and the subkey's own signature back. `at` is when the
    /// self-signatures are made.
    pub fn certificate(&self, at: u32) -> Vec<u8> {
        let primary = self.primary_body();
        let fp = fingerprint_of(&primary);
        let (issuer_fp, issuer_id) = Self::issued_by(&fp);
        let mut out = packet(tag::PUBLIC_KEY, &primary);
        for (k, uid) in self.user_ids.iter().enumerate() {
            let mut hashed = subpacket(sub::CREATED, &at.to_be_bytes());
            hashed.extend(subpacket(sub::KEY_FLAGS, &[0x01]));
            if self.expiry > 0 {
                hashed.extend(subpacket(sub::KEY_EXPIRES, &self.expiry.to_be_bytes()));
            }
            hashed.extend(subpacket(sub::PREFERRED_SYMMETRIC, &[9, 8, 7]));
            hashed.extend(subpacket(sub::PREFERRED_HASH, &[10, 9, 8]));
            hashed.extend(subpacket(sub::PREFERRED_COMPRESSION, &[0]));
            hashed.extend(subpacket(sub::FEATURES, &[0x01]));
            if k == 0 {
                hashed.extend(subpacket(sub::PRIMARY_USER_ID, &[1]));
            }
            hashed.extend(issuer_fp.clone());
            let mut prefix = key_hashed(&primary);
            prefix.push(0xB4);
            prefix.extend_from_slice(&(uid.len() as u32).to_be_bytes());
            prefix.extend_from_slice(uid.as_bytes());
            out.extend(packet(tag::USER_ID, uid.as_bytes()));
            out.extend(packet(
                tag::SIGNATURE,
                &signature(
                    &self.primary_seed,
                    sig::POSITIVE_CERTIFICATION,
                    Hash::Sha512,
                    &prefix,
                    &hashed,
                    &issuer_id,
                ),
            ));
        }
        let subkey = self.subkey_body();
        let sub_fp = fingerprint_of(&subkey);
        let mut both = key_hashed(&primary);
        both.extend(key_hashed(&subkey));
        // The subkey signs back that it belongs to this primary key.
        let (sub_issuer_fp, sub_issuer_id) = Self::issued_by(&sub_fp);
        let mut back_hashed = subpacket(sub::CREATED, &at.to_be_bytes());
        back_hashed.extend(sub_issuer_fp);
        let back = signature(
            &self.subkey_seed,
            sig::PRIMARY_BINDING,
            Hash::Sha512,
            &both,
            &back_hashed,
            &sub_issuer_id,
        );
        let mut hashed = subpacket(sub::CREATED, &at.to_be_bytes());
        hashed.extend(subpacket(sub::KEY_FLAGS, &[0x02]));
        if self.expiry > 0 {
            let expires = self
                .expiry
                .saturating_add(self.created)
                .saturating_sub(self.subkey_created);
            hashed.extend(subpacket(sub::KEY_EXPIRES, &expires.to_be_bytes()));
        }
        hashed.extend(issuer_fp);
        hashed.extend(subpacket(sub::EMBEDDED_SIGNATURE, &back));
        out.extend(packet(tag::PUBLIC_SUBKEY, &subkey));
        out.extend(packet(
            tag::SIGNATURE,
            &signature(
                &self.primary_seed,
                sig::SUBKEY_BINDING,
                Hash::Sha512,
                &both,
                &hashed,
                &issuer_id,
            ),
        ));
        out
    }

    /// A revocation certificate for the whole key, made at `at`.
    pub fn revocation(&self, at: u32, reason: Reason, text: &str) -> Vec<u8> {
        let primary = self.primary_body();
        let fp = fingerprint_of(&primary);
        let (issuer_fp, issuer_id) = Self::issued_by(&fp);
        let mut hashed = subpacket(sub::CREATED, &at.to_be_bytes());
        let mut why = alloc::vec![reason.code()];
        why.extend_from_slice(text.as_bytes());
        hashed.extend(subpacket(sub::REVOCATION_REASON, &why));
        hashed.extend(issuer_fp);
        packet(
            tag::SIGNATURE,
            &signature(
                &self.primary_seed,
                sig::KEY_REVOCATION,
                Hash::Sha512,
                &key_hashed(&primary),
                &hashed,
                &issuer_id,
            ),
        )
    }

    /// A detached signature over `data` by the signing subkey, made at
    /// `at`.
    pub fn sign(&self, data: &[u8], at: u32, hash: Hash) -> Vec<u8> {
        let sub_fp = fingerprint_of(&self.subkey_body());
        let (issuer_fp, issuer_id) = Self::issued_by(&sub_fp);
        let mut hashed = subpacket(sub::CREATED, &at.to_be_bytes());
        hashed.extend(issuer_fp);
        packet(
            tag::SIGNATURE,
            &signature(
                &self.subkey_seed,
                sig::BINARY,
                hash,
                data,
                &hashed,
                &issuer_id,
            ),
        )
    }

    /// A secret key or subkey packet's body: the public body, then the
    /// secret part unencrypted (string-to-key usage 0), its MPI and its
    /// two-octet checksum.
    fn secret_part(seed: &[u8; 32]) -> Vec<u8> {
        let mut out = alloc::vec![0];
        let m = mpi(seed);
        let sum = m.iter().fold(0u16, |s, b| s.wrapping_add(u16::from(*b)));
        out.extend(m);
        out.extend_from_slice(&sum.to_be_bytes());
        out
    }

    /// The transferable secret key, as `gpg --export-secret-keys` writes
    /// it with no passphrase: the secret primary key and subkey with the
    /// certificate's user IDs and signatures. For `paperkey` and the
    /// tests; Faraday does not put it in the Outbox.
    pub fn secret_key(&self, at: u32) -> Vec<u8> {
        let public = self.certificate(at);
        let mut primary = self.primary_body();
        primary.extend(Self::secret_part(&self.primary_seed));
        let mut subkey = self.subkey_body();
        subkey.extend(Self::secret_part(&self.subkey_seed));
        // The certificate's packets, with the two keys swapped for their
        // secret forms.
        let mut out = Vec::new();
        let mut at_byte = 0;
        while at_byte < public.len() {
            let (t, start, end) = next_packet(&public, at_byte);
            match t {
                tag::PUBLIC_KEY => out.extend(packet(tag::SECRET_KEY, &primary)),
                tag::PUBLIC_SUBKEY => out.extend(packet(tag::SECRET_SUBKEY, &subkey)),
                _ => out.extend_from_slice(&public[at_byte..end]),
            }
            let _ = start;
            at_byte = end;
        }
        primary.zeroize();
        subkey.zeroize();
        out
    }

    /// The secret parts in `paperkey`'s text form, without its comment
    /// header: what a person copies onto paper to rebuild the key from the
    /// public certificate (`PLAN.md` §7).
    pub fn paperkey(&self) -> String {
        let mut data = alloc::vec![0u8];
        for (seed, created) in [
            (&self.primary_seed, self.created),
            (&self.subkey_seed, self.subkey_created),
        ] {
            let body = public_body(seed, created);
            data.push(4);
            data.extend_from_slice(&fingerprint_of(&body));
            let secret = Self::secret_part(seed);
            data.extend_from_slice(&(secret.len() as u16).to_be_bytes());
            data.extend(secret);
        }
        let text = paperkey_lines(&data);
        data.zeroize();
        text
    }
}

/// The packet at `at`: its tag, where its body starts, and where it ends.
/// Reads only the new-format headers this crate writes.
fn next_packet(bytes: &[u8], at: usize) -> (u8, usize, usize) {
    let t = bytes[at] & 0x3f;
    let first = bytes[at + 1] as usize;
    let (len, head) = if first < 192 {
        (first, 2)
    } else if first < 224 {
        (((first - 192) << 8) + bytes[at + 2] as usize + 192, 3)
    } else {
        let n = u32::from_be_bytes([bytes[at + 2], bytes[at + 3], bytes[at + 4], bytes[at + 5]]);
        (n as usize, 6)
    };
    (t, at + head, at + head + len)
}

/// CRC-24 as OpenPGP armour and `paperkey` use it.
pub fn crc24(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xB7_04CE;
    for &b in data {
        crc ^= u32::from(b) << 16;
        for _ in 0..8 {
            crc <<= 1;
            if crc & 0x100_0000 != 0 {
                crc ^= 0x186_4CFB;
            }
        }
    }
    crc & 0xFF_FFFF
}

/// `paperkey`'s base-16 lines: 22 bytes a line, numbered, each ending in
/// its own CRC-24, and a last line with the CRC-24 of the whole.
fn paperkey_lines(data: &[u8]) -> String {
    use core::fmt::Write;
    let mut out = String::new();
    let mut n = 0;
    for chunk in data.chunks(22) {
        n += 1;
        let _ = write!(out, "{n:3}: ");
        for b in chunk {
            let _ = write!(out, "{b:02X} ");
        }
        let _ = writeln!(out, "{:06X}", crc24(chunk));
    }
    let _ = writeln!(out, "{:3}: {:06X}", n + 1, crc24(data));
    out
}

/// What an armoured block holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Armor {
    /// A certificate, or a revocation certificate.
    PublicKey,
    /// A detached signature.
    Signature,
}

/// Base64, as armour writes it.
fn base64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (u32::from(c[0]) << 16)
            | (u32::from(*c.get(1).unwrap_or(&0)) << 8)
            | u32::from(*c.get(2).unwrap_or(&0));
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Packets in ASCII armour.
pub fn armor(kind: Armor, data: &[u8]) -> String {
    let name = match kind {
        Armor::PublicKey => "PGP PUBLIC KEY BLOCK",
        Armor::Signature => "PGP SIGNATURE",
    };
    let mut out = String::new();
    out.push_str("-----BEGIN ");
    out.push_str(name);
    out.push_str("-----\n\n");
    let body = base64(data);
    for line in body.as_bytes().chunks(64) {
        out.push_str(core::str::from_utf8(line).expect("base64 is ASCII"));
        out.push('\n');
    }
    let crc = crc24(data).to_be_bytes();
    out.push('=');
    out.push_str(&base64(&crc[1..]));
    out.push_str("\n-----END ");
    out.push_str(name);
    out.push_str("-----\n");
    out
}

/// A fingerprint as GnuPG prints it: upper-case hex in groups of four.
pub fn fingerprint_text(fp: &[u8; 20]) -> String {
    use core::fmt::Write;
    let mut out = String::new();
    for (i, b) in fp.iter().enumerate() {
        if i > 0 && i % 2 == 0 {
            out.push(' ');
        }
        let _ = write!(out, "{b:02X}");
    }
    out
}
