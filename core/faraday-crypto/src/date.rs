//! Civil dates and day counts: the proleptic Gregorian calendar against
//! days since 1970-01-01 (the Unix epoch's day).
//!
//! Both functions are Howard Hinnant's `days_from_civil` and
//! `civil_from_days`, from "chrono-Compatible Low-Level Date Algorithms".
//! They are exact for any day count and year whose working fits an `i64`
//! (years within ±1e15 cannot overflow); negative days are before 1970. A
//! caller checks the range of what it accepts.

/// The day count since 1970-01-01 of a proleptic Gregorian date.
///
/// `month` is 1 to 12 and `day` is 1 to the month's length; a day past the
/// month's end counts on into the next month. A month outside 1 to 12 is
/// not checked here.
pub fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = i64::from((month % 12 + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The days in month `month` (1 to 12) of `year`.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `YYYY-MM-DD` (surrounding space allowed) as year, month and day, or
/// `None` for anything else, a day the month does not have included.
pub fn parse_ymd(s: &str) -> Option<(i32, u32, u32)> {
    let mut it = s.trim().split('-');
    let y: i32 = it.next().filter(|p| p.len() == 4)?.parse().ok()?;
    let m: u32 = it.next().filter(|p| p.len() == 2)?.parse().ok()?;
    let d: u32 = it.next().filter(|p| p.len() == 2)?.parse().ok()?;
    if it.next().is_some() || !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
        return None;
    }
    Some((y, m, d))
}

/// The proleptic Gregorian year, month (1 to 12) and day of a day count
/// since 1970-01-01.
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KNOWN: [(i64, u32, u32, i64); 8] = [
        (1970, 1, 1, 0),
        (1969, 12, 31, -1),
        (2000, 2, 29, 11_016),
        (2000, 3, 1, 11_017),
        (1900, 3, 1, -25_508),
        (2038, 1, 19, 24_855),
        (2009, 1, 3, 14_247),
        (9999, 12, 31, 2_932_896),
    ];

    #[test]
    fn known_dates_have_their_day_counts() {
        for (y, m, d, days) in KNOWN {
            assert_eq!(days_from_civil(y, m, d), days, "{y}-{m}-{d}");
            assert_eq!(civil_from_days(days), (y, m, d), "{days}");
        }
    }

    #[test]
    fn every_day_from_year_0_to_10000_round_trips() {
        let first = days_from_civil(0, 1, 1);
        let last = days_from_civil(10_000, 1, 1);
        for days in first..=last {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
    }
}
