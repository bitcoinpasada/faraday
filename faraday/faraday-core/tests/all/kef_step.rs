//! A seed's Krux code on the backup checklist (`docs/KEF.md` §15.1): the
//! item is listed for a seed whose code is on a stick, reads Not made, Not
//! checked and Checked as the code is made and opened back with its
//! passphrase, and a wrong passphrase leaves it Not checked.

use crate::common::{Disk, Shell, board, open_item, take_off, type_text};
use faraday_core::backup_flow::Step;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::board::{BoardPress as BP, Chip, Thing, Verb, Zone};
use faraday_core::dice_entry::Press;
use faraday_core::kef_form::KefAct;
use faraday_core::kef_tool::KefToolAction as K;
use faraday_core::plan::{Item, Preset};
use faraday_core::seeds::SeedsAction as S;
use faraday_core::{Action, Faraday, Screen, StorageEvent, bstep, qstep, testkit};
use faraday_shell_api::{App, EntropyBytes, Event, Key};

/// A one-key wallet whose plan keeps seed A's Krux code on a new stick,
/// at the checklist.
pub(crate) fn at_the_checklist() -> Faraday {
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
    app.press(Action::KefTool(K::Backup(KefAct::FieldAgain)));
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
        app.press(Action::KefTool(K::Backup(KefAct::FieldAgain)));
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
    app.press(Action::KefTool(K::Backup(KefAct::FieldAgain)));
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
    app.press(Action::KefTool(K::Backup(KefAct::FieldAgain)));
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

#[test]
fn a_code_goes_beside_files_holding_only_the_picture_of_an_earlier_one_under_one_stem() {
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make_by_fingerprint(&mut app);
    check_with(&mut app, "a passphrase");
    let first_names = kef_files(&app);
    testkit::outbox_to_files(&mut app, &first_names);
    // Only the picture stays in Files.
    testkit::inbox_retain(&mut app, |i| !i.name.ends_with(".kef"));
    testkit::backup_drop(&mut app);
    let fp = app.session.keys.last().unwrap().fingerprint;
    app.press(Action::KeyWallet(fp.0, 1));
    app.press(Action::Seeds(S::Shape));
    app.press(Action::Seeds(S::Account(true)));
    app.press(Action::Seeds(S::Make));
    let second = app.session.wallets.len() - 1;
    app.press(Action::Backup(second));
    plan_the_code(&mut app, 1);
    open_item(&mut app, bstep::KEF);
    make_by_fingerprint(&mut app);
    let mut names = kef_files(&app);
    names.sort();
    assert_eq!(names.len(), 2, "{names:?}");
    let stems: Vec<&str> = names
        .iter()
        .map(|n| n.rsplit_once('.').map_or(n.as_str(), |(s, _)| s))
        .collect();
    assert_eq!(stems[0], stems[1], "one stem: {names:?}");
    assert!(stems[0].ends_with("-2"), "{names:?}");
}

/// As [`at_the_checklist`], the code on Paper at the first place and on
/// no stick, with `camera` or none.
fn on_paper(camera: bool) -> Faraday {
    let mut app = at_the_checklist();
    reopen_on_paper(&mut app, true);
    app.storage(StorageEvent::Cameras(if camera {
        vec![("cam".to_string(), "Camera".to_string())]
    } else {
        Vec::new()
    }));
    app
}

/// The plan made again with the code on Paper and, if `stick`, on a new
/// stick.
fn reopen_on_paper(app: &mut Faraday, paper_only: bool) {
    let w = app.backup().unwrap().wallet;
    testkit::backup_drop(app);
    app.press(Action::Backup(w));
    testkit::backup_as(app, Preset::Paper);
    board(app, BP::Menu(Chip::List(Thing::Seed(0))));
    board(app, BP::MenuVerb(Verb::Kef));
    board(app, BP::MenuTo(Zone::Paper(0)));
    if !paper_only {
        board(app, BP::Menu(Chip::List(Thing::Seed(0))));
        board(app, BP::MenuVerb(Verb::Kef));
        board(app, BP::MenuTo(Zone::NewStick(0)));
    }
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

/// Whether `text` is drawn, scrolling down to it.
fn seen(app: &mut Faraday, text: &str) -> bool {
    for _ in 0..30 {
        if crate::common::texts(app).iter().any(|t| t == text) {
            return true;
        }
        app.event(Event::Scroll {
            x: 600,
            y: 400,
            dy: 120,
        });
    }
    false
}

#[test]
fn the_code_on_paper_shows_its_size_the_sheet_and_a_grid_whose_mark_is_kept() {
    let mut app = on_paper(true);
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    for t in ["Size", "By hand"] {
        assert!(
            crate::common::texts(&mut app)
                .iter()
                .any(|x| x.starts_with(t)),
            "{t}"
        );
    }
    let texts = crate::common::texts(&mut app);
    assert!(
        texts.iter().any(|t| t.starts_with("By hand: Sheet: Faraday Starter, Blank templates, Krux code, ")),
        "{texts:?}"
    );
    assert!(seen(&mut app, "Blocks"), "{texts:?}");
    assert!(seen(&mut app, "Rows"));
    assert!(seen(&mut app, "Block A1"));
    let mark = |app: &mut Faraday, n: usize| {
        app.press(Action::KefTool(K::Backup(KefAct::Pin(n))));
        crate::common::texts(app)
            .into_iter()
            .find(|t| t.starts_with("Block ") && t != "Block A1")
            .expect("the mark moved")
    };
    let before = mark(&mut app, 36);
    // A 33 x 33 code has blocks past the 36th, and the last can be marked.
    let marked = mark(&mut app, 48);
    assert_ne!(marked, before);
    // Closed and opened again, the mark stands.
    app.press(Action::BStep(bstep::KEF));
    open_item(&mut app, bstep::KEF);
    assert!(seen(&mut app, &marked), "{marked}");
}

#[test]
fn on_paper_with_a_camera_the_card_offers_scan_my_copy_and_not_check_my_code() {
    let mut app = on_paper(true);
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    assert!(seen(&mut app, "Scan my copy"));
    assert!(!seen(&mut app, "Check my code"));
    assert!(!done(&app), "not checked until the drawn copy is scanned");
    assert!(seen(&mut app, "Drawn copy: Not scanned"));
    // The code made stays Not checked, never Not made, when the item closes.
    assert_eq!(status(&mut app), "Not checked");
    let mut shell = Shell::new();
    to_lock(&mut app, &mut shell);
    assert!(app.flow().is_none(), "the lock is held");
}

#[test]
fn a_code_checked_with_no_camera_stays_checked_when_a_camera_is_plugged_in() {
    let mut app = on_paper(false);
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    assert!(seen(&mut app, "Check my code"));
    check_with(&mut app, "a passphrase");
    assert!(done(&app));
    app.storage(StorageEvent::Cameras(vec![(
        "cam".to_string(),
        "Camera".to_string(),
    )]));
    assert!(done(&app));
    assert_eq!(status(&mut app), "Checked");
    let w = app.backup().unwrap().wallet;
    testkit::backup_drop(&mut app);
    app.press(Action::Backup(w));
    app.press(Action::BChecklist);
    app.settle();
    assert!(done(&app));
}

#[test]
fn check_my_code_alone_on_paper_ends_the_item_with_no_camera() {
    let mut app = on_paper(false);
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    assert!(done(&app));
    open_item(&mut app, bstep::KEF);
    assert!(seen(&mut app, "Drawn copy: Not scanned"));
}

#[test]
fn not_on_paper_takes_the_code_off_every_place_and_the_item_goes_with_no_stick_copy() {
    let mut app = on_paper(true);
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    app.press(Action::KefTool(K::Backup(KefAct::NotOnPaper)));
    assert!(
        !app.backup_items_now(&testkit::compute())
            .contains(&bstep::KEF),
        "no copy left anywhere"
    );
    // The form is put away: the checklist moved on and typing goes nowhere.
    assert_ne!(app.backup().and_then(|b| b.open), Some(bstep::KEF));
    // With a stick copy too, the item stays.
    let mut app = at_the_checklist();
    reopen_on_paper(&mut app, false);
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    app.press(Action::KefTool(K::Backup(KefAct::NotOnPaper)));
    assert!(
        app.backup_items_now(&testkit::compute())
            .contains(&bstep::KEF)
    );
    assert!(!seen(&mut app, "Not on Paper"));
}

/// The code made with a label of `n` characters, a side of its own.
fn make_labelled(app: &mut Faraday, n: usize) {
    app.press(Action::KefTool(K::Backup(KefAct::Options)));
    app.press(Action::KefTool(K::Backup(KefAct::Id(
        faraday_core::kef_form::IdKind::Label,
    ))));
    app.press(Action::KefTool(K::Backup(KefAct::Label)));
    type_in(app, &"x".repeat(n));
    app.press(Action::KefTool(K::Backup(KefAct::Back)));
    app.press(Action::KefTool(K::Backup(KefAct::Field)));
    type_in(app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::FieldAgain)));
    type_in(app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::Make)));
    testkit::settle(app);
}

#[test]
fn the_templates_item_names_the_krux_code_sheet_at_the_codes_side() {
    let mut app = on_paper(true);
    open_item(&mut app, bstep::KEF);
    make_labelled(&mut app, 30);
    let side = crate::common::texts(&mut app)
        .into_iter()
        .find_map(|t| {
            t.strip_prefix("Sheet: Faraday Starter, Blank templates, Krux code, ")
                .map(str::to_string)
        })
        .or_else(|| {
            crate::common::texts(&mut app).into_iter().find_map(|t| {
                t.strip_prefix("By hand: Sheet: Faraday Starter, Blank templates, Krux code, ")
                    .map(str::to_string)
            })
        })
        .expect("the sheet's name");
    assert!(!side.starts_with("33 "), "a side of its own: {side}");
    app.press(Action::BStep(bstep::KEF));
    open_item(&mut app, bstep::BLANK);
    let want = format!("Krux code, {side}");
    let texts = crate::common::texts(&mut app);
    assert!(
        texts
            .iter()
            .any(|t| t.contains(&want) && t.contains("Blank templates")),
        "{want}: {texts:?}"
    );
}

#[test]
fn a_code_larger_than_the_largest_sheet_reads_by_hand_no() {
    let mut app = on_paper(true);
    open_item(&mut app, bstep::KEF);
    app.press(Action::KefTool(K::Backup(KefAct::Options)));
    app.press(Action::KefTool(K::Backup(KefAct::Id(
        faraday_core::kef_form::IdKind::Label,
    ))));
    app.press(Action::KefTool(K::Backup(KefAct::Label)));
    type_in(&mut app, &"x".repeat(60));
    app.press(Action::KefTool(K::Backup(KefAct::Back)));
    app.press(Action::KefTool(K::Backup(KefAct::Field)));
    type_in(&mut app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::FieldAgain)));
    type_in(&mut app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::Make)));
    testkit::settle(&mut app);
    assert!(
        seen(&mut app, "By hand: No"),
        "{:?}",
        crate::common::texts(&mut app)
    );
    assert!(seen(&mut app, "Largest: 37 × 37"));
    assert!(!seen(&mut app, "Blocks"));
}

#[test]
fn a_record_with_the_old_eight_field_code_line_still_reads() {
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    let w = app.backup().unwrap().wallet;
    let text = String::from_utf8_lossy(&testkit::backups_record(&app)).to_string();
    assert!(
        text.contains("\nkef\t") || text.starts_with("kef\t"),
        "{text}"
    );
    let old: Vec<String> = text
        .lines()
        .map(|l| {
            if l.starts_with("kef\t") {
                let f: Vec<&str> = l.split('\t').collect();
                assert_eq!(f.len(), 11, "{l}");
                f[..8].join("\t")
            } else {
                l.to_string()
            }
        })
        .collect();
    testkit::restore_backups(&mut app, format!("{}\n", old.join("\n")).as_bytes());
    // The backup opened again reads the item Checked.
    testkit::backup_drop(&mut app);
    app.press(Action::Backup(w));
    plan_the_code(&mut app, 1);
    assert!(done(&app), "the older line still names the code");
    assert_eq!(status(&mut app), "Checked");
}

/// The made code's base43 text, as its picture holds it.
fn code_text(app: &Faraday) -> Vec<u8> {
    let file = app
        .outbox
        .iter()
        .find(|i| i.name.ends_with(".kef"))
        .expect("the code's file");
    faraday_codec::kef::base43_encode(&file.bytes).into_bytes()
}

fn scan_copy(app: &mut Faraday, bytes: Vec<u8>) {
    app.event(Event::Scanned { bytes });
    let _ = app.frame();
}

fn scan_note(app: &Faraday) -> String {
    app.scan().and_then(|s| s.note.clone()).unwrap_or_default()
}

fn open_scan_my_copy(app: &mut Faraday) {
    app.press(Action::KefTool(K::Backup(KefAct::ScanCopy)));
    assert!(app.scan().is_some(), "the camera is open");
}

#[test]
fn scanning_the_made_code_then_its_passphrase_checks_it_and_the_record_keeps_it() {
    let mut app = on_paper(true);
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    assert!(!done(&app));
    let text = code_text(&app);
    open_scan_my_copy(&mut app);
    scan_copy(&mut app, text);
    assert!(app.scan().is_none(), "the camera closed");
    type_in(&mut app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::Compare)));
    testkit::settle(&mut app);
    assert!(done(&app));
    assert_eq!(status(&mut app), "Checked");
    // Lock and unlock: the backup is made again from the record.
    let w = app.backup().unwrap().wallet;
    testkit::backup_drop(&mut app);
    app.press(Action::Backup(w));
    app.press(Action::BChecklist);
    app.settle();
    assert!(done(&app));
}

#[test]
fn another_code_reads_not_this_code_and_three_closings_offer_not_on_paper() {
    let mut app = on_paper(true);
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    // Many wrong frames in one opening of the camera count one miss.
    open_scan_my_copy(&mut app);
    for n in 1..=5 {
        scan_copy(&mut app, b"HTTPS://EXAMPLE.ORG/OTHER".to_vec());
        assert_eq!(scan_note(&app), "Not this code");
        assert!(app.scan().is_some(), "the camera stays open after {n}");
    }
    app.press(Action::Cancel);
    assert!(!seen(&mut app, "5 scans without a match"));
    assert!(!seen(&mut app, "3 scans without a match"));
    for _ in 0..2 {
        open_scan_my_copy(&mut app);
        app.press(Action::Cancel);
    }
    assert!(seen(&mut app, "3 scans without a match"));
    assert!(seen(&mut app, "Not on Paper"));
    app.press(Action::KefTool(K::Backup(KefAct::NotOnPaper)));
    assert!(
        !app.backup_items_now(&testkit::compute())
            .contains(&bstep::KEF)
    );
}

#[test]
fn a_wrong_passphrase_after_a_good_scan_reads_so_and_the_right_one_checks() {
    let mut app = on_paper(true);
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    let text = code_text(&app);
    open_scan_my_copy(&mut app);
    scan_copy(&mut app, text);
    type_in(&mut app, "not it");
    app.press(Action::KefTool(K::Backup(KefAct::Compare)));
    testkit::settle(&mut app);
    assert!(seen(&mut app, "Not matching. Try the passphrase again."));
    assert!(!done(&app));
    for _ in 0.."not it".len() {
        app.event(Event::Key(Key::Backspace));
    }
    type_in(&mut app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::Compare)));
    testkit::settle(&mut app);
    assert!(done(&app));
}

#[test]
fn nothing_scanned_for_the_drawn_code_is_loaded_or_filed() {
    let mut app = on_paper(true);
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    let (keys, wallets, inbox, outbox) = (
        app.session.keys.len(),
        app.session.wallets.len(),
        app.inbox().len(),
        app.outbox.len(),
    );
    let words = testkit::test_words(testkit::TEST_SEEDS[0].0);
    open_scan_my_copy(&mut app);
    scan_copy(&mut app, words.into_bytes());
    scan_copy(&mut app, b"HTTPS://EXAMPLE.ORG".to_vec());
    assert_eq!(app.session.keys.len(), keys);
    assert_eq!(app.session.wallets.len(), wallets);
    assert_eq!(app.inbox().len(), inbox);
    assert_eq!(app.outbox.len(), outbox);
}

#[test]
fn in_practice_the_sample_tile_gives_the_made_code() {
    let mut app = on_paper(true);
    app.session.set_demo(testkit::demo_samples());
    app.samples = Some(testkit::demo_samples());
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    open_scan_my_copy(&mut app);
    app.press(Action::ScanSample);
    assert!(app.scan().is_none(), "the sample was the code");
    type_in(&mut app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::Compare)));
    testkit::settle(&mut app);
    assert!(done(&app));
}

#[test]
fn a_code_larger_than_the_largest_sheet_has_no_scan_my_copy() {
    let mut app = on_paper(true);
    open_item(&mut app, bstep::KEF);
    app.press(Action::KefTool(K::Backup(KefAct::Options)));
    app.press(Action::KefTool(K::Backup(KefAct::Id(
        faraday_core::kef_form::IdKind::Label,
    ))));
    app.press(Action::KefTool(K::Backup(KefAct::Label)));
    type_in(&mut app, &"x".repeat(60));
    app.press(Action::KefTool(K::Backup(KefAct::Back)));
    app.press(Action::KefTool(K::Backup(KefAct::Field)));
    type_in(&mut app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::FieldAgain)));
    type_in(&mut app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::Make)));
    testkit::settle(&mut app);
    assert!(!seen(&mut app, "Scan my copy"));
    let mut small = on_paper(true);
    open_item(&mut small, bstep::KEF);
    make(&mut small);
    assert!(seen(&mut small, "Scan my copy"));
}

#[test]
fn not_on_paper_puts_the_form_away_with_what_was_typed_in_its_field() {
    let mut app = on_paper(false);
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    app.press(Action::KefTool(K::Backup(KefAct::Check)));
    type_in(&mut app, "half a passphr");
    assert!(testkit::secret_typed(&app));
    app.press(Action::KefTool(K::Backup(KefAct::NotOnPaper)));
    assert!(!testkit::secret_typed(&app), "the field is empty");
    type_in(&mut app, "more");
    assert!(!testkit::secret_typed(&app), "typing does not reach it");
}

#[test]
fn a_code_larger_than_the_largest_sheet_counts_no_sheet_in_the_templates_item() {
    let mut app = on_paper(true);
    open_item(&mut app, bstep::KEF);
    app.press(Action::KefTool(K::Backup(KefAct::Options)));
    app.press(Action::KefTool(K::Backup(KefAct::Id(
        faraday_core::kef_form::IdKind::Label,
    ))));
    app.press(Action::KefTool(K::Backup(KefAct::Label)));
    type_in(&mut app, &"x".repeat(60));
    app.press(Action::KefTool(K::Backup(KefAct::Back)));
    app.press(Action::KefTool(K::Backup(KefAct::Field)));
    type_in(&mut app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::FieldAgain)));
    type_in(&mut app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::Make)));
    testkit::settle(&mut app);
    app.press(Action::BStep(bstep::KEF));
    open_item(&mut app, bstep::BLANK);
    let texts = crate::common::texts(&mut app);
    assert!(
        texts.iter().any(|t| t.contains("Krux code, 33"))
            && !texts.iter().any(|t| t.contains("Krux code, 4")),
        "{texts:?}"
    );
}

#[test]
fn a_code_checked_off_paper_reads_not_checked_once_put_on_paper_with_a_camera() {
    let mut app = at_the_checklist();
    app.storage(StorageEvent::Cameras(vec![(
        "cam".to_string(),
        "Camera".to_string(),
    )]));
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    assert!(done(&app));
    assert_eq!(status(&mut app), "Checked");
    // Back at the plan, the code is also put on Paper at the first place.
    app.press(Action::BStep(bstep::KEF));
    app.press(Action::BPlan);
    board(&mut app, BP::Menu(Chip::List(Thing::Seed(0))));
    board(&mut app, BP::MenuVerb(Verb::Kef));
    board(&mut app, BP::MenuTo(Zone::Paper(0)));
    board(&mut app, BP::Menu(Chip::List(Thing::Seed(0))));
    board(&mut app, BP::MenuVerb(Verb::Add));
    board(&mut app, BP::MenuTo(Zone::Paper(0)));
    app.press(Action::BChecklist);
    app.settle();
    assert!(
        app.backup_items_now(&testkit::compute())
            .contains(&bstep::KEF),
        "no item"
    );
    assert!(!done(&app));
}

/// A code checked by passphrase while kept only on a stick, the backup
/// dropped and opened again, the code put on Paper at the first place, and
/// its item opened.
fn checked_reopened_then_put_on_paper(camera: bool) -> Faraday {
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    assert!(done(&app));
    reopen(&mut app);
    if camera {
        app.storage(StorageEvent::Cameras(vec![(
            "cam".to_string(),
            "Camera".to_string(),
        )]));
    }
    app.press(Action::BPlan);
    board(&mut app, BP::Menu(Chip::List(Thing::Seed(0))));
    board(&mut app, BP::MenuVerb(Verb::Kef));
    board(&mut app, BP::MenuTo(Zone::Paper(0)));
    app.press(Action::BQ(qstep::PLAN));
    app.press(Action::BChecklist);
    app.settle();
    // The blank template to print comes first on Paper.
    app.press(Action::BNext(bstep::BLANK));
    open_item(&mut app, bstep::KEF);
    app
}

#[test]
fn a_reopened_code_checked_off_paper_reads_not_checked_once_put_on_paper_with_a_camera() {
    let app = checked_reopened_then_put_on_paper(true);
    assert!(!done(&app));
}

#[test]
fn a_reopened_code_checked_off_paper_stays_checked_on_paper_with_no_camera() {
    let app = checked_reopened_then_put_on_paper(false);
    assert!(done(&app));
}

#[test]
fn a_record_kept_before_paper_was_kept_reads_as_before() {
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    make(&mut app);
    check_with(&mut app, "a passphrase");
    let w = app.backup().unwrap().wallet;
    let text = String::from_utf8_lossy(&testkit::backups_record(&app)).to_string();
    let old: Vec<String> = text
        .lines()
        .map(|l| {
            if l.starts_with("kef\t") {
                l.split('\t').take(10).collect::<Vec<_>>().join("\t")
            } else {
                l.to_string()
            }
        })
        .collect();
    testkit::restore_backups(&mut app, format!("{}\n", old.join("\n")).as_bytes());
    testkit::backup_drop(&mut app);
    app.press(Action::Backup(w));
    plan_the_code(&mut app, 1);
    assert!(done(&app), "Checked after the older record was read");
}

/// With a stick attached the Krux code item takes no passphrase, typed or
/// rolled, and a code is not made from one typed before the stick came.
#[test]
fn with_a_stick_attached_the_krux_code_takes_no_passphrase() {
    let line = "Keys load only with no stick attached";
    let stick = |on: bool| {
        StorageEvent::Sticks(if on {
            vec![faraday_core::StickInfo {
                make: Default::default(),
                serial: None,
                id: "a".into(),
                label: "STICK".into(),
                boot: false,
                files: Vec::new(),
                vaults: Vec::new(),
            }]
        } else {
            Vec::new()
        })
    };
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    app.press(Action::KefTool(K::Backup(KefAct::Field)));
    type_in(&mut app, "a passphrase");
    app.press(Action::KefTool(K::Backup(KefAct::FieldAgain)));
    type_in(&mut app, "a passphrase");
    app.storage(stick(true));
    if app.sheet == Some(faraday_core::Sheet::Lock) {
        app.press(Action::NotNow);
    }
    open_item(&mut app, bstep::KEF);
    app.event(Event::Key(Key::Enter));
    app.press(Action::KefTool(K::Backup(KefAct::Make)));
    testkit::settle(&mut app);
    assert!(kef_files(&app).is_empty(), "no code from the passphrase");
    assert_eq!(app.toast_text(), Some(line));
    app.storage(stick(false));
    app.press(Action::KefTool(K::Backup(KefAct::Make)));
    testkit::settle(&mut app);
    assert!(
        kef_files(&app).is_empty(),
        "the passphrase typed before is gone"
    );
}

/// Dice rolled before a stick came do not become the Krux code item's
/// passphrase while it is attached, and the Options sheet closing does not
/// give the passphrase field focus.
#[test]
fn with_a_stick_attached_rolled_dice_are_not_used_and_options_gives_no_focus() {
    let stick = |on: bool| {
        StorageEvent::Sticks(if on {
            vec![faraday_core::StickInfo {
                make: Default::default(),
                serial: None,
                id: "a".into(),
                label: "STICK".into(),
                boot: false,
                files: Vec::new(),
                vaults: Vec::new(),
            }]
        } else {
            Vec::new()
        })
    };
    let mut app = at_the_checklist();
    open_item(&mut app, bstep::KEF);
    app.press(Action::KefTool(K::Backup(KefAct::Options)));
    app.press(Action::KefTool(K::Backup(KefAct::Rounds)));
    for _ in 0..8 {
        app.event(Event::Key(Key::Backspace));
    }
    type_in(&mut app, "10000");
    app.press(Action::KefTool(K::Backup(KefAct::Back)));
    app.press(Action::KefTool(K::Backup(KefAct::Dice)));
    app.press(Action::KefTool(K::Backup(KefAct::DiceField)));
    type_in(&mut app, &"12345".repeat(6));
    app.storage(stick(true));
    if app.sheet == Some(faraday_core::Sheet::Lock) {
        app.press(Action::NotNow);
    }
    open_item(&mut app, bstep::KEF);
    app.press(Action::KefTool(K::BackupDice(Press::Use)));
    assert_eq!(
        app.toast_text(),
        Some("Keys load only with no stick attached")
    );
    app.storage(stick(false));
    app.press(Action::KefTool(K::Backup(KefAct::Make)));
    testkit::settle(&mut app);
    assert!(kef_files(&app).is_empty(), "the rolls are gone");
    app.storage(stick(true));
    if app.sheet == Some(faraday_core::Sheet::Lock) {
        app.press(Action::NotNow);
    }
    open_item(&mut app, bstep::KEF);
    app.press(Action::KefTool(K::Backup(KefAct::Options)));
    app.press(Action::KefTool(K::Backup(KefAct::Back)));
    app.event(Event::Key(Key::Char('x')));
    assert_ne!(
        app.toast_text(),
        Some("Keys load only with no stick attached")
    );
}
