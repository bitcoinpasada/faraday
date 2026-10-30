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

use crate::strings::EN;
use crate::strings::fill;
use faraday_crypto::work::Compute;
use std::collections::VecDeque;

use faraday_shell_api::Command;

use crate::medium::Medium;
use crate::out::Out;
use crate::secrets::Exposure;
use crate::wallet::{self, FileKind};
use crate::{
    Action, Faraday, QR_PARTS, QrFormat, QrSource, QrView, ScanPurpose, ScanState, Screen, Sheet,
    StorageCommand,
};

/// The largest file sent: what the Faraday file envelope carries.
pub const MAX_SEND: u64 = faraday_qr::envelope::MAX_FILE as u64;

/// What the Transfer screen holds.
#[derive(Debug, Default)]
pub struct TransferState {
    /// The folder the shell lists and saves into, as it names it.
    pub dir: String,
    /// Its files, newest first, with their sizes.
    pub files: Vec<(String, u64)>,
    /// The shell can open the folder in this computer's file manager.
    pub opens: bool,
    /// The file being read to send, by its path.
    pub reading: Option<String>,
    /// Why the last file picked was not sent.
    pub refused: Option<String>,
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
    pub saved: Vec<(String, bool)>,
}

/// Transfer's state, with its logic.
#[derive(Debug, Default)]
pub struct TransferArea {
    state: TransferState,
    /// A file is dragged over the window: the page is edged as the
    /// place it drops (`docs/DESIGN.md` §4.17, Edged drop target).
    hovered: bool,
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
            QrSource::Text(String::from_utf8_lossy(bytes).into_owned())
        }
        _ => QrSource::File(name.to_string(), bytes.to_vec()),
    }
}

/// A whole transfer as Receive saves it: a name when it brought one, the
/// extension for one without, and the bytes. A transaction is saved as
/// hex text.
fn received(a: faraday_qr::Arrived) -> (Option<String>, &'static str, Vec<u8>) {
    match crate::arrival(a) {
        (n, "txn", d) => (n, "txt", d),
        other => other,
    }
}

/// Costs a PSBT parse (through `read_psbt`). What one code read on
/// Receive did: still a part, a whole file, or nothing worth keeping
/// (said in the note).
fn read_any(
    c: &Compute,
    scan: &mut ScanState,
    bytes: &[u8],
) -> Option<(Option<String>, &'static str, Vec<u8>)> {
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
    match scan.assembler.feed(text) {
        faraday_qr::Step::Part {
            what,
            have,
            total,
            missing,
        } => {
            scan.note = Some(crate::part_note(
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
                scan.note = Some(why.to_string());
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
                    // A kind this build does not read is kept as the
                    // CBOR it carried.
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
        let path = format!("{dir}/{name}");
        self.send(path, size, medium, out);
    }

    /// Sends the file at `path`, `size` bytes long, once the shell has
    /// read it; one over the envelope's limit is refused unread.
    pub(crate) fn send(&mut self, path: String, size: u64, medium: Medium, out: &mut Out) {
        self.state.refused = None;
        if size > MAX_SEND {
            self.state.refused = Some(fill(
                EN.transfer_larger_than,
                &[&crate::screens::size_text(MAX_SEND), medium.a()],
            ));
            return;
        }
        if size == 0 {
            self.state.refused = Some(fill(EN.transfer_is_empty, &[base_name(&path)]));
            return;
        }
        self.state.reading = Some(path.clone());
        out.storage_out.push_back(StorageCommand::ReadHost { path });
    }

    /// Costs what `QrView::of` costs, and a classification of the file.
    /// A file the shell read for Send: the codes it shows as, or `None`
    /// when it was not asked for.
    pub(crate) fn read(
        &mut self,
        c: &Compute,
        path: String,
        mut bytes: Vec<u8>,
        online: bool,
        medium: Medium,
        out: &mut Out,
    ) -> Option<Result<QrView, String>> {
        if self.state.reading.as_deref() != Some(path.as_str()) || !online {
            zeroize::Zeroize::zeroize(&mut bytes);
            return None;
        }
        self.state.reading = None;
        if bytes.len() as u64 > MAX_SEND {
            zeroize::Zeroize::zeroize(&mut bytes);
            self.send(path, MAX_SEND + 1, medium, out);
            return None;
        }
        let name = base_name(&path).to_string();
        let kind = wallet::classify(c, &name, &bytes);
        let secret = kind.exposure() == Exposure::Secret;
        let source = qr_source(&name, &bytes, kind);
        zeroize::Zeroize::zeroize(&mut bytes);
        Some(
            QrView::of(c, &name, source, QrFormat::Ur, QR_PARTS[1]).map(|mut v| {
                v.secret |= secret;
                v
            }),
        )
    }

    /// A file the shell could not read for Send.
    pub(crate) fn read_failed(&mut self, path: String, reason: String) {
        if self.state.reading.as_deref() != Some(path.as_str()) {
            return;
        }
        self.state.reading = None;
        self.state.refused = Some(format!("{}: {reason}", base_name(&path)));
    }

    /// The Downloads folder as the shell lists it. Whether anything
    /// changed, to draw again.
    pub(crate) fn listed(&mut self, dir: String, files: Vec<(String, u64)>, opens: bool) -> bool {
        let t = &mut self.state;
        if t.dir == dir && t.files == files && t.opens == opens {
            return false;
        }
        t.dir = dir;
        t.files = files;
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
    ) -> Option<(String, zeroize::Zeroizing<Vec<u8>>)> {
        let (named, ext, data) = read_any(c, scan, bytes)?;
        let digest = {
            let mut whole = named.clone().unwrap_or_default().into_bytes();
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
                format!("received-{}.{ext}", self.state.received)
            }
        };
        // A secret the device let out past its secret sheet is saved as
        // asked, past the hold, and said to be one.
        let secret = wallet::classify(c, &name, &data).exposure() == Exposure::Secret
            || crate::seed_in_code(c, bytes).is_some()
            || crate::scan_key::secret_code(c, &data, &String::from_utf8_lossy(&data)).is_some();
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
        name: String,
        data: Vec<u8>,
        secret: bool,
        out: &mut Out,
    ) {
        scan.note = Some(fill(EN.transfer_saving, &[&name.to_string()]));
        self.state.saving.push_back(secret);
        out.storage_out
            .push_back(StorageCommand::SaveDownload { name, bytes: data });
    }

    /// The shell saved a received file, or could not; Receive's camera
    /// (`scan`, when open) says so.
    pub(crate) fn saved(&mut self, saved: Result<String, String>, scan: Option<&mut ScanState>) {
        let secret = self.state.saving.pop_front().unwrap_or(false);
        let warn = secret || saved.is_err();
        let line = match saved {
            Ok(path) if secret => fill(EN.transfer_saved_secret, &[&path.to_string()]),
            Ok(path) => fill(EN.transfer_saved, &[&path.to_string()]),
            Err(reason) => fill(EN.transfer_not_saved, &[&reason.to_string()]),
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

    /// Sends file `i` of the Downloads list.
    pub(crate) fn transfer_pick(&mut self, i: usize) {
        if !self.online || self.screen != Screen::Transfer {
            return;
        }
        self.transfer.pick(i, self.medium, &mut self.out);
    }

    /// A file dropped on the window: Transfer opens and sends it.
    /// A file dragged over the window, or the drag gone: the page is
    /// edged while one is over it, where a drop is taken.
    pub(crate) fn transfer_hovered(&mut self, over: bool) {
        self.transfer.hovered = over && self.online;
    }

    pub(crate) fn transfer_dropped(&mut self, path: String, size: u64) {
        self.transfer.hovered = false;
        if !self.online {
            return;
        }
        if self.screen != Screen::Transfer {
            self.transfer_open();
        }
        self.transfer.send(path, size, self.medium, &mut self.out);
    }

    /// A file the shell read for Send: shown as codes.
    pub(crate) fn transfer_read(&mut self, path: String, bytes: Vec<u8>) {
        let c = self.compute();
        let view = self
            .transfer
            .read(&c, path, bytes, self.online, self.medium, &mut self.out);
        if let Some(view) = view {
            self.open_qr(view);
        }
    }

    /// A file the shell could not read for Send.
    pub(crate) fn transfer_read_failed(&mut self, path: String, reason: String) {
        self.transfer.read_failed(path, reason);
    }

    /// The Downloads folder as the shell lists it. Whether anything
    /// changed, to draw again.
    pub(crate) fn transfer_listed(
        &mut self,
        dir: String,
        files: Vec<(String, u64)>,
        opens: bool,
    ) -> bool {
        self.transfer.listed(dir, files, opens)
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
        let Some(scan) = self.camera.scan_mut() else {
            return;
        };
        let Some((name, data)) = self.transfer.scanned(&c, scan, &bytes, &mut self.out) else {
            return;
        };
        // A secret waits for **Hold to save without encryption** (F129).
        let dir = self.transfer.state.dir.clone();
        self.plain_ask(crate::secrets::Plain {
            name: name.clone(),
            on: dir,
            keep: EN.plain_computer.to_string(),
            then: crate::secrets::PlainThen::Transfer(name, data),
        });
    }

    /// A received secret, past its hold: saved into the folder, and the
    /// camera's sheet back for the next code.
    pub(crate) fn transfer_save(&mut self, name: String, data: zeroize::Zeroizing<Vec<u8>>) {
        self.transfer_skip();
        let Some(scan) = self.camera.scan_mut() else {
            return;
        };
        self.transfer
            .save(scan, name, data.to_vec(), true, &mut self.out);
    }

    /// A received secret not saved: the camera's sheet back, where it is
    /// still on.
    pub(crate) fn transfer_skip(&mut self) {
        if self.camera.scan_mut().is_some() {
            self.sheet = Some(Sheet::Scan);
        }
    }

    /// The shell saved a received file, or could not.
    pub(crate) fn transfer_saved(&mut self, saved: Result<String, String>) {
        self.transfer.saved(saved, self.camera.scan_mut());
    }

    /// Opens the Downloads folder in this computer's file manager.
    pub(crate) fn transfer_open_folder(&mut self) {
        self.transfer.open_folder(self.online, &mut self.out);
    }

    /// Handles the Transfer screen's actions (`Transfer*`).
    pub(crate) fn transfer_act(&mut self, action: Action) {
        match action {
            Action::TransferSend(i) => self.transfer_pick(i),
            Action::TransferReceive => self.transfer_receive(),
            Action::TransferOpenFolder => self.transfer_open_folder(),
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}
