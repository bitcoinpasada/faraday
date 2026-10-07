//! The signed-message screens, built from `docs/DESIGN.md` §5: the
//! Document that shows the message whole with the key, the address and
//! the format above it, the Hold that signs, the Result, the QR the
//! signature travels on, and the Result a checked message gets.
//!
//! Reads [`SignMessage`] and [`CheckMessage`] through their accessors;
//! nothing here mutates. §4.5 gives every long string — the address, the
//! signature — its reference row.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::keys::ScriptType;
use osk_psbt::message::Format;
use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Above, Action, Chrome, Entry, Hold, Qr, Reading, Row};
use osk_ui::widgets::keyboard::{ALL_KEYS, DONE_DISABLED, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::message::{CheckMessage, SignMessage, Stage};
use crate::sign::Save;
use crate::strings::Strings;
use crate::{OpenSigner, ids, strings, text};

impl OpenSigner {
    pub(crate) fn view_sign_message(&self, flow: &SignMessage) -> Node {
        self.with_chrome(Some(ids::BACK), |c| match flow.stage() {
            Stage::Typing => self.message_typing(c, flow),
            Stage::Message => self.message_document(c, flow),
            Stage::Confirm => self.message_confirm(c, flow),
            Stage::Result => self.message_result(c, flow),
            Stage::Qr => self.message_qr(c, flow),
        })
    }

    pub(crate) fn view_check_message(&self, check: &CheckMessage) -> Node {
        self.with_chrome(Some(ids::BACK), |c| {
            if check.reading() {
                self.checked_document(c, check)
            } else {
                self.checked_result(c, check)
            }
        })
    }

    /// §5 Entry, "Sign a message": the text being typed, on the
    /// keyboard that takes every character a message can hold. ✓ is
    /// dead while the field is empty (§4.3).
    fn message_typing(&self, c: &Chrome<'_>, flow: &SignMessage) -> Node {
        let s = self.strings();
        screens::entry(
            c,
            Entry {
                candidates: None,
                title: s.msg_title,
                value: String::from(flow.text()),
                mono: false,
                above: Above::Nothing,
                words: None,
                eye: None,
                keyboard: (ids::MSG_KEYBOARD, KeyboardKind::Passphrase),
                enabled: if flow.text().trim().is_empty() {
                    ALL_KEYS | DONE_DISABLED
                } else {
                    ALL_KEYS
                },
                error: None,
            },
        )
    }

    /// §5 Document: the message whole, with what it will be signed with
    /// as rows above it. §4.2 puts each mode on its own value row, so the
    /// key, the script type and the format each open their Choice, and
    /// §4.12 keeps what a message signature proves on a Learn page.
    fn message_document(&self, c: &Chrome<'_>, flow: &SignMessage) -> Node {
        let s = self.strings();
        let mut rows = Vec::new();
        // §4.4: a chooser of one key states nothing, so the row is drawn
        // only where more than one key could sign.
        if self.keys.len() > 1 {
            rows.push(Row::Fingerprint {
                id: Some(ids::MSG_KEY),
                label: String::from(s.sign_key_row),
                value: self
                    .keys
                    .get(flow.key())
                    .map(|k| text::fingerprint_hex(k.fingerprint))
                    .unwrap_or_default(),
            });
        }
        rows.push(Row::Value {
            id: ids::MSG_SCRIPT,
            label: String::from(s.script_type_row),
            value: String::from(text::script_short(flow.script(), s)),
            tone: Tone::Text,
        });
        rows.push(Row::Reference {
            id: ids::MSG_ADDRESS,
            label: String::from(s.row_address),
            value: self.message_address(flow),
        });
        if flow.format_offered() {
            rows.push(Row::Value {
                id: ids::MSG_FORMAT,
                label: String::from(s.msg_format_row),
                value: String::from(format_name(flow.format(), s)),
                tone: Tone::Text,
            });
        }
        screens::reading(
            c,
            Reading {
                title: s.msg_title,
                rows,
                paragraphs: paragraphs(flow.text()),
                actions: vec![Action::new(ids::MSG_CONTINUE, s.action_continue)],
            },
        )
    }

    /// §5 Hold: what will be signed for, as a table, and the hold.
    /// §4.13 puts no sentence above it.
    fn message_confirm(&self, c: &Chrome<'_>, flow: &SignMessage) -> Node {
        let s = self.strings();
        let mut rows = vec![
            components::Record::fingerprint(
                s.sign_key_row,
                self.keys
                    .get(flow.key())
                    .map(|k| text::fingerprint_hex(k.fingerprint))
                    .unwrap_or_default(),
            ),
            components::Record::reference(
                ids::MSG_ADDRESS,
                s.row_address,
                self.message_address(flow),
            ),
            components::Record::text(s.msg_format_row, format_name(flow.format(), s), Tone::Text),
        ];
        if let Some(badge) = components::network_badge(text::network(self.network)) {
            rows.insert(0, components::Record::badge(badge));
        }
        screens::hold(
            c,
            Hold {
                warnings: Vec::new(),
                title: s.sign_confirm_title,
                rows,
                sign_with: None,
                then_with: None,
                id: ids::MSG_HOLD,
                label: s.sign_hold,
                danger: false,
                enabled: true,
                secondary: None,
            },
        )
    }

    /// §5 Result: the signature, the address it is for and the format it
    /// is in, with the two ways out §4.13 gives it — the QR a verifier
    /// reads, and the file.
    fn message_result(&self, c: &Chrome<'_>, flow: &SignMessage) -> Node {
        let s = self.strings();
        let Some(out) = flow.outcome() else {
            return self.message_confirm(c, flow);
        };
        let mut rows = vec![
            components::Record::reference(ids::MSG_ADDRESS, s.row_address, out.address.clone()),
            components::Record::text(s.msg_format_row, format_name(out.format, s), Tone::Text),
            components::Record::reference(
                ids::MSG_SIGNATURE,
                s.sign_signature,
                out.signature.clone(),
            ),
        ];
        match out.save {
            Save::Idle => {}
            Save::Waiting => rows.push(components::Record::text(
                s.sign_file_row,
                String::from(s.sign_waiting),
                Tone::Muted,
            )),
            Save::Written => rows.push(components::Record::text(
                s.sign_file_row,
                strings::fill1(s.sign_saved_as, crate::message::SIGNATURE_FILE),
                Tone::Text,
            )),
            Save::Failed => rows.push(components::Record::text(
                s.sign_file_row,
                String::from(s.sign_not_saved),
                Tone::Caution,
            )),
        }
        rows.extend(self.copy_record());
        let second = if out.save == Save::Written {
            Action::new(ids::MSG_DONE, s.action_done)
        } else {
            Action::new(ids::MSG_SAVE, s.action_save_file)
        };
        screens::result(
            c,
            screens::Result {
                caption: self.notice().map(String::from),
                title: s.msg_title,
                icon: Icon::Success,
                tone: Tone::Success,
                result: s.msg_signed_title,
                rows,
                actions: vec![second, Action::new(ids::MSG_QR, s.action_show_qr)],
            },
        )
    }

    /// §5 QR: the signature as one code. A message signature is short
    /// enough for one, so the Animated row is dimmed with the reason
    /// §4.11 asks for rather than offering a switch that does nothing.
    fn message_qr(&self, c: &Chrome<'_>, flow: &SignMessage) -> Node {
        let s = self.strings();
        let Some(matrix) = flow.matrix() else {
            return self.message_result(c, flow);
        };
        screens::qr(
            c,
            Qr {
                title: s.msg_qr_title,
                matrix,
                label: s.msg_qr_label,
                toggle: Some((ids::MSG_QR_ANIMATED, s.sign_qr_animated)),
                animated: false,
                forced: Some(String::from(s.msg_qr_one_code)),
                progress: None,
                save: self.png_row(),
                caption: self.notice().map(String::from),
            },
        )
    }

    /// §5 Result: whether the signature holds, the address and format it
    /// was read as, and the row that opens the message itself.
    fn checked_result(&self, c: &Chrome<'_>, check: &CheckMessage) -> Node {
        let s = self.strings();
        let mut rows = vec![components::Record::reference(
            ids::MSG_ADDRESS,
            s.row_address,
            String::from(check.address()),
        )];
        if let Some(checked) = check.checked() {
            rows.push(components::Record::text(
                s.msg_format_row,
                format_name(checked.format, s),
                Tone::Text,
            ));
            rows.push(components::Record::text(
                s.msg_checked_row,
                text::script_short(checked.script, s),
                Tone::Text,
            ));
        }
        rows.push(components::Record::value(
            ids::MSG_READ,
            s.msg_message_row,
            characters(check.message(), s),
        ));
        let (icon, tone, title) = match check.checked() {
            Some(_) => (Icon::Success, Tone::Success, s.msg_valid_title),
            None => (Icon::Error, Tone::Danger, s.msg_invalid_title),
        };
        screens::result(
            c,
            screens::Result {
                caption: None,
                title: s.msg_check_row,
                icon,
                tone,
                result: title,
                rows,
                // §5 Result is a terminal screen with no action at all,
                // and this one is not the end of anything: Done is the
                // way back to Home the chevron also is.
                actions: vec![Action::new(ids::MSG_DONE, s.action_done)],
            },
        )
    }

    /// §5 Document: the message a check was over, with the address it
    /// was checked against above it.
    fn checked_document(&self, c: &Chrome<'_>, check: &CheckMessage) -> Node {
        let s = self.strings();
        let rows = vec![Row::Reference {
            id: ids::MSG_ADDRESS,
            label: String::from(s.row_address),
            value: String::from(check.address()),
        }];
        screens::reading(
            c,
            Reading {
                title: s.msg_title,
                rows,
                paragraphs: paragraphs(check.message()),
                actions: Vec::new(),
            },
        )
    }
}

/// The message as the Document's paragraphs: one per line, so the shape
/// it was written in survives the screen.
fn paragraphs(text: &str) -> Vec<String> {
    text.split('\n')
        .filter(|line| !line.trim().is_empty())
        .map(String::from)
        .collect()
}

/// How long the message is, which is what the row that opens it states.
fn characters(text: &str, s: &Strings) -> String {
    strings::fill1(
        s.inspect_length,
        &alloc::format!("{}", text.chars().count()),
    )
}

/// The name of a signature format, which is both its Choice row and
/// what the result states.
pub(crate) fn format_name(format: Format, s: &Strings) -> &'static str {
    match format {
        Format::Bip137 => s.msg_bip137,
        Format::Bip322 => s.msg_bip322,
    }
}

/// §4.11: BIP-322's "simple" signature has no form for a nested-segwit
/// address, so the row is dimmed with the reason rather than left off.
pub(crate) fn format_reason(
    format: Format,
    script: ScriptType,
    s: &Strings,
) -> Option<&'static str> {
    (!Format::for_script(script).contains(&format)).then_some(s.msg_bip322_nested)
}
