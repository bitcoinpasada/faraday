//! Settings on the stick (`docs/history/PLAN.md` §5.2): `faraday-settings.txt` on a
//! stick's data partition, so the settings outlive a power-off.
//!
//! The file is plain text, because the theme and scale are wanted on the
//! passphrase screen and so cannot be inside a vault. It is read once, at
//! boot, from the boot stick, and is untrusted input (`docs/history/PLAN.md` §4.4): a
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

use crate::strings::EN;
use crate::strings::fill;
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

/// The settings: what the person chose for the display, motion, QR
/// speed, idle times, time zone and guided mode. Its fields are private;
/// the rest of the crate reads them through the accessors and changes
/// them through the methods.
pub(crate) struct Settings {
    /// The person's display scale, in percent of the automatic one.
    scale_pct: u16,
    /// The palette the screens are drawn in.
    theme: crate::ui::Theme,
    /// Reduce motion: nothing slides, glides, coasts or stretches.
    reduce_motion: bool,
    /// Minutes without input before the session locks; 0 for never
    /// (`docs/history/PLAN.md` §12.3).
    idle_lock_min: u16,
    /// How long each frame of an animated code stays up, in ms.
    qr_frame_ms: u64,
    /// Minutes without input before the machine powers off, when nothing
    /// is held and the Outbox is empty; 0 for never.
    idle_off_min: u16,
    /// The time zone, minutes east of UTC (`docs/TOTP.md` §2.1): −720 to
    /// +840, a multiple of 15.
    tz_min: i16,
    /// Guided mode: flows show their written walk-through.
    guided: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            scale_pct: 100,
            theme: crate::ui::Theme::default(),
            reduce_motion: false,
            idle_lock_min: crate::IDLE_LOCK_MIN,
            qr_frame_ms: crate::QR_SPEEDS[0],
            idle_off_min: crate::IDLE_OFF_MIN,
            tz_min: 0,
            // A device that has never had a setting saved opens Steps
            // only; Guided, with its explanations, is a choice (F82). A saved
            // value, from the kept state or the boot stick's settings
            // file, overwrites this before the person sees a screen.
            guided: false,
        }
    }
}

impl Settings {
    /// The display scale, in percent.
    pub(crate) fn scale_pct(&self) -> u16 {
        self.scale_pct
    }

    /// The palette.
    pub(crate) fn theme(&self) -> crate::ui::Theme {
        self.theme
    }

    /// Whether motion is reduced.
    pub(crate) fn reduce_motion(&self) -> bool {
        self.reduce_motion
    }

    /// Minutes of idle before the lock; 0 for never.
    pub(crate) fn idle_lock_min(&self) -> u16 {
        self.idle_lock_min
    }

    /// Milliseconds each frame of an animated code stays up.
    pub(crate) fn qr_frame_ms(&self) -> u64 {
        self.qr_frame_ms
    }

    /// Minutes of idle before the power-off; 0 for never.
    pub(crate) fn idle_off_min(&self) -> u16 {
        self.idle_off_min
    }

    /// The time zone, minutes east of UTC.
    pub(crate) fn tz_min(&self) -> i16 {
        self.tz_min
    }

    /// Whether guided mode is on.
    pub(crate) fn guided(&self) -> bool {
        self.guided
    }

    /// Sets the display scale.
    pub(crate) fn set_scale(&mut self, pct: u16) {
        self.scale_pct = pct;
    }

    /// Sets the palette.
    pub(crate) fn set_theme(&mut self, theme: crate::ui::Theme) {
        self.theme = theme;
    }

    /// Sets reduce motion.
    pub(crate) fn set_reduce_motion(&mut self, on: bool) {
        self.reduce_motion = on;
    }

    /// Sets the idle lock time.
    pub(crate) fn set_idle_lock_min(&mut self, m: u16) {
        self.idle_lock_min = m;
    }

    /// Sets the idle power-off time.
    pub(crate) fn set_idle_off_min(&mut self, m: u16) {
        self.idle_off_min = m;
    }

    /// Sets how long each frame of an animated code stays up.
    pub(crate) fn set_qr_frame_ms(&mut self, ms: u64) {
        self.qr_frame_ms = ms;
    }

    /// Sets guided mode.
    pub(crate) fn set_guided(&mut self, on: bool) {
        self.guided = on;
    }

    /// Steps the time zone by `delta` minutes, stopping at −12:00 and
    /// +14:00.
    pub(crate) fn step_tz(&mut self, delta: i16) {
        self.tz_min = (self.tz_min + delta).clamp(-720, 840);
    }

    /// The settings a stick carries, one `key=value` line each: what the
    /// file holds after its first line, and the start of the kept set.
    pub(crate) fn body(&self) -> String {
        format!(
            "scale={}\ntheme={}\nmotion={}\nguided={}\nqr-ms={}\nidle-lock={}\nidle-off={}\ntz={}\n",
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
            crate::vault::tz_offset_text(self.tz_min),
        )
    }

    /// Takes one setting if its value is one Settings offers. A line from
    /// a stick (`from_stick`) cannot choose never for an idle time.
    /// Returns whether it was a scale, which the glue lays out at once.
    pub(crate) fn apply(&mut self, key: &str, value: &str, from_stick: bool) -> bool {
        match key {
            "scale" => {
                if let Ok(p) = value.parse::<u16>()
                    && (50..=300).contains(&p)
                {
                    self.scale_pct = p;
                    return true;
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
            "tz" => {
                if let Some(tz) = crate::vault::parse_tz_offset(value) {
                    self.tz_min = tz;
                }
            }
            _ => {}
        }
        false
    }
}

impl Faraday {
    /// The settings a stick carries (see [`Settings::body`]).
    pub(crate) fn settings_body(&self) -> String {
        self.settings.body()
    }

    /// The settings file's whole text.
    pub(crate) fn settings_file_text(&self) -> String {
        format!("{HEADER}\n{}", self.settings_body())
    }

    /// Takes one setting if its value is one Settings offers; a scale is
    /// laid out at once.
    pub(crate) fn apply_setting(&mut self, key: &str, value: &str, from_stick: bool) {
        if self.settings.apply(key, value, from_stick) {
            self.layout_scale();
        }
    }

    /// The scale just set, from the kept set or a stick, laid out at once.
    fn layout_scale(&mut self) {
        if let Some(d) = self.last_display {
            // Only what this display adds is dropped: the first request
            // for entropy may still be waiting.
            let before = self.out.commands.len();
            self.display(d);
            let mut i = before;
            while i < self.out.commands.len() {
                if self.out.commands[i] == faraday_shell_api::Command::RequestEntropy {
                    self.out.commands.remove(i);
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
        self.stick.settings = Some(self.settings_body());
        self.save_boxes();
    }

    /// Takes the storage answers about the settings file and hands every
    /// other event back.
    pub(crate) fn settings_event(&mut self, event: StorageEvent) -> Option<StorageEvent> {
        let boot = |app: &Faraday, id: &str| app.stick.iter().any(|s| s.id == id && s.boot);
        match event {
            StorageEvent::Read { stick, name, bytes } if is_file(&name) => {
                if self.stick.settings.is_none() && boot(self, &stick) {
                    self.settings_from_stick(&bytes);
                }
            }
            StorageEvent::ReadFailed { stick, name, .. } if is_file(&name) => {
                if self.stick.settings.is_none() && boot(self, &stick) {
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
                self.visit.log.push((
                    fill(EN.stick_settings_wrote, &[&wrote_as.to_string()]),
                    true,
                ));
            }
            StorageEvent::WriteFailed { name, reason, .. } if is_file(&name) => {
                self.visit.log.push((
                    fill(
                        EN.stick_settings_not_written,
                        &[&name.to_string(), &reason.to_string()],
                    ),
                    false,
                ));
            }
            other => return Some(other),
        }
        None
    }

    /// Whether the visit's Settings row is ticked: as the person set it,
    /// or by default on the boot stick when the settings changed.
    pub fn visit_settings_on(&self) -> bool {
        self.visit.settings.unwrap_or_else(|| {
            self.stick.get(self.visit.stick).is_some_and(|s| s.boot)
                && self
                    .stick
                    .settings()
                    .is_some_and(|b| *b != self.settings_body())
        })
    }
}

impl Faraday {
    /// The person's display scale, in percent of the automatic one.
    pub fn scale_pct(&self) -> u16 {
        self.settings.scale_pct()
    }

    /// The palette the screens are drawn in.
    pub fn theme(&self) -> crate::ui::Theme {
        self.settings.theme()
    }

    /// Reduce motion: nothing slides, glides, coasts or stretches.
    pub fn reduce_motion(&self) -> bool {
        self.settings.reduce_motion()
    }

    /// Minutes without input before the session locks; 0 for never.
    pub fn idle_lock_min(&self) -> u16 {
        self.settings.idle_lock_min()
    }

    /// How long each frame of an animated code stays up, in ms.
    pub fn qr_frame_ms(&self) -> u64 {
        self.settings.qr_frame_ms()
    }

    /// Minutes without input before the machine powers off, when nothing
    /// is held and the Outbox is empty; 0 for never.
    pub fn idle_off_min(&self) -> u16 {
        self.settings.idle_off_min()
    }

    /// The time zone, minutes east of UTC: −720 to +840, a multiple of 15.
    pub fn tz_min(&self) -> i16 {
        self.settings.tz_min()
    }

    /// Guided mode: flows show their written walk-through.
    pub fn guided(&self) -> bool {
        self.settings.guided()
    }
}
