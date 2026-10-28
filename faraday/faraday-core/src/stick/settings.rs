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
use faraday_shell_api::{Command, Orientation};

/// The file's name on a stick.
pub const FILE: &str = "faraday-settings.txt";

/// The file's first line.
pub const HEADER: &str = "faraday-settings 1";

/// The largest file read. Its ten short lines are well under it.
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
    /// Which way a panel that can turn shows the app
    /// (`docs/ORIENTATION.md` §6); Landscape by default (F100).
    orientation: Orientation,
    /// The panel's own orientation, once the shell has said it can turn
    /// its frame: Rotation is offered. Never in the file or the kept
    /// state; the shell says it again at every start.
    rotation_native: Option<Orientation>,
    /// The scheme the screens are drawn in: a theme, a mode and a
    /// contrast level (`docs/THEMES.md` §3).
    theme: crate::ui::Theme,
    /// How large text is drawn (`docs/TEXT-SIZE.md` §2).
    text: crate::ui::TextSize,
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
    /// Faraday Demo's host holds the app (`docs/DESKTOP-HOST.md`):
    /// its stick pills, Load samples, the preview size and every demo
    /// aid stand in the host's strip, outside the app, so the app draws
    /// none of them. Never in the file or the kept state.
    hosted: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            scale_pct: 100,
            orientation: Orientation::Landscape,
            rotation_native: None,
            theme: crate::ui::Theme::default(),
            text: crate::ui::TextSize::Standard,
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
            hosted: false,
        }
    }
}

impl Settings {
    /// The display scale, in percent.
    pub(crate) fn scale_pct(&self) -> u16 {
        self.scale_pct
    }

    /// Which way a panel that can turn shows the app.
    pub(crate) fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Sets which way a panel that can turn shows the app.
    pub(crate) fn set_orientation(&mut self, o: Orientation) {
        self.orientation = o;
    }

    /// The panel's own orientation, where Rotation is offered.
    pub(crate) fn rotation_native(&self) -> Option<Orientation> {
        self.rotation_native
    }

    /// Notes that the shell can turn a panel whose own orientation is
    /// `native`.
    pub(crate) fn set_rotation_native(&mut self, native: Orientation) {
        self.rotation_native = Some(native);
    }

    /// The scheme.
    pub(crate) fn theme(&self) -> crate::ui::Theme {
        self.theme
    }

    /// How large text is drawn.
    pub(crate) fn text_size(&self) -> crate::ui::TextSize {
        self.text
    }

    /// Sets how large text is drawn.
    pub(crate) fn set_text_size(&mut self, text: crate::ui::TextSize) {
        self.text = text;
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

    /// Whether Faraday Demo's host holds the app.
    pub(crate) fn hosted(&self) -> bool {
        self.hosted
    }

    /// Notes that Faraday Demo's host holds the app.
    pub(crate) fn set_hosted(&mut self, on: bool) {
        self.hosted = on;
    }

    /// Sets the display scale.
    pub(crate) fn set_scale(&mut self, pct: u16) {
        self.scale_pct = pct;
    }

    /// Sets the scheme.
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
            "scale={}\nrotation={}\ntheme={}\nmode={}\ncontrast={}\ntext={}\nmotion={}\nguided={}\nqr-ms={}\nidle-lock={}\nidle-off={}\ntz={}\n",
            self.scale_pct,
            self.orientation.id(),
            self.theme.name.id(),
            self.theme.mode.id(),
            self.theme.contrast.id(),
            self.text.id(),
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
    /// Returns whether it was a scale or a text size, which the glue lays
    /// out at once.
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
            // Only the two Rotation offers; anything else leaves
            // Landscape (`docs/ORIENTATION.md` §6).
            "rotation" => {
                if let Some(o) = Orientation::from_id(value) {
                    self.orientation = o;
                }
            }
            // The lines are read in the order written, `theme=` first: a
            // theme of before the three choices opens in the mode and
            // contrast it maps to (`docs/THEMES.md` §13), and a `mode=`
            // or `contrast=` line after it sets that part as written.
            "theme" => {
                if let Some(t) = crate::ui::Theme::from_stored(value) {
                    self.theme = t;
                }
            }
            "mode" => {
                if let Some(mode) = crate::ui::Mode::from_id(value) {
                    self.theme.mode = mode;
                }
            }
            "contrast" => {
                if let Some(contrast) = crate::ui::Contrast::from_id(value) {
                    self.theme.contrast = contrast;
                }
            }
            // Only the sizes Settings knows; anything else leaves the size
            // as it was (`docs/TEXT-SIZE.md` §2, §6).
            "text" => {
                if let Some(t) = crate::ui::TextSize::from_id(value) {
                    self.text = t;
                    return true;
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

    /// Takes one setting if its value is one Settings offers; a scale or
    /// a text size is laid out at once, and a rotation asked of the
    /// shell at once.
    pub(crate) fn apply_setting(&mut self, key: &str, value: &str, from_stick: bool) {
        let turned = self.settings.orientation();
        if self.settings.apply(key, value, from_stick) {
            self.layout_scale();
        }
        if self.settings.orientation() != turned {
            self.ask_orientation();
        }
    }

    /// Asks the shell to show the app the chosen way round, where it
    /// offered Rotation (`docs/ORIENTATION.md` §6, §7). Any other shell
    /// is asked nothing.
    fn ask_orientation(&mut self) {
        if self.settings.rotation_native().is_some() {
            let o = self.settings.orientation();
            self.out.commands.push_back(Command::Orientation(o));
        }
    }

    /// The shell can turn a panel whose own orientation is `native`
    /// (`Event::RotationOffered`): Settings offers Rotation, and the
    /// shell is asked for the kept choice.
    pub(crate) fn rotation_offered_by(&mut self, native: Orientation) {
        self.settings.set_rotation_native(native);
        self.ask_orientation();
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
    /// Settings' Appearance presses: the display scale, the preview size,
    /// the text size, motion and the palette. Laying the screens out again on a new
    /// scale costs what a display event does.
    pub(crate) fn appearance_act(&mut self, a: crate::Action) {
        use crate::Action;
        match a {
            Action::Scale(pct) => {
                self.settings.set_scale(pct);
                if let Some(d) = self.last_display {
                    self.display(d);
                }
                self.save_boxes();
            }
            // The shell turns its frame and tells the turned size, which
            // lays the screens out again (`docs/ORIENTATION.md` §7), with
            // Display in view: from the page of every section, Display
            // opens as its own page for a compact window, and from that
            // page the page of every section comes back where it was.
            Action::Rotation(o) => {
                if self.settings.orientation() == o {
                    return;
                }
                self.settings.set_orientation(o);
                if crate::panes::settings_listed(self.window()) {
                    self.settings_section(None);
                } else {
                    let display = crate::screens::settings_display_section();
                    self.settings_section(Some(display));
                }
                self.ask_orientation();
                self.save_boxes();
            }
            // Laid out again at once, as a scale is: the text size counts
            // in the one-column decision (`docs/TEXT-SIZE.md` §4).
            Action::TextSize(t) => {
                self.settings.set_text_size(t);
                if let Some(d) = self.last_display {
                    self.display(d);
                }
                self.save_boxes();
            }
            Action::ReduceMotion(on) => {
                self.settings.set_reduce_motion(on);
                self.scroll.stop();
                self.save_boxes();
            }
            Action::Theme(theme) => {
                self.settings.set_theme(theme);
                self.save_boxes();
            }
            _ => {}
        }
    }

    /// Draws the screens in `theme` from the next frame on and keeps
    /// nothing: the Starter's held app, in the scheme the Starter's own
    /// Appearance chose (`docs/THEMES.md` §14), which no settings file
    /// or kept set holds.
    pub fn draw_in(&mut self, theme: crate::ui::Theme) {
        self.settings.set_theme(theme);
        self.out.dirty = true;
    }

    /// Draws text at `text` from the next frame on, laid out again for
    /// it, and keeps nothing: the Starter's held app and the Faraday
    /// Demo it holds, at the size the Starter's own About chose
    /// (`docs/TEXT-SIZE.md` §6), which no settings file or kept set of
    /// theirs holds.
    pub fn draw_text_at(&mut self, text: crate::ui::TextSize) {
        if self.settings.text_size() == text {
            return;
        }
        self.settings.set_text_size(text);
        if let Some(d) = self.last_display {
            self.display(d);
        }
        self.out.dirty = true;
    }

    /// How large the person has text drawn (Settings › Appearance,
    /// `docs/TEXT-SIZE.md` §2).
    pub fn text_size(&self) -> crate::ui::TextSize {
        self.settings.text_size()
    }

    /// The person's display scale, in percent of the automatic one.
    pub fn scale_pct(&self) -> u16 {
        self.settings.scale_pct()
    }

    /// Which way the person chose a panel that can turn shows the app
    /// (Settings › Display › Rotation). A lookup.
    pub fn orientation(&self) -> Orientation {
        self.settings.orientation()
    }

    /// Whether Settings offers Rotation: the shell said it can turn its
    /// frame. A lookup.
    pub fn rotation_offered(&self) -> bool {
        self.settings.rotation_native().is_some()
    }

    /// The scheme the screens are drawn in.
    pub fn theme(&self) -> crate::ui::Theme {
        self.settings.theme()
    }

    /// The display the app was last given, if any: what Faraday Demo's
    /// host holds it at ([`crate::Host::strip_for`]). A lookup.
    pub fn last_display_info(&self) -> Option<faraday_shell_api::DisplayInfo> {
        self.last_display
    }

    /// Whether Faraday Demo's host holds the app
    /// (`docs/DESKTOP-HOST.md`): then the app draws no stick pills, no
    /// Load samples, no Preview size and no "Demo only" card; the host's
    /// strip holds them. A lookup.
    pub fn hosted(&self) -> bool {
        self.settings.hosted()
    }

    /// Notes that Faraday Demo's host holds the app ([`crate::Host`]).
    pub(crate) fn set_hosted(&mut self, on: bool) {
        self.settings.set_hosted(on);
        self.out.dirty = true;
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
