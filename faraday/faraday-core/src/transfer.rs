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

use std::collections::VecDeque;

use osk_shell_api::Command;

use crate::secrets::Exposure;
use crate::wallet::{self, FileKind};
use crate::{
    Faraday, QR_PARTS, QrFormat, QrSource, QrView, ScanPurpose, ScanState, Screen, Sheet,
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

/// What one code read on Receive did: still a part, a whole file, or
/// nothing worth keeping (said in the note).
fn read_any(scan: &mut ScanState, bytes: &[u8]) -> Option<(Option<String>, &'static str, Vec<u8>)> {
    scan.reads += 1;
    let Ok(raw) = std::str::from_utf8(bytes) else {
        // Bytes that are not text: a PSBT, or kept as they are.
        return Some(if wallet::read_psbt(bytes).is_some() {
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
            scan.note = Some(crate::part_note(what, have, total, &missing));
            return None;
        }
        faraday_qr::Step::Refused(why) => {
            scan.note = Some(why);
            return None;
        }
        faraday_qr::Step::Done(arrived) => return Some(received(arrived)),
        faraday_qr::Step::NotMine => {}
    }
    if osk_codec::ur::is_ur(text) {
        return match scan.decoder.receive(text) {
            Ok(true) => {
                let got = match scan.decoder.message() {
                    Some(osk_codec::ur::Message::Psbt(p)) => Some((None, "psbt", p.clone())),
                    Some(osk_codec::ur::Message::Bytes(b)) => Some(match std::str::from_utf8(b) {
                        Ok(t) => received(faraday_qr::classify_text(t)),
                        Err(_) => (None, "bin", b.clone()),
                    }),
                    Some(osk_codec::ur::Message::Other { ur_type, cbor }) => {
                        Some(match faraday_qr::registry::read(ur_type, cbor) {
                            Ok(Some(faraday_qr::registry::Read::Psbt(p))) => (None, "psbt", p),
                            Ok(Some(faraday_qr::registry::Read::Keys(t)))
                            | Ok(Some(faraday_qr::registry::Read::Descriptor(t))) => {
                                (None, "txt", t.into_bytes())
                            }
                            // A kind this build does not read is kept as
                            // the CBOR it carried.
                            _ => (None, "cbor", cbor.clone()),
                        })
                    }
                    None => None,
                };
                scan.decoder = osk_codec::ur::Decoder::new();
                got
            }
            Ok(false) => {
                if let Some((have, of)) = scan.decoder.progress() {
                    scan.note = Some(format!("UR: part {have} of {of}"));
                }
                None
            }
            Err(e) => {
                scan.note = Some(format!("Not a part of this code: {e:?}"));
                None
            }
        };
    }
    if wallet::read_psbt(bytes).is_some() {
        return Some((None, "psbt", bytes.to_vec()));
    }
    Some(received(faraday_qr::classify_text(text)))
}

impl Faraday {
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
        let Some((name, size)) = self.transfer.files.get(i).cloned() else {
            return;
        };
        let dir = self.transfer.dir.trim_end_matches('/');
        let path = format!("{dir}/{name}");
        self.transfer_send(path, size);
    }

    /// Sends the file at `path`, `size` bytes long, once the shell has
    /// read it; one over the envelope's limit is refused unread.
    fn transfer_send(&mut self, path: String, size: u64) {
        self.transfer.refused = None;
        if size > MAX_SEND {
            self.transfer.refused = Some(format!(
                "Larger than 256 KiB: carry it on {}",
                self.medium.a()
            ));
            return;
        }
        if size == 0 {
            self.transfer.refused = Some(format!("{} is empty", base_name(&path)));
            return;
        }
        self.transfer.reading = Some(path.clone());
        self.storage_out
            .push_back(StorageCommand::ReadHost { path });
    }

    /// A file dropped on the window: Transfer opens and sends it.
    pub(crate) fn transfer_dropped(&mut self, path: String, size: u64) {
        if !self.online {
            return;
        }
        if self.screen != Screen::Transfer {
            self.transfer_open();
        }
        self.transfer_send(path, size);
    }

    /// A file the shell read for Send: shown as codes.
    pub(crate) fn transfer_read(&mut self, path: String, mut bytes: Vec<u8>) {
        if self.transfer.reading.as_deref() != Some(path.as_str()) || !self.online {
            zeroize::Zeroize::zeroize(&mut bytes);
            return;
        }
        self.transfer.reading = None;
        if bytes.len() as u64 > MAX_SEND {
            zeroize::Zeroize::zeroize(&mut bytes);
            return self.transfer_send(path, MAX_SEND + 1);
        }
        let name = base_name(&path).to_string();
        let kind = wallet::classify(&name, &bytes);
        let secret = kind.exposure() == Exposure::Secret;
        let source = qr_source(&name, &bytes, kind);
        zeroize::Zeroize::zeroize(&mut bytes);
        let view = QrView::of(&name, source, QrFormat::Ur, QR_PARTS[1]).map(|mut v| {
            v.secret |= secret;
            v
        });
        self.open_qr(view);
    }

    /// A file the shell could not read for Send.
    pub(crate) fn transfer_read_failed(&mut self, path: String, reason: String) {
        if self.transfer.reading.as_deref() != Some(path.as_str()) {
            return;
        }
        self.transfer.reading = None;
        self.transfer.refused = Some(format!("{}: {reason}", base_name(&path)));
    }

    /// The Downloads folder as the shell lists it. Whether anything
    /// changed, to draw again.
    pub(crate) fn transfer_listed(
        &mut self,
        dir: String,
        files: Vec<(String, u64)>,
        opens: bool,
    ) -> bool {
        let t = &mut self.transfer;
        if t.dir == dir && t.files == files && t.opens == opens {
            return false;
        }
        t.dir = dir;
        t.files = files;
        t.opens = opens;
        true
    }

    /// Receive: the camera, reading anything, until Cancel.
    pub(crate) fn transfer_receive(&mut self) {
        if !self.online {
            return;
        }
        self.transfer.last = None;
        self.transfer.refused = None;
        let mut scan = ScanState::default();
        scan.purpose = ScanPurpose::Transfer;
        self.scan = Some(scan);
        self.sheet = Some(Sheet::Scan);
        self.commands.push_back(Command::CameraOn);
    }

    /// One code read on Receive. A whole file goes to the shell to save
    /// in Downloads; the camera stays on for the next.
    pub(crate) fn transfer_scanned(&mut self, bytes: zeroize::Zeroizing<Vec<u8>>) {
        let Some(scan) = self.scan.as_mut() else {
            return;
        };
        let Some((named, ext, data)) = read_any(scan, &bytes) else {
            return;
        };
        let digest = {
            use osk_bip::bitcoin::hashes::{Hash, sha256};
            let mut whole = named.clone().unwrap_or_default().into_bytes();
            whole.push(0);
            whole.extend_from_slice(ext.as_bytes());
            whole.push(0);
            whole.extend_from_slice(&data);
            let d = sha256::Hash::hash(&whole).to_byte_array();
            zeroize::Zeroize::zeroize(&mut whole);
            d
        };
        if self.transfer.last == Some(digest) {
            // Already saved: the line says so again.
            let said = self.transfer.saved.last().map(|(l, _)| l.clone());
            if let Some(scan) = self.scan.as_mut() {
                scan.note = said;
            }
            return;
        }
        self.transfer.last = Some(digest);
        let name = match named {
            Some(n) => n,
            None => {
                self.transfer.received += 1;
                format!("received-{}.{ext}", self.transfer.received)
            }
        };
        // A secret the device let out past its secret sheet is saved as
        // asked, and said to be one.
        let secret = wallet::classify(&name, &data).exposure() == Exposure::Secret
            || crate::seed_in_code(&bytes).is_some()
            || crate::secret_text(&String::from_utf8_lossy(&data)).is_some();
        if let Some(scan) = self.scan.as_mut() {
            scan.note = Some(format!("Saving {name}"));
        }
        self.transfer.saving.push_back(secret);
        self.storage_out
            .push_back(StorageCommand::SaveDownload { name, bytes: data });
    }

    /// The shell saved a received file, or could not.
    pub(crate) fn transfer_saved(&mut self, saved: Result<String, String>) {
        let secret = self.transfer.saving.pop_front().unwrap_or(false);
        let warn = secret || saved.is_err();
        let line = match saved {
            Ok(path) if secret => format!("Saved {path}: this file holds a secret"),
            Ok(path) => format!("Saved {path}"),
            Err(reason) => format!("Not saved: {reason}"),
        };
        if let Some(scan) = self.scan.as_mut()
            && scan.purpose == ScanPurpose::Transfer
        {
            scan.note = Some(line.clone());
        }
        self.transfer.saved.push((line, warn));
    }

    /// Opens the Downloads folder in this computer's file manager.
    pub(crate) fn transfer_open_folder(&mut self) {
        if self.online && self.transfer.opens {
            self.storage_out.push_back(StorageCommand::OpenDownloads);
        }
    }
}
