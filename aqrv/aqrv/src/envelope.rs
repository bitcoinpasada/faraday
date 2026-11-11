//! Making and reading an envelope (`docs/AQRV.md` §6): the keys from
//! Argon2id and HKDF-Expand, ChaCha20 over `P`, and HMAC-SHA-256 for the
//! tag of every kind.

use argon2::Algorithm;
use chacha20::ChaCha20;
use chacha20::cipher::{KeyIvInit, StreamCipher};
use faraday_crypto::work::Compute;
use faraday_crypto::{Argon2Cost, Argon2Error, Secret, hkdf_sha256_expand, hmac_sha256};
use subtle::ConstantTimeEq;
use unicode_normalization::UnicodeNormalization;
use zeroize::Zeroizing;

use crate::item::{self, Item};
use crate::{Header, Kind, NotOpened, Refused, Size, Step, recognise};

/// §6's HKDF labels, which name the format.
const LABEL_ENC: &[u8] = b"argon-qr-vault-1 enc";
const LABEL_MAC: &[u8] = b"argon-qr-vault-1 mac";

/// §5: three passes over the memory, four lanes.
const PASSES: u32 = 3;
const LANES: u32 = 4;

/// An envelope opened: what §9 read from it, and the item.
pub struct Opened {
    /// The size and step.
    pub header: Header,
    /// The item, read by its kind's layout.
    pub item: Item,
}

/// Every intermediate value of §6, as the vectors list them. Nothing here
/// is printable.
pub struct Workings {
    /// UTF-8 of the passphrase's NFC form.
    pub pw: Zeroizing<Vec<u8>>,
    /// Argon2id's output.
    pub prk: Zeroizing<Vec<u8>>,
    /// The encryption key.
    pub k_enc: Zeroizing<Vec<u8>>,
    /// The MAC key.
    pub k_mac: Zeroizing<Vec<u8>>,
    /// ChaCha20's first room bytes.
    pub keystream: Zeroizing<Vec<u8>>,
    /// `P`: `C` decrypted, whether or not any tag matched.
    pub p: Zeroizing<Vec<u8>>,
    /// The envelope's `C`.
    pub c: Zeroizing<Vec<u8>>,
    /// The HMAC's message, `H || N || salt || kind || C`, of the one kind
    /// whose tag matches; None when none or more than one does.
    pub message: Option<Zeroizing<Vec<u8>>>,
    /// Each kind's tag, cut to the size's tag length (the rest is zero).
    pub tags: Vec<(Kind, [u8; 8])>,
}

/// The three values Argon2id and HKDF-Expand give (§6).
struct Keys {
    prk: Zeroizing<Vec<u8>>,
    k_enc: Zeroizing<Vec<u8>>,
    k_mac: Zeroizing<Vec<u8>>,
}

/// Costs one Argon2id run at `step`'s memory: `pw` and `salt` to the two
/// keys (§6).
fn derive_keys(c: &Compute, pw: &[u8], salt: &[u8], step: Step) -> Result<Keys, Argon2Error> {
    let cost = Argon2Cost {
        memory_kib: step.memory_kib(),
        passes: PASSES,
        lanes: LANES,
    };
    let mut prk = Secret::new([0u8; 32]);
    faraday_crypto::argon2(c, Algorithm::Argon2id, pw, salt, cost, &mut prk)?;
    let expand = |label: &[u8]| {
        let mut key = Zeroizing::new(vec![0u8; 32]);
        hkdf_sha256_expand(prk.expose(), label, &mut key).expect("32 bytes is within HKDF's limit");
        key
    };
    Ok(Keys {
        k_enc: expand(LABEL_ENC),
        k_mac: expand(LABEL_MAC),
        prk: Zeroizing::new(prk.expose().to_vec()),
    })
}

/// §10: the UTF-8 bytes of the passphrase's NFC form.
fn passphrase_bytes(passphrase: &str) -> Zeroizing<Vec<u8>> {
    // NFC can lengthen text, so this leaves room for it: nothing is
    // reallocated, and no copy is left unwiped.
    let mut nfc = Zeroizing::new(String::with_capacity(passphrase.len() * 4));
    nfc.extend(passphrase.nfc());
    Zeroizing::new(nfc.as_bytes().to_vec())
}

/// §6: ChaCha20 under `k_enc`, a nonce of twelve zero bytes and counter 0,
/// the first `room` bytes.
fn keystream(k_enc: &[u8], room: usize) -> Zeroizing<Vec<u8>> {
    let mut stream = Zeroizing::new(vec![0u8; room]);
    ChaCha20::new_from_slices(k_enc, &[0u8; 12])
        .expect("a 32-byte key and a 12-byte nonce")
        .apply_keystream(&mut stream);
    stream
}

/// §6: `P` XOR `stream`, which encrypts and decrypts alike.
fn xor(a: &[u8], stream: &[u8]) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(a.iter().zip(stream).map(|(x, y)| x ^ y).collect())
}

/// §6: `H || N || salt || kind || C`.
fn message(header: u8, size: Size, salt: &[u8], kind: Kind, c: &[u8]) -> Zeroizing<Vec<u8>> {
    let mut m = Zeroizing::new(Vec::with_capacity(3 + salt.len() + c.len()));
    m.push(header);
    m.push(size.bytes() as u8);
    m.extend_from_slice(salt);
    m.push(kind as u8);
    m.extend_from_slice(c);
    m
}

/// §6: the tag of every kind the size holds, each cut to the size's tag
/// length. A kind the size does not hold is never computed.
fn all_tags(k_mac: &[u8], header: u8, size: Size, salt: &[u8], c: &[u8]) -> Vec<(Kind, [u8; 8])> {
    let held = Kind::ALL.into_iter().filter(|kind| size.holds(*kind));
    held.map(|kind| {
        let full = hmac_sha256(k_mac, &message(header, size, salt, kind, c));
        let mut tag = [0u8; 8];
        tag[..size.tag_len()].copy_from_slice(&full[..size.tag_len()]);
        (kind, tag)
    })
    .collect()
}

/// Costs one Argon2id run at the memory of `step`: makes an envelope
/// (§6 Writing). `random` fills each buffer it is given, in the order of
/// §6: the salt (16 bytes, 11 at the 32-byte size), then for a codex32 string one byte for the
/// item's unused low bits, then the padding (not asked for when the item
/// fills the room). If another kind's tag equals this kind's, all three
/// are drawn again.
pub fn seal(
    c: &Compute,
    item: &Item,
    passphrase: &str,
    step: Step,
    size: Size,
    random: &mut dyn FnMut(&mut [u8]),
) -> Result<Vec<u8>, Refused> {
    if passphrase.is_empty() {
        return Err(Refused::EmptyPassphrase);
    }
    let laid = item::lay_out(item, size)?;
    let pw = passphrase_bytes(passphrase);
    let header = step.header();
    loop {
        let mut salt = vec![0u8; size.salt_len()];
        random(&mut salt);

        // `P`: the item, its unused low bits filled, then the padding.
        let mut p = Zeroizing::new(Vec::with_capacity(size.room()));
        p.extend_from_slice(&laid.bytes);
        if let Some(mask) = laid.unused_mask {
            let mut fill = [0u8; 1];
            random(&mut fill);
            *p.last_mut().expect("an item has bytes") |= fill[0] & mask;
        }
        let item_len = p.len();
        p.resize(size.room(), 0);
        if item_len < size.room() {
            random(&mut p[item_len..]);
        }

        let keys = derive_keys(c, &pw, &salt, step).map_err(|_| Refused::NotEnoughMemory)?;
        let encrypted = xor(&p, &keystream(&keys.k_enc, size.room()));
        let tags = all_tags(&keys.k_mac, header, size, &salt, &encrypted);

        // §6 Writing step 3: a tag another kind shares means another try.
        let own = tags.iter().find(|(kind, _)| *kind == laid.kind);
        let own = own.expect("every kind has a tag").1;
        let shared = tags
            .iter()
            .filter(|(kind, tag)| *kind != laid.kind && bool::from(tag.ct_eq(&own)))
            .count();
        if shared > 0 {
            continue;
        }

        let mut out = vec![header];
        out.extend_from_slice(&salt);
        out.extend_from_slice(&encrypted);
        out.extend_from_slice(&own[..size.tag_len()]);
        return Ok(out);
    }
}

/// What reading made: the header, the values, and which kinds' tags match.
struct Read {
    header: Header,
    workings: Workings,
    matches: Vec<Kind>,
}

/// Costs one Argon2id run at the envelope's step: §6 Reading steps 1 and
/// 2, every tag computed in full and compared with the stored one.
fn read(c: &Compute, envelope: &[u8], passphrase: &str) -> Result<Read, NotOpened> {
    let header = recognise(envelope).ok_or(NotOpened::NotRecognised)?;
    let size = header.size;
    let salt = &envelope[1..1 + size.salt_len()];
    let encrypted = &envelope[1 + size.salt_len()..envelope.len() - size.tag_len()];
    let stored = &envelope[envelope.len() - size.tag_len()..];

    let pw = passphrase_bytes(passphrase);
    let keys = derive_keys(c, &pw, salt, header.step).map_err(|_| NotOpened::NotEnoughMemory)?;
    let stream = keystream(&keys.k_enc, size.room());
    let tags = all_tags(&keys.k_mac, envelope[0], size, salt, encrypted);

    let mut matches = Vec::new();
    for (kind, tag) in &tags {
        if bool::from(tag[..size.tag_len()].ct_eq(stored)) {
            matches.push(*kind);
        }
    }
    let message = match matches[..] {
        [kind] => Some(message(envelope[0], size, salt, kind, encrypted)),
        _ => None,
    };
    let workings = Workings {
        p: xor(encrypted, &stream),
        c: Zeroizing::new(encrypted.to_vec()),
        pw,
        prk: keys.prk,
        k_enc: keys.k_enc,
        k_mac: keys.k_mac,
        keystream: stream,
        message,
        tags,
    };
    Ok(Read {
        header,
        workings,
        matches,
    })
}

/// Costs one Argon2id run at the envelope's step: opens an envelope
/// (§6 Reading). A wrong passphrase and a damaged code are both
/// [`NotOpened::NoTag`].
pub fn open(c: &Compute, envelope: &[u8], passphrase: &str) -> Result<Opened, NotOpened> {
    let read = read(c, envelope, passphrase)?;
    let kind = match read.matches[..] {
        [] => return Err(NotOpened::NoTag),
        [kind] => kind,
        _ => return Err(NotOpened::TwoTags),
    };
    let item = item::read(kind, &read.workings.p).map_err(|rule| NotOpened::Layout(kind, rule))?;
    Ok(Opened {
        header: read.header,
        item,
    })
}

/// Costs one Argon2id run at the envelope's step: §6's values for an
/// envelope and a passphrase, whether or not it opens.
pub fn workings(c: &Compute, envelope: &[u8], passphrase: &str) -> Result<Workings, NotOpened> {
    read(c, envelope, passphrase).map(|read| read.workings)
}
