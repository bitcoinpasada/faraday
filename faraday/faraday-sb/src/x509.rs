//! An RSA-2048 key and its self-signed X.509 certificate, the shape a
//! Secure Boot PK, KEK or db key takes, and CMS signatures made with one.

use alloc::string::String;
use alloc::vec::Vec;

use der::asn1::{Any, ObjectIdentifier, OctetString, SetOfVec};
use der::{DateTime, Decode, Encode, Tag};
use rsa::RsaPrivateKey;
use rsa::pkcs1v15::{SigningKey, VerifyingKey};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey};
use rsa::signature::{RandomizedSigner, SignatureEncoding, Verifier};
use sha2::{Digest, Sha256};
use x509_cert::Certificate;
use x509_cert::attr::{Attribute, AttributeTypeAndValue};
use x509_cert::builder::profile::BuilderProfile;
use x509_cert::builder::{Builder, CertificateBuilder};
use x509_cert::certificate::TbsCertificate;
use x509_cert::ext::Extension;
use x509_cert::ext::pkix::{BasicConstraints, KeyUsage, KeyUsages, SubjectKeyIdentifier};
use x509_cert::name::{Name, RdnSequence, RelativeDistinguishedName};
use x509_cert::serial_number::SerialNumber;
use x509_cert::spki::{SubjectPublicKeyInfoOwned, SubjectPublicKeyInfoRef};
use x509_cert::time::{Time, Validity};
use zeroize::Zeroizing;

use crate::Error;
use crate::cms::{IssuerAndSerial, SignerInfo};

/// commonName.
const COMMON_NAME: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.3");

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
    let (y, m, d) = faraday_crypto::date::civil_from_days(days);
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
fn time(unix: u64) -> Result<Time, Error> {
    DateTime::from_unix_duration(core::time::Duration::from_secs(unix))
        .map(Time::from)
        .map_err(|_| Error::Key)
}

/// A name of one common name.
fn name(cn: &str) -> Result<Name, Error> {
    let atv = AttributeTypeAndValue {
        oid: COMMON_NAME,
        value: Any::new(Tag::Utf8String, cn.as_bytes().to_vec()).map_err(|_| Error::Key)?,
    };
    let rdn = RelativeDistinguishedName::try_from(alloc::vec![atv]).map_err(|_| Error::Key)?;
    // `Name` is read from the DER of its sequence: it has no constructor
    // from one without the crate's `hazmat` feature.
    let sequence = RdnSequence::from(alloc::vec![rdn])
        .to_der()
        .map_err(|_| Error::Key)?;
    Name::from_der(&sequence).map_err(|_| Error::Key)
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

/// A certificate read as DER: whole, with nothing after it. `None` for
/// bytes that are not one (BER, a length that runs past the end, a
/// missing field, trailing bytes).
pub fn read(cert: &[u8]) -> Option<Certificate> {
    Certificate::from_der(cert).ok()
}

/// Whether `sig` is the certificate's key's signature over `data`.
pub fn verify(cert: &[u8], data: &[u8], sig: &[u8]) -> bool {
    let Some(spki) =
        read(cert).and_then(|c| c.tbs_certificate().subject_public_key_info().to_der().ok())
    else {
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

/// The extensions of [`Profile::Ca`], in the order Faraday has always
/// written them.
struct CaProfile {
    subject: Name,
}

impl BuilderProfile for CaProfile {
    fn get_issuer(&self, subject: &Name) -> Name {
        subject.clone()
    }

    fn get_subject(&self) -> Name {
        self.subject.clone()
    }

    fn build_extensions(
        &self,
        spk: SubjectPublicKeyInfoRef<'_>,
        _issuer_spk: SubjectPublicKeyInfoRef<'_>,
        _tbs: &TbsCertificate,
    ) -> x509_cert::builder::Result<Vec<Extension>> {
        let extension = |id: ObjectIdentifier, value: Vec<u8>| -> der::Result<Extension> {
            Ok(Extension {
                extn_id: id,
                critical: true,
                extn_value: OctetString::new(value)?,
            })
        };
        let basic = BasicConstraints {
            ca: true,
            path_len_constraint: None,
        };
        let usage =
            KeyUsage(KeyUsages::DigitalSignature | KeyUsages::KeyCertSign | KeyUsages::CRLSign);
        let ski = SubjectKeyIdentifier(OctetString::new(
            Sha256::digest(spk.to_der()?)[..20].to_vec(),
        )?);
        Ok(alloc::vec![
            extension(ObjectIdentifier::new_unwrap("2.5.29.19"), basic.to_der()?)?,
            extension(ObjectIdentifier::new_unwrap("2.5.29.15"), usage.to_der()?)?,
            Extension {
                critical: false,
                ..extension(ObjectIdentifier::new_unwrap("2.5.29.14"), ski.to_der()?)?
            },
        ])
    }
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
    let spki = SubjectPublicKeyInfoOwned::from_der(
        key.to_public_key()
            .to_public_key_der()
            .map_err(|_| Error::Key)?
            .as_bytes(),
    )
    .map_err(|_| Error::Key)?;
    let der = Zeroizing::new(
        key.to_pkcs8_der()
            .map_err(|_| Error::Key)?
            .as_bytes()
            .to_vec(),
    );
    let mut serial = *serial;
    serial[0] &= 0x7f;
    let until = match profile {
        Profile::Ca { years } => time(now + u64::from(years) * 31_557_600)?,
    };
    let validity = Validity::new(time(now)?, until);
    let builder = CertificateBuilder::new(
        CaProfile { subject: name(cn)? },
        SerialNumber::new(&serial).map_err(|_| Error::Key)?,
        validity,
        spki,
    )
    .map_err(|_| Error::Key)?;
    let signer = SigningKey::<Sha256>::new(key);
    let certificate = builder
        .build_with_rng::<_, rsa::pkcs1v15::Signature, _>(&signer, blind)
        .map_err(|_| Error::Key)?;
    let cert = certificate.to_der().map_err(|_| Error::Key)?;
    let tbs = certificate
        .tbs_certificate()
        .to_der()
        .map_err(|_| Error::Key)?;
    // The self-signature is checked against the certificate's own key: a
    // fault in the private-key operation never leaves in a certificate.
    if !verify(&cert, &tbs, certificate.signature().raw_bytes()) {
        return Err(Error::Key);
    }
    Ok(Pair { key: der, cert })
}

/// A certificate's first common name.
pub fn common_name(cert: &Certificate) -> Option<String> {
    let found = cert
        .tbs_certificate()
        .subject()
        .iter()
        .find(|atv| atv.oid == COMMON_NAME)?;
    Some(String::from_utf8_lossy(found.value.value()).into_owned())
}

/// A CMS SignerInfo for `cert`'s key: version 1, issuer and serial,
/// SHA-256 and RSA, the signed attributes when there are any, and the
/// signature.
pub fn signer_info(
    cert: &[u8],
    signed_attrs: Option<SetOfVec<Attribute>>,
    signature: &[u8],
) -> Result<SignerInfo, Error> {
    let c = read(cert).ok_or(Error::Key)?;
    Ok(SignerInfo {
        sid: IssuerAndSerial {
            issuer: c.tbs_certificate().issuer().clone(),
            serial: c.tbs_certificate().serial_number().clone(),
        },
        signed_attrs,
        signature: OctetString::new(signature.to_vec()).map_err(|_| Error::Key)?,
    })
}
