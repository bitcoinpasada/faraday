//! The scanner, built from `docs/DESIGN.md` §5: the Scanner screen with
//! its state inside the viewfinder, the Result behind a QR that could
//! not be used, the Menu behind one nothing recognises, and the Result
//! that cautions before loading words a camera has just read.
//!
//! §4.9 puts the expected type in the title ("Scan a transaction"), so
//! no sentence under the app bar says it again, and the app bar's back
//! chevron is how the scanner is left. The word for what a camera reads
//! is "QR" on every screen here, never "code".

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::organisms::TileSpec;
use osk_ui::screens::{self, Action, Chrome, Entry, Row, Scanner};
use osk_ui::widgets::keyboard::{ALL_KEYS, DONE_DISABLED, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::scan::{Expect, ScanStage, ScanState};
use crate::{OpenSigner, ids, strings, text};

impl OpenSigner {
    pub(crate) fn view_scan(&self) -> Node {
        self.with_chrome(Some(ids::BACK), |c| {
            let s = self.strings();
            let Some(scan) = self.scan.as_ref() else {
                // The screen and the scanner's state are set together, so
                // there is no state here to report.
                return screens::menu(c, s.scan_title, None, Vec::new(), Vec::new());
            };
            if let Some(reason) = scan.error() {
                return self.scan_not_usable(c, reason);
            }
            match scan.stage() {
                ScanStage::Camera | ScanStage::Unavailable | ScanStage::Waiting => {
                    self.scan_viewfinder(c, scan)
                }
                ScanStage::Unknown => self.scan_unknown(c, scan),
                ScanStage::WordsCaution => self.scan_words(c, scan),
                ScanStage::BackupPassphrase => self.scan_backup_pass(c, scan),
            }
        })
    }

    /// §5 Scanner: "square viewfinder with the state inside, optional
    /// 'Read a file' row." One screen for all three states a camera is
    /// in — live, absent, waiting on a file — because they differ in
    /// the state line and in nothing else.
    fn scan_viewfinder(&self, c: &Chrome<'_>, scan: &ScanState) -> Node {
        let s = self.strings();
        let progress = scan.progress();
        // What just happened to the clipboard takes the state line for
        // a moment, because it is what the person is waiting to read.
        let state = match scan.note(self.now_ms) {
            Some(note) => String::from(note),
            None => match (scan.stage(), progress) {
                (ScanStage::Unavailable, _) => String::from(s.scan_no_camera),
                (ScanStage::Waiting, _) => String::from(s.scan_waiting),
                (_, Some((got, total))) => strings::fill(
                    s.scan_ur_progress,
                    &[&alloc::format!("{got}"), &alloc::format!("{total}")],
                ),
                _ => String::from(s.scan_looking),
            },
        };
        let parts = progress.map(|(got, total)| {
            if total == 0 {
                0.0
            } else {
                got as f32 / total as f32
            }
        });
        screens::scanner(
            c,
            Scanner {
                title: self.scan_title(),
                state: &state,
                parts,
                ways: self.scan_ways(scan.expect()),
                // The tree shares the buffer rather than copying it; the
                // last holder to let go wipes it.
                action: None,
                preview: scan.preview().map(|p| components::Preview {
                    width: p.width(),
                    height: p.height(),
                    pixels: p.shared(),
                    chroma: p.shared_chroma(),
                }),
            },
        )
    }

    /// §4.9 (PLANNING §16.88): the band under the square. "Read a
    /// file" is always there, "Paste" wherever the value is not a
    /// secret, and "Type" wherever the flow has a keyboard for it.
    /// Convert key adds the keys this device already holds. A way this
    /// shell cannot serve is a dimmed tile, and §4.11 says a dimmed
    /// control needs no words: dim already means unavailable.
    fn scan_ways(&self, expect: Expect) -> Vec<TileSpec> {
        let s = self.strings();
        let tile = |id, icon, label: &'static str, enabled| TileSpec {
            id,
            icon,
            label: String::from(label),
            enabled,
            badge: None,
        };
        let mut ways = vec![tile(ids::SCAN_FILE, Icon::File, s.scan_from_file, true)];
        if expect == Expect::ConvertKey && !self.keys.is_empty() {
            ways.push(tile(
                ids::SCAN_USE_KEY,
                Icon::Fingerprint,
                s.tool_use_loaded_key,
                true,
            ));
        }
        // §4.10: words are a secret, so the one scanner that expects
        // them has no way in from the clipboard at all.
        if expect != Expect::Seed {
            ways.push(tile(
                ids::SCAN_PASTE,
                Icon::Paste,
                s.scan_paste,
                self.has_clipboard,
            ));
        }
        if OpenSigner::types_for(expect) {
            ways.push(tile(ids::SCAN_TYPE, Icon::Keyboard, s.scan_type, true));
        }
        ways
    }

    /// Whether the flow behind this scanner has a keyboard for the
    /// value, which is what the "Type" tile opens.
    pub(crate) fn types_for(expect: Expect) -> bool {
        matches!(
            expect,
            Expect::Seed
                | Expect::Address
                | Expect::Cosigner
                | Expect::Message
                | Expect::Hashes
                | Expect::Encodings
                | Expect::Descriptor
                | Expect::ConvertKey
                | Expect::Policy
        )
    }

    /// §4.9: "The expected type is in the title."
    fn scan_title(&self) -> &'static str {
        let s = self.strings();
        match self.scan.as_ref().map(ScanState::expect) {
            Some(Expect::Seed) => s.scan_title_seed,
            Some(Expect::Psbt) => s.scan_title_psbt,
            Some(Expect::Address) => s.scan_title_address,
            Some(Expect::Wallet) => s.scan_title_wallet,
            Some(Expect::Cosigner) => s.build_scan_title,
            Some(Expect::Message) => s.scan_title_message,
            Some(Expect::SignedMessage) => s.scan_title_signed,
            Some(Expect::Backup) => s.scan_title_backup,
            Some(Expect::Note) => s.note_title,
            Some(Expect::Transaction | Expect::CompareTransaction) => s.scan_title_psbt,
            Some(Expect::Hashes) => s.tools_hashes,
            Some(Expect::Encodings) => s.tools_encodings,
            Some(Expect::Descriptor) => s.tools_descriptor_checksum,
            Some(Expect::ConvertKey) => s.tools_convert_key,
            Some(Expect::Policy) => s.tools_miniscript,
            Some(Expect::SilentPayment) => s.silent_check_row,
            Some(Expect::SilentPrevious) => s.silent_previous_row,
            _ => s.scan_title,
        }
    }

    /// §5 Result: a code was read and could not be used. The reason is
    /// the value of one row, in the two or three words §4.11 gives a
    /// reason, and "Try again" puts the camera back on.
    fn scan_not_usable(&self, c: &Chrome<'_>, reason: &str) -> Node {
        let s = self.strings();
        // A secret is not an unusable payload: it is one this device
        // will not take. The screen says which, and where it came from.
        let refused = reason == s.scan_reason_secret;
        let pasted = self.scan.as_ref().is_some_and(ScanState::pasted);
        let (title, result) = if refused {
            (s.scan_refused_title, s.scan_refused_result)
        } else {
            (self.scan_title(), s.scan_error_title)
        };
        screens::result(
            c,
            screens::Result {
                caption: None,
                title,
                icon: Icon::Error,
                tone: Tone::Danger,
                result,
                // The row names where the payload came from, so that a
                // refused paste does not report itself as a QR.
                rows: vec![components::Record::text(
                    if pasted { s.row_clipboard } else { s.row_qr },
                    reason,
                    Tone::Text,
                )],
                actions: vec![Action::new(ids::SCAN_AGAIN, s.action_again)],
            },
        )
    }

    /// §5 Menu, "Unknown QR": what was read, the bytes themselves one
    /// tap away as §4.5's reference row, and the four ways to read
    /// them: as a transaction, as text, as bytes to hash, and as a
    /// string to decode.
    fn scan_unknown(&self, c: &Chrome<'_>, scan: &ScanState) -> Node {
        let s = self.strings();
        let bytes = scan.unknown();
        screens::menu(
            c,
            s.scan_unknown_title,
            None,
            vec![
                // §4.11: how many bytes were read is a fact, not a
                // destination, so it is a table row above the rows that
                // open something rather than a menu row that opens
                // nothing.
                Row::Flat {
                    label: String::from(s.row_bytes),
                    value: alloc::format!("{}", bytes.len()),
                    mono: false,
                },
                Row::Reference {
                    id: ids::SCAN_HEX,
                    label: String::from(s.row_hex),
                    value: text::hex(bytes),
                },
                // §4.1 makes the icon optional, and the verb is what
                // the row is for: the label takes the panel's width.
                Row::Menu {
                    id: ids::SCAN_AS_PSBT,
                    icon: None,
                    label: String::from(s.scan_as_psbt),
                    value: None,
                    tone: Tone::Text,
                },
                Row::Menu {
                    id: ids::SCAN_AS_TEXT,
                    icon: None,
                    label: String::from(s.scan_as_text),
                    value: None,
                    tone: Tone::Text,
                },
                Row::Menu {
                    id: ids::SCAN_AS_HASHES,
                    icon: None,
                    label: String::from(s.scan_as_hashes),
                    value: None,
                    tone: Tone::Text,
                },
                Row::Menu {
                    id: ids::SCAN_AS_ENCODINGS,
                    icon: None,
                    label: String::from(s.scan_as_encodings),
                    value: None,
                    tone: Tone::Text,
                },
            ],
            Vec::new(),
        )
    }

    /// §5 Entry, "Backup passphrase": the one thing a scanned encrypted
    /// backup still needs. A passphrase that does not open it says so
    /// under the field and the entry stays, since the backup is still
    /// in hand.
    fn scan_backup_pass(&self, c: &Chrome<'_>, scan: &ScanState) -> Node {
        let s = self.strings();
        let n = scan.backup_pass_len();
        let mut masked: String = core::iter::repeat_n('\u{2022}', n.saturating_sub(1)).collect();
        match scan.backup_visible_char(self.now_ms) {
            Some(ch) => masked.push(ch),
            None if n > 0 => masked.push('\u{2022}'),
            None => {}
        }
        screens::entry(
            c,
            Entry {
                candidates: None,
                title: s.backup_pass_title,
                value: masked,
                mono: true,
                above: screens::Above::Nothing,
                words: None,
                eye: None,
                keyboard: (ids::SCAN_PASS_KEYBOARD, KeyboardKind::Passphrase),
                enabled: if scan.backup_pass_ready() {
                    ALL_KEYS
                } else {
                    DONE_DISABLED
                },
                error: scan
                    .backup_wrong()
                    .then(|| String::from(s.backup_wrong_pass)),
            },
        )
    }

    /// §5 Result: the code put the words in front of a camera. The
    /// caution is the coloured title and the count; Load is the way on
    /// and the chevron the way out (§2.1: no sentence repeats either).
    fn scan_words(&self, c: &Chrome<'_>, scan: &ScanState) -> Node {
        let s = self.strings();
        let words = scan.pending_words().unwrap_or(0);
        screens::result(
            c,
            screens::Result {
                caption: None,
                title: self.scan_title(),
                icon: Icon::Warning,
                tone: Tone::Caution,
                result: s.scan_words_title,
                rows: vec![components::Record::text(
                    s.load_words_row,
                    alloc::format!("{words}"),
                    Tone::Text,
                )],
                actions: vec![Action::new(ids::SCAN_CONTINUE, s.scan_words_load)],
            },
        )
    }
}
