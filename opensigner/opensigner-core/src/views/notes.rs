//! Notes, recovery sheets, and the form an export leaves the device in
//! (`docs/PLANNING.md` §16.112), built from `docs/DESIGN.md` §5: the
//! Menu of the notes in hand, the Entry one is typed on, the Document
//! that shows one whole, the Choice "Which form?", the Entry the
//! passphrase is typed on twice, and the Result and QR the sealed file
//! leaves on.
//!
//! The Document draws a note as text rather than as a secret panel: a
//! note is what a person reads, and a screen that hides it hides the
//! point of it. What hides a note is the passphrase it is exported
//! under.

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_codec::qr::{Ecc, Payload};
use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Above, Action, Entry, Item, Qr, Reading, Result, Row};
use osk_ui::widgets::keyboard::{ALL_KEYS, DONE_DISABLED, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::sign::Save;
use crate::{Form, FormFor, OpenSigner, WalletRef, ids, notes, strings};

impl OpenSigner {
    /// §5 Menu, "Notes": the note in hand, the notes kept on the
    /// device, the recovery sheets kept here whose wallet is not in
    /// use, then the two ways to a new one.
    pub(crate) fn view_notes(&self) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            let mut rows = Vec::new();
            if !self.note.is_empty() {
                rows.push(Row::Menu {
                    id: ids::at(ids::FORM_BASE, 0),
                    icon: Some(Icon::Receipt),
                    label: note_label(&self.note),
                    value: None,
                    tone: Tone::Text,
                });
            }
            for i in 0..self.kept_note_count() {
                if self.kept_note_in_hand(i) {
                    continue;
                }
                rows.push(Row::Menu {
                    id: ids::at(ids::NOTES_KEPT_BASE, i),
                    icon: Some(Icon::Receipt),
                    label: note_title(&self.kept_note_label(i)),
                    value: None,
                    tone: Tone::Text,
                });
            }
            // A kept sheet whose wallet has gone is still the document
            // an heir is left with, so it is listed here by the wallet
            // it names until that wallet is added again (§16.112 E3).
            for i in self.orphan_sheets() {
                rows.push(Row::Menu {
                    id: ids::at(ids::NOTES_SHEET_BASE, i),
                    icon: Some(Icon::Receipt),
                    label: components::elide(&self.orphan_sheet_label(i)),
                    value: Some(String::from(s.wallet_sheet_row)),
                    tone: Tone::Text,
                });
            }
            if rows.is_empty() {
                rows.push(Row::Fact {
                    label: String::from(s.tools_notes),
                    value: String::from(s.notes_none),
                    mono: false,
                    tone: Tone::Muted,
                });
            }
            rows.push(Row::Menu {
                id: ids::NOTES_NEW,
                icon: Some(Icon::Keyboard),
                label: String::from(s.notes_new),
                value: None,
                tone: Tone::Text,
            });
            rows.push(Row::Menu {
                id: ids::NOTES_READ_FILE,
                icon: Some(Icon::Scan),
                label: String::from(s.load_source_backup),
                value: None,
                tone: Tone::Text,
            });
            screens::menu(c, s.tools_notes, None, rows, vec![])
        })
    }

    /// §5 Entry, "Note": the text being typed, on the keyboard every
    /// other free text in the app is typed on. ✓ is dead while the
    /// field is empty (§4.3).
    pub(crate) fn view_note_text(&self) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            screens::entry(
                c,
                Entry {
                    candidates: None,
                    title: s.note_title,
                    value: String::from(self.note.text()),
                    mono: false,
                    above: Above::Nothing,
                    words: None,
                    eye: None,
                    keyboard: (ids::NOTE_KEYBOARD, KeyboardKind::Passphrase),
                    enabled: if self.note.is_empty() {
                        ALL_KEYS | DONE_DISABLED
                    } else {
                        ALL_KEYS
                    },
                    error: None,
                },
            )
        })
    }

    /// §5 Document, "Note": the text whole, how long it is, and the two
    /// things that can be done with it.
    pub(crate) fn view_note(&self) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            screens::reading(
                c,
                Reading {
                    title: s.note_title,
                    rows: {
                        let mut rows = vec![Row::Fact {
                            label: String::from(s.note_length_row),
                            value: alloc::format!("{}", self.note.chars()),
                            mono: false,
                            tone: Tone::Text,
                        }];
                        rows.extend(self.keep_toggle(ids::NOTE_KEEP, self.note_is_kept()));
                        rows
                    },
                    paragraphs: paragraphs(self.note.text()),
                    actions: vec![
                        Action::new(ids::NOTE_EXPORT, s.wallet_export),
                        Action::new(ids::NOTE_FORGET, s.note_forget),
                    ],
                },
            )
        })
    }

    /// §5 Document, "Recovery sheet": the wallet's descriptor, what the
    /// person calls it, and the note — the document an heir is left
    /// with.
    pub(crate) fn view_sheet(&self, wallet: WalletRef) -> Node {
        let s = self.strings();
        let descriptor = self.sheet_descriptor(wallet).unwrap_or_default();
        let name = self.wallet_name(wallet).map(String::from);
        let note = self.sheet_note_text(wallet);
        self.with_chrome(Some(ids::BACK), |c| {
            let mut rows = vec![Row::Reference {
                id: ids::SHEET_EXPORT,
                label: String::from(s.sheet_descriptor_row),
                value: descriptor.clone(),
            }];
            if let Some(name) = &name {
                rows.push(Row::Fact {
                    label: String::from(s.wallet_name),
                    value: name.clone(),
                    mono: false,
                    tone: Tone::Text,
                });
            }
            rows.push(Row::Menu {
                id: ids::SHEET_NOTE,
                icon: Some(Icon::Keyboard),
                label: String::from(s.sheet_write_note),
                value: Some(if note.is_empty() {
                    String::from(s.sheet_note_empty)
                } else {
                    alloc::format!("{}", note.chars().count())
                }),
                tone: Tone::Text,
            });
            rows.extend(self.keep_toggle(ids::SHEET_KEEP, self.sheet_is_kept(wallet)));
            rows.push(Row::Menu {
                id: ids::SHEET_READ,
                icon: Some(Icon::Scan),
                label: String::from(s.load_source_backup),
                value: None,
                tone: Tone::Text,
            });
            screens::reading(
                c,
                Reading {
                    title: s.wallet_sheet_row,
                    rows,
                    paragraphs: paragraphs(&note),
                    actions: vec![Action::new(ids::SHEET_EXPORT, s.wallet_export)],
                },
            )
        })
    }

    /// §5 Document: a recovery sheet that arrived encrypted and was
    /// opened. Its descriptor is a wallet this device can register, so
    /// the one action does that.
    pub(crate) fn view_opened_sheet(&self) -> Node {
        let s = self.strings();
        let (descriptor, name, note) = match &self.opened_sheet {
            Some(sheet) => (
                String::from_utf8_lossy(&sheet.descriptor).into_owned(),
                String::from_utf8_lossy(&sheet.name).into_owned(),
                String::from_utf8_lossy(&sheet.note).into_owned(),
            ),
            None => (String::new(), String::new(), String::new()),
        };
        let addable = self.opened_sheet_policy().is_some();
        self.with_chrome(Some(ids::BACK), |c| {
            let mut rows = vec![Row::Reference {
                id: ids::SHEET_ADD_WALLET,
                label: String::from(s.sheet_descriptor_row),
                value: descriptor.clone(),
            }];
            if !name.is_empty() {
                rows.push(Row::Fact {
                    label: String::from(s.wallet_name),
                    value: name.clone(),
                    mono: false,
                    tone: Tone::Text,
                });
            }
            let actions = if addable {
                vec![Action::new(ids::SHEET_ADD_WALLET, s.sheet_add_wallet)]
            } else {
                Vec::new()
            };
            screens::reading(
                c,
                Reading {
                    title: s.wallet_sheet_row,
                    rows,
                    paragraphs: paragraphs(&note),
                    actions,
                },
            )
        })
    }

    /// §5 Choice, "Which form?": how what is being exported leaves the
    /// device, each form with the programs that open it as its second
    /// line. The plain file is offered only for what holds no key.
    pub(crate) fn view_export_form(&self) -> Node {
        let s = self.strings();
        let (what, chosen) = match self.form {
            Some(pair) => pair,
            None => (FormFor::Note, Form::Oskb),
        };
        let items: Vec<Item> = Form::ALL
            .iter()
            .enumerate()
            .filter(|(_, form)| **form != Form::Plain || plain_offered(what))
            .map(|(i, form)| {
                let id = ids::at(ids::FORM_BASE, i);
                let label = match form {
                    Form::Oskb => s.export_form_oskb,
                    Form::Kdbx => s.export_form_kdbx,
                    Form::Plain => s.export_form_plain,
                };
                let mut item = Item::chosen(id, label, *form == chosen);
                item.subtitle = Some(String::from(form_opens(*form, s)));
                item
            })
            .collect();
        self.with_chrome(Some(ids::BACK), |c| {
            screens::choice(
                c,
                s.export_form_title,
                items,
                Action::new(ids::FORM_CONTINUE, s.action_continue),
            )
        })
    }

    /// §5 Entry: the passphrase a note or a sheet is sealed under, the
    /// same two typings a key's backup asks for.
    pub(crate) fn view_seal_pass(&self) -> Node {
        let s = self.strings();
        let n = self.seal_pass.len();
        let mut masked = String::new();
        for _ in 0..n.saturating_sub(1) {
            masked.push('\u{2022}');
        }
        match self.seal_pass.visible_char(self.now_ms) {
            Some(c) if n > 0 => masked.push(c),
            None if n > 0 => masked.push('\u{2022}'),
            _ => {}
        }
        self.with_chrome(Some(ids::BACK), |c| {
            screens::entry(
                c,
                Entry {
                    candidates: None,
                    title: if self.seal_pass.repeat() {
                        s.backup_pass_repeat
                    } else {
                        s.backup_pass_title
                    },
                    value: masked,
                    mono: true,
                    above: Above::Nothing,
                    words: None,
                    eye: None,
                    keyboard: (ids::SEAL_PASS_KEYBOARD, KeyboardKind::Passphrase),
                    enabled: if self.seal_pass.ready() {
                        ALL_KEYS
                    } else {
                        DONE_DISABLED
                    },
                    error: self
                        .seal_pass
                        .mismatch()
                        .then(|| String::from(s.backup_pass_mismatch)),
                },
            )
        })
    }

    /// §5 Result: the sealed file, how many bytes it is, the Argon2id
    /// memory it was made at, what it is sealed with and what reads it
    /// (§16.134 rule 2), and the ways off the device. A plain file is
    /// not sealed, so it states its size alone.
    pub(crate) fn view_sealed(&self) -> Node {
        let s = self.strings();
        let plain = self.form_is_plain();
        let mut rows = vec![components::Record::text(
            s.row_bytes,
            alloc::format!("{}", self.sealed.len()),
            Tone::Text,
        )];
        if !plain {
            rows.push(self.memory_record());
            rows.extend(self.encryption_records(&self.sealed, self.form_is_kdbx()));
        }
        let name = self.sealed_file_name();
        rows.extend(self.sealed_save_record(self.sealed_save, &name));
        let first = if self.sealed_save == Save::Written {
            Action::new(ids::SEAL_DONE, s.action_done)
        } else {
            Action::new(ids::SEAL_SAVE, s.action_save_file)
        };
        // A KDBX database is kilobytes and the apps that read one read
        // files, so the form that has no QR does not offer one; an
        // encrypted backup offers one where it fits one code.
        let mut actions = vec![first];
        if !plain && !self.form_is_kdbx() && self.sealed_fits_qr() {
            actions.push(Action::new(ids::SEAL_SHOW_QR, s.action_show_qr));
        }
        self.with_chrome(Some(ids::BACK), |c| {
            screens::result(
                c,
                Result {
                    caption: None,
                    title: s.seal_title,
                    icon: Icon::Success,
                    tone: Tone::Success,
                    result: s.seal_result,
                    rows,
                    actions,
                },
            )
        })
    }

    /// §5 QR: the sealed file in one code. The bytes are ciphertext, so
    /// the screen is a QR and not a secret panel.
    pub(crate) fn view_sealed_qr(&self) -> Node {
        let s = self.strings();
        let matrix = osk_codec::qr::encode(Payload::Bytes(&self.sealed), Ecc::Low)
            .ok()
            .map(Rc::new);
        self.with_chrome(Some(ids::BACK), |c| {
            let Some(matrix) = matrix.clone() else {
                return screens::result(
                    c,
                    Result {
                        caption: None,
                        title: s.seal_title,
                        icon: Icon::Error,
                        tone: Tone::Danger,
                        result: s.export_too_long,
                        rows: Vec::new(),
                        actions: vec![Action::new(ids::SEAL_DONE, s.action_done)],
                    },
                );
            };
            screens::qr(
                c,
                Qr {
                    title: s.seal_title,
                    matrix,
                    // §4.9: the label says what the code carries, never
                    // the title again.
                    label: s.export_form_oskb,
                    toggle: None,
                    animated: false,
                    forced: None,
                    progress: None,
                    save: self.png_row(),
                    caption: self.notice().map(String::from),
                },
            )
        })
    }

    /// The Result's Memory row: the Argon2id memory an encrypted export
    /// is made at.
    pub(crate) fn memory_record(&self) -> components::Record {
        let s = self.strings();
        components::Record::text(
            s.backup_memory_row,
            strings::fill1(
                s.backup_memory_value,
                &alloc::format!("{}", self.backup_memory_kib() / 1024),
            ),
            Tone::Text,
        )
    }

    /// The rows after Memory on every Result that sealed something
    /// (`docs/PLANNING.md` §16.134 rule 2): the format and its version,
    /// the cipher, the key derivation as the file's own header states
    /// it, and the programs that read it back.
    pub(crate) fn encryption_records(&self, bytes: &[u8], kdbx: bool) -> Vec<components::Record> {
        let s = self.strings();
        let (format, cipher, cost, opens) = if kdbx {
            (
                String::from(s.export_form_kdbx),
                s.sealed_cipher_kdbx,
                osk_backup::kdbx::cost_of(bytes),
                form_opens(Form::Kdbx, s),
            )
        } else {
            let version = bytes.get(4).copied().unwrap_or(osk_backup::oskb::VERSION);
            (
                strings::fill1(s.sealed_format_oskb, &alloc::format!("{version}")),
                s.sealed_cipher_oskb,
                osk_backup::oskb::read_header(bytes).ok(),
                form_opens(Form::Oskb, s),
            )
        };
        let mut rows = vec![
            components::Record::text(s.sealed_format_row, format, Tone::Text),
            components::Record::text(s.sealed_cipher_row, String::from(cipher), Tone::Text),
        ];
        if let Some(cost) = cost {
            let passes = match cost.passes {
                1 => String::from(s.sealed_kdf_one_pass),
                n => strings::fill1(s.sealed_kdf_passes, &alloc::format!("{n}")),
            };
            let lanes = match cost.lanes {
                1 => String::from(s.sealed_kdf_one_lane),
                n => strings::fill1(s.sealed_kdf_lanes, &alloc::format!("{n}")),
            };
            rows.push(components::Record::text(
                s.sealed_kdf_row,
                strings::fill(s.sealed_kdf_value, &[&passes, &lanes]),
                Tone::Text,
            ));
        }
        rows.push(components::Record::text(
            s.sealed_read_row,
            String::from(opens),
            Tone::Text,
        ));
        rows
    }

    /// §4.2 Toggle: whether this note or this sheet is in the blob's
    /// notes record. A device that keeps nothing has no record for it
    /// to be in, so the row is absent there; a record with all eight
    /// slots taken carries the row dead, with the count as its reason
    /// (`docs/PLANNING.md` §16.112 pass E3).
    fn keep_toggle(&self, id: ids::Id, on: bool) -> Option<Row> {
        let s = self.strings();
        if !self.wallet_keep_offered() {
            return None;
        }
        Some(Row::Toggle {
            id,
            label: String::from(s.wallet_keep_row),
            on,
            reason: (!on && self.kept_items_full()).then(|| String::from(s.notes_keep_full)),
        })
    }

    /// The row a Result carries for a file the shell was asked to
    /// write, whichever flow asked.
    fn sealed_save_record(&self, save: Save, name: &str) -> Vec<components::Record> {
        let s = self.strings();
        match save {
            Save::Idle => Vec::new(),
            Save::Waiting => vec![components::Record::text(
                s.sign_file_row,
                String::from(s.sign_waiting),
                Tone::Muted,
            )],
            Save::Written => vec![components::Record::text(
                s.sign_file_row,
                strings::fill1(s.sign_saved_as, name),
                Tone::Text,
            )],
            Save::Failed => vec![components::Record::text(
                s.sign_file_row,
                String::from(s.sign_not_saved),
                Tone::Caution,
            )],
        }
    }
}

/// The programs that open `form`, which is its row's second line on
/// "Which form?" and the "Read with" value on the Result.
fn form_opens(form: Form, s: &strings::Strings) -> &'static str {
    match form {
        Form::Oskb => s.export_form_oskb_opens,
        Form::Kdbx => s.export_form_kdbx_opens,
        Form::Plain => s.export_form_plain_opens,
    }
}

/// Whether a plain text file is one of the forms: a note and a sheet
/// hold no key, and nothing that holds one is offered in the clear.
fn plain_offered(what: FormFor) -> bool {
    !matches!(what, FormFor::Backup(_))
}

/// The row that names the note in hand: its first line, cut to what a
/// row holds.
fn note_label(note: &notes::Note) -> String {
    note_title(note.text())
}

/// How many characters of a note's first line name it on Notes: two lines
/// of a row's label on the panel.
const NOTE_TITLE_CHARS: usize = 40;

/// A note's name on Notes: its first line, cut after the last whole word
/// that fits [`NOTE_TITLE_CHARS`] with an ellipsis. A note is words, so it
/// is not elided the way a string compared character by character is.
fn note_title(text: &str) -> String {
    let line = text.lines().next().unwrap_or("").trim();
    if line.chars().count() <= NOTE_TITLE_CHARS {
        return String::from(line);
    }
    let cut = line
        .char_indices()
        .nth(NOTE_TITLE_CHARS)
        .map_or(line.len(), |(i, _)| i);
    let head = &line[..cut];
    let head = head.rfind(' ').map_or(head, |i| &head[..i]).trim_end();
    alloc::format!("{head}{}", osk_ui::tokens::ELLIPSIS)
}

/// The text as paragraphs, one per line, so a note keeps the shape it
/// was written in.
fn paragraphs(text: &str) -> Vec<String> {
    text.lines().map(String::from).collect()
}
