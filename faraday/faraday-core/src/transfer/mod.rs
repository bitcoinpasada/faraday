//! Transfer: the online app (the desktop) as the device's QR link. Send
//! shows any file from this computer's Downloads folder, or one dropped
//! on the window, as codes for the device's camera; Receive reads the
//! device's codes with this computer's camera and has the shell save
//! each whole file into Downloads. It is a pipe: nothing received goes
//! into this app's Inbox or session.
//!
//! The shell lists, reads and writes Downloads
//! ([`StorageEvent::HostFiles`], [`StorageCommand::ReadHost`],
//! [`StorageCommand::SaveDownload`]); the app does no I/O. Transfer
//! exists only when [`Faraday::online`] is set, never on the device.

pub(crate) mod screen;
mod text;

pub use text::{MAX_TEXT, SendText, TextCheck, TextPress};

use crate::strings::EN;
use faraday_crypto::work::Compute;
use faraday_text::{Text, text};
use std::collections::VecDeque;

use faraday_shell_api::Command;

use crate::medium::Medium;
use crate::out::Out;
use crate::secrets::Exposure;
use crate::wallet::{self, FileKind};
use crate::{
    Action, Faraday, QR_PARTS, QrFormat, QrSource, QrView, ScanPurpose, ScanState, Screen, Sheet,
    StorageCommand, StorageEvent,
};

/// The largest file sent: what the Faraday file envelope carries.
pub const MAX_SEND: u64 = faraday_qr::envelope::MAX_FILE as u64;

/// What the Transfer screen holds.
#[derive(Debug, Default)]
pub struct TransferState {
    /// The folder the shell lists and saves into, as it names it.
    pub dir: Text,
    /// Its files, newest first, with their sizes.
    pub files: Vec<(Text, u64)>,
    /// The shell can open the folder in this computer's file manager.
    pub opens: bool,
    /// The file being read to send, by its path.
    pub reading: Option<Text>,
    /// Why the last file picked was not sent.
    pub refused: Option<Text>,
    /// Files received without a name of their own so far, for
    /// `received-{n}`.
    pub received: u32,
    /// Saves asked for and not yet answered: whether each holds a secret.
    saving: VecDeque<bool>,
    /// The SHA-256 of the last file this Receive saved: the camera reads
    /// a code held up again and again, and the device's animated code
    /// comes round again, and each is saved once.
    last: Option<[u8; 32]>,
    /// What was saved this session: the line said for each, and whether
    /// it is a warning (a secret saved, or a save that failed).
    pub saved: Vec<(Text, bool)>,
}

/// Transfer's state, with its logic.
#[derive(Debug, Default)]
pub struct TransferArea {
    state: TransferState,
    /// A file is dragged over the window: the page is edged as the
    /// place it drops (`docs/DESIGN.md` §4.17, Edged drop target).
    hovered: bool,
    /// The Send text card's field.
    text: SendText,
}

/// The last part of a path.
fn base_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// The code a file goes as: what a wallet reads as itself, anything
/// else in the Faraday file envelope (as Files' Show as QR sends it).
pub(crate) fn qr_source(name: &str, bytes: &[u8], kind: FileKind) -> QrSource {
    match kind {
        FileKind::Psbt => QrSource::Psbt(bytes.to_vec()),
        FileKind::Wallet | FileKind::Key | FileKind::Message | FileKind::Share => {
            QrSource::Text(faraday_text::Text::from_utf8_lossy(bytes))
        }
        _ => QrSource::File(faraday_text::Text::of(name), bytes.to_vec()),
    }
}

/// A whole transfer as Receive saves it: a name when it brought one, the
/// extension for one without, and the bytes. A transaction is saved as
/// hex text.
fn received(a: faraday_qr::Arrived) -> (Option<faraday_text::Text>, &'static str, Vec<u8>) {
    match crate::arrival(a) {
        (n, "txn", d) => (n, "txt", d),
        other => other,
    }
}

/// Costs a PSBT parse (through `read_psbt`). What one code read on
/// Receive did: still a part, a whole file, or nothing worth keeping
/// (said in the note). `secret` is set when the registry said the file is
/// a private key or a seed, which the saved CBOR no longer shows.
fn read_any(
    c: &Compute,
    scan: &mut ScanState,
    bytes: &[u8],
    secret: &mut bool,
) -> Option<(Option<faraday_text::Text>, &'static str, Vec<u8>)> {
    scan.reads += 1;
    let Ok(raw) = std::str::from_utf8(bytes) else {
        // Bytes that are not text: a PSBT, or kept as they are.
        return Some(if wallet::read_psbt(c, bytes).is_some() {
            (None, "psbt", bytes.to_vec())
        } else {
            (None, "bin", bytes.to_vec())
        });
    };
    let text = raw.trim();
    match scan.assembler.feed(raw) {
        faraday_qr::Step::Part {
            what,
            have,
            total,
            missing,
        } => {
            scan.note = Some(crate::qr_view::part_note(
                crate::qr_view::transfer_name(what),
                have,
                total,
                &missing,
            ));
            return None;
        }
        faraday_qr::Step::Refused(why) => {
            scan.note = Some(crate::qr_view::scan_refusal(why));
            return None;
        }
        faraday_qr::Step::Done(arrived) => return Some(received(arrived)),
        faraday_qr::Step::NotMine => {}
    }
    if faraday_codec::ur::is_ur(text) {
        let message = match crate::ur_feed(&mut scan.decoder, text) {
            Ok(crate::UrFed::Whole(m)) => m,
            Ok(crate::UrFed::More { restarted }) => {
                scan.note = crate::ur_note(&scan.decoder, restarted);
                return None;
            }
            Err(why) => {
                scan.note = Some(Text::of(why));
                return None;
            }
        };
        return Some(match message {
            faraday_codec::ur::Message::Psbt(p) => (None, "psbt", p),
            faraday_codec::ur::Message::Bytes(b) => match std::str::from_utf8(&b) {
                Ok(t) => received(faraday_qr::classify_text(t)),
                Err(_) => (None, "bin", b),
            },
            faraday_codec::ur::Message::Other { ur_type, cbor } => {
                match faraday_qr::registry::read(&ur_type, &cbor) {
                    Ok(Some(faraday_qr::registry::Read::Psbt(p))) => (None, "psbt", p),
                    Ok(Some(faraday_qr::registry::Read::Keys(t)))
                    | Ok(Some(faraday_qr::registry::Read::Descriptor(t))) => {
                        (None, "txt", t.into_bytes())
                    }
                    // A private key or a seed is kept as the CBOR it
                    // carried and is a secret: the hold saves it.
                    Err(
                        faraday_qr::registry::Error::Private | faraday_qr::registry::Error::Seed,
                    ) => {
                        *secret = true;
                        (None, "cbor", cbor)
                    }
                    // A kind this build does not read, and a registry UR
                    // it refused as not in the registry's shape, are kept
                    // as the CBOR they carried.
                    _ => (None, "cbor", cbor),
                }
            }
        });
    }
    if wallet::read_psbt(c, bytes).is_some() {
        return Some((None, "psbt", bytes.to_vec()));
    }
    Some(received(faraday_qr::classify_text(text)))
}

impl TransferArea {
    /// What Transfer holds.
    pub fn state(&self) -> &TransferState {
        &self.state
    }

    /// The Send text card's field and what its check said.
    pub fn text(&self) -> &SendText {
        &self.text
    }

    /// Empties the Send text card's field, wiping what it held.
    pub(crate) fn clear_text(&mut self) {
        if !self.text.is_empty() {
            self.text.clear();
        }
    }

    /// Whether a file is dragged over the window, where a drop is taken.
    pub fn hovered(&self) -> bool {
        self.hovered
    }

    /// The path and size of file `i` of the Downloads list, to send.
    pub(crate) fn pick(&mut self, i: usize, medium: Medium, out: &mut Out) {
        let Some((name, size)) = self.state.files.get(i).cloned() else {
            return;
        };
        let dir = self.state.dir.trim_end_matches('/');
        let path = text!("{dir}/{}", name.as_str());
        self.send(path, size, medium, out);
    }

    /// Sends the file at `path`, `size` bytes long, once the shell has
    /// read it; one over the envelope's limit is refused unread.
    pub(crate) fn send(&mut self, path: Text, size: u64, medium: Medium, out: &mut Out) {
        self.state.refused = None;
        if size > MAX_SEND {
            self.state.refused = Some(Text::fill(
                EN.transfer_larger_than,
                &[&crate::screens::size_text(MAX_SEND), medium.a()],
            ));
            return;
        }
        if size == 0 {
            self.state.refused = Some(Text::fill(EN.transfer_is_empty, &[base_name(&path)]));
            return;
        }
        out.storage_out.push_back(StorageCommand::ReadHost {
            path: faraday_text::Text::of(&path),
        });
        self.state.reading = Some(path);
    }

    /// Costs what `QrView::of` costs, and a classification of the file.
    /// A file the shell read for Send: the codes it shows as, or `None`
    /// when it was not asked for.
    pub(crate) fn read(
        &mut self,
        c: &Compute,
        path: Text,
        mut bytes: Vec<u8>,
        online: bool,
        medium: Medium,
        out: &mut Out,
    ) -> Option<Result<QrView, Text>> {
        if self.state.reading.as_ref() != Some(&path) || !online {
            zeroize::Zeroize::zeroize(&mut bytes);
            return None;
        }
        self.state.reading = None;
        if bytes.len() as u64 > MAX_SEND {
            zeroize::Zeroize::zeroize(&mut bytes);
            self.send(path, MAX_SEND + 1, medium, out);
            return None;
        }
        let name = Text::of(base_name(&path));
        let kind = wallet::classify(c, &name, &bytes);
        let secret = kind.exposure() == Exposure::Secret;
        let source = qr_source(&name, &bytes, kind);
        zeroize::Zeroize::zeroize(&mut bytes);
        Some(
            QrView::of(c, &name, source, QrFormat::Ur, QR_PARTS[1])
                .map(|mut v| {
                    v.secret |= secret;
                    v
                })
                .map_err(|e| Text::of(&e)),
        )
    }

    /// A file the shell could not read for Send.
    pub(crate) fn read_failed(&mut self, path: Text, reason: &str) {
        if self.state.reading.as_ref() != Some(&path) {
            return;
        }
        self.state.reading = None;
        self.state.refused = Some(text!("{}: {reason}", base_name(&path)));
    }

    /// The Downloads folder as the shell lists it. Whether anything
    /// changed, to draw again.
    pub(crate) fn listed(
        &mut self,
        dir: &str,
        files: &[(impl AsRef<str>, u64)],
        opens: bool,
    ) -> bool {
        let t = &mut self.state;
        if t.dir == dir
            && t.opens == opens
            && t.files.len() == files.len()
            && t.files
                .iter()
                .zip(files)
                .all(|(a, b)| a.0 == b.0.as_ref() && a.1 == b.1)
        {
            return false;
        }
        t.dir = Text::of(dir);
        t.files = files
            .iter()
            .map(|(name, size)| (Text::of(name.as_ref()), *size))
            .collect();
        t.opens = opens;
        true
    }

    /// Receive starts afresh: nothing saved last, nothing refused.
    pub(crate) fn receive_begin(&mut self) {
        self.state.last = None;
        self.state.refused = None;
    }

    /// Costs what reading a code costs (a PSBT parse, a SHA-256 of the
    /// file). One code read on Receive, into the camera's `scan`. A whole
    /// file goes to the shell to save in Downloads; the camera stays on
    /// for the next. A secret is not saved: it is returned, its name and
    /// bytes, for the hold that saves it (F129).
    pub(crate) fn scanned(
        &mut self,
        c: &Compute,
        scan: &mut ScanState,
        bytes: &zeroize::Zeroizing<Vec<u8>>,
        out: &mut Out,
    ) -> Option<(Text, zeroize::Zeroizing<Vec<u8>>)> {
        let mut registry_secret = false;
        let (named, ext, data) = read_any(c, scan, bytes, &mut registry_secret)?;
        let digest = {
            let mut whole = named
                .as_ref()
                .map_or_else(Vec::new, |n| n.as_bytes().to_vec());
            whole.push(0);
            whole.extend_from_slice(ext.as_bytes());
            whole.push(0);
            whole.extend_from_slice(&data);
            let d = faraday_crypto::sha256_whole(c, &whole);
            zeroize::Zeroize::zeroize(&mut whole);
            d
        };
        if self.state.last == Some(digest) {
            // Already saved: the line says so again.
            scan.note = self.state.saved.last().map(|(l, _)| l.clone());
            return None;
        }
        self.state.last = Some(digest);
        let name = match named {
            Some(n) => n,
            None => {
                self.state.received += 1;
                text!("received-{}.{ext}", self.state.received)
            }
        };
        // A secret the device let out past its secret sheet is saved as
        // asked, past the hold, and said to be one.
        let secret = registry_secret
            || wallet::classify(c, &name, &data).exposure() == Exposure::Secret
            || crate::seed_in_code(c, bytes).is_some()
            || crate::scan_read::secret_kind(c, &data).is_some();
        if secret {
            return Some((name, zeroize::Zeroizing::new(data)));
        }
        self.save(scan, name, data, false, out);
        None
    }

    /// File `name` to the shell, to save in Downloads; `secret` when it
    /// holds one.
    fn save(
        &mut self,
        scan: &mut ScanState,
        name: Text,
        data: Vec<u8>,
        secret: bool,
        out: &mut Out,
    ) {
        scan.note = Some(Text::fill(EN.transfer_saving, &[&name]));
        self.state.saving.push_back(secret);
        out.storage_out.push_back(StorageCommand::SaveDownload {
            name: faraday_text::Text::of(&name),
            bytes: data,
        });
    }

    /// The shell saved a received file, or could not; Receive's camera
    /// (`scan`, when open) says so.
    pub(crate) fn saved(
        &mut self,
        saved: Result<impl AsRef<str>, impl AsRef<str>>,
        scan: Option<&mut ScanState>,
    ) {
        let secret = self.state.saving.pop_front().unwrap_or(false);
        let warn = secret || saved.is_err();
        let line = match saved {
            Ok(path) if secret => Text::fill(EN.transfer_saved_secret, &[path.as_ref()]),
            Ok(path) => Text::fill(EN.transfer_saved, &[path.as_ref()]),
            Err(reason) => Text::fill(EN.transfer_not_saved, &[reason.as_ref()]),
        };
        if let Some(scan) = scan
            && scan.purpose == ScanPurpose::Transfer
        {
            scan.note = Some(line.clone());
        }
        self.state.saved.push((line, warn));
    }

    /// Opens the Downloads folder in this computer's file manager.
    pub(crate) fn open_folder(&self, online: bool, out: &mut Out) {
        if online && self.state.opens {
            out.storage_out.push_back(StorageCommand::OpenDownloads);
        }
    }
}

impl Faraday {
    /// What Transfer holds.
    pub fn transfer(&self) -> &TransferState {
        self.transfer.state()
    }

    /// Opens Transfer: only in the online app.
    pub(crate) fn transfer_open(&mut self) {
        if !self.online {
            return;
        }
        self.screen = Screen::Transfer;
        self.list_offset = 0.0;
    }

    /// The Send text card's field.
    pub fn transfer_text(&self) -> &SendText {
        self.transfer.text()
    }

    /// Whether the Send text card is on screen and takes typing.
    pub(crate) fn transfer_text_here(&self) -> bool {
        self.online && self.screen == Screen::Transfer && self.sheet.is_none()
    }

    /// Costs what [`text::check`] costs. A key typed into the Send text
    /// card's field: whether it was taken.
    pub(crate) fn transfer_key(&mut self, key: faraday_shell_api::Key) -> bool {
        if !self.transfer_text_here() {
            return false;
        }
        let c = self.compute();
        self.transfer.text.key(&c, key)
    }

    /// Ctrl+V on Transfer: the shell is asked for the clipboard's text.
    pub(crate) fn transfer_paste(&mut self) {
        if self.transfer_text_here() && self.clipboard {
            self.out.commands.push_back(Command::RequestClipboard {
                kind: faraday_shell_api::FileKind::Text,
            });
        }
    }

    /// Costs what [`text::check`] costs. The clipboard's text, asked for by
    /// Paste, into the Send text card's field. Returns whether it was
    /// taken here.
    pub(crate) fn transfer_pasted(&mut self, text: &str) -> bool {
        if !self.transfer_text_here() {
            return false;
        }
        if !text.is_empty() {
            let c = self.compute();
            self.transfer.text.put(&c, text);
        }
        true
    }

    /// Sends file `i` of the Downloads list.
    pub(crate) fn transfer_pick(&mut self, i: usize) {
        if !self.online || self.screen != Screen::Transfer {
            return;
        }
        self.transfer.pick(i, self.medium, &mut self.out);
    }

    /// What the host says of a file dragged, dropped, read or saved.
    pub(crate) fn transfer_event(&mut self, event: StorageEvent) {
        match event {
            StorageEvent::Hovered(over) => self.transfer_hovered(over),
            StorageEvent::Dropped { path, size } => self.transfer_dropped(path, size),
            StorageEvent::HostRead { path, bytes } => self.transfer_read(path, bytes),
            StorageEvent::HostReadFailed { path, reason } => {
                self.transfer_read_failed(path, reason);
            }
            StorageEvent::Saved { path } => self.transfer_saved(Ok(&path)),
            StorageEvent::SaveFailed { reason } => self.transfer_saved(Err(&reason)),
            // `storage` passes only the events matched above.
            _ => {}
        }
    }

    /// A file dropped on the window: Transfer opens and sends it.
    /// A file dragged over the window, or the drag gone: the page is
    /// edged while one is over it, where a drop is taken.
    pub(crate) fn transfer_hovered(&mut self, over: bool) {
        self.transfer.hovered = over && self.online;
    }

    pub(crate) fn transfer_dropped(&mut self, path: impl AsRef<str>, size: u64) {
        self.transfer.hovered = false;
        if !self.online {
            return;
        }
        if self.screen != Screen::Transfer {
            self.transfer_open();
        }
        self.transfer
            .send(Text::of(path.as_ref()), size, self.medium, &mut self.out);
    }

    /// A file the shell read for Send: shown as codes.
    pub(crate) fn transfer_read(&mut self, path: impl AsRef<str>, bytes: Vec<u8>) {
        let c = self.compute();
        let view = self.transfer.read(
            &c,
            Text::of(path.as_ref()),
            bytes,
            self.online,
            self.medium,
            &mut self.out,
        );
        if let Some(view) = view {
            self.open_qr(view);
        }
    }

    /// A file the shell could not read for Send.
    pub(crate) fn transfer_read_failed(&mut self, path: impl AsRef<str>, reason: impl AsRef<str>) {
        self.transfer
            .read_failed(Text::of(path.as_ref()), reason.as_ref());
    }

    /// The Downloads folder as the shell lists it. Whether anything
    /// changed, to draw again.
    pub(crate) fn transfer_listed(
        &mut self,
        dir: impl AsRef<str>,
        files: Vec<(impl AsRef<str>, u64)>,
        opens: bool,
    ) -> bool {
        self.transfer.listed(dir.as_ref(), &files, opens)
    }

    /// Receive: the camera, reading anything, until Cancel.
    pub(crate) fn transfer_receive(&mut self) {
        if !self.online {
            return;
        }
        self.transfer.receive_begin();
        let mut scan = ScanState::default();
        scan.purpose = ScanPurpose::Transfer;
        self.camera.open(scan);
        self.sheet = Some(Sheet::Scan);
        self.out.commands.push_back(Command::CameraOn);
    }

    /// One code read on Receive.
    pub(crate) fn transfer_scanned(&mut self, bytes: zeroize::Zeroizing<Vec<u8>>) {
        let c = self.compute();
        let Some(Some((name, data))) = self
            .camera
            .with_scan(|scan| self.transfer.scanned(&c, scan, &bytes, &mut self.out))
        else {
            return;
        };
        // A secret waits for **Hold to save without encryption** (F129).
        self.plain_ask(crate::secrets::Plain {
            name: faraday_text::Text::of(name.as_str()),
            on: faraday_text::Text::of(self.transfer.state.dir.as_str()),
            keep: faraday_text::Text::of(EN.plain_computer),
            then: crate::secrets::PlainThen::Transfer(faraday_text::Text::of(name.as_str()), data),
        });
    }

    /// A received secret, past its hold: saved into the folder, and the
    /// camera's sheet back for the next code.
    pub(crate) fn transfer_save(
        &mut self,
        name: impl AsRef<str>,
        data: zeroize::Zeroizing<Vec<u8>>,
    ) {
        self.transfer_skip();
        self.camera.with_scan(|scan| {
            self.transfer.save(
                scan,
                Text::of(name.as_ref()),
                data.to_vec(),
                true,
                &mut self.out,
            )
        });
    }

    /// A received secret not saved: the camera's sheet back, where it is
    /// still on.
    pub(crate) fn transfer_skip(&mut self) {
        if self.camera.scan().is_some() {
            self.sheet = Some(Sheet::Scan);
        }
    }

    /// The shell saved a received file, or could not.
    pub(crate) fn transfer_saved(&mut self, saved: Result<&str, &str>) {
        self.camera
            .with_scan_or_none(|scan| self.transfer.saved(saved, scan));
    }

    /// Opens the Downloads folder in this computer's file manager.
    pub(crate) fn transfer_open_folder(&mut self) {
        self.transfer.open_folder(self.online, &mut self.out);
    }

    /// Costs what [`text::check`] costs. A press on the Send text card:
    /// Paste asks for the clipboard, Clear wipes the field, Show as QR
    /// checks the text once more and opens the sheet.
    pub(crate) fn transfer_text_press(&mut self, press: TextPress) {
        if !self.transfer_text_here() {
            return;
        }
        match press {
            TextPress::Paste => self.transfer_paste(),
            TextPress::Clear => self.transfer.text.clear(),
            TextPress::Show => {
                let c = self.compute();
                self.transfer.text.recheck(&c);
                let Some(text) = self.transfer.text.sendable().map(Text::of) else {
                    return;
                };
                let view = QrView::text(&c, EN.transfer_text_sheet_title, &text);
                self.open_qr(view);
            }
        }
    }

    /// Handles the Transfer screen's actions (`Transfer*`).
    pub(crate) fn transfer_act(&mut self, action: Action) {
        match action {
            Action::TransferSend(i) => self.transfer_pick(i),
            Action::TransferReceive => self.transfer_receive(),
            Action::TransferOpenFolder => self.transfer_open_folder(),
            Action::TransferText(press) => self.transfer_text_press(press),
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}
