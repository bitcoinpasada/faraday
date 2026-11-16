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
        signed: false,
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
        signed: true,
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
        signed: true,
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

#[test]
fn a_windows_stick_shows_no_line() {
    let mut s = starter_with(
        windows(true),
        stick_with(Some(&"8".repeat(64))),
        images("0.2.0"),
    );
    assert!(!shows(&mut s, EN.starter_stick_boot_differs));
    assert!(
        commands(&mut s)
            .iter()
            .all(|c| !matches!(c, StarterCommand::JudgeBoot { .. }))
    );
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
