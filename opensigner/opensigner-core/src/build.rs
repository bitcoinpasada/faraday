//! Add a wallet (`docs/PLANNING.md` §16.104 rule 2, §16.106): one wizard
//! for all six kinds.
//!
//! The wizard asks the kind first, then — for FROST — how many keys the
//! group has and how many must sign, then which loaded keys it is built
//! from, then what the kind needs: the script type for a single-sig or a
//! multisig wallet, how many must sign for a multisig one, nothing for
//! MuSig2 or a taproot multisig, and for a recovery wallet up to three
//! recovery paths — the keys that spend later, the wait before they can,
//! each wait longer than the last — and the form. Every kind ends
//! at the same wallet review and "Add this wallet"; accepting a FROST
//! review runs the dealer, whose computed keys are shown as words,
//! quizzed and loaded.
//!
//! What it produces is what a scanned policy produces, so the wallet is
//! on Wallets, kept with the keys on a device that keeps them, and
//! exported like any other.
//!
//! Nothing here is secret: an account extended public key, a fingerprint
//! and a derivation path are the public half of a key. The dealt secrets
//! live in [`crate::threshold::Dealer`], which holds no heap text.

use alloc::string::String;
use alloc::vec::Vec;

use osk_bip::bitcoin::NetworkKind;
use osk_bip::bitcoin::bip32::{DerivationPath, Xpub};
use osk_bip::coldcard;
use osk_bip::keys::{Fingerprint, MultisigScriptType, Network, ScriptType};
use osk_bip::musig;
pub use osk_bip::policy::PolicyKey;
use osk_bip::policy::WalletPolicy;
use osk_bip::{multisig_config, slip132, xkey};

use osk_bip::recovery::{self, Form, Path, Recovery, RecoveryPolicy};

use crate::threshold::{COUNTS, Dealer, MAX_SHARES};

/// Keys a multisig wallet takes, which is what `OP_CHECKMULTISIG`
/// takes.
pub const MAX_KEYS: usize = 15;

/// The delays "After how long?" offers, in days. A wait outside them is
/// typed on the Days pad.
pub const DELAYS: [u32; 4] = [30, 90, 180, 365];

/// The recovery paths the wizard gathers. A descriptor read from a
/// coordinator may hold any number, and the review states every one;
/// this is how many the wizard asks for.
pub const MAX_WIZARD_PATHS: usize = 3;

/// The digits the Days pad takes, which is what [`recovery::MAX_DAYS`]
/// has.
const MAX_DAYS_DIGITS: usize = 3;

/// The kinds of wallet Add a wallet builds, in the order the Choice
/// lists them: the three every coordinator speaks, then the two under
/// one key, then the one with a delay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletKind {
    /// One key, one script type.
    Single,
    /// `sortedmulti` over two to fifteen keys, `k` of which sign.
    Multisig,
    /// `tr(H,sortedmulti_a(k,…))`: BIP 387's tapscript multisig under
    /// BIP 341's NUMS point, so the only way to spend is the leaf.
    TaprootMultisig,
    /// `tr(musig(…))`, where every key signs.
    MuSig2,
    /// A FROST group: `t` of its `n` 24-word keys sign a taproot key
    /// path.
    Frost,
    /// The primary keys spend today; the recovery keys spend once the
    /// delay has passed.
    Recovery,
    /// A silent payments wallet over one key (BIP-352): a scan key and
    /// a spend key at BIP-352's paths, an address a payer can publish,
    /// and outputs no descriptor lists (`docs/PLANNING.md` §16.113).
    Silent,
}

impl WalletKind {
    /// Every kind, in the order the Choice lists them.
    pub const ALL: [WalletKind; 7] = [
        WalletKind::Single,
        WalletKind::Multisig,
        WalletKind::TaprootMultisig,
        WalletKind::MuSig2,
        WalletKind::Frost,
        WalletKind::Recovery,
        WalletKind::Silent,
    ];

    /// Whether the wallet is built on one key, which is what makes a
    /// tap on another row move the check rather than be refused.
    pub fn one_key(self) -> bool {
        matches!(self, WalletKind::Single | WalletKind::Silent)
    }

    /// Whether a cosigner that is not a key of this device can join,
    /// which is what puts "Scan a key" under the loaded keys.
    pub fn takes_cosigners(self) -> bool {
        matches!(
            self,
            WalletKind::Multisig
                | WalletKind::MuSig2
                | WalletKind::TaprootMultisig
                | WalletKind::Recovery
        )
    }

    /// The script types the kind's Script step offers, as indices into
    /// [`ScriptType::ALL`]. An empty list is a kind with no script step.
    pub fn scripts(self) -> &'static [usize] {
        match self {
            WalletKind::Single => &[0, 1, 2, 3],
            WalletKind::Multisig => &[1, 2],
            // A recovery wallet is one script or one tree, which is
            // what `osk_bip::recovery::Form` offers.
            WalletKind::Recovery => &[2, 3],
            _ => &[],
        }
    }
}

/// The account each multisig script type reads a cosigner's key at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `wsh(sortedmulti(k,@0/**,…))`, keys at BIP-48 `2'`.
    SegWit,
    /// `sh(wsh(sortedmulti(k,…)))`, keys at BIP-48 `1'`.
    Nested,
    /// `tr(musig(@0,@1,…)/**)`, keys at BIP-86.
    MuSig,
}

impl Kind {
    /// The BIP-48 account a cosigner contributes, for the two multisig
    /// types.
    pub fn multisig(self) -> Option<MultisigScriptType> {
        match self {
            Kind::SegWit => Some(MultisigScriptType::NativeSegwit),
            Kind::Nested => Some(MultisigScriptType::NestedSegwit),
            Kind::MuSig => None,
        }
    }

    /// The account a Coldcard export offers for this script type.
    pub fn cosigner(self) -> coldcard::Cosigner {
        match self.multisig() {
            Some(script) => coldcard::Cosigner::Multisig(script),
            None => coldcard::Cosigner::Taproot,
        }
    }
}

/// Where the wizard is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Which kind of wallet.
    Kind,
    /// FROST: how many keys the group has.
    Count,
    /// FROST: how many of them must sign.
    Quorum,
    /// Which keys the wallet is built from. For a recovery wallet these
    /// are the keys that spend with no wait.
    Keys,
    /// Recovery: which keys spend once the wait has passed.
    Later,
    /// Recovery: how many of those must sign.
    LaterThreshold,
    /// Recovery: how long the wait is.
    Delay,
    /// Recovery: a wait of its own number of days, typed on the pad.
    Days,
    /// Recovery: whether a further path follows this one.
    Another,
    /// Typing a cosigner's key, which the scanner's "Type" row opens.
    Type,
    /// Which script type.
    Script,
    /// How many must sign, for a multisig wallet.
    Threshold,
    /// The wallet, as its own review.
    Review,
    /// One computed FROST key's words.
    Words,
    /// The helper toggle and "Start quiz".
    QuizStart,
    /// That key's quiz.
    Quiz,
    /// The caution behind "Skip quiz".
    QuizSkip,
    /// The group record as a QR, with Save and Continue.
    Record,
    /// Leaving with keys gathered: keep them or discard them.
    Discard,
    /// Started from a key's page: whether the wallet is over that key
    /// or over a passphrase key of its words (`docs/PLANNING.md`
    /// §16.129 rule 2).
    Passphrase,
}

/// Why a key read here cannot join the wallet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The text is no key at all.
    NotAKey,
    /// The text is a whole wallet, which is loaded rather than built.
    Wallet,
    /// An extended public key with no origin, which BIP-388 has no form
    /// for in a wallet of more than one key.
    NoOrigin,
    /// A mainnet key on a device on a test network.
    Mainnet,
    /// A test-network key on a mainnet device.
    Testnet,
    /// A key the wallet already holds.
    Duplicate,
}

/// One key of the wallet being built.
#[derive(Debug, Clone)]
pub struct BuiltKey {
    key: PolicyKey,
    /// The loaded key this is, where it is one of this device's. A
    /// scanned cosigner has none.
    loaded: Option<usize>,
    /// The text a scanned cosigner was read from, so that changing the
    /// script type reads it again at the account the new type asks for.
    source: Option<String>,
}

impl BuiltKey {
    /// The key itself, with the origin it states.
    pub fn key(&self) -> &PolicyKey {
        &self.key
    }

    /// Whether this device holds the private key of this cosigner,
    /// which is §4.4's glyph.
    pub fn mine(&self) -> bool {
        self.loaded.is_some()
    }

    /// Which loaded key this is, where it is one.
    pub fn loaded(&self) -> Option<usize> {
        self.loaded
    }

    /// The text a scanned cosigner was read from.
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }
}

/// Why the number typed on the Days pad cannot be the wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaysRefusal {
    /// More days than a relative timelock can hold.
    TooMany,
    /// Not longer than the wait of the path before this one. The first
    /// path's floor is zero days, so a typed zero lands here too.
    TooShort,
}

/// One recovery path of the wallet being built: who spends once its
/// wait has passed, and how long that wait is.
struct LaterPath {
    keys: Vec<BuiltKey>,
    /// How many of those keys must sign.
    threshold: usize,
    /// The wait, in days.
    days: u32,
    /// Whether the wait is the pad's rather than one of [`DELAYS`].
    typed: bool,
}

impl LaterPath {
    /// A path whose wait starts at the shortest offered one longer than
    /// `previous`, which is the wait of the path before it and zero for
    /// the first. Where no offered wait is longer, the path opens on the
    /// typed row, a day after the one before it.
    fn new(previous: u32) -> LaterPath {
        let offered = DELAYS.iter().copied().find(|days| *days > previous);
        LaterPath {
            keys: Vec::new(),
            threshold: 1,
            days: offered.unwrap_or_else(|| (previous + 1).min(recovery::MAX_DAYS)),
            typed: offered.is_none(),
        }
    }
}

/// The wallet being built. See the module documentation.
pub struct WalletWizard {
    step: Step,
    /// The step the discard confirm covers, which "Keep" returns to.
    resume: Step,
    /// Index into [`WalletKind::ALL`].
    kind: usize,
    /// Index into [`ScriptType::ALL`].
    script: usize,
    keys: Vec<BuiltKey>,
    /// Signatures a multisig wallet requires.
    threshold: usize,
    /// FROST: how many keys the group has.
    n: u8,
    /// FROST: how many of them must sign.
    t: u8,
    /// Recovery: the paths that spend once their waits have passed,
    /// in the order they were gathered, which is shortest wait first.
    later: Vec<LaterPath>,
    /// Recovery: which of those paths the keys being gathered belong to,
    /// and `None` while the primary path's are. One Keys screen serves
    /// every path, so which list a tap lands in is state rather than a
    /// reading of the step.
    gather_later: Option<usize>,
    /// Recovery: whether "Another path later?" has Yes checked.
    another: bool,
    /// Recovery: the days typed on the Days pad.
    typed_days: String,
    /// A cosigner's key being typed on the [`Step::Type`] entry. Public
    /// data: an extended public key and the origin it states.
    typed: String,
    /// Why the key on that entry cannot join, once ✓ has been pressed
    /// on a key the reader accepted and the wallet refused.
    type_refusal: Option<Refusal>,
    /// The deal, once a FROST review has been accepted.
    dealer: Option<Dealer>,
    /// The loaded key the wizard was started with, from that key's page
    /// (§16.129 rule 1). `None` from Wallets › Add a wallet.
    in_hand: Option<usize>,
    /// Whether "Passphrase?" has Yes checked.
    passphrase: bool,
    /// Whether the kind step went on past the Keys step with the key in
    /// hand, which is where Back from the step after returns to.
    keys_skipped: bool,
}

impl Default for WalletWizard {
    fn default() -> Self {
        Self::new()
    }
}

impl WalletWizard {
    /// The wizard as it opens: the kind step, single-sig checked, SegWit
    /// as the script type, three keys and two signatures for FROST.
    pub fn new() -> Self {
        WalletWizard {
            step: Step::Kind,
            resume: Step::Keys,
            kind: 0,
            script: 2,
            keys: Vec::new(),
            threshold: 2,
            later: Vec::new(),
            gather_later: None,
            another: false,
            typed_days: String::new(),
            n: COUNTS[0],
            t: 2,
            typed: String::new(),
            type_refusal: None,
            dealer: None,
            in_hand: None,
            passphrase: false,
            keys_skipped: false,
        }
    }

    /// The wizard as a key's page opens it: "Passphrase?" with No
    /// checked, and the loaded key at `key` in hand (§16.129 rules 1
    /// and 2).
    pub fn from_key(key: usize) -> Self {
        WalletWizard {
            step: Step::Passphrase,
            in_hand: Some(key),
            ..Self::new()
        }
    }

    /// The loaded key the wizard was started with, where it was started
    /// from a key's page.
    pub fn in_hand(&self) -> Option<usize> {
        self.in_hand
    }

    /// Puts the loaded key at `key` in hand, which is what a passphrase
    /// opened on the way does: the wizard goes on over the new key.
    pub fn set_in_hand(&mut self, key: usize) {
        self.in_hand = Some(key);
    }

    /// Whether "Passphrase?" has Yes checked.
    pub fn passphrase(&self) -> bool {
        self.passphrase
    }

    /// Checks one of its two rows.
    pub fn set_passphrase(&mut self, yes: bool) {
        self.passphrase = yes;
    }

    /// Records whether the kind step went on past the Keys step.
    pub fn set_keys_skipped(&mut self, skipped: bool) {
        self.keys_skipped = skipped;
    }

    /// Which step.
    pub fn step(&self) -> Step {
        self.step
    }

    /// Goes to `step`. Arriving at one of the key steps says which of a
    /// recovery wallet's paths the taps that follow gather into, and
    /// arriving at a recovery path's first step starts that path.
    pub fn go(&mut self, step: Step) {
        match step {
            Step::Keys => self.gather_later = None,
            Step::Later => {
                if self.later.is_empty() {
                    self.later.push(LaterPath::new(0));
                }
                self.gather_later = Some(self.later.len() - 1);
            }
            // §4.2: a question opens on its default, and going on
            // without another path is the default.
            Step::Another => self.another = false,
            Step::Days => self.typed_days.clear(),
            _ => {}
        }
        self.step = step;
    }

    /// The kind being built.
    pub fn kind(&self) -> WalletKind {
        WalletKind::ALL[self.kind]
    }

    /// Checks the kind at `index`, which starts the keys over: what
    /// counts as a usable key, and the account each contributes,
    /// differ between the four.
    pub fn set_kind(&mut self, index: usize) {
        if index < WalletKind::ALL.len() && index != self.kind {
            self.kind = index;
            self.keys.clear();
            self.later.clear();
            self.script = if WalletKind::ALL[index] == WalletKind::Multisig {
                2
            } else {
                self.script
            };
        }
    }

    /// The script type, as an index into [`ScriptType::ALL`].
    pub fn script_index(&self) -> usize {
        self.script
    }

    /// The script type the wallet pays to.
    pub fn script(&self) -> ScriptType {
        ScriptType::ALL[self.script.min(ScriptType::ALL.len() - 1)]
    }

    /// Checks the script type at `index` among [`ScriptType::ALL`].
    pub fn set_script(&mut self, index: usize) {
        if self.kind().scripts().contains(&index) {
            self.script = index;
        }
    }

    /// The account kind a cosigner of this wallet contributes, which is
    /// what a scanned key is read at.
    pub fn account_kind(&self) -> Kind {
        match self.kind() {
            WalletKind::MuSig2 => Kind::MuSig,
            // BIP 48 assigns no account to a tapscript multisig and none
            // to a recovery wallet; both read a cosigner at `2'`, which
            // is the convention §16.106 records and the tests use.
            WalletKind::TaprootMultisig | WalletKind::Recovery => Kind::SegWit,
            _ => match self.script() {
                ScriptType::NestedSegwit => Kind::Nested,
                _ => Kind::SegWit,
            },
        }
    }

    /// How many keys a FROST group has.
    pub fn n(&self) -> usize {
        usize::from(self.n)
    }

    /// How many of them must sign.
    pub fn t(&self) -> usize {
        usize::from(self.t)
    }

    /// Checks a FROST key count.
    pub fn set_n(&mut self, n: u8) {
        self.n = n;
        // A threshold can never exceed the count, and two is the
        // smallest that is a threshold at all.
        self.t = self.t.clamp(2, self.n);
    }

    /// Checks how many of them must sign.
    pub fn set_t(&mut self, t: u8) {
        if t >= 2 && t <= self.n {
            self.t = t;
        }
    }

    /// The keys gathered so far, in the order they were chosen. On the
    /// Later step of a recovery wallet this is that path's list, so that
    /// one Keys screen serves every path.
    pub fn keys(&self) -> &[BuiltKey] {
        if self.on_later() {
            self.later_keys()
        } else {
            &self.keys
        }
    }

    /// Whether the step now on screen gathers a recovery path's keys.
    /// The typing entry counts: it was opened from one of the lists.
    pub fn on_later(&self) -> bool {
        self.gather_later.is_some() && matches!(self.step, Step::Later | Step::Type)
    }

    /// The keys that spend with no wait.
    pub fn primary_keys(&self) -> &[BuiltKey] {
        &self.keys
    }

    /// Which recovery path the later steps answer for: the one being
    /// gathered, or the last one started.
    pub fn later_path(&self) -> usize {
        let last = self.later.len().saturating_sub(1);
        self.gather_later.unwrap_or(last).min(last)
    }

    /// How many recovery paths the wallet has so far.
    pub fn later_paths(&self) -> usize {
        self.later.len()
    }

    /// The keys of the recovery path the later steps answer for.
    pub fn later_keys(&self) -> &[BuiltKey] {
        self.later_path_keys(self.later_path())
    }

    /// The keys of the recovery path at `i`.
    pub fn later_path_keys(&self, i: usize) -> &[BuiltKey] {
        self.later.get(i).map_or(&[], |p| p.keys.as_slice())
    }

    /// The step a key typed at the entry returns to.
    pub fn typing_from(&self) -> Step {
        if self.gather_later.is_some() {
            Step::Later
        } else {
            Step::Keys
        }
    }

    /// The list the step now on screen gathers into.
    fn gathered_mut(&mut self) -> &mut Vec<BuiltKey> {
        match self.gather_later.filter(|_| self.on_later()) {
            Some(i) if i < self.later.len() => &mut self.later[i].keys,
            _ => &mut self.keys,
        }
    }

    /// How many of that path's keys must sign, never more than it has.
    pub fn later_threshold(&self) -> usize {
        let path = self.later_path();
        let keys = self.later_path_keys(path).len().max(1);
        self.later
            .get(path)
            .map_or(1, |p| p.threshold)
            .clamp(1, keys)
    }

    /// Checks that.
    pub fn set_later_threshold(&mut self, threshold: usize) {
        let path = self.later_path();
        if let Some(p) = self.later.get_mut(path) {
            p.threshold = threshold;
        }
    }

    /// The wait before that path can spend, in days.
    pub fn days(&self) -> u32 {
        self.later.get(self.later_path()).map_or(0, |p| p.days)
    }

    /// Checks a wait among the offered ones.
    pub fn set_days(&mut self, days: u32) {
        if days <= self.previous_days() {
            return;
        }
        let path = self.later_path();
        if let Some(p) = self.later.get_mut(path) {
            p.days = days;
            p.typed = false;
        }
    }

    /// Checks the row that asks for a number of days instead. The wait
    /// itself does not change until one is typed.
    pub fn set_typed_wait(&mut self) {
        let path = self.later_path();
        if let Some(p) = self.later.get_mut(path) {
            p.typed = true;
        }
    }

    /// Whether this path's wait is the typed row's rather than one of
    /// the offered ones.
    pub fn typed_wait(&self) -> bool {
        self.later.get(self.later_path()).is_some_and(|p| p.typed)
    }

    /// The wait of the path before this one, in days, and zero for the
    /// first: every later path opens after the one before it.
    pub fn previous_days(&self) -> u32 {
        match self.later_path() {
            0 => 0,
            i => self.later.get(i - 1).map_or(0, |p| p.days),
        }
    }

    /// That wait in blocks, which is what the script holds.
    pub fn delay_blocks(&self) -> u32 {
        recovery::blocks_for_days(self.days()).unwrap_or(recovery::MAX_DELAY)
    }

    /// What has been typed on the Days pad.
    pub fn days_typed(&self) -> &str {
        &self.typed_days
    }

    /// Types one digit of it.
    pub fn days_push(&mut self, c: char) {
        if c.is_ascii_digit() && self.typed_days.len() < MAX_DAYS_DIGITS {
            self.typed_days.push(c);
        }
    }

    /// Deletes the last digit.
    pub fn days_pop(&mut self) {
        self.typed_days.pop();
    }

    /// The number of days typed so far, where one has been.
    pub fn days_value(&self) -> Option<u32> {
        self.typed_days.parse().ok()
    }

    /// Why that number cannot be the wait, where it cannot.
    pub fn days_refusal(&self) -> Option<DaysRefusal> {
        let days = self.days_value()?;
        if days > recovery::MAX_DAYS {
            return Some(DaysRefusal::TooMany);
        }
        (days <= self.previous_days()).then_some(DaysRefusal::TooShort)
    }

    /// Whether ✓ on that pad can take what is typed.
    pub fn days_ready(&self) -> bool {
        self.days_value().is_some() && self.days_refusal().is_none()
    }

    /// Takes the typed number as this path's wait.
    pub fn take_typed_days(&mut self) {
        if !self.days_ready() {
            return;
        }
        let Some(days) = self.days_value() else {
            return;
        };
        let path = self.later_path();
        if let Some(p) = self.later.get_mut(path) {
            p.days = days;
            p.typed = true;
        }
    }

    /// Whether "Another path later?" has Yes checked.
    pub fn another(&self) -> bool {
        self.another
    }

    /// Checks one of its two rows.
    pub fn set_another(&mut self, another: bool) {
        self.another = another;
    }

    /// Whether a further recovery path can follow this one: the wizard
    /// gathers [`MAX_WIZARD_PATHS`] of them, and each wait must be
    /// longer than the last, so a path that already waits the longest a
    /// timelock can state is the last.
    pub fn can_add_path(&self) -> bool {
        self.later.len() < MAX_WIZARD_PATHS && self.days() < recovery::MAX_DAYS
    }

    /// Begins the next recovery path, which the Later step then gathers
    /// the keys of.
    pub fn add_path(&mut self) {
        if !self.can_add_path() {
            return;
        }
        let previous = self.days();
        self.later.push(LaterPath::new(previous));
        self.gather_later = Some(self.later.len() - 1);
    }

    /// Which form a recovery wallet pays to.
    pub fn form(&self) -> Form {
        match self.script() {
            ScriptType::Taproot => Form::Taproot,
            _ => Form::SegWit,
        }
    }

    /// Signatures a multisig wallet requires, never more than the keys
    /// it has.
    pub fn threshold(&self) -> usize {
        self.threshold.clamp(1, self.keys.len().max(1))
    }

    /// Checks a threshold.
    pub fn set_threshold(&mut self, threshold: usize) {
        self.threshold = threshold;
    }

    /// Opens the entry the scanner's "Type" row leads to, with an empty
    /// field.
    pub fn type_key(&mut self) {
        self.typed.clear();
        self.type_refusal = None;
        self.step = Step::Type;
    }

    /// Why what was typed cannot join, where ✓ has said so.
    pub fn type_refusal(&self) -> Option<Refusal> {
        self.type_refusal
    }

    /// Records that.
    pub fn set_type_refusal(&mut self, refusal: Refusal) {
        self.type_refusal = Some(refusal);
    }

    /// What has been typed on that entry.
    pub fn typed(&self) -> &str {
        &self.typed
    }

    /// Types one character of it. An extended key and the origin before
    /// it are ASCII, so nothing else is taken.
    pub fn type_push(&mut self, c: char) {
        if c.is_ascii_graphic() && self.typed.len() < 256 {
            self.typed.push(c);
            self.type_refusal = None;
        }
    }

    /// Deletes the last character.
    pub fn type_pop(&mut self) {
        self.typed.pop();
        self.type_refusal = None;
    }

    /// How many keys this kind takes before Continue is live.
    pub fn needs(&self) -> usize {
        match self.kind() {
            WalletKind::Single | WalletKind::Silent => 1,
            WalletKind::Frost => self.t(),
            WalletKind::Recovery => 1,
            _ => 2,
        }
    }

    /// Whether the wallet has the keys it needs.
    pub fn ready(&self) -> bool {
        match self.kind() {
            WalletKind::Single | WalletKind::Silent => self.keys.len() == 1,
            WalletKind::Frost => self.keys.len() == self.t(),
            // Either path of a recovery wallet is one key or more, and
            // the step on screen is the one Continue answers for.
            WalletKind::Recovery => !self.keys().is_empty(),
            _ => self.keys.len() >= 2,
        }
    }

    /// Whether another key can join: a multisig wallet holds
    /// [`MAX_KEYS`], which is what its script takes; a single-sig wallet
    /// one; a FROST group exactly the number that must sign.
    pub fn can_add(&self) -> bool {
        match self.kind() {
            WalletKind::Single | WalletKind::Silent => self.keys.is_empty(),
            WalletKind::MuSig2 => true,
            WalletKind::Frost => self.keys.len() < self.t(),
            WalletKind::Recovery => self.keys().len() < recovery::MAX_KEYS,
            WalletKind::Multisig | WalletKind::TaprootMultisig => self.keys.len() < MAX_KEYS,
        }
    }

    /// Whether `key` is already a key of this wallet, which is what puts
    /// the check on its row.
    ///
    /// A recovery wallet's paths are checked together: a key on two of
    /// them would make the review of who can spend a lie, and
    /// `osk_bip::recovery` refuses it.
    pub fn holds(&self, key: &PolicyKey) -> bool {
        self.keys
            .iter()
            .chain(self.later.iter().flat_map(|p| p.keys.iter()))
            .any(|k| k.key.xpub() == key.xpub())
    }

    /// Whether the loaded key at `i` is chosen on the step now drawn.
    pub fn holds_loaded(&self, i: usize) -> bool {
        self.keys().iter().any(|k| k.loaded == Some(i))
    }

    /// Whether the loaded key at `i` is chosen on another of the
    /// wallet's paths, which is what dims its row.
    pub fn held_elsewhere(&self, i: usize) -> bool {
        if self.kind() != WalletKind::Recovery {
            return false;
        }
        let here = self.gather_later.filter(|_| self.on_later());
        let held = |keys: &[BuiltKey]| keys.iter().any(|k| k.loaded == Some(i));
        (here.is_some() && held(&self.keys))
            || self
                .later
                .iter()
                .enumerate()
                .any(|(j, p)| here != Some(j) && held(&p.keys))
    }

    /// Adds `key`. `loaded` is which loaded key it is, where it is one
    /// of this device's, and `source` the text a scanned one was read
    /// from.
    pub fn add(
        &mut self,
        key: PolicyKey,
        loaded: Option<usize>,
        source: Option<String>,
    ) -> Result<(), Refusal> {
        if self.holds(&key) {
            return Err(Refusal::Duplicate);
        }
        if !self.can_add() {
            return Err(Refusal::Duplicate);
        }
        self.gathered_mut().push(BuiltKey {
            key,
            loaded,
            source,
        });
        Ok(())
    }

    /// Removes the key at `index`.
    pub fn remove(&mut self, index: usize) {
        let list = self.gathered_mut();
        if index < list.len() {
            list.remove(index);
        }
    }

    /// Takes every key off the step now drawn, which is what a tap that
    /// moves a one-key wallet's check does before it adds.
    pub fn clear_keys(&mut self) {
        self.gathered_mut().clear();
    }

    /// Takes the loaded key at `i` off the wallet, which is what a
    /// second tap on its row does.
    pub fn remove_loaded(&mut self, i: usize) {
        self.gathered_mut().retain(|k| k.loaded != Some(i));
    }

    /// Replaces the gathered keys, which is what happens when the script
    /// type changes and every key has to be read at another account.
    pub fn set_keys(&mut self, keys: Vec<BuiltKey>) {
        self.keys = keys;
    }

    /// The same for the recovery path at `i`, whose keys are read again
    /// when the form changes.
    pub fn set_later_keys(&mut self, i: usize, keys: Vec<BuiltKey>) {
        if let Some(p) = self.later.get_mut(i) {
            p.keys = keys;
        }
    }

    /// A key rebuilt at another account, keeping where it came from.
    pub fn rebuilt(old: &BuiltKey, key: PolicyKey) -> BuiltKey {
        BuiltKey {
            key,
            loaded: old.loaded,
            source: old.source.clone(),
        }
    }

    /// The wallet the steps so far describe, once the keys are in.
    /// `None` for a FROST group, whose descriptor exists only once the
    /// deal has run.
    ///
    /// A MuSig2 wallet writes its participants in BIP-327's key order,
    /// so the same keys always make the same wallet whichever order
    /// they were gathered in. A multisig wallet keeps the order they
    /// were gathered in: `sortedmulti` sorts the script itself, and the
    /// key vector is what a person compares against a coordinator's.
    pub fn policy(&self) -> Option<WalletPolicy> {
        if self.kind() == WalletKind::Recovery {
            return self.recovery_policy()?.to_wallet_policy(self.form()).ok();
        }
        // A FROST group has no descriptor until it is dealt, and a
        // silent payments wallet has none at all: both are built where
        // the review is accepted.
        if !self.ready() || matches!(self.kind(), WalletKind::Frost | WalletKind::Silent) {
            return None;
        }
        let keys = self.key_texts();
        let refs: Vec<&str> = keys.iter().map(String::as_str).collect();
        let template = self.template();
        WalletPolicy::from_parts(&template, &refs).ok()
    }

    /// The recovery wallet the primary path and the later paths
    /// describe, once every path has a key. `None` for every other kind.
    pub fn recovery_policy(&self) -> Option<RecoveryPolicy> {
        if self.kind() != WalletKind::Recovery || self.keys.is_empty() || self.later.is_empty() {
            return None;
        }
        if self.later.iter().any(|p| p.keys.is_empty()) {
            return None;
        }
        Some(RecoveryPolicy {
            primary: Path {
                threshold: self.threshold(),
                keys: self.keys.iter().map(|k| k.key.key_text()).collect(),
            },
            recovery: self
                .later
                .iter()
                .map(|p| Recovery {
                    delay: recovery::blocks_for_days(p.days).unwrap_or(recovery::MAX_DELAY),
                    path: Path {
                        threshold: p.threshold.clamp(1, p.keys.len().max(1)),
                        keys: p.keys.iter().map(|k| k.key.key_text()).collect(),
                    },
                })
                .collect(),
        })
    }

    /// The key vector, in the order the policy writes it.
    fn key_texts(&self) -> Vec<String> {
        if self.kind() != WalletKind::MuSig2 {
            return self.keys.iter().map(|k| k.key.key_text()).collect();
        }
        let pubkeys: Vec<osk_bip::bitcoin::secp256k1::PublicKey> =
            self.keys.iter().map(|k| k.key.xpub().public_key).collect();
        musig::sort_keys(&pubkeys)
            .into_iter()
            .filter_map(|p| {
                self.keys
                    .iter()
                    .find(|k| k.key.xpub().public_key == p)
                    .map(|k| k.key.key_text())
            })
            .collect()
    }

    /// The descriptor template, with `@i` in place of every key.
    fn template(&self) -> String {
        let n = self.keys.len();
        match self.kind() {
            WalletKind::MuSig2 => {
                let keys: Vec<String> = (0..n).map(|i| alloc::format!("@{i}")).collect();
                alloc::format!("tr(musig({})/**)", keys.join(","))
            }
            WalletKind::Single => {
                let inner = match self.script() {
                    ScriptType::Legacy => "pkh(@0/**)",
                    ScriptType::NestedSegwit => "sh(wpkh(@0/**))",
                    ScriptType::NativeSegwit => "wpkh(@0/**)",
                    ScriptType::Taproot => "tr(@0/**)",
                };
                String::from(inner)
            }
            // BIP 387's sorted tapscript multisig under BIP 341's `H`:
            // the internal key is not a key of the wallet, so it stays
            // in the template's text.
            WalletKind::TaprootMultisig => {
                let keys: Vec<String> = (0..n).map(|i| alloc::format!("@{i}/**")).collect();
                let nums = nums_hex();
                alloc::format!(
                    "tr({nums},sortedmulti_a({},{}))",
                    self.threshold(),
                    keys.join(",")
                )
            }
            _ => {
                let keys: Vec<String> = (0..n).map(|i| alloc::format!("@{i}/**")).collect();
                let multi = alloc::format!("sortedmulti({},{})", self.threshold(), keys.join(","));
                match self.script() {
                    ScriptType::NestedSegwit => alloc::format!("sh(wsh({multi}))"),
                    _ => alloc::format!("wsh({multi})"),
                }
            }
        }
    }

    // ----- the deal -----

    /// The dealer, once a FROST review has been accepted.
    pub fn dealer(&self) -> Option<&Dealer> {
        self.dealer.as_ref()
    }

    /// That dealer, to act on.
    pub fn dealer_mut(&mut self) -> Option<&mut Dealer> {
        self.dealer.as_mut()
    }

    /// Keeps the deal `dealer` and puts the first computed key's words on
    /// screen.
    pub fn set_dealer(&mut self, dealer: Dealer) {
        self.dealer = Some(dealer);
        self.step = Step::Words;
    }

    /// One step back. `false` leaves the wizard. Back walks the steps
    /// and keeps what each gathered; the discard confirm is a step of
    /// its own, asked once, on the way out of the first step with keys
    /// in hand.
    pub fn back(&mut self) -> bool {
        self.step = match self.step {
            // §16.129 rule 5: from a key's page, the first step's Back
            // is the page.
            Step::Passphrase => return false,
            Step::Kind if self.gathered_beyond_hand() => {
                self.resume = Step::Kind;
                Step::Discard
            }
            Step::Kind => return false,
            Step::Count => Step::Kind,
            Step::Quorum => Step::Count,
            Step::Keys => match self.kind() {
                WalletKind::Frost => Step::Quorum,
                _ => Step::Kind,
            },
            // The chevron leaves the path, which for every path but the
            // first is the question that offered it.
            Step::Later => match self.later_path() {
                0 => {
                    self.gather_later = None;
                    self.before_later()
                }
                i => {
                    self.later.pop();
                    self.gather_later = Some(i - 1);
                    Step::Another
                }
            },
            Step::LaterThreshold => Step::Later,
            Step::Delay => self.before_delay(),
            Step::Days => Step::Delay,
            Step::Another => Step::Delay,
            Step::Type if self.gather_later.is_some() => Step::Later,
            Step::Type => Step::Keys,
            Step::Script if self.kind() == WalletKind::Recovery => self.before_script(),
            Step::Script => self.before_after_keys(),
            Step::Threshold if self.kind() == WalletKind::Recovery => Step::Keys,
            Step::Threshold => Step::Script,
            Step::Review => match self.kind() {
                WalletKind::Single => Step::Script,
                WalletKind::Multisig => Step::Threshold,
                WalletKind::Recovery => Step::Script,
                _ => self.before_after_keys(),
            },
            // Once the group is dealt there is no going back to the
            // choices that made it: the words on screen are the only
            // copy, so the chevron walks the keys and then stops.
            Step::Words => {
                let moved = self
                    .dealer
                    .as_mut()
                    .is_some_and(crate::threshold::Dealer::previous_member);
                if !moved {
                    return false;
                }
                Step::QuizStart
            }
            Step::QuizStart => Step::Words,
            Step::Quiz | Step::QuizSkip => Step::QuizStart,
            Step::Record => {
                if let Some(d) = self.dealer.as_mut() {
                    d.last_member();
                }
                Step::QuizStart
            }
            Step::Discard => self.resume,
        };
        true
    }

    /// "Keep" on the discard confirm, which is also what the system
    /// back does there: back to the step it covered.
    pub fn keep(&mut self) {
        self.step = self.resume;
    }

    /// Whether any key has been gathered, on the wallet's keys or on one
    /// of a recovery wallet's later paths.
    pub fn gathered_any(&self) -> bool {
        !self.keys.is_empty() || self.later.iter().any(|p| !p.keys.is_empty())
    }

    /// Whether a key other than the one in hand has been gathered, which
    /// is what the discard question guards (§16.129 rule 5). With no key
    /// in hand this is [`WalletWizard::gathered_any`].
    pub fn gathered_beyond_hand(&self) -> bool {
        let hand = self.in_hand;
        self.keys.iter().any(|k| hand.is_none() || k.loaded != hand)
            || self.later.iter().any(|p| !p.keys.is_empty())
    }

    /// The step before the one the Keys step leads to: the Keys step,
    /// or the kind step where it went past the Keys step with the key
    /// in hand.
    fn before_after_keys(&self) -> Step {
        if self.keys_skipped {
            Step::Kind
        } else {
            Step::Keys
        }
    }

    /// The step after the keys, which is what Continue on the Keys step
    /// goes to.
    pub fn after_keys(&self) -> Step {
        match self.kind() {
            WalletKind::Single | WalletKind::Multisig => Step::Script,
            WalletKind::TaprootMultisig => Step::Threshold,
            // A path of one key needs no threshold: one key signs.
            WalletKind::Recovery if self.keys.len() > 1 => Step::Threshold,
            WalletKind::Recovery => Step::Later,
            _ => Step::Review,
        }
    }

    /// The step after a recovery path's keys.
    pub fn after_later(&self) -> Step {
        if self.later_keys().len() > 1 {
            Step::LaterThreshold
        } else {
            Step::Delay
        }
    }

    /// The step after that path's wait: the question that offers another
    /// path, where another can follow, and otherwise the form.
    pub fn after_delay(&self) -> Step {
        if self.can_add_path() {
            Step::Another
        } else {
            Step::Script
        }
    }

    /// The step after that question.
    pub fn after_another(&self) -> Step {
        if self.another {
            Step::Later
        } else {
            Step::Script
        }
    }

    /// The step before a recovery path's keys, which the chevron takes.
    fn before_later(&self) -> Step {
        if self.keys.len() > 1 {
            Step::Threshold
        } else {
            Step::Keys
        }
    }

    /// The step before the wait.
    fn before_delay(&self) -> Step {
        if self.later_keys().len() > 1 {
            Step::LaterThreshold
        } else {
            Step::Later
        }
    }

    /// The step before a recovery wallet's form.
    fn before_script(&self) -> Step {
        if self.can_add_path() {
            Step::Another
        } else {
            Step::Delay
        }
    }

    /// The step after the script type.
    pub fn after_script(&self) -> Step {
        match self.kind() {
            WalletKind::Multisig => Step::Threshold,
            _ => Step::Review,
        }
    }

    /// The step after a threshold Choice.
    pub fn after_threshold(&self) -> Step {
        match self.kind() {
            WalletKind::Recovery => Step::Later,
            _ => Step::Review,
        }
    }

    /// The step the kind's counts lead to, which is where "Which kind?"
    /// continues.
    pub fn after_kind(&self) -> Step {
        match self.kind() {
            WalletKind::Frost => Step::Count,
            _ => Step::Keys,
        }
    }
}

/// BIP 341's `H` written out, which is what a taproot multisig's
/// template carries as its internal key.
fn nums_hex() -> String {
    let mut out = String::new();
    for byte in osk_bip::tapmulti::NUMS {
        out.push(char::from_digit(u32::from(byte >> 4), 16).expect("nibble"));
        out.push(char::from_digit(u32::from(byte & 0x0f), 16).expect("nibble"));
    }
    out
}

/// The most keys a FROST group made here has.
pub const MAX_GROUP: usize = MAX_SHARES;

/// A wallet's network, as the record's version bytes state it.
pub fn network_kind(network: Network) -> NetworkKind {
    match network {
        Network::Mainnet => NetworkKind::Main,
        _ => NetworkKind::Test,
    }
}

/// The key `text` states, for a wallet whose cosigners contribute
/// `kind`'s account, on `network`.
///
/// What is accepted is a key with an origin in descriptor notation, with
/// or without a trailing path; a single-key descriptor, whose key it
/// takes; and a Coldcard export, whose account for this script type it
/// takes. A bare extended public key states no origin, and BIP-388
/// writes an origin on every key of a wallet of more than one, so
/// nothing is invented for it.
pub fn read_key(text: &str, kind: Kind, network: Network) -> Result<PolicyKey, Refusal> {
    let text = text.trim();
    if text.is_empty() {
        return Err(Refusal::NotAKey);
    }
    if coldcard::looks_like_export(text) {
        let key = coldcard::cosigner_key(text, kind.cosigner()).map_err(|_| Refusal::NotAKey)?;
        return accept(&key, network);
    }
    if is_wallet(text) {
        return Err(Refusal::Wallet);
    }
    if let Ok(policy) = WalletPolicy::from_descriptor(text) {
        let key = policy.keys().first().ok_or(Refusal::NotAKey)?;
        return checked(key.clone(), network);
    }
    if let Some(key) = origin_key(text) {
        return accept(&key, network);
    }
    if xkey::decode_xpub(text).is_ok() || slip132::decode_xpub(text).is_ok() {
        return Err(Refusal::NoOrigin);
    }
    Err(Refusal::NotAKey)
}

/// The same, with the `h` a single-key descriptor writes a hardened
/// level in, which is the form `AccountXpub::descriptor` uses and the
/// one a wallet over one key keeps.
pub fn account_key_h(fingerprint: Fingerprint, path: &DerivationPath, xpub: &Xpub) -> String {
    let mut out = alloc::format!("[{fingerprint}");
    for child in path {
        out.push_str(&alloc::format!("/{child:#}"));
    }
    out.push_str(&alloc::format!("]{xpub}"));
    out
}

/// `[fingerprint/path]xpub` for an account of a key this device holds,
/// with the apostrophes BIP-388 writes a hardened level in.
pub fn account_key(fingerprint: Fingerprint, path: &DerivationPath, xpub: &Xpub) -> String {
    let mut out = alloc::format!("[{fingerprint}");
    for child in path {
        out.push_str(&alloc::format!("/{child}"));
    }
    out.push_str(&alloc::format!("]{xpub}"));
    out
}

/// Whether `text` describes a whole wallet rather than one key: a
/// BIP-388 policy, a multisig descriptor, a `musig()` expression or a
/// coordinator's config file.
fn is_wallet(text: &str) -> bool {
    text.contains('@')
        || text.contains("multi(")
        || text.contains("musig(")
        || multisig_config::looks_like_config(text)
}

/// `[fingerprint/path]xpub`, with whatever derivation follows the key
/// dropped: what the wallet takes is the account key and its origin,
/// and the chains are the template's business.
fn origin_key(text: &str) -> Option<String> {
    let (origin, rest) = text.strip_prefix('[')?.split_once(']')?;
    let key = rest.split('/').next()?;
    Some(alloc::format!("[{origin}]{key}"))
}

/// The key `text` is, once it parses and passes [`checked`].
fn accept(text: &str, network: Network) -> Result<PolicyKey, Refusal> {
    // The template is only the shape a lone key is read in; what comes
    // back is the key, and the wallet's own template is built later.
    let policy = WalletPolicy::from_parts("wpkh(@0/**)", &[text]).map_err(|_| Refusal::NotAKey)?;
    let key = policy.keys().first().ok_or(Refusal::NotAKey)?.clone();
    checked(key, network)
}

/// The key, if it states an origin and belongs to this device's chain.
fn checked(key: PolicyKey, network: Network) -> Result<PolicyKey, Refusal> {
    if key.fingerprint().is_none() {
        return Err(Refusal::NoOrigin);
    }
    let mainnet = key.xpub().network == NetworkKind::Main;
    if mainnet != network.is_mainnet() {
        return Err(if mainnet {
            Refusal::Mainnet
        } else {
            Refusal::Testnet
        });
    }
    Ok(key)
}
