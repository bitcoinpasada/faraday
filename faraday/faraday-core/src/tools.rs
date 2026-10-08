//! Tools: OpenSigner's six calculators (`opensigner_core::tools`), hashes,
//! encodings, a descriptor's checksum, an extended key in every spelling,
//! an amount in every unit and the miniscript compiler, with Faraday's
//! screen over them (`docs/OPENSIGNER-PARITY.md` plan C4). Nothing here
//! is a secret: an extended private key is refused, and nothing is kept.

use opensigner_core::tools::Tool;
use osk_bip::compile::PolicyScript;
use osk_codec::encodings::ReadAs;

use crate::{Faraday, Screen};

/// What the Tools screen holds.
pub struct ToolsState {
    /// The calculator open.
    pub tool: Tool,
    /// What is typed.
    pub typed: String,
    /// How the Hashes field is read.
    pub read_as: ReadAs,
    /// What the Units field is typed in: 0 sat, 1 BTC, 2 mBTC, 3 bits.
    pub unit: u8,
    /// What a compiled policy is wrapped in.
    pub script: PolicyScript,
    /// Where it returns to.
    pub back: Screen,
}

/// A calculator's name, as OpenSigner writes it.
pub fn tool_name(t: Tool) -> &'static str {
    use opensigner_core::strings::EN;
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
            typed: String::new(),
            read_as: ReadAs::Auto,
            unit: 0,
            script: PolicyScript::Segwit,
            back: self.screen,
        });
        self.screen = Screen::Tools;
    }

    /// The loaded keys a policy may name by fingerprint: each one's
    /// multisig account key with both chains.
    pub fn tools_keys(&self) -> Vec<(String, String)> {
        self.session
            .keys
            .iter()
            .filter_map(|k| {
                let text = crate::create::NewKind::Multi.key_text(&k.master).ok()?;
                Some((
                    crate::wallet::fp_text(k.master.fingerprint()),
                    format!("{text}/<0;1>/*"),
                ))
            })
            .collect()
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
    pub(crate) fn tools_key(&mut self, key: osk_shell_api::Key) -> bool {
        use osk_shell_api::Key as K;
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
