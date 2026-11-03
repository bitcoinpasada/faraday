//! What a stick's figure at Bring in the sticks tells of it (`docs/STICK-SETUP.md` §5).

use super::*;

/// Stick 2 brought in at Bring in the sticks, reporting `make`, under the
/// label `label`.
fn checked_with(label: &str, make: faraday_core::StickMake) -> Faraday {
    let mut shell = Shell::new();
    let mut d = Disk::empty("S1", label);
    d.make = make;
    shell.add(d);
    let mut app = started(Kind::Gpg);
    through_where(&mut shell, &mut app);
    bring_in(&mut shell, &mut app, "S1", false);
    app
}

#[test]
fn a_checked_figure_tells_the_sticks_maker_model_size_and_name() {
    let mut app = checked_with(
        "BACKUP",
        faraday_core::StickMake {
            maker: Some("SanDisk".into()),
            model: Some("Ultra".into()),
            size: Some(15_700_000_000),
        },
    );
    assert!(has(&mut app, "SanDisk Ultra · 16 GB · BACKUP"));
}

#[test]
fn a_cards_figure_tells_its_name_size_and_label() {
    let mut app = checked_with(
        "DATA",
        faraday_core::StickMake {
            maker: None,
            model: Some("SD32G".into()),
            size: Some(31_900_000_000),
        },
    );
    assert!(has(&mut app, "SD32G · 32 GB · DATA"));
}

#[test]
fn a_small_stick_reads_in_megabytes_and_one_that_tells_nothing_shows_its_name() {
    let mut app = checked_with(
        "TINY",
        faraday_core::StickMake {
            maker: None,
            model: None,
            size: Some(512_000_000),
        },
    );
    assert!(has(&mut app, "512 MB · TINY"));
    let mut app = checked_with("PLAIN", Default::default());
    assert!(has(&mut app, "PLAIN"));
}
