//! Tools › the calculators (`docs/DESIGN.md` §5): one Entry each, and
//! one Record for the answer.
//!
//! No key, nothing stored, nothing secret, so no screen here carries an
//! eye. What each tool is for is the Learn page "Tools"; a working
//! screen carries labels, values and actions and never an explanation.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_codec::encodings::Encoding;
use osk_ui::components::{self, Denomination};
use osk_ui::geom::SizeClass;
use osk_ui::layout::Node;
use osk_ui::screens::{self, Above, Action, Chrome, Entry, FactRow, Record};
use osk_ui::widgets::keyboard::{self, ALL_KEYS, KeyboardKind};
use osk_ui::widgets::{Icon, Tone};

use crate::strings::Strings;
use crate::tools::Tool;
use crate::{OpenSigner, ids, strings, text};
use osk_bip::compile::{self, PolicyScript};
use osk_bip::descriptor::checksum_facts;
use osk_bip::slip132::{KeyReading, key_facts};
use osk_codec::encodings::ReadAs;

/// How a mode of the Hashes field is named, for the row and its Choice.
pub(crate) fn read_as_name(read_as: ReadAs, s: &Strings) -> &'static str {
    match read_as {
        ReadAs::Auto => s.tool_read_auto,
        ReadAs::Text => s.tool_read_text,
        ReadAs::Hex => s.tool_read_hex,
    }
}

/// How a unit is named, for the mode row, its Choice and the fact rows.
pub(crate) fn unit_name(unit: Denomination, s: &Strings) -> &'static str {
    match unit {
        Denomination::Sat => s.tool_sats,
        Denomination::Btc => s.tool_btc,
        Denomination::MBtc => s.tool_mbtc,
        Denomination::Bits => s.tool_bits,
    }
}

/// How a compiled policy's wrapper is named, for the mode row and its
/// Choice.
pub(crate) fn policy_script_name(script: PolicyScript, s: &Strings) -> &'static str {
    match script {
        PolicyScript::Segwit => s.tool_policy_segwit,
        PolicyScript::Taproot => s.tool_policy_taproot,
    }
}

/// How an encoding is named on the Record's first row.
fn encoding_name(encoding: Encoding, s: &Strings) -> &'static str {
    match encoding {
        Encoding::Base58Check => s.tool_base58check,
        Encoding::Base58 => s.tool_base58,
        Encoding::Bech32 => s.tool_bech32,
        Encoding::Bech32m => s.tool_bech32m,
        Encoding::Hex => s.tool_hex,
    }
}

impl OpenSigner {
    /// §5 Entry: the field of one calculator, with the mode row the two
    /// tools that have one carry above it.
    pub(crate) fn view_tool(&self, tool: Tool) -> Node {
        self.with_chrome(Some(ids::BACK), |c| match tool {
            Tool::Units => self.units_entry(c),
            _ => self.tool_entry(c, tool),
        })
    }

    /// The four calculators whose ✓ opens a Record.
    fn tool_entry(&self, c: &Chrome<'_>, tool: Tool) -> Node {
        let s = self.strings();
        let typed = self.calc.typed();
        let (title, above, error): (&str, Above, Option<String>) = match tool {
            Tool::Hashes => (
                s.tool_hashes_title,
                Above::Fact(FactRow::mode(
                    ids::TOOL_MODE,
                    s.tool_read_as,
                    read_as_name(self.calc.read_as(), s),
                )),
                None,
            ),
            Tool::Encodings => (
                s.tool_encoding_title,
                Above::Nothing,
                (!typed.trim().is_empty() && osk_codec::encodings::read(typed).is_none())
                    .then(|| String::from(s.tool_not_an_encoding)),
            ),
            Tool::Descriptor => (s.tool_descriptor_title, Above::Nothing, None),
            Tool::ConvertKey => (
                s.tool_key_title,
                Above::Nothing,
                match key_facts(typed, self.network) {
                    KeyReading::Private => Some(String::from(s.tool_public_only)),
                    KeyReading::None if !typed.trim().is_empty() => {
                        Some(String::from(s.tool_not_a_key))
                    }
                    _ => None,
                },
            ),
            Tool::Miniscript => (
                s.tool_policy_title,
                {
                    let mode = FactRow::mode(
                        ids::TOOL_MODE,
                        s.tool_policy_script,
                        policy_script_name(self.calc.script(), s),
                    );
                    // The compiler takes a loaded key's fingerprint in
                    // place of the key, so the screen states which
                    // fingerprints it will take.
                    let loaded: Vec<String> = self
                        .policy_key_expressions()
                        .into_iter()
                        .map(|(fingerprint, _)| fingerprint)
                        .collect();
                    if loaded.is_empty() {
                        Above::Fact(mode)
                    } else {
                        Above::Facts(vec![
                            mode,
                            FactRow::new(s.tool_policy_keys, loaded.join(", "), true),
                        ])
                    }
                },
                // The compiler's own words, on one line: what is wrong
                // with the policy is a fact about the policy, and this
                // device has no better way to say it.
                match compile::compile(typed, self.calc.script(), &self.policy_key_expressions()) {
                    Err(reason) if !reason.is_empty() => Some(reason),
                    _ => None,
                },
            ),
            Tool::Units => unreachable!("drawn by units_entry"),
        };
        screens::entry(
            c,
            Entry {
                title,
                value: String::from(typed),
                mono: tool != Tool::Hashes,
                above,
                candidates: None,
                words: None,
                eye: None,
                keyboard: (ids::TOOL_KEYBOARD, KeyboardKind::Passphrase),
                // §4.3: ✓ is dead until the field says something this
                // tool can answer.
                enabled: if self
                    .calc
                    .ready(tool, self.network, &self.policy_key_expressions())
                {
                    ALL_KEYS
                } else {
                    ALL_KEYS | keyboard::DONE_DISABLED
                },
                error,
            },
        )
    }

    /// §5 Entry, "Units": the digit pad and the unit being typed as the
    /// mode row above the field. Where the class has the room, the other
    /// three units are fact rows beside it and change with every digit,
    /// so the screen is the answer; ✓ opens the same three as a Record
    /// either way, which is what a 240 dp panel gets instead of a block
    /// it can only show half of.
    fn units_entry(&self, c: &Chrome<'_>) -> Node {
        let s = self.strings();
        let from = self.calc.from();
        let mode = FactRow::mode(ids::TOOL_MODE, s.tool_from, unit_name(from, s));
        let above = if c.class() == SizeClass::Small {
            Above::Fact(mode)
        } else {
            let mut rows = vec![mode];
            rows.extend(self.unit_facts());
            Above::Facts(rows)
        };
        screens::entry(
            c,
            Entry {
                title: s.tool_units_title,
                value: String::from(self.calc.typed()),
                mono: true,
                above,
                candidates: None,
                words: None,
                eye: None,
                keyboard: (ids::TOOL_KEYBOARD, KeyboardKind::Pin),
                enabled: if self.calc.ready(Tool::Units, self.network, &[]) {
                    ALL_KEYS
                } else {
                    ALL_KEYS | keyboard::DONE_DISABLED
                },
                error: None,
            },
        )
    }

    /// §5 Record, "Units": the amount that was typed first, in the unit
    /// it was typed in, then the same amount in the other three. The
    /// row a person reads back is the one they entered.
    fn unit_rows(&self) -> Vec<components::Record> {
        let s = self.strings();
        let from = self.calc.from();
        let mut rows = alloc::vec![components::Record::mono(
            unit_name(from, s),
            components::denominated(self.calc.sats(), from),
        )];
        rows.extend(
            self.unit_facts()
                .into_iter()
                .map(|f| components::Record::mono(f.label, f.value)),
        );
        rows
    }

    /// The amount in every unit but the one being typed.
    fn unit_facts(&self) -> Vec<FactRow> {
        let s = self.strings();
        let sats = self.calc.sats();
        Denomination::ALL
            .into_iter()
            .filter(|u| *u != self.calc.from())
            .map(|u| FactRow::new(unit_name(u, s), components::denominated(sats, u), true))
            .collect()
    }

    /// §5 Record: the answer of one calculator, its long strings as the
    /// reference rows §4.5 gives every string a person compares.
    pub(crate) fn view_tool_result(&self, tool: Tool) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            let (title, rows) = match tool {
                Tool::Hashes => (s.tools_hashes, self.hash_rows()),
                Tool::Encodings => (s.tools_encodings, self.encoding_rows()),
                Tool::Descriptor => (s.tools_descriptor_checksum, self.checksum_rows()),
                Tool::ConvertKey => (s.tools_convert_key, self.key_rows()),
                Tool::Miniscript => (s.tools_miniscript, self.policy_rows()),
                Tool::Units => (s.tools_units, self.unit_rows()),
            };
            screens::record(
                c,
                Record {
                    title,
                    key: None,
                    network: components::Network::Mainnet,
                    rows,
                    warnings: Vec::new(),
                    pager: None,
                    action: self.tool_action(tool),
                },
            )
        })
    }

    /// The one action a calculator's answer carries: a compiled policy
    /// whose keys are all extended public keys with an origin is a
    /// wallet, and the way on is to review it.
    fn tool_action(&self, tool: Tool) -> Option<Action> {
        if tool != Tool::Miniscript {
            return None;
        }
        self.policy_facts()?
            .wallet
            .as_ref()
            .map(|_| Action::new(ids::TOOL_LOAD_WALLET, self.strings().tool_policy_load))
    }

    /// What the compiler made of what is typed.
    pub(crate) fn policy_facts(&self) -> Option<compile::PolicyFacts> {
        compile::compile(
            self.calc.typed(),
            self.calc.script(),
            &self.policy_key_expressions(),
        )
        .ok()
    }

    /// The compiled descriptor, the ways it can be spent, and the keys
    /// it names.
    fn policy_rows(&self) -> Vec<components::Record> {
        let s = self.strings();
        let Some(f) = self.policy_facts() else {
            return Vec::new();
        };
        let mut rows = self.reference_rows(Tool::Miniscript);
        for (i, path) in f.paths.iter().enumerate() {
            rows.push(components::Record::text(
                strings::fill1(s.wallet_spend_path, &alloc::format!("{}", i + 1)),
                text::spend_path(path, s),
                Tone::Text,
            ));
        }
        for (i, key) in f.keys.iter().enumerate() {
            rows.push(components::Record::mono(
                s.sign_key_row,
                alloc::format!("{} \u{00b7} {key}", text::key_letter(i)),
            ));
        }
        rows
    }

    /// The long strings one calculator's answer carries, each with the
    /// offset of the reference row that shows it. The Record draws them
    /// and a tap on one opens Compare, so both read the same list.
    pub(crate) fn tool_strings(&self, tool: Tool) -> Vec<(usize, &'static str, String)> {
        let s = self.strings();
        match tool {
            Tool::Hashes => {
                let h = osk_bip::hashes::hashes(&self.calc.input());
                vec![
                    (0, s.tool_sha256_row, text::hex(&h.sha256)),
                    (1, s.tool_sha256d_row, text::hex(&h.sha256d)),
                    (2, s.tool_hash160_row, text::hex(&h.hash160)),
                ]
            }
            Tool::Encodings => {
                let Some(r) = osk_codec::encodings::read(self.calc.typed()) else {
                    return Vec::new();
                };
                let mut out = vec![(0, s.row_bytes, text::hex(&r.bytes))];
                if r.encoding == Encoding::Hex {
                    let hrp = osk_bip::address::hrp(self.network);
                    out.push((
                        1,
                        s.tool_base58check,
                        osk_codec::encodings::base58check(0, &r.bytes),
                    ));
                    if let Some(b) = osk_codec::encodings::bech32(hrp, &r.bytes) {
                        out.push((2, s.tool_bech32, b));
                    }
                    if let Some(m) = osk_codec::encodings::bech32m(hrp, &r.bytes) {
                        out.push((3, s.tool_bech32m, m));
                    }
                }
                out
            }
            Tool::Descriptor => match checksum_facts(self.calc.typed()) {
                Some(f) => vec![(0, s.tool_descriptor_row, f.with_checksum)],
                None => Vec::new(),
            },
            Tool::ConvertKey => {
                let KeyReading::Public(f) = key_facts(self.calc.typed(), self.network) else {
                    return Vec::new();
                };
                let mut out = vec![(0, s.tool_bip32_row, f.bip32.clone())];
                for (i, (script, spelling)) in f.slip132.iter().enumerate() {
                    out.push((1 + i, text::script_short(*script, s), spelling.clone()));
                }
                out
            }
            Tool::Miniscript => match self.policy_facts() {
                Some(f) => vec![(0, s.tool_descriptor_row, f.descriptor)],
                None => Vec::new(),
            },
            Tool::Units => Vec::new(),
        }
    }

    /// A reference row for each string a tool worked out (§4.5).
    fn reference_rows(&self, tool: Tool) -> Vec<components::Record> {
        self.tool_strings(tool)
            .into_iter()
            .map(|(at, label, value)| {
                components::Record::reference(ids::at(ids::TOOL_ROW_BASE, at), label, value)
            })
            .collect()
    }

    /// The input's length, then its three hashes.
    fn hash_rows(&self) -> Vec<components::Record> {
        let s = self.strings();
        let input = self.calc.input();
        let mut rows = vec![components::Record::text(
            s.tool_length_row,
            strings::fill1(s.tool_bytes, &alloc::format!("{}", input.len())),
            Tone::Text,
        )];
        rows.extend(self.reference_rows(Tool::Hashes));
        rows
    }

    /// What the string is, what it decodes to, and — from hex — the
    /// other spellings of the same bytes.
    fn encoding_rows(&self) -> Vec<components::Record> {
        let s = self.strings();
        let Some(r) = osk_codec::encodings::read(self.calc.typed()) else {
            return Vec::new();
        };
        let mut strings_left = self.tool_strings(Tool::Encodings).into_iter();
        let mut rows = vec![components::Record::text(
            s.tool_encoding_row,
            encoding_name(r.encoding, s),
            Tone::Text,
        )];
        if let Some((at, label, value)) = strings_left.next() {
            rows.push(components::Record::reference(
                ids::at(ids::TOOL_ROW_BASE, at),
                label,
                value,
            ));
        }
        rows.push(components::Record::text(
            s.tool_length_row,
            strings::fill1(s.tool_bytes, &alloc::format!("{}", r.bytes.len())),
            Tone::Text,
        ));
        if let Some(v) = r.version {
            rows.push(components::Record::mono(
                s.tool_version_row,
                text::hex(&[v]),
            ));
        }
        if let Some(hrp) = &r.hrp {
            rows.push(components::Record::mono(s.tool_hrp_row, hrp.clone()));
        }
        if let Some(ok) = r.checksum {
            rows.push(components::Record::text(
                s.tool_checksum_row,
                if ok {
                    s.tool_checksum_valid
                } else {
                    s.tool_checksum_wrong
                },
                if ok { Tone::Text } else { Tone::Danger },
            ));
        }
        if r.encoding == Encoding::Hex {
            rows.push(components::Record::mono(
                s.tool_hrp_row,
                osk_bip::address::hrp(self.network),
            ));
            for (at, label, value) in strings_left {
                rows.push(components::Record::reference(
                    ids::at(ids::TOOL_ROW_BASE, at),
                    label,
                    value,
                ));
            }
        }
        rows
    }

    /// The descriptor with its checksum, the verdict on the one it
    /// arrived with, and whether it parses as a wallet.
    fn checksum_rows(&self) -> Vec<components::Record> {
        let s = self.strings();
        let Some(f) = checksum_facts(self.calc.typed()) else {
            return Vec::new();
        };
        // The verdict first: a checksum tool answers whether the one
        // that arrived is right, and says so before it says anything
        // else.
        let (verdict, tone) = match f.given {
            Some((_, true)) => (s.tool_checksum_valid, Tone::Text),
            Some((_, false)) => (s.tool_checksum_wrong, Tone::Danger),
            None => (s.tool_checksum_missing, Tone::Caution),
        };
        let mut rows = alloc::vec![
            components::Record::text(s.tool_checksum_row, verdict, tone),
            components::Record::mono(
                s.tool_computed_checksum_row,
                alloc::format!("#{}", f.checksum),
            ),
        ];
        rows.extend(self.reference_rows(Tool::Descriptor));
        if f.wallet.is_some() {
            rows.push(components::Record::action(
                ids::TOOL_LOAD_WALLET,
                Icon::Wallet,
                s.tool_policy_load,
            ));
        }
        rows
    }

    /// Every spelling of one extended public key, and what it says about
    /// itself.
    fn key_rows(&self) -> Vec<components::Record> {
        let s = self.strings();
        let KeyReading::Public(f) = key_facts(self.calc.typed(), self.network) else {
            return Vec::new();
        };
        let mut rows = self.reference_rows(Tool::ConvertKey);
        rows.push(components::Record::text(
            s.tool_network_row,
            f.network.name(),
            Tone::Text,
        ));
        rows.push(components::Record::text(
            s.tool_depth_row,
            alloc::format!("{}", f.depth),
            Tone::Text,
        ));
        rows.push(components::Record::mono(
            s.explore_fingerprint,
            f.fingerprint.clone(),
        ));
        rows.push(components::Record::text(
            s.tool_child_row,
            f.child.clone(),
            Tone::Text,
        ));
        rows
    }
}
