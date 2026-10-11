//! §4.9 Codes: the one QR a screen shows, and the BC-UR run a payload
//! too dense for one is split into.
//!
//! "The module pitch floor (`QR_MIN_PITCH_MM`, 0.5 mm) is a hard floor at
//! the class's side. A payload whose one QR would fall below it is split
//! into as many BC-UR parts as keep every part above the floor: the part
//! count follows from the side and the floor, not from a fixed fragment
//! size."
//!
//! So the arithmetic runs in one direction. The display and the class's
//! square give a largest QR version ([`osk_ui::widgets::qr_max_version_at_pitch`]);
//! that version gives an alphanumeric capacity; the capacity gives the
//! fragment size; the fragment size gives the part count. Nothing here
//! names a number of parts or a number of bytes.
//!
//! Both the Sign result and the wallet export use it, because §4.9 says
//! "The wallet export animates on the panel like any other payload."

use alloc::rc::Rc;
use alloc::string::String;

use osk_codec::qr::{self, Ecc, Payload, QrMatrix};
use osk_codec::ur;

/// Milliseconds each part of an animated run stays on screen.
pub const UR_FRAME_MS: u64 = 250;

/// Largest fragment in bytes the search starts from. A fragment larger
/// than this needs a version no class's square reaches at the pitch
/// floor, so starting higher only costs a round of the search.
pub const UR_FRAGMENT_MAX: usize = 400;

/// Smallest fragment in bytes. Below this the per-part overhead — the
/// `ur:` header, the sequence, the checksum — is most of the code, and
/// the run grows without the parts getting easier to read.
pub const UR_FRAGMENT_MIN: usize = 20;

/// Fountain parts checked past the message's own fragments, because a
/// mixed part carries the same fragment length and a longer sequence
/// number.
const EXTRA_PARTS: usize = 2;

/// Rounds of the fragment search. Each one divides the length by the
/// ratio it overshot by, so two are usually enough; this is the stop.
const SEARCH_ROUNDS: usize = 8;

/// What a payload animates as: the registered UR type its parts carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrKind {
    /// `ur:crypto-psbt`, for a transaction.
    Psbt,
    /// `ur:bytes`, for anything else this app shows: a descriptor, an
    /// account key.
    Bytes,
}

/// The largest QR version that keeps the module pitch at or above
/// [`osk_ui::widgets::QR_MIN_PITCH_MM`] in a square of `side_px` on a
/// `dpi` display. Version 1 where the square is too small for even that,
/// so the caller always has a version to aim at.
pub fn max_version(side_px: i32, dpi: u16) -> u8 {
    osk_ui::widgets::qr_max_version_at_pitch(side_px, dpi).unwrap_or(1)
}

/// Whether `payload` fits one QR at `version` in byte mode: the test
/// §4.9 makes between one static code and an animated run.
pub fn fits_one_code(payload: &[u8], version: u8) -> bool {
    qr::encode_bounded(Payload::Bytes(payload), Ecc::Low, version).is_ok()
}

/// The fragment size in bytes at which every part of `payload`'s BC-UR
/// run encodes inside `version` — which is what keeps every part at or
/// above the pitch floor.
///
/// A part is a fixed header and two alphanumeric characters per byte, so
/// one measurement gives the ratio to shrink by; the loop is there for
/// the header's own growth as the sequence numbers get longer.
pub fn fragment_len(kind: UrKind, payload: &[u8], version: u8) -> usize {
    let capacity = qr::alnum_capacity(version, Ecc::Low);
    let mut len = UR_FRAGMENT_MAX.min(payload.len().max(UR_FRAGMENT_MIN));
    for _ in 0..SEARCH_ROUNDS {
        let worst = worst_part_chars(kind, payload, len);
        if worst <= capacity || len <= UR_FRAGMENT_MIN {
            break;
        }
        let scaled = len.saturating_mul(capacity) / worst.max(1);
        len = scaled.min(len - 1).max(UR_FRAGMENT_MIN);
    }
    len
}

/// The longest part, in characters, of the run `payload` makes at
/// `fragment_len`: every fragment of the message and a couple of the
/// fountain parts after them.
fn worst_part_chars(kind: UrKind, payload: &[u8], fragment_len: usize) -> usize {
    let Some(mut encoder) = encoder(kind, payload, fragment_len) else {
        return usize::MAX;
    };
    let parts = encoder.fragment_count() + EXTRA_PARTS;
    (0..parts)
        .map(|_| encoder.next_part().chars().count())
        .max()
        .unwrap_or(0)
}

/// A BC-UR encoder for `payload` with fragments of `fragment_len` bytes.
fn encoder(kind: UrKind, payload: &[u8], fragment_len: usize) -> Option<ur::Encoder> {
    match kind {
        UrKind::Psbt => ur::Encoder::psbt(payload, fragment_len),
        UrKind::Bytes => ur::Encoder::bytes(payload, fragment_len),
    }
    .ok()
}

/// An animated BC-UR run on a QR screen: the encoder, the code on
/// screen, the part number and when the next one is due (§4.9).
pub struct UrRun {
    encoder: ur::Encoder,
    matrix: Option<Rc<QrMatrix>>,
    part: usize,
    next_frame_ms: u64,
}

impl UrRun {
    /// A run of `payload`, split so that every part encodes inside
    /// `version` — the version the class's square and the pitch floor
    /// allow on this display.
    pub fn new(kind: UrKind, payload: &[u8], version: u8, now_ms: u64) -> Option<Self> {
        let encoder = encoder(kind, payload, fragment_len(kind, payload, version))?;
        let mut run = UrRun {
            encoder,
            matrix: None,
            part: 0,
            next_frame_ms: now_ms,
        };
        run.advance(now_ms);
        Some(run)
    }

    /// Encodes the next part.
    ///
    /// Bytewords are lowercase; upper-casing them lets the part go in
    /// alphanumeric mode, the densest a UR fits (BCR-2020-005 recommends
    /// it), which is what the fragment size was measured against.
    pub fn advance(&mut self, now_ms: u64) {
        let part = self.encoder.next_part().to_ascii_uppercase();
        self.matrix = qr::encode(Payload::Alphanumeric(part.as_bytes()), Ecc::Low)
            .ok()
            .map(Rc::new);
        self.part = self.encoder.emitted();
        self.next_frame_ms = now_ms + UR_FRAME_MS;
    }

    /// A tick at `now_ms`: advances when the frame is due. Returns
    /// whether the code on screen changed.
    pub fn tick(&mut self, now_ms: u64) -> bool {
        let due = now_ms >= self.next_frame_ms;
        if due {
            self.advance(now_ms);
        }
        due
    }

    /// The code on screen.
    pub fn matrix(&self) -> Option<Rc<QrMatrix>> {
        self.matrix.clone()
    }

    /// `(part shown, fragments in the message)`. The part number keeps
    /// counting past the fragment count: later parts are fountain mixes.
    pub fn part(&self) -> (usize, usize) {
        (self.part, self.encoder.fragment_count())
    }

    /// The whole run's parts, for a test that has to check every one of
    /// them: each part's text, in order, once round the message.
    pub fn message_parts(kind: UrKind, payload: &[u8], version: u8) -> alloc::vec::Vec<String> {
        let Some(mut encoder) = encoder(kind, payload, fragment_len(kind, payload, version)) else {
            return alloc::vec::Vec::new();
        };
        let n = encoder.fragment_count();
        (0..n)
            .map(|_| encoder.next_part().to_ascii_uppercase())
            .collect()
    }
}
