//! The Spend tab when something is loaded already: it opens on what is
//! loaded and the next step for each wallet instead of the walk-through,
//! which stays one press away and is what someone with nothing loaded
//! still sees first.

use faraday_core::spend_backup::{Open, SpendBackupAction as F, page};
use faraday_core::testkit;
use faraday_core::testkit::Plain;
use faraday_core::wallet::step;
use faraday_core::{Action, Faraday, Screen, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware, TouchPhase};

fn kit(id: &str) -> testkit::Kit {
    testkit::kits().into_iter().find(|k| k.id == id).unwrap()
}

fn with_files(files: Vec<(String, Vec<u8>)>) -> Faraday {
    let mut app = Faraday::new();
    faraday_core::testkit::move_to(&mut app.session, faraday_core::testkit::NET).unwrap();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 768,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(files),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app
}

fn add_key(app: &mut Faraday, n: usize) {
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[n].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
}

fn savings() -> (String, Vec<u8>) {
    (
        "savings-wallet.txt".to_string(),
        kit("savings").descriptor.into_bytes(),
    )
}

fn savings_psbt() -> (String, Vec<u8>) {
    let (u, _) = testkit::spend_of(&kit("savings")).unwrap();
    ("savings-unsigned.psbt".to_string(), u.to_bytes())
}

#[test]
fn with_nothing_loaded_the_tab_is_the_walk_through() {
    let mut app = with_files(Vec::new());
    app.press(Action::Nav(Screen::SpendBackup));
    assert!(!app.spend_backup_overview());
}

#[test]
fn a_ready_wallet_goes_straight_to_its_next_page() {
    let mut app = with_files(vec![savings()]);
    add_key(&mut app, 0);
    add_key(&mut app, 1);
    testkit::load_wallet(&mut app, 0);
    app.press(Action::Nav(Screen::SpendBackup));
    assert!(
        app.spend_backup_overview(),
        "the tab opens on what is loaded"
    );
    let _ = app.frame();
    assert!(app.offers(Action::SpendBackup(F::SpendFrom(0))));
    app.press(Action::SpendBackup(F::SpendFrom(0)));
    assert!(!app.spend_backup_overview());
    assert_eq!(app.spend_backup_wallet(), Some(0));
    assert_eq!(
        app.spend_backup().open,
        Some(Open::Page(page::CHECK)),
        "the map, the stick and the wallet are behind it"
    );
}

#[test]
fn a_ready_wallet_with_its_transaction_in_files_goes_to_signing() {
    let mut app = with_files(vec![savings(), savings_psbt()]);
    add_key(&mut app, 0);
    add_key(&mut app, 1);
    testkit::load_wallet(&mut app, 0);
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::SpendBackup(F::SpendFrom(0)));
    assert!(
        app.spend().is_some(),
        "the PSBT in Files is the one to sign"
    );
    assert_eq!(app.screen, Screen::SpendBackup);
    assert_eq!(app.spend_backup().open, Some(Open::Spend));
    assert!(app.spend().unwrap().steps.contains(&step::SIGN));
}

/// A wallet made here with its keys loaded asks for no seeds: its
/// page offers no Add the seeds that can sign.
#[test]
fn a_wallet_made_here_with_its_keys_loaded_asks_for_no_seeds() {
    use faraday_core::strings::EN;
    let c = testkit::compute();
    let mut app = with_files(vec![savings_psbt()]);
    add_key(&mut app, 0);
    add_key(&mut app, 1);
    app.session
        .add_wallet_confirmed(
            &c,
            "Savings",
            &kit("savings").descriptor,
            EN.create_make_made_here,
            faraday_core::wallet::Origin::Statement,
        )
        .unwrap();
    // The keys not checked show their block above Who signs: a display
    // tall enough that nothing of the page is below the fold.
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 4000,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::SpendBackup(F::SpendFrom(0)));
    // Walked to the signers' card, the current step.
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::Step(step::SIGNERS));
    assert_eq!(
        app.spend_backup_open_card(),
        Some(faraday_core::spend_backup::CardId::Step(step::SIGNERS))
    );
    let _ = app.frame();
    let texts = app.drawn_texts();
    assert!(
        texts.iter().any(|t| t == EN.spend_backup_who_signs),
        "{texts:?}"
    );
    assert!(
        !texts.iter().any(|t| t == EN.spend_backup_add_the_seeds),
        "{texts:?}"
    );
}

#[test]
fn a_wallet_short_of_keys_is_opened_rather_than_spent() {
    let mut app = with_files(vec![savings()]);
    add_key(&mut app, 0);
    testkit::load_wallet(&mut app, 0);
    app.press(Action::Nav(Screen::SpendBackup));
    let _ = app.frame();
    assert!(!app.offers(Action::SpendBackup(F::SpendFrom(0))));
    assert!(app.offers(Action::OpenWallet(0)));
}

#[test]
fn a_seed_with_no_wallet_spends_as_a_single_key_wallet() {
    let mut app = with_files(Vec::new());
    add_key(&mut app, 0);
    app.press(Action::Nav(Screen::SpendBackup));
    assert!(app.spend_backup_overview());
    let _ = app.frame();
    assert!(app.offers(Action::SpendBackup(F::SeedWallet)));
    app.press(Action::SpendBackup(F::SeedWallet));
    assert_eq!(app.session.wallets.len(), 1);
    assert_eq!(app.spend_backup_wallet(), Some(0));
}

/// The Spending wallet's spend in the signing review, with its first
/// output stating, or not, an origin under a key that is not loaded.
fn spending_review(claimed: bool) -> Faraday {
    spending_with(claimed, true)
}

/// The single-key wallet's spend, with its key loaded or not.
fn spending_with(claimed: bool, key_loaded: bool) -> Faraday {
    let kit = kit("spending");
    let mut psbt = testkit::unsigned(&kit).unwrap().inner().clone();
    if claimed {
        let key = psbt.outputs[1]
            .bip32_derivation
            .keys()
            .next()
            .copied()
            .unwrap();
        let path = psbt.outputs[1].bip32_derivation[&key].1.clone();
        psbt.outputs[0].bip32_derivation.insert(
            key,
            (
                faraday_bip::bitcoin::bip32::Fingerprint::from([0xde, 0xad, 0xbe, 0xef]),
                path,
            ),
        );
    }
    let psbt = faraday_psbt::Psbt::from(psbt);
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("spend.psbt"), psbt.to_bytes()),
            (
                faraday_text::Text::of("spending-wallet.txt"),
                kit.descriptor.clone().into_bytes(),
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    testkit::load_wallet(&mut app, 1);
    if key_loaded {
        add_key(&mut app, 0);
    }
    app.press(Action::StartSpend(0));
    app
}

/// The side panel's Change line counts verified change only: an output
/// that is only claimed as change is among what it sends (F136), so the
/// panel names the wallet of the change that is, and never reads Unverified.
#[test]
fn the_panel_counts_only_verified_change() {
    // The value beside the panel's Change label.
    let panel_change = |claimed: bool| {
        let mut app = spending_review(claimed);
        let shown = crate::common::texts(&mut app);
        let panel = shown
            .iter()
            .position(|t| t == "This transaction")
            .unwrap_or_else(|| panic!("{shown:?}"));
        let label = shown[panel..]
            .iter()
            .position(|t| t == "Change")
            .unwrap_or_else(|| panic!("{shown:?}"));
        shown[panel + label + 1].clone()
    };
    // The panel names the wallet the verified change goes to (F254).
    let wallet = spending_review(false).session.wallets[0].name.to_string();
    assert_eq!(panel_change(false), wallet);
    assert_eq!(panel_change(true), wallet);
}

#[test]
fn the_walk_through_is_one_press_away() {
    let mut app = with_files(Vec::new());
    add_key(&mut app, 0);
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::SpendBackup(F::Walkthrough));
    assert!(!app.spend_backup_overview());
    assert_eq!(app.spend_backup().open, Some(Open::Page(page::MAP)));
}

/// The ink boxes of a loaded wallet's name, shape and keys' line on the
/// Pi's panel at `size`, with Savings loaded and one of its keys.
fn card_lines_at(size: faraday_core::ui::TextSize) -> Vec<(String, (i32, i32, i32, i32))> {
    let mut app = with_files(vec![savings()]);
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
    add_key(&mut app, 0);
    testkit::load_wallet(&mut app, 0);
    app.press(Action::TextSize(size));
    app.press(Action::Nav(Screen::SpendBackup));
    let lines: Vec<_> = app
        .drawn_text_boxes()
        .plain()
        .into_iter()
        .filter(|(t, _)| t == "Savings" || t.starts_with("2 of 3") || t.starts_with("1 key"))
        .collect();
    // The card's own name, the one just over its shape: the bar at the
    // top names the wallet too.
    let shape = lines
        .iter()
        .find(|(t, _)| t.starts_with("2 of 3"))
        .map(|(_, b)| b.1);
    let shape = shape.expect("the wallet's shape is shown");
    let card: Vec<_> = lines
        .into_iter()
        .filter(|(t, b)| t != "Savings" || (b.1 < shape && shape - b.1 < 2 * b.3))
        .collect();
    assert_eq!(card.len(), 3, "{card:?}");
    card
}

/// On a one-column page a loaded wallet's card grows with larger text:
/// its name, its shape and its keys' line stand one under another, none
/// over the next, at every text size.
#[test]
fn a_loaded_wallets_card_keeps_its_lines_apart_at_every_text_size() {
    use faraday_core::ui::TextSize;
    for size in TextSize::ALL {
        let lines = card_lines_at(size);
        let mut tops: Vec<_> = lines.iter().map(|(_, b)| *b).collect();
        tops.sort_by_key(|b| b.1);
        for pair in tops.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            assert!(a.1 + a.3 <= b.1 + 1, "{size:?}: {lines:?}");
        }
    }
}

/// A step's count never states a total that a later step changes: the
/// number of steps is not known before the flow's input is read.
#[test]
fn a_spend_s_step_count_states_no_total() {
    let mut app = spending_review(false);
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let shown = crate::common::texts(&mut app);
    assert!(
        shown.iter().any(|t| t.starts_with("Step ")),
        "a step is counted: {shown:?}"
    );
    assert!(
        !shown
            .iter()
            .any(|t| t.starts_with("Step ") && t.contains(" of ")),
        "a step count states a total: {shown:?}"
    );
}

/// Spend from a backup counts its steps without a total: the list grows
/// once the transaction is read.
#[test]
fn spending_from_a_backup_counts_its_steps_without_a_total() {
    use faraday_core::strings::EN;
    let c = testkit::compute();
    let mut app = with_files(vec![savings_psbt()]);
    app.event(Event::Display(DisplayInfo {
        width: 480,
        height: 640,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    add_key(&mut app, 0);
    add_key(&mut app, 1);
    app.session
        .add_wallet_confirmed(
            &c,
            "Savings",
            &kit("savings").descriptor,
            EN.create_make_made_here,
            faraday_core::wallet::Origin::Statement,
        )
        .unwrap();
    app.press(Action::Nav(Screen::SpendBackup));
    app.press(Action::SpendBackup(F::SpendFrom(0)));
    let shown = crate::common::texts(&mut app);
    assert!(
        shown.iter().any(|t| t.starts_with("Step ")),
        "a step is counted: {shown:?}"
    );
    assert!(
        !shown
            .iter()
            .any(|t| t.starts_with("Step ") && t.contains(" of ")),
        "a step count states a total: {shown:?}"
    );
}

/// The window made `width` x `height` at 160 dpi.
fn window(app: &mut Faraday, (width, height): (u16, u16)) {
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.settle();
    let _ = app.frame();
}

/// Where `text` is drawn, the first of it: (x, y, w, h).
fn drawn_at(app: &mut Faraday, text: &str) -> Option<(i32, i32, i32, i32)> {
    let _ = app.frame();
    app.drawn_text_boxes()
        .plain()
        .into_iter()
        .find(|(t, _)| t == text)
        .map(|b| b.1)
}

/// On a wide window one step's card is drawn at a time, and the pane holds
/// Your choices' rows with the transaction's review under them.
#[test]
fn a_wide_window_shows_one_step_and_the_review_under_your_choices() {
    let mut app = spending_review(false);
    window(&mut app, (1366, 768));
    let shown = crate::common::texts(&mut app);
    let counts: Vec<&String> = shown.iter().filter(|t| t.starts_with("Step ")).collect();
    assert_eq!(counts.len(), 1, "one card at a time: {shown:?}");
    assert!(!counts[0].contains(" of "), "{shown:?}");
    let pos = |what: &str| {
        shown
            .iter()
            .position(|t| t == what)
            .unwrap_or_else(|| panic!("{what}: {shown:?}"))
    };
    let (choices, review) = (pos("Your choices"), pos("This transaction"));
    assert!(choices < review, "the rows come first: {shown:?}");
    for fact in ["Wallet", "Sends", "Fee", "Change", "Txid"] {
        assert!(
            shown[review..].iter().any(|t| t == fact),
            "{fact}: {shown:?}"
        );
    }
    // The facts are plain lines: a step is reached by Your choices.
    assert!(
        !crate::common::offered(&mut app, Action::Step(step::CHECK)),
        "a step ahead is not offered by a fact"
    );
}

/// On a low wide window the pane scrolls within itself, and the signatures
/// box under the facts is reached by it.
#[test]
fn on_a_low_wide_window_the_signatures_box_is_reached_by_scrolling_the_pane() {
    let mut app = spending_review(false);
    window(&mut app, (1366, 600));
    assert!(
        !crate::common::offered(&mut app, Action::Primary),
        "the box is below the window"
    );
    let head = drawn_at(&mut app, "Your choices").expect("the pane is shown");
    let (x, y) = (
        u16::try_from(head.0 + 20).unwrap(),
        u16::try_from(head.1 + 80).unwrap(),
    );
    app.event(Event::Hover { x, y });
    app.event(Event::Scroll { x, y, dy: 1200 });
    app.event(Event::ScrollEnd { x, y });
    app.settle();
    assert!(app.choices_offset() > 0.0, "the pane scrolled");
    assert!(
        crate::common::offered(&mut app, Action::Primary),
        "the box's action is reached"
    );
}

/// Once the transaction is signed, the rows of the steps before it take no
/// press: it is not signed again from another step (Back stops there).
#[test]
fn after_signing_the_rows_before_it_take_no_press() {
    let mut app = spending_review(false);
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::Step(step::SIGN));
    assert!(
        crate::common::offered(&mut app, Action::Step(step::TRANSACTION)),
        "before signing the earlier rows are offered"
    );
    app.press(Action::SignHere);
    assert!(!app.spend().unwrap().spend.signed_here.is_empty());
    for before in [step::WALLET, step::TRANSACTION, step::CHECK, step::SIGN] {
        assert!(
            !crate::common::offered(&mut app, Action::Step(before)),
            "step {before} takes a press after signing"
        );
    }
}

/// What Your choices says of the keys at Signers: the value of each "Key"
/// row, as drawn.
fn key_rows(app: &mut Faraday) -> Vec<String> {
    key_rows_at(app, (1920, 1080))
}

fn key_rows_at(app: &mut Faraday, size: (u16, u16)) -> Vec<String> {
    key_runs_at(app, size)
        .into_iter()
        .map(|r| r.concat())
        .collect()
}

/// The key rows of the pane, each as the runs drawn on its line: the
/// fingerprint, then the name and the mark where there are any.
fn key_runs_at(app: &mut Faraday, size: (u16, u16)) -> Vec<Vec<String>> {
    window(app, size);
    let ink = app.drawn_ink();
    let mut runs: Vec<(i32, i32, String)> = Vec::new();
    for i in ink {
        if let faraday_draw::canvas::InkKind::Text { text, .. } = i.kind {
            runs.push((i.rect.x, i.rect.y, text.as_str().to_string()));
        }
    }
    // The pane's rows stand at its heading's left edge, under it.
    let Some(&(hx, hy, _)) = runs.iter().find(|(.., r)| r == "Your choices") else {
        return Vec::new();
    };
    let fp = |t: &str| t.len() == 8 && t.chars().all(|c| c.is_ascii_hexdigit());
    let mut rows = Vec::new();
    for (x, y, r) in &runs {
        if (x - hx).abs() > 4 || *y <= hy || !fp(r) {
            continue;
        }
        let mut line: Vec<(i32, String)> = runs
            .iter()
            .filter(|(x2, y2, _)| (y2 - y).abs() <= 3 && x2 + 4 >= *x)
            .filter(|(x2, _, r2)| x2 == x || r2.starts_with(" · "))
            .map(|(x2, _, r2)| (*x2, r2.clone()))
            .collect();
        line.sort_by_key(|a| a.0);
        rows.push(line.into_iter().map(|(_, r)| r).collect());
    }
    rows
}

/// The row of the one key that signs here, among the wallet's three.
fn signing_row(rows: &[String]) -> &String {
    let own: Vec<&String> = rows
        .iter()
        .filter(|r| r.ends_with(" · Signs here"))
        .collect();
    assert_eq!(own.len(), 1, "{rows:?}");
    own[0]
}

fn marks(app: &mut Faraday) -> (usize, usize) {
    let rows = key_rows(app);
    assert!(
        !rows.is_empty(),
        "no key row is drawn: {:?}",
        app.drawn_text_boxes()
            .iter()
            .map(|(t, _)| t.as_str().to_string())
            .collect::<Vec<_>>()
    );
    let here = rows.iter().filter(|t| t.ends_with(" · Signs here")).count();
    let signed = rows.iter().filter(|t| t.ends_with(" · Signed")).count();
    (here, signed)
}

/// Savings' spend (a multisig, so it has the Signers step) with one of its
/// keys loaded, open on Check.
fn savings_spend() -> Faraday {
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("spend.psbt"), unsigned.to_bytes()),
            (
                faraday_text::Text::of("savings-wallet.txt"),
                k.descriptor.clone().into_bytes(),
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let at = |app: &Faraday, n: &str| app.inbox().iter().position(|i| i.name == n).unwrap();
    let w = at(&app, "savings-wallet.txt");
    testkit::load_wallet(&mut app, w);
    crate::common::add_key(&mut app, "bacon");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    app
}

/// A key at Signers is listed in Your choices as signing here before it has
/// signed, as signed once its signature is in the file, and as signed no
/// longer when the spend is started again on the unsigned PSBT.
#[test]
fn a_key_reads_signed_only_while_its_signature_is_in_the_file() {
    let mut app = savings_spend();
    app.press(Action::Step(step::SIGNERS));
    assert_eq!(marks(&mut app), (1, 0), "loaded, not signed yet");
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::Step(step::SIGN));
    assert_eq!(
        marks(&mut app),
        (1, 0),
        "on the Sign card, still not signed"
    );
    app.press(Action::SignHere);
    assert!(!app.spend().unwrap().spend.signed_here.is_empty());
    app.press(Action::Step(step::SIGNERS));
    assert_eq!(
        marks(&mut app),
        (0, 1),
        "signed, no longer only signing here"
    );
    // The PSBT replaced by the unsigned one: the signature is gone.
    app.press(Action::StartSpend(0));
    app.press(Action::Step(step::SIGNERS));
    assert_eq!(
        marks(&mut app),
        (1, 0),
        "a fresh spend of the unsigned file"
    );
}

/// The centre of the drawn key row whose answer reads `value`. The card
/// may draw the same fingerprint: the pane's row stands at the left edge
/// of its "Your choices" heading, under it.
fn key_row_at(app: &mut Faraday, value: &str) -> (u16, u16) {
    let boxes = app.drawn_text_boxes();
    let (hx, hy) = boxes
        .iter()
        .find(|(t, _)| t.as_str() == "Your choices")
        .map(|(_, (x, y, ..))| (*x, *y))
        .expect("the pane's heading");
    let (_, (x, y, w, h)) = boxes
        .iter()
        .find(|(t, (x, y, ..))| t.as_str() == &value[..8] && (x - hx).abs() <= 4 && *y > hy)
        .unwrap_or_else(|| panic!("no key row {value:?} in {boxes:?}"));
    ((x + w / 2) as u16, (y + h / 2) as u16)
}

fn tap_at(app: &mut Faraday, (x, y): (u16, u16)) {
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    let _ = app.frame();
}

/// A press on a key's row, drawn in Your choices, opens the Signers step
/// from a step after it.
#[test]
fn a_press_on_a_key_row_opens_the_signers_step() {
    let mut app = savings_spend();
    app.press(Action::Step(step::SIGNERS));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::Step(step::SIGN));
    assert_eq!(app.spend().unwrap().open, Some(step::SIGN));
    let rows = key_rows(&mut app);
    let at = key_row_at(&mut app, signing_row(&rows));
    tap_at(&mut app, at);
    assert_eq!(app.spend().unwrap().open, Some(step::SIGNERS));
}

/// A key row on the step that is open takes no press: no target is drawn
/// over it.
#[test]
fn a_key_row_on_the_open_step_takes_no_press() {
    let mut app = savings_spend();
    app.press(Action::Step(step::SIGNERS));
    let rows = key_rows(&mut app);
    let (x, y) = key_row_at(&mut app, signing_row(&rows));
    let over = app.drawn_targets().into_iter().any(|(_, t, ..)| {
        let (x, y) = (i32::from(x), i32::from(y));
        t.x <= x && x <= t.x + t.w && t.y <= y && y <= t.y + t.h
    });
    assert!(!over, "a press is drawn over the open step's key row");
}

fn spend_with_key_named(label: &str) -> Faraday {
    let mut app = savings_spend();
    let at = app
        .session
        .keys
        .position(testkit::seed_fingerprint(0))
        .unwrap();
    app.session.keys.set_label(at, label);
    app.press(Action::Step(step::SIGNERS));
    app
}

/// The narrowest window that draws Your choices beside the card.
fn narrowest_pane_window(app: &mut Faraday) -> (u16, u16) {
    for w in (600..1400).step_by(4) {
        if !key_rows_at(app, (w, 768)).is_empty() {
            return (w, 768);
        }
    }
    panic!("no window draws the pane");
}

/// A long name is cut, never the mark, at the pane's narrowest and widest.
#[test]
fn a_long_key_name_is_cut_and_the_mark_is_drawn_whole() {
    let mut app = spend_with_key_named("A very long name for the first of the three keys");
    let narrow = narrowest_pane_window(&mut app);
    for size in [narrow, (1920, 1080)] {
        let rows = key_rows_at(&mut app, size);
        assert_eq!(rows.len(), 3, "{size:?}: {rows:?}");
        let own = signing_row(&rows);
        assert!(own.contains('…'), "{size:?}: the name is cut: {rows:?}");
    }
}

/// A name that could read as a mark is left out of the row of a key that
/// has no mark: the row is the fingerprint only, so no name stands as "Signed".
#[test]
fn a_name_that_could_be_a_mark_is_left_out_of_an_unmarked_key() {
    for name in ["Signed", "Cold · Signed"] {
        let mut app = spend_with_key_named(name);
        // Its backup is owed: Sign refuses it, so it has no mark.
        testkit::mark_made_here(&mut app, testkit::seed_fingerprint(0));
        for size in [narrowest_pane_window(&mut app), (1920, 1080)] {
            let rows = key_rows_at(&mut app, size);
            assert_eq!(rows.len(), 3, "{name}, {size:?}: {rows:?}");
            assert!(
                rows.iter().all(|r| r.len() == 8),
                "{name}, {size:?}: {rows:?}"
            );
        }
    }
    // A complete spend: the third key has nothing to sign and no mark either.
    let mut app = spend_with_key_named("Signed");
    app.press(Action::Step(step::SIGN));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::SignHere);
    app.press(Action::Step(step::SIGNERS));
    let rows = key_rows(&mut app);
    assert!(rows.iter().all(|r| !r.contains("Signed ·")), "{rows:?}");
}

/// A key whose backup is still owed has no mark: Sign refuses it.
#[test]
fn a_key_made_here_whose_backup_waits_has_no_mark() {
    let mut app = savings_spend();
    testkit::mark_made_here(&mut app, testkit::seed_fingerprint(0));
    app.press(Action::Step(step::SIGNERS));
    assert_eq!(marks(&mut app), (0, 0));
}

/// The 2-of-3 signed by the other two keys elsewhere is complete: the
/// third key, loaded, has nothing to sign, and the two that signed read
/// signed.
#[test]
fn a_spend_complete_from_elsewhere_marks_no_key_signs_here() {
    let k = kit("savings");
    let (unsigned, cosigned) = testkit::spend_of(&k).unwrap();
    let cosigned = cosigned.expect("savings has a cosigner").1;
    // A second cosigner (Test key 3) adds its signature to the file.
    let mut other = testkit::started();
    other.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("spend.psbt"), cosigned.to_bytes()),
            (
                faraday_text::Text::of("savings-wallet.txt"),
                k.descriptor.clone().into_bytes(),
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let w = other
        .inbox()
        .iter()
        .position(|i| i.name == "savings-wallet.txt")
        .unwrap();
    testkit::load_wallet(&mut other, w);
    crate::common::add_key(&mut other, "summer");
    other.press(Action::StartSpend(
        other
            .inbox()
            .iter()
            .position(|i| i.name == "spend.psbt")
            .unwrap(),
    ));
    testkit::steps_before_sign_done(&mut other);
    other.press(Action::Step(step::SIGN));
    other.press(Action::SignHere);
    let full = other.spend().unwrap().spend.psbt.to_bytes();
    drop(unsigned);
    // The first key, loaded here, opens the file the other two signed.
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("spend.psbt"), full),
            (
                faraday_text::Text::of("savings-wallet.txt"),
                k.descriptor.clone().into_bytes(),
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let w = app
        .inbox()
        .iter()
        .position(|i| i.name == "savings-wallet.txt")
        .unwrap();
    testkit::load_wallet(&mut app, w);
    crate::common::add_key(&mut app, "bacon");
    app.press(Action::StartSpend(
        app.inbox()
            .iter()
            .position(|i| i.name == "spend.psbt")
            .unwrap(),
    ));
    assert!(app.spend().unwrap().complete, "two of three have signed");
    app.press(Action::Step(step::SIGNERS));
    assert_eq!(marks(&mut app), (0, 2));
}

/// A p2wpkh PSBT with one input of test key 1 and one of test key 2, as a
/// coordinator that knows no wallet writes it; `edit` changes it before it
/// is written.
fn two_owner(edit: impl Fn(&mut faraday_bip::bitcoin::Psbt)) -> Vec<u8> {
    use faraday_bip::bitcoin::bip32::{DerivationPath, Fingerprint as Fp};
    use faraday_bip::bitcoin::{
        Amount, OutPoint, PublicKey, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
        absolute::LockTime, transaction::Version,
    };
    use std::str::FromStr;
    let c = testkit::compute();
    let mut session = testkit::session();
    let p = DerivationPath::from_str("m/84'/1'/0'/0/0").unwrap();
    let mut owners = Vec::new();
    for (word, label) in &testkit::TEST_SEEDS[..2] {
        let held = session
            .keys
            .add_words(&c, &testkit::test_words(word), label, None)
            .unwrap();
        let x = session
            .keys
            .xpub_at(&c, session.keys.position(held).unwrap(), &p)
            .unwrap();
        owners.push((PublicKey::new(x.public_key), held));
    }
    let prevs: Vec<Transaction> = owners
        .iter()
        .enumerate()
        .map(|(i, (pk, _))| Transaction {
            version: Version::TWO,
            lock_time: LockTime::from_height(i as u32 + 1).unwrap(),
            input: vec![],
            output: vec![TxOut {
                value: Amount::from_sat(50_000),
                script_pubkey: ScriptBuf::new_p2wpkh(&pk.wpubkey_hash().unwrap()),
            }],
        })
        .collect();
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: prevs
            .iter()
            .map(|t| TxIn {
                previous_output: OutPoint::new(t.compute_txid(), 0),
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::new(),
            })
            .collect(),
        output: vec![TxOut {
            value: Amount::from_sat(99_000),
            script_pubkey: prevs[0].output[0].script_pubkey.clone(),
        }],
    };
    let mut inner = faraday_bip::bitcoin::Psbt::from_unsigned_tx(tx).unwrap();
    for (i, prev) in prevs.into_iter().enumerate() {
        inner.inputs[i].witness_utxo = Some(prev.output[0].clone());
        inner.inputs[i].non_witness_utxo = Some(prev);
        inner.inputs[i]
            .bip32_derivation
            .insert(owners[i].0.inner, (Fp::from(owners[i].1.0), p.clone()));
    }
    edit(&mut inner);
    faraday_psbt::Psbt::from(inner).to_bytes()
}

/// An app with `psbt` in the inbox, no wallet, the keys named by their
/// seeds loaded, the spend started and open on Signers.
fn app_with_psbt(psbt: Vec<u8>, keys: &[&str]) -> Faraday {
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(faraday_text::Text::of("spend.psbt"), psbt)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    for k in keys {
        crate::common::add_key(&mut app, k);
    }
    app.press(Action::StartSpend(0));
    app.press(Action::Step(step::SIGNERS));
    app
}

/// Two single-key owners with one input each and no wallet: each key is
/// still to sign its own input (Signs here), and once both have signed
/// nothing is left to sign, so neither reads Signs here (the spend is
/// complete, and no key signed every input, so none reads Signed).
#[test]
fn inputs_of_two_loaded_keys_say_signs_here_only_while_a_key_has_one_to_sign() {
    let mut app = app_with_psbt(two_owner(|_| {}), &["bacon", "zebra"]);
    assert_eq!(marks(&mut app), (2, 0), "{:?}", key_rows(&mut app));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::Step(step::SIGN));
    app.press(Action::SignHere);
    app.press(Action::Step(step::SIGNERS));
    assert_eq!(marks(&mut app), (0, 0), "{:?}", key_rows(&mut app));
}

/// A spend not complete, one input of a loaded key already signed in the
/// file and the other of a second loaded key: the first has nothing left to
/// sign, so only the second reads Signs here (the spend is not complete, so
/// only `to_sign` says it).
#[test]
fn a_loaded_key_with_nothing_left_to_sign_on_an_incomplete_spend_has_no_mark() {
    // Only the first key is loaded where the file is signed.
    let mut first = app_with_psbt(two_owner(|_| {}), &["bacon"]);
    testkit::steps_before_sign_done(&mut first);
    first.press(Action::Step(step::SIGN));
    first.press(Action::SignHere);
    let signed = first.spend().unwrap().spend.psbt.to_bytes();
    assert!(!first.spend().unwrap().complete, "one input is still open");
    // The signed file comes in again, both keys loaded.
    let mut app = app_with_psbt(signed, &["bacon", "zebra"]);
    assert!(!app.spend().unwrap().complete);
    assert_eq!(marks(&mut app), (1, 0), "{:?}", key_rows(&mut app));
}

/// A spend whose inputs are all final has nothing to sign: no key is
/// marked.
#[test]
fn a_spend_with_every_input_final_marks_no_key() {
    use faraday_bip::bitcoin::Witness;
    let psbt = two_owner(|p| {
        for i in &mut p.inputs {
            i.final_script_witness = Some(Witness::from_slice(&[vec![1u8; 71], vec![2u8; 33]]));
        }
    });
    let mut app = app_with_psbt(psbt, &["bacon", "zebra"]);
    assert_eq!(marks(&mut app), (0, 0), "{:?}", key_rows(&mut app));
}

/// While any key of the spend has its backup owed, Sign refuses every key,
/// so no loaded key says Signs here.
#[test]
fn no_key_says_signs_here_while_another_keys_backup_is_owed() {
    let mut app = app_with_psbt(two_owner(|_| {}), &["bacon", "zebra"]);
    assert_eq!(marks(&mut app), (2, 0));
    testkit::mark_made_here(&mut app, testkit::seed_fingerprint(0));
    app.press(Action::Step(step::SIGN));
    app.press(Action::Step(step::SIGNERS));
    assert_eq!(marks(&mut app), (0, 0), "{:?}", key_rows(&mut app));
}

/// A MuSig2 partial in the file that this device did not write is "In file",
/// not Signed, and not Signs here either.
#[test]
fn a_musig_partial_in_the_file_is_not_marked_signed() {
    let (one, _) = testkit::musig_two_inputs().unwrap();
    let policy = testkit::musig_policy().unwrap();
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("spend.psbt"), one.to_bytes()),
            (
                faraday_text::Text::of("musig-wallet.txt"),
                policy
                    .to_descriptor_checksummed()
                    .as_str()
                    .as_bytes()
                    .to_vec(),
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let at = |app: &Faraday, n: &str| app.inbox().iter().position(|i| i.name == n).unwrap();
    let w = at(&app, "musig-wallet.txt");
    testkit::load_wallet(&mut app, w);
    crate::common::add_key(&mut app, "bacon");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    app.press(Action::Step(step::SIGNERS));
    let (here, signed) = marks(&mut app);
    assert_eq!(signed, 0, "{:?}", key_rows(&mut app));
    assert_eq!(here, 0, "{:?}", key_rows(&mut app));
}

/// A fee that waits for its hold is no stop for Sign: the person holds it
/// and the key signs, so the mark is true.
#[test]
fn a_fee_waiting_for_its_hold_leaves_signs_here() {
    let psbt = two_owner(|p| {
        p.unsigned_tx.output[0].value = faraday_bip::bitcoin::Amount::from_sat(10_000)
    });
    let mut app = app_with_psbt(psbt, &["bacon", "zebra"]);
    assert_eq!(marks(&mut app), (2, 0), "{:?}", key_rows(&mut app));
}

/// A warning that refuses the whole PSBT (a blocked one) leaves no key
/// saying Signs here, though each still has an input it could sign.
#[test]
fn no_key_says_signs_here_while_a_warning_blocks_sign() {
    let psbt = two_owner(|p| {
        let utxo = p.inputs[1].witness_utxo.as_mut().unwrap();
        utxo.value = faraday_bip::bitcoin::Amount::from_sat(1);
    });
    let mut app = app_with_psbt(psbt, &["bacon", "zebra"]);
    assert_eq!(marks(&mut app), (0, 0), "{:?}", key_rows(&mut app));
}

/// A key loaded of no interest to the spend that owes its backup does not
/// stop the keys of the spend: only a key the spend uses does.
#[test]
fn a_waiting_key_the_spend_does_not_use_leaves_signs_here() {
    let mut app = app_with_psbt(two_owner(|_| {}), &["bacon", "zebra", "summer"]);
    testkit::mark_made_here(&mut app, testkit::seed_fingerprint(2));
    assert_eq!(marks(&mut app), (2, 0), "{:?}", key_rows(&mut app));
}

fn hex(f: faraday_bip::keys::Fingerprint) -> String {
    f.0.iter().map(|b| format!("{b:02x}")).collect()
}

/// A 2-of-3 whose second slot is key 1's fingerprint with another key
/// (so Review stops with a wallet not registered once key 1 is loaded): the keys loaded
/// are listed once each, and with key 1 loaded at Signers no key reads
/// Signs here, since Sign refuses the PSBT.
#[test]
fn a_wallet_slot_under_a_loaded_keys_fingerprint_with_another_key_marks_nothing() {
    let k = kit("savings");
    let (unsigned, _) = testkit::spend_of(&k).unwrap();
    let (one, two) = (
        hex(testkit::seed_fingerprint(0)),
        hex(testkit::seed_fingerprint(1)),
    );
    let descriptor = k
        .descriptor
        .split('#')
        .next()
        .unwrap()
        .replace(&format!("[{two}/"), &format!("[{one}/"));
    assert_ne!(descriptor, k.descriptor);
    let mut app = testkit::started();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![
            (faraday_text::Text::of("spend.psbt"), unsigned.to_bytes()),
            (
                faraday_text::Text::of("savings-wallet.txt"),
                descriptor.into_bytes(),
            ),
        ]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    let at = |app: &Faraday, n: &str| app.inbox().iter().position(|i| i.name == n).unwrap();
    let w = at(&app, "savings-wallet.txt");
    testkit::load_wallet(&mut app, w);
    crate::common::add_key(&mut app, "summer");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::Step(step::SIGNERS));
    // Loading a key leaves the spend; it is opened again.
    crate::common::add_key(&mut app, "bacon");
    app.press(Action::StartSpend(at(&app, "spend.psbt")));
    app.press(Action::Step(step::SIGNERS));
    // Sign refuses the PSBT (wallet not registered): no row says Signs
    // here, and a fingerprint is listed once.
    assert!(
        app.spend().unwrap().inspection.warnings.iter().any(|w| {
            w.level == faraday_psbt::Level::Blocked
                && w.kind == faraday_psbt::WarningKind::WalletNotRegistered
        }),
        "no blocking warning"
    );
    let rows = key_rows(&mut app);
    assert_eq!(rows.len(), 2, "one row per fingerprint: {rows:?}");
    assert!(
        rows.iter().all(|r| !r.ends_with(" · Signs here")),
        "{rows:?}"
    );
}

/// A third input, a one-key P2WSH script of no loaded wallet naming key 1
/// (a caution only), which the signer leaves unsigned.
fn two_owner_and_script() -> Vec<u8> {
    use faraday_bip::bitcoin::{
        Amount, OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness,
        absolute::LockTime, opcodes::all::OP_CHECKSIG, script::Builder, transaction::Version,
    };
    two_owner(|p| {
        let (pk, (fp, path)) = p.inputs[0]
            .bip32_derivation
            .iter()
            .next()
            .map(|(k, v)| (*k, v.clone()))
            .unwrap();
        let script = Builder::new()
            .push_key(&faraday_bip::bitcoin::PublicKey::new(pk))
            .push_opcode(OP_CHECKSIG)
            .into_script();
        let spk = ScriptBuf::new_p2wsh(&script.wscript_hash());
        let prev = Transaction {
            version: Version::TWO,
            lock_time: LockTime::from_height(7).unwrap(),
            input: vec![],
            output: vec![TxOut {
                value: Amount::from_sat(50_000),
                script_pubkey: spk,
            }],
        };
        p.unsigned_tx.input.push(TxIn {
            previous_output: OutPoint::new(prev.compute_txid(), 0),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        });
        p.unsigned_tx.output[0].value = Amount::from_sat(149_000);
        let mut input = faraday_bip::bitcoin::psbt::Input {
            witness_utxo: Some(prev.output[0].clone()),
            non_witness_utxo: Some(prev),
            witness_script: Some(script),
            ..Default::default()
        };
        input.bip32_derivation.insert(pk, (fp, path));
        p.inputs.push(input);
    })
}

/// Key 1 signed what it could in this session (input 0; the signer leaves
/// the script input, of no wallet, unsigned): it does not read Signs here
/// while the spend is kept open, though `to_sign` still lists it for the
/// script input.
#[test]
fn a_key_that_signed_here_does_not_read_signs_here_for_an_input_the_signer_skips() {
    let mut app = app_with_psbt(two_owner_and_script(), &["bacon", "zebra"]);
    app.press(Action::Step(step::SIGNERS));
    assert_eq!(marks(&mut app), (2, 0), "{:?}", key_rows(&mut app));
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::Step(step::SIGN));
    app.press(Action::SignHere);
    assert!(
        !app.spend().unwrap().spend.signed_here.is_empty(),
        "nothing was signed"
    );
    app.press(Action::Step(step::SIGNERS));
    // No row says Signs here or Signed.
    assert_eq!(marks(&mut app), (0, 0), "{:?}", key_rows(&mut app));
}

/// A single-key wallet has no Signers step: once Sign is reached its key
/// stands under Sign's row, marked as a multisig's key is; before that, and
/// in the review, there is no row.
#[test]
fn a_single_key_wallet_lists_its_key_under_sign_marked_signs_here() {
    let mut app = spending_review(false);
    assert!(
        !app.spend().unwrap().steps.contains(&step::SIGNERS),
        "a single-key wallet has no Signers step"
    );
    assert!(
        key_rows(&mut app).is_empty(),
        "no row before Sign: the review"
    );
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::Step(step::SIGN));
    let rows = key_rows(&mut app);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(rows[0].ends_with(" · Signs here"), "{rows:?}");
}

/// A single-key wallet whose key is not loaded here has a row with no mark.
#[test]
fn a_single_key_wallet_whose_key_is_not_loaded_has_no_mark() {
    let mut app = spending_with(false, false);
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::Step(step::SIGN));
    let rows = key_rows(&mut app);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(
        !rows[0].contains("Signs here") && !rows[0].contains("Signed"),
        "{rows:?}"
    );
}

/// After the key signs, its row reads Signed, and the row stays.
#[test]
fn a_single_key_wallet_reads_signed_after_it_signs() {
    let mut app = spending_review(false);
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::Step(step::SIGN));
    app.press(Action::SignHere);
    let rows = key_rows(&mut app);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(rows[0].ends_with(" · Signed"), "{rows:?}");
}

/// Every key row draws its fingerprint whole, and a marked row its mark.
#[test]
fn a_key_row_draws_its_fingerprint_and_its_mark_whole() {
    let mut app = spending_review(false);
    testkit::steps_before_sign_done(&mut app);
    app.press(Action::Step(step::SIGN));
    let rows = key_rows(&mut app);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(rows[0].ends_with(" · Signs here"), "{rows:?}");
    // A multisig's rows are drawn the same way.
    let mut app = savings_spend();
    app.press(Action::Step(step::SIGNERS));
    let rows = key_rows(&mut app);
    assert_eq!(rows.len(), 3, "{rows:?}");
    signing_row(&rows);
}
