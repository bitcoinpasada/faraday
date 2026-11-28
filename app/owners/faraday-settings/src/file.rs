//! The settings file on the boot stick (`faraday-settings.txt`), ported from `main`'s
//! `stick/settings.rs` and `stick/settings_choice.rs` (the specification, 03 §7.2).
//!
//! The file is plain text, because the theme and scale are wanted on the passphrase screen and so
//! cannot be inside a vault. It comes from a stick and is untrusted: a line is taken only when its
//! key is known and its value is one Settings itself offers, a file too large, not text or not
//! this format is ignored whole, and a stick cannot turn an idle time off (`never`). The
//! signed-amount memory's seal, the nonce check's history, the clock's offset and the network are
//! not file keys: they are chosen on the device each session and carried through a lock only.

use faraday_kernel::settings::{
    Appearance, Choice, Contrast, MotionSpeed, MotionStyle, Settings, TextSize, ThemeId,
};
use faraday_port::values::Orientation;
use faraday_templates::Theme;

/// The file's name on a stick.
pub const FILE: &str = "faraday-settings.txt";

/// The file's first line.
pub const HEADER: &str = "faraday-settings 1";

/// The largest file read. Its ten short lines are well under it.
pub const MAX: usize = 4096;

/// The display scales Settings offers, in percent.
pub const SCALES: [u16; 7] = [75, 90, 100, 110, 125, 150, 200];

/// The animated code's frame times Settings offers, in ms (`faraday_qr::send::SPEEDS` on `main`).
pub const QR_SPEEDS: [u32; 3] = [500, 333, 200];

/// The idle lock times Settings offers, in minutes; 0 is never.
pub const IDLE_LOCK_CHOICES: [u16; 5] = [2, 5, 10, 30, 0];

/// The idle power-off times Settings offers, in minutes; 0 is never.
pub const IDLE_OFF_CHOICES: [u16; 5] = [10, 20, 30, 60, 0];

/// The time zones' bounds, in minutes east of UTC: -12:00 and +14:00.
const TZ_MIN: i16 = -720;
const TZ_MAX: i16 = 840;

/// Whether a name is the settings file, as FAT compares names.
pub fn is_file(name: &str) -> bool {
    name.eq_ignore_ascii_case(FILE)
}

/// Where a line came from. A line from a stick cannot choose never for an idle time; what the
/// device kept itself (the carry) can.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Source {
    Stick,
    Kept,
}

/// The scheme the appearance names: its theme, mode and contrast, as one of the kit's rows.
fn scheme(a: &Appearance) -> Theme {
    let contrast = match a.contrast {
        Contrast::Standard => 0,
        Contrast::Medium => 1,
        Contrast::High => 2,
    };
    let row = usize::from(a.theme.0) * 6 + usize::from(a.dark) * 3 + contrast;
    Theme::from_row(row).unwrap_or_default()
}

/// Puts `theme` into the appearance as its three parts.
fn set_scheme(a: &mut Appearance, theme: Theme) {
    let row = theme.row();
    a.theme = ThemeId((row / 6) as u8);
    a.dark = (row / 3) % 2 == 1;
    a.contrast = match row % 3 {
        0 => Contrast::Standard,
        1 => Contrast::Medium,
        _ => Contrast::High,
    };
}

/// The word a settings file writes the orientation as.
fn orientation_id(o: Orientation) -> &'static str {
    match o {
        Orientation::Landscape => "landscape",
        Orientation::Portrait => "portrait",
    }
}

/// The orientation `id` names exactly, or `None`.
fn orientation_from_id(id: &str) -> Option<Orientation> {
    match id {
        "landscape" => Some(Orientation::Landscape),
        "portrait" => Some(Orientation::Portrait),
        _ => None,
    }
}

fn text_size(t: TextSize) -> faraday_templates::TextSize {
    match t {
        TextSize::Standard => faraday_templates::TextSize::Standard,
        TextSize::Large => faraday_templates::TextSize::Large,
        TextSize::Larger => faraday_templates::TextSize::Larger,
    }
}

fn motion_id(m: MotionStyle) -> &'static str {
    match m {
        MotionStyle::Smooth => "smooth",
        MotionStyle::Crossfade => "crossfade",
        MotionStyle::Off => "off",
    }
}

/// The choice a `motion=` word names; `full` and `reduced` are what files wrote while Reduce
/// motion was a switch.
fn motion_from_id(id: &str) -> Option<MotionStyle> {
    match id {
        "full" | "smooth" => Some(MotionStyle::Smooth),
        "crossfade" => Some(MotionStyle::Crossfade),
        "reduced" | "off" => Some(MotionStyle::Off),
        _ => None,
    }
}

fn speed_id(s: MotionSpeed) -> &'static str {
    match s {
        MotionSpeed::Quick => "quick",
        MotionSpeed::Medium => "medium",
        MotionSpeed::Relaxed => "relaxed",
    }
}

fn speed_from_id(id: &str) -> Option<MotionSpeed> {
    match id {
        "quick" => Some(MotionSpeed::Quick),
        "medium" => Some(MotionSpeed::Medium),
        "relaxed" => Some(MotionSpeed::Relaxed),
        _ => None,
    }
}

/// A time zone offset, minutes east of UTC, as `+02:00`, `-03:30` or `+00:00`: the sign always
/// written, two digits each of hours and minutes.
pub(crate) fn tz_offset_text(tz_min: i16) -> String {
    let sign = if tz_min < 0 { '-' } else { '+' };
    let abs = tz_min.unsigned_abs();
    format!("{sign}{:02}:{:02}", abs / 60, abs % 60)
}

/// `s` as a time zone offset `tz_offset_text` writes: the sign, two digits of hours, `:`, two
/// digits of minutes a multiple of 15, in range -12:00 to +14:00.
///
/// `main` read each digit field as a number, which let a sign inside a field through (`+-1:00`
/// read as -01:00); here a field is two ASCII digits, so those are refused.
pub(crate) fn parse_tz_offset(s: &str) -> Option<i16> {
    let b = s.as_bytes();
    if b.len() != 6 || b[3] != b':' {
        return None;
    }
    let sign: i16 = match b[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let digits = |from: usize| -> Option<i16> {
        let (tens, ones) = (b[from], b[from + 1]);
        (tens.is_ascii_digit() && ones.is_ascii_digit())
            .then(|| i16::from(tens - b'0') * 10 + i16::from(ones - b'0'))
    };
    let hh = digits(1)?;
    let mm = digits(4)?;
    if !matches!(mm, 0 | 15 | 30 | 45) {
        return None;
    }
    let total = sign * (hh * 60 + mm);
    (TZ_MIN..=TZ_MAX).contains(&total).then_some(total)
}

/// The settings a stick carries, one `key=value` line each: what the file holds after its first
/// line.
pub fn body(s: &Settings) -> String {
    let theme = scheme(&s.appearance);
    format!(
        "scale={}\nrotation={}\ntheme={}\nmode={}\ncontrast={}\ntext={}\n{}speed={}\nqr-ms={}\nidle-lock={}\nidle-off={}\n{}",
        s.scale_pct,
        orientation_id(s.rotation),
        theme.name.id(),
        theme.mode.id(),
        theme.contrast.id(),
        text_size(s.appearance.text).id(),
        // Only a choice is kept, so a default does not outlive the screen it was for.
        s.appearance
            .motion
            .map(|m| format!("motion={}\n", motion_id(m)))
            .unwrap_or_default(),
        speed_id(s.appearance.speed),
        s.qr_frame_ms,
        s.idle_lock_min,
        s.idle_off_min,
        // Only a chosen zone is kept: Not set has no line.
        s.zone_min
            .map(|m| format!("tz={}\n", tz_offset_text(m)))
            .unwrap_or_default(),
    )
}

/// The settings file's whole text.
pub fn file_text(s: &Settings) -> String {
    format!("{HEADER}\n{}", body(s))
}

/// Takes one setting if its value is one Settings offers; anything else leaves the field as it
/// was.
pub(crate) fn apply(s: &mut Settings, key: &str, value: &str, from: Source) {
    match key {
        "scale" => {
            if let Ok(p) = value.parse::<u16>()
                && SCALES.contains(&p)
            {
                s.scale_pct = p;
            }
        }
        // Only the two Rotation offers; anything else leaves Landscape.
        "rotation" => {
            if let Some(o) = orientation_from_id(value) {
                s.rotation = o;
            }
        }
        // The lines are read in the order written, `theme=` first: a theme of before the three
        // choices opens in the mode and contrast it maps to, and a `mode=` or `contrast=` line
        // after it sets that part as written.
        "theme" => {
            if let Some(t) = Theme::from_stored(value) {
                set_scheme(&mut s.appearance, t);
            }
        }
        // The file's `mode=` is Light or Dark; it is not the network the app runs on.
        "mode" => {
            if let Some(t) = Theme::all().find(|t| t.mode.id() == value) {
                s.appearance.dark = (t.row() / 3) % 2 == 1;
            }
        }
        "contrast" => {
            if let Some(t) = Theme::all().find(|t| t.contrast.id() == value) {
                set_contrast(&mut s.appearance, t.row() % 3);
            }
        }
        // Only the sizes Settings knows; anything else leaves the size as it was.
        "text" => {
            if let Some(t) = [TextSize::Standard, TextSize::Large, TextSize::Larger]
                .into_iter()
                .find(|t| text_size(*t).id() == value)
            {
                s.appearance.text = t;
            }
        }
        // An unknown word leaves the default.
        "motion" => {
            if let Some(m) = motion_from_id(value) {
                s.appearance.motion = Some(m);
            }
        }
        "speed" => {
            if let Some(v) = speed_from_id(value) {
                s.appearance.speed = v;
            }
        }
        // Kept in files written before Guided mode was taken out.
        "guided" => {}
        "qr-ms" => {
            if let Ok(ms) = value.parse::<u32>()
                && QR_SPEEDS.contains(&ms)
            {
                s.qr_frame_ms = ms;
            }
        }
        "idle-lock" => {
            if let Ok(m) = value.parse::<u16>()
                && IDLE_LOCK_CHOICES.contains(&m)
                && !(from == Source::Stick && m == 0)
            {
                s.idle_lock_min = m;
            }
        }
        "idle-off" => {
            if let Ok(m) = value.parse::<u16>()
                && IDLE_OFF_CHOICES.contains(&m)
                && !(from == Source::Stick && m == 0)
            {
                s.idle_off_min = m;
            }
        }
        "tz" => {
            if let Some(tz) = parse_tz_offset(value) {
                s.zone_min = Some(tz);
            }
        }
        _ => {}
    }
}

fn set_contrast(a: &mut Appearance, level: usize) {
    a.contrast = match level {
        0 => Contrast::Standard,
        1 => Contrast::Medium,
        _ => Contrast::High,
    };
}

/// The boot stick's settings file arrived: applied line by line, or ignored whole when it is too
/// large, not text, or not this format.
pub(crate) fn read(s: &mut Settings, bytes: &[u8]) {
    if bytes.len() <= MAX
        && let Ok(text) = core::str::from_utf8(bytes)
    {
        let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
        if lines.next() == Some(HEADER) {
            for line in lines {
                if let Some((k, v)) = line.split_once('=') {
                    apply(s, k, v, Source::Stick);
                }
            }
        }
    }
}

/// Reads what the device kept itself (the carry's body text): the same rules, except that an
/// idle time may be never.
pub(crate) fn read_kept(s: &mut Settings, body: &str) {
    for line in body.lines() {
        if let Some((k, v)) = line.split_once('=') {
            apply(s, k, v, Source::Kept);
        }
    }
}

/// Applies one choice made on the device, with the same offered-value checks as the file.
/// Returns whether the settings changed. A zone is clamped to -12:00 .. +14:00 as `main`'s
/// `set_tz` did.
pub(crate) fn choose(s: &mut Settings, choice: Choice) -> bool {
    match choice {
        Choice::IdleLock(m) if IDLE_LOCK_CHOICES.contains(&m) => set(&mut s.idle_lock_min, m),
        Choice::IdleOff(m) if IDLE_OFF_CHOICES.contains(&m) => set(&mut s.idle_off_min, m),
        Choice::QrFrame(ms) if QR_SPEEDS.contains(&ms) => set(&mut s.qr_frame_ms, ms),
        Choice::Zone(min) => set(&mut s.zone_min, Some(min.clamp(TZ_MIN, TZ_MAX))),
        Choice::Appearance(a) if Theme::from_row(usize::from(a.theme.0) * 6).is_some() => {
            let changed = s.appearance != a;
            s.appearance = a;
            changed
        }
        Choice::Scale(p) if SCALES.contains(&p) => set(&mut s.scale_pct, p),
        Choice::Rotation(o) => {
            let changed = s.rotation != o;
            s.rotation = o;
            changed
        }
        Choice::SealAmounts(on) => set(&mut s.seal_amounts, on),
        Choice::NonceHistory(on) => set(&mut s.nonce_history, on),
        _ => false,
    }
}

fn set<T: PartialEq>(field: &mut T, value: T) -> bool {
    let changed = *field != value;
    *field = value;
    changed
}

#[cfg(test)]
mod tests;
