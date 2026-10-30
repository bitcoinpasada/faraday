//! What to write's Name field (`docs/STICK-IDENTITY.md` §6): empty at
//! first and written as no name; a name typed in lower case is written in
//! capitals and the field says so; a name FAT cannot hold, or a boot
//! partition's, keeps Continue from going on; the caret moves with the
//! keys (`docs/TEXT-CURSOR.md`); Confirm shows the name, the write
//! carries it and Done says it was expected; keep data offers no field.

use faraday_shell_api::{App, BootState, DisplayInfo, Event, Key, SecureHardware, TouchPhase};
use faraday_starter::api::{
    Bus, Computer, Disk, FaradayStick, Mode, Os, Payload, PayloadFile, StarterCommand, StarterEvent,
};
use faraday_starter::decide::Target;
use faraday_starter::{Page, Starter, StarterAction};

const GB: u64 = 1_000_000_000;
const SDB: &str = "/org/freedesktop/UDisks2/block_devices/sdb";

fn starter() -> Starter {
    let mut s = Starter::new();
    s.event(Event::Display(DisplayInfo {
        width: 1280,
        height: 800,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    s.starter_event(StarterEvent::Computer(Computer {
        os: Os::Linux,
        maker: "Example Computers".into(),
        model: "Example 14".into(),
        family: String::new(),
        uefi: true,
        arm: false,
        secure_boot: Some(true),
        tpm_unlock: false,
    }));
    s.starter_event(StarterEvent::Payload(Some(Payload {
        version: "0.3.0".into(),
        commit: "89abcdef0123456789abcdef0123456789abcdef".into(),
        files: vec![PayloadFile {
            name: "faraday-stick-0.3.0-x86_64-uefi-strict.img.xz".into(),
            sha256: "a".repeat(64),
            inner: "faraday-stick-0.3.0-x86_64-uefi-strict.img".into(),
            inner_sha256: "b".repeat(64),
            inner_size: 202_395_648,
            build_id: "0.3.0+89abcdef0123".into(),
        }],
    })));
    s.starter_event(StarterEvent::Disks(vec![Disk {
        id: SDB.into(),
        device: "/dev/sdb".into(),
        vendor: "SanDisk".into(),
        model: "Ultra Fit".into(),
        serial: "AAA111".into(),
        size: 16 * GB,
        bus: Bus::Usb,
        volumes: Vec::new(),
        in_use: None,
        faraday: Some(FaradayStick {
            release: Some("6.6.84-faraday-0.2.0".into()),
            read: true,
        }),
    }]));
    s
}

fn shows(s: &mut Starter, text: &str) -> bool {
    s.drawn_texts().iter().any(|t| t.contains(text))
}

fn commands(s: &mut Starter) -> Vec<StarterCommand> {
    std::iter::from_fn(|| s.next_command()).collect()
}

/// What to write for the stick, `mode` ticked.
fn to_mode(s: &mut Starter, mode: Mode) {
    s.press(StarterAction::Make(Target::Stick));
    let n = StarterAction::Disk(s.disk_number(SDB));
    s.press(n);
    s.press(StarterAction::Mode(mode));
    assert_eq!(s.page, Page::Mode);
    commands(s);
}

/// The Name field tapped where it is drawn, as a person taps it.
fn tap_field(s: &mut Starter) {
    let _ = s.drawn_texts();
    let (x, y) = s.where_offered(StarterAction::Name).expect("the field");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        s.event(Event::Touch { x, y, phase });
    }
}

fn type_text(s: &mut Starter, text: &str) {
    for c in text.chars() {
        s.event(Event::Key(Key::Char(c)));
    }
}

fn typed(s: &Starter) -> String {
    s.name_field().expect("offered").typed.clone()
}

fn written(s: &mut Starter) -> Option<String> {
    s.press(StarterAction::Continue);
    assert_eq!(s.page, Page::Confirm);
    s.press(StarterAction::Write);
    match commands(s).as_slice() {
        [StarterCommand::Write(o)] => o.name.clone(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn with_nothing_typed_the_stick_is_written_with_no_name() {
    let mut s = starter();
    to_mode(&mut s, Mode::Whole);
    assert!(shows(&mut s, "Name"));
    assert!(shows(&mut s, "No name"));
    assert_eq!(written(&mut s), None);
    s.starter_event(StarterEvent::Written(Ok(())));
    assert!(shows(
        &mut s,
        "Matches the release image, except this stick's own IDs, as expected"
    ));
}

#[test]
fn a_name_typed_in_lower_case_is_written_in_capitals_and_the_field_says_so() {
    let mut s = starter();
    to_mode(&mut s, Mode::Whole);
    tap_field(&mut s);
    type_text(&mut s, "my stick");
    assert_eq!(typed(&s), "my stick");
    assert!(shows(&mut s, "Written as MY STICK"));
    s.press(StarterAction::Continue);
    assert!(shows(&mut s, "MY STICK"));
    s.press(StarterAction::Write);
    let c = commands(&mut s);
    assert!(
        matches!(c.as_slice(), [StarterCommand::Write(o)] if o.name.as_deref() == Some("MY STICK")),
        "{c:?}"
    );
    s.starter_event(StarterEvent::Written(Ok(())));
    assert!(shows(
        &mut s,
        "Matches the release image, except this stick's own IDs and its name, as expected"
    ));
}

#[test]
fn a_name_fat_cannot_hold_or_a_boot_partitions_keeps_continue_from_going_on() {
    for (name, why) in [
        ("TWELVE CHARS", "At most 11 characters"),
        ("A/B", "Letters, digits, spaces"),
        ("efi", "are boot partitions' names"),
        ("bootfs", "are boot partitions' names"),
    ] {
        let mut s = starter();
        to_mode(&mut s, Mode::Whole);
        tap_field(&mut s);
        type_text(&mut s, name);
        assert!(shows(&mut s, why), "{name}");
        s.press(StarterAction::Continue);
        assert_eq!(s.page, Page::Mode, "{name}");
        // Taken back to a name that can be written, it goes on.
        tap_field(&mut s);
        for _ in 0..name.chars().count() {
            s.event(Event::Key(Key::Backspace));
        }
        type_text(&mut s, "DATA");
        assert_eq!(written(&mut s).as_deref(), Some("DATA"), "{name}");
    }
}

#[test]
fn the_caret_moves_with_the_keys_and_typing_goes_in_at_it() {
    let mut s = starter();
    to_mode(&mut s, Mode::Whole);
    tap_field(&mut s);
    type_text(&mut s, "ACD");
    s.event(Event::Key(Key::Home));
    s.event(Event::Key(Key::Right));
    type_text(&mut s, "B");
    assert_eq!(typed(&s), "ABCD");
    s.event(Event::Key(Key::End));
    s.event(Event::Key(Key::Left));
    s.event(Event::Key(Key::Delete));
    s.event(Event::Key(Key::Backspace));
    assert_eq!(typed(&s), "AB");
    // A press anywhere else lets typing go.
    s.press(StarterAction::Mode(Mode::Whole));
    type_text(&mut s, "X");
    assert_eq!(typed(&s), "AB");
}

#[test]
fn keep_data_offers_no_name_and_writes_none() {
    let mut s = starter();
    to_mode(&mut s, Mode::KeepData);
    assert!(s.name_field().is_none());
    assert!(s.where_offered(StarterAction::Name).is_none());
    assert_eq!(written(&mut s), None);
}
