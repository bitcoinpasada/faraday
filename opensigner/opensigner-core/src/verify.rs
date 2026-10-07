//! Verify › Address (UX.md §7.4, G1): is this address mine, and which
//! key and index is it? Searches the first [`SEARCH_DEPTH`] receive and
//! change addresses of every wallet in use and of every loaded key and
//! script type. Reading the address is [`osk_bip::address`].

use alloc::string::String;

use osk_bip::keys::{Fingerprint, Network, ScriptType};
use osk_ui::widgets::keyboard::{self, KeyMask, KeyboardKind};

/// Addresses searched per chain, key and script type.
pub const SEARCH_DEPTH: u32 = 100;

/// What the search found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressResult {
    /// Not a Bitcoin address.
    Invalid,
    /// An address of another network than the one set.
    WrongNetwork,
    /// One of ours.
    Yours {
        /// The key.
        fingerprint: Fingerprint,
        /// Its script type.
        script: ScriptType,
        /// Change chain.
        change: bool,
        /// Address index.
        index: u32,
    },
    /// One of a wallet in use.
    Wallet {
        /// Position in the session's wallets.
        wallet: usize,
        /// Change chain.
        change: bool,
        /// Address index.
        index: u32,
    },
    /// A silent payment address (BIP-352): no wallet's chain of
    /// addresses holds one, because nothing is ever paid to it
    /// directly. The screen states what it is and shows it whole
    /// (`docs/PLANNING.md` §16.113).
    Silent,
    /// Not among the first [`SEARCH_DEPTH`] of any loaded key.
    NotFound,
}

/// Where the Verify › Address screen is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyStage {
    /// Typing an address on the ASCII keyboard, which is where the
    /// screen opens: the ways an address arrives are the scanner's rows
    /// now, and this screen is the one of them that types (PLANNING
    /// §16.88).
    Typing,
    /// A result for the address checked.
    Result,
}

/// The screen's state.
pub struct VerifyState {
    stage: VerifyStage,
    input: String,
    result: Option<AddressResult>,
}

impl Default for VerifyState {
    fn default() -> Self {
        Self::new()
    }
}

impl VerifyState {
    /// At the entry menu, with nothing typed.
    pub fn new() -> Self {
        VerifyState {
            stage: VerifyStage::Typing,
            input: String::new(),
            result: None,
        }
    }

    /// Opens the address field.
    pub fn type_address(&mut self) {
        self.stage = VerifyStage::Typing;
        self.input.clear();
        self.result = None;
    }

    /// Where the screen is.
    pub fn stage(&self) -> VerifyStage {
        self.stage
    }

    /// The address typed or checked.
    pub fn input(&self) -> &str {
        &self.input
    }

    /// The result of the last check.
    pub fn result(&self) -> Option<&AddressResult> {
        self.result.as_ref()
    }

    /// Types one character; bech32 and base58 are ASCII, so anything
    /// else is dropped.
    pub fn push(&mut self, c: char) {
        if c.is_ascii_graphic() && self.input.len() < 128 {
            self.input.push(c);
        }
    }

    /// Deletes the last character.
    pub fn pop(&mut self) {
        self.input.pop();
    }

    /// Records a result for `address`.
    pub fn set_result(&mut self, address: &str, result: AddressResult) {
        self.input = String::from(address);
        self.result = Some(result);
        self.stage = VerifyStage::Result;
    }

    /// Back to an empty address field.
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// One step back: a result goes back to the empty field, and the
    /// field itself leaves the screen.
    pub fn back(&mut self) -> bool {
        match self.stage {
            VerifyStage::Typing => false,
            VerifyStage::Result => {
                self.clear();
                true
            }
        }
    }

    /// Whether the text typed so far is an address on some network,
    /// which is what makes ✓ live (§4.3).
    pub fn parses(&self) -> bool {
        osk_bip::address::parses(&self.input)
    }
}

/// The keys the address keyboard still offers after `typed`: those that
/// can begin or continue an address on `network`, and no others (§4.3,
/// the rule the BIP-39 keyboard follows for letters).
pub fn address_keys(typed: &str, network: Network) -> KeyMask {
    let mut mask = 0;
    let mut next = String::from(typed);
    for c in ('a'..='z').chain('A'..='Z').chain('0'..='9') {
        next.truncate(typed.len());
        next.push(c);
        if osk_bip::address::is_address_prefix(&next, network) {
            mask |= keyboard::key_bit(KeyboardKind::Address, c);
        }
    }
    mask
}

impl From<osk_bip::address::Error> for AddressResult {
    fn from(e: osk_bip::address::Error) -> Self {
        match e {
            osk_bip::address::Error::Invalid => AddressResult::Invalid,
            osk_bip::address::Error::WrongNetwork => AddressResult::WrongNetwork,
            osk_bip::address::Error::Silent => AddressResult::Silent,
        }
    }
}
