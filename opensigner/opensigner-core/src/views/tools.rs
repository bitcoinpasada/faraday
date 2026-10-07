//! Tools (`docs/DESIGN.md` §5 Menu): the standalone calculators. Every
//! row is live and needs no key: each opens a tool that takes what is
//! typed or scanned, works something out from it, and keeps nothing.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_psbt::verify::{Comparison, Field, SigningChange, Where};
use osk_ui::components;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Action, Row};
use osk_ui::widgets::{Icon, Tone};

use crate::strings::Strings;
use crate::{OpenSigner, ids, strings};

impl OpenSigner {
    pub(crate) fn view_tools(&self) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            let mut rows = Vec::new();
            for (id, icon, label) in [
                (ids::TOOLS_EXPLORER, Icon::Explore, s.tools_explorer),
                (ids::TOOLS_WORD_LIST, Icon::List, s.tools_word_list),
                (ids::TOOLS_DICE, Icon::Dice, s.tools_dice_passphrase),
                // Checking a signature needs no key of this device's,
                // so the row is live with nothing loaded.
                (ids::MSG_CHECK_ROW, Icon::EnvelopeOpen, s.msg_check_row),
            ] {
                rows.push(Row::Menu {
                    id,
                    icon: Some(icon),
                    label: String::from(label),
                    value: None,
                    tone: Tone::Text,
                });
            }
            // §3: one glyph per meaning, so each calculator carries the
            // mark of what it works on rather than the area's wrench.
            for (i, (icon, label)) in [
                (Icon::Hashtag, s.tools_hashes),
                (Icon::Code, s.tools_encodings),
                (Icon::CheckDouble, s.tools_descriptor_checksum),
                (Icon::Swap, s.tools_convert_key),
                (Icon::Bitcoin, s.tools_units),
                (Icon::Diagram, s.tools_miniscript),
            ]
            .into_iter()
            .enumerate()
            {
                rows.push(Row::Menu {
                    id: ids::at(ids::TOOLS_CALC_BASE, i),
                    icon: Some(icon),
                    label: String::from(label),
                    value: None,
                    tone: Tone::Text,
                });
            }
            for (id, icon, label) in [
                (ids::TOOLS_DECODE, Icon::Receipt, s.tools_decode),
                // Two transactions compared, which needs no key either
                // (`docs/PLANNING.md` §16.111 rule 4).
                (ids::TOOLS_COMPARE, Icon::CheckDouble, s.tools_compare),
                // Notes: text a person writes or reads here and takes
                // away plain or sealed (`docs/PLANNING.md` §16.112).
                (ids::TOOLS_NOTES, Icon::Receipt, s.tools_notes),
                // Which Lightning node a backup belongs to, which is an
                // identity and so carries the fingerprint glyph
                // (`docs/PLANNING.md` §16.116).
                (ids::TOOLS_LIGHTNING, Icon::Fingerprint, s.tools_lightning),
            ] {
                rows.push(Row::Menu {
                    id,
                    icon: Some(icon),
                    label: String::from(label),
                    value: None,
                    tone: Tone::Text,
                });
            }
            screens::menu(c, s.tools_title, None, rows, vec![])
        })
    }

    /// Tools › Compare transactions (`docs/PLANNING.md` §16.111 rule
    /// 4): while only the first transaction has arrived, §5 Menu with
    /// the way to the second; once both have, §5 Result with one row
    /// per field that differs.
    pub(crate) fn view_compare_tx(&self) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            let Some(comparison) = self.compare_tx.comparison() else {
                let mut rows = Vec::new();
                if let Some(txid) = self.compare_tx.first_txid() {
                    rows.push(Row::Value {
                        id: ids::COMPARE_TX_SECOND,
                        label: String::from(s.compare_tx_first),
                        value: components::elide(&alloc::format!("{txid}")),
                        tone: Tone::Text,
                    });
                }
                rows.push(Row::Menu {
                    id: ids::COMPARE_TX_SECOND,
                    icon: Some(Icon::Scan),
                    label: String::from(s.compare_tx_read_second),
                    value: None,
                    tone: Tone::Text,
                });
                return screens::menu(c, s.tools_compare, None, rows, vec![]);
            };
            let same = comparison.same();
            screens::result(
                c,
                screens::Result {
                    caption: None,
                    title: s.tools_compare,
                    icon: if same { Icon::Success } else { Icon::Info },
                    tone: if same { Tone::Success } else { Tone::Info },
                    result: if same {
                        s.compare_tx_same
                    } else {
                        s.compare_tx_different
                    },
                    rows: self.compare_rows(comparison),
                    actions: vec![Action::new(ids::COMPARE_TX_DONE, s.action_done)],
                },
            )
        })
    }

    /// One row per field that differs: the transaction's own fields,
    /// then the signing state.
    fn compare_rows(&self, comparison: &Comparison) -> Vec<components::Record> {
        let s = self.strings();
        let mut rows = Vec::new();
        for diff in comparison.transaction.iter().chain(&comparison.metadata) {
            let label = alloc::format!(
                "{} \u{00b7} {}",
                self.compare_where(diff.at),
                field_words(diff.field, s)
            );
            // A row states the two values where they fit beside their
            // label; a script or a map of key origins does not, and the
            // row says that it differs (§4.11).
            let value = if diff.a.chars().count() <= SHORT && diff.b.chars().count() <= SHORT {
                strings::fill(s.compare_tx_changed, &[&diff.a, &diff.b])
            } else {
                String::from(s.compare_tx_differs)
            };
            rows.push(components::Record::text(label, value, Tone::Text));
        }
        for diff in &comparison.signing {
            let label = alloc::format!(
                "{} \u{00b7} {}",
                strings::fill1(s.compare_tx_input, &alloc::format!("{}", diff.input)),
                s.compare_tx_signing
            );
            let value = match &diff.change {
                SigningChange::OnlyFirst(key) => {
                    strings::fill1(s.compare_tx_signed_first, &short_key(key))
                }
                SigningChange::OnlySecond(key) => {
                    strings::fill1(s.compare_tx_signed_second, &short_key(key))
                }
                SigningChange::FinalizedFirst => String::from(s.compare_tx_final_first),
                SigningChange::FinalizedSecond => String::from(s.compare_tx_final_second),
            };
            rows.push(components::Record::text(label, value, Tone::Text));
        }
        rows
    }

    /// Where a difference is, as the row's label says it.
    fn compare_where(&self, at: Where) -> String {
        let s = self.strings();
        match at {
            Where::Transaction => String::from(s.compare_tx_transaction),
            Where::Input(i) => strings::fill1(s.compare_tx_input, &alloc::format!("{i}")),
            Where::Output(i) => strings::fill1(s.compare_tx_output, &alloc::format!("{i}")),
        }
    }
}

/// How many characters of a value fit beside its label before the row
/// says only that the field differs.
const SHORT: usize = 24;

/// The head of a public key, which is how a key with no origin is named
/// wherever one is named.
fn short_key(key: &str) -> String {
    key.chars().take(8).collect()
}

/// The field's name.
fn field_words(field: Field, s: &Strings) -> &'static str {
    match field {
        Field::Version => s.cmp_version,
        Field::Locktime => s.cmp_locktime,
        Field::InputCount => s.cmp_inputs,
        Field::OutputCount => s.cmp_outputs,
        Field::Spends => s.cmp_spends,
        Field::Sequence => s.cmp_sequence,
        Field::Amount => s.cmp_amount,
        Field::Script => s.cmp_script,
        Field::PreviousTransaction => s.cmp_previous_tx,
        Field::SpentOutput => s.cmp_spent_output,
        Field::RedeemScript => s.cmp_redeem_script,
        Field::WitnessScript => s.cmp_witness_script,
        Field::Sighash => s.cmp_sighash,
        Field::KeyOrigins => s.cmp_key_origins,
        Field::TaprootKeyOrigins => s.cmp_tap_key_origins,
        Field::InternalKey => s.cmp_internal_key,
        Field::MerkleRoot => s.cmp_merkle_root,
        Field::LeafScripts => s.cmp_leaf_scripts,
        Field::Unknown => s.cmp_unknown,
        Field::Proprietary => s.cmp_proprietary,
        Field::Xpubs => s.cmp_xpubs,
    }
}
