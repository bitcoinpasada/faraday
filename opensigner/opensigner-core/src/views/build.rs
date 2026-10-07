//! Add a wallet's screens (`docs/DESIGN.md` §5): the kind Choice, the
//! FROST counts, the Keys check list, the script-type Choice, the
//! threshold Choice, the review the wallet is, the dealt keys' words and
//! their quiz, the group record, and the confirm that guards the keys on
//! the way out.

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::bip39::Language;
use osk_bip::keys::ScriptType;
use osk_codec::qr::{Ecc, Payload};
use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Above, Action, Chrome, Entry, Field, Item, Pad, PadKind, Qr, Result};
use osk_ui::widgets::keyboard::{ALL_KEYS, DONE_DISABLED, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::build::{BuiltKey, DaysRefusal, Step, WalletKind, WalletWizard};
use crate::load::EntryList;
use crate::sign::Save;
use crate::threshold::COUNTS;
use crate::views::{quiz as quiz_view, words};
use crate::{OpenSigner, ids, strings, text};

impl OpenSigner {
    pub(crate) fn view_build(&self, w: &WalletWizard) -> Node {
        let s = self.strings();
        // §16.128 rule 3: the discard question has no Back chevron, so
        // no path returns to it from itself.
        let chevron = (w.step() != Step::Discard).then_some(ids::BACK);
        self.with_chrome(chevron, |c| match w.step() {
            Step::Kind => self.build_kind_step(c, w),
            Step::Count => self.build_count_step(c, w),
            Step::Quorum => self.build_quorum_step(c, w),
            Step::Keys | Step::Later => self.build_keys_step(c, w),
            Step::LaterThreshold => self.build_later_threshold_step(c, w),
            Step::Delay => self.build_delay_step(c, w),
            Step::Days => self.build_days_step(c, w),
            Step::Another => self.build_another_step(c, w),
            Step::Type => self.build_type_step(c, w),
            Step::Script => self.build_script_step(c, w),
            Step::Threshold => self.build_threshold_step(c, w),
            Step::Review => self.build_review(c, w),
            Step::Words => self.build_words_step(c, w),
            Step::QuizStart => quiz_view::quiz_start(
                c,
                w.dealer().is_some_and(|d| d.helper()),
                self.first_run_done(),
                s,
            ),
            Step::Quiz => match w.dealer().and_then(crate::threshold::Dealer::quiz) {
                Some(q) if q.wrong_slot().is_some() => quiz_view::quiz_wrong(c, q, s),
                Some(q) => quiz_view::quiz_question(c, q, EntryList::Bip39(Language::English), s),
                None => Node::column(),
            },
            Step::QuizSkip => crate::views::create::skip_step(c, s),
            Step::Record => self.build_record_step(c, w),
            Step::Discard => self.build_discard(c, w),
            Step::Passphrase => self.build_passphrase_step(c, w),
        })
    }

    /// §5 Choice, "What kind of wallet?": the four kinds §16.104 rule 2
    /// names, in the order it names them.
    fn build_kind_step(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let items: Vec<Item> = WalletKind::ALL
            .iter()
            .enumerate()
            .map(|(i, kind)| {
                Item::chosen(
                    ids::at(ids::BUILD_KIND_BASE, i),
                    kind_name(*kind, self),
                    *kind == w.kind(),
                )
            })
            .collect();
        screens::choice(
            c,
            s.build_kind_title,
            items,
            Action::new(ids::BUILD_KIND_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "How many keys?": the group sizes a FROST wallet made
    /// here has.
    fn build_count_step(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let items = COUNTS
            .iter()
            .enumerate()
            .map(|(i, n)| {
                Item::chosen(
                    ids::at(ids::THRESHOLD_COUNT_BASE, i),
                    alloc::format!("{n}"),
                    usize::from(*n) == w.n(),
                )
            })
            .collect();
        screens::choice(
            c,
            s.threshold_count_title,
            items,
            Action::new(ids::THRESHOLD_COUNT_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "How many must sign?", for a FROST group.
    fn build_quorum_step(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let items = (2..=w.n())
            .map(|t| {
                Item::chosen(
                    ids::at(ids::THRESHOLD_QUORUM_BASE, t - 2),
                    alloc::format!("{t}"),
                    t == w.t(),
                )
            })
            .collect();
        screens::choice(
            c,
            s.build_threshold_title,
            items,
            Action::new(ids::THRESHOLD_QUORUM_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "Keys · 2": the loaded keys with a check, in Keys'
    /// order, a key this kind cannot use dimmed with §4.11's reason;
    /// then the cosigners scanned in, each of which a tap takes back
    /// out; then "Scan a key" where the kind takes one.
    fn build_keys_step(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let count = alloc::format!("{}", w.keys().len());
        // §4.1: a Later step carries which path it is where the wallet
        // has more than one.
        let title = match (w.kind(), w.on_later()) {
            (WalletKind::Recovery, false) => strings::fill1(s.build_now_title, &count),
            (WalletKind::Recovery, true) if w.later_paths() > 1 => strings::fill(
                s.build_later_n_title,
                &[&alloc::format!("{}", w.later_path() + 1), &count],
            ),
            (WalletKind::Recovery, true) => strings::fill1(s.build_later_title, &count),
            _ => strings::fill1(s.build_keys_title, &count),
        };
        let mut items: Vec<Item> = Vec::new();
        for (i, k) in self.keys.iter().enumerate() {
            let fingerprint = text::fingerprint_hex(k.fingerprint);
            match self.build_key_reason(w, i) {
                Some(reason) => items.push(Item::dimmed(fingerprint, reason).mono()),
                None => items.push(Item::key(
                    ids::at(ids::BUILD_WHICH_BASE, i),
                    fingerprint,
                    self.key_made_of(i),
                    w.holds_loaded(i),
                )),
            }
        }
        for (j, key) in w
            .keys()
            .iter()
            .enumerate()
            .filter(|(_, k)| k.loaded().is_none())
        {
            items.push(
                Item::key(
                    ids::at(ids::BUILD_KEY_ROW_BASE, j),
                    key_label(key, self),
                    key_path(key),
                    true,
                )
                .mono_subtitle(),
            );
        }
        if w.kind().takes_cosigners() {
            items.push(if w.can_add() {
                Item::new(ids::BUILD_WHICH_SCAN, s.build_scan_key)
            } else {
                Item::dimmed(s.build_scan_key, s.build_reason_full)
            });
        }
        // §4.11: what a group of this shape needs, stated once rather
        // than left for a dead Continue to imply.
        if w.kind() == WalletKind::Frost && w.keys().len() < w.needs() {
            items.push(Item::dead(strings::fill1(
                s.build_needs_keys,
                &alloc::format!("{}", w.needs()),
            )));
        }
        let continue_id = if w.on_later() {
            ids::BUILD_LATER_CONTINUE
        } else {
            ids::BUILD_CONTINUE
        };
        screens::choice(
            c,
            &title,
            items,
            Action::when(continue_id, s.action_continue, w.ready()),
        )
    }

    /// §5 Choice, "How many must sign later?": one row per threshold the
    /// recovery path's keys allow.
    fn build_later_threshold_step(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let items = (1..=w.later_keys().len())
            .map(|n| {
                Item::chosen(
                    ids::at(ids::BUILD_LATER_THRESHOLD_BASE, n - 1),
                    alloc::format!("{n}"),
                    n == w.later_threshold(),
                )
            })
            .collect();
        screens::choice(
            c,
            s.build_threshold_later_title,
            items,
            Action::new(ids::BUILD_LATER_THRESHOLD_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "After how long?": the waits, each with the blocks it
    /// is, which is what the script holds and what a coordinator shows;
    /// then the row that asks for a number of days instead. A path after
    /// another opens after it, so the waits at or below the path before
    /// this one are dimmed with §4.11's reason.
    fn build_delay_step(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let previous = w.previous_days();
        let too_short = strings::fill1(s.build_delay_too_short, &alloc::format!("{previous}"));
        let mut items: Vec<Item> = crate::build::DELAYS
            .iter()
            .enumerate()
            .map(|(i, days)| {
                let label = strings::fill1(s.build_delay_days, &alloc::format!("{days}"));
                if *days <= previous {
                    return Item::dimmed(label, too_short.clone());
                }
                let blocks = osk_bip::recovery::blocks_for_days(*days).unwrap_or(0);
                Item::valued(
                    ids::at(ids::BUILD_DELAY_BASE, i),
                    label,
                    strings::fill1(s.build_delay_blocks, &text::thousands(blocks)),
                    !w.typed_wait() && *days == w.days(),
                )
            })
            .collect();
        items.push(if w.typed_wait() {
            Item::valued(
                ids::BUILD_DELAY_TYPE,
                s.build_delay_type,
                strings::fill1(s.build_delay_days, &alloc::format!("{}", w.days())),
                true,
            )
        } else {
            Item::chosen(ids::BUILD_DELAY_TYPE, s.build_delay_type, false)
        });
        screens::choice(
            c,
            s.build_delay_title,
            items,
            Action::new(ids::BUILD_DELAY_CONTINUE, s.action_continue),
        )
    }

    /// §5 Pad, "Days": the count typed on the digits pad, the blocks it
    /// is under the field, and ✓ live only while the number is a wait
    /// this wallet can hold.
    fn build_days_step(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let blocks = w
            .days_value()
            .and_then(osk_bip::recovery::blocks_for_days)
            .map(|b| strings::fill1(s.build_delay_blocks, &text::thousands(b)));
        let error = w.days_refusal().map(|refusal| match refusal {
            DaysRefusal::TooMany => strings::fill1(
                s.build_days_too_many,
                &alloc::format!("{}", osk_bip::recovery::MAX_DAYS),
            ),
            DaysRefusal::TooShort => strings::fill1(
                s.build_delay_too_short,
                &alloc::format!("{}", w.previous_days()),
            ),
        });
        screens::pad(
            c,
            Pad {
                title: String::from(s.build_days_title),
                id: ids::BUILD_DAYS_PAD,
                kind: PadKind::Pin {
                    scramble: None,
                    done: w.days_ready(),
                },
                field: Field::Count(String::from(w.days_typed())),
                progress: None,
                entries: None,
                words: None,
                eye: None,
                caption: blocks,
                error,
                action: None,
            },
        )
    }

    /// §5 Choice, "Another path later?": No first, because a wallet with
    /// the paths it has is finished.
    fn build_another_step(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let items = vec![
            Item::chosen(ids::BUILD_ANOTHER_NO, s.build_another_no, !w.another()),
            Item::chosen(ids::BUILD_ANOTHER_YES, s.build_another_yes, w.another()),
        ];
        screens::choice(
            c,
            s.build_another_path_title,
            items,
            Action::new(ids::BUILD_ANOTHER_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "Passphrase?": the first step of a wallet started from
    /// a key's page, No checked (`docs/PLANNING.md` §16.129 rule 2).
    fn build_passphrase_step(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let items = vec![
            Item::chosen(
                ids::BUILD_PASSPHRASE_NO,
                s.build_passphrase_no,
                !w.passphrase(),
            ),
            Item::chosen(
                ids::BUILD_PASSPHRASE_YES,
                s.build_passphrase_yes,
                w.passphrase(),
            ),
        ];
        screens::choice(
            c,
            s.build_passphrase_title,
            items,
            Action::new(ids::BUILD_PASSPHRASE_CONTINUE, s.action_continue),
        )
    }

    /// §5 Entry, "Cosigner key": what the scanner's "Type" row opens.
    /// The same text the file and the QR routes read — an extended
    /// public key, or one with the origin it states before it — typed on
    /// the passphrase keyboard, which is the one with every character a
    /// key expression takes.
    fn build_type_step(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let typed = w.typed();
        let read = crate::build::read_key(typed, w.account_kind(), self.network);
        let error = match (&read, w.type_refusal()) {
            (_, Some(refusal)) => Some(String::from(self.build_reason(refusal))),
            (Err(refusal), None) if !typed.trim().is_empty() => {
                Some(String::from(self.build_reason(*refusal)))
            }
            _ => None,
        };
        screens::entry(
            c,
            Entry {
                candidates: None,
                title: s.build_type_title,
                value: String::from(typed),
                mono: true,
                above: Above::Nothing,
                words: None,
                eye: None,
                keyboard: (ids::BUILD_TYPE_KEYBOARD, KeyboardKind::Passphrase),
                enabled: if read.is_ok() {
                    ALL_KEYS
                } else {
                    ALL_KEYS | DONE_DISABLED
                },
                error,
            },
        )
    }

    /// §5 Choice, "Which script type?": the script types the kind has a
    /// template for, with the ones it has none for dimmed (§4.11), in
    /// the order the single-sig list reads.
    fn build_script_step(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let offered = w.kind().scripts();
        let items: Vec<Item> = ScriptType::ALL
            .iter()
            .enumerate()
            .map(|(i, script)| {
                let label = text::script_short(*script, s);
                if offered.contains(&i) {
                    Item::chosen(
                        ids::at(ids::BUILD_SCRIPT_BASE, i),
                        label,
                        i == w.script_index(),
                    )
                } else {
                    Item::dead(label)
                }
            })
            .collect();
        screens::choice(
            c,
            s.build_script_title,
            items,
            Action::new(ids::BUILD_SCRIPT_CONTINUE, s.action_continue),
        )
    }

    /// §5 Choice, "How many must sign?": one row per threshold the keys
    /// allow.
    fn build_threshold_step(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let items = (1..=w.primary_keys().len())
            .map(|n| {
                Item::chosen(
                    ids::at(ids::BUILD_THRESHOLD_BASE, n - 1),
                    alloc::format!("{n}"),
                    n == w.threshold(),
                )
            })
            .collect();
        let title = if w.kind() == WalletKind::Recovery {
            s.build_threshold_now_title
        } else {
            s.build_threshold_title
        };
        screens::choice(
            c,
            title,
            items,
            Action::new(ids::BUILD_THRESHOLD_CONTINUE, s.action_continue),
        )
    }

    /// §5 Record: the wallet, read exactly as a wallet that arrived from
    /// a coordinator is, with the one action that puts it in use. A
    /// FROST group has no descriptor until it is dealt, so its review is
    /// what it will be — the kind, the script, the chosen keys and the
    /// network — and "Add this wallet" is what deals it.
    fn build_review(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        if w.kind() == WalletKind::Frost {
            return self.frost_review(c, w);
        }
        if w.kind() == WalletKind::Silent {
            return self.silent_review(c, w);
        }
        let Some(policy) = w.policy() else {
            return self.build_keys_step(c, w);
        };
        let doc = self.wallet_doc(policy, true);
        screens::record(
            c,
            screens::Record {
                pager: None,
                title: s.build_new,
                key: None,
                network: components::Network::Mainnet,
                rows: self.built_wallet_rows(&doc),
                warnings: self.swap_warnings(&doc),
                // §2.7: a review carrying a danger card is held.
                action: Some(if doc.swap.is_some() {
                    Action::holding(ids::BUILD_ADD_WALLET, s.wallet_use)
                } else {
                    Action::new(ids::BUILD_ADD_WALLET, s.wallet_use)
                }),
            },
        )
    }

    /// §5 Record, the silent payments review: the kind, the network,
    /// the key the two halves come from, and the address a payer will
    /// be given. There is no descriptor to state, so there is none on
    /// the screen.
    fn silent_review(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let mut rows = vec![
            components::Record::text(s.threshold_confirm_wallet, s.wallet_silent, Tone::Text),
            components::Record::text(s.confirm_network, self.network.name(), Tone::Text),
        ];
        if let Some(key) = w.keys().first() {
            rows.push(components::Record::glyph(
                self.wallet_glyph(true),
                s.sign_key_row,
                key_label(key, self),
                Tone::Text,
            ));
        }
        let address = self
            .silent_built()
            .map(|record| record.address())
            .unwrap_or_default();
        rows.push(components::Record::mono(
            s.silent_address_value_row,
            address,
        ));
        screens::record(
            c,
            screens::Record {
                pager: None,
                title: s.build_new,
                key: None,
                network: text::network(self.network),
                rows,
                warnings: Vec::new(),
                action: Some(Action::new(ids::BUILD_ADD_WALLET, s.wallet_use)),
            },
        )
    }

    /// §5 Record, the FROST review.
    fn frost_review(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let kind = alloc::format!(
            "{} \u{00b7} {} \u{00b7} {}",
            s.wallet_threshold,
            strings::fill(
                s.wallet_quorum,
                &[&alloc::format!("{}", w.t()), &alloc::format!("{}", w.n()),],
            ),
            s.script_taproot
        );
        let mut rows = vec![
            components::Record::text(s.threshold_confirm_wallet, kind, Tone::Text),
            components::Record::text(s.confirm_network, self.network.name(), Tone::Text),
        ];
        let n = alloc::format!("{}", w.n());
        for (i, key) in w.keys().iter().enumerate() {
            rows.push(components::Record::glyph(
                self.wallet_glyph(true),
                strings::fill(s.wallet_member, &[&alloc::format!("{}", i + 1), &n]),
                key_label(key, self),
                Tone::Text,
            ));
        }
        screens::record(
            c,
            screens::Record {
                pager: None,
                title: s.threshold_confirm_title,
                key: None,
                network: text::network(self.network),
                rows,
                warnings: Vec::new(),
                action: Some(Action::when(
                    ids::BUILD_ADD_WALLET,
                    s.wallet_use,
                    !self.entropy_pending(),
                )),
            },
        )
    }

    /// §5 Words: one computed key's 24 words, under that key's own
    /// fingerprint — which is the one the wallet's Keys review shows and
    /// the one a person copies beside the words.
    fn build_words_step(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let Some(d) = w.dealer() else {
            return Node::column();
        };
        let Some(m) = d.words(d.show_at()) else {
            return Node::column();
        };
        let title = match d.share_fingerprint(d.show_at()) {
            Some(fp) => text::fingerprint_hex(fp),
            None => String::from(s.value_none),
        };
        words::words_screen(
            c,
            words::WordsScreen {
                title,
                list: EntryList::Bip39(Language::English),
                indices: m.indices(),
                revealed: self.revealed(ids::CREATE_REVEAL),
                rows: words::Rows::Words {
                    numbers: d.numbers(),
                },
                page: d.page(),
                remaining: self.reveal_ring(),
                extra: None,
                action: Action::new(ids::THRESHOLD_WORDS_CONTINUE, s.action_continue),
            },
            s,
        )
    }

    /// §5 QR, "Group record": the record as one code, with the ways to
    /// take it off the device.
    fn build_record_step(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        let Some(d) = w.dealer() else {
            return Node::column();
        };
        let record = d.record();
        let text = record.to_text();
        let matrix = osk_codec::qr::encode(Payload::Bytes(text.as_bytes()), Ecc::Low)
            .ok()
            .map(Rc::new);
        let Some(matrix) = matrix else {
            return screens::result(
                c,
                Result {
                    caption: None,
                    title: s.threshold_record_title,
                    icon: Icon::Error,
                    tone: Tone::Caution,
                    result: s.export_too_long,
                    rows: Vec::new(),
                    actions: vec![Action::new(
                        ids::THRESHOLD_RECORD_CONTINUE,
                        s.action_continue,
                    )],
                },
            );
        };
        screens::qr_with_actions(
            c,
            Qr {
                title: s.threshold_record_title,
                matrix,
                label: &text::fingerprint_hex(record.fingerprint()),
                toggle: None,
                animated: false,
                forced: None,
                progress: None,
                save: self.png_row(),
                caption: self.notice().map(String::from),
            },
            vec![
                Action::when(
                    ids::THRESHOLD_RECORD_SAVE,
                    s.action_save_file,
                    d.save() != Save::Waiting,
                ),
                Action::new(ids::THRESHOLD_RECORD_CONTINUE, s.action_continue),
            ],
        )
    }

    /// §5 Result: what leaving with keys gathered costs, as one fact,
    /// and the two ways out. §4.13 gives the accent to the safe one.
    fn build_discard(&self, c: &Chrome<'_>, w: &WalletWizard) -> Node {
        let s = self.strings();
        screens::result(
            c,
            Result {
                caption: None,
                title: s.build_new,
                icon: Icon::Warning,
                tone: Tone::Caution,
                result: s.build_discard_title,
                rows: vec![components::Record::text(
                    s.wallet_keys,
                    alloc::format!("{}", w.keys().len()),
                    Tone::Text,
                )],
                actions: vec![
                    Action::new(ids::BUILD_DISCARD, s.explore_discard),
                    Action::new(ids::BUILD_KEEP, s.explore_keep),
                ],
            },
        )
    }
}

/// What a kind is called, in §4.6's vocabulary.
fn kind_name(kind: WalletKind, app: &OpenSigner) -> &'static str {
    let s = app.strings();
    match kind {
        WalletKind::Single => s.add_single,
        WalletKind::Multisig => s.add_multisig,
        WalletKind::TaprootMultisig => s.add_taproot_multisig,
        WalletKind::MuSig2 => s.wallet_musig,
        WalletKind::Frost => s.wallet_threshold,
        WalletKind::Recovery => s.add_recovery,
        WalletKind::Silent => s.wallet_silent,
    }
}

/// A key's label: the master it descends from, or what is missing.
fn key_label(key: &BuiltKey, app: &OpenSigner) -> String {
    match key.key().fingerprint() {
        Some(fp) => text::fingerprint_hex(fp),
        None => String::from(app.strings().wallet_origin_unknown),
    }
}

/// The account path the key was read at.
fn key_path(key: &BuiltKey) -> String {
    key.key().path().map(text::path).unwrap_or_default()
}
