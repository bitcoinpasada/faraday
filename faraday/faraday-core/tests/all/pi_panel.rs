//! The Pi's 2.8-inch panel, 268 × 358 dp: nothing on any page, step, tab
//! or sheet is cut off at its sides or drawn over other text, in every
//! mode, on the stick's Pi (480 × 640 pixels at 286 dpi) and in Faraday
//! Demo's Pi preview (the app framed at the panel's size in the window).
//! Every control and every line of text stands inside the panel and
//! inside the card or sheet it is drawn in, at every place the page
//! scrolls to; no line of text is drawn over another; no two touch
//! targets of the bar across the top overlap; what runs past the foot of a
//! page that scrolls, or past the sides of a view that pans sideways, is
//! not counted (`docs/LAYOUT.md` §8).
//!
//! Each page is walked: checked, then every step card, tab and section
//! it offers is pressed in turn and the page checked again, so a step or
//! a tab added to a flow is covered without being named here. The
//! screens and sheets are listed by a match that fails to build when one
//! is added and not listed.

use crate::common::add_key;
use faraday_core::backup_flow::steps::FlowAction;
use faraday_core::backup_flow::{BackupFlow, Kind as FlowKind, Plan, Step as FlowStep};
use faraday_core::catalog::TILES;
use faraday_core::codex32::{C32, Card, SPLITS, Split, WAYS, Way as C32Way};
use faraday_core::family::FamilyAction as F;
use faraday_core::in_progress::Going;
use faraday_core::keygen::{Start, Way, kstep};
use faraday_core::nav::{BarTool, MenuPress, Overflow};
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, Host, ScanPurpose, Screen, Sheet, StorageEvent, testkit};
use faraday_shell_api::{
    App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware, TouchPhase,
};
use faraday_ui::keyboard::OskPress;

/// A string drawn and the box of its ink, pixels (x, y, w, h).
type DrawnText = (String, (i32, i32, i32, i32));

/// The Pi panel's size in dp, as Faraday Demo's Size menu lists it.
const PI: (u16, u16) = (268, 358);

/// The most presses a page's walk makes.
const PRESSES: usize = 60;

/// Where the panel is.
#[derive(Clone, Copy, Debug)]
enum On {
    /// The stick's Pi: 480 × 640 pixels at 286 dpi.
    Stick,
    /// Faraday Demo's Pi preview, in a 1280 × 800 window at 160 dpi.
    Preview,
}

/// The mode the app is in.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Mainnet,
    Testnet,
    Practice,
}

/// The app on the panel, held by Faraday Demo's host in the preview and
/// by a bare host, standing for the device's screen, on the stick.
struct Rig {
    host: Host,
    mode: Mode,
    /// The app's display, pixels.
    w: u16,
    h: u16,
    label: String,
}

impl std::ops::Deref for Rig {
    type Target = Faraday;
    fn deref(&self) -> &Faraday {
        &self.host.app
    }
}

impl std::ops::DerefMut for Rig {
    fn deref_mut(&mut self) -> &mut Faraday {
        &mut self.host.app
    }
}

fn display(width: u16, height: u16, dpi: u16) -> Event {
    Event::Display(DisplayInfo {
        width,
        height,
        dpi,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    })
}

/// The app on `on` in `mode`, with `inbox` in Files, on Home.
fn rig(on: On, mode: Mode, inbox: Vec<(String, Vec<u8>)>) -> Rig {
    let mut app = Faraday::new();
    match mode {
        Mode::Mainnet => {}
        Mode::Testnet => testkit::move_to(&mut app.session, testkit::NET).unwrap(),
        Mode::Practice => testkit::into_demo(&mut app),
    }
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x31 + i * 7; 32])));
    }
    let host = match on {
        On::Stick => {
            app.event(display(480, 640, 286));
            Host::bare(app)
        }
        On::Preview => {
            app.shell_sticks = true;
            app.online = true;
            app.set_roller_offered(true);
            if mode == Mode::Practice {
                app.samples = Some(testkit::demo_samples());
            }
            let mut host = Host::new(app);
            host.event(display(1280, 800, 160));
            host.set_preview(Some(PI));
            host
        }
    };
    let mut rig = Rig {
        host,
        mode,
        w: 0,
        h: 0,
        label: format!("{on:?} {mode:?}"),
    };
    rig.storage(StorageEvent::Restored {
        inbox,
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    rig.press(Action::OpenSetTime);
    rig.press(Action::SetTimeKeep);
    rig.press(Action::Nav(Screen::Home));
    let _ = rig.frame();
    let (w, h) = rig.size();
    let f = match on {
        On::Stick => 286.0 / 160.0,
        On::Preview => 1.0,
    };
    rig.w = (w * f).round() as u16;
    rig.h = (h * f).round() as u16;
    rig
}

/// Lists every value of an enum, failing to build when one is added
/// and not listed.
macro_rules! every {
    ($t:ident: $($v:ident $(($arg:expr))?),* $(,)?) => {{
        #[allow(dead_code)]
        fn listed(x: $t) {
            match x {
                $($t::$v { .. } => {})*
            }
        }
        vec![$($t::$v $(($arg))?),*]
    }};
}

fn screens() -> Vec<Screen> {
    every!(Screen: Home, WalletsStart, Wallets, Spend, Files, Visit, Entry, Backup,
        Message, CheckMessage, Create, Restore, Settings, SetTime, Randomness, Vaults,
        Backups, CreateVault, KeyGen, Bip85, Convert, KefTool, QrTool, Silent, Explore, Lightning, Tools, Unlock,
        VaultContents, Codes, Family, Vanity, Decode, NonceCheck, SignerCheck, Catalog, Transfer, BackupFlow)
}

fn sheets() -> Vec<Sheet> {
    let mut all = every!(Sheet: Lock, LockAsk, Power, Qr, Scan, Locked, NewInput, Learn, WordList,
        SecretOut, WriteOut, IdleWarn, Potential, Import, Pull, Chart, BoardList, BoardSees,
        BoardPartials, BoardMenu, SeedRead, KefRead, Delete, Zeroing, Taken, NameTaken, Devices, VaultSave,
        CancelFlow(Going::Create), CancelKey, FileMenu, Rename, Details, UnlockLoad, SaveAlso, CopyTo, SaveKeeps, Trim, OlderCopy,
        ChildRecord, ChildNotice,
        PassphraseSeeds, GpgSign, Mode, Samples, NavMenu(None), KeyLengths, KeyPath, Account, Register,
        GpgRestore, FigMenu, LaterAsk, StickSheet, KefOptions, VaultPassphrase, VaultMemory,
        Sweep);
    // A compact bar's overflow menu, holding every tool it can.
    all.push(Sheet::NavMenu(Some(Overflow {
        own: true,
        how: true,
        help: true,
        keyboard: true,
        keyboard_up: true,
        eye: true,
    })));
    all
}

fn flow_steps() -> Vec<FlowStep> {
    every!(FlowStep: Where, Sticks, Vaults, Make, Lock, Write, Done)
}

fn flow_kinds() -> Vec<FlowKind> {
    every!(FlowKind: Gpg, SecureBoot, Wallet)
}

/// Opens `s` as a person does: by the navigation, or where it has no
/// place there, by what starts it.
fn open(rig: &mut Rig, s: Screen) {
    let a = match s {
        Screen::KeyGen => Action::KeyGen(None),
        Screen::CreateVault => Action::Vault(V::Create),
        Screen::BackupFlow => {
            let c = testkit::compute();
            let i = testkit::flow_push(rig, BackupFlow::new(&c, FlowKind::Gpg));
            Action::Flow(FlowAction::Show(i))
        }
        _ => Action::Nav(s),
    };
    rig.press(a);
    rig.settle();
}

/// The flow on show left by its Cancel, and Home.
fn leave(rig: &mut Rig) {
    rig.press(Action::CancelFlow);
    if matches!(rig.sheet, Some(Sheet::CancelFlow(_))) {
        rig.press(Action::CancelFlowNow);
    }
    rig.sheet = None;
    rig.press(Action::Nav(Screen::Home));
}

/// Make codex32 shares stands on `card`.
fn on_card(rig: &Rig, card: Card) {
    let open = rig
        .keygen()
        .and_then(|k| k.codex32.as_ref())
        .and_then(|m| m.open());
    assert_eq!(open, Some(card), "{}: Make codex32 shares", rig.label);
}

/// Whether a press only opens or closes a part of the page: a step card,
/// a tab, a section, a category or an item.
fn opens_a_part(a: &Action) -> bool {
    matches!(
        a,
        Action::Step(_)
            | Action::BStep(_)
            | Action::MStep(_)
            | Action::CStep(_)
            | Action::SStep(_)
            | Action::PStep(_)
            | Action::KStep(_)
            | Action::RStep(_)
            | Action::SettingsSection(Some(_))
            | Action::BackupsPage(_)
            | Action::Codex32(C32::Card(_) | C32::Tab(_) | C32::Panel(_) | C32::View(_))
            | Action::Vault(V::CStep(_) | V::Category(_) | V::Item(_))
            | Action::Family(F::Card(_))
            | Action::Flow(FlowAction::Open(_))
    )
}

/// Text boxes, pixels, that overlap by more than a pixel each way.
/// A toast (Material's snackbar) stands over the page by design: its line
/// is not counted. Nor are two marks of a grid, each a character or two (a
/// worksheet's cell and its index), which sit close by design.
fn drawn_over(texts: &[DrawnText], toast: Option<&str>) -> Vec<String> {
    let mut out = Vec::new();
    for (i, (a, ra)) in texts.iter().enumerate() {
        for (b, rb) in &texts[i + 1..] {
            let marks = a.chars().count() < 3 && b.chars().count() < 3;
            if (a == b && ra == rb) || marks || toast.is_some_and(|t| t == a || t == b) {
                continue;
            }
            let w = (ra.0 + ra.2).min(rb.0 + rb.2) - ra.0.max(rb.0);
            let h = (ra.1 + ra.3).min(rb.1 + rb.3) - ra.1.max(rb.1);
            if w > 1 && h > 1 {
                out.push(format!("text {a:?} drawn over {b:?} by {w} × {h} px"));
            }
        }
    }
    out
}

/// The controls of the bar across the page's top (64 dp) whose touch
/// targets overlap: a press there could reach either.
fn head_row_overlaps(rig: &Rig) -> Vec<String> {
    let f = f32::from(rig.w) / rig.size().0;
    let bar = (64.0 * f).round() as i32 + 1;
    let boxes: Vec<(Action, (i32, i32, i32, i32))> = rig
        .drawn_actions()
        .into_iter()
        .filter_map(|a| rig.hit_box(a).map(|b| (a, b)))
        .map(|(a, b)| {
            let b = (
                i32::from(b.0),
                i32::from(b.1),
                i32::from(b.2),
                i32::from(b.3),
            );
            (a, b)
        })
        .filter(|(_, b)| b.1 < bar / 2 && b.1 + b.3 <= bar)
        .collect();
    let mut out = Vec::new();
    for (i, (a, ra)) in boxes.iter().enumerate() {
        for (b, rb) in &boxes[i + 1..] {
            let w = (ra.0 + ra.2).min(rb.0 + rb.2) - ra.0.max(rb.0);
            let h = (ra.1 + ra.3).min(rb.1 + rb.3) - ra.1.max(rb.1);
            if w > 1 && h > 1 {
                out.push(format!(
                    "touch targets of {a:?} and {b:?} overlap by {w} px"
                ));
            }
        }
    }
    out
}

/// What the page drawn now has cut off at its sides or drawn over other
/// text, from its top to its foot, each named with `what`; and what it
/// offers to a press anywhere along it.
fn check(rig: &mut Rig, what: &str) -> (Vec<String>, Vec<Action>) {
    let what = format!("{}, {what}", rig.label);
    let (x, y) = (rig.w / 2, rig.h / 2);
    let dy = (rig.h / 4) as i16;
    let mut found: Vec<String> = Vec::new();
    let mut offered: Vec<Action> = Vec::new();
    let mut last = None;
    for _ in 0..80 {
        let (past, texts) = rig.drawn_past_sides_and_text();
        let mut lines = past;
        if rig.sheet.is_none() {
            let toast = rig.toast_text().map(str::to_string);
            lines.extend(drawn_over(&texts, toast.as_deref()));
            lines.extend(head_row_overlaps(rig));
        }
        for p in lines {
            let line = format!("{what}: {p}");
            if !found.contains(&line) {
                found.push(line);
            }
        }
        for a in rig.drawn_actions() {
            if !offered.contains(&a) {
                offered.push(a);
            }
        }
        if last.as_ref() == Some(&texts) {
            break;
        }
        last = Some(texts);
        rig.event(Event::Scroll { x, y, dy });
        rig.settle();
    }
    for _ in 0..80 {
        rig.event(Event::Scroll { x, y, dy: -dy });
    }
    rig.settle();
    (found, offered)
}

/// The page drawn now checked, then each part it offers opened in turn
/// and the page checked again, through the parts those open.
fn walk(rig: &mut Rig, what: &str) -> Vec<String> {
    walk_through(rig, what, opens_a_part)
}

/// Whether a press only shows another tab of the page.
fn shows_a_tab(a: &Action) -> bool {
    matches!(
        a,
        Action::Codex32(C32::Tab(_) | C32::Panel(_) | C32::View(_))
    )
}

/// The page drawn now checked, then each tab it offers shown in turn: for
/// a page part way through a flow, whose steps a press would move.
fn look(rig: &mut Rig, what: &str) -> Vec<String> {
    walk_through(rig, what, shows_a_tab)
}

/// The page drawn now checked, then each press `part` names that it
/// offers made in turn and the page checked again, through the ones
/// those offer.
fn walk_through(rig: &mut Rig, what: &str, part: fn(&Action) -> bool) -> Vec<String> {
    let (mut found, mut offered) = check(rig, what);
    let mut pressed: Vec<Action> = Vec::new();
    for _ in 0..PRESSES {
        let Some(a) = offered
            .iter()
            .copied()
            .find(|a| part(a) && !pressed.contains(a))
        else {
            break;
        };
        pressed.push(a);
        rig.press(a);
        rig.settle();
        let (f, o) = check(rig, &format!("{what}, {a:?}"));
        found.extend(f);
        for a in o {
            if !offered.contains(&a) {
                offered.push(a);
            }
        }
    }
    found
}

/// Each screen walked, and each sheet over Home, and the scanner.
fn every_screen(rig: &mut Rig, state: &str) -> Vec<String> {
    let mut found = Vec::new();
    for s in screens() {
        rig.press(Action::Nav(Screen::Home));
        rig.sheet = None;
        open(rig, s);
        let what = format!("{state} {s:?} (shows {:?})", rig.screen);
        found.extend(walk(rig, &what));
    }
    rig.press(Action::Nav(Screen::Home));
    for sheet in sheets() {
        rig.sheet = Some(sheet);
        rig.settle();
        found.extend(check(rig, &format!("{state} Home under {sheet:?}")).0);
        rig.sheet = None;
    }
    for purpose in [ScanPurpose::Files, ScanPurpose::Seed, ScanPurpose::Intake] {
        testkit::open_scan(rig, purpose);
        rig.settle();
        found.extend(check(rig, &format!("{state} scanner for {purpose:?}")).0);
        rig.press(Action::Cancel);
    }
    found
}

fn kit(id: &str) -> testkit::Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

/// Savings' wallet file and its unsigned spend in Files, with test keys
/// 1 and 2 loaded and the wallet loaded.
fn savings_loaded(on: On, mode: Mode) -> Rig {
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let mut rig = rig(
        on,
        mode,
        vec![
            ("savings-wallet.txt".to_string(), k.descriptor.into_bytes()),
            ("savings-unsigned.psbt".to_string(), unsigned.to_bytes()),
        ],
    );
    let at = |app: &Faraday, name: &str| app.inbox().iter().position(|i| i.name == name).unwrap();
    let w = at(&rig, "savings-wallet.txt");
    rig.press(Action::LoadWallet(w));
    add_key(&mut rig, testkit::TEST_SEEDS[0].0);
    add_key(&mut rig, testkit::TEST_SEEDS[1].0);
    rig.press(Action::Nav(Screen::Home));
    rig
}

/// Every step card of the spend open in turn, then signed here.
fn every_spend_step(rig: &mut Rig) -> Vec<String> {
    let at = rig
        .inbox()
        .iter()
        .position(|i| i.name == "savings-unsigned.psbt")
        .unwrap();
    rig.press(Action::StartSpend(at));
    let mut found = Vec::new();
    let steps = [
        step::WALLET,
        step::CHECK,
        step::TRANSACTION,
        step::TXID,
        step::SIGNERS,
        step::SIGN,
    ];
    for s in steps {
        rig.press(Action::Step(s));
        rig.settle();
        found.extend(check(rig, &format!("spend step {s}")).0);
    }
    testkit::steps_before_sign_done(rig);
    rig.press(Action::SignHere);
    rig.settle();
    found.extend(walk(rig, "spend signed"));
    found
}

/// The Spend tab's pages, from the map to the transaction, by its own
/// Next on each.
fn every_family_page(rig: &mut Rig) -> Vec<String> {
    use faraday_core::family::page;
    rig.press(Action::Nav(Screen::Family));
    let mut found = Vec::new();
    for p in [page::MAP, page::CHECK, page::WRITE] {
        found.extend(walk(rig, &format!("Spend tab page {p}")));
        rig.press(Action::Family(F::Next(p)));
        rig.settle();
    }
    found.extend(walk(rig, "Spend tab page after Write"));
    found
}

/// The wallet and its keys loaded: every screen, Savings open, a spend's
/// steps and the Spend tab's pages, then the same online.
fn loaded(on: On, mode: Mode) -> Vec<String> {
    let mut rig = savings_loaded(on, mode);
    let mut found = every_screen(&mut rig, "savings");
    rig.press(Action::Nav(Screen::Wallets));
    rig.press(Action::OpenWallet(0));
    rig.settle();
    found.extend(walk(&mut rig, "savings, Savings open"));
    rig.press(Action::BackupChecklist(0));
    rig.settle();
    found.extend(walk(&mut rig, "savings, Savings' backup checklist"));
    found.extend(every_spend_step(&mut rig));
    let mut rig = savings_loaded(on, mode);
    found.extend(every_family_page(&mut rig));
    found
}

fn kpress(rig: &mut Rig, a: Action, what: &str, found: &mut Vec<String>) {
    rig.press(a);
    rig.settle();
    found.extend(look(rig, what));
}

/// New key: each way to start walked; a BIP-39 key from coin flips
/// through its words and quiz; This device's Generator with dice of
/// one's own; SLIP-39 shares through their check, shares and quiz.
fn new_key(rig: &mut Rig) -> Vec<String> {
    use faraday_core::fresh::OwnSource;
    use faraday_core::fresh::page::Act;
    use faraday_core::keygen::STARTS;
    let mut found = Vec::new();
    for (i, s) in STARTS.iter().enumerate() {
        rig.press(Action::Nav(Screen::Home));
        rig.press(Action::KeyGen(None));
        rig.press(Action::KStart(i as u8));
        rig.settle();
        found.extend(walk(rig, &format!("New key, {s:?}")));
        leave(rig);
    }
    // Length, its Other menu, Randomness and the Generator.
    rig.press(Action::Nav(Screen::Home));
    rig.press(Action::KeyGen(None));
    rig.settle();
    found.extend(walk(rig, "New key, Length"));
    leave(rig);
    rig.press(Action::KeyGen(None));
    rig.settle();
    rig.press(Action::KOtherLengths);
    rig.settle();
    found.extend(check(rig, "New key, Length's Other menu").0);
    rig.press(Action::Cancel);
    kpress(rig, Action::KWords(12), "New key, Randomness", &mut found);
    rig.press(Action::KWay(Way::Device.index()));
    rig.event(Event::Entropy(EntropyBytes::new([0x6c; 32])));
    rig.settle();
    found.extend(look(rig, "New key, Generator"));
    rig.press(Action::Own(Act::Source(OwnSource::Dice)));
    for f in [3, 1, 4] {
        rig.press(Action::Own(Act::Roll(f)));
    }
    rig.settle();
    found.extend(check(rig, "New key, Generator on Dice").0);
    kpress(
        rig,
        Action::Own(Act::Add),
        "New key, Generator with dice added",
        &mut found,
    );
    leave(rig);
    // BIP-39 from coin flips, through the words and the quiz.
    rig.press(Action::Nav(Screen::Home));
    rig.press(Action::KeyGen(None));
    rig.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    rig.press(Action::KWords(12));
    rig.press(Action::KWay(Way::Coins.index()));
    kpress(rig, Action::KNext, "New key, BIP-39 flips", &mut found);
    for i in 0..128 {
        rig.press(Action::KFlip(i % 3 != 1));
    }
    for n in 0..4 {
        if rig.keygen().is_some_and(|k| k.quiz.is_some()) {
            break;
        }
        kpress(
            rig,
            Action::KNext,
            &format!("New key, BIP-39 Continue {n}"),
            &mut found,
        );
    }
    quiz(rig, "New key, BIP-39", &mut found);
    leave(rig);
    // SLIP-39 shares, through their check, the shares and the quiz.
    rig.press(Action::Nav(Screen::Home));
    rig.press(Action::KeyGenSlip39);
    rig.press(Action::KWords(20));
    rig.press(Action::KWay(Way::Coins.index()));
    kpress(rig, Action::KNext, "New key, SLIP-39 flips", &mut found);
    for i in 0..128 {
        rig.press(Action::KFlip(i % 3 != 1));
    }
    for n in 0..4 {
        if rig.keygen().is_some_and(|k| k.quiz.is_some()) {
            break;
        }
        kpress(
            rig,
            Action::KNext,
            &format!("New key, SLIP-39 Continue {n}"),
            &mut found,
        );
    }
    let answered = quiz(rig, "New key, SLIP-39", &mut found);
    // Practice takes only its samples: it makes no shares.
    assert!(
        answered > 0 || rig.mode == Mode::Practice,
        "{}: SLIP-39's quiz",
        rig.label
    );
    leave(rig);
    found
}

/// The quiz answered right, walked part way and at its end.
fn quiz(rig: &mut Rig, what: &str, found: &mut Vec<String>) -> usize {
    let mut answered = 0;
    for n in 0..200 {
        let Some(slot) = rig
            .keygen()
            .and_then(|k| k.quiz.as_ref())
            .filter(|_| {
                !rig.keygen()
                    .is_some_and(|k| k.done[usize::from(kstep::QUIZ)])
            })
            .map(|q| q.correct_slot() as u8)
        else {
            break;
        };
        rig.press(Action::KQuiz(slot));
        answered += 1;
        if n == 1 {
            rig.settle();
            found.extend(check(rig, &format!("{what} quiz part way")).0);
        }
    }
    rig.settle();
    found.extend(look(rig, &format!("{what} quiz done")));
    answered
}

const CHARSET: &[u8; 32] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// BIP 93's vector 2, shares A and C (`tools/vectors/bip93/vectors.json`).
const SHARES: [&str; 2] = [
    "ms12namea320zyxwvutsrqpnmlkjhgfedcaxrpp870hkkqrm",
    "ms12namecacdefghjklmnpqrstuvwxyz023ftr2gdzmpy6pn",
];

fn c32(rig: &mut Rig, p: C32) {
    rig.press(Action::Codex32(p));
}

fn c32_key(rig: &mut Rig, ch: char) {
    c32(rig, C32::Key(OskPress::Char(ch)));
}

/// Character `ch` rolled on the booklet's sheet: each die's bit 0 as 4
/// then 3, 1 as 3 then 4.
fn roll(rig: &mut Rig, ch: u8) {
    let v = CHARSET.iter().position(|&x| x == ch).unwrap() as u8;
    let bits: Vec<u8> = (0..5).map(|d| v >> (4 - d) & 1).collect();
    for &b in &bits {
        c32_key(rig, if b == 0 { '4' } else { '3' });
    }
    for &b in &bits {
        c32_key(rig, if b == 0 { '3' } else { '4' });
    }
}

/// Make codex32 shares, 2 of 3 named "name", on dice, through every card
/// and tab: the scheme, the identifier, the entropy, the entry part way
/// and done (its Dice, Checksum, String and Working tabs), the
/// worksheets' tabs, each string typed back, and Strings made.
/// Opened from New key, or from Make a new wallet's single key with its
/// backup in shares (`from_create`), where the scheme is the Quorum's and
/// the bar carries Cancel.
fn codex32_shares(rig: &mut Rig, from_create: bool) -> Vec<String> {
    let mut found = Vec::new();
    rig.press(Action::Nav(Screen::Home));
    if from_create {
        to_codex32_from_create(rig);
    } else {
        rig.press(Action::KeyGen(None));
        rig.press(Action::KStart(Start::MakeCodex32.index()));
        rig.settle();
        on_card(rig, Card::Scheme);
        found.extend(look(rig, "codex32 Scheme"));
        let two = SPLITS.iter().position(|s| *s == Split::TwoOfThree).unwrap();
        c32(rig, C32::Split(two as u8));
        c32(rig, C32::Next);
        rig.settle();
    }
    on_card(rig, Card::Identifier);
    found.extend(look(rig, "codex32 Identifier"));
    for ch in "na".chars() {
        c32_key(rig, ch);
    }
    rig.settle();
    found.extend(check(rig, "codex32 Identifier part typed").0);
    for ch in "me".chars() {
        c32_key(rig, ch);
    }
    c32(rig, C32::Next);
    let dice = WAYS.iter().position(|w| *w == C32Way::Dice).unwrap();
    c32(rig, C32::Way(dice as u8));
    rig.settle();
    on_card(rig, Card::Entropy);
    found.extend(look(rig, "codex32 Entropy on dice"));
    c32(rig, C32::Next);
    rig.settle();
    on_card(rig, Card::Enter);
    found.extend(look(rig, "codex32 Enter, nothing rolled"));
    let a = SHARES[0].as_bytes();
    for &ch in &a[9..15] {
        roll(rig, ch);
    }
    for f in ['3', '4', '1'] {
        c32_key(rig, f);
    }
    rig.settle();
    found.extend(look(rig, "codex32 Enter, part way"));
    c32(rig, C32::Panel(0));
    c32(rig, C32::View(0));
    for _ in 0..3 {
        c32(rig, C32::Key(OskPress::Back));
    }
    for &ch in &a[15..35] {
        roll(rig, ch);
    }
    for &ch in &SHARES[1].as_bytes()[9..35] {
        roll(rig, ch);
    }
    rig.settle();
    found.extend(look(rig, "codex32 Enter, done"));
    c32(rig, C32::Panel(0));
    c32(rig, C32::View(0));
    c32(rig, C32::Next);
    rig.settle();
    // Practice's roller takes every other value: the strings rolled here
    // are not complete there.
    if rig.mode == Mode::Practice {
        leave(rig);
        return found;
    }
    on_card(rig, Card::Sheets);
    found.extend(look(rig, "codex32 Sheets"));
    c32(rig, C32::Next);
    rig.settle();
    on_card(rig, Card::Strings);
    found.extend(look(rig, "codex32 Strings"));
    let all: Vec<String> = rig
        .keygen()
        .and_then(|k| k.codex32.as_ref())
        .and_then(|m| m.strings())
        .map(|s| s.iter().map(|s| s.to_string()).collect())
        .unwrap_or_default();
    for (n, s) in all.iter().enumerate() {
        for ch in s[3..].chars() {
            c32_key(rig, ch);
        }
        rig.settle();
        found.extend(check(rig, &format!("codex32 Strings, string {n} typed")).0);
        c32(rig, C32::Next);
    }
    rig.settle();
    on_card(rig, Card::Made);
    found.extend(look(rig, "codex32 Made"));
    leave(rig);
    found
}

/// The rows of the Kind card, all its kinds listed: the card opened by
/// `card` where it is closed, and its more kinds by `more`.
fn kind_rows(rig: &mut Rig, what: &str, card: Action, more: Action) -> Vec<u8> {
    let rows = |offered: Vec<Action>| -> Vec<u8> {
        offered
            .into_iter()
            .filter_map(|a| match a {
                Action::CKind(k) | Action::RKind(k) => Some(k),
                _ => None,
            })
            .collect()
    };
    let (_, offered) = check(rig, what);
    if rows(offered).is_empty() {
        rig.press(card);
        rig.settle();
    }
    let (_, offered) = check(rig, what);
    if offered.contains(&more) {
        rig.press(more);
        rig.settle();
    }
    rows(check(rig, what).1)
}

/// Make a new wallet's single key with its backup in shares, at 2 of 3:
/// Make codex32 shares opened for its key, on the identifier.
fn to_codex32_from_create(rig: &mut Rig) {
    use faraday_core::create::NewKind;
    use faraday_core::cstep;
    let shares = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Shares)
        .unwrap();
    rig.press(Action::Nav(Screen::WalletsStart));
    rig.press(Action::CreateWallet);
    rig.press(Action::CKind(shares as u8));
    rig.press(Action::CNext(cstep::QUORUM));
    rig.press(Action::CVaults(false));
    rig.settle();
}

/// Make a new wallet and Restore: each kind each lists, walked.
fn wallet_flows(rig: &mut Rig) -> Vec<String> {
    use faraday_core::{cstep, rstep};
    let mut found = Vec::new();
    for (what, start, more, kind_card, row) in [
        (
            "Make a new wallet",
            Action::CreateWallet,
            Action::CMoreKinds,
            Action::CStep(cstep::KIND),
            Action::CKind as fn(u8) -> Action,
        ),
        (
            "Restore",
            Action::RestoreWallet,
            Action::RMoreKinds,
            Action::RStep(rstep::KIND),
            Action::RKind as fn(u8) -> Action,
        ),
    ] {
        rig.press(Action::Nav(Screen::WalletsStart));
        rig.press(start);
        rig.settle();
        found.extend(walk(rig, what));
        let rows = kind_rows(rig, what, kind_card, more);
        assert!(
            rows.len() > 3,
            "{}: {what} lists its kinds: {rows:?} on {:?}",
            rig.label,
            rig.screen
        );
        for k in rows {
            kind_rows(rig, what, kind_card, more);
            rig.press(row(k));
            rig.settle();
            found.extend(walk(rig, &format!("{what}, kind {k}")));
            if !matches!(rig.screen, Screen::Create | Screen::Restore) {
                rig.press(Action::Nav(if what == "Restore" {
                    Screen::Restore
                } else {
                    Screen::Create
                }));
            }
        }
        leave(rig);
        rig.press(Action::Nav(Screen::Home));
    }
    found
}

/// Each of Tools' tiles opened and walked.
fn tools(rig: &mut Rig) -> Vec<String> {
    let mut found = Vec::new();
    for (i, t) in TILES.iter().enumerate() {
        rig.press(Action::Nav(Screen::Home));
        rig.sheet = None;
        rig.press(Action::Nav(Screen::Tools));
        rig.press(Action::Catalog(i as u8));
        rig.settle();
        found.extend(walk(rig, &format!("Tools, {}", t.name)));
        leave(rig);
    }
    found
}

/// The test vault unlocked: its contents walked, every category and item;
/// a GPG key and a Secure Boot set made; the backup flow of each kind at
/// each of its steps.
fn vaults(on: On, mode: Mode) -> Vec<String> {
    let mut rig = rig(
        on,
        mode,
        vec![("vault.kdbx".into(), testkit::test_vault().unwrap())],
    );
    rig.press(Action::Nav(Screen::Vaults));
    rig.press(Action::Vault(V::Open(0)));
    rig.settle();
    let mut found = walk(&mut rig, "Unlock");
    rig.press(Action::Vault(V::FocusPassphrase));
    crate::common::type_text(&mut rig, testkit::VAULT_PASSPHRASE);
    rig.press(Action::Vault(V::Unlock));
    crate::common::settle(&mut rig, 100);
    if rig.sheet == Some(Sheet::UnlockLoad) {
        rig.settle();
        found.extend(check(&mut rig, "Unlock's Load sheet").0);
        rig.press(Action::Vault(V::LoadTicked));
    }
    assert_eq!(
        rig.vaults().open.len(),
        1,
        "{}: the test vault is open",
        rig.label
    );
    rig.press(Action::Nav(Screen::VaultContents));
    rig.settle();
    found.extend(walk(&mut rig, "Vault contents"));
    for kind in flow_kinds() {
        for s in flow_steps() {
            let c = testkit::compute();
            let mut f = BackupFlow::with_plan(&c, kind, Plan::key_default());
            f.step = s;
            let i = testkit::flow_push(&mut rig, f);
            rig.press(Action::Flow(FlowAction::Show(i)));
            rig.settle();
            found.extend(walk(&mut rig, &format!("backup flow {kind:?} at {s:?}")));
            rig.sheet = None;
        }
    }
    found
}

fn assert_none(found: &[String]) {
    assert!(
        found.is_empty(),
        "cut off or drawn over on the Pi's panel ({}):\n{}",
        found.len(),
        found.join("\n")
    );
}

/// The tests for one panel in one mode.
macro_rules! panel_tests {
    ($name:ident, $on:expr, $mode:expr) => {
        mod $name {
            use super::*;

            #[test]
            fn nothing_is_cut_off_on_any_screen_or_sheet_with_nothing_loaded() {
                let mut rig = rig($on, $mode, Vec::new());
                assert_none(&every_screen(&mut rig, "empty"));
            }

            #[test]
            fn nothing_is_cut_off_on_new_key() {
                let mut rig = rig($on, $mode, Vec::new());
                assert_none(&new_key(&mut rig));
            }

            #[test]
            fn nothing_is_cut_off_on_make_codex32_shares() {
                let mut rig = rig($on, $mode, Vec::new());
                assert_none(&codex32_shares(&mut rig, false));
            }

            #[test]
            fn nothing_is_cut_off_on_make_codex32_shares_for_a_new_wallet() {
                let mut rig = rig($on, $mode, Vec::new());
                assert_none(&codex32_shares(&mut rig, true));
            }

            #[test]
            fn nothing_is_cut_off_on_make_a_new_wallet_or_restore() {
                let mut rig = rig($on, $mode, Vec::new());
                assert_none(&wallet_flows(&mut rig));
            }

            #[test]
            fn nothing_is_cut_off_on_any_tool() {
                let mut rig = rig($on, $mode, Vec::new());
                assert_none(&tools(&mut rig));
            }

            #[test]
            fn nothing_is_cut_off_in_a_vault_or_a_backup_flow() {
                assert_none(&vaults($on, $mode));
            }
        }
    };
}

panel_tests!(on_the_stick_on_mainnet, On::Stick, Mode::Mainnet);
panel_tests!(on_the_stick_in_practice, On::Stick, Mode::Practice);
panel_tests!(in_the_demo_preview_in_practice, On::Preview, Mode::Practice);

#[test]
fn nothing_is_cut_off_with_a_wallet_and_its_keys_on_the_stick() {
    assert_none(&loaded(On::Stick, Mode::Testnet));
}

#[test]
fn nothing_is_cut_off_with_a_wallet_and_its_keys_in_the_demo_preview() {
    assert_none(&loaded(On::Preview, Mode::Testnet));
}

#[test]
fn nothing_is_cut_off_on_any_screen_of_the_online_app() {
    let mut rig = savings_loaded(On::Stick, Mode::Testnet);
    rig.online = true;
    let mut found = every_screen(&mut rig, "online");
    found.extend(every_spend_step(&mut rig));
    assert_none(&found);
}

/// **Save without encryption?** (F129), from a BIP-85 password's
/// **Unprotected** on the secret sheet, on the stick and in the demo
/// preview.
#[test]
fn nothing_is_cut_off_on_save_without_encryption() {
    let mut found = Vec::new();
    for on in [On::Stick, On::Preview] {
        let mut rig = rig(on, Mode::Testnet, Vec::new());
        add_key(&mut rig, testkit::TEST_SEEDS[0].0);
        rig.press(Action::Bip85);
        for a in [Action::PApp(4), Action::PNext, Action::PNext, Action::POut] {
            rig.press(a);
        }
        rig.press(Action::SecretUnprotected);
        rig.settle();
        assert!(testkit::plain_asked(&rig).is_some(), "the hold is asked");
        found.extend(check(&mut rig, "save without encryption").0);
    }
    assert_none(&found);
}

/// The plain save's hold reads whole on the stick's panel, onto as many
/// lines as it needs, and the sheet's other lines and buttons stay clear of
/// it.
#[test]
fn on_the_small_panel_the_plain_save_hold_reads_whole_clear_of_the_sheets_other_buttons() {
    let mut rig = rig(On::Stick, Mode::Testnet, Vec::new());
    add_key(&mut rig, testkit::TEST_SEEDS[0].0);
    rig.press(Action::Bip85);
    for a in [Action::PApp(4), Action::PNext, Action::PNext, Action::POut] {
        rig.press(a);
    }
    let page = rig.drawn_past_sides_and_text().1;
    rig.press(Action::SecretUnprotected);
    rig.settle();
    assert!(testkit::plain_asked(&rig).is_some(), "the hold is asked");
    let texts = rig.drawn_texts();
    assert!(
        texts.iter().all(|t| !t.contains('\u{2026}')),
        "a label is cut: {texts:?}"
    );
    let label = faraday_core::strings::EN.plain_hold;
    let drawn = texts.join(" ");
    for word in label.split(' ') {
        assert!(
            drawn.contains(word),
            "{word:?} of the hold is not drawn: {texts:?}"
        );
    }
    sheet_clear(&mut rig, "the plain save sheet", &page);
}

/// The round after a lock stopped on a codex32 string this power-on does
/// not hold (`docs/CODEX32.md` §9.5): its Make card names the string and
/// offers typing it, and Add a key opens on the codex32 form for it, on
/// the stick and in the demo preview.
#[test]
fn nothing_is_cut_off_where_the_round_types_a_codex32_string() {
    let mut found = Vec::new();
    for on in [On::Stick, On::Preview] {
        let mut rig = rig(on, Mode::Testnet, Vec::new());
        let c = testkit::compute();
        let mut f = BackupFlow::with_plan(&c, FlowKind::Wallet, Plan::key_default());
        f.step = FlowStep::Make;
        f.made = true;
        f.adding = true;
        f.live.c32_wanted = Some(faraday_core::plan::codex32::Wanted {
            key: [0xfa, 0xb6, 0x86, 0x8a],
            k: 2,
            identifier: *b"name",
            length: 48,
            index: b'd',
        });
        let i = testkit::flow_push(&mut rig, f);
        rig.press(Action::Flow(FlowAction::Show(i)));
        rig.settle();
        let (_, texts) = rig.drawn_past_sides_and_text();
        assert!(
            texts
                .iter()
                .any(|(t, _)| t == "Share D of name is not here"),
            "{}: the string is named",
            rig.label
        );
        assert!(
            rig.drawn_actions()
                .contains(&Action::Flow(FlowAction::TypeString))
        );
        found.extend(check(&mut rig, "the round's codex32 string").0);
        rig.press(Action::Flow(FlowAction::TypeString));
        rig.settle();
        assert_eq!(rig.screen, Screen::Entry, "{}", rig.label);
        found.extend(walk(&mut rig, "typing the round's codex32 string"));
    }
    assert_none(&found);
}

/// A taproot multisig wallet whose key path a key not of the wallet
/// spends (F129): its sheet, its card and its spend's check, with the
/// danger line naming the key, are whole on the Pi panel.
#[test]
fn nothing_is_cut_off_where_a_wallet_states_another_key_spends_it_alone() {
    let p = "m/48'/1'/0'/3'";
    let k = testkit::Kit {
        id: "shared",
        name: "Shared",
        descriptor: format!(
            "tr(79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798,multi_a(2,{}/<0;1>/*,{}/<0;1>/*,{}/<0;1>/*))",
            testkit::key(0, p),
            testkit::key(1, p),
            testkit::key(2, p)
        ),
        cosigner: None,
        fee: 1_820,
    };
    let unsigned = testkit::unsigned(&k).unwrap();
    let mut rig = rig(
        On::Stick,
        Mode::Testnet,
        vec![
            ("shared-wallet.txt".to_string(), k.descriptor.into_bytes()),
            ("shared-unsigned.psbt".to_string(), unsigned.to_bytes()),
        ],
    );
    let at = |app: &Faraday, name: &str| app.inbox().iter().position(|i| i.name == name).unwrap();
    let w = at(&rig, "shared-wallet.txt");
    rig.press(Action::LoadWallet(w));
    rig.settle();
    assert_eq!(rig.sheet, Some(Sheet::KeyPath));
    let mut found = check(&mut rig, "the key path sheet").0;
    rig.press(Action::KeyPathHold);
    add_key(&mut rig, testkit::TEST_SEEDS[0].0);
    add_key(&mut rig, testkit::TEST_SEEDS[1].0);
    rig.press(Action::Nav(Screen::Wallets));
    rig.settle();
    found.extend(check(&mut rig, "the wallet card").0);
    let psbt = at(&rig, "shared-unsigned.psbt");
    rig.press(Action::StartSpend(psbt));
    assert!(rig.spend().is_some());
    rig.press(Action::Step(step::TRANSACTION));
    rig.settle();
    found.extend(check(&mut rig, "the spend's check").0);
    assert_none(&found);
}

/// A spend from a wallet not registered here, its PSBT carrying the
/// cosigners' extended keys: the spend's check with the blocked line and
/// **Register this wallet**, the sheet with the wallet's shape, its three
/// cosigners, the name and the hold, the sheet refusing a PSBT whose keys
/// do not give its input, and the spend once registered, are
/// whole on the Pi panel.
#[test]
fn nothing_is_cut_off_where_a_wallet_is_registered_from_its_spend() {
    let k = kit("savings");
    let unsigned = testkit::unsigned_with_xpubs(&k).unwrap();
    // The same spend whose third key's entry is another account's key.
    let c = testkit::compute();
    let policy = faraday_bip::policy::WalletPolicy::parse_any(&c, &k.descriptor).unwrap();
    let other = faraday_bip::policy::WalletPolicy::parse_any(
        &c,
        &format!("wpkh({}/<0;1>/*)", testkit::key(2, "m/48'/1'/1'/2'")),
    )
    .unwrap();
    let mut wrong = unsigned.clone();
    let xpubs = &mut wrong.inner_mut().xpub;
    let origin = xpubs.remove(policy.keys()[2].xpub()).unwrap();
    xpubs.insert(*other.keys()[0].xpub(), origin);
    let mut rig = rig(
        On::Stick,
        Mode::Testnet,
        vec![
            ("savings-unsigned.psbt".to_string(), unsigned.to_bytes()),
            ("savings-wrong.psbt".to_string(), wrong.to_bytes()),
        ],
    );
    add_key(&mut rig, testkit::TEST_SEEDS[0].0);
    rig.press(Action::StartSpend(1));
    rig.press(Action::RegisterOpen);
    rig.press(Action::RegisterHold);
    rig.settle();
    assert!(rig.session.wallets.is_empty());
    let mut found = check(&mut rig, "the register sheet, refused").0;
    rig.press(Action::RegisterCancel);
    rig.press(Action::StartSpend(0));
    assert!(rig.spend().is_some());
    if rig.spend().unwrap().open != Some(step::TRANSACTION) {
        rig.press(Action::Step(step::TRANSACTION));
    }
    rig.settle();
    found.extend(check(&mut rig, "the spend's check, blocked").0);
    rig.press(Action::RegisterOpen);
    rig.settle();
    assert_eq!(rig.sheet, Some(Sheet::Register));
    found.extend(check(&mut rig, "the register sheet").0);
    rig.press(Action::RegisterField);
    rig.settle();
    found.extend(check(&mut rig, "the register sheet, its name typed").0);
    rig.press(Action::RegisterHold);
    rig.settle();
    assert_eq!(rig.session.wallets.len(), 1);
    found.extend(check(&mut rig, "the spend, registered").0);
    assert_none(&found);
}

/// A tap, as a finger gives it, at the middle of what `a` is pressed on.
fn tap(rig: &mut Rig, a: Action) {
    let (x, y) = rig
        .where_offered(a)
        .unwrap_or_else(|| panic!("{a:?} is not on the panel"));
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        rig.event(Event::Touch { x, y, phase });
    }
    rig.settle();
    let _ = rig.frame();
}

/// New key from Make a new wallet's single key, its dice rolled: Key is
/// next.
fn new_key_from_create(rig: &mut Rig) {
    rig.press(Action::CreateWallet);
    rig.press(Action::CKind(0));
    rig.press(Action::KeyGen(Some(0)));
    rig.press(Action::KStart(faraday_core::keygen::Start::Make.index()));
    rig.press(Action::KWords(12));
    rig.press(Action::KNext);
    let mut x: u32 = 0x2545_f491;
    for _ in 0..128 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        rig.press(Action::KRoll((x % 6) as u8 + 1));
    }
    rig.settle();
}

/// The ⋮ on the bar now, and what its menu holds.
fn overflow_on_the_bar(rig: &Rig) -> Option<Overflow> {
    rig.drawn_actions().into_iter().find_map(|a| match a {
        Action::NavMenu(MenuPress::Open(Some(o))) => Some(o),
        _ => None,
    })
}

#[test]
fn on_the_pi_a_flows_bar_too_full_for_its_question_mark_offers_learn_in_its_overflow_menu() {
    for mode in [Mode::Testnet, Mode::Practice] {
        let mut rig = rig(On::Stick, mode, Vec::new());
        // New key's passphrase, from Make a new wallet, typed on the
        // panel's keyboard: Cancel, the keyboard button and the mode pill
        // leave no room for the ?.
        new_key_from_create(&mut rig);
        rig.press(Action::KNext);
        rig.settle();
        rig.press(Action::KPassField(0));
        rig.settle();
        let _ = rig.frame();
        let label = format!("{mode:?}, New key's passphrase");
        assert!(
            rig.offers(Action::Osk(OskPress::Char('q'))),
            "{label}: the keyboard is up"
        );
        let found = check(&mut rig, &label).0;
        assert_none(&found);
        assert!(!rig.offers(Action::Learn), "{label}: no ? on the bar");
        for kept in [Action::CancelFlow, Action::ModeSheet] {
            assert!(rig.offers(kept), "{label}: {kept:?} stays on the bar");
        }
        let held = overflow_on_the_bar(&rig).unwrap_or_else(|| panic!("{label}: a ⋮"));
        assert!(held.help, "{label}: the ⋮ holds Learn");
        // The ⋮ opens the menu; its Learn opens the page's Learn.
        tap(&mut rig, Action::NavMenu(MenuPress::Open(Some(held))));
        assert_eq!(rig.sheet, Some(Sheet::NavMenu(Some(held))), "{label}");
        assert!(rig.drawn_texts().iter().any(|t| t == "Learn"), "{label}");
        assert_none(&check(&mut rig, &format!("{label}, its overflow menu")).0);
        tap(&mut rig, Action::NavMenu(MenuPress::Tool(BarTool::Help)));
        assert_eq!(
            rig.sheet,
            Some(Sheet::Learn),
            "{label}: Learn opens the Learn sheet"
        );
        assert_eq!(rig.screen, Screen::KeyGen, "{label}: on the same page");
        // The keyboard button went in with it: the menu's row brings the
        // keyboard up or puts it away, as the button does, and says which.
        assert!(held.keyboard, "{label}: the ⋮ holds the keyboard");
        rig.sheet = None;
        rig.settle();
        let _ = rig.frame();
        for _ in 0..2 {
            let up = rig.offers(Action::Osk(OskPress::Char('q')));
            let held = overflow_on_the_bar(&rig).unwrap_or_else(|| panic!("{label}: a ⋮"));
            tap(&mut rig, Action::NavMenu(MenuPress::Open(Some(held))));
            let row = if up { "Hide keyboard" } else { "Show keyboard" };
            assert!(rig.drawn_texts().iter().any(|t| t == row), "{label}: {row}");
            tap(
                &mut rig,
                Action::NavMenu(MenuPress::Tool(BarTool::Keyboard)),
            );
            assert_eq!(rig.sheet, None, "{label}");
            assert_eq!(
                rig.offers(Action::Osk(OskPress::Char('q'))),
                !up,
                "{label}: {row}"
            );
        }
        leave(&mut rig);
    }
}

#[test]
fn where_the_page_offers_cryptography_the_overflow_menu_holds_it_with_the_question_mark() {
    let mut rig = rig(On::Stick, Mode::Testnet, Vec::new());
    new_key_from_create(&mut rig);
    let _ = rig.frame();
    assert!(rig.offers(Action::CancelFlow));
    let held = overflow_on_the_bar(&rig).expect("a ⋮");
    assert!(held.how && held.help, "Cryptography first, with the ?");
}

/// The name in the bar of the step flow on show, as drawn: its text and
/// its height in dp. The bar's other texts (the ?, Cancel, the mode pill,
/// where Back goes) are not it.
fn flow_bar_name(rig: &mut Rig) -> Option<(String, f32)> {
    let _ = rig.frame();
    let f = f32::from(rig.w) / rig.size().0;
    let others = ["?", "Cancel", "Testnet", "Practice", "Start", "Home"];
    rig.drawn_past_sides_and_text()
        .1
        .into_iter()
        .filter(|(t, r)| (r.1 as f32) < 32.0 * f && !others.contains(&t.as_str()))
        .map(|(t, r)| (t, r.3 as f32 / f))
        .next()
}

/// Off mainnet the mode pill stands on the bar's second line beside where
/// Back goes, so a page's name keeps the whole first line: it is drawn
/// whole, and the pill is drawn whole and can be pressed.
#[test]
fn on_the_pi_the_mode_pill_leaves_a_pages_name_whole_on_the_bars_first_line() {
    // F223: off mainnet the pill stands on the bar's second line.
    for (mode, pill) in [(Mode::Testnet, "Testnet"), (Mode::Practice, "Practice")] {
        {
            let (screen, name) = (Screen::Convert, "Convert a seed");
            let mut rig = rig(On::Stick, mode, Vec::new());
            rig.press(Action::Convert(faraday_core::convert::ConvertAction::Open));
            rig.settle();
            let _ = rig.frame();
            let f = f32::from(rig.w) / rig.size().0;
            let texts = rig.drawn_past_sides_and_text().1;
            let label = format!("{mode:?}, {screen:?}");
            let drawn = rig.drawn_texts();
            assert!(
                drawn.iter().any(|t| t == name),
                "{label}: the name {name:?} is whole, drawn: {drawn:?}"
            );
            let (_, r) = texts
                .iter()
                .find(|(t, _)| t == pill)
                .unwrap_or_else(|| panic!("{label}: the pill {pill:?} is drawn whole"));
            assert!(
                (r.1 as f32) >= 32.0 * f,
                "{label}: the pill is on the second line"
            );
            assert!(
                rig.offers(Action::ModeSheet),
                "{label}: the pill is pressed"
            );
        }
    }
}

/// Sign a transaction with nothing loaded names itself on the bar beside
/// Cancel: whole words or none, and never under Cancel.
#[test]
fn on_the_pi_sign_a_transaction_with_nothing_loaded_names_itself_beside_cancel() {
    const NAME: &str = "Sign a transaction";
    for mode in [Mode::Mainnet, Mode::Testnet] {
        let mut rig = rig(On::Stick, mode, Vec::new());
        open(&mut rig, Screen::Spend);
        let (_, texts) = rig.drawn_past_sides_and_text();
        let cancel = texts
            .iter()
            .find(|(t, _)| t == "Cancel")
            .unwrap_or_else(|| panic!("{mode:?}: Cancel is on the bar"));
        // Where no whole word fits beside the tools the bar names
        // nothing.
        let Some(name) = texts
            .iter()
            .find(|(t, _)| NAME.starts_with(t.trim_end_matches('…')) && t.len() > 1)
        else {
            continue;
        };
        let words = name.0.trim_end_matches('…');
        assert!(
            words == NAME || NAME[words.len()..].starts_with(' '),
            "{mode:?}: {:?} cuts a word",
            name.0
        );
        assert!(
            name.1.0 + name.1.2 <= cancel.1.0,
            "{mode:?}: {:?} runs under Cancel",
            name.0
        );
    }
}

/// The Learn sheet's tab of the page on show reads whole where the panel
/// has room for one tab only: its words wrap inside the tab, none runs
/// under an arrow or Done.
#[test]
fn on_the_pi_the_learn_sheets_tab_name_is_read_whole_between_the_arrows() {
    let mut rig = rig(On::Stick, Mode::Mainnet, Vec::new());
    open(&mut rig, Screen::KeyGen);
    rig.press(Action::Learn);
    let _ = rig.frame();
    let pages = rig.learn().pages.len();
    assert!(pages > 2, "the sheet has tabs to move between");
    for p in 0..pages {
        rig.press(Action::LearnPage(p as u8));
        let (_, texts) = rig.drawn_past_sides_and_text();
        let title = faraday_core::learn::tab_label(rig.learn().pages[p]);
        let words: Vec<&str> = title.split(' ').collect();
        let others: Vec<_> = (0..pages)
            .filter(|&j| j != p)
            .filter_map(|j| rig.hit_box(Action::LearnPage(j as u8)))
            .map(|b| [b.0, b.1, b.2, b.3].map(i32::from))
            .collect();
        assert!(!others.is_empty(), "{title}: an arrow beside its tab");
        let in_row = |r: &(i32, i32, i32, i32)| {
            others
                .iter()
                .any(|b| r.1 >= b[1] && r.1 + r.3 <= b[1] + b[3])
        };
        let tab: Vec<_> = texts
            .iter()
            .filter(|(t, r)| in_row(r) && t.split(' ').all(|w| words.contains(&w)))
            .collect();
        assert!(!tab.is_empty(), "{title}: its tab is drawn");
        let own = rig.hit_box(Action::LearnPage(p as u8)).expect("its tab");
        let mut drawn = String::new();
        for (t, r) in &tab {
            // A label cut by its tab's edge is drawn flush against it:
            // a whole one stands clear of both.
            let clear = 8;
            assert!(
                r.0 >= i32::from(own.0) + clear
                    && r.0 + r.2 <= i32::from(own.0) + i32::from(own.2) - clear,
                "{title}: {t:?} touches the edge of its tab"
            );
            drawn.push_str(t);
            drawn.push(' ');
            for b in &others {
                let across = r.0 < b[0] + b[2] && b[0] < r.0 + r.2;
                let level = r.1 < b[1] + b[3] && b[1] < r.1 + r.3;
                assert!(
                    !(across && level),
                    "{title}: {t:?} runs under another tab or arrow"
                );
            }
        }
        assert_eq!(
            drawn.trim(),
            title,
            "{title}: every word of the tab is drawn whole"
        );
    }
}

#[test]
fn on_the_small_panel_both_segments_of_a_vaults_choice_are_on_the_page_and_the_second_can_be_pressed()
 {
    use crate::backup_flow_wallet::offered_down;
    use crate::common::{create_single_key, keep_time, process, texts};
    use faraday_core::board::{BoardPress, Thing, Zone};
    use faraday_core::{Action, cstep};
    use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    keep_time(&mut app);
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 286,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    create_single_key(&mut app);
    app.press(Action::CVaults(true));
    app.press(Action::CNext(cstep::WHERE));
    let _ = app.frame();
    let second = Action::Board(BoardPress::Form(Zone::List, Thing::Vault(0), 1));
    let first = Action::Board(BoardPress::Form(Zone::List, Thing::Vault(0), 0));
    assert!(offered_down(&mut app, first), "the choice is never offered");
    for a in [first, second] {
        let (x, _, w, _) = app
            .hit_box(a)
            .unwrap_or_else(|| panic!("{a:?} is not offered"));
        assert!(
            u32::from(x) + u32::from(w) <= 480,
            "{a:?} runs off the window: x {x}, w {w}"
        );
    }
    assert!(texts(&mut app).iter().any(|t| t == "Existing"));
    app.press(second);
    let _ = app.frame();
    assert!(
        texts(&mut app).iter().any(|t| t.starts_with("Existing")),
        "the second segment was not taken"
    );
}

#[test]
fn on_the_pi_a_step_flows_bar_names_it_whole_smaller_where_its_tools_leave_less_room() {
    let name = |mode: Mode| {
        let mut rig = rig(On::Stick, mode, Vec::new());
        rig.press(Action::KeyGen(None));
        rig.settle();
        flow_bar_name(&mut rig).expect("New key's bar names it")
    };
    // On Mainnet the ? alone stands beside it: the bar's own size.
    let (text, roomy) = name(Mode::Mainnet);
    assert_eq!(text, "Add a key");
    // On Testnet the mode pill stands on the bar's second line, so the
    // name has the first line to itself: whole, at the bar's own size.
    let (text, pi) = name(Mode::Testnet);
    assert_eq!(text, "Add a key", "whole beside the ?");
    assert_eq!(pi, roomy, "the name's size is the same with the pill");
    // Create vault's name beside Cancel and the ?: whole words or none,
    // never a piece of a word.
    let mut rig = rig(On::Stick, Mode::Mainnet, Vec::new());
    rig.press(Action::Vault(V::Create));
    rig.settle();
    let shown = flow_bar_name(&mut rig).filter(|(t, _)| t != "Size");
    if let Some((text, _)) = shown {
        let words = text.trim_end_matches('…');
        assert!(
            "Create a vault".starts_with(words) && !words.is_empty(),
            "{text:?}"
        );
        assert!(
            text == "Create a vault" || "Create a vault"[words.len()..].starts_with(' '),
            "{text:?} cuts a word"
        );
    }
}

#[test]
fn a_size_keeps_its_number_and_its_unit_together() {
    for bytes in [900_u64, 9_000, 9_000_000, 9_000_000_000] {
        let text = faraday_core::vault::file_text(bytes as usize);
        assert!(!text.contains(' '), "{text:?} can break between its parts");
    }
    assert!(!faraday_core::vault::mib_text(14_950).contains(' '));
}

/// What a page drawn now, scrolled to each place, shows cut off or drawn
/// over other text, as one message.
fn drawn_clear(rig: &mut Rig, what: &str) {
    let found = check(rig, what).0;
    assert!(found.is_empty(), "{}", found.join("\n"));
}

#[test]
fn on_the_small_panel_an_existing_vaults_line_on_the_layout_step_is_clear_of_the_choice_under_it() {
    let mut rig = rig(
        On::Stick,
        Mode::Mainnet,
        vec![("vault.kdbx".into(), testkit::test_vault().unwrap())],
    );
    let c = testkit::compute();
    let i = testkit::flow_push(&mut rig, BackupFlow::new(&c, FlowKind::Gpg));
    rig.press(Action::Flow(FlowAction::Show(i)));
    rig.press(Action::Flow(FlowAction::AddPlace));
    rig.press(Action::Flow(FlowAction::PlaceVault(1, 1)));
    rig.press(Action::Flow(FlowAction::PlaceVault(2, 1)));
    rig.press(Action::Flow(FlowAction::VaultExisting(0)));
    rig.settle();
    drawn_clear(&mut rig, "the layout step with an existing vault");
    let under = text_under_control(&mut rig, Action::Flow(FlowAction::VaultExisting(0)));
    assert!(under.is_empty(), "{}", under.join("\n"));
    // Down at the vaults' lines, where the choice stands under them.
    let (x, y) = (rig.w / 2, rig.h / 2);
    for _ in 0..6 {
        rig.event(Event::Scroll { x, y, dy: 160 });
    }
    rig.settle();
    let texts = rig.drawn_texts();
    assert!(
        texts.iter().any(|t| t.contains("an existing vault")),
        "not the layout step: {texts:?}"
    );
}

/// The lines of text that run under the touch target of `action`, at each
/// place the page scrolls to: a line of text is inside the control (its
/// label) or clear of it. `action` must be offered somewhere along the page.
fn text_under_control(rig: &mut Rig, action: Action) -> Vec<String> {
    let (x, y) = (rig.w / 2, rig.h / 2);
    let mut found = Vec::new();
    let mut offered = false;
    let mut last = None;
    for _ in 0..40 {
        let texts = rig.drawn_past_sides_and_text().1;
        if let Some(b) = rig.hit_box(action) {
            offered = true;
            let b = (
                i32::from(b.0),
                i32::from(b.1),
                i32::from(b.2),
                i32::from(b.3),
            );
            for (t, r) in &texts {
                let w = (b.0 + b.2).min(r.0 + r.2) - b.0.max(r.0);
                let h = (b.1 + b.3).min(r.1 + r.3) - b.1.max(r.1);
                let inside =
                    r.0 >= b.0 && r.0 + r.2 <= b.0 + b.2 && r.1 >= b.1 && r.1 + r.3 <= b.1 + b.3;
                if w > 1 && h > 1 && !inside {
                    found.push(format!("text {t:?} runs under {action:?} by {w} x {h} px"));
                }
            }
        }
        if last.as_ref() == Some(&texts) {
            break;
        }
        last = Some(texts);
        rig.event(Event::Scroll { x, y, dy: 160 });
        rig.settle();
    }
    for _ in 0..40 {
        rig.event(Event::Scroll { x, y, dy: -160 });
    }
    rig.settle();
    assert!(offered, "{action:?} is never offered");
    found.dedup();
    found
}

/// Whether a text `want` holds is drawn anywhere along the page, from its
/// top to its foot; the page is left at its top.
fn scrolled_to_see(rig: &mut Rig, want: impl Fn(&str) -> bool) -> bool {
    let (x, y) = (rig.w / 2, rig.h / 2);
    let mut seen = false;
    let mut last = None;
    for _ in 0..40 {
        let texts = rig.drawn_texts();
        seen |= texts.iter().any(|t| want(t));
        if last.as_ref() == Some(&texts) {
            break;
        }
        last = Some(texts);
        rig.event(Event::Scroll { x, y, dy: 160 });
        rig.settle();
    }
    for _ in 0..40 {
        rig.event(Event::Scroll { x, y, dy: -160 });
    }
    rig.settle();
    seen
}

/// What the sheet drawn now, scrolled to each place, shows cut off at its
/// sides or drawn over other text.
///
/// `page` is what the page under the sheet draws: its text is not the
/// sheet's.
fn sheet_clear(rig: &mut Rig, what: &str, page: &[DrawnText]) {
    let (x, y) = (rig.w / 2, rig.h / 2);
    let dy = (rig.h / 4) as i16;
    let mut found: Vec<String> = Vec::new();
    let mut last = None;
    for _ in 0..40 {
        let (past, texts) = rig.drawn_past_sides_and_text();
        found.extend(past);
        let own: Vec<DrawnText> = texts
            .iter()
            .filter(|t| !page.contains(t))
            .cloned()
            .collect();
        found.extend(drawn_over(&own, None));
        if last.as_ref() == Some(&texts) {
            break;
        }
        last = Some(texts);
        rig.event(Event::Scroll { x, y, dy });
        rig.settle();
    }
    found.dedup();
    assert!(found.is_empty(), "{what}: {}", found.join("\n"));
}

/// The test vault unlocked on the stick's panel.
fn vault_open() -> Rig {
    let mut rig = rig(
        On::Stick,
        Mode::Mainnet,
        vec![("vault.kdbx".into(), testkit::test_vault().unwrap())],
    );
    rig.press(Action::Nav(Screen::Vaults));
    rig.press(Action::Vault(V::Open(0)));
    rig.press(Action::Vault(V::FocusPassphrase));
    crate::common::type_text(&mut rig, testkit::VAULT_PASSPHRASE);
    rig.press(Action::Vault(V::Unlock));
    crate::common::settle(&mut rig, 100);
    if rig.sheet == Some(Sheet::UnlockLoad) {
        rig.press(Action::Vault(V::LoadTicked));
    }
    rig.press(Action::Nav(Screen::VaultContents));
    rig.settle();
    rig
}

#[test]
fn on_the_small_panel_the_vault_memory_sheet_keeps_each_size_clear_of_its_label() {
    let mut rig = vault_open();
    rig.storage(StorageEvent::Memory {
        available_mib: 15_000,
    });
    let page = rig.drawn_past_sides_and_text().1;
    rig.press(Action::Vault(V::MemoryAsk));
    rig.settle();
    assert!(
        rig.drawn_texts()
            .iter()
            .any(|t| t.contains("MiB") || t.contains("GiB")),
        "no memory sizes on the sheet: {:?}",
        rig.drawn_texts()
    );
    sheet_clear(&mut rig, "the memory sheet", &page);
}

#[test]
fn on_the_small_panel_the_lock_sheets_randomness_line_is_clear_of_its_button() {
    let mut rig = vault_open();
    // The vault holds a change the lock will seal.
    let id = rig.vaults().open[0].id;
    testkit::vault_changed(&mut rig, id);
    let page = rig.drawn_past_sides_and_text().1;
    rig.sheet = Some(Sheet::LockAsk);
    rig.settle();
    // Down to the foot of the sheet, where the Randomness line is.
    sheet_clear(&mut rig, "the lock sheet", &page);
    let texts = rig.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Randomness"),
        "the Randomness line is cut short: {texts:?}"
    );
}

#[test]
fn on_the_small_panel_restore_names_a_key_that_is_not_here_whole_beside_change() {
    use faraday_core::rstep;
    let k = kit("savings");
    let mut rig = rig(
        On::Stick,
        Mode::Testnet,
        vec![("savings-wallet.txt".to_string(), k.descriptor.into_bytes())],
    );
    rig.press(Action::RestoreWallet);
    rig.press(Action::RKind(4));
    rig.press(Action::RNext(rstep::KIND));
    rig.press(Action::RUse(0));
    rig.press(Action::RVaults(false));
    rig.press(Action::RNext(rstep::WHERE));
    let w = rig
        .restore()
        .and_then(|r| r.wallet)
        .expect("the description made a wallet");
    let empty: Vec<u8> = rig
        .session
        .slots(&rig.session.wallets[w])
        .iter()
        .enumerate()
        .filter(|(_, s)| s.held_by.is_none())
        .map(|(k, _)| k as u8)
        .collect();
    assert!(!empty.is_empty());
    for k in empty {
        rig.press(Action::RAbsent(k));
    }
    rig.settle();
    let texts = rig.drawn_texts();
    assert!(
        texts.iter().any(|t| t == "Not here"),
        "Not here is not shown whole: {texts:?}"
    );
    drawn_clear(&mut rig, "restore's keys step");
}

#[test]
fn on_the_small_panel_a_typed_messages_count_and_continue_do_not_share_a_line() {
    let mut rig = rig(On::Stick, Mode::Testnet, Vec::new());
    let c = testkit::compute();
    let kit = kit("spending");
    rig.session
        .keys
        .add_words(
            &c,
            &testkit::test_words(testkit::TEST_SEEDS[0].0),
            "Bacon",
            None,
        )
        .expect("the key");
    rig.session
        .add_wallet(
            &c,
            kit.name,
            &kit.descriptor,
            "test",
            faraday_core::wallet::Origin::Named,
        )
        .expect("the wallet");
    rig.press(Action::SignMessage);
    rig.press(Action::MWallet(0));
    rig.press(Action::MNext(0));
    if !testkit::message(&rig).unwrap().typing {
        rig.press(Action::MType);
    }
    testkit::message_text(&mut rig, "I control this address.");
    rig.settle();
    assert!(
        scrolled_to_see(&mut rig, |t| t == "23 characters"),
        "the count of characters is not on the page: {:?}",
        rig.drawn_texts()
    );
    drawn_clear(&mut rig, "the message's text step");
}

#[test]
fn on_the_small_panel_a_found_vanity_address_offers_both_buttons_clear_of_each_other() {
    use faraday_core::vanity::{VanityAction as Va, vstep};
    let mut rig = rig(On::Stick, Mode::Testnet, Vec::new());
    add_key(&mut rig, testkit::TEST_SEEDS[0].0);
    rig.press(Action::Vanity(Va::Open));
    let first = rig.session.keys[0].fingerprint.0;
    rig.press(Action::Vanity(Va::Key(first)));
    rig.press(Action::Vanity(Va::Next(vstep::DIAL)));
    rig.press(Action::Vanity(Va::Next(vstep::SCRIPT)));
    for c in "qq".chars() {
        rig.event(Event::Key(faraday_shell_api::Key::Char(c)));
    }
    rig.press(Action::Vanity(Va::Next(vstep::PREFIX)));
    rig.press(Action::Vanity(Va::Start));
    for t in 1..5000u64 {
        rig.event(Event::Tick { now_ms: t * 100 });
        if rig.vanity().is_some_and(|v| v.grind.find.is_some()) {
            break;
        }
    }
    rig.settle();
    let (open, again) = (Action::Vanity(Va::Use), Action::Vanity(Va::Start));
    let mut shown = false;
    for _ in 0..20 {
        let _ = rig.drawn_texts();
        if rig.drawn_actions().contains(&open) && rig.drawn_actions().contains(&again) {
            shown = true;
            break;
        }
        let (x, y) = (rig.w / 2, rig.h / 2);
        rig.event(Event::Scroll { x, y, dy: 160 });
        rig.settle();
    }
    assert!(
        shown,
        "both buttons are never on the page: {:?} {:?}",
        rig.drawn_texts(),
        rig.screen
    );
    let (a, b) = (rig.hit_box(open).unwrap(), rig.hit_box(again).unwrap());
    let apart = a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1;
    assert!(apart, "the buttons overlap: {a:?} {b:?}");
    // Neither crosses the page's margin at the right.
    let right = u32::from(rig.w) - u32::from(a.0);
    for (name, c) in [("Open this wallet", a), ("Search again", b)] {
        assert!(
            u32::from(c.0) + u32::from(c.2) <= right,
            "{name} crosses the page's margin: {c:?}"
        );
    }
}

#[test]
fn on_the_small_panel_the_scan_sheets_camera_buttons_leave_room_above_cancel() {
    let mut rig = rig(On::Stick, Mode::Mainnet, Vec::new());
    rig.storage(StorageEvent::Cameras(vec![
        ("/dev/video0".into(), "Integrated Camera".into()),
        ("/dev/video2".into(), "USB Camera".into()),
    ]));
    testkit::open_scan(&mut rig, ScanPurpose::Seed);
    rig.settle();
    let _ = rig.drawn_texts();
    let c = rig.hit_box(Action::Cancel).expect("Cancel is not offered");
    for i in 0..2u8 {
        let b = rig
            .hit_box(Action::ScanCamera(i))
            .expect("a camera button is not offered");
        assert!(
            b.1 + b.3 <= c.1,
            "camera button {i} {b:?} runs into Cancel {c:?}"
        );
    }
}

/// Whether the ink at `r` lies inside `b`, both (x, y, w, h) in pixels.
fn inside(r: (i32, i32, i32, i32), b: (u16, u16, u16, u16)) -> bool {
    let (x, y, w, h) = (
        i32::from(b.0),
        i32::from(b.1),
        i32::from(b.2),
        i32::from(b.3),
    );
    r.0 >= x && r.1 >= y && r.0 + r.2 <= x + w && r.1 + r.3 <= y + h
}

/// What is wrong with the button `a`, a chip chosen when `chosen`: its
/// label (whole) and, chosen, its check are each inside its box.
fn chip_faults(rig: &mut Rig, a: Action, label: &str, chosen: bool) -> Vec<String> {
    use faraday_draw::canvas::InkKind;
    use faraday_draw::widgets::Icon;
    // Down the page to where the chip is.
    for _ in 0..20 {
        let _ = rig.frame();
        if rig
            .hit_box(a)
            .is_some_and(|b| u32::from(b.1) + u32::from(b.3) < 600)
        {
            break;
        }
        rig.event(Event::Scroll {
            x: 240,
            y: 320,
            dy: 100,
        });
    }
    let Some(b) = rig.hit_box(a) else {
        return vec![format!("{label:?} is not offered")];
    };
    let centre = |r: (i32, i32, i32, i32)| (r.0 + r.2 / 2, r.1 + r.3 / 2);
    let on_it = |r: (i32, i32, i32, i32)| {
        let (cx, cy) = centre(r);
        cx >= i32::from(b.0)
            && cx < i32::from(b.0) + i32::from(b.2)
            && cy >= i32::from(b.1)
            && cy < i32::from(b.1) + i32::from(b.3)
    };
    let mut out = Vec::new();
    let (mut text, mut ticks) = (0, 0);
    for ink in rig.drawn_ink() {
        let r = (ink.rect.x, ink.rect.y, ink.rect.w, ink.rect.h);
        match ink.kind {
            InkKind::Text { text: t, .. } if t == label && on_it(r) => {
                text += 1;
                if !inside(r, b) {
                    out.push(format!("{label:?} runs outside its chip {b:?}: {r:?}"));
                }
            }
            InkKind::Icon(Icon::Done) if on_it(r) => {
                ticks += 1;
                if !inside(r, b) {
                    out.push(format!(
                        "{label:?}'s check runs outside its chip {b:?}: {r:?}"
                    ));
                }
            }
            _ => {}
        }
    }
    if text == 0 {
        out.push(format!(
            "{label:?} is not drawn in its chip {b:?}: {:?}",
            rig.drawn_text_boxes()
        ));
    }
    if ticks != usize::from(chosen) {
        out.push(format!("{label:?} has {ticks} checks, chosen {chosen}"));
    }
    out
}

/// The panel on Add randomness with `source` chosen.
fn add_randomness(source: faraday_core::fresh::OwnSource) -> Rig {
    use faraday_core::fresh::page::Act;
    let mut rig = rig(On::Stick, Mode::Mainnet, Vec::new());
    rig.set_pointer_keys_offered(true);
    rig.press(Action::Nav(Screen::Settings));
    crate::common::settings_section(&mut rig, "Randomness");
    rig.press(Action::Own(Act::Open));
    rig.press(Action::Own(Act::Source(source)));
    let _ = rig.frame();
    rig
}

#[test]
fn on_the_pi_a_chosen_card_rank_keeps_its_check_clear_of_the_ranks_beside_it() {
    use faraday_core::fresh::OwnSource;
    use faraday_core::fresh::page::Act;
    let mut rig = add_randomness(OwnSource::Cards);
    for r in 0..13u8 {
        rig.press(Action::Own(Act::Rank(r)));
        let label = faraday_entropy::RANKS[usize::from(r)].to_string();
        let found = chip_faults(&mut rig, Action::Own(Act::Rank(r)), &label, true);
        assert!(found.is_empty(), "rank {label}: {found:?}");
        let mine = rig.hit_box(Action::Own(Act::Rank(r))).unwrap();
        for o in (0..13u8).filter(|&o| o != r) {
            let Some(b) = rig.hit_box(Action::Own(Act::Rank(o))) else {
                continue;
            };
            let apart = mine.0 + mine.2 <= b.0
                || b.0 + b.2 <= mine.0
                || mine.1 + mine.3 <= b.1
                || b.1 + b.3 <= mine.1;
            assert!(apart, "rank {r} and {o} overlap: {mine:?}, {b:?}");
        }
    }
}

#[test]
fn on_the_pi_chosen_pointer_and_keys_keeps_its_check_inside_its_chip() {
    use faraday_core::fresh::OwnSource;
    use faraday_core::fresh::page::Act;
    let mut rig = add_randomness(OwnSource::PointerKeys);
    let a = Action::Own(Act::Source(OwnSource::PointerKeys));
    let found = chip_faults(&mut rig, a, "Pointer and keys", true);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn on_the_pi_change_passphrase_is_inside_its_button() {
    let mut rig = rig(
        On::Stick,
        Mode::Mainnet,
        vec![("vault.kdbx".into(), testkit::test_vault().unwrap())],
    );
    rig.press(Action::Nav(Screen::Vaults));
    rig.press(Action::Vault(V::Open(0)));
    rig.settle();
    rig.press(Action::Vault(V::FocusPassphrase));
    crate::common::type_text(&mut rig, testkit::VAULT_PASSPHRASE);
    rig.press(Action::Vault(V::Unlock));
    crate::common::settle(&mut rig, 100);
    if rig.sheet == Some(Sheet::UnlockLoad) {
        rig.settle();
        rig.press(Action::Vault(V::LoadTicked));
    }
    rig.press(Action::Nav(Screen::VaultContents));
    rig.settle();
    let found = chip_faults(
        &mut rig,
        Action::Vault(V::ChangePassphrase),
        "Change passphrase",
        false,
    );
    assert!(found.is_empty(), "{found:?}");
}

/// The strings the frame draws, once the page is scrolled down far enough
/// to draw every one of `want`.
fn scrolled_to(rig: &mut Rig, want: &[&str]) -> Vec<DrawnText> {
    for _ in 0..20 {
        let _ = rig.frame();
        let drawn = rig.drawn_text_boxes();
        if want.iter().all(|w| drawn.iter().any(|(t, _)| t == w)) {
            return drawn;
        }
        rig.event(Event::Scroll {
            x: 240,
            y: 320,
            dy: 100,
        });
    }
    panic!("{want:?} never shown together");
}

#[test]
fn on_the_pi_unlock_on_a_wallet_is_lined_up_with_the_buttons_around_it() {
    let k = kit("savings");
    let mut rig = rig(
        On::Stick,
        Mode::Testnet,
        vec![
            ("vault.kdbx".into(), testkit::test_vault().unwrap()),
            ("savings-wallet.txt".to_string(), k.descriptor.into_bytes()),
        ],
    );
    let w = rig
        .inbox()
        .iter()
        .position(|i| i.name == "savings-wallet.txt")
        .unwrap();
    rig.press(Action::LoadWallet(w));
    rig.press(Action::Nav(Screen::Wallets));
    rig.settle();
    let drawn = scrolled_to(&mut rig, &["Unlock", "Spend from this wallet"]);
    let middle = |name: &str| {
        let r = drawn.iter().find(|(t, _)| t == name).unwrap().1;
        r.0 + r.2 / 2
    };
    let (unlock, spend) = (middle("Unlock"), middle("Spend from this wallet"));
    assert!(
        (unlock - spend).abs() <= 2,
        "Unlock centred at {unlock}, Spend at {spend}"
    );
}

#[test]
fn on_the_small_panel_a_vaults_header_in_files_and_homes_backups_summary_read_whole() {
    let mut rig = rig(
        On::Stick,
        Mode::Mainnet,
        vec![(
            "a-long-vault-name-for-the-family.kdbx".into(),
            testkit::test_vault().unwrap(),
        )],
    );
    rig.press(Action::Nav(Screen::Vaults));
    rig.press(Action::Vault(V::Open(0)));
    rig.press(Action::Vault(V::FocusPassphrase));
    crate::common::type_text(&mut rig, testkit::VAULT_PASSPHRASE);
    rig.press(Action::Vault(V::Unlock));
    crate::common::settle(&mut rig, 100);
    if rig.sheet == Some(Sheet::UnlockLoad) {
        rig.press(Action::Vault(V::LoadTicked));
    }
    rig.press(Action::Nav(Screen::Files));
    rig.settle();
    let texts = rig.drawn_texts();
    assert!(
        texts.iter().any(|t| t.ends_with("vault · open")) && !texts.iter().any(|t| t.contains('…')),
        "the vault card's header is cut: {texts:?}"
    );
    rig.press(Action::Nav(Screen::Home));
    rig.settle();
    let texts = rig.drawn_texts();
    assert!(
        !texts
            .iter()
            .any(|t| t.contains("not bac") && !t.ends_with("backed up")),
        "Home's Backups summary is cut: {texts:?}"
    );
}
