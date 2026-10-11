//! BC-UR (Blockchain Commons uniform resources, BCR-2020-005): the
//! animated-QR transport every coordinator reads.
//!
//! `ur:crypto-psbt` (BCR-2020-006) wraps the binary PSBT in one CBOR byte
//! string; `ur:bytes` wraps arbitrary bytes the same way. A single-part UR
//! is `ur:<type>/<bytewords>`; a multi-part one is
//! `ur:<type>/<seq>-<count>/<bytewords>` where the parts are fountain-code
//! fragments, so a receiver that misses frames still finishes.
//!
//! The whole transport is here and nothing is pulled in for it
//! (`docs/deps/ur.md`, `docs/PLANNING.md` §16.61): [`bytewords`] is
//! BCR-2020-012's alphabet with its CRC-32, [`xoshiro`] the generator
//! that picks a part's fragments, [`fountain`] the parts themselves and
//! their CBOR, and this file the `ur:` text around them. SHA-256 comes
//! from `osk-crypto`.

#![forbid(unsafe_code)]

pub(crate) mod bytewords;
mod fountain;
mod xoshiro;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

/// The UR type of a PSBT.
pub const PSBT_TYPE: &str = "crypto-psbt";
/// The UR type of raw bytes.
pub const BYTES_TYPE: &str = "bytes";

/// Most fragments a multi-part UR may claim.
///
/// The count comes out of the part's own text, and a decoder that
/// believes it reserves a slot per fragment before a single one has been
/// checked, so a one-code message can ask for as much memory as it likes.
/// The number is what the transport is for: the app splits a payload into
/// fragments of 20 to 400 bytes (`opensigner-core`'s `UR_FRAGMENT_MIN`
/// and `UR_FRAGMENT_MAX`), and a real transaction can be long — a PSBT
/// with a hundred inputs runs to several hundred parts at those sizes,
/// and requiring the previous transaction of every SegWit v0 input makes
/// it longer still. 2048 parts is 40 KiB of the smallest fragments and
/// under 1 MiB of the largest: room for any transaction a person scans
/// frame by frame, and still a bound on what a message claiming millions
/// of parts can reserve.
pub const MAX_FRAGMENTS: usize = 2048;

/// What a UR string, a part or a message failed on.
///
/// Internal: every one of these reaches the caller as an [`Error`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fault {
    /// A letter pair that is not a byteword.
    InvalidWord,
    /// The bytewords CRC-32 does not match the payload.
    InvalidChecksum,
    /// An odd number of characters, which cannot be letter pairs.
    InvalidLength,
    /// The payload is not ASCII.
    NonAscii,
    /// The part is not the CBOR array BCR-2020-005 specifies.
    Cbor,
    /// The part carries no data, no fragments or no message.
    EmptyPart,
    /// Part sequence numbers start at 1.
    InvalidSequence,
    /// The part belongs to a different message than the ones before.
    InconsistentPart,
    /// The joined fragments are longer than the message and the excess
    /// is not zero padding.
    InvalidPadding,
    /// The joined message does not match the checksum the parts carry.
    MessageChecksum,
    /// A message must not be empty.
    EmptyMessage,
    /// A fragment length must be positive.
    InvalidFragmentLen,
    /// Not `ur:`.
    InvalidScheme,
    /// `ur:` with no type.
    TypeUnspecified,
    /// A type that is not ASCII letters, digits and `-`.
    InvalidCharacters,
    /// `<seq>-<count>` missing, unparseable or zero.
    InvalidIndices,
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Fault::InvalidWord => "invalid word",
            Fault::InvalidChecksum => "invalid checksum",
            Fault::InvalidLength => "invalid length",
            Fault::NonAscii => "payload contains non-ASCII characters",
            Fault::Cbor => "invalid part encoding",
            Fault::EmptyPart => "expected non-empty part",
            Fault::InvalidSequence => "invalid sequence",
            Fault::InconsistentPart => "part is inconsistent with previous ones",
            Fault::InvalidPadding => "invalid padding",
            Fault::MessageChecksum => "invalid message checksum",
            Fault::EmptyMessage => "expected non-empty message",
            Fault::InvalidFragmentLen => "expected positive maximum fragment length",
            Fault::InvalidScheme => "invalid scheme",
            Fault::TypeUnspecified => "no type specified",
            Fault::InvalidCharacters => "type contains invalid characters",
            Fault::InvalidIndices => "invalid indices",
        })
    }
}

/// Why a UR could not be handled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Not `ur:` followed by a type and a payload.
    NotUr,
    /// Bad bytewords, checksum or indices.
    Malformed(String),
    /// A part of a different type or message than the ones before.
    Mismatch,
    /// The payload is not a CBOR byte string.
    NotByteString,
    /// The message is empty.
    Empty,
    /// The part claims more fragments than [`MAX_FRAGMENTS`].
    TooManyParts,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotUr => f.write_str("not a UR"),
            Error::Malformed(e) => write!(f, "malformed UR: {e}"),
            Error::Mismatch => f.write_str("UR part belongs to a different message"),
            Error::NotByteString => f.write_str("UR payload is not a byte string"),
            Error::Empty => f.write_str("UR message is empty"),
            Error::TooManyParts => f.write_str("UR claims too many parts"),
        }
    }
}

impl core::error::Error for Error {}

impl From<Fault> for Error {
    fn from(fault: Fault) -> Self {
        Error::Malformed(fault.to_string())
    }
}

impl Error {
    /// How a running scan reads a fault. A checksum that does not match
    /// and a part of another message are the same thing to a scanner:
    /// it is looking at a different code than the one it started on, and
    /// the caller restarts rather than reporting a broken UR.
    fn scanning(fault: Fault) -> Self {
        match fault {
            Fault::InvalidChecksum
            | Fault::InconsistentPart
            | Fault::MessageChecksum
            | Fault::InvalidIndices => Error::Mismatch,
            other => Error::Malformed(other.to_string()),
        }
    }
}

/// A decoded UR message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// `ur:crypto-psbt`: the binary PSBT.
    Psbt(Vec<u8>),
    /// `ur:bytes`: the bytes.
    Bytes(Vec<u8>),
    /// Any other type, with its CBOR payload untouched.
    Other {
        /// The type, lower-case.
        ur_type: String,
        /// The CBOR.
        cbor: Vec<u8>,
    },
}

impl Message {
    fn new(ur_type: &str, cbor: Vec<u8>) -> Result<Self, Error> {
        Ok(match ur_type {
            PSBT_TYPE => Message::Psbt(cbor_byte_string(&cbor)?),
            BYTES_TYPE => Message::Bytes(cbor_byte_string(&cbor)?),
            other => Message::Other {
                ur_type: String::from(other),
                cbor,
            },
        })
    }
}

/// Whether `text` starts with `ur:` (any case).
pub fn is_ur(text: &str) -> bool {
    text.get(..3).is_some_and(|p| p.eq_ignore_ascii_case("ur:"))
}

/// CBOR byte string (major type 2) around `bytes`.
pub fn cbor_bytes(bytes: &[u8]) -> Vec<u8> {
    let n = bytes.len();
    let mut out = Vec::with_capacity(n + 5);
    if n < 24 {
        out.push(0x40 | n as u8);
    } else if n <= 0xff {
        out.extend_from_slice(&[0x58, n as u8]);
    } else if n <= 0xffff {
        out.push(0x59);
        out.extend_from_slice(&(n as u16).to_be_bytes());
    } else {
        out.push(0x5a);
        out.extend_from_slice(&(n as u32).to_be_bytes());
    }
    out.extend_from_slice(bytes);
    out
}

/// The content of a CBOR byte string; trailing bytes are an error.
pub fn cbor_byte_string(cbor: &[u8]) -> Result<Vec<u8>, Error> {
    let (&head, rest) = cbor.split_first().ok_or(Error::NotByteString)?;
    if head >> 5 != 2 {
        return Err(Error::NotByteString);
    }
    let (len, rest) = match head & 0x1f {
        n @ 0..=23 => (usize::from(n), rest),
        24 => (
            usize::from(*rest.first().ok_or(Error::NotByteString)?),
            &rest[1..],
        ),
        25 => {
            let b = rest.get(..2).ok_or(Error::NotByteString)?;
            (usize::from(u16::from_be_bytes([b[0], b[1]])), &rest[2..])
        }
        26 => {
            let b = rest.get(..4).ok_or(Error::NotByteString)?;
            (
                u32::from_be_bytes([b[0], b[1], b[2], b[3]]) as usize,
                &rest[4..],
            )
        }
        _ => return Err(Error::NotByteString),
    };
    if rest.len() != len {
        return Err(Error::NotByteString);
    }
    Ok(rest.to_vec())
}

/// Whether a UR carries a whole message or one part of one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// The whole payload.
    SinglePart,
    /// One fountain part of it.
    MultiPart,
}

/// A UR type is ASCII letters, digits and `-`, and is not empty.
fn validate_type(ur_type: &str) -> Result<(), Fault> {
    if ur_type.is_empty()
        || !ur_type
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(Fault::InvalidCharacters);
    }
    Ok(())
}

/// `ur:<type>/<bytewords>`.
fn encode_single(data: &[u8], ur_type: &str) -> String {
    alloc::format!("ur:{ur_type}/{}", bytewords::encode(data))
}

/// The kind, type, payload and `<seq>-<count>` of one UR string.
type Decoded = (Kind, String, Vec<u8>, Option<(usize, usize)>);

fn decode_ur(text: &str) -> Result<Decoded, Fault> {
    let normalized = text.to_ascii_lowercase();
    let body = normalized.strip_prefix("ur:").ok_or(Fault::InvalidScheme)?;
    let (ur_type, rest) = body.split_once('/').ok_or(Fault::TypeUnspecified)?;
    validate_type(ur_type)?;
    match rest.rsplit_once('/') {
        None => Ok((
            Kind::SinglePart,
            String::from(ur_type),
            bytewords::decode(rest)?,
            None,
        )),
        Some((indices, payload)) => Ok((
            Kind::MultiPart,
            String::from(ur_type),
            bytewords::decode(payload)?,
            Some(decode_indices(indices)?),
        )),
    }
}

fn decode_indices(indices: &str) -> Result<(usize, usize), Fault> {
    let (sequence, count) = indices.split_once('-').ok_or(Fault::InvalidIndices)?;
    let sequence: usize = sequence.parse().map_err(|_| Fault::InvalidIndices)?;
    let count: usize = count.parse().map_err(|_| Fault::InvalidIndices)?;
    if sequence == 0 || count == 0 {
        return Err(Fault::InvalidIndices);
    }
    Ok((sequence, count))
}

/// A single-part `ur:crypto-psbt`.
pub fn encode_psbt(psbt: &[u8]) -> String {
    encode_single(&cbor_bytes(psbt), PSBT_TYPE)
}

/// A single-part `ur:bytes`.
pub fn encode_bytes(bytes: &[u8]) -> String {
    encode_single(&cbor_bytes(bytes), BYTES_TYPE)
}

/// Decodes a single-part UR.
pub fn decode_single(text: &str) -> Result<Message, Error> {
    if !is_ur(text) {
        return Err(Error::NotUr);
    }
    let (kind, ur_type, cbor, _) = decode_ur(text)?;
    if kind != Kind::SinglePart {
        return Err(Error::NotUr);
    }
    Message::new(&ur_type, cbor)
}

fn ur_type_of(text: &str) -> Result<String, Error> {
    if !is_ur(text) {
        return Err(Error::NotUr);
    }
    let ur_type = text[3..].split('/').next().unwrap_or("");
    if ur_type.is_empty() {
        return Err(Error::NotUr);
    }
    Ok(ur_type.to_ascii_lowercase())
}

/// The fragment count a multi-part part claims, read from the `<seq>-<count>`
/// segment of `ur:<type>/<seq>-<count>/<payload>`. `None` for anything
/// else, including a single-part UR, which carries no such segment.
fn claimed_count(text: &str) -> Option<usize> {
    let mut segments = text.get(3..)?.split('/');
    segments.next()?;
    let seq = segments.next()?;
    segments.next()?;
    let (_, count) = seq.split_once('-')?;
    count.parse().ok()
}

/// Emits the parts of an animated multi-part UR, endlessly: the first
/// `fragment_count` parts are the message in order, later ones are
/// fountain mixes, so a receiver that joins late still finishes.
pub struct Encoder {
    inner: fountain::Encoder,
    ur_type: &'static str,
}

impl Encoder {
    /// A `crypto-psbt` encoder with fragments of at most `fragment_len`
    /// bytes.
    pub fn psbt(psbt: &[u8], fragment_len: usize) -> Result<Self, Error> {
        Self::new(PSBT_TYPE, psbt, fragment_len)
    }

    /// A `bytes` encoder.
    pub fn bytes(bytes: &[u8], fragment_len: usize) -> Result<Self, Error> {
        Self::new(BYTES_TYPE, bytes, fragment_len)
    }

    fn new(ur_type: &'static str, payload: &[u8], fragment_len: usize) -> Result<Self, Error> {
        if payload.is_empty() {
            return Err(Error::Empty);
        }
        let inner = fountain::Encoder::new(&cbor_bytes(payload), fragment_len.max(1))?;
        Ok(Encoder { inner, ur_type })
    }

    /// Number of fragments the message was split into.
    pub const fn fragment_count(&self) -> usize {
        self.inner.fragment_count()
    }

    /// Parts emitted so far.
    pub const fn emitted(&self) -> usize {
        self.inner.current_sequence()
    }

    /// The next part, `ur:<type>/<seq>-<count>/…`.
    pub fn next_part(&mut self) -> String {
        let part = self.inner.next_part();
        alloc::format!(
            "ur:{}/{}/{}",
            self.ur_type,
            part.sequence_id(),
            bytewords::encode(&part.cbor())
        )
    }
}

/// Accumulates the parts of a multi-part UR (and accepts a single-part
/// one) until the message is complete.
#[derive(Default)]
pub struct Decoder {
    inner: Option<fountain::Decoder>,
    ur_type: Option<String>,
    done: Option<Message>,
}

impl Decoder {
    /// An empty decoder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds one UR string. Single-part URs complete at once; multi-part
    /// ones accumulate. Returns whether the message is now complete. A
    /// part claiming more than [`MAX_FRAGMENTS`] fragments is refused
    /// before anything is decoded or reserved for it.
    pub fn receive(&mut self, text: &str) -> Result<bool, Error> {
        if self.done.is_some() {
            return Ok(true);
        }
        ur_type_of(text)?;
        // Before the part is decoded, because the count is what a
        // decoder sizes itself from.
        if claimed_count(text).is_some_and(|n| n > MAX_FRAGMENTS) {
            return Err(Error::TooManyParts);
        }
        let (kind, ur_type, payload, indices) = decode_ur(text).map_err(Error::scanning)?;
        if kind == Kind::SinglePart {
            if self.inner.is_some() {
                return Err(Error::Mismatch);
            }
            self.done = Some(Message::new(&ur_type, payload)?);
            return Ok(true);
        }
        if self.ur_type.as_ref().is_some_and(|seen| seen != &ur_type) {
            return Err(Error::Mismatch);
        }
        let part = fountain::Part::from_cbor(&payload).map_err(Error::scanning)?;
        let (sequence, count) = indices.ok_or(Error::Mismatch)?;
        if part.sequence != sequence || part.sequence_count != count {
            return Err(Error::Mismatch);
        }
        let inner = self.inner.get_or_insert_with(fountain::Decoder::default);
        inner.receive(part).map_err(Error::scanning)?;
        if self.ur_type.is_none() {
            self.ur_type = Some(ur_type.clone());
        }
        if inner.complete() {
            let cbor = inner
                .message()
                .map_err(Error::scanning)?
                .ok_or(Error::Empty)?;
            self.done = Some(Message::new(&ur_type, cbor)?);
        }
        Ok(self.done.is_some())
    }

    /// `(fragments resolved, fragments in the message)`; `None` before
    /// any part arrived.
    pub fn progress(&self) -> Option<(usize, usize)> {
        if self.done.is_some() {
            let n = self.inner.as_ref().map_or(1, |d| d.fragment_count().max(1));
            return Some((n, n));
        }
        let d = self.inner.as_ref()?;
        Some((d.resolved_fragment_count().unwrap_or(0), d.fragment_count()))
    }

    /// The UR type seen so far.
    pub fn ur_type(&self) -> Option<String> {
        match (&self.done, &self.ur_type) {
            (Some(Message::Psbt(_)), _) => Some(String::from(PSBT_TYPE)),
            (Some(Message::Bytes(_)), _) => Some(String::from(BYTES_TYPE)),
            (Some(Message::Other { ur_type, .. }), _) => Some(ur_type.clone()),
            (None, seen) => seen.clone(),
        }
    }

    /// Whether the message is complete.
    pub fn is_complete(&self) -> bool {
        self.done.is_some()
    }

    /// The message, once complete.
    pub fn message(&self) -> Option<&Message> {
        self.done.as_ref()
    }

    /// Takes the message out, once complete.
    pub fn into_message(self) -> Option<Message> {
        self.done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ur::xoshiro::test_utils::make_message;

    /// The single-part and multi-part URs BCR-2020-005 and the reference
    /// implementations produce for a seeded message. These are what
    /// Sparrow, Nunchuk and BlueWallet read and what they send.
    #[test]
    fn published_vectors() {
        // BCR-2020-005 §"CBOR": the seed example, untagged CBOR
        // a10150c7098580125e2ab0981253468b2dbc52 as ur:seed.
        let cbor = [
            0xa1, 0x01, 0x50, 0xc7, 0x09, 0x85, 0x80, 0x12, 0x5e, 0x2a, 0xb0, 0x98, 0x12, 0x53,
            0x46, 0x8b, 0x2d, 0xbc, 0x52,
        ];
        assert_eq!(
            encode_single(&cbor, "seed"),
            "ur:seed/oyadgdstaslplabghydrpfmkbggufgludprfgmamdpwmox"
        );
        let m = decode_single("ur:seed/oyadgdstaslplabghydrpfmkbggufgludprfgmamdpwmox").unwrap();
        assert_eq!(
            m,
            Message::Other {
                ur_type: String::from("seed"),
                cbor: cbor.to_vec()
            }
        );
        assert_eq!(
            decode_single("ur:bytes/iehsjyhspmwfwfia").unwrap_err(),
            Error::NotByteString,
            "raw bytes without a CBOR wrapper are not ur:bytes"
        );

        // A 50-byte message as one part.
        assert_eq!(
            encode_single(&cbor_bytes(&make_message("Wolf", 50)), BYTES_TYPE),
            "ur:bytes/hdeymejtswhhylkepmykhhtsytsnoyoyaxaedsuttydmmhhpktpmsrjtgwdpfnsboxgwlbaawzuefywkdplrsrjynbvygabwjldapfcsdwkbrkch"
        );

        // The crypto-request example of BCR's ur-99-request-response.
        let request = [
            0xa2, 0x01, 0xd8, 0x25, 0x50, 0x02, 0x0c, 0x22, 0x3a, 0x86, 0xf7, 0x46, 0x46, 0x93,
            0xfc, 0x65, 0x0e, 0xf3, 0xca, 0xc0, 0x47, 0x02, 0xd9, 0x01, 0xf4, 0xa1, 0x01, 0xd9,
            0x02, 0x58, 0x58, 0x20, 0xe8, 0x24, 0x46, 0x7c, 0xaf, 0xfe, 0xaf, 0x3b, 0xbc, 0x3e,
            0x0c, 0xa0, 0x95, 0xe6, 0x60, 0xa9, 0xba, 0xd8, 0x0d, 0xdb, 0x6a, 0x91, 0x94, 0x33,
            0xa3, 0x71, 0x61, 0x90, 0x8b, 0x9a, 0x39, 0x86,
        ];
        assert_eq!(
            encode_single(&request, "crypto-request"),
            "ur:crypto-request/oeadtpdagdaobncpftlnylfgfgmuztihbawfsgrtflaotaadwkoyadtaaohdhdcxvsdkfgkepezepefrrffmbnnbmdvahnptrdtpbtuyimmemweootjshsmhlunyeslnameyhsdi"
        );

        // The 256-byte message as an animated ur:bytes stream, twenty
        // parts over nine fragments: the first nine are the fragments,
        // the rest fountain mixes.
        let mut encoder = Encoder::bytes(&make_message("Wolf", 256), 30).unwrap();
        assert_eq!(encoder.fragment_count(), 9);
        let expected = [
            "ur:bytes/1-9/lpadascfadaxcywenbpljkhdcahkadaemejtswhhylkepmykhhtsytsnoyoyaxaedsuttydmmhhpktpmsrjtdkgslpgh",
            "ur:bytes/2-9/lpaoascfadaxcywenbpljkhdcagwdpfnsboxgwlbaawzuefywkdplrsrjynbvygabwjldapfcsgmghhkhstlrdcxaefz",
            "ur:bytes/3-9/lpaxascfadaxcywenbpljkhdcahelbknlkuejnbadmssfhfrdpsbiegecpasvssovlgeykssjykklronvsjksopdzmol",
            "ur:bytes/4-9/lpaaascfadaxcywenbpljkhdcasotkhemthydawydtaxneurlkosgwcekonertkbrlwmplssjtammdplolsbrdzcrtas",
            "ur:bytes/5-9/lpahascfadaxcywenbpljkhdcatbbdfmssrkzmcwnezelennjpfzbgmuktrhtejscktelgfpdlrkfyfwdajldejokbwf",
            "ur:bytes/6-9/lpamascfadaxcywenbpljkhdcackjlhkhybssklbwefectpfnbbectrljectpavyrolkzczcpkmwidmwoxkilghdsowp",
            "ur:bytes/7-9/lpatascfadaxcywenbpljkhdcavszmwnjkwtclrtvaynhpahrtoxmwvwatmedibkaegdosftvandiodagdhthtrlnnhy",
            "ur:bytes/8-9/lpayascfadaxcywenbpljkhdcadmsponkkbbhgsoltjntegepmttmoonftnbuoiyrehfrtsabzsttorodklubbuyaetk",
            "ur:bytes/9-9/lpasascfadaxcywenbpljkhdcajskecpmdckihdyhphfotjojtfmlnwmadspaxrkytbztpbauotbgtgtaeaevtgavtny",
            "ur:bytes/10-9/lpbkascfadaxcywenbpljkhdcahkadaemejtswhhylkepmykhhtsytsnoyoyaxaedsuttydmmhhpktpmsrjtwdkiplzs",
            "ur:bytes/11-9/lpbdascfadaxcywenbpljkhdcahelbknlkuejnbadmssfhfrdpsbiegecpasvssovlgeykssjykklronvsjkvetiiapk",
            "ur:bytes/12-9/lpbnascfadaxcywenbpljkhdcarllaluzmdmgstospeyiefmwejlwtpedamktksrvlcygmzemovovllarodtmtbnptrs",
            "ur:bytes/13-9/lpbtascfadaxcywenbpljkhdcamtkgtpknghchchyketwsvwgwfdhpgmgtylctotzopdrpayoschcmhplffziachrfgd",
            "ur:bytes/14-9/lpbaascfadaxcywenbpljkhdcapazewnvonnvdnsbyleynwtnsjkjndeoldydkbkdslgjkbbkortbelomueekgvstegt",
            "ur:bytes/15-9/lpbsascfadaxcywenbpljkhdcaynmhpddpzmversbdqdfyrehnqzlugmjzmnmtwmrouohtstgsbsahpawkditkckynwt",
            "ur:bytes/16-9/lpbeascfadaxcywenbpljkhdcawygekobamwtlihsnpalnsghenskkiynthdzotsimtojetprsttmukirlrsbtamjtpd",
            "ur:bytes/17-9/lpbyascfadaxcywenbpljkhdcamklgftaxykpewyrtqzhydntpnytyisincxmhtbceaykolduortotiaiaiafhiaoyce",
            "ur:bytes/18-9/lpbgascfadaxcywenbpljkhdcahkadaemejtswhhylkepmykhhtsytsnoyoyaxaedsuttydmmhhpktpmsrjtntwkbkwy",
            "ur:bytes/19-9/lpbwascfadaxcywenbpljkhdcadekicpaajootjzpsdrbalpeywllbdsnbinaerkurspbncxgslgftvtsrjtksplcpeo",
            "ur:bytes/20-9/lpbbascfadaxcywenbpljkhdcayapmrleeleaxpasfrtrdkncffwjyjzgyetdmlewtkpktgllepfrltataztksmhkbot",
        ];
        for (index, e) in expected.into_iter().enumerate() {
            assert_eq!(encoder.emitted(), index);
            assert_eq!(encoder.next_part(), e);
        }
    }

    /// Streams the `ur` 0.5.2 crate produced for a payload that is not
    /// one of the specification's, recorded here so that the match is
    /// asserted for good once that crate is gone. Fragment lengths 20,
    /// 60 and 400 are the ends and the middle of what `opensigner-core`
    /// offers (`UR_FRAGMENT_MIN`, `UR_FRAGMENT_LEN`, `UR_FRAGMENT_MAX`).
    #[test]
    fn parts_recorded_from_the_reference_crate() {
        let payload: Vec<u8> = (0..120u32).map(|i| (i * 37 % 251) as u8).collect();

        let mut e = Encoder::psbt(&payload, 20).unwrap();
        assert_eq!(e.fragment_count(), 7);
        for expected in [
            "ur:crypto-psbt/1-7/lpadatcskncyjyrthdztgmhdksaedagejlmwrhueaydpgmktnssevabeecfrfwesgw",
            "ur:crypto-psbt/2-7/lpaoatcskncyjyrthdztgmhtlboxsowycsfsidltpsttyncxfeimmyqztahesskswn",
            "ur:crypto-psbt/3-7/lpaxatcskncyjyrthdztgmaxdegtjpmsrfvybddygoknnesswlbwethllfkgdssbch",
            "ur:crypto-psbt/4-7/lpaaatcskncyjyrthdztgmossfwncwfzihlepetyytcnfdjnmorluoamdnswswfeox",
            "ur:crypto-psbt/5-7/lpahatcskncyjyrthdztgmgdkpnyrsvebaeohdkioestwpcmfrhnlppktkbdttdsqz",
            "ur:crypto-psbt/6-7/lpamatcskncyjyrthdztgmwkckfxislgprtsaddsgrjomdrdurasdmgukspfemsths",
            "ur:crypto-psbt/7-7/lpatatcskncyjyrthdztgmntsavdbyenhplaonsgwscffmialoaeaeaeaefwsneypa",
            "ur:crypto-psbt/8-7/lpayatcskncyjyrthdztgmgdkpnyrsvebaeohdkioestwpcmfrhnlppktkadcestec",
            "ur:crypto-psbt/9-7/lpasatcskncyjyrthdztgmhtwtsektgdkiprneetwlfhhddatnosrtcmstgdiebndw",
        ] {
            assert_eq!(e.next_part(), expected);
        }

        let mut e = Encoder::psbt(&payload, 60).unwrap();
        assert_eq!(e.fragment_count(), 3);
        for expected in [
            "ur:crypto-psbt/1-3/lpadaxcskncyjyrthdzthddthdksaedagejlmwrhueaydpgmktnssevabeechtlboxsowycsfsidltpsttyncxfeimmyqztaaxdegtjpmsaocxkise",
            "ur:crypto-psbt/2-3/lpaoaxcskncyjyrthdzthddtrfvybddygoknnesswlbwethllfossfwncwfzihlepetyytcnfdjnmorluoamdngdkpnyrsvebaeohdkioepspdnsws",
            "ur:crypto-psbt/3-3/lpaxaxcskncyjyrthdzthddtstwpcmfrhnlppktkwkckfxislgprtsaddsgrjomdrdurasdmguksntsavdbyenhplaonsgwscffmialoaesftersrk",
            "ur:crypto-psbt/4-3/lpaaaxcskncyjyrthdzthddtkgbtcabdeczmecbdcabtkgecbsbzcwwtfsbdbzctbzbdwtbtcwbzbskpfrchcabdykfhkpbdchbtfrykoeyttdtpjk",
            "ur:crypto-psbt/5-3/lpahaxcskncyjyrthdzthddtcnkpcadmlbmhoyprsrahhfioksldtncmdpfmgwhnpasackbzdsktlotawdvyfsglnepfsetdbbdakolteckttlghwe",
            "ur:crypto-psbt/6-3/lpamaxcskncyjyrthdzthddtkgbtcabdeczmecbdcabtkgecbsbzcwwtfsbdbzctbzbdwtbtcwbzbskpfrchcabdykfhkpbdchbtfrykoedrgrcxin",
            "ur:crypto-psbt/7-3/lpataxcskncyjyrthdzthddtrfvybddygoknnesswlbwethllfossfwncwfzihlepetyytcnfdjnmorluoamdngdkpnyrsvebaeohdkioerhdsmtms",
            "ur:crypto-psbt/8-3/lpayaxcskncyjyrthdzthddtrfvybddygoknnesswlbwethllfossfwncwfzihlepetyytcnfdjnmorluoamdngdkpnyrsvebaeohdkioeltqzloct",
            "ur:crypto-psbt/9-3/lpasaxcskncyjyrthdzthddtvenlbdbzctbzbdkiemcwbzbsykfrbtchbdkpfhykbdcachfrkpbsbzcwbtwtbdbzctbzbdfsbtcwbzbsecasqdaaly",
        ] {
            assert_eq!(e.next_part(), expected);
        }

        // A payload that fits one fragment still animates: every part is
        // the whole message, and a receiver finishes on the first.
        let mut e = Encoder::bytes(&payload, 400).unwrap();
        assert_eq!(e.fragment_count(), 1);
        assert_eq!(
            e.next_part(),
            "ur:bytes/1-1/lpadadcskncyjyrthdzthdknhdksaedagejlmwrhueaydpgmktnssevabeechtlboxsowycsfsidltpsttyncxfeimmyqztaaxdegtjpmsrfvybddygoknnesswlbwethllfossfwncwfzihlepetyytcnfdjnmorluoamdngdkpnyrsvebaeohdkioestwpcmfrhnlppktkwkckfxislgprtsaddsgrjomdrdurasdmguksntsavdbyenhplaonsgwscffmialolschsann"
        );

        assert_eq!(
            encode_psbt(&payload),
            "ur:crypto-psbt/hdksaedagejlmwrhueaydpgmktnssevabeechtlboxsowycsfsidltpsttyncxfeimmyqztaaxdegtjpmsrfvybddygoknnesswlbwethllfossfwncwfzihlepetyytcnfdjnmorluoamdngdkpnyrsvebaeohdkioestwpcmfrhnlppktkwkckfxislgprtsaddsgrjomdrdurasdmguksntsavdbyenhplaonsgwscffmialojyrthdzt"
        );
        assert_eq!(
            decode_single(&encode_psbt(&payload)).unwrap(),
            Message::Psbt(payload)
        );
    }

    #[test]
    fn cbor_byte_strings() {
        assert_eq!(cbor_bytes(b""), alloc::vec![0x40]);
        assert_eq!(cbor_bytes(b"ab"), alloc::vec![0x42, b'a', b'b']);
        let long = alloc::vec![7u8; 300];
        let c = cbor_bytes(&long);
        assert_eq!(&c[..3], &[0x59, 0x01, 0x2c]);
        assert_eq!(cbor_byte_string(&c).unwrap(), long);
        let mid = alloc::vec![1u8; 100];
        assert_eq!(cbor_bytes(&mid)[..2], [0x58, 100]);
        assert_eq!(
            cbor_byte_string(&[0x42, 1]).unwrap_err(),
            Error::NotByteString
        );
        assert_eq!(
            cbor_byte_string(&[0xa1, 1, 2]).unwrap_err(),
            Error::NotByteString
        );
    }

    #[test]
    fn psbt_single_part_round_trip() {
        let psbt = b"psbt\xff\x01\x00\x02\x03";
        let text = encode_psbt(psbt);
        assert!(text.starts_with("ur:crypto-psbt/"));
        assert_eq!(decode_single(&text).unwrap(), Message::Psbt(psbt.to_vec()));
        assert_eq!(
            decode_single(&text.to_ascii_uppercase()).unwrap(),
            Message::Psbt(psbt.to_vec())
        );
        let mut d = Decoder::new();
        assert!(d.receive(&text).unwrap());
        assert_eq!(d.progress(), Some((1, 1)));
        assert_eq!(d.message(), Some(&Message::Psbt(psbt.to_vec())));
        let b = encode_bytes(b"data");
        assert_eq!(decode_single(&b).unwrap(), Message::Bytes(b"data".to_vec()));
        assert!(is_ur("UR:BYTES/x") && !is_ur("bytes"));
        assert_eq!(decode_single("bytes/x").unwrap_err(), Error::NotUr);
    }

    #[test]
    fn multi_part_out_of_order_and_with_losses() {
        let psbt: Vec<u8> = (0..1000u32).map(|i| (i * 7 % 251) as u8).collect();
        let mut e = Encoder::psbt(&psbt, 200).unwrap();
        let n = e.fragment_count();
        assert_eq!(n, 6, "1005 CBOR bytes in 200-byte fragments");
        let parts: Vec<String> = (0..12 * n).map(|_| e.next_part()).collect();
        assert!(parts[0].starts_with("ur:crypto-psbt/1-6/"));
        assert_eq!(e.emitted(), 12 * n);

        // Pure parts in reverse order.
        let mut d = Decoder::new();
        for (i, p) in parts[..n].iter().rev().enumerate() {
            let done = d.receive(p).unwrap();
            assert_eq!(done, i + 1 == n);
            assert_eq!(d.progress(), Some((i + 1, n)));
        }
        assert_eq!(d.message(), Some(&Message::Psbt(psbt.clone())));
        assert_eq!(d.ur_type().as_deref(), Some("crypto-psbt"));

        // Every other part, skipping half: fountain mixes fill the gaps.
        let mut d = Decoder::new();
        let mut done = false;
        for p in parts.iter().skip(1).step_by(2) {
            done = d.receive(p).unwrap();
            if done {
                break;
            }
        }
        assert!(done, "completed from mixed parts");
        assert_eq!(d.into_message(), Some(Message::Psbt(psbt.clone())));

        // A part of another message is a mismatch, not a crash.
        let other: Vec<u8> = (0..1000u32).map(|i| (i * 3 % 251) as u8).collect();
        let mut e2 = Encoder::psbt(&other, 200).unwrap();
        let foreign = e2.next_part();
        let mut d = Decoder::new();
        d.receive(&parts[0]).unwrap();
        assert_eq!(d.receive(&foreign).unwrap_err(), Error::Mismatch);
        let mut b = Encoder::bytes(b"hello", 2).unwrap();
        let bp = b.next_part();
        assert_eq!(d.receive(&bp).unwrap_err(), Error::Mismatch);
        assert!(!d.is_complete());
        assert!(matches!(Encoder::psbt(b"", 10), Err(Error::Empty)));
    }

    /// Every part a coordinator sends is upper case inside a QR code,
    /// and a scan may hand back either case.
    #[test]
    fn a_stream_decodes_in_either_case() {
        let message = b"Ten chars!";
        let mut e = Encoder::bytes(message, 5).unwrap();
        let mut d = Decoder::new();
        for _ in 0..e.fragment_count() {
            d.receive(&e.next_part().to_ascii_uppercase()).unwrap();
        }
        assert_eq!(d.into_message(), Some(Message::Bytes(message.to_vec())));
    }

    /// The strings a scanner sees that are not a UR it can use.
    #[test]
    fn strings_that_are_not_a_usable_ur() {
        for bad in [
            "uhr:bytes/aeadaolazmjendeoti",
            "ur:aeadaolazmjendeoti",
            "ur:bytes#4/aeadaolazmjendeoti",
            "ur:/aeadaolazmjendeoti",
            "ur:bytes/1-1a/aeadaolazmjendeoti",
            "ur:bytes/0-1/aeadaolazmjendeoti",
            "ur:bytes/1-0/aeadaolazmjendeoti",
            "ur:bytes/1-1/toomuch/aeadaolazmjendeoti",
        ] {
            assert!(Decoder::new().receive(bad).is_err(), "accepted {bad}");
        }
        // A part whose header disagrees with its own CBOR.
        let mut e = Encoder::bytes(b"Ten chars!", 5).unwrap();
        let tampered = e.next_part().replacen("/1-", "/2-", 1);
        assert_eq!(
            Decoder::new().receive(&tampered).unwrap_err(),
            Error::Mismatch
        );
        // A single-part UR after a multi-part one has started.
        let mut d = Decoder::new();
        d.receive(&e.next_part()).unwrap();
        assert_eq!(
            d.receive(&encode_bytes(b"data")).unwrap_err(),
            Error::Mismatch
        );
    }

    /// A hostile code can claim any number of fragments it likes, and a
    /// receiver that believes it commits memory for every one of them
    /// before a single part has been checked.
    #[test]
    fn a_part_claiming_more_parts_than_the_transport_carries_is_refused() {
        let psbt: Vec<u8> = (0..1000u32).map(|i| (i * 7 % 251) as u8).collect();
        let mut e = Encoder::psbt(&psbt, 200).unwrap();
        let part = e.next_part();
        let payload = part.rsplit('/').next().unwrap();

        let mut d = Decoder::new();
        assert_eq!(
            d.receive(&alloc::format!("ur:crypto-psbt/1-100000/{payload}")),
            Err(Error::TooManyParts)
        );
        assert!(!d.is_complete());
        assert_eq!(d.progress(), None, "nothing was started for it");

        // The real run of the same message is unaffected.
        let n = e.fragment_count();
        let mut d = Decoder::new();
        assert!(d.receive(&part).is_ok());
        for _ in 1..n {
            d.receive(&e.next_part()).unwrap();
        }
        assert_eq!(d.message(), Some(&Message::Psbt(psbt)));
    }
}
