//! What Faraday's scanner reads and its QR sender writes beyond
//! OpenSigner's codec (`docs/QR.md`): BBQr, numbered text parts, the
//! Faraday file envelope and the UR registry's key and wallet types. UR
//! itself (single and fountain-coded, `crypto-psbt` and `bytes`) stays
//! `osk-codec`'s.
//!
//! [`Assembler`] takes each scanned text in turn and says what it was: a
//! part of a transfer still missing others (and which), a whole thing that
//! arrived, a refusal with its reason, or nothing it knows, which the app
//! reads as before.

pub mod bbqr;
pub mod cbor;
pub mod envelope;
pub mod registry;
pub mod specter;
pub mod ur;

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
        /// `BBQr` or `numbered parts`.
        what: &'static str,
        /// Parts read.
        have: usize,
        /// Parts in all.
        total: usize,
        /// The parts not read yet.
        missing: Vec<usize>,
    },
    /// A whole thing.
    Done(Arrived),
    /// Refused, with the sentence a screen shows.
    Refused(String),
    /// Not a BBQr part or a numbered part.
    NotMine,
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
            return Step::Refused("A code over 4,096 characters".to_string());
        }
        if bbqr::is_part(text) {
            let c = self.bbqr.get_or_insert_with(bbqr::Collector::default);
            return match c.add(text) {
                Ok(bbqr::Got::Part {
                    have,
                    total,
                    missing,
                }) => Step::Part {
                    what: "BBQr",
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
                    Step::Refused(e.reason())
                }
            };
        }
        if let Some((n, m, body)) = specter::parse(text) {
            let c = self.specter.get_or_insert_with(specter::Collector::default);
            return match c.add(n, m, body) {
                specter::Got::Part(have, total, missing) => Step::Part {
                    what: "numbered parts",
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
                    Step::Refused(why.to_string())
                }
            };
        }
        Step::NotMine
    }

    /// Whether a transfer is part-way through.
    pub fn busy(&self) -> bool {
        self.bbqr.is_some() || self.specter.is_some()
    }
}

fn bbqr_arrived(file_type: char, data: Vec<u8>) -> Result<Arrived, String> {
    Ok(match file_type {
        'P' => Arrived::Psbt(data),
        'T' => Arrived::Transaction(data),
        'U' => Arrived::Text(String::from_utf8(data).map_err(|_| "A BBQr text that is not UTF-8")?),
        _ => {
            if envelope::looks_like(&data) {
                let (name, file, kind) = envelope::unpack(&data)?;
                Arrived::File(name, file, kind)
            } else {
                Arrived::Text(String::from_utf8(data).map_err(|_| "BBQr JSON that is not UTF-8")?)
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
    osk_bip::bitcoin::consensus::deserialize::<osk_bip::bitcoin::Transaction>(&bytes)
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
