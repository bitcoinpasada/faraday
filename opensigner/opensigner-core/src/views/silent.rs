//! A silent payments wallet's screens (`docs/PLANNING.md` §16.113),
//! built from `docs/DESIGN.md` §5: the Address the payer is given, the
//! Labels that are addresses of their own, the check that says whether
//! a transaction paid this wallet, and the two questions BIP-353's
//! record asks.

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::silent_wallet::MAX_LABELS;
use osk_codec::qr::{Ecc, Payload};
use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Above, Action, Chrome, Entry, Item, Result, Row};
use osk_ui::widgets::keyboard::{ALL_KEYS, DONE_DISABLED, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::silent::{AddressForm, Checked, DnsStep};
use crate::{OpenSigner, ids, strings};

impl OpenSigner {
    /// §5 Address: the wallet's address as a code and whole, with the
    /// Format row that shows the BIP-321 URI instead.
    pub(crate) fn view_silent_address(&self, wallet: usize, label: Option<u32>) -> Node {
        let s = self.strings();
        let title = match label {
            None => String::from(s.silent_address_row),
            Some(m) => strings::fill1(s.silent_label, &alloc::format!("{m}")),
        };
        self.with_chrome(Some(ids::BACK), |c| {
            let Some(value) = self.silent_shown(wallet, label) else {
                return screens::menu(c, &title, None, Vec::new(), Vec::new());
            };
            let Ok(matrix) = osk_codec::qr::encode(Payload::Bytes(value.as_bytes()), Ecc::Low)
            else {
                return screens::result(
                    c,
                    Result {
                        caption: None,
                        title: &title,
                        icon: Icon::Error,
                        tone: Tone::Caution,
                        result: s.export_too_long,
                        rows: Vec::new(),
                        actions: Vec::new(),
                    },
                );
            };
            screens::address(
                c,
                screens::Address {
                    copy: None,
                    format: Some((
                        ids::SILENT_FORM,
                        String::from(s.export_format_row),
                        String::from(form_name(self.silent_form, s)),
                    )),
                    save: self.address_png_row(c),
                    caption: self.notice().map(String::from),
                    title: &title,
                    label: s.silent_address_label,
                    address: (ids::ADDR_TEXT, &value),
                    code: Rc::new(matrix),
                },
            )
        })
    }

    /// §5 Menu, "Labels": one row per label handed out, each opening
    /// the same Address screen, and the row that hands out the next.
    pub(crate) fn view_silent_labels(&self, wallet: usize) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            let Some(record) = self.silent_of(wallet) else {
                return screens::menu(c, s.silent_labels_row, None, Vec::new(), Vec::new());
            };
            let labels = record.labels;
            let mut rows: Vec<Row> = Vec::new();
            for m in 1..=labels {
                let label = strings::fill1(s.silent_label, &alloc::format!("{m}"));
                match self.silent_address_text(wallet, Some(m)) {
                    Some(address) => rows.push(Row::Reference {
                        id: ids::at(ids::SILENT_LABEL_BASE, (m - 1) as usize),
                        label,
                        value: address,
                    }),
                    None => rows.push(Row::Dimmed {
                        icon: None,
                        label,
                        reason: Some(String::from(s.key_not_loaded)),
                    }),
                }
            }
            if labels < MAX_LABELS {
                rows.push(Row::Action {
                    id: ids::SILENT_ADD_LABEL,
                    icon: Icon::Qr,
                    label: String::from(s.silent_add_label),
                });
            } else {
                rows.push(Row::Dimmed {
                    icon: Some(Icon::Qr),
                    label: String::from(s.silent_add_label),
                    reason: Some(String::from(s.silent_labels_full)),
                });
            }
            screens::menu(c, s.silent_labels_row, None, rows, Vec::new())
        })
    }

    /// §5 Menu then §5 Result, "Check a payment": the way in, then the
    /// previous transaction an input still needs, then the answer.
    pub(crate) fn view_silent_check(&self, wallet: usize) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            match self
                .check_payment
                .as_ref()
                .and_then(crate::silent::Check::result)
            {
                Some(result) => self.silent_result(c, wallet, result),
                None => {
                    let mut rows = vec![Row::Menu {
                        id: ids::SILENT_READ,
                        icon: Some(Icon::Scan),
                        label: String::from(s.silent_read),
                        value: None,
                        tone: Tone::Text,
                    }];
                    // §16.113: a taproot input's public key is in the
                    // output it spends, so the flow asks for the
                    // transaction that made it, by the identifier the
                    // input names.
                    if let Some(txid) = self
                        .check_payment
                        .as_ref()
                        .and_then(crate::silent::Check::waiting_for)
                    {
                        rows.push(Row::Reference {
                            id: ids::SILENT_PREVIOUS,
                            label: String::from(s.silent_previous_row),
                            value: alloc::format!("{txid}"),
                        });
                    }
                    screens::menu(c, s.silent_check_row, None, rows, Vec::new())
                }
            }
        })
    }

    /// §5 Result: what the check came to, with one row per output that
    /// pays this wallet.
    fn silent_result(&self, c: &Chrome<'_>, wallet: usize, result: &Checked) -> Node {
        let s = self.strings();
        let _ = wallet;
        let (icon, tone, line) = match result {
            Checked::Paid(_) => (Icon::Success, Tone::Success, s.silent_paid),
            Checked::NotPaid => (Icon::Info, Tone::Text, s.silent_not_paid),
            Checked::NoInputs => (Icon::Warning, Tone::Caution, s.silent_no_inputs),
            Checked::Unsigned => (Icon::Warning, Tone::Caution, s.silent_unsigned),
            Checked::KeyNotLoaded => (Icon::Warning, Tone::Caution, s.silent_key_not_loaded),
        };
        let rows = match result {
            Checked::Paid(paid) => paid
                .iter()
                .map(|p| {
                    components::Record::text(
                        strings::fill1(s.silent_output_row, &alloc::format!("{}", p.vout)),
                        alloc::format!(
                            "{} \u{00b7} {}",
                            components::amount(p.amount.to_sat(), self.unit()),
                            match p.label {
                                Some(m) =>
                                    strings::fill1(s.silent_output_label, &alloc::format!("{m}")),
                                None => String::from(s.silent_output_plain),
                            }
                        ),
                        Tone::Text,
                    )
                })
                .collect(),
            _ => Vec::new(),
        };
        screens::result(
            c,
            Result {
                caption: None,
                title: s.silent_check_row,
                icon,
                tone,
                result: line,
                rows,
                actions: vec![Action::new(ids::SILENT_DONE, s.action_done)],
            },
        )
    }

    /// §5 Entry, "User name" then "Domain": the two names BIP-353's
    /// record is published under, typed on the name keyboard.
    pub(crate) fn view_silent_dns(&self, wallet: usize) -> Node {
        let s = self.strings();
        let _ = wallet;
        self.with_chrome(Some(ids::BACK), |c| {
            let (title, value) = match self.dns.step {
                DnsStep::User => (s.silent_user_title, self.dns.user.clone()),
                DnsStep::Domain => (s.silent_domain_title, self.dns.domain.clone()),
            };
            let empty = value.trim().is_empty();
            screens::entry(
                c,
                Entry {
                    candidates: None,
                    title,
                    value,
                    mono: true,
                    above: Above::Nothing,
                    words: None,
                    eye: None,
                    keyboard: (ids::SILENT_DNS_KEYBOARD, KeyboardKind::Passphrase),
                    enabled: if empty {
                        ALL_KEYS | DONE_DISABLED
                    } else {
                        ALL_KEYS
                    },
                    error: None,
                },
            )
        })
    }

    /// §5 Choice, "Which form?": the address itself, or the URI that
    /// carries it.
    pub(crate) fn silent_form_choice(&self, c: &Chrome<'_>, picked: usize) -> Node {
        let s = self.strings();
        let items: Vec<Item> = AddressForm::ALL
            .iter()
            .enumerate()
            .map(|(i, form)| {
                Item::chosen(ids::at(ids::PICK_BASE, i), form_name(*form, s), i == picked)
            })
            .collect();
        screens::choice(
            c,
            s.silent_form_title,
            items,
            Action::new(ids::PICK_CONTINUE, s.action_continue),
        )
    }
}

/// What a form is called, on its Choice row and on the value row that
/// opens it.
pub(crate) fn form_name(form: AddressForm, s: &crate::strings::Strings) -> &'static str {
    match form {
        AddressForm::Address => s.silent_form_address,
        AddressForm::Uri => s.silent_form_uri,
    }
}
