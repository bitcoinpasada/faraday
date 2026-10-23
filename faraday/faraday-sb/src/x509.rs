//! An RSA-2048 key and its self-signed X.509 certificate, the shape a
//! Secure Boot PK, KEK or db key takes, and CMS signatures made with one.

use alloc::string::String;
use alloc::vec::Vec;

use rsa::RsaPrivateKey;
use rsa::pkcs1v15::{SigningKey, VerifyingKey};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey};
use rsa::signature::{RandomizedSigner, SignatureEncoding, Verifier};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::Error;
use crate::der::{self, bits, explicit, null, octets, oid, seq, set, tag, tlv, uint};

/// sha256WithRSAEncryption.
pub const SHA256_RSA: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];
/// rsaEncryption.
pub const RSA: &[u64] = &[1, 2, 840, 113_549, 1, 1, 1];
/// SHA-256.
pub const SHA256: &[u64] = &[2, 16, 840, 1, 101, 3, 4, 2, 1];

/// An algorithm identifier with NULL parameters.
pub fn alg(arcs: &[u64]) -> Vec<u8> {
    seq(&[&oid(arcs), &null()])
}

/// A key and its certificate. The key is PKCS #8 DER and secret.
pub struct Pair {
    /// The private key, PKCS #8 DER.
    pub key: Zeroizing<Vec<u8>>,
    /// The certificate, DER.
    pub cert: Vec<u8>,
}

/// A Unix time as a civil date and time.
pub fn civil(unix: u64) -> (i64, u32, u32, u32, u32, u32) {
    let days = (unix / 86_400) as i64;
    let secs = unix % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (
        y,
        m,
        d,
        (secs / 3600) as u32,
        (secs / 60 % 60) as u32,
        (secs % 60) as u32,
    )
}

/// UTCTime before 2050, GeneralizedTime after, as X.509 wants.
fn time(unix: u64) -> Vec<u8> {
    let (y, mo, d, h, mi, s) = civil(unix);
    if y < 2050 {
        let t = alloc::format!("{:02}{mo:02}{d:02}{h:02}{mi:02}{s:02}Z", y % 100);
        tlv(tag::UTC_TIME, t.as_bytes())
    } else {
        let t = alloc::format!("{y:04}{mo:02}{d:02}{h:02}{mi:02}{s:02}Z");
        tlv(tag::GENERALIZED_TIME, t.as_bytes())
    }
}

/// A name of one common name.
fn name(cn: &str) -> Vec<u8> {
    let atv = seq(&[&oid(&[2, 5, 4, 3]), &tlv(tag::UTF8_STRING, cn.as_bytes())]);
    seq(&[&set(&[&atv])])
}

fn private(key: &[u8]) -> Result<RsaPrivateKey, Error> {
    RsaPrivateKey::from_pkcs8_der(key).map_err(|_| Error::Key)
}

/// An RSA PKCS #1 v1.5 signature with SHA-256 over `data`. The private
/// key operation is blinded with draws from `rng`, so its timing says
/// nothing of the key; the signature itself is the same for any `rng`.
pub fn sign(rng: &mut crate::rng::Stream, key: &[u8], data: &[u8]) -> Result<Vec<u8>, Error> {
    let k = SigningKey::<Sha256>::new(private(key)?);
    Ok(k.try_sign_with_rng(rng, data)
        .map_err(|_| Error::Key)?
        .to_vec())
}

/// Whether `sig` is the certificate's key's signature over `data`.
pub fn verify(cert: &[u8], data: &[u8], sig: &[u8]) -> bool {
    let Some(spki) = Cert::read(cert).map(|c| cert[c.spki].to_vec()) else {
        return false;
    };
    let Ok(public) = rsa::RsaPublicKey::from_public_key_der(&spki) else {
        return false;
    };
    let Ok(sig) = rsa::pkcs1v15::Signature::try_from(sig) else {
        return false;
    };
    VerifyingKey::<Sha256>::new(public)
        .verify(data, &sig)
        .is_ok()
}

/// What kind of certificate [`generate`] makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    /// Secure Boot's: a CA certificate (basic constraints, key usage,
    /// subject key identifier), valid for this many years.
    Ca {
        /// Years from `now`.
        years: u32,
    },
}

/// A new key of `key_bits` bits, drawn from `rng`, with a self-signed
/// certificate: common name `cn`, serial `serial`, valid from `now` as the
/// `profile` says; `blind` blinds the signing.
pub fn generate(
    rng: &mut crate::rng::Stream,
    blind: &mut crate::rng::Stream,
    key_bits: usize,
    cn: &str,
    serial: &[u8; 16],
    now: u64,
    profile: Profile,
) -> Result<Pair, Error> {
    let key = RsaPrivateKey::new(rng, key_bits).map_err(|_| Error::Key)?;
    let spki = key
        .to_public_key()
        .to_public_key_der()
        .map_err(|_| Error::Key)?
        .as_bytes()
        .to_vec();
    let der = Zeroizing::new(
        key.to_pkcs8_der()
            .map_err(|_| Error::Key)?
            .as_bytes()
            .to_vec(),
    );
    let mut serial = *serial;
    serial[0] &= 0x7f;
    let n = name(cn);
    let ski: Vec<u8> = Sha256::digest(&spki)[..20].to_vec();
    let ski_ext = seq(&[&oid(&[2, 5, 29, 14]), &octets(&octets(&ski))]);
    let extensions = match profile {
        Profile::Ca { .. } => seq(&[
            &seq(&[
                &oid(&[2, 5, 29, 19]),
                &tlv(tag::BOOLEAN, &[0xff]),
                &octets(&seq(&[&tlv(tag::BOOLEAN, &[0xff])])),
            ]),
            &seq(&[
                &oid(&[2, 5, 29, 15]),
                &tlv(tag::BOOLEAN, &[0xff]),
                &octets(&tlv(tag::BIT_STRING, &[0x01, 0x86])),
            ]),
            &ski_ext,
        ]),
    };
    let until = match profile {
        Profile::Ca { years } => time(now + u64::from(years) * 31_557_600),
    };
    let tbs = seq(&[
        &explicit(0, &der::int(2)),
        &uint(&serial),
        &alg(SHA256_RSA),
        &n,
        &seq(&[&time(now), &until]),
        &n,
        &spki,
        &explicit(3, &extensions),
    ]);
    let sig = sign(blind, &der, &tbs)?;
    let cert = seq(&[&tbs, &alg(SHA256_RSA), &bits(&sig)]);
    // The self-signature is checked against the certificate's own key: a
    // fault in the private-key operation never leaves in a certificate.
    if !verify(&cert, &tbs, &sig) {
        return Err(Error::Key);
    }
    Ok(Pair { key: der, cert })
}

/// Where the parts of a certificate are.
pub struct Cert {
    /// The serial number, as its whole INTEGER TLV.
    pub serial: core::ops::Range<usize>,
    /// The issuer name, as its whole TLV.
    pub issuer: core::ops::Range<usize>,
    /// The subject's public key info, as its whole TLV.
    pub spki: core::ops::Range<usize>,
    /// The subject's common name, when it has one.
    pub cn: Option<String>,
    /// The start of its validity, as a Unix time.
    pub not_before: Option<u64>,
}

impl Cert {
    /// Reads a certificate's parts.
    pub fn read(cert: &[u8]) -> Option<Cert> {
        let (_, outer, _) = der::read(cert, 0)?;
        let (_, tbs, _) = der::read(cert, outer.start)?;
        let mut at = tbs.start;
        let mut whole = Vec::new();
        while at < tbs.end {
            let (t, value, next) = der::read(cert, at)?;
            whole.push((t, at..next, value));
            at = next;
        }
        // Skip the explicit version.
        let base = usize::from(whole.first()?.0 == 0xa0);
        let serial = whole.get(base)?.1.clone();
        let issuer = whole.get(base + 2)?.1.clone();
        let subject = whole.get(base + 4)?.2.clone();
        let spki = whole.get(base + 5)?.1.clone();
        let not_before = whole.get(base + 3).and_then(|(_, _, v)| {
            let (t, value, _) = der::read(cert, v.start)?;
            unix_of(t, &cert[value])
        });
        Some(Cert {
            serial,
            issuer,
            spki,
            cn: common_name(&cert[subject]),
            not_before,
        })
    }
}

/// A UTCTime or GeneralizedTime as a Unix time.
fn unix_of(t: u8, v: &[u8]) -> Option<u64> {
    let (year, rest) = match t {
        tag::UTC_TIME => {
            let yy = digits(v.get(0..2)?)?;
            (if yy < 50 { 2000 + yy } else { 1900 + yy }, v.get(2..)?)
        }
        tag::GENERALIZED_TIME => (digits(v.get(0..4)?)?, v.get(4..)?),
        _ => return None,
    };
    let m = digits(rest.get(0..2)?)?;
    let d = digits(rest.get(2..4)?)?;
    let (h, mi, s) = (
        digits(rest.get(4..6)?)?,
        digits(rest.get(6..8)?)?,
        digits(rest.get(8..10)?)?,
    );
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || rest.get(10) != Some(&b'Z') {
        return None;
    }
    // Howard Hinnant's days-from-civil.
    let y = i64::from(year) - i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = i64::from(if m > 2 { m - 3 } else { m + 9 });
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    u64::try_from(days * 86_400 + i64::from(h * 3600 + mi * 60 + s)).ok()
}

fn digits(b: &[u8]) -> Option<u32> {
    b.iter().try_fold(0u32, |n, &c| {
        c.is_ascii_digit().then(|| n * 10 + u32::from(c - b'0'))
    })
}

/// The first common name in a name's value.
fn common_name(name: &[u8]) -> Option<String> {
    for (_, rdn) in der::children(name)? {
        for (_, atv) in der::children(&name[rdn.clone()])? {
            let atv = &name[rdn.clone()][atv];
            let parts = der::children(atv)?;
            let (_, o) = parts.first()?;
            if atv[o.clone()] == [0x55, 0x04, 0x03] {
                let (_, v) = parts.get(1)?;
                return Some(String::from_utf8_lossy(&atv[v.clone()]).into_owned());
            }
        }
    }
    None
}

/// A CMS SignerInfo for `cert`'s key: version 1, issuer and serial,
/// SHA-256 and RSA, the signed attributes when there are any, and the
/// signature.
pub fn signer_info(
    cert: &[u8],
    signed_attrs: Option<&[u8]>,
    signature: &[u8],
) -> Result<Vec<u8>, Error> {
    let c = Cert::read(cert).ok_or(Error::Key)?;
    let ias = seq(&[&cert[c.issuer], &cert[c.serial]]);
    let mut parts: Vec<Vec<u8>> = alloc::vec![der::int(1), ias, alg(SHA256)];
    if let Some(a) = signed_attrs {
        parts.push(a.to_vec());
    }
    parts.push(alg(RSA));
    parts.push(octets(signature));
    let refs: Vec<&[u8]> = parts.iter().map(Vec::as_slice).collect();
    Ok(seq(&refs))
}
