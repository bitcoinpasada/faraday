//! Tools: the six calculators: hashes, encodings, a
//! descriptor's checksum, an extended key in every spelling, an amount in
//! every unit and the miniscript compiler, with Faraday's screen over them.
//! Nothing here is a secret: an
//! extended private key is refused. What the open calculator works out
//! of what is typed is kept on its state by what it is worked out from
//! ([`Faraday::tools_reading`]), so a frame drawn again works out
//! nothing.

pub(crate) mod screen;

use crate::strings::{EN, fill};
use crate::wallet::KeyTexts;
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
    /// The three hashes of the bytes typed; `None` when the text is not
    /// the hex it is read as.
    Hashes(Option<faraday_bip::hashes::Hashes>),
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

/// The places after the point an amount in unit `unit` (an index into
/// [`UNITS`]) is typed to: none for sat, 8 for BTC, 5 for mBTC, 2 for
/// bits, so that the last place is one satoshi.
pub fn unit_places(unit: u8) -> usize {
    UNITS
        .get(usize::from(unit))
        .map_or(0, |(_, f)| f.ilog10() as usize)
}

/// The satoshi an amount typed in unit `unit` is, up to every satoshi
/// there will be: the whole part and up to [`unit_places`] after the
/// point; nothing typed, or only a point, is 0.
pub fn units_sats(typed: &str, unit: u8) -> u64 {
    let factor = UNITS.get(usize::from(unit)).map_or(1, |(_, f)| *f);
    let (whole, frac) = typed.split_once('.').unwrap_or((typed, ""));
    let whole: u64 = whole.parse().unwrap_or(0);
    let mut part: u64 = 0;
    for k in 0..unit_places(unit) {
        let d = frac
            .as_bytes()
            .get(k)
            .filter(|b| b.is_ascii_digit())
            .map_or(0, |b| u64::from(b - b'0'));
        part = part * 10 + d;
    }
    whole
        .saturating_mul(factor)
        .saturating_add(part)
        .min(MAX_SATS)
}

/// Whether the Units field, holding `typed` in unit `unit`, takes `c`:
/// digits, and one point in a unit with places after it, as many places
/// as reach one satoshi.
fn units_takes(typed: &str, unit: u8, c: char) -> bool {
    let places = unit_places(unit);
    match typed.split_once('.') {
        _ if c == '.' => places > 0 && !typed.contains('.'),
        Some((_, frac)) => c.is_ascii_digit() && frac.len() < places,
        None => c.is_ascii_digit(),
    }
}

/// Tools' state: the open calculator, and what is typed in the Tools
/// page's Find a tool.
#[derive(Default)]
pub struct ToolsArea {
    /// The calculator on show, while Tools is open.
    tools: Option<ToolsState>,
    /// What is typed in Tools' Find a tool.
    catalog_find: String,
}

impl ToolsArea {
    /// The open calculator's state, with Tools open.
    pub fn state(&self) -> Option<&ToolsState> {
        self.tools.as_ref()
    }

    /// What is typed in Find a tool.
    pub fn find(&self) -> &str {
        &self.catalog_find
    }

    /// Opens Tools, returning to `back`.
    pub(crate) fn open(&mut self, back: Screen) {
        self.tools = Some(ToolsState {
            tool: Tool::Hashes,
            typed: crate::secret_text::SecretText::new(),
            read_as: ReadAs::Auto,
            unit: 0,
            script: PolicyScript::Segwit,
            back,
            read: std::cell::RefCell::new(None),
        });
    }

    /// Closes Tools and keeps nothing.
    pub(crate) fn close(&mut self) {
        self.tools = None;
    }

    /// Costs what `account_text` costs. The loaded keys a policy may name
    /// by fingerprint: each one's multisig account key with both chains.
    pub fn keys(c: &Compute, keys: &faraday_keys::Keys) -> Vec<(String, String)> {
        keys.all()
            .enumerate()
            .filter_map(|(i, k)| {
                let text = keys
                    .account_text(c, i, crate::create::NewKind::Multi, 0)
                    .ok()?;
                Some((
                    crate::wallet::fp_text(k.fingerprint),
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
    pub(crate) fn reading(
        &self,
        c: &Compute,
        session: &crate::wallet::Session,
    ) -> Option<std::rc::Rc<Reading>> {
        use faraday_codec::encodings::{read as encoding_of, read_input};
        let t = self.tools.as_ref()?;
        if t.typed.trim().is_empty() {
            return None;
        }
        let network = session.network();
        let keys = if t.tool == Tool::Miniscript {
            Self::keys(c, &session.keys)
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
            Tool::Hashes => Reading::Hashes(
                read_input(t.typed.trim(), t.read_as)
                    .map(|bytes| faraday_bip::hashes::hashes(&bytes)),
            ),
            Tool::Encodings => Reading::Encodings(encoding_of(t.typed.trim())),
            Tool::Descriptor => {
                Reading::Descriptor(faraday_bip::descriptor::checksum_facts(c, &t.typed))
            }
            Tool::ConvertKey => Reading::Key(faraday_bip::slip132::key_facts(&t.typed, network)),
            Tool::Units => Reading::Nothing,
            Tool::Miniscript => Reading::Miniscript(
                faraday_bip::compile::compile(c, &t.typed, t.script, &keys)
                    .map_err(|r| refusal_text(&r)),
            ),
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

    /// One press on the screen, other than opening it.
    pub(crate) fn act(&mut self, action: crate::Action) {
        use crate::Action as A;
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
                Tool::Units => {
                    t.unit = i.min(3);
                    // Places the new unit does not have are dropped.
                    let places = unit_places(t.unit);
                    if let Some(at) = t.typed.find('.') {
                        let keep = if places == 0 { at } else { at + 1 + places };
                        t.typed.truncate(keep);
                    }
                }
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

    /// Typing into the field. Returns whether the key was taken and,
    /// for Escape, where the screen goes back to.
    pub(crate) fn key(&mut self, key: faraday_shell_api::Key) -> (bool, Option<Screen>) {
        use faraday_shell_api::Key as K;
        let Some(t) = self.tools.as_mut() else {
            return (false, None);
        };
        match key {
            K::Char(c) if t.tool == Tool::Units && !units_takes(&t.typed, t.unit, c) => {}
            K::Char(c) => {
                if t.typed.chars().count() < 1024 {
                    t.typed.push(c);
                }
            }
            K::Backspace => {
                t.typed.pop();
            }
            K::Escape => {
                let back = t.back;
                self.tools = None;
                return (true, Some(back));
            }
            _ => return (false, None),
        }
        (true, None)
    }

    /// Typing on the Tools page goes to Find a tool. Returns whether the
    /// key was taken.
    pub(crate) fn find_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        match key {
            K::Char(c) if !c.is_control() && self.catalog_find.chars().count() < 40 => {
                self.catalog_find.push(c);
            }
            K::Backspace => {
                self.catalog_find.pop();
            }
            K::Escape => self.catalog_find.clear(),
            _ => return false,
        }
        true
    }

    /// The calculator's state, for the test kit.
    #[cfg(feature = "testkit")]
    pub(crate) fn state_mut(&mut self) -> Option<&mut ToolsState> {
        self.tools.as_mut()
    }
}

impl Faraday {
    /// Opens Tools.
    pub(crate) fn tools_open(&mut self) {
        self.tools.open(self.screen);
        self.screen = Screen::Tools;
    }

    /// Costs what `account_text` costs. The loaded keys a policy may name
    /// by fingerprint ([`ToolsArea::keys`]).
    pub fn tools_keys(&self, c: &Compute) -> Vec<(String, String)> {
        ToolsArea::keys(c, &self.session.keys)
    }

    /// Costs what the open calculator works out the first time, a lookup
    /// after ([`ToolsArea::reading`]).
    pub(crate) fn tools_reading(&self, c: &Compute) -> Option<std::rc::Rc<Reading>> {
        self.tools.reading(c, &self.session)
    }

    /// One press on the screen.
    pub(crate) fn tools_act(&mut self, action: crate::Action) {
        if action == crate::Action::Tools {
            return self.tools_open();
        }
        self.tools.act(action);
    }

    /// Typing into the field. Returns whether the key was taken.
    pub(crate) fn tools_key(&mut self, key: faraday_shell_api::Key) -> bool {
        if self.screen != Screen::Tools {
            return false;
        }
        let (taken, back) = self.tools.key(key);
        if let Some(back) = back {
            self.screen = back;
        }
        taken
    }

    /// Typing on the Tools page goes to Find a tool. Returns whether the
    /// key was taken.
    pub(crate) fn catalog_key(&mut self, key: faraday_shell_api::Key) -> bool {
        if self.screen != Screen::Catalog || !self.tools.find_key(key) {
            return false;
        }
        self.list_offset = 0.0;
        true
    }
}

/// What the Miniscript compiler's line says when it refuses: the app's
/// words for which kind, then the compiler's own message.
fn refusal_text(r: &faraday_bip::compile::Refusal) -> String {
    use faraday_bip::compile::Refusal;
    match r {
        Refusal::Empty => String::new(),
        Refusal::Policy(m) => fill(EN.tools_miniscript_not_policy, &[m]),
        Refusal::Script(m) => fill(EN.tools_miniscript_no_script, &[m]),
    }
}
