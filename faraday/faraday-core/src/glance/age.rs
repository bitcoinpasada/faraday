//! How long ago a copy was checked (`docs/BACKUP-FLOW.md` §18.3): a date
//! kept in the vault becomes an age, and a copy is due once it is 12
//! months old or has no date, with the clock checked.

use crate::strings::EN;
use crate::strings::fill;

/// A copy is due when its last check is this many whole months old.
pub const INTERVAL_MONTHS: u32 = 12;

/// How long a check lasts: 12 months, or a wallet's margin in days where
/// that is shorter (`docs/BACKUP-FLOW.md` §18.3). Against the 12 months
/// ending today, a margin of 365 days is shorter when they hold 29 February.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Interval {
    /// The margin of a wallet with a relative wait, in days (never under
    /// 1), where it is shorter than 12 months.
    days: Option<u32>,
}

impl Interval {
    /// The interval for a wallet whose margin is `margin` days, `None`
    /// for a wallet with no relative wait.
    pub fn of(margin: Option<u32>) -> Interval {
        Interval {
            days: margin.filter(|m| *m <= 365).map(|m| m.max(1)),
        }
    }

    /// The interval in words: "12 months", "27 days".
    pub fn text(self) -> String {
        match self.days {
            Some(1) => EN.glance_interval_day.to_string(),
            Some(d) => fill(EN.glance_interval_days, &[&d.to_string()]),
            None => EN.glance_interval_months.to_string(),
        }
    }
}

/// A calendar day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Day {
    y: i32,
    m: u32,
    d: u32,
}

impl Day {
    /// `YYYY-MM-DD`, or `None`.
    fn parse(s: &str) -> Option<Day> {
        let (y, m, d) = faraday_crypto::date::parse_ymd(s)?;
        Some(Day { y, m, d })
    }

    /// Days since 1970-01-01.
    fn number(self) -> i64 {
        faraday_crypto::date::days_from_civil(i64::from(self.y), self.m, self.d)
    }
}

fn month_days(y: i32, m: u32) -> u32 {
    faraday_crypto::date::days_in_month(y, m)
}

/// Whole months from `from` to `to` (not before it): a month-end day
/// counts as complete on the last day of a shorter month.
fn months(from: Day, to: Day) -> u32 {
    let total = (to.y - from.y) * 12 + to.m as i32 - from.m as i32;
    let day = from.d.min(month_days(to.y, to.m));
    (total - i32::from(to.d < day)).max(0) as u32
}

/// How old `date` is on `today`: `None` with no checked clock (`today`
/// empty), no date, a date that is not one, or one later than today.
/// Days, whole months.
fn span(date: &str, today: &str) -> Option<(i64, u32)> {
    let t = Day::parse(today)?;
    let d = Day::parse(date)?;
    (d <= t).then(|| (t.number() - d.number(), months(d, t)))
}

/// A date as an age: "today", "yesterday", "{n} days ago" under 60 days,
/// "{n} months ago" under 24 whole months, "{n} years ago" after. `None`
/// where no age is told: no checked clock, no date, or a date later than
/// today (shown as the date).
pub fn age_text(date: &str, today: &str) -> Option<String> {
    let (days, m) = span(date, today)?;
    Some(match days {
        0 => EN.glance_age_today.to_string(),
        1 => EN.glance_age_yesterday.to_string(),
        2..60 => fill(EN.glance_age_days, &[&days.to_string()]),
        _ if m == 1 => EN.glance_age_month.to_string(),
        _ if m < 24 => fill(EN.glance_age_months, &[&m.to_string()]),
        _ => fill(EN.glance_age_years, &[&(m / 12).to_string()]),
    })
}

/// Whole days from `date` to `today`: `None` where no age is told (as
/// [`age_text`]).
pub(crate) fn age_days(date: &str, today: &str) -> Option<i64> {
    span(date, today).map(|(days, _)| days)
}

/// Whether a copy last checked on `date` is due on `today`: its age is
/// the interval or more, or it has no date. Never with the clock not
/// checked, and not for a date later than today.
pub fn is_due(date: &str, today: &str, iv: Interval) -> bool {
    if Day::parse(today).is_none() {
        return false;
    }
    match Day::parse(date) {
        None => true,
        Some(_) => span(date, today).is_some_and(|(days, m)| {
            m >= INTERVAL_MONTHS || iv.days.is_some_and(|n| days >= i64::from(n))
        }),
    }
}

/// A date with its age: "2025-08-14 · 14 months ago"; the date alone
/// where no age is told, "Checked · no date" for none.
pub(crate) fn date_age(date: &str, today: &str) -> String {
    match age_text(date, today) {
        Some(a) => fill(EN.glance_sheet_date_age, &[date, &a]),
        None if date.is_empty() => EN.glance_checked.to_string(),
        None => date.to_string(),
    }
}

/// What a place's copies say of when they were checked, worked out once
/// where the chart is made.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeAge {
    /// The clock is checked: ages and due marks are told.
    pub clock: bool,
    /// The oldest date among its copies that have one.
    pub oldest: Option<String>,
    /// A copy there was never checked.
    pub never: bool,
    /// A copy there was checked with no date.
    pub undated: bool,
    /// The oldest date as an age.
    pub text: Option<String>,
    /// A copy there is due.
    pub due: bool,
    /// A stick there was read from an older copy of its vault.
    pub older: bool,
}

impl NodeAge {
    /// From each copy's last check as the vault keeps it (`None`: never,
    /// empty: no date) on `today`, `older` where a stick there was read
    /// from an older copy of its vault; `None` for a place with no copies.
    pub fn of(
        checked: &[Option<String>],
        older: bool,
        today: &str,
        iv: Interval,
    ) -> Option<NodeAge> {
        if checked.is_empty() {
            return None;
        }
        let clock = Day::parse(today).is_some();
        let dated = checked.iter().flatten().filter(|d| !d.is_empty());
        let oldest = dated.filter_map(|d| Day::parse(d).map(|p| (p, d))).min();
        let oldest = oldest.map(|(_, d)| d.clone());
        let never = checked.iter().any(Option::is_none);
        let undated = checked.iter().flatten().any(|d| Day::parse(d).is_none());
        let due =
            clock && (older || never || checked.iter().flatten().any(|d| is_due(d, today, iv)));
        Some(NodeAge {
            clock,
            text: oldest.as_deref().and_then(|d| age_text(d, today)),
            oldest,
            never,
            undated,
            due,
            older,
        })
    }

    /// The line under a place's name: its text, whether `WARN`.
    pub fn line(&self) -> (String, bool) {
        if self.older {
            (EN.glance_older_copy.to_string(), true)
        } else if self.never {
            (EN.glance_never_checked.to_string(), true)
        } else if self.undated && self.oldest.is_none() {
            (EN.glance_checked.to_string(), self.due)
        } else {
            let when = self
                .text
                .clone()
                .or_else(|| self.oldest.clone())
                .unwrap_or_default();
            (fill(EN.glance_last_checked_n, &[&when]), self.due)
        }
    }
}

impl NodeAge {
    /// The place's sheet value for its oldest copy: the date with its age
    /// and the due mark, and whether it reads `WARN`.
    pub(crate) fn sheet_value(&self, today: &str) -> (String, bool) {
        let base = if self.older {
            EN.glance_older_copy.to_string()
        } else if self.never {
            EN.glance_never_checked.to_string()
        } else {
            date_age(self.oldest.as_deref().unwrap_or(""), today)
        };
        due_value(base, self.due)
    }
}

/// `base`, with the due mark where `due`.
pub(crate) fn due_value(base: String, due: bool) -> (String, bool) {
    if due {
        (fill(EN.glance_sheet_due, &[&base]), true)
    } else {
        (base, false)
    }
}
