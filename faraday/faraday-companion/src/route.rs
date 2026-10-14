//! What arrives and where it goes (`docs/COMPANION.md` §5): every code
//! read by the camera and every paste goes through [`Reader::read`],
//! which answers what it was. A secret that stays on the Faraday device
//! is refused first, before anything else reads it, and again once a
//! transfer in parts is whole; the bytes are wiped as the answer is
//! given.

use faraday_intake::Secret;
use faraday_qr::bitlocker::KeyText;
use faraday_qr::{Arrived, Assembler, Step};
use osk_codec::PayloadKind;
use osk_codec::ur::{Decoder, Message};
use zeroize::{Zeroize, Zeroizing};

use crate::store::Kind;

/// What one code or paste was.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// A part of a transfer: `have` of `total`, and the parts still
    /// missing where the transport says (BBQr, numbered parts).
    Part {
        /// Parts read.
        have: usize,
        /// Parts in all.
        total: usize,
        /// The parts not read yet, numbered from 1; empty for UR, whose
        /// fountain parts have no fixed numbers.
        missing: Vec<usize>,
    },
    /// A secret that stays on the Faraday device: nothing is kept.
    Refused(Secret),
    /// A whole thing to keep in Received: the name it brought, if any,
    /// what it is, and the bytes.
    Arrived {
        /// The name from a Faraday file envelope.
        name: Option<String>,
        /// What it is.
        kind: Kind,
        /// The bytes, as kept.
        bytes: Vec<u8>,
        /// The extension a file without a name is kept under.
        ext: &'static str,
    },
    /// A BitLocker recovery key's text (`docs/COMPANION.md` §4.6).
    BitLocker(KeyText),
    /// An address, shown as text with its kind.
    Address {
        /// The address.
        text: String,
        /// A silent payment address rather than an ordinary one.
        silent: bool,
    },
    /// The address of one of the website's guides that `faraday-learn`
    /// has (`docs/COMPANION.md` §4.7): the Companion's own copy opens,
    /// by the page's id.
    Guide(&'static str),
    /// A web address: shown as text and never opened.
    Web(String),
    /// Any other text: shown, and kept only when the person says so.
    Text(String),
    /// A code that could not be read: a part of another transfer, a
    /// damaged part.
    NotRead,
}

/// Transfers being put together across codes.
#[derive(Default)]
pub struct Reader {
    assembler: Assembler,
    decoder: Decoder,
}

impl Reader {
    /// Drops whatever was part-way through.
    pub fn reset(&mut self) {
        self.assembler = Assembler::default();
        self.decoder = Decoder::new();
    }

    /// Whether a transfer is part-way through.
    pub fn busy(&self) -> bool {
        self.assembler.busy() || self.decoder.progress().is_some()
    }

    /// One code's bytes, or one paste's.
    pub fn read(&mut self, bytes: &[u8]) -> Outcome {
        if let Some(s) = faraday_intake::secret(bytes) {
            return self.refuse(s);
        }
        let Ok(raw) = std::str::from_utf8(bytes) else {
            return if osk_codec::classify(bytes) == PayloadKind::Psbt {
                arrived(Kind::Psbt, bytes.to_vec(), "psbt")
            } else {
                arrived(Kind::File, bytes.to_vec(), "bin")
            };
        };
        if let Some(key) = KeyText::read(raw) {
            return Outcome::BitLocker(key);
        }
        let text = raw.trim();
        match self.assembler.feed(text) {
            Step::Part {
                have,
                total,
                missing,
                ..
            } => {
                return Outcome::Part {
                    have,
                    total,
                    missing,
                };
            }
            Step::Done(a) => return self.whole(a),
            Step::Refused(_) => return Outcome::NotRead,
            Step::NotMine => {}
        }
        if osk_codec::ur::is_ur(text) {
            return match self.decoder.receive(text) {
                Ok(true) => {
                    let message = std::mem::take(&mut self.decoder).into_message();
                    match message {
                        Some(m) => self.ur(m),
                        None => Outcome::NotRead,
                    }
                }
                Ok(false) => match self.decoder.progress() {
                    Some((have, total)) => Outcome::Part {
                        have,
                        total,
                        missing: Vec::new(),
                    },
                    None => Outcome::NotRead,
                },
                Err(_) => {
                    self.decoder = Decoder::new();
                    Outcome::NotRead
                }
            };
        }
        self.text(text)
    }

    /// Refuses `s`: what was part-way through goes too.
    fn refuse(&mut self, s: Secret) -> Outcome {
        self.reset();
        Outcome::Refused(s)
    }

    /// A transfer in parts that is whole.
    fn whole(&mut self, a: Arrived) -> Outcome {
        match a {
            Arrived::Psbt(p) => self.checked(Kind::Psbt, p, "psbt"),
            Arrived::Transaction(t) => {
                let hex: String = t.iter().map(|b| format!("{b:02x}")).collect();
                arrived(Kind::Transaction, hex.into_bytes(), "txt")
            }
            Arrived::File(name, data, _) => {
                let data = Zeroizing::new(data);
                if let Some(s) = faraday_intake::secret(&data) {
                    return self.refuse(s);
                }
                Outcome::Arrived {
                    name: Some(name),
                    kind: kind_of_file(&data),
                    bytes: data.to_vec(),
                    ext: "bin",
                }
            }
            Arrived::Keys(t) | Arrived::Descriptor(t) => {
                self.checked(Kind::Description, t.into_bytes(), "txt")
            }
            Arrived::Text(t) => {
                let t = Zeroizing::new(t);
                self.text(t.trim())
            }
        }
    }

    /// `bytes` kept as `kind`, unless they are a secret.
    fn checked(&mut self, kind: Kind, bytes: Vec<u8>, ext: &'static str) -> Outcome {
        let mut bytes = bytes;
        if let Some(s) = faraday_intake::secret(&bytes) {
            bytes.zeroize();
            return self.refuse(s);
        }
        arrived(kind, bytes, ext)
    }

    /// A whole UR.
    fn ur(&mut self, m: Message) -> Outcome {
        match m {
            Message::Psbt(p) => self.checked(Kind::Psbt, p, "psbt"),
            Message::Bytes(b) => {
                let b = Zeroizing::new(b);
                if let Some(s) = faraday_intake::secret(&b) {
                    return self.refuse(s);
                }
                match std::str::from_utf8(&b) {
                    Ok(t) => self.whole(faraday_qr::classify_text(t)),
                    Err(_) => arrived(kind_of_file(&b), b.to_vec(), "bin"),
                }
            }
            Message::Other { ur_type, cbor } => {
                let cbor = Zeroizing::new(cbor);
                if let Some(s) = faraday_intake::secret_ur(&ur_type, &cbor) {
                    return self.refuse(s);
                }
                match faraday_qr::registry::read(&ur_type, &cbor) {
                    Ok(Some(faraday_qr::registry::Read::Psbt(p))) => {
                        self.checked(Kind::Psbt, p, "psbt")
                    }
                    Ok(Some(faraday_qr::registry::Read::Keys(t)))
                    | Ok(Some(faraday_qr::registry::Read::Descriptor(t))) => {
                        self.checked(Kind::Description, t.into_bytes(), "txt")
                    }
                    // A kind this build does not read is kept as the CBOR
                    // it carried, as the device's Receive keeps one.
                    _ => arrived(Kind::File, cbor.to_vec(), "cbor"),
                }
            }
        }
    }

    /// One whole text.
    fn text(&mut self, text: &str) -> Outcome {
        if let Some(s) = faraday_intake::secret(text.as_bytes()) {
            return self.refuse(s);
        }
        if let Some(key) = KeyText::read(text) {
            return Outcome::BitLocker(key);
        }
        if let Some(id) = guide(text) {
            return Outcome::Guide(id);
        }
        match faraday_qr::classify_text(text) {
            Arrived::File(name, data, _) => {
                return Outcome::Arrived {
                    name: Some(name),
                    kind: kind_of_file(&data),
                    bytes: data,
                    ext: "bin",
                };
            }
            Arrived::Transaction(t) => {
                let hex: String = t.iter().map(|b| format!("{b:02x}")).collect();
                return arrived(Kind::Transaction, hex.into_bytes(), "txt");
            }
            _ => {}
        }
        match osk_codec::classify(text.as_bytes()) {
            PayloadKind::Psbt => match psbt_bytes(text.as_bytes()) {
                Some(p) => arrived(Kind::Psbt, p, "psbt"),
                None => Outcome::Text(text.to_string()),
            },
            k if is_description(&k) => arrived(Kind::Description, text.as_bytes().to_vec(), "txt"),
            PayloadKind::Address => Outcome::Address {
                text: text.to_string(),
                silent: false,
            },
            PayloadKind::SilentAddress => Outcome::Address {
                text: text.to_string(),
                silent: true,
            },
            _ if is_web(text) => Outcome::Web(text.to_string()),
            // Any other form the device's description reader reads: a
            // Coldcard export, a wallet .json, `listdescriptors`.
            _ if faraday_intake::wallet::read_wallet(text).is_ok() => {
                arrived(Kind::Description, text.as_bytes().to_vec(), "txt")
            }
            _ => Outcome::Text(text.to_string()),
        }
    }
}

fn arrived(kind: Kind, bytes: Vec<u8>, ext: &'static str) -> Outcome {
    Outcome::Arrived {
        name: None,
        kind,
        bytes,
        ext,
    }
}

/// Whether `k` is a wallet description, as the device reads one: a
/// descriptor, an account key, a coordinator's config.
fn is_description(k: &PayloadKind) -> bool {
    matches!(
        k,
        PayloadKind::Descriptor { .. }
            | PayloadKind::Xpub
            | PayloadKind::ColdcardXpubs
            | PayloadKind::MultisigConfig
            | PayloadKind::BsmsSigner
            | PayloadKind::BsmsDescriptor
    )
}

/// A PSBT's binary bytes, from binary or base64.
pub fn psbt_bytes(bytes: &[u8]) -> Option<Vec<u8>> {
    let t = bytes.trim_ascii();
    if t.starts_with(b"psbt\xff") {
        return Some(t.to_vec());
    }
    let text = std::str::from_utf8(t).ok()?;
    let b = osk_bip::base64::decode(text).ok()?;
    b.starts_with(b"psbt\xff").then_some(b)
}

/// What a file's bytes are, as Received lists it.
pub fn kind_of_file(bytes: &[u8]) -> Kind {
    match osk_codec::classify(bytes) {
        PayloadKind::Psbt if psbt_bytes(bytes).is_some() => Kind::Psbt,
        k if is_description(&k) => Kind::Description,
        _ => Kind::File,
    }
}

/// The id of the page a website guide's address names
/// (`https://faradayoffline.app/start/<id>`, `docs/STARTER.md` §7.4), when
/// `faraday-learn` or `faraday-guides` has that page.
pub fn guide(text: &str) -> Option<&'static str> {
    let rest = text.strip_prefix(faraday_learn::GUIDES)?;
    let id = rest.strip_suffix('/').unwrap_or(rest);
    faraday_learn::ids()
        .chain(faraday_guides::ids())
        .find(|i| *i == id)
}

/// Whether `text` is one web address.
fn is_web(text: &str) -> bool {
    let lower = |p: &str| {
        text.get(..p.len())
            .is_some_and(|t| t.eq_ignore_ascii_case(p))
    };
    (lower("https://") || lower("http://")) && !text.contains(char::is_whitespace)
}
