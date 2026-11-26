//! Tools: the six calculators: hashes, encodings, a
//! descriptor's checksum, an extended key in every spelling, an amount in
//! every unit and the miniscript compiler, with Faraday's screen over them.
//! Nothing here is a secret: an
//! extended private key is refused. What the open calculator works out
//! of what is typed is kept on its state by what it is worked out from
//! ([`Faraday::tools_reading`]), so a frame drawn again works out
//! nothing.

pub(crate) mod flow;
pub(crate) mod screen;
pub(crate) mod search;

use crate::strings::EN;
use crate::wallet::KeyTexts;
use faraday_bip::compile::PolicyScript;
use faraday_codec::encodings::ReadAs;
use faraday_crypto::work::Compute;
use faraday_text::{Text, text};

use crate::{Faraday, Screen, Sheet};

/// The most characters a scan or a file puts in the field.
const FIELD_MAX: usize = 8192;

/// Whether the calculator has **Scan** and **From Files**: the ones whose
/// input usually arrives as a code or a file.
pub(crate) fn takes_codes(t: Tool) -> bool {
    matches!(t, Tool::Descriptor | Tool::ConvertKey | Tool::Miniscript)
}

/// Whether a file of Files can go into such a field: a wallet description,
/// an account key or plain text, never one that holds a secret.
pub(crate) fn file_fits(kind: crate::wallet::FileKind) -> bool {
    use crate::wallet::FileKind as K;
    matches!(kind, K::Wallet | K::Key | K::Text)
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

/// Every satoshi there will ever be.
pub(crate) const MAX_SATS: u64 = 2_100_000_000_000_000;

/// The most amounts Units holds at once, one tab each.
pub(crate) const MAX_AMOUNTS: usize = 6;

/// One amount of Units: what is typed and the unit it is typed in.
#[derive(Default)]
struct UnitAmount {
    typed: crate::secret_text::SecretText,
    unit: u8,
}

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
    /// Units' amounts, one slot each; the slot of the amount shown holds
    /// nothing, its text and unit being `typed` and `unit`.
    amounts: Vec<UnitAmount>,
    /// Which of the amounts is shown.
    amount: usize,
    /// Where it returns to.
    pub back: Screen,
    /// The step open while the tool is drawn as a flow ([`flow`]).
    pub(crate) open: u8,
    /// How far that flow's card is scrolled.
    pub(crate) scroll: crate::flow::Scroll,
    /// What the calculator worked out last, by what it worked it out
    /// from ([`Faraday::tools_reading`]).
    read: std::cell::RefCell<Option<KeptReading>>,
}

impl ToolsState {
    /// How many amounts Units holds.
    pub fn amount_count(&self) -> usize {
        self.amounts.len()
    }

    /// Which amount is shown.
    pub fn amount_shown(&self) -> usize {
        self.amount
    }

    /// The text and unit of amount `i`.
    pub fn amount_of(&self, i: usize) -> Option<(&str, u8)> {
        if i == self.amount {
            Some((self.typed.as_str(), self.unit))
        } else {
            self.amounts.get(i).map(|a| (a.typed.as_str(), a.unit))
        }
    }

    /// Puts the amount shown away in its slot, moved and not copied;
    /// `typed` and `unit` are then free for another.
    fn park_amount(&mut self) {
        if let Some(slot) = self.amounts.get_mut(self.amount) {
            std::mem::swap(&mut self.typed, &mut slot.typed);
            slot.unit = self.unit;
        }
    }

    /// Takes amount `i` out of its slot to be the one shown.
    fn take_amount(&mut self, i: usize) {
        if let Some(slot) = self.amounts.get_mut(i) {
            std::mem::swap(&mut self.typed, &mut slot.typed);
            self.unit = slot.unit;
            self.amount = i;
        }
    }

    /// Shows amount `i`, keeping the one shown as it is.
    fn show_amount(&mut self, i: usize) {
        if self.tool != Tool::Units || i == self.amount || i >= self.amounts.len() {
            return;
        }
        self.park_amount();
        self.take_amount(i);
    }

    /// A new, empty amount, shown; none past [`MAX_AMOUNTS`].
    fn add_amount(&mut self) {
        if self.tool != Tool::Units || self.amounts.len() >= MAX_AMOUNTS {
            return;
        }
        self.park_amount();
        self.amounts.push(UnitAmount::default());
        self.take_amount(self.amounts.len() - 1);
    }

    /// Takes amount `i` away; the last one cannot go. When the one shown
    /// goes, the one that took its place is shown.
    fn remove_amount(&mut self, i: usize) {
        if self.tool != Tool::Units || self.amounts.len() <= 1 || i >= self.amounts.len() {
            return;
        }
        let shown = i == self.amount;
        if shown {
            self.typed.clear();
        } else {
            self.park_amount();
        }
        self.amounts.remove(i);
        // Where the amount that is shown stands now.
        let now = if shown {
            i.min(self.amounts.len() - 1)
        } else if i < self.amount {
            self.amount - 1
        } else {
            self.amount
        };
        self.take_amount(now);
    }
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
    Miniscript(Result<faraday_bip::compile::PolicyFacts, Text>),
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
    keys: Vec<(Text, Text)>,
    reading: std::rc::Rc<Reading>,
}

/// A calculator's name.
pub(crate) fn tool_name(t: Tool) -> &'static str {
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
pub(crate) const UNITS: [(&str, u64); 4] = [
    ("sat", 1),
    ("BTC", 100_000_000),
    ("mBTC", 100_000),
    ("bits", 100),
];

/// The places after the point an amount in unit `unit` (an index into
/// [`UNITS`]) is typed to: none for sat, 8 for BTC, 5 for mBTC, 2 for
/// bits, so that the last place is one satoshi.
pub(crate) fn unit_places(unit: u8) -> usize {
    UNITS
        .get(usize::from(unit))
        .map_or(0, |(_, f)| f.ilog10() as usize)
}

/// The satoshi an amount typed in unit `unit` is, up to every satoshi
/// there will be: the whole part and up to [`unit_places`] after the
/// point; nothing typed, or only a point, is 0.
pub(crate) fn units_sats(typed: &str, unit: u8) -> u64 {
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

/// A page's name and its way back, as the top bar draws them.
type FlowHead = (Text, Option<(Text, crate::Action)>);

/// Tools' state: the open calculator, and what is typed in the one search
/// of every menu page.
#[derive(Default)]
pub struct ToolsArea {
    /// The calculator on show, while Tools is open.
    tools: Option<ToolsState>,
    /// What is typed in the search: one text for every menu page.
    home_find: Text,
    /// The words of the Home search whose result was pressed, kept for
    /// the way back from what it opened (`in_progress`'s place).
    home_find_kept: Text,
    /// Whether Home's search bar has been pressed, so it shows its caret
    /// while empty.
    home_focus: bool,
    /// Where Home's search bar was last drawn in its page, so typing can
    /// scroll the page to it.
    home_bar_y: std::cell::Cell<f32>,
    /// Whether the last bar drawn held the three pills (not the one Start
    /// button): Wallets' page leaves out the rows they repeat.
    pills_up: std::cell::Cell<bool>,
    /// Whether the page is to follow the results after the next frame.
    home_follow: std::cell::Cell<bool>,
    /// The name and way back a flow's step last drew in the top bar, so a
    /// search typed over it keeps them (`docs/WORKSPACE.md` §4).
    flow_head: std::cell::RefCell<Option<FlowHead>>,
    /// Convert a seed, a tool with a flow of its own.
    pub(crate) convert: crate::convert::ConvertArea,
    /// The Krux code tool.
    pub(crate) kef_tool: crate::kef_tool::KefToolArea,
    /// Check a timestamp.
    pub(crate) ots: crate::ots_check::OtsArea,
    /// My seed makes my wallet.
    pub(crate) seed_check: crate::seed_check::SeedArea,
    /// Check a multisig setup.
    pub(crate) mscheck: crate::multisig_check::MsArea,
    /// Read a Bitcoin Core wallet.
    pub(crate) core_wallet: crate::core_wallet::tool::CoreWalletArea,
    /// Decode a Lightning invoice.
    pub(crate) invoice: crate::invoice::InvoiceArea,
    /// Check an address.
    pub(crate) address_check: crate::address_check::AddressCheckArea,
}

impl ToolsArea {
    /// The open calculator's state, with Tools open.
    pub fn state(&self) -> Option<&ToolsState> {
        self.tools.as_ref()
    }

    pub(crate) fn set_scroll(&mut self, to: crate::flow::Scroll) {
        if let Some(t) = self.tools.as_mut() {
            t.scroll = to;
        }
    }

    /// How far the flow's card is scrolled, units, while Tools is open.
    pub(crate) fn scroll_y(&self) -> Option<f32> {
        self.tools.as_ref().map(|t| t.scroll.y)
    }

    /// The flow's card scrolled to `y` units, while Tools is open.
    pub(crate) fn set_scroll_y(&mut self, y: f32) {
        if let Some(t) = self.tools.as_mut() {
            t.scroll.y = y;
        }
    }

    /// Opens the Descriptor calculator with `text` typed in, for the test kit.
    #[cfg(feature = "testkit")]
    pub(crate) fn type_descriptor(&mut self, text: &str) {
        if let Some(t) = self.tools.as_mut() {
            t.tool = Tool::Descriptor;
            for ch in text.chars() {
                t.typed.push(ch);
            }
        }
    }

    /// Whether the xpub calculator is open with nothing typed: where
    /// Practice's Use a sample is drawn. A lookup.
    pub(crate) fn xpub_empty(&self) -> bool {
        self.tools
            .as_ref()
            .is_some_and(|t| t.tool == Tool::ConvertKey && t.typed.is_empty())
    }

    /// Types `text` into the field, as a person would.
    pub(crate) fn type_sample(&mut self, text: &str) {
        if let Some(t) = self.tools.as_mut() {
            t.typed.clear();
            for ch in text.chars() {
                t.typed.push(ch);
            }
        }
    }

    /// What is typed in the search.
    pub fn home_find(&self) -> &str {
        &self.home_find
    }

    /// Clears Home's search bar, so Home shows Home again.
    pub(crate) fn clear_home_find(&mut self) {
        self.home_find.clear();
        self.home_focus = false;
    }

    /// Keeps the words in Home's search bar, as a result is pressed.
    pub(crate) fn keep_home_find(&mut self) {
        self.home_find_kept = self.home_find.clone();
    }

    /// Puts the kept words back in Home's search bar, and forgets them.
    pub(crate) fn restore_home_find(&mut self) {
        self.home_find = std::mem::take(&mut self.home_find_kept);
    }

    /// Forgets the kept words.
    pub(crate) fn drop_home_find_kept(&mut self) {
        self.home_find_kept.clear();
    }

    /// Whether Home's search bar has been pressed and shows its caret.
    pub(crate) fn home_focused(&self) -> bool {
        self.home_focus
    }

    /// Keeps the heading a flow's step drew in the top bar.
    pub(crate) fn note_flow_head(&self, head: Option<&FlowHead>) {
        *self.flow_head.borrow_mut() = head.cloned();
    }

    /// The heading a flow's step last drew in the top bar.
    pub(crate) fn flow_head(&self) -> Option<FlowHead> {
        self.flow_head.borrow().clone()
    }

    /// Keeps whether the bar drew the three pills.
    pub(crate) fn note_pills(&self, up: bool) {
        self.pills_up.set(up);
    }

    /// Whether the last bar drawn held the three pills.
    pub(crate) fn pills_up(&self) -> bool {
        self.pills_up.get()
    }

    /// Keeps where Home's search bar is drawn in its page.
    pub(crate) fn note_home_bar(&self, y: f32) {
        self.home_bar_y.set(y);
    }

    /// The page offset that puts Home's search bar near the top, past
    /// which scrolling to the results' foot does not go.
    pub(crate) fn home_bar_offset(&self) -> f32 {
        (self.home_bar_y.get() - 12.0).max(0.0)
    }

    /// Asks for the page to be scrolled after the next frame to the
    /// results' foot, no further than the bar at its top.
    pub(crate) fn follow_home_find(&self) {
        self.home_follow.set(true);
    }

    /// Takes the ask of [`ToolsArea::follow_home_find`].
    pub(crate) fn take_home_follow(&self) -> bool {
        self.home_follow.replace(false)
    }

    /// Takes the caret off Home's search bar, as a press on another field
    /// does.
    pub(crate) fn unfocus_home_find(&mut self) {
        self.home_focus = false;
    }

    /// Focuses Home's search bar, as a press on it does.
    pub(crate) fn focus_home_find(&mut self) {
        self.home_focus = true;
    }

    /// Opens Tools, returning to `back`.
    pub(crate) fn open(&mut self, back: Screen) {
        self.tools = Some(ToolsState {
            tool: Tool::Hashes,
            typed: crate::secret_text::SecretText::new(),
            read_as: ReadAs::Auto,
            unit: 0,
            script: PolicyScript::Segwit,
            amounts: vec![UnitAmount::default()],
            amount: 0,
            back,
            open: flow::INPUT,
            scroll: crate::flow::Scroll::default(),
            read: std::cell::RefCell::new(None),
        });
    }

    /// Drops what a screen left behind: Tools' calculator unless Tools is
    /// on screen, an invoice's text unless its screen is.
    pub(crate) fn tidy(&mut self, screen: Screen, card_open: impl Fn(usize) -> bool) {
        if screen != Screen::Tools {
            self.tools = None;
        }
        if screen != Screen::Invoice {
            self.invoice.clear();
        }
        let held = self.address_check.held_for().is_some_and(card_open);
        if !held {
            self.address_check.release();
        }
        if screen != Screen::CheckAddress && !held {
            self.address_check.clear();
        }
    }

    /// Costs what `account_text` costs. The loaded keys a policy may name
    /// by fingerprint: each one's multisig account key with both chains.
    pub fn keys(c: &Compute, keys: &faraday_keys::Keys) -> Vec<(Text, Text)> {
        keys.all()
            .enumerate()
            .filter_map(|(i, k)| {
                let text = keys
                    .account_text(c, i, crate::create::NewKind::Multi, 0)
                    .ok()?;
                Some((
                    crate::wallet::fp_text(k.fingerprint),
                    text!("{}/<0;1>/*", &*text),
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
        // The library takes the token and key as its own owned text: these
        // are public account keys, copied only when a policy is compiled.
        let named: Vec<_> = keys
            .iter()
            .map(|(token, key)| ((&**token).into(), (&**key).into()))
            .collect();
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
                faraday_bip::compile::compile(c, &t.typed, t.script, &named)
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
                    t.unit = 0;
                    t.amounts = vec![UnitAmount::default()];
                    t.amount = 0;
                    t.open = flow::INPUT;
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
            A::TAddAmount => t.add_amount(),
            A::TAmount(i) => t.show_amount(usize::from(i)),
            A::TRemoveAmount(i) => t.remove_amount(usize::from(i)),
            // The result waits for something typed.
            A::TStep(k)
                if k <= flow::RESULT
                    && t.tool != Tool::Units
                    && t.open != k
                    && (k == flow::INPUT || !t.typed.is_empty()) =>
            {
                t.open = k;
                t.scroll.follow = true;
            }
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

    /// Typing on a menu page goes to its search bar. Returns whether the key was
    /// taken.
    pub(crate) fn home_find_key(&mut self, key: faraday_shell_api::Key) -> bool {
        use faraday_shell_api::Key as K;
        match key {
            K::Char(c) if !c.is_control() && self.home_find.chars().count() < 40 => {
                self.home_find.push(c);
            }
            K::Backspace => {
                self.home_find.pop();
            }
            K::Escape => self.home_find.clear(),
            _ => return false,
        }
        true
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
    pub fn tools_keys(&self, c: &Compute) -> Vec<(Text, Text)> {
        ToolsArea::keys(c, &self.session.keys)
    }

    /// Costs what the open calculator works out the first time, a lookup
    /// after ([`ToolsArea::reading`]).
    pub(crate) fn tools_reading(&self, c: &Compute) -> Option<std::rc::Rc<Reading>> {
        self.tools.reading(c, &self.session)
    }

    /// A press in the Convert or Krux code tool.
    pub(crate) fn tool_act(&mut self, action: crate::Action) {
        match action {
            crate::Action::Convert(c) => crate::convert::act(self, c),
            crate::Action::KefTool(k) => crate::kef_tool::act(self, k),
            crate::Action::OtsCheck(k) => crate::ots_check::act(self, k),
            crate::Action::SeedCheck(k) => crate::seed_check::act(self, k),
            crate::Action::CoreWallet(k) => crate::core_wallet::tool::act(self, k),
            _ => {}
        }
        crate::stick::keep_focus_off(self);
    }

    /// Typing into one of those tools. Returns whether the key was taken.
    pub(crate) fn tool_key(&mut self, key: faraday_shell_api::Key) -> bool {
        crate::convert::key(self, key)
            || crate::kef_tool::key(self, key)
            || crate::ots_check::key(self, key)
            || crate::seed_check::key(self, key)
            || crate::core_wallet::tool::key(self, key)
    }

    /// After a press: those tools end unless they are on show.
    pub(crate) fn tool_tidy(&mut self) {
        crate::convert::tidy(self);
        crate::kef_tool::tidy(self);
        crate::ots_check::tidy(self);
        crate::seed_check::tidy(self);
        crate::multisig_check::tidy(self);
        crate::core_wallet::tool::tidy(self);
        // Leaving Transfer wipes the text typed or pasted for Send text.
        #[cfg(feature = "demo")]
        if self.screen != Screen::Transfer {
            self.transfer.clear_text();
        }
    }

    /// One press on the screen.
    pub(crate) fn tools_act(&mut self, action: crate::Action) {
        if action == crate::Action::Tools {
            return self.tools_open();
        }
        if action == crate::Action::TTake {
            return self.tools_take();
        }
        match action {
            crate::Action::TScan => self.tools_scan(),
            crate::Action::TFile(i) => self.tools_file(i),
            _ => self.tools.act(action),
        }
    }

    /// **Scan** under the field: the scanner for text into it.
    fn tools_scan(&mut self) {
        if !self.tools.state().is_some_and(|t| takes_codes(t.tool)) {
            return;
        }
        let mut scan = crate::qr_view::ScanState::default();
        scan.purpose = crate::ScanPurpose::ToolText;
        self.camera.open(scan);
        self.sheet = Some(Sheet::Scan);
        self.out
            .commands
            .push_back(faraday_shell_api::Command::CameraOn);
    }

    /// **From Files**: the file at `i` in Files read into the field, as
    /// typed text is. A file of another kind, or one with a secret, is not.
    fn tools_file(&mut self, i: usize) {
        if !self.tools.state().is_some_and(|t| takes_codes(t.tool)) {
            return;
        }
        let Some(item) = self.inbox.get(i) else {
            return;
        };
        if !file_fits(item.kind) {
            return;
        }
        let text = faraday_text::Text::from_utf8_lossy(&item.bytes);
        self.tools_fill(text.trim());
    }

    /// The field takes `text` in place of what it holds, or a toast says
    /// it does not fit.
    fn tools_fill(&mut self, text: &str) {
        if text.chars().count() > FIELD_MAX {
            self.toast(EN.tools_too_long);
            return;
        }
        if let Some(t) = self.tools.tools.as_mut() {
            t.typed.set(text);
        }
    }

    /// Costs what [`Faraday::scanned_plain_text`] costs. A code read on
    /// Convert an xpub's, Descriptor checksum's or Miniscript's scanner:
    /// its text goes into the field and the scanner closes; a secret or
    /// any other kind of code is named on the scanner and nothing goes in.
    /// Returns whether the scanner on screen is that one.
    pub(crate) fn tools_scanned(&mut self, bytes: &zeroize::Zeroizing<Vec<u8>>) -> bool {
        use crate::scan_read::Kind;
        if self.camera.scan().map(|s| s.purpose) != Some(crate::ScanPurpose::ToolText) {
            return false;
        }
        let Some(text) = self.scanned_plain_text(
            bytes,
            crate::scan_read::secret_kind,
            &[Kind::Text, Kind::Wallet, Kind::AccountKey],
        ) else {
            return true;
        };
        if text.trim().chars().count() > FIELD_MAX {
            self.scan_say(EN.tools_too_long);
            return true;
        }
        self.tools_fill(text.trim());
        self.scan_close();
        true
    }

    /// Costs what `add_wallet` costs: the typed descriptor goes to Wallets
    /// as a descriptor with its origins does, through the wallet
    /// confirmation sheet where it asks for one.
    fn tools_take(&mut self) {
        let Some(t) = self.tools.state() else {
            return;
        };
        let text = Text::of(t.typed.trim());
        let c = self.compute();
        match self.session.add_wallet(
            &c,
            EN.tools_wallet,
            &text,
            EN.tools_typed_in_a_calculator,
            crate::wallet::Origin::Named,
        ) {
            Ok(i) => {
                self.wallets.set_wallet(i);
                self.screen = Screen::Wallets;
                self.refresh_spend();
            }
            // Held for its sheet: its hold runs this again and opens Wallets.
            Err(crate::wallet::Refusal::Unconfirmed) => {}
            Err(e) => self.toast(&e.text()),
        }
    }

    /// Typing into the field. Returns whether the key was taken.
    pub(crate) fn tools_key(&mut self, key: faraday_shell_api::Key) -> bool {
        #[cfg(feature = "demo")]
        if self.transfer_key(key) {
            return true;
        }
        if self.screen == Screen::Invoice {
            return crate::invoice::key(self, key);
        }
        if self.screen == Screen::CheckAddress {
            return crate::address_check::key(self, key);
        }
        if self.screen != Screen::Tools {
            return false;
        }
        // On a flow's result card the field is not there to type in.
        if flow::contained_tool(self)
            && self.tools.state().is_some_and(|t| t.open != flow::INPUT)
            && matches!(
                key,
                faraday_shell_api::Key::Char(_) | faraday_shell_api::Key::Backspace
            )
        {
            return false;
        }
        let (taken, back) = self.tools.key(key);
        if let Some(back) = back {
            self.screen = back;
        }
        taken
    }

    /// Typing on a menu page goes to its search bar, on every window but
    /// the Pi's menu. Returns whether the key was taken.
    pub(crate) fn home_key(&mut self, key: faraday_shell_api::Key) -> bool {
        let in_flow = crate::screens::in_flow(self) && self.tools.home_focused();
        if !(self.home_search_here() || in_flow) {
            return false;
        }
        // Escape is the search's only while something is typed; with
        // nothing typed it goes on to leave the page or the step.
        if key == faraday_shell_api::Key::Escape && self.tools.home_find().is_empty() {
            return false;
        }
        if !self.tools.home_find_key(key) {
            return false;
        }
        if self.screen != Screen::Home {
            self.list_offset = 0.0;
        }
        // The results stand under the buttons: the page scrolls to the
        // foot, as far as it reaches, so the bar and its results show.
        self.tools.follow_home_find();
        true
    }

    /// After a frame: Home's page scrolled to its results' foot, no
    /// further than the search bar at its top, once something typed or
    /// cleared asked for it.
    pub(crate) fn follow_home_find(&mut self) {
        if !self.tools.take_home_follow() {
            return;
        }
        let Some(e) = self.region_now() else {
            return;
        };
        let want = e.max.min(self.tools.home_bar_offset()).max(0.0);
        if (want - self.list_offset).abs() > 0.5 {
            self.list_offset = want;
            self.out.redraw();
            self.out
                .commands
                .push_back(faraday_shell_api::Command::Draw);
        }
    }

    /// Whether the page on show has the search and takes typing: a menu
    /// page's, not on the Pi's menu (its compact, short Home) and not under
    /// a sheet. Tools' phone page has it too.
    pub(crate) fn home_search_here(&self) -> bool {
        crate::screens::bar_on(self.screen)
            && self.sheet.is_none()
            && (self.screen == Screen::Catalog || self.nav() != crate::nav::Nav::Menu)
    }
}

/// What the Miniscript compiler's line says when it refuses: the app's
/// words for which kind, then the compiler's own message where it has one.
fn refusal_text(r: &faraday_bip::compile::Refusal) -> Text {
    use faraday_bip::compile::Refusal;
    match r {
        Refusal::Empty => Text::new(),
        Refusal::Policy(m) => Text::fill(EN.tools_miniscript_not_policy, &[m]),
        Refusal::Script(m) => Text::fill(EN.tools_miniscript_no_script, &[m]),
        Refusal::LoneKeyTaproot => Text::of(EN.tools_miniscript_lone_key),
    }
}

impl Faraday {
    /// What the start-up self-test found, once it has run: how many
    /// checks passed, or the name of the first that failed.
    pub fn selftest(&self) -> Option<faraday_selftest::Outcome> {
        self.selftest
    }

    /// The self-test has run and every check passed.
    ///
    /// Used by faraday-host.
    #[doc(hidden)]
    pub fn selftest_passed(&self) -> bool {
        matches!(self.selftest, Some(Ok(_)))
    }

    /// The self-test has run and a check failed: the app is blocked.
    pub(crate) fn selftest_failed(&self) -> bool {
        matches!(self.selftest, Some(Err(_)))
    }

    /// Runs `checks` as the self-test, in place of
    /// [`faraday_selftest::CHECKS`]: for a test that a failed check blocks the
    /// app.
    #[doc(hidden)]
    #[cfg(feature = "testkit")]
    pub fn run_selftest_with(&mut self, checks: &[faraday_selftest::Check]) {
        let c = self.compute();
        self.selftest = Some(faraday_selftest::run_checks(&c, checks));
        if self.selftest_failed() {
            self.sheet = None;
        }
        self.out.redraw();
    }
}

// An area that holds areas: its own fields are listed with no rest
// pattern, so a new tool does not compile until it says what Lock does
// with it (`docs/AT-LOCK.md` §5).
impl crate::at_lock::AtLock for ToolsArea {
    fn holds(&self) -> crate::at_lock::Holds {
        use crate::at_lock::{AtLock as A, Holds, public};
        let ToolsArea {
            tools,
            home_find,
            home_find_kept,
            home_focus,
            home_bar_y,
            pills_up,
            home_follow,
            flow_head,
            convert,
            kef_tool,
            ots,
            seed_check,
            mscheck,
            core_wallet,
            invoice,
            address_check,
        } = self;
        public(home_find);
        public(home_find_kept);
        public(home_focus);
        public(home_bar_y);
        public(pills_up);
        public(home_follow);
        public(ots);
        public(invoice);
        public(address_check);
        // A copy of the heading the step on screen draws: what a name in
        // it belongs to is counted where it is kept. Lock takes the copy,
        // and so does the first frame outside a flow.
        let _ = flow_head;
        // The calculator's typed line: an extended private key typed is
        // refused, but it was typed.
        Holds::typed(tools.as_ref().is_some_and(|t| !t.typed.is_empty()))
            .or(A::holds(convert))
            .or(A::holds(kef_tool))
            .or(A::holds(seed_check))
            .or(A::holds(mscheck))
            .or(A::holds(core_wallet))
    }
    fn lock(&mut self) {
        use crate::at_lock::AtLock as A;
        let ToolsArea {
            tools,
            home_find: _,
            home_find_kept,
            home_focus: _,
            home_bar_y: _,
            pills_up: _,
            home_follow: _,
            flow_head,
            convert,
            kef_tool,
            ots: _,
            seed_check,
            mscheck,
            core_wallet,
            invoice: _,
            address_check: _,
        } = self;
        *tools = None;
        home_find_kept.clear();
        flow_head.take();
        A::lock(convert);
        A::lock(kef_tool);
        A::lock(seed_check);
        A::lock(mscheck);
        A::lock(core_wallet);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::at_lock::AtLock;

    /// A flow's heading can carry a wallet's name: a lock drops it. It
    /// counts as nothing held, so a step with nothing taken holds nothing.
    #[test]
    fn a_noted_flow_heading_goes_at_a_lock() {
        let mut area = ToolsArea::default();
        let head: FlowHead = (Text::of("Make a new wallet: Savings"), None);
        area.note_flow_head(Some(&head));
        // A copy of what the step on screen draws: nothing of its own to
        // lock, and a lock takes it all the same.
        assert!(!area.holds().any());
        area.lock();
        assert!(!area.holds().any());
        assert!(area.flow_head().is_none());
    }
}
