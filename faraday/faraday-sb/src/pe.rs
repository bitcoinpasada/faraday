//! Authenticode for a PE/COFF image such as `BOOTX64.EFI` (Microsoft's
//! "Windows Authenticode Portable Executable Signature Format"): the image
//! hash, a SHA-256 signature by the db key appended as the certificate
//! table, and checking an existing signature against a db certificate.

use alloc::vec::Vec;

use sha2::{Digest, Sha256};

use der::asn1::Any;
use der::{Decode, Encode, SliceReader};

use crate::Error;
use crate::cms::{self, ContentInfo, SignedData, SignerInfo};
use crate::x509;

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
    // The certificate table follows the headers (it is appended to the
    // image); one that starts inside them would overlap the very fields
    // signing rewrites.
    if let Some((a, n)) = certs
        && (a < size_of_headers || a + n > pe.len())
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

/// The image as signing takes it: any certificate table dropped, its
/// entry zeroed, and padded with zeros to a multiple of eight bytes.
fn unsigned(pe: &[u8]) -> Result<(Layout, Vec<u8>), Error> {
    let l = layout(pe)?;
    let mut image = pe.to_vec();
    if let Some((at, _)) = l.certs {
        image.truncate(at);
    }
    image[l.cert_dir..l.cert_dir + 8].fill(0);
    while !image.len().is_multiple_of(8) {
        image.push(0);
    }
    Ok((l, image))
}

/// Costs a SHA-256 of the image: the image hash it has once signed, by
/// Faraday's signing or any signer that pads it to eight bytes as Authenticode
/// does. Equal to [`image_hash`] of every signed copy of the same program,
/// whoever signed it; a byte changed anywhere it covers gives another.
pub fn hash_as_signed(pe: &[u8]) -> Result<[u8; 32], Error> {
    image_hash(&unsigned(pe)?.1)
}

/// Costs an RSA signature. Any image signed by `signer`, with no release
/// behind it: for this crate's tests and examples, the Starter's tests and
/// the fuzz targets, under the `sign-any` feature, which no shipped binary
/// turns on. The app signs through [`crate::release::sign`] (F134).
#[cfg(feature = "sign-any")]
pub fn sign(
    rng: &mut crate::rng::Stream,
    pe: &[u8],
    signer: &x509::Pair,
) -> Result<Vec<u8>, Error> {
    sign_image(rng, pe, signer)
}

/// Costs an RSA signature. The image signed by `signer` (the db key):
/// any certificate table it had is dropped, the image is padded to eight
/// bytes, and the signature goes after it as the new table. `rng` blinds
/// the signing. Reached only through [`crate::release::sign`], or
/// `sign` under `sign-any`.
pub(crate) fn sign_image(
    rng: &mut crate::rng::Stream,
    pe: &[u8],
    signer: &x509::Pair,
) -> Result<Vec<u8>, Error> {
    let (l, mut image) = unsigned(pe)?;
    let digest = image_hash(&image)?;
    let content = Any::encode_from(&cms::spc_content(&digest)?)?;
    let content_digest: [u8; 32] = Sha256::digest(content.value()).into();
    let attrs = cms::image_attributes(&content_digest)?;
    let sig = x509::sign(rng, &signer.key, &attrs.to_der()?)?;
    let info = x509::signer_info(&signer.cert, Some(attrs), &sig)?;
    let signed_data = SignedData::new(cms::SPC_INDIRECT_DATA, Some(content), &signer.cert, &info)?;
    let content_info = ContentInfo(signed_data).to_der()?;
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

/// Why [`transplant`] took no signature from a signed copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transplant {
    /// The release's own image is not one this crate reads.
    Release,
    /// The signed copy is not the release's program: not an image, or its
    /// image hash is another's.
    Other,
    /// The signed copy carries no certificate table.
    Unsigned,
    /// Its certificate table is not one signature alone at its end:
    /// bytes after it, a table that does not end the file, or more than
    /// one entry in it.
    Layout,
}

/// Costs two Authenticode image hashes: the release's image, prepared as
/// signing prepares it (padded to eight bytes, no table), with the
/// checksum and certificate-table entry of `signed` and then `signed`'s
/// certificate table, when `signed` is the same program with one
/// signature appended and nothing else. Every byte the signature does
/// not cover is then the release's by construction; whether the
/// signature holds is [`signer`]'s to say, on the result.
pub fn transplant(release: &[u8], signed: &[u8]) -> Result<Vec<u8>, Transplant> {
    let (l, mut image) = unsigned(release).map_err(|_| Transplant::Release)?;
    let s = layout(signed).map_err(|_| Transplant::Other)?;
    let ours = image_hash(&image).map_err(|_| Transplant::Release)?;
    if image_hash(signed).map_err(|_| Transplant::Other)? != ours
        || s.checksum != l.checksum
        || s.cert_dir != l.cert_dir
    {
        return Err(Transplant::Other);
    }
    let Some((at, size)) = s.certs else {
        return Err(Transplant::Unsigned);
    };
    // One WIN_CERTIFICATE, its length rounded to eight the whole table,
    // the table where the release's image ends and ending the file.
    let first = u32_at(signed, at).ok_or(Transplant::Layout)?;
    if at != image.len() || at + size != signed.len() || first < 8 || first.div_ceil(8) * 8 != size
    {
        return Err(Transplant::Layout);
    }
    image[l.checksum..l.checksum + 4].copy_from_slice(&signed[s.checksum..s.checksum + 4]);
    image[l.cert_dir..l.cert_dir + 8].copy_from_slice(&signed[s.cert_dir..s.cert_dir + 8]);
    image.extend_from_slice(&signed[at..]);
    Ok(image)
}

/// Who signed an image, as the signature itself says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signer {
    /// No signature.
    Unsigned,
    /// A signature that no certificate it carries checks: unreadable,
    /// over another image, or by a key it does not carry.
    Broken,
    /// Signed over this image by the key of this certificate (DER),
    /// which the signature carries.
    By(Vec<u8>),
}

/// Costs an RSA verification per certificate the signature carries: the
/// certificate, of those carried in the image's first signature, whose
/// key made that signature over this image. Says nothing of whether a
/// computer trusts it: that is its db's to say.
pub fn signer(pe: &[u8]) -> Result<Signer, Error> {
    let l = layout(pe)?;
    let Some((at, size)) = l.certs else {
        return Ok(Signer::Unsigned);
    };
    let carried = carried_certs(pe, at, size).unwrap_or_default();
    for cert in carried {
        if read_signature(pe, at, size, &cert) == Some(Check::Valid) {
            return Ok(Signer::By(cert));
        }
    }
    Ok(Signer::Broken)
}

/// The SignedData of the first signature, read from the certificate
/// table: its 8-byte header, then a ContentInfo, then padding.
fn signed_data(pe: &[u8], at: usize, size: usize) -> Option<SignedData> {
    let table = pe.get(at..at + size)?;
    if u16_at(table, 6)? != 0x0002 {
        return None;
    }
    // The table's own length field may be under the 8-byte header.
    let ci = table.get(8..u32_at(table, 0)?.min(table.len()))?;
    let mut reader = SliceReader::new(ci).ok()?;
    ContentInfo::decode(&mut reader).ok().map(|c| c.0)
}

/// The certificates the first signature carries: SignedData's
/// `[0] certificates`, each whole.
fn carried_certs(pe: &[u8], at: usize, size: usize) -> Option<Vec<Vec<u8>>> {
    let sd = signed_data(pe, at, size)?;
    sd.certs.iter().map(|c| c.to_der().ok()).collect()
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
    let sd = signed_data(pe, at, size)?;
    // The content is Authenticode's SpcIndirectDataContent.
    let spc = sd.content.as_ref()?;
    let content: cms::SpcIndirectData = Decode::from_der(&spc.to_der().ok()?).ok()?;
    if content.message_digest.digest.as_bytes() != image_hash(pe).ok()? {
        return Some(Check::Altered);
    }
    let info_der = sd.signer_infos.first()?.to_der().ok()?;
    let info = SignerInfo::from_der(&info_der).ok()?;
    let attrs = info.signed_attrs?;
    // RFC 5652 11.2: exactly one message digest attribute, with one value,
    // and it must name the content.
    let content_digest: [u8; 32] = Sha256::digest(spc.value()).into();
    let mut digests = attrs.iter().filter(|a| a.oid == cms::MESSAGE_DIGEST);
    let named_digest = match (digests.next(), digests.next()) {
        (Some(a), None) => {
            let mut values = a.values.iter();
            match (values.next(), values.next()) {
                (Some(v), None) => v.value() == content_digest,
                _ => return None,
            }
        }
        (Some(_), Some(_)) => return None,
        _ => false,
    };
    if !named_digest {
        return Some(Check::Altered);
    }
    let ok = x509::verify(
        cert,
        &cms::signed_attrs_bytes(&info_der)?,
        info.signature.as_bytes(),
    );
    Some(if ok { Check::Valid } else { Check::OtherSigner })
}
