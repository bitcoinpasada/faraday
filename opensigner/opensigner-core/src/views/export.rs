//! The export of a key or of a wallet, built from `docs/DESIGN.md` §5: a
//! Menu of what the export is, the string itself as a reference row, and
//! the QR screen the last row opens.
//!
//! The format and the script type are value rows: §4.2 shows a mode as
//! its value and opens the Choice screen for it. The path is a fact of
//! the key, so it is a value row with nothing behind it.

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::keys::ScriptType;
use osk_codec::qr::{Ecc, Payload};
use osk_ui::layout::Node;
use osk_ui::screens::{self, Above, Chrome, Entry, FactRow, Qr, Row};
use osk_ui::widgets::keyboard::{ALL_KEYS, DONE_DISABLED, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::views::detail::account_path;
use crate::{Account, ExportFormat, Exported, KeyExportStep, OpenSigner, WalletRef, ids, text};

impl OpenSigner {
    pub(crate) fn view_export(&self, owner: Exported) -> Node {
        self.with_chrome(Some(ids::BACK), |c| {
            let s = self.strings();
            let e = &self.export;
            let format = self.export_format();
            let title = match owner {
                Exported::Account(..) => s.export_key_title,
                Exported::Wallet(_) => s.export_title,
            };
            // §16.113: the scan descriptor carries the scan private
            // key, so the row that shows it opens a §5 Secret screen
            // with the eye, and the format row stays where it is.
            if format == ExportFormat::SilentScan
                && let Exported::Wallet(WalletRef::Policy(wallet)) = owner
                && self.silent_of(wallet).is_some()
            {
                let rows = vec![
                    Row::Value {
                        id: ids::EXPORT_FORMAT,
                        label: String::from(s.export_format_row),
                        value: String::from(format_name(format, s)),
                        tone: Tone::Text,
                    },
                    Row::Menu {
                        id: ids::SILENT_SECRET_ROW,
                        icon: Some(Icon::Eye),
                        label: String::from(s.export_silent_scan),
                        value: None,
                        tone: Tone::Text,
                    },
                ];
                return screens::menu(c, title, None, rows, Vec::new());
            }
            let Some(value) = self.export_value(owner) else {
                // A key whose account is still being derived says so; a
                // wallet no longer in use has an empty screen and the
                // chevron out (§4.14).
                return screens::menu(c, title, None, Vec::new(), Vec::new());
            };
            if e.qr {
                return self.export_qr(c, title, &value);
            }
            let mut rows = vec![Row::Value {
                id: ids::EXPORT_FORMAT,
                label: String::from(s.export_format_row),
                value: String::from(format_name(format, s)),
                tone: Tone::Text,
            }];
            // A key's account states its path, which is what a
            // coordinator pairs with the key (§16.110 rule 1); a
            // wallet's script type is a fact of its template and its
            // keys carry their own origins, so neither is a choice
            // here, and a wallet over one key states its path too.
            if let Exported::Account(_, account) = owner {
                rows.push(Row::Fact {
                    label: String::from(s.row_path),
                    value: alloc::format!(
                        "{} \u{00b7} {}",
                        account.path(self.network),
                        self.network.name()
                    ),
                    mono: true,
                    tone: Tone::Text,
                });
            }
            if let Exported::Wallet(WalletRef::Policy(wallet)) = owner
                && let Some(policy) = self.wallets.get(wallet)
            {
                let script = policy.script_type();
                rows.push(Row::Fact {
                    label: String::from(s.script_type_row),
                    value: String::from(text::script_short(script, s)),
                    mono: false,
                    tone: Tone::Text,
                });
                if matches!(policy.template(), osk_bip::policy::Template::Single { .. }) {
                    rows.push(Row::Fact {
                        label: String::from(s.row_path),
                        value: alloc::format!(
                            "{} \u{00b7} {}",
                            account_path(script, self.network),
                            self.network.name()
                        ),
                        mono: true,
                        tone: Tone::Text,
                    });
                }
            }
            let tail = vec![
                // A descriptor is read by its parts, so its row names the
                // parts a person checks (function, origin fingerprint,
                // checksum) rather than a head and tail cut mid-token;
                // an account key is one string and elides as one.
                if format == ExportFormat::Descriptor {
                    Row::MonoValue {
                        id: ids::EXPORT_TEXT,
                        label: String::from(format_name(format, s)),
                        value: text::descriptor_summary(&value),
                    }
                } else {
                    Row::Reference {
                        id: ids::EXPORT_TEXT,
                        label: String::from(format_name(format, s)),
                        value,
                    }
                },
                Row::Menu {
                    id: ids::EXPORT_SHOW,
                    icon: Some(Icon::Qr),
                    label: String::from(s.action_show_qr),
                    value: None,
                    tone: Tone::Text,
                },
            ];
            rows.extend(tail);
            // §16.114: the Core import is a file a person carries to
            // their node, so the format that writes one offers the save
            // and states what became of it.
            if format == ExportFormat::CoreImport {
                rows.push(Row::Action {
                    id: ids::EXPORT_SAVE,
                    icon: Icon::Download,
                    label: String::from(s.action_save_file),
                });
                if let Some(state) = self.export_save_row() {
                    rows.push(state);
                }
            }
            // §4.10: an export is public, so a row copies it.
            if let Some(row) = self.copy_row() {
                rows.push(match row.reason {
                    Some(reason) => Row::Dimmed {
                        icon: Some(row.icon),
                        label: row.label,
                        reason: Some(reason),
                    },
                    None => Row::Action {
                        id: row.id,
                        icon: row.icon,
                        label: row.label,
                    },
                });
            }
            if let Some(notice) = self.notice() {
                rows.push(Row::Flat {
                    label: String::from(s.action_copy),
                    value: String::from(notice),
                    mono: false,
                });
            }
            screens::menu(c, title, None, rows, Vec::new())
        })
    }

    /// §5 QR, "Wallet export": the code a coordinator scans, with the
    /// format it carries as its label.
    ///
    /// §4.9: "The wallet export animates on the panel like any other
    /// payload." One code where its modules clear the pitch floor at the
    /// class's side on this display, the BC-UR parts where they do not,
    /// with the toggle dimmed and the reason "too dense" as the
    /// transaction's is.
    fn export_qr(&self, c: &Chrome<'_>, title: &'static str, value: &str) -> Node {
        let s = self.strings();
        let e = &self.export;
        let format = self.export_format();
        let matrix = match &e.run {
            Some(run) => run.matrix(),
            None => osk_codec::qr::encode(Payload::Bytes(value.as_bytes()), Ecc::Low)
                .ok()
                .map(Rc::new),
        };
        let Some(matrix) = matrix else {
            return screens::result(
                c,
                screens::Result {
                    caption: None,
                    title,
                    icon: Icon::Error,
                    tone: Tone::Caution,
                    result: s.export_too_long,
                    rows: Vec::new(),
                    actions: Vec::new(),
                },
            );
        };
        let progress = e
            .run
            .as_ref()
            .map(|run| run.part())
            .filter(|(_, parts)| *parts > 0)
            .map(|(part, parts)| {
                let shown = (part - 1) % parts + 1;
                (
                    shown as f32 / parts as f32,
                    crate::strings::fill(
                        s.words_page,
                        &[&alloc::format!("{shown}"), &alloc::format!("{parts}")],
                    ),
                )
            });
        screens::qr(
            c,
            Qr {
                title,
                matrix,
                // §4.9: the label under the square names what it is,
                // "never the screen's own title again", so it is the
                // format the coordinator is being handed.
                label: format_name(format, s),
                toggle: Some((ids::EXPORT_ANIMATED, s.sign_qr_animated)),
                animated: e.animated,
                forced: (!e.scans).then(|| String::from(s.sign_qr_dense)),
                progress,
                save: self.png_row(),
                caption: self.notice().map(String::from),
            },
        )
    }
}

impl OpenSigner {
    /// What became of the Core import file's Save, as the row under it:
    /// nothing before it is asked for, then waiting, the name it was
    /// written under, or that it was not written (§5 Menu).
    fn export_save_row(&self) -> Option<Row> {
        let s = self.strings();
        let (value, mono) = match self.export_save {
            crate::sign::Save::Idle => return None,
            crate::sign::Save::Waiting => (String::from(s.sign_waiting), false),
            crate::sign::Save::Written => (
                crate::strings::fill1(s.sign_saved_as, &self.core_import_file_name()),
                true,
            ),
            crate::sign::Save::Failed => (String::from(s.sign_not_saved), false),
        };
        Some(Row::Flat {
            label: String::from(s.sign_file_row),
            value,
            mono,
        })
    }
}

/// The name of an export format, which is both its Choice row and what
/// the coordinator is handed.
pub(crate) fn format_name(format: ExportFormat, s: &crate::strings::Strings) -> &'static str {
    match format {
        ExportFormat::Descriptor => s.export_descriptor,
        ExportFormat::Xpub => s.export_xpub,
        ExportFormat::Slip132 => s.export_slip132,
        ExportFormat::Policy => s.export_policy,
        ExportFormat::Bsms => s.export_bsms,
        ExportFormat::BsmsSigner => s.export_bsms_signer,
        ExportFormat::CoreImport => s.export_core_import,
        ExportFormat::SilentScan => s.export_silent_scan,
        ExportFormat::SilentUri => s.export_silent_uri,
        ExportFormat::SilentDns => s.export_silent_dns,
    }
}

impl OpenSigner {
    /// Whether this export format is offered for what is being exported.
    /// A key is handed one of the three forms of its account key; a
    /// wallet is handed its descriptor or its policy text, which no
    /// single key has. A wallet whose key came with no origin has no
    /// BIP-388 form, since the standard writes an origin on every key,
    /// so the policy is not among its formats; a threshold wallet does
    /// have a text of its own, which is its group record, and that is
    /// what its policy row hands over. SLIP-132 has no
    /// prefix for P2TR and will not get one — a Taproot key is handed a
    /// descriptor — so §4.11's dimmed reason does not apply in any of
    /// these cases: the condition is not one the person could change,
    /// and the row is left off the list.
    pub(crate) fn format_offered(
        &self,
        format: ExportFormat,
        owner: Exported,
        script: ScriptType,
    ) -> bool {
        let _ = script;
        let owner = match owner {
            // §16.110 rule 1: every account is handed over as the
            // account key with its origin; a single-signature account
            // also has the SLIP-132 form, and a BIP-48 account has BIP
            // 129's key record. SLIP-132 has no prefix for P2TR.
            Exported::Account(_, account) => {
                return match (format, account) {
                    (ExportFormat::Xpub, _) => true,
                    (ExportFormat::Slip132, Account::Single(script)) => {
                        script != ScriptType::Taproot
                    }
                    (ExportFormat::BsmsSigner, Account::Multi(_)) => true,
                    _ => false,
                };
            }
            Exported::Wallet(owner) => owner,
        };
        match owner {
            WalletRef::Typed | WalletRef::Key(_) => false,
            WalletRef::Policy(wallet) => {
                let Some(policy) = self.wallets.get(wallet) else {
                    return false;
                };
                // §16.113: a silent payments wallet has three forms and
                // none of anyone else's. It has no descriptor to hand
                // over — BIP-392's is the scan private key — and no
                // chain of addresses for Bitcoin Core to watch.
                let silent = matches!(
                    format,
                    ExportFormat::SilentScan | ExportFormat::SilentUri | ExportFormat::SilentDns
                );
                if policy.silent().is_some() {
                    return silent;
                }
                if silent {
                    return false;
                }
                // A wallet over one key this device holds is one
                // account, so the two account-key forms stand beside
                // the descriptor; SLIP-132 has no prefix for P2TR and
                // never will (§15 item 35).
                // BIP 129 covers the multisig wallets a coordinator sets
                // up: `wsh(sortedmulti(…))` and its nested form, which
                // is what its descriptor record can carry.
                // Round 1 is a key's, not a wallet's, and it is served
                // from the key page (§16.110 rule 1).
                if format == ExportFormat::BsmsSigner {
                    return false;
                }
                // §16.114: every wallet with a descriptor is one Core
                // can be told to watch, and the import file is that
                // descriptor whatever the wallet's kind.
                if format == ExportFormat::CoreImport {
                    return true;
                }
                if format == ExportFormat::Bsms {
                    return matches!(policy.template(), osk_bip::policy::Template::Multi { .. })
                        && self.bsms_record(policy).is_some();
                }
                if let osk_bip::policy::Template::Single { script } = policy.template()
                    && self.single_account(policy).is_some()
                {
                    return match format {
                        ExportFormat::Policy => false,
                        ExportFormat::Slip132 => script != ScriptType::Taproot,
                        _ => true,
                    };
                }
                if format == ExportFormat::Policy {
                    return policy.record().is_some()
                        || policy.keys().iter().all(|k| k.fingerprint().is_some());
                }
                format == ExportFormat::Descriptor
            }
        }
    }
}

impl OpenSigner {
    /// A key's account for a coordinator (`docs/PLANNING.md` §16.110
    /// rules 1 and 2): the Export screen, and the two values BIP 129's
    /// key record is written from.
    pub(crate) fn view_key_export(
        &self,
        key: usize,
        account: Account,
        step: KeyExportStep,
    ) -> Node {
        match step {
            KeyExportStep::Export => self.view_export(Exported::Account(key, account)),
            KeyExportStep::Token => self.view_bsms_token(),
            KeyExportStep::Description => self.view_bsms_description(),
        }
    }

    /// §5 Entry, "Session token": the hex keyboard, with the preset row
    /// §4.3 puts above the field writing the token that says the
    /// session is not encrypted. ✓ is live on that token and on a nonce
    /// of either length BIP 129 defines.
    fn view_bsms_token(&self) -> Node {
        let s = self.strings();
        let token = self.bsms.token.clone();
        self.with_chrome(Some(ids::BACK), |c| {
            screens::entry(
                c,
                Entry {
                    candidates: None,
                    title: s.bsms_token_title,
                    value: token.clone(),
                    mono: true,
                    above: Above::Fact(FactRow {
                        id: Some(ids::BSMS_TOKEN_NONE),
                        label: String::from(s.bsms_token_none),
                        value: String::from(osk_bip::bsms::NO_ENCRYPTION),
                        mono: true,
                        glyph: false,
                    }),
                    words: None,
                    eye: None,
                    keyboard: (ids::BSMS_TOKEN_KEYBOARD, KeyboardKind::Hex),
                    enabled: if osk_bip::bsms::is_token(&token) {
                        ALL_KEYS
                    } else {
                        DONE_DISABLED
                    },
                    error: None,
                },
            )
        })
    }

    /// §5 Entry, "Description": the name keyboard, the fingerprint
    /// already in the field, and the error line where what is typed is
    /// longer than the eighty characters BIP 129 allows.
    fn view_bsms_description(&self) -> Node {
        let s = self.strings();
        let description = self.bsms.description.clone();
        let over = description.chars().count() > crate::BSMS_DESCRIPTION_MAX;
        self.with_chrome(Some(ids::BACK), |c| {
            screens::entry(
                c,
                Entry {
                    candidates: None,
                    title: s.bsms_description_title,
                    value: description.clone(),
                    mono: false,
                    above: Above::Nothing,
                    words: None,
                    eye: None,
                    keyboard: (ids::BSMS_DESCRIPTION_KEYBOARD, KeyboardKind::Passphrase),
                    enabled: if over { DONE_DISABLED } else { ALL_KEYS },
                    error: over.then(|| String::from(s.bsms_description_long)),
                },
            )
        })
    }
}

impl OpenSigner {
    /// §5 Secret, "Scan descriptor": BIP-392's `sp(spscan1q…)` whole,
    /// masked until a finger is on the panel or the eye is running.
    /// §4.10 keeps a secret off the clipboard and out of a code, so the
    /// screen carries the format row, the string and nothing else.
    pub(crate) fn view_silent_secret(&self, wallet: usize) -> Node {
        self.with_chrome(Some(ids::BACK), |c| self.silent_secret(c, wallet))
    }

    fn silent_secret(&self, c: &Chrome<'_>, wallet: usize) -> Node {
        let s = self.strings();
        let value = self.silent_descriptor(wallet).unwrap_or_default();
        screens::secret(
            c,
            screens::Secret {
                title: s.export_silent_scan,
                value: screens::Value::Text(&value),
                revealed: self.revealed(ids::SILENT_REVEAL),
                panel: ids::SILENT_REVEAL,
                eye: Some(ids::SECRET_EYE),
                remaining: self.reveal_ring(),
                rows: Vec::new(),
                action: None,
                secondary: None,
                pager: None,
            },
        )
    }
}
