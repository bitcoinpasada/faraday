//! How long ago each copy was checked (`docs/BACKUP-FLOW.md` §18.3): a
//! date becomes an age, a copy is due after 12 months or with no date, and
//! with the clock not checked no age or due mark is shown.

use crate::chart_edit::{
    clock_kept, edit_and_confirm, fresh_vault, line_holding, node_of, one_key_wait, open, planned,
    recovery_with_vault, saved_at_lock, settle, single_key_with_vault, unlocked_again, vault_file,
};
use crate::common::type_text;
use faraday_core::glance::age::{Interval, NodeAge, is_due};
use faraday_core::glance::sheet::{ChartAction as C, Target};
use faraday_core::glance::{Press, age_text};
use faraday_core::plan::{At, Edit, What};
use faraday_core::set_time::Focus;
use faraday_core::vault::VaultAction as V;
use faraday_core::wallet_backup::Tone;
use faraday_core::{Action, Faraday, Screen, Sheet, testkit};
use faraday_shell_api::{App, Event, Key};

/// Set the time by hand to `date` at noon.
fn set_clock(app: &mut Faraday, date: &str) {
    set_clock_at(app, date, "12:00");
}

/// Set the time by hand to `date` at `time`.
fn set_clock_at(app: &mut Faraday, date: &str, time: &str) {
    app.press(Action::OpenSetTime);
    // The fields start with the clock's own date and time.
    app.press(Action::SetTimeFocus(Focus::Date));
    for _ in 0..10 {
        app.event(Event::Key(Key::Backspace));
    }
    type_text(app, date);
    app.press(Action::SetTimeFocus(Focus::Time));
    for _ in 0..5 {
        app.event(Event::Key(Key::Backspace));
    }
    type_text(app, time);
    app.press(Action::SetTimeSet);
}

/// Whether the screen says `want`, across the lines a text wraps to.
fn shown(app: &mut Faraday, want: &str) -> bool {
    app.drawn_texts().join(" ").contains(want)
}

/// The wallet's card, drawn once.
fn card(app: &mut Faraday, w: usize) {
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
}

/// A wallet with a plan and a vault, both places marked checked on the
/// test clock's day (2026-10-03).
fn checked_place() -> (Faraday, usize) {
    let (mut app, w) = single_key_with_vault();
    clock_kept(&mut app);
    mark_both(&mut app, w);
    (app, w)
}

/// Both places of `w` marked checked on the test clock's day.
fn mark_both(app: &mut Faraday, w: usize) {
    for place in ["Place 1", "Place 2"] {
        let (n, _) = node_of(app, w, place);
        app.press(Action::Chart(C::Checked(Press::Loaded(w), n)));
    }
}

/// Place `place`'s line on Backups: its state and tone.
fn backups_line(app: &mut Faraday, place: &str) -> (String, Tone) {
    let c = testkit::compute();
    app.press(Action::Nav(Screen::Backups));
    let _ = app.frame();
    let all = app.backups_now(&c);
    let l = all[0].lines.clone().expect("a plan");
    let l = l.into_iter().find(|l| l.place == place).unwrap();
    (l.state, l.tone)
}

#[test]
fn a_date_reads_as_an_age_at_each_boundary() {
    let t = "2026-10-23";
    let age = |d: &str| age_text(d, t).unwrap_or_default();
    assert_eq!(age("2026-10-23"), "today");
    assert_eq!(age("2026-10-22"), "yesterday");
    assert_eq!(age("2026-10-20"), "3 days ago");
    assert_eq!(age("2026-08-25"), "59 days ago");
    assert_eq!(age("2026-08-24"), "1 month ago");
    assert_eq!(age("2024-11-23"), "23 months ago");
    assert_eq!(age("2024-10-23"), "2 years ago");
    // A month-end day is whole on the last day of a shorter month.
    assert_eq!(
        age_text("2024-02-29", "2025-02-28").unwrap(),
        "12 months ago"
    );
    assert_eq!(
        age_text("2025-08-31", "2026-08-30").unwrap(),
        "11 months ago"
    );
    assert_eq!(
        age_text("2025-08-31", "2026-09-30").unwrap(),
        "13 months ago"
    );
}

#[test]
fn a_copy_is_due_after_twelve_months_or_with_no_date_once_the_clock_is_checked() {
    let t = "2026-10-23";
    assert_eq!(age_text("2025-08-14", t).unwrap(), "14 months ago");
    assert!(is_due("2025-08-14", t, Interval::default()));
    assert!(!is_due("2026-01-10", t, Interval::default()));
    assert!(is_due("", t, Interval::default()));
    assert!(!is_due("2025-08-14", "", Interval::default()));
    assert!(!is_due("", "", Interval::default()));
}

#[test]
fn a_date_later_than_today_shows_no_age_and_is_not_due() {
    assert_eq!(age_text("2026-11-01", "2026-10-23"), None);
    assert!(!is_due("2026-11-01", "2026-10-23", Interval::default()));
    let (mut app, w) = checked_place();
    set_clock(&mut app, "2026-09-01");
    card(&mut app, w);
    let (_, node) = node_of(&app, w, "Place 1");
    let age = node.age.expect("the place has an age");
    assert_eq!((age.text, age.due), (None, false));
    assert!(shown(&mut app, "Last checked 2026-10-03"));
    assert!(!shown(&mut app, "To check"));
}

#[test]
fn a_place_checked_over_a_year_ago_says_so_and_is_due_on_its_card_and_sheet() {
    let (mut app, w) = checked_place();
    set_clock(&mut app, "2027-12-20");
    card(&mut app, w);
    let (n, node) = node_of(&app, w, "Place 1");
    assert_eq!(
        node.age.as_ref().unwrap().text.as_deref(),
        Some("14 months ago")
    );
    assert!(node.age.as_ref().unwrap().due);
    assert!(shown(&mut app, "Last checked 14 months ago"));
    assert!(shown(&mut app, "To check: Place 1"));
    // The strip's press opens that place's sheet.
    assert!(app.offers(Action::Chart(C::Open(Press::Loaded(w), Target::Node(n)))));
    open(&mut app, Press::Loaded(w), Target::Node(n));
    assert!(shown(&mut app, "2026-10-03 · 14 months ago · To check"));
}

#[test]
fn a_place_becomes_due_at_midnight_on_a_card_left_open() {
    let (mut app, w) = checked_place();
    set_clock_at(&mut app, "2027-10-02", "23:59");
    card(&mut app, w);
    assert!(shown(&mut app, "Last checked 11 months ago"));
    assert!(!shown(&mut app, "To check"));
    // Two minutes pass with nothing pressed.
    let now = app.now_ms();
    app.event(Event::Tick {
        now_ms: now + 120_000,
    });
    let _ = app.frame();
    assert!(shown(&mut app, "Last checked 12 months ago"));
    assert!(shown(&mut app, "To check: Place 1"));
}

#[test]
fn a_place_checked_within_the_year_has_nothing_to_check() {
    let (mut app, w) = checked_place();
    set_clock(&mut app, "2027-01-10");
    card(&mut app, w);
    assert!(shown(&mut app, "Last checked 3 months ago"));
    assert!(shown(&mut app, "Every place checked in the last 12 months"));
    assert!(!shown(&mut app, "To check"));
}

#[test]
fn marking_a_place_checked_changes_its_age_line_on_the_next_frame() {
    let (mut app, w) = checked_place();
    set_clock(&mut app, "2027-12-20");
    card(&mut app, w);
    assert!(shown(&mut app, "Last checked 14 months ago"));
    for place in ["Place 1", "Place 2"] {
        let (n, _) = node_of(&app, w, place);
        app.press(Action::Chart(C::Checked(Press::Loaded(w), n)));
    }
    let _ = app.frame();
    assert!(shown(&mut app, "Last checked today"));
    assert!(!shown(&mut app, "Last checked 14 months ago"));
    assert!(!shown(&mut app, "To check"));
    assert!(shown(&mut app, "Every place checked in the last 12 months"));
}

#[test]
fn with_the_clock_not_checked_no_age_or_due_mark_is_shown_and_the_strip_asks_for_the_time() {
    let (mut app, w) = single_key_with_vault();
    card(&mut app, w);
    assert_eq!(app.screen, Screen::Wallets);
    assert!(shown(&mut app, "Never checked"));
    assert!(!shown(&mut app, "To check"));
    assert!(shown(&mut app, "Set the time to see what is due"));
    assert!(app.offers(Action::OpenSetTime));
    // Marked checked with no clock: the date is empty and no age is told.
    let (n, _) = node_of(&app, w, "Place 1");
    app.press(Action::Chart(C::Checked(Press::Loaded(w), n)));
    let _ = app.frame();
    assert!(!shown(&mut app, "To check"));
    assert!(shown(&mut app, "Set the time to see what is due"));
}

#[test]
fn a_paper_place_with_a_date_reads_its_age_on_the_backups_screen() {
    let (mut app, w) = checked_place();
    let c = testkit::compute();
    app.press(Action::Nav(Screen::Backups));
    let _ = app.frame();
    let line = |app: &Faraday| {
        let all = app.backups_now(&c);
        let l = all[0].lines.clone().expect("a plan");
        let l = l.into_iter().find(|l| l.place == "Place 1").unwrap();
        (l.state, l.tone)
    };
    assert_eq!(line(&app), ("Checked today".to_string(), Tone::Ok));
    set_clock(&mut app, "2027-12-20");
    app.press(Action::Nav(Screen::Backups));
    let _ = app.frame();
    assert_eq!(
        line(&app),
        ("Checked 14 months ago · To check".to_string(), Tone::Warn)
    );
    let _ = w;
}

#[test]
fn a_place_with_a_copy_never_checked_reads_never_checked_though_another_is_dated() {
    let t = "2026-10-03";
    let age = NodeAge::of(
        &[Some("2026-10-03".to_string()), None],
        false,
        t,
        Interval::default(),
    )
    .unwrap();
    assert_eq!(age.line(), ("Never checked".to_string(), true));
    assert!(age.due);
}

#[test]
fn a_wallets_margin_shortens_the_interval_and_the_strip_names_it() {
    let short = Interval::of(Some(27));
    let t = "2026-10-23";
    assert!(is_due("2026-08-24", t, short));
    assert!(!is_due("2026-08-24", t, Interval::default()));
    assert!(!is_due("2026-10-10", t, short));
    assert_eq!(short.text(), "27 days");
    assert_eq!(Interval::of(Some(400)).text(), "12 months");
    assert_eq!(Interval::of(None).text(), "12 months");
}

#[test]
fn a_due_place_carries_the_to_check_mark_on_its_node() {
    let (mut app, w) = checked_place();
    set_clock(&mut app, "2027-12-20");
    card(&mut app, w);
    let marks = app
        .drawn_texts()
        .iter()
        .filter(|t| t.as_str() == "To check")
        .count();
    assert_eq!(marks, 2, "{:?}", app.drawn_texts());
}

#[test]
fn a_recovery_wallets_card_tells_its_shorter_interval_and_a_copy_due_inside_the_year() {
    // A wait of 28 days: the margin is 25 days.
    let (mut app, w) = recovery_with_vault(4032);
    clock_kept(&mut app);
    mark_both(&mut app, w);
    card(&mut app, w);
    assert!(shown(&mut app, "Every place checked in the last 25 days"));
    set_clock(&mut app, "2026-12-01");
    card(&mut app, w);
    assert!(shown(&mut app, "To check: Place 1, Place 2"));
    assert!(!shown(&mut app, "Every place checked"));
}

/// A wallet whose vault stick was read from an older copy of its vault,
/// opened on its card.
fn older_stick_read() -> Faraday {
    let (mut app, _) = single_key_with_vault();
    clock_kept(&mut app);
    let (outbox, _) = saved_at_lock(&mut app);
    let (name, old) = vault_file(outbox);
    let new = testkit::kdbx_reseal(&old, "test phrase", 9).unwrap();
    let files = vec![("a-new.kdbx".to_string(), new), (name.clone(), old)];
    // The newer copy is opened and locked, so the older one stops at its
    // dialog and is opened from there.
    let mut app = unlocked_again(files, &[&name], 0);
    testkit::vault_lock_one(&mut app, 0);
    app.press(Action::Nav(Screen::Vaults));
    app.press(Action::Vault(V::Open(1)));
    type_text(&mut app, "test phrase");
    app.press(Action::Vault(V::Unlock));
    settle(&mut app, |a| a.sheet == Some(Sheet::OlderCopy));
    app.press(Action::Vault(V::OlderOpen));
    settle(&mut app, |a| !a.vaults().open.is_empty());
    app.press(Action::Vault(V::LoadWithKeys(0)));
    app.press(Action::OpenWallet(0));
    let _ = app.frame();
    app
}

#[test]
fn a_place_whose_stick_was_read_from_an_older_copy_reads_older_copy_though_nothing_is_dated() {
    let mut app = older_stick_read();
    let (node, _) = line_holding(&app, 0, What::VaultStick(0));
    assert_eq!(
        backups_line(&mut app, &node.name),
        ("Older copy".to_string(), Tone::Warn)
    );
}

#[test]
fn a_place_holding_only_the_vault_stick_read_from_an_older_copy_reads_older_copy_as_on_the_chart() {
    let mut app = older_stick_read();
    // The paper copies at the stick's place move to the other place.
    let (stick_at, _) = line_holding(&app, 0, What::VaultStick(0));
    let other = At::Place(
        1 - match stick_at.at {
            At::Place(p) => p,
            at => panic!("the stick is at {at:?}"),
        },
    );
    for what in [What::Words(0), What::Sheet] {
        edit_and_confirm(
            &mut app,
            Press::Loaded(0),
            Edit::Move(stick_at.at, what, other),
        );
    }
    let (node, _) = line_holding(&app, 0, What::VaultStick(0));
    assert_eq!(node.whats, vec![What::VaultStick(0)], "a stick-only place");
    let on_chart = node.state.clone().map(|(t, _)| t);
    assert_eq!(
        backups_line(&mut app, &node.name),
        ("Older copy".to_string(), Tone::Warn)
    );
    assert_eq!(on_chart.as_deref(), Some("Older copy"));
}

#[test]
fn a_place_checked_with_no_clock_reads_to_check_once_the_clock_is_checked() {
    let (mut app, w) = single_key_with_vault();
    mark_both(&mut app, w);
    assert_eq!(
        backups_line(&mut app, "Place 1"),
        ("Checked".to_string(), Tone::Ok)
    );
    clock_kept(&mut app);
    assert_eq!(
        backups_line(&mut app, "Place 1"),
        ("To check".to_string(), Tone::Warn)
    );
}

#[test]
fn a_margin_of_nothing_is_one_day_due_the_next_day() {
    let none = Interval::of(Some(0));
    assert_eq!(none.text(), "1 day");
    assert!(!is_due("2026-10-23", "2026-10-23", none));
    assert!(is_due("2026-10-23", "2026-10-24", none));
}

#[test]
fn a_margin_of_a_year_counts_against_twelve_months_over_a_leap_day() {
    let year = Interval::of(Some(365));
    assert!(is_due("2027-03-01", "2028-02-29", year));
    assert!(!is_due("2027-03-01", "2028-02-29", Interval::default()));
}

/// WCAG's contrast ratio of two sRGB colours.
fn ratio([r1, g1, b1]: [u8; 3], [r2, g2, b2]: [u8; 3]) -> f64 {
    let lum = |c: [f64; 3]| {
        let l = |v: f64| {
            let v = v / 255.0;
            if v <= 0.039_28 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * l(c[0]) + 0.7152 * l(c[1]) + 0.0722 * l(c[2])
    };
    let a = lum([r1, g1, b1].map(f64::from));
    let b = lum([r2, g2, b2].map(f64::from));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[test]
fn the_set_the_time_button_and_its_line_read_on_the_strip_in_every_theme() {
    let (mut app, w) = single_key_with_vault();
    card(&mut app, w);
    for theme in faraday_ui::Theme::all() {
        app.press(Action::Theme(theme));
        crate::common::a_moment(&mut app);
        let boxes = app.drawn_text_boxes();
        let frame = app.frame();
        let (fw, rgba) = (usize::from(frame.width), frame.rgba.to_vec());
        for label in ["Set the time", "Set the time to see what is due"] {
            let b = boxes
                .iter()
                .find(|(t, _)| t == label)
                .unwrap_or_else(|| panic!("{theme:?}: {label} is drawn"))
                .1;
            // The fill just left of the text, inside the strip and the button.
            let at = |x: i32, y: i32| {
                let i = (y as usize * fw + x as usize) * 4;
                [rgba[i], rgba[i + 1], rgba[i + 2]]
            };
            let under = at(b.0 - 2, b.1 + b.3 / 2);
            // The text's own colour: its darkest or lightest ink, the one
            // furthest from the fill.
            let mut ink = under;
            let mut best = 1.0;
            for x in b.0..b.0 + b.2 {
                for y in b.1..b.1 + b.3 {
                    let p = at(x, y);
                    let r = ratio(p, under);
                    if r > best {
                        best = r;
                        ink = p;
                    }
                }
            }
            let got = ratio(ink, under);
            assert!(got >= 4.5, "{theme:?}: {label} on the strip {got:.2}:1");
        }
    }
}

#[test]
fn a_copy_never_checked_is_not_shown_in_the_success_tone_on_its_sheet() {
    use faraday_draw::canvas::InkKind;
    let (mut app, w) = single_key_with_vault();
    card(&mut app, w);
    let (n, _) = node_of(&app, w, "Place 1");
    open(&mut app, Press::Loaded(w), Target::Node(n));
    let ink = app.drawn_ink();
    // The sheet stands at the right: the rightmost such text.
    let colour = |want: &str| {
        ink.iter()
            .filter_map(|i| match &i.kind {
                InkKind::Text { text, color, .. } if text == want => Some((i.rect.x, *color)),
                _ => None,
            })
            .max_by_key(|(x, _)| *x)
            .map(|(_, c)| c)
            .unwrap_or_else(|| panic!("{want} is drawn"))
    };
    let ok = faraday_ui::Theme::default().role(faraday_ui::pal::Role::Ok);
    let never = colour("Never checked");
    assert_ne!(
        (never.r, never.g, never.b),
        (ok.r, ok.g, ok.b),
        "Never checked reads in the success tone"
    );
    // The plain value tone, as the line above it.
    assert_eq!(never, colour("Can spend"));
}

/// The colour of the drawn line that contains `part`.
fn ink_of(app: &mut Faraday, part: &str) -> String {
    use faraday_draw::canvas::InkKind;
    app.drawn_ink()
        .into_iter()
        .find_map(|i| match i.kind {
            InkKind::Text { text, color, .. } if text.contains(part) => Some(format!("{color:?}")),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{part} is not drawn"))
}

#[test]
fn a_recovery_wallets_copies_are_due_at_its_margin_and_i_moved_every_coin_dates_the_coins() {
    // A wait of 52 560 blocks: the margin is 328 days, under 12 months.
    let (mut app, w) = recovery_with_vault(52_560);
    clock_kept(&mut app);
    mark_both(&mut app, w);
    card(&mut app, w);
    assert!(shown(&mut app, "Every place checked in the last 328 days"));
    assert!(!shown(&mut app, "Coins last moved"));
    app.press(Action::Chart(C::Moved(Press::Loaded(w))));
    card(&mut app, w);
    assert!(shown(&mut app, "Coins last moved"));
    assert!(shown(&mut app, "today · the wait ends at about 365 days"));
    // The date is in the vault: it survives a lock and an unlock.
    let (outbox, _) = saved_at_lock(&mut app);
    let (name, _) = vault_file(outbox.clone());
    let mut app = unlocked_again(outbox, &[&name], 0);
    app.press(Action::Vault(V::LoadTicked));
    let w = (0..2)
        .find(|&i| {
            card(&mut app, i);
            shown(&mut app, "Spend paths")
        })
        .expect("the recovery wallet is loaded");
    assert!(shown(&mut app, "today · the wait ends at about 365 days"));
    // The warning tone from the margin on, not before.
    set_clock(&mut app, "2027-08-26");
    card(&mut app, w);
    assert!(shown(&mut app, "10 months ago · the wait"));
    assert_eq!(
        ink_of(&mut app, "10 months ago · the wait"),
        ink_of(&mut app, "Any time")
    );
    set_clock(&mut app, "2027-08-27");
    card(&mut app, w);
    assert!(shown(&mut app, "10 months ago · the wait"));
    assert_ne!(
        ink_of(&mut app, "10 months ago · the wait"),
        ink_of(&mut app, "Any time")
    );
}

#[test]
fn with_no_vault_open_the_action_reads_open_a_vault_to_keep_this_and_holds_nothing() {
    // A plan made with no vault open, the clock checked.
    let mut app = crate::chart_edit::desktop();
    let w = one_key_wait(&mut app, "", Some(52_560));
    planned(&mut app, w, 2);
    clock_kept(&mut app);
    app.press(Action::OpenWallet(w));
    let _ = app.frame();
    assert!(shown(&mut app, "Open a vault to keep this"));
    assert!(!shown(&mut app, "I moved every coin"));
    app.press(Action::Chart(C::Moved(Press::Loaded(w))));
    // Nothing was held: with a vault open there is no date.
    fresh_vault(&mut app);
    card(&mut app, w);
    assert!(shown(&mut app, "I moved every coin"));
    assert!(!shown(&mut app, "Coins last moved"));
}

#[test]
fn with_the_clock_not_checked_the_card_offers_set_the_time_until_it_is_set() {
    let (mut app, w) = recovery_with_vault(52_560);
    card(&mut app, w);
    assert!(app.offers(Action::OpenSetTime));
    assert!(!app.offers(Action::Chart(C::Moved(Press::Loaded(w)))));
    assert!(!shown(&mut app, "I moved every coin"));
    clock_kept(&mut app);
    card(&mut app, w);
    assert!(shown(&mut app, "I moved every coin"));
    app.press(Action::Chart(C::Moved(Press::Loaded(w))));
    card(&mut app, w);
    assert!(shown(&mut app, "today · the wait ends at about 365 days"));
}

#[test]
fn a_recovery_wallet_with_no_plan_has_no_i_moved_every_coin() {
    let mut app = crate::wallet_paths::with(crate::wallet_paths::YEAR);
    let _ = app.frame();
    assert!(shown(&mut app, "Spend paths"));
    assert!(!shown(&mut app, "I moved every coin"));
    assert!(!shown(&mut app, "Set the time"));
    assert!(!shown(&mut app, "Open a vault to keep this"));
}

#[test]
fn the_moved_note_round_trips_and_a_malformed_one_is_refused() {
    use faraday_core::plan::Note;
    let n = Note::Moved("2026-10-03".to_string());
    assert_eq!(n.to_text(), "moved 2026-10-03");
    assert_eq!(Note::from_text(&n.to_text(), 1), Some(n));
    for bad in [
        "moved",
        "moved ",
        "moved yesterday",
        "moved 2026-13-01",
        "moved 2026-10-03 extra",
        "moved 2026-10-03 x y",
    ] {
        assert_eq!(Note::from_text(bad, 1), None, "{bad}");
    }
}
