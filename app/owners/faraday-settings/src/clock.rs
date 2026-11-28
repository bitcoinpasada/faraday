//! Set the time: what is typed, the checks on it and the offset it makes (`main`'s
//! `set_time.rs`). What is set is an offset against the computer's own clock; Faraday never
//! writes that clock.

use faraday_kernel::{Clock, Public};
use faraday_text::Text;

/// A computer clock below this (2020-01-01) is not a reading: a person's offset is what counts.
pub const CLOCK_FLOOR: u64 = 1_577_836_800;

/// Which of Set the time's two fields typing goes to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Focus {
    /// `YYYY-MM-DD`.
    Date,
    /// `HH:MM`, 24-hour.
    Time,
}

/// What Set the time holds: the two fields, where typing goes and whether the list of zones is
/// shown in their place. Nothing here is secret.
#[derive(Default, Public)]
pub(crate) struct Form {
    date: Text,
    time: Text,
    focus: Option<Focus>,
    zones: bool,
}

/// What a key typed on Set the time asks for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Typed {
    /// Not taken.
    Ignored,
    /// Taken by the fields.
    Taken,
    /// Enter with a real date and time: Set.
    Set,
    /// Escape: Back, or close the list of zones.
    Back,
}

impl Form {
    pub(crate) fn date(&self) -> &Text {
        &self.date
    }

    pub(crate) fn time(&self) -> &Text {
        &self.time
    }

    pub(crate) fn focus(&self) -> Option<Focus> {
        self.focus
    }

    pub(crate) fn zones(&self) -> bool {
        self.zones
    }

    /// Shows or hides the list of zones; typing goes to no field while it is shown.
    pub(crate) fn set_zones(&mut self, on: bool) {
        self.zones = on;
        if on {
            self.focus = None;
        }
    }

    pub(crate) fn set_focus(&mut self, focus: Focus) {
        self.focus = Some(focus);
    }

    pub(crate) fn date_empty(&self) -> bool {
        self.date.is_empty()
    }

    pub(crate) fn fill_date(&mut self, date: &str) {
        self.date.set(date);
    }

    /// Opens clean: the date starts at `date` when the clock is known, typing goes to Time.
    pub(crate) fn open(&mut self, date: Option<&str>) {
        self.date = Text::default();
        self.time = Text::default();
        self.zones = false;
        if let Some(d) = date {
            self.date.set(d);
        }
        self.focus = Some(Focus::Time);
    }

    /// Whether a real date and a real 24-hour time are typed.
    pub(crate) fn ready(&self) -> bool {
        valid_date(self.date.as_str()).is_some() && valid_time(self.time.as_str()).is_some()
    }

    /// The local time typed, as seconds since 1970 read as if local, at :00 seconds, with its
    /// hour and minute; `None` unless both are real.
    pub(crate) fn typed_local(&self) -> Option<(i64, u32, u32)> {
        let (y, m, d) = valid_date(self.date.as_str())?;
        let (h, mi) = valid_time(self.time.as_str())?;
        let local =
            days_from_civil(y, m, d) * 86_400 + i64::from(h) * 3600 + i64::from(mi) * 60;
        Some((local, h, mi))
    }

    /// A key typed: into the focused field, or to move focus. Only digits, `-` and `:` are taken.
    pub(crate) fn key(&mut self, key: Key) -> Typed {
        if self.zones {
            return if key == Key::Escape {
                Typed::Back
            } else {
                Typed::Ignored
            };
        }
        let Some(focus) = self.focus else {
            return Typed::Ignored;
        };
        match key {
            Key::Char(c) if c.is_ascii_digit() || c == '-' || c == ':' => {
                self.field(focus).push(c);
            }
            Key::Erase => {
                self.field(focus).pop();
            }
            Key::Tab => {
                self.focus = Some(match focus {
                    Focus::Date => Focus::Time,
                    Focus::Time => Focus::Date,
                });
            }
            Key::Enter if self.ready() => return Typed::Set,
            Key::Escape => return Typed::Back,
            _ => return Typed::Ignored,
        }
        Typed::Taken
    }

    fn field(&mut self, focus: Focus) -> &mut Text {
        match focus {
            Focus::Date => &mut self.date,
            Focus::Time => &mut self.time,
        }
    }
}

/// The keys Set the time takes, as the product maps them from the shell's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Key {
    Char(char),
    Erase,
    Tab,
    Enter,
    Escape,
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// The days a month of year `y` has, 0 for an impossible month.
fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(y) => 29,
        2 => 28,
        _ => 0,
    }
}

/// `s[a..b]` as a number of ASCII digits alone: a sign or a space inside a field is no number
/// (`main` let a leading `+` through).
fn digits(s: &str, a: usize, b: usize) -> Option<u32> {
    let part = s.get(a..b)?;
    if part.is_empty() || !part.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    part.parse().ok()
}

/// `s` as `YYYY-MM-DD`, a real calendar date from 2020-01-01 to 2099-12-31.
pub(crate) fn valid_date(s: &str) -> Option<(i64, u32, u32)> {
    if s.len() != 10 || s.as_bytes().get(4) != Some(&b'-') || s.as_bytes().get(7) != Some(&b'-') {
        return None;
    }
    let y = i64::from(digits(s, 0, 4)?);
    let m = digits(s, 5, 7)?;
    let d = digits(s, 8, 10)?;
    if !(2020..=2099).contains(&y) || !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
        return None;
    }
    Some((y, m, d))
}

/// `s` as `HH:MM`, 24-hour.
pub(crate) fn valid_time(s: &str) -> Option<(u32, u32)> {
    if s.len() != 5 || s.as_bytes().get(2) != Some(&b':') {
        return None;
    }
    let h = digits(s, 0, 2)?;
    let m = digits(s, 3, 5)?;
    (h <= 23 && m <= 59).then_some((h, m))
}

/// Days since 1970-01-01 of a civil date (Hinnant's algorithm).
pub(crate) fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = i64::from((m + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The civil date of a day count since 1970-01-01.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let m = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// Unix seconds turned into local seconds for a zone, minutes east of UTC.
pub(crate) fn local_secs(unix_secs: u64, tz_min: i16) -> u64 {
    let local = i64::try_from(unix_secs).unwrap_or(i64::MAX) + i64::from(tz_min) * 60;
    u64::try_from(local).unwrap_or(0)
}

/// `YYYY-MM-DD` of Unix seconds.
pub(crate) fn date_text(unix_secs: u64) -> Text {
    let days = i64::try_from(unix_secs / 86_400).unwrap_or(0);
    let (y, m, d) = civil_from_days(days);
    faraday_text::text!("{y:04}-{m:02}-{d:02}")
}

/// `HH:MM:SS` of Unix seconds.
pub(crate) fn time_text(unix_secs: u64) -> Text {
    let s = unix_secs % 86_400;
    faraday_text::text!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

/// Now on this computer's clock, Unix seconds: the reading corrected by the offset a time set by
/// hand gives it. With no offset, a reading below [`CLOCK_FLOOR`] is not known; with one and no
/// reading, the time counts on from Set by the adapter's own ticks.
pub(crate) fn now(clock: &Clock) -> Option<u64> {
    let raw = if clock.unix_secs != 0 {
        clock.unix_secs
    } else if clock.offset_secs != 0 {
        clock.now_ms / 1000
    } else {
        return None;
    };
    if clock.offset_secs != 0 {
        let at = i64::try_from(raw).unwrap_or(i64::MAX).saturating_add(clock.offset_secs);
        Some(u64::try_from(at).unwrap_or(0))
    } else {
        (raw >= CLOCK_FLOOR).then_some(raw)
    }
}

/// What the time starts from when it is not the computer's clock: no reading at all, or a time
/// set by hand counted on from Set.
pub(crate) fn source(clock: &Clock) -> Option<Source> {
    if clock.unix_secs != 0 {
        return None;
    }
    Some(if clock.offset_secs != 0 {
        Source::CountedFromSet
    } else {
        Source::NoReading
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Source {
    CountedFromSet,
    NoReading,
}

/// The offset Set makes: the local time typed, read in the zone, against the adapter's reading
/// now (its ticks when it has none).
pub(crate) fn offset_for(local: i64, tz_min: i16, clock: &Clock) -> i64 {
    let utc = local - i64::from(tz_min) * 60;
    let raw = if clock.unix_secs != 0 {
        clock.unix_secs
    } else {
        clock.now_ms / 1000
    };
    utc - i64::try_from(raw).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests;
