//! A seed's Krux code on the backup checklist (`docs/KEF.md` §15.1): the
//! item is listed for a seed whose code is on a stick, reads Not made, Not
//! checked and Checked as the code is made and opened back with its
//! passphrase, and a wrong passphrase leaves it Not checked.

use crate::common::{Disk, Shell, board, open_item, take_off, type_text};
use faraday_core::backup_flow::Step;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::board::{BoardPress as BP, Chip, Thing, Verb, Zone};
use faraday_core::kef_form::KefAct;
use faraday_core::kef_tool::KefToolAction as K;
use faraday_core::plan::{Item, Preset};
use faraday_core::seeds::SeedsAction as S;
use faraday_core::{Action, Faraday, Screen, bstep, qstep, testkit};
use faraday_shell_api::{App, EntropyBytes, Event, Key};

/// A one-key wallet whose plan keeps seed A's Krux code on a new stick,
/// at the checklist.
fn at_the_checklist() -> Faraday {
    at_the_checklist_on(1)
}

/// As [`at_the_checklist`], the code on `sticks` new sticks.
fn at_the_checklist_on(sticks: usize) -> Faraday {
    let mut app = testkit::started();
    for i in 0..8u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x40 + i; 32])));
    }
    app.press(Action::Entry(None));
    type_text(&mut app, &testkit::test_words(testkit::TEST_SEEDS[1].0));
    app.press(Action::EntryAdd);
    let fp = app.session.keys.last().unwrap().fingerprint;
    app.press(Action::KeyWallet(fp.0, 1));
    app.press(Action::Seeds(S::Make));
    let w = app.session.wallets.len() - 1;
    app.press(Action::Backup(w));
    plan_the_code(&mut app, sticks);
    app
}

/// The plan, from the standard paper one: seed A's Krux code on `sticks`
/// new sticks and nothing else of it, then the checklist.
fn plan_the_code(app: &mut Faraday, sticks: usize) {
    testkit::backup_as(app, Preset::Paper);
    board(app, BP::Menu(Chip::List(Thing::Seed(0))));
    board(app, BP::MenuVerb(Verb::Kef));
    board(app, BP::MenuTo(Zone::NewStick(0)));
    for _ in 1..sticks {
        board(app, BP::Menu(Chip::List(Thing::Seed(0))));
        board(app, BP::MenuVerb(Verb::Kef));
        board(app, BP::MenuTo(Zone::NewStick(0)));
    }
    // The words are kept in the vault of another backup: only the code.
    take_off(app, Zone::Paper(0), Thing::Seed(0));
    app.press(Action::BQ(qstep::PLAN));
    app.press(Action::BChecklist);
    if app
        .backup_items_now(&testkit::compute())
        .contains(&bstep::BLANK)
    {
        app.press(Action::BNext(bstep::BLANK));
    }
    app.settle();
}

fn done(app: &Faraday) -> bool {
    app.backup_item_done_now(&testkit::compute(), Item::KefCode(0))
}

/// The status the item's row reads once the card is put away, the card
/// opened again after.
fn status(app: &mut Faraday) -> &'static str {
    if app.backup().and_then(|b| b.open) == Some(bstep::KEF) {
        app.press(Action::BStep(bstep::KEF));
    }
    let texts = crate::common::texts(app);
    let row = texts
        .iter()
        .position(|t| t.starts_with("Krux code of"))
        .expect("the item's row");
    let read = ["Not made", "Not checked", "Checked"]
        .into_iter()
        .find(|s| texts[row + 1..].iter().take(3).any(|t| t == s))
        .unwrap_or("none");
    open_item(app, bstep::KEF);
    read
}

fn type_in(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn make(app: &mut Faraday) {
    app.press(Action::KefTool(K::Backup(KefAct::Options)));
    app.press(Action::KefTool(K::Backup(KefAct::Rounds)));
    for _ in 0..8 {
        app.event(Event::Key(Key::Backspace));
    }
    type_in(app, "10000");
    app.press(Action::KefTool(K::Backup(KefAct::Back)));
    app.press(Action::KefTool(K::Backup(KefAct::Field)));
    type_in(app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::Make)));
    testkit::settle(app);
}

fn check_with(app: &mut Faraday, passphrase: &str) {
    app.press(Action::KefTool(K::Backup(KefAct::Check)));
    type_in(app, passphrase);
    app.press(Action::KefTool(K::Backup(KefAct::Compare)));
    testkit::settle(app);
}

#[test]
fn the_item_is_listed_for_a_seed_with_a_code_on_a_stick() {
    let c = testkit::compute();
    let app = at_the_checklist();
    let items = app.backup_items_now(&c);
    assert!(items.contains(&bstep::KEF), "{items:?}");
    if let Some(sheets) = items.iter().position(|n| *n == bstep::SHEETS) {
        assert!(items.iter().position(|n| *n == bstep::KEF) < Some(sheets));
    }
    assert!(!done(&app));
}

#[test]
fn the_item_reads_not_checked_once_made_and_checked_once_opened_back() {
    let mut app = at_the_checklist();
    assert_eq!(status(&mut app), "Not made");
    open_item(&mut app, bstep::KEF);
    assert_eq!(status(&mut app), "Not made");
    make(&mut app);
    assert!(!done(&app), "made, not checked");
    assert_eq!(status(&mut app), "Not checked");
    check_with(&mut app, "a passphrase");
    assert!(done(&app));
    assert_eq!(status(&mut app), "Checked");
    // Put away and opened again, it is still checked.
    app.press(Action::BStep(bstep::KEF));
    open_item(&mut app, bstep::KEF);
    assert!(done(&app));
    assert_eq!(status(&mut app), "Checked");
}

#[test]
fn a_wrong_passphrase_leaves_it_not_checked() {
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "another one");
    assert!(!done(&app));
    assert_eq!(status(&mut app), "Not checked");
}

#[test]
fn back_at_the_plan_typing_makes_no_code() {
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    let sent = app.outbox.len();
    app.press(Action::BPlan);
    type_in(&mut app, "Home");
    app.event(Event::Key(Key::Enter));
    testkit::settle(&mut app);
    assert_eq!(app.outbox.len(), sent, "nothing new in the Outbox");
    app.press(Action::BChecklist);
    assert_eq!(status(&mut app), "Not made");
}

#[test]
fn leaving_the_item_or_the_checklist_wipes_the_passphrase() {
    for leave in [Action::BStep(bstep::KEF), Action::BPlan] {
        let mut app = at_the_checklist();
        open_item(&mut app, bstep::KEF);
        app.press(Action::KefTool(K::Backup(KefAct::Field)));
        type_in(&mut app, "typed and left");
        app.press(leave);
        app.press(Action::BChecklist);
        open_item(&mut app, bstep::KEF);
        make(&mut app);
        check_with(&mut app, "a passphrase");
        assert!(done(&app), "{leave:?}: the old text is gone");
    }
}

fn kef_files(app: &Faraday) -> Vec<String> {
    app.outbox
        .iter()
        .filter(|i| i.name.starts_with("krux-v"))
        .map(|i| i.name.clone())
        .collect()
}

/// What each stick of the lock holds, by slot.
fn stick_lists(app: &Faraday) -> Vec<Vec<String>> {
    app.flow()
        .expect("the lock's flow")
        .slots
        .iter()
        .map(|s| s.files.clone())
        .collect()
}

fn to_lock(app: &mut Faraday, shell: &mut Shell) {
    testkit::type_passphrases(app);
    if app
        .backup_items_now(&testkit::compute())
        .contains(&bstep::ENVELOPE)
    {
        app.press(Action::BNext(bstep::ENVELOPE));
    }
    shell.flow(app, F::WalletLock);
}

#[test]
fn the_sticks_holding_the_code_list_its_two_files_at_lock_and_write() {
    let mut shell = Shell::new();
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    let names = kef_files(&app);
    assert_eq!(names.len(), 2, "{names:?}");
    to_lock(&mut app, &mut shell);
    let lists = stick_lists(&app);
    let holding: Vec<_> = lists
        .iter()
        .filter(|l| names.iter().all(|n| l.contains(n)))
        .collect();
    assert_eq!(holding.len(), 1, "{lists:?}");
    assert!(
        lists
            .iter()
            .all(|l| names.iter().all(|n| l.contains(n)) || names.iter().all(|n| !l.contains(n))),
        "never one file without the other: {lists:?}"
    );
}

#[test]
fn a_code_not_made_holds_the_lock() {
    let mut shell = Shell::new();
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    assert!(!done(&app));
    to_lock(&mut app, &mut shell);
    assert!(app.flow().is_none(), "no lock while the code is not made");
    assert!(!app.restart_requested());
}

/// Locks and writes the backup to the sticks of `shell`; what they hold.
fn write_sticks(app: &mut Faraday, shell: &mut Shell) -> Vec<(String, Vec<u8>)> {
    for k in 1..=4u32 {
        let mut d = Disk::empty(&format!("S{k}"), "KEYSTICK");
        d.serial = Some(k);
        shell.add(d);
    }
    to_lock(app, shell);
    shell.pump(app);
    assert!(app.restart_requested(), "locked: {:?}", app.toast_text());
    *app = shell.restart();
    let n = app.flow().unwrap().slots.len();
    for k in 1..=n {
        let d = format!("S{k}");
        shell.plug(app, &d);
        for _ in 0..3 {
            let Some(r) = app.flow().and_then(|f| f.live.reading.as_ref()) else {
                break;
            };
            if r.problem.is_some() || r.bound {
                break;
            }
            let a = if r.hold { F::Hold } else { F::Tap };
            shell.flow(app, a);
        }
        shell.pull(app, &d);
    }
    assert_eq!(app.flow().unwrap().step, Step::Done);
    (1..=n)
        .flat_map(|k| shell.disk(&format!("S{k}")).files.clone())
        .collect()
}

/// The picture on the sticks reads as the base43 text of the `.kef` file.
fn picture_reads_as_file(disk: &[(String, Vec<u8>)]) {
    let c = testkit::compute();
    let png = disk.iter().find(|(f, _)| f.ends_with(".png")).unwrap();
    let file = disk.iter().find(|(f, _)| f.ends_with(".kef")).unwrap();
    let read = faraday_files::qr_in_png(&c, &png.1).unwrap().to_vec();
    assert_eq!(
        read,
        vec![faraday_codec::kef::base43_encode(&file.1).into_bytes()]
    );
}

#[test]
fn both_files_reach_the_stick_and_the_picture_reads_back() {
    let mut shell = Shell::new();
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    let names = kef_files(&app);
    let disk = write_sticks(&mut app, &mut shell);
    for n in &names {
        assert!(disk.iter().any(|(f, _)| f == n), "{n} on a stick");
    }
    picture_reads_as_file(&disk);
}

#[test]
fn a_code_already_written_once_goes_on_the_next_sticks_too() {
    let mut shell = Shell::new();
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    let names = kef_files(&app);
    let right = app
        .outbox
        .iter()
        .find(|i| i.name.ends_with(".png"))
        .unwrap()
        .bytes
        .clone();
    for i in app.outbox.iter_mut().filter(|i| i.name.ends_with(".png")) {
        i.bytes = vec![0; 8];
    }
    testkit::outbox_to_files(&mut app, &names);
    reopen(&mut app);
    assert!(done(&app));
    let disk = write_sticks(&mut app, &mut shell);
    for n in &names {
        assert!(disk.iter().any(|(f, _)| f == n), "{n} on a stick");
    }
    let png = disk.iter().find(|(f, _)| f.ends_with(".png")).unwrap();
    assert_eq!(png.1, right, "the picture is the code's own");
    picture_reads_as_file(&disk);
}

#[test]
fn another_code_of_the_same_name_made_later_is_left_alone_and_not_put_on_the_stick() {
    let mut shell = Shell::new();
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    let names = kef_files(&app);
    let mut others: Vec<_> = app
        .outbox
        .iter()
        .filter(|i| names.contains(&i.name))
        .cloned()
        .collect();
    testkit::outbox_to_files(&mut app, &names);
    reopen(&mut app);
    for i in &mut others {
        let last = i.bytes.len() - 1;
        i.bytes[last] ^= 1;
    }
    let other = others.clone();
    app.outbox.extend(others);
    assert!(!done(&app), "Not made while another file holds the names");
    to_lock(&mut app, &mut shell);
    assert!(app.flow().is_none(), "Lock and write is refused");
    for o in &other {
        let now = app.outbox.iter().find(|i| i.name == o.name).unwrap();
        assert_eq!(now.bytes, o.bytes, "{} untouched", o.name);
    }
    // Made again, the new code is put beside the other, which stays.
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    assert!(done(&app));
    let disk = write_sticks(&mut app, &mut shell);
    let file = disk.iter().find(|(f, _)| f.ends_with(".kef")).unwrap();
    assert_ne!(file.1, other[0].bytes, "not the other code");
    for o in &other {
        assert!(
            app.outbox
                .iter()
                .any(|i| i.name == o.name && i.bytes == o.bytes),
            "{} is still there",
            o.name
        );
    }
    picture_reads_as_file(&disk);
}

#[test]
fn a_second_stick_holding_the_code_gets_both_files_too() {
    let mut shell = Shell::new();
    let mut app = at_the_checklist_on(2);
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    let names = kef_files(&app);
    to_lock(&mut app, &mut shell);
    let lists = stick_lists(&app);
    let holding = lists
        .iter()
        .filter(|l| names.iter().all(|n| l.contains(n)))
        .count();
    assert_eq!(holding, 2, "{lists:?}");
}

#[test]
fn the_item_and_the_stick_lists_stay_when_the_backup_is_opened_again() {
    let mut shell = Shell::new();
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    let w = app.backup().unwrap().wallet;
    faraday_core::testkit::backup_drop(&mut app);
    // The unlock brings the plan back with the vault; here it is made again.
    app.press(Action::Backup(w));
    plan_the_code(&mut app, 1);
    assert!(done(&app), "Checked after the backup was dropped");
    let names = kef_files(&app);
    to_lock(&mut app, &mut shell);
    assert!(
        stick_lists(&app)
            .iter()
            .any(|l| names.iter().all(|n| l.contains(n))),
        "the files are still listed"
    );
}

/// What the lock does to the backup under way, the plan made again as the
/// unlock brings it back.
fn reopen(app: &mut Faraday) {
    let w = app.backup().unwrap().wallet;
    testkit::backup_drop(app);
    app.press(Action::Backup(w));
    plan_the_code(app, 1);
}

#[test]
fn a_code_closed_before_it_is_checked_is_read_back_from_its_file() {
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    let before = kef_files(&app);
    reopen(&mut app);
    assert!(!done(&app));
    assert_eq!(status(&mut app), "Not checked");
    // The Code card is back with its Check my code, and no passphrase was
    // asked for it.
    check_with(&mut app, "a passphrase");
    assert!(done(&app));
    assert_eq!(kef_files(&app), before, "the same two files");
}

#[test]
fn a_code_whose_file_is_gone_reads_not_made_and_offers_the_first_step() {
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    reopen(&mut app);
    app.outbox.retain(|i| !i.name.ends_with(".kef"));
    assert!(!done(&app));
    assert_eq!(status(&mut app), "Not made");
    make(&mut app);
    assert_eq!(status(&mut app), "Not checked");
}

#[test]
fn a_record_kept_before_the_code_files_were_named_still_loads() {
    let mut shell = Shell::new();
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    to_lock(&mut app, &mut shell);
    shell.pump(&mut app);
    let _ = shell.restart();
    let (inbox, outbox, mut kept) = shell.boxes.clone();
    let mut stripped = 0;
    for (n, bytes) in kept.iter_mut() {
        if n == "backups" {
            let text = String::from_utf8_lossy(bytes).to_string();
            // The line as it was kept before the hash and the options.
            let old: Vec<String> = text
                .lines()
                .map(|l| {
                    if l.starts_with("kef\t") {
                        stripped += 1;
                        l.split('\t').take(5).collect::<Vec<_>>().join("\t")
                    } else {
                        l.to_string()
                    }
                })
                .collect();
            *bytes = format!("{}\n", old.join("\n")).into_bytes();
        }
    }
    assert_eq!(stripped, 1, "the record named the code");
    let mut app = crate::common::process(inbox, outbox, kept);
    app.press(Action::Nav(Screen::Backups));
    let texts = crate::common::texts(&mut app);
    assert!(
        texts.iter().any(|t| t.starts_with("Backup of")),
        "the wallet's record still reads: {texts:?}"
    );
}

#[test]
fn a_code_made_and_not_checked_holds_the_lock() {
    let mut shell = Shell::new();
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    assert!(!done(&app));
    to_lock(&mut app, &mut shell);
    assert!(app.flow().is_none(), "no lock before the code is checked");
    assert!(!app.restart_requested());
}

#[test]
fn a_code_written_to_the_sticks_still_reads_checked_and_shows_its_code() {
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    let card = testkit::kef_card_png(&app, 0).expect("the picture");
    let names = kef_files(&app);
    testkit::outbox_to_files(&mut app, &names);
    assert!(kef_files(&app).is_empty());
    reopen(&mut app);
    assert!(done(&app), "Checked after the sticks were written");
    assert_eq!(status(&mut app), "Checked");
    assert_eq!(
        testkit::kef_card_png(&app, 0),
        Some(card),
        "the same Code card"
    );
    // The record kept its line through the visit.
    reopen(&mut app);
    assert!(done(&app));
}

#[test]
fn another_code_under_the_same_name_is_not_this_one() {
    let mut shell = Shell::new();
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    reopen(&mut app);
    assert!(done(&app));
    for i in app.outbox.iter_mut().filter(|i| i.name.ends_with(".kef")) {
        let last = i.bytes.len() - 1;
        i.bytes[last] ^= 1;
    }
    assert!(!done(&app));
    assert_eq!(status(&mut app), "Not made");
    to_lock(&mut app, &mut shell);
    assert!(app.flow().is_none(), "nothing of it is listed for a stick");
}

#[test]
fn a_code_with_one_of_its_files_gone_reads_not_made() {
    for gone in [".kef", ".png"] {
        let mut app = at_the_checklist();
        open_item(&mut app, bstep::KEF);
        make(&mut app);
        check_with(&mut app, "a passphrase");
        app.outbox.retain(|i| !i.name.ends_with(gone));
        assert!(!done(&app), "{gone} gone, form open");
        reopen(&mut app);
        app.outbox.retain(|i| !i.name.ends_with(gone));
        assert!(!done(&app), "{gone} gone, reopened");
    }
}

#[test]
fn a_code_made_with_other_options_reopens_to_the_same_picture() {
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    app.press(Action::KefTool(K::Backup(KefAct::Options)));
    app.press(Action::KefTool(K::Backup(KefAct::Version(5))));
    app.press(Action::KefTool(K::Backup(KefAct::Low)));
    app.press(Action::KefTool(K::Backup(KefAct::Id(
        faraday_core::kef_form::IdKind::Label,
    ))));
    type_in(&mut app, "mine");
    app.press(Action::KefTool(K::Backup(KefAct::Back)));
    app.press(Action::KefTool(K::Backup(KefAct::Field)));
    type_in(&mut app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::Make)));
    testkit::settle(&mut app);
    let made = testkit::kef_card_png(&app, 0).expect("the picture");
    reopen(&mut app);
    open_item(&mut app, bstep::KEF);
    assert_eq!(testkit::kef_card_png(&app, 0), Some(made));
}

/// As [`make`], the code's id the wallet's fingerprint, so that two wallets
/// of one seed name their codes alike.
fn make_by_fingerprint(app: &mut Faraday) {
    app.press(Action::KefTool(K::Backup(KefAct::Options)));
    app.press(Action::KefTool(K::Backup(KefAct::Id(
        faraday_core::kef_form::IdKind::Fingerprint,
    ))));
    app.press(Action::KefTool(K::Backup(KefAct::Rounds)));
    for _ in 0..8 {
        app.event(Event::Key(Key::Backspace));
    }
    type_in(app, "10000");
    app.press(Action::KefTool(K::Backup(KefAct::Back)));
    app.press(Action::KefTool(K::Backup(KefAct::Field)));
    type_in(app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::Make)));
    testkit::settle(app);
}

#[test]
fn a_second_wallet_of_the_same_seed_keeps_the_first_wallets_code_in_files() {
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make_by_fingerprint(&mut app);
    check_with(&mut app, "a passphrase");
    let first = app.backup().unwrap().wallet;
    let first_names = kef_files(&app);
    testkit::outbox_to_files(&mut app, &first_names);
    testkit::backup_drop(&mut app);
    // The same seed in a second wallet, its code made and locked in turn.
    let fp = app.session.keys.last().unwrap().fingerprint;
    app.press(Action::KeyWallet(fp.0, 1));
    app.press(Action::Seeds(S::Shape));
    app.press(Action::Seeds(S::Account(true)));
    app.press(Action::Seeds(S::Make));
    let second = app.session.wallets.len() - 1;
    assert_ne!(first, second);
    app.press(Action::Backup(second));
    plan_the_code(&mut app, 1);
    open_item(&mut app, bstep::KEF);
    make_by_fingerprint(&mut app);
    check_with(&mut app, "a passphrase");
    let second_names = kef_files(&app);
    assert_eq!(second_names.len(), 2, "{second_names:?}");
    assert!(
        second_names.iter().all(|n| !first_names.contains(n)),
        "the second code goes beside the first: {first_names:?} {second_names:?}"
    );
    testkit::outbox_to_files(&mut app, &second_names);
    let in_files = app
        .inbox()
        .iter()
        .filter(|i| i.name.starts_with("krux-v"))
        .count();
    assert_eq!(in_files, 4, "both codes are in Files");
    assert!(done(&app), "the second wallet's item");
    testkit::backup_drop(&mut app);
    app.press(Action::Backup(first));
    plan_the_code(&mut app, 1);
    assert!(done(&app), "the first wallet's item still reads its status");
    assert_eq!(status(&mut app), "Checked");
}
