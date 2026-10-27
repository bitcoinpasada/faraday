//! A build ID read out of a kernel release string, described and compared
//! (`docs/DECISIONS.md` F5), as the Starter's Keep data shows and decides
//! by: a test release over another commit's test release fits; two builds
//! of the same commit with different uncommitted changes are not the same
//! build; the same build string is refused; a dev source warns over
//! anything that is not itself dev; a dev source fits a dev target of
//! another commit; a higher version number is Newer. And Settings, which
//! no longer offers to upgrade a stick (F128).

use faraday_core::build::{Fit, build_id, compare, version};
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// How writing `source` over a stick holding `target` stands, both as
/// kernel release strings.
fn fit(target: Option<&str>, source: &str) -> Fit {
    compare(
        target.and_then(build_id),
        build_id(source).expect("a Faraday release string"),
    )
}

#[test]
fn a_test_release_fits_over_another_commit_s_test_release() {
    assert_eq!(
        fit(
            Some("6.6.84-faraday-0.1.0+bbbbbbbbbbbb.test"),
            "6.6.84-faraday-0.1.0+aaaaaaaaaaaa.test"
        ),
        Fit::Fits
    );
}

#[test]
fn two_test_builds_of_the_same_commit_with_different_changes_are_not_the_same_build() {
    assert_eq!(
        fit(
            Some("6.6.84-faraday-0.1.0+aaaaaaaaaaaa.dirty-22222222.test"),
            "6.6.84-faraday-0.1.0+aaaaaaaaaaaa.dirty-11111111.test"
        ),
        Fit::Fits
    );
}

#[test]
fn a_target_with_the_same_build_string_is_the_same() {
    let release = "6.6.84-faraday-0.1.0+aaaaaaaaaaaa.dirty-11111111.test";
    assert_eq!(fit(Some(release), release), Fit::Same);
}

#[test]
fn a_dev_source_warns_over_a_test_a_release_and_an_old_target() {
    let dev = "6.6.84-faraday-0.1.0+aaaaaaaaaaaa.dev";
    assert_eq!(
        fit(Some("6.6.84-faraday-0.1.0+bbbbbbbbbbbb.test"), dev),
        Fit::Dev
    );
    // A released version, higher than the source's: Dev still outranks
    // Newer (docs/DECISIONS.md F5's order).
    assert_eq!(fit(Some("6.6.84-faraday-0.9.0"), dev), Fit::Dev);
    assert_eq!(fit(None, dev), Fit::Dev);
}

#[test]
fn a_dev_source_fits_a_dev_target_of_another_commit() {
    assert_eq!(
        fit(
            Some("6.6.84-faraday-0.1.0+cccccccccccc.dev"),
            "6.6.84-faraday-0.1.0+aaaaaaaaaaaa.dev"
        ),
        Fit::Fits
    );
}

#[test]
fn a_higher_version_number_is_newer() {
    assert_eq!(
        fit(
            Some("6.6.84-faraday-0.9.0+bbbbbbbbbbbb.test"),
            "6.6.84-faraday-0.1.0+aaaaaaaaaaaa.test"
        ),
        Fit::Newer
    );
}

#[test]
fn a_release_a_test_release_a_dev_build_and_the_old_format_are_named() {
    assert_eq!(version(None), "0.1.0 or earlier");
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

#[test]
fn about_shows_the_version_label_local_build_without_a_faraday_build_id() {
    let mut app = device();
    app.press(Action::Nav(Screen::Settings));
    crate::common::settings_section(&mut app, "About");
    let texts = app.drawn_texts();
    // The test binary carries no FARADAY_BUILD (faraday_core::lib.rs).
    assert!(faraday_core::BUILD.is_none());
    let want = format!("Faraday {}", faraday_core::version_label());
    assert!(want.ends_with("(local build)"), "{want:?}");
    assert!(texts.iter().any(|t| t == &want), "{want:?}: {texts:?}");
}

#[test]
fn settings_offers_no_upgrade_of_a_faraday_stick() {
    for online in [false, true] {
        let mut app = device();
        app.online = online;
        app.press(Action::Nav(Screen::Settings));
        let _ = app.frame();
        let texts = app.drawn_texts();
        assert!(
            texts.iter().any(|t| t == "About"),
            "online {online}: Settings is drawn whole: {texts:?}"
        );
        assert!(
            !texts.iter().any(|t| t.contains("Upgrade")),
            "online {online}: Settings offers no upgrade: {texts:?}"
        );
    }
}
