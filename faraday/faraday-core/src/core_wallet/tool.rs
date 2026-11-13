//! Read a Bitcoin Core wallet in Tools (`docs/decisions/F229.md`): a step
//! flow of two cards, File and Result. The `wallet.dat` is chosen from
//! Files, with the passphrase typed when the wallet is encrypted; Read
//! opens it, checks its records and keys, and keeps what it found as text.
//! The passphrase is cleared as soon as the read ends and never shown
//! again; the private keys are never held here. The drawing is
//! `screen.rs`.

use super::{Fault, OutputType, Passphrase, Refusal, Wallet, is_wallet_file, read_file};
use crate::secret_text::SecretText;
use crate::strings::EN;
use crate::vault::TextBox;
use crate::{Action, Faraday, Screen, flow};
use faraday_bip::keys::Network;
use faraday_crypto::work::Compute;
use faraday_shell_api::Key;
use faraday_text::{Text, text};

/// The cards, in order.
pub mod cstep {
    /// The file and its passphrase.
    pub const FILE: u8 = 0;
    /// What was found.
    pub const RESULT: u8 = 1;
    /// How many cards.
    pub const COUNT: usize = 2;
}

/// A press in the tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreWalletAction {
    /// The Tools tile.
    Open,
    /// A card's header.
    Step(u8),
    /// This file of Files, from its row: the tool opens with it chosen.
    OpenFile(usize),
    /// This file of Files, chosen on the File card.
    File(usize),
    /// The passphrase field takes focus.
    Focus,
    /// The eye on the passphrase.
    Eye,
    /// Read the chosen file.
    Read,
    /// This active pair, by its place in the list, goes to Wallets.
    Take(usize),
}

/// The most failed records the Result lists.
pub(crate) const MAX_LINES: usize = 12;

/// The longest passphrase the field takes, in bytes.
const PASSPHRASE_MAX: usize = 1024;

/// How a read ended, as the Result shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Every record and every key read passed.
    Whole,
    /// A record failed.
    NotWhole,
    /// The passphrase does not open the wallet.
    WrongPassphrase,
}

/// A descriptor of the wallet, as text.
pub(crate) struct DescriptorText {
    /// The descriptor as Core lists it.
    pub text: Text,
    /// Active receive, active change and the range, one line each.
    pub notes: Vec<Text>,
}

/// An active receive descriptor of one output type and the active change
/// descriptor of the same type, as Take sends them.
pub struct TakePair {
    /// The output type, as the Result names it.
    pub label: &'static str,
    /// Receive first, then change when there is one; each as Core lists
    /// it, the checksum cut.
    pub texts: Vec<Text>,
}

/// The Result card's text, made once when the file is read.
pub(crate) struct Shown {
    /// The verdict.
    pub verdict: Verdict,
    /// Label and value, one pair each.
    pub facts: Vec<(Text, Text)>,
    /// Each descriptor.
    pub descriptors: Vec<DescriptorText>,
    /// The `combo()` descriptors Core keeps and Faraday does not take, as
    /// stored, up to [`MAX_LINES`], then a line saying how many more; the
    /// label first. Empty when there are none.
    pub kept: Vec<Text>,
    /// The failed records, up to [`MAX_LINES`], then a line saying how
    /// many more.
    pub lines: Vec<Text>,
    /// Said of a whole wallet that has no active receive descriptor.
    pub no_active: bool,
}

/// The tool's state.
pub struct CoreWalletState {
    pub(crate) open: Option<u8>,
    pub(crate) scroll: flow::Scroll,
    pub(crate) back: Screen,
    /// The name of the file chosen.
    pub(crate) file: Option<Text>,
    pub(crate) passphrase: TextBox,
    pub(crate) typing: bool,
    pub(crate) revealed: bool,
    /// Why the file was refused.
    pub(crate) refused: Option<Text>,
    pub(crate) shown: Option<Shown>,
    /// The active pairs, for Take to Wallets; empty when the file is not
    /// whole.
    pub(crate) take: Vec<TakePair>,
    /// Why Take refuses: the reader's first line, for a file not whole.
    take_refused: Option<Text>,
}

impl CoreWalletState {
    fn new(back: Screen) -> CoreWalletState {
        CoreWalletState {
            open: Some(cstep::FILE),
            scroll: flow::Scroll::default(),
            back,
            file: None,
            passphrase: TextBox::default(),
            typing: false,
            revealed: false,
            refused: None,
            shown: None,
            take: Vec::new(),
            take_refused: None,
        }
    }

    /// Whether the card is filled.
    pub(crate) fn done(&self, step: u8) -> bool {
        match step {
            cstep::FILE => self.file.is_some(),
            _ => self.shown.is_some(),
        }
    }

    /// Whether the tool holds a passphrase typed.
    pub(crate) fn holds_secret(&self) -> bool {
        !self.passphrase.text.is_empty()
    }

    /// Whether the passphrase field has focus.
    pub fn typing(&self) -> bool {
        self.typing
    }
}

/// The tool's place in Tools.
#[derive(Default)]
pub struct CoreWalletArea {
    state: Option<CoreWalletState>,
}

impl CoreWalletArea {
    /// The state, while the tool is open.
    pub fn state(&self) -> Option<&CoreWalletState> {
        self.state.as_ref()
    }

    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        if let Some(s) = self.state.as_mut() {
            s.scroll = to;
        }
    }

    /// How far the column is scrolled, units, while it is open.
    pub(crate) fn scroll_y(&self) -> Option<f32> {
        self.state.as_ref().map(|x| x.scroll.y)
    }

    /// The column scrolled to `y` units, while it is open.
    pub(crate) fn set_scroll_y(&mut self, y: f32) {
        if let Some(x) = self.state.as_mut() {
            x.scroll.y = y;
        }
    }

    pub(crate) fn clear(&mut self) {
        self.state = None;
    }
}

/// The state, while the tool is open.
pub fn state(app: &Faraday) -> Option<&CoreWalletState> {
    app.tools.core_wallet.state()
}

fn st(app: &mut Faraday) -> Option<&mut CoreWalletState> {
    app.tools.core_wallet.state.as_mut()
}

/// One refusal of the reader, as one plain line.
fn fault_text(f: Fault) -> &'static str {
    match f {
        Fault::Layout => EN.core_wallet_fault_layout,
        Fault::UnknownFlags => EN.core_wallet_fault_flags,
        Fault::Descriptor => EN.core_wallet_fault_descriptor,
        Fault::Checksum => EN.core_wallet_fault_checksum,
        Fault::Id => EN.core_wallet_fault_id,
        Fault::Range => EN.core_wallet_fault_range,
        Fault::KeyNetwork => EN.core_wallet_fault_key_network,
        Fault::NotHeld => EN.core_wallet_fault_not_held,
        Fault::Repeated => EN.core_wallet_fault_repeated,
        Fault::KeyMalformed => EN.core_wallet_fault_key_malformed,
        Fault::KeyHash => EN.core_wallet_fault_key_hash,
        Fault::KeyPublic => EN.core_wallet_fault_key_public,
        Fault::NoMasterKey => EN.core_wallet_fault_no_master,
        Fault::ManyMasterKeys => EN.core_wallet_fault_many_master,
        Fault::PlainKeyEncrypted => EN.core_wallet_fault_plain_key,
        Fault::MasterMethod => EN.core_wallet_fault_master_method,
        Fault::MasterRounds => EN.core_wallet_fault_master_rounds,
        Fault::CacheMissing => EN.core_wallet_fault_cache_missing,
        Fault::CacheKey => EN.core_wallet_fault_cache_key,
        Fault::CacheUnchecked => EN.core_wallet_fault_cache_unchecked,
    }
}

/// Bitcoin Core's client version number as `27.0.0`.
fn version_text(v: u32) -> Text {
    text!("{}.{}.{}", v / 10_000, v / 100 % 100, v % 100)
}

/// `name (count)` for each, joined.
fn counted(list: &[(impl AsRef<str>, usize)]) -> Text {
    let mut out = Text::new();
    for (i, (n, k)) in list.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&text!("{} ({k})", n.as_ref()));
    }
    out
}

/// The label of the kept `combo()` descriptors, then each one's text and a
/// line for those past [`MAX_LINES`]; nothing when there are none.
fn kept_lines(w: &Wallet) -> Vec<Text> {
    if w.kept.is_empty() {
        return Vec::new();
    }
    let mut out = vec![Text::fill(
        EN.core_wallet_kept,
        &[&Text::num(w.kept.len() as u64)],
    )];
    out.extend(w.kept.iter().take(MAX_LINES).map(|k| k.text.duplicate()));
    if w.kept.len() > MAX_LINES {
        out.push(Text::fill(
            EN.core_wallet_kept_more,
            &[&Text::num((w.kept.len() - MAX_LINES) as u64)],
        ));
    }
    out
}

/// Each descriptor as text, with its active marks and range.
fn descriptor_texts(w: &Wallet) -> Vec<DescriptorText> {
    w.descriptors
        .iter()
        .map(|d| {
            let mut notes = Vec::new();
            for a in w.active.iter().filter(|a| a.id == d.id) {
                notes.push(Text::of(if a.internal {
                    EN.core_wallet_active_change
                } else {
                    EN.core_wallet_active_receive
                }));
            }
            if let Some((first, last)) = d.range {
                notes.push(Text::fill(
                    EN.core_wallet_range,
                    &[
                        &Text::int(i64::from(first)),
                        &Text::int(i64::from(last)),
                        &Text::int(i64::from(d.next_index)),
                    ],
                ));
            }
            DescriptorText {
                text: d.listed.duplicate(),
                notes,
            }
        })
        .collect()
}

/// The output types in the order the pairs are offered.
const TYPE_ORDER: [OutputType; 4] = [
    OutputType::Legacy,
    OutputType::P2shSegwit,
    OutputType::Bech32,
    OutputType::Bech32m,
];

/// What a receive descriptor is, named from its script: one key under
/// `pkh`, `sh(wpkh)`, `wpkh` or `tr`, any other script (a multisig under
/// `wsh` or `sh`) as a script. The slot Core marks it active in does not
/// say: Core marks a `wsh` descriptor active as Native SegWit.
fn descriptor_label(text: &str) -> &'static str {
    if text.starts_with("pkh(") {
        EN.core_wallet_type_legacy
    } else if text.starts_with("sh(wpkh(") {
        EN.core_wallet_type_p2sh_segwit
    } else if text.starts_with("wpkh(") {
        EN.core_wallet_type_segwit
    } else if text.starts_with("tr(") {
        EN.core_wallet_type_taproot
    } else {
        EN.core_wallet_type_script
    }
}

/// The active pairs of a wallet read: for each output type, in [`TYPE_ORDER`], the active
/// receive descriptor and the active change one of the same type.
pub fn active_pairs(w: &Wallet) -> Vec<TakePair> {
    let text = |internal: bool, t: OutputType| {
        let a = w
            .active
            .iter()
            .find(|a| a.internal == internal && a.output_type == t)?;
        let d = w.descriptor(&a.id)?;
        Some(Text::of(d.listed.split('#').next().unwrap_or(&d.listed)))
    };
    TYPE_ORDER
        .into_iter()
        .filter_map(|t| {
            let receive = text(false, t)?;
            let label = descriptor_label(&receive);
            let mut texts = vec![receive];
            texts.extend(text(true, t));
            Some(TakePair { label, texts })
        })
        .collect()
}

/// The Result's text, worked out once from the wallet read.
fn shown(w: &Wallet) -> (Shown, (Vec<TakePair>, Option<Text>)) {
    let verdict = match (w.passphrase, w.is_whole()) {
        (Passphrase::Wrong, _) => Verdict::WrongPassphrase,
        (_, true) => Verdict::Whole,
        (_, false) => Verdict::NotWhole,
    };
    let mut facts = vec![(
        Text::of(EN.core_wallet_network),
        Text::of(crate::wallet::network_name(w.network)),
    )];
    if let Some(v) = w.version {
        facts.push((Text::of(EN.core_wallet_version), version_text(v)));
    }
    facts.push((
        Text::of(EN.core_wallet_encryption),
        match w.master_keys.first() {
            Some(m) => Text::fill(EN.core_wallet_encrypted, &[&Text::num(u64::from(m.rounds))]),
            None => Text::of(EN.core_wallet_not_encrypted),
        },
    ));
    facts.push((
        Text::of(EN.core_wallet_keys),
        match w.passphrase {
            Passphrase::NotNeeded | Passphrase::Opened => Text::fill(
                EN.core_wallet_keys_checked,
                &[&Text::num(w.keys_passed as u64)],
            ),
            Passphrase::NotGiven => Text::fill(
                EN.core_wallet_keys_not_checked,
                &[&Text::num(w.keys_unchecked as u64)],
            ),
            Passphrase::Wrong => Text::fill(
                EN.core_wallet_keys_not_opened,
                &[&Text::num(w.keys_unchecked as u64)],
            ),
        },
    ));
    facts.push((
        Text::of(EN.core_wallet_labels),
        Text::of(&Text::num(w.labels.len() as u64)),
    ));
    facts.push((
        Text::of(EN.core_wallet_transactions),
        Text::of(&Text::num(w.tx_count as u64)),
    ));
    if !w.unread.is_empty() {
        facts.push((Text::of(EN.core_wallet_not_read), counted(&w.unread)));
    }
    if !w.unknown.is_empty() || w.unknown_more > 0 {
        let mut line = counted(&w.unknown);
        if w.unknown_more > 0 {
            if !line.is_empty() {
                line.push_str(", ");
            }
            line.push_str(&Text::fill(
                EN.core_wallet_unknown_more,
                &[&Text::num(w.unknown_more as u64)],
            ));
        }
        facts.push((Text::of(EN.core_wallet_unknown), line));
    }
    let descriptors = descriptor_texts(w);
    let kept = kept_lines(w);
    let mut lines: Vec<Text> = w
        .failures
        .iter()
        .take(MAX_LINES)
        .map(|f| Text::fill(EN.core_wallet_failed, &[&f.record, fault_text(f.fault)]))
        .collect();
    let failed_more = (w.failures.len() + w.failures_more).saturating_sub(MAX_LINES);
    if failed_more > 0 {
        lines.push(Text::fill(
            EN.core_wallet_failed_more,
            &[&Text::num(failed_more as u64)],
        ));
    }
    // A file that is not whole gives nothing to Wallets. Otherwise each
    // output type with an active receive descriptor is one pair with its
    // active change descriptor; an inactive descriptor is never taken.
    // The descriptors go as Core lists them, with their origins and
    // account keys; the checksum was checked when the file was read, and
    // the wallet reader takes the text without it.
    let take = if w.is_whole() {
        (active_pairs(w), None)
    } else {
        let line = lines.first().map(Text::duplicate).unwrap_or_else(|| {
            Text::of(match verdict {
                Verdict::WrongPassphrase => EN.core_wallet_wrong_passphrase,
                _ => EN.core_wallet_not_whole,
            })
        });
        (Vec::new(), Some(line))
    };
    (
        Shown {
            verdict,
            facts,
            descriptors,
            kept,
            lines,
            no_active: verdict == Verdict::Whole && take.0.is_empty(),
        },
        take,
    )
}

pub(crate) fn act(app: &mut Faraday, action: CoreWalletAction) {
    match action {
        CoreWalletAction::Open => open(app, None),
        CoreWalletAction::OpenFile(i) => {
            let name = app.inbox.get(i).map(|item| Text::of(&item.name));
            open(app, name);
        }
        CoreWalletAction::Step(k) => {
            if let Some(s) = st(app) {
                s.open = if s.open == Some(k) { None } else { Some(k) };
                s.typing = false;
                s.scroll.follow = true;
            }
        }
        CoreWalletAction::File(i) => {
            let Some(name) = app.inbox.get(i).map(|item| Text::of(&item.name)) else {
                return;
            };
            if let Some(s) = st(app) {
                choose(s, name);
            }
        }
        CoreWalletAction::Focus => {
            if crate::stick::refuses_secret(app) {
                return;
            }
            if let Some(s) = st(app) {
                s.typing = true;
            }
        }
        CoreWalletAction::Eye => {
            if let Some(s) = st(app) {
                s.revealed = !s.revealed;
            }
        }
        CoreWalletAction::Read => read(app),
        CoreWalletAction::Take(i) => take(app, i),
    }
}

fn open(app: &mut Faraday, file: Option<Text>) {
    let back = app.screen;
    let mut s = CoreWalletState::new(back);
    if let Some(name) = file {
        choose(&mut s, name);
    }
    app.tools.core_wallet.state = Some(s);
    app.screen = Screen::CoreWallet;
}

/// A file chosen: what was read of another is dropped.
fn choose(s: &mut CoreWalletState, name: Text) {
    s.file = Some(name);
    s.refused = None;
    s.shown = None;
    s.take.clear();
    s.take_refused = None;
    s.typing = false;
}

/// A read of the chosen file, owned, for the worker.
pub(crate) struct ReadWork {
    name: Text,
    bytes: zeroize::Zeroizing<Vec<u8>>,
    beside: Vec<Text>,
    mode: Network,
    passphrase: Option<SecretText>,
}

/// How a read ended.
pub(crate) struct ReadDone {
    result: Result<Wallet, Refusal>,
}

impl ReadWork {
    /// Costs what `read_file` costs: its passphrase stretch.
    pub(crate) fn run(self, c: &Compute) -> ReadDone {
        let result = read_file(
            c,
            &self.name,
            &self.bytes,
            &self.beside,
            self.mode,
            self.passphrase.as_ref(),
        );
        ReadDone { result }
    }
}

/// The chosen file and the passphrase typed, taken for a read. The
/// passphrase leaves the field here: it is cleared whatever comes of the
/// read.
fn begin(app: &mut Faraday) -> Option<ReadWork> {
    let mode = app.session.network();
    let beside: Vec<Text> = app.inbox.iter().map(|i| Text::of(&i.name)).collect();
    let s = app.tools.core_wallet.state.as_mut()?;
    let name = s.file.as_ref()?.duplicate();
    let Some(item) = app.inbox.iter().find(|i| name == i.name.as_str()) else {
        s.refused = Some(Text::of(EN.core_wallet_none_in_files));
        return None;
    };
    let given = !s.passphrase.text.is_empty();
    let passphrase = given.then(|| std::mem::take(&mut s.passphrase.text));
    s.passphrase.clear();
    s.revealed = false;
    s.typing = false;
    s.scroll.follow = true;
    Some(ReadWork {
        name,
        bytes: item.bytes.clone(),
        beside,
        mode,
        passphrase,
    })
}

/// A press on Read: starts the job that reads the chosen file with the
/// passphrase typed, and keeps the Result's text when it comes back.
fn read(app: &mut Faraday) {
    if st(app).is_some_and(|s| s.holds_secret()) && crate::stick::refuses_secret(app) {
        return;
    }
    if let Some(work) = begin(app) {
        app.start(
            crate::worker::Job::CoreWalletRead(Box::new(work)),
            EN.working_core_wallet,
        );
    }
}

/// Costs what `read_file` costs: reads the chosen file now, with no
/// passphrase, for Practice's sample.
fn read_now(app: &mut Faraday, c: &Compute) {
    if let Some(work) = begin(app) {
        let done = work.run(c);
        read_done(app, done);
    }
}

/// A read ended: the Result's text is kept, or the refusal.
pub(crate) fn read_done(app: &mut Faraday, done: ReadDone) {
    let Some(s) = st(app) else { return };
    match done.result {
        Ok(w) => {
            let (text, take) = shown(&w);
            s.shown = Some(text);
            s.take = take.0;
            s.take_refused = take.1;
            s.refused = None;
            s.open = Some(cstep::RESULT);
        }
        Err(e) => {
            s.shown = None;
            s.take.clear();
            s.take_refused = None;
            s.refused = Some(refusal_text(&e));
            s.open = Some(cstep::FILE);
        }
    }
    s.scroll.follow = true;
}

/// A press on Practice's **Use faraday** in the passphrase field.
pub(crate) fn fill_faraday(app: &mut Faraday) {
    if let Some(s) = st(app) {
        s.passphrase.text.set(crate::samples::PASSPHRASE);
    }
}

fn refusal_text(e: &Refusal) -> Text {
    e.message()
}

/// Costs what `add_wallet` costs: the descriptors go to Wallets as a
/// descriptor with its origins does.
fn take(app: &mut Faraday, pair: usize) {
    let Some(s) = app.tools.core_wallet.state() else {
        return;
    };
    if let Some(line) = s.take_refused.as_ref().map(Text::duplicate) {
        app.toast(&line);
        return;
    }
    let Some(chosen) = s.take.get(pair) else {
        return;
    };
    let mut text = Text::new();
    for (i, t) in chosen.texts.iter().enumerate() {
        if i > 0 {
            text.push('\n');
        }
        text.push_str(t);
    }
    let source = s.file.as_ref().map(Text::duplicate).unwrap_or_default();
    let c = app.compute();
    let name = crate::register::name_of_file(&source);
    match app
        .session
        .add_wallet(&c, &name, &text, &source, crate::wallet::Origin::Named)
    {
        Ok(i) => {
            app.wallets.set_wallet(i);
            app.screen = Screen::Wallets;
            app.refresh_spend();
        }
        Err(e) => app.toast(&e.text()),
    }
}

/// A stick is attached: the passphrase field loses focus and, with
/// `wipe`, what was typed.
pub(crate) fn stick_attached(app: &mut Faraday, wipe: bool) {
    if let Some(s) = st(app) {
        s.typing = false;
        if wipe {
            s.passphrase.clear();
        }
    }
}

/// The field typing goes to, with what is typed.
pub(crate) fn field(app: &Faraday) -> Option<(Action, &str)> {
    if app.screen != Screen::CoreWallet {
        return None;
    }
    let s = state(app)?;
    s.typing.then(|| {
        (
            Action::CoreWallet(CoreWalletAction::Focus),
            s.passphrase.text.as_str(),
        )
    })
}

pub(crate) fn key(app: &mut Faraday, key: Key) -> bool {
    if app.screen != Screen::CoreWallet || app.sheet.is_some() {
        return false;
    }
    let Some(s) = st(app) else { return false };
    if !s.typing {
        return false;
    }
    if crate::stick::refuses_secret_key(app, key) {
        return true;
    }
    let Some(s) = st(app) else { return false };
    match key {
        Key::Char(ch) if !ch.is_control() => {
            if s.passphrase.text.len() + ch.len_utf8() <= PASSPHRASE_MAX {
                s.passphrase.text.push(ch);
            }
            true
        }
        Key::Backspace => {
            s.passphrase.text.pop();
            true
        }
        Key::Enter => {
            act(app, CoreWalletAction::Read);
            true
        }
        Key::Escape => {
            s.typing = false;
            true
        }
        _ => false,
    }
}

/// Closes the eye: the idle warning.
pub(crate) fn hide(app: &mut Faraday) {
    if let Some(s) = st(app) {
        s.revealed = false;
    }
}

/// After a press: the tool ends, the passphrase with it, unless it is on
/// show.
pub(crate) fn tidy(app: &mut Faraday) {
    if app.screen != Screen::CoreWallet {
        app.tools.core_wallet.clear();
    }
}

/// The tile: the tool, with Files' wallet file when there is one, else
/// (Practice) the two sample files put in Files, the plain one read.
pub(crate) fn open_tile(app: &mut Faraday) {
    if let Some(i) = app.inbox.iter().position(|i| is_wallet_file(&i.bytes)) {
        act(app, CoreWalletAction::OpenFile(i));
    } else {
        open_sample(app);
    }
}

/// Costs what `read_file` costs: Practice's stand-in for a file to copy
/// in: the plain and the encrypted sample wallet go into Files, and the
/// tool opens with the plain one read.
fn open_sample(app: &mut Faraday) {
    let Some(set) = app.put_on() else { return };
    let inputs = set.inputs;
    if inputs.core_wallet.is_empty() {
        return;
    }
    let c = app.compute();
    let names = [
        EN.sample_core_wallet_plain_file,
        EN.sample_core_wallet_encrypted_file,
    ];
    for (bytes, name) in inputs.core_wallet.iter().zip(names) {
        if !app.inbox.iter().any(|i| i.name == name) {
            app.inbox.push(crate::Item::new(&c, name, bytes.to_vec()));
        }
    }
    act(app, CoreWalletAction::Open);
    if let Some(s) = st(app) {
        choose(s, Text::of(names[0]));
    }
    read_now(app, &c);
}

/// The tool's steps, plugged into Back, Forward and the choices that move on.
pub(crate) struct CoreWalletSteps;

/// The one [`CoreWalletSteps`].
pub(crate) static CORE_WALLET_STEPS: CoreWalletSteps = CoreWalletSteps;

impl crate::trail::StepFlow for CoreWalletSteps {
    fn steps(&self, app: &Faraday) -> Option<crate::trail::Steps> {
        let s = state(app)?;
        Some(crate::trail::Steps::new(
            (0..cstep::COUNT as u8).collect(),
            s.open,
        ))
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        Action::CoreWallet(CoreWalletAction::Step(k))
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        match k {
            cstep::FILE => EN.core_wallet_card_file,
            _ => EN.core_wallet_card_result,
        }
    }
}
