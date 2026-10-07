//! Authenticode for a PE/COFF image such as `BOOTX64.EFI` (Microsoft's
//! "Windows Authenticode Portable Executable Signature Format"): the image
//! hash, a SHA-256 signature by the db key appended as the certificate
//! table, and checking an existing signature against a db certificate.

use alloc::vec::Vec;

use sha2::{Digest, Sha256};

use crate::Error;
use crate::der::{self, explicit, octets, oid, seq, set, tlv};
use crate::x509::{self, alg};

/// SPC_INDIRECT_DATA_OBJID.
const SPC_INDIRECT_DATA: &[u64] = &[1, 3, 6, 1, 4, 1, 311, 2, 1, 4];
/// SPC_PE_IMAGE_DATAOBJ.
const SPC_PE_IMAGE_DATA: &[u64] = &[1, 3, 6, 1, 4, 1, 311, 2, 1, 15];
/// SPC_SP_OPUS_INFO_OBJID.
const SPC_SP_OPUS_INFO: &[u64] = &[1, 3, 6, 1, 4, 1, 311, 2, 1, 12];
/// id-contentType.
const CONTENT_TYPE: &[u64] = &[1, 2, 840, 113_549, 1, 9, 3];
/// id-messageDigest.
const MESSAGE_DIGEST: &[u64] = &[1, 2, 840, 113_549, 1, 9, 4];
/// id-signedData.
const SIGNED_DATA: &[u64] = &[1, 2, 840, 113_549, 1, 7, 2];

/// Where the parts Authenticode needs are in an image.
struct Layout {
    checksum: usize,
    cert_dir: usize,
    size_of_headers: usize,
    sections: Vec<(usize, usize)>,
    /// The certificate table's offset and size, when there is one.
    certs: Option<(usize, usize)>,
}

fn u16_at(b: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?) as usize)
}

fn u32_at(b: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?) as usize)
}

fn layout(pe: &[u8]) -> Result<Layout, Error> {
    let bad = Error::Image;
    if pe.get(0..2) != Some(b"MZ") {
        return Err(bad);
    }
    let nt = u32_at(pe, 0x3c).ok_or(bad)?;
    if pe.get(nt..nt + 4) != Some(b"PE\0\0") {
        return Err(bad);
    }
    let coff = nt + 4;
    let sections = u16_at(pe, coff + 2).ok_or(bad)?;
    let opt_size = u16_at(pe, coff + 16).ok_or(bad)?;
    let opt = coff + 20;
    let (dirs, count) = match u16_at(pe, opt).ok_or(bad)? {
        0x20b => (opt + 112, u32_at(pe, opt + 108).ok_or(bad)?),
        0x10b => (opt + 96, u32_at(pe, opt + 92).ok_or(bad)?),
        _ => return Err(bad),
    };
    if count < 5 {
        return Err(bad);
    }
    let cert_dir = dirs + 4 * 8;
    let cert_at = u32_at(pe, cert_dir).ok_or(bad)?;
    let cert_size = u32_at(pe, cert_dir + 4).ok_or(bad)?;
    let table = opt + opt_size;
    let mut secs = Vec::new();
    for i in 0..sections {
        let s = table + i * 40;
        let size = u32_at(pe, s + 16).ok_or(bad)?;
        let at = u32_at(pe, s + 20).ok_or(bad)?;
        if size > 0 {
            if at + size > pe.len() {
                return Err(bad);
            }
            secs.push((at, size));
        }
    }
    secs.sort_unstable();
    let size_of_headers = u32_at(pe, opt + 60).ok_or(bad)?;
    if size_of_headers > pe.len() || cert_dir + 8 > size_of_headers {
        return Err(bad);
    }
    let certs = (cert_at != 0 && cert_size != 0).then_some((cert_at, cert_size));
    if let Some((a, n)) = certs
        && a + n > pe.len()
    {
        return Err(bad);
    }
    Ok(Layout {
        checksum: opt + 64,
        cert_dir,
        size_of_headers,
        sections: secs,
        certs,
    })
}

/// The Authenticode SHA-256 of an image: everything but its checksum, its
/// certificate-table entry and the certificate table.
pub fn image_hash(pe: &[u8]) -> Result<[u8; 32], Error> {
    let l = layout(pe)?;
    let mut h = Sha256::new();
    h.update(&pe[..l.checksum]);
    h.update(&pe[l.checksum + 4..l.cert_dir]);
    h.update(&pe[l.cert_dir + 8..l.size_of_headers]);
    let mut hashed = l.size_of_headers;
    for (at, size) in &l.sections {
        h.update(&pe[*at..at + size]);
        hashed += size;
    }
    let end = l.certs.map_or(pe.len(), |(a, _)| a);
    if end > hashed {
        h.update(&pe[hashed..end]);
    }
    Ok(h.finalize().into())
}

/// The SpcIndirectDataContent naming an image hash.
fn spc_content(digest: &[u8; 32]) -> Vec<u8> {
    let obsolete: Vec<u8> = "<<<Obsolete>>>"
        .encode_utf16()
        .flat_map(u16::to_be_bytes)
        .collect();
    let file = explicit(0, &tlv(0xa2, &tlv(0x80, &obsolete)));
    let image_data = seq(&[&tlv(der::tag::BIT_STRING, &[0]), &file]);
    seq(&[
        &seq(&[&oid(SPC_PE_IMAGE_DATA), &image_data]),
        &seq(&[&alg(x509::SHA256), &octets(digest)]),
    ])
}

/// The image signed by `signer` (the db key): any certificate table it
/// had is dropped, the image is padded to eight bytes, and the signature
/// goes after it as the new table.
pub fn sign(pe: &[u8], signer: &x509::Pair) -> Result<Vec<u8>, Error> {
    let l = layout(pe)?;
    let mut image = pe.to_vec();
    if let Some((at, _)) = l.certs {
        image.truncate(at);
    }
    image[l.cert_dir..l.cert_dir + 8].fill(0);
    while !image.len().is_multiple_of(8) {
        image.push(0);
    }
    let digest = image_hash(&image)?;
    let content = spc_content(&digest);
    let (_, value, _) = der::read(&content, 0).ok_or(Error::Image)?;
    let content_digest: [u8; 32] = Sha256::digest(&content[value]).into();
    let attrs = [
        seq(&[&oid(CONTENT_TYPE), &set(&[&oid(SPC_INDIRECT_DATA)])]),
        seq(&[&oid(SPC_SP_OPUS_INFO), &set(&[&seq(&[])])]),
        seq(&[&oid(MESSAGE_DIGEST), &set(&[&octets(&content_digest)])]),
    ];
    let attr_set = set(&[&attrs[0], &attrs[1], &attrs[2]]);
    let sig = x509::sign(&signer.key, &attr_set)?;
    let mut implicit = attr_set.clone();
    implicit[0] = 0xa0;
    let info = x509::signer_info(&signer.cert, Some(&implicit), &sig)?;
    let signed_data = seq(&[
        &der::int(1),
        &set(&[&alg(x509::SHA256)]),
        &seq(&[&oid(SPC_INDIRECT_DATA), &explicit(0, &content)]),
        &tlv(0xa0, &signer.cert),
        &set(&[&info]),
    ]);
    let content_info = seq(&[&oid(SIGNED_DATA), &explicit(0, &signed_data)]);
    let mut table = Vec::new();
    let length = (8 + content_info.len()).div_ceil(8) * 8;
    table.extend_from_slice(&(length as u32).to_le_bytes());
    table.extend_from_slice(&0x0200u16.to_le_bytes());
    table.extend_from_slice(&0x0002u16.to_le_bytes());
    table.extend_from_slice(&content_info);
    table.resize(length, 0);
    let at = image.len();
    image.extend_from_slice(&table);
    image[l.cert_dir..l.cert_dir + 4].copy_from_slice(&(at as u32).to_le_bytes());
    image[l.cert_dir + 4..l.cert_dir + 8].copy_from_slice(&(length as u32).to_le_bytes());
    Ok(image)
}

/// What checking an image's signature against a db certificate found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    /// No signature.
    Unsigned,
    /// Signed by this certificate's key, over this image.
    Valid,
    /// Signed, by another key.
    OtherSigner,
    /// The signature names a hash that is not this image's.
    Altered,
    /// The signature could not be read.
    Unreadable,
}

/// Checks the first signature in an image against `cert`.
pub fn check(pe: &[u8], cert: &[u8]) -> Result<Check, Error> {
    let l = layout(pe)?;
    let Some((at, size)) = l.certs else {
        return Ok(Check::Unsigned);
    };
    Ok(read_signature(pe, at, size, cert).unwrap_or(Check::Unreadable))
}

fn read_signature(pe: &[u8], at: usize, size: usize, cert: &[u8]) -> Option<Check> {
    let table = pe.get(at..at + size)?;
    if u16_at(table, 6)? != 0x0002 {
        return None;
    }
    let ci = &table[8..u32_at(table, 0)?.min(table.len())];
    // ContentInfo → [0] → SignedData.
    let (_, ci_v, _) = der::read(ci, 0)?;
    let parts = der::children(&ci[ci_v.clone()])?;
    let explicit_v = parts.get(1)?.1.clone();
    let inner = &ci[ci_v][explicit_v];
    let (_, sd_v, _) = der::read(inner, 0)?;
    let sd = &inner[sd_v];
    let fields = der::children(sd)?;
    // version, digestAlgorithms, encapContentInfo, [certificates], signerInfos.
    let encap = &sd[fields.get(2)?.1.clone()];
    let encap_parts = der::children(encap)?;
    let wrapped = &encap[encap_parts.get(1)?.1.clone()];
    let (_, spc_v, _) = der::read(wrapped, 0)?;
    let spc = &wrapped[spc_v.clone()];
    let spc_parts = der::children(spc)?;
    let digest_info = &spc[spc_parts.get(1)?.1.clone()];
    let di_parts = der::children(digest_info)?;
    let named = &digest_info[di_parts.get(1)?.1.clone()];
    if named != image_hash(pe).ok()? {
        return Some(Check::Altered);
    }
    let infos = &sd[fields.last()?.1.clone()];
    let (_, info_v, _) = der::read(infos, 0)?;
    let info = &infos[info_v];
    let mut attrs = None;
    let mut sig = None;
    for (t, r) in der::children(info)? {
        match t {
            0xa0 => attrs = Some(r),
            0x04 => sig = Some(r),
            _ => {}
        }
    }
    let attrs = attrs?;
    let mut attr_set = der::tlv(der::tag::SET, &info[attrs.clone()]);
    // The message digest attribute must name the content.
    let content_digest: [u8; 32] = Sha256::digest(spc).into();
    let attr_bytes = &info[attrs];
    let named_digest = der::children(attr_bytes)?.into_iter().any(|(_, a)| {
        let a = &attr_bytes[a];
        der::children(a).is_some_and(|p| {
            p.len() == 2 && a[p[0].1.clone()] == oid(MESSAGE_DIGEST)[2..] && {
                let set_v = &a[p[1].1.clone()];
                der::read(set_v, 0).is_some_and(|(_, v, _)| set_v[v] == content_digest)
            }
        })
    });
    if !named_digest {
        return Some(Check::Altered);
    }
    let ok = x509::verify(cert, &attr_set, &info[sig?]);
    attr_set.clear();
    Some(if ok { Check::Valid } else { Check::OtherSigner })
}
