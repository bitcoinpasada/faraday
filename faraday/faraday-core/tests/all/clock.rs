//! The clock (`docs/TOTP.md` §1–§3): a reading below 2020 is not known,
//! as no reading at all is; a time set by hand is an offset against the
//! shell's own clock, which survives a lock but not a power-off; the
//! time zone is a fixed offset kept in the settings file; and every time
//! shown to a person is local.

use crate::common::RecordsOf as _;
use crate::common::type_text;
use faraday_core::set_time::Focus;
use faraday_core::stick::settings::FILE;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StickInfo, StorageCommand, StorageEvent};
use faraday_shell_api::{
    App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware, TouchPhase,
};
use faraday_vault::records::kind;

fn display() -> Event {
    Event::Display(DisplayInfo {
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
    })
}

/// A fresh app with the test vault unlocked, its GPG keys category open:
/// Add then Save makes a key at the clock's current reading.
fn vault_app(clock: Option<u64>) -> Faraday {
    let mut app = Faraday::new();
    app.event(display());
    for i in 0..6u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
    let vault = testkit::test_vault().unwrap();
    app.storage(StorageEvent::Restored {
        inbox: faraday_core::files_from(vec![(faraday_text::Text::of("vault.kdbx"), vault)]),
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    if let Some(secs) = clock {
        app.storage(StorageEvent::Clock { unix_secs: secs });
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASE);
    app.press(Action::Vault(V::Unlock));
    let _ = app.frame();
    app.event(Event::Tick { now_ms: 1_000 });
    let _ = app.frame();
    if let Some(secs) = clock {
        // Resets the raw reading to this instant, so no drift from
        // settling the unlock creeps into a test's exact seconds.
        app.storage(StorageEvent::Clock { unix_secs: secs });
    }
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(4)));
    app
}

/// Adds a GPG key named "Test" to the open vault, through Add and Save,
/// as a person typing a name and pressing Save would.
fn make_gpg_key(app: &mut Faraday) {
    crate::common::make_released(app, faraday_core::backup_flow::Kind::Gpg, &["Test", ""]);
}

/// The last GPG key made in the open vault's creation time, Unix
/// seconds.
fn gpg_created(app: &Faraday) -> u32 {
    let v = &app.vaults().open[app.vaults().current];
    let r = app
        .vaults()
        .store()
        .contents_for_tests(v.id)
        .iter()
        .rev()
        .find(|r| r.kind == kind::GPG)
        .expect("a GPG record");
    crate::common::gpg_key_of(r)
        .expect("a readable key")
        .created
}

fn gpg_count(app: &Faraday) -> usize {
    app.vaults()
        .store()
        .contents_for_tests(app.vaults().open[app.vaults().current].id)
        .of(kind::GPG)
        .count()
}

fn stick(id: &str, boot: bool, file: Option<&[u8]>) -> StickInfo {
    StickInfo {
        make: Default::default(),
        serial: None,
        id: faraday_text::Text::of(id),
        label: faraday_text::Text::of(&id.to_uppercase()),
        boot,
        files: file
            .map(|f| vec![(faraday_text::Text::of(FILE), f.len() as u64)])
            .unwrap_or_default(),
        vaults: Vec::new(),
    }
}

fn pump_reads(app: &mut Faraday, file: Option<&[u8]>) {
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Read { stick, name } = c
            && name == FILE
            && let Some(bytes) = file
        {
            app.storage(StorageEvent::Read {
                stick,
                name,
                bytes: bytes.to_vec().into(),
            });
        }
    }
}

fn booted(file: Option<&[u8]>) -> Faraday {
    let mut app = Faraday::new();
    app.event(display());
    app.storage(StorageEvent::Sticks(vec![stick("boot", true, file)]));
    pump_reads(&mut app, file);
    app
}

#[test]
fn a_clock_before_2020_is_not_known() {
    // The Pi at boot: its real-time clock starts at 1970. Make a key asks
    // for the time, makes nothing, and offers no Keep this time.
    for clock in [Some(1000), None] {
        let mut app = vault_app(clock);
        let before = gpg_count(&app);
        make_gpg_key(&mut app);
        assert_eq!(app.screen, Screen::SetTime, "{clock:?}");
        assert_eq!(app.now_local_text(), "Not known");
        let _ = app.frame();
        assert!(!app.offers(Action::SetTimeKeep), "{clock:?}");
        assert_eq!(gpg_count(&app), before, "{clock:?}");
    }
}

#[test]
fn setting_the_time_by_hand_corrects_a_clock_before_2020() {
    let mut app = vault_app(Some(1000));
    app.press(Action::Nav(Screen::Settings));
    crate::common::settings_section(&mut app, "Time");
    // UTC+02:00, eight steps of 15 minutes.
    for _ in 0..8 {
        app.press(Action::TzStep(15));
    }
    app.press(Action::OpenSetTime);
    assert_eq!(app.screen, Screen::SetTime);
    app.press(Action::SetTimeFocus(Focus::Date));
    type_text(&mut app, "2026-10-11");
    app.press(Action::SetTimeFocus(Focus::Time));
    type_text(&mut app, "14:02");
    app.press(Action::SetTimeSet);
    // Back on Settings, with the toast and Now reading the time set.
    assert_eq!(app.screen, Screen::Settings);
    assert_eq!(app.toast_text(), Some("Time set to 14:02"));
    assert!(
        app.now_local_text().starts_with("2026-10-11 14:02"),
        "{}",
        app.now_local_text()
    );
    // A GPG key made at once has creation time 2026-10-11 12:02:00 UTC:
    // 14:02 at UTC+02:00.
    app.press(Action::Nav(Screen::VaultContents));
    app.press(Action::Vault(V::Category(4)));
    make_gpg_key(&mut app);
    assert_eq!(gpg_created(&app), 1_791_720_120);
}

/// Set time's fields take an edit where the caret is: the arrow keys
/// move it, a press between two digits puts it there, and what is typed
/// goes in at it (`docs/TEXT-CURSOR.md` §3).
#[test]
fn set_times_fields_take_typing_where_the_caret_is() {
    let mut app = vault_app(Some(1000));
    app.press(Action::Nav(Screen::Settings));
    crate::common::settings_section(&mut app, "Time");
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeFocus(Focus::Date));
    type_text(&mut app, "202610-11");
    app.event(Event::Key(Key::Home));
    for _ in 0..4 {
        app.event(Event::Key(Key::Right));
    }
    app.event(Event::Key(Key::Char('-')));
    app.press(Action::SetTimeFocus(Focus::Time));
    type_text(&mut app, "142");
    app.event(Event::Key(Key::Left));
    app.event(Event::Key(Key::Char('0')));
    let _ = app.frame();
    let (x, y) = app
        .where_typed(Action::SetTimeFocus(Focus::Time), 2)
        .expect("the time drawn");
    for phase in [TouchPhase::Down, TouchPhase::Up] {
        app.event(Event::Touch { x, y, phase });
    }
    app.event(Event::Key(Key::Char(':')));
    app.press(Action::TzSet(0));
    app.press(Action::SetTimeSet);
    assert_eq!(app.toast_text(), Some("Time set to 14:02"));
    assert!(
        app.now_local_text().starts_with("2026-10-11 14:02"),
        "{}",
        app.now_local_text()
    );
}

#[test]
fn a_later_clock_reading_keeps_the_same_correction() {
    let mut app = vault_app(Some(1000));
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeFocus(Focus::Date));
    type_text(&mut app, "2026-10-11");
    app.press(Action::SetTimeFocus(Focus::Time));
    type_text(&mut app, "14:02");
    app.press(Action::TzSet(0));
    app.press(Action::SetTimeSet);
    make_gpg_key(&mut app);
    let first = gpg_created(&app);
    // The shell's clock, still below the floor, reads on: the same
    // offset carries the correction across it (`docs/TOTP.md` §1.2),
    // rather than the reading falling back to not known.
    app.storage(StorageEvent::Clock { unix_secs: 2000 });
    faraday_core::testkit::vault_close_form(&mut app);
    make_gpg_key(&mut app);
    let second = gpg_created(&app);
    assert_eq!(second as i64 - first as i64, 2000 - 1000);
}

#[test]
fn the_set_time_and_its_checked_mark_survive_a_lock() {
    let mut app = vault_app(Some(1000));
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeFocus(Focus::Date));
    type_text(&mut app, "2026-10-11");
    app.press(Action::SetTimeFocus(Focus::Time));
    type_text(&mut app, "14:02");
    app.press(Action::TzSet(0));
    app.press(Action::SetTimeSet);
    assert!(app.clock_checked());
    let mut kept = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept: k, .. } = c {
            kept = k;
        }
    }
    assert!(
        kept.iter().any(|(n, _)| n == "clock"),
        "the clock is kept across a lock"
    );
    let mut next = Faraday::new();
    next.event(display());
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept,
    });
    assert!(next.clock_checked());
    // The next process starts with its own raw reading: the same offset
    // corrects it.
    next.storage(StorageEvent::Clock { unix_secs: 1000 });
    assert!(next.now_local_text().starts_with("2026-10-11 14:02"));
}

#[test]
fn a_tz_line_in_steps_of_15_minutes_is_applied() {
    let good = booted(Some(
        b"faraday-settings 1\ntz=+05:30\nscale=125\n".as_slice(),
    ));
    assert_eq!(good.tz_min(), 330);
    assert_eq!(good.scale_pct(), 125, "the rest of the file still applies");
    for bad in [
        "faraday-settings 1\ntz=+05:20\n",
        "faraday-settings 1\ntz=+15:00\n",
        "faraday-settings 1\ntz=5:30\n",
        "faraday-settings 1\ntz=x\n",
    ] {
        let app = booted(Some(bad.as_bytes()));
        assert_eq!(app.tz_min(), 0, "{bad:?}");
        assert!(!app.tz_known(), "{bad:?}");
    }
}

#[test]
fn written_to_stick_shows_the_local_time() {
    let r = receipt_heading(true);
    assert_eq!(r, "Written to FIRST at 08:02");
}

#[test]
fn written_to_stick_shows_no_time_from_a_clock_not_checked() {
    assert_eq!(receipt_heading(false), "Written to FIRST");
}

/// The receipt of a write on a clock the person has checked or not.
fn receipt_heading(checked: bool) -> String {
    let mut app = Faraday::new();
    app.event(display());
    for _ in 0..16 {
        app.press(Action::TzStep(-15));
    }
    // 2026-10-11T12:02:00Z, which is 08:02 at UTC-04:00.
    app.storage(StorageEvent::Clock {
        unix_secs: 1_791_720_120,
    });
    app.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: faraday_core::files_from(vec![(
            faraday_text::Text::of("vault.kdbx"),
            testkit::test_vault().unwrap(),
        )]),
        kept: Vec::new(),
    });
    if checked {
        app.press(Action::OpenSetTime);
        app.press(Action::SetTimeKeep);
    }
    app.storage(StorageEvent::Sticks(vec![stick("FIRST", false, None)]));
    app.press(Action::Nav(Screen::Files));
    app.press(Action::VisitWrite);
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::Write { stick, name, .. } = c {
            app.storage(StorageEvent::Written {
                stick,
                wrote_as: name.clone(),
                name,
            });
        }
    }
    let r = app.receipt().expect("a receipt");
    r.heading(app.tz_min())
}

#[test]
fn set_is_not_offered_for_an_impossible_date_or_time() {
    let mut app = vault_app(Some(1_791_000_000));
    app.press(Action::OpenSetTime);
    app.press(Action::TzSet(0));
    app.press(Action::SetTimeFocus(Focus::Date));
    type_text(&mut app, "2026-02-30");
    app.press(Action::SetTimeFocus(Focus::Time));
    type_text(&mut app, "14:02");
    app.press(Action::SetTimeSet);
    assert_eq!(app.screen, Screen::SetTime, "no such date as 2026-02-30");
    // Clears the date and types a real one, but an impossible time.
    for _ in 0.."2026-02-30".len() {
        app.press(Action::SetTimeFocus(Focus::Date));
        app.event(Event::Key(Key::Backspace));
    }
    app.press(Action::SetTimeFocus(Focus::Date));
    type_text(&mut app, "2026-02-28");
    app.press(Action::SetTimeFocus(Focus::Time));
    for _ in 0.."14:02".len() {
        app.event(Event::Key(Key::Backspace));
    }
    type_text(&mut app, "24:00");
    app.press(Action::SetTimeSet);
    assert_eq!(app.screen, Screen::SetTime, "no such time as 24:00");
}

/// A time set with no clock reading counts on the app's own ticks, which
/// start again in the next process: it is not kept across a lock, and
/// the next process has the time not known and its checked mark gone.
#[test]
fn a_time_set_with_no_clock_reading_is_not_kept_across_a_lock() {
    let mut app = vault_app(None);
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeFocus(Focus::Date));
    type_text(&mut app, "2026-10-11");
    app.press(Action::SetTimeFocus(Focus::Time));
    type_text(&mut app, "14:02");
    app.press(Action::TzSet(0));
    app.press(Action::SetTimeSet);
    assert!(app.clock_now().is_some());
    let mut kept = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept: k, .. } = c {
            kept = k;
        }
    }
    let mut next = Faraday::new();
    next.event(display());
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept,
    });
    next.event(Event::Tick { now_ms: 5_000 });
    assert_eq!(next.clock_now(), None, "{}", next.now_local_text());
    assert!(!next.clock_checked());
}

/// A stick whose settings have no zone reads Not set on Set the time, and
/// Set is offered only once a zone is chosen.
#[test]
fn a_new_sticks_zone_reads_not_set_and_set_waits_for_a_zone() {
    let mut app = vault_app(Some(1_791_000_000));
    assert!(!app.tz_known());
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeFocus(Focus::Time));
    type_text(&mut app, "14:02");
    let _ = app.frame();
    let text = app.drawn_texts().join("\n");
    assert!(text.contains("Not set"), "{text}");
    assert!(!app.offers(Action::SetTimeSet));
    assert!(!app.offers(Action::SetTimeKeep));
    // Even pressed, nothing is set.
    app.press(Action::SetTimeSet);
    assert_eq!(app.screen, Screen::SetTime);
    app.press(Action::TzSet(-300));
    let _ = app.frame();
    assert!(app.offers(Action::SetTimeSet));
    assert!(app.offers(Action::SetTimeKeep));
}

/// The zone chosen from the list is kept in the settings, so the next
/// start of the app has it and does not ask again.
#[test]
fn a_zone_chosen_from_the_list_is_kept_across_a_restart() {
    let mut app = vault_app(Some(1_791_000_000));
    app.press(Action::OpenSetTime);
    app.press(Action::TzList(true));
    let _ = app.frame();
    app.press(Action::TzSet(-300));
    assert_eq!(app.tz_min(), -300);
    let mut kept = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept: k, .. } = c {
            kept = k;
        }
    }
    let mut next = Faraday::new();
    next.event(display());
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept,
    });
    assert!(next.tz_known());
    assert_eq!(next.tz_min(), -300);
    let fresh = booted(None);
    assert!(!fresh.tz_known());
}

/// The local time typed in the zone chosen is that moment in UTC: at
/// 09:03 in UTC+05:30 the TOTP code is the one RFC 6238 gives for
/// 2033-05-18 03:33 UTC (Appendix B, time 2000000000, SHA-1, 8 digits).
#[test]
fn local_time_typed_in_a_zone_gives_the_utc_moment_the_codes_use() {
    let mut app = vault_app(None);
    app.press(Action::OpenSetTime);
    app.press(Action::TzSet(330));
    app.press(Action::SetTimeFocus(Focus::Date));
    type_text(&mut app, "2033-05-18");
    app.press(Action::SetTimeFocus(Focus::Time));
    type_text(&mut app, "09:03");
    let _ = app.frame();
    let text = app.drawn_texts().join(" ");
    assert!(text.contains("UTC+05:30 is 2033-05-18 03:33 UTC"), "{text}");
    app.press(Action::SetTimeSet);
    let now = app.clock_now().expect("the time is set");
    let code = faraday_vault::totp::parse(
        "otpauth://totp/RFC:6238?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&digits=8",
    )
    .unwrap()
    .code(now);
    assert_eq!(&*code, "69279037");
}

/// A PC whose clock is hours off (it keeps local time and says it is
/// UTC) is put right by typing the phone's time in the zone chosen.
#[test]
fn a_pc_clock_hours_off_is_put_right_by_typing_the_phones_time() {
    // 14:02 at UTC+02:00 is 12:02 UTC, but the PC says 09:02 as if UTC.
    let mut app = vault_app(Some(1_791_720_120 - 3 * 3600));
    app.press(Action::OpenSetTime);
    app.press(Action::TzSet(120));
    let _ = app.frame();
    assert!(
        app.now_local_text().starts_with("2026-10-11 11:02"),
        "reads 3 hours behind the phone: {}",
        app.now_local_text()
    );
    app.press(Action::SetTimeFocus(Focus::Time));
    type_text(&mut app, "16:02");
    app.press(Action::SetTimeSet);
    assert!(
        app.now_local_text().starts_with("2026-10-11 16:02"),
        "{}",
        app.now_local_text()
    );
}

/// A Pi has no clock before it is set: Date starts empty, so Set waits
/// for a date as well as the time; a PC's date is already there.
#[test]
fn a_pi_starts_with_date_empty_and_a_pc_with_its_date() {
    let mut pi = vault_app(Some(1000));
    pi.press(Action::OpenSetTime);
    pi.press(Action::TzSet(0));
    pi.press(Action::SetTimeFocus(Focus::Time));
    type_text(&mut pi, "14:02");
    let _ = pi.frame();
    assert!(!pi.offers(Action::SetTimeSet), "no date yet");
    pi.press(Action::SetTimeFocus(Focus::Date));
    type_text(&mut pi, "2026-10-11");
    let _ = pi.frame();
    assert!(pi.offers(Action::SetTimeSet));

    let mut pc = vault_app(Some(1_791_720_120));
    pc.press(Action::OpenSetTime);
    pc.press(Action::TzSet(0));
    pc.press(Action::SetTimeFocus(Focus::Time));
    type_text(&mut pc, "14:02");
    let _ = pc.frame();
    assert!(pc.offers(Action::SetTimeSet), "the date is the clock's");
}
