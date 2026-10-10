//! Upgrading a Faraday stick (`PLAN.md` §5.5), with the shell and the
//! boot copier stood in for: what the app asks for, and what it shows and
//! refuses given each answer. The copier itself is tested on image files
//! in `faraday-boot`, and the two together in `faraday-storage`.

use faraday_core::testkit;
use faraday_core::upgrade::{Fit, Step, version};
use faraday_core::{
    Action, BootPart, Faraday, Medium, Screen, Sheet, StickInfo, StorageCommand, StorageEvent,
};
use osk_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware};

/// A source whose release is our own choosing, rather than `read_source`'s
/// fixed `RUNNING`: the boot stick in, read as the source.
fn read_source_release(app: &mut Faraday, release: &str) {
    app.storage(StorageEvent::Boots(vec![part(
        "sda1@sda#1",
        48 * MB,
        Some(release),
        false,
    )]));
    assert_eq!(asked(app), vec![StorageCommand::BootRead]);
    app.storage(StorageEvent::BootSource {
        id: "sda1@sda#1".into(),
        release: release.into(),
        size: 48 * MB,
    });
}

/// What this Faraday's kernel carries.
const RUNNING: &str = "6.6.84-faraday-0.2.0+4d0680b1a2b3";
const MB: u64 = 1 << 20;

/// A PC whose screen is tall enough to show all of Settings unscrolled.
fn device() -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width: 1366,
        height: 2000,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    let _ = app.frame();
    app
}

fn part(id: &str, size: u64, release: Option<&str>, source: bool) -> BootPart {
    BootPart {
        id: id.into(),
        size,
        release: release.map(str::to_string),
        source,
    }
}

/// What the app has asked the shell for since last asked.
fn asked(app: &mut Faraday) -> Vec<StorageCommand> {
    std::iter::from_fn(|| app.poll_storage())
        .filter(|c| !matches!(c, StorageCommand::SaveBoxes { .. }))
        .collect()
}

/// Settings → Upgrade, on a clean device.
fn open(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    assert!(app.offers(Action::UpgradeOpen));
    app.press(Action::UpgradeOpen);
    assert_eq!(app.screen, Screen::Upgrade);
    assert!(app.upgrading());
    asked(app);
}

/// The boot stick in, read as the source.
fn read_source(app: &mut Faraday) {
    app.storage(StorageEvent::Boots(vec![part(
        "sda1@sda#1",
        48 * MB,
        Some(RUNNING),
        false,
    )]));
    assert_eq!(asked(app), vec![StorageCommand::BootRead]);
    app.storage(StorageEvent::BootSource {
        id: "sda1@sda#1".into(),
        release: RUNNING.into(),
        size: 48 * MB,
    });
}

fn upgrade(app: &Faraday) -> &faraday_core::upgrade::UpgradeState {
    app.upgrade.as_ref().expect("the upgrade is on screen")
}

#[test]
fn the_online_app_offers_no_upgrade() {
    let mut app = device();
    app.online = true;
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    assert!(!app.offers(Action::UpgradeOpen));
    app.press(Action::UpgradeOpen);
    assert_ne!(app.screen, Screen::Upgrade);
}

#[test]
fn the_stick_faraday_started_from_is_read_and_another_is_written_from_it() {
    let mut app = device();
    open(&mut app);
    assert_eq!(upgrade(&app).step(), Step::Source);
    read_source(&mut app);
    assert_eq!(upgrade(&app).step(), Step::Target);
    assert!(upgrade(&app).target().is_none());
    // The stick to upgrade: an older Faraday, with no version in it. Its
    // data partition arrives too, and is not visited.
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "sdb2@sdb#2".into(),
        label: "OSKDATA".into(),
        boot: false,
        files: vec![("vault.ofv".into(), 16_778_240)],
    }]));
    assert_eq!(app.screen, Screen::Upgrade);
    app.storage(StorageEvent::Boots(vec![
        part("sda1@sda#1", 48 * MB, Some(RUNNING), true),
        part("sdb1@sdb#2", 48 * MB, None, false),
    ]));
    let target = upgrade(&app).target().cloned().expect("a stick to upgrade");
    assert_eq!(version(target.release.as_deref()), "0.1.0 or earlier");
    assert_eq!(version(Some(RUNNING)), "0.2.0 (4d0680b1a2b3)");
    assert_eq!(upgrade(&app).fit(&target), Fit::Fits);
    assert_eq!(
        app.upgrade_data_of(&target).map(|s| s.files.len()),
        Some(1),
        "its data partition, by the same disk"
    );
    let _ = app.frame();
    assert!(app.offers(Action::UpgradeWrite));
    app.press(Action::UpgradeWrite);
    assert_eq!(
        asked(&mut app),
        vec![StorageCommand::BootWrite {
            target: "sdb1@sdb#2".into()
        }]
    );
    // Pressed again while it writes: nothing more.
    app.press(Action::UpgradeWrite);
    assert!(asked(&mut app).is_empty());
    app.storage(StorageEvent::BootWritten {
        id: "sdb1@sdb#2".into(),
        release: RUNNING.into(),
    });
    assert_eq!(upgrade(&app).step(), Step::Done);
    // Another stick, from the same source.
    app.press(Action::UpgradeAgain);
    assert_eq!(upgrade(&app).step(), Step::Target);
    // Done: back to Settings, and the copier forgets the source.
    app.press(Action::Nav(Screen::Settings));
    assert!(!app.upgrading());
    assert_eq!(asked(&mut app), vec![StorageCommand::BootForget]);
}

#[test]
fn with_both_sticks_in_the_one_holding_this_faraday_is_the_source() {
    let mut app = device();
    open(&mut app);
    // Both in at once, the older first by name: the copier decides by
    // what each holds, and the app writes to the other.
    app.storage(StorageEvent::Boots(vec![
        part("sda1@sda#1", 48 * MB, None, false),
        part("sdb1@sdb#5", 48 * MB, Some(RUNNING), false),
    ]));
    assert_eq!(asked(&mut app), vec![StorageCommand::BootRead]);
    app.storage(StorageEvent::BootSource {
        id: "sdb1@sdb#5".into(),
        release: RUNNING.into(),
        size: 48 * MB,
    });
    let target = upgrade(&app).target().cloned();
    assert_eq!(target.map(|t| t.id), Some("sda1@sda#1".to_string()));
}

#[test]
fn a_newer_stick_is_warned_about_and_a_smaller_or_same_one_is_not_written() {
    let mut app = device();
    open(&mut app);
    read_source(&mut app);
    let newer = part(
        "sdb1@sdb#2",
        48 * MB,
        Some("6.6.84-faraday-0.10.0+0123456789ab"),
        false,
    );
    let small = part("sdc1@sdc#3", 32 * MB, None, false);
    let same = part("sdd1@sdd#4", 48 * MB, Some(RUNNING), false);
    let older = part(
        "sde1@sde#5",
        48 * MB,
        Some("6.6.84-faraday-0.1.9+0123456789ab"),
        false,
    );
    app.storage(StorageEvent::Boots(vec![
        part("sda1@sda#1", 48 * MB, Some(RUNNING), true),
        newer.clone(),
        small.clone(),
        same.clone(),
        older.clone(),
    ]));
    let u = upgrade(&app);
    assert_eq!(u.targets().len(), 4);
    assert_eq!(u.fit(&newer), Fit::Newer);
    assert_eq!(u.fit(&small), Fit::TooSmall(32, 48));
    assert_eq!(u.fit(&same), Fit::Same);
    assert_eq!(u.fit(&older), Fit::Fits);
    // The newer one is written when asked: it is a warning.
    app.press(Action::UpgradeWrite);
    assert_eq!(
        asked(&mut app),
        vec![StorageCommand::BootWrite {
            target: newer.id.clone()
        }]
    );
    app.storage(StorageEvent::BootWriteFailed {
        id: newer.id.clone(),
        reason: "the stick failed".into(),
        pulled: false,
    });
    // The smaller one and the one already this version are not.
    for i in [1u8, 2] {
        app.press(Action::UpgradePick(i));
        app.press(Action::UpgradeWrite);
        assert!(asked(&mut app).is_empty(), "target {i}");
    }
}

#[test]
fn a_stick_pulled_during_the_write_is_said_and_written_again_when_back() {
    let mut app = device();
    open(&mut app);
    read_source(&mut app);
    app.storage(StorageEvent::Boots(vec![
        part("sda1@sda#1", 48 * MB, Some(RUNNING), true),
        part("sdb1@sdb#2", 48 * MB, None, false),
    ]));
    app.press(Action::UpgradeWrite);
    asked(&mut app);
    app.storage(StorageEvent::BootWriteFailed {
        id: "sdb1@sdb#2".into(),
        reason: "the stick was removed".into(),
        pulled: true,
    });
    assert_eq!(
        upgrade(&app).failed,
        Some(("the stick was removed".into(), true))
    );
    assert_eq!(upgrade(&app).step(), Step::Target);
    // Put back: a new disk number, written again from the source still
    // held.
    app.storage(StorageEvent::Boots(vec![
        part("sda1@sda#1", 48 * MB, Some(RUNNING), true),
        part("sdb1@sdb#6", 48 * MB, Some("6.6.84"), false),
    ]));
    assert!(asked(&mut app).is_empty(), "the source is not read again");
    app.press(Action::UpgradeWrite);
    assert_eq!(
        asked(&mut app),
        vec![StorageCommand::BootWrite {
            target: "sdb1@sdb#6".into()
        }]
    );
}

#[test]
fn a_refused_source_is_said_and_tried_again_only_with_another_stick() {
    let mut app = device();
    open(&mut app);
    app.storage(StorageEvent::Boots(vec![part(
        "sdb1@sdb#2",
        48 * MB,
        None,
        false,
    )]));
    assert_eq!(asked(&mut app), vec![StorageCommand::BootRead]);
    app.storage(StorageEvent::BootSourceFailed {
        reason: "No stick in holds this Faraday".into(),
    });
    assert_eq!(
        upgrade(&app).refused.as_deref(),
        Some("No stick in holds this Faraday")
    );
    // The same stick listed again: not read again.
    app.storage(StorageEvent::Boots(vec![part(
        "sdb1@sdb#2",
        48 * MB,
        None,
        true,
    )]));
    assert!(asked(&mut app).is_empty());
    // The boot stick goes in beside it.
    app.storage(StorageEvent::Boots(vec![
        part("sda1@sda#7", 48 * MB, Some(RUNNING), false),
        part("sdb1@sdb#2", 48 * MB, None, false),
    ]));
    assert_eq!(asked(&mut app), vec![StorageCommand::BootRead]);
    assert_eq!(upgrade(&app).refused, None);
}

#[test]
fn with_a_key_loaded_it_locks_first_and_the_fresh_process_opens_on_the_upgrade() {
    let mut app = device();
    app.press(Action::Entry(None));
    for c in testkit::test_words(testkit::TEST_SEEDS[0].0).chars() {
        app.event(Event::Key(Key::Char(c)));
    }
    app.press(Action::EntryAdd);
    assert!(!app.clean());
    app.press(Action::Nav(Screen::Settings));
    app.press(Action::UpgradeOpen);
    assert_eq!(app.sheet, Some(Sheet::Lock));
    assert!(!app.upgrading());
    // Not now: nothing waits.
    app.press(Action::NotNow);
    assert_eq!(app.sheet, None);
    assert!(!app.not_now);
    app.press(Action::UpgradeOpen);
    app.press(Action::Lock);
    assert!(app.restart_requested());
    let kept = std::iter::from_fn(|| app.poll_storage())
        .filter_map(|c| match c {
            StorageCommand::SaveBoxes { kept, .. } => Some(kept),
            _ => None,
        })
        .last()
        .expect("the lock keeps what the next process needs");
    // The fresh process.
    let mut next = device();
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept,
    });
    assert!(next.clean());
    assert_eq!(next.screen, Screen::Upgrade);
    assert!(next.upgrading());
}

#[test]
fn leaving_the_upgrade_ends_it_and_a_stick_then_is_visited() {
    let mut app = device();
    open(&mut app);
    read_source(&mut app);
    app.press(Action::Nav(Screen::Home));
    assert!(!app.upgrading());
    assert_eq!(asked(&mut app), vec![StorageCommand::BootForget]);
    // The copier's late answers change nothing.
    app.storage(StorageEvent::Boots(vec![part(
        "sdb1@sdb#2",
        48 * MB,
        None,
        false,
    )]));
    assert!(asked(&mut app).is_empty());
    app.storage(StorageEvent::Sticks(vec![StickInfo {
        id: "sdb2@sdb#2".into(),
        label: "OSKDATA".into(),
        boot: false,
        files: Vec::new(),
    }]));
    assert_eq!(app.screen, Screen::Visit);
}

#[test]
fn on_the_pi_the_upgrade_names_an_sd_card() {
    let mut app = device();
    app.medium = Medium::SdCard;
    app.press(Action::Nav(Screen::Settings));
    let _ = app.frame();
    assert!(app.offers(Action::UpgradeOpen));
    assert_eq!(app.medium.upgrade(), "Upgrade a Faraday SD card");
}

#[test]
fn the_upgrade_s_learn_page_takes_secure_boot_through_a_spare_stick() {
    let mut app = device();
    open(&mut app);
    let pages = app.learn_pages();
    let page = pages.first().expect("the upgrade has a Learn page");
    assert_eq!(page.title, "Upgrading a Faraday stick");
    let secure = page
        .sections
        .iter()
        .find(|s| s.heading == "With Secure Boot")
        .expect("a Secure Boot section");
    assert!(secure.paragraphs.iter().any(|p| p.starts_with("4. ")));
    // It is Faraday's alone: OpenSigner's Learn has no such page.
    assert!(
        osk_learn::EN
            .pages()
            .iter()
            .all(|p| p.title != "Upgrading a Faraday stick")
    );
}

// Build kinds (docs/DECISIONS.md F5): a test release written over another
// commit's test release fits and is written; two builds of the same
// commit with different uncommitted changes are not the same build; the
// same build string is still refused; a dev source warns over anything
// that is not itself dev, and still writes; a dev source fits a dev
// target of another commit; a higher version number is Newer.

#[test]
fn a_test_release_fits_over_another_commit_s_test_release_and_is_written() {
    let mut app = device();
    open(&mut app);
    read_source_release(&mut app, "6.6.84-faraday-0.1.0+aaaaaaaaaaaa.test");
    let target = part(
        "sdb1@sdb#2",
        48 * MB,
        Some("6.6.84-faraday-0.1.0+bbbbbbbbbbbb.test"),
        false,
    );
    app.storage(StorageEvent::Boots(vec![
        part(
            "sda1@sda#1",
            48 * MB,
            Some("6.6.84-faraday-0.1.0+aaaaaaaaaaaa.test"),
            true,
        ),
        target.clone(),
    ]));
    assert_eq!(upgrade(&app).fit(&target), Fit::Fits);
    app.press(Action::UpgradeWrite);
    assert_eq!(
        asked(&mut app),
        vec![StorageCommand::BootWrite {
            target: target.id.clone()
        }]
    );
}

#[test]
fn two_test_builds_of_the_same_commit_with_different_changes_are_not_the_same_build() {
    let mut app = device();
    open(&mut app);
    read_source_release(
        &mut app,
        "6.6.84-faraday-0.1.0+aaaaaaaaaaaa.dirty-11111111.test",
    );
    let target = part(
        "sdb1@sdb#2",
        48 * MB,
        Some("6.6.84-faraday-0.1.0+aaaaaaaaaaaa.dirty-22222222.test"),
        false,
    );
    app.storage(StorageEvent::Boots(vec![
        part(
            "sda1@sda#1",
            48 * MB,
            Some("6.6.84-faraday-0.1.0+aaaaaaaaaaaa.dirty-11111111.test"),
            true,
        ),
        target.clone(),
    ]));
    assert_eq!(
        upgrade(&app).fit(&target),
        Fit::Fits,
        "different changes on the same commit are not the same build"
    );
    app.press(Action::UpgradeWrite);
    assert_eq!(
        asked(&mut app),
        vec![StorageCommand::BootWrite {
            target: target.id.clone()
        }]
    );
}

#[test]
fn a_target_with_the_same_build_string_is_still_refused() {
    let mut app = device();
    open(&mut app);
    let release = "6.6.84-faraday-0.1.0+aaaaaaaaaaaa.dirty-11111111.test";
    read_source_release(&mut app, release);
    let target = part("sdb1@sdb#2", 48 * MB, Some(release), false);
    app.storage(StorageEvent::Boots(vec![
        part("sda1@sda#1", 48 * MB, Some(release), true),
        target.clone(),
    ]));
    assert_eq!(upgrade(&app).fit(&target), Fit::Same);
    app.press(Action::UpgradeWrite);
    assert!(asked(&mut app).is_empty());
}

#[test]
fn a_dev_source_warns_over_a_test_a_release_and_an_old_target_and_still_writes() {
    let mut app = device();
    open(&mut app);
    read_source_release(&mut app, "6.6.84-faraday-0.1.0+aaaaaaaaaaaa.dev");
    let test_target = part(
        "sdb1@sdb#2",
        48 * MB,
        Some("6.6.84-faraday-0.1.0+bbbbbbbbbbbb.test"),
        false,
    );
    // A released version, higher than the source's: Dev still outranks
    // Newer (docs/DECISIONS.md F5's order).
    let release_target = part("sdc1@sdc#3", 48 * MB, Some("6.6.84-faraday-0.9.0"), false);
    let old_target = part("sdd1@sdd#4", 48 * MB, None, false);
    app.storage(StorageEvent::Boots(vec![
        part(
            "sda1@sda#1",
            48 * MB,
            Some("6.6.84-faraday-0.1.0+aaaaaaaaaaaa.dev"),
            true,
        ),
        test_target.clone(),
        release_target.clone(),
        old_target.clone(),
    ]));
    let u = upgrade(&app);
    assert_eq!(u.fit(&test_target), Fit::Dev);
    assert_eq!(u.fit(&release_target), Fit::Dev);
    assert_eq!(u.fit(&old_target), Fit::Dev);
    // The warning does not block the write.
    app.press(Action::UpgradeWrite);
    assert_eq!(
        asked(&mut app),
        vec![StorageCommand::BootWrite {
            target: test_target.id.clone()
        }]
    );
}

#[test]
fn a_dev_source_fits_a_dev_target_of_another_commit() {
    let mut app = device();
    open(&mut app);
    read_source_release(&mut app, "6.6.84-faraday-0.1.0+aaaaaaaaaaaa.dev");
    let dev_target = part(
        "sdb1@sdb#2",
        48 * MB,
        Some("6.6.84-faraday-0.1.0+cccccccccccc.dev"),
        false,
    );
    app.storage(StorageEvent::Boots(vec![
        part(
            "sda1@sda#1",
            48 * MB,
            Some("6.6.84-faraday-0.1.0+aaaaaaaaaaaa.dev"),
            true,
        ),
        dev_target.clone(),
    ]));
    assert_eq!(upgrade(&app).fit(&dev_target), Fit::Fits);
}

#[test]
fn a_higher_version_number_is_newer() {
    let mut app = device();
    open(&mut app);
    read_source_release(&mut app, "6.6.84-faraday-0.1.0+aaaaaaaaaaaa.test");
    let target = part(
        "sdb1@sdb#2",
        48 * MB,
        Some("6.6.84-faraday-0.9.0+bbbbbbbbbbbb.test"),
        false,
    );
    app.storage(StorageEvent::Boots(vec![
        part(
            "sda1@sda#1",
            48 * MB,
            Some("6.6.84-faraday-0.1.0+aaaaaaaaaaaa.test"),
            true,
        ),
        target.clone(),
    ]));
    assert_eq!(upgrade(&app).fit(&target), Fit::Newer);
}

#[test]
fn the_upgrade_screen_names_a_release_a_test_release_a_dev_build_and_the_old_format() {
    assert_eq!(version(Some("6.6.84-faraday-0.2.0")), "0.2.0");
    assert_eq!(
        version(Some("6.6.84-faraday-0.1.0+ef24784b48d8.test")),
        "0.1.0 test release (ef24784b48d8)"
    );
    assert_eq!(
        version(Some("6.6.84-faraday-0.1.0+ef24784b48d8.dev")),
        "0.1.0 dev (ef24784b48d8)"
    );
    assert_eq!(
        version(Some("6.6.84-faraday-0.1.0+4d0680b1a2b3")),
        "0.1.0 (4d0680b1a2b3)"
    );
}

#[test]
fn about_shows_the_version_label_local_build_without_a_faraday_build_id() {
    let mut app = device();
    app.press(Action::Nav(Screen::Settings));
    let texts = app.drawn_texts();
    // The test binary carries no FARADAY_BUILD (faraday_core::lib.rs).
    assert!(faraday_core::BUILD.is_none());
    let want = format!("Faraday {}", faraday_core::version_label());
    assert!(want.ends_with("(local build)"), "{want:?}");
    assert!(texts.iter().any(|t| t == &want), "{want:?}: {texts:?}");
}
