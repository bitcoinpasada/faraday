//! Signing a message and checking a signed one (UX.md F7 and G4;
//! `docs/PLANNING.md` §8.4 item 13).
//!
//! Two flows over one text form. Signing: a message arrives as text, by
//! QR or from a file; the person reads it whole, chooses the key, the
//! address it is signed for and the format, and holds. Checking: an
//! address, a signature and a message arrive as one text, and the device
//! says whether the signature holds and which address form it read.
//!
//! Nothing here is secret. The message and the signature are public, and
//! the key is borrowed for the moment [`osk_psbt::message::sign`] runs.
//!
//! Both flows exchange [`osk_psbt::message`]'s three-line text form, so
//! an air-gapped device needs no typing.

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;

use osk_bip::keys::ScriptType;
use osk_codec::qr::{Ecc, Payload, QrMatrix};
use osk_psbt::message::{Checked, Format, SignedText, signed_text};

use crate::sign::Save;

/// The name the shell is offered for a saved signature.
pub const SIGNATURE_FILE: &str = "signature.txt";

/// The formats a Choice offers, in order.
pub const FORMATS: [Format; 2] = [Format::Bip137, Format::Bip322];

/// Where the signing flow is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Typing the message, which is what the scanner's "Type" row
    /// opens (PLANNING §16.88).
    Typing,
    /// The message, whole, with the key, the address and the format
    /// above it.
    Message,
    /// The hold that signs.
    Confirm,
    /// The signature, with the ways out.
    Result,
    /// The signature as a QR, reached from the result.
    Qr,
}

/// What signing produced.
pub struct Outcome {
    /// The address the message was signed for.
    pub address: String,
    /// The signature, base64.
    pub signature: String,
    /// The format it is in.
    pub format: Format,
    /// What became of "Save to file".
    pub save: Save,
}

/// How many characters a typed message takes. A signed message is read
/// by a person, and a field longer than this is a file, not a message.
pub const MAX_MESSAGE: usize = 512;

/// The signing flow's state: the message, what it will be signed with,
/// and what came of it.
pub struct SignMessage {
    stage: Stage,
    text: String,
    key: usize,
    script: ScriptType,
    format: Format,
    outcome: Option<Outcome>,
    matrix: Option<Rc<QrMatrix>>,
}

impl SignMessage {
    /// A flow over `text`, with the first key and the script type a new
    /// export starts at.
    pub fn new(text: String) -> Self {
        SignMessage {
            stage: Stage::Message,
            text,
            key: 0,
            script: ScriptType::NativeSegwit,
            format: Format::Bip137,
            outcome: None,
            matrix: None,
        }
    }

    /// A flow over a message the person is about to type.
    pub fn typing() -> Self {
        let mut flow = Self::new(String::new());
        flow.stage = Stage::Typing;
        flow
    }

    /// Types one character of it. A message is whatever a person wants
    /// signed, so every printable character is taken.
    pub fn type_push(&mut self, c: char) {
        if self.text.chars().count() < MAX_MESSAGE {
            self.text.push(c);
        }
    }

    /// Deletes the last one.
    pub fn type_pop(&mut self) {
        self.text.pop();
    }

    /// Leaves the field for the message as it will be signed.
    pub fn typed(&mut self) {
        if !self.text.trim().is_empty() {
            self.stage = Stage::Message;
        }
    }

    /// Where the flow is.
    pub fn stage(&self) -> Stage {
        self.stage
    }

    /// The message, whole.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Which loaded key signs.
    pub fn key(&self) -> usize {
        self.key
    }

    /// The script type whose first receive address is signed for.
    pub fn script(&self) -> ScriptType {
        self.script
    }

    /// The format the signature will be in.
    pub fn format(&self) -> Format {
        self.format
    }

    /// What signing produced.
    pub fn outcome(&self) -> Option<&Outcome> {
        self.outcome.as_ref()
    }

    /// The QR of the signature, while the QR page is on.
    pub fn matrix(&self) -> Option<Rc<QrMatrix>> {
        self.matrix.clone()
    }

    /// Chooses the key.
    pub fn set_key(&mut self, key: usize) {
        self.key = key;
    }

    /// Chooses the script type, and with it the address. A type the
    /// current format has no form of takes the format its address can
    /// carry (`osk_psbt::message::Format::for_script`).
    pub fn set_script(&mut self, script: ScriptType) {
        self.script = script;
        let formats = Format::for_script(script);
        if !formats.contains(&self.format) {
            self.format = formats[0];
        }
    }

    /// Chooses the format, where the script type has more than one.
    pub fn set_format(&mut self, format: Format) {
        if Format::for_script(self.script).contains(&format) {
            self.format = format;
        }
    }

    /// Whether the format row is offered: a script type with one form
    /// states nothing by a Choice of one.
    pub fn format_offered(&self) -> bool {
        Format::for_script(self.script).len() > 1
    }

    /// On to the hold.
    pub fn confirm(&mut self) {
        self.stage = Stage::Confirm;
    }

    /// What the flow signed.
    pub fn signed(&mut self, outcome: Outcome) {
        self.outcome = Some(outcome);
        self.stage = Stage::Result;
    }

    /// The signature as one QR code. A message signature is short
    /// enough for one; where it is not, the page has nothing to show
    /// and the result stays.
    pub fn show_qr(&mut self) {
        let Some(out) = self.outcome.as_ref() else {
            return;
        };
        self.matrix = osk_codec::qr::encode(Payload::Bytes(out.signature.as_bytes()), Ecc::Low)
            .ok()
            .map(Rc::new);
        if self.matrix.is_some() {
            self.stage = Stage::Qr;
        }
    }

    /// The bytes "Save to file" hands the shell: the three-line form.
    pub fn file_bytes(&self) -> Option<Vec<u8>> {
        let out = self.outcome.as_ref()?;
        Some(signed_text(&out.address, &out.signature, &self.text).into_bytes())
    }

    /// A write is out.
    pub fn mark_saving(&mut self) {
        if let Some(out) = self.outcome.as_mut() {
            out.save = Save::Waiting;
        }
    }

    /// The shell stored the bytes, or did not.
    pub fn mark_saved(&mut self, written: bool) {
        if let Some(out) = self.outcome.as_mut() {
            out.save = if written { Save::Written } else { Save::Failed };
        }
    }

    /// Whether a save is waiting for the shell's answer.
    pub fn saving(&self) -> bool {
        self.outcome.as_ref().map(|o| o.save) == Some(Save::Waiting)
    }

    /// One step back inside the flow; `false` leaves the screen.
    pub fn back(&mut self) -> bool {
        match self.stage {
            Stage::Typing | Stage::Message => false,
            Stage::Confirm => {
                self.stage = Stage::Message;
                true
            }
            Stage::Result => {
                // The message and the signature are one screen apart,
                // and a signature is not unmade: back leaves.
                false
            }
            Stage::Qr => {
                self.stage = Stage::Result;
                true
            }
        }
    }
}

/// A signed message that arrived to be checked, and the answer.
pub struct CheckMessage {
    text: SignedText,
    checked: Option<Checked>,
    reading: bool,
}

impl CheckMessage {
    /// The answer for `text`: `checked` is what the signature holds
    /// for, `None` when it does not hold at all.
    pub fn new(text: SignedText, checked: Option<Checked>) -> Self {
        CheckMessage {
            text,
            checked,
            reading: false,
        }
    }

    /// The address the signature was checked against.
    pub fn address(&self) -> &str {
        &self.text.address
    }

    /// The signature.
    pub fn signature(&self) -> &str {
        &self.text.signature
    }

    /// The message.
    pub fn message(&self) -> &str {
        &self.text.message
    }

    /// The format and address form the signature holds for.
    pub fn checked(&self) -> Option<Checked> {
        self.checked
    }

    /// Whether the message itself is on screen.
    pub fn reading(&self) -> bool {
        self.reading
    }

    /// Opens the message.
    pub fn read(&mut self) {
        self.reading = true;
    }

    /// One step back; `false` leaves the screen.
    pub fn back(&mut self) -> bool {
        let reading = self.reading;
        self.reading = false;
        reading
    }
}
