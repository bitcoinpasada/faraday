//! Check a multisig setup (`docs/FLOWS.md`, Check a multisig setup): a
//! descriptor, a BIP-388 policy, a setup file, a BIP 129 record or a
//! loaded wallet, read by Files' reader and shown as the rows of the
//! wallet confirmation sheet, its quorum as the wallet's quorum is worked
//! out, and which cosigner a key loaded here is. Nothing is added to the
//! session. Everything shown is public and kept until the screen is
//! left. The marks on the cosigners are worked out once, from the keys
//! loaded when the setup is taken, and cannot go out of step: the page has
//! no way to load or drop a key, a Lock ends the process, and leaving the
//! page ends its state (`tidy`), so a way back in starts empty. The
//! drawing is `screen.rs`.

use crate::strings::EN;
use crate::wallet::{Origin, Rows, Session, Wallet};
use crate::{Action, Faraday, Screen, flow};
use faraday_bip::policy::WalletPolicy;
use faraday_bip::spend::Lock;
use faraday_crypto::work::Compute;
use faraday_text::Text;

pub(crate) mod screen;

/// The cards, in order.
pub mod mstep {
    /// Where the setup comes from.
    pub const SETUP: u8 = 0;
    /// What it is.
    pub const RESULT: u8 = 1;
    /// How many cards.
    pub const COUNT: usize = 2;
}

/// A press in the check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MsAction {
    /// The screen opened with the Setup card open.
    Open,
    /// A card's header.
    Step(u8),
    /// The scanner.
    Scan,
    /// Paste, on Faraday Demo: the clipboard's text is the setup.
    Paste,
    /// This file of the Inbox, read as the setup.
    Inbox(usize),
    /// This wallet of the session, read as the setup.
    Loaded(usize),
    /// A file's ⋮ menu, **Check this setup**: the screen on that file
    /// (`true`: a file of the Outbox) with the Setup card closed.
    FromFile(bool, usize),
    /// The Result's **Another setup**.
    Another,
    /// The Result's **Check a seed against it**: the Seed and Passphrase
    /// steps of My seed makes my wallet open under the rows.
    CheckSeed,
    /// The Result's **Add this wallet**: the wallet read goes through the
    /// confirmation sheet like any wallet of several keys (F254), then
    /// the page says it is loaded here.
    Add,
}

/// What the seed given to Check a seed against it is to the setup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SeedAnswer {
    /// The seed is these cosigners' key, by their places, and the warning
    /// line where another cosigner's fingerprint is the seed's with a key
    /// that is not.
    Key(Vec<usize>, Option<Text>),
    /// Not a key of the wallet; `true` where the same words with no
    /// passphrase make a cosigner.
    Not(bool),
    /// A cosigner's fingerprint is the seed's and its key is not: the
    /// warning line.
    Changed(Text),
}

/// The Result card's rows, worked out once.
pub(crate) struct Shown {
    /// The wallet read, for the seed given against it.
    pub(crate) policy: WalletPolicy,
    /// The confirmation sheet's rows (F254), as it shows them.
    pub(crate) rows: Rows,
    /// "{m} of {n} seeds", where the shape does not state a quorum.
    pub(crate) quorum: Option<Text>,
    pub(crate) ways: Vec<Text>,
    pub(crate) ways_too_many: bool,
    pub(crate) first: Option<Text>,
    /// The name this wallet is loaded under here, if it is.
    pub(crate) loaded: Option<Text>,
    /// The place of that wallet among the loaded ones.
    pub(crate) place: Option<usize>,
}

/// The check's state.
pub struct MsState {
    pub(crate) open: Option<u8>,
    pub(crate) scroll: flow::Scroll,
    pub(crate) back: Screen,
    /// The setup's name: "Scanned", a file's or a wallet's.
    pub(crate) source: Option<Text>,
    /// Why the last setup was refused.
    pub(crate) refused: Option<Text>,
    pub(crate) shown: Option<Shown>,
    /// What the seed given to Check a seed against it is; the seed itself
    /// is the one `faraday-keys` holds for My seed makes my wallet's flow.
    pub(crate) seed: Option<SeedAnswer>,
    /// Which sample setup Use a sample gives next.
    pub(crate) sample: u8,
}

impl MsState {
    fn new(back: Screen) -> MsState {
        MsState {
            open: Some(mstep::SETUP),
            scroll: flow::Scroll::default(),
            back,
            source: None,
            refused: None,
            shown: None,
            seed: None,
            sample: 0,
        }
    }

    pub(crate) fn done(&self, _step: u8) -> bool {
        self.shown.is_some()
    }
}

/// The check's place in the app.
#[derive(Default)]
pub struct MsArea {
    state: Option<MsState>,
}

impl MsArea {
    /// The state, while the screen is open.
    pub fn state(&self) -> Option<&MsState> {
        self.state.as_ref()
    }

    pub(crate) fn set_scroll(&mut self, to: flow::Scroll) {
        if let Some(s) = self.state.as_mut() {
            s.scroll = to;
        }
    }

    pub(crate) fn scroll_y(&self) -> Option<f32> {
        self.state.as_ref().map(|x| x.scroll.y)
    }

    pub(crate) fn set_scroll_y(&mut self, y: f32) {
        if let Some(x) = self.state.as_mut() {
            x.scroll.y = y;
        }
    }

    pub(crate) fn clear(&mut self) {
        self.state = None;
    }
}

/// The state, while the screen is open.
pub fn state(app: &Faraday) -> Option<&MsState> {
    app.tools.mscheck.state()
}

fn st(app: &mut Faraday) -> Option<&mut MsState> {
    app.tools.mscheck.state.as_mut()
}

/// The screen, opened from where the person is (Check's row, Tools' tile).
pub(crate) fn open(app: &mut Faraday) {
    let back = app.screen;
    app.tools.mscheck.state = Some(MsState::new(back));
    app.screen = Screen::MultisigCheck;
}

/// After a press: the screen's state ends unless it is on show.
pub(crate) fn tidy(app: &mut Faraday) {
    if app.screen != Screen::MultisigCheck {
        app.tools.mscheck.clear();
        return;
    }
    // The seed went (a lock): its answer goes with it.
    if app.session.keys.check_facts().is_none()
        && let Some(s) = st(app)
    {
        s.seed = None;
    }
}

/// Costs what [`crate::seed_check::held_in`] costs. The seed the check
/// holds was read: its answer against the setup on show.
pub(crate) fn seed_given(app: &mut Faraday) {
    let c = app.compute();
    let Some(policy) = state(app)
        .and_then(|s| s.shown.as_ref())
        .map(|r| r.policy.clone())
    else {
        return;
    };
    let warning = |k: usize| {
        let key = &policy.keys()[k];
        match (key.fingerprint(), key.path()) {
            (Some(fp), Some(path)) => Some(Text::fill(
                EN.mscheck_wrong_key,
                &[
                    &crate::wallet::fp_text(fp),
                    &crate::register::path_text(path),
                ],
            )),
            _ => None,
        }
    };
    let answer = match crate::seed_check::held_in(&c, &app.session.keys, &policy) {
        Some(crate::seed_check::SeedIs::Key(ours, finger)) => {
            SeedAnswer::Key(ours, finger.and_then(warning))
        }
        Some(crate::seed_check::SeedIs::Fingerprint(k)) => {
            warning(k).map_or(SeedAnswer::Not(false), SeedAnswer::Changed)
        }
        Some(crate::seed_check::SeedIs::Without) => SeedAnswer::Not(true),
        Some(crate::seed_check::SeedIs::Neither) | None => SeedAnswer::Not(false),
    };
    if let Some(s) = st(app) {
        s.seed = Some(answer);
        s.scroll.follow = true;
    }
}

/// Whether a file of Files reads as a wallet of more than one key.
pub(crate) fn is_setup(policy: Option<&WalletPolicy>) -> bool {
    policy.is_some_and(|p| p.keys().len() > 1)
}

/// A file's ⋮ entry, **Check this setup**, on a file read as a wallet of
/// more than one key.
pub(crate) fn menu_line(
    policy: Option<&WalletPolicy>,
    out: bool,
    k: usize,
) -> Option<(String, Action)> {
    is_setup(policy).then(|| {
        (
            EN.files_check_setup.to_string(),
            Action::MultisigCheck(MsAction::FromFile(out, k)),
        )
    })
}

/// Whether the text holds an extended private key.
fn holds_private_key(text: &str) -> bool {
    const HEADS: [&str; 10] = [
        "xprv", "yprv", "zprv", "tprv", "uprv", "vprv", "Yprv", "Zprv", "Uprv", "Vprv",
    ];
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .any(|t| t.len() > 100 && HEADS.iter().any(|h| t.starts_with(h)))
}

/// Costs a read of the text, its checksum, its quorum and a derivation
/// for each key loaded here that a cosigner names, and an address
/// derivation: reads `text` as a wallet and works out the Result, or the
/// line why it is refused.
pub(crate) fn check_text(app: &Faraday, c: &Compute, text: &str) -> Result<Shown, Text> {
    if holds_private_key(text) {
        return Err(Text::of(EN.mscheck_private));
    }
    let trimmed = text.trim();
    let facts = faraday_bip::descriptor::script_type(trimmed)
        .and_then(|_| faraday_bip::descriptor::checksum_facts(c, trimmed));
    let given = facts.as_ref().and_then(|f| f.given.clone());
    if let (Some(f), Some((g, false))) = (&facts, &given) {
        return Err(Text::fill(EN.mscheck_checksum_fails, &[g, &f.checksum]));
    }
    let policy = crate::wallet::read_wallet(c, text).map_err(|e| e.text())?;
    if !is_setup(Some(&policy)) {
        return Err(Text::of(EN.mscheck_refused_one_key));
    }
    Ok(shown(app, c, policy, given.is_some()))
}

/// The Result for a wallet already read (a file of Files, a loaded wallet).
pub(crate) fn check_policy(
    app: &Faraday,
    c: &Compute,
    policy: &WalletPolicy,
) -> Result<Shown, Text> {
    if !is_setup(Some(policy)) {
        return Err(Text::of(EN.mscheck_refused_one_key));
    }
    Ok(shown(app, c, policy.clone(), true))
}

/// Costs what [`check_text`] costs after the read.
fn shown(app: &Faraday, c: &Compute, policy: WalletPolicy, given: bool) -> Shown {
    let session = &app.session;
    let wallet = Wallet::new("", policy.clone(), "", Origin::Named);
    let rows = Rows::of(session, c, &policy, given);
    let stated = Session::stated_quorum(&wallet).is_some();
    let (m, n) = Session::quorum(&wallet);
    let quorum = (!stated).then(|| {
        Text::fill(
            EN.mscheck_quorum_seeds,
            &[&Text::num(m as u64), &Text::num(n as u64)],
        )
    });
    let paths = if stated {
        Vec::new()
    } else {
        policy.spend_paths()
    };
    let ways_too_many = !stated && paths.is_empty();
    let ways = paths.iter().map(way).collect();
    let network = policy
        .keys()
        .first()
        .map(|k| crate::wallet::network_of_kind(k.xpub().network, session.network()))
        .unwrap_or_else(|| session.network());
    let first = policy
        .address_at(c, network, false, 0)
        .ok()
        .map(|a| Text::of(&a.to_string()));
    let same = crate::wallet::same_wallet(&policy);
    let found = session
        .wallets
        .iter()
        .position(|w| crate::wallet::same_wallet(w.policy()) == same);
    let loaded = found.map(|i| Text::of(&session.wallets[i].name));
    Shown {
        policy,
        rows,
        quorum,
        ways,
        ways_too_many,
        first,
        loaded,
        place: found,
    }
}

/// One way to spend, as a line.
fn way(p: &faraday_bip::spend::SpendPath) -> Text {
    let mut keys = String::new();
    for (i, k) in p.keys.iter().enumerate() {
        let name = String::from(&*Text::fill(EN.mscheck_key, &[&Text::num(*k as u64 + 1)]));
        keys = if i == 0 {
            name
        } else {
            String::from(&*Text::fill(
                EN.mscheck_keys_and,
                &[&keys, &name.to_lowercase()],
            ))
        };
    }
    if p.locks.is_empty() {
        return Text::fill(EN.mscheck_way_any_time, &[&keys]);
    }
    let waits: Vec<String> = p
        .locks
        .iter()
        .map(|l| match *l {
            Lock::Blocks(n) => Text::fill(EN.mscheck_wait_blocks, &[&Text::num(u64::from(n))]),
            Lock::Intervals(n) => {
                Text::fill(EN.mscheck_wait_seconds, &[&Text::num(u64::from(n) * 512)])
            }
            Lock::Height(n) => Text::fill(EN.mscheck_wait_height, &[&Text::num(u64::from(n))]),
            Lock::Time(n) => Text::fill(EN.mscheck_wait_time, &[&Text::num(u64::from(n))]),
            Lock::Preimage => Text::of(EN.mscheck_wait_preimage),
        })
        .map(|t| String::from(&*t))
        .collect();
    Text::fill(EN.mscheck_way_wait, &[&keys, &waits.join(" · ")])
}

/// The text of a file of Files.
fn item_text(app: &Faraday, out: bool, k: usize) -> Option<(Text, Text)> {
    let item = if out {
        app.outbox.get(k)
    } else {
        app.inbox.get(k)
    }?;
    Some((Text::of(&item.name), Text::from_utf8_lossy(&item.bytes)))
}

/// Takes the setup: the Result worked out, or the refusal kept.
fn take(app: &mut Faraday, source: Text, result: Result<Shown, Text>) {
    crate::seed_check::drop_for_check(app);
    let Some(s) = st(app) else { return };
    s.seed = None;
    match result {
        Ok(shown) => {
            s.source = Some(source);
            s.refused = None;
            s.shown = Some(shown);
            s.open = Some(mstep::RESULT);
        }
        Err(why) => {
            s.source = None;
            s.shown = None;
            s.refused = Some(why);
            s.open = Some(mstep::SETUP);
        }
    }
    s.scroll.follow = true;
}

pub(crate) fn act(app: &mut Faraday, action: MsAction) {
    match action {
        MsAction::Open => open(app),
        MsAction::Step(k) => {
            let open = if state(app).is_some_and(|s| s.open == Some(k)) {
                None
            } else {
                Some(k)
            };
            // The seed flow's steps are under the Result: a Result no longer
            // open is the flow left, and the seed it held is wiped.
            if open != Some(mstep::RESULT) {
                crate::seed_check::drop_for_check(app);
                if let Some(s) = st(app) {
                    s.seed = None;
                }
            }
            if let Some(s) = st(app) {
                s.open = open;
                s.scroll.follow = true;
            }
        }
        MsAction::Scan => {
            let mut scan = crate::qr_view::ScanState::default();
            scan.purpose = crate::ScanPurpose::MultisigSetup;
            app.camera.open(scan);
            app.sheet = Some(crate::Sheet::Scan);
            app.out
                .commands
                .push_back(faraday_shell_api::Command::CameraOn);
        }
        MsAction::Paste => {
            if paste_shown(app) {
                app.out
                    .commands
                    .push_back(faraday_shell_api::Command::RequestClipboard {
                        kind: faraday_shell_api::FileKind::Text,
                    });
            }
        }
        MsAction::Inbox(k) => from_file(app, false, k),
        MsAction::FromFile(out, k) => {
            if app.screen != Screen::MultisigCheck {
                app.sheet = None;
                open(app);
            }
            from_file(app, out, k);
        }
        MsAction::Loaded(i) => {
            let c = app.compute();
            let Some(w) = app.session.wallets.get(i) else {
                return;
            };
            let (name, policy) = (Text::of(&w.name), w.policy().clone());
            let result = check_policy(app, &c, &policy);
            take(app, name, result);
        }
        MsAction::Another => {
            crate::seed_check::drop_for_check(app);
            if let Some(s) = st(app) {
                s.open = Some(mstep::SETUP);
                s.seed = None;
                s.scroll.follow = true;
            }
        }
        MsAction::Add => add(app),
        MsAction::CheckSeed => {
            crate::seed_check::open_for_check(app);
            if let Some(s) = st(app) {
                s.seed = None;
                s.scroll.follow = true;
            }
        }
    }
}

/// Costs what `Session::add_policy` costs. **Add this wallet**: the
/// wallet read is added through the gate, so a wallet of several keys
/// waits for its sheet, and the press runs again once the hold confirms
/// it; a wallet already loaded, or all of whose keys are loaded keys,
/// reads as loaded at once.
fn add(app: &mut Faraday) {
    let Some((policy, source)) = state(app).and_then(|s| {
        let shown = s.shown.as_ref()?;
        Some((shown.policy.clone(), s.source.clone()?))
    }) else {
        return;
    };
    let c = app.compute();
    let name = crate::register::name_of_file(&source);
    match app
        .session
        .add_policy(&c, &name, policy, &source, Origin::Named)
    {
        Ok(i) => {
            let name = Text::of(&app.session.wallets[i].name);
            app.wallets.set_wallet(i);
            app.refresh_spend();
            if let Some(shown) = st(app).and_then(|s| s.shown.as_mut()) {
                shown.loaded = Some(name);
                shown.place = Some(i);
            }
        }
        Err(crate::wallet::Refusal::Unconfirmed) => {}
        Err(e) => app.toast(&e.text()),
    }
}

fn from_file(app: &mut Faraday, out: bool, k: usize) {
    let c = app.compute();
    let Some((name, text)) = item_text(app, out, k) else {
        return;
    };
    let result = check_text(app, &c, &text);
    take(app, name, result);
}

/// A code read on the scanner for this screen: taken as the setup, the
/// scanner closed; or the scanner says why it is not.
pub(crate) fn scanned(app: &mut Faraday, bytes: &[u8]) -> bool {
    if app.camera.scan().map(|s| s.purpose) != Some(crate::ScanPurpose::MultisigSetup) {
        return false;
    }
    let c = app.compute();
    let text = Text::from_utf8_lossy(bytes);
    match check_text(app, &c, &text) {
        Ok(shown) => {
            app.scan_close();
            take(app, Text::of(EN.mscheck_scanned), Ok(shown));
        }
        Err(why) => app.scan_say(&why),
    }
    true
}

/// Whether the Setup card hands out Paste: where the shell has a
/// clipboard (Faraday Demo), with no sheet over the card. A lookup.
pub(crate) fn paste_shown(app: &Faraday) -> bool {
    app.clipboard
        && app.sheet.is_none()
        && app.screen == Screen::MultisigCheck
        && state(app).is_some_and(|s| s.open == Some(mstep::SETUP))
}

/// Costs what [`check_text`] costs. The clipboard's text asked for by
/// Paste on the Setup card, where no scanner asked: taken as the setup,
/// or the line why it is not. Returns whether it was for the card.
pub(crate) fn pasted(app: &mut Faraday, text: &str) -> bool {
    if app.camera.scan().is_some() || !paste_shown(app) {
        return false;
    }
    if text.trim().is_empty() {
        return true;
    }
    let c = app.compute();
    let result = check_text(app, &c, text);
    take(app, Text::of(EN.mscheck_pasted), result);
    true
}

/// Costs what [`check_text`] costs. Practice's Use a sample on the Setup
/// card: the next sample setup is taken as if scanned, and the cursor
/// moves on.
pub(crate) fn use_sample(app: &mut Faraday) {
    let Some(set) = app.put_on() else { return };
    let list = crate::samples::put::setup_samples(set);
    let Some(s) = st(app) else { return };
    let at = usize::from(s.sample) % list.len().max(1);
    s.sample = ((at + 1) % list.len().max(1)) as u8;
    let Some(text) = list.get(at) else { return };
    let c = app.compute();
    let result = check_text(app, &c, text);
    take(app, Text::of(EN.mscheck_scanned), result);
}

/// Whether the Setup card hands out Use a sample: Practice with samples
/// set, the card open. A lookup.
pub(crate) fn sample_shown(app: &Faraday) -> bool {
    app.put_on().is_some() && state(app).is_some_and(|s| s.open == Some(mstep::SETUP))
}

/// The wallets of the session that are of more than one key, by place.
pub(crate) fn loaded(app: &Faraday) -> Vec<usize> {
    app.session
        .wallets
        .iter()
        .enumerate()
        .filter(|(_, w)| is_setup(Some(w.policy())))
        .map(|(i, _)| i)
        .collect()
}

/// The check's steps, plugged into Back and Forward.
pub(crate) struct MsSteps;

/// The one [`MsSteps`].
pub(crate) static MS_STEPS: MsSteps = MsSteps;

impl crate::trail::StepFlow for MsSteps {
    fn steps(&self, app: &Faraday) -> Option<crate::trail::Steps> {
        let s = state(app)?;
        Some(crate::trail::Steps::new(
            (0..mstep::COUNT as u8).collect(),
            s.open,
        ))
    }

    fn open(&self, _: &Faraday, k: u8) -> Action {
        Action::MultisigCheck(MsAction::Step(k))
    }

    fn name(&self, _: &Faraday, k: u8) -> &'static str {
        screen::TITLES[usize::from(k)]
    }
}
