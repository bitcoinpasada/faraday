//! The QR sender: a thing to send made into the codes that carry it
//! (`docs/QR.md`). A PSBT goes as `ur:crypto-psbt` or BBQr `P`; text a
//! wallet reads (a descriptor, a key) as one code while it fits, else
//! `ur:bytes`, BBQr `U` or numbered parts; a cosigner's key as
//! `ur:crypto-account` when it has one; any other file in the Faraday
//! file envelope, as `ur:bytes` or BBQr `J`.
//!
//! No wording: what was made is answered as a [`Made`], and each app
//! says it in its own words. Moved here from `faraday-core` so that the
//! desktop app and the stick make the same codes.

use faraday_codec::qr::{Ecc, Payload, QrMatrix, encode, encode_bounded};
use faraday_crypto::work::Compute;
use zeroize::{Zeroize, Zeroizing};

/// What a QR view shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A PSBT: `ur:crypto-psbt`, or BBQr `P`.
    Psbt(Vec<u8>),
    /// Text a wallet reads (a descriptor, a key): one code while it fits,
    /// else `ur:bytes`, or BBQr `U`.
    Text(String),
    /// A cosigner's account key: `ur:crypto-account` when it has one (a
    /// `wsh` multisig key, as SeedSigner shows its own), else as
    /// text; or BBQr `U` of the `[fingerprint/path]xpub` line.
    Key(String, Option<String>),
    /// Any other file, in the Faraday file envelope (`docs/QR.md`):
    /// `ur:bytes`, or BBQr `J`, as Faraday OS reads them.
    File(String, Vec<u8>),
}

impl Source {
    /// Whether numbered parts can carry it: text only.
    pub fn is_text(&self) -> bool {
        matches!(self, Source::Text(_) | Source::Key(..))
    }
}

/// How a QR view is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Uniform Resources: what Sparrow and most wallets read.
    Ur,
    /// BBQr: what Coldcard writes and SeedSigner 0.8.7 and newer read.
    Bbqr,
    /// Specter's numbered text parts, `p1of3 …`: what SeedSigner reads a
    /// descriptor or a key in when it does not read BBQr. Text only.
    Parts,
}

impl Format {
    /// The formats in the order a sheet lists them.
    pub const ALL: [Format; 3] = [Format::Ur, Format::Bbqr, Format::Parts];
}

/// The part sizes offered, in bytes of data a frame.
pub const PARTS: [usize; 3] = [100, 160, 250];

/// The frame rates offered, as milliseconds a frame: 2, 3 and 5 a
/// second (`docs/QR.md`: 2 by default).
pub const SPEEDS: [u64; 3] = [500, 333, 200];

/// What the codes are, for the line a screen writes under them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Made {
    /// `ur:crypto-psbt` in one code.
    UrPsbtOne,
    /// `ur:crypto-psbt` in this many frames.
    UrPsbtFrames(usize),
    /// Text in one code.
    TextOne,
    /// A key's line in one code.
    KeyOne,
    /// `ur:crypto-account` in one code.
    UrAccount,
    /// `ur:bytes` in one code: of a file's envelope when `file`.
    UrBytesOne {
        /// The envelope of a file, not text.
        file: bool,
    },
    /// `ur:bytes` in this many frames.
    UrBytesFrames {
        /// The envelope of a file, not text.
        file: bool,
        /// Frames.
        frames: usize,
    },
    /// BBQr of this file type in one code.
    BbqrOne(char),
    /// BBQr of this file type in this many frames.
    BbqrFrames(char, usize),
    /// Text short enough for one code, where numbered parts were asked.
    PartsOne,
    /// Numbered parts in this many frames.
    PartsNumbered(usize),
}

/// Why no codes were made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// More than BBQr carries.
    TooLargeForBbqr,
    /// The file envelope refused the file.
    Envelope(crate::envelope::Error),
    /// The codec could not make a code.
    Code,
}

fn code_error<E>(_: E) -> Error {
    Error::Code
}

/// The text of parts that may spell a secret, wiped.
fn wipe(mut parts: Vec<String>) {
    for p in &mut parts {
        p.zeroize();
    }
}

/// What a sender makes: what it is, and the codes, cycled when there is
/// more than one.
pub type Frames = (Made, Vec<QrMatrix>);

/// Costs a QR encoding. `source` written as `format`, `part` bytes a
/// frame when animated: one code where it fits, as `docs/QR.md` says
/// for each.
pub fn frames(c: &Compute, source: &Source, format: Format, part: usize) -> Result<Frames, Error> {
    Ok(match (source, format) {
        (Source::Psbt(bytes), Format::Ur) => {
            let single = Zeroizing::new(faraday_codec::ur::encode_psbt(bytes).to_ascii_uppercase());
            if let Ok(m) = encode_bounded(c, Payload::Alphanumeric(single.as_bytes()), Ecc::Low, 20)
            {
                (Made::UrPsbtOne, vec![m])
            } else {
                ur_psbt_frames(c, bytes, part)?
            }
        }
        (Source::Psbt(bytes), Format::Bbqr | Format::Parts) => bbqr_frames(c, 'P', bytes, part)?,
        (Source::Text(text), Format::Parts) | (Source::Key(text, _), Format::Parts) => {
            parts_frames(c, text, part)?
        }
        (Source::Text(text), Format::Ur) => {
            if let Ok(m) = encode_bounded(c, Payload::Bytes(text.as_bytes()), Ecc::Medium, 25) {
                (Made::TextOne, vec![m])
            } else {
                ur_bytes_frames(c, text.as_bytes(), false, part)?
            }
        }
        (Source::Text(text), Format::Bbqr) | (Source::Key(text, _), Format::Bbqr) => {
            bbqr_frames(c, 'U', text.as_bytes(), part)?
        }
        (Source::Key(_, Some(ur)), Format::Ur) => {
            let upper = Zeroizing::new(ur.to_ascii_uppercase());
            let m = encode_bounded(c, Payload::Alphanumeric(upper.as_bytes()), Ecc::Low, 20)
                .map_err(code_error)?;
            (Made::UrAccount, vec![m])
        }
        (Source::Key(text, None), Format::Ur) => {
            let m = encode_bounded(c, Payload::Bytes(text.as_bytes()), Ecc::Medium, 25)
                .map_err(code_error)?;
            (Made::KeyOne, vec![m])
        }
        (Source::File(name, data), f) => {
            let env = Zeroizing::new(envelope(name, data)?);
            match f {
                Format::Ur => ur_bytes_frames(c, &env, true, part)?,
                Format::Bbqr | Format::Parts => bbqr_frames(c, 'J', &env, part)?,
            }
        }
    })
}

fn envelope(name: &str, data: &[u8]) -> Result<Vec<u8>, Error> {
    crate::envelope::pack(&envelope_name(name), data, "file").map_err(Error::Envelope)
}

/// Costs a QR encoding (through `animated`).
fn ur_psbt_frames(c: &Compute, bytes: &[u8], part: usize) -> Result<Frames, Error> {
    let frames = animated(
        c,
        faraday_codec::ur::Encoder::psbt(bytes, part).map_err(code_error)?,
        bytes.len(),
        part,
    )?;
    Ok((Made::UrPsbtFrames(frames.len()), frames))
}

/// Costs a QR encoding (through `numbered`). Specter's numbered parts,
/// `part` characters of text a frame, one code each; a text that fits
/// one frame is one code with no number.
pub fn parts_frames(c: &Compute, text: &str, part: usize) -> Result<Frames, Error> {
    let parts = if text.chars().count() <= part {
        vec![text.to_string()]
    } else {
        crate::specter::encode(text, part)
    };
    let made = numbered(c, &parts);
    wipe(parts);
    made
}

/// Costs a QR encoding (through `encode`).
fn numbered(c: &Compute, parts: &[String]) -> Result<Frames, Error> {
    let frames = parts
        .iter()
        .map(|p| encode(c, Payload::Bytes(p.as_bytes()), Ecc::Low).map_err(code_error))
        .collect::<Result<Vec<_>, _>>()?;
    let made = if frames.len() == 1 {
        Made::PartsOne
    } else {
        Made::PartsNumbered(frames.len())
    };
    Ok((made, frames))
}

/// Costs a QR encoding (through `encode`). BBQr parts in encoding `2`,
/// one code each.
pub fn bbqr_frames(c: &Compute, t: char, data: &[u8], part: usize) -> Result<Frames, Error> {
    let parts = crate::bbqr::encode(t, data, part).ok_or(Error::TooLargeForBbqr)?;
    let frames = parts
        .iter()
        .map(|p| encode(c, Payload::Alphanumeric(p.as_bytes()), Ecc::Low).map_err(code_error))
        .collect::<Result<Vec<_>, _>>();
    wipe(parts);
    let frames = frames?;
    let made = if frames.len() == 1 {
        Made::BbqrOne(t)
    } else {
        Made::BbqrFrames(t, frames.len())
    };
    Ok((made, frames))
}

/// Costs a QR encoding (through `encode_bounded` and
/// `ur_bytes_animated`). Bytes as `ur:bytes`: one code when it fits,
/// else animated.
pub fn ur_bytes_frames(c: &Compute, data: &[u8], file: bool, part: usize) -> Result<Frames, Error> {
    let single = Zeroizing::new(faraday_codec::ur::encode_bytes(data).to_ascii_uppercase());
    if let Ok(m) = encode_bounded(c, Payload::Alphanumeric(single.as_bytes()), Ecc::Low, 20) {
        return Ok((Made::UrBytesOne { file }, vec![m]));
    }
    ur_bytes_animated(c, data, file, part)
}

/// Costs a QR encoding (through `animated`).
fn ur_bytes_animated(c: &Compute, data: &[u8], file: bool, part: usize) -> Result<Frames, Error> {
    let frames = animated(
        c,
        faraday_codec::ur::Encoder::bytes(data, part).map_err(code_error)?,
        data.len(),
        part,
    )?;
    Ok((
        Made::UrBytesFrames {
            file,
            frames: frames.len(),
        },
        frames,
    ))
}

/// A file's name as the envelope takes one: letters, digits, dots,
/// hyphens and underscores, starting with a letter or digit.
pub fn envelope_name(name: &str) -> String {
    let mut n: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._-".contains(c) {
                c
            } else {
                '-'
            }
        })
        .take(128)
        .collect();
    if !n.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        n.insert(0, 'f');
        n.truncate(128);
    }
    n
}

/// Costs a QR encoding (through `encode`). The frames of a
/// fountain-coded UR: twice the fragments it takes, so a scanner that
/// misses a few still completes on the first pass.
pub fn animated(
    c: &Compute,
    mut enc: faraday_codec::ur::Encoder,
    len: usize,
    frag: usize,
) -> Result<Vec<QrMatrix>, Error> {
    let count = (len.div_ceil(frag) * 2).max(4);
    (0..count)
        .map(|_| {
            let part = Zeroizing::new(enc.next_part().to_ascii_uppercase());
            encode(c, Payload::Alphanumeric(part.as_bytes()), Ecc::Low).map_err(code_error)
        })
        .collect()
}
