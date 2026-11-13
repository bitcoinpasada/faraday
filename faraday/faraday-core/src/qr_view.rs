//! The QR view (a code shown, in parts) and the camera's scan state.

use crate::file_stem;
use crate::strings::EN;
use crate::wallet::{Session, fp_text};
use crate::{Action, Faraday, Screen, Sheet, picture, qr_fits, secrets, transfer, wallet};
use faraday_bip::keys::Fingerprint;
use faraday_crypto::work::Compute;
use faraday_text::{Text, text};

/// What a scan pass saw of a code ([`StorageEvent::QrSeen`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seen {
    /// The frame's size.
    pub width: u16,
    /// The frame's height.
    pub height: u16,
    /// The code's corners in the frame.
    pub corners: [(u16, u16); 4],
    /// It read.
    pub read: bool,
}

/// Below this, in tenths of a pixel a module, a code is too fine for the
/// camera to read.
pub const TOO_FINE_TENTHS: u16 = 25;

/// Passes in a row that see a code too fine to read before the sheet
/// says so.
pub(crate) const TOO_FINE_PASSES: u32 = 3;

/// What the camera is reading codes for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScanPurpose {
    /// A file for Files: a PSBT, a descriptor, an xpub, a message.
    #[default]
    Files,
    /// A SeedQR for Add a key.
    Seed,
    /// One part of a key for Add a key's other forms.
    KeyPart,
    /// A TOTP setup code into a vault entry.
    VaultEntry,
    /// The hand-drawn copy of the seed with this fingerprint, on the
    /// backup's copy item: compared with it there, never loaded.
    CheckCopy(faraday_bip::keys::Fingerprint),
    /// The hand-drawn SeedQR (or, with `true`, CompactSeedQR) copy of the
    /// result on Convert a seed, for the key with this fingerprint:
    /// compared with its words there, never loaded
    /// (`docs/SEED-CONVERSION.md` §6).
    CheckConvert(faraday_bip::keys::Fingerprint, bool),
    /// The hand-drawn Codex32QR of the codex32 string open on the
    /// backup's codex32 item: compared with it there, never loaded
    /// (`docs/CODEX32.md` §9.5).
    CheckCodex32,
    /// The hand-drawn Krux code of seed `i`'s item on the backup's
    /// checklist: compared with the code made there, never loaded
    /// (`docs/KEF.md` §15.1).
    CheckKef(usize),
    /// Check a timestamp's block header: its 80 bytes or their hex, taken
    /// by the tool.
    OtsHeader,
    /// Check a timestamp's file hash: 64 hex characters, taken by the tool.
    OtsFileHash,
    /// Transfer's Receive: anything, saved by the shell into Downloads,
    /// never into the Inbox.
    Transfer,
    /// Any missing key of loaded wallet n, which Faraday works out from
    /// the seed read (`docs/history/SCAN-TO-SIGN.md` §2.1).
    WalletKey(usize),
    /// The Spend tab's What do you have?: anything in the envelope, one
    /// code after another, until Done (`docs/history/SCAN-TO-SIGN.md` §4.2).
    Intake,
    /// **Scan the public key** on Restore a GPG key: an armoured public
    /// key, filed in Files and restored from (`docs/GPG-PAPERKEY.md` §3).
    GpgPublic,
    /// **Scan the paperkey** at a restore's step 4: paperkey's base16
    /// lines or its raw form, every line checked into the restore set.
    GpgPaperkey,
    /// The Sweep sheet's: a private key, then the sweep's PSBT
    /// (`seeds::sweep`, F171).
    Sweep,
    /// Check a signer's **Scan** on the Sent card: a PSBT for that card,
    /// never filed.
    CheckSent,
    /// Check a signer's **Scan** on the Returned card.
    CheckReturned,
    /// Compare's **Scan** on the First card: a PSBT or a finished
    /// transaction for that card, never filed.
    CompareFirst,
    /// Compare's **Scan** on the Second card.
    CompareSecond,
    /// Sign a message's **Scan**: plain text for the message, never
    /// filed; any other kind is named and refused.
    MessageText,
}

/// What the camera sheet holds.
#[derive(Default)]
pub struct ScanState {
    /// The newest preview frame: width, height, luma. Wiped when it is
    /// replaced or dropped: a frame may picture a SeedQR.
    pub frame: Option<(u16, u16, zeroize::Zeroizing<Vec<u8>>)>,
    /// Its NV12 chroma plane, when the camera gives colour, wiped the same
    /// way.
    pub chroma: Option<zeroize::Zeroizing<Vec<u8>>>,
    /// What the scanner last saw of a code, and at which preview frame.
    pub seen: Option<(Seen, u32)>,
    /// Preview frames shown so far.
    pub frames: u32,
    /// Passes in a row that found a code too fine to read.
    pub too_fine: u32,
    /// A multi-part UR being put together.
    pub decoder: faraday_codec::ur::Decoder,
    /// The last thing worth saying about what was read.
    pub note: Option<Text>,
    /// Codes read so far.
    pub reads: usize,
    /// What the codes read are for.
    pub purpose: ScanPurpose,
    /// Where the Sample tile looks first among the samples it can give.
    pub sample: u8,
    /// The last single plain code's own bytes, untrimmed: what Sign a
    /// message signs. `None` for a code in parts or one that is not plain.
    pub(crate) single: Option<zeroize::Zeroizing<Vec<u8>>>,
    /// BBQr and numbered parts being put together.
    pub assembler: faraday_qr::Assembler,
    /// When the sheet began waiting for the camera's first frame
    /// (`now_ms` at the first tick it waited), until a frame comes.
    pub waiting_since: Option<u64>,
    /// Why the camera gives no picture, once that is known; cleared by
    /// the next frame.
    pub no_picture: Option<NoPicture>,
    /// While "Nothing to paste" is the state line: until when, in
    /// `now_ms`. Set by an empty paste, cleared by the tick past it.
    pub nothing_to_paste: Option<u64>,
    /// When a QR in view, large enough to read, was first seen and not
    /// read (`now_ms`): reset by a read, by a QR too fine, and by the QR
    /// leaving the view (`docs/QR.md` §8 item 8).
    pub unread_since: Option<u64>,
    /// What a scanner for anything was opened to read (`docs/SCANNING.md`
    /// §7.1): its title, and what it takes itself.
    pub ask: Ask,
    /// The read card shown in place of the picture, with the code it
    /// holds, while the camera is off (`docs/SCANNING.md` §5).
    pub card: Option<crate::scan_read::ReadCard>,
}

/// What a scanner for anything ([`ScanPurpose::Files`]) was opened to
/// read, by its opener (`docs/SCANNING.md` §7.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ask {
    /// Anything: Home's Scan, Files' and Tools' Scan a QR code.
    #[default]
    Any,
    /// A wallet's description: Restore's Scan it and Scan descriptor.
    Description,
    /// Key n's xpub (from 1), for Create's key slot.
    Xpub(u8),
    /// A cosigner's key, for Restore's seeds card.
    Cosigner,
    /// The PSBT of the spend under way: Spend's Scan a copy, the Spend
    /// tab's Scan it and Scan the PSBT.
    SpendPsbt,
    /// A PSBT to sign: Tools' Sign a transaction.
    Sign,
    /// A transaction to decode: Tools' Decode a transaction.
    Decode,
    /// An address to check: Tools' Check an address.
    Address,
    /// A Lightning invoice to decode: Decode a Lightning invoice's Scan.
    Invoice,
}

impl Ask {
    /// Whether the scanner takes every kind as Home's Scan does, titled
    /// for what it asks (Tools' Sign, Decode, Check an address), rather
    /// than handing over what its flow does not take.
    pub(crate) fn takes_all(self) -> bool {
        matches!(
            self,
            Ask::Any | Ask::Sign | Ask::Decode | Ask::Address | Ask::Invoice
        )
    }
}

impl ScanState {
    /// Whether a transfer in parts is part-way through.
    pub(crate) fn in_parts(&self) -> bool {
        self.assembler.busy() || self.decoder.progress().is_some()
    }

    /// The share of a transfer in parts read so far, for the bar inside
    /// the viewfinder; `None` when no transfer in parts is under way.
    pub fn parts_read(&self) -> Option<f32> {
        let (have, of) = self
            .decoder
            .progress()
            .or_else(|| self.assembler.progress())?;
        (of > 0).then(|| have as f32 / of as f32)
    }

    /// The state line inside the viewfinder: a passing line first, then
    /// why the camera gives no picture, then the last thing said, then
    /// what the camera is doing.
    pub fn state_line(&self) -> &str {
        if self.nothing_to_paste.is_some() {
            return EN.pasted_nothing;
        }
        if let Some(why) = self.no_picture {
            return why.line();
        }
        match (&self.note, &self.frame) {
            (Some(note), _) => note,
            (None, None) => EN.scan_starting_the_camera,
            (None, Some(_)) => EN.scan_hold_the_code,
        }
    }
}

/// Why the scanner shows no picture (`docs/QR.md` §8 item 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoPicture {
    /// The camera started and no frame came in [`faraday_draw::tokens::NO_PICTURE_MS`].
    Silent,
    /// The shell answered `CameraUnavailable`, which says no more.
    Unavailable,
    /// The shell says the camera was refused.
    Refused,
    /// The shell says another program holds the camera.
    InUse,
    /// The shell says there is no camera.
    NotFound,
}

impl NoPicture {
    /// The line the viewfinder shows.
    pub fn line(self) -> &'static str {
        match self {
            NoPicture::Silent => EN.scan_no_picture,
            NoPicture::Unavailable => EN.event_no_camera,
            NoPicture::Refused => EN.scan_camera_refused,
            NoPicture::InUse => EN.scan_camera_in_use,
            NoPicture::NotFound => EN.scan_no_camera_found,
        }
    }
}

/// Preview frames a QR the scanner found stays outlined, and counts as
/// still in view, after the scanner last saw it.
pub const SEEN_FRAMES: u32 = 10;

/// What one UR part did to the decoder ([`ur_feed`]).
pub(crate) enum UrFed {
    /// The message is whole; the decoder is left empty.
    Whole(faraday_codec::ur::Message),
    /// More parts are wanted. `restarted` when the part began another
    /// message, which replaced the one being read.
    More {
        /// The part began another message.
        restarted: bool,
    },
}

/// One UR part into `decoder` (`docs/QR.md` §8 item 1). A part of
/// another message starts that message in an empty decoder, which takes
/// the old one's place; one no decoder reads is refused and leaves the
/// message being read as it was. A message once whole is taken out, so
/// the decoder is empty for the next. The refusal is the line to show.
pub(crate) fn ur_feed(
    decoder: &mut faraday_codec::ur::Decoder,
    text: &str,
) -> Result<UrFed, &'static str> {
    let restarted = match decoder.receive(text) {
        Ok(_) => false,
        Err(faraday_codec::ur::Error::Mismatch) => {
            let mut fresh = faraday_codec::ur::Decoder::new();
            if fresh.receive(text).is_err() {
                return Err(EN.ur_feed_not_a_part);
            }
            *decoder = fresh;
            true
        }
        Err(_) => return Err(EN.ur_feed_not_a_part),
    };
    if decoder.is_complete() {
        return std::mem::take(decoder)
            .into_message()
            .map(UrFed::Whole)
            .ok_or(EN.ur_feed_not_a_part);
    }
    Ok(UrFed::More { restarted })
}

/// The state line for a UR read in parts: that another message started,
/// or how far this one is and which parts it still wants.
pub(crate) fn ur_note(decoder: &faraday_codec::ur::Decoder, restarted: bool) -> Option<Text> {
    if restarted {
        return Some(Text::of(EN.ur_feed_new_qr));
    }
    let (have, of) = decoder.progress()?;
    Some(part_note("UR", have, of, &decoder.missing()))
}

impl Drop for ScanState {
    fn drop(&mut self) {
        // A frame may hold a picture of a SeedQR: wiped here, as each
        // frame replaced is by its own guard.
        if let Some((_, _, luma)) = self.frame.as_mut() {
            zeroize::Zeroize::zeroize(&mut **luma);
        }
        if let Some(uv) = self.chroma.as_mut() {
            zeroize::Zeroize::zeroize(&mut **uv);
        }
    }
}

/// What a QR sheet shows.
pub struct QrView {
    /// What the code holds.
    pub title: Text,
    /// One line under the title: the format, and the frame count.
    pub subtitle: Text,
    /// The codes, cycled when there is more than one.
    pub frames: Vec<faraday_codec::qr::QrMatrix>,
    /// The frame shown.
    pub frame: usize,
    /// When the next frame is due.
    pub(crate) next_ms: u64,
    /// What is shown, kept so the format and size can change.
    pub source: QrSource,
    /// UR or BBQr.
    pub format: QrFormat,
    /// Bytes of data a frame, when it is animated.
    pub part: usize,
    /// A secret: whoever scans it can use it.
    pub secret: bool,
    /// Public content worth keeping as a picture, as the caller says (a
    /// descriptor, a key, a signed message; not a PSBT or a transaction,
    /// which are handed over as files or in parts): while the view shows
    /// one code, the sheet offers it as a labelled PNG.
    pub public: bool,
    /// The lines under the title in that picture, one statement each.
    pub label: Vec<Text>,
    /// The picture's file name.
    pub png_name: Text,
    /// The text the code holds, shown whole under it: a wallet's
    /// descriptor, so it can be read against the code.
    pub whole: Option<Text>,
    /// One static code with no choice of format, speed or size (a GPG
    /// key's paperkey, `docs/GPG-PAPERKEY.md` §12).
    pub one_code: bool,
}

/// A code on screen may be a seed: what it was made from, the codes
/// (each `QrMatrix` wipes its modules) and the text shown under them are
/// wiped when the sheet closes.
impl Drop for QrView {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        match &mut self.source {
            QrSource::Psbt(b) => b.zeroize(),
            QrSource::Text(t) => t.zeroize(),
            QrSource::Key(k, a) => {
                k.zeroize();
                a.zeroize();
            }
            QrSource::File(n, b) => {
                n.zeroize();
                b.zeroize();
            }
        }
        self.title.zeroize();
        self.subtitle.zeroize();
        for m in &mut self.frames {
            m.zeroize();
        }
        self.frames.clear();
        if let Some(w) = &mut self.whole {
            w.zeroize();
        }
    }
}

/// What a QR view shows: `faraday-qr`'s sender.
pub use faraday_qr::send::Source as QrSource;

/// A public file a flow makes that is offered as a code and as a
/// labelled picture of it, beside the file itself (`docs/QR.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// A wallet's descriptor.
    Descriptor(usize),
    /// A wallet's multisig config, as Coldcard, Keystone and Passport
    /// scan it.
    MultisigConfig(usize),
    /// A wallet's BIP 129 descriptor record.
    Bsms(usize),
    /// A wallet as Specter DIY's `addwallet` command, in Specter's
    /// numbered parts (`docs/INTEROP.md` §5, Specter DIY addwallet).
    SpecterDiy(usize),
    /// The account key of Create's slot n, for the cosigners.
    Key(u8),
    /// That key's signed BIP 129 key record.
    KeyBsms(u8),
    /// The account key in wallet n's slot k, held here, as the wallet's
    /// descriptor writes it.
    WalletKey(usize, u8),
    /// That key's signed BIP 129 key record.
    WalletKeyBsms(usize, u8),
    /// The account key in wallet n's slot k, held elsewhere, as the
    /// wallet's descriptor writes it (`docs/history/NEW-WALLET.md` §9.3).
    CosignerKey(usize, u8),
    /// The silent payments address on show: as it is, or as a
    /// `bitcoin:` link.
    Silent(bool),
    /// The silent payments record, `silent-{fp}.txt`.
    SilentRecord,
    /// The silent payments address's BIP-353 DNS record, `silent-{fp}-dns`.
    SilentDns,
    /// The signed message.
    Message,
    /// The selected GPG key's public certificate.
    GpgKey,
    /// A detached signature by the selected GPG key over the Inbox file
    /// whose SHA-256 begins with these eight bytes: it follows the file,
    /// not its place in the Inbox.
    GpgSignature([u8; 8]),
    /// Wallet n's receive address at index i, as the screen prints it
    /// (`docs/QR.md` §7): no picture.
    Receive(usize, u32),
    /// Vanity address's found address: no picture.
    VanityAddress,
    /// Explore's address at its path: no picture.
    ExploreAddress,
    /// Explore's public key at its path: no picture.
    ExplorePublicKey,
    /// Explore's extended public key at its path: no picture.
    ExploreXpub,
    /// The Lightning node id on show, and its picture.
    NodeId,
    /// The selected recovery sheet's descriptor in the open vault, and
    /// its picture.
    VaultSheet,
}

/// How a QR view is written: UR, BBQr or numbered parts.
pub use faraday_qr::send::Format as QrFormat;

/// The part sizes offered, in bytes of data a frame.
pub use faraday_qr::send::PARTS as QR_PARTS;

/// The frame rates offered, as milliseconds a frame: 2, 3 and 5 a
/// second (`docs/QR.md`: 2 by default).
pub use faraday_qr::send::SPEEDS as QR_SPEEDS;

/// The line under the codes the sender made: their format, and how many
/// frames.
fn made_line(made: faraday_qr::send::Made) -> Text {
    use faraday_qr::send::Made;
    let n = |k: usize| Text::num(k as u64);
    match made {
        Made::UrPsbtOne => Text::of(EN.of_ur_crypto_psbt_one),
        Made::UrPsbtFrames(k) => Text::fill(EN.of_ur_crypto_psbt_frames, &[&n(k)]),
        Made::TextOne => Text::of(EN.of_text_one_code),
        Made::KeyOne => Text::of(EN.of_key_one_code),
        Made::UrAccount => Text::of(EN.of_ur_crypto_account),
        Made::UrBytesOne { file } => Text::fill(EN.ur_bytes_frames_one, &[ur_bytes_what(file)]),
        Made::UrBytesFrames { file, frames } => Text::fill(
            EN.ur_bytes_frames_frames,
            &[ur_bytes_what(file), &n(frames)],
        ),
        Made::BbqrOne(t) => Text::fill(EN.bbqr_frames_bbqr_one, &[&text!("{t}")]),
        Made::BbqrFrames(t, k) => Text::fill(EN.bbqr_frames_bbqr_frames, &[&text!("{t}"), &n(k)]),
        Made::PartsOne => Text::of(EN.parts_frames_text_one),
        Made::PartsNumbered(k) => Text::fill(EN.parts_frames_numbered, &[&n(k)]),
    }
}

/// What `ur:bytes` carries, as the line names it.
fn ur_bytes_what(file: bool) -> &'static str {
    if file {
        EN.of_file_ur_bytes
    } else {
        "ur:bytes"
    }
}

/// Why the sender made no codes, as the sheet says it.
pub(crate) fn send_error(e: faraday_qr::send::Error) -> Text {
    use faraday_qr::send::Error;
    match e {
        Error::TooLargeForBbqr => Text::of(EN.bbqr_frames_too_large),
        Error::Envelope(why) => Text::of(envelope_text(why)),
        Error::Code => Text::of(EN.qr_send_code_failed),
    }
}

/// Why a UR registry type was refused, as the line under the scanner
/// says it.
pub(crate) fn registry_text(e: &faraday_qr::registry::Error) -> Text {
    use faraday_qr::registry::Error;
    match e {
        Error::Shape(ur_type) => Text::fill(EN.ur_registry_shape, &[ur_type]),
        Error::Psbt => Text::fill(EN.ur_registry_shape, &["PSBT"]),
        Error::Seed => Text::of(EN.ur_registry_seed),
        Error::Path => Text::of(EN.ur_registry_path),
        Error::Key => Text::of(EN.ur_registry_key),
        Error::Private => Text::of(EN.ur_registry_private_key),
        Error::NoChainCode => Text::of(EN.ur_registry_no_chain_code),
        Error::Wallet => Text::of(EN.ur_registry_wallet),
        Error::NotExtended => Text::of(EN.ur_registry_not_extended),
        Error::NoThreshold => Text::of(EN.ur_registry_no_threshold),
        Error::NoMultisigKeys => Text::of(EN.ur_registry_no_multisig_keys),
        Error::Script(t) => Text::fill(EN.ur_registry_script, &[&text!("{t}")]),
        Error::NoFingerprint => Text::of(EN.ur_registry_no_fingerprint),
        Error::NoAccountKeys => Text::of(EN.ur_registry_no_account_keys),
        Error::NoKeyRead => Text::of(EN.ur_registry_no_key_read),
    }
}

/// Why a Faraday file envelope was refused, in a few words.
fn envelope_text(e: faraday_qr::envelope::Error) -> &'static str {
    use faraday_qr::envelope::Error;
    match e {
        Error::Size => EN.envelope_size,
        Error::Kind => EN.envelope_kind,
        Error::Name => EN.envelope_name,
        Error::TooLarge => EN.envelope_too_large,
        Error::NotEnvelope => EN.envelope_not,
        Error::NoName => EN.envelope_no_name,
        Error::NoKind => EN.envelope_no_kind,
        Error::NoData => EN.envelope_no_data,
        Error::NotBase64 => EN.envelope_not_base64,
        Error::Mismatch => EN.envelope_mismatch,
    }
}

/// What a part's line calls a transfer of numbered parts or BBQr.
pub(crate) fn transfer_name(what: faraday_qr::Transfer) -> &'static str {
    match what {
        faraday_qr::Transfer::Bbqr => "BBQr",
        faraday_qr::Transfer::Numbered => EN.part_note_numbered,
    }
}

/// Why the scanner refused a code (`faraday_qr::Assembler`), as the
/// line under it says it.
pub(crate) fn scan_refusal(r: faraday_qr::Refusal) -> Text {
    use faraday_qr::Refusal;
    use faraday_qr::bbqr::{Error as Bbqr, Malformed};
    use faraday_qr::specter::Fault;
    let malformed = |m: Malformed| match m {
        Malformed::Short => EN.scan_bbqr_malformed_short,
        Malformed::Encoding => EN.scan_bbqr_malformed_encoding,
        Malformed::Count => EN.scan_bbqr_malformed_count,
        Malformed::Number => EN.scan_bbqr_malformed_number,
        Malformed::Data => EN.scan_bbqr_malformed_data,
        Malformed::Uneven => EN.scan_bbqr_malformed_uneven,
        Malformed::Inflate => EN.scan_bbqr_malformed_inflate,
    };
    match r {
        Refusal::TooLong => Text::of(EN.scan_refused_too_long),
        Refusal::Bbqr(Bbqr::Malformed(m)) => {
            Text::fill(EN.scan_refused_bbqr_malformed, &[malformed(m)])
        }
        Refusal::Bbqr(Bbqr::Type(t)) => Text::fill(EN.scan_refused_bbqr_type, &[&text!("{t}")]),
        Refusal::Bbqr(Bbqr::OtherTransfer) => Text::of(EN.scan_refused_bbqr_other),
        Refusal::Bbqr(Bbqr::Conflict) => Text::of(EN.scan_refused_bbqr_conflict),
        Refusal::Bbqr(Bbqr::TooLarge) => Text::of(EN.scan_refused_bbqr_large),
        Refusal::Numbered(Fault::OtherSet) => Text::of(EN.scan_refused_numbered_other),
        Refusal::Numbered(Fault::Conflict) => Text::of(EN.scan_refused_numbered_conflict),
        Refusal::Numbered(Fault::TooLarge) => Text::of(EN.scan_refused_numbered_large),
        Refusal::BbqrText => Text::of(EN.scan_refused_bbqr_text),
        Refusal::BbqrJson => Text::of(EN.scan_refused_bbqr_json),
        Refusal::Envelope(e) => Text::of(envelope_text(e)),
    }
}

/// The line for a code too fine for the camera: a paperkey has no smaller
/// parts or file to carry it as (`docs/GPG-PAPERKEY.md` §12.4).
pub(crate) fn too_fine_line(purpose: &ScanPurpose) -> &'static str {
    if *purpose == ScanPurpose::GpgPaperkey {
        EN.gpg_restore_too_fine
    } else {
        EN.storage_event_too_fine
    }
}

impl QrView {
    /// Costs what `secret` costs. A secret as one static code with no
    /// choice of format, speed or size, the sheet marked Secret (a GPG
    /// key's paperkey, `docs/GPG-PAPERKEY.md` §12).
    pub(crate) fn secret_one_code(c: &Compute, title: &str, text: &str) -> Result<QrView, Text> {
        QrView::secret(c, title, text).map(|mut v| {
            v.one_code = true;
            v
        })
    }

    /// Costs what `of` costs. A PSBT as `ur:crypto-psbt`: one code when it
    /// fits, else animated.
    pub fn psbt(c: &Compute, title: &str, bytes: &[u8]) -> Result<QrView, Text> {
        QrView::of(
            c,
            title,
            QrSource::Psbt(bytes.to_vec()),
            QrFormat::Ur,
            QR_PARTS[1],
        )
    }

    /// Costs what `of` costs. Text as one code while it fits version 25 at
    /// medium error correction (the rule for a descriptor), and as animated
    /// `ur:bytes` when it does not.
    pub fn text(c: &Compute, title: &str, text: &str) -> Result<QrView, Text> {
        QrView::of(
            c,
            title,
            QrSource::Text(faraday_text::Text::of(text)),
            QrFormat::Ur,
            QR_PARTS[1],
        )
    }

    /// Costs a QR encoding. A Krux encrypted code's base43 `text` as one
    /// static code in alphanumeric mode, at low error correction when
    /// `low` and medium otherwise (`docs/KEF.md` §5), public (`docs/QR.md`
    /// §7.1): the picture is named `png_name` and has `lines` under it.
    /// An error when it does not fit one code.
    pub fn kef<L: AsRef<str>>(
        c: &Compute,
        text: &str,
        low: bool,
        png_name: &str,
        lines: Vec<L>,
    ) -> Result<QrView, Text> {
        use faraday_codec::qr::{Ecc, Payload, encode_bounded};
        /// The largest code written: a descriptor's (`docs/QR.md`).
        const LARGEST: u8 = 25;
        let ecc = if low { Ecc::Low } else { Ecc::Medium };
        let code = encode_bounded(c, Payload::Alphanumeric(text.as_bytes()), ecc, LARGEST)
            .map_err(|_| Text::of(EN.kef_code_too_big))?;
        Ok(QrView {
            title: Text::of(EN.kef_code_title),
            subtitle: Text::of(EN.kef_code_subtitle),
            frames: vec![code],
            frame: 0,
            next_ms: 0,
            source: QrSource::Text(faraday_text::Text::of(text)),
            format: QrFormat::Ur,
            part: QR_PARTS[1],
            secret: false,
            public: true,
            label: texts(&lines),
            png_name: Text::of(png_name),
            whole: Some(Text::of(text)),
            one_code: true,
        })
    }

    /// Costs what `text` costs. A secret as one plain-text code, the sheet
    /// marked Secret: it offers no picture (an entry's TOTP secret,
    /// `docs/QR.md` §7.1).
    pub fn secret(c: &Compute, title: &str, text: &str) -> Result<QrView, Text> {
        QrView::text(c, title, text).map(|mut v| {
            v.secret = true;
            v
        })
    }

    /// Costs what `frames` and `classify` cost. `source` written as
    /// `format`, `part` bytes a frame when animated.
    pub fn of(
        c: &Compute,
        title: &str,
        source: QrSource,
        format: QrFormat,
        part: usize,
    ) -> Result<QrView, Text> {
        let (made, frames) =
            faraday_qr::send::frames(c, &source, format, part).map_err(send_error)?;
        let sub = made_line(made);
        let secret = match &source {
            QrSource::File(name, data) => {
                wallet::classify(c, name, data).exposure() == secrets::Exposure::Secret
            }
            _ => false,
        };
        Ok(QrView {
            title: Text::of(title),
            subtitle: sub,
            frames,
            frame: 0,
            next_ms: 0,
            source,
            format,
            part,
            secret,
            public: false,
            label: Vec::new(),
            png_name: Text::new(),
            whole: None,
            one_code: false,
        })
    }

    /// Costs what `of` costs. The same source in another format or part
    /// size.
    pub fn redo(&self, c: &Compute, format: QrFormat, part: usize) -> Result<QrView, Text> {
        let secret = self.secret;
        QrView::of(c, &self.title, self.source.clone(), format, part).map(|mut v| {
            v.secret |= secret;
            v.public = self.public;
            v.label = self.label.clone();
            v.png_name = self.png_name.clone();
            v.whole = self.whole.clone();
            v.one_code = self.one_code;
            v
        })
    }

    /// The view as public content a picture may keep: the file `name`,
    /// the title and then `lines` under the code.
    pub fn public<L: AsRef<str>>(mut self, name: &str, lines: Vec<L>) -> QrView {
        self.public = true;
        self.png_name = Text::of(name);
        self.label = texts(&lines);
        self
    }

    /// Whether the sheet offers the code as a PNG: public, not a secret,
    /// and one static code.
    pub fn offers_png(&self) -> bool {
        self.public && !self.secret && self.frames.len() == 1 && !self.png_name.is_empty()
    }

    /// Costs what `labelled_png` costs. The code on show as a labelled
    /// picture, its file's name and bytes, when it offers one.
    pub fn png(&self, c: &Compute) -> Option<(Text, Vec<u8>)> {
        if !self.offers_png() {
            return None;
        }
        let code = self.frames.first()?;
        Some((
            self.png_name.clone(),
            picture::labelled_png(c, code, &self.title, &self.label),
        ))
    }
}

/// The QR sheet's area: the code on show, while the sheet is open.
#[derive(Default)]
pub(crate) struct QrArea {
    view: Option<QrView>,
    /// The held press that opened the view (a TOTP secret's or a
    /// paperkey's code): a device arriving takes the view off the sheet
    /// and that press waits under the device check.
    held_press: Option<crate::vault::VaultAction>,
}

impl QrArea {
    /// The view on show.
    pub(crate) fn view(&self) -> Option<&QrView> {
        self.view.as_ref()
    }

    /// Shows `view` on the QR sheet.
    pub(crate) fn open(&mut self, view: QrView, sheet: &mut Option<Sheet>) {
        self.view = Some(view);
        self.held_press = None;
        *sheet = Some(Sheet::Qr);
    }

    /// The view on show was opened by held press `a`.
    pub(crate) fn opened_by(&mut self, a: crate::vault::VaultAction) {
        self.held_press = Some(a);
    }

    /// The held press that opened the view on show, if it was one.
    pub(crate) fn held_press(&self) -> Option<crate::vault::VaultAction> {
        self.held_press.filter(|_| self.view.is_some())
    }

    /// Drops the view (its source is wiped).
    pub(crate) fn close(&mut self) {
        self.view = None;
    }

    /// Takes the view out, to work on it beside the rest of the app.
    pub(crate) fn take(&mut self) -> Option<QrView> {
        self.view.take()
    }

    /// Puts back a view taken out; the sheet is as it was.
    pub(crate) fn put(&mut self, view: QrView) {
        self.view = Some(view);
    }

    /// Moves an animated view to its next frame when one is due. Whether
    /// the frame changed.
    pub(crate) fn tick(&mut self, now_ms: u64, frame_ms: u64) -> bool {
        if let Some(q) = self.view.as_mut()
            && q.frames.len() > 1
            && now_ms >= q.next_ms
        {
            q.frame = (q.frame + 1) % q.frames.len();
            q.next_ms = now_ms + frame_ms;
            return true;
        }
        false
    }
}

impl Faraday {
    /// Costs a QR code made per part. The QR view of Specter DIY's
    /// `addwallet` command for `w`, in Specter's numbered parts at the
    /// middle part size, its whole text under it.
    pub(crate) fn specter_diy_view(&self, c: &Compute, w: &wallet::Wallet) -> Result<QrView, Why> {
        let text = faraday_intake::specter_diy::command(&w.name, w.policy())
            .ok_or(EN.code_view_this_wallet_has)?;
        let title = Text::fill(EN.code_view_specter_diy, &[&text!("{}", w.name)]);
        let name = text!("{}-specter-diy.png", crate::file_stem(&w.name));
        let mut lines = texts(&self.wallet_label(w));
        lines.push(Text::of(EN.code_view_public_watch));
        let mut view = QrView::of(
            c,
            &title,
            QrSource::Text(faraday_text::Text::of(&text)),
            QrFormat::Parts,
            QR_PARTS[1],
        )?
        .public(&name, lines);
        view.whole = Some(Text::of(&text));
        Ok(view)
    }

    /// The code on show on the QR sheet, while it is open.
    pub fn qr(&self) -> Option<&QrView> {
        self.qr.view()
    }

    /// Costs what the actions below cost, the keys' and wallets' work among
    /// it: handles the QR view's and the PDF buttons' actions (`Qr*`, `ShowCode`, `CodePng`, `Pdf*`).
    pub(crate) fn qr_act(&mut self, action: Action, c: &Compute) {
        match action {
            Action::PdfInbox(i) => {
                if let Some(item) = self.inbox.get(i) {
                    let (name, bytes) = (item.name.clone(), item.bytes.clone());
                    self.make_pdf(&name, &bytes);
                }
            }
            Action::PdfOutbox(i) => {
                if let Some(item) = self.outbox.get(i) {
                    let (name, bytes) = (item.name.clone(), item.bytes.clone());
                    self.make_pdf(&name, &bytes);
                }
            }
            Action::QrSigned | Action::QrPart => {
                if let Some(s) = self.wallets.spend().filter(|s| !s.spend.nonce_hold) {
                    let title = if action == Action::QrSigned {
                        EN.act_signed_psbt
                    } else {
                        EN.act_psbt_for_the_next
                    };
                    match s.spend.out_bytes() {
                        Ok(bytes) => self.open_qr(QrView::psbt(c, title, &bytes)),
                        Err(_) => self.toast(EN.spend_v2_not_written),
                    }
                }
            }
            Action::QrWallet(i) => {
                if self.screen == Screen::Backup && self.backup.shown(i) {
                    self.backups_shown();
                }
                if i < self.session.wallets.len() {
                    let view = self.code_view(c, Code::Descriptor(i));
                    self.open_qr(view);
                }
            }
            Action::ShowCode(c) | Action::CodePng(c)
                if matches!(c, Code::GpgSignature(_)) && self.time_asked() => {}
            Action::ShowCode(code) => {
                let view = self.code_view(c, code);
                self.open_qr(view);
            }
            Action::CodePng(code) => {
                let view = self.code_view(c, code);
                self.code_png(view.as_ref());
            }
            Action::QrPng => {
                if let Some(q) = self.qr.take() {
                    self.code_png(Ok::<&QrView, &Text>(&q));
                    self.qr.put(q);
                }
            }
            Action::QrOutbox(i) => {
                if let Some(item) = self.outbox.get(i).filter(|it| qr_fits(it)) {
                    // What a wallet reads goes as itself; any other file in
                    // the Faraday file envelope.
                    let source = transfer::qr_source(&item.name, &item.bytes, item.kind);
                    let secret = item.secret;
                    let view = QrView::of(c, &item.name, source, QrFormat::Ur, QR_PARTS[1]).map(
                        |mut v| {
                            v.secret |= secret;
                            v
                        },
                    );
                    self.open_qr(view);
                }
            }
            Action::QrFormat(i) => {
                if let Some(q) = self.qr.view().filter(|q| !q.one_code) {
                    let f = QrFormat::ALL[usize::from(i) % QrFormat::ALL.len()];
                    let v = q.redo(c, f, q.part);
                    self.open_qr(v);
                }
            }
            Action::QrPartSize(n) => {
                if let Some(q) = self.qr.view().filter(|q| !q.one_code) {
                    let v = q.redo(c, q.format, n);
                    self.open_qr(v);
                }
            }
            Action::QrSpeed(ms) => {
                self.settings.set_qr_frame_ms(ms);
                self.save_boxes();
            }
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }

    // ---- the scanner of a GPG key's restore (docs/GPG-PAPERKEY.md §3) ----

    /// **Scan the public key** on Restore a GPG key, or `paperkey`, **Scan
    /// the paperkey** at step 4: the camera, for the public key to restore
    /// from or for the paperkey. The paperkey's opens only where a key may
    /// load (no stick in, as for a SeedQR) and step 4 takes its lines. A
    /// lookup.
    pub(crate) fn gpg_restore_scan(&mut self, paperkey: bool) {
        let takes = self
            .flows
            .on()
            .is_some_and(|i| self.flows.restore_typing(i));
        if paperkey && !(takes && self.may_load_keys()) {
            return;
        }
        let mut scan = ScanState::default();
        scan.purpose = if paperkey {
            ScanPurpose::GpgPaperkey
        } else {
            ScanPurpose::GpgPublic
        };
        self.camera.open(scan);
        self.sheet = Some(Sheet::Scan);
        self.out
            .commands
            .push_back(faraday_shell_api::Command::CameraOn);
    }

    /// Costs what a code read costs, a file's classifying and a SHA-1 of
    /// each key. A code read by the restore's scanner: an armoured public
    /// key is filed in Files, under `{name}-{fp8}.asc` where it is one to
    /// restore from, and its flow opens; one that is not restored here
    /// goes back to the sheet, which says why. Anything else is named on
    /// the scanner, and the camera stays on. Whether the scanner was the
    /// restore's.
    pub(crate) fn gpg_restore_scanned(&mut self, bytes: &[u8]) -> bool {
        match self.camera.scan().map(|s| s.purpose) {
            Some(ScanPurpose::GpgPublic) => {}
            Some(ScanPurpose::GpgPaperkey) => {
                self.gpg_paperkey_scanned(bytes);
                return true;
            }
            _ => return false,
        }
        let c = self.compute();
        let Some((named, _, mut data)) = self.read_code(zeroize::Zeroizing::new(bytes.to_vec()))
        else {
            return true;
        };
        let public = faraday_pgp::read_public_armor(&data) == Some(faraday_pgp::Armor::PublicKey);
        let (from, name) = match (self.gpg_public_name(&data), named) {
            (Some((None, _)), Some(n)) => (None, Text::of(&n)),
            (Some((from, name)), _) => (from, Text::of(&name)),
            (None, Some(n)) => (None, Text::of(&n)),
            (None, None) => (None, text!("scanned-{}.asc", self.camera.next_number())),
        };
        let item = crate::Item::new(&c, &self.free_inbox_name(&name), std::mem::take(&mut *data));
        if !public || item.kind != crate::FileKind::Gpg {
            self.camera.say(Text::of(EN.gpg_restore_not_public));
            return true;
        }
        let at = self.inbox_file_in(item);
        let name = self.inbox[at].name.clone();
        self.camera.close();
        self.sheet = None;
        self.out
            .commands
            .push_back(faraday_shell_api::Command::CameraOff);
        self.toast(&Text::fill(EN.scanned_is_in_files, &[&name]));
        if from.is_some() {
            self.gpg_restore_from(at);
        } else {
            self.flows.restore_sheet(&self.inbox[..]);
            self.sheet = Some(Sheet::GpgRestore);
        }
        self.out.redraw();
        true
    }

    /// Costs a CRC-24 over each line of the code and over the whole. A code
    /// read by the paperkey's scanner: every line it holds goes into the
    /// restore set, checked as a line typed is, and the camera closes; a
    /// code refused says why on the scanner and leaves the lines typed as
    /// they were. Nothing is read while a stick is in.
    fn gpg_paperkey_scanned(&mut self, bytes: &[u8]) {
        let said = if !self.stick.is_empty() {
            Err(Text::of(&crate::stick::keys_load(self.medium)))
        } else if let Some(i) = self.flows.on() {
            self.flows.restore_code(i, bytes).map_err(|e| Text::of(&e))
        } else {
            Err(Text::of(EN.gpg_restore_not_paperkey))
        };
        match said {
            Ok(()) => {
                self.camera.close();
                self.sheet = None;
                self.out
                    .commands
                    .push_back(faraday_shell_api::Command::CameraOff);
            }
            Err(why) => {
                self.camera.say(why);
            }
        }
        self.out.redraw();
    }
}

/// The progress of a multi-part code: parts read, and which are missing.
pub(crate) fn part_note(what: &str, have: usize, total: usize, missing: &[usize]) -> Text {
    let mut list = Text::new();
    for (k, m) in missing.iter().take(8).enumerate() {
        if k > 0 {
            list.push_str(", ");
        }
        list.push_str(&Text::num(*m as u64));
    }
    let more = if missing.len() > 8 { ", …" } else { "" };
    Text::fill(
        EN.part_note_of_missing,
        &[what, &text!("{have}"), &text!("{total}"), &list, more],
    )
}

/// The QR views of the public files and values a flow shows
/// (`docs/QR.md` §4 item 4, §7).
impl Faraday {
    /// Costs what `text`, `wallet_file_text`, `create_key_text` and the
    /// rest it calls cost. The QR view of a public file a flow makes: its
    /// code, and the name and label of its picture.
    pub fn code_view(&self, c: &Compute, code: Code) -> Result<QrView, Text> {
        self.code_view_inner(c, code).map_err(|why| why.0)
    }

    /// Costs what [`Faraday::code_view`] costs. It is that, its refusals as any text.
    #[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
    fn code_view_inner(&self, c: &Compute, code: Code) -> Result<QrView, Why> {
        let wallet = |i: usize| {
            self.session
                .wallets
                .get(i)
                .ok_or_else(|| Text::of(EN.code_view_that_wallet))
        };
        match code {
            Code::Descriptor(i) => {
                let w = wallet(i)?;
                let stem = file_stem(&w.name);
                let mut lines = self.silent_marks(c, i, texts(&self.wallet_label(w)))?;
                let text = w.policy().to_descriptor_checksummed();
                if let Some((_, sum)) = text.rsplit_once('#') {
                    lines.push(Text::fill(EN.code_view_descriptor, &[sum]));
                }
                let title = Text::fill(EN.code_view_wallet_descriptor, &[&text!("{}", w.name)]);
                let name = text!("{stem}-descriptor.png");
                lines.push(Text::of(EN.code_view_public_watch));
                let mut view = QrView::text(c, &title, &text)?.public(&name, lines);
                view.whole = Some(Text::of(&text));
                Ok(view)
            }
            Code::MultisigConfig(i) | Code::Bsms(i) => {
                let w = wallet(i)?;
                let what = if code == Code::MultisigConfig(i) {
                    2
                } else {
                    6
                };
                let text = self
                    .wallet_file_text(c, i, what)
                    .ok_or(EN.code_view_this_wallet_has)?;
                let (title, name, lines) = crate::public_files::wallet_code_head(self, w, what);
                Ok(QrView::text(c, &title, &text)?.public(&name, lines))
            }
            Code::SpecterDiy(i) => self.specter_diy_view(c, wallet(i)?),
            Code::Key(k) => {
                let (fp, text) = self.create_key_text(c, k)?;
                let kind = self.create.state().map_or("", |c| c.kind.name());
                let account = self.create_key_account(c, k);
                let lines = vec![
                    key_label(&fp, kind, &text),
                    Text::of(EN.code_view_public_spends),
                ];
                Ok(QrView::of(
                    c,
                    &Text::fill(EN.code_view_xpub_kind, &[&fp, kind]),
                    QrSource::Key(
                        faraday_text::Text::of(&text),
                        account.as_deref().map(faraday_text::Text::of),
                    ),
                    QrFormat::Ur,
                    QR_PARTS[1],
                )?
                .public(&text!("xpub-{}.png", fp.as_str()), lines))
            }
            Code::KeyBsms(k) => {
                let (fp, key) = self.create_key_text(c, k)?;
                let kind = self.create.state().map_or("", |c| c.kind.name());
                let (_, text) = self
                    .create_key_bsms(c, k)
                    .ok_or(EN.code_view_bip_129_covers)?;
                let lines = vec![
                    key_label(&fp, kind, &key),
                    Text::of(EN.code_view_bip_129_key),
                    Text::of(EN.code_view_public_spends),
                ];
                Ok(
                    QrView::text(c, &Text::fill(EN.code_view_key_bsms, &[&fp]), &text)?
                        .public(&text!("xpub-{}-bsms.png", fp.as_str()), lines),
                )
            }
            Code::WalletKey(i, slot) => {
                let k = self.wallet_key(c, i, slot)?;
                let account = k.standard.and_then(|kind| {
                    kind.account_ur(c, &self.session.keys, k.key, kind.has_bip87() && k.bip87)
                });
                let lines = vec![
                    key_label(&k.fp, &k.kind, &k.text),
                    Text::of(EN.code_view_public_spends),
                ];
                Ok(QrView::of(
                    c,
                    &Text::fill(
                        EN.code_view_xpub_kind,
                        &[&text!("{}", k.fp), &text!("{}", k.kind)],
                    ),
                    QrSource::Key(
                        faraday_text::Text::of(&k.text),
                        account.as_deref().map(faraday_text::Text::of),
                    ),
                    QrFormat::Ur,
                    QR_PARTS[1],
                )?
                .public(&text!("xpub-{}.png", k.fp), lines))
            }
            Code::CosignerKey(i, slot) => {
                let w = self
                    .session
                    .wallets
                    .get(i)
                    .ok_or(EN.code_view_that_wallet)?;
                if w.policy().silent().is_some() {
                    return Err(EN.code_view_this_wallet_s.into());
                }
                let pk = w
                    .policy()
                    .keys()
                    .get(usize::from(slot))
                    .ok_or(EN.code_view_this_wallet)?
                    .clone();
                let fp = pk.fingerprint().map_or_else(
                    || Text::of(EN.code_view_no_origin),
                    |f| Text::of(&fp_text(f)),
                );
                let text = pk.key_text();
                let lines = vec![
                    Text::fill(EN.code_view_key, &[&fp, &Session::shape(w)]),
                    Text::of(EN.code_view_public_spends),
                ];
                Ok(QrView::of(
                    c,
                    &Text::fill(EN.code_view_xpub, &[&fp]),
                    QrSource::Key(faraday_text::Text::of(&text), None),
                    QrFormat::Ur,
                    QR_PARTS[1],
                )?
                .public(&text!("xpub-{}.png", fp.as_str()), lines))
            }
            Code::WalletKeyBsms(i, slot) => {
                let k = self.wallet_key(c, i, slot)?;
                let (_, text) = self.wallet_key_record(c, i, slot)?;
                let lines = vec![
                    key_label(&k.fp, &k.kind, &k.text),
                    Text::of(EN.code_view_bip_129_key),
                    Text::of(EN.code_view_public_spends),
                ];
                Ok(QrView::text(
                    c,
                    &Text::fill(EN.code_view_key_bsms, &[&text!("{}", k.fp)]),
                    &text,
                )?
                .public(&text!("xpub-{}-bsms.png", k.fp), lines))
            }
            Code::Silent(_) | Code::SilentRecord | Code::SilentDns => {
                self.silent_code(c, code).map_err(Why::from)
            }
            Code::Message => {
                let m = self.message.state().ok_or(EN.code_view_no_message)?;
                let sig = m.signed.as_ref().ok_or(EN.code_view_the_message)?;
                let text =
                    faraday_keys::message::signed_text(&sig.address, &sig.signature, &m.text);
                let name = text!(
                    "message-{}.png",
                    &sig.address[sig.address.len().saturating_sub(6)..]
                );
                let lines = vec![
                    Text::fill(EN.code_view_signed_by_address, &[&sig.address]),
                    Text::of(EN.code_view_public_proves),
                ];
                Ok(QrView::text(c, EN.code_view_signed_message, &text)?.public(&name, lines))
            }
            Code::GpgKey | Code::GpgSignature(_) => self.gpg_code(c, code).map_err(Why::from),
            Code::Receive(..)
            | Code::VanityAddress
            | Code::ExploreAddress
            | Code::ExplorePublicKey
            | Code::ExploreXpub
            | Code::NodeId
            | Code::VaultSheet => self.value_code(c, code),
        }
    }
    /// Costs an address derivation for an address (two for a receive
    /// address: the one to check it is one, then the one the screen
    /// keeps), a key derivation the first time for Explore's values, a
    /// descriptor read for a recovery sheet, and what `text` costs. The QR
    /// view of a public value a screen shows (`docs/QR.md` §7): the value
    /// exactly as the screen prints it, whole under the code; a picture
    /// only for the node id and a recovery sheet.
    fn value_code(&self, c: &Compute, code: Code) -> Result<QrView, Why> {
        let plain = |title: &str, text: &str| -> Result<QrView, Why> {
            let mut view = QrView::text(c, title, text)?;
            view.whole = Some(Text::of(text));
            Ok(view)
        };
        let path = || {
            self.explore
                .path()
                .map(|p| {
                    Text::fill(
                        EN.explore_path_m,
                        &[&Text::from_iter(
                            text!("{p}")
                                .chars()
                                .map(|c| if c == '\'' { 'h' } else { c }),
                        )],
                    )
                })
                .unwrap_or_default()
        };
        let explore = || self.explore_reading(c).ok_or(EN.code_view_no_address);
        match code {
            Code::Receive(i, at) => {
                let w = self
                    .session
                    .wallets
                    .get(i)
                    .ok_or(EN.code_view_that_wallet)?;
                // A key made here that waits for its backup: no receive
                // address is shown, nor any code of one (F31).
                let none = w.policy().silent().is_some()
                    || self.receive_held(c, w)
                    || w.policy()
                        .address_at(c, self.session.network(), false, at)
                        .is_err();
                if none {
                    return Err(EN.code_view_no_address.into());
                }
                let a = self.session.address_shown(c, w, false, at);
                let title = Text::fill(
                    EN.code_view_receive,
                    &[&text!("{}", w.name), &text!("{at}")],
                );
                plain(&title, &a)
            }
            Code::VanityAddress => {
                let v = self.vanity.state().ok_or(EN.code_view_no_address)?;
                let f = v.grind.find.as_ref().ok_or(EN.code_view_no_address)?;
                if v.key.is_some_and(|k| self.key_held(c, Fingerprint(k))) {
                    return Err(EN.code_view_no_address.into());
                }
                plain(EN.vanity_vanity_address, f.address.as_str())
            }
            Code::ExploreAddress => {
                let r = explore()?;
                plain(
                    &Text::fill(EN.code_view_explore_address, &[&path()]),
                    &r.address,
                )
            }
            Code::ExplorePublicKey => {
                let r = explore()?;
                plain(
                    &Text::fill(EN.code_view_explore_public_key, &[&path()]),
                    &r.public_key,
                )
            }
            Code::ExploreXpub => {
                let r = explore()?;
                plain(&Text::fill(EN.code_view_explore_xpub, &[&path()]), &r.xpub)
            }
            Code::NodeId => {
                let l = self.lightning.state().ok_or(EN.code_view_no_node)?;
                let n = l.node.as_ref().ok_or(EN.code_view_no_node)?;
                let id = faraday_crypto::hex::encode_string(n.public());
                let kind = match l.source {
                    Some(crate::lightning::NodeSource::Key(fp)) => {
                        Text::fill(EN.lightning_ldk_node, &[&fp_text(Fingerprint(fp))])
                    }
                    Some(crate::lightning::NodeSource::KeyCln(fp)) => {
                        Text::fill(EN.lightning_cln_node, &[&fp_text(Fingerprint(fp))])
                    }
                    Some(crate::lightning::NodeSource::CoreLightning) => {
                        Text::of(EN.code_view_node_cln)
                    }
                    _ => Text::of(EN.code_view_node_lnd),
                };
                let lines = vec![kind, Text::of(EN.code_view_public_node)];
                let name = text!("lightning-{}.png", &id[..8]);
                plain(EN.code_view_node_id, &id).map(|v| v.public(&name, lines))
            }
            Code::VaultSheet => self.vault_sheet_code(c),
            _ => Err(EN.code_view_no_address.into()),
        }
    }

    /// Costs a descriptor read and what `text` costs. The selected
    /// recovery sheet's descriptor in the open vault as its record keeps
    /// it, as `Code::Descriptor` shows a wallet's: labelled with the
    /// wallet's shape and checksum, its picture `{name}-descriptor.png`.
    fn vault_sheet_code(&self, c: &Compute) -> Result<QrView, Why> {
        let (_, r) = self
            .vault_record_of(faraday_vault::records::kind::SHEET)
            .ok_or(EN.code_view_no_sheet)?;
        let (text, name, _) = faraday_keys::vault::sheet_parts(r);
        if text.is_empty() {
            return Err(EN.code_view_no_sheet.into());
        }
        let mut lines = Vec::new();
        if let Ok(policy) = wallet::read_wallet(c, &text) {
            let w = wallet::Wallet::new(
                name.as_str().into(),
                policy,
                Default::default(),
                wallet::Origin::Vault,
            );
            lines.push(Text::of(&wallet::Session::shape(&w)));
        }
        if let Some((_, sum)) = text.rsplit_once('#') {
            lines.push(Text::fill(EN.code_view_descriptor, &[sum]));
        }
        lines.push(Text::of(EN.code_view_public_watch));
        let title = Text::fill(EN.code_view_sheet, &[&name]);
        let png = text!("{}-descriptor.png", crate::file_stem(&name));
        let mut view = QrView::text(c, &title, &text)?.public(&png, lines);
        view.whole = Some(Text::of(&text));
        Ok(view)
    }

    /// Costs what `code_view` and `secret` cost. A vault page's code: the
    /// selected wallet record's descriptor, or the selected entry's TOTP
    /// secret.
    pub(crate) fn vault_qr_act(&mut self, a: crate::vault::VaultAction) {
        use crate::vault::VaultAction as V;
        let c = self.compute();
        let view = match a {
            // The QR view the Wallets page uses, several codes for a long
            // descriptor.
            V::WalletQr => self.with_vault_wallet(|app, i| app.code_view(&c, Code::Descriptor(i))),
            V::TotpQr => self.vault_totp_view(&c),
            V::PaperkeyQr => self.vault_paperkey_view(&c),
            _ => None,
        };
        if let Some(view) = view {
            self.open_qr(view);
            if matches!(a, V::TotpQr | V::PaperkeyQr) {
                self.qr.opened_by(a);
            }
        }
    }

    /// Costs what `secret` costs. The selected entry's TOTP secret as the
    /// stored `otpauth://` URI (a bare key is stored as one), one
    /// plain-text code on the sheet marked Secret, with no picture and no
    /// file: the one code of a secret that has not passed the secret
    /// sheet, shown only from the entry's own page while the URI reads as
    /// TOTP (`docs/QR.md` §7.1, F28). `None` where there is none to show.
    fn vault_totp_view(&self, c: &Compute) -> Option<Result<QrView, Text>> {
        use faraday_keys::{Part, Reveal};
        use faraday_vault::records::field;
        if !self.vault_on_page() {
            return None;
        }
        let (_, r) = self.vault_record_of(faraday_vault::records::kind::ENTRY)?;
        let (id, i) = r.home()?;
        let why = Reveal::VaultEntry(Part::Totp);
        let store = self.vaults.store();
        // An HOTP or unreadable URI has no code here.
        store.totp(id, i, None, why).filter(|t| t.code.is_ok())?;
        let title = Text::fill(
            EN.code_view_totp,
            &[r.public_text(field::TITLE).unwrap_or("")],
        );
        let uri = store.reveal(c, id, i, field::TOTP, why);
        Some(QrView::secret(c, &title, &uri))
    }

    /// Costs what `secret` costs and the paperkey's encoding. The selected
    /// GPG key's paperkey, the text its eye shows, as one static code on
    /// the sheet marked Secret, with no picture and no file, made through
    /// the eye's reveal and held with the view (`docs/GPG-PAPERKEY.md`
    /// §12, F215). `None` where there is none to show.
    fn vault_paperkey_view(&self, c: &Compute) -> Option<Result<QrView, Text>> {
        use faraday_vault::records::kind;
        if !self.vault_on_page() {
            return None;
        }
        let (_, r) = self.vault_record_of(kind::GPG)?;
        let text = crate::gpg::paperkey_shown(self.vaults.store(), r)?;
        let name = faraday_keys::vault::gpg_facts(r)
            .and_then(|f| f.user_ids.first().cloned())
            .unwrap_or_default();
        let title = Text::fill(EN.code_view_paperkey, &[&name]);
        Some(QrView::secret_one_code(c, &title, text.as_str()))
    }
}

/// The line that names a cosigner's key under its code: `Key {fp} ·
/// {kind} · {path}`, the path read from the key's `[fp/path]` origin.
fn key_label(fp: &str, kind: &str, key: &str) -> Text {
    let path = key
        .strip_prefix('[')
        .and_then(|k| k.split_once(']'))
        .and_then(|(origin, _)| origin.split_once('/'))
        .map(|(_, p)| text!("m/{p}"));
    match path {
        Some(p) => Text::fill(EN.key_label_key_kind_path, &[fp, kind, &p]),
        None => Text::fill(EN.key_label_key, &[fp, kind]),
    }
}

/// Lines of a label as wiping text.
fn texts<L: AsRef<str>>(lines: &[L]) -> Vec<Text> {
    lines.iter().map(|l| Text::of(l.as_ref())).collect()
}

/// Why a code view could not be made: the line that says it, from fixed
/// wording or from text another area made.
pub(crate) struct Why(Text);

impl<S: AsRef<str>> From<S> for Why {
    fn from(line: S) -> Why {
        Why(Text::of(line.as_ref()))
    }
}
