//! Every press a main screen offers, in the heavy session, does no
//! costly work beyond its named budget, the press and the frames after
//! it together (`docs/SPEED.md` §8). A press that needs a costly answer
//! works it out once; drawing what it opened works out nothing it could
//! have kept. On a desktop and on the Pi's panel, whose compact screens
//! offer their own presses.

use osk_shell_api::{App, Event};

use super::{DESKTOP, MAIN, PI, Panel, walk_on};
use crate::work::{Counts, KINDS, Work, since, snapshot};
use crate::{Action, Faraday, Screen};

/// Presses left out, by the start of their name: each ends the session
/// or drops what the next press needs (a lock, a wipe, a change of
/// network, a removal; a shorter idle lock locks on the next tick), or
/// is a hold that a press does not finish.
const SKIP: [&str; 17] = [
    "Lock",
    "PowerOff",
    "Restart",
    "Wipe",
    "Network",
    "Demo",
    "Forget",
    "Delete",
    "Remove",
    "Erase",
    "Upgrade",
    "Exit",
    "Hold",
    "StickPill",
    "Language",
    "Clear",
    "IdleLock",
];

/// What one press may work out, the press and the frames after it, in
/// this session, with the reason: (the press's name as far as it is
/// given, kind, budget, why). The name is the action as `Debug` prints
/// it with each number written `#`; the longest start that matches
/// wins. Every kind not listed has a budget of none.
const PRESS_BUDGETS: &[(&str, Work, u64, &str)] = &[
    (
        "",
        Work::FullCanvas,
        3,
        "a press that changes the screen fades from the frame before: one copy of it, blended into each of the two frames after",
    ),
    (
        "",
        Work::Blur,
        1,
        "a press that opens a sheet frosts the page under it once, and keeps it",
    ),
    (
        "PickWallet",
        Work::Address,
        12,
        "the wallet's card shows its first receive address: derived the first time the card shows, then kept on the wallet (Wallet::address_on) (1); each script a wallet derives for WalletMatch below is a script derivation, counted at its source (11)",
    ),
    (
        "PickWallet",
        Work::WalletMatch,
        11,
        "the card's first button asks which wallet each PSBT in Files spends from: a wallet derives its script at a PSBT's places the first time it is asked, then keeps it (Wallet::script_at); the cards drawn before asked only the wallets up to each PSBT's own",
    ),
    (
        "Vault(Category(#))",
        Work::WordsParse,
        1,
        "the seeds category opens on a key record, whose words are read from the record to show them (work::Allowed::SecretShown): nothing secret is kept",
    ),
    (
        "Backup(#)",
        Work::PlanMap,
        2,
        "Back up opens on the wallet's plan: mapped and checked for its shape and answers the first time, then kept; the eight plans kept had moved on to others",
    ),
    (
        "Step(#)",
        Work::Address,
        2,
        "a spend's step opened shows the wallet's own addresses its outputs pay (receive or change): each derived the first time it is shown, then kept on the wallet",
    ),
    (
        "Resume(Flow(#))",
        Work::FileHash,
        2,
        "the GPG key's flow resumed at its end finishes: the vault it changed is sealed and hashed for its record, and hashed once as it comes into the Outbox (Item::sha)",
    ),
];

/// A press that opens a sheet copies the page twice more than one that
/// changes the screen: the frosted page kept, and put back on the frame
/// after (`Ui::backdrop`).
const SHEET_CANVAS: u64 = 5;

/// The budget of `kind` for the press called `name`.
fn budget(name: &str, kind: Work) -> u64 {
    PRESS_BUDGETS
        .iter()
        .filter(|(start, k, ..)| *k == kind && name.starts_with(start))
        .max_by_key(|(start, ..)| start.len())
        .map_or(0, |b| b.2)
}

/// An action's name with its numbers written `#`.
fn name_of(a: Action) -> String {
    format!("{a:?}")
        .chars()
        .map(|c| if c.is_ascii_digit() { '#' } else { c })
        .collect()
}

/// `screen` drawn, at rest, with no sheet open.
fn on(app: &mut Faraday, screen: Screen) {
    if app.sheet.is_some() {
        app.press(Action::Cancel);
    }
    if app.screen != screen || app.sheet.is_some() {
        app.press(Action::Nav(screen));
    }
    app.settle();
    let _ = app.frame();
}

/// The press, then a frame, a tick and a frame: the work it asks for
/// now and on the tick after.
fn press_work(app: &mut Faraday, a: Action) -> Counts {
    let before = snapshot();
    app.press(a);
    let _ = app.frame();
    let now = app.now_ms;
    app.event(Event::Tick { now_ms: now + 16 });
    let _ = app.frame();
    since(&before)
}

/// Every press each main screen offers on `panel`, against the budgets;
/// at least `least` of them, so that a walk that reached fewer screens
/// fails rather than passes on less.
fn presses_hold_on(panel: Panel, least: usize) {
    let c = crate::testkit::compute();
    // Each screen of the walk drawn, as a person reaches it.
    let (mut app, _) = walk_on(&c, panel, &mut |app, _| {
        app.settle();
        let _ = app.frame();
    });
    let mut lines = Vec::new();
    let mut failures = Vec::new();
    let mut pressed = 0;
    let screens = MAIN.iter().copied().chain([("Spend", Screen::Spend)]);
    for (screen_name, screen) in screens {
        on(&mut app, screen);
        let mut offered: Vec<Action> = Vec::new();
        for (_, a) in app.hits.clone() {
            if !offered.contains(&a) {
                offered.push(a);
            }
        }
        for a in offered {
            let name = name_of(a);
            if SKIP.iter().any(|s| name.starts_with(s)) {
                continue;
            }
            on(&mut app, screen);
            if !app.offers(a) {
                continue;
            }
            let work = press_work(&mut app, a);
            pressed += 1;
            let mut line = format!("{screen_name}: {name}:");
            for k in KINDS {
                let n = work.of(k);
                if n == 0 {
                    continue;
                }
                line.push_str(&format!(" {k:?} {n}"));
                let mut b = budget(&name, k);
                if k == Work::FullCanvas && app.sheet.is_some() {
                    b = b.max(SHEET_CANVAS);
                }
                if n > b {
                    failures.push(format!(
                        "{screen_name}: {name} did {k:?} {n} times (budget {b})"
                    ));
                }
            }
            lines.push(line);
        }
    }
    assert!(
        pressed >= least,
        "{pressed} presses: the screens offered fewer than expected"
    );
    assert!(
        failures.is_empty(),
        "{}\n\nEach press:\n{}",
        failures.join("\n"),
        lines.join("\n")
    );
}

#[test]
fn every_press_a_main_screen_offers_does_no_costly_work_beyond_its_budget() {
    presses_hold_on(DESKTOP, 100);
}

#[test]
fn on_the_pis_panel_every_press_a_main_screen_offers_does_no_costly_work_beyond_its_budget() {
    // A compact screen offers fewer presses: one column, its menu folded.
    presses_hold_on(PI, 30);
}
