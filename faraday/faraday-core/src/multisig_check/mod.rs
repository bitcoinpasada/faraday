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
use crate::wallet::{Origin, Session, Wallet};
use crate::{Action, Faraday, Screen, flow};
use faraday_bip::bitcoin::bip32::{ChildNumber, DerivationPath};
use faraday_bip::keys::Fingerprint;
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
    /// This file of the Inbox, read as the setup.
    Inbox(usize),
    /// This wallet of the session, read as the setup.
    Loaded(usize),
    /// A file's ⋮ menu, **Check this setup**: the screen on that file
    /// (`true`: a file of the Outbox) with the Setup card closed.
    FromFile(bool, usize),
    /// The Result's **Another setup**.
    Another,
}

/// What a cosigner's row says of the keys loaded here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    /// No key loaded here is it.
    None,
    /// A loaded key's extended key at the path; `true` at a single-key
    /// account's path.
    Yours(bool),
    /// The fingerprint is a loaded key's, the key at the path is not.
    Wrong(Text),
}

/// One cosigner's row.
pub(crate) struct KeyRow {
    pub(crate) number: Text,
    pub(crate) fingerprint: Text,
    pub(crate) path: Text,
    /// The extended public key whole, in groups of four.
    pub(crate) xpub: Text,
    pub(crate) mark: Mark,
}

/// The Result card's rows, worked out once.
pub(crate) struct Shown {
    pub(crate) shape: Text,
    pub(crate) keys: Vec<KeyRow>,
    /// Keys the descriptor uses twice, as lines.
    pub(crate) repeats: Vec<Text>,
    pub(crate) checksum: Text,
    /// The key path line of a script tree, and whether it is a danger.
    pub(crate) key_path: Option<(Text, bool)>,
    /// "{m} of {n} seeds", where the shape does not state a quorum.
    pub(crate) quorum: Option<Text>,
    pub(crate) ways: Vec<Text>,
    pub(crate) ways_too_many: bool,
    pub(crate) first: Option<Text>,
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

fn path_is_single_account(path: &DerivationPath) -> bool {
    let steps: Vec<&ChildNumber> = path.into_iter().collect();
    steps.len() == 3
        && matches!(
            steps[0],
            ChildNumber::Hardened {
                index: 44 | 49 | 84 | 86
            }
        )
}

/// Costs what [`check_text`] costs after the read.
fn shown(app: &Faraday, c: &Compute, policy: WalletPolicy, given: bool) -> Shown {
    let session = &app.session;
    let wallet = Wallet::new("", policy.clone(), "", Origin::Named);
    let shape = Session::shape(&wallet);
    let keys = policy
        .keys()
        .iter()
        .enumerate()
        .map(|(i, k)| {
            let fp = k.fingerprint();
            let path = k.path();
            let mark = match (fp, path) {
                (Some(fp), Some(path)) => mark_of(session, c, k.xpub(), fp, path),
                _ => Mark::None,
            };
            KeyRow {
                number: Text::fill(EN.mscheck_key, &[&Text::num(i as u64 + 1)]),
                fingerprint: fp.map(crate::wallet::fp_text).unwrap_or_default(),
                path: path
                    .map(|p| Text::of(&crate::register::path_text(p)))
                    .unwrap_or_default(),
                xpub: groups_of_four(&k.xpub().to_string()),
                mark,
            }
        })
        .collect();
    let repeats = faraday_bip::descriptor::checksum_facts(c, &policy.to_descriptor())
        .map(|f| f.repeated)
        .unwrap_or_default()
        .iter()
        .map(|(a, b)| {
            Text::fill(
                EN.tools_key_same_as,
                &[&Text::num(*b as u64 + 1), &Text::num(*a as u64 + 1)],
            )
        })
        .collect();
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
    let checksum = if given {
        Text::fill(EN.mscheck_checksum_holds, &[policy.checksum()])
    } else {
        Text::fill(EN.mscheck_checksum_worked_out, &[policy.checksum()])
    };
    let key_path = session.key_path_line(&policy).map(|l| (l.text, l.danger));
    let network = policy
        .keys()
        .first()
        .map(|k| crate::wallet::network_of_kind(k.xpub().network, session.network()))
        .unwrap_or_else(|| session.network());
    let first = policy
        .address_at(c, network, false, 0)
        .ok()
        .map(|a| Text::of(&a.to_string()));
    Shown {
        shape,
        keys,
        repeats,
        checksum,
        key_path,
        quorum,
        ways,
        ways_too_many,
        first,
    }
}

/// Costs a key derivation: whether a key loaded here is this cosigner.
fn mark_of(
    session: &Session,
    c: &Compute,
    cosigner: &faraday_bip::bitcoin::bip32::Xpub,
    fp: Fingerprint,
    path: &DerivationPath,
) -> Mark {
    let Some(i) = session.keys.position(fp) else {
        return Mark::None;
    };
    match session.keys.xpub_at(c, i, path) {
        Some(x) if x.public_key == cosigner.public_key && x.chain_code == cosigner.chain_code => {
            Mark::Yours(path_is_single_account(path))
        }
        _ => Mark::Wrong(Text::fill(
            EN.mscheck_wrong_key,
            &[
                &crate::wallet::fp_text(fp),
                &crate::register::path_text(path),
            ],
        )),
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

/// `text` in groups of four characters.
fn groups_of_four(text: &str) -> Text {
    let chars: Vec<char> = text.chars().collect();
    let groups: Vec<String> = chars.chunks(4).map(|g| g.iter().collect()).collect();
    Text::of(&groups.join(" "))
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
    let Some(s) = st(app) else { return };
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
            if let Some(s) = st(app) {
                s.open = if s.open == Some(k) { None } else { Some(k) };
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
            if let Some(s) = st(app) {
                s.open = Some(mstep::SETUP);
                s.scroll.follow = true;
            }
        }
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
