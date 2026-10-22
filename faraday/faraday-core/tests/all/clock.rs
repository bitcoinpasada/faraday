//! The clock (`docs/TOTP.md` §1–§3): a reading below 2020 is not known,
//! as no reading at all is; a time set by hand is an offset against the
//! shell's own clock, which survives a lock but not a power-off; the
//! time zone is a fixed offset kept in the settings file; and every time
//! shown to a person is local.

use crate::common::type_text;
use faraday_core::gpg;
use faraday_core::set_time::Focus;
use faraday_core::stick::settings::FILE;
use faraday_core::testkit;
use faraday_core::vault::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StickInfo, StorageCommand, StorageEvent};
use faraday_shell_api::{App, BootState, DisplayInfo, EntropyBytes, Event, Key, SecureHardware};
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
        inbox: vec![("vault.ofv".into(), vault)],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    if let Some(secs) = clock {
        app.storage(StorageEvent::Clock { unix_secs: secs });
    }
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
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
    let v = &app.vaults.open[app.vaults.current];
    let r = app
        .vaults
        .store
        .vault_records(v.id)
        .records
        .iter()
        .rev()
        .find(|r| r.kind == kind::GPG)
        .expect("a GPG record");
    gpg::key_of(r).expect("a readable key").created
}

fn gpg_count(app: &Faraday) -> usize {
    app.vaults
        .store
        .vault_records(app.vaults.open[app.vaults.current].id)
        .of(kind::GPG)
        .count()
}

fn stick(id: &str, boot: bool, file: Option<&[u8]>) -> StickInfo {
    StickInfo {
        serial: None,
        id: id.into(),
        label: id.to_uppercase(),
        boot,
        files: file
            .map(|f| vec![(FILE.to_string(), f.len() as u64)])
            .unwrap_or_default(),
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
                bytes: bytes.to_vec(),
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

/// Settings' Time card says what reads the clock.
#[test]
fn the_time_card_says_what_the_time_is_for() {
    let mut app = vault_app(Some(1_791_000_000));
    app.press(Action::Nav(Screen::Settings));
    let texts = app.drawn_texts().join(" ");
    assert!(
        texts.contains("For TOTP codes, GPG and Secure Boot signatures"),
        "{texts}"
    );
}

#[test]
fn setting_the_time_by_hand_corrects_a_clock_before_2020() {
    let mut app = vault_app(Some(1000));
    app.press(Action::Nav(Screen::Settings));
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

#[test]
fn a_later_clock_reading_keeps_the_same_correction() {
    let mut app = vault_app(Some(1000));
    app.press(Action::OpenSetTime);
    app.press(Action::SetTimeFocus(Focus::Date));
    type_text(&mut app, "2026-10-11");
    app.press(Action::SetTimeFocus(Focus::Time));
    type_text(&mut app, "14:02");
    app.press(Action::SetTimeSet);
    make_gpg_key(&mut app);
    let first = gpg_created(&app);
    // The shell's clock, still below the floor, reads on: the same
    // offset carries the correction across it (`docs/TOTP.md` §1.2),
    // rather than the reading falling back to not known.
    app.storage(StorageEvent::Clock { unix_secs: 2000 });
    app.vaults.form = None;
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
    assert_eq!(good.tz_min, 330);
    assert_eq!(good.scale_pct, 125, "the rest of the file still applies");
    for bad in [
        "faraday-settings 1\ntz=+05:20\n",
        "faraday-settings 1\ntz=+15:00\n",
        "faraday-settings 1\ntz=5:30\n",
        "faraday-settings 1\ntz=x\n",
    ] {
        let app = booted(Some(bad.as_bytes()));
        assert_eq!(app.tz_min, 0, "{bad:?}");
    }
}

#[test]
fn written_to_stick_shows_the_local_time() {
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
        outbox: vec![("vault.ofv".into(), testkit::test_vault().unwrap())],
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Sticks(vec![stick("FIRST", false, None)]));
    app.press(Action::Nav(Screen::Visit));
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
    let r = app.receipt.as_ref().expect("a receipt");
    assert_eq!(r.heading(app.tz_min), "Written to FIRST at 08:02");
}

#[test]
fn set_is_not_offered_for_an_impossible_date_or_time() {
    let mut app = vault_app(Some(1_791_000_000));
    app.press(Action::OpenSetTime);
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
