//! A frame drawn again with nothing changed, and a pointer moved over a
//! screen, do no costly work beyond each screen's named budget
//! (`docs/SPEED.md` §3.3 and §7), and a frame makes no more allocations
//! and draws no more lines of text than each screen's measured count and
//! a small margin. The session is the heaviest the app reaches: a 2-of-3
//! whose three keys were made here, its backup on the checklist, the test
//! vault open with every wallet it keeps loaded, a GPG key's backup flow
//! under way, a spend open, and the test stick's files in the Inbox. It
//! runs on a desktop and on the Pi's panel, whose compact screens are
//! drawn by other code.

mod accessors;
mod caches;
mod fades;
mod idle;
mod keys;
mod moments;
mod presses;
mod scaling;

/// The test binary's allocator, counting each thread's allocations for
/// the frame budgets below.
#[global_allocator]
static ALLOC: faraday_alloc_count::Counting = faraday_alloc_count::Counting;

use faraday_crypto::work::Compute;
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};

use super::{Counts, KINDS, Work, since, snapshot};
use crate::backup_flow::steps::FlowAction as F;
use crate::backup_flow::{BackupFlow, Kind, Plan, Step, Written};
use crate::create::NewKind;
use crate::keygen::Way;
use crate::vault::VaultAction as V;
use crate::{Action, Faraday, Screen, Sheet, StorageEvent, bstep, cstep, testkit};

/// What one frame of a screen may work out each time it is drawn, in
/// this session, with the reason: (screen, kind, budget, why). `*` is
/// every screen; a screen's own line wins over it. Every kind not listed
/// has a budget of none. What a frame reads is kept on the state, worked
/// out once from what it describes and again only when that changes: a
/// key's fingerprint, a wallet's checksum and addresses, an account
/// key's text, each Inbox file's wallet, each open vault's wallet
/// records and word counts, each plan's map and check (`docs/SPEED.md`
/// §3.3). The frame before the one counted works out what a change left
/// to work out, so a frame drawn again works out nothing; a line here is
/// a screen that cannot keep what it draws, and says why. The session:
/// the test vault keeps 12 wallets and 1 seed, the Inbox holds the test
/// stick's files.
const BUDGETS: &[(&str, Work, u64, &str)] = &[];

/// A sheet stands on the page under it, frosted once when it opens and
/// kept: each frame puts that copy back (`Ui::backdrop`), one copy of the
/// whole canvas.
const SHEET_CANVAS: u64 = 1;

/// The budget of `kind` on `screen`.
fn budget(screen: &str, kind: Work) -> u64 {
    let of = |s: &str| {
        BUDGETS
            .iter()
            .find(|(name, k, _, _)| *name == s && *k == kind)
            .map(|b| b.2)
    };
    of(screen).or_else(|| of("*")).unwrap_or(0)
}

/// What a frame of each screen of the walk allocates and the lines of
/// text it draws, measured on this code (2026-10-21), on the desktop:
/// (screen, allocations, lines). A frame allowed [`MARGIN_PCT`] more of
/// either passes; more than that is work no one marked, and fails. A
/// change that draws more on purpose measures again and writes the new
/// counts here, with why.
const FRAME_COSTS: &[(&str, u64, u64)] = &[
    ("Create", 381, 63),
    ("Backup board", 4918, 118),
    ("Backup checklist", 2364, 92),
    ("Load sheet", 1046, 26),
    ("GPG key flow", 924, 63),
    ("GPG Sign a file", 4969, 93),
    ("GPG sign sheet", 511, 9),
    ("Spend", 878, 95),
    ("Home", 1256, 48),
    ("Wallets", 2117, 93),
    ("Files", 5434, 82),
    ("Vaults", 521, 43),
    ("Vault contents", 4971, 94),
    ("Backups", 1213, 77),
    ("Spend tab", 976, 68),
    ("Settings", 1118, 69),
    ("Tools", 457, 33),
    ("Power sheet", 2674, 23),
    ("Learn sheet", 71, 28),
    ("Scan sheet", 35, 4),
    ("Wallet chart", 2739, 110),
    ("Backup reopened", 1329, 82),
    ("Create, new", 445, 66),
];

/// [`FRAME_COSTS`] on the Pi's panel, its compact screens.
const FRAME_COSTS_PI: &[(&str, u64, u64)] = &[
    ("Create", 129, 20),
    ("Backup board", 1367, 22),
    ("Backup checklist", 1715, 15),
    ("Load sheet", 1037, 15),
    ("GPG key flow", 384, 20),
    ("GPG Sign a file", 4524, 17),
    ("GPG sign sheet", 540, 9),
    ("Spend", 252, 22),
    ("Home", 1087, 17),
    ("Wallets", 1012, 13),
    ("Files", 5768, 18),
    ("Vaults", 143, 12),
    ("Vault contents", 4524, 17),
    ("Backups", 696, 23),
    ("Spend tab", 630, 18),
    ("Settings", 966, 17),
    ("Tools", 51, 4),
    ("Power sheet", 2701, 8),
    ("Learn sheet", 133, 14),
    ("Scan sheet", 53, 4),
    ("Wallet chart", 1240, 13),
    ("Backup reopened", 660, 21),
    ("Create, new", 172, 17),
];

/// How far over its measured counts a frame may go, in percent of them,
/// and at least [`MARGIN_MIN`]: room for a list one row longer or a
/// label one word longer, not for a screen that does something new.
const MARGIN_PCT: u64 = 10;

/// The least margin, for a screen whose counts are small.
const MARGIN_MIN: u64 = 8;

/// The most `measured` may become.
fn with_margin(measured: u64) -> u64 {
    measured + (measured * MARGIN_PCT / 100).max(MARGIN_MIN)
}

/// The clock: October 2026.
const CLOCK: u64 = 1_791_482_652;

/// A panel a session is drawn on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Panel {
    /// Pixels across.
    pub width: u16,
    /// Pixels down.
    pub height: u16,
    /// Dots per inch, from which the app picks its size class.
    pub dpi: u16,
}

/// A desktop window.
pub(super) const DESKTOP: Panel = Panel {
    width: 1366,
    height: 768,
    dpi: 160,
};

/// The Pi's 2.8-inch panel, portrait: its compact screens.
pub(super) const PI: Panel = Panel {
    width: 480,
    height: 640,
    dpi: 286,
};

/// `panel` given to the app as its display.
pub(super) fn show(app: &mut Faraday, panel: Panel) {
    app.event(Event::Display(DisplayInfo {
        width: panel.width,
        height: panel.height,
        dpi: panel.dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
}

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

/// Frames and ticks until whatever runs on ticks is done.
fn ticks(app: &mut Faraday, from: u64, n: u64) {
    for t in from..from + n {
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

/// A key New key makes in Create's slot `slot` from coin flips that
/// differ by `seed`.
fn key_made_here(app: &mut Faraday, slot: u8, seed: usize) {
    app.press(Action::KeyGen(Some(slot)));
    app.press(Action::KWords(12));
    app.press(Action::KWay(Way::Coins.index()));
    app.press(Action::KNext);
    for i in 0..128 {
        app.press(Action::KFlip(
            !(i * 7 + i / 5 + seed * 13 + i * seed).is_multiple_of(3),
        ));
    }
    app.press(Action::KNext);
    app.press(Action::KLock);
    assert!(
        app.keygen.as_ref().is_some_and(|k| k.locked),
        "slot {slot}: the key did not lock in"
    );
    app.press(Action::KNext);
    assert_eq!(app.screen, Screen::Create, "slot {slot}: back on Create");
}

/// The heaviest session on a desktop: see the module's comment. Returns
/// the app and the 2-of-3's index.
pub(super) fn heavy() -> (Faraday, usize) {
    heavy_on(DESKTOP)
}

/// The heaviest session on `panel`.
pub(super) fn heavy_on(panel: Panel) -> (Faraday, usize) {
    let mut app = Faraday::new();
    show(&mut app, panel);
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
    let mut inbox = testkit::files().expect("the test stick's files");
    inbox.push((
        "SHA256SUMS".into(),
        b"3d6f  faraday-0.1.0-linux-x86_64\n".to_vec(),
    ));
    app.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    app.storage(StorageEvent::Clock { unix_secs: CLOCK });
    app.vaults.ms_per_unit = Some(180);
    // On testnet, the test stick's network.
    testkit::move_to(&mut app.session, testkit::NET).unwrap();
    assert_eq!(app.session.network(), testkit::NET);
    let _ = app.frame();

    // A 2-of-3 made in Create, each key from New key in its slot.
    app.press(Action::CreateWallet);
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .expect("multisig") as u8;
    app.press(Action::CKind(multi));
    app.press(Action::CNext(cstep::KIND));
    let mut w = None;
    for _ in 0..16 {
        let c = app.create.as_ref().expect("Create is open");
        if let Some(b) = c.built {
            w = Some(b);
            break;
        }
        let k = c
            .open
            .unwrap_or_else(|| panic!("no card open: {:?}", c.error));
        if k == cstep::WHERE {
            app.press(Action::CVaults(false));
        }
        if k == cstep::KEYS && app.session.keys.is_empty() {
            for slot in 0..3u8 {
                key_made_here(&mut app, slot, usize::from(slot) + 1);
            }
        }
        app.press(Action::CNext(k));
    }
    let w = w.expect("the 2-of-3 was made");
    assert_eq!(app.session.keys.len(), 3);
    (app, w)
}

/// The test vault, unlocked from the Vaults tab with its first passphrase.
pub(super) fn unlock_test_vault(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Vaults));
    let n = app
        .vault_files()
        .iter()
        .position(|f| f.name == "vault.ofv")
        .expect("the test vault is listed");
    app.press(Action::Vault(V::Open(n)));
    type_text(app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    for t in 1..60u64 {
        if !app.vaults.open.is_empty() {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

/// The work of one frame of the screen as it stands, drawn again with
/// nothing changed: the frame before it warms what is kept.
fn frame_work(app: &mut Faraday) -> Counts {
    frame_costs(app).0
}

/// [`frame_work`], with the allocations the frame made and the lines of
/// text it drew.
fn frame_costs(app: &mut Faraday) -> (Counts, u64, u64) {
    let lines = |app: &Faraday| app.canvas.as_ref().map_or(0, |c| c.lines_drawn());
    app.settle();
    let _ = app.frame();
    app.dirty = true;
    let _ = app.frame();
    let before = snapshot();
    let (allocs, drawn) = (faraday_alloc_count::allocations(), lines(app));
    app.dirty = true;
    let _ = app.frame();
    (
        since(&before),
        faraday_alloc_count::allocations() - allocs,
        lines(app) - drawn,
    )
}

/// The work of moving the pointer over the screen, the events alone.
fn hover_work(app: &mut Faraday) -> Counts {
    let before = snapshot();
    for (x, y) in [(300, 200), (700, 400), (1000, 600), (120, 700)] {
        app.event(Event::Hover { x, y });
    }
    app.event(Event::HoverEnd);
    since(&before)
}

/// Each screen's counts, and the failures against [`BUDGETS`] and the
/// frame costs.
struct Report {
    /// The frame costs measured, against.
    costs: &'static [(&'static str, u64, u64)],
    lines: Vec<String>,
    /// Each screen's allocations and lines, as [`FRAME_COSTS`] writes
    /// them.
    measured: Vec<String>,
    failures: Vec<String>,
}

impl Report {
    fn new(costs: &'static [(&'static str, u64, u64)]) -> Self {
        Report {
            costs,
            lines: Vec::new(),
            measured: Vec::new(),
            failures: Vec::new(),
        }
    }

    fn check(&mut self, app: &mut Faraday, name: &str) {
        let (frame, allocs, drawn) = frame_costs(app);
        let hover = hover_work(app);
        self.measured
            .push(format!("    (\"{name}\", {allocs}, {drawn}),"));
        match self.costs.iter().find(|(n, ..)| *n == name) {
            None => self.failures.push(format!(
                "{name}: no measured frame costs (allocations {allocs}, lines {drawn})"
            )),
            Some(&(_, a, l)) => {
                if allocs > with_margin(a) {
                    self.failures.push(format!(
                        "{name}: a frame made {allocs} allocations, measured {a} (most {}): a frame did work no one marked",
                        with_margin(a)
                    ));
                }
                if drawn > with_margin(l) {
                    self.failures.push(format!(
                        "{name}: a frame drew {drawn} lines of text, measured {l} (most {}): a frame drew what no one marked",
                        with_margin(l)
                    ));
                }
            }
        }
        let mut line = format!("{name} ({:?}, {:?}):", app.screen, app.sheet);
        for &k in &KINDS {
            let n = frame.of(k);
            if n > 0 {
                line.push_str(&format!(" {k:?} {n}"));
            }
            let mut budget = budget(name, k);
            if k == Work::FullCanvas && app.sheet.is_some() {
                budget = budget.max(SHEET_CANVAS);
            }
            if n > budget {
                self.failures.push(format!(
                    "{name}: a frame did {k:?} {n} times (budget {budget}): a frame did work the drawing memo should keep (docs/SPEED.md §3.3)"
                ));
            }
            let h = hover.of(k);
            if h > 0 {
                self.failures.push(format!(
                    "{name}: a pointer move did {k:?} {h} times: an event that changes nothing does no costly work (docs/SPEED.md §3.5)"
                ));
            }
        }
        self.lines.push(line);
    }
}

/// The heavy session walked through every screen the budget covers on
/// a desktop: `stop` is called at each, with its name, on the app as it
/// stands there. Returns the app at the end and the 2-of-3's index.
pub(super) fn walk(c: &Compute, stop: &mut dyn FnMut(&mut Faraday, &str)) -> (Faraday, usize) {
    walk_on(c, DESKTOP, stop)
}

/// [`walk`] on `panel`.
#[expect(clippy::too_many_lines, reason = "counted in lint-layout (F74)")]
pub(super) fn walk_on(
    c: &Compute,
    panel: Panel,
    stop: &mut dyn FnMut(&mut Faraday, &str),
) -> (Faraday, usize) {
    let (mut app, w) = heavy_on(panel);
    stop(&mut app, "Create");

    // The backup: the board, then the checklist.
    app.press(Action::CBackup);
    testkit::backup_as(&mut app, crate::plan::Preset::PaperVault);
    assert_eq!(app.screen, Screen::Backup);
    stop(&mut app, "Backup board");
    app.press(Action::BChecklist);
    if app.backup_items_now(c).contains(&bstep::BLANK) {
        app.press(Action::BNext(bstep::BLANK));
    }
    stop(&mut app, "Backup checklist");

    // The test vault unlocked from the Vaults tab: its Load sheet, then
    // everything it keeps loaded.
    unlock_test_vault(&mut app);
    assert_eq!(
        app.sheet,
        Some(Sheet::UnlockLoad),
        "{:?}",
        app.vaults.unlock_error
    );
    stop(&mut app, "Load sheet");
    app.press(Action::Vault(V::LoadTicked));
    // Every test wallet loaded beside the 2-of-3, as Load would.
    for kit in testkit::kits() {
        let _ = app.session.add_wallet(
            c,
            kit.name,
            &kit.descriptor,
            "test",
            crate::wallet::Origin::Named,
        );
    }
    assert!(app.session.wallets.len() > 5, "the test wallets loaded");

    // A GPG key's flow on Make over the open vault, made and released:
    // Sign a file, and its sheet.
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeKeep);
    let (salt, name) = {
        let o = &app.vaults.open[app.vaults.current];
        (o.header().salt, o.name.clone())
    };
    let mut plan = Plan::key_default();
    plan.vaults[0].salt = Some(salt);
    plan.vaults[0].file = Some(name);
    plan.vaults[0].existing = true;
    let mut f = BackupFlow::with_plan(c, Kind::Gpg, plan);
    f.step = Step::Make;
    app.backup_flows.push(f);
    app.press(Action::Flow(F::Show(app.backup_flows.len() - 1)));
    app.press(Action::Vault(V::FocusField(0)));
    type_text(&mut app, "Release");
    app.press(Action::Flow(F::Make));
    ticks(&mut app, 300, 2);
    assert!(
        app.backup_flows.last().is_some_and(|f| f.made),
        "the GPG key was made"
    );
    stop(&mut app, "GPG key flow");
    for s in &mut app.backup_flows.last_mut().expect("the flow").slots {
        s.written = Some(Written {
            stick: String::new(),
            serial: None,
            label: "STICK".into(),
            time: None,
            files: Vec::new(),
        });
    }
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(Kind::Gpg.category())));
    app.press(Action::Vault(V::GpgSignPick));
    stop(&mut app, "GPG Sign a file");
    let listed = app
        .inbox
        .iter()
        .filter(|i| !i.name.ends_with(".asc"))
        .position(|i| i.name == "SHA256SUMS")
        .expect("SHA256SUMS is listed");
    app.press(Action::Vault(V::GpgTick(listed)));
    app.press(Action::Vault(V::GpgSign));
    assert_eq!(app.sheet, Some(Sheet::GpgSign), "the GPG sign sheet opened");
    stop(&mut app, "GPG sign sheet");
    app.press(Action::Cancel);

    // A spend of Savings open.
    let psbt = app
        .inbox
        .iter()
        .position(|i| i.name == "savings-unsigned.psbt")
        .expect("the spend is in the Inbox");
    app.press(Action::StartSpend(psbt));
    stop(&mut app, "Spend");

    for (name, screen) in MAIN {
        app.press(Action::Nav(screen));
        stop(&mut app, name);
    }
    for (name, a) in SHEETS {
        app.press(a);
        assert!(app.sheet.is_some(), "{name} opened");
        stop(&mut app, name);
        app.press(Action::Cancel);
    }
    app.press(Action::OpenWallet(w));
    stop(&mut app, "Wallet chart");
    app.press(Action::Backup(w));
    stop(&mut app, "Backup reopened");
    app.press(Action::CreateWallet);
    stop(&mut app, "Create, new");
    (app, w)
}

/// The sheets every main screen offers, by name, each with what opens it.
pub(super) const SHEETS: [(&str, Action); 3] = [
    ("Power sheet", Action::PowerAsk),
    ("Learn sheet", Action::Learn),
    ("Scan sheet", Action::Scan),
];

/// The main screens, each reached from the navigation, by name.
pub(super) const MAIN: [(&str, Screen); 9] = [
    ("Home", Screen::Home),
    ("Wallets", Screen::Wallets),
    ("Files", Screen::Files),
    ("Vaults", Screen::Vaults),
    ("Vault contents", Screen::VaultContents),
    ("Backups", Screen::Backups),
    ("Spend tab", Screen::Family),
    ("Settings", Screen::Settings),
    ("Tools", Screen::Tools),
];

/// The walk on `panel`, each screen checked against the budgets and
/// `costs`.
fn budgets_hold_on(panel: Panel, costs: &'static [(&'static str, u64, u64)]) {
    let c = crate::testkit::compute();
    let start = snapshot();
    let mut r = Report::new(costs);
    let _ = walk_on(&c, panel, &mut |app, name| r.check(app, name));
    assert!(
        since(&start).of(Work::MasterKey) >= 3,
        "the three keys made here were counted"
    );
    assert!(
        r.failures.is_empty(),
        "{}\n\nEach screen's frame:\n{}\n\nEach screen's allocations and lines:\n{}",
        r.failures.join("\n"),
        r.lines.join("\n"),
        r.measured.join("\n")
    );
}

#[test]
fn a_frame_drawn_again_and_a_pointer_move_do_no_costly_work_beyond_the_budget() {
    budgets_hold_on(DESKTOP, FRAME_COSTS);
}

#[test]
fn on_the_pis_panel_a_frame_drawn_again_and_a_pointer_move_do_no_costly_work_beyond_the_budget() {
    budgets_hold_on(PI, FRAME_COSTS_PI);
}
