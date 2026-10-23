//! The backup checklist's presses that nothing else marks, through the
//! lock that Write to sticks makes and the unlock after it
//! (`docs/BACKUP-FLOW.md` §17).

use crate::backup_flow_wallet::{
    PHRASE, restart, single_key_plan, single_key_ready, step, write_sticks,
};
use crate::common::*;
use faraday_core::backup_flow::Step;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::plan::Item;
use faraday_core::testkit;
use faraday_core::{Action, Screen, bstep};

#[test]
fn the_templates_and_envelopes_pressed_stay_done_through_the_lock_and_the_unlock() {
    let c = testkit::compute();
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_plan(&mut shell, false, true, true);
    assert!(app.backup_item_done_now(&c, Item::Templates));
    assert!(app.backup_item_done_now(&c, Item::Envelopes));
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3"]);
    assert_eq!(step(&app), Step::Done);
    shell.flow(&mut app, F::UnlockDone(0));
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    unlock(&mut app, PHRASE);
    assert_eq!(app.screen, Screen::Backup, "{:?}", app.toast_text());
    assert!(app.backup_item_done_now(&c, Item::Templates), "templates");
    assert!(app.backup_item_done_now(&c, Item::Envelopes), "envelopes");
    // Every item is done with nothing pressed again.
    for n in app.backup_items_now(&c) {
        let item = bstep::item(n).unwrap();
        assert!(app.backup_item_done_now(&c, item), "{item:?} is not done");
    }
}

#[test]
fn the_done_card_unlocks_to_the_checklist_done_and_the_files_read_written_to() {
    let c = testkit::compute();
    let mut shell = Shell::new();
    let (mut app, _, _) = single_key_ready(&mut shell);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    // While the flow is left, a stick visit lists its files In the backup.
    shell.flow(&mut app, F::Back);
    let wrote: Vec<String> = app
        .backup_flows()
        .iter()
        .flat_map(|f| f.slots.iter().flat_map(|s| s.files.clone()))
        .collect();
    assert!(!wrote.is_empty());
    for n in &wrote {
        assert!(app.in_the_backup(n), "{n} is in the backup");
    }
    shell.flow(&mut app, F::Show(0));
    write_sticks(&mut shell, &mut app, &["S1", "S2", "S3"]);
    assert_eq!(step(&app), Step::Done);
    for n in &wrote {
        assert!(!app.in_the_backup(n), "{n} is done");
    }
    // Unlock: the wallet's checklist, done, with nothing For the stick.
    shell.flow(&mut app, F::UnlockDone(0));
    faraday_core::testkit::vault_speed(&mut app, Some(180));
    unlock(&mut app, PHRASE);
    assert_eq!(app.screen, Screen::Backup, "{:?}", app.toast_text());
    assert!(app.backup_item_done_now(&c, Item::Sticks));
    assert!(app.outbox.is_empty());
    // The description's files went with their stick's write, the item
    // that says so (`docs/BACKUP-FLOW.md` §17.12 item 6).
    expect_part(&mut app, "3 of 3 written");
    assert!(!offered(&mut app, Action::WriteAsk), "no Write to a stick");
}
