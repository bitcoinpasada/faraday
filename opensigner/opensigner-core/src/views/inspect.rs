//! What a scanned descriptor, extended public key or text says, built
//! from `docs/DESIGN.md` §5.
//!
//! Each is a Record: the facts a person checks as one fact per row, with
//! the string itself as §4.5's reference row, which opens the Compare
//! screen. A descriptor is compared as structure rather than chunked in
//! fours (§4.5), and its own tokens are what says which key it names.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::keys::ScriptType;
use osk_bip::policy::{Template, WalletPolicy};
use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Action};
use osk_ui::tokens;
use osk_ui::widgets::{Icon, Tone, WarningLevel};

use crate::inspect::InspectDoc;
use crate::{OpenSigner, ids, strings, text};
use osk_bip::descriptor::{origin_fingerprints, script_type};
use osk_bip::keys::Network;
use osk_bip::slip132::network_kind;

impl OpenSigner {
    pub(crate) fn view_inspect(&self) -> Node {
        self.with_chrome(Some(ids::BACK), |c| {
            let s = self.strings();
            let Some(doc) = self.inspect.as_ref() else {
                return screens::result(
                    c,
                    screens::Result {
                        caption: None,
                        title: s.inspect_text,
                        icon: Icon::Info,
                        tone: Tone::Muted,
                        result: s.inspect_empty,
                        rows: Vec::new(),
                        actions: vec![Action::new(ids::INSPECT_DONE, s.action_done)],
                    },
                );
            };
            let rows = if doc.descriptor {
                self.descriptor_rows(doc)
            } else if doc.title == s.inspect_xpub {
                self.xpub_rows(doc)
            } else {
                self.text_rows(doc)
            };
            screens::record(
                c,
                screens::Record {
                    pager: None,
                    title: doc.title,
                    key: None,
                    network: components::Network::Mainnet,
                    rows,
                    warnings: self.swap_warnings(doc),
                    action: Some(self.inspect_action(doc)),
                },
            )
        })
    }

    /// What a wallet is in a row: "2 of 3 · SegWit", "MuSig2 · 2 keys
    /// · Taproot" for a wallet of one aggregate key, which has keys but
    /// no quorum, or "Miniscript · SegWit" and "Taproot tree" for the
    /// wallets whose script says how they can be spent (§4.6's
    /// vocabulary for the script, never `wsh`).
    pub(crate) fn wallet_label(&self, policy: &WalletPolicy) -> alloc::string::String {
        let s = self.strings();
        let script = text::script_short(policy.script_type(), s);
        // §16.113: a silent payments wallet is named by its kind alone.
        // Its outputs are taproot by definition, so the script would be
        // a fact the name already carries.
        if policy.silent().is_some() {
            return alloc::string::String::from(s.wallet_silent);
        }
        // A wallet whose second path opens after a wait is named for
        // that, whatever shape the compiler gave it.
        if osk_bip::recovery::RecoveryPolicy::from_wallet_policy(policy).is_some() {
            return alloc::format!("{} \u{00b7} {script}", s.wallet_recovery);
        }
        // BIP 387's tapscript multisig states its quorum, which is the
        // leaf's rather than the template's.
        if let Some((m, n)) = policy.tapscript_quorum() {
            return alloc::format!(
                "{} \u{00b7} {script}",
                strings::fill(
                    s.wallet_quorum,
                    &[&alloc::format!("{m}"), &alloc::format!("{n}")]
                )
            );
        }
        match (policy.template(), policy.quorum()) {
            (_, Some((m, n))) => alloc::format!(
                "{} \u{00b7} {script}",
                strings::fill(
                    s.wallet_quorum,
                    &[&alloc::format!("{m}"), &alloc::format!("{n}")]
                )
            ),
            (Template::MuSig, _) => alloc::format!(
                "{} \u{00b7} {} \u{00b7} {script}",
                s.wallet_musig,
                self.musig_keys(policy)
            ),
            // A threshold wallet has no quorum of signatures either, and
            // the `t` of `n` it states is a fact about its shares.
            (Template::Threshold { .. }, _) => alloc::format!(
                "{} \u{00b7} {} \u{00b7} {script}",
                s.wallet_threshold,
                self.threshold_quorum(policy)
            ),
            (Template::Miniscript { .. }, _) => {
                alloc::format!("{} \u{00b7} {script}", s.wallet_miniscript)
            }
            // A taproot tree is taproot by definition, so the script
            // would be the same word twice.
            (Template::Tree, _) => alloc::string::String::from(s.wallet_tree),
            // A single-key wallet is named by the master its key comes
            // from and what it pays to, since one key makes one wallet
            // per script type (`docs/PLANNING.md` §16.104 rule 2). A key
            // read from a bare extended public key names no master, so
            // what is left to say is the script alone.
            _ => match policy.keys().first().and_then(|k| k.fingerprint()) {
                Some(fp) => {
                    alloc::format!("{} \u{00b7} {script}", text::fingerprint_hex(fp))
                }
                None => alloc::string::String::from(script),
            },
        }
    }

    /// How many of a threshold wallet's shares sign: "2 of 3".
    fn threshold_quorum(&self, policy: &WalletPolicy) -> alloc::string::String {
        let (t, n) = match policy.template() {
            Template::Threshold {
                threshold,
                participants,
            } => (threshold, participants),
            _ => (0, 0),
        };
        strings::fill(
            self.strings().wallet_quorum,
            &[&alloc::format!("{t}"), &alloc::format!("{n}")],
        )
    }

    /// How many keys aggregate into a MuSig2 wallet: "2 keys".
    fn musig_keys(&self, policy: &WalletPolicy) -> alloc::string::String {
        strings::fill(
            self.strings().wallet_musig_keys,
            &[&alloc::format!("{}", policy.keys().len())],
        )
    }

    /// The danger card a review carries when the wallet is one key away
    /// from a wallet in use, or when its one key claims a master this
    /// device holds (UX.md E2).
    pub(crate) fn swap_warnings(&self, doc: &InspectDoc) -> Vec<(WarningLevel, String, String)> {
        match doc.swap.as_ref() {
            Some(swap) => vec![(
                WarningLevel::Danger,
                String::from(swap.title),
                swap.value.clone(),
            )],
            None => Vec::new(),
        }
    }

    /// The one way on: Done for anything read, and for a wallet the
    /// choice to use it or, when it is already in use, to forget it.
    ///
    /// §2.7: a review carrying a danger card is held rather than tapped.
    fn inspect_action(&self, doc: &InspectDoc) -> Action {
        let s = self.strings();
        match doc.policy.as_ref() {
            // Reached from the wallet's own menu, where Forget is a row:
            // the document is read and closed.
            _ if doc.from_menu => Action::new(ids::INSPECT_DONE, s.action_done),
            Some(policy) if self.wallet_in_use(policy) => {
                Action::new(ids::INSPECT_FORGET_WALLET, s.wallet_forget)
            }
            Some(_) if doc.swap.is_some() => Action::holding(ids::INSPECT_USE_WALLET, s.wallet_use),
            Some(_) => Action::new(ids::INSPECT_USE_WALLET, s.wallet_use),
            None => Action::new(ids::INSPECT_DONE, s.action_done),
        }
    }

    /// A descriptor's facts: whether its checksum holds, what it pays
    /// to, the keys it names, and the descriptor itself one tap away.
    ///
    /// A wallet policy is the same review with the quorum it needs and,
    /// once it is in use, the state that says so.
    pub(crate) fn descriptor_rows(&self, doc: &InspectDoc) -> Vec<components::Record> {
        self.descriptor_rows_of(doc, false)
    }

    /// The same rows for a wallet built on this device. Nothing was
    /// given to this device to check, so there is no checksum row, and
    /// the keys are the rows that follow, so there is no count of them.
    pub(crate) fn built_wallet_rows(&self, doc: &InspectDoc) -> Vec<components::Record> {
        self.descriptor_rows_of(doc, true)
    }

    fn descriptor_rows_of(&self, doc: &InspectDoc, built: bool) -> Vec<components::Record> {
        let s = self.strings();
        let (checksum, tone) = match doc.checksum {
            Some(true) => (s.inspect_checksum_ok, Tone::Success),
            Some(false) => (s.inspect_checksum_bad, Tone::Danger),
            None => (s.value_none, Tone::Muted),
        };
        let mut rows = Vec::new();
        if let Some(policy) = doc.policy.as_ref() {
            // A wallet whose second path opens after a wait states who
            // spends now and who spends then, rather than a miniscript
            // nobody reads. A Liana file arrives this way too.
            if let Some(recovery) = osk_bip::recovery::RecoveryPolicy::from_wallet_policy(policy) {
                rows.push(components::Record::text(
                    s.wallet_kind_row,
                    s.wallet_recovery,
                    Tone::Text,
                ));
                rows.push(components::Record::text(
                    s.wallet_recovery_now,
                    strings::fill(
                        s.wallet_quorum,
                        &[
                            &alloc::format!("{}", recovery.primary.threshold),
                            &alloc::format!("{}", recovery.primary.keys.len()),
                        ],
                    ),
                    Tone::Text,
                ));
                for path in &recovery.recovery {
                    rows.push(components::Record::text(
                        strings::fill1(
                            s.wallet_recovery_after,
                            &alloc::format!("{}", osk_bip::recovery::days_of_blocks(path.delay)),
                        ),
                        strings::fill(
                            s.wallet_recovery_value,
                            &[
                                &alloc::format!("{}", path.path.threshold),
                                &alloc::format!("{}", path.path.keys.len()),
                                &text::thousands(path.delay),
                            ],
                        ),
                        Tone::Text,
                    ));
                }
            } else if let Some((m, n)) = policy.tapscript_quorum() {
                // BIP 387's tapscript multisig has no `quorum()`: the
                // threshold is the leaf's, not the template's.
                rows.push(components::Record::text(
                    s.sign_multisig,
                    strings::fill(
                        s.wallet_quorum,
                        &[&alloc::format!("{m}"), &alloc::format!("{n}")],
                    ),
                    Tone::Text,
                ));
            } else {
                match (policy.template(), policy.quorum()) {
                    (_, Some((m, n))) => rows.push(components::Record::text(
                        s.sign_multisig,
                        strings::fill(
                            s.wallet_quorum,
                            &[&alloc::format!("{m}"), &alloc::format!("{n}")],
                        ),
                        Tone::Text,
                    )),
                    // A MuSig2 wallet signs with one key and states how
                    // many keys make it, since it has no quorum to state.
                    (Template::MuSig, _) => rows.push(components::Record::text(
                        s.wallet_musig,
                        self.musig_keys(policy),
                        Tone::Text,
                    )),
                    // A threshold wallet states how many of its shares sign.
                    (Template::Threshold { .. }, _) => rows.push(components::Record::text(
                        s.wallet_threshold,
                        self.threshold_quorum(policy),
                        Tone::Text,
                    )),
                    (Template::Miniscript { .. }, _) => rows.push(components::Record::text(
                        s.wallet_kind_row,
                        s.wallet_miniscript,
                        Tone::Text,
                    )),
                    (Template::Tree, _) => rows.push(components::Record::text(
                        s.wallet_kind_row,
                        s.wallet_tree,
                        Tone::Text,
                    )),
                    _ => {}
                }
            }
            if self.wallet_in_use(policy) {
                rows.push(components::Record::text(
                    s.wallet_title,
                    s.wallet_in_use,
                    Tone::Success,
                ));
            }
        }
        if !built {
            rows.push(components::Record::text(s.inspect_checksum, checksum, tone));
        }
        // BIP 129 round 2: what every signer checks before registering
        // the wallet — the paths, the first address, and its own place
        // in the key list. The quorum row above is the M of N.
        if let Some(bsms) = doc.bsms.as_ref() {
            rows.push(components::Record::text(
                s.inspect_bsms_paths,
                bsms.paths.clone(),
                Tone::Text,
            ));
            rows.push(components::Record::text(
                s.inspect_bsms_first,
                bsms.first_address.clone(),
                Tone::Text,
            ));
            let ours = match bsms.ours {
                Some(i) => strings::fill1(s.wallet_path_key, &text::key_letter(i)),
                None => alloc::string::String::from(s.inspect_bsms_not_ours),
            };
            rows.push(components::Record::text(
                s.inspect_bsms_ours,
                ours,
                Tone::Text,
            ));
        }
        if let Some(script) = script_type(&doc.text) {
            rows.push(components::Record::text(
                s.script_type_row,
                text::script_short(script, s),
                Tone::Text,
            ));
        }
        // A taproot wallet built on BIP 341's NUMS point can be spent
        // only through its tree, which is a fact of the wallet and not
        // of any one key.
        if doc
            .policy
            .as_ref()
            .is_some_and(|p| p.key_path_unspendable())
        {
            rows.push(components::Record::text(
                s.wallet_key_path,
                s.wallet_unspendable,
                Tone::Text,
            ));
        }
        // One row per way the coins can be spent, in the order the
        // script writes them: for a tree, the key path and then the
        // leaves. The letters name the key rows below.
        if let Some(policy) = doc.policy.as_ref() {
            for (i, path) in policy.spend_paths().into_iter().enumerate() {
                rows.push(components::Record::text(
                    strings::fill1(s.wallet_spend_path, &alloc::format!("{}", i + 1)),
                    text::spend_path(&path, s),
                    Tone::Text,
                ));
            }
        }
        // A FROST wallet's members are the record's public shares: each
        // row names the member and states the fingerprint of its public
        // share, with the key glyph where a loaded key computes it.
        if let Some(record) = doc.policy.as_ref().and_then(|p| p.record()) {
            let n = alloc::format!("{}", record.n());
            for i in 0..record.n() {
                let value = match record.share_fingerprint(i) {
                    Some(fp) => text::fingerprint_hex(fp),
                    None => alloc::string::String::from(s.wallet_origin_unknown),
                };
                // A member this device holds a key for carries the key
                // glyph; the rest carry the eye.
                let held = record
                    .info
                    .pubshares
                    .get(i)
                    .copied()
                    .flatten()
                    .is_some_and(|p| self.keys.iter().any(|k| k.share == Some(p)));
                rows.push(components::Record::glyph(
                    self.wallet_glyph(held),
                    strings::fill(s.wallet_member, &[&alloc::format!("{}", i + 1), &n]),
                    value,
                    Tone::Text,
                ));
            }
            rows.push(components::Record::mono_value(
                ids::INSPECT_TEXT,
                s.inspect_descriptor,
                text::descriptor_summary(&doc.text),
            ));
            return rows;
        }
        // Where the document is a wallet, the keys come from the wallet,
        // so a key that arrived with no origin is one of the rows and
        // says what is missing instead of going unlisted.
        // A wallet whose spend paths are listed names its keys by letter
        // there, so the key rows carry the same letter.
        let lettered = doc
            .policy
            .as_ref()
            .is_some_and(|p| !p.spend_paths().is_empty());
        let keys: Vec<(bool, alloc::string::String)> = match doc.policy.as_ref() {
            Some(policy) => policy
                .keys()
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    let (mine, value) = match k.fingerprint() {
                        Some(fp) => (
                            self.keys.iter().any(|x| x.fingerprint == fp),
                            text::fingerprint_hex(fp),
                        ),
                        None => (false, alloc::string::String::from(s.wallet_origin_unknown)),
                    };
                    match lettered {
                        true => (
                            mine,
                            alloc::format!("{} \u{00b7} {value}", text::key_letter(i)),
                        ),
                        false => (mine, value),
                    }
                })
                .collect(),
            None => origin_fingerprints(&doc.text)
                .into_iter()
                .map(|fingerprint| {
                    let holds_key = self
                        .keys
                        .iter()
                        .any(|k| text::fingerprint_hex(k.fingerprint) == fingerprint);
                    (holds_key, fingerprint)
                })
                .collect(),
        };
        if !built {
            rows.push(components::Record::text(
                s.row_keys,
                alloc::format!("{}", keys.len()),
                Tone::Text,
            ));
        }
        let changed = doc.swap.as_ref().map(|swap| swap.key);
        for (i, (holds_key, value)) in keys.into_iter().enumerate() {
            // §4.4's glyph: the key when this device holds the private
            // key of this cosigner, the eye when it holds the public key
            // alone. The MINE badge means an output or an address that
            // pays one of this device's wallets, and nothing else.
            // The key a danger card names is the row a person is being
            // asked to look at, so the row carries the caution tone.
            let tone = if changed == Some(i) {
                Tone::Caution
            } else {
                Tone::Text
            };
            rows.push(components::Record::glyph(
                self.wallet_glyph(holds_key),
                s.sign_key_row,
                value,
                tone,
            ));
        }
        // A descriptor is read by its parts, so the row names the parts
        // a person checks; the whole descriptor is one tap away.
        rows.push(components::Record::mono_value(
            ids::INSPECT_TEXT,
            s.inspect_descriptor,
            text::descriptor_summary(&doc.text),
        ));
        rows
    }

    /// An extended public key's facts: the chain it belongs to, whether
    /// a loaded key derives it, and the key itself one tap away.
    fn xpub_rows(&self, doc: &InspectDoc) -> Vec<components::Record> {
        let s = self.strings();
        let mut rows = Vec::new();
        if let Some(kind) = network_kind(&doc.text) {
            // The four test networks share one set of version bytes.
            let network = match kind {
                osk_bip::bitcoin::NetworkKind::Main => Network::Mainnet,
                osk_bip::bitcoin::NetworkKind::Test => Network::Testnet,
            };
            rows.push(components::Record::text(
                s.confirm_network,
                network.name(),
                Tone::Text,
            ));
        }
        // The comparison needs an account key of a loaded key to compare
        // with; with nothing loaded the row would state no fact, so it
        // is left off (§2.1).
        if !self.keys.is_empty() {
            let mine = self.account_key_matches(&doc.text);
            rows.push(components::Record::text(
                s.row_mine,
                if mine { s.value_yes } else { s.value_no },
                if mine { Tone::Success } else { Tone::Text },
            ));
        }
        rows.push(components::Record::reference(
            ids::INSPECT_TEXT,
            s.inspect_xpub,
            doc.text.clone(),
        ));
        rows
    }

    /// Plain text: how long it is and the text itself — whole in the
    /// table when it is short enough for one line, and §4.5's reference
    /// row when it is not.
    fn text_rows(&self, doc: &InspectDoc) -> Vec<components::Record> {
        let s = self.strings();
        let n = doc.text.chars().count();
        // A string no longer than its own elision gains nothing from a
        // screen of its own, and words read worse in groups of four.
        let short = n <= tokens::ELIDE_HEAD + tokens::ELIDE_TAIL && !doc.text.contains('\n');
        let value = if short {
            components::Record::text(s.inspect_text, doc.text.clone(), Tone::Text)
        } else {
            components::Record::reference(ids::INSPECT_TEXT, s.inspect_text, doc.text.clone())
        };
        vec![
            components::Record::text(
                s.inspect_length_row,
                strings::fill1(s.inspect_length, &alloc::format!("{n}")),
                Tone::Text,
            ),
            value,
        ]
    }

    /// Whether `key` is an account key of a loaded key, at any of the
    /// four script types §4.6 names.
    fn account_key_matches(&self, key: &str) -> bool {
        self.keys.iter().any(|k| {
            ScriptType::ALL.iter().any(|script| {
                self.cached_account(k.fingerprint, *script)
                    .is_some_and(|a| a.xpub_string() == key || a.slip132_string() == key)
            })
        })
    }
}
