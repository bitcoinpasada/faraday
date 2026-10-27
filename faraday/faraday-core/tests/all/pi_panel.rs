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
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, Host, ScanPurpose, Screen, Sheet, StorageEvent, testkit};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, SecureHardware};
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
        Backups, CreateVault, KeyGen, Bip85, Silent, Explore, Lightning, Tools, Unlock,
        VaultContents, Codes, Family, Vanity, Decode, Catalog, Transfer, BackupFlow)
}

fn sheets() -> Vec<Sheet> {
    every!(Sheet: Lock, LockAsk, Power, Qr, Scan, Locked, NewInput, Learn, WordList,
        SecretOut, WriteOut, IdleWarn, Potential, Import, Pull, Chart, BoardList, BoardSees,
        BoardPartials, BoardMenu, SeedRead, Delete, Zeroing, Taken, Devices, VaultSave,
        CancelFlow(Going::Create), CancelKey, FileMenu, Rename, Details, UnlockLoad,
        PassphraseSeeds, GpgSign, Mode, Samples, NavMenu, KeyLengths, KeyPath, Register)
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

/// The controls of the bar across the page's top (44 dp) whose touch
/// targets overlap: a press there could reach either.
fn head_row_overlaps(rig: &Rig) -> Vec<String> {
    let f = f32::from(rig.w) / rig.size().0;
    let bar = (44.0 * f).round() as i32 + 1;
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
        rig.press(Action::KStep(kstep::START));
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
        rig.press(Action::KStep(kstep::START));
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
        vec![("vault.ofv".into(), testkit::test_vault().unwrap())],
    );
    rig.press(Action::Nav(Screen::Vaults));
    rig.press(Action::Vault(V::Open(0)));
    rig.settle();
    let mut found = walk(&mut rig, "Unlock");
    rig.press(Action::Vault(V::FocusPassphrase));
    crate::common::type_text(&mut rig, testkit::VAULT_PASSPHRASES[0]);
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
panel_tests!(on_the_stick_on_testnet, On::Stick, Mode::Testnet);
panel_tests!(on_the_stick_in_practice, On::Stick, Mode::Practice);
panel_tests!(in_the_demo_preview_on_testnet, On::Preview, Mode::Testnet);
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
