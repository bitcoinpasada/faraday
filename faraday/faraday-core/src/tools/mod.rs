//! Tools: the six calculators: hashes, encodings, a
//! descriptor's checksum, an extended key in every spelling, an amount in
//! every unit and the miniscript compiler, with Faraday's screen over them.
//! Nothing here is a secret: an
//! extended private key is refused. What the open calculator works out
//! of what is typed is kept on its state by what it is worked out from
//! ([`Faraday::tools_reading`]), so a frame drawn again works out
//! nothing.

pub(crate) mod screen;

use crate::strings::EN;
use faraday_bip::compile::PolicyScript;
use faraday_codec::encodings::ReadAs;
use faraday_crypto::work::Compute;

use crate::{Faraday, Screen};

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

/// Every satoshi there will ever be.
pub const MAX_SATS: u64 = 2_100_000_000_000_000;

/// What the Tools screen holds.
pub struct ToolsState {
    /// The calculator open.
    pub tool: Tool,
    /// What is typed: an extended private key is refused, but it was
    /// typed.
    pub typed: crate::secret_text::SecretText,
    /// How the Hashes field is read.
    pub read_as: ReadAs,
    /// What the Units field is typed in: 0 sat, 1 BTC, 2 mBTC, 3 bits.
    pub unit: u8,
    /// What a compiled policy is wrapped in.
    pub script: PolicyScript,
    /// Where it returns to.
    pub back: Screen,
    /// What the calculator worked out last, by what it worked it out
    /// from ([`Faraday::tools_reading`]).
    read: std::cell::RefCell<Option<KeptReading>>,
}

/// What the open calculator worked out of what is typed.
pub enum Reading {
    /// Nothing to work out: the amount is converted as it is drawn.
    Nothing,
    /// The three hashes of the bytes typed.
    Hashes(faraday_bip::hashes::Hashes),
    /// What the string is encoded in, when it is.
    Encodings(Option<faraday_codec::encodings::Reading>),
    /// The descriptor's checksum and whether it is a wallet here.
    Descriptor(Option<faraday_bip::descriptor::ChecksumFacts>),
    /// The extended key in every spelling.
    Key(faraday_bip::slip132::KeyReading),
    /// The policy compiled, or the compiler's refusal.
    Miniscript(Result<faraday_bip::compile::PolicyFacts, String>),
}

/// A [`Reading`] with what it was worked out from: the calculator, the
/// text typed (wiped when dropped), its modes, the network and the keys
/// a policy may name.
struct KeptReading {
    tool: Tool,
    typed: crate::secret_text::SecretText,
    read_as: ReadAs,
    script: PolicyScript,
    network: faraday_bip::keys::Network,
    keys: Vec<(String, String)>,
    reading: std::rc::Rc<Reading>,
}

/// A calculator's name.
pub fn tool_name(t: Tool) -> &'static str {
    match t {
        Tool::Hashes => EN.tools_hashes,
        Tool::Encodings => EN.tools_encodings,
        Tool::Descriptor => EN.tools_descriptor_checksum,
        Tool::ConvertKey => EN.tools_convert_key,
        Tool::Units => EN.tools_units,
        Tool::Miniscript => EN.tools_miniscript,
    }
}

/// Satoshi in one of each unit, in the order the unit row lists them.
pub const UNITS: [(&str, u64); 4] = [
    ("sat", 1),
    ("BTC", 100_000_000),
    ("mBTC", 100_000),
    ("bits", 100),
];

impl Faraday {
    /// Opens Tools.
    pub(crate) fn tools_open(&mut self) {
        self.tools = Some(ToolsState {
            tool: Tool::Hashes,
            typed: crate::secret_text::SecretText::new(),
            read_as: ReadAs::Auto,
            unit: 0,
            script: PolicyScript::Segwit,
            back: self.screen,
            read: std::cell::RefCell::new(None),
        });
        self.screen = Screen::Tools;
    }

    /// Costs what `account_text` costs. The loaded keys a policy may name
    /// by fingerprint: each one's multisig account key with both chains.
    pub fn tools_keys(&self, c: &Compute) -> Vec<(String, String)> {
        self.session
            .keys
            .iter()
            .filter_map(|k| {
                let text = k.account_text(c, crate::create::NewKind::Multi, 0).ok()?;
                Some((
                    crate::wallet::fp_text(k.fingerprint()),
                    format!("{text}/<0;1>/*"),
                ))
            })
            .collect()
    }

    /// Costs what the open calculator works out the first time (a parse
    /// of a descriptor, a policy compiled, the hashes), a lookup after:
    /// what it worked out of what is typed, kept on its state by the
    /// calculator, the text, its modes, the network and the keys a policy
    /// may name. `None` with Tools closed or nothing typed.
    pub(crate) fn tools_reading(&self, c: &Compute) -> Option<std::rc::Rc<Reading>> {
        use faraday_codec::encodings::{read as encoding_of, read_input};
        let t = self.tools.as_ref()?;
        if t.typed.trim().is_empty() {
            return None;
        }
        let network = self.session.network();
        let keys = if t.tool == Tool::Miniscript {
            self.tools_keys(c)
        } else {
            Vec::new()
        };
        if let Some(k) = t.read.borrow().as_ref()
            && k.tool == t.tool
            && k.typed == *t.typed.as_str()
            && k.read_as == t.read_as
            && k.script == t.script
            && k.network == network
            && k.keys == keys
        {
            return Some(k.reading.clone());
        }
        let reading = std::rc::Rc::new(match t.tool {
            Tool::Hashes => Reading::Hashes(faraday_bip::hashes::hashes(&read_input(
                t.typed.trim(),
                t.read_as,
            ))),
            Tool::Encodings => Reading::Encodings(encoding_of(t.typed.trim())),
            Tool::Descriptor => {
                Reading::Descriptor(faraday_bip::descriptor::checksum_facts(c, &t.typed))
            }
            Tool::ConvertKey => Reading::Key(faraday_bip::slip132::key_facts(&t.typed, network)),
            Tool::Units => Reading::Nothing,
            Tool::Miniscript => {
                Reading::Miniscript(faraday_bip::compile::compile(c, &t.typed, t.script, &keys))
            }
        });
        *t.read.borrow_mut() = Some(KeptReading {
            tool: t.tool,
            typed: t.typed.clone(),
            read_as: t.read_as,
            script: t.script,
            network,
            keys,
            reading: reading.clone(),
        });
        Some(reading)
    }

    /// One press on the screen.
    pub(crate) fn tools_act(&mut self, action: crate::Action) {
        use crate::Action as A;
        if action == A::Tools {
            return self.tools_open();
        }
        let Some(t) = self.tools.as_mut() else {
            return;
        };
        match action {
            A::TTool(i) => {
                if let Some(&tool) = Tool::ALL.get(usize::from(i)) {
                    t.tool = tool;
                    t.typed.clear();
                }
            }
            A::TMode(i) => match t.tool {
                Tool::Hashes => {
                    if let Some(&r) = ReadAs::ALL.get(usize::from(i)) {
                        t.read_as = r;
                    }
                }
                Tool::Units => t.unit = i.min(3),
                Tool::Miniscript => {
                    if let Some(&s) = PolicyScript::ALL.get(usize::from(i)) {
                        t.script = s;
                    }
                }
                _ => {}
            },
            A::TClear => t.typed.clear(),
            _ => {}
        }
    }

    /// Typing into the field. Returns whether the key was taken.
    pub(crate) fn tools_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        if self.screen != Screen::Tools {
            return false;
        }
        let Some(t) = self.tools.as_mut() else {
            return false;
        };
        match key {
            K::Char(c) if t.tool == Tool::Units && !c.is_ascii_digit() => return true,
            K::Char(c) => {
                if t.typed.chars().count() < 1024 {
                    t.typed.push(c);
                }
            }
            K::Backspace => {
                t.typed.pop();
            }
            K::Escape => {
                self.screen = t.back;
                self.tools = None;
            }
            _ => return false,
        }
        true
    }
}
