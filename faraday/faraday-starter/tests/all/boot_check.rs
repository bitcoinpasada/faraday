//! A Faraday stick's boot partition against the release the Starter
//! carries, each time the stick is seen (`docs/STARTER.md` §10.4): the
//! match line, the mismatch line with the rewrite offered, the boot file
//! the person signed, and nothing where no sum is listed or the boot
//! partition was not read.

use faraday_shell_api::App;
use faraday_starter::api::{
    Computer, Disk, FoundImage, ImageFinding, Source, StarterCommand, StarterEvent,
};
use faraday_starter::decide::{BootCheck, boot_check};
use faraday_starter::{Starter, StarterAction};
use faraday_strings::EN;

use crate::starter::{BOOT_SUM, display, faraday_stick, images, linux, windows};

const GB: u64 = 1_000_000_000;

fn starter_with(computer: Computer, stick: Disk, images: Vec<ImageFinding>) -> Starter {
    let mut s = Starter::new();
    s.event(display(1280, 800, 160));
    s.starter_event(StarterEvent::Computer(computer));
    s.starter_event(StarterEvent::ImagesFound(images));
    s.starter_event(StarterEvent::Disks(vec![stick]));
    s
}

fn shows(s: &mut Starter, text: &str) -> bool {
    s.drawn_texts().iter().any(|t| t.contains(text))
}

fn commands(s: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| s.next_command()).collect()
}

fn stick_with(sum: Option<&str>) -> Disk {
    let mut d = faraday_stick("sdb", "Ultra Fit", 16 * GB, Some("6.6.84-faraday-0.2.0"));
    d.faraday.as_mut().expect("a stick").boot_sha256 = sum.map(str::to_string);
    d
}

fn listed_without_a_sum() -> Vec<ImageFinding> {
    images("0.2.0")
        .into_iter()
        .map(|f| match f {
            ImageFinding::Found(i) => ImageFinding::Found(FoundImage {
                boot_sha256: None,
                ..i
            }),
            other => other,
        })
        .collect()
}

#[test]
fn a_stick_whose_boot_partition_is_the_releases_shows_the_match_line() {
    let mut s = starter_with(
        linux(true),
        stick_with(Some(&BOOT_SUM.repeat(64))),
        images("0.2.0"),
    );
    assert!(shows(&mut s, EN.starter_stick_boot_matches));
    assert!(!shows(&mut s, EN.starter_stick_boot_differs));
    assert!(
        commands(&mut s)
            .iter()
            .all(|c| !matches!(c, StarterCommand::JudgeBoot { .. }))
    );
}

#[test]
fn a_boot_partition_with_one_byte_changed_shows_the_mismatch_line_and_offers_the_rewrite() {
    let mut s = starter_with(
        linux(true),
        stick_with(Some(&"8".repeat(64))),
        images("0.2.0"),
    );
    let asked = commands(&mut s);
    let Some(StarterCommand::JudgeBoot {
        disk,
        serial,
        size,
        sha256,
        want,
        source: Source::Found { .. },
    }) = asked
        .into_iter()
        .find(|c| matches!(c, StarterCommand::JudgeBoot { .. }))
    else {
        panic!("the shell is not asked to judge the partition");
    };
    // Nothing is said until the shell has judged it.
    assert!(!shows(&mut s, EN.starter_stick_boot_matches));
    assert!(!shows(&mut s, EN.starter_stick_boot_differs));
    // The shell is asked about the partition as it was listed.
    assert_eq!(sha256, "8".repeat(64));
    s.starter_event(StarterEvent::BootJudged {
        disk,
        serial,
        size,
        sha256,
        want,
        signed: Some(false),
    });
    assert!(shows(&mut s, EN.starter_stick_boot_differs));
    assert!(!shows(&mut s, EN.starter_stick_boot_matches));
    // The rewrite is the Make a Faraday stick the line sits in.
    assert!(shows(&mut s, EN.starter_item_make));
    assert!(
        s.offers_action(StarterAction::Item(
            faraday_starter::decide::Item::MakeStick
        )) || s.offers_action(StarterAction::Make(faraday_starter::decide::Target::Stick))
    );
}

#[test]
fn a_boot_partition_with_the_boot_file_the_person_signed_reads_as_expected() {
    let mut s = starter_with(
        linux(true),
        stick_with(Some(&"8".repeat(64))),
        images("0.2.0"),
    );
    s.starter_event(StarterEvent::BootJudged {
        disk: "sdb".into(),
        serial: "SN-sdb".into(),
        size: 16 * GB,
        sha256: "8".repeat(64),
        want: BOOT_SUM.repeat(64),
        signed: Some(true),
    });
    assert!(shows(&mut s, EN.starter_stick_boot_signed));
    assert!(!shows(&mut s, EN.starter_stick_boot_differs));
}

#[test]
fn a_judgement_of_a_partition_the_stick_no_longer_has_says_nothing() {
    let mut s = starter_with(
        linux(true),
        stick_with(Some(&"8".repeat(64))),
        images("0.2.0"),
    );
    s.starter_event(StarterEvent::BootJudged {
        disk: "sdb".into(),
        serial: "SN-sdb".into(),
        size: 16 * GB,
        sha256: "7".repeat(64),
        want: BOOT_SUM.repeat(64),
        signed: Some(true),
    });
    assert!(!shows(&mut s, EN.starter_stick_boot_signed));
}

#[test]
fn an_image_with_no_boot_sum_listed_shows_no_line_and_asks_nothing() {
    let mut s = starter_with(
        linux(true),
        stick_with(Some(&"8".repeat(64))),
        listed_without_a_sum(),
    );
    assert!(!shows(&mut s, EN.starter_stick_boot_matches));
    assert!(!shows(&mut s, EN.starter_stick_boot_differs));
    assert!(
        commands(&mut s)
            .iter()
            .all(|c| !matches!(c, StarterCommand::JudgeBoot { .. }))
    );
    let stick = stick_with(Some(&"8".repeat(64)));
    let f = stick.faraday.as_ref().expect("a stick");
    assert_eq!(boot_check(f, None, Some(false)), None);
    assert_eq!(boot_check(f, None, None), None);
}

#[test]
fn a_stick_whose_boot_partition_was_not_read_shows_no_line() {
    let mut s = starter_with(linux(true), stick_with(None), images("0.2.0"));
    assert!(!shows(&mut s, EN.starter_stick_boot_matches));
    assert!(!shows(&mut s, EN.starter_stick_boot_differs));
}

/// A Faraday stick as Windows lists it without elevation: its boot
/// partition not read.
fn unread_windows_stick() -> Disk {
    let mut d = stick_with(None);
    d.id = r"\\.\PhysicalDrive2".into();
    let f = d.faraday.as_mut().expect("a stick");
    f.read = false;
    f.release = None;
    d
}

/// The press of Check this stick, and the one read it asks the elevated
/// copy for, as the shell is told it.
fn checked(s: &mut Starter) -> StarterCommand {
    let _ = s.drawn_texts();
    assert!(s.offers_action(StarterAction::CheckStick));
    s.press(StarterAction::CheckStick);
    let asked = commands(s);
    assert_eq!(asked.len(), 1, "one read, one administrator prompt");
    asked.into_iter().next().expect("one command")
}

fn answered(s: &mut Starter, asked: StarterCommand, sum: &str, signed: Option<bool>) {
    let StarterCommand::ReadVersion {
        disk, serial, size, ..
    } = asked
    else {
        panic!("the shell is not asked to read the stick");
    };
    s.starter_event(StarterEvent::VersionRead {
        disk,
        serial,
        size,
        result: Ok((Some("6.6.84-faraday-0.2.0".into()), Some(sum.into()))),
        signed,
    });
}

#[test]
fn a_windows_stick_shows_no_line_before_the_button_and_the_match_line_after_it() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    assert!(shows(&mut s, EN.starter_stick_check));
    assert!(!shows(&mut s, EN.starter_stick_boot_matches));
    assert!(!shows(&mut s, EN.starter_stick_boot_differs));
    // Nothing is asked of the shell before the press.
    assert!(commands(&mut s).is_empty());
    let asked = checked(&mut s);
    assert!(matches!(
        asked,
        StarterCommand::ReadVersion {
            source: Some(Source::Found { .. }),
            ..
        }
    ));
    answered(&mut s, asked, &BOOT_SUM.repeat(64), None);
    assert!(shows(&mut s, EN.starter_stick_boot_matches));
    assert!(!shows(&mut s, EN.starter_stick_check));
    assert!(
        commands(&mut s)
            .iter()
            .all(|c| !matches!(c, StarterCommand::JudgeBoot { .. }))
    );
}

#[test]
fn a_windows_stick_with_one_byte_changed_shows_the_mismatch_line_and_offers_the_rewrite() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    let asked = checked(&mut s);
    answered(&mut s, asked, &"8".repeat(64), Some(false));
    s.press(StarterAction::Home);
    assert!(shows(&mut s, EN.starter_stick_boot_differs));
    assert!(!shows(&mut s, EN.starter_stick_boot_matches));
    assert!(s.offers_action(StarterAction::Item(
        faraday_starter::decide::Item::MakeStick
    )));
    // The read judged it: the shell is not asked a second time.
    assert!(
        commands(&mut s)
            .iter()
            .all(|c| !matches!(c, StarterCommand::JudgeBoot { .. }))
    );
}

#[test]
fn a_windows_stick_with_the_boot_file_the_person_signed_reads_as_expected() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    let asked = checked(&mut s);
    answered(&mut s, asked, &"8".repeat(64), Some(true));
    s.press(StarterAction::Home);
    assert!(shows(&mut s, EN.starter_stick_boot_signed));
    assert!(!shows(&mut s, EN.starter_stick_boot_differs));
}

#[test]
fn a_linux_stick_is_not_offered_the_button() {
    let mut s = starter_with(
        linux(true),
        stick_with(Some(&BOOT_SUM.repeat(64))),
        images("0.2.0"),
    );
    s.press(StarterAction::MakePage);
    assert!(!shows(&mut s, EN.starter_stick_check));
}

#[test]
fn the_decision_tells_match_signed_and_differing_apart() {
    let stick = stick_with(Some(&"8".repeat(64)));
    let f = stick.faraday.as_ref().expect("a stick");
    let want = "8".repeat(64);
    assert_eq!(boot_check(f, Some(&want), None), Some(BootCheck::Matches));
    assert_eq!(
        boot_check(f, Some(&"9".repeat(64)), Some(true)),
        Some(BootCheck::MatchesSigned)
    );
    assert_eq!(
        boot_check(f, Some(&"9".repeat(64)), Some(false)),
        Some(BootCheck::Differs)
    );
}

fn another_release() -> Vec<ImageFinding> {
    images("0.2.0")
        .into_iter()
        .map(|f| match f {
            ImageFinding::Found(i) => ImageFinding::Found(FoundImage {
                boot_sha256: Some("5".repeat(64)),
                ..i
            }),
            other => other,
        })
        .collect()
}

#[test]
fn keep_datas_read_judges_nothing_and_the_button_stays_for_the_stick_it_read() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::Make(faraday_starter::decide::Target::Stick));
    let n = StarterAction::Disk(s.disk_number(r"\\.\PhysicalDrive2"));
    s.press(n);
    s.press(StarterAction::Mode(faraday_starter::api::Mode::KeepData));
    let asked = commands(&mut s);
    assert!(
        matches!(
            asked.as_slice(),
            [StarterCommand::ReadVersion { source: None, .. }]
        ),
        "keep data's read waits for nothing but the boot partition"
    );
    answered(
        &mut s,
        asked.into_iter().next().expect("one command"),
        &"8".repeat(64),
        None,
    );
    s.press(StarterAction::Home);
    s.press(StarterAction::MakePage);
    assert!(shows(&mut s, EN.starter_stick_check));
    assert!(!shows(&mut s, EN.starter_stick_boot_differs));
}

#[test]
fn a_windows_judgement_made_against_one_release_shows_nothing_for_another() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    let asked = checked(&mut s);
    answered(&mut s, asked, &"8".repeat(64), Some(true));
    assert!(!shows(&mut s, EN.starter_stick_check));
    s.starter_event(StarterEvent::ImagesFound(another_release()));
    assert!(shows(&mut s, EN.starter_stick_check));
    assert!(!shows(&mut s, EN.starter_stick_boot_signed));
    s.starter_event(StarterEvent::ImagesFound(images("0.2.0")));
    assert!(!shows(&mut s, EN.starter_stick_check));
}

#[test]
fn a_linux_judgement_made_against_one_release_is_asked_again_for_another() {
    let mut s = starter_with(
        linux(true),
        stick_with(Some(&"8".repeat(64))),
        images("0.2.0"),
    );
    s.starter_event(StarterEvent::BootJudged {
        disk: "sdb".into(),
        serial: "SN-sdb".into(),
        size: 16 * GB,
        sha256: "8".repeat(64),
        want: BOOT_SUM.repeat(64),
        signed: Some(true),
    });
    assert!(shows(&mut s, EN.starter_stick_boot_signed));
    let _ = commands(&mut s);
    s.starter_event(StarterEvent::ImagesFound(another_release()));
    assert!(!shows(&mut s, EN.starter_stick_boot_signed));
    assert!(
        commands(&mut s)
            .iter()
            .any(|c| matches!(c, StarterCommand::JudgeBoot { .. }))
    );
}

/// A long line wraps on the card: its first words are what is looked for.
fn first_words(text: &str) -> String {
    text.split(' ').take(3).collect::<Vec<_>>().join(" ")
}

/// A press of Check this stick that the elevated copy answers with `why`.
fn check_failed(s: &mut Starter, why: faraday_starter::api::Reason) {
    let StarterCommand::ReadVersion {
        disk, serial, size, ..
    } = checked(s)
    else {
        panic!("the shell is not asked to read the stick");
    };
    s.starter_event(StarterEvent::VersionRead {
        disk,
        serial,
        size,
        result: Err(why),
        signed: None,
    });
}

#[test]
fn a_declined_check_says_so_on_the_card_where_it_was_pressed() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    let asked = checked(&mut s);
    let StarterCommand::ReadVersion {
        disk, serial, size, ..
    } = asked
    else {
        panic!("the shell is not asked to read the stick");
    };
    s.starter_event(StarterEvent::VersionRead {
        disk,
        serial,
        size,
        result: Err(faraday_starter::api::Reason::Declined),
        signed: None,
    });
    assert!(shows(&mut s, &first_words(EN.starter_why_declined)));
    assert!(shows(&mut s, EN.starter_stick_check));
}

#[test]
fn two_judgements_of_one_partition_answered_in_reverse_order_each_show_under_their_own_release() {
    let mut s = starter_with(
        linux(true),
        stick_with(Some(&"8".repeat(64))),
        images("0.2.0"),
    );
    let judge = |s: &mut Starter| {
        commands(s)
            .into_iter()
            .find_map(|c| match c {
                StarterCommand::JudgeBoot {
                    disk,
                    serial,
                    size,
                    sha256,
                    want,
                    ..
                } => Some((disk, serial, size, sha256, want)),
                _ => None,
            })
            .expect("the shell is asked to judge the partition")
    };
    let a = judge(&mut s);
    s.starter_event(StarterEvent::ImagesFound(another_release()));
    let b = judge(&mut s);
    assert_ne!(a.4, b.4);
    let answer = |s: &mut Starter, k: (String, String, u64, String, String), signed| {
        s.starter_event(StarterEvent::BootJudged {
            disk: k.0,
            serial: k.1,
            size: k.2,
            sha256: k.3,
            want: k.4,
            signed,
        });
    };
    answer(&mut s, b, Some(false));
    answer(&mut s, a, Some(true));
    assert!(shows(&mut s, EN.starter_stick_boot_differs));
    assert!(!shows(&mut s, EN.starter_stick_boot_signed));
    s.starter_event(StarterEvent::ImagesFound(images("0.2.0")));
    assert!(shows(&mut s, EN.starter_stick_boot_signed));
    assert!(!shows(&mut s, EN.starter_stick_boot_differs));
}

#[test]
fn a_stick_the_elevated_copy_cannot_judge_says_so_and_is_not_offered_the_check_again() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    check_failed(&mut s, faraday_starter::api::Reason::NotJudged);
    assert!(shows(&mut s, &first_words(EN.starter_why_not_judged)));
    assert!(!shows(&mut s, EN.starter_stick_check));
    assert!(!shows(&mut s, EN.starter_stick_boot_matches));
    assert!(!shows(&mut s, EN.starter_stick_boot_signed));
    assert!(!s.offers_action(StarterAction::CheckStick));
    // Another release is a new question.
    s.starter_event(StarterEvent::ImagesFound(another_release()));
    assert!(shows(&mut s, EN.starter_stick_check));
    // And the line about the release before is not said under it.
    assert!(!shows(&mut s, &first_words(EN.starter_why_not_judged)));
}

#[test]
fn a_declined_check_is_not_said_under_another_release() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    check_failed(&mut s, faraday_starter::api::Reason::Declined);
    assert!(shows(&mut s, &first_words(EN.starter_why_declined)));
    s.starter_event(StarterEvent::ImagesFound(another_release()));
    assert!(!shows(&mut s, &first_words(EN.starter_why_declined)));
}

#[test]
fn keep_data_ticked_on_another_stick_does_not_take_the_checks_failure_as_its_reason() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    let mut other = unread_windows_stick();
    other.id = r"\\.\PhysicalDrive3".into();
    other.serial = "SN-other".into();
    s.starter_event(StarterEvent::Disks(vec![unread_windows_stick(), other]));
    s.press(StarterAction::MakePage);
    let StarterCommand::ReadVersion {
        disk, serial, size, ..
    } = checked(&mut s)
    else {
        panic!("the shell is not asked to read the stick");
    };
    s.press(StarterAction::Make(faraday_starter::decide::Target::Stick));
    let n = StarterAction::Disk(s.disk_number(r"\\.\PhysicalDrive3"));
    s.press(n);
    s.press(StarterAction::Mode(faraday_starter::api::Mode::KeepData));
    s.starter_event(StarterEvent::VersionRead {
        disk,
        serial,
        size,
        result: Err(faraday_starter::api::Reason::Declined),
        signed: None,
    });
    assert!(!shows(&mut s, &first_words(EN.starter_why_declined)));
}

#[test]
fn two_linux_sticks_that_cannot_be_judged_each_say_so() {
    let mut first = stick_with(Some(&"8".repeat(64)));
    first.id = "sdb".into();
    let mut second = stick_with(Some(&"7".repeat(64)));
    second.id = "sdc".into();
    second.serial = "SN-sdc".into();
    let mut s = Starter::new();
    s.event(display(1280, 800, 160));
    s.starter_event(StarterEvent::Computer(linux(true)));
    s.starter_event(StarterEvent::ImagesFound(images("0.2.0")));
    s.starter_event(StarterEvent::Disks(vec![first, second]));
    for c in commands(&mut s) {
        if let StarterCommand::JudgeBoot {
            disk,
            serial,
            size,
            sha256,
            want,
            ..
        } = c
        {
            s.starter_event(StarterEvent::BootJudged {
                disk,
                serial,
                size,
                sha256,
                want,
                signed: None,
            });
        }
    }
    s.press(StarterAction::MakePage);
    assert!(shows(&mut s, &first_words(EN.starter_why_not_judged)));
}

#[test]
fn a_failed_check_shows_on_the_card_for_that_stick_only() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    // A read nobody pressed Check this stick for (keep data's) says
    // nothing on the card.
    s.starter_event(StarterEvent::VersionRead {
        disk: r"\\.\PhysicalDrive2".into(),
        serial: "SN-sdb".into(),
        size: 16 * GB,
        result: Err(faraday_starter::api::Reason::Declined),
        signed: None,
    });
    assert!(!shows(&mut s, &first_words(EN.starter_why_declined)));
    check_failed(&mut s, faraday_starter::api::Reason::Declined);
    assert!(shows(&mut s, &first_words(EN.starter_why_declined)));
    // Another stick first in the list: the line is not about it.
    let mut other = unread_windows_stick();
    other.id = r"\\.\PhysicalDrive1".into();
    s.starter_event(StarterEvent::Disks(vec![other, unread_windows_stick()]));
    assert!(!shows(&mut s, &first_words(EN.starter_why_declined)));
    // The stick pulled out: the line is gone when it comes back.
    s.starter_event(StarterEvent::Disks(vec![unread_windows_stick()]));
    check_failed(&mut s, faraday_starter::api::Reason::Declined);
    s.starter_event(StarterEvent::Disks(Vec::new()));
    s.starter_event(StarterEvent::Disks(vec![unread_windows_stick()]));
    assert!(!shows(&mut s, &first_words(EN.starter_why_declined)));
}

#[test]
fn a_stick_whose_sum_is_the_releases_shows_the_match_line_when_the_read_came_with_no_judgement() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    let asked = checked(&mut s);
    answered(&mut s, asked, &BOOT_SUM.repeat(64), None);
    s.press(StarterAction::Home);
    assert!(shows(&mut s, EN.starter_stick_boot_matches));
    assert!(!shows(&mut s, &first_words(EN.starter_why_not_judged)));
}

#[test]
fn a_stick_read_but_not_judged_says_it_could_not_be_checked_and_is_not_offered_again() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    let asked = checked(&mut s);
    answered(&mut s, asked, &"8".repeat(64), None);
    assert!(shows(&mut s, &first_words(EN.starter_why_not_judged)));
    assert!(!shows(&mut s, EN.starter_stick_boot_differs));
    assert!(!shows(&mut s, EN.starter_stick_boot_matches));
    assert!(!s.offers_action(StarterAction::CheckStick));
}

#[test]
fn a_linux_stick_with_no_release_to_judge_by_says_it_could_not_be_checked() {
    let mut s = starter_with(
        linux(true),
        stick_with(Some(&"8".repeat(64))),
        images("0.2.0"),
    );
    let StarterCommand::JudgeBoot {
        disk,
        serial,
        size,
        sha256,
        want,
        ..
    } = commands(&mut s)
        .into_iter()
        .find(|c| matches!(c, StarterCommand::JudgeBoot { .. }))
        .expect("the shell is asked to judge the partition")
    else {
        unreachable!()
    };
    s.starter_event(StarterEvent::BootJudged {
        disk,
        serial,
        size,
        sha256,
        want,
        signed: None,
    });
    s.press(StarterAction::MakePage);
    assert!(shows(&mut s, &first_words(EN.starter_why_not_judged)));
    assert!(!shows(&mut s, EN.starter_stick_boot_differs));
    assert!(
        !commands(&mut s)
            .iter()
            .any(|c| matches!(c, StarterCommand::JudgeBoot { .. })),
        "asked again"
    );
}

#[test]
fn another_stick_under_the_same_disk_id_does_not_show_the_old_failure_line() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    check_failed(&mut s, faraday_starter::api::Reason::Declined);
    assert!(shows(&mut s, &first_words(EN.starter_why_declined)));
    let mut other = unread_windows_stick();
    other.serial = "SN-other".into();
    s.starter_event(StarterEvent::Disks(vec![other]));
    assert!(!shows(&mut s, &first_words(EN.starter_why_declined)));
}

#[test]
fn a_read_of_another_stick_does_not_clear_the_failure_line() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    check_failed(&mut s, faraday_starter::api::Reason::Declined);
    s.starter_event(StarterEvent::VersionRead {
        disk: r"\\.\PhysicalDrive7".into(),
        serial: "SN-elsewhere".into(),
        size: 8 * GB,
        result: Ok((Some("6.6.84-faraday-0.2.0".into()), None)),
        signed: None,
    });
    assert!(shows(&mut s, &first_words(EN.starter_why_declined)));
}

#[test]
fn keep_data_ticked_while_a_check_read_fails_shows_the_reason_on_its_page() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    let asked = checked(&mut s);
    let StarterCommand::ReadVersion {
        disk, serial, size, ..
    } = asked
    else {
        panic!("the shell is not asked to read the stick");
    };
    s.press(StarterAction::Make(faraday_starter::decide::Target::Stick));
    let n = StarterAction::Disk(s.disk_number(r"\\.\PhysicalDrive2"));
    s.press(n);
    s.press(StarterAction::Mode(faraday_starter::api::Mode::KeepData));
    s.starter_event(StarterEvent::VersionRead {
        disk,
        serial,
        size,
        result: Err(faraday_starter::api::Reason::Declined),
        signed: None,
    });
    assert!(shows(&mut s, &first_words(EN.starter_why_declined)));
}

#[test]
fn a_stick_written_after_it_could_not_be_judged_is_not_said_to_be_unjudged_any_more() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    check_failed(&mut s, faraday_starter::api::Reason::NotJudged);
    assert!(shows(&mut s, &first_words(EN.starter_why_not_judged)));
    s.press(StarterAction::Make(faraday_starter::decide::Target::Stick));
    let n = StarterAction::Disk(s.disk_number(r"\\.\PhysicalDrive2"));
    s.press(n);
    s.press(StarterAction::Mode(faraday_starter::api::Mode::Whole));
    s.press(StarterAction::Continue);
    s.press(StarterAction::Write);
    let _ = commands(&mut s);
    s.starter_event(StarterEvent::Written(Ok(())));
    s.press(StarterAction::MakePage);
    assert!(!shows(&mut s, &first_words(EN.starter_why_not_judged)));
    assert!(s.offers_action(StarterAction::CheckStick));
}

#[test]
fn keep_data_stays_ticked_on_a_stick_when_another_sticks_check_read_answers() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    let mut other = unread_windows_stick();
    other.id = r"\\.\PhysicalDrive3".into();
    other.serial = "SN-other".into();
    s.starter_event(StarterEvent::Disks(vec![unread_windows_stick(), other]));
    s.press(StarterAction::MakePage);
    let StarterCommand::ReadVersion {
        disk, serial, size, ..
    } = checked(&mut s)
    else {
        panic!("the shell is not asked to read the stick");
    };
    s.press(StarterAction::Make(faraday_starter::decide::Target::Stick));
    let n = StarterAction::Disk(s.disk_number(r"\\.\PhysicalDrive3"));
    s.press(n);
    s.press(StarterAction::Mode(faraday_starter::api::Mode::KeepData));
    s.starter_event(StarterEvent::VersionRead {
        disk,
        serial,
        size,
        result: Ok((Some("0.2.0".into()), Some("8".repeat(64)))),
        signed: None,
    });
    assert!(commands(&mut s).iter().any(|c| matches!(
        c,
        StarterCommand::ReadVersion { disk, .. } if disk == r"\\.\PhysicalDrive3"
    )));
}

#[test]
fn a_stick_unplugged_and_plugged_in_again_shows_no_match_line_until_it_is_read_again() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    let asked = checked(&mut s);
    answered(&mut s, asked, &BOOT_SUM.repeat(64), None);
    s.press(StarterAction::Home);
    assert!(shows(&mut s, EN.starter_stick_boot_matches));
    s.starter_event(StarterEvent::Disks(vec![]));
    s.starter_event(StarterEvent::Disks(vec![unread_windows_stick()]));
    assert!(!shows(&mut s, EN.starter_stick_boot_matches));
}

#[test]
fn a_stick_that_could_not_be_checked_and_later_reads_as_the_releases_shows_only_the_match_line() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    let asked = checked(&mut s);
    answered(&mut s, asked.clone(), &"8".repeat(64), None);
    assert!(shows(&mut s, &first_words(EN.starter_why_not_judged)));
    answered(&mut s, asked, &BOOT_SUM.repeat(64), None);
    assert!(shows(&mut s, EN.starter_stick_boot_matches));
    assert!(!shows(&mut s, &first_words(EN.starter_why_not_judged)));
}

#[test]
fn a_read_answered_after_its_stick_was_unplugged_does_not_bring_a_match_line_back_on_replug() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    s.press(StarterAction::MakePage);
    let asked = checked(&mut s);
    s.starter_event(StarterEvent::Disks(vec![]));
    answered(&mut s, asked, &BOOT_SUM.repeat(64), None);
    s.starter_event(StarterEvent::Disks(vec![unread_windows_stick()]));
    s.press(StarterAction::Home);
    assert!(!shows(&mut s, EN.starter_stick_boot_matches));
}

#[test]
fn keep_datas_failed_read_of_one_stick_gives_no_reason_on_the_stick_chosen_since() {
    let mut s = starter_with(windows(true), unread_windows_stick(), images("0.2.0"));
    let mut other = unread_windows_stick();
    other.id = r"\\.\PhysicalDrive3".into();
    other.serial = "SN-other".into();
    s.starter_event(StarterEvent::Disks(vec![unread_windows_stick(), other]));
    s.press(StarterAction::MakePage);
    s.press(StarterAction::Make(faraday_starter::decide::Target::Stick));
    let first = StarterAction::Disk(s.disk_number(r"\\.\PhysicalDrive2"));
    s.press(first);
    s.press(StarterAction::Mode(faraday_starter::api::Mode::KeepData));
    let StarterCommand::ReadVersion {
        disk, serial, size, ..
    } = commands(&mut s).into_iter().next().expect("a read")
    else {
        panic!("the shell is not asked to read the stick");
    };
    s.press(StarterAction::Back);
    let n = StarterAction::Disk(s.disk_number(r"\\.\PhysicalDrive3"));
    s.press(n);
    s.press(StarterAction::Mode(faraday_starter::api::Mode::KeepData));
    s.starter_event(StarterEvent::VersionRead {
        disk,
        serial,
        size,
        result: Err(faraday_starter::api::Reason::Declined),
        signed: None,
    });
    assert!(!shows(&mut s, &first_words(EN.starter_why_declined)));
}
