//! TOTP codes (`docs/TOTP.md` §5–§8): what a person sees on the way into
//! the codes screen, on it, and on an entry's detail, driven through
//! `Action`s and `StorageEvent`s as `clock.rs`, `import.rs` and `gpg.rs`
//! do.

use faraday_core::catalog::TILES;
use faraday_core::codes::{CodeKind, codes_rows};
use faraday_core::set_time::Focus;
use faraday_core::testkit;
use faraday_core::vaults::VaultAction as V;
use faraday_core::{Action, Faraday, Screen, StorageCommand, StorageEvent};
use faraday_vault::records::{field, kind};
use faraday_vault::{Contents, Cost, Record};
use osk_shell_api::{
    App, BootState, Command, DisplayInfo, EntropyBytes, Event, Key, SecureHardware,
};

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

fn type_text(app: &mut Faraday, text: &str) {
    for c in text.chars() {
        app.event(Event::Key(Key::Char(c)));
    }
}

fn settle(app: &mut Faraday, until: impl Fn(&Faraday) -> bool) {
    for t in 1..60u64 {
        if until(app) {
            break;
        }
        let _ = app.frame();
        app.event(Event::Tick { now_ms: t * 1000 });
    }
}

fn tile(name: &str) -> u8 {
    TILES.iter().position(|t| t.name == name).unwrap() as u8
}

/// A fresh app with the test vault's file in the Inbox, not unlocked,
/// the clock as given and `online` as given.
fn fresh_app(clock: Option<u64>, online: bool) -> Faraday {
    let mut app = Faraday::new();
    app.online = online;
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
    app
}

/// Presses the Tools tile, types the test vault's passphrase and
/// unlocks: the way in `docs/TOTP.md` §5.2 describes with no vault open.
fn open_codes_from_tools(app: &mut Faraday) {
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::Catalog(tile("TOTP codes")));
    assert_eq!(app.screen, Screen::Unlock, "no vault open leads to Unlock");
    type_text(app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    settle(app, |a| !a.vaults.open.is_empty());
}

/// A minimal one-slot vault holding one entry with this field 6 value.
fn entry_vault(passphrase: &str, label: &str, title: &str, totp: &[u8], seed: u8) -> Vec<u8> {
    let main = Contents {
        records: vec![
            Record::new(kind::SLOT_LABEL).with(field::LABEL, label.as_bytes()),
            Record::new(kind::ENTRY)
                .with(field::TITLE, title.as_bytes())
                .with(field::TOTP, totp),
        ],
    };
    let cost = Cost {
        memory_kib: 64 * 1024,
        passes: 3,
        lanes: 1,
    };
    faraday_vault::create(
        faraday_vault::SLOT_SIZES[0],
        cost,
        &[passphrase.as_bytes()],
        &[main],
        &[seed; 32],
    )
    .unwrap()
}

#[test]
fn from_tools_with_no_vault_open_codes_leads_to_unlock_then_the_codes() {
    // Online so the clock needs no Set the time stop on the way.
    let mut app = fresh_app(Some(2_000_000_000), true);
    open_codes_from_tools(&mut app);
    assert_eq!(app.screen, Screen::Codes);
}

#[test]
fn a_pi_like_clock_opens_set_the_time_first_then_shows_the_code() {
    // Below the floor, as the Pi reports at boot: not known.
    let mut app = fresh_app(Some(1000), false);
    open_codes_from_tools(&mut app);
    assert_eq!(
        app.screen,
        Screen::SetTime,
        "the clock is not known, so Set the time comes first"
    );
    assert_eq!(app.set_time.back_to, Screen::Codes);
    assert!(
        app.clock_now().is_none(),
        "no Keep this time without a clock"
    );
    app.press(Action::SetTimeFocus(Focus::Date));
    type_text(&mut app, "2026-10-11");
    app.press(Action::SetTimeFocus(Focus::Time));
    type_text(&mut app, "14:02");
    app.press(Action::SetTimeSet);
    assert_eq!(app.screen, Screen::Codes, "Set goes back to the codes");
    let now = app.clock_now().expect("the clock is now known");
    let rows = codes_rows(&app, now);
    assert_eq!(rows.len(), 1);
    let expected = faraday_vault::totp::parse("otpauth://totp/Example:you?secret=JBSWY3DPEHPK3PXP")
        .unwrap()
        .code(now);
    match &rows[0].kind {
        CodeKind::Code { grouped, .. } => {
            assert_eq!(*grouped, faraday_vault::totp::grouped(&expected));
        }
        other => panic!("expected a code, got {other:?}"),
    }
}

#[test]
fn a_stick_like_known_clock_offers_keep_this_time_and_stays_checked() {
    // Known, but from the shell's own clock: Set the time still asks,
    // with Keep this time available.
    let mut app = fresh_app(Some(2_000_000_000), false);
    open_codes_from_tools(&mut app);
    assert_eq!(app.screen, Screen::SetTime);
    assert!(
        app.clock_now().is_some(),
        "Keep this time needs a known clock"
    );
    app.press(Action::SetTimeKeep);
    assert_eq!(app.screen, Screen::Codes);
    assert!(app.clock_checked());
    // The rest of this power-on: opening the codes again asks nothing.
    app.press(Action::Nav(Screen::Catalog));
    app.press(Action::OpenCodes);
    assert_eq!(app.screen, Screen::Codes);
}

#[test]
fn the_checked_clock_survives_a_lock() {
    let mut app = fresh_app(Some(2_000_000_000), false);
    open_codes_from_tools(&mut app);
    app.press(Action::SetTimeKeep);
    assert!(app.clock_checked());
    let mut kept = Vec::new();
    while let Some(c) = app.poll_storage() {
        if let StorageCommand::SaveBoxes { kept: k, .. } = c {
            kept = k;
        }
    }
    assert!(kept.iter().any(|(n, _)| n == "clock"));
    let mut next = Faraday::new();
    next.online = false;
    next.event(display());
    next.storage(StorageEvent::Restored {
        inbox: Vec::new(),
        outbox: Vec::new(),
        kept,
    });
    next.storage(StorageEvent::Clock {
        unix_secs: 2_000_000_000,
    });
    assert!(
        next.clock_checked(),
        "the checked mark comes back across a lock"
    );
}

#[test]
fn the_desktop_online_clock_opens_the_codes_directly() {
    let mut app = fresh_app(Some(2_000_000_000), true);
    open_codes_from_tools(&mut app);
    assert_eq!(app.screen, Screen::Codes);
}

#[test]
fn two_open_vaults_each_show_their_rows_and_a_hotp_entry_shows_no_code() {
    let mut app = Faraday::new();
    app.online = true;
    app.event(display());
    for i in 0..6u8 {
        app.event(Event::Entropy(EntropyBytes::new([0x60 + i; 32])));
    }
    let first = testkit::test_vault().unwrap();
    let second = entry_vault(
        "second",
        "Cold storage",
        "Exchange",
        b"otpauth://hotp/Exchange?secret=JBSWY3DPEHPK3PXP",
        0x11,
    );
    app.storage(StorageEvent::Restored {
        inbox: vec![("vault.ofv".into(), first), ("second.ofv".into(), second)],
        outbox: Vec::new(),
        kept: Vec::new(),
    });
    app.storage(StorageEvent::Clock {
        unix_secs: 2_000_000_000,
    });
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(0)));
    type_text(&mut app, testkit::VAULT_PASSPHRASES[0]);
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| a.vaults.open.len() == 1);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(1)));
    type_text(&mut app, "second");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| a.vaults.open.len() == 2);
    assert_eq!(app.vaults.open.len(), 2);
    app.press(Action::OpenCodes);
    assert_eq!(app.screen, Screen::Codes);
    let now = app.clock_now().unwrap();
    let rows = codes_rows(&app, now);
    assert_eq!(rows.len(), 2, "one row per vault's TOTP entry");
    assert!(
        rows.iter().all(|r| !r.vault_name.is_empty()),
        "each row names its vault with more than one open"
    );
    let hotp = rows
        .iter()
        .find(|r| r.title == "Exchange")
        .expect("the HOTP entry is listed");
    assert!(matches!(hotp.kind, CodeKind::Hotp));
}

#[test]
fn the_code_shown_changes_when_the_tick_crosses_the_period() {
    let mut app = fresh_app(Some(2_000_000_000), true);
    open_codes_from_tools(&mut app);
    let now = app.clock_now().unwrap();
    let before = codes_rows(&app, now);
    let code_before = match &before[0].kind {
        CodeKind::Code { grouped, .. } => grouped.clone(),
        _ => panic!("expected a code"),
    };
    let later = now + 30;
    let after = codes_rows(&app, later);
    let code_after = match &after[0].kind {
        CodeKind::Code { grouped, .. } => grouped.clone(),
        _ => panic!("expected a code"),
    };
    assert_ne!(
        code_before, code_after,
        "a full period on, the code changed"
    );
}

#[test]
fn the_codes_on_screen_move_on_each_second_with_no_input() {
    // The stick: Keep this time, then the codes, with no sheet over them.
    let mut app = fresh_app(Some(2_000_000_000), false);
    open_codes_from_tools(&mut app);
    app.press(Action::SetTimeKeep);
    assert_eq!(app.screen, Screen::Codes);
    let drawn = |app: &mut Faraday, now_ms: u64| {
        app.event(Event::Tick { now_ms });
        let asked = std::iter::from_fn(|| app.poll_command()).any(|c| c == Command::Draw);
        (asked, app.frame().rgba.to_vec())
    };
    let (_, first) = drawn(&mut app, 100_000);
    let (asked, next) = drawn(&mut app, 101_000);
    assert!(asked, "a second later the screen asks to be drawn");
    assert_ne!(first, next, "the seconds left moved on");
}

#[test]
fn the_entry_form_saves_a_bare_key_and_refuses_garbage() {
    let mut app = fresh_app(Some(2_000_000_000), true);
    open_codes_from_tools(&mut app);
    app.press(Action::CodesAdd);
    app.press(Action::Vault(V::FocusField(0)));
    type_text(&mut app, "New site");
    app.press(Action::Vault(V::FocusField(5)));
    type_text(&mut app, "JBSW Y3DP EHPK 3PXP");
    app.press(Action::Vault(V::FormSave));
    assert_eq!(
        app.screen,
        Screen::Codes,
        "saving from the codes screen comes back to it"
    );
    let now = app.clock_now().unwrap();
    let rows = codes_rows(&app, now);
    let new = rows
        .iter()
        .find(|r| r.title == "New site")
        .expect("the new entry is in the list");
    assert!(matches!(new.kind, CodeKind::Code { .. }));

    // Garbage is refused, the form kept.
    app.press(Action::CodesAdd);
    app.press(Action::Vault(V::FocusField(0)));
    type_text(&mut app, "Bad");
    app.press(Action::Vault(V::FocusField(5)));
    // A word, though every letter is Base32: too short for a key.
    type_text(&mut app, "hello");
    app.press(Action::Vault(V::FormSave));
    assert!(app.vaults.form.is_some(), "the form is kept on a refusal");
    assert_eq!(app.toast_text(), Some("Not a TOTP key"));
}

#[test]
fn scanning_a_setup_code_from_the_codes_screen_returns_to_it() {
    let mut app = fresh_app(Some(2_000_000_000), true);
    open_codes_from_tools(&mut app);
    app.press(Action::Vault(V::ScanEntry));
    app.event(Event::Scanned {
        bytes: b"otpauth://totp/Scanned:me?secret=JBSWY3DPEHPK3PXP".to_vec(),
    });
    assert_eq!(
        app.screen,
        Screen::Codes,
        "scanned from the codes screen, back on it"
    );
    app.press(Action::Vault(V::FormSave));
    let now = app.clock_now().unwrap();
    let rows = codes_rows(&app, now);
    assert!(rows.iter().any(|r| r.title == "Scanned"));
}
