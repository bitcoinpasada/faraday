//! The scanner's state (UX.md §4 "Scan routing", §6 "scanner"): whether
//! the camera is on, the multi-part UR being collected, the unknown
//! payload waiting for a "treat as" choice, and the words waiting behind
//! the plain-text caution.
//!
//! Frames arrive for the preview alone: the shell reads the codes in
//! them and sends each one as `Event::Scanned`, and [`decode_frame`] is
//! the policy it reads them with, so that there is one definition of it
//! wherever a shell decodes. The payload is classified by
//! [`osk_codec::classify`] and the routing itself is in `lib.rs`
//! (`OpenSigner::route_payload`), because it changes screens.
//! A decoded SeedQR is a secret for the instant it exists here: the
//! bytes ride in a [`Zeroizing`] vector and become a [`LoadWizard`]
//! (fixed-size, zeroized on drop) before the frame is dropped.
//!
//! [`LoadWizard`]: crate::load::LoadWizard

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_codec::ur;
use osk_ui::tokens;
use zeroize::{Zeroize, Zeroizing};

use crate::load::LoadWizard;
use crate::pass_entry::PassEntry;

/// What the scanner was opened for: a hint on screen. Routing does not
/// depend on it (UX.md §4: one scanner, one routing table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    /// Home › Scan: anything.
    Any,
    /// Load › "Scan a SeedQR".
    Seed,
    /// Sign › "Scan QR".
    Psbt,
    /// Verify › "Scan address".
    Address,
    /// Add › "Scan a wallet": a BIP-388 policy, a multipath descriptor
    /// or a coordinator's multisig config.
    Wallet,
    /// The wallet builder › "Scan a key": one cosigner's account key,
    /// with the origin it states.
    Cosigner,
    /// Sign › "Sign a message": any text at all, since a message is
    /// whatever a person wants signed and no classifier knows it.
    Message,
    /// Verify › "Signed message": the three-line text form
    /// (`crate::message`).
    SignedMessage,
    /// Load › "Encrypted backup": an `osk-backup`, from a QR or a file.
    Backup,
    /// Tools › Notes › "Read a file": a plain text file, or an
    /// `osk-backup` holding a note (`docs/PLANNING.md` §16.112 rule 2).
    Note,
    /// Tools › "Decode a transaction": a PSBT the way Sign takes one, or
    /// a raw transaction in hex or in bytes.
    Transaction,
    /// Tools › "Compare transactions": one of the two PSBTs, read the
    /// way Sign takes one (`docs/PLANNING.md` §16.111 rule 4).
    CompareTransaction,
    /// Tools › Hashes: any text at all, since anything can be hashed.
    Hashes,
    /// Tools › Encodings: a string in one of the five encodings.
    Encodings,
    /// Tools › Descriptor checksum: a descriptor, with or without one.
    Descriptor,
    /// Tools › Convert key: an extended public key.
    ConvertKey,
    /// Tools › Miniscript: a concrete policy.
    Policy,
    /// A silent payments wallet › "Check a payment": a raw transaction
    /// or a PSBT (`docs/PLANNING.md` §16.113).
    SilentPayment,
    /// The same flow's "Previous transaction": the transaction that
    /// made the output an input spends, which is where a taproot
    /// input's public key is.
    SilentPrevious,
}

impl Expect {
    /// The calculator this scanner feeds, where it feeds one.
    pub fn tool(self) -> Option<crate::tools::Tool> {
        use crate::tools::Tool;
        match self {
            Expect::Hashes => Some(Tool::Hashes),
            Expect::Encodings => Some(Tool::Encodings),
            Expect::Descriptor => Some(Tool::Descriptor),
            Expect::ConvertKey => Some(Tool::ConvertKey),
            Expect::Policy => Some(Tool::Miniscript),
            _ => None,
        }
    }
}

/// The camera's latest frame, reduced for the viewfinder
/// (`docs/DESIGN.md` §4.9). A frame may picture a SeedQR, so the pixels
/// are wiped when the last holder lets go of them: this state when a
/// frame is replaced or the scanner closes, or the drawn tree that
/// shares the buffer rather than copying it every render. The chroma
/// the camera sent with the luma, where it sent any, is kept beside it
/// and follows the same rules.
pub struct Preview {
    width: u16,
    height: u16,
    pixels: Rc<Zeroizing<Vec<u8>>>,
    chroma: Option<Rc<Zeroizing<Vec<u8>>>>,
}

impl Preview {
    /// A preview of a frame already reduced to the size it is drawn at.
    pub fn new(width: u16, height: u16, pixels: Vec<u8>, chroma: Option<Vec<u8>>) -> Self {
        Preview {
            width,
            height,
            pixels: Rc::new(Zeroizing::new(pixels)),
            chroma: chroma.map(|uv| Rc::new(Zeroizing::new(uv))),
        }
    }

    /// Width in pixels.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Height in pixels.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// The pixels, 8-bit luma, row-major.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// The shared buffer, for the tree that draws it.
    pub fn shared(&self) -> Rc<Zeroizing<Vec<u8>>> {
        Rc::clone(&self.pixels)
    }

    /// The frame's NV12 chroma plane, shared the same way, or `None`
    /// where the camera gave no colour and the preview is grey.
    pub fn shared_chroma(&self) -> Option<Rc<Zeroizing<Vec<u8>>>> {
        self.chroma.as_ref().map(Rc::clone)
    }

    /// Whether the frame carries colour.
    pub fn has_colour(&self) -> bool {
        self.chroma.is_some()
    }
}

/// How far a frame is turned clockwise before anything reads it: the
/// camera on a device someone built is mounted whichever way its case
/// allows, and only the user can say which (`docs/DESIGN.md` §4.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CameraRotation {
    /// The frame arrives upright.
    #[default]
    Deg0,
    /// A quarter turn clockwise.
    Deg90,
    /// A half turn.
    Deg180,
    /// A quarter turn anticlockwise.
    Deg270,
}

/// The rotations the setting offers, in the order the Choice lists them.
pub const CAMERA_ROTATIONS: [CameraRotation; 4] = [
    CameraRotation::Deg0,
    CameraRotation::Deg90,
    CameraRotation::Deg180,
    CameraRotation::Deg270,
];

/// Turns a frame clockwise by `rotation`, returning the new size and the
/// pixels. A quarter turn either way swaps width and height. One pass
/// over the source, one write per pixel, and no buffer but the output.
pub fn rotate(
    width: usize,
    height: usize,
    luma: &[u8],
    rotation: CameraRotation,
) -> (u16, u16, Vec<u8>) {
    if rotation == CameraRotation::Deg0 || width == 0 || height == 0 || luma.len() < width * height
    {
        return (width as u16, height as u16, luma.to_vec());
    }
    let mut out = vec![0u8; width * height];
    for y in 0..height {
        let row = y * width;
        for x in 0..width {
            let at = match rotation {
                CameraRotation::Deg0 => row + x,
                CameraRotation::Deg90 => x * height + (height - 1 - y),
                CameraRotation::Deg180 => (height - 1 - y) * width + (width - 1 - x),
                CameraRotation::Deg270 => (width - 1 - x) * height + y,
            };
            out[at] = luma[row + x];
        }
    }
    let (w, h) = match rotation {
        CameraRotation::Deg90 | CameraRotation::Deg270 => (height, width),
        _ => (width, height),
    };
    (w as u16, h as u16, out)
}

/// The size of the NV12 chroma plane for a `width` by `height` frame:
/// one U and V pair per 2 x 2 block of luma, so `⌈width / 2⌉` pairs
/// across and `⌈height / 2⌉` down.
pub fn chroma_size(width: usize, height: usize) -> (usize, usize) {
    (width.div_ceil(2), height.div_ceil(2))
}

/// Turns a frame's chroma plane clockwise by `rotation`, the way
/// [`rotate`] turns its luma: the plane is `⌈width / 2⌉` pairs across
/// and each pair moves as a unit, so a quarter turn transposes it.
///
/// `width` and `height` are the luma's, before the turn. The result is
/// the plane for the turned luma.
pub fn rotate_chroma(
    width: usize,
    height: usize,
    chroma: &[u8],
    rotation: CameraRotation,
) -> Vec<u8> {
    let (cw, ch) = chroma_size(width, height);
    if rotation == CameraRotation::Deg0 || cw == 0 || ch == 0 || chroma.len() < cw * ch * 2 {
        return chroma.to_vec();
    }
    let mut out = vec![0u8; cw * ch * 2];
    for y in 0..ch {
        for x in 0..cw {
            let at = match rotation {
                CameraRotation::Deg0 => y * cw + x,
                CameraRotation::Deg90 => x * ch + (ch - 1 - y),
                CameraRotation::Deg180 => (ch - 1 - y) * cw + (cw - 1 - x),
                CameraRotation::Deg270 => (cw - 1 - x) * ch + y,
            };
            let (from, to) = ((y * cw + x) * 2, at * 2);
            out[to] = chroma[from];
            out[to + 1] = chroma[from + 1];
        }
    }
    out
}

/// Reduces a frame's chroma plane by `factor`, the factor [`reduce`]
/// shrank its luma by: the box average of every `factor × factor` block
/// of pairs, U and V averaged separately, so the reduced chroma still
/// carries one pair per 2 x 2 block of the reduced luma.
///
/// `width` and `height` are the luma's, before the reduction.
pub fn reduce_chroma(width: usize, height: usize, chroma: &[u8], factor: usize) -> Vec<u8> {
    let (cw, ch) = chroma_size(width, height);
    let factor = factor.max(1);
    if cw == 0 || ch == 0 || chroma.len() < cw * ch * 2 {
        return Vec::new();
    }
    if factor == 1 {
        return chroma[..cw * ch * 2].to_vec();
    }
    let (out_w, out_h) = (cw.div_ceil(factor), ch.div_ceil(factor));
    let mut out = vec![0u8; out_w * out_h * 2];
    for oy in 0..out_h {
        let rows = (oy * factor)..((oy + 1) * factor).min(ch);
        for ox in 0..out_w {
            let cols = (ox * factor)..((ox + 1) * factor).min(cw);
            let (mut u, mut v, mut n) = (0u32, 0u32, 0u32);
            for y in rows.clone() {
                for x in cols.clone() {
                    let at = (y * cw + x) * 2;
                    u += u32::from(chroma[at]);
                    v += u32::from(chroma[at + 1]);
                    n += 1;
                }
            }
            let at = (oy * out_w + ox) * 2;
            out[at] = (u / n.max(1)) as u8;
            out[at + 1] = (v / n.max(1)) as u8;
        }
    }
    out
}

/// The factor a frame `width` by `height` is reduced by so that its
/// shorter side is at most [`tokens::PREVIEW_MAX`] pixels.
pub fn reduce_factor(width: usize, height: usize) -> usize {
    width.min(height).div_ceil(tokens::PREVIEW_MAX).max(1)
}

/// Reduces a frame for the preview: the box average of every
/// `factor × factor` block, where the factor is [`reduce_factor`]. Each
/// source byte is read once and nothing is allocated but the output, so
/// a 640 × 480 frame ten times a second is affordable on a Pi.
///
/// A block at the right or bottom edge that runs off the frame averages
/// the part of it that exists.
pub fn reduce(width: usize, height: usize, luma: &[u8]) -> (u16, u16, Vec<u8>) {
    if width == 0 || height == 0 || luma.len() < width * height {
        return (0, 0, Vec::new());
    }
    let factor = reduce_factor(width, height);
    let (out_w, out_h) = (width.div_ceil(factor), height.div_ceil(factor));
    let mut out = vec![0u8; out_w * out_h];
    for oy in 0..out_h {
        let rows = (oy * factor)..((oy + 1) * factor).min(height);
        for ox in 0..out_w {
            let cols = (ox * factor)..((ox + 1) * factor).min(width);
            let mut sum = 0u32;
            let mut n = 0u32;
            for y in rows.clone() {
                for x in cols.clone() {
                    sum += u32::from(luma[y * width + x]);
                    n += 1;
                }
            }
            out[oy * out_w + ox] = (sum / n.max(1)) as u8;
        }
    }
    (out_w as u16, out_h as u16, out)
}

/// The first code in an 8-bit luma image, or `None` when there is none
/// to find. A shell decoding its own frames uses this, or
/// [`decode_frame`] for the whole policy.
///
/// A decoded code may be a SeedQR, so the bytes are a secret from the
/// moment they exist: the caller wipes them when it lets go of them.
pub fn decode_one(width: usize, height: usize, luma: &[u8]) -> Option<Vec<u8>> {
    osk_codec::decode_luma(width, height, luma)
        .into_iter()
        .next()
        .map(|d| d.bytes)
}

/// The first code in a camera frame, read the way the scanner reads
/// one: the reduced copy first, then the full frame.
///
/// The reduction is a box average of the frame ([`reduce`]), and a code
/// held up to the viewfinder is still several pixels a module in it:
/// reading that costs a quarter of the pixels. The full frame is the
/// fallback, for a code too small or too dense to survive the
/// reduction, and it is skipped where the frame did not reduce.
pub fn decode_frame(width: usize, height: usize, luma: &[u8]) -> Option<Vec<u8>> {
    let (rw, rh, reduced) = reduce(width, height, luma);
    if let Some(bytes) = decode_one(usize::from(rw), usize::from(rh), &reduced) {
        return Some(bytes);
    }
    if reduce_factor(width, height) > 1 {
        decode_one(width, height, luma)
    } else {
        None
    }
}

/// Where the scanner is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanStage {
    /// `CameraOn` was sent; frames are decoded as they come.
    Camera,
    /// The shell answered `CameraUnavailable`; the file row is offered.
    Unavailable,
    /// A file request is out.
    Waiting,
    /// A code was read but matched nothing: hex and the "treat as" rows.
    Unknown,
    /// Plain-text words were read: the caution before Load.
    WordsCaution,
    /// An encrypted backup was read: the passphrase that opens it.
    BackupPassphrase,
}

/// The scanner's state.
pub struct ScanState {
    expect: Expect,
    stage: ScanStage,
    camera_on: bool,
    ur: Option<ur::Decoder>,
    error: Option<String>,
    unknown: Vec<u8>,
    pending: Option<LoadWizard>,
    preview: Option<Preview>,
    /// An encrypted backup that was read, while its passphrase is being
    /// typed. Ciphertext, so it is a plain vector.
    backup: Vec<u8>,
    /// The passphrase being typed against it.
    pass: PassEntry,
    /// Whether the last try did not open the backup.
    wrong: bool,
    /// A line the viewfinder shows in place of its state for a moment,
    /// with the millisecond it stops at: "Nothing to paste".
    note: Option<(String, u64)>,
    /// Whether the payload being routed came off the clipboard, which
    /// is what a refusal names as its source.
    pasted: bool,
}

impl ScanState {
    /// A scanner opened for `expect`, before the camera answered.
    pub fn new(expect: Expect) -> Self {
        ScanState {
            expect,
            stage: ScanStage::Camera,
            camera_on: true,
            ur: None,
            error: None,
            unknown: Vec::new(),
            pending: None,
            preview: None,
            backup: Vec::new(),
            pass: PassEntry::new(),
            wrong: false,
            note: None,
            pasted: false,
        }
    }

    /// The payload about to be routed came off the clipboard.
    pub fn set_pasted(&mut self) {
        self.pasted = true;
    }

    /// Whether it did.
    pub fn pasted(&self) -> bool {
        self.pasted
    }

    /// Shows `text` inside the viewfinder until `until_ms`, in place of
    /// the state line.
    pub fn set_note(&mut self, text: String, until_ms: u64) {
        self.note = Some((text, until_ms));
    }

    /// That line while it lasts.
    pub fn note(&self, now_ms: u64) -> Option<&str> {
        self.note
            .as_ref()
            .filter(|(_, until)| now_ms < *until)
            .map(|(text, _)| text.as_str())
    }

    /// What the scanner was opened for.
    pub fn expect(&self) -> Expect {
        self.expect
    }

    /// Where the scanner is.
    pub fn stage(&self) -> ScanStage {
        self.stage
    }

    /// Whether `CameraOn` is outstanding (so leaving must send
    /// `CameraOff`).
    pub fn camera_on(&self) -> bool {
        self.camera_on
    }

    /// Why the last code could not be used, in the two or three words
    /// §4.11 gives a reason: "not a SeedQR", "unsupported type".
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Sets that reason, which takes the screen as a Result.
    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }

    /// The shell has no camera. There will be no frames, so the last
    /// preview goes with the same wipe a replaced one gets.
    pub fn unavailable(&mut self) {
        self.camera_on = false;
        self.stage = ScanStage::Unavailable;
        self.preview = None;
    }

    /// The camera's latest frame, reduced, or `None` before the first
    /// one arrives.
    pub fn preview(&self) -> Option<&Preview> {
        self.preview.as_ref()
    }

    /// Keeps `pixels` as the preview, wiping whatever it replaces.
    /// `chroma` is the frame's NV12 chroma plane where the camera gave
    /// one, and the preview is grey without it.
    pub fn set_preview(
        &mut self,
        width: u16,
        height: u16,
        pixels: Vec<u8>,
        chroma: Option<Vec<u8>>,
    ) {
        self.preview = Some(Preview {
            width,
            height,
            pixels: Rc::new(Zeroizing::new(pixels)),
            chroma: chroma.map(|uv| Rc::new(Zeroizing::new(uv))),
        });
    }

    /// A file request went out.
    pub fn request_file(&mut self) {
        self.stage = ScanStage::Waiting;
        self.error = None;
    }

    /// The shell has no file channel: back to whatever the scanner was
    /// doing before the request. The scan screen offers the file row
    /// only as a way past a camera it does not have, so there is
    /// nothing to report here (§2.1).
    pub fn file_unavailable(&mut self) {
        self.resume();
    }

    /// The shell answered `FileCancelled`: back to where the file was
    /// asked for, as a cancelled pick leaves every other screen.
    pub fn file_cancelled(&mut self) {
        self.resume();
    }

    /// Back to scanning (or the file row), keeping the camera as it was.
    pub fn resume(&mut self) {
        self.stage = if self.camera_on {
            ScanStage::Camera
        } else {
            ScanStage::Unavailable
        };
        self.unknown.clear();
        self.pasted = false;
        self.pending = None;
        self.clear_backup();
        self.error = None;
    }

    /// A code that matched nothing.
    pub fn set_unknown(&mut self, bytes: Vec<u8>) {
        self.unknown = bytes;
        self.stage = ScanStage::Unknown;
        self.error = None;
    }

    /// The unknown code's bytes.
    pub fn unknown(&self) -> &[u8] {
        &self.unknown
    }

    /// Takes the unknown code's bytes for a "treat as" choice.
    pub fn take_unknown(&mut self) -> Vec<u8> {
        core::mem::take(&mut self.unknown)
    }

    /// Plain-text words were read: hold the wizard behind the caution.
    pub fn set_pending_words(&mut self, wizard: LoadWizard) {
        self.pending = Some(wizard);
        self.stage = ScanStage::WordsCaution;
        self.error = None;
    }

    /// Takes the wizard waiting behind the caution.
    pub fn take_pending(&mut self) -> Option<LoadWizard> {
        self.pending.take()
    }

    /// How many words that wizard holds, for the caution's one row.
    pub fn pending_words(&self) -> Option<u8> {
        self.pending.as_ref().map(LoadWizard::count)
    }

    /// An encrypted backup was read: hold the bytes and ask for the
    /// passphrase that opens them.
    pub fn set_backup(&mut self, bytes: Vec<u8>) {
        self.backup = bytes;
        self.pass.zeroize();
        self.wrong = false;
        self.stage = ScanStage::BackupPassphrase;
        self.error = None;
    }

    /// The backup waiting for its passphrase.
    pub fn backup(&self) -> &[u8] {
        &self.backup
    }

    /// Characters typed against it.
    pub fn backup_pass_len(&self) -> usize {
        self.pass.len()
    }

    /// Whether ✓ is live.
    pub fn backup_pass_ready(&self) -> bool {
        self.pass.long_enough()
    }

    /// Whether the last try did not open the backup.
    pub fn backup_wrong(&self) -> bool {
        self.wrong
    }

    /// The last typed character while it is still unmasked at `now_ms`.
    pub fn backup_visible_char(&self, now_ms: u64) -> Option<char> {
        self.pass.visible_char(now_ms)
    }

    /// When that character masks, if one is showing.
    pub fn mask_deadline(&self) -> Option<u64> {
        self.pass.mask_deadline()
    }

    /// The passphrase itself, for the one call that stretches it.
    pub fn backup_pass(&self) -> &[u8] {
        self.pass.expose()
    }

    /// Types one character of it at time `now_ms`.
    pub fn backup_push(&mut self, c: char, now_ms: u64) {
        self.pass.push(c, now_ms);
        self.wrong = false;
    }

    /// Deletes the last one.
    pub fn backup_pop(&mut self) {
        self.pass.pop();
    }

    /// The passphrase did not open the backup: the entry stays and says
    /// so under the field.
    pub fn backup_wrong_pass(&mut self) {
        self.wrong = true;
    }

    /// Drops the backup and whatever was typed against it.
    pub fn clear_backup(&mut self) {
        self.backup.zeroize();
        self.backup.clear();
        self.pass.zeroize();
        self.wrong = false;
    }

    /// The multi-part UR decoder, created on the first part.
    pub fn ur(&mut self) -> &mut ur::Decoder {
        self.ur.get_or_insert_with(ur::Decoder::new)
    }

    /// Drops a half-collected UR (a part of another message arrived).
    pub fn reset_ur(&mut self) {
        self.ur = None;
    }

    /// `(fragments resolved, fragments in the message)` of the UR being
    /// collected.
    pub fn progress(&self) -> Option<(usize, usize)> {
        self.ur.as_ref().and_then(ur::Decoder::progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_factor_keeps_the_shorter_side_under_the_maximum() {
        assert_eq!(reduce_factor(640, 480), 2, "480 halves to 240");
        assert_eq!(reduce_factor(400, 400), 1, "already at the maximum");
        assert_eq!(reduce_factor(4096, 3072), 8);
        assert_eq!(reduce_factor(1, 1), 1);
    }

    #[test]
    fn reduce_box_averages_every_block() {
        // 4 x 4, values 0..16, reduced by 2: each output is the mean of
        // its 2 x 2 block.
        let luma: Vec<u8> = (0..16).collect();
        let (w, h, out) = reduce(4, 4, &luma);
        assert_eq!((w, h), (4, 4), "a small frame is left alone");
        assert_eq!(out, luma);

        // A frame whose shorter side is over the maximum reduces, and a
        // block that runs off the edge averages what exists.
        let wide = vec![10u8; 801 * 801];
        let (w, h, out) = reduce(801, 801, &wide);
        assert_eq!(reduce_factor(801, 801), 3);
        assert_eq!((w, h), (267, 267));
        assert!(out.iter().all(|&p| p == 10), "a flat frame stays flat");

        // The average itself, on a frame that does reduce: half the rows
        // black and half white gives the mid grey.
        let mut half = vec![0u8; 800 * 800];
        for (y, row) in half.chunks_exact_mut(800).enumerate() {
            if y % 2 == 1 {
                row.fill(200);
            }
        }
        let (_, _, out) = reduce(800, 800, &half);
        assert!(out.iter().all(|&p| p == 100), "0 and 200 average to 100");
    }

    #[test]
    fn rotation_turns_a_frame_clockwise() {
        // 3 x 2:
        //   1 2 3
        //   4 5 6
        let luma = [1u8, 2, 3, 4, 5, 6];
        assert_eq!(
            rotate(3, 2, &luma, CameraRotation::Deg0),
            (3, 2, luma.to_vec())
        );
        // A quarter turn clockwise, 2 x 3:
        //   4 1
        //   5 2
        //   6 3
        assert_eq!(
            rotate(3, 2, &luma, CameraRotation::Deg90),
            (2, 3, vec![4, 1, 5, 2, 6, 3])
        );
        assert_eq!(
            rotate(3, 2, &luma, CameraRotation::Deg180),
            (3, 2, vec![6, 5, 4, 3, 2, 1])
        );
        // A quarter turn anticlockwise, 2 x 3:
        //   3 6
        //   2 5
        //   1 4
        assert_eq!(
            rotate(3, 2, &luma, CameraRotation::Deg270),
            (2, 3, vec![3, 6, 2, 5, 1, 4])
        );
        // Four quarter turns are the frame again.
        let (w, h, mut px) = (3usize, 2usize, luma.to_vec());
        let (mut w, mut h) = (w, h);
        for _ in 0..4 {
            let (nw, nh, out) = rotate(w, h, &px, CameraRotation::Deg90);
            w = usize::from(nw);
            h = usize::from(nh);
            px = out;
        }
        assert_eq!((w, h, px), (3, 2, luma.to_vec()));
    }
}
