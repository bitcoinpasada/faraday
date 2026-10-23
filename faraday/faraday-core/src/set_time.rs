//! Setting the time by hand (`docs/TOTP.md` §1.2 and §3): a screen
//! reached from Settings' Time section. What is set is an offset against
//! the shell's own clock; Faraday never writes the computer's clock
//! itself (`docs/TOTP.md` decision 7).

use crate::strings::EN;
use crate::strings::fill;
use faraday_shell_api::Key as KeyIn;

use crate::vault::TextBox;
use crate::{Action, Faraday, Screen, Sheet};

/// Which of Set the time's two fields typing goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    /// `YYYY-MM-DD`.
    Date,
    /// `HH:MM`, 24-hour.
    Time,
}

/// Set the time's area: what is typed, where typing goes, and where Back
/// returns to. Its fields are private; the glue below reaches the clock.
pub(crate) struct SetTimeArea {
    /// The date typed, `YYYY-MM-DD`.
    date: TextBox,
    /// The time typed, `HH:MM`.
    time: TextBox,
    /// Where typing goes; `None` while neither field is focused.
    focus: Option<Focus>,
    /// The screen Back, Set and Keep this time return to: Settings, or
    /// (batch 2) the codes screen.
    back_to: Screen,
    /// The sheet Set the time was opened from (Sign with {key}, F63),
    /// shown again over `back_to` by Back, Set and Keep this time.
    sheet: Option<Sheet>,
}

impl Default for SetTimeArea {
    fn default() -> Self {
        SetTimeArea {
            date: TextBox::default(),
            time: TextBox::default(),
            focus: None,
            back_to: Screen::Settings,
            sheet: None,
        }
    }
}

/// What a key typed on the Set the time screen asks for.
enum Typed {
    /// Not taken.
    Ignored,
    /// Taken by the fields.
    Taken,
    /// Enter with a real date and time: Set.
    Set,
    /// Escape: Back.
    Back,
}

impl SetTimeArea {
    /// The date typed.
    pub(crate) fn date(&self) -> &TextBox {
        &self.date
    }

    /// The time typed.
    pub(crate) fn time(&self) -> &TextBox {
        &self.time
    }

    /// Where typing goes.
    pub(crate) fn focus(&self) -> Option<Focus> {
        self.focus
    }

    /// The screen Back, Set and Keep this time return to.
    pub(crate) fn back_to(&self) -> Screen {
        self.back_to
    }

    /// Focuses a field.
    pub(crate) fn set_focus(&mut self, f: Focus) {
        self.focus = Some(f);
    }

    /// Opens on `screen`, which it returns to, and the sheet over it
    /// (taken from `sheet`), which it shows again; the date starts at
    /// `local_date` when the clock is known.
    pub(crate) fn open(
        &mut self,
        screen: Screen,
        sheet: &mut Option<Sheet>,
        local_date: Option<&str>,
    ) {
        self.back_to = screen;
        self.sheet = sheet.take();
        self.date = TextBox::default();
        self.time = TextBox::default();
        if let Some(d) = local_date {
            self.date.text.set(d);
        }
        self.focus = Some(Focus::Time);
    }

    /// Whether a real date and a real 24-hour time are typed.
    pub(crate) fn ready(&self) -> bool {
        valid_date(&self.date.text).is_some() && valid_time(&self.time.text).is_some()
    }

    /// The local time typed, as seconds since 1970 read as if local, at
    /// :00 seconds, with its hour and minute; `None` unless both are
    /// real.
    pub(crate) fn typed_local(&self) -> Option<(i64, u32, u32)> {
        let (y, m, d) = valid_date(&self.date.text)?;
        let (h, mi) = valid_time(&self.time.text)?;
        let local = days_from_civil(y, m, d) * 86_400 + i64::from(h) * 3600 + i64::from(mi) * 60;
        Some((local, h, mi))
    }

    /// The screen to go back to, and the sheet to show again over it.
    pub(crate) fn take_return(&mut self) -> (Screen, Option<Sheet>) {
        (self.back_to, self.sheet.take())
    }

    /// A key typed: into the focused field, or to move focus.
    fn key(&mut self, key: KeyIn) -> Typed {
        let Some(focus) = self.focus else {
            return Typed::Ignored;
        };
        match key {
            KeyIn::Char(c) if c.is_ascii_digit() || c == '-' || c == ':' => {
                self.field(focus).text.push(c);
            }
            KeyIn::Backspace => {
                self.field(focus).text.pop();
            }
            KeyIn::Tab => {
                self.focus = Some(match focus {
                    Focus::Date => Focus::Time,
                    Focus::Time => Focus::Date,
                });
            }
            KeyIn::Enter if self.ready() => return Typed::Set,
            KeyIn::Escape => return Typed::Back,
            _ => return Typed::Ignored,
        }
        Typed::Taken
    }

    fn field(&mut self, focus: Focus) -> &mut TextBox {
        match focus {
            Focus::Date => &mut self.date,
            Focus::Time => &mut self.time,
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
                let local = crate::vault::local_secs(t, self.tz_min());
                format!(
                    "{} {}",
                    crate::gpg::date(local),
                    crate::vault::time_of_day_secs(local)
                )
            }
            None => EN.set_time_not_known.to_string(),
        }
    }

    /// Steps the time zone by `delta` minutes, stopping at −12:00 and
    /// +14:00 (`docs/TOTP.md` §2.1). The same action for Settings' row
    /// and Set the time's context row.
    pub(crate) fn tz_step(&mut self, delta: i16) {
        self.settings.step_tz(delta);
        self.save_boxes();
    }

    /// Opens Set the time from `self.screen`, which it returns to, and
    /// from the sheet over it, which it shows again.
    pub(crate) fn open_set_time(&mut self) {
        let date = self
            .clock_now()
            .map(|t| crate::gpg::date(crate::vault::local_secs(t, self.tz_min())));
        self.set_time
            .open(self.screen, &mut self.sheet, date.as_deref());
        self.screen = Screen::SetTime;
    }

    /// Before something that keeps the time it reads (a GPG key's
    /// creation, a signature, a Secure Boot certificate): with the clock
    /// not checked (the stick and the Pi until the person says), opens
    /// Set the time over the page, which it returns to, and says so. The
    /// person then presses again. A GPG key's creation time is part of
    /// its fingerprint, so a wrong clock is not put right later
    /// (`docs/TOTP.md` §1.3).
    pub(crate) fn time_asked(&mut self) -> bool {
        if self.clock_checked() {
            return false;
        }
        self.open_set_time();
        true
    }

    /// Whether Set the time's Set button is enabled: a real date and a
    /// real 24-hour time are typed.
    pub(crate) fn set_time_ready(&self) -> bool {
        self.set_time.ready()
    }

    /// Set: the local time typed, at :00 seconds, becomes the offset
    /// against the shell's own clock (`docs/TOTP.md` §1.2 and §3). Never
    /// writes the computer's clock.
    pub(crate) fn set_time_set(&mut self) {
        let Some((local, h, mi)) = self.set_time.typed_local() else {
            return;
        };
        let utc = local - i64::from(self.tz_min()) * 60;
        let raw = self.vaults.unix_secs.unwrap_or(0) as i64
            + (self.now_ms.saturating_sub(self.vaults.clock_at_ms) / 1000) as i64;
        self.vaults.clock_offset = Some(utc - raw);
        self.vaults.clock_checked = true;
        self.toast(&fill(
            EN.set_time_done_toast,
            &[&format!("{h:02}"), &format!("{mi:02}")],
        ));
        self.set_time_return();
        self.save_boxes();
    }

    /// Back to the screen and the sheet Set the time was opened from.
    /// Signs nothing and makes nothing there (F63).
    fn set_time_return(&mut self) {
        let (back_to, sheet) = self.set_time.take_return();
        self.return_back(back_to);
        if self.sheet.is_none() {
            self.sheet = sheet;
        }
    }

    /// Keep this time: marks the clock checked, sets no offset.
    pub(crate) fn set_time_keep(&mut self) {
        if self.clock_now().is_some() {
            self.vaults.clock_checked = true;
        }
        self.set_time_return();
        self.save_boxes();
    }

    /// Back: goes back without marking anything.
    pub(crate) fn set_time_back(&mut self) {
        self.set_time_return();
    }

    /// Typing on the Set the time screen; `false` when it is not shown or
    /// nothing is focused.
    pub(crate) fn set_time_key(&mut self, key: KeyIn) -> bool {
        if self.screen != Screen::SetTime {
            return false;
        }
        match self.set_time.key(key) {
            Typed::Ignored => return false,
            Typed::Taken => {}
            Typed::Set => self.set_time_set(),
            Typed::Back => self.set_time_back(),
        }
        true
    }

    /// The screen Set the time returns to.
    pub fn set_time_returns_to(&self) -> Screen {
        self.set_time.back_to()
    }
}

impl Faraday {
    /// Handles the clock's actions (`TzStep`, `OpenSetTime`, `SetTime*`).
    pub(crate) fn time_act(&mut self, action: Action) {
        match action {
            Action::TzStep(delta) => self.tz_step(delta),
            Action::OpenSetTime => self.open_set_time(),
            Action::SetTimeFocus(f) => self.set_time.set_focus(f),
            Action::SetTimeSet => {
                if self.set_time_ready() {
                    self.set_time_set();
                }
            }
            Action::SetTimeKeep => self.set_time_keep(),
            Action::SetTimeBack => self.set_time_back(),
            // `act_inner` passes only the actions matched above.
            _ => unreachable!(),
        }
    }
}
