//! What Faraday writes (`docs/APK-SIGNING.md` §6): the content digest of
//! an unsigned APK, the v2 and v3 signer blocks, the APK Signing Block,
//! and the signed file, which is the unsigned one with the block inserted
//! and one offset changed.
//!
//! Every signature is checked against the certificate, with the public
//! key read from the certificate and not from the private key, before
//! anything is returned: a signature made with a fault in the private-key
//! operation would give the key away to anyone holding it and the
//! certificate.

use alloc::vec;
use alloc::vec::Vec;

use faraday_sb::rng::Stream;
use faraday_sb::x509;
use sha2::{Digest, Sha256};

use crate::zip::{Apk, BLOCK_MAGIC, Source};
use crate::{Error, key};

/// RSASSA-PKCS1-v1_5 with SHA-256 and the 1 MiB chunk digest: the APK
/// signature algorithm `0x0103`.
const ALGORITHM: u32 = 0x0103;
/// The v2 block's ID.
const V2_ID: u32 = 0x7109_871a;
/// The v3 block's ID.
const V3_ID: u32 = 0xf053_68c0;
/// The padding pair's ID.
const PADDING_ID: u32 = 0x4272_6577;
/// The v2 attribute that says a v3 block was there.
const V3_STRIPPING_ID: u32 = 0xbeef_f00d;
/// The chunk a section is cut into.
const CHUNK: usize = 1 << 20;

/// The v3 signer's SDK range for an APK whose `minSdkVersion` is
/// `min_sdk`, as `apksigner` writes it (`docs/APK-SIGNING.md` §2.2).
pub fn sdk_range(min_sdk: u32) -> (u32, u32) {
    // Measured from `apksigner` 36.0.0 on fixtures whose `minSdkVersion`
    // is 21, 26 and 33: the range is 24 (Android 7.0, the oldest the
    // refusals of `Apk::read` let through) to 2147483647 whatever the
    // APK's minimum is.
    let _ = min_sdk;
    (24, i32::MAX as u32)
}

/// The APK Signing Block starts at a multiple of this many bytes, as
/// `apksigner` places it: zero bytes after the last entry make it so.
pub const PAGE: u64 = 4096;

/// The zero bytes between the last entry and the block for an APK whose
/// central directory starts at `cd_offset`.
pub fn padding_before_block(cd_offset: u64) -> u64 {
    (PAGE - cd_offset % PAGE) % PAGE
}

/// What signing makes: the block to insert and the end record to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signed {
    /// Zero bytes after the last entry, so that the block starts on a
    /// page; they are part of what the content digest covers.
    pub padding: u64,
    /// The APK Signing Block, inserted before the central directory.
    pub block: Vec<u8>,
    /// The end of central directory record, its central directory
    /// offset raised by the block's length.
    pub eocd: Vec<u8>,
}

fn lp(b: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(4 + b.len());
    v.extend_from_slice(&(b.len() as u32).to_le_bytes());
    v.extend_from_slice(b);
    v
}

fn digest_chunk(data: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update([0xa5]);
    h.update((data.len() as u32).to_le_bytes());
    h.update(data);
    h.finalize().into()
}

/// The content digest of an unsigned APK: the entries, the central
/// directory and the end record, each in 1 MiB chunks, the top digest
/// over the chunk digests (`docs/APK-SIGNING.md` §6 step 1).
pub fn content_digest(src: &mut dyn Source, apk: &Apk, padding: u64) -> Result<[u8; 32], Error> {
    let mut digests: Vec<u8> = Vec::new();
    let mut count = 0u32;
    let mut buf = vec![0u8; CHUNK];
    // The entries and the zero bytes after them are one section; the
    // central directory is the next.
    for (start, len, real) in [
        (0, apk.cd_offset + padding, apk.cd_offset),
        (apk.cd_offset, apk.cd_size, apk.cd_size),
    ] {
        let mut done = 0u64;
        while done < len {
            let n = (len - done).min(CHUNK as u64) as usize;
            buf[..n].fill(0);
            let have = real.saturating_sub(done).min(n as u64) as usize;
            if have > 0 && !src.read_at(start + done, &mut buf[..have]) {
                return Err(Error::Read);
            }
            digests.extend_from_slice(&digest_chunk(&buf[..n]));
            count += 1;
            done += n as u64;
        }
    }
    // The end record counts as if its central directory offset were
    // where the signing block starts, which is what a verifier restores.
    let mut eocd = apk.eocd.clone();
    let block_start = u32::try_from(apk.cd_offset + padding).map_err(|_| Error::Read)?;
    eocd[16..20].copy_from_slice(&block_start.to_le_bytes());
    for part in eocd.chunks(CHUNK) {
        digests.extend_from_slice(&digest_chunk(part));
        count += 1;
    }
    let mut top = Sha256::new();
    top.update([0x5a]);
    top.update(count.to_le_bytes());
    top.update(&digests);
    Ok(top.finalize().into())
}

/// A signature record: the algorithm and the signature.
fn signature_record(sig: &[u8]) -> Vec<u8> {
    let mut r = ALGORITHM.to_le_bytes().to_vec();
    r.extend_from_slice(&lp(sig));
    lp(&r)
}

fn digests_field(digest: &[u8; 32]) -> Vec<u8> {
    let mut d = ALGORITHM.to_le_bytes().to_vec();
    d.extend_from_slice(&lp(digest));
    lp(&lp(&d))
}

fn sign_checked(rng: &mut Stream, key: &[u8], cert: &[u8], data: &[u8]) -> Result<Vec<u8>, Error> {
    let sig = x509::sign(rng, key, data).map_err(|_| Error::Key)?;
    if !x509::verify(cert, data, &sig) {
        return Err(Error::Signature);
    }
    Ok(sig)
}

fn v2_value(
    rng: &mut Stream,
    key: &[u8],
    cert: &[u8],
    spki: &[u8],
    digest: &[u8; 32],
) -> Result<Vec<u8>, Error> {
    let mut attribute = V3_STRIPPING_ID.to_le_bytes().to_vec();
    attribute.extend_from_slice(&3u32.to_le_bytes());
    let mut signed = digests_field(digest);
    signed.extend_from_slice(&lp(&lp(cert)));
    signed.extend_from_slice(&lp(&lp(&attribute)));
    // The fourth element `apksigner` writes, empty.
    signed.extend_from_slice(&lp(&[]));
    let sig = sign_checked(rng, key, cert, &signed)?;
    let mut signer = lp(&signed);
    signer.extend_from_slice(&lp(&signature_record(&sig)));
    signer.extend_from_slice(&lp(spki));
    Ok(lp(&lp(&signer)))
}

fn v3_value(
    rng: &mut Stream,
    key: &[u8],
    cert: &[u8],
    spki: &[u8],
    digest: &[u8; 32],
    sdk: (u32, u32),
) -> Result<Vec<u8>, Error> {
    let mut signed = digests_field(digest);
    signed.extend_from_slice(&lp(&lp(cert)));
    signed.extend_from_slice(&sdk.0.to_le_bytes());
    signed.extend_from_slice(&sdk.1.to_le_bytes());
    signed.extend_from_slice(&lp(&[]));
    let sig = sign_checked(rng, key, cert, &signed)?;
    let mut signer = lp(&signed);
    signer.extend_from_slice(&sdk.0.to_le_bytes());
    signer.extend_from_slice(&sdk.1.to_le_bytes());
    signer.extend_from_slice(&lp(&signature_record(&sig)));
    signer.extend_from_slice(&lp(spki));
    Ok(lp(&lp(&signer)))
}

/// The APK Signing Block around ID-value pairs, padded with the padding
/// pair to a multiple of 4096 bytes as `apksigner` pads it.
fn signing_block(mut pairs: Vec<(u32, Vec<u8>)>) -> Vec<u8> {
    let sum = |pairs: &[(u32, Vec<u8>)]| -> usize { pairs.iter().map(|p| 8 + 4 + p.1.len()).sum() };
    let total = 8 + sum(&pairs) + 8 + 16;
    if total % 4096 != 0 {
        let mut pad = 4096 - total % 4096;
        if pad < 12 {
            pad += 4096;
        }
        pairs.push((PADDING_ID, vec![0; pad - 12]));
    }
    let size = (sum(&pairs) + 8 + 16) as u64;
    let mut out = Vec::with_capacity(size as usize + 8);
    out.extend_from_slice(&size.to_le_bytes());
    for (id, value) in &pairs {
        out.extend_from_slice(&((4 + value.len()) as u64).to_le_bytes());
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(value);
    }
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(BLOCK_MAGIC);
    out
}

/// Signs an APK read by [`Apk::read`] with `key` (PKCS #8 DER), whose
/// certificate is `cert`. `rng` blinds each private-key operation and is
/// the caller's, keyed with bytes of the system's entropy of this signing
/// alone (`docs/APK-SIGNING.md` §2.3).
pub fn sign(
    rng: &mut Stream,
    src: &mut dyn Source,
    apk: &Apk,
    key: &[u8],
    cert: &[u8],
) -> Result<Signed, Error> {
    let spki = key::public_key(cert).ok_or(Error::Key)?;
    let padding = padding_before_block(apk.cd_offset);
    let digest = content_digest(src, apk, padding)?;
    let v2 = v2_value(rng, key, cert, spki, &digest)?;
    let v3 = v3_value(rng, key, cert, spki, &digest, sdk_range(apk.info.min_sdk))?;
    let block = signing_block(vec![(V2_ID, v2), (V3_ID, v3)]);
    // The end record's central directory offset, raised by the block.
    let offset = apk
        .cd_offset
        .checked_add(padding + block.len() as u64)
        .and_then(|o| u32::try_from(o).ok())
        .ok_or(Error::Read)?;
    let mut eocd = apk.eocd.clone();
    eocd[16..20].copy_from_slice(&offset.to_le_bytes());
    Ok(Signed {
        padding,
        block,
        eocd,
    })
}

/// The signed file: the entries, zero bytes to the next page, the block,
/// the central directory unchanged, and the end record.
pub fn assemble(src: &mut dyn Source, apk: &Apk, signed: &Signed) -> Result<Vec<u8>, Error> {
    let cd_end = apk.cd_offset + apk.cd_size;
    let mut out = Vec::with_capacity(apk.size as usize + signed.block.len());
    let mut buf = vec![0u8; apk.cd_offset as usize];
    if !src.read_at(0, &mut buf) {
        return Err(Error::Read);
    }
    out.extend_from_slice(&buf);
    out.resize(out.len() + signed.padding as usize, 0);
    out.extend_from_slice(&signed.block);
    let mut cd = vec![0u8; (cd_end - apk.cd_offset) as usize];
    if !src.read_at(apk.cd_offset, &mut cd) {
        return Err(Error::Read);
    }
    out.extend_from_slice(&cd);
    out.extend_from_slice(&signed.eocd);
    Ok(out)
}

/// The unsigned file back from a signed one whose entries were
/// `entries_len` bytes long: the padding and the block removed and the
/// offset restored. The length is the caller's because the padding is
/// zeros, which the last entry's data may end in too. `None` when
/// `signed` is not what [`assemble`] writes.
pub fn strip(signed: &[u8], entries_len: usize) -> Option<Vec<u8>> {
    // The signed file fails `Apk::read` as already signed; find the end
    // record and the block by hand.
    let n = signed.len();
    let tail = &signed[n.saturating_sub(22 + 65_535)..];
    let i = (0..tail.len().checked_sub(21)?).rev().find(|&i| {
        tail[i..i + 4] == [0x50, 0x4b, 0x05, 0x06]
            && i + 22 + u16::from_le_bytes([tail[i + 20], tail[i + 21]]) as usize == tail.len()
    })?;
    let eocd_at = n - tail.len() + i;
    let cd_off = u32::from_le_bytes(signed[eocd_at + 16..eocd_at + 20].try_into().ok()?) as usize;
    let block_size = u64::from_le_bytes(
        signed
            .get(cd_off.checked_sub(24)?..cd_off - 16)?
            .try_into()
            .ok()?,
    ) as usize;
    let block_start = cd_off.checked_sub(block_size + 8)?;
    let start = entries_len;
    if start > block_start || signed[start..block_start].iter().any(|&b| b != 0) {
        return None;
    }
    let mut out = signed[..start].to_vec();
    out.extend_from_slice(&signed[cd_off..eocd_at]);
    let mut eocd = signed[eocd_at..].to_vec();
    eocd[16..20].copy_from_slice(&(start as u32).to_le_bytes());
    out.extend_from_slice(&eocd);
    Some(out)
}
