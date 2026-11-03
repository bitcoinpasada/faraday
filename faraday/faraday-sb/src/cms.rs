//! CMS SignedData (RFC 5652) and Authenticode's structures (Microsoft's
//! Authenticode specification), written and read as `der` types. Only
//! the parts Secure Boot uses: a SignedData with one SignerInfo, and the
//! `.auth` file's detached form of it.

use alloc::vec::Vec;

use der::asn1::{Any, BitString, ContextSpecific, ObjectIdentifier, OctetString, SetOfVec};
use der::{
    Decode, DecodeValue, Encode, EncodeValue, Header, Length, Reader, Sequence, Tag, TagMode,
    TagNumber, Writer,
};
use x509_cert::attr::Attribute;
use x509_cert::name::Name;
use x509_cert::serial_number::SerialNumber;
use x509_cert::spki::AlgorithmIdentifierOwned;

/// id-data.
pub const DATA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.1");
/// id-signedData.
pub const SIGNED_DATA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.2");
/// SHA-256.
pub const SHA256: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.1");
/// rsaEncryption.
pub const RSA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.1");
/// id-contentType.
pub const CONTENT_TYPE: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.3");
/// id-messageDigest.
pub const MESSAGE_DIGEST: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.4");
/// SPC_INDIRECT_DATA_OBJID.
pub const SPC_INDIRECT_DATA: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.3.6.1.4.1.311.2.1.4");
/// SPC_PE_IMAGE_DATAOBJ.
pub const SPC_PE_IMAGE_DATA: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.3.6.1.4.1.311.2.1.15");
/// SPC_SP_OPUS_INFO_OBJID.
pub const SPC_SP_OPUS_INFO: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.3.6.1.4.1.311.2.1.12");

/// An algorithm identifier with NULL parameters.
pub fn alg(oid: ObjectIdentifier) -> AlgorithmIdentifierOwned {
    AlgorithmIdentifierOwned {
        oid,
        parameters: Some(Any::null()),
    }
}

/// A SEQUENCE struct: its fields in order, each an encodable type.
macro_rules! sequence {
    ($(#[$m:meta])* $name:ident { $($(#[$fm:meta])* $f:ident: $t:ty),* $(,)? }) => {
        $(#[$m])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct $name {
            $($(#[$fm])* pub $f: $t,)*
        }

        impl EncodeValue for $name {
            fn value_len(&self) -> der::Result<Length> {
                let mut n = Length::ZERO;
                $(n = (n + self.$f.encoded_len()?)?;)*
                Ok(n)
            }

            fn encode_value(&self, writer: &mut impl Writer) -> der::Result<()> {
                $(self.$f.encode(writer)?;)*
                Ok(())
            }
        }

        impl<'a> Sequence<'a> for $name {}
    };
}

/// A plain SEQUENCE struct, read field by field as written.
macro_rules! plain_sequence {
    ($(#[$m:meta])* $name:ident { $($(#[$fm:meta])* $f:ident: $t:ty),* $(,)? }) => {
        sequence!($(#[$m])* $name { $($(#[$fm])* $f: $t),* });

        impl<'a> DecodeValue<'a> for $name {
            type Error = der::Error;

            fn decode_value<R: Reader<'a>>(reader: &mut R, _header: Header) -> der::Result<Self> {
                Ok(Self { $($f: reader.decode()?),* })
            }
        }
    };
}

plain_sequence! {
    /// A signer named by its certificate's issuer and serial number.
    IssuerAndSerial {
        /// The issuer name.
        issuer: Name,
        /// The serial number.
        serial: SerialNumber,
    }
}

plain_sequence! {
    /// A digest algorithm and a digest.
    DigestInfo {
        /// The algorithm.
        algorithm: AlgorithmIdentifierOwned,
        /// The digest.
        digest: OctetString,
    }
}

plain_sequence! {
    /// Authenticode's SpcPeImageData: no flags, and the file link text.
    SpcPeImageData {
        /// The flags, a BIT STRING with no bits.
        flags: BitString,
        /// `[0]` around the link: `[2]` around the file name.
        file: Any,
    }
}

plain_sequence! {
    /// Authenticode's SpcIndirectDataContent: what is signed, and the
    /// image hash it names.
    SpcIndirectData {
        /// SpcAttributeTypeAndOptionalValue, as it is carried.
        data: Any,
        /// The image hash.
        message_digest: DigestInfo,
    }
}

plain_sequence! {
    /// Authenticode's SpcAttributeTypeAndOptionalValue for an image.
    SpcImageAttribute {
        /// SPC_PE_IMAGE_DATAOBJ.
        attribute: ObjectIdentifier,
        /// The image data.
        value: SpcPeImageData,
    }
}

/// An SpcIndirectDataContent naming `digest`, the SHA-256 of an image.
pub fn spc_content(digest: &[u8; 32]) -> der::Result<SpcIndirectData> {
    // not wording: Authenticode's fixed SpcLink text
    let obsolete: Vec<u8> = "<<<Obsolete>>>"
        .encode_utf16()
        .flat_map(u16::to_be_bytes)
        .collect();
    let context = |number: u32, constructed: bool, bytes: Vec<u8>| {
        Any::new(TagNumber(number).context_specific(constructed), bytes)
    };
    let url = context(0, false, obsolete)?;
    let file = context(2, true, url.to_der()?)?;
    let file = context(0, true, file.to_der()?)?;
    let image = SpcImageAttribute {
        attribute: SPC_PE_IMAGE_DATA,
        value: SpcPeImageData {
            flags: BitString::new(0, Vec::new())?,
            file,
        },
    };
    Ok(SpcIndirectData {
        data: Any::encode_from(&image)?,
        message_digest: DigestInfo {
            algorithm: alg(SHA256),
            digest: OctetString::new(digest.to_vec())?,
        },
    })
}

/// The signed attributes of an image's SignerInfo: content type, SpcSpOpusInfo
/// and the digest of the content.
pub fn image_attributes(content_digest: &[u8; 32]) -> der::Result<SetOfVec<Attribute>> {
    let one = |oid: ObjectIdentifier, value: Any| -> der::Result<Attribute> {
        Ok(Attribute {
            oid,
            values: SetOfVec::try_from(alloc::vec![value])?,
        })
    };
    SetOfVec::try_from(alloc::vec![
        one(
            CONTENT_TYPE,
            Any::new(Tag::ObjectIdentifier, SPC_INDIRECT_DATA.as_bytes())?
        )?,
        one(SPC_SP_OPUS_INFO, Any::new(Tag::Sequence, Vec::new())?)?,
        one(
            MESSAGE_DIGEST,
            Any::encode_from(&OctetString::new(content_digest.to_vec())?)?
        )?,
    ])
}

/// A SignerInfo: version 1, issuer and serial, SHA-256 and RSA, the
/// signed attributes when there are any, and the signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignerInfo {
    /// The signer's certificate, by issuer and serial.
    pub sid: IssuerAndSerial,
    /// `[0]` the signed attributes.
    pub signed_attrs: Option<SetOfVec<Attribute>>,
    /// The signature.
    pub signature: OctetString,
}

impl SignerInfo {
    /// Whether the signature covers attributes.
    fn attrs(&self) -> Option<ContextSpecific<SetOfVec<Attribute>>> {
        self.signed_attrs.clone().map(|value| ContextSpecific {
            tag_number: TagNumber(0),
            tag_mode: TagMode::Implicit,
            value,
        })
    }
}

impl EncodeValue for SignerInfo {
    fn value_len(&self) -> der::Result<Length> {
        let mut n = 1u8.encoded_len()?;
        n = (n + self.sid.encoded_len()?)?;
        n = (n + alg(SHA256).encoded_len()?)?;
        n = (n + self.attrs().encoded_len()?)?;
        n = (n + alg(RSA).encoded_len()?)?;
        n + self.signature.encoded_len()?
    }

    fn encode_value(&self, writer: &mut impl Writer) -> der::Result<()> {
        1u8.encode(writer)?;
        self.sid.encode(writer)?;
        alg(SHA256).encode(writer)?;
        self.attrs().encode(writer)?;
        alg(RSA).encode(writer)?;
        self.signature.encode(writer)
    }
}

impl<'a> Sequence<'a> for SignerInfo {}

impl<'a> DecodeValue<'a> for SignerInfo {
    type Error = der::Error;

    fn decode_value<R: Reader<'a>>(reader: &mut R, _header: Header) -> der::Result<Self> {
        let _version: u8 = reader.decode()?;
        let sid = reader.decode()?;
        let _digest: AlgorithmIdentifierOwned = reader.decode()?;
        let signed_attrs =
            ContextSpecific::<SetOfVec<Attribute>>::decode_implicit(reader, TagNumber(0))?
                .map(|a| a.value);
        let _signature_alg: AlgorithmIdentifierOwned = reader.decode()?;
        let signature = reader.decode()?;
        // Unsigned attributes, if any, are not read.
        while !reader.is_finished() {
            let _: Any = reader.decode()?;
        }
        Ok(Self {
            sid,
            signed_attrs,
            signature,
        })
    }
}

/// A SignedData with one signer: SHA-256, the content `content` when it
/// is not detached, the signer's certificate and its SignerInfo.
#[derive(Clone, Debug)]
pub struct SignedData {
    /// The content, `[0]` around it; none when detached.
    pub content: Option<Any>,
    /// The content type.
    pub content_type: ObjectIdentifier,
    /// The certificates the signature carries, each whole.
    pub certs: Vec<Any>,
    /// The signers.
    pub signer_infos: Vec<Any>,
}

impl SignedData {
    /// A SignedData for one signer.
    pub fn new(
        content_type: ObjectIdentifier,
        content: Option<Any>,
        cert: &[u8],
        info: &SignerInfo,
    ) -> der::Result<Self> {
        Ok(Self {
            content,
            content_type,
            certs: alloc::vec![Any::from_der(cert)?],
            signer_infos: alloc::vec![Any::encode_from(info)?],
        })
    }

    fn encapsulated(&self) -> ContentPair {
        ContentPair {
            content_type: self.content_type,
            content: self.content.clone().map(|value| ContextSpecific {
                tag_number: TagNumber(0),
                tag_mode: TagMode::Explicit,
                value,
            }),
        }
    }

    fn certificates(&self) -> ContextSpecific<Vec<Any>> {
        ContextSpecific {
            tag_number: TagNumber(0),
            tag_mode: TagMode::Implicit,
            value: self.certs.clone(),
        }
    }

    fn infos(&self) -> der::Result<SetOfVec<Any>> {
        SetOfVec::try_from(self.signer_infos.clone())
    }
}

sequence! {
    /// EncapsulatedContentInfo.
    ContentPair {
        /// The content type.
        content_type: ObjectIdentifier,
        /// `[0]` the content, when it is carried.
        content: Option<ContextSpecific<Any>>,
    }
}

impl<'a> DecodeValue<'a> for ContentPair {
    type Error = der::Error;

    fn decode_value<R: Reader<'a>>(reader: &mut R, _header: Header) -> der::Result<Self> {
        let content_type = reader.decode()?;
        let content = ContextSpecific::<Any>::decode_explicit(reader, TagNumber(0))?;
        Ok(Self {
            content_type,
            content,
        })
    }
}

impl EncodeValue for SignedData {
    fn value_len(&self) -> der::Result<Length> {
        let digests = SetOfVec::try_from(alloc::vec![alg(SHA256)])?;
        let mut n = 1u8.encoded_len()?;
        n = (n + digests.encoded_len()?)?;
        n = (n + self.encapsulated().encoded_len()?)?;
        n = (n + self.certificates().encoded_len()?)?;
        n + self.infos()?.encoded_len()?
    }

    fn encode_value(&self, writer: &mut impl Writer) -> der::Result<()> {
        1u8.encode(writer)?;
        SetOfVec::try_from(alloc::vec![alg(SHA256)])?.encode(writer)?;
        self.encapsulated().encode(writer)?;
        self.certificates().encode(writer)?;
        self.infos()?.encode(writer)
    }
}

impl<'a> Sequence<'a> for SignedData {}

impl<'a> DecodeValue<'a> for SignedData {
    type Error = der::Error;

    fn decode_value<R: Reader<'a>>(reader: &mut R, _header: Header) -> der::Result<Self> {
        let _version: u8 = reader.decode()?;
        let _digests: SetOfVec<AlgorithmIdentifierOwned> = reader.decode()?;
        let encap: ContentPair = reader.decode()?;
        let certs = ContextSpecific::<Vec<Any>>::decode_implicit(reader, TagNumber(0))?
            .map(|c| c.value)
            .unwrap_or_default();
        // Revocation lists, if any, are not read.
        let _ = ContextSpecific::<Vec<Any>>::decode_implicit(reader, TagNumber(1))?;
        // The SET OF SignerInfo as it is in the file: `SetOfVec` would sort
        // it, and the first signer is the first one written.
        let set: Any = reader.decode()?;
        if set.header().tag() != Tag::Set {
            return Err(der::ErrorKind::TagUnexpected {
                expected: Some(Tag::Set),
                actual: set.header().tag(),
            }
            .into());
        }
        let mut inner = der::SliceReader::new(set.value())?;
        let mut infos = Vec::new();
        while !inner.is_finished() {
            infos.push(Any::decode(&mut inner)?);
        }
        Ok(Self {
            content: encap.content.map(|c| c.value),
            content_type: encap.content_type,
            certs,
            signer_infos: infos,
        })
    }
}

/// The signed attributes of a SignerInfo as its signature covers them
/// (RFC 5652 5.4): the `[0]` element exactly as it is written, with the
/// tag changed to SET (0x31). `None` when there are none or the
/// SignerInfo is not one. The attributes are not sorted or re-encoded:
/// `SetOfVec` would put them in DER order.
pub fn signed_attrs_bytes(info: &[u8]) -> Option<Vec<u8>> {
    let info = Any::from_der(info).ok()?;
    let mut reader = der::SliceReader::new(info.value()).ok()?;
    // Version, issuer and serial, digest algorithm.
    for _ in 0..3 {
        Any::decode(&mut reader).ok()?;
    }
    let attrs = Any::decode(&mut reader).ok()?;
    let mut bytes = attrs.to_der().ok()?;
    if bytes.first() != Some(&0xa0) {
        return None;
    }
    bytes[0] = 0x31;
    Some(bytes)
}

/// A ContentInfo around a SignedData, as `.p7`, Authenticode and
/// `openssl cms` carry it.
#[derive(Clone, Debug)]
pub struct ContentInfo(pub SignedData);

impl EncodeValue for ContentInfo {
    fn value_len(&self) -> der::Result<Length> {
        SIGNED_DATA.encoded_len()? + self.explicit().encoded_len()?
    }

    fn encode_value(&self, writer: &mut impl Writer) -> der::Result<()> {
        SIGNED_DATA.encode(writer)?;
        self.explicit().encode(writer)
    }
}

impl ContentInfo {
    fn explicit(&self) -> ContextSpecific<SignedData> {
        ContextSpecific {
            tag_number: TagNumber(0),
            tag_mode: TagMode::Explicit,
            value: self.0.clone(),
        }
    }
}

impl<'a> Sequence<'a> for ContentInfo {}

impl<'a> DecodeValue<'a> for ContentInfo {
    type Error = der::Error;

    fn decode_value<R: Reader<'a>>(reader: &mut R, _header: Header) -> der::Result<Self> {
        let oid: ObjectIdentifier = reader.decode()?;
        if oid != SIGNED_DATA {
            return Err(der::ErrorKind::OidUnknown { oid }.into());
        }
        let inner = ContextSpecific::<SignedData>::decode_explicit(reader, TagNumber(0))?
            .ok_or_else(|| {
                der::Error::from(der::ErrorKind::TagUnexpected {
                    expected: None,
                    actual: Tag::Null,
                })
            })?;
        Ok(Self(inner.value))
    }
}
