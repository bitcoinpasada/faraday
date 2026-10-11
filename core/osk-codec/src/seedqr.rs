//! SeedQR and CompactSeedQR (SeedSigner's `docs/seed_qr/README.md`).
//!
//! **SeedQR**: every word's 0-based wordlist index written as four
//! zero-padded decimal digits, concatenated, encoded in numeric mode. 12
//! words are 48 digits (a 25×25 code), 24 words are 96 digits (29×29).
//!
//! **CompactSeedQR**: the entropy bytes behind the words, without the
//! checksum bits, in byte mode. 12 words are 16 bytes (21×21), 24 words
//! are 32 bytes (25×25).
//!
//! Neither format carries the wordlist language: the specification
//! assumes English, and a SeedQR made from another list decodes to the
//! same indices. The Load wizard therefore still asks the language after
//! a scan (`docs/UX.md` §7.2), and [`from_digits`] / [`from_entropy`]
//! take it as a parameter.
//!
//! Payloads are secrets: the digit buffer is a fixed array inside a
//! [`Secret`], the entropy comes out of [`Mnemonic::entropy`] already
//! wrapped, and both encoders return a zeroizing [`QrMatrix`].

use core::fmt;

use osk_bip::bip39::{self, Entropy, Language, MAX_WORDS, Mnemonic};
use osk_crypto::Secret;
use zeroize::Zeroize;

use crate::qr::{self, Ecc, Payload, QrMatrix};

/// Digits of a 24-word SeedQR, the longest.
pub const MAX_DIGITS: usize = MAX_WORDS * 4;

/// Why a payload is not a SeedQR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// Not 48 or 96 digits, or not 16 or 32 bytes.
    BadLength,
    /// A non-digit character in a SeedQR.
    BadCharacter,
    /// A word index of 2048 or more.
    IndexOutOfRange,
    /// The words do not form a valid mnemonic.
    Mnemonic(bip39::Error),
    /// The QR encoder refused the payload (cannot happen for valid
    /// mnemonics; kept for completeness).
    Qr(qr::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadLength => f.write_str("not a SeedQR: wrong length"),
            Error::BadCharacter => f.write_str("not a SeedQR: non-digit character"),
            Error::IndexOutOfRange => f.write_str("not a SeedQR: word index 2048 or more"),
            Error::Mnemonic(e) => write!(f, "{e}"),
            Error::Qr(e) => write!(f, "{e}"),
        }
    }
}

impl core::error::Error for Error {}

/// The ASCII digits of a SeedQR, at most [`MAX_DIGITS`] of them.
pub struct Digits {
    buf: [u8; MAX_DIGITS],
    len: u8,
}

impl Digits {
    /// The digits.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..usize::from(self.len)]
    }
}

impl AsRef<[u8]> for Digits {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Zeroize for Digits {
    fn zeroize(&mut self) {
        self.buf.zeroize();
        self.len = 0;
    }
}

/// The SeedQR digit stream of `m`.
pub fn to_digits(m: &Mnemonic) -> Secret<Digits> {
    let mut out = Secret::new(Digits {
        buf: [b'0'; MAX_DIGITS],
        len: 0,
    });
    let d = out.expose_mut();
    for (i, &w) in m.indices().iter().enumerate() {
        let mut v = w;
        for k in (0..4).rev() {
            d.buf[i * 4 + k] = b'0' + (v % 10) as u8;
            v /= 10;
        }
    }
    d.len = (m.word_count() * 4) as u8;
    out
}

/// The CompactSeedQR bytes of `m`: its entropy.
pub fn to_entropy(m: &Mnemonic) -> Secret<Entropy> {
    m.entropy()
}

/// The words behind a SeedQR digit stream, read with wordlist `lang`.
pub fn from_digits(digits: &[u8], lang: Language) -> Result<Mnemonic, Error> {
    if !matches!(digits.len(), 48 | 96) {
        return Err(Error::BadLength);
    }
    let mut indices = Secret::new([0u16; MAX_WORDS]);
    let count = digits.len() / 4;
    for (i, chunk) in digits.chunks(4).enumerate() {
        let mut v = 0u16;
        for &c in chunk {
            if !c.is_ascii_digit() {
                return Err(Error::BadCharacter);
            }
            v = v * 10 + u16::from(c - b'0');
        }
        if v >= 2048 {
            return Err(Error::IndexOutOfRange);
        }
        indices.expose_mut()[i] = v;
    }
    Mnemonic::from_indices(lang, &indices.expose()[..count]).map_err(Error::Mnemonic)
}

/// The words behind a CompactSeedQR: 16 or 32 entropy bytes, read with
/// wordlist `lang`.
pub fn from_entropy(bytes: &[u8], lang: Language) -> Result<Mnemonic, Error> {
    if !matches!(bytes.len(), 16 | 32) {
        return Err(Error::BadLength);
    }
    Mnemonic::from_entropy(lang, bytes).map_err(Error::Mnemonic)
}

/// The SeedQR of `m`: numeric mode, level L, as the specification says.
pub fn encode_seedqr(m: &Mnemonic) -> Result<QrMatrix, Error> {
    let digits = to_digits(m);
    qr::encode(Payload::Numeric(digits.expose().as_bytes()), Ecc::Low).map_err(Error::Qr)
}

/// The CompactSeedQR of `m`: byte mode, level L.
pub fn encode_compact(m: &Mnemonic) -> Result<QrMatrix, Error> {
    let entropy = to_entropy(m);
    qr::encode(Payload::Bytes(entropy.expose().as_bytes()), Ecc::Low).map_err(Error::Qr)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The error of a result whose success type is not `Debug`.
    fn err<T>(r: Result<T, Error>) -> Error {
        match r {
            Ok(_) => panic!("expected an error"),
            Err(e) => e,
        }
    }

    use alloc::vec::Vec;

    /// The worked examples of the SeedQR specification (SeedSigner
    /// `docs/seed_qr/README.md`, "Test SeedQRs"): words, digit stream,
    /// CompactSeedQR bytes.
    const VECTORS: &[(&str, &str, &[u8])] = &[
        (
            "attack pizza motion avocado network gather crop fresh patrol unusual wild holiday candy pony ranch winter theme error hybrid van cereal salon goddess expire",
            "011513251154012711900771041507421289190620080870026613431420201617920614089619290300152408010643",
            b"\x0et\xb6A\x07\xf9L\xc0\xcc\xfa\xe6\xa1=\xcb\xec6b\x15O\xecg\xe0\xe0\t\x99\xc0x\x92Y}\x19\n",
        ),
        (
            "atom solve joy ugly ankle message setup typical bean era cactus various odor refuse element afraid meadow quick medal plate wisdom swap noble shallow",
            "011416550964188800731119157218870156061002561932122514430573003611011405110613292018175411971576",
            b"\x0eY\xdd\xe2v\x00\x93\x17\xf1'_\x13\x89\x88\x80x\xc9\x93h\xd1\xe8$\x89\xb5\xf6)S\x1f\xc5\xb6\xa5n",
        ),
        (
            "sound federal bonus bleak light raise false engage round stock update render quote truck quality fringe palace foot recipe labor glow tortoise potato still",
            "166206750203018810361417065805941507171219081456140818651401074412730727143709940798183613501710",
            b"\xcf\xca\x8ce\x8b\xc8\x19bT\x92R\xbcz\xc3\xba[\x0b\x01\xd2k\xca\xe8\x9f+^\xce\xbe&=\xcb*6",
        ),
        (
            "forum undo fragile fade shy sign arrest garment culture tube off merit",
            "073318950739065415961602009907670428187212261116",
            b"[\xbd\x9dq\xa8\xecy\x90\x83\x1a\xff5\x9dBeE",
        ),
        (
            "good battle boil exact add seed angle hurry success glad carbon whisper",
            "080301540200062600251559007008931730078802752004",
            b"dbhd' 3\x85\xc23}\xd8LP\x89\xfd",
        ),
        (
            "approve fruit lens brass ring actual stool coin doll boss strong rate",
            "008607501025021714880023171503630517020917211425",
            b"\n\xcb\xba\x00\x8d\x9b\xa0\x05\xf5\x99k@\xa3G\\\xd9",
        ),
        (
            "dignity utility vacant shiver thought canoe feel multiply item youth actor coyote",
            "049619221923158517990268067811630950204300210397",
            b">\x1e\x0b\xc1\xe3\x1e\x0eC\x154\x8bv\xdf\xec\n\x98",
        ),
        (
            "corn voice scrap arrow original diamond trial property benefit choose junk lock",
            "038719631547010112530489185713790169032209701051",
            b"0~\xaf\x05\x86Y\xcazz\rc\x15%\t\xe5A",
        ),
        (
            "vocal tray giggle tool duck letter category pattern train magnet excite swamp",
            "196218530783182905421028028912901848107106301753",
            b"\xf5\\\xf5\x87\xf2T=\x01\t\r\n\xe7\x10\xbd;m",
        ),
    ];

    #[test]
    fn specification_vectors_encode_and_decode() {
        for (words, digits, entropy) in VECTORS {
            let m = Mnemonic::parse(Language::English, words).unwrap();
            let d = to_digits(&m);
            assert_eq!(d.expose().as_bytes(), digits.as_bytes(), "{words}");
            let e = to_entropy(&m);
            assert_eq!(e.expose().as_bytes(), *entropy, "{words}");
            let back = from_digits(digits.as_bytes(), Language::English).unwrap();
            assert_eq!(back.indices(), m.indices());
            let back = from_entropy(entropy, Language::English).unwrap();
            assert_eq!(back.indices(), m.indices());
            // Sizes from the specification.
            let (std_size, compact_size) = if m.word_count() == 12 {
                (25, 21)
            } else {
                (29, 25)
            };
            assert_eq!(encode_seedqr(&m).unwrap().size(), std_size);
            assert_eq!(encode_compact(&m).unwrap().size(), compact_size);
        }
    }

    #[test]
    fn round_trips_in_every_language() {
        // Language is not encoded: the same indices come back whatever
        // list they are read with, and the checksum is list-independent.
        for lang in Language::ALL {
            for len in [16usize, 32] {
                let entropy: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
                let m = Mnemonic::from_entropy(lang, &entropy).unwrap();
                let d = to_digits(&m);
                assert_eq!(d.expose().as_bytes().len(), m.word_count() * 4);
                let back = from_digits(d.expose().as_bytes(), lang).unwrap();
                assert_eq!(back.indices(), m.indices());
                assert_eq!(back.language(), lang);
                let back = from_entropy(&entropy, lang).unwrap();
                assert_eq!(back.indices(), m.indices());
                for other in Language::ALL {
                    let cross = from_digits(d.expose().as_bytes(), other).unwrap();
                    assert_eq!(cross.indices(), m.indices());
                }
            }
        }
    }

    #[test]
    fn rejects_bad_payloads() {
        assert_eq!(
            err(from_digits(b"1234", Language::English)),
            Error::BadLength
        );
        let mut d = [b'0'; 48];
        d[0] = b'x';
        assert_eq!(err(from_digits(&d, Language::English)), Error::BadCharacter);
        let d = [b'9'; 48];
        assert_eq!(
            err(from_digits(&d, Language::English)),
            Error::IndexOutOfRange
        );
        // All zeros: "abandon" × 12 fails the checksum.
        let d = [b'0'; 48];
        assert!(matches!(
            err(from_digits(&d, Language::English)),
            Error::Mnemonic(_)
        ));
        assert_eq!(
            err(from_entropy(&[0u8; 20], Language::English)),
            Error::BadLength
        );
    }
}
