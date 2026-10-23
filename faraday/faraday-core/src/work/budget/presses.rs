//! Every press a main screen offers, in the heavy session, does no
//! costly work beyond its named budget, the press and the frames after
//! it together (`docs/SPEED.md` §8). A press that needs a costly answer
//! works it out once; drawing what it opened works out nothing it could
//! have kept. On a desktop and on the Pi's panel, whose compact screens
//! offer their own presses.

use faraday_shell_api::{App, Event};

use super::{DESKTOP, MAIN, PI, Panel, walk_on};
use crate::work::{Counts, KINDS, Work, since, snapshot};
use crate::{Action, Faraday, Screen, testkit};

/// Presses left out, by the start of their name: each ends the session
/// or drops what the next press needs (a lock, a wipe, a change of
/// mode (`Mode(..)`; the pill's `ModeSheet` is walked), a removal; a shorter idle lock locks on the next tick), or
/// is a hold that a press does not finish.
const SKIP: [&str; 17] = [
    "Lock",
    "PowerOff",
    "Restart",
    "Wipe",
    "Mode(",
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
        "SamplesLoad",
        Work::Kdf,
        3,
        "Load ticked stretches each key it adds once (PBKDF2; the backup's Argon2id run too): here the three keys of Savings. A key already loaded is looked up first, not derived again",
    ),
    (
        "SamplesLoad",
        Work::MasterKey,
        3,
        "each key added has its master key and fingerprint made once (three: Savings' keys)",
    ),
    (
        "SamplesLoad",
        Work::KeyDerive,
        3,
        "each key added derives the account keys its entry keeps (three: Savings' keys)",
    ),
    (
        "SamplesLoad",
        Work::WordsParse,
        3,
        "each words sample is parsed once, as typed words are (three: Savings' keys)",
    ),
    (
        "SamplesLoad",
        Work::FileHash,
        3,
        "each key added gets the id its entry keeps, a hash of its secret (three: Savings' keys)",
    ),
    (
        "LoadWallet",
        Work::DescriptorParse,
        1,
        "a wallet file in Files is read once as a typed descriptor is, when its Load is pressed",
    ),
    (
        "SamplesLoad",
        Work::DescriptorParse,
        1,
        "each ticked wallet is read once as a typed descriptor is (one: Savings)",
    ),
    (
        "Samples",
        Work::DescriptorParse,
        1,
        "the sheet opened with a wallet loaded reads each sample wallet once, to show the one loaded ticked and dimmed (one in this test; the real data has thirteen); its frames read nothing",
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
        for (_, a) in app.pointer.hits().to_vec() {
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

/// The mode pill's press and the mode sheet's frames, in a mode the pill
/// shows: nothing costly beyond the fade and the frosted page every sheet
/// takes (`docs/MODES.md` §3), at rest and drawn again.
fn mode_sheet_holds_on(panel: Panel) {
    let (mut app, _) = super::heavy_on(panel);
    // The heavy session is on Testnet: the pill shows.
    on(&mut app, Screen::Home);
    assert!(app.offers(Action::ModeSheet), "the pill is on Home");
    let work = press_work(&mut app, Action::ModeSheet);
    assert_eq!(app.sheet, Some(crate::Sheet::Mode));
    for k in KINDS {
        let b = match k {
            Work::FullCanvas => SHEET_CANVAS,
            Work::Blur => 1,
            _ => 0,
        };
        assert!(work.of(k) <= b, "ModeSheet did {k:?} {} times", work.of(k));
    }
    // The sheet drawn again at rest: nothing costly at all.
    app.settle();
    let _ = app.frame();
    let before = snapshot();
    for _ in 0..3 {
        let _ = app.frame();
    }
    let again = since(&before);
    for k in KINDS {
        assert!(
            again.of(k) == 0,
            "the sheet's frames did {k:?} {}",
            again.of(k)
        );
    }
    // Not now: the page comes back, a press like any that leaves a sheet.
    let work = press_work(&mut app, Action::Cancel);
    assert_eq!(app.sheet, None);
    for k in KINDS {
        let b = if k == Work::FullCanvas { 3 } else { 0 };
        assert!(work.of(k) <= b, "Not now did {k:?} {}", work.of(k));
    }
}

#[test]
fn the_mode_pills_press_and_the_sheets_frames_do_no_costly_work() {
    mode_sheet_holds_on(DESKTOP);
    mode_sheet_holds_on(PI);
}

/// The samples a Demo session offers in this test: the core's own
/// handouts, and the Savings wallet over the first three of them (the
/// real data is in `faraday-samples`, which core does not link).
fn savings_samples() -> &'static crate::samples::Samples {
    use crate::samples::{SampleKind, SampleScript, SampleWallet, Samples};
    static SET: std::sync::OnceLock<Samples> = std::sync::OnceLock::new();
    SET.get_or_init(|| {
        let keys = testkit::demo_samples().keys;
        let fps: Vec<[u8; 4]> = ["bacon", "zebra", "summer"]
            .iter()
            .filter_map(|w| keys.iter().find(|k| k.word == *w).and_then(|k| k.plain))
            .collect();
        let wallet = SampleWallet {
            name: "sample_savings",
            kind: SampleKind::Multisig { need: 2, total: 3 },
            script: SampleScript::NativeSegwit,
            text: Box::leak(testkit::savings().into_boxed_str()),
            spend: &[],
            keys: Box::leak(fps.into_boxed_slice()),
        };
        Samples {
            keys,
            wallets: Box::leak(vec![wallet].into_boxed_slice()),
            inputs: testkit::demo_samples().inputs,
        }
    })
}

/// The press `a` and the frames after it, against the budget its name has
/// (a press that opens a sheet copies the canvas more).
fn press_within(app: &mut Faraday, a: Action, canvas: u64) {
    let name = name_of(a);
    let work = press_work(app, a);
    let mut over = Vec::new();
    for k in KINDS {
        let b = if k == Work::FullCanvas {
            budget(&name, k).max(canvas)
        } else {
            budget(&name, k)
        };
        if work.of(k) > b {
            over.push(format!("{k:?} {} (budget {b})", work.of(k)));
        }
    }
    assert!(over.is_empty(), "{name} did {}", over.join(", "));
}

/// Frames drawn again on the sheet at rest do nothing costly.
fn sheet_frames_are_free(app: &mut Faraday, what: &str) {
    app.settle();
    let _ = app.frame();
    let before = snapshot();
    for _ in 0..3 {
        let _ = app.frame();
    }
    let again = since(&before);
    for k in KINDS {
        assert!(again.of(k) == 0, "{what} did {k:?} {}", again.of(k));
    }
}

/// Load samples in Demo: the pill's press, the sheet's frames, a tick and
/// Load ticked, each against its budget; the sheet at rest does nothing
/// costly (`docs/MODES.md` §10 M6).
fn samples_sheet_holds_on(panel: Panel) {
    use crate::samples::Pick;
    let mut app = Faraday::new();
    super::show(&mut app, panel);
    app.session = crate::wallet::Session::on(testkit::NET);
    app.session.set_demo(savings_samples());
    app.samples = Some(savings_samples());
    app.set_modes(crate::DESKTOP_MODES);
    on(&mut app, Screen::Home);
    assert!(app.offers(Action::Samples), "the pill is on Home");
    // Nothing is loaded: the sheet opens without reading a wallet.
    press_within(&mut app, Action::Samples, SHEET_CANVAS);
    assert_eq!(app.sheet, Some(crate::Sheet::Samples));
    sheet_frames_are_free(&mut app, "the sheet's frames");
    press_within(&mut app, Action::SampleTick(Pick::Wallet(0)), 3);
    press_within(&mut app, Action::SampleTick(Pick::Key(0)), 3);
    // Load ticked derives the keys and reads the wallet, once, in the press.
    press_within(&mut app, Action::SamplesLoad, 3);
    assert_eq!(app.sheet, None);
    assert_eq!(app.session.wallets.len(), 1);
    // The sheet again, with a wallet loaded: it reads each sample wallet
    // once, when it opens, and its frames still do nothing costly.
    on(&mut app, Screen::Home);
    press_within(&mut app, Action::Samples, SHEET_CANVAS);
    sheet_frames_are_free(&mut app, "the loaded sheet's frames");
}

#[test]
fn the_load_samples_pill_its_sheet_and_load_ticked_do_no_costly_work_beyond_their_budgets() {
    samples_sheet_holds_on(DESKTOP);
    samples_sheet_holds_on(PI);
}
