use super::*;

/// Invariant: A `--version` output is a Faraday program's only when its first line is the program's name, a space and a build string, under the name now or an earlier one.
/// Regression: A file of another program, a name with no build, a name that only starts like ours or a later line is taken as one of ours, so the Starter replaces or trusts a file that is not its own.
/// Owner: app
#[test]
fn only_a_line_of_our_own_name_and_a_build_is_a_version_output() {
    let line = version_line(Program::DesktopApp, "1.2.3+abc.test");
    assert_eq!(
        parse_version_output(&line),
        Some((Program::DesktopApp, "1.2.3+abc.test"))
    );
    let old = format!("{} 0.9", EN.name_faraday_starter_app_old);
    assert_eq!(parse_version_output(&old), Some((Program::StarterApp, "0.9")));
    assert_eq!(
        parse_version_output(&format!("  {}  \nignored", version_line(Program::StarterApp, "2.0"))),
        Some((Program::StarterApp, "2.0"))
    );
    for refused in [
        String::new(),
        EN.name_faraday_desktop.to_string(),
        format!("{} ", EN.name_faraday_desktop),
        format!("{}x 1.0", EN.name_faraday_desktop),
        format!("other\n{}", version_line(Program::DesktopApp, "1.0")),
        "Some other program 1.0".to_string(),
    ] {
        assert_eq!(parse_version_output(&refused), None, "{refused:?}");
    }
}

/// Invariant: About names a plain build as a local build of the crate's version, and a build with an ID by that ID described; a release is an ID with no plus.
/// Regression: A local build is shown as a release, or a test release's ID is shown raw.
/// Owner: app
#[test]
fn a_local_build_says_so_and_a_release_has_no_plus() {
    assert_eq!(label(None), fill(EN.version_label_local, &[VERSION]));
    assert_eq!(label(Some("1.0.0")), "1.0.0");
    assert_ne!(label(Some("1.0.0+abc1234.test")), "1.0.0+abc1234.test");
}
