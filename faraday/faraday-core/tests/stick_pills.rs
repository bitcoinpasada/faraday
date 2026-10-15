//! The stick pills across the top of the desktop app and the website
//! demo: the boot stick and Stick 1 by default, a click to insert or
//! pull, one pill for each stick a backup flow needs, and what the flow
//! asks for at each step.

mod common;

use common::*;
use faraday_core::backup_flow::Step;
use faraday_core::backup_flow_steps::FlowAction as F;
use faraday_core::create::NewKind;
use faraday_core::{Action, BOOT_FOLDER, Faraday, Screen, SticksWanted, StorageEvent, cstep};
use osk_shell_api::{App, Event, TouchPhase};

/// A desktop app with the boot stick and Stick 1 in, as the desktop
/// starts.
fn desktop(shell: &mut Shell) -> Faraday {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    app.shell_sticks = true;
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeKeep);
    for folder in [BOOT_FOLDER, "STICK-1"] {
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
fn the_default_pills_are_the_boot_stick_and_stick_1_both_in() {
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell);
    let t = drawn(&mut app);
    assert!(t.iter().any(|s| s == "Boot stick · in"), "{t:?}");
    assert!(t.iter().any(|s| s == "Stick 1 · in"), "{t:?}");
    assert!(!t.iter().any(|s| s.starts_with("Stick 2")), "{t:?}");
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
    click(&mut app, Action::StickPill(1));
    shell_follows(&mut shell, &mut app);
    let t = drawn(&mut app);
    assert!(t.iter().any(|s| s == "Stick 1 · out"), "{t:?}");
    assert!(t.iter().any(|s| s == "Boot stick · in"), "{t:?}");
    click(&mut app, Action::StickPill(1));
    shell_follows(&mut shell, &mut app);
    let t = drawn(&mut app);
    assert!(t.iter().any(|s| s == "Stick 1 · in"), "{t:?}");
}

#[test]
fn several_sticks_may_be_in_at_once() {
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell);
    app.press(Action::StickPill(0));
    assert_eq!(app.take_stick_toggles(), vec![BOOT_FOLDER.to_string()]);
    app.press(Action::StickPill(2));
    shell_follows(&mut shell, &mut app);
    let t = drawn(&mut app);
    for want in ["Boot stick · in", "Stick 1 · in", "Stick 2 · in"] {
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
        "Stick 1 · in",
        "Stick 2 · out",
        "Stick 3 · out",
    ] {
        assert!(t.iter().any(|s| s == want), "{want} in {t:?}");
    }
    assert!(!t.iter().any(|s| s.starts_with("Stick 4")), "{t:?}");
}

#[test]
fn five_vaults_give_the_boot_stick_and_stick_1_to_stick_5() {
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell);
    create_with_vaults(&mut app, 5);
    let pills = app.stick_pills();
    let labels: Vec<&str> = pills.iter().map(|p| p.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Boot stick",
            "Stick 1",
            "Stick 2",
            "Stick 3",
            "Stick 4",
            "Stick 5"
        ]
    );
}

#[test]
fn the_flow_reports_the_stick_it_asks_for_at_each_step() {
    let mut shell = Shell::new();
    let mut app = desktop(&mut shell);
    assert_eq!(app.sticks_wanted(), None, "no flow, no sticks asked for");
    shell.pull(&mut app, BOOT_FOLDER);
    shell.pull(&mut app, "STICK-1");
    create_with_vaults(&mut app, 3);
    assert_eq!(app.flow().unwrap().step, Step::Sticks);
    let at = |count, now| Some(SticksWanted { count, now });
    assert_eq!(app.sticks_wanted(), at(3, Some(1)));
    // Each stick is brought in and pulled, one at a time.
    for (k, next) in [(1usize, Some(2)), (2, Some(3)), (3, None)] {
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
    assert_eq!(asked, vec![1]);
}

#[test]
fn a_new_start_with_nothing_plugged_shows_every_pill_out() {
    let mut app = process(Vec::new(), Vec::new(), Vec::new());
    app.shell_sticks = true;
    app.storage(StorageEvent::Sticks(Vec::new()));
    let t = drawn(&mut app);
    assert!(t.iter().any(|s| s == "Boot stick · out"), "{t:?}");
    assert!(t.iter().any(|s| s == "Stick 1 · out"), "{t:?}");
}

#[test]
fn with_the_boot_stick_and_stick_1_in_a_backup_flow_reads_stick_1_and_does_not_ask_for_one_at_a_time()
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
        "Stick 1 was read"
    );
}
