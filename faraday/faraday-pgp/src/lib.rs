//! The OpenPGP Faraday writes (`docs/history/PLAN.md` §7): version-4 packets for one
//! kind of key, an Ed25519 certification key with an Ed25519 signing
//! subkey, and what is made with it: the public certificate, detached
//! signatures, a revocation certificate, and the secret parts in
//! `paperkey`'s text form.
//!
//! The only OpenPGP read from outside is [`read_public`]: it says whether
//! a file is one ASCII-armoured block of public packets, a public key or a
//! signature, and reads every packet in full to its exact end, so that no
//! byte of it is a secret in another packet's clothes. The inputs to
//! everything else are
//! the key's own seeds, its creation times, its user IDs and its expiry,
//! which a vault keeps (`docs/VAULT.md` §7, type 7). Every signature is
//! Ed25519's and so deterministic: the same key and time give the same
//! bytes.
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
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

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
    /// The reason GnuPG's `--generate-revocation` menu number names:
    /// 0 none, 1 compromised, 2 superseded, 3 no longer used. The menu's
    /// order is not the order of the codes in the packet.
    pub fn from_menu(n: u8) -> Option<Reason> {
        match n {
            0 => Some(Reason::Unspecified),
            1 => Some(Reason::Compromised),
            2 => Some(Reason::Superseded),
            3 => Some(Reason::Retired),
            _ => None,
        }
    }

    fn code(self) -> u8 {
        match self {
            Reason::Unspecified => 0,
            Reason::Superseded => 1,
            Reason::Compromised => 2,
            Reason::Retired => 3,
        }
    }
}

/// What GnuPG 2.4 writes in a `.rev` file after the key's listing.
const REVOCATION_TEXT: &str = "
A revocation certificate is a kind of \"kill switch\" to publicly
declare that a key shall not anymore be used.  It is not possible
to retract such a revocation certificate once it has been published.

Use it to revoke this key in case of a compromise or loss of
the secret key.  However, if the secret key is still accessible,
it is better to generate a new revocation certificate and give
a reason for the revocation.  For details see the description of
of the gpg command \"--generate-revocation\" in the GnuPG manual.

To avoid an accidental use of this file, a colon has been inserted
before the 5 dashes below.  Remove this colon with a text editor
before importing and publishing this revocation certificate.

";

/// One key: the primary that certifies, the subkey that signs, and what
/// the self-signatures state. The seeds are secret and wiped on drop.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Key {
    /// The primary key's Ed25519 seed.
    pub primary_seed: [u8; 32],
    /// When the primary key was made, Unix seconds.
    #[zeroize(skip)]
    pub created: u32,
    /// The signing subkey's Ed25519 seed.
    pub subkey_seed: [u8; 32],
    /// When the subkey was made, Unix seconds.
    #[zeroize(skip)]
    pub subkey_created: u32,
    /// The user IDs, the first one primary.
    #[zeroize(skip)]
    pub user_ids: Vec<String>,
    /// Seconds after creation that the key expires; 0 for never.
    #[zeroize(skip)]
    pub expiry: u32,
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

/// [`mpi`] of a secret, in memory that is wiped, sized once so that no
/// copy is left behind by a reallocation.
fn mpi_secret(bytes: &[u8]) -> Zeroizing<Vec<u8>> {
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(bytes.len());
    let b = &bytes[start..];
    let bits = if b.is_empty() {
        0
    } else {
        (b.len() as u16 - 1) * 8 + (8 - b[0].leading_zeros() as u16)
    };
    let mut out = Zeroizing::new(Vec::with_capacity(2 + b.len()));
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
            // Counted from the subkey's creation. A subkey made after the
            // key's expiry would count 0, which reads as "never": it
            // expires one second after it was made instead.
            let expires = self
                .expiry
                .saturating_add(self.created)
                .saturating_sub(self.subkey_created)
                .max(1);
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

    /// A revocation certificate as a file, laid out as GnuPG 2.4 writes
    /// `openpgp-revocs.d/<FPR>.rev`: its explanatory text with the key as
    /// `gpg --list-keys` prints it, then the armoured certificate with a
    /// colon before the first line's dashes, so that `gpg --import`
    /// refuses the file until the colon is taken out by hand. The dates
    /// are UTC, as GnuPG prints them.
    pub fn revocation_file(&self, revocation: &[u8]) -> String {
        let mut out = String::new();
        out.push_str("This is a revocation certificate for the OpenPGP key:\n\npub   ed25519 ");
        out.push_str(&utc_date(u64::from(self.created)));
        out.push_str(" [C]");
        if self.expiry > 0 {
            out.push_str(" [expires: ");
            out.push_str(&utc_date(u64::from(self.created) + u64::from(self.expiry)));
            out.push(']');
        }
        out.push_str("\n      ");
        for b in self.fingerprint() {
            use core::fmt::Write;
            let _ = write!(out, "{b:02X}");
        }
        out.push('\n');
        for uid in &self.user_ids {
            out.push_str("uid          ");
            out.push_str(uid);
            out.push('\n');
        }
        out.push_str(REVOCATION_TEXT);
        out.push(':');
        out.push_str(&armor_with(
            Armor::PublicKey,
            revocation,
            Some("This is a revocation certificate"),
        ));
        out
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
    fn secret_part(seed: &[u8; 32]) -> Zeroizing<Vec<u8>> {
        let m = mpi_secret(seed);
        let sum = m.iter().fold(0u16, |s, b| s.wrapping_add(u16::from(*b)));
        let mut out = Zeroizing::new(Vec::with_capacity(1 + m.len() + 2));
        out.push(0);
        out.extend_from_slice(&m);
        out.extend_from_slice(&sum.to_be_bytes());
        out
    }

    /// The transferable secret key, as `gpg --export-secret-keys` writes
    /// it with no passphrase: the secret primary key and subkey with the
    /// certificate's user IDs and signatures. For `paperkey` and the
    /// tests; Faraday does not put it in the Outbox. Wiped on drop.
    pub fn secret_key(&self, at: u32) -> Zeroizing<Vec<u8>> {
        let public = self.certificate(at);
        let mut primary = Zeroizing::new(Vec::with_capacity(128));
        primary.extend_from_slice(&self.primary_body());
        primary.extend_from_slice(&Self::secret_part(&self.primary_seed));
        let mut subkey = Zeroizing::new(Vec::with_capacity(128));
        subkey.extend_from_slice(&self.subkey_body());
        subkey.extend_from_slice(&Self::secret_part(&self.subkey_seed));
        // The certificate's packets, with the two keys swapped for their
        // secret forms. Sized once, so that no copy is left by growing.
        let mut out = Zeroizing::new(Vec::with_capacity(public.len() + 2 * 128));
        let mut at_byte = 0;
        while at_byte < public.len() {
            let (t, _, end) = next_packet(&public, at_byte);
            match t {
                tag::PUBLIC_KEY => push_packet(&mut out, tag::SECRET_KEY, &primary),
                tag::PUBLIC_SUBKEY => push_packet(&mut out, tag::SECRET_SUBKEY, &subkey),
                _ => out.extend_from_slice(&public[at_byte..end]),
            }
            at_byte = end;
        }
        out
    }

    /// The secret parts in `paperkey`'s text form, without its comment
    /// header: what a person copies onto paper to rebuild the key from the
    /// public certificate (`docs/history/PLAN.md` §7). Compute it only to show it; it is
    /// wiped on drop.
    pub fn paperkey(&self) -> Zeroizing<String> {
        let mut data = Zeroizing::new(Vec::with_capacity(1 + 2 * (1 + 20 + 2 + 40)));
        data.push(0u8);
        for (seed, created) in [
            (&self.primary_seed, self.created),
            (&self.subkey_seed, self.subkey_created),
        ] {
            let body = public_body(seed, created);
            data.push(4);
            data.extend_from_slice(&fingerprint_of(&body));
            let secret = Self::secret_part(seed);
            data.extend_from_slice(&(secret.len() as u16).to_be_bytes());
            data.extend_from_slice(&secret);
        }
        paperkey_lines(&data)
    }
}

/// A packet with a new-format header appended to `out`, with no copy of
/// the body left outside it.
fn push_packet(out: &mut Vec<u8>, tag: u8, body: &[u8]) {
    out.push(0xC0 | tag);
    out.extend(length(body.len()));
    out.extend_from_slice(body);
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
fn paperkey_lines(data: &[u8]) -> Zeroizing<String> {
    use core::fmt::Write;
    // 78 bytes a line at most, sized once.
    let mut out = Zeroizing::new(String::with_capacity((data.len().div_ceil(22) + 1) * 80));
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
    armor_with(kind, data, None)
}

/// Packets in ASCII armour, with a `Comment:` header when one is given.
pub fn armor_with(kind: Armor, data: &[u8], comment: Option<&str>) -> String {
    let name = match kind {
        Armor::PublicKey => "PGP PUBLIC KEY BLOCK",
        Armor::Signature => "PGP SIGNATURE",
    };
    let mut out = String::new();
    out.push_str("-----BEGIN ");
    out.push_str(name);
    out.push_str("-----\n");
    if let Some(c) = comment {
        out.push_str("Comment: ");
        out.push_str(c);
        out.push('\n');
    }
    out.push('\n');
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

/// A Unix time as a UTC date, `YYYY-MM-DD`, as GnuPG prints dates.
pub fn utc_date(unix: u64) -> String {
    use core::fmt::Write;
    let days = (unix / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    let mut out = String::new();
    let _ = write!(out, "{y:04}-{m:02}-{d:02}");
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

/// The time of the first signature packet in `packets`: the self-signatures
/// of a certificate are all made at one time, and a revocation is one
/// signature.
pub fn signature_time(packets: &[u8]) -> Option<u32> {
    let mut at = 0;
    while at + 2 < packets.len() {
        let packet = packets[at] & 0x3f;
        let first = packets[at + 1] as usize;
        let (len, head) = if first < 192 {
            (first, 2)
        } else if first < 224 {
            (((first - 192) << 8) + packets[at + 2] as usize + 192, 3)
        } else {
            return None;
        };
        let body = packets.get(at + head..at + head + len)?;
        if packet == tag::SIGNATURE {
            // version, type, algorithm, hash, hashed length (2), then a
            // subpacket: length 5, type 2, four bytes.
            return Some(u32::from_be_bytes(body.get(8..12)?.try_into().ok()?));
        }
        at += head + len;
    }
    None
}

/// What one block of public armour holds, once it has read in full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicArmor {
    /// A public key or a signature.
    pub kind: Armor,
    /// The tag of every packet, in order. Never a secret key's.
    pub tags: Vec<u8>,
    /// Every piece of text in it: the armour headers' values, the user
    /// IDs, and the text a signature's subpackets carry. A caller that can
    /// tell a seed or a private key from text runs its detectors over
    /// these; the packets can be nothing else that hides one.
    pub texts: Vec<String>,
}

/// Whether `bytes` are nothing but one ASCII-armoured OpenPGP block of
/// public packets, and which kind; see [`read_public`].
pub fn read_public_armor(bytes: &[u8]) -> Option<Armor> {
    read_public(bytes).map(|p| p.kind)
}

/// The block [`read_block`] reads, only when its checksum is there and
/// right. Without it a file is not taken for public: GnuPG 2.4 writes the
/// checksum, and RFC 9580 makes it optional for a reader, which is a
/// reason to read the block (see [`read_block`]) and not a reason to take
/// it for public.
pub fn read_public(bytes: &[u8]) -> Option<PublicArmor> {
    let (block, crc_ok) = read_block(bytes)?;
    crc_ok.then_some(block)
}

/// One ASCII-armoured OpenPGP block of public packets (RFC 4880 §6, RFC
/// 9580 §6), and whether its checksum line is there and right. A missing
/// or wrong checksum does not refuse the block, as RFC 9580 has it.
///
/// Blank lines may come before and after, line ends may be LF or CRLF, and
/// the headers must be the five GnuPG knows (Version, Comment, Hash,
/// Charset, MessageID), written `Name: value`. Every packet is read to its
/// exact end: a public key or subkey is a version-4 public key with
/// nothing after its last field, a user ID is UTF-8 text, a signature is a
/// version-4 signature with well-formed subpackets and nothing after its
/// last field. A key block starts with a public key packet and holds only
/// those, user IDs and signatures; a signature block holds only
/// signatures. A secret key packet, a user attribute, a trust packet or
/// any other packet refuses the block, and so does a secret packet whose
/// tag was changed to a public one: its body does not end where a public
/// key's does.
pub fn read_block(bytes: &[u8]) -> Option<(PublicArmor, bool)> {
    if !bytes.is_ascii() {
        return None;
    }
    let mut lines = bytes
        .trim_ascii()
        .split(|&b| b == b'\n')
        .map(|l| l.strip_suffix(b"\r").unwrap_or(l));
    let (kind, end): (Armor, &[u8]) = match lines.next()? {
        b"-----BEGIN PGP PUBLIC KEY BLOCK-----" => {
            (Armor::PublicKey, b"-----END PGP PUBLIC KEY BLOCK-----")
        }
        b"-----BEGIN PGP SIGNATURE-----" => (Armor::Signature, b"-----END PGP SIGNATURE-----"),
        _ => return None,
    };
    let mut texts: Vec<String> = Vec::new();
    // Headers, up to the empty line.
    loop {
        let line = lines.next()?;
        if line.is_empty() {
            break;
        }
        let colon = line.windows(2).position(|w| w == b": ")?;
        let (name, value) = (&line[..colon], &line[colon + 2..]);
        if !matches!(
            name,
            b"Version" | b"Comment" | b"Hash" | b"Charset" | b"MessageID"
        ) || !value.iter().all(|&b| (0x20..0x7f).contains(&b))
        {
            return None;
        }
        texts.push(String::from_utf8_lossy(value).into_owned());
    }
    // The body: base64 across lines, then an optional checksum line, then
    // the end line, and nothing after it.
    let mut b64 = Base64Reader::default();
    let mut checksum: Option<u32> = None;
    loop {
        let line = lines.next()?;
        if line == end {
            break;
        }
        if line.len() == 5 && line[0] == b'=' {
            let mut v = 0u32;
            for &b in &line[1..] {
                v = v << 6 | u32::from(base64_value(b)?);
            }
            checksum = Some(v);
            if lines.next()? != end {
                return None;
            }
            break;
        }
        if line.is_empty() {
            return None;
        }
        for &b in line {
            b64.push(b)?;
        }
    }
    if lines.next().is_some() || !b64.complete() {
        return None;
    }
    let data = b64.data;
    if data.is_empty() {
        return None;
    }
    let crc_ok = checksum == Some(crc24(&data));
    // Whole packets, each read to its end.
    let mut tags = Vec::new();
    let mut at = 0usize;
    while at < data.len() {
        let first = data[at];
        if first & 0x80 == 0 {
            return None;
        }
        let byte = |n: usize| data.get(at + n).copied().map(usize::from);
        let (packet_tag, head, len) = if first & 0x40 != 0 {
            let len0 = byte(1)?;
            let (head, len) = match len0 {
                0..=191 => (2, len0),
                192..=223 => (3, ((len0 - 192) << 8) + byte(2)? + 192),
                255 => (
                    6,
                    byte(2)? << 24 | byte(3)? << 16 | byte(4)? << 8 | byte(5)?,
                ),
                // A partial body length.
                _ => return None,
            };
            (first & 0x3f, head, len)
        } else {
            let (head, len) = match first & 3 {
                0 => (2, byte(1)?),
                1 => (3, byte(1)? << 8 | byte(2)?),
                2 => (
                    5,
                    byte(1)? << 24 | byte(2)? << 16 | byte(3)? << 8 | byte(4)?,
                ),
                // An indeterminate length.
                _ => return None,
            };
            ((first >> 2) & 0x0f, head, len)
        };
        let from = at.checked_add(head)?;
        let to = from.checked_add(len)?;
        let body = data.get(from..to)?;
        let ok = match (kind, packet_tag) {
            (Armor::PublicKey, tag::PUBLIC_KEY) => public_key_body(body),
            (Armor::PublicKey, tag::PUBLIC_SUBKEY) => !tags.is_empty() && public_key_body(body),
            (Armor::PublicKey, tag::USER_ID) => {
                !tags.is_empty() && push_text(&mut texts, body, true)
            }
            (_, tag::SIGNATURE) => {
                (kind == Armor::Signature || !tags.is_empty())
                    && signature_body(body, &mut texts, 0)
            }
            _ => false,
        };
        // A key block starts with a public key.
        if !ok || (kind == Armor::PublicKey && tags.is_empty() && packet_tag != tag::PUBLIC_KEY) {
            return None;
        }
        tags.push(packet_tag);
        at = to;
    }
    Some((PublicArmor { kind, tags, texts }, crc_ok))
}

/// Text from a packet or subpacket into `texts`; `strict` refuses what is
/// not UTF-8, otherwise it is kept as far as it reads.
fn push_text(texts: &mut Vec<String>, bytes: &[u8], strict: bool) -> bool {
    match core::str::from_utf8(bytes) {
        Ok(t) => texts.push(String::from(t)),
        Err(_) if !strict => texts.push(String::from_utf8_lossy(bytes).into_owned()),
        Err(_) => return false,
    }
    true
}

/// A reader over a packet's bytes.
struct Rd<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Rd<'a> {
    fn new(b: &'a [u8]) -> Self {
        Rd { b, at: 0 }
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.b.get(self.at..self.at.checked_add(n)?)?;
        self.at += n;
        Some(s)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn u16(&mut self) -> Option<usize> {
        let s = self.take(2)?;
        Some(usize::from(u16::from_be_bytes([s[0], s[1]])))
    }

    fn done(&self) -> bool {
        self.at == self.b.len()
    }

    /// An MPI: its bit count, then exactly that many bits' bytes, the
    /// first with its top bit where the count puts it.
    fn mpi(&mut self) -> Option<()> {
        let bits = self.u16()?;
        let bytes = self.take(bits.div_ceil(8))?;
        if bits > 0
            && (bytes[0] == 0 || bytes.len() * 8 - bytes[0].leading_zeros() as usize != bits)
        {
            return None;
        }
        Some(())
    }

    /// A curve's OID, as its length and bytes.
    fn oid(&mut self) -> Option<()> {
        let n = self.u8()?;
        if n == 0 || n == 0xff {
            return None;
        }
        self.take(usize::from(n)).map(|_| ())
    }
}

/// Whether `body` is a version-4 public key, primary or subkey alike, to
/// its last byte.
fn public_key_body(body: &[u8]) -> bool {
    let mut r = Rd::new(body);
    let fields = (|| {
        if r.u8()? != 4 {
            return None;
        }
        r.take(4)?;
        match r.u8()? {
            1..=3 => {
                r.mpi()?;
                r.mpi()?;
            }
            16 | 20 => {
                for _ in 0..3 {
                    r.mpi()?;
                }
            }
            17 => {
                for _ in 0..4 {
                    r.mpi()?;
                }
            }
            // ECDH: the curve, the point, and the key derivation's
            // parameters.
            18 => {
                r.oid()?;
                r.mpi()?;
                if r.u8()? != 3 || r.u8()? != 1 {
                    return None;
                }
                r.take(2)?;
            }
            19 | 22 => {
                r.oid()?;
                r.mpi()?;
            }
            // X25519, Ed25519, X448, Ed448: raw keys.
            25 | 27 => {
                r.take(32)?;
            }
            26 => {
                r.take(56)?;
            }
            28 => {
                r.take(57)?;
            }
            _ => return None,
        }
        Some(())
    })();
    fields.is_some() && r.done()
}

/// Whether `body` is a version-4 signature to its last byte, its
/// subpackets well formed; the text they carry goes into `texts`.
fn signature_body(body: &[u8], texts: &mut Vec<String>, depth: u8) -> bool {
    let mut r = Rd::new(body);
    let fields = (|| {
        if r.u8()? != 4 {
            return None;
        }
        r.take(3)?;
        for _ in 0..2 {
            let n = r.u16()?;
            let area = r.take(n)?;
            subpackets(area, texts, depth)?;
        }
        r.take(2)?;
        match body[2] {
            1 | 3 => r.mpi()?,
            17 | 19 | 22 => {
                r.mpi()?;
                r.mpi()?;
            }
            27 => {
                r.take(64)?;
            }
            28 => {
                r.take(114)?;
            }
            _ => return None,
        }
        Some(())
    })();
    fields.is_some() && r.done()
}

/// A signature's subpackets, each with the length its type needs.
fn subpackets(area: &[u8], texts: &mut Vec<String>, depth: u8) -> Option<()> {
    let mut r = Rd::new(area);
    while !r.done() {
        let n = match r.u8()? {
            n @ 0..=191 => usize::from(n),
            n @ 192..=254 => ((usize::from(n) - 192) << 8) + usize::from(r.u8()?) + 192,
            _ => {
                let s = r.take(4)?;
                u32::from_be_bytes([s[0], s[1], s[2], s[3]]) as usize
            }
        };
        let sub = r.take(n)?;
        let (&kind, data) = sub.split_first()?;
        let ok = match kind & 0x7f {
            2 | 3 | 9 => data.len() == 4,
            4 | 25 => data.len() == 1,
            5 => data.len() == 2,
            16 => data.len() == 8,
            27 => !data.is_empty(),
            // A version-4 key's fingerprint, or a version-6 key's.
            33 => (data.len() == 21 && data[0] == 4) || (data.len() == 33 && data[0] == 6),
            32 => depth == 0 && signature_body(data, texts, depth + 1),
            // A notation: flags, two lengths, the name, the value.
            20 => {
                let mut nr = Rd::new(data);
                (|| {
                    let flags = nr.take(4)?[0];
                    let (nl, vl) = (nr.u16()?, nr.u16()?);
                    let name = nr.take(nl)?;
                    let value = nr.take(vl)?;
                    nr.done().then(|| {
                        push_text(texts, name, true) && push_text(texts, value, flags & 0x80 != 0)
                    })
                })()
                .unwrap_or(false)
            }
            // A regular expression, a keyserver, a policy URI, a signer's
            // user ID.
            6 | 24 | 26 | 28 => push_text(texts, data, true),
            // A reason for revocation: a code, then text.
            29 => !data.is_empty() && push_text(texts, &data[1..], true),
            // Lists and flags, and whatever a later version adds: framed
            // right, and its text, when it reads as some, kept.
            _ => {
                if data.len() >= 8 {
                    push_text(texts, data, false);
                }
                true
            }
        };
        if !ok {
            return None;
        }
    }
    Some(())
}

/// The value of one base64 character.
fn base64_value(b: u8) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// A base64 decoder fed one character at a time, so that the lines of an
/// armoured body need no joining.
#[derive(Default)]
struct Base64Reader {
    data: Vec<u8>,
    quantum: [u8; 4],
    have: usize,
    pads: usize,
    closed: bool,
}

impl Base64Reader {
    fn push(&mut self, b: u8) -> Option<()> {
        if self.closed {
            return None;
        }
        if b == b'=' {
            if self.have < 2 {
                return None;
            }
            self.pads += 1;
        } else if self.pads > 0 {
            return None;
        } else {
            self.quantum[self.have] = base64_value(b)?;
            self.have += 1;
        }
        if self.have + self.pads == 4 {
            let q = self.quantum;
            let bytes = [
                q[0] << 2 | q[1] >> 4,
                q[1] << 4 | q[2] >> 2,
                q[2] << 6 | q[3],
            ];
            self.data.extend_from_slice(&bytes[..self.have - 1]);
            self.closed = self.pads > 0;
            self.have = 0;
            self.pads = 0;
        }
        Some(())
    }

    /// Every quantum was finished.
    fn complete(&self) -> bool {
        self.have == 0 && self.pads == 0
    }
}
