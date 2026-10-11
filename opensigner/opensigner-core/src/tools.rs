//! Tools › the standalone calculators (`docs/DESIGN.md` §5): hashes,
//! encodings, a descriptor checksum, a key conversion, the units, and
//! the miniscript compiler.
//!
//! Nothing here is a secret and nothing is stored. Each tool takes a
//! string or a number, works something out from it, and shows the
//! answer; no screen carries an eye, and leaving the tool drops what
//! was typed. What each one works out is in the core crates
//! (`docs/PLANNING.md` §16.139): `osk_bip::hashes`,
//! `osk_codec::encodings`, `osk_bip::descriptor::checksum_facts`,
//! `osk_bip::slip132::key_facts` and `osk_bip::compile`. This file holds
//! what is typed into them.

use alloc::string::String;
use alloc::vec::Vec;

use osk_bip::compile::{PolicyScript, compile};
use osk_bip::descriptor::checksum_facts;
use osk_bip::keys::Network;
use osk_bip::slip132::{KeyReading, key_facts};
use osk_codec::encodings::{self, ReadAs, read_input};
use osk_psbt::bitcoin::Txid;
use osk_psbt::verify::Comparison;
use osk_ui::components::Denomination;

/// Tools › Compare transactions: the two PSBTs read one after the
/// other, and what differs between them (`docs/PLANNING.md` §16.111
/// rule 4).
///
/// Nothing here is secret and nothing is kept: leaving the tool drops
/// the first transaction and the comparison with it.
#[derive(Debug, Default)]
pub struct CompareTransactions {
    first: Option<osk_psbt::Psbt>,
    comparison: Option<Comparison>,
}

impl CompareTransactions {
    /// An empty tool, waiting for the first transaction.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the first transaction has been read.
    pub fn has_first(&self) -> bool {
        self.first.is_some()
    }

    /// The id of the first transaction, which is what the row that
    /// names it shows.
    pub fn first_txid(&self) -> Option<Txid> {
        Some(self.first.as_ref()?.unsigned_tx().compute_txid())
    }

    /// Takes `psbt` as the first transaction, dropping whatever was
    /// there.
    pub fn set_first(&mut self, psbt: osk_psbt::Psbt) {
        self.first = Some(psbt);
        self.comparison = None;
    }

    /// Compares `psbt` with the first, which must have been read.
    pub fn compare_with(&mut self, psbt: &osk_psbt::Psbt) {
        if let Some(first) = self.first.as_ref() {
            self.comparison = Some(osk_psbt::verify::compare(first, psbt));
        }
    }

    /// What differs, once both have been read.
    pub fn comparison(&self) -> Option<&Comparison> {
        self.comparison.as_ref()
    }

    /// Drops both transactions and the comparison.
    pub fn clear(&mut self) {
        self.first = None;
        self.comparison = None;
    }
}

/// The calculators, in the order Tools lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// SHA-256, SHA-256d and HASH160 of what is typed.
    Hashes,
    /// What a string is encoded in and what it decodes to.
    Encodings,
    /// A descriptor's checksum, and whether the one it arrived with
    /// holds.
    Descriptor,
    /// Every spelling of one extended public key.
    ConvertKey,
    /// One amount in all four units at once.
    Units,
    /// A policy compiled into a descriptor.
    Miniscript,
}

impl Tool {
    /// The six, in the order Tools lists them.
    pub const ALL: [Tool; 6] = [
        Tool::Hashes,
        Tool::Encodings,
        Tool::Descriptor,
        Tool::ConvertKey,
        Tool::Units,
        Tool::Miniscript,
    ];
}

/// How many characters a calculator's field takes. Long enough for a
/// multisig descriptor, which is the longest string any of them reads.
const MAX_CHARS: usize = 1024;

/// How many digits the Units field takes: 21 million bitcoin is fifteen
/// digits of satoshi.
const MAX_DIGITS: usize = 16;

/// The state of whichever calculator is open. One is open at a time, so
/// one field, one mode and one unit serve all five.
#[derive(Debug, Default)]
pub struct Calculator {
    typed: String,
    read_as: Option<ReadAs>,
    from: Denomination,
    script: PolicyScript,
}

impl Calculator {
    /// An empty field, read automatically, counting satoshi.
    pub fn new() -> Self {
        Calculator {
            typed: String::new(),
            read_as: None,
            from: Denomination::Sat,
            script: PolicyScript::Segwit,
        }
    }

    /// What a compiled policy is wrapped in.
    pub fn script(&self) -> PolicyScript {
        self.script
    }

    /// Sets that, which leaves what is typed where it is: the same
    /// policy in the other wrapper is another descriptor, and the fact
    /// rows say what it is.
    pub fn set_script(&mut self, script: PolicyScript) {
        self.script = script;
    }

    /// What has been typed.
    pub fn typed(&self) -> &str {
        &self.typed
    }

    /// How the Hashes field is read.
    pub fn read_as(&self) -> ReadAs {
        self.read_as.unwrap_or(ReadAs::Auto)
    }

    /// Forces how the field is read.
    pub fn set_read_as(&mut self, read_as: ReadAs) {
        self.read_as = Some(read_as);
    }

    /// Which unit the Units field is typed in.
    pub fn from(&self) -> Denomination {
        self.from
    }

    /// Sets that unit, which leaves the digits where they are: the same
    /// number in another unit is another amount, and the fact rows say
    /// what it is.
    pub fn set_from(&mut self, from: Denomination) {
        self.from = from;
    }

    /// Empties the field.
    pub fn clear(&mut self) {
        self.typed.clear();
    }

    /// Puts `text` in the field, in place of whatever was there: what a
    /// scan or a paste hands the tool. The field's own limits still
    /// hold, so a payload longer than the tool takes is cut to it.
    pub fn set(&mut self, tool: Tool, text: String) {
        self.typed.clear();
        for c in text.chars() {
            if !self.push(tool, c) {
                break;
            }
        }
    }

    /// Types one character; ignored where the field is full or the tool
    /// takes no such character.
    pub fn push(&mut self, tool: Tool, c: char) -> bool {
        let max = if tool == Tool::Units {
            MAX_DIGITS
        } else {
            MAX_CHARS
        };
        if tool == Tool::Units && !c.is_ascii_digit() {
            return false;
        }
        if self.typed.chars().count() >= max {
            return false;
        }
        self.typed.push(c);
        true
    }

    /// Removes the last character.
    pub fn pop(&mut self) {
        self.typed.pop();
    }

    /// The amount the Units field names, in satoshi, capped at the
    /// twenty-one million that exist.
    pub fn sats(&self) -> u64 {
        let digits: u64 = self.typed.parse().unwrap_or(0);
        digits.saturating_mul(self.from.sat()).min(MAX_SATS)
    }

    /// Whether ✓ leads anywhere: the field says something this tool can
    /// work out an answer from.
    pub fn ready(&self, tool: Tool, network: Network, keys: &[(String, String)]) -> bool {
        match tool {
            Tool::Hashes => !self.typed.trim().is_empty(),
            Tool::Encodings => encodings::read(&self.typed).is_some(),
            Tool::Descriptor => checksum_facts(&self.typed).is_some(),
            Tool::ConvertKey => matches!(key_facts(&self.typed, network), KeyReading::Public(_)),
            // The Units screen is the answer on a class with the room
            // for it, and ✓ opens the same three units as a Record on
            // one without.
            Tool::Units => !self.typed.is_empty(),
            Tool::Miniscript => compile(&self.typed, self.script, keys).is_ok(),
        }
    }

    /// The bytes the Hashes field names under the mode it is read in.
    pub fn input(&self) -> Vec<u8> {
        read_input(self.typed.trim(), self.read_as())
    }
}

/// Every satoshi there will ever be.
pub const MAX_SATS: u64 = 2_100_000_000_000_000;
