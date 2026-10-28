//! What Faraday's scanner reads and its QR sender writes beyond
//! the codec (`docs/QR.md`): BBQr, numbered text parts, the
//! Faraday file envelope and the UR registry's key and wallet types. UR
//! itself (single and fountain-coded, `crypto-psbt` and `bytes`, and a
//! single part of any type, `faraday_codec::ur::encode_single`) stays
//! `faraday-codec`'s; [`send`] makes the codes a thing is sent as, in every
//! format a Faraday app offers.
//!
//! [`Assembler`] takes each scanned text in turn and says what it was: a
//! part of a transfer still missing others (and which), a whole thing that
//! arrived, a refusal with its reason, or nothing it knows, which the app
//! reads as before.

pub mod bbqr;
pub mod cbor;
pub mod envelope;
pub mod registry;
pub mod send;
pub mod specter;

/// What a whole transfer turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arrived {
    /// A PSBT's bytes.
    Psbt(Vec<u8>),
    /// A finished transaction's bytes.
    Transaction(Vec<u8>),
    /// A file from a Faraday file envelope: name, bytes, kind.
    File(String, Vec<u8>, String),
    /// Cosigner key lines.
    Keys(String),
    /// A descriptor.
    Descriptor(String),
    /// Any other text.
    Text(String),
}

/// What one scanned text did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// A part of a transfer: `have` of `total`, `missing` numbered from 1.
    Part {
        /// BBQr or numbered parts.
        what: Transfer,
        /// Parts read.
        have: usize,
        /// Parts in all.
        total: usize,
        /// The parts not read yet.
        missing: Vec<usize>,
    },
    /// A whole thing.
    Done(Arrived),
    /// Refused, and why: a kind, which the app words.
    Refused(Refusal),
    /// Not a BBQr part or a numbered part.
    NotMine,
}

/// Which kind of transfer a part is of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transfer {
    /// BBQr.
    Bbqr,
    /// Specter's numbered text parts.
    Numbered,
}

/// Why a scanned text was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// A code over [`bbqr::MAX_FRAME`] characters.
    TooLong,
    /// A BBQr part or transfer.
    Bbqr(bbqr::Error),
    /// Numbered parts.
    Numbered(specter::Fault),
    /// A BBQr text (`U`) that is not UTF-8.
    BbqrText,
    /// BBQr JSON (`J`) that is neither an envelope nor UTF-8.
    BbqrJson,
    /// A Faraday file envelope.
    Envelope(envelope::Error),
}

/// Transfers being put together, across scans.
#[derive(Debug, Default)]
pub struct Assembler {
    bbqr: Option<bbqr::Collector>,
    specter: Option<specter::Collector>,
}

impl Assembler {
    /// Takes one scanned text.
    pub fn feed(&mut self, text: &str) -> Step {
        let text = text.trim();
        if text.len() > bbqr::MAX_FRAME {
            return Step::Refused(Refusal::TooLong);
        }
        if bbqr::is_part(text) {
            let c = self.bbqr.get_or_insert_with(bbqr::Collector::default);
            return match c.add(text) {
                Ok(bbqr::Got::Part {
                    have,
                    total,
                    missing,
                }) => Step::Part {
                    what: Transfer::Bbqr,
                    have,
                    total,
                    missing,
                },
                Ok(bbqr::Got::Done { file_type, data }) => {
                    self.bbqr = None;
                    match bbqr_arrived(file_type, data) {
                        Ok(a) => Step::Done(a),
                        Err(why) => Step::Refused(why),
                    }
                }
                Err(e) => {
                    // A part of another transfer starts that one afresh;
                    // anything else leaves the transfer as it was.
                    if e == bbqr::Error::OtherTransfer {
                        self.bbqr = None;
                    }
                    Step::Refused(Refusal::Bbqr(e))
                }
            };
        }
        if let Some((n, m, body)) = specter::parse(text) {
            let c = self.specter.get_or_insert_with(specter::Collector::default);
            return match c.add(n, m, body) {
                specter::Got::Part(have, total, missing) => Step::Part {
                    what: Transfer::Numbered,
                    have,
                    total,
                    missing,
                },
                specter::Got::Done(t) => {
                    self.specter = None;
                    Step::Done(classify_text(&t))
                }
                specter::Got::Refused(why) => {
                    self.specter = None;
                    Step::Refused(Refusal::Numbered(why))
                }
            };
        }
        Step::NotMine
    }

    /// Parts read, of how many, of the transfer part-way through.
    pub fn progress(&self) -> Option<(usize, usize)> {
        self.bbqr
            .as_ref()
            .and_then(bbqr::Collector::progress)
            .or_else(|| self.specter.as_ref().and_then(specter::Collector::progress))
    }

    /// Whether a transfer is part-way through.
    pub fn busy(&self) -> bool {
        self.bbqr.is_some() || self.specter.is_some()
    }
}

fn bbqr_arrived(file_type: char, data: Vec<u8>) -> Result<Arrived, Refusal> {
    Ok(match file_type {
        'P' => Arrived::Psbt(data),
        'T' => Arrived::Transaction(data),
        'U' => Arrived::Text(String::from_utf8(data).map_err(|_| Refusal::BbqrText)?),
        _ => {
            if envelope::looks_like(&data) {
                let (name, file, kind) = envelope::unpack(&data).map_err(Refusal::Envelope)?;
                Arrived::File(name, file, kind)
            } else {
                Arrived::Text(String::from_utf8(data).map_err(|_| Refusal::BbqrJson)?)
            }
        }
    })
}

/// A finished transaction's bytes, when `text` is one in hex.
pub fn transaction_hex(text: &str) -> Option<Vec<u8>> {
    let t = text.trim();
    if t.len() < 120 || !t.len().is_multiple_of(2) || !t.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let bytes: Vec<u8> = (0..t.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&t[i..i + 2], 16).ok())
        .collect::<Option<_>>()?;
    faraday_bip::bitcoin::consensus::deserialize::<faraday_bip::bitcoin::Transaction>(&bytes)
        .ok()
        .map(|_| bytes)
}

/// A single text, as what it is: a Faraday file envelope, a finished
/// transaction in hex, or text.
pub fn classify_text(text: &str) -> Arrived {
    if envelope::looks_like(text.as_bytes())
        && let Ok((name, data, kind)) = envelope::unpack(text.as_bytes())
    {
        return Arrived::File(name, data, kind);
    }
    if let Some(tx) = transaction_hex(text) {
        return Arrived::Transaction(tx);
    }
    Arrived::Text(text.to_string())
}
