//! Settings' cards in the menu page's columns: they read in their order
//! down the first column and then the second, and the two columns come
//! out about equal in height at every width and text size.

use faraday_core::strings::EN;
use faraday_core::ui::TextSize;
use faraday_core::{Action, Faraday, Screen};
use faraday_shell_api::{App, BootState, DisplayInfo, Event, SecureHardware};

/// A card's edge at 160 dpi: what its ink stands in from its box.
const CARD_EDGE: i32 = 12;

/// Settings on a `width` × `height` window at 160 dpi.
fn settings_at(width: u16, height: u16) -> Faraday {
    let mut app = Faraday::new();
    app.event(Event::Display(DisplayInfo {
        width,
        height,
        dpi: 160,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: SecureHardware::None,
        boot: BootState::Unknown,
        memory_mib: None,
    }));
    app.press(Action::Nav(Screen::Settings));
    app
}

/// Every card title drawn with the box of the ink under it, pixels:
/// (left, top, bottom of the lowest line in the card's column below it).
fn titles(app: &mut Faraday) -> Vec<(&'static str, i32, i32)> {
    let _ = app.frame();
    let ink = app.drawn_text_boxes();
    let names = [
        EN.settings_lock_and_power,
        EN.settings_time,
        EN.own_randomness,
        EN.settings_appearance,
        EN.settings_display,
        EN.settings_what_remembered,
        EN.settings_about,
    ];
    names
        .iter()
        .map(|&n| {
            let b = ink
                .iter()
                .find(|(t, _)| t == n)
                .unwrap_or_else(|| panic!("{n:?} drawn"))
                .1;
            (n, b.0, b.1)
        })
        .collect()
}

#[test]
fn settings_columns_come_out_about_equal_and_keep_the_order() {
    for size in [TextSize::Standard, TextSize::Larger] {
        for (w, h) in [(1280, 2600), (1440, 2600), (1920, 2600), (2560, 2600)] {
            let mut app = settings_at(w, h);
            app.press(Action::TextSize(size));
            let cards = titles(&mut app);
            let ink = app.drawn_text_boxes();
            let first_x = cards[0].1;
            let second: Vec<_> = cards.iter().filter(|c| c.1 > first_x + 100).collect();
            if second.is_empty() {
                continue; // one column at this width and text size
            }
            // Order: down the first column, then the second.
            let col = |c: &(&str, i32, i32)| usize::from(c.1 > first_x + 100);
            assert!(
                cards.windows(2).all(|p| col(&p[0]) <= col(&p[1])),
                "{w}x{h}"
            );
            assert!(
                cards
                    .windows(2)
                    .all(|p| col(&p[0]) != col(&p[1]) || p[0].2 < p[1].2),
                "{w}x{h}: down each column in order"
            );
            // Each column's foot: its lowest line, and the tallest card.
            let left_foot = ink
                .iter()
                .filter(|(_, b)| b.0 >= first_x - 10 && b.0 < second[0].1 - 10)
                .map(|(_, b)| b.1 + b.3)
                .max()
                .unwrap();
            let right_foot = ink
                .iter()
                .filter(|(_, b)| b.0 >= second[0].1 - 10)
                .map(|(_, b)| b.1 + b.3)
                .max()
                .unwrap();
            let tops: Vec<i32> = cards.iter().map(|c| c.2).collect();
            let tallest = (0..cards.len())
                .map(|i| {
                    let col_foot = if col(&cards[i]) == 0 {
                        left_foot
                    } else {
                        right_foot
                    };
                    let next = cards[i + 1..]
                        .iter()
                        .find(|c| col(c) == col(&cards[i]))
                        .map_or(col_foot + 2 * CARD_EDGE, |c| c.2);
                    next - tops[i]
                })
                .max()
                .unwrap();
            assert!(
                (left_foot - right_foot).abs() <= tallest,
                "{w}x{h} {size:?}: columns end at {left_foot} and {right_foot}, tallest card {tallest}"
            );
        }
    }
}
