//! On a wide window off Mainnet, a flow's column ends before the mode's
//! pill and the ? at the top right: nothing of it, scrolled to the top,
//! is under either.

use faraday_bip::keys::Network;
use faraday_core::{Action, Faraday, Screen, testkit};
use faraday_shell_api::{App, Event, Key};

fn wide_testnet(width: u16, height: u16) -> Faraday {
    let mut app = testkit::started();
    testkit::move_to(&mut app.session, Network::Testnet).unwrap();
    app.event(Event::Display(faraday_shell_api::DisplayInfo {
        width,
        height,
        dpi: 96,
        inset_bottom: 0,
        inset_top: 0,
        buttons: 0,
        camera_fixed: false,
        secure: faraday_shell_api::SecureHardware::None,
        boot: faraday_shell_api::BootState::Unknown,
        memory_mib: None,
    }));
    app
}

fn boxed(app: &Faraday, a: Action) -> Option<(i32, i32, i32, i32)> {
    let b = app.hit_box(a)?;
    Some((
        i32::from(b.0),
        i32::from(b.1),
        i32::from(b.2),
        i32::from(b.3),
    ))
}

/// Whether the screen had a pill to stand clear of.
fn clear_of_top_row(app: &mut Faraday, what: &str) -> bool {
    let _ = app.frame();
    let Some(pill) = boxed(app, Action::ModeSheet) else {
        return false;
    };
    let zones: Vec<_> = [Some(pill), boxed(app, Action::Learn)]
        .into_iter()
        .flatten()
        .collect();
    for (t, r) in app.drawn_text_boxes() {
        for &z in &zones {
            let inside =
                r.0 >= z.0 && r.0 + r.2 <= z.0 + z.2 && r.1 >= z.1 && r.1 + r.3 <= z.1 + z.3;
            let meets = r.0 < z.0 + z.2 && z.0 < r.0 + r.2 && r.1 < z.1 + z.3 && z.1 < r.1 + r.3;
            assert!(inside || !meets, "{what}: {t:?} {r:?} under {z:?}");
        }
    }
    // A flow's Cancel is its column's right end: it stops short of the pill.
    for (t, r) in app.drawn_text_boxes() {
        if t == "Cancel" {
            assert!(
                r.0 + r.2 < pill.0,
                "{what}: {t:?} {r:?} reaches the pill {pill:?}"
            );
            CANCELS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    true
}

static CANCELS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[test]
fn flows_end_before_the_testnet_pill_and_the_question_mark() {
    for (w, h) in [(1280, 800), (1920, 1080)] {
        let mut app = wide_testnet(w, h);
        app.press(Action::Nav(Screen::Restore));
        let mut checked = usize::from(clear_of_top_row(&mut app, "Restore"));
        // Make a new wallet, started with a key: its Cancel is the column's
        // right end.
        app.press(Action::Entry(None));
        for c in testkit::test_words("bacon").chars() {
            app.event(Event::Key(Key::Char(c)));
        }
        app.press(Action::EntryAdd);
        app.press(Action::CreateWallet);
        app.press(Action::CNext(faraday_core::cstep::KIND));
        checked += usize::from(clear_of_top_row(&mut app, "Make a new wallet"));
        app.press(Action::Bip85);
        checked += usize::from(clear_of_top_row(&mut app, "BIP 85"));
        for tile in 0..8 {
            app.press(Action::Nav(Screen::Catalog));
            app.press(Action::Catalog(tile));
            checked += usize::from(clear_of_top_row(&mut app, &format!("tile {tile}")));
        }
        assert!(
            CANCELS.load(std::sync::atomic::Ordering::Relaxed) > 0,
            "no flow was open"
        );
        assert!(checked >= 3, "only {checked} pages had the pill");
    }
}
