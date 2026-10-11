//! Settings and the screens its rows open, built from
//! `docs/DESIGN.md` §5: the Settings Menu, the Setting Choices behind
//! its value rows, the About Menu, the two Holds that wipe, and the
//! terminal Result the session ends on.
//!
//! §4.2 Setting: a Choice whose check is on the current value and which
//! has no Continue — the tap applies and the screen stays. §2.1 leaves
//! every note sentence off: a row states its value, and the value is
//! the whole of what the screen has to say.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use osk_bip::keys::Network;
use osk_shell_api::{BootState, SecureHardware};
use osk_ui::components::{self, Unit};
use osk_ui::layout::Node;
use osk_ui::screens::{self, Action, Hold, Item, Row};
use osk_ui::widgets::{Icon, Tone};

use crate::scan::{CAMERA_ROTATIONS, CameraRotation};
use crate::session::{LOCK_OPTIONS, WIPE_OPTIONS};
use crate::{BACKUP_MEMORY, NONCES, OpenSigner, SCHNORRS, Setting, UNITS, ids, strings};

impl OpenSigner {
    /// §5 Menu, "Settings": one row per setting, its value under its
    /// label, then About and the two wipes.
    pub(crate) fn view_settings(&self) -> Node {
        let s = self.strings();
        let can_lock = !self.keys.is_empty() && self.has_pin();
        let mut rows = vec![
            Row::Value {
                id: ids::SETTINGS_NETWORK_ROW,
                label: String::from(s.settings_network),
                value: String::from(self.network.name()),
                tone: Tone::Text,
            },
            Row::Value {
                id: ids::SETTINGS_UNIT_ROW,
                label: String::from(s.settings_unit),
                value: String::from(self.unit_name(self.unit)),
                tone: Tone::Text,
            },
            Row::Value {
                id: ids::SETTINGS_LOCK_AFTER_ROW,
                label: String::from(s.settings_lock_after),
                value: self.duration(self.session.lock_after_ms()),
                tone: Tone::Text,
            },
            Row::Value {
                id: ids::SETTINGS_WIPE_AFTER_ROW,
                label: String::from(s.settings_wipe_after),
                value: self.wipe_after_name(),
                // §4.8 State value: no auto-wipe is a state to act on,
                // so the value carries the caution rather than a card.
                tone: if self.session.wipe_after_ms().is_none() {
                    Tone::Caution
                } else {
                    Tone::Text
                },
            },
        ];
        // §4.9: the rotation is for a camera someone mounted in a case
        // they built. A shell whose frames are always upright has
        // nothing to choose, so the row is not there.
        if !self.camera_fixed() {
            rows.push(Row::Value {
                id: ids::SETTINGS_CAMERA_ROTATION_ROW,
                label: String::from(s.settings_camera_rotation),
                value: String::from(self.rotation_name(self.camera_rotation())),
                tone: Tone::Text,
            });
        }
        rows.push(Row::Toggle {
            id: ids::SETTINGS_SCRAMBLE,
            label: String::from(s.settings_scramble),
            on: self.session.scramble_pin(),
            reason: None,
        });
        rows.push(Row::Value {
            id: ids::SETTINGS_NONCE_ROW,
            label: String::from(s.settings_nonce),
            value: String::from(self.nonce_name(self.nonce())),
            tone: Tone::Text,
        });
        rows.push(Row::Value {
            id: ids::SETTINGS_SCHNORR_ROW,
            label: String::from(s.settings_schnorr),
            value: String::from(self.schnorr_name(self.schnorr())),
            tone: Tone::Text,
        });
        // §16.112: the Argon2id memory every encrypted export is made
        // at. The value is the MiB, which is what a file's header says
        // and what another device will have to allocate to open it.
        rows.push(Row::Value {
            id: ids::SETTINGS_BACKUP_MEMORY_ROW,
            label: String::from(s.settings_backup_memory),
            value: strings::fill1(
                s.backup_memory_value,
                &alloc::format!("{}", self.backup_memory_kib() / 1024),
            ),
            tone: Tone::Text,
        });
        rows.push(if can_lock {
            Row::Menu {
                id: ids::SETTINGS_LOCK,
                icon: Some(Icon::Lock),
                label: String::from(s.settings_lock_now),
                value: None,
                tone: Tone::Text,
            }
        } else {
            Row::Dimmed {
                icon: Some(Icon::Lock),
                label: String::from(s.settings_lock_now),
                reason: Some(String::from(s.reason_needs_key)),
            }
        });
        rows.push(Row::Fact {
            label: String::from(s.settings_memory_key),
            value: String::from(if self.session.is_weak() {
                s.settings_memory_key_weak
            } else {
                s.settings_memory_key_ok
            }),
            mono: false,
            tone: if self.session.is_weak() {
                Tone::Danger
            } else {
                Tone::Text
            },
        });
        // §5 Menu, Settings: the row a kept key adds, between the
        // session rows and About. Forgetting the stored key is the key's
        // own Forget, and Wipe all keys takes it too.
        if self.kept_rows_offered() {
            rows.push(Row::Menu {
                id: ids::KEEP_DURESS_ROW,
                icon: Some(Icon::UserSecret),
                label: String::from(s.keep_duress_row),
                value: None,
                tone: Tone::Text,
            });
        }
        rows.push(Row::Menu {
            id: ids::SETTINGS_ABOUT_ROW,
            icon: Some(Icon::Info),
            label: String::from(s.settings_about),
            value: None,
            tone: Tone::Text,
        });
        // The first run's document, reopened. It leaves the flag alone:
        // reading it again is not the first run happening again.
        rows.push(Row::Menu {
            id: ids::SETTINGS_START_HERE_ROW,
            icon: Some(Icon::Flag),
            label: String::from(s.learn.start_here.title),
            value: None,
            tone: Tone::Text,
        });
        rows.push(if self.keys.is_empty() {
            Row::Dimmed {
                icon: Some(Icon::Trash),
                label: String::from(s.settings_wipe_row),
                reason: Some(String::from(s.reason_needs_key)),
            }
        } else {
            Row::Menu {
                id: ids::SETTINGS_WIPE_ROW,
                icon: Some(Icon::Trash),
                label: String::from(s.settings_wipe_row),
                value: None,
                tone: Tone::Text,
            }
        });
        rows.push(Row::Menu {
            id: ids::SETTINGS_EXIT_ROW,
            icon: Some(Icon::Power),
            label: String::from(s.settings_exit_row),
            value: None,
            tone: Tone::Text,
        });
        self.with_chrome(Some(ids::BACK), |c| {
            // §4.12: the toggle is a switch, and nothing on the row
            // explains it.
            screens::menu(c, s.settings_title, None, rows, Vec::new())
        })
    }

    /// §4.2 Setting: "The same rows with the check on the current value;
    /// no Continue." The row that opened it is the question, so the
    /// title is the setting's name.
    pub(crate) fn view_setting(&self, setting: Setting) -> Node {
        let s = self.strings();
        let (title, items) = match setting {
            Setting::Network => (
                s.settings_network,
                Network::ALL
                    .iter()
                    .enumerate()
                    .map(|(i, n)| {
                        Item::chosen(
                            ids::at(ids::SETTINGS_NET_BASE, i),
                            n.name(),
                            *n == self.network,
                        )
                    })
                    .collect(),
            ),
            Setting::Unit => (
                s.settings_unit,
                UNITS
                    .iter()
                    .enumerate()
                    .map(|(i, u)| {
                        Item::chosen(
                            ids::at(ids::SETTINGS_UNIT_BASE, i),
                            self.unit_name(*u),
                            *u == self.unit,
                        )
                    })
                    .collect(),
            ),
            Setting::CameraRotation => (
                s.settings_camera_rotation,
                CAMERA_ROTATIONS
                    .iter()
                    .enumerate()
                    .map(|(i, r)| {
                        Item::chosen(
                            ids::at(ids::SETTINGS_CAMERA_ROTATION_BASE, i),
                            self.rotation_name(*r),
                            *r == self.camera_rotation(),
                        )
                    })
                    .collect(),
            ),
            Setting::Nonce => (
                s.settings_nonce,
                NONCES
                    .iter()
                    .enumerate()
                    .map(|(i, n)| {
                        Item::chosen(
                            ids::at(ids::SETTINGS_NONCE_BASE, i),
                            self.nonce_name(*n),
                            *n == self.nonce(),
                        )
                    })
                    .collect(),
            ),
            Setting::Schnorr => (
                s.settings_schnorr,
                SCHNORRS
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        Item::chosen(
                            ids::at(ids::SETTINGS_SCHNORR_BASE, i),
                            self.schnorr_name(*c),
                            *c == self.schnorr(),
                        )
                    })
                    .collect(),
            ),
            Setting::BackupMemory => (
                s.settings_backup_memory,
                BACKUP_MEMORY
                    .iter()
                    .enumerate()
                    .map(|(i, kib)| {
                        // §4.2: the row carries a value, and here the
                        // value is the trade-off in a few words. The
                        // recommended row says so; the two ends say
                        // what choosing them costs.
                        let line = if *kib == self.recommended_memory_kib() {
                            Some(s.settings_backup_memory_recommended)
                        } else if i == 0 {
                            Some(s.settings_backup_memory_anywhere)
                        } else if i == BACKUP_MEMORY.len() - 1 {
                            Some(s.settings_backup_memory_slowest)
                        } else {
                            None
                        };
                        let mut item = Item::chosen(
                            ids::at(ids::SETTINGS_BACKUP_MEMORY_BASE, i),
                            strings::fill1(
                                s.backup_memory_value,
                                &alloc::format!("{}", kib / 1024),
                            ),
                            *kib == self.backup_memory_kib(),
                        );
                        item.subtitle = line.map(String::from);
                        item
                    })
                    .collect(),
            ),
            Setting::Lock => (
                s.settings_lock_after,
                LOCK_OPTIONS
                    .iter()
                    .enumerate()
                    .map(|(i, ms)| {
                        Item::chosen(
                            ids::at(ids::SETTINGS_LOCK_AFTER_BASE, i),
                            self.duration(*ms),
                            *ms == self.session.lock_after_ms(),
                        )
                    })
                    .collect(),
            ),
            Setting::Wipe => (
                s.settings_wipe_after,
                WIPE_OPTIONS
                    .iter()
                    .enumerate()
                    .map(|(i, ms)| {
                        Item::chosen(
                            ids::at(ids::SETTINGS_WIPE_AFTER_BASE, i),
                            ms.map_or_else(|| String::from(s.settings_never), |m| self.duration(m)),
                            *ms == self.session.wipe_after_ms(),
                        )
                    })
                    .collect(),
            ),
        };
        self.with_chrome(Some(ids::BACK), |c| screens::settings(c, title, items))
    }

    /// §5 Menu, "About": what this build is, and the self-test it can
    /// run again.
    pub(crate) fn view_about(&self) -> Node {
        let s = self.strings();
        // The self-test runs on the first display event, before any
        // frame, so About always has an outcome to state.
        let selftest = self.selftest().map(|outcome| match outcome {
            Ok(n) => (
                strings::fill1(s.settings_selftest_passed, &alloc::format!("{n}")),
                Tone::Text,
            ),
            Err(check) => (
                strings::fill1(s.settings_selftest_failed, check),
                Tone::Danger,
            ),
        });
        let hash = match self.build.core_hash {
            Some(hash) => Row::Reference {
                id: ids::ABOUT_HASH,
                label: String::from(s.settings_core_hash),
                value: String::from(hash),
            },
            None => Row::Fact {
                label: String::from(s.settings_core_hash),
                value: String::from(s.settings_core_hash_none),
                mono: false,
                tone: Tone::Muted,
            },
        };
        let mut rows = vec![
            Row::Fact {
                label: String::from(s.settings_version),
                value: String::from(self.build.version),
                mono: false,
                tone: Tone::Text,
            },
            Row::Value {
                id: ids::ABOUT_TIER,
                label: String::from(s.settings_tier),
                value: String::from(self.tier.badge(s)),
                tone: Tone::Text,
            },
            hash,
            // §5 Menu, About: what backs this device\'s secure element,
            // which is what decides whether a key can be kept on it.
            Row::Fact {
                label: String::from(s.keep_secure_row),
                value: String::from(self.secure_name()),
                mono: false,
                tone: Tone::Text,
            },
            // What the platform's attestation says about the boot the
            // device came up from. An unverified boot weakens what the
            // element above promises, so the row is the caution tone.
            Row::Fact {
                label: String::from(s.settings_boot),
                value: String::from(self.boot_name()),
                mono: false,
                tone: match self.boot() {
                    BootState::Unverified => Tone::Caution,
                    _ => Tone::Text,
                },
            },
        ];
        if let Some((value, tone)) = selftest {
            rows.push(Row::Fact {
                label: String::from(s.settings_selftest),
                value,
                mono: false,
                tone,
            });
        }
        self.with_chrome(Some(ids::BACK), |c| {
            screens::menu(
                c,
                s.settings_about,
                None,
                rows,
                vec![Action::new(ids::SETTINGS_SELFTEST, s.settings_selftest_run)],
            )
        })
    }

    /// §5 Hold, "Wipe all keys": what will go as a table, and the hold.
    pub(crate) fn view_wipe_all(&self) -> Node {
        let s = self.strings();
        self.wipe_screen(
            s.settings_wipe_row,
            ids::SETTINGS_WIPE,
            s.settings_wipe_hold,
        )
    }

    /// §5 Hold, "Wipe and exit".
    pub(crate) fn view_wipe_and_exit(&self) -> Node {
        let s = self.strings();
        self.wipe_screen(
            s.settings_exit_title,
            ids::SETTINGS_EXIT,
            s.settings_exit_hold,
        )
    }

    /// §5 Result, "Keys removed": what a finished wipe leaves, which is
    /// the confirmation the empty Home used to have to carry.
    pub(crate) fn view_wiped(&self, keys: usize) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            screens::result(
                c,
                screens::Result {
                    caption: None,
                    title: s.settings_wipe_row,
                    icon: Icon::Success,
                    tone: Tone::Success,
                    result: s.settings_wiped_title,
                    rows: vec![components::Record::text(
                        s.row_keys,
                        alloc::format!("{keys}"),
                        Tone::Text,
                    )],
                    actions: vec![Action::new(ids::SETTINGS_WIPED_DONE, s.action_done)],
                },
            )
        })
    }

    /// §4.14 Terminal state: "Result layout with no action and no back
    /// chevron." It stays until the shell acts on the Exit command.
    pub(crate) fn view_session_ended(&self) -> Node {
        let s = self.strings();
        // Leaving by Back clears memory and keeps the device's copy;
        // "Wipe and exit" forgets that too. The screen says which.
        let (title, keys) = if self.ended_by_leave {
            (s.leave_title, s.leave_ended_keys)
        } else {
            (s.settings_exit_title, s.settings_ended_keys)
        };
        self.with_chrome(None, |c| {
            screens::result(
                c,
                screens::Result {
                    caption: None,
                    title,
                    icon: Icon::Success,
                    tone: Tone::Success,
                    result: s.settings_ended_title,
                    rows: vec![components::Record::text(s.row_keys, keys, Tone::Text)],
                    actions: Vec::new(),
                },
            )
        })
    }

    /// The one Hold both wipes are: the keys it removes as a table, and
    /// the verb on the button. §4.13 puts no sentence above it. A wipe
    /// a person asks for is the only one that takes the key kept on the
    /// device, so the table says so while one is kept.
    fn wipe_screen(&self, title: &'static str, hold: ids::Id, label: &'static str) -> Node {
        let s = self.strings();
        self.with_chrome(Some(ids::BACK), |c| {
            let mut rows = vec![components::Record::text(
                s.row_keys,
                alloc::format!("{}", self.keys.len()),
                Tone::Text,
            )];
            if self.kept_rows_offered() {
                rows.push(components::Record::text(
                    s.keep_stored_row,
                    s.settings_wipe_stored,
                    Tone::Danger,
                ));
            }
            screens::hold(
                c,
                Hold {
                    warnings: Vec::new(),
                    title,
                    rows,
                    sign_with: None,
                    then_with: None,
                    id: hold,
                    label,
                    danger: true,
                    enabled: true,
                    secondary: None,
                },
            )
        })
    }

    /// About and the kept-key row: "StrongBox", "TEE" or "none".
    pub(crate) fn secure_name(&self) -> &'static str {
        let s = self.strings();
        match self.secure() {
            SecureHardware::None => s.value_none,
            SecureHardware::Tee => s.keep_secure_tee,
            SecureHardware::StrongBox => s.keep_secure_strongbox,
        }
    }

    /// About's verified-boot row: "yes", "no" or "unknown".
    pub(crate) fn boot_name(&self) -> &'static str {
        let s = self.strings();
        match self.boot() {
            BootState::Unknown => s.value_unknown,
            BootState::Verified => s.value_yes,
            BootState::Unverified => s.value_no,
        }
    }

    /// §4.7: the setting's rows read "sats" and "BTC".
    pub(crate) fn unit_name(&self, unit: Unit) -> &'static str {
        let s = self.strings();
        match unit {
            Unit::Sat => s.settings_unit_sat,
            Unit::Btc => s.settings_unit_btc,
        }
    }

    /// §4.9: the rotation's rows read "0°", "90°", "180°", "270°".
    pub(crate) fn rotation_name(&self, rotation: CameraRotation) -> &'static str {
        let s = self.strings();
        match rotation {
            CameraRotation::Deg0 => s.settings_camera_rotation_0,
            CameraRotation::Deg90 => s.settings_camera_rotation_90,
            CameraRotation::Deg180 => s.settings_camera_rotation_180,
            CameraRotation::Deg270 => s.settings_camera_rotation_270,
        }
    }

    /// The nonce setting's rows read "Low R" and "First".
    pub(crate) fn nonce_name(&self, nonce: osk_psbt::Nonce) -> &'static str {
        let s = self.strings();
        match nonce {
            osk_psbt::Nonce::LowR => s.settings_nonce_low_r,
            osk_psbt::Nonce::First => s.settings_nonce_first,
        }
    }

    /// The Schnorr setting's rows read "Deterministic" and "Fresh
    /// randomness".
    pub(crate) fn schnorr_name(&self, schnorr: osk_psbt::Schnorr) -> &'static str {
        let s = self.strings();
        match schnorr {
            osk_psbt::Schnorr::Deterministic => s.settings_schnorr_deterministic,
            osk_psbt::Schnorr::Fresh => s.settings_schnorr_fresh,
        }
    }

    /// The auto-wipe timer, or "never".
    fn wipe_after_name(&self) -> String {
        let s = self.strings();
        self.session
            .wipe_after_ms()
            .map_or_else(|| String::from(s.settings_never), |m| self.duration(m))
    }

    /// `30 s`, `2 min`.
    fn duration(&self, ms: u64) -> String {
        let s = self.strings();
        if ms < 60_000 {
            strings::fill1(s.duration_seconds, &alloc::format!("{}", ms / 1000))
        } else {
            strings::fill1(s.duration_minutes, &alloc::format!("{}", ms / 60_000))
        }
    }
}
