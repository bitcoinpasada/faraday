//! The stick pills across the top of the desktop app and the website
//! demo: the boot stick (Stick 1) and Stick 2 by default, a click to insert or
//! pull, one pill for each stick a backup flow needs, and what the flow
//! asks for at each step. On a device (the PC stick, the Pi) the flow's
//! sticks alone, lit only for the stick recognised, with no press; the
//! card's "Mark it as Stick 2" and "This is Stick 2" (BACKUP-FLOW §16).
//! On every shell the boot stick is Stick 1 and a backup's sticks are
//! numbered from 2.

use crate::common::*;
use faraday_core::backup_flow::steps::FlowAction as F;
use faraday_core::backup_flow::{Kind, Slot, Step, Target};
use faraday_core::create::NewKind;
use faraday_core::{
    Action, BOOT_FOLDER, Faraday, Medium, Screen, SticksWanted, StorageEvent, cstep,
};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware, TouchPhase};

/// A desktop app with the boot stick and Stick 2 in, as the desktop
/// starts.
fn desktop(shell: &mut Shell) -> Faraday {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    app.shell_sticks = true;
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeKeep);
    for folder in [BOOT_FOLDER, "STICK-2"] {
        let mut d = Disk::empty(folder, folder);
        // As the desktop reports its boot folder.
        d.boot = folder == BOOT_FOLDER;
        d.serial = Some(if folder == BOOT_FOLDER { 1 } else { 2 });
        shell.add(d);
        shell.plug(&mut app, folder);
    }
    app
}

/// What the pills say, as drawn.
fn drawn(app: &mut Faraday) -> Vec<String> {
    let _ = app.frame();
    app.drawn_texts()
}

fn click(app: &mut Faraday, a: Action) {
    let _ = app.frame();
    let (x, y) = app.where_offered(a).expect("the pill is on screen");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
}

/// The shell of the desktop app, in little: a folder a click names is
/// plugged in when it is out and pulled when it is in.
fn shell_follows(shell: &mut Shell, app: &mut Faraday) {
    for folder in app.take_stick_toggles() {
        if shell.present.contains(&folder) {
            shell.pull(app, &folder);
        } else {
            if !shell.disks.iter().any(|d| d.id == folder) {
                shell.add(Disk::empty(&folder, &folder));
            }
            shell.plug(app, &folder);
        }
    }
}

#[test]
fn the_default_pills_are_the_boot_stick_and_stick_2_both_in() {
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell);
    let t = drawn(&mut app);
    assert!(t.iter().any(|s| s == "Boot stick · in"), "{t:?}");
    assert!(t.iter().any(|s| s == "Stick 2 · in"), "{t:?}");
    assert!(!t.iter().any(|s| s.starts_with("Stick 1")), "{t:?}");
    assert!(!t.iter().any(|s| s.starts_with("Stick 3")), "{t:?}");
}

#[test]
fn an_app_with_no_pills_draws_none() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    let t = drawn(&mut app);
    assert!(!t.iter().any(|s| s.starts_with("Boot stick")), "{t:?}");
}

#[test]
fn a_click_pulls_an_inserted_stick_and_a_second_click_puts_it_back() {
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell);
    click(&mut app, Action::StickPill(2));
    shell_follows(&mut shell, &mut app);
    let t = drawn(&mut app);
    assert!(t.iter().any(|s| s == "Stick 2 · out"), "{t:?}");
    assert!(t.iter().any(|s| s == "Boot stick · in"), "{t:?}");
    click(&mut app, Action::StickPill(2));
    shell_follows(&mut shell, &mut app);
    let t = drawn(&mut app);
    assert!(t.iter().any(|s| s == "Stick 2 · in"), "{t:?}");
}

#[test]
fn several_sticks_may_be_in_at_once() {
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell);
    app.press(Action::StickPill(1));
    assert_eq!(app.take_stick_toggles(), vec![BOOT_FOLDER.to_string()]);
    app.press(Action::StickPill(3));
    assert_eq!(app.take_stick_toggles(), vec!["STICK-3".to_string()]);
    app.press(Action::StickPill(3));
    shell_follows(&mut shell, &mut app);
    let t = drawn(&mut app);
    for want in ["Boot stick · in", "Stick 2 · in", "Stick 3 · in"] {
        assert!(t.iter().any(|s| s == want), "{want} in {t:?}");
    }
}

fn create_with_vaults(app: &mut Faraday, count: usize) {
    app.press(Action::CreateWallet);
    let multi = NewKind::ALL
        .iter()
        .position(|k| *k == NewKind::Multi)
        .unwrap() as u8;
    app.press(Action::CKind(multi));
    app.press(Action::CVaults(true));
    app.press(Action::CNext(cstep::WHERE));
    app.press(Action::CVaultN(count as u16));
    app.press(Action::CNext(cstep::HOW_MANY));
    assert_eq!(app.screen, Screen::BackupFlow);
}

#[test]
fn a_wallet_backup_with_three_vaults_shows_the_boot_stick_and_three_sticks() {
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell);
    create_with_vaults(&mut app, 3);
    let t = drawn(&mut app);
    for want in [
        "Boot stick · in",
        "Stick 2 · in",
        "Stick 3 · out",
        "Stick 4 · out",
    ] {
        assert!(t.iter().any(|s| s == want), "{want} in {t:?}");
    }
    assert!(!t.iter().any(|s| s.starts_with("Stick 5")), "{t:?}");
}

#[test]
fn five_vaults_give_the_boot_stick_and_stick_2_to_stick_6() {
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell);
    create_with_vaults(&mut app, 5);
    let pills = app.stick_pills();
    let labels: Vec<&str> = pills.iter().map(|p| p.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Boot stick",
            "Stick 2",
            "Stick 3",
            "Stick 4",
            "Stick 5",
            "Stick 6"
        ]
    );
}

#[test]
fn the_flow_reports_the_stick_it_asks_for_at_each_step() {
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell);
    assert_eq!(app.sticks_wanted(), None, "no flow, no sticks asked for");
    shell.pull(&mut app, BOOT_FOLDER);
    shell.pull(&mut app, "STICK-2");
    create_with_vaults(&mut app, 3);
    assert_eq!(app.flow().unwrap().step, Step::Sticks);
    let at = |count, now| Some(SticksWanted { count, now });
    // By the pills' numbers: the first backup stick is Stick 2.
    assert_eq!(app.sticks_wanted(), at(3, Some(2)));
    // Each stick is brought in and pulled, one at a time.
    for (k, next) in [(1usize, Some(3)), (2, Some(4)), (3, None)] {
        let mut d = Disk::empty(&format!("V{k}"), "KEYSTICK");
        d.serial = Some(100 + k as u32);
        shell.add(d);
        shell.plug(&mut app, &format!("V{k}"));
        shell.flow(&mut app, F::Tap);
        shell.pull(&mut app, &format!("V{k}"));
        assert_eq!(app.sticks_wanted(), at(3, next), "after stick {k}");
    }
}

#[test]
fn the_pills_show_the_stick_asked_for_as_the_one_to_insert() {
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell);
    create_with_vaults(&mut app, 3);
    let asked: Vec<usize> = app
        .stick_pills()
        .iter()
        .filter(|p| p.asked)
        .map(|p| p.n)
        .collect();
    assert_eq!(asked, vec![2]);
}

#[test]
fn a_new_start_with_nothing_plugged_shows_every_pill_out() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    app.shell_sticks = true;
    app.storage(StorageEvent::Sticks(Vec::new()));
    let t = drawn(&mut app);
    assert!(t.iter().any(|s| s == "Boot stick · out"), "{t:?}");
    assert!(t.iter().any(|s| s == "Stick 2 · out"), "{t:?}");
}

#[test]
fn with_the_boot_stick_and_stick_2_in_a_backup_flow_reads_stick_2_and_does_not_ask_for_one_at_a_time()
 {
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell);
    create_with_vaults(&mut app, 3);
    shell.pump(&mut app);
    let t = drawn(&mut app);
    assert!(
        !t.iter().any(|s| s == "One stick at a time"),
        "no crowd warning: {t:?}"
    );
    assert!(
        app.flow().unwrap().live.reading.is_some() || app.flow().unwrap().slots[0].bound.is_some(),
        "Stick 2 was read"
    );
}

// ---- on a device: the PC stick and the Pi (docs/BACKUP-FLOW.md §16) ----

/// A device with the clock kept: its sticks are real, put in by hand.
fn device() -> Faraday {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    assert!(!app.shell_sticks);
    keep_time(&mut app);
    app
}

/// Which pills are lit, by number.
fn lit(app: &Faraday) -> Vec<usize> {
    app.stick_pills()
        .iter()
        .filter(|p| p.inserted)
        .map(|p| p.n)
        .collect()
}

/// A GPG key's flow on Bring in the sticks: two places.
fn gpg_on_sticks(shell: &mut Shell) -> Faraday {
    let mut app = device();
    start(&mut app, Kind::Gpg);
    shell.flow(&mut app, F::Continue);
    assert_eq!(app.flow().unwrap().step, Step::Sticks);
    app
}

/// A stick with a file on it to tell it by.
fn marked_disk(id: &str, label: &str, serial: u32) -> Disk {
    let mut d = Disk::empty(id, label);
    d.files = vec![("notes.txt".into(), format!("{id}'s notes").into_bytes())];
    d.serial = Some(serial);
    d
}

#[test]
fn on_a_device_there_are_no_pills_without_a_flow_and_one_per_stick_of_the_flow() {
    let mut app = device();
    assert!(app.stick_pills().is_empty());
    let t = drawn(&mut app);
    assert!(!t.iter().any(|s| s.starts_with("Stick ")), "{t:?}");
    create_with_vaults(&mut app, 3);
    let labels: Vec<String> = app.stick_pills().into_iter().map(|p| p.label).collect();
    assert_eq!(labels, ["Boot stick", "Stick 2", "Stick 3", "Stick 4"]);
    let t = drawn(&mut app);
    for want in ["Stick 2 · out", "Stick 3 · out", "Stick 4 · out"] {
        assert!(t.iter().any(|s| s == want), "{want} in {t:?}");
    }
    // On the Pi the medium is an SD card.
    app.medium = Medium::SdCard;
    let labels: Vec<String> = app.stick_pills().into_iter().map(|p| p.label).collect();
    assert_eq!(labels, ["Boot card", "Card 2", "Card 3", "Card 4"]);
}

#[test]
fn on_a_device_the_boot_pill_comes_first_and_no_pill_is_a_press() {
    let mut shell = Shell::new();
    let mut boot = Disk::empty("BOOT", "FARADAY");
    boot.boot = true;
    shell.add(boot);
    let mut app = device();
    shell.plug(&mut app, "BOOT");
    create_with_vaults(&mut app, 3);
    let pills = app.stick_pills();
    assert_eq!(pills[0].n, 1);
    assert_eq!(pills[0].label, "Boot stick");
    assert!(pills[0].inserted, "the boot stick is in");
    let t = drawn(&mut app);
    assert!(t.iter().any(|s| s == "Boot stick · in"), "{t:?}");
    for n in 0..=4 {
        assert!(!app.offers(Action::StickPill(n)), "pill {n} is offered");
    }
    app.press(Action::StickPill(2));
    assert!(app.take_stick_toggles().is_empty());
}

#[test]
fn the_first_backup_stick_is_stick_2_on_every_shell() {
    // Faraday Desktop and the demo.
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell);
    create_with_vaults(&mut app, 2);
    let labels: Vec<String> = app.stick_pills().into_iter().map(|p| p.label).collect();
    assert_eq!(labels, ["Boot stick", "Stick 2", "Stick 3"]);
    assert_eq!(app.sticks_wanted().unwrap().now, Some(2));
    app.press(Action::StickPill(2));
    assert_eq!(app.take_stick_toggles(), ["STICK-2".to_string()]);
    // A device: the PC stick, then the Pi.
    let mut app = device();
    create_with_vaults(&mut app, 2);
    let labels: Vec<String> = app.stick_pills().into_iter().map(|p| p.label).collect();
    assert_eq!(labels, ["Boot stick", "Stick 2", "Stick 3"]);
    app.medium = Medium::SdCard;
    let labels: Vec<String> = app.stick_pills().into_iter().map(|p| p.label).collect();
    assert_eq!(labels, ["Boot card", "Card 2", "Card 3"]);
}

#[test]
fn a_stick_bound_to_its_place_lights_its_own_pill_only() {
    let mut shell = Shell::new();
    shell.add(marked_disk("S1", "WORKSTICK", 5));
    let mut app = gpg_on_sticks(&mut shell);
    assert!(lit(&app).is_empty());
    let asked: Vec<usize> = app
        .stick_pills()
        .iter()
        .filter(|p| p.asked)
        .map(|p| p.n)
        .collect();
    assert_eq!(asked, [2]);
    shell.plug(&mut app, "S1");
    // Read, not yet confirmed: not recognised.
    assert!(lit(&app).is_empty());
    shell.flow(&mut app, F::Tap);
    assert_eq!(lit(&app), [2]);
    shell.pull(&mut app, "S1");
    assert!(lit(&app).is_empty());
    // Put back while Stick 3 is wanted: it is Stick 2, and says so.
    shell.plug(&mut app, "S1");
    assert_eq!(lit(&app), [2]);
    expect(&mut app, "This is Stick 2");
}

/// A GPG key made on a device from two empty sticks of one label and
/// serial, S1 and S2, and locked: the machine restarted, on step 6.
fn gpg_at_write(shell: &mut Shell) -> Faraday {
    shell.add(Disk::empty("S1", "KEYSTICK"));
    shell.add(Disk::empty("S2", "KEYSTICK"));
    let mut app = gpg_on_sticks(shell);
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    shell.plug(&mut app, "S2");
    shell.flow(&mut app, F::Hold);
    shell.pull(&mut app, "S2");
    create_vault(shell, &mut app, 0, "first vault phrase");
    make_key(shell, &mut app, &["Release", "release@example.org"]);
    assert!(app.flow().unwrap().made, "{:?}", app.toast_text());
    shell.flow(&mut app, F::MadeContinue);
    let app = shell.lock(&mut app);
    assert_eq!(app.flow().unwrap().step, Step::Write);
    app
}

#[test]
fn on_a_device_the_writes_name_each_stick_by_its_number() {
    let mut shell = Shell::new();
    let mut app = gpg_at_write(&mut shell);
    expect(&mut app, "Stick 2 · Vault 1 (Place 1)");
    expect(&mut app, "Stick 3 · Vault 1 (Place 2)");
    expect(&mut app, "Insert Stick 2: Vault 1 (Place 1)");
    shell.plug(&mut app, "S1");
    shell.pull(&mut app, "S1");
    expect(&mut app, "Insert Stick 3: Vault 1 (Place 2)");
    app.medium = Medium::SdCard;
    expect(&mut app, "Insert Card 3: Vault 1 (Place 2)");
    app.medium = Medium::Stick;
    shell.plug(&mut app, "S2");
    shell.flow(&mut app, F::Hold);
    shell.pull(&mut app, "S2");
    assert_eq!(app.flow().unwrap().step, Step::Done);
    // The Done card's rows, once it has opened.
    settle_n(&mut app, 100, 5);
    expect_part(&mut app, "Stick 2 · Vault 1 (Place 1) · KEYSTICK");
    expect_part(&mut app, "Stick 3 · Vault 1 (Place 2) · KEYSTICK");
}

#[test]
fn a_stick_that_matches_no_slot_lights_none_and_says_so() {
    let mut shell = Shell::new();
    let mut app = gpg_at_write(&mut shell);
    shell.plug(&mut app, "S1");
    assert_eq!(lit(&app), [2], "the stick being written is Stick 2");
    shell.pull(&mut app, "S1");
    // Stick 2, written, put back for Place 2.
    shell.plug(&mut app, "S1");
    expect(&mut app, "This stick has the first copy");
    expect(&mut app, "This is Stick 2");
    assert_eq!(lit(&app), [2]);
    shell.pull(&mut app, "S1");
    // A stick from elsewhere.
    shell.add(marked_disk("S3", "OTHER", 77));
    shell.plug(&mut app, "S3");
    expect(&mut app, "Not Stick 3, read for Vault 1 (Place 2)");
    expect(&mut app, "Not one of this backup's sticks");
    assert!(lit(&app).is_empty());
}

#[test]
fn the_mark_line_shows_once_a_stick_is_bound_and_goes_when_it_is_pulled() {
    let mut shell = Shell::new();
    shell.add(marked_disk("S1", "WORKSTICK", 5));
    shell.add(marked_disk("S2", "SPARE", 6));
    let mut app = gpg_on_sticks(&mut shell);
    shell.plug(&mut app, "S1");
    assert!(!has(&mut app, "Mark it as Stick 2"));
    shell.flow(&mut app, F::Tap);
    expect(&mut app, "Mark it as Stick 2");
    shell.pull(&mut app, "S1");
    assert!(
        !has(&mut app, "Mark it as Stick 2"),
        "{:?}",
        texts(&mut app)
    );
    shell.plug(&mut app, "S2");
    shell.flow(&mut app, F::Tap);
    expect(&mut app, "Mark it as Stick 3");
    assert!(!has(&mut app, "Mark it as Stick 2"));
    // On the Pi it is a card.
    app.medium = Medium::SdCard;
    expect(&mut app, "Mark it as Card 3");
}

#[test]
fn on_the_pis_panel_a_five_stick_flow_fits_on_one_row_in_numbers_alone() {
    let mut app = device();
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
    app.medium = Medium::SdCard;
    assert!(app.is_compact());
    create_with_vaults(&mut app, 5);
    // The boot card's pill, then the five.
    assert_eq!(app.stick_pills().len(), 6);
    let _ = app.frame();
    let boxes = app.drawn_text_boxes();
    // Each number's topmost drawing is its pill, across the top.
    let pills: Vec<(i32, i32, i32, i32)> = (1..=6)
        .map(|n| {
            boxes
                .iter()
                .filter(|(t, _)| *t == n.to_string())
                .map(|(_, b)| *b)
                .min_by_key(|b| b.1)
                .unwrap_or_else(|| panic!("no pill {n} in {boxes:?}"))
        })
        .collect();
    // No pill in its short form: the card rows below read "Card 2 · …".
    assert!(
        !boxes
            .iter()
            .any(|(t, _)| t == "Boot card" || (2..=6).any(|n| *t == format!("Card {n}"))),
        "{boxes:?}"
    );
    // A glyph's ink sits a pixel or so higher or lower than another's.
    let y = pills[0].1;
    assert!(
        pills.iter().all(|b| (b.1 - y).abs() <= 3),
        "one row: {pills:?}"
    );
    assert!(y < 60, "across the top: {pills:?}");
    assert!(
        pills.iter().all(|b| b.0 >= 0 && b.0 + b.2 <= 480),
        "on the panel: {pills:?}"
    );
}

/// What the pills say: the asked one's number and the lit ones'.
fn asked(app: &Faraday) -> Vec<usize> {
    app.stick_pills()
        .iter()
        .filter(|p| p.asked)
        .map(|p| p.n)
        .collect()
}

#[test]
fn the_boot_stick_taken_for_a_place_is_stick_1_and_the_others_count_on_from_2() {
    let mut shell = Shell::new();
    let mut boot = Disk::empty("BOOT", "FARADAY");
    boot.boot = true;
    shell.add(boot);
    let mut app = device();
    shell.plug(&mut app, "BOOT");
    start(&mut app, Kind::Gpg);
    shell.flow(&mut app, F::Continue);
    assert_eq!(app.flow().unwrap().step, Step::Sticks);
    let k = app.sticks().iter().position(|s| s.boot).unwrap();
    shell.flow(&mut app, F::UseBoot(k));
    shell.flow(&mut app, F::Tap);
    let f = app.flow().unwrap();
    assert!(f.slots[0].bound.is_some(), "Place 1 is the boot stick's");
    // One number for the boot stick: its own pill, Stick 1.
    assert_eq!(lit(&app), [1]);
    let labels: Vec<String> = app.stick_pills().into_iter().map(|p| p.label).collect();
    assert_eq!(labels, ["Boot stick", "Stick 2"]);
    assert_eq!(asked(&app), [2]);
    expect(&mut app, "Boot stick · Vault 1 (Place 1)");
    expect(&mut app, "Stick 2 · Vault 1 (Place 2)");
    expect(&mut app, "Insert Stick 2: Vault 1 (Place 2)");
    assert!(!has_part(&mut app, "Mark it as"), "{:?}", texts(&mut app));
    // One stick, one place: the boot stick is not offered again.
    assert!(!offered(&mut app, Action::Flow(F::UseBoot(k))));
    assert!(
        !has_part(&mut app, "Use Boot stick"),
        "{:?}",
        texts(&mut app)
    );
    shell.flow(&mut app, F::UseBoot(k));
    assert!(app.flow().unwrap().slots[1].bound.is_none());
    assert!(app.flow().unwrap().live.reading.is_none());
}

#[test]
fn the_pill_edged_and_the_insert_line_name_the_same_stick() {
    let mut shell = Shell::new();
    shell.add(marked_disk("S1", "WORKSTICK", 5));
    let mut app = device();
    create_with_vaults(&mut app, 2);
    // Place 1 keeps a stick of files too, which the write step asks for.
    let i = app.backup_flows().len() - 1;
    faraday_core::testkit::flows_mut(&mut app)[i].slots.insert(
        1,
        Slot {
            target: Target::PlaceFiles(0),
            files: Vec::new(),
            bound: None,
            written: None,
        },
    );
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    assert_eq!(app.flow().unwrap().step, Step::Sticks);
    // Vault 2's stick is wanted: Stick 4, after the stick of files.
    assert_eq!(app.sticks_wanted().unwrap().now, Some(4));
    assert_eq!(asked(&app), [4]);
    expect(&mut app, "Insert Stick 4: Vault 2");
    expect(&mut app, "Stick 3 · Files (Place 1)");
}

#[test]
fn a_blank_stick_put_back_while_another_is_wanted_reads_as_its_number() {
    let mut shell = Shell::new();
    shell.add(Disk::empty("S1", "KEYSTICK"));
    let mut app = gpg_on_sticks(&mut shell);
    shell.plug(&mut app, "S1");
    shell.flow(&mut app, F::Tap);
    shell.pull(&mut app, "S1");
    // Stick 2 put back while Stick 3 is wanted: it reads alike.
    shell.plug(&mut app, "S1");
    expect(&mut app, "Hold: this is the stick for Vault 1 (Place 2)");
    expect(&mut app, "This reads as Stick 2");
    // Not settled: no pill is lit.
    assert!(lit(&app).is_empty());
}
