//! Setting the time by hand (`docs/TOTP.md` §1.2 and §3): a screen
//! reached from Settings' Time section. What is set is an offset against
//! the shell's own clock; Faraday never writes the computer's clock
//! itself (`docs/TOTP.md` decision 7).

use osk_shell_api::Key as KeyIn;

use crate::vaults::TextBox;
use crate::{Faraday, Screen};

/// Which of Set the time's two fields typing goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// `YYYY-MM-DD`.
    Date,
    /// `HH:MM`, 24-hour.
    Time,
}

/// Set the time's own state.
pub struct SetTime {
    /// The date typed, `YYYY-MM-DD`.
    pub date: TextBox,
    /// The time typed, `HH:MM`.
    pub time: TextBox,
    /// Where typing goes; `None` while neither field is focused.
    pub focus: Option<Focus>,
    /// The screen Back, Set and Keep this time return to: Settings, or
    /// (batch 2) the codes screen.
    pub back_to: Screen,
}

impl Default for SetTime {
    fn default() -> Self {
        SetTime {
            date: TextBox::default(),
            time: TextBox::default(),
            focus: None,
            back_to: Screen::Settings,
        }
    }
}

/// Whether `y` is a leap year.
fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// The days a month of year `y` has, 0 for an impossible month.
fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(y) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Howard Hinnant's civil-to-days, the inverse of [`crate::gpg::date`]'s
/// days-to-civil.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = u64::from((m + 9) % 12);
    let doy = ((153 * mp + 2) / 5) as i64 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `s` as `YYYY-MM-DD`, a real calendar date from 2020-01-01 to
/// 2099-12-31.
fn valid_date(s: &str) -> Option<(i64, u32, u32)> {
    if s.len() != 10 || s.as_bytes().get(4) != Some(&b'-') || s.as_bytes().get(7) != Some(&b'-') {
        return None;
    }
    let y: i64 = s.get(0..4)?.parse().ok()?;
    let m: u32 = s.get(5..7)?.parse().ok()?;
    let d: u32 = s.get(8..10)?.parse().ok()?;
    if !(2020..=2099).contains(&y) || !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
        return None;
    }
    Some((y, m, d))
}

/// `s` as `HH:MM`, 24-hour.
fn valid_time(s: &str) -> Option<(u32, u32)> {
    if s.len() != 5 || s.as_bytes().get(2) != Some(&b':') {
        return None;
    }
    let h: u32 = s.get(0..2)?.parse().ok()?;
    let m: u32 = s.get(3..5)?.parse().ok()?;
    (h <= 23 && m <= 59).then_some((h, m))
}

impl Faraday {
    /// The live local date and time, seconds shown, or "Not known"
    /// (`docs/TOTP.md` §2.1).
    pub fn now_local_text(&self) -> String {
        match self.clock_now() {
            Some(t) => {
                let local = crate::vaults::local_secs(t, self.tz_min);
                format!(
                    "{} {}",
                    crate::gpg::date(local),
                    crate::vaults::time_of_day_secs(local)
                )
            }
            None => "Not known".to_string(),
        }
    }

    /// Steps the time zone by `delta` minutes, stopping at −12:00 and
    /// +14:00 (`docs/TOTP.md` §2.1). The same action for Settings' row
    /// and Set the time's context row.
    pub(crate) fn tz_step(&mut self, delta: i16) {
        self.tz_min = (self.tz_min + delta).clamp(-720, 840);
        self.save_boxes();
    }

    /// Opens Set the time from `self.screen`, which it returns to.
    pub(crate) fn open_set_time(&mut self) {
        self.set_time.back_to = self.screen;
        self.set_time.date = TextBox::default();
        self.set_time.time = TextBox::default();
        if let Some(t) = self.clock_now() {
            let local = crate::vaults::local_secs(t, self.tz_min);
            self.set_time.date.text.set(&crate::gpg::date(local));
        }
        self.set_time.focus = Some(Focus::Time);
        self.screen = Screen::SetTime;
    }

    /// Whether Set the time's Set button is enabled: a real date and a
    /// real 24-hour time are typed.
    pub(crate) fn set_time_ready(&self) -> bool {
        valid_date(&self.set_time.date.text).is_some()
            && valid_time(&self.set_time.time.text).is_some()
    }

    /// Set: the local time typed, at :00 seconds, becomes the offset
    /// against the shell's own clock (`docs/TOTP.md` §1.2 and §3). Never
    /// writes the computer's clock.
    pub(crate) fn set_time_set(&mut self) {
        let Some((y, m, d)) = valid_date(&self.set_time.date.text) else {
            return;
        };
        let Some((h, mi)) = valid_time(&self.set_time.time.text) else {
            return;
        };
        let local = days_from_civil(y, m, d) * 86_400 + i64::from(h) * 3600 + i64::from(mi) * 60;
        let utc = local - i64::from(self.tz_min) * 60;
        let raw = self.vaults.unix_secs.unwrap_or(0) as i64
            + (self.now_ms.saturating_sub(self.vaults.clock_at_ms) / 1000) as i64;
        self.vaults.clock_offset = Some(utc - raw);
        self.vaults.clock_checked = true;
        self.toast(&format!("Time set to {h:02}:{mi:02}"));
        self.screen = self.set_time.back_to;
        self.save_boxes();
    }

    /// Keep this time: marks the clock checked, sets no offset.
    pub(crate) fn set_time_keep(&mut self) {
        if self.clock_now().is_some() {
            self.vaults.clock_checked = true;
        }
        self.screen = self.set_time.back_to;
        self.save_boxes();
    }

    /// Back: goes back without marking anything.
    pub(crate) fn set_time_back(&mut self) {
        self.screen = self.set_time.back_to;
    }

    /// Typing on the Set the time screen; `false` when it is not shown or
    /// nothing is focused.
    pub(crate) fn set_time_key(&mut self, key: KeyIn) -> bool {
        if self.screen != Screen::SetTime {
            return false;
        }
        let Some(focus) = self.set_time.focus else {
            return false;
        };
        match key {
            KeyIn::Char(c) if c.is_ascii_digit() || c == '-' || c == ':' => {
                self.set_time_box(focus).text.push(c);
            }
            KeyIn::Backspace => {
                self.set_time_box(focus).text.pop();
            }
            KeyIn::Tab => {
                self.set_time.focus = Some(match focus {
                    Focus::Date => Focus::Time,
                    Focus::Time => Focus::Date,
                });
            }
            KeyIn::Enter if self.set_time_ready() => self.set_time_set(),
            KeyIn::Escape => self.set_time_back(),
            _ => return false,
        }
        true
    }

    fn set_time_box(&mut self, focus: Focus) -> &mut TextBox {
        match focus {
            Focus::Date => &mut self.set_time.date,
            Focus::Time => &mut self.set_time.time,
        }
    }
}
