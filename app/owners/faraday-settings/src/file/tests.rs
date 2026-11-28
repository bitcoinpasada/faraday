use faraday_kernel::settings::{
    Appearance, Choice, Contrast, MotionSpeed, MotionStyle, Settings, ThemeId,
};
use faraday_port::values::Orientation;

use super::*;

fn file(lines: &str) -> Vec<u8> {
    format!("{HEADER}\n{lines}").into_bytes()
}

fn from_stick(bytes: &[u8]) -> Settings {
    let mut s = Settings::default();
    read(&mut s, bytes);
    s
}

fn same(a: &Settings, b: &Settings) -> bool {
    body(a) == body(b) && a.zone_min == b.zone_min
}

/// Invariant: A settings file of exactly the largest size is read and one a byte over it is ignored whole.
/// Regression: The size check is dropped or moves one byte, so an oversized file from a stick is parsed or a full one refused.
/// Owner: app
#[test]
fn a_file_over_the_largest_size_is_ignored_whole() {
    let mut full = file("scale=150\n").into_iter().collect::<Vec<u8>>();
    // Comment-like lines with no `=` pad the file to the size without changing a setting.
    while full.len() < MAX {
        full.push(b'x');
    }
    assert_eq!(full.len(), MAX);
    assert_eq!(from_stick(&full).scale_pct, 150);
    full.push(b'x');
    assert!(same(&from_stick(&full), &Settings::default()));
}

/// Invariant: A settings file that is not text, or does not open with the format's first line, leaves the defaults.
/// Regression: A file that is not UTF-8, has no header, a longer header or a header alone is applied line by line anyway.
/// Owner: app
#[test]
fn a_file_not_in_the_format_leaves_the_defaults() {
    let mut not_text = file("scale=150\n");
    not_text.push(0xff);
    for bytes in [
        not_text,
        b"scale=150\n".to_vec(),
        format!("{HEADER}0\nscale=150\n").into_bytes(),
        format!("{HEADER} \nscale=150\n").into_bytes(),
        HEADER.as_bytes().to_vec(),
        Vec::new(),
    ] {
        assert!(same(&from_stick(&bytes), &Settings::default()));
    }
}

/// Invariant: A line is taken only when its key is known and its value is one Settings offers; any other line, cut short or not, leaves that setting as it was.
/// Regression: A truncated, unknown or out-of-range line sets a guessed value, or fails the whole file.
/// Owner: app
#[test]
fn a_damaged_line_leaves_its_setting_and_no_other() {
    let s = from_stick(&file(
        "scale=1\nscale\nrotation=sideways\nunknown=1\nguided=on\nqr-ms=499\ntext=huge\nspeed=\nidle-lock=3\nidle-off=x\nqr-ms=333\nrotation=portrait\n",
    ));
    let want = Settings {
        qr_frame_ms: 333,
        rotation: Orientation::Portrait,
        ..Settings::default()
    };
    assert!(same(&s, &want));
}

/// Invariant: A file written on a computer that ends its lines with a carriage return reads as the same file.
/// Regression: The carriage return stays in the header or in a value, so the file is ignored or a value is refused.
/// Owner: app
#[test]
fn carriage_returns_at_the_line_ends_are_read_through() {
    let s = from_stick(format!("{HEADER}\r\nscale=125\r\nmotion=off\r\n").as_bytes());
    assert_eq!(s.scale_pct, 125);
    assert_eq!(s.appearance.motion, Some(MotionStyle::Off));
}

/// Invariant: Only the scales and animated-code speeds Settings offers are taken from a file.
/// Regression: Any number parses into the scale or the frame time, so a stick sets a scale of 1 percent or a frame time of 0.
/// Owner: app
#[test]
fn a_scale_or_frame_time_not_offered_is_refused() {
    let s = from_stick(&file(
        "scale=0\nscale=300\nscale=-100\nqr-ms=0\nqr-ms=100\n",
    ));
    assert!(same(&s, &Settings::default()));
    let s = from_stick(&file("scale=75\nscale=76\nqr-ms=200\nqr-ms=201\n"));
    assert_eq!((s.scale_pct, s.qr_frame_ms), (75, 200));
}

/// Invariant: A stick cannot turn an idle time off, but the device's own kept settings can.
/// Regression: The never rule is dropped for a stick, or applied to the carry, so a stuck-in file switches off the idle lock or a person's own never is lost across a lock.
/// Owner: app
#[test]
fn never_for_an_idle_time_is_kept_from_the_carry_and_refused_from_a_stick() {
    let from_a_stick = from_stick(&file("idle-lock=0\nidle-off=0\n"));
    assert_eq!(
        (from_a_stick.idle_lock_min, from_a_stick.idle_off_min),
        (30, 60)
    );
    let mut kept = Settings::default();
    read_kept(&mut kept, "idle-lock=0\nidle-off=0\n");
    assert_eq!((kept.idle_lock_min, kept.idle_off_min), (0, 0));
    // The offered times are the stick's too.
    let s = from_stick(&file("idle-lock=2\nidle-off=10\n"));
    assert_eq!((s.idle_lock_min, s.idle_off_min), (2, 10));
}

/// Invariant: A theme named before the three choices opens in its own mode and contrast, and a later `mode=` or `contrast=` line sets that part as written.
/// Regression: The lines are applied out of order or the old theme words stop mapping, so a kept Dark scheme opens Light.
/// Owner: app
#[test]
fn an_old_theme_word_opens_in_its_scheme_and_later_lines_override_it() {
    let s = from_stick(&file("theme=high-contrast-dark\n"));
    assert!(s.appearance.dark);
    assert_eq!(s.appearance.contrast, Contrast::High);
    let s = from_stick(&file(
        "theme=high-contrast-dark\nmode=light\ncontrast=medium\n",
    ));
    assert!(!s.appearance.dark);
    assert_eq!(s.appearance.contrast, Contrast::Medium);
    let swapped = from_stick(&file("mode=dark\ntheme=light\n"));
    assert!(!swapped.appearance.dark);
    let unknown = from_stick(&file("theme=solarized\nmode=sepia\ncontrast=extreme\n"));
    assert!(same(&unknown, &Settings::default()));
}

/// Invariant: The words written while Reduce motion was a switch still read, and an unknown motion word leaves motion not chosen.
/// Regression: `full` and `reduced` stop reading, so a person's Reduce motion is lost, or an unknown word turns motion off.
/// Owner: app
#[test]
fn old_motion_words_read_and_an_unknown_one_leaves_motion_unchosen() {
    assert_eq!(
        from_stick(&file("motion=reduced\n")).appearance.motion,
        Some(MotionStyle::Off)
    );
    assert_eq!(
        from_stick(&file("motion=full\n")).appearance.motion,
        Some(MotionStyle::Smooth)
    );
    assert_eq!(from_stick(&file("motion=wobbly\n")).appearance.motion, None);
    assert_eq!(
        from_stick(&file("speed=quick\n")).appearance.speed,
        MotionSpeed::Quick
    );
}

/// Invariant: A time zone is read only as a sign, two digits, a colon and two digits, in quarters of an hour from -12:00 to +14:00.
/// Regression: The zone is read looser than the format, so `+-1:00` or `+14:15` sets a zone no one chose.
/// Owner: app
#[test]
fn a_time_zone_outside_the_format_is_refused() {
    for (text, want) in [
        ("+00:00", Some(0)),
        ("-12:00", Some(-720)),
        ("+14:00", Some(840)),
        ("+05:45", Some(345)),
        ("-03:30", Some(-210)),
        ("+14:15", None),
        ("-12:15", None),
        ("+13:10", None),
        ("+5:00", None),
        ("+05:0", None),
        ("0500", None),
        ("+05-00", None),
        ("+-1:00", None),
        ("++1:00", None),
        ("+00:+0", None),
        ("+ 1:00", None),
        ("+05:00 ", None),
        ("", None),
        ("+٠٥:٠٠", None),
    ] {
        assert_eq!(parse_tz_offset(text), want, "{text}");
    }
    assert_eq!(tz_offset_text(-210), "-03:30");
}

/// Invariant: The file's text reads back as the settings it was written from, a motion choice and a zone are written only when chosen, and the file's own keys are all it writes.
/// Regression: A setting is written but not read (or the reverse), or a default motion or an unset zone is written and outlives the screen it was for.
/// Owner: app
#[test]
fn the_files_text_reads_back_and_writes_only_what_was_chosen() {
    let defaults = Settings::default();
    let text = file_text(&defaults);
    assert!(!text.contains("motion="));
    assert!(!text.contains("tz="));
    assert!(text.starts_with(HEADER));
    assert!(same(&from_stick(text.as_bytes()), &defaults));

    let chosen = Settings {
        scale_pct: 200,
        rotation: Orientation::Portrait,
        appearance: Appearance {
            theme: ThemeId(3),
            dark: true,
            contrast: Contrast::High,
            motion: Some(MotionStyle::Crossfade),
            speed: MotionSpeed::Quick,
            ..Settings::default().appearance
        },
        qr_frame_ms: 200,
        idle_lock_min: 5,
        idle_off_min: 20,
        zone_min: Some(-210),
        seal_amounts: true,
        nonce_history: true,
        offset_secs: 90,
        checked: true,
    };
    let text = file_text(&chosen);
    assert!(text.contains("motion=crossfade\n"));
    assert!(text.contains("tz=-03:30\n"));
    for key in ["seal", "nonce", "offset", "checked"] {
        assert!(!text.contains(key), "{key}");
    }
    let back = from_stick(text.as_bytes());
    assert_eq!(body(&back), body(&chosen));
    assert_eq!(back.appearance, chosen.appearance);
    assert_eq!(back.zone_min, Some(-210));
    // What the file does not hold stays as the device had it.
    assert!(!back.seal_amounts && !back.nonce_history && !back.checked);
}

/// Invariant: A file cannot set what the device chooses each session: the Remembered choices, the clock's offset or whether it was checked.
/// Regression: A new key is accepted from a stick, which takes in more than the file ever held.
/// Owner: app
#[test]
fn a_file_cannot_set_what_it_never_held() {
    let s = from_stick(&file(
        "seal-amounts=true\nnonce-history=true\nseal_amounts=1\nnonce_history=1\noffset=100\nchecked=true\noffset_secs=5\nnetwork=mainnet\n",
    ));
    assert!(!s.seal_amounts && !s.nonce_history && !s.checked);
    assert_eq!(s.offset_secs, 0);
}

/// Invariant: A choice made on the device takes only a value Settings offers, and a zone is held between -12:00 and +14:00.
/// Regression: A choice sent by another feature skips the offered-value check the file has, or a zone beyond the bounds is kept.
/// Owner: app
#[test]
fn a_choice_made_on_the_device_is_held_to_what_is_offered() {
    let mut s = Settings::default();
    assert!(!choose(&mut s, Choice::Scale(101)));
    assert!(!choose(&mut s, Choice::IdleLock(3)));
    assert!(!choose(&mut s, Choice::QrFrame(1)));
    let mut off_table = s.appearance;
    off_table.theme = ThemeId(200);
    assert!(!choose(&mut s, Choice::Appearance(off_table)));
    assert!(same(&s, &Settings::default()));
    assert!(choose(&mut s, Choice::IdleLock(0)));
    assert_eq!(s.idle_lock_min, 0);
    assert!(choose(&mut s, Choice::Zone(5000)));
    assert_eq!(s.zone_min, Some(840));
    assert!(choose(&mut s, Choice::Zone(-5000)));
    assert_eq!(s.zone_min, Some(-720));
    assert!(!choose(&mut s, Choice::Zone(-720)));
}
