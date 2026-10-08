//! Settings on the stick (`PLAN.md` §5.2): `faraday-settings.txt` on a
//! stick's data partition, so the settings outlive a power-off.
//!
//! The file is plain text, because the theme and scale are wanted on the
//! passphrase screen and so cannot be inside a vault. It is read once, at
//! boot, from the boot stick, and is untrusted input (`PLAN.md` §4.4): a
//! line is taken only when its key is known and its value is one Settings
//! itself offers, and nothing read from it makes a fresh boot less
//! protected than the defaults. **Never** for the idle times, the
//! signed-amount memory's seal and the network are not in the file; they
//! are chosen on the device each session.
//!
//! A stick visit offers the file as one more row to write, ticked on the
//! boot stick when the settings differ from what the boot stick holds,
//! unticked on any other stick. The disk process writes it over the file
//! already there (`faraday_files::SETTINGS_FILE`, the same name).

use crate::{Faraday, StorageEvent};

/// The file's name on a stick.
pub const FILE: &str = "faraday-settings.txt";

/// The file's first line.
pub const HEADER: &str = "faraday-settings 1";

/// The largest file read. Seven short lines are well under it.
pub const MAX: usize = 4096;

/// The idle lock times Settings offers, in minutes; 0 is never.
pub const IDLE_LOCK_CHOICES: [u16; 5] = [2, 5, 10, 30, 0];

/// The idle power-off times Settings offers, in minutes; 0 is never.
pub const IDLE_OFF_CHOICES: [u16; 5] = [10, 20, 30, 60, 0];

/// Whether a name is the settings file, as FAT compares names.
pub fn is_file(name: &str) -> bool {
    name.eq_ignore_ascii_case(FILE)
}

impl Faraday {
    /// The settings a stick carries, one `key=value` line each: what the
    /// file holds after its first line, and the start of the kept set.
    pub(crate) fn settings_body(&self) -> String {
        format!(
            "scale={}\ntheme={}\nmotion={}\nguided={}\nqr-ms={}\nidle-lock={}\nidle-off={}\n",
            self.scale_pct,
            self.theme.id(),
            if self.reduce_motion {
                "reduced"
            } else {
                "full"
            },
            u8::from(self.guided),
            self.qr_frame_ms,
            self.idle_lock_min,
            self.idle_off_min,
        )
    }

    /// The settings file's whole text.
    pub(crate) fn settings_file_text(&self) -> String {
        format!("{HEADER}\n{}", self.settings_body())
    }

    /// Takes one setting if its value is one Settings offers. A line from
    /// a stick (`from_stick`) cannot choose never for an idle time.
    pub(crate) fn apply_setting(&mut self, key: &str, value: &str, from_stick: bool) {
        match key {
            "scale" => {
                if let Ok(p) = value.parse::<u16>()
                    && (50..=300).contains(&p)
                {
                    self.set_scale_kept(p);
                }
            }
            "theme" => {
                if let Some(t) = crate::ui::Theme::from_id(value) {
                    self.theme = t;
                }
            }
            "motion" => match value {
                "reduced" => self.reduce_motion = true,
                "full" => self.reduce_motion = false,
                _ => {}
            },
            "guided" => match value {
                "1" => self.guided = true,
                "0" => self.guided = false,
                _ => {}
            },
            "qr-ms" => {
                if let Ok(ms) = value.parse::<u64>()
                    && crate::QR_SPEEDS.contains(&ms)
                {
                    self.qr_frame_ms = ms;
                }
            }
            "idle-lock" => {
                if let Ok(m) = value.parse::<u16>()
                    && IDLE_LOCK_CHOICES.contains(&m)
                    && !(from_stick && m == 0)
                {
                    self.idle_lock_min = m;
                }
            }
            "idle-off" => {
                if let Ok(m) = value.parse::<u16>()
                    && IDLE_OFF_CHOICES.contains(&m)
                    && !(from_stick && m == 0)
                {
                    self.idle_off_min = m;
                }
            }
            _ => {}
        }
    }

    /// A scale from the kept set or a stick, laid out at once.
    fn set_scale_kept(&mut self, pct: u16) {
        self.scale_pct = pct;
        if let Some(d) = self.last_display {
            // Only what this display adds is dropped: the first request
            // for entropy may still be waiting.
            let before = self.commands.len();
            self.display(d);
            let mut i = before;
            while i < self.commands.len() {
                if self.commands[i] == osk_shell_api::Command::RequestEntropy {
                    self.commands.remove(i);
                } else {
                    i += 1;
                }
            }
        }
    }

    /// The boot stick's settings file arrived: applied line by line, or
    /// ignored whole when it is too large, not text, or not this format.
    /// Either way what the boot stick holds is now known.
    pub(crate) fn settings_from_stick(&mut self, bytes: &[u8]) {
        if bytes.len() <= MAX
            && let Ok(text) = std::str::from_utf8(bytes)
        {
            let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
            if lines.next() == Some(HEADER) {
                for line in lines {
                    if let Some((k, v)) = line.split_once('=') {
                        self.apply_setting(k, v, true);
                    }
                }
            }
        }
        self.settings_on_stick_known();
    }

    /// What the boot stick holds is the settings now: read from it, or the
    /// settings it would have given had it held a file.
    pub(crate) fn settings_on_stick_known(&mut self) {
        self.stick_settings = Some(self.settings_body());
        self.save_boxes();
    }

    /// Takes the storage answers about the settings file and hands every
    /// other event back.
    pub(crate) fn settings_event(&mut self, event: StorageEvent) -> Option<StorageEvent> {
        let boot = |app: &Faraday, id: &str| app.sticks.iter().any(|s| s.id == id && s.boot);
        match event {
            StorageEvent::Read { stick, name, bytes } if is_file(&name) => {
                if self.stick_settings.is_none() && boot(self, &stick) {
                    self.settings_from_stick(&bytes);
                }
            }
            StorageEvent::ReadFailed { stick, name, .. } if is_file(&name) => {
                if self.stick_settings.is_none() && boot(self, &stick) {
                    self.settings_on_stick_known();
                }
            }
            StorageEvent::Written {
                stick,
                name,
                wrote_as,
            } if is_file(&name) => {
                if boot(self, &stick) {
                    self.settings_on_stick_known();
                }
                self.visit.settings = None;
                self.visit
                    .log
                    .push((format!("Wrote {wrote_as}, read back and matched"), true));
            }
            StorageEvent::WriteFailed { name, reason, .. } if is_file(&name) => {
                self.visit
                    .log
                    .push((format!("{name} not written: {reason}"), false));
            }
            other => return Some(other),
        }
        None
    }

    /// Whether the visit's Settings row is ticked: as the person set it,
    /// or by default on the boot stick when the settings changed.
    pub fn visit_settings_on(&self) -> bool {
        self.visit.settings.unwrap_or_else(|| {
            self.sticks.get(self.visit.stick).is_some_and(|s| s.boot)
                && self
                    .stick_settings
                    .as_ref()
                    .is_some_and(|b| *b != self.settings_body())
        })
    }
}
