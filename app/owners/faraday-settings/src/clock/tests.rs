use super::*;

fn clock(unix_secs: u64, offset_secs: i64, now_ms: u64) -> Clock {
    Clock {
        now_ms,
        unix_secs,
        offset_secs,
        set: false,
        last_input_ms: 0,
    }
}

fn typed(form: &mut Form, text: &str) {
    for c in text.chars() {
        form.key(Key::Char(c));
    }
}

/// Invariant: The date typed is a real calendar date from 2020-01-01 to 2099-12-31 in the form YYYY-MM-DD, and nothing else is.
/// Regression: A date outside the years, a day the month does not have, a leap day in a common year, a sign inside a field or a wrong length is taken as a time to set.
/// Owner: app
#[test]
fn a_date_is_a_real_calendar_date_from_2020_to_2099() {
    for ok in ["2020-01-01", "2099-12-31", "2024-02-29"] {
        assert!(valid_date(ok).is_some(), "{ok}");
    }
    for refused in [
        "2019-12-31",
        "2000-01-01",
        "2100-01-01",
        "2023-02-29",
        "2100-02-29",
        "2024-04-31",
        "2024-13-01",
        "2024-00-10",
        "2024-01-00",
        "2024-1-01",
        "2024-01-011",
        "20240101",
        "2024/01/01",
        "2024-+1-05",
        "+024-01-05",
        "",
    ] {
        assert_eq!(valid_date(refused), None, "{refused}");
    }
    assert_eq!(valid_date("2096-02-29"), Some((2096, 2, 29)));
}

/// Invariant: The time typed is HH:MM on the 24-hour clock, hours to 23 and minutes to 59.
/// Regression: 24:00, 12:60, a time without its colon or with a sign or the wrong length is taken as a time to set.
/// Owner: app
#[test]
fn a_time_is_hh_mm_on_a_24_hour_clock() {
    assert_eq!(valid_time("00:00"), Some((0, 0)));
    assert_eq!(valid_time("23:59"), Some((23, 59)));
    for refused in ["24:00", "12:60", "1200", "1:30", "12:3", "12-30", "+1:30", "12:+5", ""] {
        assert_eq!(valid_time(refused), None, "{refused}");
    }
}

/// Invariant: Only digits, minus and colon are typed into a field; Tab swaps fields, and Enter sets only a real date and time.
/// Regression: A letter or a space is typed into a field, or Enter sets a half-typed time.
/// Owner: app
#[test]
fn only_digits_minus_and_colon_are_typed_and_enter_needs_a_real_time() {
    let mut form = Form::default();
    form.open(Some("2025-03-04"));
    assert_eq!(form.focus(), Some(Focus::Time));
    assert_eq!(form.key(Key::Char('a')), Typed::Ignored);
    assert_eq!(form.key(Key::Char(' ')), Typed::Ignored);
    typed(&mut form, "12:3");
    assert_eq!(form.time().as_str(), "12:3");
    assert_eq!(form.key(Key::Enter), Typed::Ignored);
    typed(&mut form, "0");
    assert_eq!(form.key(Key::Enter), Typed::Set);
    assert_eq!(form.key(Key::Erase), Typed::Taken);
    assert_eq!(form.time().as_str(), "12:3");
    assert_eq!(form.key(Key::Tab), Typed::Taken);
    assert_eq!(form.focus(), Some(Focus::Date));
    assert_eq!(form.key(Key::Escape), Typed::Back);
}

/// Invariant: While the list of zones is shown no field takes a key, and Escape closes the list before it goes back.
/// Regression: Typing goes into a hidden field behind the zone list, or Escape leaves Set the time with the list still open.
/// Owner: app
#[test]
fn the_zone_list_takes_no_typing_and_escape_closes_it() {
    let mut form = Form::default();
    form.open(None);
    form.set_zones(true);
    assert_eq!(form.focus(), None);
    assert_eq!(form.key(Key::Char('1')), Typed::Ignored);
    assert_eq!(form.key(Key::Escape), Typed::Back);
    assert!(form.zones());
}

/// Invariant: The local time typed becomes the offset against the computer's clock in the chosen zone, and the clock reads that time at once.
/// Regression: The zone is added instead of taken off, so the clock is wrong by twice the zone, or the offset is worked from a stale reading.
/// Owner: app
#[test]
fn set_makes_the_offset_that_reads_the_time_typed() {
    let mut form = Form::default();
    form.open(Some("2025-06-15"));
    typed(&mut form, "14:30");
    let (local, h, m) = form.typed_local().expect("a real time");
    assert_eq!((h, m), (14, 30));
    let reading = 1_790_000_000;
    let tz = 120;
    let offset = offset_for(local, tz, &clock(reading, 0, 5_000));
    let after = clock(reading, offset, 5_000);
    let t = now(&after).expect("known");
    assert_eq!(local_secs(t, tz), u64::try_from(local).expect("positive"));
    assert_eq!(date_text(local_secs(t, tz)).as_str(), "2025-06-15");
    assert_eq!(&time_text(local_secs(t, tz)).as_str()[..5], "14:30");
}

/// Invariant: With no offset a reading below 2020-01-01 or none at all is not a known time; with an offset and no reading the time counts on from Set by the adapter's ticks.
/// Regression: A computer that boots at 1970 is taken as the time, or a time set by hand with no reading stops counting.
/// Owner: app
#[test]
fn a_clock_below_the_floor_is_not_known_until_the_person_sets_it() {
    assert_eq!(now(&clock(0, 0, 9_000)), None);
    assert_eq!(now(&clock(CLOCK_FLOOR - 1, 0, 9_000)), None);
    assert_eq!(now(&clock(CLOCK_FLOOR, 0, 9_000)), Some(CLOCK_FLOOR));
    assert_eq!(source(&clock(0, 0, 0)), Some(Source::NoReading));
    let local = 1_790_000_000;
    let offset = offset_for(local, 0, &clock(0, 0, 10_000));
    assert_eq!(now(&clock(0, offset, 10_000)), Some(1_790_000_000));
    assert_eq!(now(&clock(0, offset, 70_000)), Some(1_790_000_060));
    assert_eq!(source(&clock(0, offset, 0)), Some(Source::CountedFromSet));
    assert_eq!(source(&clock(CLOCK_FLOOR, 0, 0)), None);
}
