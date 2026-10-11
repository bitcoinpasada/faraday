//! Settings that survive a restart (`docs/PLANNING.md` §6): what a
//! person chose is what the device has next time it starts, and a
//! damaged or foreign file is a device with its defaults rather than a
//! device that will not start.

mod common;

use common::{Harness, PANEL};
use opensigner_core::scan::CameraRotation;
use opensigner_core::{AssuranceTier, ids, strings};
use osk_bip::keys::Network;
use osk_psbt::{Nonce, Schnorr};
use osk_shell_api::{App, BootState, Command, DisplayInfo, Event, SecureHardware, TouchPhase};
use osk_ui::Id;
use osk_ui::components::Unit;

/// Taps `id` and returns the commands the tap produced, which is how the
/// shell sees a settings change.
fn tap_for_commands(h: &mut Harness, id: Id) -> Vec<Command> {
    let (x, y) = h.center(id);
    h.app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Down,
    });
    h.app.event(Event::Touch {
        x,
        y,
        phase: TouchPhase::Up,
    });
    h.drain()
}

/// The bytes of the last `StoreSettings` in `commands`.
fn stored(commands: &[Command]) -> Option<Vec<u8>> {
    commands.iter().rev().find_map(|c| match c {
        Command::StoreSettings { bytes } => Some(bytes.clone()),
        _ => None,
    })
}

/// Settings › `row` › the option at `i`, returning what the shell was
/// asked to keep.
fn choose(h: &mut Harness, row: Id, base: u32, i: usize) -> Option<Vec<u8>> {
    if h.app.screen() != opensigner_core::ScreenKind::Settings {
        h.open_settings();
    }
    h.tap(row);
    let commands = tap_for_commands(h, ids::at(base, i));
    h.tap(ids::BACK);
    stored(&commands)
}

/// A device given `bytes` at start, as a shell hands back what it kept.
fn restarted(bytes: Vec<u8>) -> Harness {
    restarted_on(PANEL, bytes)
}

/// The same, on a chosen display.
fn restarted_on(display: DisplayInfo, bytes: Vec<u8>) -> Harness {
    let mut h = Harness::new(display);
    h.send(Event::Settings { bytes });
    h
}

/// A phone: the platform hands the shell upright frames, so there is no
/// camera for a person to have mounted sideways.
const FIXED_CAMERA: DisplayInfo = DisplayInfo {
    width: PANEL.width,
    height: PANEL.height,
    dpi: PANEL.dpi,
    inset_bottom: PANEL.inset_bottom,
    inset_top: PANEL.inset_top,
    buttons: PANEL.buttons,
    camera_fixed: true,
    secure: SecureHardware::None,
    boot: BootState::Unknown,
    memory_mib: None,
};

/// A settings file from a device whose camera was mounted a quarter turn
/// round.
fn turned_file() -> Vec<u8> {
    b"opensigner-settings 1\n\
      network=mainnet\n\
      unit=sat\n\
      lock_after_ms=300000\n\
      wipe_after_ms=never\n\
      scramble_pin=off\n\
      camera_rotation=270\n"
        .to_vec()
}

/// §4.9: the rotation is for a camera someone mounted in a case they
/// built. A device whose frames are always upright does not offer it,
/// and a card carried over from one that did cannot turn its frames.
#[test]
fn a_device_whose_camera_is_fixed_has_no_camera_rotation_row() {
    let mut h = restarted_on(FIXED_CAMERA, turned_file());
    h.open_settings();
    let texts = h.app.texts();
    let label = strings::EN.settings_camera_rotation;
    assert!(
        !texts.iter().any(|t| t == label),
        "no Camera rotation row: {texts:?}"
    );
    assert_eq!(h.app.camera_rotation(), CameraRotation::Deg0);

    // And what it keeps says so, so the file stops carrying a turn that
    // nothing on this device can undo.
    let commands = tap_for_commands(&mut h, ids::SETTINGS_SCRAMBLE);
    let bytes = stored(&commands).expect("the shell is asked to keep the settings");
    let text = String::from_utf8(bytes).expect("settings are text");
    assert!(
        text.contains("camera_rotation=0\n"),
        "a fixed camera is written as no turn: {text}"
    );
}

#[test]
fn a_device_with_a_mounted_camera_keeps_the_row_and_the_turn() {
    let mut h = restarted_on(PANEL, turned_file());
    assert_eq!(h.app.camera_rotation(), CameraRotation::Deg270);
    h.open_settings();
    let texts = h.app.texts();
    for value in [
        strings::EN.settings_camera_rotation,
        strings::EN.settings_camera_rotation_270,
    ] {
        assert!(texts.iter().any(|t| t == value), "Settings shows {value}");
    }
}

#[test]
fn every_setting_a_person_chose_is_there_after_a_restart() {
    let mut h = Harness::new(PANEL);
    choose(
        &mut h,
        ids::SETTINGS_NETWORK_ROW,
        ids::SETTINGS_NET_BASE,
        Network::ALL
            .iter()
            .position(|n| *n == Network::Regtest)
            .unwrap(),
    );
    choose(&mut h, ids::SETTINGS_UNIT_ROW, ids::SETTINGS_UNIT_BASE, 1);
    choose(
        &mut h,
        ids::SETTINGS_CAMERA_ROTATION_ROW,
        ids::SETTINGS_CAMERA_ROTATION_BASE,
        1,
    );
    choose(
        &mut h,
        ids::SETTINGS_LOCK_AFTER_ROW,
        ids::SETTINGS_LOCK_AFTER_BASE,
        3,
    );
    choose(
        &mut h,
        ids::SETTINGS_WIPE_AFTER_ROW,
        ids::SETTINGS_WIPE_AFTER_BASE,
        2,
    );
    choose(&mut h, ids::SETTINGS_NONCE_ROW, ids::SETTINGS_NONCE_BASE, 1);
    choose(
        &mut h,
        ids::SETTINGS_SCHNORR_ROW,
        ids::SETTINGS_SCHNORR_BASE,
        1,
    );
    // The shuffle last, so the bytes the shell keeps carry all eight.
    let commands = tap_for_commands(&mut h, ids::SETTINGS_SCRAMBLE);
    let bytes = stored(&commands).expect("the shell is asked to keep the settings");

    let mut next = restarted(bytes);
    assert_eq!(next.app.network(), Network::Regtest);
    assert_eq!(next.app.unit(), Unit::Btc);
    assert_eq!(next.app.camera_rotation(), CameraRotation::Deg90);
    assert_eq!(next.app.lock_after_ms(), 900_000);
    assert_eq!(next.app.wipe_after_ms(), Some(1_800_000));
    assert!(next.app.scramble_pin());
    assert_eq!(next.app.nonce(), Nonce::First);
    assert_eq!(next.app.schnorr(), Schnorr::Fresh);
    // And the Settings screen states them.
    next.open_settings();
    let texts = next.app.texts();
    for value in [
        "regtest",
        "BTC",
        "90°",
        "15 min",
        "30 min",
        strings::EN.settings_nonce_first,
        strings::EN.settings_schnorr_fresh,
    ] {
        assert!(
            texts.iter().any(|t| t == value),
            "Settings shows {value}: {texts:?}"
        );
    }
}

#[test]
fn a_file_written_by_hand_is_what_the_device_starts_with() {
    let h = restarted(
        b"opensigner-settings 1\n\
          network=signet\n\
          unit=btc\n\
          lock_after_ms=300000\n\
          wipe_after_ms=never\n\
          scramble_pin=on\n\
          camera_rotation=180\n\
          nonce=first\n\
          schnorr=fresh\n"
            .to_vec(),
    );
    assert_eq!(h.app.network(), Network::Signet);
    assert_eq!(h.app.unit(), Unit::Btc);
    assert_eq!(h.app.lock_after_ms(), 300_000);
    assert_eq!(h.app.wipe_after_ms(), None);
    assert!(h.app.scramble_pin());
    assert_eq!(h.app.camera_rotation(), CameraRotation::Deg180);
    assert_eq!(h.app.nonce(), Nonce::First);
    assert_eq!(h.app.schnorr(), Schnorr::Fresh);
}

/// A device that was never told, and a file from a version that had no
/// such key, both sign with Low R and with no auxiliary randomness, and
/// the Settings rows say so.
#[test]
fn the_nonce_is_low_r_until_someone_chooses_otherwise() {
    for mut h in [
        Harness::new(PANEL),
        restarted(
            b"opensigner-settings 1\n\
              unit=btc\n"
                .to_vec(),
        ),
    ] {
        assert_eq!(h.app.nonce(), Nonce::LowR);
        assert_eq!(h.app.schnorr(), Schnorr::Deterministic);
        h.open_settings();
        let texts = h.app.texts();
        assert!(
            texts.iter().any(|t| t == strings::EN.settings_nonce)
                && texts.iter().any(|t| t == strings::EN.settings_nonce_low_r),
            "Settings states the nonce: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t == strings::EN.settings_schnorr)
                && texts
                    .iter()
                    .any(|t| t == strings::EN.settings_schnorr_deterministic),
            "Settings states the Schnorr setting: {texts:?}"
        );
    }
}

#[test]
fn nothing_is_kept_until_the_user_changes_something() {
    let mut h = Harness::new(PANEL);
    let start = h.drain();
    assert!(
        stored(&start).is_none(),
        "starting up is not a reason to write"
    );
    h.app.event(Event::Settings {
        bytes: b"opensigner-settings 1\nunit=btc\n".to_vec(),
    });
    let loaded = h.drain();
    assert!(
        stored(&loaded).is_none(),
        "applying what was kept is not a reason to write it again"
    );
    // Nor is time passing, or walking around the app.
    h.tick(60_000);
    h.open_settings();
    h.tap(ids::SETTINGS_ABOUT_ROW);
    h.tap(ids::BACK);
    let idle = h.drain();
    assert!(stored(&idle).is_none());
}

#[test]
fn a_damaged_or_foreign_file_leaves_the_defaults() {
    let defaults = Harness::new(PANEL);
    let (network, unit, lock, wipe, rotation) = (
        defaults.app.network(),
        defaults.app.unit(),
        defaults.app.lock_after_ms(),
        defaults.app.wipe_after_ms(),
        defaults.app.camera_rotation(),
    );

    for bytes in [
        b"\x00\xff not a settings file at all".to_vec(),
        b"network=signet\nunit=btc\n".to_vec(),
        b"opensigner-settings 9\nnetwork=signet\n".to_vec(),
        Vec::new(),
    ] {
        let h = restarted(bytes);
        assert_eq!(h.app.network(), network);
        assert_eq!(h.app.unit(), unit);
        assert_eq!(h.app.lock_after_ms(), lock);
        assert_eq!(h.app.wipe_after_ms(), wipe);
        assert_eq!(h.app.camera_rotation(), rotation);
        assert!(!h.app.scramble_pin());
    }

    // One value that makes no sense keeps that one default; the rest of
    // the file still applies, unknown keys and all.
    let h = restarted(
        b"opensigner-settings 1\n\
          network=signet\n\
          unit=furlongs\n\
          camera_rotation=45\n\
          favourite_colour=green\n\
          scramble_pin=on\n"
            .to_vec(),
    );
    assert_eq!(h.app.network(), Network::Signet);
    assert_eq!(h.app.unit(), unit, "an unreadable unit stays the default");
    assert_eq!(h.app.camera_rotation(), rotation);
    assert!(h.app.scramble_pin());
}

/// The settings file is writable by anything that can reach the card or
/// the config directory, so a timer it names that Settings does not offer
/// is not a device that locks in a millisecond or never wipes: it is a
/// device with that timer at its default.
#[test]
fn a_timer_no_one_could_have_chosen_leaves_the_default() {
    let defaults = Harness::new(PANEL);
    let (lock, wipe) = (defaults.app.lock_after_ms(), defaults.app.wipe_after_ms());

    let h = restarted(
        b"opensigner-settings 1\n\
          lock_after_ms=1\n\
          wipe_after_ms=18446744073709551615\n"
            .to_vec(),
    );
    assert_eq!(h.app.lock_after_ms(), lock);
    assert_eq!(h.app.wipe_after_ms(), wipe);

    // A value the Settings screen offers still applies.
    let h = restarted(
        b"opensigner-settings 1\n\
          lock_after_ms=30000\n\
          wipe_after_ms=3600000\n"
            .to_vec(),
    );
    assert_eq!(h.app.lock_after_ms(), 30_000);
    assert_eq!(h.app.wipe_after_ms(), Some(3_600_000));
}

#[test]
fn a_loaded_wipe_shorter_than_the_loaded_lock_comes_out_consistent() {
    let h =
        restarted(b"opensigner-settings 1\nlock_after_ms=900000\nwipe_after_ms=300000\n".to_vec());
    assert_eq!(h.app.wipe_after_ms(), Some(300_000));
    assert_eq!(
        h.app.lock_after_ms(),
        300_000,
        "the lock comes down to the wipe, as a tap on the Choice would"
    );
}

#[test]
fn a_tier_d_device_never_asks_the_shell_to_keep_anything() {
    let mut h = Harness::with_tier(PANEL, AssuranceTier::D, true);
    assert!(
        choose(&mut h, ids::SETTINGS_UNIT_ROW, ids::SETTINGS_UNIT_BASE, 1).is_none(),
        "Tier D persists nothing"
    );
    assert_eq!(h.app.unit(), Unit::Btc, "the setting still applies here");
    let commands = tap_for_commands(&mut h, ids::SETTINGS_SCRAMBLE);
    assert!(stored(&commands).is_none());
}

/// About states what the platform said about the device's boot: the
/// device vouches for the system it is running, or it does not say. A
/// device that reports an unverified boot never reaches About, because
/// the app refuses to run on it (`tests/keep.rs`).
#[test]
fn about_states_the_verified_boot_the_shell_reported() {
    let s = &strings::EN;
    for (boot, value) in [
        (BootState::Unknown, s.value_unknown),
        (BootState::Verified, s.value_yes),
    ] {
        let mut h = Harness::new(DisplayInfo { boot, ..PANEL });
        h.open_settings();
        h.tap(ids::SETTINGS_ABOUT_ROW);
        let texts = h.app.texts();
        let label = texts
            .iter()
            .position(|t| t == s.settings_boot)
            .expect("the verified-boot row");
        assert_eq!(
            texts.get(label + 1).map(String::as_str),
            Some(value),
            "the row says something else for {boot:?}"
        );
    }
}

// ----- The Argon2id memory an encrypted export is made at
// (`docs/PLANNING.md` §16.112) -----

/// The same display with `memory_mib` reported.
fn with_memory(mib: Option<u32>) -> DisplayInfo {
    DisplayInfo {
        memory_mib: mib,
        ..PANEL
    }
}

/// The recommendation follows the memory the shell reports: 256 MiB
/// where the device has at least a gigabyte, and the cost that opens
/// anywhere below that. A shell that says nothing gets the latter.
#[test]
fn the_recommended_backup_memory_follows_the_device() {
    use opensigner_core::BACKUP_MEMORY;
    for (reported, want) in [
        (None, BACKUP_MEMORY[0]),
        (Some(512), BACKUP_MEMORY[0]),
        (Some(1024), BACKUP_MEMORY[1]),
        (Some(8192), BACKUP_MEMORY[1]),
    ] {
        let mut h = Harness::new(with_memory(reported));
        h.open_settings();
        let want_row = strings::fill1(strings::EN.backup_memory_value, &format!("{}", want / 1024));
        assert!(
            h.app.texts().contains(&want_row),
            "{reported:?} MiB should recommend {want_row}: {:?}",
            h.app.texts()
        );
    }
}

/// The row opens a Choice of three costs, each stating its trade-off;
/// what is chosen is what the next backup's header carries and what the
/// device comes back to.
#[test]
fn the_backup_memory_setting_changes_the_header_and_persists() {
    use opensigner_core::BACKUP_MEMORY;

    // The header of the file this writes is asserted below, so this
    // device stretches at the cost a device stretches at.
    let mut h = Harness::at_device_cost(with_memory(Some(8192)));
    h.open_settings();
    h.tap(ids::SETTINGS_BACKUP_MEMORY_ROW);
    for line in [
        strings::EN.settings_backup_memory_anywhere,
        strings::EN.settings_backup_memory_recommended,
        strings::EN.settings_backup_memory_slowest,
    ] {
        assert!(
            h.app.texts().contains(&String::from(line)),
            "every row states its trade-off: {:?}",
            h.app.texts()
        );
    }
    h.tap(ids::BACK);

    // The cheapest cost, chosen, is what a backup made afterwards says
    // in its header.
    let kept = choose(
        &mut h,
        ids::SETTINGS_BACKUP_MEMORY_ROW,
        ids::SETTINGS_BACKUP_MEMORY_BASE,
        0,
    )
    .expect("the shell was asked to keep it");
    h.go_home();
    h.start_load(&common::ABANDON);
    h.finish_load(None);
    h.open_key(0);
    h.tap(ids::DETAIL_BACKUP);
    h.tap(ids::BACKUP_ENCRYPTED);
    h.tap(ids::FORM_CONTINUE);
    h.type_text("a long enough one");
    h.key(osk_shell_api::Key::Enter);
    h.type_text("a long enough one");
    h.key(osk_shell_api::Key::Enter);
    h.seen.clear();
    h.tap(ids::BACKUP_SAVE);
    let bytes = h
        .seen
        .iter()
        .find_map(|c| match c {
            Command::WriteFile { bytes, .. } => Some(bytes.clone()),
            _ => None,
        })
        .expect("the backup was written");
    assert_eq!(
        osk_backup::oskb::cost_of(&bytes).map(|c| c.memory_kib),
        Some(BACKUP_MEMORY[0]),
        "the file was made at the cost that was chosen"
    );

    // And the device comes back to it, whatever it would recommend.
    let back = restarted_on(with_memory(Some(8192)), kept);
    let mut back = back;
    back.open_settings();
    let cheap = strings::fill1(
        strings::EN.backup_memory_value,
        &format!("{}", BACKUP_MEMORY[0] / 1024),
    );
    assert!(
        back.app.texts().contains(&cheap),
        "the chosen cost survived the restart: {:?}",
        back.app.texts()
    );
}
