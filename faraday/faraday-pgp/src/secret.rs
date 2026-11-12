//! The armoured secret key Faraday writes for a GPG key in a KDBX vault
//! (`docs/VAULT.md` §7, `gpg`): [`Key::secret_key`]'s packets in ASCII
//! armour, and the reader that takes back that and nothing else.
//!
//! The reader does not parse a secret key in general. It pulls the two
//! seeds, their dates and the user IDs out of the packets, takes the
//! expiry and the signing time from the certificate those make, writes the
//! key out again with [`Key::secret_key`] and takes the input only when the
//! two are the same bytes. A key with a passphrase, another curve, a
//! packet more or less, or anything else GnuPG would write differently is
//! not Faraday's and reads as `None`. Secret key material: this file is
//! in lint-secrets' list, so its text is bytes in wiping buffers.

use alloc::vec::Vec;
use base64ct::{Base64, Encoding};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::length::{NewFormat, new_format_whole};
use crate::{Armor, Certificate, Key, armor, crc24, public_body, tag};

const BEGIN: &[u8] = b"-----BEGIN PGP PRIVATE KEY BLOCK-----";
const END: &[u8] = b"-----END PGP PRIVATE KEY BLOCK-----";

/// Costs Ed25519 signatures. `key`'s secret key (packets from
/// [`Key::secret_key`], self-signatures made at `at`) in ASCII armour, as
/// bytes in memory that is wiped: the lines of 64 characters GnuPG writes,
/// a CRC-24 line, and no headers; the reader allows them, as the
/// bytes written are checked whole. [`read_secret`] takes it back.
pub fn armor_secret(key: &Key, at: u32) -> Zeroizing<Vec<u8>> {
    let packets = key.secret_key(at);
    let mut b64 = Zeroizing::new(alloc::vec![0u8; packets.len().div_ceil(3) * 4]);
    let n = Base64::encode(&packets, &mut b64)
        .expect("the buffer is sized for the base64 of the packets")
        .len();
    let mut out = Zeroizing::new(Vec::with_capacity(
        BEGIN.len() + END.len() + n + n / 64 + 16,
    ));
    out.extend_from_slice(BEGIN);
    out.extend_from_slice(b"\n\n");
    for line in b64[..n].chunks(64) {
        out.extend_from_slice(line);
        out.push(b'\n');
    }
    let crc = crc24(&packets).to_be_bytes();
    let mut sum = [0u8; 4];
    Base64::encode(&crc[1..], &mut sum).expect("three bytes make four characters");
    out.push(b'=');
    out.extend_from_slice(&sum);
    out.push(b'\n');
    out.extend_from_slice(END);
    out.push(b'\n');
    out
}

/// The packets of one armoured block whose lines are `begin` and `end`
/// (headers of the form `Name: value` allowed): line ends LF or CRLF, blank lines around it, and a
/// CRC-24 line that is there and right. The bytes are in memory that is
/// wiped. `None` for anything else.
pub fn dearmor(begin: &[u8], end: &[u8], bytes: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    if !bytes.is_ascii() {
        return None;
    }
    let mut lines = bytes
        .trim_ascii()
        .split(|&b| b == b'\n')
        .map(|l| l.strip_suffix(b"\r").unwrap_or(l));
    if lines.next()? != begin {
        return None;
    }
    // Headers, `Name: value`, up to the blank line.
    loop {
        let line = lines.next()?;
        if line.is_empty() {
            break;
        }
        let colon = line.windows(2).position(|w| w == b": ")?;
        if colon == 0 || !line.iter().all(|&b| (0x20..0x7f).contains(&b)) {
            return None;
        }
    }
    let mut b64 = Zeroizing::new(Vec::with_capacity(bytes.len()));
    let mut checksum = None;
    loop {
        let line = lines.next()?;
        if line == end {
            break;
        }
        if line.len() == 5 && line[0] == b'=' {
            let mut crc = [0u8; 3];
            Base64::decode(&line[1..], &mut crc).ok()?;
            checksum = Some(u32::from_be_bytes([0, crc[0], crc[1], crc[2]]));
            if lines.next()? != end {
                return None;
            }
            break;
        }
        if line.is_empty() {
            return None;
        }
        b64.extend_from_slice(line);
    }
    if lines.next().is_some() {
        return None;
    }
    let mut data = Zeroizing::new(alloc::vec![0u8; b64.len()]);
    let n = Base64::decode(&b64[..], &mut data).ok()?.len();
    data.truncate(n);
    (!data.is_empty() && checksum == Some(crc24(&data))).then_some(data)
}

/// One packet at `at` in the new format Faraday writes: its tag, its body
/// and where it ends.
fn packet_at(data: &[u8], at: usize) -> Option<(u8, &[u8], usize)> {
    let first = *data.get(at)?;
    if first & 0xC0 != 0xC0 {
        return None;
    }
    let (len, octets) = new_format_whole(data, at.checked_add(1)?, NewFormat::Whole)?;
    let from = at.checked_add(1 + octets)?;
    let to = from.checked_add(len)?;
    Some((first & 0x3f, data.get(from..to)?, to))
}

/// A secret key packet's seed and creation time: the public body of an
/// Ed25519 key, string-to-key usage 0, the seed as an MPI and its
/// checksum.
fn seed_of(body: &[u8]) -> Option<(Zeroizing<[u8; 32]>, u32)> {
    let created = u32::from_be_bytes(body.get(1..5)?.try_into().ok()?);
    let public = public_body(&[0; 32], created).len();
    if body.len() < public + 5 || body[public] != 0 {
        return None;
    }
    let bits = usize::from(u16::from_be_bytes([body[public + 1], body[public + 2]]));
    let n = bits.div_ceil(8);
    if n > 32 || body.len() != public + 3 + n + 2 {
        return None;
    }
    let mut seed = Zeroizing::new([0u8; 32]);
    seed[32 - n..].copy_from_slice(&body[public + 3..public + 3 + n]);
    Some((seed, created))
}

/// Costs Ed25519 signatures. The key `armored` holds and the time its
/// self-signatures were made, when it is exactly what [`armor_secret`]
/// writes for that key at that time (see the file's text); otherwise
/// `None`. The seeds are wiped when the key drops.
pub fn read_secret_at(armored: &[u8]) -> Option<(Key, u32)> {
    let data = dearmor(BEGIN, END, armored)?;
    let mut primary = None;
    let mut subkey = None;
    let mut user_ids = Vec::new();
    let mut public = Vec::with_capacity(data.len());
    let mut at = 0;
    while at < data.len() {
        let (t, body, end) = packet_at(&data, at)?;
        match t {
            tag::SECRET_KEY if primary.is_none() && at == 0 => {
                let (seed, created) = seed_of(body)?;
                public.extend(crate::packet(tag::PUBLIC_KEY, &public_body(&seed, created)));
                primary = Some((seed, created));
            }
            tag::SECRET_SUBKEY if subkey.is_none() && primary.is_some() => {
                let (seed, created) = seed_of(body)?;
                public.extend(crate::packet(
                    tag::PUBLIC_SUBKEY,
                    &public_body(&seed, created),
                ));
                subkey = Some((seed, created));
            }
            tag::USER_ID if subkey.is_none() => {
                user_ids.push(core::str::from_utf8(body).ok()?.into());
                public.extend_from_slice(&data[at..end]);
            }
            tag::SIGNATURE => public.extend_from_slice(&data[at..end]),
            _ => return None,
        }
        at = end;
    }
    let (primary, created) = primary?;
    let (subkey, subkey_created) = subkey?;
    let cert = Certificate::read(armor(Armor::PublicKey, &public).as_bytes()).ok()?;
    let at = cert.certified_at()?;
    // The seeds are copied into the key from references, so no by-value
    // copy of either is left on the stack; the key itself is moved by
    // value into the `then_some` below and again in `read_secret`, and
    // each move may leave a copy in that function's frame, not wiped.
    let mut key = Key {
        primary_seed: [0; 32],
        created,
        subkey_seed: [0; 32],
        subkey_created,
        user_ids,
        expiry: cert.expiry(),
    };
    key.primary_seed.copy_from_slice(&primary[..]);
    key.subkey_seed.copy_from_slice(&subkey[..]);
    bool::from(key.secret_key(at).ct_eq(&data[..])).then_some((key, at))
}

/// Costs Ed25519 signatures. The key `armored` holds, when it is exactly
/// what [`armor_secret`] writes ([`read_secret_at`]); otherwise `None`.
pub fn read_secret(armored: &[u8]) -> Option<Key> {
    read_secret_at(armored).map(|(key, _)| key)
}
