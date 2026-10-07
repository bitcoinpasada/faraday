//! The Sign screens, built from `docs/DESIGN.md` §5: the entry Menu, the
//! review Record, one Record per output, the inputs, the warnings, the
//! Hold that signs, the Result, the Signatures menu and the transaction
//! QR.
//!
//! Reads [`SignFlow`] through its accessors; nothing here mutates. Every
//! amount goes through §4.7's one unit, and every long string is §4.5's
//! reference row, which opens the Compare screen.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_psbt::bitcoin::Amount;
use osk_psbt::threshold::Refusal;
use osk_psbt::verify::{NonceMode, Unchecked, Verdict};
use osk_psbt::{
    InputInfo, Inspection, Level, MusigRole, OutputKind, SpendRoute, Warning, WarningKind,
};
use osk_ui::components::{self, OutputBadge};
use osk_ui::geom::SizeClass;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Action, Chrome, Hold, KeyContext, Qr};
use osk_ui::tokens;
use osk_ui::widgets::{Icon, Tone, WarningLevel};

use crate::sign::{QrMode, Save, SignFlow, SignatureRow, Stage, Step};
use crate::strings::Strings;
use crate::{OpenSigner, ids, strings, text};

/// The fingerprints joined as one value line, as §4.4 joins them.
fn joined(keys: &[String]) -> String {
    keys.join(" \u{00b7} ")
}

impl OpenSigner {
    /// The chip of one share of the threshold wallet this transaction
    /// spends. §4.4's chip is a fingerprint, and the row of them does
    /// not wrap, so a share is its own fingerprint here exactly as a key
    /// is its master's; which share it is is the record's business and
    /// the wallet page's.
    fn threshold_share_label(&self, insp: &Inspection, id: u32) -> String {
        let s = self.strings();
        let Some(t) = insp.threshold.as_ref() else {
            return String::from(s.value_none);
        };
        match self
            .wallets
            .get(t.wallet)
            .and_then(|p| p.record())
            .and_then(|r| r.share_fingerprint(id as usize))
        {
            Some(fp) => text::fingerprint_hex(fp),
            None => strings::fill(
                s.wallet_member,
                &[&alloc::format!("{}", id + 1), &alloc::format!("{}", t.n)],
            ),
        }
    }

    /// §4.7 Amount: every amount on a Sign screen in the one unit the
    /// Settings › Unit row names, and never an echo in the other
    /// (§2.9).
    fn amount(&self, a: Amount) -> String {
        components::amount(a.to_sat(), self.unit())
    }

    /// `100 000 sats · SegWit · 73c5da0a`: what is known about one
    /// input, as the value beside its number (§4.11).
    fn input_value(&self, i: &InputInfo, s: &Strings) -> String {
        let mut value = i
            .amount
            .map_or_else(|| String::from(s.value_none), |a| self.amount(a));
        value.push_str(&alloc::format!(
            " \u{00b7} {}",
            text::script_kind(i.script_type, s)
        ));
        if let Some(o) = &i.origin {
            value.push_str(&alloc::format!(" \u{00b7} {}", o.fingerprint));
        }
        // The way the transaction spends a miniscript or taproot-tree
        // wallet, where the PSBT says which way that is: the leaf it
        // carries a script for, the key path, or the one way through a
        // `wsh` script its keys and its sequence leave.
        match &i.spend_route {
            Some(SpendRoute::KeyPath) => {
                value.push_str(&alloc::format!(" \u{00b7} {}", s.wallet_path_keypath));
            }
            Some(SpendRoute::Script(path)) => {
                value.push_str(&alloc::format!(" \u{00b7} {}", text::spend_path(path, s)));
            }
            None => {}
        }
        value
    }

    /// §4.4 Key context on a review screen, naming the keys that sign
    /// this transaction and opening the transaction's own Keys review
    /// (`docs/PLANNING.md` §16.110 rule 3). A 268 dp panel spends its
    /// height on the table, so there the row is left off and Confirm
    /// names the key instead.
    ///
    /// Where no loaded key signs anything, the row names the keys the
    /// transaction asks for, which is what the review behind it is
    /// about. A transaction that names no key at all has no row.
    fn review_key_context(&self, insp: &Inspection, reading: bool) -> Option<KeyContext> {
        if self.class() == SizeClass::Small {
            return None;
        }
        let named = self.transaction_keys(reading);
        if named.is_empty() {
            return None;
        }
        let mut keys: Vec<String> = insp
            .participating_keys
            .iter()
            .map(|fp| text::fingerprint_hex(*fp))
            .collect();
        // A share of a threshold wallet is what signs where no key can,
        // and the review names it as it names a key (§16.103).
        if let Some(t) = insp.threshold.as_ref() {
            for id in &t.ours {
                keys.push(self.threshold_share_label(insp, *id));
            }
        }
        if keys.is_empty() {
            keys = named.into_iter().map(|(_, label, _)| label).collect();
        }
        Some(KeyContext {
            id: Some(ids::SIGN_KEYS),
            label: String::from(self.strings().sign_key_row),
            keys,
        })
    }

    pub(crate) fn view_sign(&self, flow: &SignFlow) -> Node {
        self.with_chrome(Some(ids::BACK), |c| match flow.stage() {
            Stage::Wizard(step) => match flow.inspection() {
                Some(insp) => match step {
                    Step::Summary => self.sign_summary(c, flow, insp),
                    Step::Outputs => self.sign_outputs(c, flow, insp),
                    Step::Inputs => self.sign_inputs(c, flow, insp),
                    Step::Warnings => self.sign_warnings(c, flow),
                    Step::Confirm => self.sign_confirm(c, flow, insp),
                    Step::Result => self.sign_result(c, flow, insp),
                    Step::Signatures => self.sign_signatures(c, flow),
                    Step::Qr => self.sign_qr(c, flow),
                },
                None => self.sign_entry(c, flow),
            },
            _ => self.sign_entry(c, flow),
        })
    }

    /// §5 Result: the bytes that arrived are not a transaction. There
    /// is no Sign entry any more — a transaction reaches the flow by
    /// being scanned or read from a file — so this is the only thing
    /// the screen shows outside the review wizard.
    fn sign_entry(&self, c: &Chrome<'_>, _flow: &SignFlow) -> Node {
        let s = self.strings();
        screens::result(
            c,
            screens::Result {
                caption: None,
                title: s.sign_title,
                icon: Icon::Error,
                tone: Tone::Danger,
                result: s.sign_parse_error_title,
                rows: vec![components::Record::text(
                    s.sign_read_row,
                    s.sign_not_psbt,
                    Tone::Danger,
                )],
                actions: vec![Action::new(ids::SIGN_BACK, s.action_again)],
            },
        )
    }

    /// The way on from a review page: Continue to the next one, and on
    /// the last page of a transaction that is only being read, Done,
    /// which leaves the tool. A signing review's first page is dead
    /// until a loaded key can sign something in the transaction.
    fn review_on(&self, flow: &SignFlow, step: Step, label: &'static str) -> Action {
        let s = self.strings();
        if flow.reading() {
            return if flow.last_review_step() == step {
                Action::new(ids::DECODE_DONE, s.action_done)
            } else {
                Action::new(ids::SIGN_CONTINUE, label)
            };
        }
        let live = step != Step::Summary || flow.participates();
        let action = Action::when(ids::SIGN_CONTINUE, label, live);
        if live {
            return action;
        }
        // Reading a transaction before loading a key is an ordinary
        // thing to do, and what to change is not on the screen unless
        // the dead action says it (§4.11).
        action.reason(if self.keys.is_empty() {
            s.sign_no_key
        } else {
            s.sign_keys_not_in
        })
    }

    /// §5 Record, "Review": what leaves, what it costs, where it goes,
    /// what comes back and how many inputs pay for it — one fact per row,
    /// with the recipient as the reference row §4.5 gives every address.
    fn sign_summary(&self, c: &Chrome<'_>, flow: &SignFlow, insp: &Inspection) -> Node {
        let s = self.strings();
        let (label, sent) = if insp.is_self_transfer {
            (s.sign_move, insp.total_out)
        } else {
            (s.sign_amount, insp.amount_to_others)
        };
        let mut rows = vec![components::Record::mono(label, self.amount(sent))];
        rows.extend(components::fee_rows(
            s.sign_fee,
            s.sign_rate,
            insp.fee.to_sat(),
            self.unit(),
            insp.fee_rate_sat_vb,
        ));
        if let Some(o) = insp.outputs.iter().find(|o| !o.is_ours()) {
            rows.push(components::Record::reference(
                ids::SIGN_TO,
                s.sign_to,
                o.address.clone(),
            ));
        }
        rows.extend(self.change_rows(flow, insp));
        rows.push(components::Record::text(
            s.sign_inputs,
            alloc::format!("{}", insp.inputs.len()),
            Tone::Text,
        ));
        // A transaction being read is identified by what it is: the id
        // the network will know it by, which a signing review has no
        // room for and no use for until it is signed.
        if let Some(txid) = flow.reading().then(|| flow.unsigned_txid()).flatten() {
            rows.push(components::Record::reference(
                ids::SIGN_TXID,
                s.sign_txid,
                alloc::format!("{txid}"),
            ));
        }
        screens::record(
            c,
            screens::Record {
                pager: None,
                title: s.sign_summary_title,
                key: self.review_key_context(insp, flow.reading()),
                network: text::network(self.network),
                rows,
                warnings: Vec::new(),
                action: Some(self.review_on(flow, Step::Summary, s.sign_review)),
            },
        )
    }

    /// §4.8 State value: what comes back to this wallet, in the tone the
    /// state deserves — the amount when the device derived the address
    /// from a loaded key, "not verified" in the danger tone when it could
    /// not. Not a badge: it is a state to act on.
    ///
    /// The verified case states itself too (§16.46), with §4.8's Mine
    /// badge under the amount, so that a person who never walks the
    /// per-output screens still reads the state on the review.
    fn change_rows(&self, flow: &SignFlow, insp: &Inspection) -> Vec<components::Record> {
        let s = self.strings();
        // A review that signs nothing and was given nothing to compare
        // against has not looked for change, so it states that rather
        // than a verdict about nothing.
        if flow.reading() && !flow.has_context() {
            return vec![components::Record::text(
                s.sign_change,
                s.decode_no_key,
                Tone::Muted,
            )];
        }
        let unverified = insp
            .outputs
            .iter()
            .any(|o| matches!(o.kind, OutputKind::UnverifiedChange { .. }));
        if unverified {
            return vec![components::Record::text(
                s.sign_change,
                s.sign_change_unverified,
                Tone::Danger,
            )];
        }
        if insp.change_total.to_sat() == 0 {
            return vec![components::Record::text(
                s.sign_change,
                s.value_none,
                Tone::Muted,
            )];
        }
        vec![
            components::Record::mono(s.sign_change, self.amount(insp.change_total)),
            components::Record::badge(components::output_badge(OutputBadge::Mine)),
        ]
    }

    /// §5 Record, "Output 1 of 2": the amount, the address as a reference
    /// row, and the badges §4.8 gives an output that pays a loaded key or
    /// is too small to spend. Continue walks the run and then leaves it,
    /// so §4.1's pager is not needed: the title already counts.
    fn sign_outputs(&self, c: &Chrome<'_>, flow: &SignFlow, insp: &Inspection) -> Node {
        let s = self.strings();
        let i = flow.output();
        let title = strings::fill(
            s.sign_output_title,
            &[
                &alloc::format!("{}", i + 1),
                &alloc::format!("{}", insp.outputs.len()),
            ],
        );
        let mut rows = Vec::new();
        if let Some(o) = insp.outputs.get(i) {
            rows.push(components::Record::mono(
                s.sign_amount,
                self.amount(o.amount),
            ));
            rows.push(components::Record::reference(
                ids::at(ids::SIGN_OUT_BASE, o.index),
                s.sign_to,
                o.address.clone(),
            ));
            match &o.kind {
                OutputKind::Change { change, index, .. }
                | OutputKind::WalletChange { change, index, .. } => {
                    rows.push(components::Record::badge(components::output_badge(
                        OutputBadge::Mine,
                    )));
                    rows.push(components::Record::mono(
                        s.row_path,
                        alloc::format!("{}/{}/{}", tokens::ELLIPSIS, u8::from(*change), index),
                    ));
                }
                OutputKind::UnverifiedChange { .. } if !flow.reading() || flow.has_context() => {
                    rows.push(components::Record::badge(components::output_badge(
                        OutputBadge::MineNotVerified,
                    )));
                }
                // With no key and nothing to sign, an output that names
                // a key this device does not hold is a payment to
                // someone else and carries no badge.
                OutputKind::UnverifiedChange { .. } => {}
                OutputKind::Recipient => {}
            }
            if o.is_dust {
                rows.push(components::Record::badge(components::output_badge(
                    OutputBadge::Dust,
                )));
            }
        }
        screens::record(
            c,
            screens::Record {
                pager: None,
                title: &title,
                key: None,
                network: text::network(self.network),
                rows,
                warnings: Vec::new(),
                action: Some(Action::new(ids::SIGN_CONTINUE, s.action_continue)),
            },
        )
    }

    /// §5 Record, "Inputs": what the transaction spends, the policy it
    /// spends it under, and one row per input.
    fn sign_inputs(&self, c: &Chrome<'_>, flow: &SignFlow, insp: &Inspection) -> Node {
        let s = self.strings();
        let ours = insp.inputs.iter().filter(|i| i.is_ours).count();
        let mut rows = vec![
            components::Record::text(
                s.sign_inputs,
                alloc::format!(
                    "{} \u{00b7} {}",
                    insp.inputs.len(),
                    strings::fill1(s.sign_inputs_ours, &alloc::format!("{ours}"))
                ),
                Tone::Text,
            ),
            components::Record::mono(s.sign_total_in, self.amount(insp.total_in)),
            components::Record::text(
                s.sign_rbf,
                if insp.rbf { s.value_yes } else { s.value_no },
                Tone::Text,
            ),
            components::Record::text(
                s.sign_locktime,
                alloc::format!("{}", insp.locktime),
                Tone::Text,
            ),
        ];
        if let Some(ms) = &insp.multisig {
            rows.push(components::Record::text(
                s.sign_multisig,
                strings::fill(
                    s.sign_multisig_value,
                    &[
                        &alloc::format!("{}", ms.m),
                        &alloc::format!("{}", ms.n),
                        &alloc::format!("{}", ms.signed()),
                    ],
                ),
                Tone::Text,
            ));
            let chips: Vec<(String, bool)> = ms
                .cosigners
                .iter()
                .map(|c| {
                    (
                        c.fingerprint
                            .map_or_else(|| String::from(s.value_none), text::fingerprint_hex),
                        c.ours,
                    )
                })
                .collect();
            rows.push(components::Record::line(
                s.sign_cosigners,
                components::key_chips(c.class(), &chips),
            ));
        }
        // A MuSig2 wallet has no quorum to state: the output carries one
        // key and one signature, so what the row says is how many keys
        // aggregate into it (§16.70).
        if let Some(m) = &insp.musig {
            rows.push(components::Record::text(
                s.wallet_musig,
                strings::fill1(s.wallet_musig_keys, &alloc::format!("{}", m.keys)),
                Tone::Text,
            ));
        }
        // §16.111 rule 1: the count of the signatures already on the
        // transaction, and the way to the page that says what each of
        // them is. Absent when the transaction carries none.
        if flow.signature_count() > 0 {
            rows.push(components::Record::value(
                ids::SIGN_SIGNATURES,
                s.sign_signatures,
                alloc::format!("{}", flow.signature_count()),
            ));
        }
        // §4.11: "One fact per row." An input is a row of the table like
        // every other: its number is the label, and what is known about
        // it is the value, which wraps to a second line where the value
        // column is too narrow for it.
        for i in &insp.inputs {
            // Mono: the value is an amount, a script and the key it pays
            // from, read character by character like the total above.
            rows.push(components::Record::mono(
                strings::fill1(s.sign_input_row, &alloc::format!("{}", i.index)),
                self.input_value(i, s),
            ));
        }
        screens::record(
            c,
            screens::Record {
                pager: None,
                title: s.sign_inputs_title,
                key: None,
                network: text::network(self.network),
                rows,
                warnings: Vec::new(),
                action: Some(self.review_on(flow, Step::Inputs, s.action_continue)),
            },
        )
    }

    /// §5 Record, "Warnings": the ranked cards, and the one toggle §4.11
    /// asks a danger for before the review may go on. A blocked warning
    /// is offered no toggle and the action stays dead: the review ends
    /// here.
    fn sign_warnings(&self, c: &Chrome<'_>, flow: &SignFlow) -> Node {
        let s = self.strings();
        let warnings: Vec<(WarningLevel, String, String)> = flow
            .ranked_warnings()
            .into_iter()
            .map(|w| {
                let (label, value) = warning_words(w, s);
                (level_of(w.level), label, value)
            })
            .collect();
        let rows = if flow.has_danger() && !flow.has_blocked() {
            vec![components::Record::toggle(
                ids::SIGN_ACK,
                s.sign_acknowledge,
                flow.acknowledged(),
            )]
        } else {
            Vec::new()
        };
        screens::record(
            c,
            screens::Record {
                pager: None,
                title: s.sign_warnings_title,
                key: None,
                network: text::network(self.network),
                rows,
                warnings,
                action: Some(if flow.reading() {
                    self.review_on(flow, Step::Warnings, s.action_continue)
                } else {
                    Action::when(
                        ids::SIGN_CONTINUE,
                        s.action_continue,
                        flow.can_acknowledge(),
                    )
                }),
            },
        )
    }

    /// §5 Hold, "Confirm": what will happen as a table, the keys that can
    /// sign as chips, and the hold. §4.13 puts no sentence above it.
    fn sign_confirm(&self, c: &Chrome<'_>, flow: &SignFlow, insp: &Inspection) -> Node {
        let s = self.strings();
        let (label, sent) = if insp.is_self_transfer {
            (s.sign_move, insp.total_out)
        } else {
            (s.sign_amount, insp.amount_to_others)
        };
        let mut rows = vec![
            components::Record::mono(label, self.amount(sent)),
            components::Record::mono(s.sign_fee, self.amount(insp.fee)),
        ];
        if let Some(o) = insp.outputs.iter().find(|o| !o.is_ours()) {
            rows.push(components::Record::reference(
                ids::SIGN_TO,
                s.sign_to,
                o.address.clone(),
            ));
        }
        // The network is the first thing in the block, as on Review:
        // a test network is read before the amount, not after the address.
        if let Some(badge) = components::network_badge(text::network(self.network)) {
            rows.insert(0, components::Record::badge(badge));
        }
        if let Some(e) = flow.error() {
            rows.push(components::Record::text(s.sign_failed, e, Tone::Danger));
        }
        let mut keys: Vec<screens::SigningKey> = insp
            .participating_keys
            .iter()
            .enumerate()
            .map(|(i, fp)| {
                (
                    ids::at(ids::SIGN_KEY_BASE, i),
                    text::fingerprint_hex(*fp),
                    flow.is_selected(*fp),
                )
            })
            .collect();
        // A share of a threshold wallet signs as a key does, and its
        // chip is its label; it is always on, because the device signs
        // with the share it holds (§16.103).
        let mine: Vec<u32> = insp
            .threshold
            .as_ref()
            .map_or_else(Vec::new, |t| t.ours.clone());
        for id in &mine {
            keys.push((
                ids::at(ids::SIGN_SHARE_BASE, *id as usize),
                self.threshold_share_label(insp, *id),
                true,
            ));
        }
        // §4.4: a row of one chip offers nothing, so it is drawn only
        // where more than one key could sign.
        let sign_with = (keys.len() > 1).then(|| (String::from(s.sign_sign_with), keys));
        // At the first location the other shares that will sign are
        // chosen here, and exactly `t - 1` of them.
        let then_with = flow.threshold_first().map(|_| {
            let chips: Vec<screens::SigningKey> = flow
                .threshold_candidates()
                .into_iter()
                .map(|id| {
                    (
                        ids::at(ids::SIGN_OTHER_BASE, id as usize),
                        self.threshold_share_label(insp, id),
                        flow.is_other(id),
                    )
                })
                .collect();
            (String::from(s.sign_then_with), chips)
        });
        screens::hold(
            c,
            Hold {
                warnings: Vec::new(),
                title: s.sign_confirm_title,
                rows,
                sign_with,
                then_with,
                id: ids::SIGN_HOLD,
                label: if flow.shares_nonce() {
                    s.sign_hold_share
                } else {
                    s.sign_hold
                },
                danger: false,
                enabled: flow.can_sign(),
                secondary: None,
            },
        )
    }

    /// §5 Result: what was signed, and the two ways out §4.13 gives it —
    /// the QR a coordinator reads, and the file or Done.
    fn sign_result(&self, c: &Chrome<'_>, flow: &SignFlow, insp: &Inspection) -> Node {
        let s = self.strings();
        let Some(out) = flow.outcome() else {
            return self.sign_confirm(c, flow, insp);
        };
        // A pass that wrote no signature and shared a nonce is round 1
        // of MuSig2: what it leaves behind is the open session
        // (§16.100).
        let shared = out.signatures.is_empty() && out.nonces_shared > 0;
        let (icon, tone, result) = match (out.complete, shared) {
            (true, _) => (Icon::Success, Tone::Success, s.sign_result_complete),
            (false, true) => (Icon::Info, Tone::Info, s.sign_result_nonce),
            (false, false) => (Icon::Info, Tone::Info, s.sign_result_partial),
        };
        let mut keys: Vec<String> = Vec::new();
        for sig in &out.signatures {
            let name = text::fingerprint_hex(sig.fingerprint);
            if !keys.contains(&name) {
                keys.push(name);
            }
        }
        let mut rows = if shared {
            vec![
                components::Record::text(
                    s.sign_session_row,
                    String::from(s.sign_session_open),
                    Tone::Text,
                ),
                components::Record::text(
                    s.sign_session_ends_row,
                    String::from(s.sign_session_ends),
                    Tone::Text,
                ),
            ]
        } else {
            vec![
                components::Record::fingerprint(
                    if keys.len() > 1 {
                        s.sign_keys_row
                    } else {
                        s.sign_key_row
                    },
                    joined(&keys),
                ),
                components::Record::text(
                    s.sign_inputs,
                    strings::fill(
                        s.words_page,
                        &[
                            &alloc::format!("{}", out.signatures.len()),
                            &alloc::format!("{}", insp.inputs.len()),
                        ],
                    ),
                    Tone::Text,
                ),
            ]
        };
        match out.save {
            Save::Idle => {}
            Save::Waiting => rows.push(components::Record::text(
                s.sign_file_row,
                String::from(s.sign_waiting),
                Tone::Muted,
            )),
            Save::Written => rows.push(components::Record::text(
                s.sign_file_row,
                strings::fill1(s.sign_saved_as, &out.name_hint),
                Tone::Text,
            )),
            Save::Failed => rows.push(components::Record::text(
                s.sign_file_row,
                String::from(s.sign_not_saved),
                Tone::Caution,
            )),
        }
        // A MuSig2 input's partial signatures are what says how far the
        // transaction has got: one of two is a pass that ended in a
        // partial signature, two of two is the aggregate that finalized
        // it.
        // A threshold input's partial signatures say how far the
        // transaction has got: one of two is a pass that wrote one, two
        // of two is the aggregate that finalized it (§16.103).
        let signatures = match out.threshold {
            Some((t, partials)) => strings::fill(
                s.words_page,
                &[&alloc::format!("{partials}"), &alloc::format!("{t}")],
            ),
            None => match &insp.musig {
                Some(m) => {
                    let partials = m.partial_signatures
                        + out
                            .signatures
                            .iter()
                            .filter(|sig| sig.musig == Some(MusigRole::Partial))
                            .count();
                    strings::fill(
                        s.words_page,
                        &[&alloc::format!("{partials}"), &alloc::format!("{}", m.keys)],
                    )
                }
                None => alloc::format!("{}", out.signatures.len()),
            },
        };
        rows.push(components::Record::value(
            ids::SIGN_SIGNATURES,
            s.sign_signatures,
            signatures,
        ));
        // §4.10: the signed transaction is public, so it is one of the
        // strings a row copies.
        rows.extend(self.copy_record());
        let second = if out.save == Save::Written {
            Action::new(ids::SIGN_DONE, s.action_done)
        } else {
            Action::new(ids::SIGN_SAVE, s.action_save_file)
        };
        screens::result(
            c,
            screens::Result {
                caption: self.notice().map(String::from),
                title: s.sign_title,
                icon,
                tone,
                result,
                rows,
                // A carry file never goes through a code: the secret
                // nonce in it is not shown, and this device cannot scan
                // its own later anyway (§16.103).
                actions: match out.carry.is_some() {
                    true => vec![second],
                    false => vec![second, Action::new(ids::SIGN_QR, s.action_show_qr)],
                },
            },
        )
    }

    /// §5 Record, "Signatures": one row per signature the transaction
    /// carries, whoever made it — the input and the key as the label,
    /// what checking it came to as the value, and the nonce rule beside
    /// it for a key this device holds (`docs/PLANNING.md` §16.111).
    /// The row opens the signature whole on Compare (§4.5).
    fn sign_signatures(&self, c: &Chrome<'_>, flow: &SignFlow) -> Node {
        let s = self.strings();
        let mut rows = Vec::new();
        if let Some(txid) = flow.outcome().and_then(|o| o.txid) {
            rows.push(components::Record::reference(
                ids::SIGN_TXID,
                s.sign_txid,
                alloc::format!("{txid}"),
            ));
        }
        for (n, sig) in flow.signature_rows().iter().enumerate() {
            rows.push(components::Record::value(
                ids::at(ids::SIGN_SIG_BASE, n),
                alloc::format!(
                    "{} \u{00b7} {}",
                    strings::fill1(s.sign_input_row, &alloc::format!("{}", sig.input)),
                    sig.name
                ),
                signature_value(sig, s),
            ));
        }
        screens::record(
            c,
            screens::Record {
                pager: None,
                title: s.sign_signatures_title,
                key: None,
                network: text::network(self.network),
                rows,
                warnings: Vec::new(),
                action: None,
            },
        )
    }

    /// §5 QR, "Transaction": the code at the class's side, the Animated
    /// toggle §4.9 dims when one code would be too dense to read, and the
    /// progress row that counts the parts.
    fn sign_qr(&self, c: &Chrome<'_>, flow: &SignFlow) -> Node {
        let s = self.strings();
        let Some(matrix) = flow.qr_matrix() else {
            return self.sign_signatures(c, flow);
        };
        let progress = flow
            .qr_part()
            .filter(|(_, total)| *total > 0)
            .map(|(part, total)| {
                let shown = (part - 1) % total + 1;
                (
                    shown as f32 / total as f32,
                    strings::fill(
                        s.words_page,
                        &[&alloc::format!("{shown}"), &alloc::format!("{total}")],
                    ),
                )
            });
        screens::qr(
            c,
            Qr {
                title: s.sign_qr_title,
                matrix,
                label: s.sign_qr_label,
                toggle: Some((ids::SIGN_QR_ANIMATED, s.sign_qr_animated)),
                animated: flow.qr_mode() == Some(QrMode::Ur),
                forced: (!flow.static_qr_scans()).then(|| String::from(s.sign_qr_dense)),
                progress,
                save: self.png_row(),
                caption: self.notice().map(String::from),
            },
        )
    }
}

/// §4.11's rank as the card draws it. A block is drawn in the danger
/// tone; its label is what says it cannot be signed.
fn level_of(level: Level) -> WarningLevel {
    match level {
        Level::Blocked | Level::Danger => WarningLevel::Danger,
        Level::Caution => WarningLevel::Caution,
        Level::Info => WarningLevel::Info,
    }
}

/// §4.11 Warning: "A card: level icon, label, value." What the warning is
/// about on the left, what is wrong with it on the right — never the
/// inspector's sentence, which is a paragraph on a working screen (§2.1).
///
/// A card that blocks signing carries "Blocked" in place of the field,
/// because that is the fact the screen is there for: the flow ends here.
fn warning_words(w: &Warning, s: &Strings) -> (String, String) {
    let (label, value) = match &w.kind {
        WarningKind::UnverifiedChange => (s.sign_change, String::from(s.sign_change_unverified)),
        WarningKind::ChangeSpoof => (s.sign_change, String::from(s.warn_change_spoof)),
        WarningKind::HighFee { pct_of_amount } => (
            s.sign_fee,
            strings::fill1(s.warn_high_fee, &alloc::format!("{pct_of_amount}")),
        ),
        WarningKind::DustOutput => (s.warn_output, String::from(s.warn_dust)),
        WarningKind::UnusualSighash => (s.warn_sighash_label, String::from(s.warn_sighash)),
        WarningKind::MixedScriptTypes => (s.sign_inputs, String::from(s.warn_mixed_scripts)),
        WarningKind::NetworkMismatch => (s.confirm_network, String::from(s.warn_network)),
        WarningKind::NoParticipatingKey => (s.sign_key_row, String::from(s.warn_no_key)),
        WarningKind::UnknownDerivation => (s.row_path, String::from(s.warn_unknown_path)),
        WarningKind::AbsurdFeeRate => (s.sign_rate, String::from(s.warn_absurd_rate)),
        WarningKind::LocktimeInFuture => (s.sign_locktime, String::from(s.warn_locktime)),
        WarningKind::NonStandardScript => (s.warn_output, String::from(s.warn_non_standard)),
        WarningKind::AddressReuse => (s.row_address, String::from(s.warn_address_reuse)),
        WarningKind::UtxoMismatch => (s.sign_inputs, String::from(s.warn_utxo_mismatch)),
        WarningKind::MissingUtxo => (s.sign_inputs, String::from(s.warn_missing_utxo)),
        WarningKind::UnsupportedInput => (s.sign_inputs, String::from(s.warn_unsupported)),
        WarningKind::MusigWalletNotRegistered => {
            (s.sign_inputs, String::from(s.warn_musig_not_registered))
        }
        WarningKind::MusigNonceReplaced => {
            (s.sign_inputs, String::from(s.warn_musig_nonce_replaced))
        }
        WarningKind::ThresholdWalletNotRegistered => {
            (s.sign_inputs, String::from(s.warn_threshold_not_registered))
        }
        WarningKind::ThresholdRefused(refusal) => (
            s.sign_inputs,
            String::from(match refusal {
                Refusal::AnotherTransaction => s.warn_threshold_other_tx,
                Refusal::AnotherShare => s.warn_threshold_other_share,
                Refusal::SignerSet => s.warn_threshold_signer_set,
                Refusal::NonceMismatch => s.warn_threshold_nonce,
                Refusal::PartialSig => s.warn_threshold_partial_sig,
                Refusal::NoSection => s.warn_threshold_no_file,
                Refusal::NoNonces => s.warn_threshold_no_nonces,
                Refusal::NotASigner => s.warn_threshold_not_a_signer,
                Refusal::ChosenCount => s.warn_threshold_chosen,
            }),
        ),
        WarningKind::AmountUnverified => (s.sign_inputs, String::from(s.warn_amount_unverified)),
        // §16.111: what is wrong with a signature is named on the card,
        // because the card is the only place the input and the key it
        // is under are said together.
        WarningKind::InvalidSignature { input, key } => (
            s.sign_signatures,
            strings::fill(s.warn_signature_invalid, &[key, &alloc::format!("{input}")]),
        ),
        WarningKind::NonceReuse { key } => {
            (s.sign_signatures, strings::fill1(s.warn_nonce_reuse, key))
        }
        WarningKind::Nondeterministic { input, key } => (
            s.sign_signatures,
            strings::fill(s.warn_nondeterministic, &[key, &alloc::format!("{input}")]),
        ),
        WarningKind::SignatureUnchecked { input, reason } => (
            s.sign_signatures,
            strings::fill(
                s.warn_signature_unchecked,
                &[&alloc::format!("{input}"), unchecked_words(*reason, s)],
            ),
        ),
        WarningKind::Inscription {
            input,
            content_type,
        } => (
            s.warn_inscription_label,
            strings::fill(
                s.warn_inscription,
                &[
                    &alloc::format!("{input}"),
                    if content_type.is_empty() {
                        s.warn_inscription_unknown
                    } else {
                        content_type
                    },
                ],
            ),
        ),
        WarningKind::TimelockNotMet { path, .. } => (
            s.wallet_spend_path_row,
            strings::fill1(s.warn_timelock_not_met, &text::spend_path(path, s)),
        ),
    };
    let label = if w.level == Level::Blocked {
        s.warn_blocked
    } else {
        label
    };
    (String::from(label), value)
}

/// Why a signature was not checked, in the words the page and the card
/// both use.
fn unchecked_words(reason: Unchecked, s: &Strings) -> &'static str {
    match reason {
        Unchecked::NoPrevout => s.sign_sig_no_prevout,
        Unchecked::UnsupportedScript => s.sign_sig_unsupported,
        Unchecked::UnknownKey => s.sign_sig_unknown_key,
    }
}

/// What one signature's row says: the verdict, and for a key this
/// device holds the nonce rule that made it.
fn signature_value(sig: &SignatureRow, s: &Strings) -> String {
    let mut value = match sig.verdict {
        Verdict::Valid => String::from(s.sign_sig_valid),
        Verdict::Invalid => String::from(s.sign_sig_invalid),
        Verdict::Unchecked(reason) => {
            strings::fill1(s.sign_sig_unchecked, unchecked_words(reason, s))
        }
    };
    // §16.111 rule 3: the rule that made the nonce, where the key is
    // one this device can recompute under.
    if let Some(mode) = sig.determinism {
        let told = match mode {
            Some(mode) => strings::fill1(s.sign_sig_deterministic, nonce_words(mode, s)),
            None => String::from(s.sign_sig_not_deterministic),
        };
        value.push_str(&alloc::format!(" \u{00b7} {told}"));
    }
    value
}

/// The nonce rule's own name, which Settings names the two RFC 6979
/// ones by.
fn nonce_words(mode: NonceMode, s: &Strings) -> &'static str {
    match mode {
        NonceMode::LowR => s.settings_nonce_low_r,
        NonceMode::First => s.settings_nonce_first,
        NonceMode::Bip340 => s.sign_nonce_bip340,
    }
}
