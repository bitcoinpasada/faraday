//! A lock whose plan record no longer fits one of its vaults goes no
//! further: the toast says why, nothing is written to a stick, and every
//! vault keeps the plan it had.

use crate::backup_flow_wallet::{
    PHRASE, PHRASE_2, flow_vault_add, restart, single_key_ready, step, two_of_three_ready,
};
use crate::codex32_kept::unlock_on_vaults;
use crate::common::RecordsOf as _;
use crate::common::*;
use faraday_core::backup_flow::Step;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::board::BoardPress as BP;
use faraday_core::board::{Chip, Thing, Zone};
use faraday_core::plan::Item;
use faraday_core::plan::text::{seals_in, with_seals};
use faraday_core::plan::{At, Note, What};
use faraday_core::strings::EN;
use faraday_core::testkit;
use faraday_core::vault::{SealSeen, VaultSummary};
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, Event, Key};
use faraday_vault::Record;
use faraday_vault::records::{field, kind};

const FULL: &str = "More than a vault can hold";

/// Open vault `o`'s plan record: its size and its answers.
fn plan_of(app: &Faraday, o: usize) -> Vec<(usize, String)> {
    contents(app, o)
        .of(kind::PLAN)
        .map(|(_, r)| {
            let answers = r.text(field::PLAN_ANSWERS).unwrap_or_default();
            (r.size(), answers.to_string())
        })
        .collect()
}

const NAME: &str = "Tin box behind the cellar shelf";

/// The toast showing has had its time by `now_ms` and a minute: the tick
/// that first sees it starts its time.
fn toast_gone(app: &mut Faraday, now_ms: u64) {
    app.event(Event::Tick { now_ms });
    app.event(Event::Tick {
        now_ms: now_ms + 60_000,
    });
    assert_eq!(app.toast_text(), None);
}

/// On the checklist: back to the board, Place 1 named, and the checklist
/// made again, ready for Lock.
fn name_a_place(app: &mut Faraday) {
    app.press(Action::BPlan);
    app.press(Action::BName(0));
    type_text(app, NAME);
    app.event(Event::Key(Key::Enter));
    assert_eq!(app.place_name(0), NAME);
    app.press(Action::BChecklist);
    testkit::settle(app);
    // What the checklist said has gone: a toast after this is a later
    // press's.
    toast_gone(app, 10_000_000);
}

/// Open vault `o` takes a long note, so it is the one a limit fills first.
fn weigh_down(app: &mut Faraday, o: usize) {
    let id = app.vaults().open[o].id;
    let note = Record::new(kind::NOTE).with(field::NOTE, &[b'n'; 4_000]);
    testkit::vault_edit(app, id, |s| s.insert(id, note));
}

/// The notes of open vault `o`'s plan record.
fn notes_of(app: &Faraday, o: usize) -> Vec<String> {
    let held = contents(app, o);
    held.of(kind::PLAN)
        .flat_map(|(_, r)| {
            r.fields
                .iter()
                .filter(|f| f.number == field::PLAN_NOTE)
                .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// A plan record of what open vault `o`'s says, with `notes` added.
fn plan_copy(app: &Faraday, o: usize, notes: &[Note]) -> Record {
    let held = contents(app, o);
    let (_, r) = held.of(kind::PLAN).next().expect("a plan record");
    let mut copy = Record::new(kind::PLAN);
    for f in [field::PLAN_WALLET, field::PLAN_ANSWERS] {
        copy = copy.with(f, r.text(f).expect("the field").as_bytes());
    }
    for note in notes {
        copy.push(field::PLAN_NOTE, note.to_text().as_bytes());
    }
    copy
}

/// Open vault `o` keeps no plan.
fn forget_plan(app: &mut Faraday, o: usize) {
    let id = app.vaults().open[o].id;
    testkit::vault_edit(app, id, |s| s.remove_where(id, |r| r.kind() == kind::PLAN));
}

/// Lock, refused with `toast`: the app stays on Back up, and no vault's
/// plan, no stick and no waiting file changed.
fn lock_refused(shell: &mut Shell, app: &mut Faraday, toast: &str) {
    let open = app.vaults().open.len();
    let had: Vec<_> = (0..open).map(|o| plan_of(app, o)).collect();
    let sticks = disks(shell, &["S1", "S2", "S3"]);
    let queued = waiting(app);
    shell.flow(app, F::WalletLock);
    testkit::settle(app);
    assert_eq!(app.toast_text(), Some(toast));
    assert_eq!(app.screen, Screen::Backup, "the lock went on");
    assert!(!app.restart_requested(), "the lock went through");
    assert_eq!(app.vaults().open.len(), open);
    for (o, had) in had.iter().enumerate() {
        assert_eq!(&plan_of(app, o), had, "vault {o}'s plan changed");
    }
    assert_eq!(disks(shell, &["S1", "S2", "S3"]), sticks);
    assert_eq!(waiting(app), queued, "the lock queued a file");
}

/// The toast of a plan open vault `o` is too full for.
fn full(app: &Faraday, o: usize) -> String {
    format!("{}: {FULL}", &*app.vaults().open[o].name)
}

/// Vault `o` holds what it holds now and `room` bytes more.
fn leave_room(app: &mut Faraday, o: usize, room: usize) {
    let id = app.vaults().open[o].id;
    testkit::vault_edit(app, id, |s| s.lower_limit(0));
    let used = app.vaults().records(id).used(&testkit::compute());
    testkit::vault_edit(app, id, |s| s.lower_limit(used + room));
}

/// What every stick holds.
fn disks(shell: &Shell, names: &[&str]) -> Vec<Vec<(String, Vec<u8>)>> {
    names
        .iter()
        .map(|d| {
            shell
                .disk(d)
                .files
                .iter()
                .map(|(n, b)| (n.to_string(), b.to_vec()))
                .collect()
        })
        .collect()
}

/// The files waiting for a stick.
fn waiting(app: &Faraday) -> Vec<String> {
    app.outbox.iter().map(|i| i.name.to_string()).collect()
}

#[test]
fn a_plan_that_no_longer_fits_its_vault_stops_the_lock() {
    let mut shell = Shell::light();
    let (mut app, _, _) = single_key_ready(&mut shell);
    let had = plan_of(&app, 0);
    let sticks = disks(&shell, &["S1", "S2", "S3"]);
    leave_room(&mut app, 0, 0);
    name_a_place(&mut app);
    assert_eq!(plan_of(&app, 0), had, "the name fitted");
    let queued = waiting(&app);

    shell.flow(&mut app, F::WalletLock);
    testkit::settle(&mut app);

    assert_eq!(app.toast_text(), Some(full(&app, 0).as_str()));
    assert_eq!(app.screen, Screen::Backup, "the lock went on");
    assert!(!app.restart_requested(), "the lock went through");
    assert_eq!(app.vaults().open.len(), 1);
    assert_eq!(plan_of(&app, 0), had, "the vault's plan changed");
    assert_eq!(disks(&shell, &["S1", "S2", "S3"]), sticks);
    assert_eq!(waiting(&app), queued, "the lock queued a file");
    // The plan under way is still the person's.
    assert_eq!(app.place_name(0), NAME);

    // With room, the checklist made again keeps the name and the same
    // press locks, on to the sticks.
    leave_room(&mut app, 0, 4_000);
    app.press(Action::BPlan);
    app.press(Action::BChecklist);
    testkit::settle(&mut app);
    assert_ne!(plan_of(&app, 0), had, "the name is not kept");
    shell.flow(&mut app, F::WalletLock);
    let app = restart(&mut shell, &mut app);
    assert_eq!(app.screen, Screen::BackupFlow);
    assert_eq!(step(&app), Step::Write);
}

/// A vault after the first one asked is the full one: the ones before it
/// are not written either.
#[test]
fn a_plan_that_does_not_fit_a_later_vault_is_written_in_none() {
    let mut shell = Shell::light();
    let mut app = two_of_three_ready(&mut shell, false);
    weigh_down(&mut app, 1);
    leave_room(&mut app, 1, 0);
    name_a_place(&mut app);
    // The first vault's is for the lock to write.
    forget_plan(&mut app, 0);
    let toast = full(&app, 1);
    lock_refused(&mut shell, &mut app, &toast);
}

/// A vault filled after the checklist saved the plan into it: the lock
/// leaves that plan as it is, yet the other vaults now get the plan too, and
/// the full one has no room for their lines.
#[test]
fn a_vault_filled_after_its_plan_was_saved_stops_the_lock() {
    let mut shell = Shell::light();
    let mut app = two_of_three_ready(&mut shell, false);
    name_a_place(&mut app);
    assert!(!plan_of(&app, 0).is_empty());
    assert!(plan_of(&app, 1).is_empty() && plan_of(&app, 2).is_empty());
    // The limit is every vault's: the note leaves the others room.
    weigh_down(&mut app, 0);
    leave_room(&mut app, 0, 0);
    let toast = full(&app, 0);
    lock_refused(&mut shell, &mut app, &toast);

    // With room, the same press locks.
    leave_room(&mut app, 0, 4_000);
    toast_gone(&mut app, 20_000_000);
    shell.flow(&mut app, F::WalletLock);
    testkit::settle(&mut app);
    shell.pump(&mut app);
    assert!(app.restart_requested(), "{:?}", app.toast_text());
}

/// A vault set up before the keys and then taken off the plan on the
/// board gets no plan at Make the checklist or at Lock and write, and
/// the lock drops the one it kept.
#[test]
fn a_vault_set_up_before_the_keys_and_taken_off_the_plan_gets_none() {
    let mut shell = Shell::light();
    let mut app = two_of_three_ready(&mut shell, false);
    name_a_place(&mut app);
    let had = plan_of(&app, 0);
    assert!(!had.is_empty());
    app.press(Action::BPlan);
    vault_off(&mut app, 0);
    app.press(Action::BChecklist);
    testkit::settle(&mut app);
    toast_gone(&mut app, 20_000_000);
    // No other vault of the plan is written yet: its own is as it was.
    assert_eq!(plan_of(&app, 0), had, "Make the checklist wrote into it");
    // Lock and write gives the plan to the other vaults; the copy of this
    // one that it seals keeps none.
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(&mut shell, &mut app);
    unlock_on_vaults(&mut app, "main.kdbx", PHRASE);
    assert_eq!(contents(&app, 0).of(kind::KEY).count(), 1, "the vault");
    assert_eq!(plan_of(&app, 0), [], "it keeps a plan");
}

/// A vault takes, after the lock's plan, a line for each other vault as
/// that one is sealed. At the least room at which the lock is not refused
/// every vault still seals; below it nothing is changed. `before` is done
/// once the first vault has its limit, `after` once the place is named.
fn the_lock_has_room_to_seal(before: impl Fn(&mut Faraday), after: impl Fn(&mut Faraday)) {
    // Whether the lock of three vaults went on with `limit` bytes for
    // what the first holds.
    let goes_on = |limit: usize| {
        let mut shell = Shell::light();
        let mut app = two_of_three_ready(&mut shell, false);
        let id = app.vaults().open[0].id;
        testkit::vault_edit(&mut app, id, |s| s.lower_limit(limit));
        before(&mut app);
        name_a_place(&mut app);
        after(&mut app);
        let had: Vec<_> = (0..app.vaults().open.len())
            .map(|o| plan_of(&app, o))
            .collect();
        shell.flow(&mut app, F::WalletLock);
        testkit::settle(&mut app);
        shell.pump(&mut app);
        if app.screen == Screen::Backup {
            assert!(app.toast_text().is_some_and(|t| t.ends_with(FULL)));
            for (o, had) in had.iter().enumerate() {
                assert_eq!(&plan_of(&app, o), had, "{limit}: vault {o}");
            }
            return false;
        }
        assert!(
            app.restart_requested() && app.vaults().open.is_empty(),
            "{limit} bytes: the lock went on and left {} vaults unsealed",
            app.vaults().open.len()
        );
        true
    };
    let (mut refused, mut through) = (2_800, 8_000);
    assert!(!goes_on(refused));
    assert!(goes_on(through));
    while through - refused > 32 {
        let mid = (refused + through) / 2;
        if goes_on(mid) {
            through = mid;
        } else {
            refused = mid;
        }
    }
}

#[test]
fn a_lock_that_is_let_through_has_room_to_seal_every_vault() {
    the_lock_has_room_to_seal(|_| {}, |_| {});
}

/// A vault that is not open, known from when it was, keeps the wallet's
/// plan: each plan gets a line for it too.
#[test]
fn a_lock_has_room_for_the_line_of_a_vault_that_is_not_open() {
    let closed = |app: &mut Faraday| {
        let text = contents(app, 0)
            .of(kind::PLAN)
            .find_map(|(_, r)| r.text(field::PLAN_WALLET).map(|t| t.to_string()))
            .expect("a plan record");
        let w = faraday_core::wallet::read_wallet(&testkit::compute(), &text)
            .expect("the wallet's description");
        let summary = VaultSummary {
            id: [0x5c; 32],
            planned: vec![faraday_text::Text::of(w.checksum())],
            planned_ids: vec![faraday_text::Text::of(&w.id().to_hex())],
            seal: Some(SealSeen {
                count: 3,
                digest: [0x5d; 32],
                planned: Vec::new(),
            }),
            ..VaultSummary::default()
        };
        testkit::change_vault_summaries(app, |all| all.push(summary));
    };
    the_lock_has_room_to_seal(closed, |_| {});
}

/// The second vault is left out of keeping the plan: it is sealed at the
/// lock with its keys, and each plan gets a line for it.
#[test]
fn a_lock_has_room_for_the_line_of_a_vault_left_out() {
    the_lock_has_room_to_seal(
        |_| {},
        |app| {
            app.press(Action::BPlan);
            board(app, BP::VaultKeeps(1));
            app.press(Action::BChecklist);
            testkit::settle(app);
            toast_gone(app, 15_000_000);
        },
    );
}

/// Open vault `o`'s kept summary says it holds no wallet and no plan.
fn summary_holds_nothing(app: &mut Faraday, o: usize) {
    let id = app.vaults().open[o].vault_id();
    testkit::change_vault_summaries(app, |all| {
        let s = all.iter_mut().find(|s| s.id == id).expect("its summary");
        s.wallets.clear();
        s.sums.clear();
        s.wallet_ids.clear();
        s.planned.clear();
        s.planned_ids.clear();
    });
}

/// Vault `v` is left out of keeping the plan, on the board, and the
/// checklist made again.
fn leave_out(app: &mut Faraday, v: usize) {
    app.press(Action::BPlan);
    board(app, BP::VaultKeeps(v));
    app.press(Action::BChecklist);
    testkit::settle(app);
    toast_gone(app, 15_000_000);
}

/// A vault takes the description from its checklist item, which is no
/// press on the vault, and is then left out: it is sealed before the
/// others, and each of their plans gets a line for it.
#[test]
fn a_lock_has_room_after_a_checklist_item_saved_into_a_vault_left_out() {
    the_lock_has_room_to_seal(
        |_| {},
        |app| {
            // The first vault as it was before its item: no description
            // and no plan.
            let id = app.vaults().open[0].id;
            testkit::vault_edit(app, id, |s| {
                s.remove_where(id, |r| [kind::WALLET, kind::PLAN].contains(&r.kind()));
            });
            summary_holds_nothing(app, 0);
            app.press(Action::BVaultSave(0));
            assert_eq!(contents(app, 0).of(kind::WALLET).count(), 1);
            leave_out(app, 0);
        },
    );
}

/// A vault left out gets its line whatever its summary says it holds.
#[test]
fn a_lock_has_room_for_a_vault_left_out_whatever_its_summary_says() {
    the_lock_has_room_to_seal(
        |_| {},
        |app| {
            leave_out(app, 0);
            summary_holds_nothing(app, 0);
        },
    );
}

/// A fourth vault, added on the board, takes the description from its
/// checklist item into the current vault, and is then taken off the plan.
fn take_off_a_vault_after_its_item(app: &mut Faraday) {
    testkit::inbox_file(app, "odd.kdbx", vault("Odd", PHRASE, 0x44));
    unlock_on_vaults(app, "odd.kdbx", PHRASE);
    testkit::vault_current(app, 3);
    app.press(Action::Nav(Screen::Backup));
    toast_gone(app, 15_000_000);
    app.press(Action::BPlan);
    board(app, BP::AddVault);
    board(app, BP::Chip(Chip::List(Thing::Vault(3))));
    board(app, BP::Zone(Zone::NewStick(1)));
    board(app, BP::Chip(Chip::List(Thing::Whole)));
    board(app, BP::Zone(Zone::Vault(1, 1, 3)));
    app.press(Action::BChecklist);
    testkit::settle(app);
    app.press(Action::BVaultSave(3));
    testkit::settle(app);
    assert_eq!(contents(app, 3).of(kind::WALLET).count(), 1);
    app.press(Action::BPlan);
    vault_off(app, 3);
    app.press(Action::BChecklist);
    testkit::settle(app);
    toast_gone(app, 30_000_000);
    assert_eq!(plan_of(app, 3), [], "it keeps a plan");
}

/// The vault taken off stays open with the wallet, and each plan gets a
/// line for it. A refusal names the vault that is full, and no open vault
/// changes.
#[test]
fn a_lock_has_room_for_a_vault_taken_off_the_plan_after_its_item() {
    the_lock_has_room_to_seal(|_| {}, take_off_a_vault_after_its_item);

    let mut shell = Shell::light();
    let mut app = two_of_three_ready(&mut shell, false);
    name_a_place(&mut app);
    take_off_a_vault_after_its_item(&mut app);
    // The limit is every vault's: the note leaves the others room.
    weigh_down(&mut app, 0);
    leave_room(&mut app, 0, 0);
    let toast = full(&app, 0);
    lock_refused(&mut shell, &mut app, &toast);
}

/// The answers of each of the plan records `plans`, without the `seals`
/// lines a seal adds.
fn unsealed(plans: &[(usize, String)]) -> Vec<String> {
    plans
        .iter()
        .map(|(_, answers)| {
            answers
                .lines()
                .filter(|l| !l.starts_with("seals "))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect()
}

/// A wallet's earlier saves put the plan in the first vault only, which is
/// then left out: it keeps that plan, the one record of the places' names,
/// until a save writes the plan into another vault.
#[test]
fn a_vault_left_out_keeps_its_plan_until_another_takes_it() {
    let mut shell = Shell::light();
    let mut app = two_of_three_ready(&mut shell, false);
    assert!(plan_of(&app, 1).is_empty() && plan_of(&app, 2).is_empty());
    weigh_down(&mut app, 1);
    leave_room(&mut app, 1, 0);
    app.press(Action::BPlan);
    board(&mut app, BP::VaultKeeps(0));
    let had = plan_of(&app, 0);
    assert!(!had.is_empty());
    app.press(Action::BChecklist);
    testkit::settle(&mut app);
    toast_gone(&mut app, 10_000_000);
    assert_eq!(plan_of(&app, 0), had, "after the checklist");
    let toast = full(&app, 1);
    lock_refused(&mut shell, &mut app, &toast);

    // An ordinary Lock, then the vault opened again.
    app.press(Action::LockAsk);
    app.press(Action::Lock);
    let mut app = restart(&mut shell, &mut app);
    unlock_on_vaults(&mut app, "main.kdbx", PHRASE);
    assert_eq!(
        unsealed(&plan_of(&app, 0)),
        unsealed(&had),
        "after the lock"
    );
}

/// A fourth vault, `file`, of no part in the wallet, opened and made
/// current beside the three; no vault keeps the plan, so the current one is
/// among those the lock saves it into.
fn another_vault_current(app: &mut Faraday, file: Vec<u8>) -> usize {
    testkit::inbox_file(app, "odd.kdbx", file);
    unlock_on_vaults(app, "odd.kdbx", PHRASE);
    for o in 0..3 {
        forget_plan(app, o);
    }
    testkit::vault_current(app, 3);
    app.press(Action::Nav(Screen::Backup));
    toast_gone(app, 20_000_000);
    3
}

/// A vault that is no part of the wallet, current while no vault keeps the
/// plan, is one the lock saves it into: full, it stops the lock and is
/// named.
#[test]
fn a_full_current_vault_of_no_part_in_the_wallet_stops_the_lock() {
    let mut shell = Shell::light();
    let mut app = two_of_three_ready(&mut shell, false);
    name_a_place(&mut app);
    let odd = another_vault_current(&mut app, vault("Odd", PHRASE, 0x44));
    weigh_down(&mut app, odd);
    leave_room(&mut app, odd, 0);
    let toast = full(&app, odd);
    lock_refused(&mut shell, &mut app, &toast);
}

/// A vault of a newer format of Faraday's, open to read only, that holds
/// the plan record `plan` if one is given.
fn newer_format_vault(plan: Option<&Record>) -> Vec<u8> {
    testkit::kdbx_edited(
        "Newer",
        PHRASE,
        9,
        |doc| {
            if let Some(plan) = plan {
                faraday_vault::kdbx::items::put_plan(doc, 1, plan).expect("the plan is written");
            }
        },
        |xml| {
            let at = xml.find("faraday.format").unwrap();
            let v = at + xml[at..].find("<Value>1</Value>").unwrap();
            let end = v + "<Value>1</Value>".len();
            format!("{}<Value>99</Value>{}", &xml[..v], &xml[end..])
        },
    )
    .unwrap()
}

/// A vault the plan goes into that is open to read only takes no plan: the
/// lock stops and names it.
#[test]
fn a_read_only_vault_the_plan_goes_into_stops_the_lock() {
    let mut shell = Shell::light();
    let mut app = two_of_three_ready(&mut shell, false);
    name_a_place(&mut app);
    let ro = another_vault_current(&mut app, newer_format_vault(None));
    let name = app.vaults().open[ro].name.to_string();
    let toast = format!("{name}: {}", EN.vaults_cannot_write);
    lock_refused(&mut shell, &mut app, &toast);
}

/// A read-only vault takes no write: where the plan it keeps is the one the
/// lock saves, the lock goes on.
#[test]
fn a_read_only_vault_with_the_current_plan_lets_the_lock_go_on() {
    let mut shell = Shell::light();
    let mut app = two_of_three_ready(&mut shell, false);
    name_a_place(&mut app);
    let held = contents(&app, 0);
    let (_, r) = held.of(kind::PLAN).next().expect("a plan record");
    let mut whole = Record::new(kind::PLAN);
    for f in &r.fields {
        whole.push(f.number, &f.bytes);
    }
    let ro = another_vault_current(&mut app, newer_format_vault(Some(&whole)));
    assert_eq!(plan_of(&app, ro).len(), 1, "it holds the plan");
    shell.flow(&mut app, F::WalletLock);
    testkit::settle(&mut app);
    shell.pump(&mut app);
    assert!(app.restart_requested(), "{:?}", app.toast_text());
}

/// A vault left out of keeping the plan keeps the one it had while another
/// vault refuses the lock's save. The second vault's plan is one kept
/// earlier: the checklist does not write that vault, the lock does.
#[test]
fn a_left_out_vault_keeps_its_plan_while_another_refuses() {
    let mut shell = Shell::light();
    let mut app = two_of_three_ready(&mut shell, false);
    let copy = plan_copy(&app, 0, &[]);
    let id = app.vaults().open[1].id;
    testkit::vault_edit(&mut app, id, |s| s.insert(id, copy));
    let had = plan_of(&app, 1);
    assert!(!had.is_empty());
    weigh_down(&mut app, 2);
    leave_room(&mut app, 2, 0);
    app.press(Action::BPlan);
    board(&mut app, BP::VaultKeeps(1));
    app.press(Action::BChecklist);
    testkit::settle(&mut app);
    toast_gone(&mut app, 10_000_000);
    assert_eq!(plan_of(&app, 1), had, "the checklist wrote the vault");
    let toast = full(&app, 2);
    lock_refused(&mut shell, &mut app, &toast);
}

/// A place removed on the board while a save is refused: the next save
/// still moves the notes of the places after it.
#[test]
fn a_refused_save_keeps_the_boards_edits_for_the_notes() {
    let mut shell = Shell::light();
    let (mut app, _, _) = single_key_ready(&mut shell);
    let date = faraday_text::Text::of("2026-10-10");
    let at = |p: u8| Note::Checked(At::Place(p.into()), What::words(0), date.clone());
    let copy = plan_copy(&app, 0, &[at(1)]);
    forget_plan(&mut app, 0);
    let id = app.vaults().open[0].id;
    testkit::vault_edit(&mut app, id, |s| s.insert(id, copy));
    assert_eq!(notes_of(&app, 0), [at(1).to_text().to_string()]);
    leave_room(&mut app, 0, 0);
    app.press(Action::BPlan);
    app.press(Action::BName(1));
    type_text(&mut app, NAME);
    app.event(Event::Key(Key::Enter));
    board(&mut app, BP::RemovePlace(0));
    board(&mut app, BP::RemovePlaceSure(0));
    app.press(Action::BChecklist);
    testkit::settle(&mut app);
    assert_eq!(app.toast_text(), Some(full(&app, 0).as_str()));
    assert_eq!(notes_of(&app, 0), [at(1).to_text().to_string()]);
    leave_room(&mut app, 0, 4_000);
    app.press(Action::BPlan);
    app.press(Action::BChecklist);
    testkit::settle(&mut app);
    assert_eq!(notes_of(&app, 0), [at(0).to_text().to_string()]);
}

/// Two of the wallet's vaults, open after the lock, with a seed added to
/// the second on the plan: ready for the round's Copy.
fn round_ready(shell: &mut Shell) -> Faraday {
    let mut app = two_of_three_ready(shell, false);
    shell.flow(&mut app, F::WalletLock);
    let mut app = restart(shell, &mut app);
    shell.flow(&mut app, F::ChangePlan);
    flow_vault_add(shell, &mut app, 1, 0);
    shell.flow(&mut app, F::Continue);
    open_vault(shell, &mut app, 0, PHRASE);
    open_vault(shell, &mut app, 1, PHRASE_2);
    app
}

/// The first vault takes a line for the second as that one is sealed. At
/// the least room at which the round goes on, an ordinary Lock still seals
/// every vault; below it the round copies nothing. The first vault's plan
/// record starts without the second's line, so that one is to come. With
/// `unchanged`, its answers already say what the round would write.
fn the_round_has_room_to_seal(unchanged: bool) {
    let goes_on = |room: usize| {
        let mut shell = Shell::light();
        let mut app = round_ready(&mut shell);
        let second = app.vaults().open[1].vault_id();
        let mut copy = plan_copy(&app, 0, &[]);
        let kept = faraday_text::Text::of(copy.text(field::PLAN_ANSWERS).unwrap_or_default());
        let mut seals = seals_in(&kept);
        seals.retain(|l| l.id != second);
        let answers = if unchanged {
            app.flow()
                .and_then(|f| f.wallet.as_ref())
                .expect("the wallet's flow")
                .answers
                .to_text()
        } else {
            kept
        };
        copy.set(field::PLAN_ANSWERS, with_seals(&answers, &seals).as_bytes());
        let id = app.vaults().open[0].id;
        forget_plan(&mut app, 0);
        testkit::vault_edit(&mut app, id, |s| s.insert(id, copy));
        let lines: Vec<_> = plan_of(&app, 0)
            .iter()
            .flat_map(|(_, answers)| seals_in(answers))
            .collect();
        assert!(lines.iter().all(|l| l.id != second), "its line is there");
        weigh_down(&mut app, 0);
        leave_room(&mut app, 0, room);
        let had = (plan_of(&app, 0), contents(&app, 1).of(kind::KEY).count());
        shell.flow(&mut app, F::CopyKey);
        testkit::settle(&mut app);
        if app.flow().is_some_and(|f| f.adding) && step(&app) == Step::Make {
            assert_eq!(app.toast_text(), Some(full(&app, 0).as_str()), "{room}");
            let now = (plan_of(&app, 0), contents(&app, 1).of(kind::KEY).count());
            assert_eq!(now, had, "{room}: the round wrote");
            return false;
        }
        app.press(Action::LockAsk);
        top_up(&mut app);
        app.press(Action::Lock);
        testkit::settle(&mut app);
        shell.pump(&mut app);
        assert!(
            app.restart_requested() && app.vaults().open.is_empty(),
            "{room} bytes: the round went on and left {} vaults unsealed",
            app.vaults().open.len()
        );
        true
    };
    let (mut refused, mut through) = (0, 4_000);
    assert!(!goes_on(refused));
    assert!(goes_on(through));
    while through - refused > 32 {
        let mid = (refused + through) / 2;
        if goes_on(mid) {
            through = mid;
        } else {
            refused = mid;
        }
    }
}

#[test]
fn a_round_that_is_let_through_has_room_to_seal_every_vault() {
    the_round_has_room_to_seal(false);
}

#[test]
fn a_round_has_room_for_the_lines_of_a_plan_record_it_leaves_as_it_is() {
    the_round_has_room_to_seal(true);
}

/// The round for a vault the plan gains, open vault `o` of the first two
/// being too full for what the round writes into it: it stops the round
/// and is named, and nothing is copied.
fn the_round_stops_at_a_full_vault(o: usize) {
    let mut shell = Shell::light();
    let mut app = round_ready(&mut shell);
    weigh_down(&mut app, o);
    leave_room(&mut app, o, 0);
    let held = |app: &Faraday| -> Vec<_> {
        (0..2)
            .map(|o| (plan_of(app, o), contents(app, o).of(kind::KEY).count()))
            .collect()
    };
    let had = held(&app);
    shell.flow(&mut app, F::CopyKey);
    testkit::settle(&mut app);
    assert_eq!(app.toast_text(), Some(full(&app, o).as_str()));
    assert_eq!(held(&app), had, "the round wrote");
    assert!(app.flow().unwrap().adding, "the round went on");
    assert_eq!(step(&app), Step::Make);

    // With room, the same press copies.
    leave_room(&mut app, o, 4_000);
    shell.flow(&mut app, F::CopyKey);
    testkit::settle(&mut app);
    assert_ne!(held(&app), had);
}

/// A vault of the plan the round adds nothing to: its plan record, as the
/// round rewrites it, does not fit.
#[test]
fn a_plan_that_does_not_fit_stops_the_round_for_an_added_vault() {
    the_round_stops_at_a_full_vault(0);
}

/// The added vault itself cannot take what the round copies into it.
#[test]
fn a_full_added_vault_stops_the_round_and_is_named() {
    the_round_stops_at_a_full_vault(1);
}

/// Every open vault is left out, the first keeping the plan it has, and
/// the vault that is to keep it is not made yet: the checklist writes
/// nothing, and the board's edits and the thing added there wait for the
/// save that writes.
#[test]
fn a_save_into_no_vault_keeps_the_boards_edits_for_the_one_that_writes() {
    let mut shell = Shell::light();
    let mut app = two_of_three_ready(&mut shell, false);
    let date = faraday_text::Text::of("2026-10-10");
    let at = |p: u8| Note::Checked(At::Place(p.into()), What::words(0), date.clone());
    let copy = plan_copy(&app, 0, &[at(1)]);
    forget_plan(&mut app, 0);
    let id = app.vaults().open[0].id;
    testkit::vault_edit(&mut app, id, |s| s.insert(id, copy));
    let had = plan_of(&app, 0);

    app.press(Action::BPlan);
    board(&mut app, BP::AddVault);
    board(&mut app, BP::Chip(Chip::List(Thing::Vault(3))));
    board(&mut app, BP::Zone(Zone::NewStick(1)));
    // The new vault, the second stick at place 1, takes a seed, so that
    // the checklist can be made.
    board(&mut app, BP::Chip(Chip::List(Thing::Seed(0))));
    board(&mut app, BP::Zone(Zone::Vault(1, 1, 3)));
    assert_eq!(app.backup().unwrap().answers.empty_vault(), None);
    for v in 0..3 {
        board(&mut app, BP::VaultKeeps(v));
    }
    board(&mut app, BP::RemovePlace(0));
    board(&mut app, BP::AddPlace);
    let last = app.backup().unwrap().answers.places - 1;
    board(&mut app, BP::Chip(Chip::List(Thing::Seed(0))));
    board(&mut app, BP::Zone(Zone::Paper(last)));
    let added = app.backup().unwrap().added;
    assert!(added.is_some(), "the board added nothing");
    for turn in 0..2 {
        if turn > 0 {
            app.press(Action::BPlan);
        }
        app.press(Action::BChecklist);
        testkit::settle(&mut app);
        assert_eq!(plan_of(&app, 0), had, "{turn}: the checklist wrote");
        assert_eq!(app.backup().unwrap().added, added, "{turn}");
        assert_eq!(app.backup().unwrap().open, Some(Item::Copy(0)), "{turn}");
    }

    // The vault keeps the plan again: this save writes it.
    app.press(Action::BPlan);
    board(&mut app, BP::VaultKeeps(0));
    app.press(Action::BChecklist);
    testkit::settle(&mut app);
    assert_ne!(plan_of(&app, 0), had, "nothing was written");
    assert_eq!(notes_of(&app, 0), [at(0).to_text().to_string()]);
    assert_eq!(app.backup().unwrap().added, None);
}
