//! The settings that survive a restart, as bytes (`docs/PLANNING.md`
//! §6, §5.3, §10.1).
//!
//! Ten non-secret choices are kept: the network, the unit, the
//! auto-lock and auto-wipe timers, the PIN-pad shuffle, the camera
//! rotation, the ECDSA nonce, the Schnorr auxiliary randomness, the
//! Argon2id memory an encrypted export is made at, and whether the
//! first run is over.
//! Nothing else is: no key, nothing about
//! a key, no session key, no self-test result, no screen and no stack.
//! A shell keeps the bytes
//! [`write`] produces and hands them back once at start; [`read`] turns
//! them back into a [`Settings`].
//!
//! The format is a few lines of text, so that no dependency is needed to
//! produce or to read one, and so that a person looking at the file on a
//! card can see what their device will do:
//!
//! ```text
//! opensigner-settings 1
//! network=mainnet
//! unit=sat
//! lock_after_ms=300000
//! wipe_after_ms=never
//! scramble_pin=off
//! camera_rotation=90
//! nonce=low_r
//! schnorr=deterministic
//! first_run_done=on
//! backup_memory_kib=262144
//! ```
//!
//! The memory line is absent until the person chooses one, because
//! until then the device's own recommendation stands and a file that
//! recorded the recommendation would freeze it.
//!
//! Reading is forgiving in one direction only. A missing or unknown
//! header line leaves every default; an unknown key is ignored; a value
//! that does not parse leaves that one setting at its default and the
//! rest still apply. Nothing here fails: a damaged file is a device with
//! its defaults, never a device that will not start.
//!
//! A value is read only if it is one a person could have chosen on the
//! device. The two timers are the ones that matter: the file is
//! writable by anything that can reach the card or the config directory,
//! so a value outside [`crate::session::LOCK_OPTIONS`] or
//! [`crate::session::WIPE_OPTIONS`] — an hour-long lock, a wipe that
//! never comes — leaves that timer at its default rather than becoming
//! a device that locks later than any setting offers.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use osk_bip::keys::Network;
use osk_psbt::{Nonce, Schnorr};
use osk_ui::components::Unit;

use crate::scan::CameraRotation;
use crate::session::{DEFAULT_LOCK_MS, DEFAULT_WIPE_MS, LOCK_OPTIONS, WIPE_OPTIONS};

/// The first line of a settings file: the format's name and its version.
const HEADER: &str = "opensigner-settings 1";

/// The auto-wipe value for "no auto-wipe".
const NEVER: &str = "never";

/// The toggle values.
const ON: &str = "on";
/// See [`ON`].
const OFF: &str = "off";

/// What a restart restores. The defaults are the ones a device that has
/// never been given a settings file starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// The chain every key is derived on.
    pub network: Network,
    /// The unit every amount is shown in.
    pub unit: Unit,
    /// The auto-lock timer, in milliseconds.
    pub lock_after_ms: u64,
    /// The auto-wipe timer in milliseconds, or `None` for never.
    pub wipe_after_ms: Option<u64>,
    /// Whether the PIN pad is shuffled.
    pub scramble_pin: bool,
    /// How far a camera frame is turned before it is read.
    pub camera_rotation: CameraRotation,
    /// Which RFC 6979 nonce every ECDSA signature uses.
    pub nonce: Nonce,
    /// Where a Schnorr signature's auxiliary randomness comes from.
    pub schnorr: Schnorr,
    /// Whether the Start here document has been left once. Until it is
    /// true, it is the screen a device with nothing loaded opens on.
    pub first_run_done: bool,
    /// The Argon2id memory an encrypted export is made at, in KiB, once
    /// the person has chosen one (`docs/PLANNING.md` §16.112). `None`
    /// leaves the device's own recommendation standing.
    pub backup_memory_kib: Option<u32>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            network: Network::Mainnet,
            unit: Unit::default(),
            lock_after_ms: DEFAULT_LOCK_MS,
            wipe_after_ms: Some(DEFAULT_WIPE_MS),
            scramble_pin: false,
            camera_rotation: CameraRotation::default(),
            nonce: Nonce::default(),
            schnorr: Schnorr::default(),
            first_run_done: false,
            backup_memory_kib: None,
        }
    }
}

/// The bytes a shell keeps for `settings`.
pub fn write(settings: &Settings) -> Vec<u8> {
    let wipe = match settings.wipe_after_ms {
        Some(ms) => format!("{ms}"),
        None => String::from(NEVER),
    };
    let text = format!(
        "{HEADER}\nnetwork={}\nunit={}\nlock_after_ms={}\nwipe_after_ms={wipe}\nscramble_pin={}\ncamera_rotation={}\nnonce={}\nschnorr={}\nfirst_run_done={}\n",
        settings.network.name(),
        unit_name(settings.unit),
        settings.lock_after_ms,
        if settings.scramble_pin { ON } else { OFF },
        rotation_degrees(settings.camera_rotation),
        nonce_name(settings.nonce),
        schnorr_name(settings.schnorr),
        if settings.first_run_done { ON } else { OFF },
    );
    let text = match settings.backup_memory_kib {
        Some(kib) => format!("{text}backup_memory_kib={kib}\n"),
        None => text,
    };
    text.into_bytes()
}

/// The settings `bytes` hold, with a default wherever they hold nothing
/// this version understands.
pub fn read(bytes: &[u8]) -> Settings {
    let mut settings = Settings::default();
    let Ok(text) = core::str::from_utf8(bytes) else {
        return settings;
    };
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some(HEADER) {
        return settings;
    }
    for line in lines {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim_end();
        match key.trim() {
            "network" => {
                if let Some(n) = Network::from_name(value) {
                    settings.network = n;
                }
            }
            "unit" => {
                if let Some(u) = unit_of(value) {
                    settings.unit = u;
                }
            }
            "lock_after_ms" => {
                if let Ok(ms) = value.parse()
                    && LOCK_OPTIONS.contains(&ms)
                {
                    settings.lock_after_ms = ms;
                }
            }
            "wipe_after_ms" => {
                if value == NEVER {
                    settings.wipe_after_ms = None;
                } else if let Ok(ms) = value.parse()
                    && WIPE_OPTIONS.contains(&Some(ms))
                {
                    settings.wipe_after_ms = Some(ms);
                }
            }
            "scramble_pin" => match value {
                ON => settings.scramble_pin = true,
                OFF => settings.scramble_pin = false,
                _ => {}
            },
            "camera_rotation" => {
                if let Some(r) = rotation_of(value) {
                    settings.camera_rotation = r;
                }
            }
            "nonce" => {
                if let Some(n) = nonce_of(value) {
                    settings.nonce = n;
                }
            }
            "schnorr" => {
                if let Some(c) = schnorr_of(value) {
                    settings.schnorr = c;
                }
            }
            // Only a cost the device offers: the file is writable by
            // anything that can reach the card, and a cost no screen
            // offers would be a device that cannot open its own files.
            "backup_memory_kib" => {
                if let Ok(kib) = value.parse()
                    && crate::BACKUP_MEMORY.contains(&kib)
                {
                    settings.backup_memory_kib = Some(kib);
                }
            }
            "first_run_done" => match value {
                ON => settings.first_run_done = true,
                OFF => settings.first_run_done = false,
                _ => {}
            },
            _ => {}
        }
    }
    settings
}

/// The unit's name in a file: `sat` or `btc`.
fn unit_name(unit: Unit) -> &'static str {
    match unit {
        Unit::Sat => "sat",
        Unit::Btc => "btc",
    }
}

/// The unit a name written by [`unit_name`] means.
fn unit_of(name: &str) -> Option<Unit> {
    match name {
        "sat" => Some(Unit::Sat),
        "btc" => Some(Unit::Btc),
        _ => None,
    }
}

/// The rotation as the degrees a file names it by.
fn rotation_degrees(rotation: CameraRotation) -> u16 {
    match rotation {
        CameraRotation::Deg0 => 0,
        CameraRotation::Deg90 => 90,
        CameraRotation::Deg180 => 180,
        CameraRotation::Deg270 => 270,
    }
}

/// The rotation a number of degrees means.
fn rotation_of(degrees: &str) -> Option<CameraRotation> {
    match degrees {
        "0" => Some(CameraRotation::Deg0),
        "90" => Some(CameraRotation::Deg90),
        "180" => Some(CameraRotation::Deg180),
        "270" => Some(CameraRotation::Deg270),
        _ => None,
    }
}

/// The nonce's name in a file: `low_r` or `first`.
fn nonce_name(nonce: Nonce) -> &'static str {
    match nonce {
        Nonce::LowR => "low_r",
        Nonce::First => "first",
    }
}

/// The nonce a name written by [`nonce_name`] means.
fn nonce_of(name: &str) -> Option<Nonce> {
    match name {
        "low_r" => Some(Nonce::LowR),
        "first" => Some(Nonce::First),
        _ => None,
    }
}

/// The Schnorr choice's name in a file: `deterministic` or `fresh`.
fn schnorr_name(schnorr: Schnorr) -> &'static str {
    match schnorr {
        Schnorr::Deterministic => "deterministic",
        Schnorr::Fresh => "fresh",
    }
}

/// The Schnorr choice a name written by [`schnorr_name`] means.
fn schnorr_of(name: &str) -> Option<Schnorr> {
    match name {
        "deterministic" => Some(Schnorr::Deterministic),
        "fresh" => Some(Schnorr::Fresh),
        _ => None,
    }
}
